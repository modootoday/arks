//! M26 shared-index N-way coordinator (additive to L3 worktree/ref-shard).
//!
//! Protocol (normative):
//! 1. Exclusive product lease on [`SHARED_INDEX_RESOURCE`] (`index:shared`).
//! 2. Mutate `$GIT_DIR/index` only via `arks-index` lock write
//!    (`index.lock` O_CREAT|O_EXCL → rename) — lease ≠ skip Git locks.
//! 3. Release the lease.
//!
//! Cite: [api-lockfile](https://git-scm.com/docs/api-lockfile),
//! [gitformat-index](https://git-scm.com/docs/gitformat-index),
//! `.spec/adr/20261110074000-…adr.md`.
//! Gate: **G34** (≠ G22/G24). Zero gix. No CELIR ObjectStore.

use crate::error::LeaseError;
use crate::manager::LeaseManager;
use crate::resource::index_scope;
use arks_index::{sort_entries, Index, IndexEntry};
use arks_oid::OidKind;
use std::path::Path;
use std::sync::Mutex;
use std::time::Duration;

/// Typed exclusive resource for the shared Git index writer path.
pub const SHARED_INDEX_RESOURCE: &str = "index:shared";

/// Canonical resource id (stable with [`index_scope`]).
pub fn shared_index_resource() -> String {
    index_scope("shared")
}

/// Receipt after a leased shared-index write that still took `index.lock`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SharedIndexWriteReceipt {
    pub holder: String,
    pub resource_id: String,
    pub entry_count: usize,
    pub index_lock_honored: bool,
}

/// Merge `additions` into the shared index under lease + Git `index.lock`.
///
/// Grants exclusive [`SHARED_INDEX_RESOURCE`] (or renews for same holder),
/// reads the existing index if present, appends + sorts, writes via
/// [`Index::write_path`] (exclusive `index.lock` then rename), then releases.
///
/// Concurrent callers without the lease receive [`LeaseError::AlreadyHeld`] and
/// MUST retry — they MUST NOT skip `index.lock`.
pub fn apply_shared_index_entries(
    index_path: &Path,
    leases: &Mutex<LeaseManager>,
    holder: &str,
    ttl: Duration,
    additions: &[IndexEntry],
    hash_kind: OidKind,
) -> Result<SharedIndexWriteReceipt, LeaseError> {
    let resource = shared_index_resource();

    {
        let mut mgr = leases
            .lock()
            .map_err(|_| LeaseError::Pipeline("lease mutex poisoned".into()))?;
        mgr.grant(&resource, holder, ttl)?;
    }

    let apply = (|| {
        let mut entries = if index_path.is_file() {
            Index::read_path(index_path, hash_kind)
                .map_err(|e| LeaseError::Pipeline(format!("read index: {e}")))?
                .entries
        } else {
            Vec::new()
        };
        entries.extend(additions.iter().cloned());
        sort_entries(&mut entries);
        let idx = Index {
            version: 2,
            entries,
            extensions: Vec::new(),
        };
        let count = idx.entries.len();
        idx.write_path(index_path, hash_kind)
            .map_err(|e| LeaseError::Pipeline(format!("index.lock write: {e}")))?;
        Ok(SharedIndexWriteReceipt {
            holder: holder.to_string(),
            resource_id: resource.clone(),
            entry_count: count,
            index_lock_honored: true,
        })
    })();

    {
        let mut mgr = leases
            .lock()
            .map_err(|_| LeaseError::Pipeline("lease mutex poisoned".into()))?;
        let _ = mgr.release(&resource, holder);
    }

    apply
}

/// Attempt grant without writing — used to prove second holder `AlreadyHeld`.
pub fn try_grant_shared_index(
    leases: &mut LeaseManager,
    holder: &str,
    ttl: Duration,
) -> Result<(), LeaseError> {
    leases.grant(shared_index_resource(), holder, ttl)?;
    Ok(())
}
