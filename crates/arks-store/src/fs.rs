//! Filesystem Git object backend (`objects/` root: loose + pack/idx).
//!
//! Primary: <https://git-scm.com/docs/gitrepository-layout>,
//! <https://git-scm.com/docs/gitformat-loose>,
//! <https://git-scm.com/docs/gitformat-pack>

use crate::{ObjectStore, StoreError};
use arks_hash::ObjectFormat;
use arks_object::{ObjectError, RawObject, loose_rel_path, read_loose_hot, write_loose};
use arks_oid::Oid;
use arks_pack::{DecodedPack, PackIndex, PackIndexEntry};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

/// Loose + pack object store rooted at a Git `objects/` directory.
///
/// Pf4-2 / Pf5: `RwLock` so parallel checkout readers share the pack cache without
/// exclusive-lock serialization on every blob get.
#[derive(Debug)]
pub struct FilesystemGitStore {
    objects_dir: PathBuf,
    /// Same-process inserts (e.g. after pack ingest) before disk re-read.
    overlay: RwLock<HashMap<Oid, RawObject>>,
    /// Lazily decoded pack objects under `objects/pack/`.
    pack_objects: RwLock<HashMap<Oid, RawObject>>,
    packs_loaded: RwLock<bool>,
}

impl Clone for FilesystemGitStore {
    fn clone(&self) -> Self {
        Self {
            objects_dir: self.objects_dir.clone(),
            overlay: RwLock::new(self.overlay.read().expect("overlay").clone()),
            pack_objects: RwLock::new(self.pack_objects.read().expect("pack_objects").clone()),
            packs_loaded: RwLock::new(*self.packs_loaded.read().expect("packs_loaded")),
        }
    }
}

impl FilesystemGitStore {
    /// Opens a store at `objects_dir`, creating the directory if missing.
    pub fn new(objects_dir: impl Into<PathBuf>) -> Result<Self, StoreError> {
        let objects_dir = objects_dir.into();
        fs::create_dir_all(&objects_dir).map_err(|e| StoreError::Io(e.to_string()))?;
        Ok(Self::open(objects_dir))
    }

    /// Opens a store at `objects_dir` without creating directories.
    pub fn open(objects_dir: impl Into<PathBuf>) -> Self {
        Self {
            objects_dir: objects_dir.into(),
            overlay: RwLock::new(HashMap::new()),
            pack_objects: RwLock::new(HashMap::new()),
            packs_loaded: RwLock::new(false),
        }
    }

    pub fn objects_dir(&self) -> &Path {
        &self.objects_dir
    }

    /// Enumerate OIDs present as loose objects and/or loaded pack objects.
    ///
    /// Used by optional CELIR rebuild-from-`.git` (feature `celir-index`).
    /// This is arks-owned Git content discovery — **not** a CELIR API.
    pub fn collect_oids(&self) -> Result<Vec<Oid>, StoreError> {
        let mut oids = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for entry in fs::read_dir(&self.objects_dir).map_err(|e| StoreError::Io(e.to_string()))? {
            let entry = entry.map_err(|e| StoreError::Io(e.to_string()))?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.len() != 2 || !name.chars().all(|c| c.is_ascii_hexdigit()) {
                continue;
            }
            if !entry.path().is_dir() {
                continue;
            }
            for file in fs::read_dir(entry.path()).map_err(|e| StoreError::Io(e.to_string()))? {
                let file = file.map_err(|e| StoreError::Io(e.to_string()))?;
                if !file.path().is_file() {
                    continue;
                }
                let rest = file.file_name();
                let rest = rest.to_string_lossy();
                let hex = format!("{name}{rest}");
                let oid = Oid::from_hex(&hex).map_err(|e| StoreError::Io(e.to_string()))?;
                if seen.insert(oid) {
                    oids.push(oid);
                }
            }
        }
        for &oid in self.overlay.read().expect("overlay").keys() {
            if seen.insert(oid) {
                oids.push(oid);
            }
        }
        self.load_packs_if_needed()?;
        for &oid in self.pack_objects.read().expect("pack_objects").keys() {
            if seen.insert(oid) {
                oids.push(oid);
            }
        }
        oids.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
        Ok(oids)
    }

    fn loose_path(&self, oid: &Oid) -> PathBuf {
        self.objects_dir.join(loose_rel_path(oid))
    }

