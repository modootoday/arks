//! Object-store facade for Git-compatible object I/O (M2 Phase A).
//!
//! Contract:
//! - `ObjectStore`: `put`, `get`, `exists` on `arks-object` types
//! - Default backend for tests: [`MemoryObjectStore`]
//! - Filesystem Git backend: [`FilesystemGitStore`] (loose + pack/idx read; pack ingest)
//! - Optional feature `celir-index` (default OFF): indexing-service companion only —
//!   **not** an ObjectStore backend (`.spec/sot/storage-celir-boundary.sot.md`,
//!   `.spec/adr/20261110052030-celir-index-optional-product-path-dep.adr.md`)
//! - No `gix`. No product-path `git`.
//!
//! Primary: <https://git-scm.com/docs/gitrepository-layout>
//! Cite: `.spec/sot/arks-store.sot.md`, `.spec/plan/20261108090647-m2-git-first-storage-facade.plan.md`

#![forbid(unsafe_code)]

mod fs;
mod memory;

#[cfg(feature = "celir-index")]
pub mod celir_index;

pub use arks_hash::ObjectFormat;
pub use arks_object::RawObject;
pub use arks_oid::Oid;

pub use fs::{FilesystemGitStore, PendingPackPersist, write_promisor_marker};
pub use memory::MemoryObjectStore;

/// Git-first object store facade (filesystem `.git` / loose objects).
pub trait ObjectStore {
    /// Store-specific error type.
    type Error;

    /// Store an object; returns its OID for the given repository object format.
    fn put(&mut self, format: ObjectFormat, obj: &RawObject) -> Result<Oid, Self::Error>;

    /// Load an object by OID.
    fn get(&self, oid: &Oid) -> Result<RawObject, Self::Error>;

    /// Whether an object is present.
    fn exists(&self, oid: &Oid) -> Result<bool, Self::Error>;
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum StoreError {
    #[error("object not found: {0}")]
    NotFound(String),
    #[error("I/O: {0}")]
    Io(String),
    #[error(transparent)]
    Object(#[from] arks_object::ObjectError),
}

#[cfg(test)]
mod facade_tests {
    use super::*;
    use arks_object::ObjectKind;

    /// Prove callers can use either backend through the trait object surface.
    fn put_get_exists<S: ObjectStore>(store: &mut S, format: ObjectFormat, payload: &[u8])
    where
        S::Error: std::fmt::Debug,
    {
        let obj = RawObject::new(ObjectKind::Blob, payload);
        let oid = store.put(format, &obj).expect("put");
        assert!(store.exists(&oid).expect("exists"));
        assert_eq!(store.get(&oid).expect("get"), obj);
    }

    #[test]
    fn memory_backend_via_trait() {
        let mut store = MemoryObjectStore::new();
        put_get_exists(&mut store, ObjectFormat::Sha1, b"via-trait");
    }

    #[test]
    fn filesystem_backend_via_trait() {
        let dir = tempfile::TempDir::new().unwrap();
        let mut store = FilesystemGitStore::new(dir.path().join("objects")).unwrap();
        put_get_exists(&mut store, ObjectFormat::Sha1, b"via-trait");
    }

    #[test]
    fn memory_and_filesystem_sha256_roundtrip() {
        let obj = RawObject::new(ObjectKind::Blob, b"");
        let expect = "473a0f4c3be8a93681a267e3b1e9a7dcda1185436fe141f7749120a303721813";

        let mut mem = MemoryObjectStore::new();
        let mem_oid = mem.put(ObjectFormat::Sha256, &obj).unwrap();
        assert_eq!(mem_oid.to_hex(), expect);
        assert_eq!(mem.get(&mem_oid).unwrap(), obj);

        let dir = tempfile::TempDir::new().unwrap();
        let mut fs = FilesystemGitStore::new(dir.path().join("objects")).unwrap();
        let fs_oid = fs.put(ObjectFormat::Sha256, &obj).unwrap();
        assert_eq!(fs_oid.to_hex(), expect);
        assert_eq!(fs.get(&fs_oid).unwrap(), obj);
        assert!(fs.exists(&fs_oid).unwrap());
    }
}
