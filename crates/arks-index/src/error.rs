//! Index parse/write errors.

use std::path::PathBuf;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum IndexError {
    #[error("index file corrupt: {0}")]
    Corrupt(String),
    #[error("unsupported index version: {0}")]
    UnsupportedVersion(u32),
    #[error("trailing checksum mismatch")]
    Checksum,
    #[error("I/O error: {0}")]
    Io(String),
    #[error("failed to acquire index lock: {0}")]
    LockFailed(String),
    #[error("entries are not sorted by path then stage")]
    UnsortedEntries,
    #[error("refused index extension: {0}")]
    RefusedExtension(String),
    #[error("truncated index file")]
    Truncated,
    #[error("bad index signature (expected DIRC)")]
    BadSignature,
    #[error("path contains NUL byte")]
    PathNul,
    #[error("read path: {path}: {message}")]
    ReadPath {
        path: PathBuf,
        message: String,
    },
}
