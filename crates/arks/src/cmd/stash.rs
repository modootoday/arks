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
    abbrev_hex, commit_signature, committer_signature, head_blob_map, resolve_rev, tree_blob_map,
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
    use crate::cmd::opt_parse::{Mode, parse};
    use crate::cmd::porcelain_opts as t;
    let (sub, rest) = match args.first().map(String::as_str) {
        None => ("push", &args[..0]),
        Some(s) if s.starts_with('-') => ("push", args),
        Some(s) => (s, &args[1..]),
    };
    match sub {
        "push" | "save" => {
            let table = if sub == "push" {
                t::STASH__PUSH
            } else {
                t::STASH__SAVE
            };
            let p = parse(&format!("stash {sub}"), rest, table, Mode::default())?;
            if p.on("patch") {
                let mut a = vec!["-p".to_string()];
                if let Some(m) = p.value("message") {
                    a.extend(["-m".to_string(), m]);
                }
                if p.on("quiet") {
                    a.push("-q".to_string());
                }
                a.push("--".to_string());
                a.extend(p.args());
                a.extend(p.after_dashdash());
                return cmd_stash_patch(&a);
            }
            let mut o = PushOpts {
                message: p.value("message"),
                quiet: p.on("quiet"),
                keep_index: p.on("keep-index"),
                staged: p.on("staged"),
                untracked: if p.on("all") {
                    Untracked::All
                } else if p.on("include-untracked") {
                    Untracked::NotIgnored
                } else {
                    Untracked::None
                },
                pathspecs: Vec::new(),
            };
            let mut words = p.args();
            words.extend(p.after_dashdash());
            if sub == "save" {
                // `save [<message>]`: the words form the message.
                if !words.is_empty() {
                    o.message = Some(words.join(" "));
                }
            } else {
                if let Some(f) = p.value("pathspec-from-file") {
                    if !words.is_empty() {
                        return Err(CliError::fatal(
                            "'--pathspec-from-file' and pathspec arguments cannot be used together",
                        ));
                    }
                    words =
                        crate::cmd::opt_parse::read_pathspec_file(&f, p.on("pathspec-file-nul"))?;
                } else if p.on("pathspec-file-nul") {
                    return Err(CliError::fatal(
                        "the option '--pathspec-file-nul' requires '--pathspec-from-file'",
                    ));
                }
                o.pathspecs = words;
            }
            if o.staged && o.untracked != Untracked::None {
                return Err(CliError::fatal(
                    "options '--staged' and '--include-untracked' cannot be used together",
                ));
            }
            stash_push(&o)
        }
        "list" => stash_list(rest),
        "pop" | "apply" => {
            let p = parse(
                &format!("stash {sub}"),
                rest,
                if sub == "pop" {
                    t::STASH__POP
                } else {
                    t::STASH__APPLY
                },
                Mode::default(),
            )?;
            stash_apply(&p.args(), p.on("quiet"), p.on("index"), sub == "pop")
        }
        "show" => stash_show(rest),
        "drop" => {
            let p = parse("stash drop", rest, t::STASH__DROP, Mode::default())?;
            let repo = open_repo()?;
            let sel = entry_index(&p.args())?;
            stash_drop(&repo, &sel, p.on("quiet"))
        }
        "clear" => {
            if !rest.is_empty() {
                return Err(CliError::Exit(129, "usage: git stash clear".into()));
            }
            stash_clear()
        }
        "branch" => stash_branch(rest),
        "create" => {
            let o = PushOpts {
                message: (!rest.is_empty()).then(|| rest.join(" ")),
                ..PushOpts::default()
            };
            if let Some(w) = create_stash(&mut open_repo()?, &o)? {
                println!("{}", w.commit.to_hex());
            }
            Ok(())
        }
        "store" => {
            let p = parse("stash store", rest, t::STASH__STORE, Mode::default())?;
            let a = p.args();
            if a.len() != 1 {
                return Err(CliError::Exit(
                    1,
                    "\"git stash store\" requires one <commit> argument".into(),
                ));
            }
            let repo = open_repo()?;
            // Pin 2.39.5 stores any object name it can resolve (the
            // stash-like check arrived in 2.43); an unknown name fails with 1.
            let oid = if a[0].starts_with("stash@{") || a[0].starts_with("refs/stash@{") {
                entry(&repo, &entry_index(&a)?)?
            } else {
                resolve_rev(&repo, &a[0]).map_err(|_| {
                    CliError::Exit(1, format!("Cannot update refs/stash with {}", a[0]))
                })?
            };
            let msg = p
                .value("message")
                .unwrap_or_else(|| "Created via \"git stash store\".".into());
            store(&repo, oid, &msg)
        }
        other => Err(CliError::fatal(format!(
            "subcommand wasn't specified; 'push' can't be assumed due to unexpected token '{other}'"
        ))),
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Untracked {
    #[default]
    None,
    /// `-u`: untracked files that are not ignored.
    NotIgnored,
    /// `-a`: untracked and ignored files.
    All,
}

#[derive(Default)]
struct PushOpts {
    message: Option<String>,
    quiet: bool,
    keep_index: bool,
    staged: bool,
    untracked: Untracked,
    pathspecs: Vec<String>,
}

/// Public for FS3 `stash -p`.
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
    write_tree_from_index(repo.store_mut(), entries, format)
}

