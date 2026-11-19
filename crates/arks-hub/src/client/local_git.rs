//! In-process Git helpers for forge leaves that touch a local checkout
//! (`repo clone`, `repo sync`, `issue develop --checkout`). arks crates only —
//! never spawns git/gh (rule 30).
//!
//! Semantics cited from git-scm primary docs:
//! - fast-forward: <https://git-scm.com/docs/git-merge#_fast_forward_merge>
//! - clean-tree requirement mirrors `git status --untracked-files=no`
//!   (<https://git-scm.com/docs/git-status>): staged or unstaged changes to
//!   tracked paths make the tree dirty; untracked files do not.

use arks_index::Index;
use arks_object::{ObjectKind, RawObject};
use arks_oid::Oid;
use arks_refs::RefTarget;
use arks_repo::Repository;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Clone `url` into `dest` choosing the transport the way `arks clone` does.
pub fn clone_url(url: &str, dest: &Path, depth: Option<u32>) -> Result<(), String> {
    let opts = arks_remote::FetchOptions {
        depth,
        ..Default::default()
    };
    let r = if arks_remote::HttpRemoteUrl::parse(url).is_ok() {
        arks_remote::clone_http_with(url, dest, &opts)
    } else if arks_remote::SshRemoteUrl::parse(url).is_ok() {
        arks_remote::clone_ssh_with(url, dest, &opts)
    } else if arks_remote::GitDaemonUrl::parse(url).is_ok() {
        arks_remote::clone_git_with(url, dest, &opts)
    } else if arks_remote::LocalRemoteUrl::parse(url).is_ok() {
        arks_remote::clone_local_with(url, dest, &opts)
    } else {
        return Err(format!(
            "unsupported clone URL '{url}' (http(s), ssh://, user@host:path, git://, local path)"
        ));
    };
    r.map_err(|e| e.to_string())
}

/// Refuse a non-empty destination (git-clone: "already exists and is not an empty directory").
pub fn check_clone_dest(dest: &Path) -> Result<(), String> {
    if !dest.exists() {
        return Ok(());
    }
    if !dest.is_dir() {
        return Err(format!(
            "destination path '{}' already exists and is not a directory",
            dest.display()
        ));
    }
    let mut it = fs::read_dir(dest).map_err(|e| e.to_string())?;
    if it.next().is_some() {
        return Err(format!(
            "destination path '{}' already exists and is not an empty directory",
            dest.display()
        ));
    }
    Ok(())
}

/// Current branch short name when HEAD is symbolic.
pub fn current_branch(repo: &Repository) -> Option<String> {
    match repo.head_target().ok()? {
        RefTarget::Symbolic(s) => Some(s.trim_start_matches("refs/heads/").to_string()),
        RefTarget::Direct(_) => None,
    }
}

/// `true` when tracked paths differ between HEAD, the index and the worktree.
pub fn is_dirty(repo: &Repository) -> Result<bool, String> {
    let work = repo
        .work_tree()
        .ok_or_else(|| "bare repository has no work tree".to_string())?
        .to_path_buf();
    let head = arks_repo::head_blob_map(repo).map_err(|e| e.to_string())?;
    let index_path = repo.index_path();
    let entries = if index_path.is_file() {
        Index::read_path(&index_path, repo.oid_kind())
            .map_err(|e| e.to_string())?
            .entries
    } else {
        Vec::new()
    };
    let mut staged: HashMap<String, Oid> = HashMap::with_capacity(entries.len());
    for e in &entries {
        if e.stage() != 0 {
            return Ok(true);
        }
        staged.insert(e.path.clone(), e.oid);
    }
    if staged.len() != head.len() || staged.iter().any(|(p, o)| head.get(p) != Some(o)) {
        return Ok(true);
    }
    for e in &entries {
        // Gitlinks (160000) are not materialized as files.
        if e.mode & 0o170000 == 0o160000 {
            continue;
        }
        let p = work.join(&e.path);
        let bytes = match fs::symlink_metadata(&p) {
            Ok(m) if m.file_type().is_symlink() => fs::read_link(&p)
                .map(|t| t.to_string_lossy().into_owned().into_bytes())
                .map_err(|e| e.to_string())?,
            Ok(m) if m.is_file() => fs::read(&p).map_err(|e| e.to_string())?,
            _ => return Ok(true),
        };
        let oid = RawObject::new(ObjectKind::Blob, bytes)
            .oid(repo.object_format())
            .map_err(|e| e.to_string())?;
        if oid != e.oid {
            return Ok(true);
        }
    }
    Ok(false)
}

/// `true` when `ancestor` is reachable from `tip` (fast-forward possible).
pub fn is_ancestor(repo: &Repository, ancestor: &Oid, tip: &Oid) -> Result<bool, String> {
    if ancestor == tip {
        return Ok(true);
    }
    let set = arks_repo::ancestor_set(repo, *tip).map_err(|e| e.to_string())?;
    Ok(set.contains(ancestor))
}

/// Move `refs/heads/<branch>` to `new`; when it is the checked-out branch,
/// rewrite the worktree + index (tracked paths only) to match.
pub fn move_branch(repo: &mut Repository, branch: &str, new: &Oid) -> Result<(), String> {
    let full = format!("refs/heads/{branch}");
    let old = repo.refs().resolve(&full).ok();
    if current_branch(repo).as_deref() == Some(branch) {
        let work = repo
            .work_tree()
            .ok_or_else(|| "bare repository has no work tree".to_string())?
            .to_path_buf();
        arks_remote::checkout_commit(repo, &work, new).map_err(|e| e.to_string())?;
    }
    repo.refs_mut()
        .update(&full, new, old.as_ref())
        .map_err(|e| e.to_string())
}

/// Fetch every branch of `remote` with the in-process client.
pub fn fetch(repo: &mut Repository, remote: &str) -> Result<arks_remote::FetchOutcome, String> {
    let cfg = repo.git_dir().join("config");
    let r = arks_remote::get_remote(&cfg, remote)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("no such remote `{remote}`"))?;
    if arks_remote::LocalRemoteUrl::parse(&r.url).is_ok()
        && arks_remote::HttpRemoteUrl::parse(&r.url).is_err()
        && arks_remote::SshRemoteUrl::parse(&r.url).is_err()
        && arks_remote::GitDaemonUrl::parse(&r.url).is_err()
    {
        return Err(format!(
            "remote `{remote}` is a local path ({}); fetch is claimed for http(s), ssh and git:// transports only",
            r.url
        ));
    }
    arks_remote::fetch_remote_with(repo, &r.url, remote, &arks_remote::FetchOptions::default())
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clone_dest_rules() {
        let d = tempfile::tempdir().unwrap();
        assert!(check_clone_dest(&d.path().join("missing")).is_ok());
        assert!(check_clone_dest(d.path()).is_ok());
        fs::write(d.path().join("f"), b"x").unwrap();
        assert!(check_clone_dest(d.path()).is_err());
        assert!(check_clone_dest(&d.path().join("f")).is_err());
    }
}
