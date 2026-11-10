//! Lease errors.

use thiserror::Error;

/// Errors from lease grant / release / durable I/O / mode valves.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum LeaseError {
    #[error("resource already leased by {holder}: {resource_id}")]
    AlreadyHeld { resource_id: String, holder: String },
    #[error("lease not held by {holder}: {resource_id}")]
    NotHeld { resource_id: String, holder: String },
    #[error("lease not found: {0}")]
    NotFound(String),
    /// Historical M10/M22 refuse variant — unused after M23 unlock; retained so
    /// older match arms / CLI mappings stay compiling. Prefer not to emit.
    #[error(
        "WriterMode::HighConcurrency refused (legacy): L3 claimed shape is Claimed — use WriterMode::HighConcurrency for the lease+batch-apply path"
    )]
    HighConcurrencyRefused,
    #[error("durable lease I/O: {0}")]
    Io(String),
    #[error("durable lease corrupt: {0}")]
    Corrupt(String),
    #[error("L3 pipeline: {0}")]
    Pipeline(String),
}
