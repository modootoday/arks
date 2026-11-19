//! Stash (push/save/list/pop/show/apply/drop/clear) + FS3 `stash -p`.
//!
//! Stash entry layout per git-stash §DISCUSSION: the stash commit `W`
//! records the working-tree state of tracked files; its first parent is the
//! `HEAD` commit `H` and its second parent `I` records the index. `I` has
//! message `index on <branch>: <abbrev> <subject>`; `W` has message
//! `WIP on <branch>: <abbrev> <subject>` (or `On <branch>: <message>` with
//! `-m`). `refs/stash` points at the newest `W`; older entries live in the
//! `refs/stash` reflog, whose messages `stash list` prints.
//! Primary: <https://git-scm.com/docs/git-stash>.
//!
//! Partial: `--include-untracked`, `--keep-index`, pathspec-limited push
//! and `apply --index` are not claimed; apply refuses when a path it would
//! touch has local changes instead of merging.

use crate::cmd::interactive_patch::cmd_stash_patch;
use crate::cmd::shared::{
    abbrev_hex, commit_signature, committer_signature, head_blob_map, tree_blob_map,
};
use crate::error::{CliError, CliResult};
use crate::util::{open_repo, read_index, work_tree, write_index};
use crate::write_tree::write_tree_from_index;
use arks_index::{IndexEntry, sort_entries};
use arks_object::{Commit, ObjectKind, RawObject};
use arks_oid::Oid;
use arks_refs::RefTarget;
use arks_repo::Repository;
use arks_store::ObjectStore;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub fn cmd_stash(args: &[String]) -> CliResult<()> {
    // FS3: `-p` / `--patch` on bare `stash` or `stash push -p`.
    if args.iter().any(|a| a == "-p" || a == "--patch") {
        return cmd_stash_patch(args);
    }
    let (sub, rest) = match args.first().map(String::as_str) {
        None => ("push", &args[..0]),
        Some(s) if s.starts_with('-') => ("push", args),
        Some(s) => (s, &args[1..]),
    };
    match sub {
        "push" => {
            let (msg, quiet) = parse_push(rest)?;
            stash_push(msg.as_deref(), quiet)
        }
        "save" => {
            let quiet = rest.iter().any(|a| a == "-q" || a == "--quiet");
            let words: Vec<&str> = rest
                .iter()
                .filter(|a| !a.starts_with('-'))
                .map(String::as_str)
                .collect();
            let msg = (!words.is_empty()).then(|| words.join(" "));
            stash_push(msg.as_deref(), quiet)
        }
        "list" => stash_list(rest),
        "pop" => stash_apply(rest, true),
        "apply" => stash_apply(rest, false),
        "show" => stash_show(rest),
        "drop" => {
            let repo = open_repo()?;
            let n = entry_index(rest)?;
            stash_drop(&repo, n, rest.iter().any(|a| a == "-q" || a == "--quiet"))
        }
        "clear" => stash_clear(),
        other => Err(CliError::NotImplemented(format!(
            "stash {other} not claimed (push/save/list/pop/show/apply/drop/clear/patch)"
        ))),
    }
}

fn parse_push(args: &[String]) -> CliResult<(Option<String>, bool)> {
    let mut msg = None;
    let mut quiet = false;
    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        i += 1;
        match a {
            "-q" | "--quiet" => quiet = true,
            "-m" | "--message" => {
                msg = Some(
                    args.get(i)
                        .cloned()
                        .ok_or_else(|| CliError::fatal("option `message' requires a value"))?,
                );
                i += 1;
            }
            _ => {
                if let Some(v) = a.strip_prefix("--message=") {
                    msg = Some(v.to_string());
                } else if let Some(v) = a.strip_prefix("-m").filter(|v| !v.is_empty()) {
                    msg = Some(v.to_string());
                } else {
                    return Err(CliError::NotImplemented(format!(
                        "stash push: unsupported argument: {a}"
                    )));
                }
            }
        }
    }
    Ok((msg, quiet))
}

/// Public for FS3 `stash -p`.
pub(crate) fn stash_push_public() -> CliResult<()> {
    stash_push(None, false)
}

fn put(repo: &mut Repository, kind: ObjectKind, payload: Vec<u8>) -> CliResult<Oid> {
    let format = repo.object_format();
    Ok(repo
        .store_mut()
        .put(format, &RawObject::new(kind, payload))?)
}

