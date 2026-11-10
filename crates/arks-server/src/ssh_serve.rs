//! In-process SSH listener for `arks serve` (Missing/Partial — fail-closed).
//!
//! Ops SSHD + `arks upload-pack`/`receive-pack` remains the valid SSH remote
//! path today. This module exists so the serve umbrella has an honest SSH
//! track entry point without inventing Claimed in-process SSHD.
//!
//! Cite: [[sot:root/arks-self-host-server]]

use crate::error::ServerError;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

/// Options for a future in-process SSH git listener (design only).
#[derive(Debug, Clone)]
pub struct ServeSshOptions {
    pub listen: SocketAddr,
    /// Repository or base path the listener would serve (unused while Missing).
    pub path: PathBuf,
}

/// Attempt to run an in-process SSH listener.
///
/// Always fails closed today — Missing/Partial, not Claimed. Callers MUST NOT
/// silent-degrade to HTTP-only when SSH was left enabled.
pub fn serve_ssh_listener(
    _opts: ServeSshOptions,
    _stop: Arc<AtomicBool>,
) -> Result<(), ServerError> {
    Err(ServerError::NotImplemented(
        "in-process SSH listener for `arks serve` is Missing/Partial \
         ([[sot:root/arks-self-host-server]]); fail-closed — use `--no-ssh`, \
         or ops SSHD + `arks upload-pack`/`receive-pack` for SSH remotes today"
            .into(),
    ))
}