/// A created (not yet stored) stash commit and what it covered.
struct Created {
    commit: Oid,
    message: String,
    head: Oid,
    /// Tracked paths whose index or work-tree state was stashed.
    tracked: Vec<String>,
    /// Untracked (and with `-a` ignored) files recorded in the third parent.
    untracked: Vec<String>,
}

fn pathspec_rel_all(work: &Path, specs: &[String]) -> CliResult<Vec<String>> {
    specs
        .iter()
        .map(|s| crate::cmd::core::pathspec_rel(work, s))
        .collect()
}

/// Build the stash commits (`I`, optional `U`, `W`) without touching refs,
/// the index or the work tree. `None` when there is nothing to save.
fn create_stash(repo: &mut Repository, o: &PushOpts) -> CliResult<Option<Created>> {
    let head = repo
        .head_oid()
        .map_err(|_| CliError::fatal("You do not have the initial commit yet"))?;
    let work = work_tree(repo)?;
    let index = read_index(repo)?;
    if index.entries.iter().any(|e| e.stage() != 0) {
        return Err(CliError::fatal("could not save index tree"));
    }
    let specs = pathspec_rel_all(&work, &o.pathspecs)?;
    let matches = |p: &str| crate::cmd::log_format::path_in_specs(p, &specs);
    let indexed: std::collections::HashSet<String> =
        index.entries.iter().map(|e| e.path.clone()).collect();
    let mut untracked = Vec::new();
    match o.untracked {
        Untracked::None => {}
        Untracked::NotIgnored => {
            crate::cmd::shared::collect_untracked(&work, &work, &indexed, &mut untracked)?
        }
        Untracked::All => crate::cmd::shared::collect_untracked_with(
            &work,
            &work,
            &indexed,
            &mut untracked,
            None,
        )?,
    }
    untracked.retain(|p| matches(p));
    let head_map = head_blob_map(repo)?;
    for s in &specs {
        let hit = indexed
            .iter()
            .chain(head_map.keys())
            .chain(untracked.iter())
            .any(|p| crate::cmd::log_format::path_in_specs(p, std::slice::from_ref(s)));
        if !hit {
            return Err(CliError::Exit(
                1,
                format!(
                    "error: pathspec '{s}' did not match any file(s) known to git\nDid you forget to 'git add'?"
                ),
            ));
        }
    }
    let i_tree = tree_of(repo, &index.entries)?;
    // Work-tree state of tracked files (deleted files drop out); outside the
    // pathspec, and with --staged, the index state is kept.
    let mut w_entries = Vec::with_capacity(index.entries.len());
    let mut tracked = Vec::new();
    for e in &index.entries {
        let staged_change = head_map.get(&e.path) != Some(&e.oid);
        if o.staged || !matches(&e.path) {
            if o.staged && staged_change && matches(&e.path) {
                tracked.push(e.path.clone());
            }
            w_entries.push(e.clone());
            continue;
        }
        let abs = work.join(&e.path);
        let Ok(meta) = fs::symlink_metadata(&abs) else {
            tracked.push(e.path.clone());
            continue;
        };
        if !meta.is_file() {
            continue;
        }
        let mut ent = e.clone();
        if !crate::cmd::shared::index_stat_matches(e, &meta) {
            let bytes = fs::read(&abs)?;
            ent.size = bytes.len().min(u32::MAX as usize) as u32;
            ent.oid = put(repo, ObjectKind::Blob, bytes)?;
            ent.mode = crate::cmd::shared::file_mode(&meta);
        }
        if staged_change || ent.oid != e.oid || ent.mode != e.mode {
            tracked.push(e.path.clone());
        }
        w_entries.push(ent);
    }
    // Paths staged for deletion inside the pathspec.
    for p in head_map.keys() {
        if !indexed.contains(p) && matches(p) {
            tracked.push(p.clone());
        }
    }
    tracked.sort();
    tracked.dedup();
    if tracked.is_empty() && untracked.is_empty() {
        return Ok(None);
    }
    let w_tree = tree_of(repo, &w_entries)?;
    let h_commit = crate::cmd::log_format::read_commit(repo, &head)?;
    let branch = match repo.head_target()? {
        RefTarget::Symbolic(s) => s.strip_prefix("refs/heads/").unwrap_or(&s).to_string(),
        RefTarget::Direct(_) => "(no branch)".to_string(),
    };
    let base = format!(
        "{branch}: {} {}",
        abbrev_hex(repo, &head, 7),
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
    let i_commit = commit(repo, i_tree, vec![head], format!("index on {base}\n"))?;
    let mut parents = vec![head, i_commit];
    if !untracked.is_empty() {
        let mut u_entries = Vec::with_capacity(untracked.len());
        for p in &untracked {
            let abs = work.join(p);
            let meta = fs::symlink_metadata(&abs)?;
            let bytes = if meta.file_type().is_symlink() {
                fs::read_link(&abs)?
                    .to_string_lossy()
                    .into_owned()
                    .into_bytes()
            } else {
                fs::read(&abs)?
            };
            let oid = put(repo, ObjectKind::Blob, bytes)?;
            u_entries.push(crate::cmd::working_tree::stat_entry(p, oid, &meta));
        }
        sort_entries(&mut u_entries);
        let u_tree = tree_of(repo, &u_entries)?;
        parents.push(commit(
            repo,
            u_tree,
            Vec::new(),
            format!("untracked files on {base}\n"),
        )?);
    }
    let w_msg = match &o.message {
        Some(m) => format!("On {branch}: {m}"),
        None => format!("WIP on {base}"),
    };
    // git-stash stores the stash commit message without a trailing newline.
    let w_commit = commit(repo, w_tree, parents, w_msg.clone())?;
    Ok(Some(Created {
        commit: w_commit,
        message: w_msg,
        head,
        tracked,
        untracked,
    }))
}

/// Point `refs/stash` at `w` and log `msg` (`stash store`).
fn store(repo: &Repository, w: Oid, msg: &str) -> CliResult<()> {
    let mut repo_mut = open_repo()?;
    let old = repo.refs().resolve("refs/stash").ok();
    if old == Some(w) {
        // Storing the current entry again changes nothing (no reflog line).
        return Ok(());
    }
    repo_mut.refs_mut().update("refs/stash", &w, old.as_ref())?;
    crate::reflog::append_reflog(repo, "refs/stash", old.as_ref(), &w, msg)?;
    Ok(())
}

/// Write `oid` (or remove the file when `None`) at `path` and return the
/// matching stage-0 index entry.
fn checkout_blob(
    repo: &Repository,
    work: &Path,
    path: &str,
    side: Option<(Oid, u32)>,
) -> CliResult<Option<IndexEntry>> {
    let abs = work.join(path);
    match side {
        Some((oid, mode)) => {
            if let Some(parent) = abs.parent() {
                fs::create_dir_all(parent)?;
            }
            let _ = fs::remove_file(&abs);
            fs::write(&abs, repo.get_object(&oid)?.payload)?;
            #[cfg(unix)]
            if mode == 0o100755 {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&abs, fs::Permissions::from_mode(0o755))?;
            }
            let meta = fs::metadata(&abs)?;
            let mut e = crate::cmd::working_tree::stat_entry(path, oid, &meta);
            e.mode = mode;
            Ok(Some(e))
        }
        None => {
            let _ = fs::remove_file(&abs);
            remove_empty_parents(work, &abs);
            Ok(None)
        }
    }
}

