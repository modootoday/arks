//! Three-way tree merge for M15 Fs-2 (diverged histories).
//!
//! Cite: <https://git-scm.com/docs/git-merge>, <https://git-scm.com/docs/git-merge-base>
//! Conflicted paths land as index stages 1/2/3 + worktree conflict markers.

use crate::error::{CliError, CliResult};
use crate::util::write_index;
use crate::write_tree::write_tree_from_index;
use arks_index::{Index, IndexEntry, sort_entries};
use arks_object::{Commit, ObjectKind, RawObject, TreeMode};
use arks_oid::Oid;
use arks_repo::{Repository, merge_base_oid};
use arks_store::ObjectStore;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

#[derive(Clone)]
struct Side {
    mode: u32,
    oid: Oid,
}

#[derive(Default)]
pub(crate) struct MergeResult {
    pub tree: Option<Oid>,
    pub conflicts: Vec<String>,
}

/// Three-way merge of commit tips into the index + worktree; returns merge tree
/// when conflict-free. On conflicts, index/worktree are left conflicted and
/// `conflicts` is non-empty (caller MUST NOT create a merge commit).
pub(crate) fn merge_commits_three_way(
    repo: &mut Repository,
    work: &Path,
    ours: &Oid,
    theirs: &Oid,
) -> CliResult<MergeResult> {
    let base = merge_base_oid(repo, ours, theirs)?
        .ok_or_else(|| CliError::Msg("merge: no common ancestor".into()))?;
    let ours_c = load_commit(repo, ours)?;
    let theirs_c = load_commit(repo, theirs)?;
    let base_c = load_commit(repo, &base)?;

    let base_side = blob_sides(repo, &base_c.tree)?;
    let ours_side = blob_sides(repo, &ours_c.tree)?;
    let theirs_side = blob_sides(repo, &theirs_c.tree)?;

    let mut paths = BTreeSet::new();
    paths.extend(base_side.keys().cloned());
    paths.extend(ours_side.keys().cloned());
    paths.extend(theirs_side.keys().cloned());

    let mut entries: Vec<IndexEntry> = Vec::new();
    let mut conflicts = Vec::new();

    for path in paths {
        let b = base_side.get(&path);
        let o = ours_side.get(&path);
        let t = theirs_side.get(&path);
        match (b, o, t) {
            (None, None, None) => {}
            (Some(x), Some(y), Some(z)) if x.oid == y.oid && y.oid == z.oid => {
                entries.push(stage0(&path, y));
                write_blob(repo, work, &path, y)?;
            }
            (_, Some(y), Some(z)) if y.oid == z.oid && y.mode == z.mode => {
                entries.push(stage0(&path, y));
                write_blob(repo, work, &path, y)?;
            }
            (Some(x), Some(y), Some(z)) if x.oid == y.oid && x.mode == y.mode => {
                // unchanged in ours → take theirs
                entries.push(stage0(&path, z));
                write_blob(repo, work, &path, z)?;
            }
            (Some(x), Some(y), Some(z)) if x.oid == z.oid && x.mode == z.mode => {
                // unchanged in theirs → take ours
                entries.push(stage0(&path, y));
                write_blob(repo, work, &path, y)?;
            }
            (None, Some(y), None) => {
                entries.push(stage0(&path, y));
                write_blob(repo, work, &path, y)?;
            }
            (None, None, Some(z)) => {
                entries.push(stage0(&path, z));
                write_blob(repo, work, &path, z)?;
            }
            (Some(_), None, None) => {
                // deleted both sides relative to keep? deleted in both → omit
                let abs = work.join(&path);
                let _ = fs::remove_file(&abs);
            }
            (Some(x), Some(y), None) if x.oid == y.oid => {
                // deleted in theirs, unchanged in ours → delete
                let abs = work.join(&path);
                let _ = fs::remove_file(&abs);
            }
            (Some(x), None, Some(z)) if x.oid == z.oid => {
                // deleted in ours, unchanged in theirs → delete
                let abs = work.join(&path);
                let _ = fs::remove_file(&abs);
            }
            _ => {
                // Conflict: stages 1/2/3 + markers
                conflicts.push(path.clone());
                if let Some(x) = b {
                    entries.push(stage_n(&path, x, 1));
                }
                if let Some(y) = o {
                    entries.push(stage_n(&path, y, 2));
                }
                if let Some(z) = t {
                    entries.push(stage_n(&path, z, 3));
                }
                write_conflict_file(repo, work, &path, o, t)?;
            }
        }
    }

    sort_entries(&mut entries);
    let index = Index {
        version: 2,
        entries,
        extensions: Vec::new(),
    };
    write_index(repo, &index)?;

    if !conflicts.is_empty() {
        return Ok(MergeResult {
            tree: None,
            conflicts,
        });
    }

    let tree = if index.entries.is_empty() {
        let obj = RawObject::new(ObjectKind::Tree, Vec::new());
        let format = repo.object_format();
        repo.store_mut().put(format, &obj)?
    } else {
        let format = repo.object_format();
        write_tree_from_index(repo.store_mut(), &index.entries, format)?
    };
    Ok(MergeResult {
        tree: Some(tree),
        conflicts,
    })
}

