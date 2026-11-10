//! Loose Git object encode/decode (claimed subset).
//!
//! Contract (M1):
//! - Kinds: `blob`, `tree`, `commit`, `tag`
//! - Uncompressed form: `type SP size NUL` + payload
//! - Loose store: zlib-compress that stream; path `objects/aa/bb…`
//! - OID via `arks-hash::hash_object` + `arks-oid::Oid`
//! - No pack writers as product claims. No `gix`. No product-path `git`.
//!
//! Primary: <https://git-scm.com/docs/gitformat-loose>
//! Cite: `.spec/research/20261108071913-git-objects-pack.research.md`

#![forbid(unsafe_code)]

mod commit;
mod patch;
mod tree;

pub use commit::{Commit, Signature};
pub use patch::{ParsedFile, ParsedPatch, parse_patch};
pub use tree::{
    Tree, TreeEntry, TreeMode, cmp_tree_entries, sort_tree_entries, tree_entries_sorted,
    tree_sort_key,
};

use arks_hash::{Digest, ObjectFormat, hash_object};
use arks_oid::{Oid, OidError, OidKind};
use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// Claimed Git object kinds for M1 loose encode/decode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ObjectKind {
    Blob,
    Tree,
    Commit,
    Tag,
}

impl ObjectKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Blob => "blob",
            Self::Tree => "tree",
            Self::Commit => "commit",
            Self::Tag => "tag",
        }
    }

    pub fn parse(s: &str) -> Result<Self, ObjectError> {
        match s {
            "blob" => Ok(Self::Blob),
            "tree" => Ok(Self::Tree),
            "commit" => Ok(Self::Commit),
            "tag" => Ok(Self::Tag),
            other => Err(ObjectError::UnknownKind(other.to_string())),
        }
    }
}

/// Uncompressed object content (kind + payload).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawObject {
    pub kind: ObjectKind,
    pub payload: Vec<u8>,
}

impl RawObject {
    pub fn new(kind: ObjectKind, payload: impl Into<Vec<u8>>) -> Self {
        Self {
            kind,
            payload: payload.into(),
        }
    }

    /// Serialize uncompressed object bytes (`type SP size NUL` + payload).
    pub fn encode_uncompressed(&self) -> Vec<u8> {
        let type_name = self.kind.as_str();
        let size = self.payload.len().to_string();
        let mut out = Vec::with_capacity(type_name.len() + 1 + size.len() + 1 + self.payload.len());
        out.extend_from_slice(type_name.as_bytes());
        out.push(b' ');
        out.extend_from_slice(size.as_bytes());
        out.push(0);
        out.extend_from_slice(&self.payload);
        out
    }

    pub fn decode_uncompressed(bytes: &[u8]) -> Result<Self, ObjectError> {
        let nul = bytes
            .iter()
            .position(|&b| b == 0)
            .ok_or(ObjectError::MissingHeaderNul)?;
        let header = std::str::from_utf8(&bytes[..nul]).map_err(|_| ObjectError::HeaderUtf8)?;
        let (type_name, size_str) = header.split_once(' ').ok_or(ObjectError::BadHeader)?;
        let kind = ObjectKind::parse(type_name)?;
        let size: usize = size_str.parse().map_err(|_| ObjectError::BadSize)?;
        let payload = &bytes[nul + 1..];
        if payload.len() != size {
            return Err(ObjectError::SizeMismatch {
                declared: size,
                actual: payload.len(),
            });
        }
        Ok(Self {
            kind,
            payload: payload.to_vec(),
        })
    }

    pub fn digest(&self, format: ObjectFormat) -> Digest {
        hash_object(format, self.kind.as_str(), &self.payload)
    }

    pub fn oid(&self, format: ObjectFormat) -> Result<Oid, ObjectError> {
        let d = self.digest(format);
        let kind = match format {
            ObjectFormat::Sha1 => OidKind::Sha1,
            ObjectFormat::Sha256 => OidKind::Sha256,
        };
        Oid::from_bytes(kind, d.as_bytes()).map_err(ObjectError::Oid)
    }
}

