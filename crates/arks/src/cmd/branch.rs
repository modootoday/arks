//! Branch / checkout / switch.

use crate::cmd::shared::resolve_rev;
use crate::error::{CliError, CliResult};
use crate::util::{open_repo, read_index, refuse_unknown_flags, work_tree, write_index};
use arks_index::{IndexEntry, sort_entries};
use arks_refs::RefTarget;
use std::collections::HashMap;
use std::fs;
pub fn cmd_checkout(args: &[String]) -> CliResult<()> {
    // M16 Sy-1/Sy-2: `--detach` + `--orphan` claimed (git-checkout 2.39.0).
    // Cite: https://git-scm.com/docs/git-checkout/2.39.0
    // Cloud drop-in §O: `-B <branch> [<start>]`, `-b <branch> <start>`, `-q`.
    let args = &crate::util::split_short_bundles(args, "qf", "bB");
    crate::util::set_quiet_from(args);
    refuse_unknown_flags(
        args,
        &["-b", "-B", "--", "--detach", "--orphan", "-q", "--quiet"],
    )?;
    let create = args.iter().any(|a| a == "-b" || a == "-B");
    let force_create = args.iter().any(|a| a == "-B");
    let detach = args.iter().any(|a| a == "--detach");
    let orphan_idx = args.iter().position(|a| a == "--orphan");
    let dashdash = args.iter().position(|a| a == "--");
    if let Some(i) = dashdash {
        let paths: Vec<&String> = args[i + 1..].iter().collect();
        if paths.is_empty() {
            return Err(CliError::Usage("checkout -- <path>...".into()));
        }
        // `checkout <tree-ish> -- <path>` updates both the index and the work tree.
        let source = args[..i].iter().find(|a| !a.starts_with('-'));
        return match source {
            Some(rev) => restore_paths_mode(&paths, true, true, Some(rev), true),
            None => restore_paths_mode(&paths, false, true, None, true),
        };
    }
    if let Some(i) = orphan_idx {
        let name = args
            .get(i + 1)
            .filter(|a| !a.starts_with('-'))
            .ok_or_else(|| {
                CliError::Usage("checkout --orphan <new-branch> [<start-point>]".into())
            })?;
        // Remaining non-flag args after the orphan name are the optional start-point.
        let start = args
            .iter()
            .enumerate()
            .filter(|(idx, a)| *idx != i && *idx != i + 1 && !a.starts_with('-'))
            .map(|(_, a)| a.as_str())
            .next();
        return checkout_orphan(name, start);
    }
    let positional: Vec<&String> = args.iter().filter(|a| !a.starts_with('-')).collect();
    if create {
        if positional.is_empty() || positional.len() > 2 {
            return Err(CliError::Usage(
                "checkout -b|-B <branch> [<start-point>]".into(),
            ));
        }
        let name = positional[0];
        let mut repo = open_repo()?;
        let head = repo.head_oid().ok();
        let oid = match positional.get(1) {
            Some(start) => resolve_rev(&repo, start)?,
            None => head.ok_or_else(|| CliError::Msg("HEAD has no commit".into()))?,
        };
        let full = branch_ref(name);
        let existed = repo.refs().resolve(&full).is_ok();
        if existed && !force_create {
            return Err(CliError::Msg(format!(
                "a branch named '{name}' already exists"
            )));
        }
        // Two-way switch first: refuse without moving refs when local changes conflict;
        // git skips the tree switch only for `-b <new>` without a start point.
        let carried = if head != Some(oid) || positional.len() > 1 {
            let work = work_tree(&repo)?;
            crate::cmd::switch_tree::two_way_switch(&repo, &work, head.as_ref(), &oid)?
        } else {
            Vec::new()
        };
        let from = crate::reflog::head_label(&repo);
        let old_tip = repo.refs().resolve(&full).ok();
        repo.refs_mut().update(&full, &oid, None)?;
        repo.refs_mut().set_symbolic("HEAD", &full)?;
        let start = positional.get(1).map(|s| s.as_str()).unwrap_or("HEAD");
        let _ = crate::reflog::append_reflog(
            &repo,
            &full,
            old_tip.as_ref(),
            &oid,
            &format!("branch: Created from {start}"),
        );
        let _ = crate::reflog::append_reflog(
            &repo,
            "HEAD",
            head.as_ref(),
            &oid,
            &format!("checkout: moving from {from} to {name}"),
        );
        for l in carried.iter().filter(|_| !crate::util::quiet()) {
            println!("{l}");
        }
        if existed {
            if !crate::util::quiet() {
                eprintln!("Switched to and reset branch '{name}'");
            }
        } else {
            if !crate::util::quiet() {
                eprintln!("Switched to a new branch '{name}'");
            }
        }
        return Ok(());
    }
    if detach {
        return match positional.as_slice() {
            [] => detach_to("HEAD"),
            [rev] => detach_to(rev),
            _ => Err(CliError::Usage("checkout --detach [<commit>]".into())),
        };
    }
    if positional.is_empty() {
        return Err(CliError::Usage(
            "checkout <branch> | checkout -- <path>".into(),
        ));
    }
    // git-checkout: an argument that names no branch or commit but a path is a
    // pathspec (`checkout <path>` == `checkout -- <path>`).
    let repo = open_repo()?;
    let first = positional[0];
    let is_rev =
        repo.refs().resolve(&branch_ref(first)).is_ok() || resolve_rev(&repo, first).is_ok();
    if !is_rev {
        let work = work_tree(&repo)?;
        let rel = crate::cmd::core::pathspec_rel(&work, first).unwrap_or_default();
        let tracked = read_index(&repo)?
            .entries
            .iter()
            .any(|e| e.path == rel || e.path.starts_with(&format!("{rel}/")));
        if tracked {
            return restore_paths_mode(&positional, false, true, None, true);
        }
    } else if positional.len() > 1 {
        return restore_paths_mode(&positional[1..], true, true, Some(first), true);
    }
    let is_branch = repo.refs().resolve(&branch_ref(first)).is_ok();
    drop(repo);
    if positional.len() != 1 {
        return Err(CliError::Usage(
            "checkout <branch> | checkout -- <path>".into(),
        ));
    }
    // A commit-ish that is not a branch name detaches HEAD (git-checkout).
    if !is_branch {
        return detach_to(first);
    }
    switch_branch(positional[0], false)
}