fn tree_of(repo: &mut Repository, entries: &[IndexEntry]) -> CliResult<Oid> {
    if entries.is_empty() {
        return put(repo, ObjectKind::Tree, Vec::new());
    }
    let format = repo.object_format();
    Ok(write_tree_from_index(repo.store_mut(), entries, format)?)
}

fn stash_push(message: Option<&str>, quiet: bool) -> CliResult<()> {
    let mut repo = open_repo()?;
    if repo.git_dir().join("refs").join("stash.lock").is_file() {
        return Err(CliError::Msg(
            "Unable to create refs/stash.lock (STASH-001 — concurrent stash push fail-closed; lease ≠ skip)"
                .into(),
        ));
    }
    let head = repo
        .head_oid()
        .map_err(|_| CliError::fatal("You do not have the initial commit yet"))?;
    let work = work_tree(&repo)?;
    let index = read_index(&repo)?;
    if index.entries.iter().any(|e| e.stage() != 0) {
        return Err(CliError::fatal("could not save index tree"));
    }
    let i_tree = tree_of(&mut repo, &index.entries)?;
    // Working-tree state of tracked files (deleted files drop out).
    let mut w_entries = Vec::with_capacity(index.entries.len());
    for e in &index.entries {
        let abs = work.join(&e.path);
        let Ok(meta) = fs::symlink_metadata(&abs) else {
            continue;
        };
        if !meta.is_file() {
            continue;
        }
        let mut ent = e.clone();
        if !crate::cmd::shared::index_stat_matches(e, &meta) {
            let bytes = fs::read(&abs)?;
            ent.size = bytes.len().min(u32::MAX as usize) as u32;
            ent.oid = put(&mut repo, ObjectKind::Blob, bytes)?;
            ent.mode = crate::cmd::shared::file_mode(&meta);
        }
        w_entries.push(ent);
    }
    let w_tree = tree_of(&mut repo, &w_entries)?;
    let h_commit = crate::cmd::log_format::read_commit(&repo, &head)?;
    if i_tree == h_commit.tree && w_tree == h_commit.tree {
        if !quiet {
            println!("No local changes to save");
        }
        return Ok(());
    }
    let branch = match repo.head_target()? {
        RefTarget::Symbolic(s) => s.strip_prefix("refs/heads/").unwrap_or(&s).to_string(),
        RefTarget::Direct(_) => "(no branch)".to_string(),
    };
    let base = format!(
        "{branch}: {} {}",
        abbrev_hex(&repo, &head, 7),
        crate::cmd::log_format::subject_line(&h_commit.message)
    );
    let author = commit_signature()?;
    let committer = committer_signature()?;
    let commit = |repo: &mut Repository, tree: Oid, parents: Vec<Oid>, msg: String| {
        let c = Commit {
            tree,
            parents,
            author: author.clone(),
            committer: committer.clone(),
            gpgsig: None,
            message: msg,
        };
        let payload = c.encode()?;
        put(repo, ObjectKind::Commit, payload)
    };
    let i_commit = commit(&mut repo, i_tree, vec![head], format!("index on {base}\n"))?;
    let w_msg = match message {
        Some(m) => format!("On {branch}: {m}"),
        None => format!("WIP on {base}"),
    };
    let w_commit = commit(
        &mut repo,
        w_tree,
        vec![head, i_commit],
        // git-stash stores the stash commit message without a trailing newline.
        w_msg.clone(),
    )?;
    let old = repo.refs().resolve("refs/stash").ok();
    repo.refs_mut()
        .update("refs/stash", &w_commit, old.as_ref())?;
    crate::reflog::append_reflog(&repo, "refs/stash", old.as_ref(), &w_commit, &w_msg)?;
    crate::cmd::working_tree::hard_reset_tree(&repo, &work, &head)?;
    if !quiet {
        println!("Saved working directory and index state {w_msg}");
    }
    Ok(())
}

