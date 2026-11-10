//! L2 durable lease state under `$GIT_DIR/arks/leases/`.

use crate::error::LeaseError;
use crate::manager::LeaseManager;
use crate::types::LeaseRecord;
use std::fs;
use std::path::{Path, PathBuf};

/// Relative durable path (stock Git ignores unknown `.git` subtrees).
pub const LEASES_REL: &str = "arks/leases/leases.json";

/// On-disk lease table for L2 (durable across arks processes).
#[derive(Debug)]
pub struct DurableLeaseStore {
    path: PathBuf,
    manager: LeaseManager,
}

impl DurableLeaseStore {
    /// Open or create the durable store under `git_dir`.
    pub fn open(git_dir: impl AsRef<Path>) -> Result<Self, LeaseError> {
        let path = git_dir.as_ref().join(LEASES_REL);
        let mut store = Self {
            path,
            manager: LeaseManager::new(),
        };
        store.reload()?;
        Ok(store)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn manager(&self) -> &LeaseManager {
        &self.manager
    }

    pub fn manager_mut(&mut self) -> &mut LeaseManager {
        &mut self.manager
    }

    /// Load from disk (missing file = empty table).
    pub fn reload(&mut self) -> Result<(), LeaseError> {
        if !self.path.exists() {
            self.manager = LeaseManager::new();
            return Ok(());
        }
        let bytes = fs::read(&self.path).map_err(|e| LeaseError::Io(e.to_string()))?;
        if bytes.is_empty() {
            self.manager = LeaseManager::new();
            return Ok(());
        }
        let records: Vec<LeaseRecord> =
            serde_json::from_slice(&bytes).map_err(|e| LeaseError::Corrupt(e.to_string()))?;
        self.manager.replace_all(records);
        Ok(())
    }

    /// Persist current manager snapshot.
    pub fn save(&self) -> Result<(), LeaseError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|e| LeaseError::Io(e.to_string()))?;
        }
        let records = self.manager.snapshot();
        let bytes =
            serde_json::to_vec_pretty(&records).map_err(|e| LeaseError::Corrupt(e.to_string()))?;
        let tmp = self.path.with_extension("json.tmp");
        fs::write(&tmp, &bytes).map_err(|e| LeaseError::Io(e.to_string()))?;
        fs::rename(&tmp, &self.path).map_err(|e| LeaseError::Io(e.to_string()))?;
        Ok(())
    }
}
