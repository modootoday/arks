//! Repository discovery / composition errors.

use arks_object::ObjectError;
use arks_refs::RefsError;
use arks_store::StoreError;

#[derive(Debug, thiserror::Error)]
pub enum RepoError {
    #[error("repository not found from {0}")]
    NotFound(String),
    #[error("invalid .git file: {0}")]
    InvalidGitFile(String),
    #[error("refs: {0}")]
    Refs(#[from] RefsError),
    #[error("store: {0}")]
    Store(#[from] StoreError),
    #[error("object: {0}")]
    Object(#[from] ObjectError),
    #[error("I/O: {0}")]
    Io(String),
}

impl PartialEq for RepoError {
    fn eq(&self, other: &Self) -> bool {
        self.to_string() == other.to_string()
    }
}

impl Eq for RepoError {}