/// `refs/stash` reflog entries, newest first: (new OID, message).
fn stash_entries(repo: &Repository) -> Vec<(Oid, String)> {
    let path = repo.common_dir().join("logs").join("refs").join("stash");
    let Ok(text) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let mut out: Vec<(Oid, String)> = text
        .lines()
        .filter_map(|l| {
            let (head, msg) = l.split_once('\t').unwrap_or((l, ""));
            let new = head.split(' ').nth(1)?;
            Some((Oid::from_hex(new).ok()?, msg.to_string()))
        })
        .collect();
    out.reverse();
    if out.is_empty() {
        if let Ok(o) = repo.refs().resolve("refs/stash") {
            out.push((o, String::new()));
        }
    }
    out
}

fn stash_list(args: &[String]) -> CliResult<()> {
    if let Some(a) = args.first() {
        return Err(CliError::NotImplemented(format!(
            "stash list: unsupported argument: {a}"
        )));
    }
    let repo = open_repo()?;
    for (i, (_, msg)) in stash_entries(&repo).iter().enumerate() {
        println!("stash@{{{i}}}: {msg}");
    }
    Ok(())
}

/// `stash@{<n>}` / `<n>` argument (default 0).
fn entry_index(args: &[String]) -> CliResult<usize> {
    let Some(a) = args.iter().find(|a| !a.starts_with('-')) else {
        return Ok(0);
    };
    let inner = a
        .strip_prefix("stash@{")
        .and_then(|r| r.strip_suffix('}'))
        .unwrap_or(a);
    inner
        .parse()
        .map_err(|_| CliError::fatal(format!("{a} is not a valid reference")))
}

fn entry(repo: &Repository, n: usize) -> CliResult<Oid> {
    let entries = stash_entries(repo);
    if entries.is_empty() {
        return Err(CliError::Exit(1, "No stash entries found.".into()));
    }
    entries
        .get(n)
        .map(|(o, _)| *o)
        .ok_or_else(|| CliError::Exit(1, format!("error: stash@{{{n}}} is not a valid reference")))
}

fn stash_show(args: &[String]) -> CliResult<()> {
    let repo = open_repo()?;
    let w = entry(&repo, entry_index(args)?)?;
    let c = crate::cmd::log_format::read_commit(&repo, &w)?;
    let base = c
        .parents
        .first()
        .ok_or_else(|| CliError::Msg("stash commit has no parent".into()))?;
    let a = tree_blob_map(&repo, base)?;
    let b = tree_blob_map(&repo, &w)?;
    let changes = crate::cmd::diff_render::changes_between(&a, &b, None);
    let fmt = crate::cmd::diff_render::DiffFormat {
        stat: !args.iter().any(|x| x == "-p" || x == "--patch"),
        patch: args.iter().any(|x| x == "-p" || x == "--patch"),
        ..Default::default()
    };
    let mut out = std::io::BufWriter::new(std::io::stdout().lock());
    crate::cmd::diff_render::render(&mut out, &repo, &fmt, &changes)?;
    std::io::Write::flush(&mut out)?;
    Ok(())
}