fn remove_empty_parents(work: &Path, abs: &Path) {
    let mut dir = abs.parent();
    while let Some(d) = dir {
        if d == work || fs::remove_dir(d).is_err() {
            break;
        }
        dir = d.parent();
    }
}

fn stash_push(o: &PushOpts) -> CliResult<()> {
    let mut repo = open_repo()?;
    if repo.git_dir().join("refs").join("stash.lock").is_file() {
        return Err(CliError::Msg(
            "Unable to create refs/stash.lock (STASH-001 — concurrent stash push fail-closed; lease ≠ skip)"
                .into(),
        ));
    }
    let Some(c) = create_stash(&mut repo, o)? else {
        if !o.quiet {
            println!("No local changes to save");
        }
        return Ok(());
    };
    store(&repo, c.commit, &c.message)?;
    let work = work_tree(&repo)?;
    let head = c.head;
    if o.pathspecs.is_empty() && !o.staged && !o.keep_index {
        crate::cmd::working_tree::hard_reset_tree(&repo, &work, &head)?;
    } else {
        // Reset only the stashed paths: to HEAD, or with --keep-index the
        // work tree to the index; with --staged only the staged changes go.
        let head_tree = crate::cmd::log_format::read_commit(&repo, &head)?.tree;
        let head_entries = head_entry_map(&repo, &head_tree)?;
        let mut index = read_index(&repo)?;
        let idx: HashMap<String, IndexEntry> = index
            .entries
            .iter()
            .map(|e| (e.path.clone(), e.clone()))
            .collect();
        for p in &c.tracked {
            let h = head_entries.get(p).copied();
            let cur = idx.get(p);
            if o.keep_index && !o.staged {
                let side = cur.map(|e| (e.oid, e.mode));
                if let Some(e) = checkout_blob(&repo, &work, p, side)? {
                    index.entries.retain(|x| &x.path != p);
                    index.entries.push(e);
                }
                continue;
            }
            if o.staged {
                // Undo the staged change; the work tree follows only when it
                // held exactly the staged content.
                let clean = worktree_differs(&repo, &work, p, cur.map(|e| &e.oid)).map(|d| !d)?;
                index.entries.retain(|x| &x.path != p);
                if clean {
                    if let Some(e) = checkout_blob(&repo, &work, p, h)? {
                        index.entries.push(e);
                    }
                } else if let Some((oid, mode)) = h {
                    let mut e = cur.cloned().unwrap_or_else(|| {
                        crate::cmd::working_tree::stat_entry(
                            p,
                            oid,
                            &fs::metadata(work.join(p))
                                .unwrap_or_else(|_| fs::metadata(&work).unwrap()),
                        )
                    });
                    e.oid = oid;
                    e.mode = mode;
                    e.mtime_sec = 0;
                    index.entries.push(e);
                }
                continue;
            }
            index.entries.retain(|x| &x.path != p);
            if let Some(e) = checkout_blob(&repo, &work, p, h)? {
                index.entries.push(e);
            }
        }
        sort_entries(&mut index.entries);
        write_index(&repo, &index)?;
    }
    for p in &c.untracked {
        let abs = work.join(p);
        let _ = fs::remove_file(&abs);
        remove_empty_parents(&work, &abs);
    }
    if !o.quiet {
        println!("Saved working directory and index state {}", c.message);
    }
    Ok(())
}

