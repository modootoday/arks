//! `git notes`: every subcommand and option of `git notes -h`.
//!
//! Notes live in a commit history under the notes ref (`refs/notes/commits`
//! unless `--ref`, `GIT_NOTES_REF` or `core.notesRef` name another): each
//! change writes a tree mapping object ids to note blobs (fan-out trees are
//! read, flat trees are written) and a commit `Notes added by 'git notes
//! <subcommand>'` on top of the previous one, as git does.
//! Cite: <https://git-scm.com/docs/git-notes> (behaviour, not code).

use crate::cmd::opt_parse::{Item, Mode, Parsed, parse};
use crate::cmd::porcelain_opts as t;
use crate::cmd::shared::{commit_signature, committer_signature, resolve_rev};
use crate::error::{CliError, CliResult};
use crate::util::open_repo;
use arks_object::{Commit, ObjectKind, RawObject, Tree, TreeEntry, TreeMode};
use arks_oid::Oid;
use arks_repo::Repository;
use arks_store::ObjectStore;
use std::collections::BTreeMap;

const USAGE: &str = "usage: git notes [--ref <notes-ref>] [list [<object>]]
   or: git notes [--ref <notes-ref>] add [-f] [--allow-empty] [--[no-]separator|--separator=<paragraph-break>] [--[no-]stripspace] [-m <msg> | -F <file> | (-c | -C) <object>] [<object>]
   or: git notes [--ref <notes-ref>] copy [-f] <from-object> <to-object>
   or: git notes [--ref <notes-ref>] append [--allow-empty] [--[no-]separator|--separator=<paragraph-break>] [--[no-]stripspace] [-m <msg> | -F <file> | (-c | -C) <object>] [<object>]
   or: git notes [--ref <notes-ref>] edit [--allow-empty] [<object>]
   or: git notes [--ref <notes-ref>] show [<object>]
   or: git notes [--ref <notes-ref>] merge [-v | -q] [-s <strategy>] <notes-ref>
   or: git notes merge --commit [-v | -q]
   or: git notes merge --abort [-v | -q]
   or: git notes [--ref <notes-ref>] remove [<object>...]
   or: git notes [--ref <notes-ref>] prune [-n] [-v]
   or: git notes [--ref <notes-ref>] get-ref";

fn full_ref(name: &str) -> String {
    if name.starts_with("refs/notes/") {
        name.to_string()
    } else if let Some(rest) = name.strip_prefix("notes/") {
        format!("refs/notes/{rest}")
    } else if name.starts_with("refs/") {
        name.to_string()
    } else {
        format!("refs/notes/{name}")
    }
}

pub fn cmd_notes(args: &[String]) -> CliResult<()> {
    let top = parse(
        "notes",
        args,
        t::NOTES,
        Mode {
            stop_at_non_option: true,
            ..Mode::default()
        },
    )?;
    let repo = open_repo()?;
    let notes_ref = match top.value("ref") {
        Some(r) => full_ref(&r),
        None => std::env::var("GIT_NOTES_REF")
            .ok()
            .filter(|s| !s.is_empty())
            .or_else(|| arks_config::ConfigSet::load(Some(repo.common_dir())).get("core.notesRef"))
            .map(|r| full_ref(&r))
            .unwrap_or_else(|| "refs/notes/commits".to_string()),
    };
    drop(repo);
    let mut rest = top.args();
    rest.extend(top.after_dashdash().into_iter().collect::<Vec<_>>());
    let Some(sub) = rest.first().cloned() else {
        return list(&notes_ref, &[]);
    };
    let sub_args = &rest[1..];
    let p = |defs| parse(&format!("notes {sub}"), sub_args, defs, Mode::default());
    match sub.as_str() {
        "list" => {
            let p = p(&[])?;
            list(&notes_ref, &p.args())
        }
        "add" => add_like(&notes_ref, "add", &p(t::NOTES__ADD)?),
        "append" => add_like(&notes_ref, "append", &p(t::NOTES__APPEND)?),
        "edit" => add_like(&notes_ref, "edit", &p(t::NOTES__EDIT)?),
        "copy" => copy(&notes_ref, &p(t::NOTES__COPY)?),
        "show" => {
            let p = p(&[])?;
            show(&notes_ref, &p.args())
        }
        "remove" => remove(&notes_ref, &p(t::NOTES__REMOVE)?),
        "prune" => prune(&notes_ref, &p(t::NOTES__PRUNE)?),
        "get-ref" => {
            let _ = p(&[])?;
            println!("{notes_ref}");
            Ok(())
        }
        "merge" => merge(&notes_ref, &p(t::NOTES__MERGE)?),
        other => Err(CliError::Exit(
            129,
            format!("error: unknown subcommand: `{other}'\n{USAGE}"),
        )),
    }
}

