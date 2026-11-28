//! Multi-table Git Reftable stack (`reftable/tables.list` + geometric factor-2 compaction).
//!
//! Primary reference: Git `Documentation/technical/reftable.txt`
//! (§Update transactions, §Compaction) and `reftable/stack.c`.

use crate::RefsError;
use crate::reftable::block::{LogRecord, RefRecord, RefValue};
use crate::reftable::table::{DEFAULT_BLOCK_SIZE, TableReader, write_table};
use crate::target::RefTarget;
use arks_oid::{Oid, OidKind};
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

/// Summary of a verified reftable stack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackVerifyReport {
    pub table_count: usize,
    pub total_refs: usize,
    pub max_update_index: u64,
}

/// Manages a directory of immutable `.ref` tables listed in `tables.list`.
#[derive(Debug, Clone)]
pub struct ReftableStack {
    dir: PathBuf,
    oid_kind: OidKind,
}

struct LockFileGuard(PathBuf);
impl Drop for LockFileGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

impl ReftableStack {
    pub fn open(reftable_dir: impl AsRef<Path>, oid_kind: OidKind) -> Self {
        Self {
            dir: reftable_dir.as_ref().to_path_buf(),
            oid_kind,
        }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn tables_list_path(&self) -> PathBuf {
        self.dir.join("tables.list")
    }

    pub fn is_initialized(&self) -> bool {
        self.tables_list_path().is_file()
    }

    /// Initialize an empty `reftable/` directory with an empty `tables.list`.
    pub fn init(&self) -> Result<(), RefsError> {
        fs::create_dir_all(&self.dir)
            .map_err(|e| RefsError::Io(format!("create {}: {e}", self.dir.display())))?;
        let list = self.tables_list_path();
        if !list.exists() {
            fs::write(&list, "")
                .map_err(|e| RefsError::Io(format!("write {}: {e}", list.display())))?;
        }
        Ok(())
    }

    fn acquire_lock(&self) -> Result<LockFileGuard, RefsError> {
        fs::create_dir_all(&self.dir)
            .map_err(|e| RefsError::Io(format!("create {}: {e}", self.dir.display())))?;
        let lock_path = self.dir.join("tables.list.lock");
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock_path)
            .map_err(|_| RefsError::LockFailed(lock_path.display().to_string()))?;
        Ok(LockFileGuard(lock_path))
    }

    pub fn table_names(&self) -> Result<Vec<String>, RefsError> {
        let list_path = self.tables_list_path();
        if !list_path.is_file() {
            return Ok(Vec::new());
        }
        let text = fs::read_to_string(&list_path)
            .map_err(|e| RefsError::Io(format!("read {}: {e}", list_path.display())))?;
        Ok(text
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(String::from)
            .collect())
    }

    fn load_tables(&self) -> Result<Vec<(String, TableReader)>, RefsError> {
        let names = self.table_names()?;
        let mut out = Vec::with_capacity(names.len());
        for name in names {
            let path = self.dir.join(&name);
            let bytes = fs::read(&path)
                .map_err(|e| RefsError::Io(format!("read reftable {}: {e}", path.display())))?;
            let reader = TableReader::from_bytes(bytes)?;
            out.push((name, reader));
        }
        Ok(out)
    }

    /// Read a single ref by exact name (newest table wins; deletion tombstones return `NotFound`).
    pub fn read_ref(&self, name: &str) -> Result<RefTarget, RefsError> {
        let tables = self.load_tables()?;
        for (_tname, reader) in tables.iter().rev() {
            if let Some(rec) = reader.seek_ref(name)? {
                return match rec.value {
                    RefValue::Deletion => Err(RefsError::NotFound(name.to_string())),
                    RefValue::Direct(oid) | RefValue::Peeled { value: oid, .. } => {
                        Ok(RefTarget::Direct(oid))
                    }
                    RefValue::Symbolic(target) => Ok(RefTarget::Symbolic(target)),
                };
            }
        }
        Err(RefsError::NotFound(name.to_string()))
    }

