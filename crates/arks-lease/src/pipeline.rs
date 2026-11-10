//! L3 claimed-shape pipeline: parallel prep + lease + short ordered Git-lock batch apply.
//!
//! Shape locked by `.spec/adr/20261109194200-…adr.md`:
//! - Prep blob/tree payloads outside exclusive lock CS
//! - Lease grant on ref-shard / worktree domain
//! - Ordered tip updates under documented Git `*.lock` bytes
//! - Tip verify; release
//!
//! Shared-index N-way is **out**. Lease ≠ skip Git locks.
//! `WriterMode::HighConcurrency` remains refused until M23 unlock receipts.

use crate::error::LeaseError;
use crate::manager::LeaseManager;
use crate::resource::{domain_worktree, refns};
use arks_hash::ObjectFormat;
use arks_object::{ObjectKind, RawObject};
use arks_oid::Oid;
use arks_refs::FilesystemRefStore;
use arks_store::{FilesystemGitStore, ObjectStore};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// One object to hash/materialize during the parallel-prep phase.
#[derive(Debug, Clone)]
pub struct PrepItem {
    pub kind: ObjectKind,
    pub payload: Vec<u8>,
}

/// Object after prep: OID known; payload ready to materialize.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedObject {
    pub oid: Oid,
    pub kind: ObjectKind,
    pub payload: Vec<u8>,
}

/// One tip mutation in an ordered batch (ref-shard isolated).
#[derive(Debug, Clone)]
pub struct TipUpdate {
    /// Full refname, e.g. `refs/heads/agent-017`.
    pub refname: String,
    /// Optional CAS expected tip.
    pub expected_old: Option<Oid>,
    /// New tip OID (must already exist in the object store).
    pub new_oid: Oid,
    /// Lease resource for this shard (`refns:…` or `domain:worktree:…`).
    pub shard_resource: String,
}

/// Receipt after ordered batch apply + tip verify.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchApplyReceipt {
    pub applied: Vec<(String, Oid)>,
    pub tips_verified: bool,
    pub shards_leased: Vec<String>,
}

/// Hash prep items (CPU) — safe to call from parallel workers; no Git locks.
pub fn prep_hash(
    items: &[PrepItem],
    format: ObjectFormat,
) -> Result<Vec<PreparedObject>, LeaseError> {
    items
        .iter()
        .map(|item| {
            let obj = RawObject::new(item.kind, item.payload.clone());
            let oid = obj
                .oid(format)
                .map_err(|e| LeaseError::Pipeline(format!("prep hash: {e}")))?;
            Ok(PreparedObject {
                oid,
                kind: item.kind,
                payload: item.payload.clone(),
            })
        })
        .collect()
}

/// Parallel CPU prep via scoped threads (no shared mutable store).
pub fn prep_hash_parallel(
    items: &[PrepItem],
    format: ObjectFormat,
    workers: usize,
) -> Result<Vec<PreparedObject>, LeaseError> {
    if items.is_empty() {
        return Ok(Vec::new());
    }
    let workers = workers.clamp(1, items.len());
    if workers == 1 {
        return prep_hash(items, format);
    }
    let chunk = items.len().div_ceil(workers);
    let mut out: Vec<Option<PreparedObject>> = (0..items.len()).map(|_| None).collect();
    std::thread::scope(|scope| {
        let mut handles = Vec::new();
        for (w, chunk_items) in items.chunks(chunk).enumerate() {
            let base = w * chunk;
            handles.push(scope.spawn(move || {
                let prepared = prep_hash(chunk_items, format)?;
                Ok::<_, LeaseError>((base, prepared))
            }));
        }
        for h in handles {
            let (base, prepared) = h
                .join()
                .map_err(|_| LeaseError::Pipeline("prep worker panicked".into()))??;
            for (i, p) in prepared.into_iter().enumerate() {
                out[base + i] = Some(p);
            }
        }
        Ok::<(), LeaseError>(())
    })?;
    out.into_iter()
        .map(|o| o.ok_or_else(|| LeaseError::Pipeline("prep gap".into())))
        .collect()
}

/// Materialize prepared objects into the filesystem store (short sequential IO).
pub fn materialize_objects(
    store: &mut FilesystemGitStore,
    format: ObjectFormat,
    prepared: &[PreparedObject],
) -> Result<(), LeaseError> {
    for p in prepared {
        let obj = RawObject::new(p.kind, p.payload.clone());
        let oid = store
            .put(format, &obj)
            .map_err(|e| LeaseError::Pipeline(format!("materialize: {e}")))?;
        if oid != p.oid {
            return Err(LeaseError::Pipeline(format!(
                "OID mismatch after put: expected {} got {}",
                p.oid.to_hex(),
                oid.to_hex()
            )));
        }
    }
    Ok(())
}

/// Build a typed ref-namespace shard resource id.
pub fn shard_refns(refname: &str) -> String {
    refns(refname)
}

/// Build a typed worktree-domain shard resource id.
pub fn shard_worktree(id: &str) -> String {
    domain_worktree(id)
}

