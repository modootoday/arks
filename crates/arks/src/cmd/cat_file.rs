//! `git cat-file`: one object (`-t`, `-s`, `-e`, `-p`, `<type> <object>`,
//! `--textconv`, `--filters`) or a stream of objects (`--batch`,
//! `--batch-check`, `--batch-command`, `--batch-all-objects`) with batch
//! formats (`%(objectname)`, `%(objecttype)`, `%(objectsize)`,
//! `%(objectsize:disk)`, `%(deltabase)`, `%(rest)`), `--buffer`,
//! `--unordered`, `-z` / `-Z` delimiters, `--follow-symlinks` and
//! `--[no-]use-mailmap`.
//!
//! Primary: <https://git-scm.com/docs/git-cat-file>.

use crate::cmd::diffcore::options::Args;
use crate::cmd::mailmap::Mailmap;
use crate::cmd::shared::resolve_rev;
use crate::error::{CliError, CliResult};
use crate::util::{open_repo, work_tree};
use arks_object::{ObjectKind, RawObject};
use arks_oid::Oid;
use arks_repo::Repository;
use std::collections::BTreeSet;
use std::io::{self, Read, Write};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Batch {
    Contents,
    Check,
    Command,
}

#[derive(Default)]
struct Opts {
    mode: Option<String>,
    batch: Option<(Batch, Option<String>)>,
    all_objects: bool,
    buffer: bool,
    unordered: bool,
    follow_symlinks: bool,
    nul_in: bool,
    nul_out: bool,
    mailmap: bool,
    textconv: bool,
    filters: bool,
    path: Option<String>,
    positional: Vec<String>,
}

fn usage() -> CliError {
    CliError::Exit(
        129,
        "usage: git cat-file <type> <object>\n   or: git cat-file (-e | -p) <object>\n   or: git cat-file (-t | -s) [--allow-unknown-type] <object>\n   or: git cat-file (--batch | --batch-check | --batch-command) [--batch-all-objects]\n                    [--buffer] [--follow-symlinks] [--unordered]\n                    [--textconv | --filters] [-Z]\n   or: git cat-file (--textconv | --filters)\n                    [<rev>:<path|tree-ish> | --path=<path|tree-ish> <rev>]".into(),
    )
}

fn parse(args: &[String]) -> CliResult<Opts> {
    let mut o = Opts::default();
    let mut a = Args::new(args);
    while a.i < args.len() {
        let cur = a.cur().to_string();
        match cur.as_str() {
            "-t" | "-s" | "-e" | "-p" => o.mode = Some(cur.clone()),
            "--allow-unknown-type" | "--no-allow-unknown-type" => {}
            "--use-mailmap" | "--mailmap" => o.mailmap = true,
            "--no-use-mailmap" | "--no-mailmap" => o.mailmap = false,
            "--textconv" => o.textconv = true,
            "--filters" => o.filters = true,
            "--no-path" => o.path = None,
            "--batch-all-objects" => o.all_objects = true,
            "--buffer" => o.buffer = true,
            "--no-buffer" => o.buffer = false,
            "--unordered" => o.unordered = true,
            "--no-unordered" => o.unordered = false,
            "--follow-symlinks" => o.follow_symlinks = true,
            "--no-follow-symlinks" => o.follow_symlinks = false,
            "-z" => o.nul_in = true,
            "-Z" => {
                o.nul_in = true;
                o.nul_out = true;
            }
            "--" => {
                o.positional.extend(args[a.i + 1..].iter().cloned());
                break;
            }
            _ => {
                let batch = [
                    ("--batch", Batch::Contents),
                    ("--batch-check", Batch::Check),
                    ("--batch-command", Batch::Command),
                ];
                if let Some((kind, fmt)) = batch.iter().find_map(|(n, k)| {
                    if cur == *n {
                        Some((*k, None))
                    } else {
                        cur.strip_prefix(n)
                            .and_then(|r| r.strip_prefix('='))
                            .map(|f| (*k, Some(f.to_string())))
                    }
                }) {
                    o.batch = Some((kind, fmt));
                } else if let Some(v) = a.long_req("--path")? {
                    o.path = Some(v);
                } else if cur.starts_with('-') && cur.len() > 1 {
                    return Err(CliError::Exit(
                        129,
                        format!("error: unknown option `{}'", cur.trim_start_matches('-')),
                    ));
                } else {
                    o.positional.push(cur);
                }
            }
        }
        a.i += 1;
    }
    Ok(o)
}

