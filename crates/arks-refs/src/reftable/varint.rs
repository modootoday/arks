//! Git Reftable variable-length unsigned integer encoding.
//!
//! Primary reference: Git `Documentation/technical/reftable.txt` §Varint encoding
//! and `reftable/basics.c` (`put_var_int` / `get_var_int`).

use crate::RefsError;

/// Encode a `u64` using Git's big-endian continuation-bit varint format.
///
/// Each continuation byte contributes `((val + 1) << 7)` (avoiding redundant
/// encodings of small values, matching `git/reftable/basics.c`).
pub fn put_var_int(out: &mut Vec<u8>, mut val: u64) {
    let mut buf = [0u8; 10];
    let mut n = 9usize;
    buf[n] = (val & 0x7f) as u8;
    val >>= 7;
    while val > 0 {
        val -= 1;
        n -= 1;
        buf[n] = 0x80 | ((val & 0x7f) as u8);
        val >>= 7;
    }
    out.extend_from_slice(&buf[n..]);
}

/// Decode a Git varint from `buf`, returning `(value, bytes_consumed)`.
pub fn get_var_int(buf: &[u8]) -> Result<(u64, usize), RefsError> {
    if buf.is_empty() {
        return Err(RefsError::Corrupt("reftable: truncated varint".into()));
    }
    let mut i = 0usize;
    let mut val = u64::from(buf[0] & 0x7f);
    while (buf[i] & 0x80) != 0 {
        i += 1;
        if i >= buf.len() || i >= 10 {
            return Err(RefsError::Corrupt(
                "reftable: varint overflow or truncated".into(),
            ));
        }
        val = val
            .checked_add(1)
            .and_then(|v| v.checked_shl(7))
            .ok_or_else(|| RefsError::Corrupt("reftable: varint overflow".into()))?;
        val |= u64::from(buf[i] & 0x7f);
    }
    Ok((val, i + 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn varint_roundtrip_known_values() {
        let cases = [
            0u64,
            1,
            127,
            128,
            129,
            255,
            256,
            16_384,
            16_511,
            16_512,
            1 << 20,
            1 << 32,
            u64::MAX / 2,
            u64::MAX - 1,
        ];
        for &v in &cases {
            let mut buf = Vec::new();
            put_var_int(&mut buf, v);
            let (decoded, used) = get_var_int(&buf).unwrap();
            assert_eq!(decoded, v, "value={v} buf={buf:02x?}");
            assert_eq!(used, buf.len());
        }
        // Known Git encoding: 0 -> [0x00], 127 -> [0x7f], 128 -> [0x80, 0x00]
        let mut b = Vec::new();
        put_var_int(&mut b, 128);
        assert_eq!(b, vec![0x80, 0x00]);
    }
}
