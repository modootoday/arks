//! L0 ref locks (`git-update-ref` pattern).
//!
//! Cite: <https://git-scm.com/docs/git-update-ref>
//! Cite: <https://git-scm.com/docs/api-lockfile>

use crate::RefsError;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

/// Path of the exclusive lock sibling for `ref_path`.
pub fn ref_lock_path(ref_path: &Path) -> PathBuf {
    let mut s = ref_path.as_os_str().to_os_string();
    s.push(".lock");
    PathBuf::from(s)
}

fn lock_path(ref_path: &Path) -> PathBuf {
    ref_lock_path(ref_path)
}

/// Create `ref.lock` exclusively, write `payload`, rename over `ref_path`.
///
/// Durability matches stock Git 2.39 when `core.fsync` is unset: no `fsync`
/// before rename (clean-room strace: `git tag` → 0 fsync; forced `sync_all`
/// was a multi-ms tax on Docker bind mounts and broke G19 short porcelain).
/// Cite: <https://git-scm.com/docs/git-config#Documentation/git-config.txt-corefsync>
pub fn write_ref_locked(ref_path: &Path, payload: &[u8]) -> Result<(), RefsError> {
    if let Some(parent) = ref_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| RefsError::Io(e.to_string()))?;
    }
    let lock = lock_path(ref_path);
    let mut f = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&lock)
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::AlreadyExists {
                RefsError::LockFailed(format!(
                    "{} (ref locked; retry after the holder finishes — exclusive Git lock, no skip)",
                    lock.display()
                ))
            } else {
                RefsError::Io(e.to_string())
            }
        })?;
    f.write_all(payload)
        .map_err(|e| RefsError::Io(e.to_string()))?;
    drop(f);
    std::fs::rename(&lock, ref_path).map_err(|e| RefsError::Io(e.to_string()))
}

/// Lock exclusively, delete `ref_path`, remove lock file.
pub fn delete_ref_locked(ref_path: &Path) -> Result<(), RefsError> {
    if let Some(parent) = ref_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| RefsError::Io(e.to_string()))?;
    }
    let lock = lock_path(ref_path);
    let _f: File = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&lock)
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::AlreadyExists {
                RefsError::LockFailed(format!(
                    "{} (ref locked; retry after the holder finishes — exclusive Git lock, no skip)",
                    lock.display()
                ))
            } else {
                RefsError::Io(e.to_string())
            }
        })?;
    if ref_path.exists() {
        std::fs::remove_file(ref_path).map_err(|e| RefsError::Io(e.to_string()))?;
    }
    std::fs::remove_file(&lock).map_err(|e| RefsError::Io(e.to_string()))
}