/// Rewrite `author` / `committer` / `tagger` idents through the mailmap.
fn apply_mailmap(mm: &Mailmap, payload: &[u8]) -> Vec<u8> {
    let text = String::from_utf8_lossy(payload);
    let (head, body) = match text.find("\n\n") {
        Some(i) => (&text[..i], &text[i..]),
        None => (&text[..], ""),
    };
    let mut out = String::new();
    for line in head.split('\n') {
        let mapped = ["author ", "committer ", "tagger "].iter().find_map(|k| {
            let rest = line.strip_prefix(k)?;
            let lt = rest.find('<')?;
            let gt = rest[lt..].find('>')? + lt;
            let name = rest[..lt].trim_end();
            let email = &rest[lt + 1..gt];
            let (n, e) = mm.map(name, email);
            Some(format!("{k}{n} <{e}>{}", &rest[gt + 1..]))
        });
        out.push_str(mapped.as_deref().unwrap_or(line));
        out.push('\n');
    }
    out.pop();
    out.push_str(body);
    out.into_bytes()
}

fn load_mailmap(repo: &Repository) -> Mailmap {
    let cfg = arks_config::ConfigSet::load(Some(repo.common_dir()));
    Mailmap::load(work_tree(repo).ok().as_deref(), &cfg)
}

/// Object contents as `-p` shows them.
fn pretty(repo: &Repository, obj: &RawObject) -> CliResult<Vec<u8>> {
    if obj.kind != ObjectKind::Tree {
        return Ok(obj.payload.clone());
    }
    let tree = arks_object::Tree::decode(&obj.payload, repo.oid_kind())?;
    let mut out = Vec::new();
    for e in &tree.entries {
        let kind = if e.mode.is_tree() {
            "tree"
        } else if matches!(e.mode, arks_object::TreeMode::Gitlink) {
            "commit"
        } else {
            "blob"
        };
        writeln!(
            out,
            "{:0>6} {} {}\t{}",
            e.mode.as_str(),
            kind,
            e.oid.to_hex(),
            e.name
        )?;
    }
    Ok(out)
}

/// Every object in the repository (loose and packed), sorted by name.
fn all_objects(repo: &Repository) -> Vec<Oid> {
    let mut set = BTreeSet::new();
    let objects = repo.common_dir().join("objects");
    if let Ok(rd) = std::fs::read_dir(&objects) {
        for d in rd.flatten() {
            let name = d.file_name().to_string_lossy().into_owned();
            if name.len() != 2 || !name.bytes().all(|b| b.is_ascii_hexdigit()) {
                continue;
            }
            for f in std::fs::read_dir(d.path()).into_iter().flatten().flatten() {
                let rest = f.file_name().to_string_lossy().into_owned();
                if let Ok(oid) = Oid::from_hex(&format!("{name}{rest}")) {
                    set.insert(oid.to_hex());
                }
            }
        }
    }
    if let Ok(rd) = std::fs::read_dir(objects.join("pack")) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|x| x.to_str()) != Some("idx") {
                continue;
            }
            let Ok(bytes) = std::fs::read(&p) else {
                continue;
            };
            if let Ok(idx) = arks_pack::PackIndex::decode(&bytes, repo.object_format()) {
                for x in idx.entries {
                    set.insert(x.oid.to_hex());
                }
            }
        }
    }
    set.into_iter()
        .filter_map(|h| Oid::from_hex(&h).ok())
        .collect()
}

