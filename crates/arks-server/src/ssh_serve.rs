//! In-process SSH listener for `arks serve` (Partial — fail-closed without SSH FFI).
//!
//! Max honest wire today:
//! - resolve host-key + `authorized_keys` paths (env / options / defaults)
//! - prove TCP bind on the requested listen address
//! - refuse closed: no russh/libssh/OpenSSL SSH stack in-tree (FFI Missing)
//!
//! Ops SSHD + `arks upload-pack`/`receive-pack` remains the valid SSH remote path.
//! Cite: [[sot:root/arks-self-host-server]]

use crate::error::ServerError;
use std::net::{SocketAddr, TcpListener};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

/// Options for the in-process SSH git listener (Partial; not Claimed).
#[derive(Debug, Clone)]
pub struct ServeSshOptions {
    pub listen: SocketAddr,
    /// Repository or base path the listener would serve.
    pub path: PathBuf,
    /// Host key path (OpenSSH-format expected when Claimed). Default / env override.
    pub host_key: PathBuf,
    /// `authorized_keys` path for public-key auth (when Claimed).
    pub authorized_keys: PathBuf,
}

impl ServeSshOptions {
    /// Build options with default key paths under `~/.config/arks/ssh/` (or env).
    pub fn with_defaults(listen: SocketAddr, path: PathBuf) -> Self {
        Self {
            listen,
            path,
            host_key: default_host_key_path(),
            authorized_keys: default_authorized_keys_path(),
        }
    }
}

/// Default host key: `ARKS_SERVE_SSH_HOST_KEY` or `~/.config/arks/ssh/host_key`.
pub fn default_host_key_path() -> PathBuf {
    if let Ok(p) = std::env::var("ARKS_SERVE_SSH_HOST_KEY") {
        if !p.is_empty() {
            return PathBuf::from(p);
        }
    }
    arks_config_dir().join("ssh").join("host_key")
}

/// Default authorized_keys: `ARKS_SERVE_SSH_AUTHORIZED_KEYS` or
/// `~/.config/arks/ssh/authorized_keys`.
pub fn default_authorized_keys_path() -> PathBuf {
    if let Ok(p) = std::env::var("ARKS_SERVE_SSH_AUTHORIZED_KEYS") {
        if !p.is_empty() {
            return PathBuf::from(p);
        }
    }
    arks_config_dir().join("ssh").join("authorized_keys")
}

fn arks_config_dir() -> PathBuf {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    home.join(".config").join("arks")
}

fn path_status(p: &Path) -> &'static str {
    if p.is_file() {
        "present"
    } else if p.exists() {
        "exists-not-file"
    } else {
        "missing"
    }
}

/// Attempt to run an in-process SSH listener.
///
/// Advances as far as honest wire without inventing Claimed SSH:
/// validates key paths, proves `TcpListener::bind`, then **fail-closed**
/// because the SSH protocol stack / FFI is Missing (no russh/libssh dep).
///
/// Callers MUST NOT silent-degrade to HTTP-only when SSH was left enabled.
pub fn serve_ssh_listener(
    opts: ServeSshOptions,
    _stop: Arc<AtomicBool>,
) -> Result<(), ServerError> {
    let host_st = path_status(&opts.host_key);
    let auth_st = path_status(&opts.authorized_keys);
    let repo_st = if opts.path.exists() {
        "present"
    } else {
        "missing"
    };

    // Prove the listen address is bindable (max honest TCP wire). Drop immediately —
    // staying up without an SSH handshake would mislead clients.
    let bind_note = match TcpListener::bind(opts.listen) {
        Ok(listener) => {
            let local = listener
                .local_addr()
                .map(|a| a.to_string())
                .unwrap_or_else(|_| opts.listen.to_string());
            drop(listener);
            format!("tcp_bind=ok local={local}")
        }
        Err(e) => {
            return Err(e.into());
        }
    };

    Err(ServerError::NotImplemented(format!(
        "in-process SSH listener for `arks serve` is Partial/Missing — fail-closed \
         ([[sot:root/arks-self-host-server]]); SSH protocol FFI not in-tree \
         (no russh/libssh Claimed). wire={{listen={listen}; {bind_note}; \
         host_key={hk} ({host_st}); authorized_keys={ak} ({auth_st}); \
         path={path} ({repo_st})}}. use `--no-ssh`, or ops SSHD + \
         `arks upload-pack`/`receive-pack` for SSH remotes today; \
         set ARKS_SERVE_SSH_HOST_KEY / ARKS_SERVE_SSH_AUTHORIZED_KEYS or \
         `arks serve config set ssh_host_key|…` when preparing keys",
        listen = opts.listen,
        hk = opts.host_key.display(),
        ak = opts.authorized_keys.display(),
        path = opts.path.display(),
    )))
}
