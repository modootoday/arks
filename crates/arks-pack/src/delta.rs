//! Pack delta application.
//!
//! Size encoding and copy/insert instructions follow
//! <https://git-scm.com/docs/gitformat-pack> (deltified representation).
//! A copy size of zero means `0x10000`. Opcode zero is reserved.

use crate::error::PackError;

/// Encode a non-negative integer in the delta size encoding (least significant
/// 7-bit group first). Not the pack offset encoding.
///
pub(crate) fn write_size(buf: &mut Vec<u8>, mut value: u64) {
    loop {
        let mut byte = (value & 0x7f) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        buf.push(byte);
        if value == 0 {
            break;
        }
    }
}

pub(crate) fn read_size(data: &[u8], i: &mut usize) -> Result<u64, PackError> {
    let mut size = 0u64;
    let mut shift = 0u32;
    loop {
        if *i >= data.len() {
            return Err(PackError::Truncated);
        }
        let b = data[*i];
        *i += 1;
        if shift >= 64 {
            return Err(PackError::SizeOverflow);
        }
        let bits = u64::from(b & 0x7f);
        let add = bits.checked_shl(shift).ok_or(PackError::SizeOverflow)?;
        size = size.checked_add(add).ok_or(PackError::SizeOverflow)?;
        if b & 0x80 == 0 {
            return Ok(size);
        }
        shift = shift.checked_add(7).ok_or(PackError::SizeOverflow)?;
    }
}

/// Reconstruct a target object from `base` and a zlib-inflated delta.
pub(crate) fn apply_delta(base: &[u8], delta: &[u8]) -> Result<Vec<u8>, PackError> {
    let mut i = 0usize;
    let base_size = read_size(delta, &mut i)?;
    let result_size = read_size(delta, &mut i)?;
    if base_size != base.len() as u64 {
        return Err(PackError::BadDelta("base size mismatch"));
    }
    let result_len = usize::try_from(result_size).map_err(|_| PackError::SizeOverflow)?;
    let mut out = Vec::with_capacity(result_len);

    while i < delta.len() {
        let cmd = delta[i];
        i += 1;
        if cmd == 0 {
            return Err(PackError::BadDelta("reserved opcode"));
        }
        if cmd & 0x80 != 0 {
            let (off, size) = read_copy(delta, &mut i, cmd)?;
            let end = off
                .checked_add(size)
                .ok_or(PackError::BadDelta("copy out of bounds"))?;
            if end > base.len() as u64 {
                return Err(PackError::BadDelta("copy out of bounds"));
            }
            let start = off as usize;
            let end = end as usize;
            push_within(&mut out, result_len, &base[start..end])?;
        } else {
            let n = usize::from(cmd);
            let end = i
                .checked_add(n)
                .ok_or(PackError::BadDelta("truncated instruction"))?;
            if end > delta.len() {
                return Err(PackError::BadDelta("truncated instruction"));
            }
            push_within(&mut out, result_len, &delta[i..end])?;
            i = end;
        }
    }

    if out.len() != result_len {
        return Err(PackError::BadDelta("result size mismatch"));
    }
    Ok(out)
}

fn read_copy(data: &[u8], i: &mut usize, cmd: u8) -> Result<(u64, u64), PackError> {
    let mut off = 0u64;
    let mut size = 0u64;
    for bit in 0..4 {
        if cmd & (1u8 << bit) != 0 {
            off |= u64::from(take(data, i)?) << (8 * bit);
        }
    }
    for bit in 0..3 {
        if cmd & (1u8 << (4 + bit)) != 0 {
            size |= u64::from(take(data, i)?) << (8 * bit);
        }
    }
    // gitformat-pack: omitted size bytes default to zero, and size zero means 0x10000.
    if size == 0 {
        size = 0x10000;
    }
    Ok((off, size))
}

fn take(data: &[u8], i: &mut usize) -> Result<u8, PackError> {
    if *i >= data.len() {
        return Err(PackError::BadDelta("truncated instruction"));
    }
    let b = data[*i];
    *i += 1;
    Ok(b)
}

fn push_within(out: &mut Vec<u8>, result_len: usize, bytes: &[u8]) -> Result<(), PackError> {
    if out.len().saturating_add(bytes.len()) > result_len {
        return Err(PackError::BadDelta("result size exceeded"));
    }
    out.extend_from_slice(bytes);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_and_insert_hello() {
        let base = b"hello world";
        let mut delta = Vec::new();
        write_size(&mut delta, base.len() as u64);
        write_size(&mut delta, 11);
        // Copy 6 bytes from offset 0 ("hello "), then insert "there".
        delta.extend_from_slice(&[0x90, 6]);
        delta.push(5);
        delta.extend_from_slice(b"there");
        let got = apply_delta(base, &delta).unwrap();
        assert_eq!(got, b"hello there");
    }

    #[test]
    fn omitted_middle_offset_byte_keeps_place() {
        // gitformat-pack: skipping offset2 does not shift offset3 into its place.
        let mut base = vec![0u8; 300];
        base[256] = 0xab;
        base[257] = 0xcd;
        let mut delta = Vec::new();
        write_size(&mut delta, 300);
        write_size(&mut delta, 2);
        // offset = 256 (offset2 only), size = 2 (size1).
        delta.extend_from_slice(&[0x92, 0x01, 0x02]);
        let got = apply_delta(&base, &delta).unwrap();
        assert_eq!(got, [0xab, 0xcd]);
    }

    #[test]
    fn copy_size_zero_means_64k() {
        let base = vec![b'a'; 0x10000];
        let mut delta = Vec::new();
        write_size(&mut delta, 0x10000);
        write_size(&mut delta, 0x10000);
        delta.push(0x80);
        let got = apply_delta(&base, &delta).unwrap();
        assert_eq!(got, base);
    }

    #[test]
    fn refuse_reserved_opcode() {
        let mut delta = Vec::new();
        write_size(&mut delta, 1);
        write_size(&mut delta, 1);
        delta.push(0);
        assert_eq!(
            apply_delta(b"x", &delta),
            Err(PackError::BadDelta("reserved opcode"))
        );
    }

    #[test]
    fn refuse_base_size_mismatch_and_short_result() {
        let mut delta = Vec::new();
        write_size(&mut delta, 2);
        write_size(&mut delta, 1);
        assert_eq!(
            apply_delta(b"x", &delta),
            Err(PackError::BadDelta("base size mismatch"))
        );
        let mut delta = Vec::new();
        write_size(&mut delta, 1);
        write_size(&mut delta, 2);
        delta.push(1);
        delta.push(b'x');
        assert_eq!(
            apply_delta(b"x", &delta),
            Err(PackError::BadDelta("result size mismatch"))
        );
    }
}