fn load_commit(repo: &Repository, oid: &Oid) -> CliResult<Commit> {
    let obj = repo.get_object(oid)?;
    if obj.kind != ObjectKind::Commit {
        return Err(CliError::Msg(format!("{} is not a commit", oid.to_hex())));
    }
    Ok(Commit::decode(&obj.payload)?)
}

fn blob_sides(repo: &Repository, tree: &Oid) -> CliResult<BTreeMap<String, Side>> {
    let mut out = BTreeMap::new();
    walk_tree(repo, tree, "", &mut out)?;
    Ok(out)
}

fn walk_tree(
    repo: &Repository,
    tree_oid: &Oid,
    prefix: &str,
    out: &mut BTreeMap<String, Side>,
) -> CliResult<()> {
    let obj = repo.get_object(tree_oid)?;
    if obj.kind != ObjectKind::Tree {
        return Err(CliError::Msg(format!("not a tree {}", tree_oid.to_hex())));
    }
    let tree = arks_object::Tree::decode(&obj.payload, repo.oid_kind())?;
    for e in tree.entries {
        let path = if prefix.is_empty() {
            e.name.clone()
        } else {
            format!("{prefix}/{}", e.name)
        };
        match e.mode {
            TreeMode::Tree => walk_tree(repo, &e.oid, &path, out)?,
            TreeMode::Gitlink => {
                // Treat gitlink as opaque blob-like for merge (no recurse).
                out.insert(
                    path,
                    Side {
                        mode: 0o160000,
                        oid: e.oid,
                    },
                );
            }
            TreeMode::File | TreeMode::Executable | TreeMode::Symlink => {
                let mode = match e.mode {
                    TreeMode::Executable => 0o100755,
                    TreeMode::Symlink => 0o120000,
                    _ => 0o100644,
                };
                out.insert(path, Side { mode, oid: e.oid });
            }
        }
    }
    Ok(())
}

fn stage0(path: &str, s: &Side) -> IndexEntry {
    stage_n(path, s, 0)
}

fn stage_n(path: &str, s: &Side, stage: u16) -> IndexEntry {
    let flags = ((stage & 3) << 12) | ((path.len().min(0x0fff)) as u16);
    IndexEntry {
        ctime_sec: 0,
        ctime_nsec: 0,
        mtime_sec: 0,
        mtime_nsec: 0,
        dev: 0,
        ino: 0,
        mode: s.mode,
        uid: 0,
        gid: 0,
        size: 0,
        oid: s.oid,
        flags,
        flags_extended: None,
        path: path.to_owned(),
    }
}

fn write_blob(repo: &Repository, work: &Path, path: &str, s: &Side) -> CliResult<()> {
    if s.mode == 0o160000 {
        let abs = work.join(path);
        if let Some(parent) = abs.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::create_dir_all(&abs)?;
        return Ok(());
    }
    let obj = repo.get_object(&s.oid)?;
    let abs = work.join(path);
    if let Some(parent) = abs.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&abs, &obj.payload)?;
    Ok(())
}

fn write_conflict_file(
    repo: &Repository,
    work: &Path,
    path: &str,
    ours: Option<&Side>,
    theirs: Option<&Side>,
) -> CliResult<()> {
    let mut body = String::new();
    body.push_str("<<<<<<< ours\n");
    if let Some(o) = ours {
        if o.mode != 0o160000 {
            let obj = repo.get_object(&o.oid)?;
            body.push_str(&String::from_utf8_lossy(&obj.payload));
            if !body.ends_with('\n') {
                body.push('\n');
            }
        }
    }
    body.push_str("=======\n");
    if let Some(t) = theirs {
        if t.mode != 0o160000 {
            let obj = repo.get_object(&t.oid)?;
            body.push_str(&String::from_utf8_lossy(&obj.payload));
            if !body.ends_with('\n') {
                body.push('\n');
            }
        }
    }
    body.push_str(">>>>>>> theirs\n");
    let abs = work.join(path);
    if let Some(parent) = abs.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(abs, body)?;
    Ok(())
}
