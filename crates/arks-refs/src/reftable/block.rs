//! Git Reftable block encoder/decoder (`r`, `i`, `o`, `g` blocks).
//!
//! Primary reference: Git `Documentation/technical/reftable.txt`
//! (§Ref block format, §Index block format, §Obj block format, §Log block format).

use crate::RefsError;
use crate::reftable::varint::{get_var_int, put_var_int};
use arks_oid::{Oid, OidKind};
use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use std::io::{Read, Write};

pub const BLOCK_TYPE_REF: u8 = b'r';
pub const BLOCK_TYPE_INDEX: u8 = b'i';
pub const BLOCK_TYPE_OBJ: u8 = b'o';
pub const BLOCK_TYPE_LOG: u8 = b'g';

pub const DEFAULT_RESTART_INTERVAL: usize = 16;

/// Value of a ref record inside an `'r'` block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefValue {
    /// `0x0`: deletion tombstone (shadows older tables in the stack).
    Deletion,
    /// `0x1`: direct object ID.
    Direct(Oid),
    /// `0x2`: direct object ID plus peeled target object ID (annotated tag).
    Peeled { value: Oid, target: Oid },
    /// `0x3`: symbolic reference target (e.g. `refs/heads/main`).
    Symbolic(String),
}

impl RefValue {
    pub fn value_type(&self) -> u8 {
        match self {
            RefValue::Deletion => 0x0,
            RefValue::Direct(_) => 0x1,
            RefValue::Peeled { .. } => 0x2,
            RefValue::Symbolic(_) => 0x3,
        }
    }
}

/// Decoded or to-be-encoded ref record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefRecord {
    pub refname: String,
    pub update_index: u64,
    pub value: RefValue,
}

/// Decoded or to-be-encoded reflog record (`'g'` block).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogRecord {
    pub refname: String,
    pub update_index: u64,
    pub update: Option<LogUpdate>,
}

/// Non-deletion reflog entry payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogUpdate {
    pub old_oid: Oid,
    pub new_oid: Oid,
    pub name: String,
    pub email: String,
    pub time: u64,
    pub tz_offset: i16,
    pub message: String,
}

/// Index record pointing to a child block's start offset in the table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexRecord {
    pub last_key: Vec<u8>,
    pub offset: u64,
}

/// Compute longest common prefix length of two byte slices.
fn common_prefix_len(a: &[u8], b: &[u8]) -> usize {
    a.iter().zip(b.iter()).take_while(|(x, y)| x == y).count()
}

/// Encode a single `'r'` block from sorted `RefRecord`s.
///
/// `header_off` is `24` (v1) or `28` (v2) when this is the first block in the
/// table (offset 0), and `0` for subsequent blocks.
pub fn encode_ref_block(
    records: &[RefRecord],
    min_update_index: u64,
    header_off: usize,
    restart_interval: usize,
) -> Result<Vec<u8>, RefsError> {
    let interval = restart_interval.max(1);
    let mut body = Vec::new();
    // Reserve 4-byte block header (block_type + uint24 block_len).
    body.extend_from_slice(&[BLOCK_TYPE_REF, 0, 0, 0]);

    let mut restarts: Vec<u32> = Vec::new();
    let mut prev_key: &[u8] = &[];

    for (idx, rec) in records.iter().enumerate() {
        let key = rec.refname.as_bytes();
        let is_restart = idx % interval == 0;
        let prefix_len = if is_restart {
            let off = (header_off + body.len()) as u32;
            restarts.push(off);
            0
        } else {
            common_prefix_len(prev_key, key)
        };
        let suffix = &key[prefix_len..];
        let vtype = u64::from(rec.value.value_type());
        put_var_int(&mut body, prefix_len as u64);
        put_var_int(&mut body, ((suffix.len() as u64) << 3) | vtype);
        body.extend_from_slice(suffix);

        let delta = rec
            .update_index
            .checked_sub(min_update_index)
            .ok_or_else(|| {
                RefsError::Corrupt("reftable: ref update_index < min_update_index".into())
            })?;
        put_var_int(&mut body, delta);

        match &rec.value {
            RefValue::Deletion => {}
            RefValue::Direct(oid) => body.extend_from_slice(oid.as_bytes()),
            RefValue::Peeled { value, target } => {
                body.extend_from_slice(value.as_bytes());
                body.extend_from_slice(target.as_bytes());
            }
            RefValue::Symbolic(target) => {
                let tb = target.as_bytes();
                put_var_int(&mut body, tb.len() as u64);
                body.extend_from_slice(tb);
            }
        }
        prev_key = key;
    }

    append_restart_table(&mut body, &restarts)?;
    patch_block_len(&mut body, header_off)?;
    Ok(body)
}