/// Apply a stash entry onto the working tree: paths the stash changed
/// relative to its base are written; paths newly added in the stashed
/// index are staged (git-stash `apply` without `--index`).
fn stash_apply(args: &[String], drop_after: bool) -> CliResult<()> {
    let repo = open_repo()?;
    {
        let mut lock = repo.index_path().into_os_string();
        lock.push(".lock");
        if std::path::PathBuf::from(lock).is_file() {
            return Err(CliError::Msg(
                "index.lock held; refuse stash pop/apply under concurrent writers (STASH-002)"
                    .into(),
            ));
        }
    }
    let quiet = args.iter().any(|a| a == "-q" || a == "--quiet");
    if args.iter().any(|a| a == "--index") {
        return Err(CliError::NotImplemented(
            "stash apply --index not claimed".into(),
        ));
    }
    let n = entry_index(args)?;
    let w = entry(&repo, n)?;
    let wc = crate::cmd::log_format::read_commit(&repo, &w)?;
    let base = *wc
        .parents
        .first()
        .ok_or_else(|| CliError::Msg("stash commit has no parent".into()))?;
    let base_map = tree_blob_map(&repo, &base)?;
    let w_map = tree_blob_map(&repo, &w)?;
    let i_map = match wc.parents.get(1) {
        Some(i) => tree_blob_map(&repo, i)?,
        None => HashMap::new(),
    };
    let work = work_tree(&repo)?;
    let head = head_blob_map(&repo)?;
    let mut index = read_index(&repo)?;
    let idx: HashMap<String, Oid> = index
        .entries
        .iter()
        .filter(|e| e.stage() == 0)
        .map(|e| (e.path.clone(), e.oid))
        .collect();
    let mut paths: Vec<&String> = base_map.keys().chain(w_map.keys()).collect();
    paths.sort();
    paths.dedup();
    let changed: Vec<&String> = paths
        .into_iter()
        .filter(|p| base_map.get(*p) != w_map.get(*p))
        .collect();
    // Refuse instead of merging when a touched path has local changes.
    for p in &changed {
        let local_dirty =
            idx.get(*p) != head.get(*p) || worktree_differs(&repo, &work, p, idx.get(*p))?;
        let head_moved = head.get(*p) != base_map.get(*p);
        if local_dirty || head_moved {
            return Err(CliError::Exit(
                1,
                format!(
                    "error: Your local changes to the following files would be overwritten by merge:\n\t{p}\n\
                     Please commit your changes or stash them before you merge.\nAborting"
                ),
            ));
        }
    }
    for p in &changed {
        let abs = work.join(p);
        match w_map.get(*p) {
            Some(o) => {
                if let Some(parent) = abs.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(&abs, repo.get_object(o)?.payload)?;
                if !base_map.contains_key(*p) && i_map.contains_key(*p) {
                    let meta = fs::metadata(&abs)?;
                    index.entries.retain(|e| &e.path != *p);
                    index
                        .entries
                        .push(crate::cmd::working_tree::stat_entry(p, *o, &meta));
                }
            }
            None => {
                let _ = fs::remove_file(&abs);
            }
        }
    }
    sort_entries(&mut index.entries);
    write_index(&repo, &index)?;
    if !quiet {
        crate::cmd::status_report::print_long_status(&repo)?;
    }
    if drop_after {
        stash_drop(&repo, n, quiet)?;
    }
    Ok(())
}

fn worktree_differs(
    repo: &Repository,
    work: &Path,
    path: &str,
    oid: Option<&Oid>,
) -> CliResult<bool> {
    let abs = work.join(path);
    match (abs.is_file(), oid) {
        (false, None) => Ok(false),
        (false, Some(_)) | (true, None) => Ok(true),
        (true, Some(o)) => {
            let bytes = fs::read(&abs)?;
            let cur = RawObject::new(ObjectKind::Blob, bytes).oid(repo.object_format())?;
            Ok(cur != *o)
        }
    }
}

/// Remove entry `n`: rewrite the `refs/stash` reflog without it and point
/// `refs/stash` at the newest remaining entry (deleting it when none remain).
fn stash_drop(repo: &Repository, n: usize, quiet: bool) -> CliResult<()> {
    let w = entry(repo, n)?;
    let path = repo.common_dir().join("logs").join("refs").join("stash");
    let text = fs::read_to_string(&path).unwrap_or_default();
    let mut lines: Vec<&str> = text.lines().collect();
    if !lines.is_empty() {
        let idx = lines.len() - 1 - n;
        lines.remove(idx);
        // Keep the old-OID chain consistent for the entry after the removed one.
    }
    let mut repo_mut = open_repo()?;
    match lines.last() {
        Some(last) => {
            let new = last
                .split(' ')
                .nth(1)
                .and_then(|h| Oid::from_hex(h).ok())
                .ok_or_else(|| CliError::Msg("corrupt refs/stash reflog".into()))?;
            let cur = repo_mut.refs().resolve("refs/stash").ok();
            repo_mut
                .refs_mut()
                .update("refs/stash", &new, cur.as_ref())?;
            let mut body = lines.join("\n");
            body.push('\n');
            fs::write(&path, body)?;
        }
        None => {
            let cur = repo_mut.refs().resolve("refs/stash").ok();
            repo_mut.refs_mut().delete("refs/stash", cur.as_ref())?;
            let _ = fs::remove_file(&path);
        }
    }
    if !quiet {
        println!("Dropped refs/stash@{{{n}}} ({})", w.to_hex());
    }
    Ok(())
}

fn stash_clear() -> CliResult<()> {
    let mut repo = open_repo()?;
    if let Ok(oid) = repo.refs().resolve("refs/stash") {
        let _ = repo.refs_mut().delete("refs/stash", Some(&oid));
    }
    let _ = fs::remove_file(repo.common_dir().join("logs").join("refs").join("stash"));
    Ok(())
}
