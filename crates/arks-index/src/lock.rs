//! L0 index lock (`index.lock` → `index`).
//!
//! Cite: <https://git-scm.com/docs/api-lockfile>
//! Cite: <https://git-scm.com/docs/gitformat-index>

use crate::error::IndexError;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

/// Path of the exclusive lock sibling for `index_path` (`index` → `index.lock`).
pub fn index_lock_path(index_path: &Path) -> PathBuf {
    let mut s = index_path.as_os_str().to_os_string();
    s.push(".lock");
    PathBuf::from(s)
}

/// Write bytes to `index.lock` exclusively, rename over `index_path`.
///
/// Does **not** skip an existing lock (lease ≠ skip Git locks on Git-compat paths).
/// No `fsync` by default — matches stock Git 2.39 when `core.fsync` is unset
/// (same rationale as `arks-refs` lockfile writes; G19 / M17 Pf3-2).
/// Cite: <https://git-scm.com/docs/git-config#Documentation/git-config.txt-corefsync>
pub fn write_index_locked(index_path: &Path, payload: &[u8]) -> Result<(), IndexError> {
    if let Some(parent) = index_path.parent() {
        fs::create_dir_all(parent).map_err(|e| IndexError::Io(e.to_string()))?;
    }
    let lock = index_lock_path(index_path);
    let mut f = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&lock)
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::AlreadyExists {
                // IDX-001 / LOCK-001 / LOCK-004 honesty: exclusive O_CREAT|O_EXCL;
                // porcelain fail-closed (die class) — no soft skip dialect.
                IndexError::LockFailed(format!(
                    "{} (index locked; LOCK-001 exclusive create; LOCK-004 porcelain die-class; retry after the holder finishes — no skip)",
                    lock.display()
                ))
            } else {
                IndexError::Io(e.to_string())
            }
        })?;
    f.write_all(payload)
        .map_err(|e| IndexError::Io(e.to_string()))?;
    drop(f);
    fs::rename(&lock, index_path).map_err(|e| IndexError::Io(e.to_string()))
}
