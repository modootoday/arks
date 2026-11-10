//! In-process SSH listener for `arks serve` (Partial — fail-closed without SSH FFI).
//!
//! Max honest wire today:
//! - resolve host-key + `authorized_keys` paths (env / options / defaults)
//! - prove TCP bind on the requested listen address
//! - optional one-shot identification exchange (RFC 4253 banners)
//! - optional SSH_MSG_KEXINIT binary-packet scaffold (algorithm lists advertise Missing)
//! - host-key / authorized_keys file-format probes (auth scaffolding; no crypto Claimed)
//! - refuse closed: no russh/libssh/OpenSSL SSH stack in-tree (KEX crypto / auth / channel Missing)
//!
//! Ops SSHD + `arks upload-pack`/`receive-pack` remains the valid SSH remote path.
//! Cite: [[sot:root/arks-self-host-server]]

use crate::error::ServerError;
use std::io::{Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
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

/// Probe host-key bytes for known OpenSSH / PEM markers (auth scaffolding; no load Claimed).
fn probe_host_key_format(path: &Path) -> String {
    if !path.is_file() {
        return "absent".into();
    }
    let mut f = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(e) => return format!("unreadable ({e})"),
    };
    let mut buf = [0u8; 96];
    let n = f.read(&mut buf).unwrap_or(0);
    let head = String::from_utf8_lossy(&buf[..n]);
    if head.starts_with("openssh-key-v1") || head.contains("BEGIN OPENSSH PRIVATE KEY") {
        "openssh-private-key-marker".into()
    } else if head.contains("BEGIN RSA PRIVATE KEY")
        || head.contains("BEGIN EC PRIVATE KEY")
        || head.contains("BEGIN PRIVATE KEY")
    {
        "pem-private-key-marker".into()
    } else if head.starts_with("ssh-") || head.starts_with("ecdsa-") || head.starts_with("sk-") {
        "public-key-line-like".into()
    } else if n == 0 {
        "empty-file".into()
    } else {
        "unknown-bytes".into()
    }
}

/// Count non-empty, non-comment lines in authorized_keys (auth scaffolding; no verify Claimed).
fn probe_authorized_keys(path: &Path) -> String {
    if !path.is_file() {
        return "absent".into();
    }
    let raw = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => return format!("unreadable ({e})"),
    };
    let keys = raw
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .count();
    format!("lines={keys}")
}

fn ssh_name_list(names: &[&str]) -> Vec<u8> {
    let joined = names.join(",");
    let mut out = Vec::with_capacity(4 + joined.len());
    out.extend_from_slice(&(joined.len() as u32).to_be_bytes());
    out.extend_from_slice(joined.as_bytes());
    out
}

/// RFC 4253 SSH_MSG_KEXINIT payload with algorithms that advertise Missing crypto.
/// This is scaffolding only — must not be read as negotiated KEX Claimed.
fn build_kexinit_payload() -> Vec<u8> {
    let mut p = Vec::with_capacity(256);
    p.push(20); // SSH_MSG_KEXINIT
    p.extend_from_slice(&[0u8; 16]); // cookie (zeros; Partial)
    // Explicit Missing names — do not advertise curve25519/ssh-ed25519 as if supported.
    p.extend(ssh_name_list(&["arks-partial-missing-kex"]));
    p.extend(ssh_name_list(&["arks-partial-missing-hostkey"]));
    for _ in 0..6 {
        // encryption / mac / compression × direction
        p.extend(ssh_name_list(&["arks-partial-missing"]));
    }
    p.extend(ssh_name_list(&[])); // languages c→s
    p.extend(ssh_name_list(&[])); // languages s→c
    p.push(0); // first_kex_packet_follows = false
    p.extend_from_slice(&0u32.to_be_bytes()); // reserved
    p
}

/// Write one unencrypted SSH binary packet (no MAC — pre-NEWKEYS).
fn write_ssh_packet(stream: &mut TcpStream, payload: &[u8]) -> std::io::Result<()> {
    // Unencrypted: (packet_length + 4) must be multiple of 8; padding ≥ 4.
    let mut padding_len = 4usize;
    while (4 + 1 + payload.len() + padding_len) % 8 != 0 {
        padding_len += 1;
    }
    let packet_length = (1 + payload.len() + padding_len) as u32;
    stream.write_all(&packet_length.to_be_bytes())?;
    stream.write_all(&[padding_len as u8])?;
    stream.write_all(payload)?;
    stream.write_all(&vec![0u8; padding_len])?;
    Ok(())
}

