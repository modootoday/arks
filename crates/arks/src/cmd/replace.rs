//! `replace` — create, edit, graft, list and delete `refs/replace/*`.
//!
//! Modes and messages follow git-replace(1) and observed stock git output:
//! `<object> <replacement>`, `-e/--edit`, `-g/--graft`, `--convert-graft-file`,
//! `-d/--delete`, `-l/--list [<pattern>]` with `--format=short|medium|long`,
//! `-f/--force` and `--raw`.
//! Cite: <https://git-scm.com/docs/git-replace>

use crate::cmd::opt_parse::{Mode, OptDef, Takes::*, o, parse};
use crate::cmd::shared::resolve_rev;
use crate::error::{CliError, CliResult};
use crate::util::open_repo;
use arks_object::{ObjectKind, RawObject, Tree, TreeEntry, TreeMode};
use arks_oid::Oid;
use arks_repo::Repository;
use arks_store::ObjectStore;
use std::fs;

const USAGE: &str = "usage: git replace [-f] <object> <replacement>
   or: git replace [-f] --edit <object>
   or: git replace [-f] --graft <commit> [<parent>...]
   or: git replace [-f] --convert-graft-file
   or: git replace -d <object>...
   or: git replace [--format=<format>] [-l [<pattern>]]

    -l, --list            list replace refs
    -d, --delete          delete replace refs
    -e, --edit            edit existing object
    -g, --graft           change a commit's parents
    --convert-graft-file  convert existing graft file
    -f, --[no-]force      replace the ref if it exists
    --[no-]raw            do not pretty-print contents for --edit
    --[no-]format <format>
                          use this format
";

const DEFS: &[OptDef] = &[
    o("list", 'l', No, false),
    o("delete", 'd', No, false),
    o("edit", 'e', No, false),
    o("graft", 'g', No, false),
    o("convert-graft-file", ' ', No, false),
    o("force", 'f', No, true),
    o("raw", ' ', No, true),
    o("format", ' ', Req, true),
];

fn usage_fatal(msg: &str) -> CliError {
    CliError::Exit(129, format!("fatal: {msg}\n\n{USAGE}"))
}

/// git's builtin returning -1 exits with status 255.
fn fail(msg: impl Into<String>) -> CliError {
    CliError::Exit(255, msg.into())
}

fn mode_label(key: &str) -> &'static str {
    match key {
        "list" => "-l",
        "delete" => "-d",
        "edit" => "-e",
        "graft" => "-g",
        _ => "--convert-graft-file",
    }
}

pub fn cmd_replace(args: &[String]) -> CliResult<()> {
    let p = parse("replace", args, DEFS, Mode::default())?;
    // OPT_CMDMODE: a second, different mode is refused.
    let mut cmdmode: Option<&'static str> = None;
    for (key, _, _) in p.opts() {
        if matches!(
            key,
            "list" | "delete" | "edit" | "graft" | "convert-graft-file"
        ) {
            match cmdmode {
                Some(prev) if prev != key => {
                    return Err(CliError::Exit(
                        129,
                        format!(
                            "error: {} is incompatible with {}",
                            mode_label(key),
                            mode_label(prev)
                        ),
                    ));
                }
                _ => cmdmode = Some(key),
            }
        }
    }
    let force = p.on("force");
    let raw = p.on("raw");
    let format = p.last("format").and_then(|v| v);
    let argv = p.args();
    let mode = cmdmode.unwrap_or(if argv.is_empty() { "list" } else { "replace" });
    if format.is_some() && mode != "list" {
        return Err(usage_fatal("--format cannot be used when not listing"));
    }
    if force && matches!(mode, "list" | "delete") {
        return Err(usage_fatal(
            "-f only makes sense when writing a replacement",
        ));
    }
    if raw && mode != "edit" {
        return Err(usage_fatal("--raw only makes sense with --edit"));
    }
    match mode {
        "delete" => {
            if argv.is_empty() {
                return Err(usage_fatal("-d needs at least one argument"));
            }
            delete_replace_refs(&argv)
        }
        "replace" => {
            if argv.len() != 2 {
                return Err(usage_fatal("bad number of arguments"));
            }
            let mut repo = open_repo()?;
            replace_object(&mut repo, &argv[0], &argv[1], force)
        }
        "edit" => {
            if argv.len() != 1 {
                return Err(usage_fatal("-e needs exactly one argument"));
            }
            let mut repo = open_repo()?;
            edit_object(&mut repo, &argv[0], force, raw)
        }
        "graft" => {
            if argv.is_empty() {
                return Err(usage_fatal("-g needs at least one argument"));
            }
            let mut repo = open_repo()?;
            create_graft(&mut repo, &argv, force, false)
        }
        "convert-graft-file" => {
            if !argv.is_empty() {
                return Err(usage_fatal("--convert-graft-file takes no argument"));
            }
            convert_graft_file(force)
        }
        _ => {
            if argv.len() > 1 {
                return Err(usage_fatal("only one pattern can be given with -l"));
            }
            list_replace_refs(argv.first().map(String::as_str), format.as_deref())
        }
    }
}

