//! Merge and pull (M15 Fs-1/Fs-2).
//!
//! Cite: <https://git-scm.com/docs/git-merge>
//! Fs-1: FF + `--no-ff` when FF-capable.
//! Fs-2: diverged three-way merge (clean or conflicted index).

use crate::cmd::merge_engine::merge_three_way_opts;
use crate::cmd::shared::{commit_signature, current_branch_name, is_ancestor, resolve_rev};
use crate::error::{CliError, CliResult};
use crate::util::{open_repo, refuse_unknown_flags, work_tree};
use arks_object::{Commit, ObjectKind, RawObject};
use arks_refs::RefTarget;
use arks_store::ObjectStore;

pub fn cmd_merge(args: &[String]) -> CliResult<()> {
    // Cloud drop-in §O: `--no-edit` (arks never opens an editor for merges),
    // `-m <msg>`, `-q`. Cite: https://git-scm.com/docs/git-merge
    let (args, message) = take_merge_message(args)?;
    let args = &args;
    refuse_unknown_flags(
        args,
        &[
            "--ff-only",
            "--no-ff",
            "--ff",
            "--no-edit",
            "-q",
            "--quiet",
            "--no-stat",
        ],
    )?;
    let ff_only = args.iter().any(|a| a == "--ff-only");
    let no_ff = args.iter().any(|a| a == "--no-ff");
    if ff_only && no_ff {
        return Err(CliError::Usage(
            "merge: --ff-only and --no-ff are mutually exclusive".into(),
        ));
    }
    let repo = open_repo()?;
    // SEQ-004 (M20 Cx-6): rerere rr-cache lock during merge.
    let rr_lock = repo.common_dir().join("rr-cache.lock");
    if rr_lock.is_file() {
        return Err(CliError::Msg(format!(
            "{} (SEQ-004 — concurrent rerere rr-cache writes refuse; serialize conflict resolution)",
            rr_lock.display()
        )));
    }
    let positional: Vec<&String> = args.iter().filter(|a| !a.starts_with('-')).collect();
    if positional.len() != 1 {
        return Err(CliError::Usage("merge [--ff-only|--no-ff] <branch>".into()));
    }
    merge_onto_msg(positional[0], ff_only, no_ff, message)
}

/// Refuse when the index differs from HEAD or a tracked file differs from the index.
/// Refuse a merge over local changes: any staged change (git-merge: "abort
/// if there are any changes registered in the index relative to HEAD"), or
/// a work-tree change on a path the merge touches (`touched` = paths that
/// differ between HEAD and the merged-in commit).
/// Cite: https://git-scm.com/docs/git-merge#_pre_merge_checks
fn refuse_dirty_tracked(
    repo: &arks_repo::Repository,
    touched: &std::collections::HashSet<String>,
) -> CliResult<()> {
    let head = crate::cmd::shared::head_blob_map(repo)?;
    let index = crate::util::read_index(repo)?;
    let work = work_tree(repo)?;
    let mut dirty = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for e in index.entries.iter().filter(|e| e.stage() == 0) {
        seen.insert(e.path.clone());
        let abs = work.join(&e.path);
        let wt_same = match std::fs::metadata(&abs) {
            Ok(m) if crate::cmd::shared::index_stat_matches(e, &m) => true,
            Ok(_) => {
                std::fs::read(&abs).ok().and_then(|b| {
                    RawObject::new(ObjectKind::Blob, b)
                        .oid(repo.object_format())
                        .ok()
                }) == Some(e.oid)
            }
            Err(_) => false,
        };
        if head.get(&e.path) != Some(&e.oid) || (!wt_same && touched.contains(&e.path)) {
            dirty.push(e.path.clone());
        }
    }
    dirty.extend(head.keys().filter(|k| !seen.contains(*k)).cloned());
    if dirty.is_empty() {
        return Ok(());
    }
    dirty.sort();
    let list: String = dirty.iter().map(|p| format!("\t{p}\n")).collect();
    Err(CliError::Exit(
        2,
        format!(
            "error: Your local changes to the following files would be overwritten by merge:\n{list}Please commit your changes or stash them before you merge.\nAborting\nMerge with strategy ort failed."
        ),
    ))
}

/// Paths whose blobs differ between the trees of `a` and `b`.
fn touched_paths(
    repo: &arks_repo::Repository,
    a: &arks_oid::Oid,
    b: &arks_oid::Oid,
) -> CliResult<std::collections::HashSet<String>> {
    let x = crate::cmd::shared::tree_blob_map(repo, a)?;
    let y = crate::cmd::shared::tree_blob_map(repo, b)?;
    Ok(x.keys()
        .chain(y.keys())
        .filter(|k| x.get(*k) != y.get(*k))
        .cloned()
        .collect())
}

