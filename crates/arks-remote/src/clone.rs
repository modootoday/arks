//! Clone composition: init layout, fetch/copy, baseline checkout of the HEAD tree.
//!
//! Non-bare only. The index is written so later `add` / `status` / `commit`
//! see the checked-out files. Symlink blobs are materialized as file bytes
//! with the symlink index mode; gitlinks are refused.
//!
//! Local path / same-host `file://` prefers hardlinks for object files when the
//! filesystem allows (git-clone local default), else copies. Pack files are
//! left in place; checkout reads through the pack-aware store. Not `--shared`
//! alternates; not piped upload-pack.

use crate::config::add_remote;
use crate::error::RemoteError;
use crate::fetch::{FetchOptions, fetch_http_with, fetch_ssh_with};
use crate::url::{HttpRemoteUrl, LocalRemoteUrl, SshRemoteUrl};
use arks_refs::RefTarget;
use arks_repo::Repository;
use std::fs;
use std::path::{Path, PathBuf};

const CORE_CONFIG: &[u8] = b"\
[core]
\trepositoryformatversion = 0
\tfilemode = true
\tbare = false
\tlogallrefupdates = true
";

pub fn clone_http(url: &str, dest: &Path) -> Result<(), RemoteError> {
    clone_http_with(url, dest, &FetchOptions::default())
}

pub fn clone_http_with(url: &str, dest: &Path, opts: &FetchOptions) -> Result<(), RemoteError> {
    HttpRemoteUrl::parse(url)?;
    let mut opts = opts.clone();
    // RAM-first: decode+cache, checkout, then same-thread pack+idx persist.
    // Uses deferred_owned (no pack.to_vec). Never spawn a persist thread —
    // Docker multi-CPU ads made overlap thrash J-CLONE-LOCAL (~0.74).
    opts.defer_pack_disk = true;
    clone_via_fetch(url, dest, move |repo, url| {
        fetch_http_with(repo, url, "origin", &opts)
    })
}

/// Clone over OpenSSH (`ssh://` or SCP-like `git@host:path`).
pub fn clone_ssh(url: &str, dest: &Path) -> Result<(), RemoteError> {
    clone_ssh_with(url, dest, &FetchOptions::default())
}

pub fn clone_ssh_with(url: &str, dest: &Path, opts: &FetchOptions) -> Result<(), RemoteError> {
    SshRemoteUrl::parse(url)?;
    let mut opts = opts.clone();
    opts.defer_pack_disk = true;
    clone_via_fetch(url, dest, move |repo, url| {
        fetch_ssh_with(repo, url, "origin", &opts)
    })
}

fn clone_via_fetch<F>(url: &str, dest: &Path, mut do_fetch: F) -> Result<(), RemoteError>
where
    F: FnMut(&mut Repository, &str) -> Result<crate::fetch::FetchOutcome, RemoteError>,
{
    prepare_dest(dest)?;
    let git_dir = dest.join(".git");
    init_git_dir(&git_dir)?;
    let config = git_dir.join("config");
    add_remote(&config, "origin", url.trim())?;

    let mut repo = Repository::open(&git_dir).map_err(|e| RemoteError::Msg(e.to_string()))?;
    let outcome = do_fetch(&mut repo, url)?;
    // Checkout from RAM pack-cache first; persist pack+idx after (no overlap thread).
    let pending = repo.store_mut().take_pending_pack();
    if let (Some(branch), Some(oid)) = (outcome.head_branch.clone(), outcome.head_oid) {
        repo.refs_mut()
            .update(&branch, &oid, None)
            .map_err(|e| RemoteError::Msg(e.to_string()))?;
        repo.refs_mut()
            .set_symbolic("HEAD", &branch)
            .map_err(|e| RemoteError::Msg(e.to_string()))?;
        crate::checkout::checkout_commit(&repo, dest, &oid)?;
    }
    if let Some(pending) = pending {
        pending
            .persist()
            .map_err(|e| RemoteError::Msg(format!("pack persist: {e}")))?;
    }
    Ok(())
}

/// Clone from a local filesystem path or same-host `file://` URL.
///
/// Claimed subset: recursive `objects/` hardlink-or-copy (packs kept;
/// pack-aware store), heads/tags + `origin` tracking, baseline worktree
/// checkout. Cite git-clone(1) local form (hardlinks when possible).
/// `--depth` is refused (no pack-protocol deepen on this path).
pub fn clone_local(url: &str, dest: &Path) -> Result<(), RemoteError> {
    clone_local_with(url, dest, &FetchOptions::default())
}

