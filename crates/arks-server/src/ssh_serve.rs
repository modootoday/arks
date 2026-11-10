//! In-process SSH listener for `arks serve` (Partial — fail-closed without Claimed SSH stack).
//!
//! Max honest wire today (structured stages):
//! 1. resolve host-key + `authorized_keys` paths (env / options / defaults)
//! 2. prove TCP bind on the requested listen address
//! 3. optional RFC 4253 identification exchange
//! 4. optional SSH_MSG_KEXINIT exchange + peer name-list parse
//! 5. local curve25519-sha256 ECDH self-check (+ peer ECDH INIT when offered)
//! 6. host-key sign / publickey auth / channel / git subsystem — **Missing** (fail-closed)
//!
//! Design ≠ Claimed handshake. Ops SSHD + `arks upload-pack`/`receive-pack` remains valid.
//! Cite: [[sot:root/arks-self-host-server]]

use crate::error::ServerError;
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;
use x25519_dalek::{PublicKey, StaticSecret};

/// Named fail-closed stages for in-process SSH Partial (not Claimed).
/// `AuthPublickey` / `ChannelSession` are scaffolding labels in refuse text only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SshStage {
    IdExchange,
    KexInit,
    AlgoSelect,
    Ecdh,
    HostKeySign,
}

impl SshStage {
    fn as_str(self) -> &'static str {
        match self {
            Self::IdExchange => "id_exchange",
            Self::KexInit => "kexinit",
            Self::AlgoSelect => "algo_select",
            Self::Ecdh => "ecdh_curve25519",
            Self::HostKeySign => "host_key_sign",
        }
    }
}

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

fn read_ssh_name_list(buf: &[u8], off: &mut usize) -> Option<Vec<String>> {
    if *off + 4 > buf.len() {
        return None;
    }
    let len = u32::from_be_bytes(buf[*off..*off + 4].try_into().ok()?) as usize;
    *off += 4;
    if *off + len > buf.len() {
        return None;
    }
    let s = std::str::from_utf8(&buf[*off..*off + len]).ok()?;
    *off += len;
    if s.is_empty() {
        return Some(Vec::new());
    }
    Some(s.split(',').map(|x| x.to_string()).collect())
}

/// RFC 4253 SSH_MSG_KEXINIT — advertise real curve25519 KEX name; hostkey/enc remain Missing.
/// Host-key **sign** is not Claimed — do not advertise `ssh-ed25519` as if sign works.
fn build_kexinit_payload() -> Vec<u8> {
    let mut p = Vec::with_capacity(256);
    p.push(20); // SSH_MSG_KEXINIT
    // Non-zero cookie from SHA-256 of a fixed label (deterministic Partial; not CSPRNG Claimed).
    let cookie = Sha256::digest(b"arks-ssh-partial-kexinit-cookie");
    p.extend_from_slice(&cookie[..16]);
    p.extend(ssh_name_list(&[
        "curve25519-sha256",
        "curve25519-sha256@libssh.org",
    ]));
    // Honest Missing hostkey name — algo select fail-closed vs stock OpenSSH is expected.
    p.extend(ssh_name_list(&["arks-partial-missing-hostkey-sign"]));
    for _ in 0..6 {
        p.extend(ssh_name_list(&["arks-partial-missing"]));
    }
    p.extend(ssh_name_list(&[]));
    p.extend(ssh_name_list(&[]));
    p.push(0); // first_kex_packet_follows = false
    p.extend_from_slice(&0u32.to_be_bytes());
    p
}

/// Write one unencrypted SSH binary packet (no MAC — pre-NEWKEYS).
fn write_ssh_packet(stream: &mut TcpStream, payload: &[u8]) -> std::io::Result<()> {
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

fn read_ssh_packet(stream: &mut TcpStream, max_payload: usize) -> std::io::Result<Vec<u8>> {
    let mut lenbuf = [0u8; 4];
    stream.read_exact(&mut lenbuf)?;
    let packet_length = u32::from_be_bytes(lenbuf) as usize;
    if packet_length < 5 || packet_length > max_payload + 256 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("ssh packet_length out of range ({packet_length})"),
        ));
    }
    let mut rest = vec![0u8; packet_length];
    stream.read_exact(&mut rest)?;
    let padding_len = rest[0] as usize;
    if padding_len + 1 > rest.len() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "ssh padding_length invalid",
        ));
    }
    let payload_end = rest.len() - padding_len;
    Ok(rest[1..payload_end].to_vec())
}

