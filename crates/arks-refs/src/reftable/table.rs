//! Single immutable Git Reftable `.ref` file reader and writer (v1 & v2).
//!
//! Primary reference: Git `Documentation/technical/reftable.txt`
//! (§Header, §Ref block, §Index block, §Log block, §Footer).

use crate::RefsError;
use crate::reftable::block::{
    BLOCK_TYPE_INDEX, BLOCK_TYPE_LOG, BLOCK_TYPE_REF, DEFAULT_RESTART_INTERVAL, IndexRecord,
    LogRecord, RefRecord, decode_index_block, decode_log_block, decode_ref_block,
    encode_index_block, encode_log_block, encode_ref_block, read_block_header, seek_ref_in_block,
};
use arks_oid::OidKind;

pub const MAGIC: &[u8; 4] = b"REFT";
pub const DEFAULT_BLOCK_SIZE: u32 = 4096;

/// Header metadata shared between header and footer of a `.ref` table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TableHeader {
    pub version: u8,
    pub block_size: u32,
    pub min_update_index: u64,
    pub max_update_index: u64,
    pub oid_kind: OidKind,
}

impl TableHeader {
    pub fn header_size(&self) -> usize {
        if self.version == 2 { 28 } else { 24 }
    }

    pub fn footer_size(&self) -> usize {
        if self.version == 2 { 72 } else { 68 }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.header_size());
        out.extend_from_slice(MAGIC);
        out.push(self.version);
        out.push(((self.block_size >> 16) & 0xff) as u8);
        out.push(((self.block_size >> 8) & 0xff) as u8);
        out.push((self.block_size & 0xff) as u8);
        out.extend_from_slice(&self.min_update_index.to_be_bytes());
        out.extend_from_slice(&self.max_update_index.to_be_bytes());
        if self.version == 2 {
            match self.oid_kind {
                OidKind::Sha1 => out.extend_from_slice(b"sha1"),
                OidKind::Sha256 => out.extend_from_slice(b"s256"),
            }
        }
        out
    }

    pub fn decode(buf: &[u8]) -> Result<Self, RefsError> {
        if buf.len() < 24 || &buf[..4] != MAGIC {
            return Err(RefsError::Corrupt(
                "reftable: missing REFT magic or truncated header".into(),
            ));
        }
        let version = buf[4];
        if version != 1 && version != 2 {
            return Err(RefsError::Corrupt(format!(
                "reftable: unsupported format version {version}"
            )));
        }
        let block_size = ((buf[5] as u32) << 16) | ((buf[6] as u32) << 8) | (buf[7] as u32);
        let min_update_index = u64::from_be_bytes(buf[8..16].try_into().unwrap());
        let max_update_index = u64::from_be_bytes(buf[16..24].try_into().unwrap());
        let oid_kind = if version == 2 {
            if buf.len() < 28 {
                return Err(RefsError::Corrupt("reftable: truncated v2 header".into()));
            }
            match &buf[24..28] {
                b"sha1" => OidKind::Sha1,
                b"s256" => OidKind::Sha256,
                other => {
                    return Err(RefsError::Corrupt(format!(
                        "reftable: unknown v2 hash_id {other:?}"
                    )));
                }
            }
        } else {
            OidKind::Sha1
        };
        Ok(Self {
            version,
            block_size,
            min_update_index,
            max_update_index,
            oid_kind,
        })
    }
}

/// Footer metadata including section offsets and verified CRC-32.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TableFooter {
    pub header: TableHeader,
    pub ref_index_position: u64,
    pub obj_position: u64,
    pub obj_id_len: u8,
    pub obj_index_position: u64,
    pub log_position: u64,
    pub log_index_position: u64,
    pub crc32: u32,
}

