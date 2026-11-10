//! Build a Git tree object from a flat index (Wave L commit path).

use crate::error::{CliError, CliResult};
use arks_index::IndexEntry;
use arks_object::{ObjectKind, RawObject, Tree, TreeEntry, TreeMode};
use arks_oid::Oid;
use arks_store::{FilesystemGitStore, ObjectFormat, ObjectStore};
use std::collections::BTreeMap;

pub fn write_tree_from_index(
    store: &mut FilesystemGitStore,
    entries: &[IndexEntry],
    format: ObjectFormat,
) -> CliResult<Oid> {
    let staged: Vec<_> = entries.iter().filter(|e| e.stage() == 0).cloned().collect();
    write_tree_level(store, &staged, "", format)
}

fn write_tree_level(
    store: &mut FilesystemGitStore,
    entries: &[IndexEntry],
    prefix: &str,
    format: ObjectFormat,
) -> CliResult<Oid> {
    // Group by first path component under `prefix`.
    let mut files: BTreeMap<String, &IndexEntry> = BTreeMap::new();
    let mut dirs: BTreeMap<String, Vec<IndexEntry>> = BTreeMap::new();

    for e in entries {
        let rel = if prefix.is_empty() {
            e.path.as_str()
        } else {
            e.path
                .strip_prefix(prefix)
                .and_then(|s| s.strip_prefix('/'))
                .ok_or_else(|| CliError::Msg(format!("index path {} not under {prefix}", e.path)))?
        };
        if let Some((name, rest)) = rel.split_once('/') {
            dirs.entry(name.to_owned()).or_default().push(IndexEntry {
                path: e.path.clone(),
                ..e.clone()
            });
            let _ = rest;
        } else {
            files.insert(rel.to_owned(), e);
        }
    }

    let mut tree_entries = Vec::new();
    for (name, ent) in files {
        let mode = TreeMode::from_index_mode(ent.mode)?;
        tree_entries.push(TreeEntry {
            mode,
            name,
            oid: ent.oid,
        });
    }
    for (name, children) in dirs {
        let child_prefix = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}/{name}")
        };
        let oid = write_tree_level(store, &children, &child_prefix, format)?;
        tree_entries.push(TreeEntry {
            mode: TreeMode::Tree,
            name,
            oid,
        });
    }

    let tree = Tree::new(tree_entries)?;
    let obj = RawObject::new(ObjectKind::Tree, tree.encode()?);
    Ok(store.put(format, &obj)?)
}