fn resolve_object(repo: &Repository, rev: Option<&String>) -> CliResult<Oid> {
    let rev = rev.map(String::as_str).unwrap_or("HEAD");
    resolve_rev(repo, rev)
        .map_err(|_| CliError::fatal(format!("failed to resolve '{rev}' as a valid ref.")))
}

/// object hex -> note blob, read from the notes commit's tree (any fan-out).
fn load(repo: &Repository, notes_ref: &str) -> CliResult<(Option<Oid>, BTreeMap<String, Oid>)> {
    let Ok(tip) = repo.refs().resolve(notes_ref) else {
        return Ok((None, BTreeMap::new()));
    };
    let obj = repo.get_object(&tip)?;
    let tree = match obj.kind {
        ObjectKind::Commit => Commit::decode(&obj.payload)?.tree,
        ObjectKind::Tree => tip,
        _ => {
            return Err(CliError::fatal(format!(
                "{notes_ref} does not point to a notes commit"
            )));
        }
    };
    let mut map = BTreeMap::new();
    let mut stack = vec![(String::new(), tree)];
    while let Some((prefix, t)) = stack.pop() {
        let o = repo.get_object(&t)?;
        for e in Tree::decode(&o.payload, repo.oid_kind())?.entries {
            let name = format!("{prefix}{}", e.name);
            if e.mode.is_tree() {
                stack.push((name, e.oid));
            } else {
                map.insert(name, e.oid);
            }
        }
    }
    let parent = (obj.kind == ObjectKind::Commit).then_some(tip);
    Ok((parent, map))
}

/// Write `notes` as a flat tree and a notes commit with `message`.
fn store(
    repo: &mut Repository,
    notes_ref: &str,
    parent: Option<Oid>,
    notes: &BTreeMap<String, Oid>,
    message: &str,
) -> CliResult<()> {
    let entries: Vec<TreeEntry> = notes
        .iter()
        .map(|(k, v)| TreeEntry {
            mode: TreeMode::File,
            name: k.clone(),
            oid: *v,
        })
        .collect();
    let tree = Tree::new(entries)?;
    let format = repo.object_format();
    let tree_oid = repo
        .store_mut()
        .put(format, &RawObject::new(ObjectKind::Tree, tree.encode()?))?;
    let commit = Commit {
        tree: tree_oid,
        parents: parent.into_iter().collect(),
        author: commit_signature()?,
        committer: committer_signature()?,
        gpgsig: None,
        message: format!("{message}\n"),
    };
    let coid = repo.store_mut().put(
        format,
        &RawObject::new(ObjectKind::Commit, commit.encode()?),
    )?;
    let old = repo.refs().resolve(notes_ref).ok();
    repo.refs_mut().update(notes_ref, &coid, old.as_ref())?;
    let _ = crate::reflog::append_reflog(
        repo,
        notes_ref,
        old.as_ref(),
        &coid,
        &format!("notes: {message}"),
    );
    Ok(())
}

fn list(notes_ref: &str, args: &[String]) -> CliResult<()> {
    let repo = open_repo()?;
    let (_, notes) = load(&repo, notes_ref)?;
    if let Some(rev) = args.first() {
        let oid = resolve_object(&repo, Some(rev))?;
        return match notes.get(&oid.to_hex()) {
            Some(n) => {
                println!("{}", n.to_hex());
                Ok(())
            }
            None => Err(CliError::Exit(
                1,
                format!("error: no note found for object {}.", oid.to_hex()),
            )),
        };
    }
    for (k, v) in &notes {
        println!("{} {k}", v.to_hex());
    }
    Ok(())
}

