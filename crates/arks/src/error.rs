//! CLI error type (exit non-zero; honest refuse messages).

use std::fmt;

#[derive(Debug)]
pub enum CliError {
    Usage(String),
    NotImplemented(String),
    Msg(String),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage(m) | Self::NotImplemented(m) | Self::Msg(m) => f.write_str(m),
        }
    }
}

impl std::error::Error for CliError {}

impl From<arks_repo::RepoError> for CliError {
    fn from(e: arks_repo::RepoError) -> Self {
        Self::Msg(e.to_string())
    }
}

impl From<arks_refs::RefsError> for CliError {
    fn from(e: arks_refs::RefsError) -> Self {
        match e {
            arks_refs::RefsError::LockFailed(m) => Self::Msg(format!(
                "{m}; AGENT-003: backoff/retry with jitter — blind retry herds amplify lock storms; no lease scheduler in M12"
            )),
            other => Self::Msg(other.to_string()),
        }
    }
}

impl From<arks_store::StoreError> for CliError {
    fn from(e: arks_store::StoreError) -> Self {
        Self::Msg(e.to_string())
    }
}

impl From<arks_object::ObjectError> for CliError {
    fn from(e: arks_object::ObjectError) -> Self {
        Self::Msg(e.to_string())
    }
}

impl From<arks_index::IndexError> for CliError {
    fn from(e: arks_index::IndexError) -> Self {
        match e {
            arks_index::IndexError::LockFailed(m) => Self::Msg(format!(
                "{m}; AGENT-003: backoff/retry with jitter — blind retry herds amplify lock storms; no lease scheduler in M12"
            )),
            other => Self::Msg(other.to_string()),
        }
    }
}

impl From<arks_oid::OidError> for CliError {
    fn from(e: arks_oid::OidError) -> Self {
        Self::Msg(e.to_string())
    }
}

impl From<std::io::Error> for CliError {
    fn from(e: std::io::Error) -> Self {
        Self::Msg(e.to_string())
    }
}

impl From<arks_pack::PackError> for CliError {
    fn from(e: arks_pack::PackError) -> Self {
        Self::Msg(e.to_string())
    }
}

impl From<arks_remote::RemoteError> for CliError {
    fn from(e: arks_remote::RemoteError) -> Self {
        match e {
            arks_remote::RemoteError::NotImplemented(m) => Self::NotImplemented(m),
            arks_remote::RemoteError::UnsupportedUrl(u) => Self::NotImplemented(format!(
                "unsupported remote URL (claimed: http(s), ssh://, user@host:path, local/file://): {u}"
            )),
            other => Self::Msg(other.to_string()),
        }
    }
}

impl From<arks_lease::LeaseError> for CliError {
    fn from(e: arks_lease::LeaseError) -> Self {
        match e {
            arks_lease::LeaseError::AlreadyHeld {
                resource_id,
                holder,
            } => Self::Msg(format!(
                "lease already held by {holder} on {resource_id}; AGENT-003: wait/backoff with jitter — do not stampede Git locks (M13 Cd-1 lease coordination; lease ≠ skip locks)"
            )),
            other => Self::Msg(other.to_string()),
        }
    }
}

pub type CliResult<T> = Result<T, CliError>;