/// Split `-m <msg>` / `--message=<msg>` out of merge args.
fn take_merge_message(args: &[String]) -> CliResult<(Vec<String>, Option<String>)> {
    let mut out = Vec::new();
    let mut msg: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        if a == "-m" || a == "--message" {
            i += 1;
            let v = args
                .get(i)
                .ok_or_else(|| CliError::Usage("merge -m <message>".into()))?;
            msg = Some(match msg {
                Some(m) => format!("{m}\n\n{v}"),
                None => v.clone(),
            });
        } else if let Some(v) = a.strip_prefix("--message=") {
            msg = Some(v.to_string());
        } else {
            out.push(a.clone());
        }
        i += 1;
    }
    Ok((out, msg))
}

pub fn cmd_pull(args: &[String]) -> CliResult<()> {
    // Cloud drop-in §O: `--ff-only`, `--no-rebase`, `--no-edit`, `pull <remote> <branch>`.
    let args = &crate::util::split_short_bundles(args, "rq", "");
    refuse_unknown_flags(
        args,
        &[
            "--rebase",
            "-r",
            "--no-rebase",
            "--ff-only",
            "--ff",
            "--no-ff",
            "--no-edit",
            "-q",
            "--quiet",
        ],
    )?;
    let rebase = args.iter().any(|a| a == "--rebase" || a == "-r")
        && !args.iter().any(|a| a == "--no-rebase");
    let ff_only = args.iter().any(|a| a == "--ff-only");
    let no_ff = args.iter().any(|a| a == "--no-ff");
    let positional: Vec<&String> = args.iter().filter(|a| !a.starts_with('-')).collect();
    let remote_name = positional.first().map(|s| s.as_str()).unwrap_or("origin");
    let mut fetch_args = vec![remote_name.to_string()];
    if let Some(b) = positional.get(1) {
        fetch_args.push((*b).clone());
    }
    crate::cmd::cmd_fetch(&fetch_args)?;
    let repo = open_repo()?;
    let branch = match positional.get(1) {
        Some(b) => b.trim_start_matches("refs/heads/").to_string(),
        None => current_branch_name(&repo)?,
    };
    let upstream = format!("refs/remotes/{remote_name}/{branch}");
    if repo.refs().resolve(&upstream).is_err() {
        return Err(CliError::Msg(format!(
            "no upstream tracking ref {upstream}; fetch may have no matching branch"
        )));
    }
    let upstream_rev = format!("{remote_name}/{branch}");
    if rebase {
        crate::cmd::replay::rebase_onto(&upstream_rev)
    } else {
        // Fs-2: allow three-way merge pull (not forced --ff-only).
        merge_onto(&upstream_rev, ff_only, no_ff)
    }
}

fn merge_onto(name: &str, ff_only: bool, no_ff: bool) -> CliResult<()> {
    merge_onto_msg(name, ff_only, no_ff, None)
}

fn merge_onto_msg(
    name: &str,
    ff_only: bool,
    no_ff: bool,
    message: Option<String>,
) -> CliResult<()> {
    let mut repo = open_repo()?;
    let theirs = resolve_rev(&repo, name)?;
    let head = repo.head_oid()?;
    if head == theirs {
        println!("Already up to date.");
        return Ok(());
    }
    // Full ancestry (all parents), git-merge-base --is-ancestor semantics.
    if is_ancestor(&repo, &theirs, &head)? {
        println!("Already up to date.");
        return Ok(());
    }
    let can_ff = is_ancestor(&repo, &head, &theirs)?;
    if can_ff {
        if ff_only || !no_ff {
            // Fast-forward keeps non-conflicting local changes (two-way switch).
            let work = work_tree(&repo)?;
            let carried =
                crate::cmd::switch_tree::two_way_switch(&repo, &work, Some(&head), &theirs)
                    .map_err(|e| CliError::Msg(e.to_string().replace("checkout", "merge")))?;
            let _ = carried;
            match repo.head_target()? {
                RefTarget::Symbolic(branch) => {
                    repo.refs_mut().update(&branch, &theirs, Some(&head))?;
                }
                RefTarget::Direct(_) => {
                    repo.refs_mut().update("HEAD", &theirs, Some(&head))?;
                }
            }
            println!(
                "Updating {}..{}",
                crate::cmd::shared::abbrev_hex(&repo, &head, 7),
                crate::cmd::shared::abbrev_hex(&repo, &theirs, 7)
            );
            println!("Fast-forward");
            print_merge_diffstat(&repo, &head, &theirs)?;
            if let RefTarget::Symbolic(branch) = repo.head_target()? {
                let _ = crate::reflog::append_head_and_branch(
                    &repo,
                    &branch,
                    Some(&head),
                    &theirs,
                    &format!("merge {name}: Fast-forward"),
                );
            }
            return Ok(());
        }
        refuse_dirty_tracked(&repo, &touched_paths(&repo, &head, &theirs)?)?;
        return create_merge_commit(&mut repo, &head, &theirs, name, None, message);
    }
    if ff_only {
        return Err(CliError::Msg("Not possible to fast-forward".into()));
    }

    // Fs-2: diverged three-way merge. Refuse when tracked files carry local changes
    // (conservative superset of git's per-path check; never overwrites local edits).
    refuse_dirty_tracked(&repo, &touched_paths(&repo, &head, &theirs)?)?;
    let work = work_tree(&repo)?;
    let result = merge_three_way_opts(&mut repo, &work, &head, &theirs, true)?;
    if !result.conflicts.is_empty() {
        eprintln!(
            "Automatic merge failed; fix conflicts and then commit the result.\nConflicted: {}",
            result.conflicts.join(", ")
        );
        return Err(CliError::Msg("merge: conflicts".into()));
    }
    let tree = result
        .tree
        .ok_or_else(|| CliError::Msg("merge: missing result tree".into()))?;
    create_merge_commit(&mut repo, &head, &theirs, name, Some(tree), message)
}

