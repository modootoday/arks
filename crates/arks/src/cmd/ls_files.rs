//! `git ls-files`: index entries (`-c`, `-s`, `-u`), work-tree state
//! (`-d`, `-m`, `-k`), untracked files (`-o`, `--directory`,
//! `--[no-]empty-directory`) and ignored ones (`-i`), with exclude rules
//! from `--exclude-standard`, `-x`, `-X` and `--exclude-per-directory`.
//! Output: status tags (`-t`, `-v`, `-f`), `--eol` line-ending columns,
//! `--format` atoms, `--abbrev`, `--debug` stat data, `-z`, `--full-name`,
//! `--deduplicate`, `--with-tree` overlay and `--error-unmatch`.
//! Index entries are shown in index order; for each, the cached line
//! comes first, then the deleted and modified ones.
//!
//! Primary: <https://git-scm.com/docs/git-ls-files>.

use crate::cmd::attr::{AttrValue, Attributes};
use crate::cmd::ignore::Ignore;
use crate::error::{CliError, CliResult};
use crate::util::{open_repo, read_index, work_tree};
use arks_index::IndexEntry;
use arks_object::{ObjectKind, RawObject};
use arks_repo::Repository;
use std::collections::HashSet;
use std::fs;
use std::io::Write;
use std::path::Path;

#[derive(Default)]
struct Opts {
    cached: bool,
    deleted: bool,
    modified: bool,
    others: bool,
    ignored: bool,
    stage: bool,
    unmerged: bool,
    killed: bool,
    resolve_undo: bool,
    directory: bool,
    no_empty_directory: bool,
    eol: bool,
    tags: bool,
    lower_assume: bool,
    lower_fsmonitor: bool,
    nul: bool,
    full_name: bool,
    error_unmatch: bool,
    exclude_standard: bool,
    excludes: Vec<String>,
    exclude_files: Vec<String>,
    per_dir: Option<String>,
    with_tree: Option<String>,
    abbrev: Option<usize>,
    debug: bool,
    dedup: bool,
    format: Option<String>,
    specs: Vec<String>,
}

fn need(args: &[String], i: usize, name: &str) -> CliResult<String> {
    args.get(i)
        .cloned()
        .ok_or_else(|| CliError::Exit(129, format!("error: option `{name}' requires a value")))
}