    /// Persist a received pack + matching idx and cache resolved objects.
    ///
    /// Writes `objects/pack/pack-<checksum>.{pack,idx}` (gitrepository-layout).
    /// Callers SHOULD prefer this over exploding every object to loose on fetch/clone.
    pub fn ingest_pack_bytes(
        &mut self,
        pack: &[u8],
        format: ObjectFormat,
    ) -> Result<(), StoreError> {
        let decoded =
            DecodedPack::decode(pack, format).map_err(|e| StoreError::Io(e.to_string()))?;
        let pack_dir = self.objects_dir.join("pack");
        fs::create_dir_all(&pack_dir).map_err(|e| StoreError::Io(e.to_string()))?;
        let hex = checksum_hex(&decoded.checksum);
        let pack_path = pack_dir.join(format!("pack-{hex}.pack"));
        let idx_path = pack_dir.join(format!("pack-{hex}.idx"));
        if !pack_path.is_file() {
            fs::write(&pack_path, pack).map_err(|e| StoreError::Io(e.to_string()))?;
        }
        if !idx_path.is_file() {
            let entries: Vec<PackIndexEntry> = decoded
                .objects
                .iter()
                .map(|o| PackIndexEntry {
                    oid: o.oid,
                    crc32: o.crc32,
                    offset: o.offset,
                })
                .collect();
            let index = PackIndex::from_entries(format, entries, decoded.checksum.clone())
                .map_err(|e| StoreError::Io(e.to_string()))?;
            let idx = index.encode().map_err(|e| StoreError::Io(e.to_string()))?;
            fs::write(&idx_path, idx).map_err(|e| StoreError::Io(e.to_string()))?;
        }
        {
            // Hot path (fetch/clone): keep one copy in pack_objects only.
            // get()/exists() already consult pack_objects after overlay.
            let mut pack_objects = self.pack_objects.write().expect("pack_objects");
            for resolved in decoded.objects {
                pack_objects.insert(resolved.oid, resolved.object);
            }
        }
        *self.packs_loaded.write().expect("packs_loaded") = true;
        Ok(())
    }

    fn load_packs_if_needed(&self) -> Result<(), StoreError> {
        if *self.packs_loaded.read().expect("packs_loaded") {
            return Ok(());
        }
        let pack_dir = self.objects_dir.join("pack");
        if !pack_dir.is_dir() {
            *self.packs_loaded.write().expect("packs_loaded") = true;
            return Ok(());
        }
        let mut loaded = HashMap::new();
        for entry in fs::read_dir(&pack_dir).map_err(|e| StoreError::Io(e.to_string()))? {
            let entry = entry.map_err(|e| StoreError::Io(e.to_string()))?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if !name.ends_with(".pack") {
                continue;
            }
            let bytes = fs::read(entry.path()).map_err(|e| StoreError::Io(e.to_string()))?;
            // Prefer SHA-1 (claimed product format); SHA-256 packs are rare here.
            let decoded = match DecodedPack::decode(&bytes, ObjectFormat::Sha1) {
                Ok(d) => d,
                Err(_) => DecodedPack::decode(&bytes, ObjectFormat::Sha256)
                    .map_err(|e| StoreError::Io(e.to_string()))?,
            };
            for resolved in decoded.objects {
                loaded.insert(resolved.oid, resolved.object);
            }
        }
        self.pack_objects
            .write()
            .expect("pack_objects")
            .extend(loaded);
        *self.packs_loaded.write().expect("packs_loaded") = true;
        Ok(())
    }
}

fn checksum_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        use std::fmt::Write as _;
        let _ = write!(out, "{b:02x}");
    }
    out
}

fn map_read_error(oid: &Oid, err: ObjectError) -> StoreError {
    match err {
        ObjectError::Io(ref msg) if io_msg_is_not_found(msg) => StoreError::NotFound(oid.to_hex()),
        other => StoreError::Object(other),
    }
}

fn io_msg_is_not_found(msg: &str) -> bool {
    let lower = msg.to_ascii_lowercase();
    lower.contains("not found")
        || lower.contains("no such file")
        || lower.contains("os error 2")
        || lower.contains("os error 3")
}

impl ObjectStore for FilesystemGitStore {
    type Error = StoreError;

    fn put(&mut self, format: ObjectFormat, obj: &RawObject) -> Result<Oid, Self::Error> {
        // Disk is source of truth after write_loose. Skipping overlay insert
        // avoids cloning every blob into RAM on bulk `add` (M17 Pf3-2).
        Ok(write_loose(&self.objects_dir, format, obj)?)
    }

