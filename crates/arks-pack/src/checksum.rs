//! Object-format trailers for pack and idx bytes.
//!
//! Primary: <https://git-scm.com/docs/gitformat-pack> (checksums and object IDs).

use crate::error::PackError;
use arks_hash::{ObjectFormat, hash_bytes};
use arks_oid::OidKind;

pub(crate) fn oid_kind(format: ObjectFormat) -> OidKind {
    match format {
        ObjectFormat::Sha1 => OidKind::Sha1,
        ObjectFormat::Sha256 => OidKind::Sha256,
    }
}

pub(crate) fn append_checksum(buf: &mut Vec<u8>, format: ObjectFormat) {
    let digest = hash_bytes(format, buf).expect("pack checksum hash");
    buf.extend_from_slice(digest.as_bytes());
}

pub(crate) fn checksum_matches(body: &[u8], sum: &[u8], format: ObjectFormat) -> bool {
    hash_bytes(format, body)
        .map(|d| d.as_bytes() == sum)
        .unwrap_or(false)
}

pub(crate) fn require_digest_len(bytes: &[u8], format: ObjectFormat) -> Result<(), PackError> {
    if bytes.len() != format.digest_len() {
        return Err(PackError::BadChecksumLength);
    }
    Ok(())
}