/// zlib-compress uncompressed object bytes for a loose object file.
///
/// Uses `Compression::fast()` — OID is over uncompressed content; any valid
/// zlib framing of the same bytes is interchangeable for loose objects
/// (stock Git accepts varying zlib levels). M17 Pf3-2 hot path.
pub fn zlib_compress(uncompressed: &[u8]) -> Result<Vec<u8>, ObjectError> {
    let mut enc = ZlibEncoder::new(Vec::new(), Compression::fast());
    enc.write_all(uncompressed)
        .map_err(|e| ObjectError::Io(e.to_string()))?;
    enc.finish().map_err(|e| ObjectError::Io(e.to_string()))
}

/// zlib-decompress a loose object file body to uncompressed object bytes.
pub fn zlib_decompress(compressed: &[u8]) -> Result<Vec<u8>, ObjectError> {
    let mut dec = ZlibDecoder::new(compressed);
    let mut out = Vec::new();
    dec.read_to_end(&mut out)
        .map_err(|e| ObjectError::Io(e.to_string()))?;
    Ok(out)
}

/// Loose object path relative to `objects/`: `aa/bbbb…`.
pub fn loose_rel_path(oid: &Oid) -> PathBuf {
    let hex = oid.to_hex();
    PathBuf::from(&hex[..2]).join(&hex[2..])
}

/// Write a loose object under `objects_dir` (the `objects` directory).
///
/// Content-addressed: if `path` already exists for this OID, return `Ok(oid)`
/// without rewriting (avoids Windows `rename` failures when the destination exists).
///
/// Pf4-2: stream zlib from header+payload (no intermediate uncompressed `Vec`) and
/// skip `create_dir_all` when the fanout directory already exists.
pub fn write_loose(
    objects_dir: &Path,
    format: ObjectFormat,
    obj: &RawObject,
) -> Result<Oid, ObjectError> {
    let oid = obj.oid(format)?;
    let rel = loose_rel_path(&oid);
    let path = objects_dir.join(&rel);
    if path.is_file() {
        return Ok(oid);
    }
    if let Some(parent) = path.parent() {
        if !parent.is_dir() {
            std::fs::create_dir_all(parent).map_err(|e| ObjectError::Io(e.to_string()))?;
        }
    }
    let compressed = zlib_compress_object(obj.kind, &obj.payload)?;
    // Atomic-ish: write temp then rename when possible.
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, &compressed).map_err(|e| ObjectError::Io(e.to_string()))?;
    // If a concurrent writer raced us into place, treat as success and drop tmp.
    if path.is_file() {
        let _ = std::fs::remove_file(&tmp);
        return Ok(oid);
    }
    std::fs::rename(&tmp, &path).map_err(|e| ObjectError::Io(e.to_string()))?;
    Ok(oid)
}

/// zlib-compress a loose object without building an intermediate uncompressed buffer.
fn zlib_compress_object(kind: ObjectKind, payload: &[u8]) -> Result<Vec<u8>, ObjectError> {
    let type_name = kind.as_str();
    let size = payload.len().to_string();
    let mut enc = ZlibEncoder::new(Vec::new(), Compression::fast());
    enc.write_all(type_name.as_bytes())
        .map_err(|e| ObjectError::Io(e.to_string()))?;
    enc.write_all(b" ")
        .map_err(|e| ObjectError::Io(e.to_string()))?;
    enc.write_all(size.as_bytes())
        .map_err(|e| ObjectError::Io(e.to_string()))?;
    enc.write_all(&[0])
        .map_err(|e| ObjectError::Io(e.to_string()))?;
    enc.write_all(payload)
        .map_err(|e| ObjectError::Io(e.to_string()))?;
    enc.finish().map_err(|e| ObjectError::Io(e.to_string()))
}