/// Local curve25519 ECDH self-check (Partial crypto evidence; not peer handshake Claimed).
fn curve25519_ecdh_selfcheck() -> Result<String, String> {
    let a_secret = StaticSecret::random();
    let a_public = PublicKey::from(&a_secret);
    let b_secret = StaticSecret::random();
    let b_public = PublicKey::from(&b_secret);
    let shared_a = a_secret.diffie_hellman(&b_public);
    let shared_b = b_secret.diffie_hellman(&a_public);
    if shared_a.as_bytes() != shared_b.as_bytes() {
        return Err("shared secret mismatch".into());
    }
    let mut h = Sha256::new();
    h.update(b"arks-ssh-partial-ecdh");
    h.update(shared_a.as_bytes());
    let digest = h.finalize();
    Ok(format!(
        "ecdh_selfcheck=ok shared_sha256={:02x}{:02x}…",
        digest[0], digest[1]
    ))
}

/// Attempt peer ECDH INIT after mutual curve25519 select (Partial; no host-key reply Claimed).
fn try_peer_ecdh(stream: &mut TcpStream, peer_kex: &[String]) -> String {
    let want = peer_kex.iter().any(|n| {
        n == "curve25519-sha256" || n == "curve25519-sha256@libssh.org"
    });
    if !want {
        return "ecdh_peer=skipped (no common curve25519-sha256)".into();
    }
    match read_ssh_packet(stream, 4096) {
        Ok(payload) if !payload.is_empty() && payload[0] == 30 => {
            // SSH_MSG_KEX_ECDH_INIT: byte + string(q_c)
            if payload.len() < 5 {
                return "ecdh_peer=init_short".into();
            }
            let qlen = u32::from_be_bytes(payload[1..5].try_into().unwrap_or([0; 4])) as usize;
            if qlen != 32 || payload.len() < 5 + qlen {
                return format!("ecdh_peer=bad_q_c_len ({qlen})");
            }
            let mut q_c = [0u8; 32];
            q_c.copy_from_slice(&payload[5..5 + 32]);
            let peer_pk = PublicKey::from(q_c);
            let secret = StaticSecret::random();
            let q_s = PublicKey::from(&secret);
            let shared = secret.diffie_hellman(&peer_pk);
            let mut h = Sha256::new();
            h.update(b"arks-ssh-partial-peer-ecdh");
            h.update(shared.as_bytes());
            h.update(q_s.as_bytes());
            let digest = h.finalize();
            // Do **not** emit SSH_MSG_KEX_ECDH_REPLY — host-key sign Missing (fail-closed next).
            format!(
                "ecdh_peer=shared_ok q_s_ready=1 shared_sha256={:02x}{:02x}… (KEX_ECDH_REPLY withheld — host_key_sign Missing)",
                digest[0], digest[1]
            )
        }
        Ok(payload) => {
            let mt = payload.first().copied().unwrap_or(0);
            format!("ecdh_peer=unexpected_msg={mt}")
        }
        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock || e.kind() == std::io::ErrorKind::TimedOut => {
            "ecdh_peer=timeout (no ECDH_INIT)".into()
        }
        Err(e) => format!("ecdh_peer=err ({e})"),
    }
}

fn parse_peer_kexinit(payload: &[u8]) -> Option<(Vec<String>, Vec<String>)> {
    if payload.first().copied()? != 20 {
        return None;
    }
    let mut off = 1 + 16; // type + cookie
    let kex = read_ssh_name_list(payload, &mut off)?;
    let hostkey = read_ssh_name_list(payload, &mut off)?;
    Some((kex, hostkey))
}

fn run_kex_stages(stream: &mut TcpStream) -> (SshStage, String) {
    let mut notes = Vec::new();

    // Stage: KexInit
    let payload = build_kexinit_payload();
    if let Err(e) = write_ssh_packet(stream, &payload) {
        return (
            SshStage::KexInit,
            format!("kexinit=write_err ({e})"),
        );
    }
    let peer_payload = match read_ssh_packet(stream, 16_384) {
        Ok(p) => p,
        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock || e.kind() == std::io::ErrorKind::TimedOut => {
            notes.push("kexinit=sent peer=(timeout)".into());
            match curve25519_ecdh_selfcheck() {
                Ok(s) => notes.push(s),
                Err(e) => notes.push(format!("ecdh_selfcheck=fail ({e})")),
            }
            notes.push("stop=host_key_sign Missing (no peer KEXINIT)".into());
            return (SshStage::HostKeySign, notes.join("; "));
        }
        Err(e) => {
            return (SshStage::KexInit, format!("kexinit=sent peer_err ({e})"));
        }
    };
    notes.push(format!("kexinit=sent peer_payload_len={}", peer_payload.len()));

    // Stage: AlgoSelect
    let (peer_kex, peer_hostkey) = match parse_peer_kexinit(&peer_payload) {
        Some(v) => v,
        None => {
            notes.push("algo_select=peer_kexinit_parse_fail".into());
            match curve25519_ecdh_selfcheck() {
                Ok(s) => notes.push(s),
                Err(e) => notes.push(format!("ecdh_selfcheck=fail ({e})")),
            }
            notes.push("stop=host_key_sign Missing".into());
            return (SshStage::HostKeySign, notes.join("; "));
        }
    };
    let kex_ok = peer_kex.iter().any(|n| {
        n == "curve25519-sha256" || n == "curve25519-sha256@libssh.org"
    });
    let hostkey_common = peer_hostkey
        .iter()
        .any(|n| n == "arks-partial-missing-hostkey-sign");
    notes.push(format!(
        "algo_select=kex_curve25519={} hostkey_common_missing_name={} peer_kex_n={} peer_hostkey_n={}",
        kex_ok,
        hostkey_common,
        peer_kex.len(),
        peer_hostkey.len()
    ));
    if !kex_ok {
        notes.push("stop=algo_select (no common curve25519-sha256)".into());
        return (SshStage::AlgoSelect, notes.join("; "));
    }

    // Stage: Ecdh (local self-check always; peer INIT best-effort)
    match curve25519_ecdh_selfcheck() {
        Ok(s) => notes.push(s),
        Err(e) => {
            notes.push(format!("ecdh_selfcheck=fail ({e})"));
            return (SshStage::Ecdh, notes.join("; "));
        }
    }
    notes.push(try_peer_ecdh(stream, &peer_kex));

    // Stage: HostKeySign — Missing (OpenSSH key parse/sign not wired)
    notes.push("stop=host_key_sign Missing (no KEX_ECDH_REPLY / NEWKEYS); auth_publickey+channel_session scaffolding only".into());
    (SshStage::HostKeySign, notes.join("; "))
}