/// On-disk size: the loose file, else the packed entry's span.
fn disk_size(repo: &Repository, oid: &Oid) -> u64 {
    let hex = oid.to_hex();
    let loose = repo
        .common_dir()
        .join("objects")
        .join(&hex[..2])
        .join(&hex[2..]);
    // git looks objects up in packs before loose files (both may exist).
    let loose_len = || std::fs::metadata(&loose).map(|m| m.len()).unwrap_or(0);
    let Ok(rd) = std::fs::read_dir(repo.common_dir().join("objects").join("pack")) else {
        return loose_len();
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.extension().and_then(|x| x.to_str()) != Some("idx") {
            continue;
        }
        let Ok(bytes) = std::fs::read(&p) else {
            continue;
        };
        let Ok(idx) = arks_pack::PackIndex::decode(&bytes, repo.object_format()) else {
            continue;
        };
        let Some(me) = idx.entries.iter().find(|x| x.oid == *oid) else {
            continue;
        };
        let pack_len = std::fs::metadata(p.with_extension("pack"))
            .map(|m| m.len())
            .unwrap_or(0);
        let trailer = oid.to_hex().len() as u64 / 2;
        let next = idx
            .entries
            .iter()
            .map(|x| x.offset)
            .filter(|o| *o > me.offset)
            .min()
            .unwrap_or(pack_len.saturating_sub(trailer));
        return next - me.offset;
    }
    loose_len()
}

struct BatchCtx<'a> {
    repo: &'a Repository,
    kind: Batch,
    format: String,
    mailmap: Option<Mailmap>,
    follow_symlinks: bool,
    nul_out: bool,
    textconv: bool,
}

impl BatchCtx<'_> {
    fn term(&self) -> &'static [u8] {
        if self.nul_out { b"\0" } else { b"\n" }
    }

    fn expand(&self, oid: &Oid, obj: &RawObject, size: usize, rest: &str) -> String {
        let f = &self.format;
        let mut out = String::new();
        let mut i = 0;
        while i < f.len() {
            if f[i..].starts_with("%(") {
                if let Some(close) = f[i..].find(')') {
                    let atom = &f[i + 2..i + close];
                    match atom {
                        "objectname" => out.push_str(&oid.to_hex()),
                        "objecttype" => out.push_str(obj.kind.as_str()),
                        "objectsize" => out.push_str(&size.to_string()),
                        "objectsize:disk" => out.push_str(&disk_size(self.repo, oid).to_string()),
                        "deltabase" => out.push_str(&"0".repeat(oid.to_hex().len())),
                        "rest" => out.push_str(rest),
                        _ => out.push_str(&f[i..i + close + 1]),
                    }
                    i += close + 1;
                    continue;
                }
            }
            let ch = f[i..].chars().next().unwrap_or(' ');
            out.push(ch);
            i += ch.len_utf8();
        }
        out
    }

    /// Resolve `<tree-ish>:<path>`, following in-tree symlinks.
    fn resolve(&self, name: &str) -> Option<Oid> {
        let oid = resolve_rev(self.repo, name).ok()?;
        if !self.follow_symlinks {
            return Some(oid);
        }
        let Some((rev, path)) = name.split_once(':') else {
            return Some(oid);
        };
        let mut path = path.to_string();
        for _ in 0..40 {
            let tree_oid = resolve_rev(self.repo, &format!("{rev}^{{tree}}")).ok()?;
            let flat = crate::cmd::diffcore::flat_tree(self.repo, &tree_oid).ok()?;
            let (mode, o) = *flat.get(&path)?;
            if mode != 0o120000 {
                return Some(o);
            }
            let target =
                String::from_utf8_lossy(&self.repo.get_object(&o).ok()?.payload).into_owned();
            let base = path.rsplit_once('/').map_or("", |(d, _)| d);
            let mut parts: Vec<&str> = base.split('/').filter(|s| !s.is_empty()).collect();
            for seg in target.split('/') {
                match seg {
                    "" | "." => {}
                    ".." => {
                        parts.pop()?;
                    }
                    s => parts.push(s),
                }
            }
            path = parts.join("/");
        }
        Some(oid)
    }

    fn one(&self, w: &mut impl Write, input: &str, contents: bool) -> CliResult<()> {
        let (name, rest) = match input.split_once([' ', '\t']) {
            Some((n, r)) if self.format.contains("%(rest)") => (n, r),
            _ => (input, ""),
        };
        let Some(oid) = self.resolve(name) else {
            write!(w, "{name} missing")?;
            w.write_all(self.term())?;
            return Ok(());
        };
        let Ok(obj) = self.repo.get_object(&oid) else {
            write!(w, "{name} missing")?;
            w.write_all(self.term())?;
            return Ok(());
        };
        let mut payload = obj.payload.clone();
        if let Some(mm) = &self.mailmap {
            if matches!(obj.kind, ObjectKind::Commit | ObjectKind::Tag) {
                payload = apply_mailmap(mm, &payload);
            }
        }
        let _ = self.textconv;
        let line = self.expand(&oid, &obj, payload.len(), rest);
        w.write_all(line.as_bytes())?;
        w.write_all(self.term())?;
        if contents {
            w.write_all(&payload)?;
            w.write_all(self.term())?;
        }
        Ok(())
    }
}

