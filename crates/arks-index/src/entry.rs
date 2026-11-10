//! Cache entries and sort order.

use crate::error::IndexError;
use arks_oid::{Oid, OidKind};

pub const CE_VALID: u16 = 0x8000;
pub const CE_EXTENDED: u16 = 0x4000;
pub const CE_STAGEMASK: u16 = 0x3000;
pub const CE_STAGE_SHIFT: u16 = 12;
pub const CE_NAMEMASK: u16 = 0x0fff;

/// One index cache entry (`gitformat-index`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexEntry {
    pub ctime_sec: u32,
    pub ctime_nsec: u32,
    pub mtime_sec: u32,
    pub mtime_nsec: u32,
    pub dev: u32,
    pub ino: u32,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub size: u32,
    pub oid: Oid,
    pub flags: u16,
    pub flags_extended: Option<u16>,
    pub path: String,
}

impl IndexEntry {
    pub fn stage(&self) -> u16 {
        (self.flags & CE_STAGEMASK) >> CE_STAGE_SHIFT
    }
}

/// Compare paths as unsigned bytes (`memcmp`), then stage.
pub fn cmp_entries(a: &IndexEntry, b: &IndexEntry) -> std::cmp::Ordering {
    let path_ord = a.path.as_bytes().cmp(b.path.as_bytes());
    if path_ord != std::cmp::Ordering::Equal {
        return path_ord;
    }
    a.stage().cmp(&b.stage())
}

pub fn entries_sorted(entries: &[IndexEntry]) -> bool {
    entries
        .windows(2)
        .all(|w| cmp_entries(&w[0], &w[1]) != std::cmp::Ordering::Greater)
}

pub(crate) fn read_entry_v2_v3(
    data: &[u8],
    offset: usize,
    version: u32,
    hash_kind: OidKind,
) -> Result<(IndexEntry, usize), IndexError> {
    let oid_len = hash_kind.digest_len();
    let min = offset + 62;
    if data.len() < min {
        return Err(IndexError::Truncated);
    }
    let mut r = offset;
    let read_u32 = |d: &[u8], o: &mut usize| -> u32 {
        let v = u32::from_be_bytes(d[*o..*o + 4].try_into().unwrap());
        *o += 4;
        v
    };
    let ctime_sec = read_u32(data, &mut r);
    let ctime_nsec = read_u32(data, &mut r);
    let mtime_sec = read_u32(data, &mut r);
    let mtime_nsec = read_u32(data, &mut r);
    let dev = read_u32(data, &mut r);
    let ino = read_u32(data, &mut r);
    let mode = read_u32(data, &mut r);
    let uid = read_u32(data, &mut r);
    let gid = read_u32(data, &mut r);
    let size = read_u32(data, &mut r);
    let oid = Oid::from_bytes(hash_kind, &data[r..r + oid_len])
        .map_err(|e| IndexError::Corrupt(format!("oid: {e}")))?;
    r += oid_len;
    if data.len() < r + 2 {
        return Err(IndexError::Truncated);
    }
    let flags = u16::from_be_bytes([data[r], data[r + 1]]);
    r += 2;
    let flags_extended = if flags & CE_EXTENDED != 0 {
        if version < 3 {
            return Err(IndexError::Corrupt(
                "extended flags in index version < 3".into(),
            ));
        }
        if data.len() < r + 2 {
            return Err(IndexError::Truncated);
        }
        let ext = u16::from_be_bytes([data[r], data[r + 1]]);
        r += 2;
        Some(ext)
    } else {
        None
    };
    let name_len = (flags & CE_NAMEMASK) as usize;
    let path_start = r;
    let path = if flags & CE_EXTENDED != 0 {
        read_path_nul(data, r)?
    } else if name_len < CE_NAMEMASK as usize {
        if data.len() < r + name_len + 1 {
            return Err(IndexError::Truncated);
        }
        let p = std::str::from_utf8(&data[r..r + name_len])
            .map_err(|_| IndexError::Corrupt("path not utf-8".into()))?;
        if data[r + name_len] != 0 {
            return Err(IndexError::Corrupt("missing path NUL".into()));
        }
        if p.as_bytes().contains(&0) {
            return Err(IndexError::PathNul);
        }
        p.to_owned()
    } else {
        return Err(IndexError::Corrupt("invalid name length in flags".into()));
    };
    let path_end = path_start + path.len() + 1;
    if data.len() < path_end {
        return Err(IndexError::Truncated);
    }
    if data[path_end - 1] != 0 {
        return Err(IndexError::Corrupt("missing path NUL".into()));
    }
    let entry_start = offset;
    let consumed = path_end - entry_start;
    let pad = (8 - (consumed % 8)) % 8;
    if data.len() < entry_start + consumed + pad {
        return Err(IndexError::Truncated);
    }
    let total = consumed + pad;
    Ok((
        IndexEntry {
            ctime_sec,
            ctime_nsec,
            mtime_sec,
            mtime_nsec,
            dev,
            ino,
            mode,
            uid,
            gid,
            size,
            oid,
            flags,
            flags_extended,
            path,
        },
        entry_start + total,
    ))
}

fn read_path_nul(data: &[u8], start: usize) -> Result<String, IndexError> {
    let rest = &data[start..];
    let nul = rest
        .iter()
        .position(|&b| b == 0)
        .ok_or(IndexError::Truncated)?;
    if rest[..nul].contains(&0) {
        return Err(IndexError::PathNul);
    }
    let p = std::str::from_utf8(&rest[..nul])
        .map_err(|_| IndexError::Corrupt("path not utf-8".into()))?;
    Ok(p.to_owned())
}

