//! Undeltified pack version 2 encoder and matching idx version 2.
//!
//! Primary: <https://git-scm.com/docs/gitformat-pack>
//! Packed bytes are the raw object payload (no `type SP size NUL` prefix).
//! The object id is still the hash of that loose header plus the payload.

use super::entry::{self, type_of_kind, zlib_compress};
use super::header;
use crate::checksum;
use crate::error::PackError;
use crate::idx::{PackIndex, PackIndexEntry};
use arks_hash::ObjectFormat;
use arks_object::RawObject;

/// Undeltified pack version 2 bytes and the matching idx version 2 bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodedPack {
    pub pack: Vec<u8>,
    pub idx: Vec<u8>,
}

impl EncodedPack {
    pub fn from_undeltified(
        format: ObjectFormat,
        objects: &[RawObject],
    ) -> Result<Self, PackError> {
        let (pack, entries) = encode_undeltified(format, objects)?;
        let hash_len = format.digest_len();
        let pack_checksum = pack[pack.len() - hash_len..].to_vec();
        let index = PackIndex::from_entries(format, entries, pack_checksum)?;
        let idx = index.encode()?;
        Ok(Self { pack, idx })
    }
}

fn encode_undeltified(
    format: ObjectFormat,
    objects: &[RawObject],
) -> Result<(Vec<u8>, Vec<PackIndexEntry>), PackError> {
    let count = u32::try_from(objects.len()).map_err(|_| PackError::CountOverflow)?;
    let mut buf = Vec::new();
    header::write_header(&mut buf, count);
    let mut entries = Vec::with_capacity(objects.len());
    for object in objects {
        let oid = object.oid(format)?;
        let offset = buf.len() as u64;
        let start = buf.len();
        entry::write_type_and_size(
            &mut buf,
            type_of_kind(object.kind),
            object.payload.len() as u64,
        );
        // gitformat-pack: undeltified compressed data is the raw payload.
        let compressed = zlib_compress(&object.payload)?;
        buf.extend_from_slice(&compressed);
        let crc32 = crc32fast::hash(&buf[start..]);
        entries.push(PackIndexEntry { oid, crc32, offset });
    }
    checksum::append_checksum(&mut buf, format);
    Ok((buf, entries))
}
