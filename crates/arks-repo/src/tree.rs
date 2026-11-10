//! Flatten trees to path→blob maps (repo graph helpers).
//!
//! Cite: <https://git-scm.com/docs/gitformat-loose>
//! Ownership: Ring 4 repo facade — not CLI porcelain.

use crate::RepoError;
use crate::repo::Repository;
use arks_object::{Commit, ObjectKind, Tree};
use arks_oid::{Oid, OidKind};
use std::collections::HashMap;

/// Recursively flatten a tree into path → blob OID (non-tree entries only).
pub fn flatten_tree(
    repo: &Repository,
    tree_oid: &Oid,
    prefix: &str,
    out: &mut HashMap<String, Oid>,
) -> Result<(), RepoError> {
    let mut entries = Vec::new();
    flatten_tree_entries(repo, tree_oid, prefix, &mut entries)?;
    for (path, oid) in entries {
        out.insert(path, oid);
    }
    Ok(())
}

/// Flatten a tree into `(path, blob_oid)` pairs (Pf4-2 ARCHIVE: avoid HashMap then re-key).
pub fn flatten_tree_entries(
    repo: &Repository,
    tree_oid: &Oid,
    prefix: &str,
    out: &mut Vec<(String, Oid)>,
) -> Result<(), RepoError> {
    let obj = repo.get_object(tree_oid)?;
    let tree = Tree::decode(&obj.payload, OidKind::Sha1).map_err(RepoError::Object)?;
    for e in tree.entries {
        let path = if prefix.is_empty() {
            e.name.clone()
        } else {
            let mut path = String::with_capacity(prefix.len() + 1 + e.name.len());
            path.push_str(prefix);
            path.push('/');
            path.push_str(&e.name);
            path
        };
        if e.mode.is_tree() {
            flatten_tree_entries(repo, &e.oid, &path, out)?;
        } else {
            out.push((path, e.oid));
        }
    }
    Ok(())
}

/// Blob map for a commit tip (HEAD tree) or empty when HEAD is missing/non-commit.
pub fn head_blob_map(repo: &Repository) -> Result<HashMap<String, Oid>, RepoError> {
    let mut map = HashMap::new();
    let Ok(oid) = repo.head_oid() else {
        return Ok(map);
    };
    let obj = repo.get_object(&oid)?;
    if obj.kind != ObjectKind::Commit {
        return Ok(map);
    }
    let c = Commit::decode(&obj.payload).map_err(RepoError::Object)?;
    flatten_tree(repo, &c.tree, "", &mut map)?;
    Ok(map)
}

/// Blob map for a commit or tree OID.
pub fn tree_blob_map(repo: &Repository, rev_oid: &Oid) -> Result<HashMap<String, Oid>, RepoError> {
    let obj = repo.get_object(rev_oid)?;
    let tree_oid = match obj.kind {
        ObjectKind::Commit => {
            Commit::decode(&obj.payload)
                .map_err(RepoError::Object)?
                .tree
        }
        ObjectKind::Tree => *rev_oid,
        _ => {
            return Err(RepoError::Io(format!(
                "{} is not a commit or tree",
                rev_oid.to_hex()
            )));
        }
    };
    let mut map = HashMap::new();
    flatten_tree(repo, &tree_oid, "", &mut map)?;
    Ok(map)
}

/// Blob map for a commit's tree (commit OID required).
pub fn commit_tree_map(
    repo: &Repository,
    commit_oid: &Oid,
) -> Result<HashMap<String, Oid>, RepoError> {
    let obj = repo.get_object(commit_oid)?;
    if obj.kind != ObjectKind::Commit {
        return Err(RepoError::Io(format!(
            "{} is not a commit",
            commit_oid.to_hex()
        )));
    }
    let commit = Commit::decode(&obj.payload).map_err(RepoError::Object)?;
    let mut map = HashMap::new();
    flatten_tree(repo, &commit.tree, "", &mut map)?;
    Ok(map)
}
