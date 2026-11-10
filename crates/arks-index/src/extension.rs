//! Optional index extensions (`gitformat-index`).

use crate::error::IndexError;

/// Raw extension payload preserved for round-trip of known-safe optional blocks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexExtension {
    pub signature: [u8; 4],
    pub data: Vec<u8>,
}

const LINK: &[u8; 4] = b"LINK";
const SDIR: &[u8; 4] = b"SDIR";

pub(crate) fn read_extensions(
    data: &[u8],
    mut offset: usize,
    checksum_len: usize,
) -> Result<(Vec<IndexExtension>, usize), IndexError> {
    let end = data.len().saturating_sub(checksum_len);
    let mut out = Vec::new();
    while offset < end {
        if offset + 8 > end {
            return Err(IndexError::Truncated);
        }
        let mut sig = [0u8; 4];
        sig.copy_from_slice(&data[offset..offset + 4]);
        offset += 4;
        let size = u32::from_be_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
        offset += 4;
        if offset + size > end {
            return Err(IndexError::Truncated);
        }
        if &sig == LINK {
            return Err(IndexError::RefusedExtension("LINK split-index".into()));
        }
        if &sig == SDIR {
            return Err(IndexError::RefusedExtension("SDIR sparse directory".into()));
        }
        if !sig[0].is_ascii_uppercase() {
            return Err(IndexError::Corrupt(format!(
                "invalid extension signature {:?}",
                std::str::from_utf8(&sig)
            )));
        }
        let payload = data[offset..offset + size].to_vec();
        offset += size;
        out.push(IndexExtension {
            signature: sig,
            data: payload,
        });
    }
    Ok((out, offset))
}

pub(crate) fn write_extensions(buf: &mut Vec<u8>, extensions: &[IndexExtension]) {
    for ext in extensions {
        buf.extend_from_slice(&ext.signature);
        buf.extend_from_slice(&(ext.data.len() as u32).to_be_bytes());
        buf.extend_from_slice(&ext.data);
    }
}

/// Extensions safe to preserve on rewrite (optional, non-split).
pub fn is_passthrough_safe(signature: &[u8; 4]) -> bool {
    signature != LINK && signature != SDIR
}