fn show(notes_ref: &str, args: &[String]) -> CliResult<()> {
    let repo = open_repo()?;
    let oid = resolve_object(&repo, args.first())?;
    let (_, notes) = load(&repo, notes_ref)?;
    let Some(n) = notes.get(&oid.to_hex()) else {
        return Err(CliError::Exit(
            1,
            format!("error: no note found for object {}.", oid.to_hex()),
        ));
    };
    let text = repo.get_object(n)?.payload;
    use std::io::Write;
    std::io::stdout().write_all(&text)?;
    Ok(())
}

/// The note text from `-m` / `-F` / `-c` / `-C` in argv order.
struct Given {
    parts: Vec<(String, bool)>,
    edit: bool,
}

fn given_message(repo: &Repository, p: &Parsed) -> CliResult<Given> {
    let mut parts = Vec::new();
    let mut edit = false;
    for it in &p.items {
        let Item::Opt {
            key, val: Some(v), ..
        } = it
        else {
            continue;
        };
        match *key {
            "message" => parts.push((v.clone(), false)),
            "file" => {
                let text = if v == "-" {
                    let mut s = String::new();
                    std::io::Read::read_to_string(&mut std::io::stdin(), &mut s)?;
                    s
                } else {
                    std::fs::read_to_string(v).map_err(|e| {
                        CliError::fatal(format!("could not open or read '{v}': {e}"))
                    })?
                };
                parts.push((text, false));
            }
            "reuse-message" | "reedit-message" => {
                let oid = resolve_rev(repo, v).map_err(|_| {
                    CliError::fatal(format!("failed to resolve '{v}' as a valid ref."))
                })?;
                let obj = repo.get_object(&oid)?;
                if obj.kind != ObjectKind::Blob {
                    return Err(CliError::fatal(format!(
                        "cannot read note data from non-blob object '{v}'."
                    )));
                }
                parts.push((String::from_utf8_lossy(&obj.payload).into_owned(), true));
                if *key == "reedit-message" {
                    edit = true;
                }
            }
            _ => {}
        }
    }
    Ok(Given { parts, edit })
}

/// git's stripspace without comment removal: trailing whitespace off,
/// blank-line runs collapsed, leading/trailing blank lines dropped.
fn stripspace(s: &str) -> String {
    crate::cmd::trailers::cleanup(s, crate::cmd::trailers::Cleanup::Whitespace, false)
}