/// Path → (blob, mode) of a tree.
fn head_entry_map(repo: &Repository, tree: &Oid) -> CliResult<HashMap<String, (Oid, u32)>> {
    let mut out = HashMap::new();
    let mut stack = vec![(String::new(), *tree)];
    while let Some((prefix, t)) = stack.pop() {
        let obj = repo.get_object(&t)?;
        for e in arks_object::Tree::decode(&obj.payload, repo.oid_kind())?.entries {
            let name = format!("{prefix}{}", e.name);
            if e.mode.is_tree() {
                stack.push((format!("{name}/"), e.oid));
            } else {
                let mode = u32::from_str_radix(e.mode.as_str(), 8).unwrap_or(0o100644);
                out.insert(name, (e.oid, mode));
            }
        }
    }
    Ok(out)
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
    // `stash list [<log-options>]`: the diff-output and --oneline subset is
    // rendered here; other log options need reflog walking (`log -g`).
    let mut oneline = false;
    let mut diff_opts = Vec::new();
    for a in args {
        match a.as_str() {
            "--oneline" => oneline = true,
            "--" => {}
            x if x.starts_with('-') => diff_opts.push(x.to_string()),
            x => {
                return Err(CliError::NotImplemented(format!(
                    "stash list: revision argument {x} not implemented"
                )));
            }
        }
    }
    let repo = open_repo()?;
    let entries = stash_entries(&repo);
    for (i, (oid, msg)) in entries.iter().enumerate() {
        if oneline {
            println!("{} refs/stash@{{{i}}}: {msg}", abbrev_hex(&repo, oid, 7));
        } else {
            println!("stash@{{{i}}}: {msg}");
        }
        if !diff_opts.is_empty() {
            let base = crate::cmd::log_format::read_commit(&repo, oid)?
                .parents
                .first()
                .copied()
                .ok_or_else(|| CliError::Msg("stash commit has no parent".into()))?;
            if !oneline {
                println!();
            }
            use std::io::Write;
            std::io::stdout().flush()?;
            let mut a = diff_opts.clone();
            a.push(base.to_hex());
            a.push(oid.to_hex());
            crate::cmd::cmd_diff(&a)?;
        }
    }
    Ok(())
}