/// `checkout --orphan <new-branch> [<start-point>]`.
///
/// Points `HEAD` at an unborn branch (symbolic ref with no tip yet), and leaves
/// the index/worktree populated from `<start-point>` (default `HEAD`) so the
/// first commit has no parents. Cite: git-checkout(1) 2.39.0 `--orphan`.
fn checkout_orphan(name: &str, start_point: Option<&str>) -> CliResult<()> {
    let mut repo = open_repo()?;
    let start = start_point.unwrap_or("HEAD");
    let oid = resolve_rev(&repo, start)?;
    let full = branch_ref(name);
    if repo.refs().read(&full).is_ok() {
        return Err(CliError::Msg(format!(
            "a branch named '{name}' already exists"
        )));
    }
    // Unborn branch: symbolic HEAD only — do not create refs/heads/<name> yet.
    repo.refs_mut().set_symbolic("HEAD", &full)?;
    let work = work_tree(&repo)?;
    arks_remote::checkout_commit(&repo, &work, &oid)?;
    println!("Switched to a new branch '{name}'");
    Ok(())
}

pub fn cmd_switch(args: &[String]) -> CliResult<()> {
    // M16 Sy-1/Sy-2: `--detach` + `--orphan` claimed (git-switch). Cite: https://git-scm.com/docs/git-switch/2.39.0
    let args = &crate::util::alias_flags(
        crate::util::split_short_bundles(args, "qd", "cC"),
        &[
            ("--create", "-c"),
            ("--force-create", "-C"),
            ("-d", "--detach"),
            ("--quiet", "-q"),
        ],
    );
    refuse_unknown_flags(args, &["-c", "-C", "--detach", "--orphan", "-q"])?;
    crate::util::set_quiet_from(args);
    let create = args.iter().any(|a| a == "-c" || a == "-C");
    let force_create = args.iter().any(|a| a == "-C");
    let detach = args.iter().any(|a| a == "--detach");
    let orphan_idx = args.iter().position(|a| a == "--orphan");
    if let Some(i) = orphan_idx {
        let name = args
            .get(i + 1)
            .filter(|a| !a.starts_with('-'))
            .ok_or_else(|| CliError::Usage("switch --orphan <new-branch>".into()))?;
        // git-switch --orphan clears the index/worktree; Sy-2 claims checkout --orphan
        // (index retained). For switch, empty index/worktree after pointing HEAD unborn.
        return switch_orphan(name);
    }
    let positional: Vec<&String> = args.iter().filter(|a| !a.starts_with('-')).collect();
    if positional.is_empty() || positional.len() > 2 || (!create && positional.len() != 1) {
        return Err(CliError::Usage(
            "switch [-c|-C <new-branch> [<start>]] [--detach] <branch|commit>".into(),
        ));
    }
    let name = positional[0];
    if detach {
        return detach_to(name);
    }
    if create {
        let mut repo = open_repo()?;
        let head = repo.head_oid().ok();
        let oid = match positional.get(1) {
            Some(start) => resolve_rev(&repo, start)?,
            None => head.ok_or_else(|| CliError::Msg("HEAD has no commit".into()))?,
        };
        let full = branch_ref(name);
        let existed = repo.refs().resolve(&full).is_ok();
        if existed && !force_create {
            return Err(CliError::Msg(format!(
                "a branch named '{name}' already exists"
            )));
        }
        // git skips the tree switch only for `-b <new>` without a start point.
        let carried = if head != Some(oid) || positional.len() > 1 {
            let work = work_tree(&repo)?;
            crate::cmd::switch_tree::two_way_switch(&repo, &work, head.as_ref(), &oid)?
        } else {
            Vec::new()
        };
        let from = crate::reflog::head_label(&repo);
        let old_tip = repo.refs().resolve(&full).ok();
        repo.refs_mut().update(&full, &oid, None)?;
        repo.refs_mut().set_symbolic("HEAD", &full)?;
        let start = positional.get(1).map(|s| s.as_str()).unwrap_or("HEAD");
        let _ = crate::reflog::append_reflog(
            &repo,
            &full,
            old_tip.as_ref(),
            &oid,
            &format!("branch: Created from {start}"),
        );
        let _ = crate::reflog::append_reflog(
            &repo,
            "HEAD",
            head.as_ref(),
            &oid,
            &format!("checkout: moving from {from} to {name}"),
        );
        for l in carried.iter().filter(|_| !crate::util::quiet()) {
            println!("{l}");
        }
        if existed {
            if !crate::util::quiet() {
                eprintln!("Switched to and reset branch '{name}'");
            }
        } else {
            if !crate::util::quiet() {
                eprintln!("Switched to a new branch '{name}'");
            }
        }
        Ok(())
    } else {
        switch_branch(name, false)
    }
}