/// Encode an `'i'` index block from sorted `IndexRecord`s.
pub fn encode_index_block(
    records: &[IndexRecord],
    restart_interval: usize,
) -> Result<Vec<u8>, RefsError> {
    let interval = restart_interval.max(1);
    let mut body = Vec::new();
    body.extend_from_slice(&[BLOCK_TYPE_INDEX, 0, 0, 0]);

    let mut restarts: Vec<u32> = Vec::new();
    let mut prev_key: &[u8] = &[];

    for (idx, rec) in records.iter().enumerate() {
        let key = rec.last_key.as_slice();
        let prefix_len = if idx % interval == 0 {
            restarts.push(body.len() as u32);
            0
        } else {
            common_prefix_len(prev_key, key)
        };
        let suffix = &key[prefix_len..];
        put_var_int(&mut body, prefix_len as u64);
        put_var_int(&mut body, (suffix.len() as u64) << 3);
        body.extend_from_slice(suffix);
        put_var_int(&mut body, rec.offset);
        prev_key = key;
    }

    append_restart_table(&mut body, &restarts)?;
    patch_block_len(&mut body, 0)?;
    Ok(body)
}

/// Encode a `'g'` (log) block (uncompressed header + zlib-compressed body+restarts).
pub fn encode_log_block(
    records: &[LogRecord],
    restart_interval: usize,
) -> Result<Vec<u8>, RefsError> {
    let interval = restart_interval.max(1);
    let mut raw = Vec::new();
    let mut restarts: Vec<u32> = Vec::new();
    let mut prev_key: Vec<u8> = Vec::new();

    for (idx, rec) in records.iter().enumerate() {
        // Log key = refname + '\0' + (!update_index).to_be_bytes()
        let mut key = Vec::with_capacity(rec.refname.len() + 9);
        key.extend_from_slice(rec.refname.as_bytes());
        key.push(0);
        key.extend_from_slice(&(!rec.update_index).to_be_bytes());

        let prefix_len = if idx % interval == 0 {
            restarts.push((4 + raw.len()) as u32);
            0
        } else {
            common_prefix_len(&prev_key, &key)
        };
        let suffix = &key[prefix_len..];
        let vtype: u64 = if rec.update.is_some() { 1 } else { 0 };
        put_var_int(&mut raw, prefix_len as u64);
        put_var_int(&mut raw, ((suffix.len() as u64) << 3) | vtype);
        raw.extend_from_slice(suffix);

        if let Some(u) = &rec.update {
            raw.extend_from_slice(u.old_oid.as_bytes());
            raw.extend_from_slice(u.new_oid.as_bytes());
            put_str(&mut raw, &u.name);
            put_str(&mut raw, &u.email);
            put_var_int(&mut raw, u.time);
            raw.extend_from_slice(&u.tz_offset.to_be_bytes());
            put_str(&mut raw, &u.message);
        }
        prev_key = key;
    }

    append_restart_table(&mut raw, &restarts)?;

    let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
    enc.write_all(&raw)
        .map_err(|e| RefsError::Io(format!("reftable log zlib encode: {e}")))?;
    let compressed = enc
        .finish()
        .map_err(|e| RefsError::Io(format!("reftable log zlib finish: {e}")))?;

    let mut block = Vec::with_capacity(4 + compressed.len());
    block.extend_from_slice(&[BLOCK_TYPE_LOG, 0, 0, 0]);
    block.extend_from_slice(&compressed);
    patch_block_len(&mut block, 0)?;
    Ok(block)
}

fn put_str(out: &mut Vec<u8>, s: &str) {
    let b = s.as_bytes();
    put_var_int(out, b.len() as u64);
    out.extend_from_slice(b);
}

fn get_str(buf: &[u8], pos: &mut usize) -> Result<String, RefsError> {
    let (len, used) = get_var_int(&buf[*pos..])?;
    *pos += used;
    let len = len as usize;
    if *pos + len > buf.len() {
        return Err(RefsError::Corrupt("reftable: truncated string".into()));
    }
    let s = String::from_utf8(buf[*pos..*pos + len].to_vec())
        .map_err(|_| RefsError::Corrupt("reftable: invalid utf8 string".into()))?;
    *pos += len;
    Ok(s)
}

