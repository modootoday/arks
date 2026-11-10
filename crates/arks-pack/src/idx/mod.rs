//! Idx version 2.
//!
//! Primary: <https://git-scm.com/docs/gitformat-pack> (version 2 pack-*.idx).
//! Magic is `\377tOc`, then version 2, fanout, names, CRC32, 4-byte offsets,
//! optional 8-byte offsets, pack checksum, and idx checksum.

mod decode;
mod encode;

use crate::error::PackError;
use arks_hash::ObjectFormat;
use arks_oid::Oid;

pub(crate) const IDX_MAGIC: [u8; 4] = [0xff, b't', b'O', b'c'];
pub(crate) const VERSION: u32 = 2;
/// Offsets at or above this value use the 8-byte large-offset table.
pub(crate) const LARGE_OFFSET_MIN: u64 = 0x8000_0000;

/// One idx record: object name, packed-entry CRC32, pack offset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackIndexEntry {
    pub oid: Oid,
    pub crc32: u32,
    pub offset: u64,
}

/// Decoded idx version 2.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackIndex {
    pub format: ObjectFormat,
    pub fanout: [u32; 256],
    pub entries: Vec<PackIndexEntry>,
    pub pack_checksum: Vec<u8>,
}

impl PackIndex {
    /// Sort `entries` by object name and build the fanout table.
    pub fn from_entries(
        format: ObjectFormat,
        mut entries: Vec<PackIndexEntry>,
        pack_checksum: Vec<u8>,
    ) -> Result<Self, PackError> {
        crate::checksum::require_digest_len(&pack_checksum, format)?;
        for entry in &entries {
            if entry.oid.kind() != crate::checksum::oid_kind(format) {
                return Err(PackError::OidWidth);
            }
        }
        entries.sort_by(|a, b| a.oid.as_bytes().cmp(b.oid.as_bytes()));
        if entries.windows(2).any(|w| w[0].oid == w[1].oid) {
            return Err(PackError::DuplicateOid);
        }
        let fanout = fanout_for(&entries)?;
        Ok(Self {
            format,
            fanout,
            entries,
            pack_checksum,
        })
    }

    /// Binary search within the fanout bucket for `oid`.
    pub fn lookup(&self, oid: &Oid) -> Option<&PackIndexEntry> {
        if oid.kind() != crate::checksum::oid_kind(self.format) {
            return None;
        }
        let bytes = oid.as_bytes();
        if bytes.is_empty() {
            return None;
        }
        let b = usize::from(bytes[0]);
        let start = if b == 0 {
            0
        } else {
            self.fanout[b - 1] as usize
        };
        let end = self.fanout[b] as usize;
        if start > end || end > self.entries.len() {
            return None;
        }
        let slice = &self.entries[start..end];
        slice
            .binary_search_by(|entry| entry.oid.as_bytes().cmp(bytes))
            .ok()
            .map(|i| &slice[i])
    }
}

pub(crate) fn fanout_for(entries: &[PackIndexEntry]) -> Result<[u32; 256], PackError> {
    let mut counts = [0u32; 256];
    for entry in entries {
        let first = usize::from(entry.oid.as_bytes()[0]);
        counts[first] = counts[first]
            .checked_add(1)
            .ok_or(PackError::CountOverflow)?;
    }
    let mut fanout = [0u32; 256];
    let mut acc = 0u32;
    for i in 0..256 {
        acc = acc.checked_add(counts[i]).ok_or(PackError::CountOverflow)?;
        fanout[i] = acc;
    }
    Ok(fanout)
}

pub(crate) fn read_u32(data: &[u8], at: usize) -> Result<u32, PackError> {
    let end = at.checked_add(4).ok_or(PackError::Truncated)?;
    let s = data.get(at..end).ok_or(PackError::Truncated)?;
    Ok(u32::from_be_bytes([s[0], s[1], s[2], s[3]]))
}

pub(crate) fn read_u64(data: &[u8], at: usize) -> Result<u64, PackError> {
    let end = at.checked_add(8).ok_or(PackError::Truncated)?;
    let s = data.get(at..end).ok_or(PackError::Truncated)?;
    Ok(u64::from_be_bytes([
        s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7],
    ]))
}
