//! Pack object entry header: type+size varint, ofs-delta offset, zlib body.
//!
//! Primary: <https://git-scm.com/docs/gitformat-pack>
//! Type in the high 3 bits of the low nibble's neighbor, size in 4+7n bits.
//! OFS offsets use the offset encoding (not the delta size encoding).

use crate::error::PackError;
use arks_object::ObjectKind;
use flate2::write::ZlibEncoder;
use flate2::{Compression, Decompress, FlushDecompress, Status};
use std::io::Write;

pub(crate) const OBJ_COMMIT: u8 = 1;
pub(crate) const OBJ_TREE: u8 = 2;
pub(crate) const OBJ_BLOB: u8 = 3;
pub(crate) const OBJ_TAG: u8 = 4;
pub(crate) const OBJ_OFS_DELTA: u8 = 6;
pub(crate) const OBJ_REF_DELTA: u8 = 7;

pub(crate) fn type_of_kind(kind: ObjectKind) -> u8 {
    match kind {
        ObjectKind::Commit => OBJ_COMMIT,
        ObjectKind::Tree => OBJ_TREE,
        ObjectKind::Blob => OBJ_BLOB,
        ObjectKind::Tag => OBJ_TAG,
    }
}

pub(crate) fn kind_of_type(type_id: u8) -> Result<ObjectKind, PackError> {
    match type_id {
        OBJ_COMMIT => Ok(ObjectKind::Commit),
        OBJ_TREE => Ok(ObjectKind::Tree),
        OBJ_BLOB => Ok(ObjectKind::Blob),
        OBJ_TAG => Ok(ObjectKind::Tag),
        5 => Err(PackError::ReservedType),
        other => Err(PackError::InvalidType(other)),
    }
}

pub(crate) fn validate_type(type_id: u8) -> Result<(), PackError> {
    match type_id {
        OBJ_COMMIT | OBJ_TREE | OBJ_BLOB | OBJ_TAG | OBJ_OFS_DELTA | OBJ_REF_DELTA => Ok(()),
        5 => Err(PackError::ReservedType),
        other => Err(PackError::InvalidType(other)),
    }
}

/// `(type, size, header_len)` from the pack type-and-size varint.
pub(crate) fn read_type_and_size(data: &[u8]) -> Result<(u8, u64, usize), PackError> {
    if data.is_empty() {
        return Err(PackError::Truncated);
    }
    let b0 = data[0];
    let type_id = (b0 >> 4) & 0x07;
    let mut size = u64::from(b0 & 0x0f);
    let mut shift = 4u32;
    let mut i = 1usize;
    let mut cont = b0 & 0x80 != 0;
    while cont {
        if i >= data.len() {
            return Err(PackError::Truncated);
        }
        let b = data[i];
        i += 1;
        if shift >= 64 {
            return Err(PackError::SizeOverflow);
        }
        let bits = u64::from(b & 0x7f);
        let add = bits.checked_shl(shift).ok_or(PackError::SizeOverflow)?;
        size = size.checked_add(add).ok_or(PackError::SizeOverflow)?;
        cont = b & 0x80 != 0;
        if cont {
            shift = shift.checked_add(7).ok_or(PackError::SizeOverflow)?;
        }
    }
    Ok((type_id, size, i))
}

pub(crate) fn write_type_and_size(buf: &mut Vec<u8>, type_id: u8, mut size: u64) {
    let mut c = (type_id << 4) | ((size as u8) & 0x0f);
    size >>= 4;
    while size > 0 {
        buf.push(c | 0x80);
        c = (size as u8) & 0x7f;
        size >>= 7;
    }
    buf.push(c);
}

/// Decode an ofs-delta relative offset. Returns `(distance_back, bytes_consumed)`.
///
/// gitformat-pack offset encoding: 7-bit groups, and for every continuation
/// byte the accumulator is incremented before the next shift.
pub(crate) fn read_ofs_offset(data: &[u8]) -> Result<(u64, usize), PackError> {
    if data.is_empty() {
        return Err(PackError::Truncated);
    }
    let mut i = 0usize;
    let mut c = data[i];
    i += 1;
    let mut ofs = u64::from(c & 0x7f);
    while c & 0x80 != 0 {
        ofs = ofs.checked_add(1).ok_or(PackError::OffsetOverflow)?;
        if ofs > (u64::MAX >> 7) {
            return Err(PackError::OffsetOverflow);
        }
        if i >= data.len() {
            return Err(PackError::Truncated);
        }
        c = data[i];
        i += 1;
        ofs = (ofs << 7) | u64::from(c & 0x7f);
    }
    Ok((ofs, i))
}

/// M4 encode does not emit ofs-deltas; unit tests use this to build fixtures.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn write_ofs_offset(buf: &mut Vec<u8>, mut offset: u64) -> Result<(), PackError> {
    if offset == 0 {
        return Err(PackError::BadOfsOffset);
    }
    let mut tmp = [0u8; 16];
    let mut n = 0usize;
    tmp[n] = (offset & 0x7f) as u8;
    n += 1;
    offset >>= 7;
    while offset > 0 {
        offset -= 1;
        if n >= tmp.len() {
            return Err(PackError::OffsetOverflow);
        }
        tmp[n] = 0x80 | ((offset & 0x7f) as u8);
        n += 1;
        offset >>= 7;
    }
    while n > 0 {
        n -= 1;
        buf.push(tmp[n]);
    }
    Ok(())
}

