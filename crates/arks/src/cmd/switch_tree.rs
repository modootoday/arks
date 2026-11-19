//! Two-way tree switch for `checkout` / `switch` / fast-forward `merge`.
//!
//! Git semantics (git-checkout §DESCRIPTION, git-read-tree §Two Tree Merge): moving
//! from the HEAD tree `H` to a target tree `T` keeps local index / working-tree changes
//! for every path that `H` and `T` agree on, updates paths that change between them,
//! and refuses (changing nothing) when a changed path has local modifications or an
//! untracked file would be overwritten.
//! Cite: `.spec/sot/cloud-agent-dropin.sot.md` §O.

use crate::cmd::shared::{file_mode, file_times, index_stat_matches};
use crate::error::{CliError, CliResult};
use crate::util::{read_index, write_index};
use arks_index::{Index, IndexEntry, sort_entries};
use arks_object::{Commit, ObjectKind, RawObject, Tree, TreeMode};
use arks_oid::{Oid, OidKind};
use arks_repo::Repository;
use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::Path;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct TreeFile {
    pub oid: Oid,
    pub mode: u32,
}

/// Flatten a commit's tree into `path → (oid, mode)` (gitlinks included as mode 160000).
pub(crate) fn commit_files(
    repo: &Repository,
    commit: &Oid,
) -> CliResult<HashMap<String, TreeFile>> {
    let obj = repo.get_object(commit)?;
    let tree = match obj.kind {
        ObjectKind::Commit => Commit::decode(&obj.payload)?.tree,
        ObjectKind::Tree => *commit,
        _ => {
            return Err(CliError::Msg(format!(
                "{} is not a commit",
                commit.to_hex()
            )));
        }
    };
    let mut out = HashMap::new();
    walk(repo, &tree, "", &mut out)?;
    Ok(out)
}

fn walk(
    repo: &Repository,
    tree: &Oid,
    prefix: &str,
    out: &mut HashMap<String, TreeFile>,
) -> CliResult<()> {
    let obj = repo.get_object(tree)?;
    let kind = match repo.object_format() {
        arks_store::ObjectFormat::Sha256 => OidKind::Sha256,
        _ => OidKind::Sha1,
    };
    for e in Tree::decode(&obj.payload, kind)?.entries {
        let path = if prefix.is_empty() {
            e.name.clone()
        } else {
            format!("{prefix}/{}", e.name)
        };
        if e.mode == TreeMode::Tree {
            walk(repo, &e.oid, &path, out)?;
        } else {
            out.insert(
                path,
                TreeFile {
                    oid: e.oid,
                    mode: e.mode.as_str().parse::<u32>().ok().map_or(0o100644, octal),
                },
            );
        }
    }
    Ok(())
}

fn octal(decimal_digits: u32) -> u32 {
    u32::from_str_radix(&decimal_digits.to_string(), 8).unwrap_or(0o100644)
}