fn kind_name(repo: &Repository, oid: &Oid) -> String {
    repo.get_object(oid)
        .map(|o| o.kind.as_str().to_string())
        .unwrap_or_else(|_| "unknown".into())
}

fn list_replace_refs(pattern: Option<&str>, format: Option<&str>) -> CliResult<()> {
    let fmt = format.unwrap_or("short");
    if !matches!(fmt, "short" | "medium" | "long") {
        return Err(fail(format!(
            "error: invalid replace format '{fmt}'\nvalid formats are 'short', 'medium' and 'long'"
        )));
    }
    let repo = open_repo()?;
    let mut refs = repo.refs().list_refs("refs/replace/")?;
    refs.sort_by(|a, b| a.0.cmp(&b.0));
    let pat = pattern.unwrap_or("*");
    for (name, oid) in refs {
        let short = name.strip_prefix("refs/replace/").unwrap_or(&name);
        if !crate::cmd::ignore::wildmatch(pat.as_bytes(), short.as_bytes()) {
            continue;
        }
        match fmt {
            "short" => println!("{short}"),
            "medium" => println!("{short} -> {}", oid.to_hex()),
            _ => {
                let obj_type = Oid::from_hex(short)
                    .map(|o| kind_name(&repo, &o))
                    .unwrap_or_else(|_| "unknown".into());
                println!(
                    "{short} ({obj_type}) -> {} ({})",
                    oid.to_hex(),
                    kind_name(&repo, &oid)
                );
            }
        }
    }
    Ok(())
}

fn delete_replace_refs(names: &[String]) -> CliResult<()> {
    let mut repo = open_repo()?;
    let mut had_error = false;
    for n in names {
        let Ok(oid) = resolve_rev(&repo, n) else {
            eprintln!("error: failed to resolve '{n}' as a valid ref");
            had_error = true;
            continue;
        };
        let hex = oid.to_hex();
        let refname = format!("refs/replace/{hex}");
        if repo.refs().resolve(&refname).is_err() {
            eprintln!("error: replace ref '{hex}' not found");
            had_error = true;
            continue;
        }
        repo.refs_mut().delete(&refname, None)?;
        println!("Deleted replace ref '{hex}'");
    }
    if had_error {
        return Err(CliError::silent(1));
    }
    Ok(())
}