fn create_merge_commit(
    repo: &mut arks_repo::Repository,
    head: &arks_oid::Oid,
    theirs: &arks_oid::Oid,
    name: &str,
    tree_override: Option<arks_oid::Oid>,
    message: Option<String>,
) -> CliResult<()> {
    let tree = if let Some(t) = tree_override {
        t
    } else {
        let tobj = repo.get_object(theirs)?;
        if tobj.kind != ObjectKind::Commit {
            return Err(CliError::Msg("merge tip is not a commit".into()));
        }
        Commit::decode(&tobj.payload)?.tree
    };
    let msg = match message {
        Some(m) if m.ends_with('\n') => m,
        Some(m) => format!("{m}\n"),
        None => format!("Merge branch '{name}'\n"),
    };
    let author = commit_signature()?;
    let committer = crate::cmd::shared::committer_signature()?;
    let commit = Commit {
        tree,
        parents: vec![*head, *theirs],
        author,
        committer,
        gpgsig: None,
        message: msg,
    };
    let cobj = RawObject::new(ObjectKind::Commit, commit.encode()?);
    let format = repo.object_format();
    let coid = repo.store_mut().put(format, &cobj)?;
    match repo.head_target()? {
        RefTarget::Symbolic(branch) => {
            repo.refs_mut().update(&branch, &coid, Some(head))?;
        }
        RefTarget::Direct(_) => {
            return Err(CliError::NotImplemented(
                "merge: detached HEAD not claimed".into(),
            ));
        }
    }
    if tree_override.is_none() {
        // `--no-ff` of a fast-forwardable branch: move index + work tree like
        // a two-way checkout so unrelated local edits are carried over.
        let work = work_tree(repo)?;
        crate::cmd::switch_tree::two_way_switch(repo, &work, Some(head), &coid)?;
    }
    // (Three-way results were already written to the index and work tree.)
    println!("Merge made by the 'ort' strategy.");
    print_merge_diffstat(repo, head, &coid)?;
    if let RefTarget::Symbolic(branch) = repo.head_target()? {
        let _ = crate::reflog::append_head_and_branch(
            repo,
            &branch,
            Some(head),
            &coid,
            &format!("merge {name}: Merge made by the 'ort' strategy."),
        );
    }
    Ok(())
}

/// git merge's closing diffstat (`--stat` block plus `create mode` /
/// `delete mode` / `mode change` summary lines) from `from` to `to`.
/// Cite: https://git-scm.com/docs/git-merge (merge.stat, default true).
fn print_merge_diffstat(
    repo: &arks_repo::Repository,
    from: &arks_oid::Oid,
    to: &arks_oid::Oid,
) -> CliResult<()> {
    use crate::cmd::switch_tree::commit_files;
    use crate::cmd::unified_diff::{StatRow, stat_change, write_stat};
    let a = commit_files(repo, from)?;
    let b = commit_files(repo, to)?;
    let mut keys: Vec<&String> = a.keys().chain(b.keys()).collect();
    keys.sort();
    keys.dedup();
    let mut rows = Vec::new();
    let mut modes = Vec::new();
    for k in keys {
        let (x, y) = (a.get(k), b.get(k));
        if x == y {
            continue;
        }
        let xp = match x {
            Some(f) => Some(repo.get_object(&f.oid)?.payload),
            None => None,
        };
        let yp = match y {
            Some(f) => Some(repo.get_object(&f.oid)?.payload),
            None => None,
        };
        rows.push(StatRow {
            path: k.clone(),
            change: stat_change(xp.as_deref(), yp.as_deref()),
        });
        match (x, y) {
            (None, Some(f)) => modes.push(format!(" create mode {:o} {k}", f.mode)),
            (Some(f), None) => modes.push(format!(" delete mode {:o} {k}", f.mode)),
            (Some(p), Some(q)) if p.mode != q.mode => {
                modes.push(format!(" mode change {:o} => {:o} {k}", p.mode, q.mode))
            }
            _ => {}
        }
    }
    let mut out = std::io::stdout().lock();
    write_stat(&mut out, &rows)?;
    for m in modes {
        std::io::Write::write_all(&mut out, format!("{m}\n").as_bytes())?;
    }
    Ok(())
}