fn append_restart_table(body: &mut Vec<u8>, restarts: &[u32]) -> Result<(), RefsError> {
    if restarts.len() > u16::MAX as usize {
        return Err(RefsError::Corrupt(
            "reftable: restart count exceeds u16".into(),
        ));
    }
    for &off in restarts {
        if off > 0x00ff_ffff {
            return Err(RefsError::Corrupt(
                "reftable: restart offset exceeds u24".into(),
            ));
        }
        body.push(((off >> 16) & 0xff) as u8);
        body.push(((off >> 8) & 0xff) as u8);
        body.push((off & 0xff) as u8);
    }
    body.extend_from_slice(&(restarts.len() as u16).to_be_bytes());
    Ok(())
}

fn patch_block_len(block: &mut [u8], header_off: usize) -> Result<(), RefsError> {
    let total = block.len() + header_off;
    if total > 0x00ff_ffff {
        return Err(RefsError::Corrupt(
            "reftable: block length exceeds u24".into(),
        ));
    }
    block[1] = ((total >> 16) & 0xff) as u8;
    block[2] = ((total >> 8) & 0xff) as u8;
    block[3] = (total & 0xff) as u8;
    Ok(())
}

/// Read the 3-byte big-endian block length from a 4-byte block header.
pub fn read_block_header(slice: &[u8]) -> Result<(u8, usize), RefsError> {
    if slice.len() < 4 {
        return Err(RefsError::Corrupt(
            "reftable: truncated block header".into(),
        ));
    }
    let btype = slice[0];
    let len = ((slice[1] as usize) << 16) | ((slice[2] as usize) << 8) | (slice[3] as usize);
    Ok((btype, len))
}

/// Parse the restart offsets at the tail of an uncompressed block payload.
/// Returns `(records_end_offset_in_block, restart_offsets_adjusted_to_block_start)`.
fn parse_restarts(block: &[u8], header_off: usize) -> Result<(usize, Vec<usize>), RefsError> {
    if block.len() < 6 {
        return Err(RefsError::Corrupt("reftable: block too short".into()));
    }
    let count = u16::from_be_bytes([block[block.len() - 2], block[block.len() - 1]]) as usize;
    let table_bytes = count * 3 + 2;
    if block.len() < 4 + table_bytes {
        return Err(RefsError::Corrupt("reftable: corrupt restart table".into()));
    }
    let rec_end = block.len() - table_bytes;
    let mut offsets = Vec::with_capacity(count);
    for i in 0..count {
        let base = rec_end + i * 3;
        let raw = ((block[base] as usize) << 16)
            | ((block[base + 1] as usize) << 8)
            | (block[base + 2] as usize);
        let adj = raw
            .checked_sub(header_off)
            .ok_or_else(|| RefsError::Corrupt("reftable: restart offset < header_off".into()))?;
        offsets.push(adj);
    }
    Ok((rec_end, offsets))
}

/// Decode all `RefRecord`s from a single `'r'` block slice.
pub fn decode_ref_block(
    block: &[u8],
    header_off: usize,
    min_update_index: u64,
    oid_kind: OidKind,
) -> Result<Vec<RefRecord>, RefsError> {
    let (btype, _) = read_block_header(block)?;
    if btype != BLOCK_TYPE_REF {
        return Err(RefsError::Corrupt(format!(
            "reftable: expected ref block 'r', got 0x{btype:02x}"
        )));
    }
    let (rec_end, _restarts) = parse_restarts(block, header_off)?;
    let mut pos = 4usize;
    let mut prev_key = Vec::new();
    let mut out = Vec::new();
    while pos < rec_end {
        let rec = decode_one_ref(block, &mut pos, &mut prev_key, min_update_index, oid_kind)?;
        out.push(rec);
    }
    Ok(out)
}

