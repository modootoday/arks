//! Baseline worktree + index checkout from a commit tree (shared by clone and Wave Pa).
//!
//! Claimed subset: blob/tree materialization; symlink as file bytes with symlink mode;
//! gitlinks refused. Primary: git-checkout(1) path restoration / switch tree write.
//!
//! M17 Pf3-2: skip rewriting worktree blobs whose index OID already matches (same
//! class of up-to-date short-circuit stock Git uses on switch/checkout).
//! Pf4-3: collect blob write jobs then parallel get+write (Mutex store).
//! Pf5: pack-cache `get_shared` (Arc) + cheap fresh-write stamps; 1-CPU-safe workers.

use crate::error::RemoteError;
use arks_index::{Index, IndexEntry, sort_entries};
use arks_object::{Commit, ObjectKind, RawObject, Tree, TreeMode};
use arks_oid::{Oid, OidKind};
use arks_repo::Repository;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;

/// Write `commit`'s tree into `work` and replace the index to match.
pub fn checkout_commit(repo: &Repository, work: &Path, commit: &Oid) -> Result<(), RemoteError> {
    let obj = repo
        .store()
        .get_shared(commit)
        .map_err(|e| RemoteError::Msg(e.to_string()))?;
    if obj.kind != ObjectKind::Commit {
        return Err(RemoteError::Protocol(format!(
            "object {} is not a commit",
            commit.to_hex()
        )));
    }
    let parsed = Commit::decode(&obj.payload).map_err(|e| RemoteError::Msg(e.to_string()))?;
    checkout_tree(repo, work, &parsed.tree)
}

/// Write a tree into `work` and replace the index to match.
pub fn checkout_tree(repo: &Repository, work: &Path, tree: &Oid) -> Result<(), RemoteError> {
    let prior = load_stage0_index(repo)?;
    let mut jobs = Vec::new();
    let mut kept = HashSet::new();
    let mut made_dirs = HashSet::new();
    collect_jobs(
        repo,
        work,
        tree,
        "",
        &prior,
        &mut jobs,
        &mut kept,
        &mut made_dirs,
    )?;
    // Drop previously tracked paths that did not reappear (hard-reset style).
    for path in prior.keys() {
        if kept.contains(path.as_str()) {
            continue;
        }
        let p = join_rel(work, path)?;
        if p.is_file() || p.is_symlink() {
            let _ = fs::remove_file(&p);
        }
    }
    let mut entries = materialize_jobs(repo, jobs)?;
    sort_entries(&mut entries);
    let index = Index {
        version: 2,
        entries,
        extensions: Vec::new(),
    };
    index
        .write_path(&repo.index_path(), OidKind::Sha1)
        .map_err(|e| RemoteError::Msg(e.to_string()))?;
    Ok(())
}

fn load_stage0_index(repo: &Repository) -> Result<HashMap<String, IndexEntry>, RemoteError> {
    let path = repo.index_path();
    if !path.is_file() {
        return Ok(HashMap::new());
    }
    let index =
        Index::read_path(&path, OidKind::Sha1).map_err(|e| RemoteError::Msg(e.to_string()))?;
    let mut map = HashMap::with_capacity(index.entries.len());
    for e in index.entries.into_iter().filter(|e| e.stage() == 0) {
        map.insert(e.path.clone(), e);
    }
    Ok(map)
}

enum CheckoutJob {
    Keep(IndexEntry),
    Write {
        rel: String,
        path: PathBuf,
        oid: Oid,
        mode: u32,
    },
}

