//! Idx version 2 decoder.
//!
//! Primary: <https://git-scm.com/docs/gitformat-pack>

use super::{IDX_MAGIC, PackIndex, PackIndexEntry, VERSION, fanout_for, read_u32, read_u64};
use crate::checksum::{self, checksum_matches};
use crate::error::PackError;
use arks_hash::ObjectFormat;
use arks_oid::Oid;

impl PackIndex {
    pub fn decode(bytes: &[u8], format: ObjectFormat) -> Result<Self, PackError> {
        let hash_len = format.digest_len();
        if bytes.len() < 8 {
            return Err(PackError::Truncated);
        }
        if bytes[..4] != IDX_MAGIC {
            return Err(PackError::BadIdxMagic);
        }
        let version = read_u32(bytes, 4)?;
        if version != VERSION {
            return Err(PackError::UnsupportedIdxVersion(version));
        }
        if bytes.len() < 8 + 256 * 4 + hash_len * 2 {
            return Err(PackError::Truncated);
        }

        let mut fanout = [0u32; 256];
        for i in 0..256 {
            fanout[i] = read_u32(bytes, 8 + i * 4)?;
            if i > 0 && fanout[i] < fanout[i - 1] {
                return Err(PackError::Fanout);
            }
        }
        let n = fanout[255] as usize;
        let names_at: usize = 8 + 256 * 4;
        let names_len = n.checked_mul(hash_len).ok_or(PackError::CountOverflow)?;
        let crc_at = names_at
            .checked_add(names_len)
            .ok_or(PackError::CountOverflow)?;
        let crc_len = n.checked_mul(4).ok_or(PackError::CountOverflow)?;
        let off_at = crc_at
            .checked_add(crc_len)
            .ok_or(PackError::CountOverflow)?;
        let off_len = n.checked_mul(4).ok_or(PackError::CountOverflow)?;
        let large_at = off_at
            .checked_add(off_len)
            .ok_or(PackError::CountOverflow)?;
        let trailer = hash_len.checked_mul(2).ok_or(PackError::CountOverflow)?;
        if bytes.len() < large_at.saturating_add(trailer) {
            return Err(PackError::Truncated);
        }
        let sum_at = bytes.len() - hash_len;
        let pack_sum_at = sum_at - hash_len;
        if pack_sum_at < large_at {
            return Err(PackError::Truncated);
        }
        let large_bytes = pack_sum_at - large_at;
        if large_bytes % 8 != 0 {
            return Err(PackError::CorruptIdx);
        }
        let large_count = large_bytes / 8;

        if !checksum_matches(&bytes[..sum_at], &bytes[sum_at..], format) {
            return Err(PackError::IdxChecksum);
        }

        let mut names = Vec::with_capacity(n);
        for i in 0..n {
            let at = names_at + i * hash_len;
            let oid = Oid::from_bytes(checksum::oid_kind(format), &bytes[at..at + hash_len])?;
            names.push(oid);
        }
        if names.windows(2).any(|w| w[0].as_bytes() >= w[1].as_bytes()) {
            return Err(PackError::UnsortedNames);
        }

        let mut crcs = Vec::with_capacity(n);
        for i in 0..n {
            crcs.push(read_u32(bytes, crc_at + i * 4)?);
        }

        let mut large_offsets = Vec::with_capacity(large_count);
        for i in 0..large_count {
            large_offsets.push(read_u64(bytes, large_at + i * 8)?);
        }

        let mut entries = Vec::with_capacity(n);
        for i in 0..n {
            let raw = read_u32(bytes, off_at + i * 4)?;
            let offset = if raw & 0x8000_0000 != 0 {
                let index = (raw & 0x7fff_ffff) as usize;
                if index >= large_offsets.len() {
                    return Err(PackError::LargeOffset);
                }
                large_offsets[index]
            } else {
                u64::from(raw)
            };
            entries.push(PackIndexEntry {
                oid: names[i],
                crc32: crcs[i],
                offset,
            });
        }

        let computed = fanout_for(&entries)?;
        if computed != fanout {
            return Err(PackError::Fanout);
        }

        Ok(Self {
            format,
            fanout,
            entries,
            pack_checksum: bytes[pack_sum_at..sum_at].to_vec(),
        })
    }
}