fn parse(args: &[String]) -> CliResult<Opts> {
    let args = crate::util::split_short_bundles(args, "cdmoiskutvfz", "xX");
    let mut o = Opts::default();
    let mut i = 0;
    while i < args.len() {
        let a = args[i].clone();
        i += 1;
        if !a.starts_with('-') || a == "-" {
            o.specs.push(a);
            continue;
        }
        match a.as_str() {
            "--" => {
                o.specs.extend(args[i..].iter().cloned());
                break;
            }
            "-c" | "--cached" => o.cached = true,
            "--no-cached" => o.cached = false,
            "-d" | "--deleted" => o.deleted = true,
            "--no-deleted" => o.deleted = false,
            "-m" | "--modified" => o.modified = true,
            "--no-modified" => o.modified = false,
            "-o" | "--others" => o.others = true,
            "--no-others" => o.others = false,
            "-i" | "--ignored" => o.ignored = true,
            "--no-ignored" => o.ignored = false,
            "-s" | "--stage" => o.stage = true,
            "--no-stage" => o.stage = false,
            "-u" | "--unmerged" => o.unmerged = true,
            "--no-unmerged" => o.unmerged = false,
            "-k" | "--killed" => o.killed = true,
            "--no-killed" => o.killed = false,
            "--resolve-undo" => o.resolve_undo = true,
            "--no-resolve-undo" => o.resolve_undo = false,
            "--directory" => o.directory = true,
            "--no-directory" => o.directory = false,
            "--empty-directory" => o.no_empty_directory = false,
            "--no-empty-directory" => o.no_empty_directory = true,
            "--eol" => o.eol = true,
            "--no-eol" => o.eol = false,
            "-t" => o.tags = true,
            "-v" => {
                o.tags = true;
                o.lower_assume = true;
            }
            "-f" => {
                o.tags = true;
                o.lower_fsmonitor = true;
            }
            "-z" => o.nul = true,
            "--full-name" => o.full_name = true,
            "--error-unmatch" => o.error_unmatch = true,
            "--no-error-unmatch" => o.error_unmatch = false,
            "--exclude-standard" => o.exclude_standard = true,
            "--no-exclude-per-directory" => o.per_dir = None,
            "--no-with-tree" => o.with_tree = None,
            "--abbrev" => o.abbrev = Some(7),
            "--no-abbrev" => o.abbrev = None,
            "--debug" => o.debug = true,
            "--no-debug" => o.debug = false,
            "--deduplicate" => o.dedup = true,
            "--no-deduplicate" => o.dedup = false,
            "--sparse" | "--no-sparse" | "--recurse-submodules" | "--no-recurse-submodules" => {}
            "-x" | "--exclude" => {
                o.excludes.push(need(&args, i, "exclude")?);
                i += 1;
            }
            "-X" | "--exclude-from" => {
                o.exclude_files.push(need(&args, i, "exclude-from")?);
                i += 1;
            }
            "--exclude-per-directory" => {
                o.per_dir = Some(need(&args, i, "exclude-per-directory")?);
                i += 1;
            }
            "--with-tree" => {
                o.with_tree = Some(need(&args, i, "with-tree")?);
                i += 1;
            }
            "--format" => {
                o.format = Some(need(&args, i, "format")?);
                i += 1;
            }
            x => {
                if let Some(v) = x.strip_prefix("--exclude=") {
                    o.excludes.push(v.to_string());
                } else if let Some(v) = x.strip_prefix("-x").filter(|v| !v.is_empty()) {
                    o.excludes.push(v.to_string());
                } else if let Some(v) = x.strip_prefix("--exclude-from=") {
                    o.exclude_files.push(v.to_string());
                } else if let Some(v) = x.strip_prefix("--exclude-per-directory=") {
                    o.per_dir = Some(v.to_string());
                } else if let Some(v) = x.strip_prefix("--with-tree=") {
                    o.with_tree = Some(v.to_string());
                } else if let Some(v) = x.strip_prefix("--format=") {
                    o.format = Some(v.to_string());
                } else if let Some(v) = x.strip_prefix("--abbrev=") {
                    let n: usize = v.parse().map_err(|_| {
                        CliError::Exit(
                            129,
                            "error: option `abbrev' expects a numerical value".into(),
                        )
                    })?;
                    o.abbrev = Some(if n == 0 { 40 } else { n.clamp(4, 40) });
                } else {
                    return Err(CliError::Exit(
                        129,
                        format!("error: unknown option `{}'", x.trim_start_matches('-')),
                    ));
                }
            }
        }
    }
    Ok(o)
}

/// `--eol` content class: `-text` (binary), `lf`, `crlf`, `mixed`, `none`.
fn eolinfo(bytes: Option<&[u8]>) -> &'static str {
    let Some(b) = bytes else { return "" };
    if b.is_empty() {
        return "none";
    }
    let (mut lf, mut crlf, mut lonecr, mut nul) = (0usize, 0usize, 0usize, false);
    let (mut printable, mut nonprintable) = (0usize, 0usize);
    for (i, &c) in b.iter().enumerate() {
        match c {
            b'\r' => {
                if b.get(i + 1) == Some(&b'\n') {
                    crlf += 1;
                } else {
                    lonecr += 1;
                }
            }
            b'\n' => {
                if i == 0 || b[i - 1] != b'\r' {
                    lf += 1;
                }
            }
            0 => nul = true,
            127 => nonprintable += 1,
            8 | 9 | 0x1b | 0x0c => printable += 1,
            c if c < 32 => nonprintable += 1,
            _ => printable += 1,
        }
    }
    if nul || lonecr > 0 || (printable >> 7) < nonprintable {
        return "-text";
    }
    match (lf > 0, crlf > 0) {
        (true, true) => "mixed",
        (true, false) => "lf",
        (false, true) => "crlf",
        (false, false) => "none",
    }
}

/// `--eol` attribute column: the `text` and `eol` attributes.
fn eolattr(attrs: &std::collections::BTreeMap<String, AttrValue>) -> String {
    let text = match attrs.get("text") {
        Some(AttrValue::Set) => "text",
        Some(AttrValue::Unset) => "-text",
        Some(AttrValue::Value(v)) if v == "auto" => "text=auto",
        _ => "",
    };
    let eol = match attrs.get("eol") {
        Some(AttrValue::Value(v)) if v == "lf" || v == "crlf" => Some(v.as_str()),
        _ => None,
    };
    match (text, eol) {
        (t, None) => t.to_string(),
        ("", Some(e)) => format!("text eol={e}"),
        ("-text", Some(_)) => "-text".to_string(),
        (t, Some(e)) => format!("{t} eol={e}"),
    }
}