    fn get(&self, oid: &Oid) -> Result<RawObject, Self::Error> {
        if let Some(obj) = self.overlay.read().expect("overlay").get(oid) {
            return Ok(obj.clone());
        }
        // Clone/fetch hot path: packs are already ingested into RAM — probe the
        // cache before a loose `is_file` syscall (220× miss on J-CLONE-LOCAL).
        if *self.packs_loaded.read().expect("packs_loaded") {
            if let Some(obj) = self.pack_objects.read().expect("pack_objects").get(oid) {
                return Ok(obj.clone());
            }
        }
        let path = self.loose_path(oid);
        if path.is_file() {
            // Pf4-2: hot-path decode without OID re-hash (path is content-addressed).
            return read_loose_hot(&self.objects_dir, oid).map_err(|e| map_read_error(oid, e));
        }
        self.load_packs_if_needed()?;
        if let Some(obj) = self.pack_objects.read().expect("pack_objects").get(oid) {
            return Ok(obj.clone());
        }
        Err(StoreError::NotFound(oid.to_hex()))
    }

    fn exists(&self, oid: &Oid) -> Result<bool, Self::Error> {
        if self.overlay.read().expect("overlay").contains_key(oid) {
            return Ok(true);
        }
        if *self.packs_loaded.read().expect("packs_loaded")
            && self
                .pack_objects
                .read()
                .expect("pack_objects")
                .contains_key(oid)
        {
            return Ok(true);
        }
        if self.loose_path(oid).is_file() {
            return Ok(true);
        }
        self.load_packs_if_needed()?;
        Ok(self
            .pack_objects
            .read()
            .expect("pack_objects")
            .contains_key(oid))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arks_object::ObjectKind;
    use arks_pack::EncodedPack;
    use tempfile::TempDir;

    fn store_in_temp() -> (TempDir, FilesystemGitStore) {
        let dir = TempDir::new().unwrap();
        let objects = dir.path().join("objects");
        let store = FilesystemGitStore::new(&objects).unwrap();
        (dir, store)
    }

    #[test]
    fn put_get_roundtrip() {
        let (_dir, mut store) = store_in_temp();
        let obj = RawObject::new(ObjectKind::Blob, b"hello");
        let oid = store.put(ObjectFormat::Sha1, &obj).unwrap();
        assert!(store.exists(&oid).unwrap());
        let got = store.get(&oid).unwrap();
        assert_eq!(got, obj);
    }

    #[test]
    fn exists_false_and_get_not_found_for_missing() {
        let (_dir, store) = store_in_temp();
        let oid = Oid::from_hex("e69de29bb2d1d6434b8b29ae775ad8c2e48c5391").unwrap();
        assert!(!store.exists(&oid).unwrap());
        assert!(matches!(store.get(&oid), Err(StoreError::NotFound(_))));
    }

    #[test]
    fn put_is_idempotent_oid() {
        let (_dir, mut store) = store_in_temp();
        let obj = RawObject::new(ObjectKind::Blob, b"");
        let a = store.put(ObjectFormat::Sha1, &obj).unwrap();
        let b = store.put(ObjectFormat::Sha1, &obj).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn empty_and_hello_known_oids() {
        let (_dir, mut store) = store_in_temp();
        let empty = RawObject::new(ObjectKind::Blob, b"");
        let empty_oid = store.put(ObjectFormat::Sha1, &empty).unwrap();
        assert_eq!(
            empty_oid.to_hex(),
            "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391"
        );
        let hello = RawObject::new(ObjectKind::Blob, b"hello");
        let hello_oid = store.put(ObjectFormat::Sha1, &hello).unwrap();
        assert_eq!(
            hello_oid.to_hex(),
            "b6fc4c620b67d95f953a5c1c1230aaab5db5a1b0"
        );
    }

    #[test]
    fn ingest_pack_roundtrip_without_loose() {
        let (_dir, mut store) = store_in_temp();
        let blob = RawObject::new(ObjectKind::Blob, b"pack-ingest");
        let encoded =
            EncodedPack::from_undeltified(ObjectFormat::Sha1, std::slice::from_ref(&blob)).unwrap();
        store
            .ingest_pack_bytes(&encoded.pack, ObjectFormat::Sha1)
            .unwrap();
        let oid = blob.oid(ObjectFormat::Sha1).unwrap();
        assert!(store.exists(&oid).unwrap());
        assert_eq!(store.get(&oid).unwrap(), blob);
        // Pack file landed on disk.
        let pack_dir = store.objects_dir().join("pack");
        let packs: Vec<_> = fs::read_dir(&pack_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().ends_with(".pack"))
            .collect();
        assert_eq!(packs.len(), 1);
        // Re-open store: pack-aware get without loose objects.
        let reopened = FilesystemGitStore::open(store.objects_dir());
        assert!(reopened.exists(&oid).unwrap());
        assert_eq!(reopened.get(&oid).unwrap(), blob);
    }
}
