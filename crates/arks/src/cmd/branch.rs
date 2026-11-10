//! Branch / checkout / switch.

use crate::cmd::shared::{current_branch_name, head_blob_map, resolve_rev, tree_blob_map};
use crate::error::{CliError, CliResult};
use crate::util::{
    open_repo, read_index, refuse_unknown_flags, rel_path_from_worktree, work_tree, write_index,
};
use arks_index::{IndexEntry, sort_entries};
use arks_refs::RefTarget;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
pub fn cmd_branch(args: &[String]) -> CliResult<()> {
    // M16 Sy-1: `-a`/`--all`, `-r`/`--remotes`, `-v` claimed (git-branch).
    refuse_unknown_flags(
        args,
        &["-d", "-D", "-m", "-a", "--all", "-r", "--remotes", "-v"],
    )?;
    let mut repo = open_repo()?;
    if args.iter().any(|a| a == "-d" || a == "-D") {
        let force = args.iter().any(|a| a == "-D");
        let name = args
            .iter()
            .find(|a| !a.starts_with('-'))
            .ok_or_else(|| CliError::Usage("branch -d|-D <name>".into()))?;
        let full = branch_ref(name);
        if !force {
            let head = current_branch_name(&repo)?;
            if head == *name {
                return Err(CliError::Msg(format!(
                    "cannot delete branch '{name}' checked out"
                )));
            }
        }
        repo.refs_mut().delete(&full, None)?;
        return Ok(());
    }
    if args.iter().any(|a| a == "-m") {
        let positional: Vec<&String> = args.iter().filter(|a| !a.starts_with('-')).collect();
        if positional.len() != 2 {
            return Err(CliError::Usage("branch -m <old> <new>".into()));
        }
        let old = branch_ref(positional[0]);
        let new = branch_ref(positional[1]);
        let oid = repo.refs().resolve(&old)?;
        repo.refs_mut().update(&new, &oid, None)?;
        repo.refs_mut().delete(&old, Some(&oid))?;
        if let Ok(RefTarget::Symbolic(s)) = repo.head_target() {
            if s == old {
                repo.refs_mut().set_symbolic("HEAD", &new)?;
            }
        }
        return Ok(());
    }
    let positional: Vec<&String> = args.iter().filter(|a| !a.starts_with('-')).collect();
    if positional.is_empty() {
        let list_all = args.iter().any(|a| a == "-a" || a == "--all");
        let list_remote = args.iter().any(|a| a == "-r" || a == "--remotes");
        let verbose = args.iter().any(|a| a == "-v");
        let cur = current_branch_name(&repo).unwrap_or_default();
        if !list_remote {
            let mut refs = repo.refs().list_refs("refs/heads/")?;
            refs.sort_by(|a, b| a.0.cmp(&b.0));
            for (name, oid) in refs {
                let short = name.strip_prefix("refs/heads/").unwrap_or(&name);
                let mark = if short == cur { "*" } else { " " };
                if verbose {
                    println!("{mark} {short} {}", &oid.to_hex()[..7]);
                } else {
                    println!("{mark} {short}");
                }
            }
        }
        if list_all || list_remote {
            let mut remotes = repo.refs().list_refs("refs/remotes/")?;
            remotes.sort_by(|a, b| a.0.cmp(&b.0));
            for (name, oid) in remotes {
                let short = name.strip_prefix("refs/remotes/").unwrap_or(&name);
                if verbose {
                    println!("  remotes/{short} {}", &oid.to_hex()[..7]);
                } else if list_remote && !list_all {
                    println!("  {short}");
                } else {
                    println!("  remotes/{short}");
                }
            }
        }
        return Ok(());
    }
    if positional.len() != 1 {
        return Err(CliError::Usage("branch [<name>]".into()));
    }
    let name = positional[0];
    let full = branch_ref(name);
    let oid = repo.head_oid()?;
    repo.refs_mut().update(&full, &oid, None)?;
    Ok(())
}

