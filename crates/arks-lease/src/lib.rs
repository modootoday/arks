//! arks-lease — default product lease runtime (M10 / Phase E) + L3 claimed-shape pipeline (M23).
//!
//! # Layers
//! - **L1** [`LeaseManager`]: in-process grant / release / expire / inspect / list
//! - **L2** [`DurableLeaseStore`]: durable under `$GIT_DIR/arks/leases/`
//! - **L3** claimed shape ([`pipeline`]): parallel prep + lease + ordered Git-lock batch apply
//! - **Valve** [`WriterMode::HighConcurrency`]: opt-in claimed shape (**Claimed** after M23 unlock)
//!
//! # Lease ≠ skip Git locks
//! On Git-compat paths, holding a product lease is **additive**. Callers MUST
//! still use `arks-refs` / `arks-index` lock APIs (`*.lock`, `index.lock`).
//! The L3 pipeline still takes Git lock bytes in the short apply CS.
//!
//! Cite: `.spec/sot/arks-lease.sot.md`, `.spec/adr/20261109194200-…adr.md`,
//! `.spec/adr/20261110074000-…adr.md` (M26 shared-index / G34).
//! No `gix`. No CELIR ObjectStore. No forge.

#![forbid(unsafe_code)]

mod durable;
mod error;
mod manager;
mod mode;
pub mod pipeline;
mod resource;
pub mod shared_index;
mod types;

pub use durable::{DurableLeaseStore, LEASES_REL};
pub use error::LeaseError;
pub use manager::LeaseManager;
pub use mode::WriterSession;
pub use pipeline::{
    BatchApplyReceipt, PrepItem, PreparedObject, TipUpdate, apply_batch, materialize_objects,
    objects_dir, prep_hash, prep_hash_parallel, shard_refns, shard_worktree,
};
pub use resource::{
    PREFIX_DOMAIN_SUBMODULE, PREFIX_DOMAIN_WORKTREE, PREFIX_HOOK, PREFIX_INDEX, PREFIX_MAINT,
    PREFIX_REFNS, domain_submodule, domain_worktree, hook_scope, index_scope, is_typed,
    maint_scope, refns,
};
pub use shared_index::{
    SHARED_INDEX_RESOURCE, SharedIndexWriteReceipt, apply_shared_index_entries,
    shared_index_resource, try_grant_shared_index,
};
pub use types::{LeaseRecord, ResourceId, WriterMode, expiry_from, now_ms, system_time_to_ms};

#[cfg(test)]
mod tests;