#[allow(clippy::too_many_arguments)]
fn collect_jobs(
    repo: &Repository,
    work: &Path,
    tree_oid: &Oid,
    prefix: &str,
    prior: &HashMap<String, IndexEntry>,
    jobs: &mut Vec<CheckoutJob>,
    kept: &mut HashSet<String>,
    made_dirs: &mut HashSet<PathBuf>,
) -> Result<(), RemoteError> {
    let obj = repo
        .store()
        .get_shared(tree_oid)
        .map_err(|e| RemoteError::Msg(e.to_string()))?;
    if obj.kind != ObjectKind::Tree {
        return Err(RemoteError::Protocol(format!(
            "object {} is not a tree",
            tree_oid.to_hex()
        )));
    }
    let tree =
        Tree::decode(&obj.payload, OidKind::Sha1).map_err(|e| RemoteError::Msg(e.to_string()))?;
    for entry in tree.entries {
        if entry.name.contains('/')
            || entry.name == "."
            || entry.name == ".."
            || entry.name.is_empty()
        {
            return Err(RemoteError::Protocol(format!(
                "unsafe tree name '{}'",
                entry.name
            )));
        }
        let rel = if prefix.is_empty() {
            entry.name.clone()
        } else {
            let mut rel = String::with_capacity(prefix.len() + 1 + entry.name.len());
            rel.push_str(prefix);
            rel.push('/');
            rel.push_str(&entry.name);
            rel
        };
        match entry.mode {
            TreeMode::Tree => {
                collect_jobs(repo, work, &entry.oid, &rel, prior, jobs, kept, made_dirs)?
            }
            TreeMode::Gitlink => {
                // Superproject checkout stages the gitlink; does not recurse into
                // the submodule (git-checkout without --recurse-submodules).
                let path = join_rel(work, &rel)?;
                ensure_dir(&path, made_dirs)?;
                kept.insert(rel.clone());
                jobs.push(CheckoutJob::Keep(IndexEntry {
                    ctime_sec: 0,
                    ctime_nsec: 0,
                    mtime_sec: 0,
                    mtime_nsec: 0,
                    dev: 0,
                    ino: 0,
                    mode: 0o160000,
                    uid: 0,
                    gid: 0,
                    size: 0,
                    oid: entry.oid,
                    flags: (rel.len().min(0x0fff)) as u16,
                    flags_extended: None,
                    path: rel,
                }));
            }
            TreeMode::File | TreeMode::Executable | TreeMode::Symlink => {
                let path = join_rel(work, &rel)?;
                let mode = match entry.mode {
                    TreeMode::Executable => 0o100755,
                    TreeMode::Symlink => 0o120000,
                    _ => 0o100644,
                };
                let prior_e = prior.get(&rel);
                let up_to_date = prior_e
                    .is_some_and(|e| e.oid == entry.oid && index_stat_matches_worktree(e, &path));
                kept.insert(rel.clone());
                if up_to_date {
                    let mut e = prior_e.cloned().unwrap();
                    e.mode = mode;
                    e.flags = (rel.len().min(0x0fff)) as u16;
                    e.path = rel;
                    jobs.push(CheckoutJob::Keep(e));
                } else {
                    if let Some(parent) = path.parent() {
                        ensure_dir(parent, made_dirs)?;
                    }
                    jobs.push(CheckoutJob::Write {
                        rel,
                        path,
                        oid: entry.oid,
                        mode,
                    });
                }
            }
        }
    }
    Ok(())
}

fn materialize_jobs(
    repo: &Repository,
    jobs: Vec<CheckoutJob>,
) -> Result<Vec<IndexEntry>, RemoteError> {
    let mut entries = Vec::with_capacity(jobs.len());
    let mut writes: Vec<(usize, String, PathBuf, Oid, u32)> = Vec::new();
    for (i, job) in jobs.into_iter().enumerate() {
        match job {
            CheckoutJob::Keep(e) => {
                entries.push((i, e));
            }
            CheckoutJob::Write {
                rel,
                path,
                oid,
                mode,
            } => {
                writes.push((i, rel, path, oid, mode));
            }
        }
    }
    // One stamp for the whole batch (clone hot path); avoids SystemTime::now per blob.
    let stamp = stamp_now();
    // Smart-HTTP clone: resolve every blob Arc under one pack-cache read lock,
    // then fs::write without per-blob RwLock (J-CLONE-LOCAL 220× lock tax).
    let oids: Vec<Oid> = writes.iter().map(|(_, _, _, oid, _)| *oid).collect();
    let cached = repo.store().lookup_pack_cached(&oids);
    let cpus = thread::available_parallelism()
        .map(|p| p.get())
        .unwrap_or(1);
    let written = if let Some(blobs) = cached {
        let prepared: Vec<(usize, String, PathBuf, Oid, u32, Arc<RawObject>)> = writes
            .into_iter()
            .zip(blobs)
            .map(|((i, rel, path, oid, mode), blob)| (i, rel, path, oid, mode, blob))
            .collect();
        materialize_cached(prepared, stamp, cpus)?
    } else {
        materialize_uncached(repo, writes, stamp, cpus)?
    };
    entries.extend(written);
    entries.sort_by_key(|(i, _)| *i);
    Ok(entries.into_iter().map(|(_, e)| e).collect())
}