    /// Merged live ref map across the entire stack (tombstones removed).
    pub fn merged_refs(&self) -> Result<BTreeMap<String, RefValue>, RefsError> {
        let tables = self.load_tables()?;
        let mut map: BTreeMap<String, RefValue> = BTreeMap::new();
        for (_tname, reader) in &tables {
            for rec in reader.all_refs()? {
                match rec.value {
                    RefValue::Deletion => {
                        map.remove(&rec.refname);
                    }
                    other => {
                        map.insert(rec.refname, other);
                    }
                }
            }
        }
        Ok(map)
    }

    /// List live direct refs matching `prefix`, sorted by refname.
    pub fn list_refs(&self, prefix: &str) -> Result<Vec<(String, Oid)>, RefsError> {
        let merged = self.merged_refs()?;
        let mut out = Vec::new();
        for (name, val) in &merged {
            if !name.starts_with(prefix) {
                continue;
            }
            if let Some(oid) = resolve_in_map(&merged, name, val) {
                out.push((name.clone(), oid));
            }
        }
        Ok(out)
    }

    /// Atomically append a batch of ref updates (and optional logs) as a new immutable `.ref`
    /// table, then run geometric factor-2 compaction if needed.
    pub fn append_transaction(
        &self,
        mut updates: Vec<(String, RefValue)>,
        logs: Vec<LogRecord>,
    ) -> Result<u64, RefsError> {
        if updates.is_empty() && logs.is_empty() {
            return Ok(0);
        }
        let _lock = self.acquire_lock()?;
        let mut tables = self.load_tables()?;
        let next_idx = tables
            .iter()
            .map(|(_, t)| t.header().max_update_index)
            .max()
            .unwrap_or(0)
            + 1;

        updates.sort_by(|a, b| a.0.cmp(&b.0));
        updates.dedup_by(|a, b| a.0 == b.0);
        let ref_records: Vec<RefRecord> = updates
            .into_iter()
            .map(|(refname, value)| RefRecord {
                refname,
                update_index: next_idx,
                value,
            })
            .collect();

        let table_bytes = write_table(
            &ref_records,
            &logs,
            next_idx,
            next_idx,
            self.oid_kind,
            DEFAULT_BLOCK_SIZE,
        )?;
        let hash_suffix = crc32fast::hash(&table_bytes);
        let new_name = format!("0x{next_idx:012x}-0x{next_idx:012x}-{hash_suffix:08x}.ref");
        let new_path = self.dir.join(&new_name);
        write_atomic(&new_path, &table_bytes)?;

        let new_reader = TableReader::from_bytes(table_bytes)?;
        tables.push((new_name, new_reader));

        // Update tables.list atomically.
        self.write_tables_list(&tables)?;

        // Geometric factor-2 auto-compaction (matching git/reftable/stack.c).
        self.auto_compact_locked(&mut tables)?;
        Ok(next_idx)
    }

    /// Compact the tail of the stack whenever `size[i-1] < 2 * size[i]` (geometric factor 2).
    fn auto_compact_locked(
        &self,
        tables: &mut Vec<(String, TableReader)>,
    ) -> Result<(), RefsError> {
        let Some(first) = suggest_compaction_start(tables) else {
            return Ok(());
        };
        self.compact_range_locked(tables, first)
    }

    /// Force full compaction of all tables into one single immutable table.
    pub fn compact_all(&self) -> Result<(), RefsError> {
        let _lock = self.acquire_lock()?;
        let mut tables = self.load_tables()?;
        if tables.len() <= 1 {
            return Ok(());
        }
        self.compact_range_locked(&mut tables, 0)
    }