/// `refs/replace/<hex>` for `object`, refusing an existing ref without `-f`.
fn check_ref_valid(repo: &Repository, object: &Oid, force: bool) -> CliResult<String> {
    // CORR-003: refuse while another writer holds refs/replace.lock.
    for lock in [
        repo.common_dir().join("refs/replace.lock"),
        repo.git_dir().join("refs/replace.lock"),
    ] {
        if lock.is_file() {
            return Err(CliError::Msg(format!(
                "{} (CORR-003 — concurrent replace-refs refuse; lease ≠ skip)",
                lock.display()
            )));
        }
    }
    let refname = format!("refs/replace/{}", object.to_hex());
    if !force && repo.refs().resolve(&refname).is_ok() {
        return Err(fail(format!(
            "error: replace ref '{refname}' already exists"
        )));
    }
    Ok(refname)
}

fn replace_object(repo: &mut Repository, obj: &str, repl: &str, force: bool) -> CliResult<()> {
    let object = resolve_rev(repo, obj)
        .map_err(|_| fail(format!("error: failed to resolve '{obj}' as a valid ref")))?;
    let refname = check_ref_valid(repo, &object, force)?;
    let replacement = resolve_rev(repo, repl)
        .map_err(|_| fail(format!("error: failed to resolve '{repl}' as a valid ref")))?;
    let (ot, rt) = (kind_name(repo, &object), kind_name(repo, &replacement));
    if !force && ot != rt {
        return Err(fail(format!(
            "error: Objects must be of the same type.\n'{obj}' points to a replaced object of type '{ot}'\nwhile '{repl}' points to a replacement object of type '{rt}'."
        )));
    }
    repo.refs_mut().update(&refname, &replacement, None)?;
    Ok(())
}

/// The content git would see for `oid`: its current replacement when one exists.
fn replaced_object(repo: &Repository, oid: &Oid) -> CliResult<RawObject> {
    let refname = format!("refs/replace/{}", oid.to_hex());
    if let Ok(repl) = repo.refs().resolve(&refname) {
        if let Ok(o) = repo.get_object(&repl) {
            return Ok(o);
        }
    }
    Ok(repo.get_object(oid)?)
}

fn launch_editor(repo: &Repository, path: &std::path::Path) -> CliResult<()> {
    let cfg = arks_config::ConfigSet::load_discovered(repo.git_dir());
    crate::cmd::bridge::proc::edit(&crate::cmd::bridge::proc::editor(&cfg), path)
}

fn pretty_tree(repo: &Repository, payload: &[u8]) -> CliResult<Vec<u8>> {
    let tree =
        Tree::decode(payload, repo.oid_kind()).map_err(|e| CliError::fatal(e.to_string()))?;
    let mut out = String::new();
    for e in &tree.entries {
        let (mode, kind) = match e.mode {
            TreeMode::Tree => ("040000", "tree"),
            TreeMode::Gitlink => ("160000", "commit"),
            m => (m.as_str(), "blob"),
        };
        out.push_str(&format!("{mode} {kind} {}\t{}\n", e.oid.to_hex(), e.name));
    }
    Ok(out.into_bytes())
}

fn parse_pretty_tree(text: &[u8]) -> CliResult<Vec<u8>> {
    let mut entries = Vec::new();
    for line in String::from_utf8_lossy(text).lines() {
        if line.is_empty() {
            continue;
        }
        let bad = || CliError::fatal(format!("input format error: {line}"));
        let (meta, name) = line.split_once('\t').ok_or_else(bad)?;
        let parts: Vec<&str> = meta.split(' ').collect();
        if parts.len() != 3 {
            return Err(bad());
        }
        let mode = TreeMode::parse(parts[0]).map_err(|_| bad())?;
        let oid = Oid::from_hex(parts[2]).map_err(|_| bad())?;
        entries.push(TreeEntry {
            mode,
            name: name.to_string(),
            oid,
        });
    }
    let tree = Tree::new(entries).map_err(|e| CliError::fatal(e.to_string()))?;
    tree.encode().map_err(|e| CliError::fatal(e.to_string()))
}