fn add_like(notes_ref: &str, sub: &str, p: &Parsed) -> CliResult<()> {
    let mut repo = open_repo()?;
    refuse_notes_contention(&repo, notes_ref)?;
    let pos = p.args();
    if pos.len() > 1 {
        return Err(CliError::Exit(
            129,
            format!("error: too many arguments\n{USAGE}"),
        ));
    }
    let oid = resolve_object(&repo, pos.first())?;
    let key = oid.to_hex();
    let (parent, mut notes) = load(&repo, notes_ref)?;
    let existing = notes
        .get(&key)
        .map(|n| {
            repo.get_object(n)
                .map(|o| String::from_utf8_lossy(&o.payload).into_owned())
        })
        .transpose()?;
    let given = given_message(&repo, p)?;
    if sub == "edit" && !given.parts.is_empty() {
        eprintln!(
            "The -m/-F/-c/-C options have been deprecated for the 'edit' subcommand.\nPlease use 'git notes add -f -m/-F/-c/-C' instead."
        );
    }
    let force = p.on("force");
    if sub == "add" && existing.is_some() && !force {
        return Err(CliError::Exit(
            1,
            format!(
                "error: Cannot add notes. Found existing notes for object {key}. Use '-f' to overwrite existing notes"
            ),
        ));
    }
    let strip = p.flag("stripspace") != Some(false);
    // Paragraph separator between -m/-F parts and for append.
    let sep: Option<String> = if p.flag("separator") == Some(false) {
        None
    } else {
        match p.last("separator") {
            Some(Some(s)) => Some(if s.ends_with('\n') {
                s
            } else {
                format!("{s}\n")
            }),
            _ => Some("\n".into()),
        }
    };
    let join = |parts: &[String]| -> String {
        let mut out = String::new();
        for (i, part) in parts.iter().enumerate() {
            if i > 0 {
                if let Some(s) = &sep {
                    out.push_str(s);
                }
            }
            out.push_str(part);
            if !out.ends_with('\n') {
                out.push('\n');
            }
        }
        out
    };
    let mut parts: Vec<String> = Vec::new();
    for (text, verbatim) in &given.parts {
        let t = if strip && !verbatim {
            stripspace(text)
        } else {
            text.clone()
        };
        if !t.trim().is_empty() || !strip {
            parts.push(t);
        }
    }
    let mut note = if given.parts.is_empty() || given.edit {
        // The editor starts from the existing note (add -f, append, edit).
        let start = if given.parts.is_empty() {
            match sub {
                "append" => String::new(),
                _ => existing.clone().unwrap_or_default(),
            }
        } else {
            join(&parts)
        };
        let edited = run_editor(&repo, &start)?;
        crate::cmd::trailers::cleanup(&edited, crate::cmd::trailers::Cleanup::Strip, true)
    } else if parts.is_empty() {
        String::new()
    } else {
        let joined = join(&parts);
        if strip || given.parts.iter().all(|(_, v)| *v) {
            joined
        } else {
            joined.trim_end_matches('\n').to_string()
        }
    };
    if !strip && given.parts.len() == 1 && !given.edit {
        note = given.parts[0].0.clone();
    }
    if sub == "append" {
        if let Some(old) = &existing {
            if !note.is_empty() {
                let mut combined = old.clone();
                if !combined.ends_with('\n') {
                    combined.push('\n');
                }
                if let Some(s) = &sep {
                    combined.push_str(s);
                }
                combined.push_str(&note);
                note = combined;
            } else {
                note = old.clone();
            }
        }
    }
    let allow_empty = p.on("allow-empty");
    if note.is_empty() && !allow_empty {
        if existing.is_some() {
            eprintln!("Removing note for object {key}");
            notes.remove(&key);
            return store(
                &mut repo,
                notes_ref,
                parent,
                &notes,
                &format!("Notes removed by 'git notes {sub}'"),
            );
        }
        return Ok(());
    }
    if sub == "add" && existing.is_some() {
        eprintln!("Overwriting existing notes for object {key}");
    }
    let format = repo.object_format();
    let blob = repo
        .store_mut()
        .put(format, &RawObject::new(ObjectKind::Blob, note.into_bytes()))?;
    notes.insert(key, blob);
    store(
        &mut repo,
        notes_ref,
        parent,
        &notes,
        &format!("Notes added by 'git notes {sub}'"),
    )
}

fn run_editor(repo: &Repository, start: &str) -> CliResult<String> {
    let path = repo.git_dir().join("NOTES_EDITMSG");
    std::fs::write(&path, start)?;
    if let Some(editor) = ["ARKS_EDITOR", "GIT_EDITOR", "EDITOR"]
        .iter()
        .find_map(|v| std::env::var(v).ok().filter(|s| !s.is_empty()))
    {
        let ok = std::process::Command::new("sh")
            .arg("-c")
            .arg(format!("{editor} \"$@\""))
            .arg(&editor)
            .arg(&path)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if !ok {
            return Err(CliError::Exit(
                1,
                "error: there was a problem with the editor".into(),
            ));
        }
    } else {
        return Err(CliError::Usage(
            "notes: no message given and no editor (set GIT_EDITOR / ARKS_EDITOR / EDITOR)".into(),
        ));
    }
    let text = std::fs::read_to_string(&path)?;
    let _ = std::fs::remove_file(&path);
    Ok(text)
}