/// Write a complete immutable `.ref` table from sorted `RefRecord`s and optional `LogRecord`s.
pub fn write_table(
    refs: &[RefRecord],
    logs: &[LogRecord],
    min_update_index: u64,
    max_update_index: u64,
    oid_kind: OidKind,
    block_size: u32,
) -> Result<Vec<u8>, RefsError> {
    let version = if oid_kind == OidKind::Sha256 { 2 } else { 1 };
    let header = TableHeader {
        version,
        block_size: if block_size == 0 {
            DEFAULT_BLOCK_SIZE
        } else {
            block_size
        },
        min_update_index,
        max_update_index,
        oid_kind,
    };

    let mut out = header.encode();
    let header_len = header.header_size();
    let mut ref_index_entries: Vec<IndexRecord> = Vec::new();
    let mut ref_index_position: u64 = 0;

    if !refs.is_empty() {
        // Chunk refs into blocks of up to ~64 records (keeping each block well within block_size).
        let chunk_size = 64usize;
        let chunks: Vec<&[RefRecord]> = refs.chunks(chunk_size).collect();
        for (idx, chunk) in chunks.iter().enumerate() {
            let block_start = if idx == 0 { 0u64 } else { out.len() as u64 };
            let h_off = if idx == 0 { header_len } else { 0 };
            let block = encode_ref_block(chunk, min_update_index, h_off, DEFAULT_RESTART_INTERVAL)?;
            out.extend_from_slice(&block);
            if let Some(last) = chunk.last() {
                ref_index_entries.push(IndexRecord {
                    last_key: last.refname.as_bytes().to_vec(),
                    offset: block_start,
                });
            }
            // Pad to block_size boundary if another block follows and block_size > 0.
            let bs = header.block_size as usize;
            if idx + 1 < chunks.len() && bs > 0 {
                let rem = out.len() % bs;
                if rem != 0 {
                    out.resize(out.len() + (bs - rem), 0);
                }
            }
        }
        if ref_index_entries.len() > 1 {
            let bs = header.block_size as usize;
            if bs > 0 {
                let rem = out.len() % bs;
                if rem != 0 {
                    out.resize(out.len() + (bs - rem), 0);
                }
            }
            ref_index_position = out.len() as u64;
            let iblock = encode_index_block(&ref_index_entries, DEFAULT_RESTART_INTERVAL)?;
            out.extend_from_slice(&iblock);
        }
    }

    let mut log_position: u64 = 0;
    if !logs.is_empty() {
        let bs = header.block_size as usize;
        if bs > 0 && !refs.is_empty() {
            let rem = out.len() % bs;
            if rem != 0 {
                out.resize(out.len() + (bs - rem), 0);
            }
        }
        log_position = out.len() as u64;
        let gblock = encode_log_block(logs, DEFAULT_RESTART_INTERVAL)?;
        out.extend_from_slice(&gblock);
    }

    // Build footer (header copy + 5x u64 offsets + CRC32).
    let mut footer = header.encode();
    footer.extend_from_slice(&ref_index_position.to_be_bytes());
    footer.extend_from_slice(&0u64.to_be_bytes()); // obj_position (59b) | obj_id_len (5b)
    footer.extend_from_slice(&0u64.to_be_bytes()); // obj_index_position
    footer.extend_from_slice(&log_position.to_be_bytes());
    footer.extend_from_slice(&0u64.to_be_bytes()); // log_index_position
    let crc = crc32fast::hash(&footer);
    footer.extend_from_slice(&crc.to_be_bytes());
    out.extend_from_slice(&footer);
    Ok(out)
}

/// Verified reader over a single immutable `.ref` table buffer.
#[derive(Debug, Clone)]
pub struct TableReader {
    data: Vec<u8>,
    footer: TableFooter,
}

impl TableReader {
    /// Parse and verify header, footer, and CRC-32 checksum of a `.ref` table.
    pub fn from_bytes(data: Vec<u8>) -> Result<Self, RefsError> {
        let header = TableHeader::decode(&data)?;
        let fsize = header.footer_size();
        if data.len() < header.header_size() + fsize {
            return Err(RefsError::Corrupt(
                "reftable: file smaller than header + footer".into(),
            ));
        }
        let footer_start = data.len() - fsize;
        let fbytes = &data[footer_start..];
        let expected_crc = u32::from_be_bytes(fbytes[fsize - 4..].try_into().unwrap());
        let actual_crc = crc32fast::hash(&fbytes[..fsize - 4]);
        if expected_crc != actual_crc {
            return Err(RefsError::Corrupt(format!(
                "reftable: footer CRC-32 mismatch (expected 0x{expected_crc:08x}, got 0x{actual_crc:08x})"
            )));
        }
        let footer_header = TableHeader::decode(fbytes)?;
        if footer_header != header {
            return Err(RefsError::Corrupt(
                "reftable: header and footer metadata mismatch".into(),
            ));
        }
        let off = header.header_size();
        let ref_index_position = u64::from_be_bytes(fbytes[off..off + 8].try_into().unwrap());
        let obj_pos_and_len = u64::from_be_bytes(fbytes[off + 8..off + 16].try_into().unwrap());
        let obj_position = obj_pos_and_len >> 5;
        let obj_id_len = (obj_pos_and_len & 0x1f) as u8;
        let obj_index_position = u64::from_be_bytes(fbytes[off + 16..off + 24].try_into().unwrap());
        let log_position = u64::from_be_bytes(fbytes[off + 24..off + 32].try_into().unwrap());
        let log_index_position = u64::from_be_bytes(fbytes[off + 32..off + 40].try_into().unwrap());

        Ok(Self {
            data,
            footer: TableFooter {
                header,
                ref_index_position,
                obj_position,
                obj_id_len,
                obj_index_position,
                log_position,
                log_index_position,
                crc32: expected_crc,
            },
        })
    }

    pub fn header(&self) -> TableHeader {
        self.footer.header
    }