pub(crate) fn zlib_compress(data: &[u8]) -> Result<Vec<u8>, PackError> {
    // OID / pack SHA cover uncompressed bytes; any valid zlib level is legal.
    // Prefer fast — matches loose-object store (arks-object) for bundle/repack.
    let mut enc = ZlibEncoder::new(Vec::new(), Compression::fast());
    enc.write_all(data)
        .map_err(|e| PackError::Zlib(e.to_string()))?;
    enc.finish().map_err(|e| PackError::Zlib(e.to_string()))
}

/// Inflate one zlib stream. Returns `(bytes, input_consumed)`.
///
/// Thin wrapper for tests / one-shot callers; pack decode uses [`inflate_with`].
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn inflate(input: &[u8], expected: usize) -> Result<(Vec<u8>, usize), PackError> {
    let mut dec = Decompress::new(true);
    inflate_with(&mut dec, input, expected)
}

/// Inflate into a reusable decompressor (pack decode hot path — no per-entry alloc,
/// no `resize(..., 0)` zero-fill; `decompress_vec` writes spare capacity only).
pub(crate) fn inflate_with(
    dec: &mut Decompress,
    input: &[u8],
    expected: usize,
) -> Result<(Vec<u8>, usize), PackError> {
    dec.reset(true);
    if expected == 0 {
        let mut dummy = [0u8; 1];
        let status = dec
            .decompress(input, &mut dummy, FlushDecompress::Finish)
            .map_err(|e| PackError::Zlib(e.to_string()))?;
        if status != Status::StreamEnd || dec.total_out() != 0 {
            return Err(PackError::Zlib("empty inflate did not end".into()));
        }
        let consumed = usize::try_from(dec.total_in()).map_err(|_| PackError::SizeOverflow)?;
        if consumed == 0 {
            return Err(PackError::Zlib("decoder consumed no bytes".into()));
        }
        return Ok((Vec::new(), consumed));
    }
    let mut out = Vec::new();
    out.try_reserve_exact(expected)
        .map_err(|_| PackError::SizeOverflow)?;
    let status = dec
        .decompress_vec(input, &mut out, FlushDecompress::Finish)
        .map_err(|e| PackError::Zlib(e.to_string()))?;
    let produced = out.len();
    let consumed = usize::try_from(dec.total_in()).map_err(|_| PackError::SizeOverflow)?;
    if status != Status::StreamEnd {
        return Err(PackError::Zlib("incomplete zlib stream".into()));
    }
    if produced != expected {
        return Err(PackError::SizeMismatch {
            declared: expected as u64,
            actual: produced,
        });
    }
    if consumed == 0 {
        return Err(PackError::Zlib("decoder consumed no bytes".into()));
    }
    Ok((out, consumed))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_size_roundtrip_widths() {
        for type_id in [1u8, 2, 3, 4, 6, 7] {
            for size in [0u64, 1, 15, 16, 127, 128, 4096, (1 << 32) + 3] {
                let mut buf = Vec::new();
                write_type_and_size(&mut buf, type_id, size);
                let (got_type, got_size, n) = read_type_and_size(&buf).unwrap();
                assert_eq!(got_type, type_id);
                assert_eq!(got_size, size);
                assert_eq!(n, buf.len());
            }
        }
    }

    #[test]
    fn ofs_offset_roundtrip() {
        let samples = [
            1u64,
            2,
            127,
            128,
            129,
            255,
            256,
            16383,
            16384,
            1_000_000,
            u32::MAX as u64,
            (1u64 << 32) + 7,
            (1u64 << 40) + 9,
        ];
        for offset in samples {
            let mut buf = Vec::new();
            write_ofs_offset(&mut buf, offset).unwrap();
            let (got, n) = read_ofs_offset(&buf).unwrap();
            assert_eq!(got, offset, "offset {offset}");
            assert_eq!(n, buf.len());
        }
    }

    #[test]
    fn inflate_stops_at_zlib_end() {
        let raw = b"hello-pack";
        let compressed = zlib_compress(raw).unwrap();
        let mut input = compressed.clone();
        input.extend_from_slice(b"TRAILER");
        let (out, n) = inflate(&input, raw.len()).unwrap();
        assert_eq!(out, raw);
        assert_eq!(n, compressed.len());
        assert_eq!(&input[n..], b"TRAILER");
    }

    #[test]
    fn refuse_type_zero_and_five() {
        assert_eq!(validate_type(0), Err(PackError::InvalidType(0)));
        assert_eq!(validate_type(5), Err(PackError::ReservedType));
        assert!(validate_type(3).is_ok());
        assert!(validate_type(6).is_ok());
    }
}