const BLOCK: usize = 16;
const MAX_CANDIDATES: usize = 64;

fn block_hash(b: &[u8]) -> u64 {
    b.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, c| {
        (h ^ u64::from(*c)).wrapping_mul(0x0100_0000_01b3)
    })
}

fn push_insert(out: &mut Vec<u8>, lit: &[u8]) {
    for chunk in lit.chunks(0x7f) {
        out.push(chunk.len() as u8);
        out.extend_from_slice(chunk);
    }
}

fn push_copy(out: &mut Vec<u8>, mut off: usize, mut len: usize) {
    while len > 0 {
        let n = len.min(0x10000);
        let mut cmd = 0x80u8;
        let mut args = Vec::with_capacity(7);
        for (bit, shift) in [(0x01u8, 0u32), (0x02, 8), (0x04, 16), (0x08, 24)] {
            let b = ((off >> shift) & 0xff) as u8;
            if b != 0 {
                cmd |= bit;
                args.push(b);
            }
        }
        let size = if n == 0x10000 { 0 } else { n };
        for (bit, shift) in [(0x10u8, 0u32), (0x20, 8), (0x40, 16)] {
            let b = ((size >> shift) & 0xff) as u8;
            if b != 0 {
                cmd |= bit;
                args.push(b);
            }
        }
        out.push(cmd);
        out.extend_from_slice(&args);
        off += n;
        len -= n;
    }
}

/// Greedy delta of `target` against `base` (16-byte block index over the
/// base, longest verified match per position). `None` when the delta is
/// not shorter than `max_size` or the base is too large to address.
pub(crate) fn create(base: &[u8], target: &[u8], max_size: usize) -> Option<Vec<u8>> {
    if base.is_empty() || base.len() > u32::MAX as usize {
        return None;
    }
    let mut out = Vec::new();
    write_size(&mut out, base.len() as u64);
    write_size(&mut out, target.len() as u64);
    let mut index: std::collections::HashMap<u64, Vec<usize>> = std::collections::HashMap::new();
    let mut p = 0;
    while p + BLOCK <= base.len() {
        let v = index.entry(block_hash(&base[p..p + BLOCK])).or_default();
        if v.len() < MAX_CANDIDATES {
            v.push(p);
        }
        p += BLOCK;
    }
    let mut lit: Vec<u8> = Vec::new();
    let mut i = 0;
    while i < target.len() {
        let mut best = (0usize, 0usize);
        if i + BLOCK <= target.len() {
            if let Some(cands) = index.get(&block_hash(&target[i..i + BLOCK])) {
                for &c in cands {
                    let mut n = 0;
                    while c + n < base.len() && i + n < target.len() && base[c + n] == target[i + n]
                    {
                        n += 1;
                    }
                    if n > best.1 {
                        best = (c, n);
                    }
                }
            }
        }
        if best.1 < BLOCK {
            lit.push(target[i]);
            i += 1;
            if out.len() + lit.len() >= max_size {
                return None;
            }
            continue;
        }
        // Grow the match backwards over pending literals.
        let (mut c, mut n) = best;
        while c > 0 && lit.last().is_some_and(|l| *l == base[c - 1]) {
            lit.pop();
            c -= 1;
            n += 1;
        }
        push_insert(&mut out, &lit);
        lit.clear();
        push_copy(&mut out, c, n);
        i = i - (best.0 - c) + n;
        if out.len() >= max_size {
            return None;
        }
    }
    push_insert(&mut out, &lit);
    (out.len() < max_size).then_some(out)
}

#[cfg(test)]
mod create_tests {
    use super::*;

    #[test]
    fn delta_round_trips() {
        let base: Vec<u8> = (0..4000u32).map(|i| (i * 7 % 251) as u8).collect();
        let mut target = base.clone();
        target[1000] ^= 0x55;
        target.extend_from_slice(b"tail bytes");
        target.splice(2000..2000, b"inserted text".iter().copied());
        let d = create(&base, &target, usize::MAX).expect("delta");
        assert!(d.len() < target.len() / 4);
        assert_eq!(apply_delta(&base, &d).unwrap(), target);
        assert!(create(&base, b"short", 3).is_none());
    }
}
