//! Open + query helpers for product-private CELIR sidecar indexes.
//!
//! Results are index hints — not ObjectStore membership proof
//! ([[sot:root/celir-index-dual-path]] §Query surface).

use celir_index::{QueryBudget, Snapshot, SnapshotOutcome, Store, StoreError, StoreLimits};
use std::path::Path;

/// Open a readonly CELIR store at a product-private sidecar root.
pub fn open_sidecar(root: impl AsRef<Path>) -> Result<Store, StoreError> {
    Store::open_readonly(root.as_ref(), StoreLimits::default())
}

/// Exact lookup for rebuild schema keys `(oid, kind)`.
///
/// Returns `Ok(Some(effect))` on Present, `Ok(None)` on Missing,
/// and `Err` on Conflict / store errors.
pub fn lookup_oid_kind(
    snapshot: &mut Snapshot,
    oid_hex: &str,
    kind: &str,
) -> Result<Option<String>, StoreError> {
    let (outcome, _usage) = snapshot.lookup(
        &[oid_hex.to_owned(), kind.to_owned()],
        QueryBudget::default(),
    )?;
    match outcome {
        SnapshotOutcome::Present(found) => Ok(found.effects.first().cloned()),
        SnapshotOutcome::Missing => Ok(None),
        SnapshotOutcome::Conflict(found) => Err(StoreError::Invalid(format!(
            "conflicting effects for oid/kind: {:?}",
            found.effects
        ))),
    }
}