pub fn clone_local_with(url: &str, dest: &Path, opts: &FetchOptions) -> Result<(), RemoteError> {
    if opts.depth.is_some() {
        return Err(RemoteError::NotImplemented(
            "clone --depth is not claimed for local path / file:// (pack-protocol deepen only)"
                .into(),
        ));
    }
    let local = LocalRemoteUrl::parse(url)?;
    let source = Repository::discover(&local.path)
        .map_err(|e| RemoteError::Msg(format!("local clone source: {e}")))?;
    prepare_dest(dest)?;
    let git_dir = dest.join(".git");
    init_git_dir(&git_dir)?;
    let config = git_dir.join("config");
    add_remote(&config, "origin", &local.raw)?;

    copy_dir_recursive(source.git_dir().join("objects"), git_dir.join("objects"))?;

    let mut repo = Repository::open(&git_dir).map_err(|e| RemoteError::Msg(e.to_string()))?;
    let head_branch = match source.head_target() {
        Ok(RefTarget::Symbolic(name)) if name.starts_with("refs/heads/") => Some(name),
        Ok(RefTarget::Direct(_)) => None,
        Ok(RefTarget::Symbolic(_)) => None,
        Err(_) => None,
    };
    let head_oid = source
        .head_oid()
        .map_err(|e| RemoteError::Msg(format!("local clone HEAD: {e}")))?;

    let refs = source
        .refs()
        .list_refs("refs/")
        .map_err(|e| RemoteError::Msg(e.to_string()))?;
    for (name, oid) in &refs {
        if name.starts_with("refs/heads/") || name.starts_with("refs/tags/") {
            repo.refs_mut()
                .update(name, oid, None)
                .map_err(|e| RemoteError::Msg(e.to_string()))?;
        }
        if let Some(rest) = name.strip_prefix("refs/heads/") {
            let tracking = format!("refs/remotes/origin/{rest}");
            repo.refs_mut()
                .update(&tracking, oid, None)
                .map_err(|e| RemoteError::Msg(e.to_string()))?;
        }
    }

    let branch = if let Some(b) = head_branch {
        repo.refs_mut()
            .update(&b, &head_oid, None)
            .map_err(|e| RemoteError::Msg(e.to_string()))?;
        repo.refs_mut()
            .set_symbolic("HEAD", &b)
            .map_err(|e| RemoteError::Msg(e.to_string()))?;
        if let Some(rest) = b.strip_prefix("refs/heads/") {
            let tracking = format!("refs/remotes/origin/{rest}");
            repo.refs_mut()
                .set_symbolic("refs/remotes/origin/HEAD", &tracking)
                .map_err(|e| RemoteError::Msg(e.to_string()))?;
        }
        b
    } else {
        let fallback = "refs/heads/main".to_string();
        repo.refs_mut()
            .update(&fallback, &head_oid, None)
            .map_err(|e| RemoteError::Msg(e.to_string()))?;
        repo.refs_mut()
            .set_symbolic("HEAD", &fallback)
            .map_err(|e| RemoteError::Msg(e.to_string()))?;
        fallback
    };
    let _ = branch;
    crate::checkout::checkout_commit(&repo, dest, &head_oid)?;
    Ok(())
}

fn init_git_dir(git_dir: &Path) -> Result<(), RemoteError> {
    fs::create_dir_all(git_dir.join("objects")).map_err(|e| RemoteError::Msg(e.to_string()))?;
    fs::create_dir_all(git_dir.join("refs/heads")).map_err(|e| RemoteError::Msg(e.to_string()))?;
    fs::create_dir_all(git_dir.join("refs/tags")).map_err(|e| RemoteError::Msg(e.to_string()))?;
    fs::write(git_dir.join("HEAD"), b"ref: refs/heads/main\n")
        .map_err(|e| RemoteError::Msg(e.to_string()))?;
    fs::write(git_dir.join("config"), CORE_CONFIG).map_err(|e| RemoteError::Msg(e.to_string()))?;
    Ok(())
}

fn prepare_dest(dest: &Path) -> Result<(), RemoteError> {
    if dest.exists() {
        if !dest.is_dir() {
            return Err(RemoteError::Msg(format!(
                "destination path '{}' exists and is not a directory",
                dest.display()
            )));
        }
        let mut entries = fs::read_dir(dest).map_err(|e| RemoteError::Msg(e.to_string()))?;
        if entries.next().is_some() {
            // NET-009 (M20 Cx-3): concurrent clone of same URL to shared dest.
            // Cite: https://git-scm.com/docs/git-clone
            return Err(RemoteError::Msg(format!(
                "destination path '{}' exists and is not an empty directory (NET-009 — concurrent clone dest collision refuse)",
                dest.display()
            )));
        }
    } else {
        fs::create_dir_all(dest).map_err(|e| RemoteError::Msg(e.to_string()))?;
    }
    Ok(())
}

fn copy_dir_recursive(src: PathBuf, dst: PathBuf) -> Result<(), RemoteError> {
    if !src.is_dir() {
        return Err(RemoteError::Msg(format!(
            "missing objects directory '{}'",
            src.display()
        )));
    }
    fs::create_dir_all(&dst).map_err(|e| RemoteError::Msg(e.to_string()))?;
    for entry in fs::read_dir(&src).map_err(|e| RemoteError::Msg(e.to_string()))? {
        let entry = entry.map_err(|e| RemoteError::Msg(e.to_string()))?;
        let ty = entry
            .file_type()
            .map_err(|e| RemoteError::Msg(e.to_string()))?;
        let to = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive(entry.path(), to)?;
        } else if ty.is_file() {
            // Parent `dst` already created above (Pf4-2: skip per-file create_dir_all).
            hardlink_or_copy(&entry.path(), &to)?;
        }
    }
    Ok(())
}

/// Prefer hardlink (git-clone local default); fall back to byte copy.
fn hardlink_or_copy(from: &Path, to: &Path) -> Result<(), RemoteError> {
    match fs::hard_link(from, to) {
        Ok(()) => Ok(()),
        Err(_) => fs::copy(from, to)
            .map(|_| ())
            .map_err(|e| RemoteError::Msg(e.to_string())),
    }
}