struct Ctx<'a> {
    o: &'a Opts,
    repo: &'a Repository,
    work: &'a Path,
    prefix: String,
    term: &'static str,
}

impl Ctx<'_> {
    fn show(&self, p: &str) -> String {
        let shown = if self.o.full_name || self.prefix.is_empty() {
            p.to_string()
        } else {
            let pre: Vec<&str> = self.prefix.split('/').collect();
            let comps: Vec<&str> = p.split('/').collect();
            let mut common = 0;
            while common < pre.len() && common + 1 < comps.len() && pre[common] == comps[common] {
                common += 1;
            }
            format!(
                "{}{}",
                "../".repeat(pre.len() - common),
                comps[common..].join("/")
            )
        };
        if self.o.nul {
            shown
        } else {
            crate::cmd::diffcore::emit::quote_path(&shown)
        }
    }

    fn attrs(&self, path: &str) -> std::collections::BTreeMap<String, AttrValue> {
        Attributes::load(self.work, self.repo.git_dir(), path).for_path(path)
    }

    fn eol_column(&self, path: &str, index: Option<&IndexEntry>) -> CliResult<String> {
        let ib = match index {
            Some(e) if e.mode & 0o170000 == 0o100000 => Some(self.repo.get_object(&e.oid)?.payload),
            _ => None,
        };
        let abs = self.work.join(path);
        let wb = if abs.is_file() && !abs.is_symlink() {
            fs::read(&abs).ok()
        } else {
            None
        };
        Ok(format!(
            "i/{:<5} w/{:<5} attr/{:<17}\t",
            eolinfo(ib.as_deref()),
            eolinfo(wb.as_deref()),
            eolattr(&self.attrs(path))
        ))
    }

    fn hex(&self, e: &IndexEntry) -> String {
        match self.o.abbrev {
            Some(n) if n < 40 => crate::cmd::shared::abbrev_hex(self.repo, &e.oid, n),
            _ => e.oid.to_hex(),
        }
    }

    fn format(&self, f: &str, e: &IndexEntry) -> CliResult<String> {
        let mut out = String::new();
        let mut rest = f;
        while let Some(i) = rest.find('%') {
            out.push_str(&rest[..i]);
            let tail = &rest[i..];
            if let Some(body) = tail.strip_prefix("%(") {
                if let Some(close) = body.find(')') {
                    let atom = &body[..close];
                    let size =
                        || -> CliResult<usize> { Ok(self.repo.get_object(&e.oid)?.payload.len()) };
                    let v = match atom {
                        "objectmode" => format!("{:06o}", e.mode),
                        "objectname" => self.hex(e),
                        "objecttype" => match e.mode & 0o170000 {
                            0o160000 => "commit".into(),
                            0o040000 => "tree".into(),
                            _ => "blob".into(),
                        },
                        "objectsize" => size()?.to_string(),
                        "objectsize:padded" => format!("{:>7}", size()?),
                        "stage" => e.stage().to_string(),
                        "path" => self.show(&e.path),
                        "eolinfo:index" => {
                            let b = self.repo.get_object(&e.oid)?.payload;
                            eolinfo(Some(&b)).to_string()
                        }
                        "eolinfo:worktree" => {
                            let abs = self.work.join(&e.path);
                            eolinfo(fs::read(&abs).ok().as_deref()).to_string()
                        }
                        "eolattr" => eolattr(&self.attrs(&e.path)),
                        _ => {
                            return Err(CliError::fatal(format!("bad ls-files format: %({atom})")));
                        }
                    };
                    out.push_str(&v);
                    rest = &body[close + 1..];
                    continue;
                }
            }
            if let Some(t) = tail.strip_prefix("%%") {
                out.push('%');
                rest = t;
            } else if let Some(t) = tail.strip_prefix("%n") {
                out.push('\n');
                rest = t;
            } else if tail.len() >= 4
                && tail.as_bytes()[1] == b'x'
                && tail[2..4].bytes().all(|b| b.is_ascii_hexdigit())
            {
                let v = u8::from_str_radix(&tail[2..4], 16).unwrap_or(b'%');
                out.push(v as char);
                rest = &tail[4..];
            } else {
                out.push('%');
                rest = &tail[1..];
            }
        }
        out.push_str(rest);
        Ok(out)
    }

    /// One index-entry line: tag, `--eol` column, then the name or stage.
    fn entry_line(&self, out: &mut impl Write, tag: &str, e: &IndexEntry) -> CliResult<()> {
        if let Some(f) = &self.o.format {
            write!(out, "{}{}", self.format(f, e)?, self.term)?;
            return Ok(());
        }
        if self.o.tags {
            write!(out, "{tag} ")?;
        }
        if self.o.eol {
            write!(out, "{}", self.eol_column(&e.path, Some(e))?)?;
        }
        if self.o.stage {
            write!(
                out,
                "{:06o} {} {}\t{}{}",
                e.mode,
                self.hex(e),
                e.stage(),
                self.show(&e.path),
                self.term
            )?;
        } else {
            write!(out, "{}{}", self.show(&e.path), self.term)?;
        }
        if self.o.debug {
            write!(
                out,
                "  ctime: {}:{}\n  mtime: {}:{}\n  dev: {}\tino: {}\n  uid: {}\tgid: {}\n  size: {}\tflags: {:x}\n",
                e.ctime_sec,
                e.ctime_nsec,
                e.mtime_sec,
                e.mtime_nsec,
                e.dev,
                e.ino,
                e.uid,
                e.gid,
                e.size,
                // In-memory flags: stage / assume-valid bits and the
                // extended flags, without the name length.
                (u32::from(e.flags) & 0xf000) | (u32::from(e.flags_extended.unwrap_or(0)) << 16)
            )?;
        }
        Ok(())
    }

    fn path_line(&self, out: &mut impl Write, tag: &str, path: &str) -> CliResult<()> {
        if self.o.tags {
            write!(out, "{tag} ")?;
        }
        if self.o.eol {
            write!(
                out,
                "{}",
                self.eol_column(path.trim_end_matches('/'), None)?
            )?;
        }
        write!(out, "{}{}", self.show(path), self.term)?;
        Ok(())
    }
}

