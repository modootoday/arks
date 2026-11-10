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
use arks_store::ObjectStore;
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

/// Collect missing object OIDs needed to materialize `commit`'s HEAD tree.
///
/// Historical name retained; for `tree:0` this may include nested trees.
/// Prefer [`missing_checkout_oids`] at new call sites.
pub fn missing_blob_oids(repo: &Repository, commit: &Oid) -> Result<Vec<Oid>, RemoteError> {
    missing_checkout_oids(repo, commit)
}

/// Collect missing tree + blob OIDs needed to materialize `commit`'s HEAD tree.
///
/// Used by partial-clone checkout backfill for `blob:none` and `tree:0`.
/// When a nested tree is absent, its OID is listed and recursion stops there
/// until a later demand-fetch round.
pub fn missing_checkout_oids(repo: &Repository, commit: &Oid) -> Result<Vec<Oid>, RemoteError> {
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
    let mut missing = Vec::new();
    let mut seen = HashSet::new();
    collect_missing_checkout_oids(repo, &parsed.tree, &mut missing, &mut seen)?;
    Ok(missing)
}

fn collect_missing_checkout_oids(
    repo: &Repository,
    tree_oid: &Oid,
    missing: &mut Vec<Oid>,
    seen: &mut HashSet<Oid>,
) -> Result<(), RemoteError> {
    if !seen.insert(*tree_oid) {
        return Ok(());
    }
    let present = repo
        .store()
        .exists(tree_oid)
        .map_err(|e| RemoteError::Msg(e.to_string()))?;
    if !present {
        missing.push(*tree_oid);
        return Ok(());
    }
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
        match entry.mode {
            TreeMode::Tree => collect_missing_checkout_oids(repo, &entry.oid, missing, seen)?,
            TreeMode::File | TreeMode::Executable | TreeMode::Symlink => {
                if !seen.insert(entry.oid) {
                    continue;
                }
                let blob_present = repo
                    .store()
                    .exists(&entry.oid)
                    .map_err(|e| RemoteError::Msg(e.to_string()))?;
                if !blob_present {
                    missing.push(entry.oid);
                }
            }
            TreeMode::Gitlink => {}
        }
    }
    Ok(())
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
    let mut entries = materialize_jobs(repo, work, jobs)?;
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
    work: &Path,
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
        materialize_cached(repo, work, prepared, stamp, cpus)?
    } else {
        materialize_uncached(repo, work, writes, stamp, cpus)?
    };
    entries.extend(written);
    entries.sort_by_key(|(i, _)| *i);
    Ok(entries.into_iter().map(|(_, e)| e).collect())
}

/// Worker count for checkout writes.
///
/// Threshold stays 8 (jobs < 8 → serial). Docker Desktop often advertises ≥8
/// host CPUs; 8-way thrash regressed J-CLONE-LOCAL. Cap at 2 matched the
/// LOCAL 0.5778 tip while keeping PATH ≤0.5× (cap 3 did not buy LOCAL ms).
fn checkout_workers(cpus: usize, jobs: usize) -> usize {
    if jobs < 8 || cpus <= 1 {
        return 1;
    }
    if cpus >= 8 {
        return 2.min(jobs);
    }
    cpus.clamp(2, 8).min(jobs)
}

