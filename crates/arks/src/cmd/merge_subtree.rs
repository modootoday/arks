//! Subtree shifting for `merge-subtree`, `git merge -s subtree` and
//! `-X subtree[=<path>]`.
//!
//! Cite: <https://git-scm.com/docs/git-merge#Documentation/git-merge.txt-subtree>
//! ("if B corresponds to a subtree of A, B is first adjusted to match the
//! tree structure of A ... This adjustment is also done to the common
//! ancestor tree") and `-X subtree=<path>` ("the specified path is prefixed
//! (or stripped from the beginning) to make the shape of two trees to match").
//!
//! The guess compares the top-level entries of two trees. Matching entries
//! (same name and object) score up, differing ones score down a little, and
//! a name present on one side only scores down, a directory much more than a
//! file (a whole missing directory is the strongest sign the trees are not
//! aligned). Every directory of ours (and of theirs, for the opposite shift)
//! is a candidate; the best candidate wins when it scores higher than the
//! unshifted trees.

use crate::error::CliResult;
use arks_object::{ObjectKind, RawObject, Tree, TreeEntry, TreeMode};
use arks_oid::Oid;
use arks_repo::Repository;
use arks_store::ObjectStore;
use std::collections::BTreeMap;

const MAX_DEPTH: usize = 8;

fn entries(repo: &Repository, tree: &Oid) -> CliResult<Vec<TreeEntry>> {
    let obj = repo.get_object(tree)?;
    if obj.kind != ObjectKind::Tree {
        return Ok(Vec::new());
    }
    Ok(Tree::decode(&obj.payload, repo.oid_kind())?.entries)
}

fn score(repo: &Repository, a: &Oid, b: &Oid) -> CliResult<i64> {
    let ea: BTreeMap<String, TreeEntry> = entries(repo, a)?
        .into_iter()
        .map(|e| (e.name.clone(), e))
        .collect();
    let eb: BTreeMap<String, TreeEntry> = entries(repo, b)?
        .into_iter()
        .map(|e| (e.name.clone(), e))
        .collect();
    let is_dir = |e: &TreeEntry| e.mode == TreeMode::Tree;
    let missing = |e: &TreeEntry| if is_dir(e) { -40 } else { -2 };
    let mut s = 0i64;
    for (name, x) in &ea {
        s += match eb.get(name) {
            Some(y) if x.oid == y.oid && is_dir(x) => 40,
            Some(y) if x.oid == y.oid && x.mode == y.mode => 10,
            Some(y) if is_dir(x) || is_dir(y) => -4,
            Some(_) => -1,
            None => missing(x),
        };
    }
    s += eb
        .iter()
        .filter(|(k, _)| !ea.contains_key(*k))
        .map(|(_, e)| missing(e))
        .sum::<i64>();
    Ok(s)
}

fn subdirs(
    repo: &Repository,
    tree: &Oid,
    prefix: &str,
    depth: usize,
    out: &mut Vec<(String, Oid)>,
) -> CliResult<()> {
    if depth >= MAX_DEPTH {
        return Ok(());
    }
    for e in entries(repo, tree)? {
        if e.mode != TreeMode::Tree {
            continue;
        }
        let path = if prefix.is_empty() {
            e.name.clone()
        } else {
            format!("{prefix}/{}", e.name)
        };
        out.push((path.clone(), e.oid));
        subdirs(repo, &e.oid, &path, depth + 1, out)?;
    }
    Ok(())
}

/// The tree at `path` inside `tree`, if it is a directory.
fn subtree_at(repo: &Repository, tree: &Oid, path: &str) -> CliResult<Option<Oid>> {
    let mut cur = *tree;
    for part in path.split('/').filter(|p| !p.is_empty()) {
        let Some(e) = entries(repo, &cur)?
            .into_iter()
            .find(|e| e.name == part && e.mode == TreeMode::Tree)
        else {
            return Ok(None);
        };
        cur = e.oid;
    }
    Ok(Some(cur))
}