/// Worker count for checkout writes.
///
/// Threshold stays 8 (jobs < 8 → serial). Docker Desktop often advertises ≥8
/// host CPUs; 8-way thrash regressed J-CLONE-LOCAL. Cap at 3 recovers
/// J-CLONE-PATH ≤0.5× headroom without the old full-parallel tax (serial
/// HTTP checkout measured slower on LOCAL than this cap).
fn checkout_workers(cpus: usize, jobs: usize) -> usize {
    if jobs < 8 || cpus <= 1 {
        return 1;
    }
    if cpus >= 8 {
        return 3.min(jobs);
    }
    cpus.clamp(2, 8).min(jobs)
}

fn materialize_cached(
    prepared: Vec<(usize, String, PathBuf, Oid, u32, Arc<RawObject>)>,
    stamp: (u32, u32),
    cpus: usize,
) -> Result<Vec<(usize, IndexEntry)>, RemoteError> {
    let workers = checkout_workers(cpus, prepared.len());
    if workers <= 1 {
        let mut out = Vec::with_capacity(prepared.len());
        for (i, rel, path, oid, mode, blob) in prepared {
            out.push((i, write_cached_blob(&rel, &path, &oid, mode, &blob, stamp)?));
        }
        return Ok(out);
    }
    let chunk = prepared.len().div_ceil(workers);
    let mut out = Vec::with_capacity(prepared.len());
    let mut worker_err: Option<RemoteError> = None;
    thread::scope(|scope| {
        let mut handles = Vec::with_capacity(workers);
        for piece in prepared.chunks(chunk) {
            handles.push(
                scope.spawn(move || -> Result<Vec<(usize, IndexEntry)>, RemoteError> {
                    let mut local = Vec::with_capacity(piece.len());
                    for (i, rel, path, oid, mode, blob) in piece {
                        local.push((*i, write_cached_blob(rel, path, oid, *mode, blob, stamp)?));
                    }
                    Ok(local)
                }),
            );
        }
        for h in handles {
            match h.join() {
                Ok(Ok(local)) => out.extend(local),
                Ok(Err(e)) => {
                    worker_err = Some(e);
                    break;
                }
                Err(_) => {
                    worker_err = Some(RemoteError::Msg("checkout: worker panicked".into()));
                    break;
                }
            }
        }
    });
    if let Some(e) = worker_err {
        return Err(e);
    }
    Ok(out)
}

fn materialize_uncached(
    repo: &Repository,
    writes: Vec<(usize, String, PathBuf, Oid, u32)>,
    stamp: (u32, u32),
    cpus: usize,
) -> Result<Vec<(usize, IndexEntry)>, RemoteError> {
    let workers = checkout_workers(cpus, writes.len());
    if workers <= 1 {
        let mut out = Vec::with_capacity(writes.len());
        for (i, rel, path, oid, mode) in writes {
            out.push((i, write_one_blob(repo, &rel, &path, &oid, mode, stamp)?));
        }
        return Ok(out);
    }
    let chunk = writes.len().div_ceil(workers);
    let mut out = Vec::with_capacity(writes.len());
    let mut worker_err: Option<RemoteError> = None;
    thread::scope(|scope| {
        let mut handles = Vec::with_capacity(workers);
        for piece in writes.chunks(chunk) {
            handles.push(
                scope.spawn(move || -> Result<Vec<(usize, IndexEntry)>, RemoteError> {
                    let mut local = Vec::with_capacity(piece.len());
                    for (i, rel, path, oid, mode) in piece {
                        local.push((*i, write_one_blob(repo, rel, path, oid, *mode, stamp)?));
                    }
                    Ok(local)
                }),
            );
        }
        for h in handles {
            match h.join() {
                Ok(Ok(local)) => out.extend(local),
                Ok(Err(e)) => {
                    worker_err = Some(e);
                    break;
                }
                Err(_) => {
                    worker_err = Some(RemoteError::Msg("checkout: worker panicked".into()));
                    break;
                }
            }
        }
    });
    if let Some(e) = worker_err {
        return Err(e);
    }
    Ok(out)
}

