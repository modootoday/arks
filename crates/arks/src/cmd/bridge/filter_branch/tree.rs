//! Trees, temporary indexes and the tree-filter checkout for filter-branch.

use crate::cmd::index_plumbing::write_worktree_file;
use crate::cmd::plumbing::flatten_tree_to_index;
use crate::error::CliResult;
use crate::write_tree::write_tree_from_index;
use arks_index::{Index, IndexEntry, sort_entries};
use arks_object::{ObjectKind, RawObject, Tree};
use arks_oid::Oid;
use arks_repo::Repository;
use arks_store::ObjectStore;
use std::fs;
use std::path::Path;

/// Index entries of `tree` (zero stat data).
pub fn entries_of(repo: &Repository, tree: &Oid) -> CliResult<Vec<IndexEntry>> {
    let mut out = Vec::new();
    flatten_tree_to_index(repo, tree, "", &mut out)?;
    sort_entries(&mut out);
    Ok(out)
}

/// The subtree at `dir` (`None` when the path is not a directory).
pub fn subtree(repo: &Repository, tree: &Oid, dir: &str) -> CliResult<Option<Oid>> {
    let mut cur = *tree;
    for part in dir.split('/').filter(|p| !p.is_empty()) {
        let obj = repo.get_object(&cur)?;
        let t = Tree::decode(&obj.payload, repo.oid_kind())?;
        match t.entries.into_iter().find(|e| e.name == part) {
            Some(e) if e.mode.is_tree() => cur = e.oid,
            _ => return Ok(None),
        }
    }
    Ok(Some(cur))
}

/// The empty tree, written to the object store.
pub fn empty_tree(repo: &mut Repository) -> CliResult<Oid> {
    let format = repo.object_format();
    Ok(repo
        .store_mut()
        .put(format, &RawObject::new(ObjectKind::Tree, Vec::new()))?)
}

pub fn write_index_file(repo: &Repository, path: &Path, entries: Vec<IndexEntry>) -> CliResult<()> {
    let idx = Index {
        version: 2,
        entries,
        extensions: Vec::new(),
    };
    idx.write_path(path, repo.oid_kind())?;
    Ok(())
}

pub fn read_index_file(repo: &Repository, path: &Path) -> CliResult<Vec<IndexEntry>> {
    if !path.is_file() {
        return Ok(Vec::new());
    }
    Ok(Index::read_path(path, repo.oid_kind())?.entries)
}

pub fn tree_of_entries(repo: &mut Repository, entries: &[IndexEntry]) -> CliResult<Oid> {
    let format = repo.object_format();
    write_tree_from_index(repo.store_mut(), entries, format)
}

/// Empty `dir` and check `entries` out into it.
pub fn checkout(repo: &Repository, dir: &Path, entries: &[IndexEntry]) -> CliResult<()> {
    if dir.exists() {
        for e in fs::read_dir(dir)? {
            let p = e?.path();
            if p.is_dir() && !p.is_symlink() {
                fs::remove_dir_all(&p)?;
            } else {
                fs::remove_file(&p)?;
            }
        }
    } else {
        fs::create_dir_all(dir)?;
    }
    for e in entries {
        write_worktree_file(repo, &dir.join(&e.path), &e.oid, e.mode)?;
    }
    Ok(())
}

/// Index entries for everything under `dir` after a tree filter: new files
/// are added and vanished files dropped; ignore rules do not apply.
/// Submodule entries survive while their directory exists.
pub fn scan(
    repo: &mut Repository,
    dir: &Path,
    before: &[IndexEntry],
) -> CliResult<Vec<IndexEntry>> {
    let gitlinks: Vec<&IndexEntry> = before.iter().filter(|e| e.mode == 0o160000).collect();
    let mut out = Vec::new();
    walk(repo, dir, "", &gitlinks, &mut out)?;
    for g in gitlinks {
        if dir.join(&g.path).is_dir() && !out.iter().any(|e: &IndexEntry| e.path == g.path) {
            out.push(g.clone());
        }
    }
    sort_entries(&mut out);
    Ok(out)
}

fn walk(
    repo: &mut Repository,
    root: &Path,
    prefix: &str,
    gitlinks: &[&IndexEntry],
    out: &mut Vec<IndexEntry>,
) -> CliResult<()> {
    let here = if prefix.is_empty() {
        root.to_path_buf()
    } else {
        root.join(prefix)
    };
    let mut names: Vec<_> = fs::read_dir(&here)?
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    for name in names {
        if prefix.is_empty() && name == ".git" {
            continue;
        }
        let rel = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}/{name}")
        };
        if gitlinks.iter().any(|g| g.path == rel) {
            continue;
        }
        let abs = root.join(&rel);
        let meta = fs::symlink_metadata(&abs)?;
        if meta.is_dir() {
            walk(repo, root, &rel, gitlinks, out)?;
            continue;
        }
        let (bytes, mode) = if meta.file_type().is_symlink() {
            (
                fs::read_link(&abs)?
                    .to_string_lossy()
                    .into_owned()
                    .into_bytes(),
                0o120000,
            )
        } else {
            (fs::read(&abs)?, crate::cmd::shared::file_mode(&meta))
        };
        let format = repo.object_format();
        let oid = repo
            .store_mut()
            .put(format, &RawObject::new(ObjectKind::Blob, bytes))?;
        let mut e = crate::cmd::working_tree::stat_entry(&rel, oid, &meta);
        e.mode = mode;
        out.push(e);
    }
    Ok(())
}
