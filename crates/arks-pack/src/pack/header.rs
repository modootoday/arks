//! Pack header: signature `PACK`, version, object count.
//!
//! Primary: <https://git-scm.com/docs/gitformat-pack>

use crate::error::PackError;

pub(crate) const SIGNATURE: &[u8; 4] = b"PACK";
pub(crate) const VERSION: u32 = 2;
pub(crate) const HEADER_LEN: usize = 12;

pub(crate) fn parse_header(data: &[u8]) -> Result<(u32, u32), PackError> {
    if data.len() < HEADER_LEN {
        return Err(PackError::Truncated);
    }
    if data[..4] != SIGNATURE[..] {
        return Err(PackError::BadPackMagic);
    }
    let version = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
    let count = u32::from_be_bytes([data[8], data[9], data[10], data[11]]);
    if version != VERSION {
        return Err(PackError::UnsupportedPackVersion(version));
    }
    Ok((version, count))
}

pub(crate) fn write_header(buf: &mut Vec<u8>, count: u32) {
    buf.extend_from_slice(SIGNATURE);
    buf.extend_from_slice(&VERSION.to_be_bytes());
    buf.extend_from_slice(&count.to_be_bytes());
}