/// A selected stash entry: its position, the reflog name git reports when
/// it is out of range, and how `drop` names it.
struct Sel {
    n: usize,
    log: &'static str,
    shown: String,
}

/// `stash@{<n>}` / `<n>` argument (default 0).
fn entry_index(args: &[String]) -> CliResult<Sel> {
    let Some(a) = args.iter().find(|a| !a.starts_with('-')) else {
        return Ok(Sel {
            n: 0,
            log: "refs/stash",
            shown: "refs/stash@{0}".into(),
        });
    };
    if a == "stash" || a == "refs/stash" {
        return Ok(Sel {
            n: 0,
            log: "refs/stash",
            shown: "refs/stash@{0}".into(),
        });
    }
    let (inner, log, shown) = if let Some(r) = a.strip_prefix("stash@{") {
        (r.strip_suffix('}'), "stash", a.clone())
    } else if let Some(r) = a.strip_prefix("refs/stash@{") {
        (r.strip_suffix('}'), "refs/stash", a.clone())
    } else {
        (
            Some(a.as_str()),
            "refs/stash",
            format!("refs/stash@{{{a}}}"),
        )
    };
    let n = inner
        .and_then(|x| x.parse().ok())
        .ok_or_else(|| CliError::Exit(1, format!("error: {a} is not a valid reference")))?;
    Ok(Sel { n, log, shown })
}

fn entry(repo: &Repository, sel: &Sel) -> CliResult<Oid> {
    let entries = stash_entries(repo);
    if entries.is_empty() {
        return Err(CliError::Exit(1, "No stash entries found.".into()));
    }
    entries.get(sel.n).map(|(o, _)| *o).ok_or_else(|| {
        CliError::fatal(format!(
            "log for '{}' only has {} entries",
            sel.log,
            entries.len()
        ))
    })
}

/// `stash branch <name> [<stash>]`: create and switch to `<name>` at the
/// stash's base commit, apply the entry there, then drop it.
/// Cite: https://git-scm.com/docs/git-stash#Documentation/git-stash.txt-branch
fn stash_branch(args: &[String]) -> CliResult<()> {
    let name = args
        .first()
        .filter(|a| !a.starts_with('-'))
        .ok_or_else(|| CliError::Exit(1, "No branch name specified".into()))?;
    let rest = &args[1..];
    let base = {
        let repo = open_repo()?;
        let w = entry(&repo, &entry_index(rest)?)?;
        let c = crate::cmd::log_format::read_commit(&repo, &w)?;
        *c.parents
            .first()
            .ok_or_else(|| CliError::Msg("stash commit has no parent".into()))?
    };
    crate::cmd::cmd_switch(&["-c".to_string(), name.clone(), base.to_hex()])?;
    stash_apply(rest, false, true, true)
}