/// A tree that holds `tree` at `path` and nothing else.
fn place_at(repo: &mut Repository, tree: &Oid, path: &str) -> CliResult<Oid> {
    let format = repo.object_format();
    let mut cur = *tree;
    for part in path.rsplit('/').filter(|p| !p.is_empty()) {
        let t = Tree::new(vec![TreeEntry {
            mode: TreeMode::Tree,
            name: part.to_string(),
            oid: cur,
        }])?;
        cur = repo
            .store_mut()
            .put(format, &RawObject::new(ObjectKind::Tree, t.encode()?))?;
    }
    Ok(cur)
}

enum Shift {
    None,
    /// Their trees go under this directory of ours.
    Down(String),
    /// Their trees are replaced by this directory of theirs.
    Up(String),
}

fn guess(repo: &Repository, ours: &Oid, theirs: &Oid) -> CliResult<Shift> {
    let mut best = (score(repo, ours, theirs)?, Shift::None);
    let mut dirs = Vec::new();
    subdirs(repo, ours, "", 0, &mut dirs)?;
    for (path, oid) in dirs {
        let s = score(repo, &oid, theirs)?;
        if s > best.0 {
            best = (s, Shift::Down(path));
        }
    }
    let mut dirs = Vec::new();
    subdirs(repo, theirs, "", 0, &mut dirs)?;
    for (path, oid) in dirs {
        let s = score(repo, ours, &oid)?;
        if s > best.0 {
            best = (s, Shift::Up(path));
        }
    }
    Ok(best.1)
}

/// `git merge -s subtree` / `-X subtree[=<path>]`: three-way merge of commit
/// `theirs` into the checked-out `ours` commit with the trees shifted first.
pub(crate) fn merge_shifted(
    repo: &mut Repository,
    work: &std::path::Path,
    ours: &Oid,
    theirs: &Oid,
    prefix: Option<&str>,
    labels: &crate::cmd::merge_engine::MergeLabels,
) -> CliResult<crate::cmd::merge_engine::MergeResult> {
    use crate::cmd::merge_strategies::{empty_tree, tree_of};
    let base_tree = match arks_repo::merge_base_oid(repo, ours, theirs)? {
        Some(b) => tree_of(repo, &b)?,
        None => empty_tree(repo)?,
    };
    let ours_tree = tree_of(repo, ours)?;
    let their_tree = tree_of(repo, theirs)?;
    let (b, t) = shift_trees(repo, &ours_tree, &base_tree, &their_tree, prefix)?;
    crate::cmd::merge_engine::merge_trees_worktree(repo, work, &b, &ours_tree, &t, true, labels)
}

/// Shift `base` and `theirs` to match `ours`; `prefix = Some(p)` names the
/// directory instead of guessing it. Returns the shifted `(base, theirs)`.
pub(crate) fn shift_trees(
    repo: &mut Repository,
    ours: &Oid,
    base: &Oid,
    theirs: &Oid,
    prefix: Option<&str>,
) -> CliResult<(Oid, Oid)> {
    let shift = match prefix {
        Some(p) => {
            let p = p.trim_matches('/').to_string();
            if subtree_at(repo, ours, &p)?.is_none() && subtree_at(repo, theirs, &p)?.is_some() {
                Shift::Up(p)
            } else {
                Shift::Down(p)
            }
        }
        None => guess(repo, ours, theirs)?,
    };
    match shift {
        Shift::None => Ok((*base, *theirs)),
        Shift::Down(p) => Ok((place_at(repo, base, &p)?, place_at(repo, theirs, &p)?)),
        Shift::Up(p) => {
            let empty = crate::cmd::merge_strategies::empty_tree(repo)?;
            let b = subtree_at(repo, base, &p)?.unwrap_or(empty);
            let t = subtree_at(repo, theirs, &p)?.unwrap_or(empty);
            Ok((b, t))
        }
    }
}