/// The subset of git's fsck header checks that `hash-object` applies to an
/// edited commit or tag before writing it.
fn fsck_header(kind: ObjectKind, data: &[u8]) -> Option<String> {
    if !matches!(kind, ObjectKind::Commit | ObjectKind::Tag) {
        return None;
    }
    // A header ends at the first blank line, or at a final newline when the
    // object has no body; anything else is unterminated.
    let end = match data.windows(2).position(|w| w == b"\n\n") {
        Some(p) => p + 1,
        None if data.ends_with(b"\n") => data.len(),
        None => return Some("unterminatedHeader: unterminated header".into()),
    };
    let header = String::from_utf8_lossy(&data[..end]).into_owned();
    let mut lines = header.lines().peekable();
    let expect = |line: Option<&str>, key: &str, id: &str| -> Option<String> {
        match line {
            Some(l) if l.starts_with(&format!("{key} ")) => None,
            _ => Some(format!("{id}: invalid format - expected '{key}' line")),
        }
    };
    if kind == ObjectKind::Commit {
        if let Some(e) = expect(lines.next(), "tree", "missingTree") {
            return Some(e);
        }
        while lines.peek().is_some_and(|l| l.starts_with("parent ")) {
            lines.next();
        }
        if let Some(e) = expect(lines.next(), "author", "missingAuthor") {
            return Some(e);
        }
        return expect(lines.next(), "committer", "missingCommitter");
    }
    if let Some(e) = expect(lines.next(), "object", "missingObject") {
        return Some(e);
    }
    if let Some(e) = expect(lines.next(), "type", "missingTypeEntry") {
        return Some(e);
    }
    expect(lines.next(), "tag", "missingTagEntry")
}

fn edit_object(repo: &mut Repository, name: &str, force: bool, raw: bool) -> CliResult<()> {
    let old = resolve_rev(repo, name)
        .map_err(|_| fail(format!("error: not a valid object name: '{name}'")))?;
    let refname = check_ref_valid(repo, &old, force)?;
    let obj = replaced_object(repo, &old)?;
    let kind = repo.get_object(&old)?.kind;
    let tmp = repo.git_dir().join("EDIT_OBJECT");
    let shown = if !raw && kind == ObjectKind::Tree {
        pretty_tree(repo, &obj.payload)?
    } else {
        obj.payload.clone()
    };
    fs::write(&tmp, &shown)?;
    let edited = launch_editor(repo, &tmp).and_then(|()| Ok(fs::read(&tmp)?));
    let _ = fs::remove_file(&tmp);
    let edited = edited?;
    let payload = if !raw && kind == ObjectKind::Tree {
        parse_pretty_tree(&edited)?
    } else {
        if let Some(e) = fsck_header(kind, &edited) {
            return Err(CliError::Exit(
                128,
                format!(
                    "error: object fails fsck: {e}\nfatal: refusing to create malformed object"
                ),
            ));
        }
        edited
    };
    let format = repo.object_format();
    let new_obj = RawObject::new(kind, payload);
    let new = new_obj.oid(format)?;
    if new == old {
        return Err(fail(format!(
            "error: new object is the same as the old one: '{}'",
            old.to_hex()
        )));
    }
    repo.store_mut().put(format, &new_obj)?;
    repo.refs_mut().update(&refname, &new, None)?;
    Ok(())
}

/// Rewrite the parent lines of a commit buffer (and drop a `gpgsig` header).
fn regraft(payload: &[u8], parents: &[Oid]) -> Vec<u8> {
    let end = payload
        .windows(2)
        .position(|w| w == b"\n\n")
        .map(|p| p + 1)
        .unwrap_or(payload.len());
    let header = String::from_utf8_lossy(&payload[..end]).into_owned();
    let mut out = String::new();
    let mut in_sig = false;
    let mut parents_done = false;
    for line in header.split_inclusive('\n') {
        if in_sig {
            if line.starts_with(' ') {
                continue;
            }
            in_sig = false;
        }
        if line.starts_with("gpgsig ") || line.starts_with("gpgsig-sha256 ") {
            in_sig = true;
            continue;
        }
        if line.starts_with("parent ") {
            continue;
        }
        out.push_str(line);
        if line.starts_with("tree ") && !parents_done {
            for p in parents {
                out.push_str(&format!("parent {}\n", p.to_hex()));
            }
            parents_done = true;
        }
    }
    let mut bytes = out.into_bytes();
    bytes.extend_from_slice(&payload[end..]);
    bytes
}

