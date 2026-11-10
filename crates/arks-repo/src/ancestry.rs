//! First-parent ancestry helpers over the commit graph.
//!
//! Cite: <https://git-scm.com/docs/gitrevisions>
//! Ownership: Ring 4 — graph walk, not CLI porcelain.

use crate::RepoError;
use crate::repo::Repository;
use arks_object::{Commit, ObjectKind};
use arks_oid::Oid;
use std::collections::HashSet;

/// True when `ancestor` appears on the first-parent chain from `tip` (inclusive).
pub fn is_ancestor_first_parent(
    repo: &Repository,
    ancestor: &Oid,
    tip: &Oid,
) -> Result<bool, RepoError> {
    let mut oid = *tip;
    loop {
        if oid == *ancestor {
            return Ok(true);
        }
        let obj = repo.get_object(&oid)?;
        if obj.kind != ObjectKind::Commit {
            return Ok(false);
        }
        let c = Commit::decode(&obj.payload).map_err(RepoError::Object)?;
        match c.parents.first() {
            Some(p) => oid = *p,
            None => return Ok(false),
        }
    }
}

/// All ancestors of `tip` (inclusive), walking every parent edge.
pub fn ancestor_set(repo: &Repository, tip: Oid) -> Result<HashSet<Oid>, RepoError> {
    let mut set = HashSet::new();
    let mut stack = vec![tip];
    while let Some(oid) = stack.pop() {
        if !set.insert(oid) {
            continue;
        }
        let obj = repo.get_object(&oid)?;
        if obj.kind != ObjectKind::Commit {
            continue;
        }
        let commit = Commit::decode(&obj.payload).map_err(RepoError::Object)?;
        stack.extend(commit.parents);
    }
    Ok(set)
}

/// Best-effort merge-base: first commit on `b`'s ancestry (including `b`) that
/// is an ancestor of `a`. Cite: <https://git-scm.com/docs/git-merge-base>
pub fn merge_base_oid(repo: &Repository, a: &Oid, b: &Oid) -> Result<Option<Oid>, RepoError> {
    let a_anc = ancestor_set(repo, *a)?;
    let mut stack = vec![*b];
    let mut seen = HashSet::new();
    while let Some(oid) = stack.pop() {
        if !seen.insert(oid) {
            continue;
        }
        if a_anc.contains(&oid) {
            return Ok(Some(oid));
        }
        let obj = repo.get_object(&oid)?;
        if obj.kind != ObjectKind::Commit {
            continue;
        }
        let commit = Commit::decode(&obj.payload).map_err(RepoError::Object)?;
        stack.extend(commit.parents);
    }
    Ok(None)
}

/// Set of OIDs on the first-parent chain from `tip` (inclusive).
pub fn first_parent_set(repo: &Repository, tip: Oid) -> Result<HashSet<Oid>, RepoError> {
    let mut set = HashSet::new();
    let mut oid = tip;
    loop {
        if !set.insert(oid) {
            break;
        }
        let obj = repo.get_object(&oid)?;
        if obj.kind != ObjectKind::Commit {
            break;
        }
        let commit = Commit::decode(&obj.payload).map_err(RepoError::Object)?;
        match commit.parents.first() {
            Some(parent) => oid = *parent,
            None => break,
        }
    }
    Ok(set)
}