/// Tree of `entries` (path, blob, mode), written to the object store.
fn tree_from_map(repo: &mut Repository, map: &HashMap<String, (Oid, u32)>) -> CliResult<Oid> {
    let mut entries: Vec<IndexEntry> = map
        .iter()
        .map(|(p, (oid, mode))| IndexEntry {
            ctime_sec: 0,
            ctime_nsec: 0,
            mtime_sec: 0,
            mtime_nsec: 0,
            dev: 0,
            ino: 0,
            mode: *mode,
            uid: 0,
            gid: 0,
            size: 0,
            oid: *oid,
            flags: p.len().min(0xfff) as u16,
            flags_extended: None,
            path: p.clone(),
        })
        .collect();
    sort_entries(&mut entries);
    tree_of(repo, &entries)
}

fn stash_show(args: &[String]) -> CliResult<()> {
    use crate::cmd::opt_parse::{Mode, parse};
    let p = parse(
        "stash show",
        args,
        crate::cmd::porcelain_opts::STASH__SHOW,
        Mode {
            keep_unknown: true,
            ..Mode::default()
        },
    )?;
    let mut repo = open_repo()?;
    let w = entry(&repo, &entry_index(&p.args())?)?;
    let c = crate::cmd::log_format::read_commit(&repo, &w)?;
    let base = *c
        .parents
        .first()
        .ok_or_else(|| CliError::Msg("stash commit has no parent".into()))?;
    let u_tree = match c.parents.get(2) {
        Some(u) => Some(crate::cmd::log_format::read_commit(&repo, u)?.tree),
        None => None,
    };
    let base_tree = crate::cmd::log_format::read_commit(&repo, &base)?.tree;
    let only_untracked = p.on("only-untracked");
    let with_untracked = p.on("include-untracked") || only_untracked;
    let (from, to) = if only_untracked {
        let empty = put(&mut repo, ObjectKind::Tree, Vec::new())?;
        (empty, u_tree.unwrap_or(empty))
    } else if let (true, Some(u)) = (with_untracked, u_tree) {
        let mut map = head_entry_map(&repo, &c.tree)?;
        for (k, v) in head_entry_map(&repo, &u)? {
            map.entry(k).or_insert(v);
        }
        (base_tree, tree_from_map(&mut repo, &map)?)
    } else {
        (base_tree, c.tree)
    };
    let mut a = p.unknown();
    if a.is_empty() {
        // stash.showStat / stash.showPatch, both defaulting to the stat view.
        let cfg = arks_config::ConfigSet::load(Some(repo.common_dir()));
        let yes = |k: &str, d: bool| {
            cfg.get(k)
                .map(|v| matches!(v.to_ascii_lowercase().as_str(), "true" | "yes" | "on" | "1"))
                .unwrap_or(d)
        };
        if yes("stash.showStat", true) {
            a.push("--stat".into());
        }
        if yes("stash.showPatch", false) {
            a.push("-p".into());
        }
    }
    a.push(from.to_hex());
    a.push(to.to_hex());
    crate::cmd::cmd_diff(&a)
}