    pub fn footer(&self) -> TableFooter {
        self.footer
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    fn footer_start(&self) -> usize {
        self.data.len() - self.footer.header.footer_size()
    }

    fn ref_section_end(&self) -> usize {
        let mut end = self.footer_start();
        for &pos in &[
            self.footer.ref_index_position,
            self.footer.obj_position,
            self.footer.log_position,
        ] {
            if pos > 0 && (pos as usize) < end {
                end = pos as usize;
            }
        }
        end
    }

    fn slice_block_at(&self, offset: usize) -> Result<(&[u8], usize, usize), RefsError> {
        let h_off = if offset == 0 {
            self.footer.header.header_size()
        } else {
            0
        };
        let start = offset + h_off;
        if start + 4 > self.footer_start() {
            return Err(RefsError::Corrupt(
                "reftable: block offset past footer".into(),
            ));
        }
        let (_btype, total_len) = read_block_header(&self.data[start..])?;
        let block_end = offset + total_len;
        if block_end > self.footer_start() || block_end < start + 4 {
            return Err(RefsError::Corrupt("reftable: invalid block length".into()));
        }
        Ok((&self.data[start..block_end], h_off, block_end))
    }

    /// Look up a single ref by exact name using index block + restart binary search.
    pub fn seek_ref(&self, name: &str) -> Result<Option<RefRecord>, RefsError> {
        let ref_end = self.ref_section_end();
        let hlen = self.footer.header.header_size();
        if ref_end <= hlen {
            return Ok(None);
        }

        // If a ref index block exists, binary search it to find the exact `'r'` block offset.
        if self.footer.ref_index_position > 0 {
            let (iblock, _, _) = self.slice_block_at(self.footer.ref_index_position as usize)?;
            let idx_recs = decode_index_block(iblock)?;
            let target_bytes = name.as_bytes();
            let pos = idx_recs.partition_point(|r| r.last_key.as_slice() < target_bytes);
            if pos >= idx_recs.len() {
                return Ok(None);
            }
            let block_off = idx_recs[pos].offset as usize;
            let (rblock, h_off, _) = self.slice_block_at(block_off)?;
            return seek_ref_in_block(
                rblock,
                h_off,
                self.footer.header.min_update_index,
                self.footer.header.oid_kind,
                name,
            );
        }

        // Otherwise scan the ref block(s) directly (usually 1 block).
        let mut off = 0usize;
        let bs = self.footer.header.block_size as usize;
        while off < ref_end {
            let (rblock, h_off, end) = self.slice_block_at(off)?;
            if rblock[0] != BLOCK_TYPE_REF {
                break;
            }
            if let Some(found) = seek_ref_in_block(
                rblock,
                h_off,
                self.footer.header.min_update_index,
                self.footer.header.oid_kind,
                name,
            )? {
                return Ok(Some(found));
            }
            off = advance_block_offset(end, bs, ref_end);
        }
        Ok(None)
    }

    /// Read all ref records in sorted order from this table.
    pub fn all_refs(&self) -> Result<Vec<RefRecord>, RefsError> {
        let ref_end = self.ref_section_end();
        let hlen = self.footer.header.header_size();
        if ref_end <= hlen {
            return Ok(Vec::new());
        }
        let mut out = Vec::new();
        let mut off = 0usize;
        let bs = self.footer.header.block_size as usize;
        while off < ref_end {
            let (rblock, h_off, end) = self.slice_block_at(off)?;
            if rblock[0] != BLOCK_TYPE_REF {
                break;
            }
            let recs = decode_ref_block(
                rblock,
                h_off,
                self.footer.header.min_update_index,
                self.footer.header.oid_kind,
            )?;
            out.extend(recs);
            off = advance_block_offset(end, bs, ref_end);
        }
        Ok(out)
    }

    /// Read all log records from this table.
    pub fn all_logs(&self) -> Result<Vec<LogRecord>, RefsError> {
        if self.footer.log_position == 0 {
            return Ok(Vec::new());
        }
        let log_end = if self.footer.log_index_position > 0 {
            self.footer.log_index_position as usize
        } else {
            self.footer_start()
        };
        let mut out = Vec::new();
        let mut off = self.footer.log_position as usize;
        let bs = self.footer.header.block_size as usize;
        while off < log_end {
            let (gblock, _, end) = self.slice_block_at(off)?;
            if gblock[0] != BLOCK_TYPE_LOG {
                break;
            }
            out.extend(decode_log_block(gblock, self.footer.header.oid_kind)?);
            off = advance_block_offset(end, bs, log_end);
        }
        Ok(out)
    }

    /// Strictly verify every block (refs, index, logs) and ordering invariants.
    pub fn verify(&self) -> Result<(), RefsError> {
        let refs = self.all_refs()?;
        for w in refs.windows(2) {
            if w[0].refname >= w[1].refname {
                return Err(RefsError::Corrupt(format!(
                    "reftable: ref records out of order ({} >= {})",
                    w[0].refname, w[1].refname
                )));
            }
        }
        if self.footer.ref_index_position > 0 {
            let (iblock, _, _) = self.slice_block_at(self.footer.ref_index_position as usize)?;
            if iblock[0] != BLOCK_TYPE_INDEX {
                return Err(RefsError::Corrupt(
                    "reftable: ref_index_position does not point to an 'i' block".into(),
                ));
            }
            let _ = decode_index_block(iblock)?;
        }
        let _ = self.all_logs()?;
        Ok(())
    }
}

fn advance_block_offset(end: usize, block_size: usize, limit: usize) -> usize {
    if block_size == 0 {
        return end;
    }
    let rem = end % block_size;
    let next = if rem == 0 {
        end
    } else {
        end + (block_size - rem)
    };
    next.min(limit)
}