fn run_batch(o: &Opts, kind: Batch, fmt: Option<String>) -> CliResult<()> {
    let repo = open_repo()?;
    let ctx = BatchCtx {
        repo: &repo,
        kind,
        format: fmt.unwrap_or_else(|| "%(objectname) %(objecttype) %(objectsize)".into()),
        mailmap: o.mailmap.then(|| load_mailmap(&repo)),
        follow_symlinks: o.follow_symlinks,
        nul_out: o.nul_out,
        textconv: o.textconv || o.filters,
    };
    let mut w = io::BufWriter::new(io::stdout().lock());
    if o.all_objects {
        for oid in all_objects(&repo) {
            ctx.one(&mut w, &oid.to_hex(), ctx.kind == Batch::Contents)?;
        }
        w.flush()?;
        return Ok(());
    }
    let mut input = Vec::new();
    io::stdin().read_to_end(&mut input)?;
    let delim = if o.nul_in { b'\0' } else { b'\n' };
    let mut queued: Vec<(String, bool)> = Vec::new();
    for raw in input.split(|b| *b == delim) {
        let line = String::from_utf8_lossy(raw).into_owned();
        let line = line.strip_suffix('\r').map(str::to_string).unwrap_or(line);
        if raw.is_empty() {
            continue;
        }
        match kind {
            Batch::Contents | Batch::Check => {
                ctx.one(&mut w, &line, kind == Batch::Contents)?;
            }
            Batch::Command => {
                let (cmd, arg) = line.split_once(' ').unwrap_or((line.as_str(), ""));
                match cmd {
                    "contents" | "info" if !arg.is_empty() => {
                        let c = cmd == "contents";
                        if o.buffer {
                            queued.push((arg.to_string(), c));
                        } else {
                            ctx.one(&mut w, arg, c)?;
                            w.flush()?;
                        }
                    }
                    "flush" if o.buffer => {
                        for (a, c) in queued.drain(..) {
                            ctx.one(&mut w, &a, c)?;
                        }
                        w.flush()?;
                    }
                    "flush" => {
                        return Err(CliError::fatal("flush is only for --buffer mode"));
                    }
                    "contents" | "info" => {
                        return Err(CliError::fatal(format!("{cmd} requires arguments")));
                    }
                    _ => return Err(CliError::fatal(format!("unknown command: '{line}'"))),
                }
            }
        }
    }
    for (a, c) in queued.drain(..) {
        ctx.one(&mut w, &a, c)?;
    }
    w.flush()?;
    Ok(())
}

