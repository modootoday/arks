//! Trailing index checksum.

use crate::error::IndexError;
use arks_hash::{ObjectFormat, hash_bytes};
use arks_oid::OidKind;

pub(crate) fn object_format(kind: OidKind) -> ObjectFormat {
    match kind {
        OidKind::Sha1 => ObjectFormat::Sha1,
        OidKind::Sha256 => ObjectFormat::Sha256,
    }
}

pub(crate) fn checksum_len(kind: OidKind) -> usize {
    kind.digest_len()
}

pub(crate) fn read_and_verify_checksum(
    data: &[u8],
    content_end: usize,
    hash_kind: OidKind,
) -> Result<(), IndexError> {
    let need = checksum_len(hash_kind);
    if data.len() != content_end + need {
        if data.len() < content_end + need {
            return Err(IndexError::Truncated);
        }
        return Err(IndexError::Corrupt("trailing bytes after checksum".into()));
    }
    let digest = hash_bytes(object_format(hash_kind), &data[..content_end]);
    if digest.as_bytes() != &data[content_end..] {
        return Err(IndexError::Checksum);
    }
    Ok(())
}

pub(crate) fn append_checksum(buf: &mut Vec<u8>, hash_kind: OidKind) {
    let digest = hash_bytes(object_format(hash_kind), buf);
    buf.extend_from_slice(digest.as_bytes());
}
