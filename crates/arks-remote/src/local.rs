//! Local-path transport (plain path or same-host `file://`): list the peer
//! repository's refs and copy missing objects between the two object stores
//! directly, as git's local protocol does for `ls-remote`, `fetch` and
//! `push`. Ref-update policy (refspecs, fast-forward checks) stays with the
//! caller.
//!
//! Primary: <https://git-scm.com/docs/git-fetch#_git_urls> (local paths),
//! <https://git-scm.com/docs/git-ls-remote>.

use crate::error::RemoteError;
use crate::ls_remote::RemoteRef;
use crate::url::LocalRemoteUrl;
use arks_object::{Commit, ObjectKind, Tree, TreeMode};
use arks_oid::Oid;
use arks_repo::Repository;
use arks_store::ObjectStore;
use std::collections::HashSet;

/// True when `url` names a local repository (plain path / same-host `file://`)
/// that exists on disk.
pub fn is_local_remote(url: &str) -> bool {
    LocalRemoteUrl::parse(url).is_ok_and(|l| l.path.exists())
}

/// Open the repository a local remote URL points at (bare or work tree).
pub fn open_local_remote(url: &str) -> Result<Repository, RemoteError> {
    let local = LocalRemoteUrl::parse(url)?;
    Repository::discover(&local.path)
        .map_err(|_| RemoteError::Msg(format!("'{url}' does not appear to be a git repository")))
}

/// Refs as `ls-remote` advertises them: `HEAD` first, then every ref under
/// `refs/` in name order, each annotated tag followed by its peeled
/// `<name>^{}` entry.
pub fn local_refs(repo: &Repository) -> Result<Vec<RemoteRef>, RemoteError> {
    let mut out = Vec::new();
    if let Ok(head) = repo.head_oid() {
        out.push(RemoteRef {
            name: "HEAD".into(),
            oid: head,
        });
    }
    let mut refs = repo
        .refs()
        .list_refs("refs/")
        .map_err(|e| RemoteError::Msg(e.to_string()))?;
    refs.sort_by(|a, b| a.0.cmp(&b.0));
    for (name, oid) in refs {
        out.push(RemoteRef {
            name: name.clone(),
            oid,
        });
        let peeled = peel_tags(repo, &oid);
        if peeled != oid {
            out.push(RemoteRef {
                name: format!("{name}^{{}}"),
                oid: peeled,
            });
        }
    }
    Ok(out)
}

/// Target of an annotated tag payload (`object <hex>` header).
fn tag_target(payload: &[u8]) -> Option<Oid> {
    String::from_utf8_lossy(payload)
        .lines()
        .take_while(|l| !l.is_empty())
        .find_map(|l| l.strip_prefix("object "))
        .and_then(|h| Oid::from_hex(h.trim()).ok())
}

fn peel_tags(repo: &Repository, oid: &Oid) -> Oid {
    let mut cur = *oid;
    while let Ok(obj) = repo.get_object(&cur) {
        match (obj.kind, tag_target(&obj.payload)) {
            (ObjectKind::Tag, Some(next)) => cur = next,
            _ => break,
        }
    }
    cur
}

/// Copy every object reachable from `tips` that `dst` lacks. The walk stops
/// at objects `dst` already has (their closure is assumed present, as for a
/// connected repository). Returns the number of objects written.
pub fn copy_objects(
    src: &Repository,
    dst: &mut Repository,
    tips: &[Oid],
) -> Result<usize, RemoteError> {
    let mut seen = HashSet::new();
    let mut stack: Vec<Oid> = tips.to_vec();
    let mut copied = 0usize;
    let format = dst.object_format();
    while let Some(oid) = stack.pop() {
        if !seen.insert(oid) || dst.get_object(&oid).is_ok() {
            continue;
        }
        let obj = src
            .get_object(&oid)
            .map_err(|_| RemoteError::Protocol(format!("missing object {}", oid.to_hex())))?;
        match obj.kind {
            ObjectKind::Commit => {
                let c = Commit::decode(&obj.payload)
                    .map_err(|e| RemoteError::Protocol(e.to_string()))?;
                stack.push(c.tree);
                stack.extend(c.parents);
            }
            ObjectKind::Tree => {
                let t = Tree::decode(&obj.payload, src.oid_kind())
                    .map_err(|e| RemoteError::Protocol(e.to_string()))?;
                stack.extend(
                    t.entries
                        .iter()
                        .filter(|e| e.mode != TreeMode::Gitlink)
                        .map(|e| e.oid),
                );
            }
            ObjectKind::Tag => stack.extend(tag_target(&obj.payload)),
            ObjectKind::Blob => {}
        }
        dst.store_mut()
            .put(format, &obj)
            .map_err(|e| RemoteError::Msg(e.to_string()))?;
        copied += 1;
    }
    Ok(copied)
}