fn switch_orphan(name: &str) -> CliResult<()> {
    // git-switch --orphan: unborn branch + empty index/worktree (differs from checkout --orphan).
    let mut repo = open_repo()?;
    let full = branch_ref(name);
    if repo.refs().read(&full).is_ok() {
        return Err(CliError::Msg(format!(
            "a branch named '{name}' already exists"
        )));
    }
    repo.refs_mut().set_symbolic("HEAD", &full)?;
    let work = work_tree(&repo)?;
    // Remove tracked files from worktree, then clear index.
    if let Ok(index) = read_index(&repo) {
        for e in index.entries.iter().filter(|e| e.stage() == 0) {
            let p = work.join(&e.path);
            if p.is_file() {
                let _ = fs::remove_file(&p);
            }
        }
    }
    write_index(
        &repo,
        &arks_index::Index {
            version: 2,
            entries: Vec::new(),
            extensions: Vec::new(),
        },
    )?;
    println!("Switched to a new branch '{name}'");
    Ok(())
}

pub(crate) fn branch_ref(name: &str) -> String {
    if name.starts_with("refs/heads/") {
        name.to_string()
    } else {
        format!("refs/heads/{name}")
    }
}

fn switch_branch(name: &str, _detach: bool) -> CliResult<()> {
    let mut repo = open_repo()?;
    let full = branch_ref(name);
    let oid = repo.refs().resolve(&full)?;
    // Same-commit switch (e.g. `branch topic` then `checkout topic`): update
    // HEAD only — skip worktree/index rewrite (git short-circuit class; M17).
    // Do **not** short-circuit when the index is empty: unbundle/fetch can advance
    // `refs/heads/<name>` while HEAD already points there, leaving an empty worktree
    // that still needs materialization (G13 bundle→checkout).
    let same_oid = repo.head_oid().ok() == Some(oid);
    let index_populated = read_index(&repo)
        .map(|idx| !idx.entries.is_empty())
        .unwrap_or(false);
    // TOCTOU-003 (M19 Cx-2): refuse checkouts that would overwrite local edits — now
    // per path via the two-way switch (git semantics: non-conflicting changes carry over).
    let mut carried = Vec::new();
    if !(same_oid && index_populated) {
        let work = work_tree(&repo)?;
        let head = repo.head_oid().ok();
        if index_populated {
            carried = crate::cmd::switch_tree::two_way_switch(&repo, &work, head.as_ref(), &oid)
                .map_err(|e| CliError::Msg(format!("{e} (TOCTOU-003)")))?;
        } else {
            arks_remote::checkout_commit(&repo, &work, &oid)?;
        }
    }
    let was_current = matches!(repo.head_target(), Ok(RefTarget::Symbolic(ref s)) if *s == full);
    let from = crate::reflog::head_label(&repo);
    let old_head = repo.head_oid().ok();
    repo.refs_mut().set_symbolic("HEAD", &full)?;
    let _ = crate::reflog::append_reflog(
        &repo,
        "HEAD",
        old_head.as_ref(),
        &oid,
        &format!("checkout: moving from {from} to {name}"),
    );
    for l in carried.iter().filter(|_| !crate::util::quiet()) {
        println!("{l}");
    }
    if was_current {
        if !crate::util::quiet() {
            eprintln!("Already on '{name}'");
        }
    } else {
        if !crate::util::quiet() {
            eprintln!("Switched to branch '{name}'");
        }
    }
    Ok(())
}

