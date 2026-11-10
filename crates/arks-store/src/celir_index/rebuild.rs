//! Rebuild product-private CELIR indexes from filesystem Git content arks owns.
//!
//! CELIR supplies build/publish only; OID enumeration is arks-owned
//! ([[sot:root/celir-index-dual-path]] §Rebuild-from-`.git`).

use crate::celir_index::sidecar::sidecar_root;
use crate::{FilesystemGitStore, ObjectStore, StoreError};
use celir_data::{Corpus, Example, ROW_LIMIT};
use celir_index::{StoreLimits, Writer};
use std::path::{Path, PathBuf};

/// Columns for the product-private rebuild schema (not a Git format).
pub const REBUILD_COLUMNS: &[&str] = &["oid", "kind"];

/// Report from a rebuild → publish cycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RebuildReport {
    pub sidecar_root: PathBuf,
    pub generation: u64,
    pub rows: usize,
    /// True when `max_rows` capped the scan before all OIDs were indexed.
    pub capped: bool,
    pub scanned: usize,
}

/// Errors specific to rebuild-from-`.git` (wraps store / CELIR failures).
#[derive(Debug, thiserror::Error)]
pub enum RebuildError {
    #[error("git store: {0}")]
    Store(#[from] StoreError),
    #[error("sidecar I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("celir index: {0}")]
    Celir(String),
    #[error("row budget exceeded ({scanned} objects; limit {limit}); use rebuild_from_git_capped")]
    RowBudget { scanned: usize, limit: usize },
    #[error("no Git objects to index")]
    Empty,
}

impl From<celir_index::StoreError> for RebuildError {
    fn from(value: celir_index::StoreError) -> Self {
        Self::Celir(value.to_string())
    }
}

/// Project Git objects into the product-private corpus schema.
pub fn corpus_from_git(
    store: &FilesystemGitStore,
    max_rows: usize,
) -> Result<(Corpus, usize, bool), RebuildError> {
    let limit = max_rows.min(ROW_LIMIT);
    if limit == 0 {
        return Err(RebuildError::Empty);
    }
    let oids = store.collect_oids()?;
    let scanned = oids.len();
    if scanned == 0 {
        return Err(RebuildError::Empty);
    }
    let capped = scanned > limit;
    let take = if capped { limit } else { scanned };
    let mut examples = Vec::with_capacity(take);
    for (i, oid) in oids.into_iter().take(take).enumerate() {
        let obj = store.get(&oid)?;
        let line = u16::try_from(i + 1).unwrap_or(u16::MAX);
        examples.push(Example {
            inputs: vec![oid.to_hex(), obj.kind.as_str().to_owned()],
            effect: obj.payload.len().to_string(),
            line,
        });
    }
    Ok((
        Corpus {
            columns: REBUILD_COLUMNS.iter().map(|s| (*s).to_owned()).collect(),
            examples,
        },
        scanned,
        capped,
    ))
}

/// Rebuild and publish into `<git-dir>/arks/celir-index/`, failing closed on row budget.
pub fn rebuild_from_git(
    store: &FilesystemGitStore,
    git_dir: impl AsRef<Path>,
) -> Result<RebuildReport, RebuildError> {
    let oids = store.collect_oids()?;
    if oids.len() > ROW_LIMIT {
        return Err(RebuildError::RowBudget {
            scanned: oids.len(),
            limit: ROW_LIMIT,
        });
    }
    rebuild_from_git_capped(store, git_dir, ROW_LIMIT)
}

/// Rebuild with an explicit row cap; sets `capped` when not all OIDs were indexed.
pub fn rebuild_from_git_capped(
    store: &FilesystemGitStore,
    git_dir: impl AsRef<Path>,
    max_rows: usize,
) -> Result<RebuildReport, RebuildError> {
    let sidecar = sidecar_root(git_dir);
    let (corpus, scanned, capped) = corpus_from_git(store, max_rows)?;
    let rows = corpus.examples.len();
    // `Writer::create` requires a missing leaf dir; parent `arks/` may exist.
    if let Some(parent) = sidecar.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if sidecar.exists() {
        std::fs::remove_dir_all(&sidecar)?;
    }
    let mut writer = Writer::create(&sidecar, StoreLimits::default())?;
    let generation = writer.commit_snapshot(&corpus, "arks-rebuild-from-git")?;
    // Drop writer lock before callers open readonly.
    drop(writer);
    Ok(RebuildReport {
        sidecar_root: sidecar,
        generation,
        rows,
        capped,
        scanned,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::celir_index::query_surface::{lookup_oid_kind, open_sidecar};
    use arks_hash::ObjectFormat;
    use arks_object::{ObjectKind, RawObject};
    use tempfile::TempDir;

    #[test]
    fn rebuild_publish_open_lookup_roundtrip() {
        let dir = TempDir::new().unwrap();
        let git = dir.path().join(".git");
        let objects = git.join("objects");
        std::fs::create_dir_all(&objects).unwrap();
        let mut store = FilesystemGitStore::new(&objects).unwrap();
        let blob = RawObject::new(ObjectKind::Blob, b"celir-rebuild");
        let oid = store.put(ObjectFormat::Sha1, &blob).unwrap();

        let report = rebuild_from_git(&store, &git).expect("rebuild");
        assert_eq!(report.rows, 1);
        assert!(!report.capped);
        assert_eq!(report.generation, 1);
        assert!(
            report.sidecar_root.ends_with(".arks\\celir-index")
                || report.sidecar_root.ends_with(".arks/celir-index")
        );
        assert!(!report.sidecar_root.starts_with(&git));

        let store_ro = open_sidecar(&report.sidecar_root).expect("open");
        let mut snap = store_ro.snapshot().expect("snapshot");
        let effect = lookup_oid_kind(&mut snap, &oid.to_hex(), "blob").expect("lookup");
        assert_eq!(effect, Some(blob.payload.len().to_string()));
    }
}