/// Read a loose object from `objects_dir` by OID.
///
/// When `verify_oid` is true, recomputes the content OID (fsck / test honesty).
/// Product store gets use [`read_loose_hot`] — stock Git does not re-hash every
/// loose read on the hot path; the path is already content-addressed.
pub fn read_loose(objects_dir: &Path, oid: &Oid) -> Result<RawObject, ObjectError> {
    read_loose_with_verify(objects_dir, oid, true)
}

/// Hot-path loose read: decode + size check only (Pf4-2 ARCHIVE/CLONE/CHERRY).
pub fn read_loose_hot(objects_dir: &Path, oid: &Oid) -> Result<RawObject, ObjectError> {
    read_loose_with_verify(objects_dir, oid, false)
}

fn read_loose_with_verify(
    objects_dir: &Path,
    oid: &Oid,
    verify_oid: bool,
) -> Result<RawObject, ObjectError> {
    let path = objects_dir.join(loose_rel_path(oid));
    let compressed = std::fs::read(&path).map_err(|e| ObjectError::Io(e.to_string()))?;
    let uncompressed = zlib_decompress(&compressed)?;
    let obj = RawObject::decode_uncompressed(&uncompressed)?;
    if verify_oid {
        let format = match oid.kind() {
            OidKind::Sha1 => ObjectFormat::Sha1,
            OidKind::Sha256 => ObjectFormat::Sha256,
        };
        let got = obj.oid(format)?;
        if got != *oid {
            return Err(ObjectError::OidMismatch {
                expected: oid.to_hex(),
                actual: got.to_hex(),
            });
        }
    }
    Ok(obj)
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ObjectError {
    #[error("unknown object kind: {0}")]
    UnknownKind(String),
    #[error("missing NUL in object header")]
    MissingHeaderNul,
    #[error("object header is not valid UTF-8")]
    HeaderUtf8,
    #[error("object header missing type/size")]
    BadHeader,
    #[error("object size is not a decimal integer")]
    BadSize,
    #[error("payload length {actual} != declared size {declared}")]
    SizeMismatch { declared: usize, actual: usize },
    #[error(transparent)]
    Oid(#[from] OidError),
    #[error("OID mismatch: expected {expected}, got {actual}")]
    OidMismatch { expected: String, actual: String },
    #[error("I/O: {0}")]
    Io(String),
    #[error("bad tree mode: {0}")]
    BadTreeMode(String),
    #[error("bad tree entry: {0}")]
    BadTreeEntry(String),
    #[error("tree entries are not sorted")]
    UnsortedTree,
    #[error("bad commit: {0}")]
    BadCommit(String),
    #[error("bad patch: {0}")]
    BadPatch(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_objects() -> (PathBuf, PathBuf) {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("arks-object-test-{stamp}"));
        let objects = root.join("objects");
        std::fs::create_dir_all(&objects).unwrap();
        (root, objects)
    }

    #[test]
    fn blob_roundtrip_memory() {
        let obj = RawObject::new(ObjectKind::Blob, b"hello");
        let bytes = obj.encode_uncompressed();
        let decoded = RawObject::decode_uncompressed(&bytes).unwrap();
        assert_eq!(decoded, obj);
        let oid = obj.oid(ObjectFormat::Sha1).unwrap();
        assert_eq!(oid.to_hex(), "b6fc4c620b67d95f953a5c1c1230aaab5db5a1b0");
    }

    #[test]
    fn empty_blob_known_oid() {
        let obj = RawObject::new(ObjectKind::Blob, b"");
        assert_eq!(
            obj.oid(ObjectFormat::Sha1).unwrap().to_hex(),
            "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391"
        );
    }

    #[test]
    fn zlib_roundtrip_preserves_bytes() {
        let obj = RawObject::new(ObjectKind::Blob, b"hello\0world");
        let raw = obj.encode_uncompressed();
        let compressed = zlib_compress(&raw).unwrap();
        let back = zlib_decompress(&compressed).unwrap();
        assert_eq!(back, raw);
    }

    #[test]
    fn refuse_unknown_kind() {
        let bad = b"wibble 1\0x";
        assert!(matches!(
            RawObject::decode_uncompressed(bad),
            Err(ObjectError::UnknownKind(_))
        ));
    }

    #[test]
    fn refuse_size_mismatch() {
        let bad = b"blob 2\0x";
        assert!(matches!(
            RawObject::decode_uncompressed(bad),
            Err(ObjectError::SizeMismatch {
                declared: 2,
                actual: 1
            })
        ));
    }

    #[test]
    fn loose_write_read_roundtrip() {
        let (root, objects) = temp_objects();
        let obj = RawObject::new(ObjectKind::Blob, b"hello");
        let oid = write_loose(&objects, ObjectFormat::Sha1, &obj).unwrap();
        let rel = loose_rel_path(&oid);
        let parts: Vec<std::borrow::Cow<'_, str>> =
            rel.iter().map(|c| c.to_string_lossy()).collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], "b6");
        assert_eq!(parts[1], "fc4c620b67d95f953a5c1c1230aaab5db5a1b0");
        let got = read_loose(&objects, &oid).unwrap();
        assert_eq!(got, obj);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn refuse_oid_mismatch_on_corrupt_payload() {
        let (root, objects) = temp_objects();
        let obj = RawObject::new(ObjectKind::Blob, b"hello");
        let oid = write_loose(&objects, ObjectFormat::Sha1, &obj).unwrap();
        let path = objects.join(loose_rel_path(&oid));
        // Compress a different object into the same path.
        let other = RawObject::new(ObjectKind::Blob, b"nope");
        let compressed = zlib_compress(&other.encode_uncompressed()).unwrap();
        std::fs::write(&path, compressed).unwrap();
        assert!(matches!(
            read_loose(&objects, &oid),
            Err(ObjectError::OidMismatch { .. })
        ));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn claimed_kinds_opaque_payload_roundtrip() {
        // M1 claims kind tokens + framing; payload grammar for tree/commit/tag
        // is deferred — opaque bytes must still round-trip header/OID binding.
        for kind in [
            ObjectKind::Blob,
            ObjectKind::Tree,
            ObjectKind::Commit,
            ObjectKind::Tag,
        ] {
            let payload = b"opaque-m1-payload\0with-nul";
            let obj = RawObject::new(kind, payload);
            let bytes = obj.encode_uncompressed();
            let decoded = RawObject::decode_uncompressed(&bytes).unwrap();
            assert_eq!(decoded, obj);
            let oid = obj.oid(ObjectFormat::Sha1).unwrap();
            assert_eq!(oid.to_hex().len(), 40);
            // Kind token participates in identity.
            if kind != ObjectKind::Blob {
                let as_blob = RawObject::new(ObjectKind::Blob, payload);
                assert_ne!(oid, as_blob.oid(ObjectFormat::Sha1).unwrap());
            }
        }
    }

    #[test]
    fn refuse_missing_header_nul() {
        assert!(matches!(
            RawObject::decode_uncompressed(b"blob 5hello"),
            Err(ObjectError::MissingHeaderNul)
        ));
    }

    #[test]
    fn refuse_bad_header_and_size() {
        assert!(matches!(
            RawObject::decode_uncompressed(b"blob\0x"),
            Err(ObjectError::BadHeader)
        ));
        assert!(matches!(
            RawObject::decode_uncompressed(b"blob abc\0x"),
            Err(ObjectError::BadSize)
        ));
    }

    #[test]
    fn refuse_corrupt_zlib() {
        assert!(matches!(
            zlib_decompress(b"not-zlib"),
            Err(ObjectError::Io(_))
        ));
    }

    #[test]
    fn write_loose_idempotent_same_oid() {
        let (root, objects) = temp_objects();
        let obj = RawObject::new(ObjectKind::Blob, b"hello");
        let oid1 = write_loose(&objects, ObjectFormat::Sha1, &obj).unwrap();
        let oid2 = write_loose(&objects, ObjectFormat::Sha1, &obj).unwrap();
        assert_eq!(oid1, oid2);
        assert_eq!(read_loose(&objects, &oid1).unwrap(), obj);
        let _ = std::fs::remove_dir_all(&root);
    }
}