fn write_cached_blob(
    rel: &str,
    path: &Path,
    oid: &Oid,
    mode: u32,
    blob: &RawObject,
    stamp: (u32, u32),
) -> Result<IndexEntry, RemoteError> {
    if blob.kind != ObjectKind::Blob {
        return Err(RemoteError::Protocol(format!("path {rel} is not a blob")));
    }
    fs::write(path, &blob.payload).map_err(|e| RemoteError::Msg(e.to_string()))?;
    let size = u32::try_from(blob.payload.len()).unwrap_or(u32::MAX);
    let (sec, nsec) = stamp;
    Ok(IndexEntry {
        ctime_sec: sec,
        ctime_nsec: nsec,
        mtime_sec: sec,
        mtime_nsec: nsec,
        dev: 0,
        ino: 0,
        mode,
        uid: 0,
        gid: 0,
        size,
        oid: *oid,
        flags: (rel.len().min(0x0fff)) as u16,
        flags_extended: None,
        path: rel.to_owned(),
    })
}

fn write_one_blob(
    repo: &Repository,
    rel: &str,
    path: &Path,
    oid: &Oid,
    mode: u32,
    stamp: (u32, u32),
) -> Result<IndexEntry, RemoteError> {
    // Arc share from pack cache — avoid deep-copying every blob on parallel clone.
    let blob = repo
        .store()
        .get_shared(oid)
        .map_err(|e| RemoteError::Msg(e.to_string()))?;
    write_cached_blob(rel, path, oid, mode, &blob, stamp)
}

fn ensure_dir(path: &Path, made: &mut HashSet<PathBuf>) -> Result<(), RemoteError> {
    if made.contains(path) {
        return Ok(());
    }
    fs::create_dir_all(path).map_err(|e| RemoteError::Msg(e.to_string()))?;
    made.insert(path.to_path_buf());
    Ok(())
}

fn join_rel(work: &Path, rel: &str) -> Result<PathBuf, RemoteError> {
    let mut path = work.to_path_buf();
    for part in rel.split('/') {
        if part.is_empty() || part == "." || part == ".." {
            return Err(RemoteError::Protocol(format!("unsafe path {rel}")));
        }
        path.push(part);
    }
    Ok(path)
}

/// Return true when cached index stat still matches the worktree file (git
/// `ce_match_stat` lite). Zeroed mtime means "untrusted / force rewrite".
fn index_stat_matches_worktree(e: &IndexEntry, path: &Path) -> bool {
    if !(path.is_file() || path.is_symlink()) {
        return false;
    }
    if e.mtime_sec == 0 && e.mtime_nsec == 0 {
        return false;
    }
    let Ok(meta) = fs::metadata(path) else {
        return false;
    };
    if meta.len() != u64::from(e.size) {
        return false;
    }
    let Ok(modified) = meta.modified() else {
        return false;
    };
    let dur = modified
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    dur.as_secs() as u32 == e.mtime_sec && dur.subsec_nanos() == e.mtime_nsec
}

/// Fresh-write clock sample without a post-write `stat` (clone hot path).
fn stamp_now() -> (u32, u32) {
    use std::time::{SystemTime, UNIX_EPOCH};
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    (dur.as_secs() as u32, dur.subsec_nanos())
}
