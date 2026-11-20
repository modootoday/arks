//! Pack version 2 writer with deltas, idx version 1 / 2, `.rev` and `.mtimes`.
//!
//! Primary: <https://git-scm.com/docs/gitformat-pack> (pack entries,
//! `OBJ_OFS_DELTA` / `OBJ_REF_DELTA`, delta instruction format, idx versions 1
//! and 2, reverse index, cruft-pack mtimes).

use crate::checksum;
use crate::delta;
use crate::error::PackError;
use crate::idx::{PackIndex, PackIndexEntry};
use crate::pack::entry::{type_of_kind, write_ofs_offset, write_type_and_size};
use arks_hash::ObjectFormat;
use arks_object::ObjectKind;
use arks_oid::Oid;
use flate2::Compression;
use flate2::write::ZlibEncoder;
use std::collections::HashMap;
use std::io::Write;

const OBJ_OFS_DELTA: u8 = 6;
const OBJ_REF_DELTA: u8 = 7;

/// Streaming pack builder: entries are appended in order, then [`finish`].
///
/// [`finish`]: PackWriter::finish
pub struct PackWriter {
    format: ObjectFormat,
    level: Compression,
    buf: Vec<u8>,
    entries: Vec<PackIndexEntry>,
    offsets: HashMap<Oid, u64>,
}

/// A finished pack: bytes, idx entries in pack order, and the trailer.
pub struct WrittenPack {
    pub pack: Vec<u8>,
    pub entries: Vec<PackIndexEntry>,
    pub checksum: Vec<u8>,
}

impl PackWriter {
    /// `count` objects will follow; `level` is the zlib level (0..=9, or
    /// `None` for the zlib default).
    pub fn new(format: ObjectFormat, count: u32, level: Option<u32>) -> Self {
        let mut buf = Vec::new();
        crate::pack::header::write_header(&mut buf, count);
        Self {
            format,
            level: level.map_or(Compression::default(), Compression::new),
            buf,
            entries: Vec::new(),
            offsets: HashMap::new(),
        }
    }

    /// Current byte length (the next entry's offset).
    pub fn len(&self) -> u64 {
        self.buf.len() as u64
    }

    /// Whether no entry was written yet (only the header).
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Offset of an entry already written.
    pub fn offset_of(&self, oid: &Oid) -> Option<u64> {
        self.offsets.get(oid).copied()
    }

    fn deflate(&self, data: &[u8]) -> Result<Vec<u8>, PackError> {
        let mut enc = ZlibEncoder::new(Vec::new(), self.level);
        enc.write_all(data)
            .map_err(|e| PackError::Zlib(e.to_string()))?;
        enc.finish().map_err(|e| PackError::Zlib(e.to_string()))
    }

    fn record(&mut self, oid: Oid, start: usize) {
        let crc32 = crc32fast::hash(&self.buf[start..]);
        self.offsets.insert(oid, start as u64);
        self.entries.push(PackIndexEntry {
            oid,
            crc32,
            offset: start as u64,
        });
    }

    /// Append a whole (undeltified) object.
    pub fn add_full(
        &mut self,
        oid: Oid,
        kind: ObjectKind,
        payload: &[u8],
    ) -> Result<(), PackError> {
        let start = self.buf.len();
        write_type_and_size(&mut self.buf, type_of_kind(kind), payload.len() as u64);
        let z = self.deflate(payload)?;
        self.buf.extend_from_slice(&z);
        self.record(oid, start);
        Ok(())
    }

    /// Append a delta against a base written earlier in this pack.
    pub fn add_ofs_delta(&mut self, oid: Oid, base: &Oid, delta: &[u8]) -> Result<(), PackError> {
        let base_off = self.offset_of(base).ok_or(PackError::BadOfsOffset)?;
        let start = self.buf.len();
        write_type_and_size(&mut self.buf, OBJ_OFS_DELTA, delta.len() as u64);
        write_ofs_offset(&mut self.buf, start as u64 - base_off)?;
        let z = self.deflate(delta)?;
        self.buf.extend_from_slice(&z);
        self.record(oid, start);
        Ok(())
    }