fn copy(notes_ref: &str, p: &Parsed) -> CliResult<()> {
    let mut repo = open_repo()?;
    refuse_notes_contention(&repo, notes_ref)?;
    let force = p.on("force");
    let (parent, mut notes) = load(&repo, notes_ref)?;
    let mut pairs: Vec<(String, String)> = Vec::new();
    if p.on("stdin") || p.value("for-rewrite").is_some() {
        let mut s = String::new();
        std::io::Read::read_to_string(&mut std::io::stdin(), &mut s)?;
        for line in s.lines() {
            let mut w = line.split_whitespace();
            if let (Some(a), Some(b)) = (w.next(), w.next()) {
                pairs.push((a.to_string(), b.to_string()));
            }
        }
    } else {
        let pos = p.args();
        match pos.len() {
            2 => pairs.push((pos[0].clone(), pos[1].clone())),
            1 => pairs.push((pos[0].clone(), "HEAD".into())),
            0 => {
                return Err(CliError::Exit(
                    129,
                    format!("error: too few arguments\n{USAGE}"),
                ));
            }
            _ => {
                return Err(CliError::Exit(
                    129,
                    format!("error: too many arguments\n{USAGE}"),
                ));
            }
        }
    }
    let mut changed = false;
    for (from, to) in pairs {
        let f = resolve_object(&repo, Some(&from))?;
        let t = resolve_object(&repo, Some(&to))?;
        let Some(note) = notes.get(&f.to_hex()).copied() else {
            return Err(CliError::Exit(
                1,
                format!(
                    "error: missing notes on source object {}. Cannot copy.",
                    f.to_hex()
                ),
            ));
        };
        if notes.contains_key(&t.to_hex()) && !force {
            return Err(CliError::Exit(
                1,
                format!(
                    "error: Cannot copy notes. Found existing notes for object {}. Use '-f' to overwrite existing notes",
                    t.to_hex()
                ),
            ));
        }
        if notes.contains_key(&t.to_hex()) {
            eprintln!("Overwriting existing notes for object {}", t.to_hex());
        }
        notes.insert(t.to_hex(), note);
        changed = true;
    }
    if !changed {
        return Ok(());
    }
    store(
        &mut repo,
        notes_ref,
        parent,
        &notes,
        "Notes added by 'git notes copy'",
    )
}

fn remove(notes_ref: &str, p: &Parsed) -> CliResult<()> {
    let mut repo = open_repo()?;
    refuse_notes_contention(&repo, notes_ref)?;
    let ignore_missing = p.on("ignore-missing");
    let mut targets = p.args();
    if p.on("stdin") {
        let mut s = String::new();
        std::io::Read::read_to_string(&mut std::io::stdin(), &mut s)?;
        targets.extend(s.split_whitespace().map(String::from));
    }
    if targets.is_empty() && !p.on("stdin") {
        targets.push("HEAD".into());
    }
    let (parent, mut notes) = load(&repo, notes_ref)?;
    let mut removed = false;
    let mut missing = false;
    for rev in &targets {
        let oid = resolve_object(&repo, Some(rev))?;
        if notes.remove(&oid.to_hex()).is_some() {
            eprintln!("Removing note for object {rev}");
            removed = true;
        } else if !ignore_missing {
            eprintln!("Object {rev} has no note");
            missing = true;
        }
    }
    if removed {
        store(
            &mut repo,
            notes_ref,
            parent,
            &notes,
            "Notes removed by 'git notes remove'",
        )?;
    }
    if missing {
        return Err(CliError::silent(1));
    }
    Ok(())
}

fn prune(notes_ref: &str, p: &Parsed) -> CliResult<()> {
    let mut repo = open_repo()?;
    let dry = p.on("dry-run");
    let verbose = p.on("verbose") || dry;
    let (parent, mut notes) = load(&repo, notes_ref)?;
    let gone: Vec<String> = notes
        .keys()
        .filter(|k| {
            Oid::from_hex(k)
                .ok()
                .map(|o| repo.get_object(&o).is_err())
                .unwrap_or(false)
        })
        .cloned()
        .collect();
    for k in &gone {
        if verbose {
            println!("{k}");
        }
        notes.remove(k);
    }
    if gone.is_empty() || dry {
        return Ok(());
    }
    store(
        &mut repo,
        notes_ref,
        parent,
        &notes,
        "Notes removed by 'git notes prune'",
    )
}

