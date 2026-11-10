//! Minimal stash (push/list/pop).

use crate::cmd::shared::commit_signature;
use crate::error::{CliError, CliResult};
use crate::util::{FORMAT, open_repo, read_index, refuse_unknown_flags, work_tree, write_index};
use crate::write_tree::write_tree_from_index;
use arks_index::Index;
use arks_object::{Commit, ObjectKind, RawObject};
use arks_store::ObjectStore;
use std::fs;
pub fn cmd_stash(args: &[String]) -> CliResult<()> {
    // M16 Sy-5: show/apply/drop/clear claimed (git-stash). `-p` interactive remains N/A.
    let sub = args.first().map(String::as_str).unwrap_or("push");
    match sub {
        "push" | "save" => {
            refuse_unknown_flags(&args[1.min(args.len())..], &[])?;
            stash_push()
        }
        "list" => {
            refuse_unknown_flags(&args.get(1..).unwrap_or(&[]), &[])?;
            stash_list()
        }
        "pop" => {
            refuse_unknown_flags(&args.get(1..).unwrap_or(&[]), &[])?;
            stash_pop()
        }
        "show" => {
            refuse_unknown_flags(&args.get(1..).unwrap_or(&[]), &[])?;
            stash_show()
        }
        "apply" => {
            refuse_unknown_flags(&args.get(1..).unwrap_or(&[]), &[])?;
            stash_apply(false)
        }
        "drop" => {
            refuse_unknown_flags(&args.get(1..).unwrap_or(&[]), &[])?;
            stash_drop()
        }
        "clear" => {
            refuse_unknown_flags(&args.get(1..).unwrap_or(&[]), &[])?;
            stash_clear()
        }
        other => Err(CliError::NotImplemented(format!(
            "stash {other} not claimed (Sy-5: push/list/pop/show/apply/drop/clear; -p N/A)"
        ))),
    }
}

fn stash_push() -> CliResult<()> {
    let mut repo = open_repo()?;
    // STASH-001 (M19 Cx-2): concurrent stash push contends on refs/stash lock.
    if repo.git_dir().join("refs").join("stash.lock").is_file() {
        return Err(CliError::Msg(
            "Unable to create refs/stash.lock (STASH-001 — concurrent stash push fail-closed; lease ≠ skip)"
                .into(),
        ));
    }
    let work = work_tree(&repo)?;
    let index = read_index(&repo)?;
    let tree = if index.entries.is_empty() {
        let obj = RawObject::new(ObjectKind::Tree, Vec::new());
        repo.store_mut().put(FORMAT, &obj)?
    } else {
        write_tree_from_index(repo.store_mut(), &index.entries)?
    };
    // Also capture dirty worktree into index temporarily for stash tree.
    let mut dirty = index.clone();
    for e in index.entries.iter().filter(|e| e.stage() == 0) {
        let abs = work.join(&e.path);
        if abs.is_file() {
            let bytes = fs::read(&abs)?;
            let obj = RawObject::new(ObjectKind::Blob, bytes.clone());
            let oid = repo.store_mut().put(FORMAT, &obj)?;
            if let Some(ent) = dirty.entries.iter_mut().find(|x| x.path == e.path) {
                ent.oid = oid;
                ent.size = bytes.len().min(u32::MAX as usize) as u32;
            }
        }
    }
    let wtree = write_tree_from_index(repo.store_mut(), &dirty.entries)?;
    let parent = repo.head_oid().ok();
    let sig = commit_signature()?;
    let msg = "WIP on stash\n".to_string();
    let commit = Commit {
        tree: wtree,
        parents: parent.into_iter().collect(),
        author: sig.clone(),
        committer: sig,
        gpgsig: None,
        message: msg,
    };
    let cobj = RawObject::new(ObjectKind::Commit, commit.encode()?);
    let coid = repo.store_mut().put(FORMAT, &cobj)?;
    // Stack: keep previous stash as parent chain via refs/stash only (single tip).
    repo.refs_mut().update("refs/stash", &coid, None)?;
    // Reset WT+index to HEAD tree (like stash push).
    if let Some(h) = parent {
        arks_remote::checkout_commit(&repo, &work, &h)?;
    } else {
        write_index(
            &repo,
            &Index {
                version: 2,
                entries: Vec::new(),
                extensions: Vec::new(),
            },
        )?;
    }
    let _ = tree;
    println!("Saved working directory and index state WIP");
    Ok(())
}

fn stash_list() -> CliResult<()> {
    let repo = open_repo()?;
    if let Ok(oid) = repo.refs().resolve("refs/stash") {
        println!("stash@{{0}}: WIP on {}", &oid.to_hex()[..7]);
    }
    Ok(())
}

fn stash_pop() -> CliResult<()> {
    stash_apply(true)
}

fn stash_show() -> CliResult<()> {
    let repo = open_repo()?;
    let oid = repo
        .refs()
        .resolve("refs/stash")
        .map_err(|_| CliError::Msg("No stash entries found.".into()))?;
    let obj = repo.get_object(&oid)?;
    let c = Commit::decode(&obj.payload)?;
    println!("stash@{{0}}: {}", c.subject());
    Ok(())
}

fn stash_apply(drop_after: bool) -> CliResult<()> {
    let mut repo = open_repo()?;
    // STASH-002 (M19 Cx-2): stash pop/apply against moving index — honor index.lock.
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
    let oid = repo
        .refs()
        .resolve("refs/stash")
        .map_err(|_| CliError::Msg("No stash entries found.".into()))?;
    let work = work_tree(&repo)?;
    arks_remote::checkout_commit(&repo, &work, &oid)?;
    if drop_after {
        repo.refs_mut().delete("refs/stash", Some(&oid))?;
        println!("Dropped refs/stash");
    } else {
        println!("Applied refs/stash");
    }
    Ok(())
}

fn stash_drop() -> CliResult<()> {
    let mut repo = open_repo()?;
    let oid = repo
        .refs()
        .resolve("refs/stash")
        .map_err(|_| CliError::Msg("No stash entries found.".into()))?;
    repo.refs_mut().delete("refs/stash", Some(&oid))?;
    println!("Dropped refs/stash {{{}}}", &oid.to_hex()[..7]);
    Ok(())
}

fn stash_clear() -> CliResult<()> {
    let mut repo = open_repo()?;
    if let Ok(oid) = repo.refs().resolve("refs/stash") {
        let _ = repo.refs_mut().delete("refs/stash", Some(&oid));
    }
    Ok(())
}