/// Binary-search an `'r'` block using its 3-byte `restart_offset` table for `target`.
pub fn seek_ref_in_block(
    block: &[u8],
    header_off: usize,
    min_update_index: u64,
    oid_kind: OidKind,
    target: &str,
) -> Result<Option<RefRecord>, RefsError> {
    let (btype, _) = read_block_header(block)?;
    if btype != BLOCK_TYPE_REF {
        return Err(RefsError::Corrupt(format!(
            "reftable: expected ref block 'r', got 0x{btype:02x}"
        )));
    }
    let (rec_end, restarts) = parse_restarts(block, header_off)?;
    if restarts.is_empty() {
        return Ok(None);
    }
    // Binary search over restart points (each restart point has prefix_len == 0).
    let target_bytes = target.as_bytes();
    let mut lo = 0usize;
    let mut hi = restarts.len();
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        let rkey = read_restart_key(block, restarts[mid])?;
        if rkey.as_slice() <= target_bytes {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    let start_idx = lo.saturating_sub(1);
    let mut pos = restarts[start_idx];
    let mut prev_key = Vec::new();
    while pos < rec_end {
        let rec = decode_one_ref(block, &mut pos, &mut prev_key, min_update_index, oid_kind)?;
        match rec.refname.as_str().cmp(target) {
            std::cmp::Ordering::Equal => return Ok(Some(rec)),
            std::cmp::Ordering::Greater => return Ok(None),
            std::cmp::Ordering::Less => {}
        }
    }
    Ok(None)
}

fn read_restart_key(block: &[u8], mut pos: usize) -> Result<Vec<u8>, RefsError> {
    let (prefix_len, u1) = get_var_int(&block[pos..])?;
    pos += u1;
    if prefix_len != 0 {
        return Err(RefsError::Corrupt(
            "reftable: restart point has non-zero prefix_len".into(),
        ));
    }
    let (suffix_and_type, u2) = get_var_int(&block[pos..])?;
    pos += u2;
    let suffix_len = (suffix_and_type >> 3) as usize;
    if pos + suffix_len > block.len() {
        return Err(RefsError::Corrupt("reftable: restart key truncated".into()));
    }
    Ok(block[pos..pos + suffix_len].to_vec())
}

fn decode_one_ref(
    block: &[u8],
    pos: &mut usize,
    prev_key: &mut Vec<u8>,
    min_update_index: u64,
    oid_kind: OidKind,
) -> Result<RefRecord, RefsError> {
    let (prefix_len, u1) = get_var_int(&block[*pos..])?;
    *pos += u1;
    let (suffix_and_type, u2) = get_var_int(&block[*pos..])?;
    *pos += u2;
    let prefix_len = prefix_len as usize;
    let suffix_len = (suffix_and_type >> 3) as usize;
    let vtype = (suffix_and_type & 0x7) as u8;

    if prefix_len > prev_key.len() || *pos + suffix_len > block.len() {
        return Err(RefsError::Corrupt(
            "reftable: invalid prefix/suffix in ref record".into(),
        ));
    }
    let mut key = Vec::with_capacity(prefix_len + suffix_len);
    key.extend_from_slice(&prev_key[..prefix_len]);
    key.extend_from_slice(&block[*pos..*pos + suffix_len]);
    *pos += suffix_len;

    let (delta, u3) = get_var_int(&block[*pos..])?;
    *pos += u3;
    let update_index = min_update_index + delta;

    let value = match vtype {
        0x0 => RefValue::Deletion,
        0x1 => {
            let oid = read_oid(block, pos, oid_kind)?;
            RefValue::Direct(oid)
        }
        0x2 => {
            let value = read_oid(block, pos, oid_kind)?;
            let target = read_oid(block, pos, oid_kind)?;
            RefValue::Peeled { value, target }
        }
        0x3 => {
            let target = get_str(block, pos)?;
            RefValue::Symbolic(target)
        }
        other => {
            return Err(RefsError::Corrupt(format!(
                "reftable: unknown ref value type 0x{other:x}"
            )));
        }
    };

    let refname = String::from_utf8(key.clone())
        .map_err(|_| RefsError::Corrupt("reftable: non-utf8 refname".into()))?;
    *prev_key = key;
    Ok(RefRecord {
        refname,
        update_index,
        value,
    })
}

fn read_oid(buf: &[u8], pos: &mut usize, oid_kind: OidKind) -> Result<Oid, RefsError> {
    let hash_len = match oid_kind {
        OidKind::Sha1 => 20,
        OidKind::Sha256 => 32,
    };
    if *pos + hash_len > buf.len() {
        return Err(RefsError::Corrupt("reftable: truncated oid".into()));
    }
    let oid = Oid::from_bytes(oid_kind, &buf[*pos..*pos + hash_len])
        .map_err(|e| RefsError::Corrupt(format!("reftable: bad oid: {e}")))?;
    *pos += hash_len;
    Ok(oid)
}

/// Decode all `IndexRecord`s from an `'i'` block.
pub fn decode_index_block(block: &[u8]) -> Result<Vec<IndexRecord>, RefsError> {
    let (btype, _) = read_block_header(block)?;
    if btype != BLOCK_TYPE_INDEX {
        return Err(RefsError::Corrupt(format!(
            "reftable: expected index block 'i', got 0x{btype:02x}"
        )));
    }
    let (rec_end, _restarts) = parse_restarts(block, 0)?;
    let mut pos = 4usize;
    let mut prev_key = Vec::new();
    let mut out = Vec::new();
    while pos < rec_end {
        let (prefix_len, u1) = get_var_int(&block[pos..])?;
        pos += u1;
        let (suffix_and_type, u2) = get_var_int(&block[pos..])?;
        pos += u2;
        let prefix_len = prefix_len as usize;
        let suffix_len = (suffix_and_type >> 3) as usize;
        if prefix_len > prev_key.len() || pos + suffix_len > block.len() {
            return Err(RefsError::Corrupt("reftable: invalid index record".into()));
        }
        let mut key = Vec::with_capacity(prefix_len + suffix_len);
        key.extend_from_slice(&prev_key[..prefix_len]);
        key.extend_from_slice(&block[pos..pos + suffix_len]);
        pos += suffix_len;
        let (offset, u3) = get_var_int(&block[pos..])?;
        pos += u3;
        prev_key = key.clone();
        out.push(IndexRecord {
            last_key: key,
            offset,
        });
    }
    Ok(out)
}

/// Decode all `LogRecord`s from a `'g'` (zlib-compressed) block.
pub fn decode_log_block(block: &[u8], oid_kind: OidKind) -> Result<Vec<LogRecord>, RefsError> {
    let (btype, _) = read_block_header(block)?;
    if btype != BLOCK_TYPE_LOG {
        return Err(RefsError::Corrupt(format!(
            "reftable: expected log block 'g', got 0x{btype:02x}"
        )));
    }
    let mut dec = ZlibDecoder::new(&block[4..]);
    let mut raw = Vec::new();
    dec.read_to_end(&mut raw)
        .map_err(|e| RefsError::Corrupt(format!("reftable: log block zlib inflate: {e}")))?;

    // Re-attach dummy 4-byte header so parse_restarts offsets match.
    let mut full = Vec::with_capacity(4 + raw.len());
    full.extend_from_slice(&block[..4]);
    full.extend_from_slice(&raw);
    let (rec_end, _restarts) = parse_restarts(&full, 0)?;

    let mut pos = 4usize;
    let mut prev_key = Vec::new();
    let mut out = Vec::new();
    while pos < rec_end {
        let (prefix_len, u1) = get_var_int(&full[pos..])?;
        pos += u1;
        let (suffix_and_type, u2) = get_var_int(&full[pos..])?;
        pos += u2;
        let prefix_len = prefix_len as usize;
        let suffix_len = (suffix_and_type >> 3) as usize;
        let vtype = (suffix_and_type & 0x7) as u8;
        if prefix_len > prev_key.len() || pos + suffix_len > full.len() {
            return Err(RefsError::Corrupt("reftable: corrupt log key".into()));
        }
        let mut key = Vec::with_capacity(prefix_len + suffix_len);
        key.extend_from_slice(&prev_key[..prefix_len]);
        key.extend_from_slice(&full[pos..pos + suffix_len]);
        pos += suffix_len;
        if key.len() < 9 || key[key.len() - 9] != 0 {
            return Err(RefsError::Corrupt(
                "reftable: malformed log key suffix".into(),
            ));
        }
        let refname = String::from_utf8(key[..key.len() - 9].to_vec())
            .map_err(|_| RefsError::Corrupt("reftable: non-utf8 log refname".into()))?;
        let inv_idx = u64::from_be_bytes(key[key.len() - 8..].try_into().unwrap());
        let update_index = !inv_idx;

        let update = match vtype {
            0x0 => None,
            0x1 => {
                let old_oid = read_oid(&full, &mut pos, oid_kind)?;
                let new_oid = read_oid(&full, &mut pos, oid_kind)?;
                let name = get_str(&full, &mut pos)?;
                let email = get_str(&full, &mut pos)?;
                let (time, ut) = get_var_int(&full[pos..])?;
                pos += ut;
                if pos + 2 > full.len() {
                    return Err(RefsError::Corrupt("reftable: truncated log tz".into()));
                }
                let tz_offset = i16::from_be_bytes([full[pos], full[pos + 1]]);
                pos += 2;
                let message = get_str(&full, &mut pos)?;
                Some(LogUpdate {
                    old_oid,
                    new_oid,
                    name,
                    email,
                    time,
                    tz_offset,
                    message,
                })
            }
            other => {
                return Err(RefsError::Corrupt(format!(
                    "reftable: unknown log value type 0x{other:x}"
                )));
            }
        };
        prev_key = key;
        out.push(LogRecord {
            refname,
            update_index,
            update,
        });
    }
    Ok(out)
}