fn merge(notes_ref: &str, p: &Parsed) -> CliResult<()> {
    let repo = open_repo()?;
    if p.on("abort") {
        if !repo.git_dir().join("NOTES_MERGE_PARTIAL").exists() {
            return Err(CliError::Exit(
                1,
                "error: failed to remove 'git notes merge' worktree".into(),
            ));
        }
        let _ = std::fs::remove_file(repo.git_dir().join("NOTES_MERGE_PARTIAL"));
        let _ = std::fs::remove_file(repo.git_dir().join("NOTES_MERGE_REF"));
        let _ = std::fs::remove_dir_all(repo.git_dir().join("NOTES_MERGE_WORKTREE"));
        return Ok(());
    }
    if p.on("commit") {
        if !repo.git_dir().join("NOTES_MERGE_PARTIAL").exists() {
            return Err(CliError::fatal("failed to read ref NOTES_MERGE_PARTIAL"));
        }
        return Err(CliError::NotImplemented(
            "notes merge --commit of a conflicted notes merge is not implemented".into(),
        ));
    }
    if let Some(s) = p.value("strategy") {
        if !matches!(
            s.as_str(),
            "manual" | "ours" | "theirs" | "union" | "cat_sort_uniq"
        ) {
            return Err(CliError::Exit(
                129,
                format!(
                    "error: unknown -s/--strategy: {s}\nusage: git notes merge [<options>] <notes-ref>\n   or: git notes merge --commit [<options>]\n   or: git notes merge --abort [<options>]"
                ),
            ));
        }
    }
    let pos = p.args();
    let [other] = pos.as_slice() else {
        return Err(CliError::Exit(
            129,
            format!("error: must specify a notes ref to merge\n{USAGE}"),
        ));
    };
    let other_ref = full_ref(other);
    let ours = repo.refs().resolve(notes_ref).ok();
    let Ok(theirs) = repo.refs().resolve(&other_ref) else {
        if ours.is_none() {
            return Err(CliError::fatal(format!(
                "Cannot merge empty notes ref ({other_ref}) into empty notes ref ({notes_ref})"
            )));
        }
        return Ok(());
    };
    let quiet = p.on("quiet");
    match ours {
        None => {
            let mut r = open_repo()?;
            r.refs_mut().update(notes_ref, &theirs, None)?;
            if !quiet && p.on("verbose") {
                println!("Fast-forward");
            }
            Ok(())
        }
        Some(o) if o == theirs || arks_repo::ancestor_set(&repo, o)?.contains(&theirs) => {
            if p.on("verbose") {
                println!("Already up to date.");
            }
            Ok(())
        }
        Some(o) if arks_repo::ancestor_set(&repo, theirs)?.contains(&o) => {
            let mut r = open_repo()?;
            r.refs_mut().update(notes_ref, &theirs, Some(&o))?;
            if p.on("verbose") {
                println!("Fast-forward");
            }
            Ok(())
        }
        Some(o) => merge_diverged(
            notes_ref,
            &other_ref,
            o,
            theirs,
            p.value("strategy").as_deref().unwrap_or("manual"),
        ),
    }
}

