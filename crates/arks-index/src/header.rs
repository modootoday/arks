//! Index header (`DIRC` + version + entry count).

use crate::error::IndexError;

pub const SIGNATURE: &[u8; 4] = b"DIRC";

const HEADER_LEN: usize = 12;

pub(crate) fn parse_header(data: &[u8]) -> Result<(u32, u32, usize), IndexError> {
    if data.len() < HEADER_LEN {
        return Err(IndexError::Truncated);
    }
    if &data[0..4] != SIGNATURE {
        return Err(IndexError::BadSignature);
    }
    let version = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
    let count = u32::from_be_bytes([data[8], data[9], data[10], data[11]]);
    Ok((version, count, HEADER_LEN))
}

pub(crate) fn write_header(buf: &mut Vec<u8>, version: u32, count: u32) {
    buf.extend_from_slice(SIGNATURE);
    buf.extend_from_slice(&version.to_be_bytes());
    buf.extend_from_slice(&count.to_be_bytes());
}

pub(crate) fn validate_version(version: u32) -> Result<(), IndexError> {
    match version {
        2..=4 => Ok(()),
        other => Err(IndexError::UnsupportedVersion(other)),
    }
}
