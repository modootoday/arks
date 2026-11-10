//! `Index` read/write orchestration.

use crate::checksum::{append_checksum, checksum_len, read_and_verify_checksum};
use crate::entry::{
    IndexEntry, cmp_entries, entries_sorted, read_entry_v2_v3, read_entry_v4, write_entry_v2,
};
use crate::error::IndexError;
use crate::extension::{IndexExtension, read_extensions, write_extensions};
use crate::header::{parse_header, validate_version, write_header};
use crate::lock::write_index_locked;
use arks_oid::OidKind;
use std::fs;
use std::path::Path;

/// Parsed `.git/index` (subset of `gitformat-index`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Index {
    pub version: u32,
    pub entries: Vec<IndexEntry>,
    pub extensions: Vec<IndexExtension>,
}

impl Index {
    pub fn read(bytes: &[u8], hash_kind: OidKind) -> Result<Self, IndexError> {
        let (version, count, mut offset) = parse_header(bytes)?;
        validate_version(version)?;
        let csum_len = checksum_len(hash_kind);
        if bytes.len() < offset + csum_len {
            return Err(IndexError::Truncated);
        }
        let mut entries = Vec::with_capacity(count as usize);
        let mut prev_path = String::new();
        for _ in 0..count {
            let (entry, next) = if version == 4 {
                read_entry_v4(bytes, offset, &prev_path, hash_kind)?
            } else {
                read_entry_v2_v3(bytes, offset, version, hash_kind)?
            };
            prev_path = entry.path.clone();
            offset = next;
            entries.push(entry);
        }
        let csum_start = bytes.len() - csum_len;
        let (extensions, ext_end) = if offset < csum_start && bytes[offset].is_ascii_uppercase() {
            read_extensions(bytes, offset, csum_len)?
        } else {
            (Vec::new(), offset)
        };
        read_and_verify_checksum(bytes, ext_end, hash_kind)?;
        Ok(Self {
            version,
            entries,
            extensions,
        })
    }

    pub fn read_path(path: &Path, hash_kind: OidKind) -> Result<Self, IndexError> {
        let bytes = fs::read(path).map_err(|e| IndexError::Io(e.to_string()))?;
        Self::read(&bytes, hash_kind).map_err(|e| match e {
            IndexError::Io(_) => e,
            other => IndexError::ReadPath {
                path: path.to_path_buf(),
                message: other.to_string(),
            },
        })
    }

    pub fn write(&self, hash_kind: OidKind) -> Result<Vec<u8>, IndexError> {
        if !entries_sorted(&self.entries) {
            return Err(IndexError::UnsortedEntries);
        }
        // gitformat-index: CE_EXTENDED / long names require version >= 3.
        // Upgrade a v2 index automatically when entries need extended flags.
        let needs_extended = self.entries.iter().any(|e| {
            e.flags_extended.is_some() || e.path.len() >= crate::entry::CE_NAMEMASK as usize
        });
        let write_version = if self.version >= 4 {
            self.version
        } else if needs_extended || self.version >= 3 {
            3
        } else {
            2
        };
        // ~62-byte fixed entry + path; pre-size for clone checkout (220+ blobs).
        let mut buf = Vec::with_capacity(12 + self.entries.len() * 96);
        write_header(&mut buf, write_version, self.entries.len() as u32);
        for entry in &self.entries {
            write_entry_v2(&mut buf, entry, hash_kind, write_version)?;
        }
        let safe: Vec<_> = self
            .extensions
            .iter()
            .filter(|e| crate::extension::is_passthrough_safe(&e.signature))
            .cloned()
            .collect();
        write_extensions(&mut buf, &safe);
        append_checksum(&mut buf, hash_kind);
        Ok(buf)
    }

    pub fn write_path(&self, path: &Path, hash_kind: OidKind) -> Result<(), IndexError> {
        let bytes = self.write(hash_kind)?;
        write_index_locked(path, &bytes)
    }
}

/// Sort entries in place (`memcmp` path order, then stage).
pub fn sort_entries(entries: &mut [IndexEntry]) {
    entries.sort_by(cmp_entries);
}