pub(crate) fn read_entry_v4(
    data: &[u8],
    offset: usize,
    prev_path: &str,
    hash_kind: OidKind,
) -> Result<(IndexEntry, usize), IndexError> {
    let oid_len = hash_kind.digest_len();
    if data.len() < offset + 62 {
        return Err(IndexError::Truncated);
    }
    let mut r = offset;
    let read_u32 = |d: &[u8], o: &mut usize| -> u32 {
        let v = u32::from_be_bytes(d[*o..*o + 4].try_into().unwrap());
        *o += 4;
        v
    };
    let ctime_sec = read_u32(data, &mut r);
    let ctime_nsec = read_u32(data, &mut r);
    let mtime_sec = read_u32(data, &mut r);
    let mtime_nsec = read_u32(data, &mut r);
    let dev = read_u32(data, &mut r);
    let ino = read_u32(data, &mut r);
    let mode = read_u32(data, &mut r);
    let uid = read_u32(data, &mut r);
    let gid = read_u32(data, &mut r);
    let size = read_u32(data, &mut r);
    let oid = Oid::from_bytes(hash_kind, &data[r..r + oid_len])
        .map_err(|e| IndexError::Corrupt(format!("oid: {e}")))?;
    r += oid_len;
    let flags = u16::from_be_bytes([data[r], data[r + 1]]);
    r += 2;
    let flags_extended = if flags & CE_EXTENDED != 0 {
        if data.len() < r + 2 {
            return Err(IndexError::Truncated);
        }
        let ext = u16::from_be_bytes([data[r], data[r + 1]]);
        r += 2;
        Some(ext)
    } else {
        None
    };
    let (strip, r1) = read_varint(data, r)?;
    let (suffix_len, r2) = read_varint(data, r1)?;
    if data.len() < r2 + suffix_len {
        return Err(IndexError::Truncated);
    }
    let suffix = std::str::from_utf8(&data[r2..r2 + suffix_len])
        .map_err(|_| IndexError::Corrupt("path suffix not utf-8".into()))?;
    let prev_len = prev_path.len();
    if strip > prev_len {
        return Err(IndexError::Corrupt(
            "v4 path strip exceeds previous path".into(),
        ));
    }
    let prefix = &prev_path[..prev_len - strip];
    let path = format!("{prefix}{suffix}");
    if path.as_bytes().contains(&0) {
        return Err(IndexError::PathNul);
    }
    let consumed = r2 - offset + suffix_len;
    let pad = (8 - (consumed % 8)) % 8;
    if data.len() < offset + consumed + pad {
        return Err(IndexError::Truncated);
    }
    Ok((
        IndexEntry {
            ctime_sec,
            ctime_nsec,
            mtime_sec,
            mtime_nsec,
            dev,
            ino,
            mode,
            uid,
            gid,
            size,
            oid,
            flags,
            flags_extended,
            path,
        },
        offset + consumed + pad,
    ))
}

fn read_varint(data: &[u8], mut offset: usize) -> Result<(usize, usize), IndexError> {
    let mut value = 0usize;
    let start = offset;
    loop {
        if offset >= data.len() {
            return Err(IndexError::Truncated);
        }
        let b = data[offset];
        offset += 1;
        value = (value << 7) | (b & 0x7f) as usize;
        if b & 0x80 == 0 {
            break;
        }
        if offset - start > 10 {
            return Err(IndexError::Corrupt("varint too long".into()));
        }
    }
    Ok((value, offset))
}

pub(crate) fn write_entry_v2(
    buf: &mut Vec<u8>,
    entry: &IndexEntry,
    hash_kind: OidKind,
    write_version: u32,
) -> Result<(), IndexError> {
    if entry.path.as_bytes().contains(&0) {
        return Err(IndexError::PathNul);
    }
    let path_bytes = entry.path.as_bytes();
    let need_extended = entry.flags_extended.is_some() || path_bytes.len() >= CE_NAMEMASK as usize;
    let entry_start = buf.len();
    let mut flags = entry.flags & !(CE_NAMEMASK);
    flags &= !CE_EXTENDED;
    if need_extended {
        if write_version < 3 {
            return Err(IndexError::Corrupt(
                "extended entry flags require index version >= 3".into(),
            ));
        }
        flags |= CE_EXTENDED | CE_NAMEMASK;
    } else {
        flags |= path_bytes.len() as u16 & CE_NAMEMASK;
    }
    buf.extend_from_slice(&entry.ctime_sec.to_be_bytes());
    buf.extend_from_slice(&entry.ctime_nsec.to_be_bytes());
    buf.extend_from_slice(&entry.mtime_sec.to_be_bytes());
    buf.extend_from_slice(&entry.mtime_nsec.to_be_bytes());
    buf.extend_from_slice(&entry.dev.to_be_bytes());
    buf.extend_from_slice(&entry.ino.to_be_bytes());
    buf.extend_from_slice(&entry.mode.to_be_bytes());
    buf.extend_from_slice(&entry.uid.to_be_bytes());
    buf.extend_from_slice(&entry.gid.to_be_bytes());
    buf.extend_from_slice(&entry.size.to_be_bytes());
    buf.extend_from_slice(entry.oid.as_bytes());
    buf.extend_from_slice(&flags.to_be_bytes());
    if need_extended {
        let ext = entry.flags_extended.unwrap_or(0);
        buf.extend_from_slice(&ext.to_be_bytes());
    }
    buf.extend_from_slice(path_bytes);
    buf.push(0);
    let consumed = buf.len() - entry_start;
    let pad = (8 - (consumed % 8)) % 8;
    buf.extend(std::iter::repeat_n(0u8, pad));
    let _ = hash_kind;
    Ok(())
}