/// Ordered batch apply under leases + Git ref locks; tip verify; release.
///
/// Critical section is per-ref `write_ref_locked` (Git lock bytes). Leases
/// pre-coordinate shards but never skip those locks.
pub fn apply_batch(
    git_dir: &Path,
    leases: &mut LeaseManager,
    holder: &str,
    ttl: Duration,
    updates: &[TipUpdate],
) -> Result<BatchApplyReceipt, LeaseError> {
    if updates.is_empty() {
        return Ok(BatchApplyReceipt {
            applied: Vec::new(),
            tips_verified: true,
            shards_leased: Vec::new(),
        });
    }

    // Deterministic lock / apply order by refname (prevents ABBA across shards).
    let mut ordered: Vec<&TipUpdate> = updates.iter().collect();
    ordered.sort_by(|a, b| a.refname.cmp(&b.refname));

    let mut shards: Vec<String> = ordered.iter().map(|u| u.shard_resource.clone()).collect();
    shards.sort();
    shards.dedup();

    for shard in &shards {
        leases.grant(shard, holder, ttl)?;
    }

    let apply_result = (|| {
        let mut refs = FilesystemRefStore::open(git_dir)
            .map_err(|e| LeaseError::Pipeline(format!("open refs: {e}")))?;
        let mut applied = Vec::with_capacity(ordered.len());
        for u in &ordered {
            refs.update(&u.refname, &u.new_oid, u.expected_old.as_ref())
                .map_err(|e| LeaseError::Pipeline(format!("ref update {}: {e}", u.refname)))?;
            applied.push((u.refname.clone(), u.new_oid.clone()));
        }

        // Tip verify — resolve direct OID after apply.
        for (name, want) in &applied {
            let got = refs
                .resolve(name)
                .map_err(|e| LeaseError::Pipeline(format!("tip verify resolve {name}: {e}")))?;
            if &got != want {
                return Err(LeaseError::Pipeline(format!(
                    "tip verify failed for {name}: expected {} got {}",
                    want.to_hex(),
                    got.to_hex()
                )));
            }
        }
        Ok(BatchApplyReceipt {
            applied,
            tips_verified: true,
            shards_leased: shards.clone(),
        })
    })();

    // Always release shards we granted (even on apply failure).
    for shard in &shards {
        let _ = leases.release(shard, holder);
    }

    apply_result
}

/// Convenience: objects dir under `$GIT_DIR/objects`.
pub fn objects_dir(git_dir: &Path) -> PathBuf {
    git_dir.join("objects")
}

#[cfg(test)]
mod tests {
    use super::*;
    use arks_object::ObjectKind;
    use tempfile::TempDir;

    fn init_git(dir: &Path) {
        let git = dir.join(".git");
        std::fs::create_dir_all(git.join("refs/heads")).unwrap();
        std::fs::create_dir_all(git.join("objects")).unwrap();
        std::fs::write(git.join("HEAD"), b"ref: refs/heads/main\n").unwrap();
    }

    fn shard_storm(n: usize, workers: usize) {
        let dir = TempDir::new().unwrap();
        init_git(dir.path());
        let git = dir.path().join(".git");
        let mut store = FilesystemGitStore::new(objects_dir(&git)).unwrap();

        let items: Vec<PrepItem> = (0..n)
            .map(|i| PrepItem {
                kind: ObjectKind::Blob,
                payload: format!("agent-{i}-body").into_bytes(),
            })
            .collect();
        let prepared = prep_hash_parallel(&items, ObjectFormat::Sha1, workers).unwrap();
        assert_eq!(prepared.len(), n);
        materialize_objects(&mut store, ObjectFormat::Sha1, &prepared).unwrap();

        let updates: Vec<TipUpdate> = prepared
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let refname = format!("refs/heads/agent-{i:03}");
                TipUpdate {
                    shard_resource: shard_refns(&refname),
                    refname,
                    expected_old: None,
                    new_oid: p.oid,
                }
            })
            .collect();

        let mut leases = LeaseManager::new();
        let receipt = apply_batch(
            &git,
            &mut leases,
            "batch-engine",
            Duration::from_secs(60),
            &updates,
        )
        .unwrap();
        assert!(receipt.tips_verified);
        assert_eq!(receipt.applied.len(), n);
        assert_eq!(leases.list().len(), 0);
    }

    #[test]
    fn prep_parallel_and_batch_apply_distinct_shards() {
        shard_storm(32, 4);
    }

    #[test]
    fn prep_parallel_and_batch_apply_hundreds() {
        // Stress-hundreds tier ([[verify/matrices/matrix-l3.md]]): N=128 distinct ref shards.
        shard_storm(128, 8);
    }

    #[test]
    fn competing_git_lock_still_fails_under_lease() {
        let dir = TempDir::new().unwrap();
        init_git(dir.path());
        let git = dir.path().join(".git");
        let mut store = FilesystemGitStore::new(objects_dir(&git)).unwrap();
        let prep = prep_hash(
            &[PrepItem {
                kind: ObjectKind::Blob,
                payload: b"x".to_vec(),
            }],
            ObjectFormat::Sha1,
        )
        .unwrap();
        materialize_objects(&mut store, ObjectFormat::Sha1, &prep).unwrap();
        let oid = prep[0].oid.clone();
        let refname = "refs/heads/shard-a";
        std::fs::write(git.join(refname).with_extension("lock"), b"").unwrap();
        // Git uses `refs/heads/shard-a.lock` sibling — create correctly.
        let _ = std::fs::remove_file(git.join(refname).with_extension("lock"));
        std::fs::write(git.join("refs/heads/shard-a.lock"), b"").unwrap();

        let mut leases = LeaseManager::new();
        let err = apply_batch(
            &git,
            &mut leases,
            "agent",
            Duration::from_secs(10),
            &[TipUpdate {
                refname: refname.into(),
                expected_old: None,
                new_oid: oid,
                shard_resource: shard_refns(refname),
            }],
        )
        .unwrap_err();
        assert!(
            matches!(err, LeaseError::Pipeline(_)),
            "expected Pipeline lock failure, got {err:?}"
        );
        assert!(leases.list().is_empty());
    }
}
