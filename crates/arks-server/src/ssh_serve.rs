//! In-process SSH listener for `arks serve` (Partial — fail-closed without SSH FFI).
//!
//! Max honest wire today:
//! - resolve host-key + `authorized_keys` paths (env / options / defaults)
//! - prove TCP bind on the requested listen address
//! - optional one-shot identification exchange (RFC 4253 banners) then fail-closed
//! - refuse closed: no russh/libssh/OpenSSL SSH stack in-tree (KEX/auth Missing)
//!
//! Ops SSHD + `arks upload-pack`/`receive-pack` remains the valid SSH remote path.
//! Cite: [[sot:root/arks-self-host-server]]

use crate::error::ServerError;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

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
/// validates key paths, proves `TcpListener::bind`, optionally exchanges
/// RFC 4253 identification strings with one peer, then **fail-closed**
/// because key exchange / auth / channel mux (SSH FFI) is Missing.
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

    let listener = TcpListener::bind(opts.listen)?;
    let local = listener
        .local_addr()
        .map(|a| a.to_string())
        .unwrap_or_else(|_| opts.listen.to_string());
    let bind_note = format!("tcp_bind=ok local={local}");

    // Identification exchange is plaintext (RFC 4253 §4.2). Proving we can
    // speak the banner line does **not** invent KEX/auth Claimed — we close
    // immediately after (or on accept timeout) with an explicit Missing boundary.
    listener.set_nonblocking(true)?;
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    let id_note = loop {
        match listener.accept() {
            Ok((mut stream, peer)) => {
                let _ = stream.set_nonblocking(false);
                let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
                let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
                // Server identification (Partial softwareversion — not a Claimed SSH stack).
                let ours = b"SSH-2.0-arks_partial_Missing_kex\r\n";
                let _ = stream.write_all(ours);
                let mut buf = [0u8; 256];
                let n = stream.read(&mut buf).unwrap_or(0);
                let peer_raw = String::from_utf8_lossy(&buf[..n]);
                let peer_id = peer_raw
                    .lines()
                    .next()
                    .unwrap_or("")
                    .trim()
                    .chars()
                    .take(80)
                    .collect::<String>();
                let _ = stream.shutdown(std::net::Shutdown::Both);
                break if peer_id.is_empty() {
                    format!("id_exchange=partial peer={peer} peer_id=(empty/timeout)")
                } else {
                    format!("id_exchange=partial peer={peer} peer_id={peer_id}")
                };
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                if std::time::Instant::now() >= deadline {
                    break "id_exchange=skipped (no peer within 2s accept timeout)".into();
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(e) => break format!("id_exchange=accept_err ({e})"),
        }
    };
    drop(listener);

    Err(ServerError::NotImplemented(format!(
        "in-process SSH listener for `arks serve` is Partial/Missing — fail-closed \
         ([[sot:root/arks-self-host-server]]); SSH KEX/auth/channel FFI not in-tree \
         (no russh/libssh Claimed). wire={{listen={listen}; {bind_note}; {id_note}; \
         host_key={hk} ({host_st}); authorized_keys={ak} ({auth_st}); \
         path={path} ({repo_st})}}. Missing boundary: key exchange, host-key load, \
         publickey auth, session/channel, git subsystem. use `--no-ssh`, or ops SSHD + \
         `arks upload-pack`/`receive-pack` for SSH remotes today; \
         set ARKS_SERVE_SSH_HOST_KEY / ARKS_SERVE_SSH_AUTHORIZED_KEYS or \
         `arks serve config set ssh_host_key|…` when preparing keys",
        listen = opts.listen,
        hk = opts.host_key.display(),
        ak = opts.authorized_keys.display(),
        path = opts.path.display(),
    )))
}
