//! Optional CELIR **indexing-service** companion (feature `celir-index`).
//!
//! Product-private index build / publish / open / query only.
//! This module is **not** an [`crate::ObjectStore`] backend.
//! VCS put/get/exists via CELIR remains Refuse
//! (`.spec/sot/storage-celir-boundary.sot.md`,
//! `.spec/sot/celir-index-dual-path.sot.md`,
//! `.spec/adr/20261110052030-celir-index-optional-product-path-dep.adr.md`).

mod query_surface;
mod rebuild;
mod runtime_jsonl;
mod sidecar;

pub use query_surface::{lookup_oid_kind, open_sidecar};
pub use rebuild::{
    corpus_from_git, rebuild_from_git, rebuild_from_git_capped, RebuildError, RebuildReport,
    REBUILD_COLUMNS,
};
pub use runtime_jsonl::{
    build_jsonl_root, oid_kind_projection, open_jsonl_root, query_exact_effects,
    write_oid_kind_jsonl,
};
pub use sidecar::{
    ensure_sidecar_root, is_sidecar_path, sidecar_root, SIDECAR_CONTAINER, SIDECAR_LEAF,
};

/// Bounded CELIR index store types (single-writer / snapshot).
pub use celir_index::{
    QueryBudget, Snapshot as CelirIndexSnapshot, SnapshotOutcome, Store as CelirIndexStore,
    StoreError as CelirIndexStoreError, StoreLimits, Writer as CelirIndexWriter,
};

/// Immutable-root publication / query composition from `celir-index-runtime`.
pub use celir_index_runtime::{
    build as celir_index_runtime_build, Budget as CelirRuntimeBudget,
    BuildReport as CelirRuntimeBuildReport, Projection as CelirRuntimeProjection,
    Query as CelirRuntimeQuery, QueryLimits as CelirRuntimeQueryLimits,
    Snapshot as CelirIndexRuntimeSnapshot, Source as CelirRuntimeSource,
};

/// Compile-time linkage probe: proves path-deps resolve under `--features celir-index`.
pub fn linkage_probe() -> &'static str {
    let _store = std::any::type_name::<CelirIndexStore>();
    let _snap = std::any::type_name::<CelirIndexRuntimeSnapshot>();
    let _corpus = std::any::type_name::<celir_data::Corpus>();
    "celir-index"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feature_links_celir_index_crates() {
        assert_eq!(linkage_probe(), "celir-index");
    }

    #[test]
    fn companion_is_not_object_store_name() {
        // Honesty: module path must not advertise ObjectStore.
        assert!(!module_path!().contains("ObjectStore"));
    }
}