/// `--directory` listing: wholly untracked directories as `dir/`.
#[allow(clippy::too_many_arguments)]
fn others_dirs(
    work: &Path,
    rel: &str,
    tracked_dirs: &HashSet<String>,
    indexed: &HashSet<String>,
    ig: &mut Option<Ignore>,
    want_ignored: bool,
    no_empty: bool,
    out: &mut Vec<String>,
) -> CliResult<()> {
    let dir = if rel.is_empty() {
        work.to_path_buf()
    } else {
        work.join(rel)
    };
    let mut ents: Vec<_> = fs::read_dir(&dir)?.flatten().collect();
    ents.sort_by_key(|e| e.file_name());
    for e in ents {
        let name = e.file_name().to_string_lossy().into_owned();
        if name == ".git" {
            continue;
        }
        let r = if rel.is_empty() {
            name
        } else {
            format!("{rel}/{name}")
        };
        let is_dir = e.path().is_dir() && !e.path().is_symlink();
        let excluded = ig.as_mut().is_some_and(|i| i.is_ignored(&r, is_dir));
        if is_dir {
            if tracked_dirs.contains(&r) {
                others_dirs(
                    work,
                    &r,
                    tracked_dirs,
                    indexed,
                    ig,
                    want_ignored,
                    no_empty,
                    out,
                )?;
                continue;
            }
            if excluded != want_ignored && !(want_ignored && !excluded) {
                continue;
            }
            if want_ignored && !excluded {
                others_dirs(
                    work,
                    &r,
                    tracked_dirs,
                    indexed,
                    ig,
                    want_ignored,
                    no_empty,
                    out,
                )?;
                continue;
            }
            let has_file = has_any_file(&e.path());
            if !has_file && no_empty {
                continue;
            }
            out.push(format!("{r}/"));
        } else if !indexed.contains(&r) && excluded == want_ignored {
            out.push(r);
        }
    }
    Ok(())
}

fn has_any_file(dir: &Path) -> bool {
    fs::read_dir(dir).into_iter().flatten().flatten().any(|e| {
        let p = e.path();
        if p.is_dir() { has_any_file(&p) } else { true }
    })
}