pub fn cmd_checkout(args: &[String]) -> CliResult<()> {
    // M16 Sy-1/Sy-2: `--detach` + `--orphan` claimed (git-checkout 2.39.0).
    // Cite: https://git-scm.com/docs/git-checkout/2.39.0
    refuse_unknown_flags(args, &["-b", "--", "--detach", "--orphan"])?;
    let create = args.iter().any(|a| a == "-b");
    let detach = args.iter().any(|a| a == "--detach");
    let orphan_idx = args.iter().position(|a| a == "--orphan");
    let dashdash = args.iter().position(|a| a == "--");
    if let Some(i) = dashdash {
        let paths: Vec<&String> = args[i + 1..].iter().collect();
        if paths.is_empty() {
            return Err(CliError::Usage("checkout -- <path>...".into()));
        }
        return restore_paths(&paths, false, true, None);
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
        if positional.len() != 1 {
            return Err(CliError::Usage("checkout -b <branch>".into()));
        }
        let name = positional[0];
        let mut repo = open_repo()?;
        let oid = repo.head_oid()?;
        let full = branch_ref(name);
        repo.refs_mut().update(&full, &oid, None)?;
        repo.refs_mut().set_symbolic("HEAD", &full)?;
        let work = work_tree(&repo)?;
        arks_remote::checkout_commit(&repo, &work, &oid)?;
        println!("Switched to a new branch '{name}'");
        return Ok(());
    }
    if detach {
        if positional.len() != 1 {
            return Err(CliError::Usage("checkout --detach <commit>".into()));
        }
        let mut repo = open_repo()?;
        let oid = resolve_rev(&repo, positional[0])?;
        repo.refs_mut().update("HEAD", &oid, None)?;
        let work = work_tree(&repo)?;
        arks_remote::checkout_commit(&repo, &work, &oid)?;
        println!("HEAD is now at {}", &oid.to_hex()[..7]);
        return Ok(());
    }
    if positional.len() != 1 {
        return Err(CliError::Usage(
            "checkout <branch> | checkout -- <path>".into(),
        ));
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
    refuse_unknown_flags(args, &["-c", "-C", "--detach", "--orphan"])?;
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
    if positional.len() != 1 {
        return Err(CliError::Usage(
            "switch [-c|-C|--detach|--orphan] <branch|commit>".into(),
        ));
    }
    let name = positional[0];
    if detach {
        let mut repo = open_repo()?;
        let oid = resolve_rev(&repo, name)?;
        repo.refs_mut().update("HEAD", &oid, None)?;
        let work = work_tree(&repo)?;
        arks_remote::checkout_commit(&repo, &work, &oid)?;
        println!("HEAD is now at {}", &oid.to_hex()[..7]);
        return Ok(());
    }
    if create {
        let mut repo = open_repo()?;
        let oid = repo.head_oid()?;
        let full = branch_ref(name);
        if force_create {
            let _ = repo.refs_mut().delete(&full, None);
        }
        repo.refs_mut().update(&full, &oid, None)?;
        repo.refs_mut().set_symbolic("HEAD", &full)?;
        let work = work_tree(&repo)?;
        arks_remote::checkout_commit(&repo, &work, &oid)?;
        println!("Switched to a new branch '{name}'");
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

fn branch_ref(name: &str) -> String {
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
    // TOCTOU-003 (M19 Cx-2): refuse checkout that would overwrite dirty worktree edits.
    if !(same_oid && index_populated) && crate::cmd::tag::worktree_dirty(&repo)? {
        return Err(CliError::Msg(
            "Your local changes would be overwritten by checkout (TOCTOU-003); commit, stash, or restore first"
                .into(),
        ));
    }
    repo.refs_mut().set_symbolic("HEAD", &full)?;
    if !(same_oid && index_populated) {
        let work = work_tree(&repo)?;
        arks_remote::checkout_commit(&repo, &work, &oid)?;
    }
    println!("Switched to branch '{name}'");
    Ok(())
}

pub(crate) fn restore_paths(
    paths: &[&String],
    staged: bool,
    worktree: bool,
    source: Option<&str>,
) -> CliResult<()> {
    let repo = open_repo()?;
    let work = work_tree(&repo)?;
    let mut index = read_index(&repo)?;
    let src_map = if let Some(rev) = source {
        let oid = resolve_rev(&repo, rev)?;
        tree_blob_map(&repo, &oid)?
    } else if staged {
        // restore --staged defaults from HEAD
        head_blob_map(&repo)?
    } else {
        // restore worktree from index
        let mut m = HashMap::new();
        for e in index.entries.iter().filter(|e| e.stage() == 0) {
            m.insert(e.path.clone(), e.oid);
        }
        m
    };
    for p in paths {
        let rel = rel_path_from_worktree(&work, Path::new(p)).unwrap_or_else(|_| (*p).clone());
        let oid = src_map
            .get(&rel)
            .copied()
            .ok_or_else(|| CliError::Msg(format!("path '{rel}' not in source")))?;
        let blob = repo.get_object(&oid)?;
        if worktree {
            let abs = work.join(&rel);
            if let Some(parent) = abs.parent() {
                fs::create_dir_all(parent)?;
            }
            let payload = arks_remote::smudge_worktree_blob(&repo, &work, &rel, &blob.payload)
                .map_err(|e| CliError::Msg(e.to_string()))?;
            fs::write(&abs, &payload)?;
        }
        if staged {
            let meta_size = blob.payload.len().min(u32::MAX as usize) as u32;
            index.entries.retain(|e| !(e.path == rel && e.stage() == 0));
            index.entries.push(IndexEntry {
                ctime_sec: 0,
                ctime_nsec: 0,
                mtime_sec: 0,
                mtime_nsec: 0,
                dev: 0,
                ino: 0,
                mode: 0o100644,
                uid: 0,
                gid: 0,
                size: meta_size,
                oid,
                flags: (rel.len().min(0x0fff)) as u16,
                flags_extended: None,
                path: rel,
            });
        }
    }
    if staged {
        sort_entries(&mut index.entries);
        write_index(&repo, &index)?;
    }
    Ok(())
}