fn materialize_cached(
    repo: &Repository,
    work: &Path,
    prepared: Vec<(usize, String, PathBuf, Oid, u32, Arc<RawObject>)>,
    stamp: (u32, u32),
    cpus: usize,
) -> Result<Vec<(usize, IndexEntry)>, RemoteError> {
    let workers = checkout_workers(cpus, prepared.len());
    if workers <= 1 {
        let mut out = Vec::with_capacity(prepared.len());
        for (i, rel, path, oid, mode, blob) in prepared {
            out.push((
                i,
                write_cached_blob(repo, work, &rel, &path, &oid, mode, &blob, stamp)?,
            ));
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
                        local.push((
                            *i,
                            write_cached_blob(repo, work, rel, path, oid, *mode, blob, stamp)?,
                        ));
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
    work: &Path,
    writes: Vec<(usize, String, PathBuf, Oid, u32)>,
    stamp: (u32, u32),
    cpus: usize,
) -> Result<Vec<(usize, IndexEntry)>, RemoteError> {
    let workers = checkout_workers(cpus, writes.len());
    if workers <= 1 {
        let mut out = Vec::with_capacity(writes.len());
        for (i, rel, path, oid, mode) in writes {
            out.push((
                i,
                write_one_blob(repo, work, &rel, &path, &oid, mode, stamp)?,
            ));
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
                        local.push((
                            *i,
                            write_one_blob(repo, work, rel, path, oid, *mode, stamp)?,
                        ));
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

#[allow(clippy::too_many_arguments)] // checkout write job fields + smudge context
fn write_cached_blob(
    repo: &Repository,
    work: &Path,
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
    let payload = apply_smudge(repo, work, rel, &blob.payload)?;
    fs::write(path, &payload).map_err(|e| RemoteError::Msg(e.to_string()))?;
    let size = u32::try_from(payload.len()).unwrap_or(u32::MAX);
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
    work: &Path,
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
    write_cached_blob(repo, work, rel, path, oid, mode, &blob, stamp)
}

/// Apply claimed smudge to blob bytes for worktree materialize (LFS in-process + filter).
pub fn smudge_worktree_blob(
    repo: &Repository,
    work: &Path,
    rel: &str,
    payload: &[u8],
) -> Result<Vec<u8>, RemoteError> {
    apply_smudge(repo, work, rel, payload)
}

/// FS6 reopen: smudge on checkout — in-process LFS pointer resolve, then `filter.*.smudge`.
fn apply_smudge(
    repo: &Repository,
    work: &Path,
    rel: &str,
    payload: &[u8],
) -> Result<Vec<u8>, RemoteError> {
    if let Some(bytes) = resolve_lfs_pointer_local(repo, payload)? {
        return Ok(bytes);
    }
    let Some(name) = attribute_filter_name(work, rel) else {
        return Ok(payload.to_vec());
    };
    let cfg = fs::read_to_string(repo.git_dir().join("config")).unwrap_or_default();
    let key = format!("filter.{name}.smudge");
    let Some(cmd) = config_value(&cfg, &key) else {
        return Ok(payload.to_vec());
    };
    run_filter_command(&cmd, work, rel, payload, &key)
}

/// In-process Git LFS pointer → `.git/lfs/objects/aa/bb/<oid>` when present.
fn resolve_lfs_pointer_local(
    repo: &Repository,
    payload: &[u8],
) -> Result<Option<Vec<u8>>, RemoteError> {
    let Ok(text) = std::str::from_utf8(payload) else {
        return Ok(None);
    };
    if !text.starts_with("version https://git-lfs.github.com/spec/v1\n")
        && !text.starts_with("version https://git-lfs.github.com/spec/v1\r\n")
    {
        return Ok(None);
    }
    let mut oid_hex: Option<&str> = None;
    let mut size: Option<u64> = None;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("oid sha256:") {
            oid_hex = Some(rest.trim());
        } else if let Some(rest) = line.strip_prefix("size ") {
            size = rest.trim().parse().ok();
        }
    }
    let Some(oid_hex) = oid_hex else {
        return Ok(None);
    };
    if oid_hex.len() != 64 || !oid_hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Ok(None);
    }
    let path = repo
        .git_dir()
        .join("lfs")
        .join("objects")
        .join(&oid_hex[..2])
        .join(&oid_hex[2..4])
        .join(oid_hex);
    if !path.is_file() {
        // Missing local object: leave pointer bytes (network LFS fetch remains git-lfs).
        return Ok(None);
    }
    let bytes = fs::read(&path).map_err(|e| RemoteError::Msg(e.to_string()))?;
    if let Some(sz) = size {
        if bytes.len() as u64 != sz {
            return Err(RemoteError::Msg(format!(
                "lfs in-process smudge: size mismatch for {oid_hex} (got {} want {sz})",
                bytes.len()
            )));
        }
    }
    Ok(Some(bytes))
}

fn attribute_filter_name(work: &Path, rel: &str) -> Option<String> {
    let attr = work.join(".gitattributes");
    let text = fs::read_to_string(attr).ok()?;
    let file = Path::new(rel)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(rel);
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split_whitespace();
        let pattern = parts.next().unwrap_or("");
        let matches = pattern == "*"
            || pattern == rel
            || pattern == file
            || (pattern.starts_with("*.") && file.ends_with(&pattern[1..]));
        if !matches {
            continue;
        }
        for a in parts {
            if let Some(name) = a.strip_prefix("filter=") {
                return Some(name.to_string());
            }
        }
    }
    None
}

fn config_value(text: &str, key: &str) -> Option<String> {
    let key = key.to_ascii_lowercase();
    let mut section = String::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if line.starts_with('[') {
            section = line
                .trim_matches(|c| c == '[' || c == ']')
                .to_ascii_lowercase()
                .replace('"', "");
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let full = if section.contains(' ') {
            let mut bits = section.split_whitespace();
            let head = bits.next().unwrap_or("");
            let name = bits.next().unwrap_or("");
            format!("{head}.{name}.{}", k.trim().to_ascii_lowercase())
        } else {
            format!("{section}.{}", k.trim().to_ascii_lowercase())
        };
        if full == key {
            return Some(v.trim().trim_matches('"').to_string());
        }
    }
    None
}

fn run_filter_command(
    cmd: &str,
    work: &Path,
    rel: &str,
    payload: &[u8],
    key: &str,
) -> Result<Vec<u8>, RemoteError> {
    let mut argv: Vec<String> = cmd.split_whitespace().map(|s| s.to_string()).collect();
    if argv.is_empty() {
        return Ok(payload.to_vec());
    }
    for a in &mut argv {
        *a = a.replace("%f", rel);
    }
    let mut child = std::process::Command::new(&argv[0])
        .args(&argv[1..])
        .current_dir(work)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::inherit())
        .spawn()
        .map_err(|e| RemoteError::Msg(format!("{key}: {e}")))?;
    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        stdin
            .write_all(payload)
            .map_err(|e| RemoteError::Msg(format!("{key} stdin: {e}")))?;
    }
    let out = child
        .wait_with_output()
        .map_err(|e| RemoteError::Msg(format!("{key}: {e}")))?;
    if !out.status.success() {
        return Err(RemoteError::Msg(format!(
            "{key} exited {}",
            out.status.code().unwrap_or(-1)
        )));
    }
    Ok(out.stdout)
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

#[cfg(test)]
mod lfs_smudge_tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn in_process_lfs_pointer_resolves_local_object() {
        let dir = TempDir::new().unwrap();
        let git = dir.path().join(".git");
        fs::create_dir_all(git.join("objects")).unwrap();
        fs::create_dir_all(git.join("refs/heads")).unwrap();
        fs::write(git.join("HEAD"), "ref: refs/heads/main\n").unwrap();
        fs::write(
            git.join("config"),
            "[core]\n\trepositoryformatversion = 0\n",
        )
        .unwrap();
        let content = b"hello-from-lfs-storage";
        let oid_hex = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let obj_path = git
            .join("lfs/objects")
            .join(&oid_hex[..2])
            .join(&oid_hex[2..4]);
        fs::create_dir_all(&obj_path).unwrap();
        fs::write(obj_path.join(oid_hex), content).unwrap();
        let repo = Repository::open(&git).unwrap();
        let pointer = format!(
            "version https://git-lfs.github.com/spec/v1\noid sha256:{oid_hex}\nsize {}\n",
            content.len()
        );
        let got = resolve_lfs_pointer_local(&repo, pointer.as_bytes())
            .unwrap()
            .expect("resolved");
        assert_eq!(got, content);
    }

    #[test]
    fn in_process_lfs_size_mismatch_refuses() {
        let dir = TempDir::new().unwrap();
        let git = dir.path().join(".git");
        fs::create_dir_all(git.join("objects")).unwrap();
        fs::create_dir_all(git.join("refs/heads")).unwrap();
        fs::write(git.join("HEAD"), "ref: refs/heads/main\n").unwrap();
        fs::write(
            git.join("config"),
            "[core]\n\trepositoryformatversion = 0\n",
        )
        .unwrap();
        let oid_hex = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let obj_path = git
            .join("lfs/objects")
            .join(&oid_hex[..2])
            .join(&oid_hex[2..4]);
        fs::create_dir_all(&obj_path).unwrap();
        fs::write(obj_path.join(oid_hex), b"xx").unwrap();
        let repo = Repository::open(&git).unwrap();
        let pointer =
            format!("version https://git-lfs.github.com/spec/v1\noid sha256:{oid_hex}\nsize 99\n");
        let err = resolve_lfs_pointer_local(&repo, pointer.as_bytes()).unwrap_err();
        assert!(err.to_string().contains("size mismatch"), "{err}");
    }
}