pub fn cmd_ls_files(args: &[String]) -> CliResult<()> {
    // Cite: https://git-scm.com/docs/git-ls-files
    let mut o = parse(args)?;
    if o.format.is_some()
        && (o.stage || o.others || o.killed || o.tags || o.resolve_undo || o.dedup || o.eol)
    {
        return Err(CliError::Exit(
            129,
            "fatal: ls-files --format cannot be used with -s, -o, -k, -t, --resolve-undo, --deduplicate, --eol".into(),
        ));
    }
    let any_mode =
        o.stage || o.deleted || o.others || o.unmerged || o.killed || o.modified || o.resolve_undo;
    if !any_mode {
        o.cached = true;
    }
    if o.unmerged {
        o.stage = true;
    }
    let has_excludes = o.exclude_standard
        || !o.excludes.is_empty()
        || !o.exclude_files.is_empty()
        || o.per_dir.is_some();
    if o.ignored && !(o.others || o.cached) {
        return Err(CliError::fatal(
            "ls-files -i must be used with either -o or -c",
        ));
    }
    if o.ignored && !has_excludes {
        return Err(CliError::fatal(
            "ls-files --ignored needs some exclude pattern",
        ));
    }
    let repo = open_repo()?;
    let work = work_tree(&repo)?;
    let mut index = read_index(&repo)?;
    if let Some(t) = &o.with_tree {
        // Paths of the tree missing from the index count as present.
        let oid = crate::cmd::shared::resolve_rev(&repo, t)
            .map_err(|_| CliError::fatal(format!("tree-ish {t} not found.")))?;
        let tree =
            crate::cmd::shared::peel_to_kind(&repo, &oid, crate::cmd::shared::ObjKind::Tree, t)?;
        let have: HashSet<String> = index.entries.iter().map(|e| e.path.clone()).collect();
        for (path, (mode, oid)) in crate::cmd::diffcore::flat_tree(&repo, &tree)? {
            if !have.contains(&path) {
                index.entries.push(IndexEntry {
                    ctime_sec: 0,
                    ctime_nsec: 0,
                    mtime_sec: 0,
                    mtime_nsec: 0,
                    dev: 0,
                    ino: 0,
                    mode,
                    uid: 0,
                    gid: 0,
                    size: 0,
                    oid,
                    flags: 1 << 12,
                    flags_extended: None,
                    path,
                });
            }
        }
        index.entries.sort_by(arks_index::cmp_entries);
    }
    let canon = |p: &Path| p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
    let prefix = canon(&crate::util::cwd()?)
        .strip_prefix(canon(&work))
        .map(|r| r.to_string_lossy().replace('\\', "/"))
        .unwrap_or_default();
    let mut specs: Vec<String> = Vec::new();
    for sp in &o.specs {
        specs.push(crate::cmd::core::pathspec_rel(&work, sp)?);
    }
    if specs.is_empty() && !prefix.is_empty() {
        specs.push(prefix.clone());
    }
    let mut matched = vec![false; specs.len()];
    let mut in_spec = |p: &str| -> bool {
        if specs.is_empty() {
            return true;
        }
        let p = p.trim_end_matches('/');
        let mut any = false;
        for (i, s) in specs.iter().enumerate() {
            if s.is_empty() || p == s || p.starts_with(&format!("{s}/")) {
                matched[i] = true;
                any = true;
            }
        }
        any
    };
    let mut ignore: Option<Ignore> = if has_excludes {
        let mut ig = if o.exclude_standard {
            Ignore::new(&work)
        } else {
            Ignore::explicit(&work)
        };
        if let Some(name) = &o.per_dir {
            ig.per_directory(name);
        }
        for x in &o.excludes {
            ig.add_patterns(x);
        }
        for f in &o.exclude_files {
            let text = fs::read_to_string(f)
                .map_err(|e| CliError::fatal(format!("cannot use {f} as an exclude file: {e}")))?;
            ig.add_patterns(&text);
        }
        Some(ig)
    } else {
        None
    };
    let ctx = Ctx {
        o: &o,
        repo: &repo,
        work: &work,
        prefix,
        term: if o.nul { "\0" } else { "\n" },
    };
    let mut out = std::io::BufWriter::new(std::io::stdout().lock());
    let indexed: HashSet<String> = index.entries.iter().map(|e| e.path.clone()).collect();
    if o.others || o.killed {
        let mut tracked_dirs = HashSet::new();
        for p in &indexed {
            let mut d = p.as_str();
            while let Some(i) = d.rfind('/') {
                d = &d[..i];
                tracked_dirs.insert(d.to_string());
            }
        }
        let mut list = Vec::new();
        if o.others && o.directory {
            others_dirs(
                &work,
                "",
                &tracked_dirs,
                &indexed,
                &mut ignore,
                o.ignored,
                o.no_empty_directory,
                &mut list,
            )?;
        } else {
            let mut all = Vec::new();
            crate::cmd::shared::collect_untracked_with(&work, &work, &indexed, &mut all, None)?;
            for p in all {
                let excluded = ignore.as_mut().is_some_and(|ig| {
                    ig.matching_with_parents(&p, false)
                        .is_some_and(|m| m.excluded)
                });
                if excluded == o.ignored {
                    list.push(p);
                }
            }
        }
        if o.killed {
            // Untracked paths in the way of an index entry (a file where a
            // tracked directory goes, or a directory for a tracked file).
            let mut killed = Vec::new();
            for e in &index.entries {
                let parts: Vec<&str> = e.path.split('/').collect();
                for k in 1..parts.len() {
                    let d = parts[..k].join("/");
                    let abs = work.join(&d);
                    if abs.is_file() && !indexed.contains(&d) {
                        killed.push(d);
                    }
                }
                let abs = work.join(&e.path);
                if abs.is_dir() && !abs.is_symlink() {
                    let mut under = Vec::new();
                    crate::cmd::shared::collect_untracked_with(
                        &work, &abs, &indexed, &mut under, None,
                    )?;
                    killed.extend(under);
                }
            }
            killed.sort();
            killed.dedup();
            for k in killed {
                if in_spec(&k) {
                    ctx.path_line(&mut out, "K", &k)?;
                }
            }
        }
        if o.others {
            for p in list {
                if in_spec(&p) {
                    ctx.path_line(&mut out, "?", &p)?;
                }
            }
        }
    }
    let mut ig_cached = ignore;
    let mut last_shown: Option<String> = None;
    let show_name = |path: &str, last: &mut Option<String>| -> bool {
        if o.dedup && !o.stage {
            if last.as_deref() == Some(path) {
                return false;
            }
            *last = Some(path.to_string());
        }
        true
    };
    if o.cached || o.stage || o.deleted || o.modified {
        for e in &index.entries {
            if !in_spec(&e.path) {
                continue;
            }
            if o.ignored
                && !ig_cached.as_mut().is_some_and(|ig| {
                    ig.matching_with_parents(&e.path, false)
                        .is_some_and(|m| m.excluded)
                })
            {
                continue;
            }
            let unmerged = e.stage() > 0;
            let skip_worktree = e.flags_extended.is_some_and(|f| f & 0x4000 != 0);
            let assume = e.flags & 0x8000 != 0;
            if (o.cached || o.stage) && !(o.unmerged && !unmerged && !o.cached) {
                let mut tag = if unmerged {
                    "M"
                } else if skip_worktree {
                    "S"
                } else {
                    "H"
                }
                .to_string();
                if o.lower_assume && assume {
                    tag = tag.to_lowercase();
                }
                if show_name(&e.path, &mut last_shown) {
                    ctx.entry_line(&mut out, &tag, e)?;
                }
            }
            if !(o.deleted || o.modified) || skip_worktree {
                continue;
            }
            let abs = work.join(&e.path);
            let meta = fs::symlink_metadata(&abs).ok();
            let gone = meta
                .as_ref()
                .is_none_or(|m| m.is_dir() && e.mode != 0o160000);
            if o.deleted && gone && show_name(&e.path, &mut last_shown) {
                ctx.entry_line(&mut out, "R", e)?;
            }
            if o.modified {
                let changed = gone
                    || unmerged
                    || match &meta {
                        Some(m) if m.is_file() => {
                            !crate::cmd::shared::index_stat_matches(e, m)
                                && RawObject::new(ObjectKind::Blob, fs::read(&abs)?)
                                    .oid(repo.object_format())?
                                    != e.oid
                        }
                        _ => false,
                    };
                if changed && show_name(&e.path, &mut last_shown) {
                    ctx.entry_line(&mut out, "C", e)?;
                }
            }
        }
    }
    out.flush()?;
    if o.error_unmatch {
        if let Some(i) = matched.iter().position(|m| !m) {
            return Err(CliError::Exit(
                1,
                format!(
                    "error: pathspec '{}' did not match any file(s) known to git\n\
                     Did you forget to 'git add'?",
                    o.specs.get(i).cloned().unwrap_or_default()
                ),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eol_classes() {
        assert_eq!(eolinfo(None), "");
        assert_eq!(eolinfo(Some(b"")), "none");
        assert_eq!(eolinfo(Some(b"a\nb\n")), "lf");
        assert_eq!(eolinfo(Some(b"a\r\nb\r\n")), "crlf");
        assert_eq!(eolinfo(Some(b"a\r\nb\n")), "mixed");
        assert_eq!(eolinfo(Some(b"ab")), "none");
        assert_eq!(eolinfo(Some(b"a\0b")), "-text");
    }
}
