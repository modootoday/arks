//! Idx version 2 encoder.
//!
//! Primary: <https://git-scm.com/docs/gitformat-pack>

use super::{IDX_MAGIC, LARGE_OFFSET_MIN, PackIndex, VERSION, fanout_for};
use crate::checksum;
use crate::error::PackError;

impl PackIndex {
    pub fn encode(&self) -> Result<Vec<u8>, PackError> {
        checksum::require_digest_len(&self.pack_checksum, self.format)?;
        for entry in &self.entries {
            if entry.oid.kind() != checksum::oid_kind(self.format) {
                return Err(PackError::OidWidth);
            }
        }
        if self
            .entries
            .windows(2)
            .any(|w| w[0].oid.as_bytes() >= w[1].oid.as_bytes())
        {
            return Err(PackError::UnsortedNames);
        }
        let fanout = fanout_for(&self.entries)?;
        if fanout != self.fanout {
            return Err(PackError::Fanout);
        }

        // Fanout(256×4) + names + crc + offsets + checksums — pre-size for clone ingest.
        let n = self.entries.len();
        let oid_len = self.format.digest_len();
        let mut buf = Vec::with_capacity(8 + 256 * 4 + n * (oid_len + 4 + 4) + 64);
        buf.extend_from_slice(&IDX_MAGIC);
        buf.extend_from_slice(&VERSION.to_be_bytes());
        for count in fanout {
            buf.extend_from_slice(&count.to_be_bytes());
        }
        for entry in &self.entries {
            buf.extend_from_slice(entry.oid.as_bytes());
        }
        for entry in &self.entries {
            buf.extend_from_slice(&entry.crc32.to_be_bytes());
        }

        let mut large = Vec::new();
        for entry in &self.entries {
            if entry.offset >= LARGE_OFFSET_MIN {
                let index = u32::try_from(large.len()).map_err(|_| PackError::OffsetOverflow)?;
                if index > 0x7fff_ffff {
                    return Err(PackError::OffsetOverflow);
                }
                let marked = 0x8000_0000 | index;
                buf.extend_from_slice(&marked.to_be_bytes());
                large.push(entry.offset);
            } else {
                let off = u32::try_from(entry.offset).map_err(|_| PackError::OffsetOverflow)?;
                buf.extend_from_slice(&off.to_be_bytes());
            }
        }
        for offset in large {
            buf.extend_from_slice(&offset.to_be_bytes());
        }
        buf.extend_from_slice(&self.pack_checksum);
        checksum::append_checksum(&mut buf, self.format);
        Ok(buf)
    }
}