/// Three-way merge of two notes histories per annotated object; the
/// strategy resolves notes changed differently on both sides.
fn merge_diverged(
    notes_ref: &str,
    other_ref: &str,
    ours: Oid,
    theirs: Oid,
    strategy: &str,
) -> CliResult<()> {
    let mut repo = open_repo()?;
    let read = |repo: &Repository, c: Option<Oid>| -> CliResult<BTreeMap<String, Oid>> {
        let Some(c) = c else {
            return Ok(BTreeMap::new());
        };
        let tree = Commit::decode(&repo.get_object(&c)?.payload)?.tree;
        let mut map = BTreeMap::new();
        let mut stack = vec![(String::new(), tree)];
        while let Some((prefix, t)) = stack.pop() {
            let o = repo.get_object(&t)?;
            for e in Tree::decode(&o.payload, repo.oid_kind())?.entries {
                let name = format!("{prefix}{}", e.name);
                if e.mode.is_tree() {
                    stack.push((name, e.oid));
                } else {
                    map.insert(name, e.oid);
                }
            }
        }
        Ok(map)
    };
    let base = arks_repo::merge_base_oid(&repo, &ours, &theirs)?;
    let b = read(&repo, base)?;
    let x = read(&repo, Some(ours))?;
    let y = read(&repo, Some(theirs))?;
    let mut keys: Vec<&String> = b.keys().chain(x.keys()).chain(y.keys()).collect();
    keys.sort();
    keys.dedup();
    let mut out = BTreeMap::new();
    for k in keys {
        let (bb, xx, yy) = (b.get(k), x.get(k), y.get(k));
        let pick = if xx == yy || yy == bb {
            xx.copied()
        } else if xx == bb {
            yy.copied()
        } else {
            match strategy {
                "ours" => xx.copied(),
                "theirs" => yy.copied(),
                "union" | "cat_sort_uniq" => {
                    eprintln!("Concatenating local and remote notes for {k}");
                    let text = |o: Option<&Oid>| -> CliResult<String> {
                        Ok(match o {
                            Some(o) => {
                                String::from_utf8_lossy(&repo.get_object(o)?.payload).into_owned()
                            }
                            None => String::new(),
                        })
                    };
                    let (tx, ty) = (text(xx)?, text(yy)?);
                    let joined = if strategy == "union" {
                        let mut j = tx.clone();
                        if !j.is_empty() && !j.ends_with('\n') {
                            j.push('\n');
                        }
                        if !j.is_empty() && !ty.is_empty() {
                            j.push('\n');
                        }
                        j.push_str(&ty);
                        j
                    } else {
                        let mut lines: Vec<&str> = tx.lines().chain(ty.lines()).collect();
                        lines.sort();
                        lines.dedup();
                        let mut j = lines.join("\n");
                        j.push('\n');
                        j
                    };
                    let format = repo.object_format();
                    Some(repo.store_mut().put(
                        format,
                        &RawObject::new(ObjectKind::Blob, joined.into_bytes()),
                    )?)
                }
                _ => {
                    return Err(CliError::NotImplemented(format!(
                        "notes merge: conflicting notes for {k} need --strategy (manual resolution is not implemented)"
                    )));
                }
            }
        };
        if let Some(v) = pick {
            out.insert(k.clone(), v);
        }
    }
    let entries: Vec<TreeEntry> = out
        .iter()
        .map(|(k, v)| TreeEntry {
            mode: TreeMode::File,
            name: k.clone(),
            oid: *v,
        })
        .collect();
    let format = repo.object_format();
    let tree_oid = repo.store_mut().put(
        format,
        &RawObject::new(ObjectKind::Tree, Tree::new(entries)?.encode()?),
    )?;
    let commit = Commit {
        tree: tree_oid,
        parents: vec![ours, theirs],
        author: commit_signature()?,
        committer: committer_signature()?,
        gpgsig: None,
        // git leaves this message without a trailing newline.
        message: format!("Merged notes from {other_ref} into {notes_ref}"),
    };
    let coid = repo.store_mut().put(
        format,
        &RawObject::new(ObjectKind::Commit, commit.encode()?),
    )?;
    repo.refs_mut().update(notes_ref, &coid, Some(&ours))?;
    Ok(())
}

/// REF-016 / MISC-017 (M20 Cx-3): notes ref lock + in-progress notes-merge state.
/// Cite: https://git-scm.com/docs/git-notes + api-lockfile
fn refuse_notes_contention(repo: &Repository, notes_ref: &str) -> CliResult<()> {
    let git = repo.git_dir();
    let common = repo.common_dir();
    if git.join("NOTES_MERGE_WORKTREE").exists()
        || git.join("NOTES_MERGE_PARTIAL").is_file()
        || git.join("NOTES_MERGE_REF").is_file()
    {
        return Err(CliError::Msg(
            "notes refused: notes-merge state present (MISC-017 — concurrent notes merge; finish or abort; no dual-merge dialect)".into(),
        ));
    }
    let notes_lock = common.join(format!("{notes_ref}.lock"));
    if notes_lock.is_file() {
        return Err(CliError::Msg(format!(
            "Unable to create '{}' (REF-016 — concurrent notes ref update fail-closed; lease ≠ skip)",
            notes_lock.display()
        )));
    }
    Ok(())
}