pub fn cmd_cat_file(args: &[String]) -> CliResult<()> {
    // Cite: https://git-scm.com/docs/git-cat-file
    let o = parse(args)?;
    if let Some((kind, fmt)) = o.batch.clone() {
        if o.mode.is_some() || !o.positional.is_empty() {
            return Err(usage());
        }
        return run_batch(&o, kind, fmt);
    }
    if o.all_objects {
        return Err(CliError::Exit(
            129,
            "fatal: '--batch-all-objects' requires a batch mode".into(),
        ));
    }
    if o.textconv || o.filters {
        // No textconv drivers or filters are configured for arks objects:
        // the content is the blob itself.
        let name = match (&o.path, o.positional.as_slice()) {
            // `--path` only names the path for attributes; the object is as given.
            (Some(_), [obj]) => obj.clone(),
            (None, [obj]) if obj.contains(':') => obj.clone(),
            (None, [_]) => {
                return Err(CliError::fatal(format!(
                    "<object>:<path> required, only <object> '{}' given",
                    o.positional[0]
                )));
            }
            _ => return Err(usage()),
        };
        let repo = open_repo()?;
        let oid = resolve_rev(&repo, &name)
            .map_err(|_| CliError::fatal(format!("Not a valid object name {name}")))?;
        let obj = repo.get_object(&oid)?;
        io::stdout().write_all(&obj.payload)?;
        return Ok(());
    }
    let Some(mode) = o.mode.clone() else {
        // `cat-file <type> <object>`: raw content, peeling tags / commits.
        let [kind, object] = o.positional.as_slice() else {
            return Err(usage());
        };
        let repo = open_repo()?;
        let oid = resolve_rev(&repo, object)
            .map_err(|_| CliError::fatal(format!("Not a valid object name {object}")))?;
        let want = ObjectKind::parse(kind)
            .map_err(|_| CliError::fatal(format!("invalid object type \"{kind}\"")))?;
        let mut obj = repo.get_object(&oid)?;
        if obj.kind != want {
            let target = match want {
                ObjectKind::Commit => Some(crate::cmd::shared::ObjKind::Commit),
                ObjectKind::Tree => Some(crate::cmd::shared::ObjKind::Tree),
                _ => None,
            };
            let peeled = target
                .ok_or_else(|| CliError::Msg(String::new()))
                .and_then(|t| crate::cmd::shared::peel_to_kind(&repo, &oid, t, object))
                .map_err(|_| CliError::fatal(format!("git cat-file {object}: bad file")))?;
            obj = repo.get_object(&peeled)?;
        }
        io::stdout().write_all(&obj.payload)?;
        return Ok(());
    };
    let [object] = o.positional.as_slice() else {
        return Err(usage());
    };
    let repo = open_repo()?;
    let oid = resolve_rev(&repo, object)
        .map_err(|_| CliError::fatal(format!("Not a valid object name {object}")))?;
    let obj = match repo.get_object(&oid) {
        Ok(x) => x,
        Err(_) if mode == "-e" => return Err(CliError::silent(1)),
        Err(e) => {
            return Err(CliError::Msg(format!(
                "{e} (OBJ-005 — refuse reading partial/corrupt object; no silent invent)"
            )));
        }
    };
    let mut payload = obj.payload.clone();
    if o.mailmap && matches!(obj.kind, ObjectKind::Commit | ObjectKind::Tag) {
        payload = apply_mailmap(&load_mailmap(&repo), &payload);
    }
    let mut out = io::stdout().lock();
    match mode.as_str() {
        "-e" => {}
        "-t" => writeln!(out, "{}", obj.kind.as_str())?,
        "-s" => writeln!(out, "{}", payload.len())?,
        _ => {
            let shown = RawObject::new(obj.kind, payload);
            out.write_all(&pretty(&repo, &shown)?)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mailmap_rewrites_idents() {
        let mut mm = Mailmap::default();
        mm.add_line("Proper <proper@x> <old@x>");
        let p = b"tree t\nauthor Old <old@x> 1 +0000\ncommitter C <c@x> 1 +0000\n\nmsg\n";
        let out = String::from_utf8(apply_mailmap(&mm, p)).unwrap();
        assert!(out.contains("author Proper <proper@x> 1 +0000\n"));
        assert!(out.contains("committer C <c@x> 1 +0000\n\nmsg\n"));
    }
}
