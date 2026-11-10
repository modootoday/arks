//! In-memory `ObjectStore` for host-advisory unit tests.

use crate::{ObjectStore, StoreError};
use arks_hash::ObjectFormat;
use arks_object::RawObject;
use arks_oid::Oid;
use std::collections::HashMap;

/// Map keyed by OID hex; no persistence.
#[derive(Debug, Default, Clone)]
pub struct MemoryObjectStore {
    by_oid: HashMap<String, RawObject>,
}

impl MemoryObjectStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl ObjectStore for MemoryObjectStore {
    type Error = StoreError;

    fn put(&mut self, format: ObjectFormat, obj: &RawObject) -> Result<Oid, Self::Error> {
        let oid = obj.oid(format)?;
        self.by_oid.insert(oid.to_hex(), obj.clone());
        Ok(oid)
    }

    fn get(&self, oid: &Oid) -> Result<RawObject, Self::Error> {
        let key = oid.to_hex();
        let obj = self
            .by_oid
            .get(&key)
            .ok_or_else(|| StoreError::NotFound(key.clone()))?
            .clone();
        let format = match oid.kind() {
            arks_oid::OidKind::Sha1 => ObjectFormat::Sha1,
            arks_oid::OidKind::Sha256 => ObjectFormat::Sha256,
        };
        let recomputed = obj.oid(format)?;
        if recomputed != *oid {
            return Err(StoreError::Object(arks_object::ObjectError::OidMismatch {
                expected: oid.to_hex(),
                actual: recomputed.to_hex(),
            }));
        }
        Ok(obj)
    }

    fn exists(&self, oid: &Oid) -> Result<bool, Self::Error> {
        Ok(self.by_oid.contains_key(&oid.to_hex()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arks_object::{ObjectKind, RawObject};

    #[test]
    fn put_get_roundtrip() {
        let mut store = MemoryObjectStore::new();
        let obj = RawObject::new(ObjectKind::Blob, b"hello");
        let oid = store.put(ObjectFormat::Sha1, &obj).unwrap();
        assert!(store.exists(&oid).unwrap());
        let got = store.get(&oid).unwrap();
        assert_eq!(got, obj);
    }

    #[test]
    fn exists_false_for_missing() {
        let store = MemoryObjectStore::new();
        let oid = Oid::from_hex("e69de29bb2d1d6434b8b29ae775ad8c2e48c5391").unwrap();
        assert!(!store.exists(&oid).unwrap());
        assert!(matches!(store.get(&oid), Err(StoreError::NotFound(_))));
    }

    #[test]
    fn put_is_idempotent_oid() {
        let mut store = MemoryObjectStore::new();
        let obj = RawObject::new(ObjectKind::Blob, b"");
        let a = store.put(ObjectFormat::Sha1, &obj).unwrap();
        let b = store.put(ObjectFormat::Sha1, &obj).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn empty_blob_known_oid() {
        let mut store = MemoryObjectStore::new();
        let obj = RawObject::new(ObjectKind::Blob, b"");
        let oid = store.put(ObjectFormat::Sha1, &obj).unwrap();
        assert_eq!(oid.to_hex(), "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391");
    }
}