/// Attempt to run an in-process SSH listener.
///
/// Advances structured KEX stages as far as honest Partial allows, then **fail-closed**
/// before Claimed handshake (host-key sign / auth / channel / git subsystem).
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

    listener.set_nonblocking(true)?;
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    let (reached, wire_note) = loop {
        match listener.accept() {
            Ok((mut stream, peer)) => {
                let _ = stream.set_nonblocking(false);
                let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
                let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
                let ours = b"SSH-2.0-arks_partial_curve25519_Missing_sign\r\n";
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
                let id_part = if peer_id.is_empty() {
                    format!("id_exchange=partial peer={peer} peer_id=(empty/timeout)")
                } else {
                    format!("id_exchange=partial peer={peer} peer_id={peer_id}")
                };
                let (stage, kex_part) = if peer_id.is_empty() {
                    // Still prove local ECDH path without a peer.
                    match curve25519_ecdh_selfcheck() {
                        Ok(s) => (
                            SshStage::HostKeySign,
                            format!("kex=skipped_no_peer_id; {s}; stop=host_key_sign Missing"),
                        ),
                        Err(e) => (
                            SshStage::Ecdh,
                            format!("kex=skipped_no_peer_id; ecdh_selfcheck=fail ({e})"),
                        ),
                    }
                } else {
                    run_kex_stages(&mut stream)
                };
                let _ = stream.shutdown(Shutdown::Both);
                break (stage, format!("{id_part}; {kex_part}"));
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                if std::time::Instant::now() >= deadline {
                    // No peer: still record local ECDH Partial evidence.
                    let ecdh = match curve25519_ecdh_selfcheck() {
                        Ok(s) => s,
                        Err(e) => format!("ecdh_selfcheck=fail ({e})"),
                    };
                    break (
                        SshStage::HostKeySign,
                        format!(
                            "id_exchange=skipped (no peer within 2s); {ecdh}; stop=host_key_sign Missing"
                        ),
                    );
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(e) => {
                break (
                    SshStage::IdExchange,
                    format!("id_exchange=accept_err ({e})"),
                );
            }
        }
    };
    drop(listener);

    Err(ServerError::NotImplemented(format!(
        "in-process SSH listener for `arks serve` is Partial/Missing — fail-closed \
         ([[sot:root/arks-self-host-server]]); reached_stage={stage} (Claimed boundary: \
         host-key cryptographic load/sign, NEWKEYS, publickey auth verify, session/channel, \
         git subsystem). wire={{listen={listen}; {bind_note}; {wire_note}; \
         host_key={hk} ({host_st}; format={host_fmt}); authorized_keys={ak} ({auth_st}; {auth_probe}); \
         path={path} ({repo_st})}}. Partial: id exchange + KEXINIT + curve25519-sha256 ECDH \
         self-check/peer shared-secret when offered; later stages auth_publickey + \
         channel_session remain Missing scaffolding only. \
         use `--no-ssh`, or ops SSHD + `arks upload-pack`/`receive-pack` for SSH remotes today; \
         set ARKS_SERVE_SSH_HOST_KEY / ARKS_SERVE_SSH_AUTHORIZED_KEYS or \
         `arks serve config set ssh_host_key|…` when preparing keys",
        stage = reached.as_str(),
        listen = opts.listen,
        hk = opts.host_key.display(),
        ak = opts.authorized_keys.display(),
        path = opts.path.display(),
    )))
}