fn try_kexinit_scaffold(stream: &mut TcpStream) -> String {
    let payload = build_kexinit_payload();
    if let Err(e) = write_ssh_packet(stream, &payload) {
        return format!("kexinit_scaffold=write_err ({e})");
    }
    // Best-effort peer binary packet length peek (may be peer KEXINIT).
    let mut lenbuf = [0u8; 4];
    match stream.read(&mut lenbuf) {
        Ok(0) => "kexinit_scaffold=sent peer_pkt=(eof)".into(),
        Ok(n) if n < 4 => format!("kexinit_scaffold=sent peer_pkt=(short {n})"),
        Ok(_) => {
            let peer_len = u32::from_be_bytes(lenbuf);
            // Drain a little so we do not leave a half-open peer spinning; then close.
            let drain = (peer_len as usize).min(256);
            let mut sink = vec![0u8; drain];
            let _ = stream.read(&mut sink);
            format!("kexinit_scaffold=sent peer_pkt_len={peer_len}")
        }
        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock || e.kind() == std::io::ErrorKind::TimedOut => {
            "kexinit_scaffold=sent peer_pkt=(timeout)".into()
        }
        Err(e) => format!("kexinit_scaffold=sent peer_pkt=(err {e})"),
    }
}

/// Attempt to run an in-process SSH listener.
///
/// Advances as far as honest wire without inventing Claimed SSH:
/// validates key paths, proves `TcpListener::bind`, optionally exchanges
/// RFC 4253 identification + KEXINIT scaffold with one peer, then **fail-closed**
/// because KEX crypto / auth / channel mux (SSH FFI) is Missing.
///
/// Callers MUST NOT silent-degrade to HTTP-only when SSH was left enabled.
pub fn serve_ssh_listener(
    opts: ServeSshOptions,
    _stop: Arc<AtomicBool>,
) -> Result<(), ServerError> {
    let host_st = path_status(&opts.host_key);
    let auth_st = path_status(&opts.authorized_keys);
    let host_fmt = probe_host_key_format(&opts.host_key);
    let auth_probe = probe_authorized_keys(&opts.authorized_keys);
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

    // Identification + KEXINIT scaffold are plaintext (RFC 4253 §4–§7). Proving we
    // can emit banners / KEXINIT shape does **not** invent crypto/auth Claimed.
    listener.set_nonblocking(true)?;
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    let id_note = loop {
        match listener.accept() {
            Ok((mut stream, peer)) => {
                let _ = stream.set_nonblocking(false);
                let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
                let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
                // Server identification (Partial softwareversion — not a Claimed SSH stack).
                let ours = b"SSH-2.0-arks_partial_Missing_crypto\r\n";
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
                let kex_note = if peer_id.is_empty() {
                    "kexinit_scaffold=skipped (no peer id)".into()
                } else {
                    try_kexinit_scaffold(&mut stream)
                };
                let _ = stream.shutdown(Shutdown::Both);
                let id_part = if peer_id.is_empty() {
                    format!("id_exchange=partial peer={peer} peer_id=(empty/timeout)")
                } else {
                    format!("id_exchange=partial peer={peer} peer_id={peer_id}")
                };
                break format!("{id_part}; {kex_note}");
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                if std::time::Instant::now() >= deadline {
                    break "id_exchange=skipped (no peer within 2s accept timeout); kexinit_scaffold=skipped".into();
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(e) => break format!("id_exchange=accept_err ({e}); kexinit_scaffold=skipped"),
        }
    };
    drop(listener);

    Err(ServerError::NotImplemented(format!(
        "in-process SSH listener for `arks serve` is Partial/Missing — fail-closed \
         ([[sot:root/arks-self-host-server]]); SSH KEX crypto/auth/channel FFI not in-tree \
         (no russh/libssh Claimed). wire={{listen={listen}; {bind_note}; {id_note}; \
         host_key={hk} ({host_st}; format={host_fmt}); authorized_keys={ak} ({auth_st}; {auth_probe}); \
         path={path} ({repo_st})}}. Claimed boundary: KEX algorithm negotiation + host-key \
         cryptographic load/sign, publickey auth verify, session/channel, git subsystem — \
         Partial scaffolding only (id exchange + KEXINIT name-lists + key-path probes). \
         use `--no-ssh`, or ops SSHD + `arks upload-pack`/`receive-pack` for SSH remotes today; \
         set ARKS_SERVE_SSH_HOST_KEY / ARKS_SERVE_SSH_AUTHORIZED_KEYS or \
         `arks serve config set ssh_host_key|…` when preparing keys",
        listen = opts.listen,
        hk = opts.host_key.display(),
        ak = opts.authorized_keys.display(),
        path = opts.path.display(),
    )))
}