/// `checkout --detach <rev>` / `switch --detach <rev>` with a two-way switch.
fn detach_to(rev: &str) -> CliResult<()> {
    let mut repo = open_repo()?;
    let oid = crate::cmd::shared::peel_to_kind(
        &repo,
        &resolve_rev(&repo, rev)?,
        crate::cmd::shared::ObjKind::Commit,
        rev,
    )?;
    let head = repo.head_oid().ok();
    // The two-way switch also reports carried local changes for a same-commit detach.
    let work = work_tree(&repo)?;
    let carried = crate::cmd::switch_tree::two_way_switch(&repo, &work, head.as_ref(), &oid)?;
    let from = crate::reflog::head_label(&repo);
    repo.refs_mut().update("HEAD", &oid, None)?;
    let _ = crate::reflog::append_reflog(
        &repo,
        "HEAD",
        head.as_ref(),
        &oid,
        &format!("checkout: moving from {from} to {rev}"),
    );
    for l in carried.iter().filter(|_| !crate::util::quiet()) {
        println!("{l}");
    }
    let subject = crate::cmd::log_format::read_commit(&repo, &oid)
        .map(|c| c.subject().to_string())
        .unwrap_or_default();
    if !crate::util::quiet() {
        eprintln!("HEAD is now at {} {subject}", &oid.to_hex()[..7]);
    }
    Ok(())
}

/// git-restore core (also `checkout -- <path>`): copy `<pathspec>` matches
/// from the source (`--source`, else HEAD for `--staged`, else the index)
/// into the index and/or working tree. Matched paths missing from the
/// source are removed from the destination (an added file is unstaged).
/// Cite: <https://git-scm.com/docs/git-restore>.
pub(crate) fn restore_paths(
    paths: &[&String],
    staged: bool,
    worktree: bool,
    source: Option<&str>,
) -> CliResult<()> {
    restore_paths_mode(paths, staged, worktree, source, false)
}

