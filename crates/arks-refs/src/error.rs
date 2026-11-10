//! Ref store errors.

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum RefsError {
    #[error("invalid refname: {0}")]
    InvalidRefname(String),
    #[error("ref not found: {0}")]
    NotFound(String),
    #[error("ref lock failed: {0}")]
    LockFailed(String),
    #[error("ref update rejected: expected {expected}, found {actual}")]
    UpdateRejected { expected: String, actual: String },
    #[error("symbolic ref loop or too deep while resolving: {0}")]
    SymbolicLoop(String),
    #[error("corrupt ref file: {0}")]
    Corrupt(String),
    #[error("I/O error: {0}")]
    Io(String),
}

impl From<std::io::Error> for RefsError {
    fn from(e: std::io::Error) -> Self {
        if e.kind() == std::io::ErrorKind::AlreadyExists {
            Self::LockFailed(e.to_string())
        } else {
            Self::Io(e.to_string())
        }
    }
}