fn peel_commit(repo: &Repository, name: &str, oid: &Oid) -> Result<Oid, String> {
    let mut cur = *oid;
    loop {
        let obj = repo.get_object(&cur).map_err(|e| e.to_string())?;
        match obj.kind {
            ObjectKind::Commit => return Ok(cur),
            ObjectKind::Tag => {
                cur = crate::cmd::shared::tag_target(&obj.payload)
                    .ok_or_else(|| format!("could not parse {name}"))?;
            }
            k => {
                return Err(format!(
                    "error: object {} is a {}, not a commit",
                    cur.to_hex(),
                    k.as_str()
                ));
            }
        }
    }
}

fn create_graft(
    repo: &mut Repository,
    argv: &[String],
    force: bool,
    gentle: bool,
) -> CliResult<()> {
    let old_ref = &argv[0];
    let old = resolve_rev(repo, old_ref)
        .map_err(|_| fail(format!("error: not a valid object name: '{old_ref}'")))?;
    let commit = peel_commit(repo, old_ref, &old)
        .map_err(|e| fail(format!("{e}\nerror: could not parse {old_ref}")))?;
    let mut parents = Vec::new();
    for p in &argv[1..] {
        let oid = resolve_rev(repo, p)
            .map_err(|_| fail(format!("error: not a valid object name: '{p}'")))?;
        let c = peel_commit(repo, p, &oid)
            .map_err(|e| fail(format!("{e}\nerror: could not parse {p} as a commit")))?;
        parents.push(c);
    }
    let obj = replaced_object(repo, &commit)?;
    if obj.payload.starts_with(b"gpgsig") || obj.payload.windows(8).any(|w| w == b"\ngpgsig ") {
        eprintln!(
            "warning: the original commit '{}' has a gpg signature\nwarning: the signature will be removed in the replacement commit!",
            old_ref
        );
    }
    let payload = regraft(&obj.payload, &parents);
    let format = repo.object_format();
    let new_obj = RawObject::new(ObjectKind::Commit, payload);
    let new = new_obj.oid(format)?;
    if new == commit {
        if gentle {
            eprintln!("warning: graft for '{}' unnecessary", commit.to_hex());
            return Ok(());
        }
        return Err(fail(format!(
            "error: new commit is the same as the old one: '{}'",
            commit.to_hex()
        )));
    }
    repo.store_mut().put(format, &new_obj)?;
    let refname = check_ref_valid(repo, &commit, force)?;
    repo.refs_mut().update(&refname, &new, None)?;
    Ok(())
}

fn convert_graft_file(force: bool) -> CliResult<()> {
    let mut repo = open_repo()?;
    let path = repo.common_dir().join("info/grafts");
    let Ok(text) = fs::read_to_string(&path) else {
        return Err(CliError::silent(1));
    };
    let mut failed = String::new();
    for line in text.lines() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let argv: Vec<String> = line.split_whitespace().map(str::to_string).collect();
        if argv.is_empty() {
            continue;
        }
        if let Err(e) = create_graft(&mut repo, &argv, force, true) {
            let m = e.to_string();
            if !m.is_empty() {
                eprintln!("{m}");
            }
            failed.push_str("\n\t");
            failed.push_str(line);
        }
    }
    if !failed.is_empty() {
        return Err(CliError::Exit(
            1,
            format!("warning: could not convert the following graft(s):\n{failed}"),
        ));
    }
    let _ = fs::remove_file(&path);
    Ok(())
}