/// [`restore_paths`]; `overlay` (git-checkout default) never removes paths
/// missing from the source.
pub(crate) fn restore_paths_mode(
    paths: &[&String],
    staged: bool,
    worktree: bool,
    source: Option<&str>,
    overlay: bool,
) -> CliResult<()> {
    let repo = open_repo()?;
    let work = work_tree(&repo)?;
    let mut index = read_index(&repo)?;
    let idx_map: HashMap<String, (arks_oid::Oid, u32)> = index
        .entries
        .iter()
        .filter(|e| e.stage() == 0)
        .map(|e| (e.path.clone(), (e.oid, e.mode)))
        .collect();
    let tree_files = |oid: &arks_oid::Oid| -> CliResult<HashMap<String, (arks_oid::Oid, u32)>> {
        let peeled = crate::cmd::shared::peel_tags(&repo, oid)?;
        Ok(crate::cmd::switch_tree::commit_files(&repo, &peeled)?
            .into_iter()
            .map(|(k, f)| (k, (f.oid, f.mode)))
            .collect())
    };
    let src_map = if let Some(rev) = source {
        tree_files(&resolve_rev(&repo, rev)?)?
    } else if staged {
        match repo.head_oid() {
            Ok(h) => tree_files(&h)?,
            Err(_) => HashMap::new(),
        }
    } else {
        idx_map.clone()
    };
    let mut matched: Vec<String> = Vec::new();
    for p in paths {
        let rel = crate::cmd::core::pathspec_rel(&work, p)?;
        let prefix = format!("{rel}/");
        let hit = |k: &String| rel.is_empty() || *k == rel || k.starts_with(&prefix);
        let before = matched.len();
        matched.extend(src_map.keys().filter(|k| hit(k)).cloned());
        let dest_known: Vec<String> = if overlay {
            Vec::new()
        } else if staged {
            idx_map.keys().filter(|k| hit(k)).cloned().collect()
        } else {
            // Work-tree restore from a tree also drops tracked paths absent there.
            idx_map
                .keys()
                .filter(|k| source.is_some() && hit(k))
                .cloned()
                .collect()
        };
        matched.extend(dest_known);
        if matched.len() == before {
            return Err(CliError::Exit(
                1,
                format!("error: pathspec '{p}' did not match any file(s) known to git"),
            ));
        }
    }
    matched.sort();
    matched.dedup();
    for rel in matched {
        let abs = work.join(&rel);
        match src_map.get(&rel) {
            Some((oid, mode)) => {
                if worktree {
                    let blob = repo.get_object(oid)?;
                    if let Some(parent) = abs.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    let payload =
                        arks_remote::smudge_worktree_blob(&repo, &work, &rel, &blob.payload)
                            .map_err(|e| CliError::Msg(e.to_string()))?;
                    fs::write(&abs, &payload)?;
                }
                if staged {
                    let size = repo.get_object(oid)?.payload.len().min(u32::MAX as usize) as u32;
                    index.entries.retain(|e| !(e.path == rel && e.stage() == 0));
                    index.entries.push(IndexEntry {
                        ctime_sec: 0,
                        ctime_nsec: 0,
                        mtime_sec: 0,
                        mtime_nsec: 0,
                        dev: 0,
                        ino: 0,
                        mode: *mode,
                        uid: 0,
                        gid: 0,
                        size,
                        oid: *oid,
                        flags: (rel.len().min(0x0fff)) as u16,
                        flags_extended: None,
                        path: rel,
                    });
                }
            }
            None => {
                if worktree && (abs.is_file() || abs.is_symlink()) {
                    fs::remove_file(&abs)?;
                }
                if staged {
                    index.entries.retain(|e| e.path != rel);
                }
            }
        }
    }
    if staged {
        sort_entries(&mut index.entries);
        write_index(&repo, &index)?;
    }
    Ok(())
}