    /// Append a delta against a base named by object id (in this pack or,
    /// for a thin pack, outside it).
    pub fn add_ref_delta(&mut self, oid: Oid, base: &Oid, delta: &[u8]) -> Result<(), PackError> {
        let start = self.buf.len();
        write_type_and_size(&mut self.buf, OBJ_REF_DELTA, delta.len() as u64);
        self.buf.extend_from_slice(base.as_bytes());
        let z = self.deflate(delta)?;
        self.buf.extend_from_slice(&z);
        self.record(oid, start);
        Ok(())
    }

    /// Append the trailer checksum.
    pub fn finish(mut self) -> WrittenPack {
        checksum::append_checksum(&mut self.buf, self.format);
        let n = self.format.digest_len();
        let checksum = self.buf[self.buf.len() - n..].to_vec();
        WrittenPack {
            pack: self.buf,
            entries: self.entries,
            checksum,
        }
    }
}

/// A delta (gitformat-pack instruction stream) turning `base` into
/// `target`, or `None` when it would not be smaller than `max_size`.
pub fn create_delta(base: &[u8], target: &[u8], max_size: usize) -> Option<Vec<u8>> {
    delta::create(base, target, max_size)
}

/// Encode an idx in version 1 (fanout, then `offset(4) + name` records).
pub fn encode_idx_v1(index: &PackIndex) -> Result<Vec<u8>, PackError> {
    let mut buf = Vec::new();
    for count in index.fanout {
        buf.extend_from_slice(&count.to_be_bytes());
    }
    for e in &index.entries {
        let off = u32::try_from(e.offset).map_err(|_| PackError::OffsetOverflow)?;
        buf.extend_from_slice(&off.to_be_bytes());
        buf.extend_from_slice(e.oid.as_bytes());
    }
    buf.extend_from_slice(&index.pack_checksum);
    checksum::append_checksum(&mut buf, index.format);
    Ok(buf)
}

fn hash_id(format: ObjectFormat) -> u32 {
    match format {
        ObjectFormat::Sha1 => 1,
        ObjectFormat::Sha256 => 2,
    }
}

/// The `.rev` reverse index: idx positions in pack-offset order.
pub fn encode_rev(index: &PackIndex) -> Vec<u8> {
    let mut order: Vec<u32> = (0..index.entries.len() as u32).collect();
    order.sort_by_key(|i| index.entries[*i as usize].offset);
    let mut buf = Vec::new();
    buf.extend_from_slice(b"RIDX");
    buf.extend_from_slice(&1u32.to_be_bytes());
    buf.extend_from_slice(&hash_id(index.format).to_be_bytes());
    for i in order {
        buf.extend_from_slice(&i.to_be_bytes());
    }
    buf.extend_from_slice(&index.pack_checksum);
    checksum::append_checksum(&mut buf, index.format);
    buf
}

/// The `.mtimes` file of a cruft pack: one mtime per idx position.
pub fn encode_mtimes(index: &PackIndex, mtimes: &HashMap<Oid, u32>) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(b"MTME");
    buf.extend_from_slice(&1u32.to_be_bytes());
    buf.extend_from_slice(&hash_id(index.format).to_be_bytes());
    for e in &index.entries {
        buf.extend_from_slice(&mtimes.get(&e.oid).copied().unwrap_or(0).to_be_bytes());
    }
    buf.extend_from_slice(&index.pack_checksum);
    checksum::append_checksum(&mut buf, index.format);
    buf
}

/// Trailer checksum of `body` in `format` (for files such as bitmaps).
pub fn trailer(body: &mut Vec<u8>, format: ObjectFormat) {
    checksum::append_checksum(body, format);
}