/// Apply a stash entry onto the working tree: paths the stash changed
/// relative to its base are written; paths newly added in the stashed
/// index are staged, and with `--index` every staged change is restored.
/// Untracked files recorded in a third parent are written back.
fn stash_apply(args: &[String], quiet: bool, with_index: bool, drop_after: bool) -> CliResult<()> {
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
    let sel = entry_index(args)?;
    let w = entry(&repo, &sel)?;
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
    if with_index {
        // `--index`: restore every staged change of the entry too.
        let mut ipaths: Vec<&String> = base_map.keys().chain(i_map.keys()).collect();
        ipaths.sort();
        ipaths.dedup();
        for p in ipaths {
            if base_map.get(p) == i_map.get(p) {
                continue;
            }
            index.entries.retain(|e| &e.path != p);
            let Some(o) = i_map.get(p) else {
                continue;
            };
            let abs = work.join(p);
            let meta = fs::metadata(&abs).or_else(|_| fs::metadata(&work))?;
            let mut e = crate::cmd::working_tree::stat_entry(p, *o, &meta);
            if worktree_differs(&repo, &work, p, Some(o))? {
                e.mtime_sec = 0;
                e.size = 0;
            }
            index.entries.push(e);
        }
    }
    sort_entries(&mut index.entries);
    write_index(&repo, &index)?;
    if let Some(u) = wc.parents.get(2) {
        let u_map = tree_blob_map(&repo, u)?;
        let mut paths: Vec<&String> = u_map.keys().collect();
        paths.sort();
        for p in &paths {
            if work.join(p).exists() {
                return Err(CliError::Exit(
                    1,
                    format!(
                        "{p} already exists, no checkout\nerror: could not restore untracked files from stash"
                    ),
                ));
            }
        }
        for p in paths {
            let abs = work.join(p);
            if let Some(parent) = abs.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&abs, repo.get_object(&u_map[p])?.payload)?;
        }
    }
    if !quiet {
        crate::cmd::status_report::print_long_status(&repo)?;
    }
    if drop_after {
        stash_drop(&repo, &sel, quiet)?;
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
fn stash_drop(repo: &Repository, sel: &Sel, quiet: bool) -> CliResult<()> {
    let n = sel.n;
    let w = entry(repo, sel)?;
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
        println!("Dropped {} ({})", sel.shown, w.to_hex());
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

/// `--autostash` of rebase / merge: stash local changes as an entry named
/// `autostash` and reset the work tree. Returns whether anything was stashed.
pub(crate) fn autostash_push() -> CliResult<bool> {
    let mut repo = open_repo()?;
    let o = PushOpts {
        message: Some("autostash".into()),
        ..PushOpts::default()
    };
    let Some(c) = create_stash(&mut repo, &o)? else {
        return Ok(false);
    };
    store(&repo, c.commit, "autostash")?;
    let work = work_tree(&repo)?;
    crate::cmd::working_tree::hard_reset_tree(&repo, &work, &c.head)?;
    println!("Created autostash: {}", abbrev_hex(&repo, &c.commit, 7));
    Ok(true)
}

/// Re-apply the autostash entry; when it does not apply, keep it and say so.
pub(crate) fn autostash_pop() -> CliResult<()> {
    match stash_apply(&[], true, false, true) {
        Ok(()) => eprintln!("Applied autostash."),
        Err(_) => eprintln!(
            "Applying autostash resulted in conflicts.\nYour changes are safe in the stash.\nYou can run \"git stash pop\" or \"git stash drop\" at any time."
        ),
    }
    Ok(())
}

/// `stash push -p`: record a stash whose index is the current index and
/// whose work tree is HEAD with `selected` (path, mode and content, `None`
/// for a deletion) applied, then print git's "Saved ..." line.
pub(crate) fn push_patch_stash(
    selected: &[(String, crate::cmd::add_patch::Side)],
    message: Option<&str>,
    quiet: bool,
) -> CliResult<()> {
    let mut repo = open_repo()?;
    let head = repo
        .head_oid()
        .map_err(|_| CliError::fatal("You do not have the initial commit yet"))?;
    let index = read_index(&repo)?;
    let i_tree = tree_of(&mut repo, &index.entries)?;
    let h_commit = crate::cmd::log_format::read_commit(&repo, &head)?;
    let mut map = head_entry_map(&repo, &h_commit.tree)?;
    for (path, side) in selected {
        match side {
            Some((mode, bytes)) => {
                let oid = put(&mut repo, ObjectKind::Blob, bytes.clone())?;
                map.insert(path.clone(), (oid, *mode));
            }
            None => {
                map.remove(path);
            }
        }
    }
    let w_tree = tree_from_map(&mut repo, &map)?;
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
    let mut commit = |tree: Oid, parents: Vec<Oid>, msg: String| -> CliResult<Oid> {
        let c = Commit {
            tree,
            parents,
            author: author.clone(),
            committer: committer.clone(),
            gpgsig: None,
            message: msg,
        };
        let payload = c.encode()?;
        put(&mut repo, ObjectKind::Commit, payload)
    };
    let i_commit = commit(i_tree, vec![head], format!("index on {base}\n"))?;
    let w_msg = match message {
        Some(m) => format!("On {branch}: {m}"),
        None => format!("WIP on {base}"),
    };
    let w_commit = commit(w_tree, vec![head, i_commit], w_msg.clone())?;
    let repo = open_repo()?;
    store(&repo, w_commit, &w_msg)?;
    if !quiet {
        println!("Saved working directory and index state {w_msg}");
    }
    Ok(())
}