    fn compact_range_locked(
        &self,
        tables: &mut Vec<(String, TableReader)>,
        first: usize,
    ) -> Result<(), RefsError> {
        if first + 1 >= tables.len() {
            return Ok(());
        }
        let slice = &tables[first..];
        let min_idx = slice
            .iter()
            .map(|(_, t)| t.header().min_update_index)
            .min()
            .unwrap_or(1);
        let max_idx = slice
            .iter()
            .map(|(_, t)| t.header().max_update_index)
            .max()
            .unwrap_or(min_idx);
        let keep_deletions = first > 0;

        let mut merged: BTreeMap<String, RefRecord> = BTreeMap::new();
        let mut logs: Vec<LogRecord> = Vec::new();
        for (_name, reader) in slice {
            for rec in reader.all_refs()? {
                merged.insert(rec.refname.clone(), rec);
            }
            logs.extend(reader.all_logs()?);
        }
        let compacted_refs: Vec<RefRecord> = merged
            .into_values()
            .filter(|r| keep_deletions || r.value != RefValue::Deletion)
            .collect();

        let bytes = write_table(
            &compacted_refs,
            &logs,
            min_idx,
            max_idx,
            self.oid_kind,
            DEFAULT_BLOCK_SIZE,
        )?;
        let suffix = crc32fast::hash(&bytes);
        let compacted_name = format!("0x{min_idx:012x}-0x{max_idx:012x}-{suffix:08x}.ref");
        let compacted_path = self.dir.join(&compacted_name);
        write_atomic(&compacted_path, &bytes)?;
        let reader = TableReader::from_bytes(bytes)?;

        let old_names: Vec<String> = tables[first..].iter().map(|(n, _)| n.clone()).collect();
        tables.truncate(first);
        tables.push((compacted_name.clone(), reader));
        self.write_tables_list(tables)?;

        for old in old_names {
            if old != compacted_name {
                let _ = fs::remove_file(self.dir.join(&old));
            }
        }
        Ok(())
    }

    fn write_tables_list(&self, tables: &[(String, TableReader)]) -> Result<(), RefsError> {
        let mut body = String::new();
        for (name, _) in tables {
            body.push_str(name);
            body.push('\n');
        }
        let tmp = self.dir.join("tables.list.tmp");
        write_atomic(&tmp, body.as_bytes())?;
        fs::rename(&tmp, self.tables_list_path())
            .map_err(|e| RefsError::Io(format!("rename tables.list: {e}")))?;
        Ok(())
    }

    /// Strictly verify every table in the stack: CRC-32, ordering, and non-overlapping update indices.
    pub fn verify(&self) -> Result<StackVerifyReport, RefsError> {
        let tables = self.load_tables()?;
        let mut prev_max = 0u64;
        for (idx, (name, reader)) in tables.iter().enumerate() {
            reader.verify()?;
            let h = reader.header();
            if h.min_update_index > h.max_update_index {
                return Err(RefsError::Corrupt(format!(
                    "reftable {name}: min_update_index ({}) > max_update_index ({})",
                    h.min_update_index, h.max_update_index
                )));
            }
            if idx > 0 && h.min_update_index <= prev_max {
                return Err(RefsError::Corrupt(format!(
                    "reftable {name}: overlapping update_index ({} <= {prev_max})",
                    h.min_update_index
                )));
            }
            prev_max = h.max_update_index;
        }
        let live = self.merged_refs()?;
        Ok(StackVerifyReport {
            table_count: tables.len(),
            total_refs: live.len(),
            max_update_index: prev_max,
        })
    }
}

/// Geometric factor-2 compaction heuristic (mirrors `suggest_compaction_segment` in `git/reftable/stack.c`).
fn suggest_compaction_start(tables: &[(String, TableReader)]) -> Option<usize> {
    if tables.len() <= 1 {
        return None;
    }
    let sizes: Vec<usize> = tables.iter().map(|(_, t)| t.len()).collect();
    let mut sum_tail = *sizes.last()?;
    let mut start = None;
    for i in (0..sizes.len() - 1).rev() {
        if sizes[i] < 2 * sum_tail {
            start = Some(i);
            sum_tail += sizes[i];
        } else {
            break;
        }
    }
    start
}

fn resolve_in_map(map: &BTreeMap<String, RefValue>, _name: &str, val: &RefValue) -> Option<Oid> {
    let mut cur = val;
    for _ in 0..32 {
        match cur {
            RefValue::Direct(oid) | RefValue::Peeled { value: oid, .. } => return Some(*oid),
            RefValue::Symbolic(target) => {
                cur = map.get(target)?;
            }
            RefValue::Deletion => return None,
        }
    }
    None
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), RefsError> {
    let mut f = fs::File::create(path)
        .map_err(|e| RefsError::Io(format!("create {}: {e}", path.display())))?;
    f.write_all(bytes)
        .map_err(|e| RefsError::Io(format!("write {}: {e}", path.display())))?;
    f.sync_all()
        .map_err(|e| RefsError::Io(format!("sync {}: {e}", path.display())))?;
    Ok(())
}