/// Switch the index and working tree from `from` (HEAD, `None` = unborn) to `to`.
/// Returns the carried local changes as `X\tpath` lines (git prints them on stdout).
pub(crate) fn two_way_switch(
    repo: &Repository,
    work: &Path,
    from: Option<&Oid>,
    to: &Oid,
) -> CliResult<Vec<String>> {
    let h = match from {
        Some(o) => commit_files(repo, o)?,
        None => HashMap::new(),
    };
    let t = commit_files(repo, to)?;
    let mut index = read_index(repo)?;
    if index.entries.iter().any(|e| e.stage() != 0) {
        return Err(CliError::Msg(
            "you need to resolve your current index first".into(),
        ));
    }
    let mut cur: HashMap<String, IndexEntry> = index
        .entries
        .drain(..)
        .map(|e| (e.path.clone(), e))
        .collect();
    let format = repo.object_format();
    let paths: BTreeSet<&String> = h.keys().chain(t.keys()).collect();
    let mut conflicts = Vec::new();
    let mut untracked = Vec::new();
    let mut updates: Vec<(String, Option<TreeFile>)> = Vec::new();
    for p in paths {
        let hv = h.get(p);
        let tv = t.get(p);
        if hv == tv {
            continue;
        }
        let iv = cur.get(p);
        let index_matches = |want: Option<&TreeFile>| match (iv, want) {
            (None, None) => true,
            (Some(e), Some(f)) => e.oid == f.oid,
            _ => false,
        };
        if !index_matches(hv) {
            if index_matches(tv) && worktree_clean(work, p, iv, format)? {
                continue; // index already holds the target content
            }
            conflicts.push(p.clone());
            continue;
        }
        if iv.is_some() {
            if !worktree_clean(work, p, iv, format)? {
                conflicts.push(p.clone());
                continue;
            }
        } else if tv.is_some() && work.join(p).exists() {
            let same = fs::read(work.join(p))
                .ok()
                .and_then(|b| RawObject::new(ObjectKind::Blob, b).oid(format).ok())
                == tv.map(|f| f.oid);
            if !same {
                untracked.push(p.clone());
                continue;
            }
        }
        updates.push((p.clone(), tv.copied()));
    }
    if !conflicts.is_empty() || !untracked.is_empty() {
        let mut msg = String::new();
        if !conflicts.is_empty() {
            msg.push_str(
                "Your local changes to the following files would be overwritten by checkout:\n",
            );
            for c in &conflicts {
                msg.push_str(&format!("\t{c}\n"));
            }
            msg.push_str("Please commit your changes or stash them before you switch branches.\n");
        }
        if !untracked.is_empty() {
            msg.push_str(
                "The following untracked working tree files would be overwritten by checkout:\n",
            );
            for c in &untracked {
                msg.push_str(&format!("\t{c}\n"));
            }
            msg.push_str("Please move or remove them before you switch branches.\n");
        }
        msg.push_str("Aborting");
        return Err(CliError::Msg(msg));
    }
    for (p, tv) in updates {
        let abs = work.join(&p);
        match tv {
            None => {
                let _ = fs::remove_file(&abs);
                cur.remove(&p);
                remove_empty_parents(work, &abs);
            }
            Some(f) => {
                if let Some(parent) = abs.parent() {
                    fs::create_dir_all(parent)?;
                }
                let blob = repo.get_object(&f.oid)?;
                if abs.is_dir() {
                    return Err(CliError::Msg(format!("{p}: directory in the way")));
                }
                let _ = fs::remove_file(&abs);
                if f.mode == 0o120000 {
                    #[cfg(unix)]
                    {
                        let target = String::from_utf8_lossy(&blob.payload).to_string();
                        std::os::unix::fs::symlink(target, &abs)?;
                    }
                    #[cfg(not(unix))]
                    fs::write(&abs, &blob.payload)?;
                } else {
                    fs::write(&abs, &blob.payload)?;
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        let perm = if f.mode == 0o100755 { 0o755 } else { 0o644 };
                        fs::set_permissions(&abs, fs::Permissions::from_mode(perm))?;
                    }
                }
                let meta = fs::symlink_metadata(&abs)?;
                let (ctime_sec, ctime_nsec, mtime_sec, mtime_nsec) = file_times(&meta);
                cur.insert(
                    p.clone(),
                    IndexEntry {
                        ctime_sec,
                        ctime_nsec,
                        mtime_sec,
                        mtime_nsec,
                        dev: 0,
                        ino: 0,
                        mode: if f.mode == 0o120000 {
                            f.mode
                        } else {
                            file_mode(&meta)
                        },
                        uid: 0,
                        gid: 0,
                        size: blob.payload.len().min(u32::MAX as usize) as u32,
                        oid: f.oid,
                        flags: (p.len().min(0x0fff)) as u16,
                        flags_extended: None,
                        path: p.clone(),
                    },
                );
            }
        }
    }
    let mut entries: Vec<IndexEntry> = cur.into_values().collect();
    sort_entries(&mut entries);
    let carried = carried_changes(work, &t, &entries, format)?;
    write_index(
        repo,
        &Index {
            version: index.version,
            entries,
            extensions: Vec::new(),
        },
    )?;
    Ok(carried)
}

/// Working-tree file matches the index entry (stat shortcut, else content hash).
fn worktree_clean(
    work: &Path,
    p: &str,
    entry: Option<&IndexEntry>,
    format: arks_store::ObjectFormat,
) -> CliResult<bool> {
    let abs = work.join(p);
    let Some(e) = entry else {
        return Ok(!abs.exists());
    };
    let Ok(meta) = fs::symlink_metadata(&abs) else {
        return Ok(false);
    };
    if index_stat_matches(e, &meta) {
        return Ok(true);
    }
    let bytes = fs::read(&abs)?;
    Ok(RawObject::new(ObjectKind::Blob, bytes).oid(format)? == e.oid)
}

fn carried_changes(
    work: &Path,
    t: &HashMap<String, TreeFile>,
    entries: &[IndexEntry],
    format: arks_store::ObjectFormat,
) -> CliResult<Vec<String>> {
    let idx: HashMap<&str, &IndexEntry> = entries.iter().map(|e| (e.path.as_str(), e)).collect();
    let mut keys: BTreeSet<&str> = t.keys().map(String::as_str).collect();
    keys.extend(idx.keys().copied());
    let mut out = Vec::new();
    for k in keys {
        match (t.get(k), idx.get(k)) {
            (Some(_), None) => out.push(format!("D\t{k}")),
            (None, Some(_)) => out.push(format!("A\t{k}")),
            (Some(f), Some(e)) => {
                if f.oid != e.oid || !worktree_clean(work, k, Some(e), format)? {
                    out.push(format!("M\t{k}"));
                }
            }
            (None, None) => {}
        }
    }
    Ok(out)
}

fn remove_empty_parents(work: &Path, file: &Path) {
    let mut cur = file.parent();
    while let Some(dir) = cur {
        if dir == work || fs::remove_dir(dir).is_err() {
            break;
        }
        cur = dir.parent();
    }
}
