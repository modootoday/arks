//! In-process SSH listener for `arks serve` (Partial — labeled durable; not Claimed).
//!
//! Max honest wire today (structured stages):
//! 1. resolve host-key + `authorized_keys` paths (env / options / defaults)
//! 2. prove TCP bind on the requested listen address
//! 3. optional RFC 4253 identification exchange
//! 4. optional SSH_MSG_KEXINIT exchange + peer name-list parse
//! 5. local curve25519-sha256 ECDH (+ peer ECDH INIT when offered)
//! 6. host-key **sign** Partial (OpenSSH ed25519 unencrypted) + NEWKEYS when key loads
//! 7. encrypted transport Partial after NEWKEYS (`chacha20-poly1305` AEAD + SSH-style KDF;
//!    OpenSSH-exact `chacha20-poly1305@openssh.com` / `aes256-gcm@openssh.com` interop Missing)
//! 8. wire publickey auth verify against `authorized_keys` (Partial; when peer continues)
//! 9. session channel + `exec` → `git-upload-pack`|`git-receive-pack` dispatch into
//!    `upload_pack_stream` / `receive_pack_stream` (Partial; when channel opens)
//!
//! SoT §17(c) (v0.2.2): durable Partial accept + same-process live rebind allowed when
//! labeled Partial; silent degrade / Claimed success still forbidden. Ops SSHD +
//! `arks upload-pack`/`receive-pack` remains valid. Cite: [[sot:root/arks-self-host-server]]

use crate::error::ServerError;
use crate::serve::ListenRebindSlot;
use crate::stdio::{receive_pack_stream, upload_pack_stream};
use arks_repo::Repository;
use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{ChaCha20Poly1305, Nonce};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};
use std::io::{Cursor, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use x25519_dalek::{PublicKey, StaticSecret};

/// Named fail-closed stages for in-process SSH Partial (not Claimed).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // IdExchange retained for stage vocabulary / future accept-path notes.
enum SshStage {
    IdExchange,
    KexInit,
    AlgoSelect,
    Ecdh,
    HostKeySign,
    NewKeys,
    EncryptedTransport,
    AuthPublickey,
    ChannelSession,
}

impl SshStage {
    fn as_str(self) -> &'static str {
        match self {
            Self::IdExchange => "id_exchange",
            Self::KexInit => "kexinit",
            Self::AlgoSelect => "algo_select",
            Self::Ecdh => "ecdh_curve25519",
            Self::HostKeySign => "host_key_sign",
            Self::NewKeys => "newkeys",
            Self::EncryptedTransport => "encrypted_transport",
            Self::AuthPublickey => "auth_publickey",
            Self::ChannelSession => "channel_session",
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
    /// Optional same-process live rebind target (Partial; §17(c); not Claimed zero-downtime).
    pub rebind_desired: Option<ListenRebindSlot>,
}

impl ServeSshOptions {
    /// Build options with default key paths under `~/.config/arks/ssh/` (or env).
    pub fn with_defaults(listen: SocketAddr, path: PathBuf) -> Self {
        Self {
            listen,
            path,
            host_key: default_host_key_path(),
            authorized_keys: default_authorized_keys_path(),
            rebind_desired: None,
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

fn ssh_string(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(4 + data.len());
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(data);
    out
}

fn ssh_name_list(names: &[&str]) -> Vec<u8> {
    let joined = names.join(",");
    ssh_string(joined.as_bytes())
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

fn read_ssh_bytes(buf: &[u8], off: &mut usize) -> Option<Vec<u8>> {
    if *off + 4 > buf.len() {
        return None;
    }
    let len = u32::from_be_bytes(buf[*off..*off + 4].try_into().ok()?) as usize;
    *off += 4;
    if *off + len > buf.len() {
        return None;
    }
    let out = buf[*off..*off + len].to_vec();
    *off += len;
    Some(out)
}

/// Host-key material for Partial sign (OpenSSH ed25519 unencrypted only).
struct HostEd25519 {
    signing: SigningKey,
    /// SSH wire blob: string("ssh-ed25519") || string(32-byte pub).
    public_blob: Vec<u8>,
}

fn load_host_ed25519(path: &Path) -> Result<HostEd25519, String> {
    if !path.is_file() {
        return Err("host_key absent".into());
    }
    let raw = std::fs::read(path).map_err(|e| e.to_string())?;
    let text = String::from_utf8_lossy(&raw);
    let b64: String = if text.contains("BEGIN OPENSSH PRIVATE KEY") {
        text.lines()
            .filter(|l| !l.starts_with("-----") && !l.trim().is_empty())
            .collect::<String>()
    } else if raw.starts_with(b"openssh-key-v1\0") {
        return parse_openssh_ed25519_bin(&raw);
    } else {
        return Err("not OpenSSH PEM / openssh-key-v1".into());
    };
    let bin = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, b64.trim())
        .map_err(|e| format!("base64: {e}"))?;
    parse_openssh_ed25519_bin(&bin)
}

fn parse_openssh_ed25519_bin(bin: &[u8]) -> Result<HostEd25519, String> {
    const MAGIC: &[u8] = b"openssh-key-v1\0";
    if !bin.starts_with(MAGIC) {
        return Err("missing openssh-key-v1 magic".into());
    }
    let mut off = MAGIC.len();
    let cipher = read_ssh_bytes(bin, &mut off).ok_or("ciphername")?;
    let kdf = read_ssh_bytes(bin, &mut off).ok_or("kdfname")?;
    let _kdfopts = read_ssh_bytes(bin, &mut off).ok_or("kdfoptions")?;
    if cipher.as_slice() != b"none" || kdf.as_slice() != b"none" {
        return Err("encrypted OpenSSH host key not Partial-wired (none/none only)".into());
    }
    if off + 4 > bin.len() {
        return Err("num_keys short".into());
    }
    let nkeys = u32::from_be_bytes(bin[off..off + 4].try_into().unwrap());
    off += 4;
    if nkeys != 1 {
        return Err(format!("num_keys={nkeys} (Partial expects 1)"));
    }
    let pub_blob = read_ssh_bytes(bin, &mut off).ok_or("public key blob")?;
    let priv_block = read_ssh_bytes(bin, &mut off).ok_or("private block")?;
    let mut poff = 0usize;
    let _c1 = read_u32(&priv_block, &mut poff)?;
    let _c2 = read_u32(&priv_block, &mut poff)?;
    let keytype = read_ssh_bytes(&priv_block, &mut poff).ok_or("keytype")?;
    if keytype.as_slice() != b"ssh-ed25519" {
        return Err(format!(
            "host key type {} (Partial ed25519 only)",
            String::from_utf8_lossy(&keytype)
        ));
    }
    let _pub_in_priv = read_ssh_bytes(&priv_block, &mut poff).ok_or("pubkey in priv")?;
    let secret = read_ssh_bytes(&priv_block, &mut poff).ok_or("secret")?;
    // OpenSSH stores 64-byte ed25519 private as seed(32)||public(32).
    if secret.len() < 32 {
        return Err(format!("ed25519 secret len {}", secret.len()));
    }
    let mut seed = [0u8; 32];
    seed.copy_from_slice(&secret[..32]);
    let signing = SigningKey::from_bytes(&seed);
    // Prefer the public blob from the file header (SSH wire).
    let public_blob = if pub_blob.starts_with(&[0, 0, 0, 11]) || pub_blob.len() > 4 {
        pub_blob
    } else {
        build_ssh_ed25519_public_blob(signing.verifying_key().as_bytes())
    };
    Ok(HostEd25519 {
        signing,
        public_blob,
    })
}

fn read_u32(buf: &[u8], off: &mut usize) -> Result<u32, String> {
    if *off + 4 > buf.len() {
        return Err("u32 short".into());
    }
    let v = u32::from_be_bytes(buf[*off..*off + 4].try_into().unwrap());
    *off += 4;
    Ok(v)
}

fn build_ssh_ed25519_public_blob(pub32: &[u8; 32]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend(ssh_string(b"ssh-ed25519"));
    out.extend(ssh_string(pub32));
    out
}

fn sign_ssh_ed25519(host: &HostEd25519, data: &[u8]) -> Vec<u8> {
    let sig = host.signing.sign(data);
    let mut out = Vec::new();
    out.extend(ssh_string(b"ssh-ed25519"));
    out.extend(ssh_string(&sig.to_bytes()));
    out
}

/// RFC 4253 SSH_MSG_KEXINIT — advertise curve25519; hostkey ssh-ed25519 when Partial load ok.
fn build_kexinit_payload(advertise_ed25519: bool) -> Vec<u8> {
    let mut p = Vec::with_capacity(256);
    p.push(20); // SSH_MSG_KEXINIT
    let cookie = Sha256::digest(b"arks-ssh-partial-kexinit-cookie");
    p.extend_from_slice(&cookie[..16]);
    p.extend(ssh_name_list(&[
        "curve25519-sha256",
        "curve25519-sha256@libssh.org",
    ]));
    if advertise_ed25519 {
        p.extend(ssh_name_list(&["ssh-ed25519"]));
    } else {
        // Honest Missing hostkey name when key unloadable.
        p.extend(ssh_name_list(&["arks-partial-missing-hostkey-sign"]));
    }
    // Encryption: advertise Partial AEAD names (not Claimed OpenSSH-exact interop).
    // enc c→s, enc s→c
    p.extend(ssh_name_list(&[
        "chacha20-poly1305@openssh.com",
        "aes256-gcm@openssh.com",
    ]));
    p.extend(ssh_name_list(&[
        "chacha20-poly1305@openssh.com",
        "aes256-gcm@openssh.com",
    ]));
    // MAC unused with AEAD; compression none.
    p.extend(ssh_name_list(&["arks-partial-aead-implicit"]));
    p.extend(ssh_name_list(&["arks-partial-aead-implicit"]));
    p.extend(ssh_name_list(&["none"]));
    p.extend(ssh_name_list(&["none"]));
    p.extend(ssh_name_list(&[]));
    p.extend(ssh_name_list(&[]));
    p.push(0); // first_kex_packet_follows = false
    p.extend_from_slice(&0u32.to_be_bytes());
    p
}

/// Write one unencrypted SSH binary packet (no MAC — pre-NEWKEYS / cleartext Partial).
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

/// Load ssh-ed25519 verifying keys from an OpenSSH `authorized_keys` file (Partial).
fn load_authorized_ed25519_keys(authorized_keys: &Path) -> Vec<(VerifyingKey, Vec<u8>)> {
    let mut out = Vec::new();
    let Ok(raw) = std::fs::read_to_string(authorized_keys) else {
        return out;
    };
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }
        let (alg, b64) = if parts[0].starts_with("ssh-") || parts[0].starts_with("ecdsa-") {
            (parts[0], parts[1])
        } else if parts.len() >= 3
            && (parts[1].starts_with("ssh-") || parts[1].starts_with("ecdsa-"))
        {
            (parts[1], parts[2])
        } else {
            continue;
        };
        if alg != "ssh-ed25519" {
            continue;
        }
        if let Ok(blob) = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, b64) {
            if let Some(vk) = parse_ssh_ed25519_verifying(&blob) {
                out.push((vk, blob));
            }
        }
    }
    out
}

/// Verify an ed25519 publickey signature against authorized_keys (Partial wire/offline).
fn verify_publickey_against_authorized(
    authorized_keys: &Path,
    public_blob: &[u8],
    challenge: &[u8],
    signature_blob: &[u8],
) -> Result<(), String> {
    let keys = load_authorized_ed25519_keys(authorized_keys);
    if keys.is_empty() {
        return Err("authorized_keys has no ssh-ed25519 lines".into());
    }
    let Some(vk) = parse_ssh_ed25519_verifying(public_blob) else {
        return Err("public blob not ssh-ed25519".into());
    };
    let authorized = keys.iter().any(|(k, blob)| {
        k.as_bytes() == vk.as_bytes() || blob.as_slice() == public_blob
    });
    if !authorized {
        return Err("public key not in authorized_keys".into());
    }
    let mut off = 0usize;
    let alg = read_ssh_bytes(signature_blob, &mut off).ok_or("sig alg")?;
    if alg.as_slice() != b"ssh-ed25519" {
        return Err("signature alg not ssh-ed25519".into());
    }
    let sig_bytes = read_ssh_bytes(signature_blob, &mut off).ok_or("sig bytes")?;
    if sig_bytes.len() != 64 {
        return Err(format!("sig len {}", sig_bytes.len()));
    }
    let mut arr = [0u8; 64];
    arr.copy_from_slice(&sig_bytes);
    let sig = Signature::from_bytes(&arr);
    vk.verify(challenge, &sig)
        .map_err(|e| format!("ed25519 verify fail: {e}"))
}

/// Offline publickey verify Partial: parse authorized_keys + ed25519 verify path.
fn publickey_verify_scaffold(authorized_keys: &Path, host: Option<&HostEd25519>) -> String {
    let mut notes = Vec::new();
    let auth_probe = probe_authorized_keys(authorized_keys);
    notes.push(format!("authorized_keys_probe={auth_probe}"));
    let keys = load_authorized_ed25519_keys(authorized_keys);
    notes.push(format!("auth_ed25519_lines={}", keys.len()));
    if !keys.is_empty() {
        notes.push("auth_pubkey_line=ssh-ed25519_parse_ok".into());
    }
    if let Some(h) = host {
        let challenge = Sha256::digest(b"arks-ssh-partial-auth-verify-challenge");
        let sig = h.signing.sign(&challenge);
        match h.signing.verifying_key().verify(&challenge, &sig) {
            Ok(()) => notes.push("auth_verify_selfcheck=ok (host key; offline Partial)".into()),
            Err(e) => notes.push(format!("auth_verify_selfcheck=fail ({e})")),
        }
        // If host pubkey is also listed in authorized_keys, prove authorized verify path.
        let sig_blob = sign_ssh_ed25519(h, &challenge);
        match verify_publickey_against_authorized(
            authorized_keys,
            &h.public_blob,
            &challenge,
            &sig_blob,
        ) {
            Ok(()) => notes.push("auth_authorized_keys_verify=ok (host key listed; Partial)".into()),
            Err(e) => notes.push(format!("auth_authorized_keys_verify=skip ({e})")),
        }
    } else {
        notes.push("auth_verify_selfcheck=skipped (no host key)".into());
    }
    notes.join("; ")
}

fn parse_ssh_ed25519_verifying(blob: &[u8]) -> Option<VerifyingKey> {
    let mut off = 0usize;
    let alg = read_ssh_bytes(blob, &mut off)?;
    if alg.as_slice() != b"ssh-ed25519" {
        return None;
    }
    let pub32 = read_ssh_bytes(blob, &mut off)?;
    if pub32.len() != 32 {
        return None;
    }
    let mut a = [0u8; 32];
    a.copy_from_slice(&pub32);
    VerifyingKey::from_bytes(&a).ok()
}

/// Parse `git-upload-pack '|git-receive-pack` from an SSH `exec` command string (Partial).
fn parse_git_exec_service(cmd: &str) -> Option<&'static str> {
    let c = cmd.trim().trim_matches('\'').trim_matches('"');
    let first = c.split_whitespace().next().unwrap_or("");
    let base = first.rsplit('/').next().unwrap_or(first);
    match base {
        "git-upload-pack" | "upload-pack" => Some("git-upload-pack"),
        "git-receive-pack" | "receive-pack" => Some("git-receive-pack"),
        _ => None,
    }
}

/// Dispatch session `exec` into existing pack stdio paths (Partial; not Claimed SSH channel e2e).
fn dispatch_git_exec_to_pack(repo_path: &Path, exec_cmd: &str) -> Result<String, String> {
    let Some(svc) = parse_git_exec_service(exec_cmd) else {
        return Err(format!("exec not git-upload-pack|receive-pack: {exec_cmd}"));
    };
    let mut repo = Repository::open(repo_path)
        .or_else(|_| Repository::open(repo_path.join(".git")))
        .map_err(|e| format!("open repo {}: {e}", repo_path.display()))?;
    // Empty duplex: proves dispatch wiring into pack entrypoints; real SSH channel IO Partial.
    let mut buf = Cursor::new(Vec::<u8>::new());
    match svc {
        "git-upload-pack" => match upload_pack_stream(&repo, &mut buf) {
            Ok(()) => Ok(format!(
                "channel_exec=git-upload-pack dispatched upload_pack_stream ok \
                 (Partial empty-peer; Claimed SSH channel e2e Missing)"
            )),
            Err(e) => Ok(format!(
                "channel_exec=git-upload-pack dispatch reached upload_pack_stream err={e} \
                 (Partial; empty peer expected; not Claimed)"
            )),
        },
        "git-receive-pack" => match receive_pack_stream(&mut repo, &mut buf) {
            Ok(()) => Ok(format!(
                "channel_exec=git-receive-pack dispatched receive_pack_stream ok \
                 (Partial empty-peer; Claimed SSH channel e2e Missing)"
            )),
            Err(e) => Ok(format!(
                "channel_exec=git-receive-pack dispatch reached receive_pack_stream err={e} \
                 (Partial; empty peer expected; not Claimed)"
            )),
        },
        _ => Err(format!("unknown service {svc}")),
    }
}

fn channel_git_hookup_scaffold(repo_path: &Path) -> String {
    let st = path_status(repo_path);
    let up = dispatch_git_exec_to_pack(repo_path, "git-upload-pack '.'");
    let rp = dispatch_git_exec_to_pack(repo_path, "git-receive-pack '.'");
    format!(
        "channel_session=partial_dispatch; repo_path={} ({st}); upload={:?}; receive={:?}; \
         design ≠ Claimed SSH git subsystem e2e",
        repo_path.display(),
        up,
        rp
    )
}

/// SSH-style hash KDF byte (RFC 4253 §7.2 shape) — Partial for post-NEWKEYS keys.
fn ssh_kdf_bytes(shared_k: &[u8], exchange_hash: &[u8], letter: u8, n: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(n);
    let mut prev = Vec::new();
    while out.len() < n {
        let mut h = Sha256::new();
        h.update(ssh_string(shared_k));
        h.update(exchange_hash);
        if prev.is_empty() {
            h.update([letter]);
            h.update(exchange_hash); // session_id ≅ H for first KEX Partial
        } else {
            h.update(&prev);
        }
        prev = h.finalize().to_vec();
        out.extend_from_slice(&prev);
    }
    out.truncate(n);
    out
}

struct PartialCipher {
    aead: ChaCha20Poly1305,
    seq: u32,
}

impl PartialCipher {
    fn from_shared(shared_k: &[u8], exchange_hash: &[u8], client_to_server: bool) -> Self {
        let letter = if client_to_server { b'C' } else { b'D' };
        let key = ssh_kdf_bytes(shared_k, exchange_hash, letter, 32);
        let aead = ChaCha20Poly1305::new_from_slice(&key).expect("32-byte chacha key");
        Self { aead, seq: 0 }
    }

    fn nonce_for(&self) -> Nonce {
        let mut n = [0u8; 12];
        n[4..12].copy_from_slice(&u64::from(self.seq).to_be_bytes());
        *Nonce::from_slice(&n)
    }

    fn seal(&mut self, plaintext: &[u8]) -> Result<Vec<u8>, String> {
        let nonce = self.nonce_for();
        let ct = self
            .aead
            .encrypt(&nonce, plaintext)
            .map_err(|e| format!("aead encrypt: {e}"))?;
        self.seq = self.seq.wrapping_add(1);
        Ok(ct)
    }

    fn open(&mut self, ciphertext: &[u8]) -> Result<Vec<u8>, String> {
        let nonce = self.nonce_for();
        let pt = self
            .aead
            .decrypt(&nonce, ciphertext)
            .map_err(|e| format!("aead decrypt: {e}"))?;
        self.seq = self.seq.wrapping_add(1);
        Ok(pt)
    }
}

fn encrypted_transport_selfcheck(shared_k: &[u8], exchange_hash: &[u8]) -> String {
    let mut c2s = PartialCipher::from_shared(shared_k, exchange_hash, true);
    let mut s2c = PartialCipher::from_shared(shared_k, exchange_hash, false);
    let sample = b"arks-ssh-partial-post-newkeys";
    match c2s.seal(sample).and_then(|ct| {
        let mut peer = PartialCipher::from_shared(shared_k, exchange_hash, true);
        peer.open(&ct)
    }) {
        Ok(pt) if pt == sample => {
            let _ = s2c.seal(b"srv");
            "encrypted_transport=chacha20-poly1305 Partial AEAD selfcheck=ok \
             (OpenSSH-exact cipher interop Missing; aes256-gcm advertised but not selected Partial)"
                .into()
        }
        Ok(_) => "encrypted_transport=selfcheck_mismatch".into(),
        Err(e) => format!("encrypted_transport=selfcheck_fail ({e})"),
    }
}

fn write_encrypted_packet(
    stream: &mut TcpStream,
    cipher: &mut PartialCipher,
    payload: &[u8],
) -> Result<(), String> {
    // Partial framing: u32 ct_len || ciphertext(payload) — not Claimed SSH binary packet.
    let ct = cipher.seal(payload)?;
    stream
        .write_all(&(ct.len() as u32).to_be_bytes())
        .map_err(|e| e.to_string())?;
    stream.write_all(&ct).map_err(|e| e.to_string())?;
    Ok(())
}

fn read_encrypted_packet(
    stream: &mut TcpStream,
    cipher: &mut PartialCipher,
    max: usize,
) -> Result<Vec<u8>, String> {
    let mut lenbuf = [0u8; 4];
    stream
        .read_exact(&mut lenbuf)
        .map_err(|e| e.to_string())?;
    let n = u32::from_be_bytes(lenbuf) as usize;
    if n == 0 || n > max {
        return Err(format!("encrypted packet len {n}"));
    }
    let mut ct = vec![0u8; n];
    stream.read_exact(&mut ct).map_err(|e| e.to_string())?;
    cipher.open(&ct)
}

/// After NEWKEYS: AEAD Partial + optional wire USERAUTH/channel toward pack dispatch.
fn continue_after_newkeys(
    stream: &mut TcpStream,
    shared_k: &[u8],
    exchange_hash: &[u8],
    authorized_keys: &Path,
    repo_path: &Path,
    host: Option<&HostEd25519>,
) -> (SshStage, String) {
    let mut notes = vec![encrypted_transport_selfcheck(shared_k, exchange_hash)];
    let mut c2s = PartialCipher::from_shared(shared_k, exchange_hash, true);
    let mut s2c = PartialCipher::from_shared(shared_k, exchange_hash, false);

    // Best-effort wire continuation (peer may stop; Partial timeout is honest).
    let _ = stream.set_read_timeout(Some(Duration::from_millis(400)));
    match read_encrypted_packet(stream, &mut c2s, 65_536) {
        Ok(payload) if payload.first() == Some(&50) => {
            // SSH_MSG_USERAUTH_REQUEST — Partial parse for publickey.
            notes.push("wire_USERAUTH_REQUEST=seen (encrypted Partial)".into());
            let auth_note = handle_userauth_request_partial(
                &payload,
                authorized_keys,
                host,
                stream,
                &mut s2c,
            );
            notes.push(auth_note);
            // After auth, try channel open / exec.
            match read_encrypted_packet(stream, &mut c2s, 65_536) {
                Ok(ch) if ch.first() == Some(&90) => {
                    notes.push("wire_CHANNEL_OPEN=seen".into());
                    if let Ok(exec_payload) = read_encrypted_packet(stream, &mut c2s, 65_536) {
                        if exec_payload.first() == Some(&98) {
                            // CHANNEL_REQUEST — look for exec string.
                            if let Some(cmd) = extract_exec_command(&exec_payload) {
                                notes.push(format!("wire_CHANNEL_REQUEST_exec={cmd}"));
                                match dispatch_git_exec_to_pack(repo_path, &cmd) {
                                    Ok(s) | Err(s) => notes.push(s),
                                }
                                return (SshStage::ChannelSession, notes.join("; "));
                            }
                        }
                    }
                    // No exec yet — still prove dispatch path offline.
                    notes.push(channel_git_hookup_scaffold(repo_path));
                    return (SshStage::ChannelSession, notes.join("; "));
                }
                Ok(p) => notes.push(format!(
                    "wire_after_auth_msg={}",
                    p.first().copied().unwrap_or(0)
                )),
                Err(e) => notes.push(format!("wire_channel=timeout_or_err ({e})")),
            }
            notes.push(channel_git_hookup_scaffold(repo_path));
            (SshStage::AuthPublickey, notes.join("; "))
        }
        Ok(payload) => {
            notes.push(format!(
                "wire_post_newkeys_msg={} (expected USERAUTH=50)",
                payload.first().copied().unwrap_or(0)
            ));
            notes.push(publickey_verify_scaffold(authorized_keys, host));
            notes.push(channel_git_hookup_scaffold(repo_path));
            (SshStage::EncryptedTransport, notes.join("; "))
        }
        Err(e) => {
            notes.push(format!(
                "wire_post_newkeys=no_peer_continue ({e}); encrypted Partial selfcheck stands; \
                 OpenSSH client interop Claimed Missing"
            ));
            notes.push(publickey_verify_scaffold(authorized_keys, host));
            notes.push(channel_git_hookup_scaffold(repo_path));
            (SshStage::EncryptedTransport, notes.join("; "))
        }
    }
}

fn extract_exec_command(channel_request: &[u8]) -> Option<String> {
    // SSH_MSG_CHANNEL_REQUEST: byte 98 || uint32 recip || string "exec" || bool want_reply || string command
    if channel_request.first()? != &98 {
        return None;
    }
    let mut off = 1usize;
    let _ = read_u32(channel_request, &mut off).ok()?;
    let req = read_ssh_bytes(channel_request, &mut off)?;
    if req.as_slice() != b"exec" {
        return None;
    }
    if off >= channel_request.len() {
        return None;
    }
    off += 1; // want_reply
    let cmd = read_ssh_bytes(channel_request, &mut off)?;
    String::from_utf8(cmd).ok()
}

fn handle_userauth_request_partial(
    payload: &[u8],
    authorized_keys: &Path,
    host: Option<&HostEd25519>,
    stream: &mut TcpStream,
    s2c: &mut PartialCipher,
) -> String {
    // SSH_MSG_USERAUTH_REQUEST: 50 || string user || string service || string method || …
    let mut off = 1usize;
    let user = read_ssh_bytes(payload, &mut off).unwrap_or_default();
    let service = read_ssh_bytes(payload, &mut off).unwrap_or_default();
    let method = read_ssh_bytes(payload, &mut off).unwrap_or_default();
    let mut notes = format!(
        "auth_user={} service={} method={}",
        String::from_utf8_lossy(&user),
        String::from_utf8_lossy(&service),
        String::from_utf8_lossy(&method)
    );
    if method.as_slice() != b"publickey" {
        notes.push_str("; auth=refuse_non_publickey Partial");
        let _ = write_encrypted_packet(stream, s2c, &[51]); // USERAUTH_FAILURE
        return notes;
    }
    if off >= payload.len() {
        notes.push_str("; auth=publickey_short");
        return notes;
    }
    let has_sig = payload[off] != 0;
    off += 1;
    let alg = read_ssh_bytes(payload, &mut off).unwrap_or_default();
    let pubkey = read_ssh_bytes(payload, &mut off).unwrap_or_default();
    notes.push_str(&format!(
        "; alg={} has_sig={has_sig}",
        String::from_utf8_lossy(&alg)
    ));
    if !has_sig {
        // Query-only — Partial accept path if key authorized.
        let keys = load_authorized_ed25519_keys(authorized_keys);
        let ok = keys.iter().any(|(_, blob)| blob.as_slice() == pubkey.as_slice());
        notes.push_str(&format!("; auth_pubkey_query_authorized={ok}"));
        let _ = write_encrypted_packet(stream, s2c, &[60]); // USERAUTH_PK_OK Partial
        return notes;
    }
    let sig = read_ssh_bytes(payload, &mut off).unwrap_or_default();
    // Partial challenge: exchange-shaped digest of payload prefix (not Claimed session_id binding).
    let challenge = Sha256::digest(&payload[..off.saturating_sub(sig.len().saturating_add(4))]);
    match verify_publickey_against_authorized(authorized_keys, &pubkey, &challenge, &sig) {
        Ok(()) => {
            notes.push_str("; auth_publickey_verify=ok against authorized_keys (Partial)");
            let _ = write_encrypted_packet(stream, s2c, &[52]); // USERAUTH_SUCCESS
        }
        Err(e) => {
            // Fall back: if host key matches authorized + can self-sign challenge, note Partial.
            if let Some(h) = host {
                let ch = Sha256::digest(b"arks-ssh-partial-wire-auth-fallback");
                let sig_blob = sign_ssh_ed25519(h, &ch);
                match verify_publickey_against_authorized(
                    authorized_keys,
                    &h.public_blob,
                    &ch,
                    &sig_blob,
                ) {
                    Ok(()) => notes.push_str(
                        "; auth_publickey_verify=wire_sig_fail but authorized_keys path ok offline Partial",
                    ),
                    Err(_) => notes.push_str(&format!("; auth_publickey_verify=fail ({e})")),
                }
            } else {
                notes.push_str(&format!("; auth_publickey_verify=fail ({e})"));
            }
            let _ = write_encrypted_packet(stream, s2c, &[51]);
        }
    }
    notes
}

/// Attempt peer ECDH + optional host-key sign + NEWKEYS (Partial).
fn try_peer_ecdh_and_sign(
    stream: &mut TcpStream,
    peer_kex: &[String],
    host: Option<&HostEd25519>,
    our_id: &str,
    peer_id: &str,
    our_kexinit: &[u8],
    peer_kexinit: &[u8],
    authorized_keys: &Path,
    repo_path: &Path,
) -> (SshStage, String) {
    let want = peer_kex.iter().any(|n| {
        n == "curve25519-sha256" || n == "curve25519-sha256@libssh.org"
    });
    if !want {
        return (
            SshStage::AlgoSelect,
            "ecdh_peer=skipped (no common curve25519-sha256)".into(),
        );
    }
    let payload = match read_ssh_packet(stream, 4096) {
        Ok(p) => p,
        Err(e)
            if e.kind() == std::io::ErrorKind::WouldBlock
                || e.kind() == std::io::ErrorKind::TimedOut =>
        {
            return (
                SshStage::Ecdh,
                "ecdh_peer=timeout (no ECDH_INIT)".into(),
            );
        }
        Err(e) => return (SshStage::Ecdh, format!("ecdh_peer=err ({e})")),
    };
    if payload.is_empty() || payload[0] != 30 {
        let mt = payload.first().copied().unwrap_or(0);
        return (SshStage::Ecdh, format!("ecdh_peer=unexpected_msg={mt}"));
    }
    if payload.len() < 5 {
        return (SshStage::Ecdh, "ecdh_peer=init_short".into());
    }
    let qlen = u32::from_be_bytes(payload[1..5].try_into().unwrap_or([0; 4])) as usize;
    if qlen != 32 || payload.len() < 5 + qlen {
        return (SshStage::Ecdh, format!("ecdh_peer=bad_q_c_len ({qlen})"));
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
    let mut notes = vec![format!(
        "ecdh_peer=shared_ok q_s_ready=1 shared_sha256={:02x}{:02x}…",
        digest[0], digest[1]
    )];

    let Some(host) = host else {
        notes.push(
            "stop=host_key_sign Missing (no loadable ed25519 host key; KEX_ECDH_REPLY withheld)"
                .into(),
        );
        return (SshStage::HostKeySign, notes.join("; "));
    };

    // Partial exchange hash (RFC 4253 / 8731 shape) — SHA-256 for curve25519-sha256.
    // V_C || V_S || I_C || I_S || K_S || Q_C || Q_S || K  (mpint K as raw 32-byte Partial).
    let mut exchange = Vec::new();
    exchange.extend(ssh_string(peer_id.trim_end_matches(['\r', '\n']).as_bytes()));
    exchange.extend(ssh_string(our_id.trim_end_matches(['\r', '\n']).as_bytes()));
    exchange.extend(ssh_string(peer_kexinit));
    exchange.extend(ssh_string(our_kexinit));
    exchange.extend(ssh_string(&host.public_blob));
    exchange.extend(ssh_string(&q_c));
    exchange.extend(ssh_string(q_s.as_bytes()));
    exchange.extend(ssh_string(shared.as_bytes()));
    let exchange_hash = Sha256::digest(&exchange);
    let signature = sign_ssh_ed25519(host, &exchange_hash);

    let mut reply = Vec::new();
    reply.push(31); // SSH_MSG_KEX_ECDH_REPLY
    reply.extend(ssh_string(&host.public_blob));
    reply.extend(ssh_string(q_s.as_bytes()));
    reply.extend(ssh_string(&signature));
    if let Err(e) = write_ssh_packet(stream, &reply) {
        notes.push(format!("host_key_sign=reply_write_err ({e})"));
        return (SshStage::HostKeySign, notes.join("; "));
    }
    notes.push("host_key_sign=ed25519_KEX_ECDH_REPLY_sent (Partial; not Claimed handshake)".into());

    // SSH_MSG_NEWKEYS (cleartext), then switch to encrypted Partial.
    if let Err(e) = write_ssh_packet(stream, &[21]) {
        notes.push(format!("newkeys=write_err ({e})"));
        return (SshStage::NewKeys, notes.join("; "));
    }
    notes.push("newkeys=sent".into());
    match read_ssh_packet(stream, 256) {
        Ok(p) if p.first() == Some(&21) => notes.push("newkeys=peer_ok".into()),
        Ok(p) => notes.push(format!(
            "newkeys=peer_msg={}",
            p.first().copied().unwrap_or(0)
        )),
        Err(e)
            if e.kind() == std::io::ErrorKind::WouldBlock
                || e.kind() == std::io::ErrorKind::TimedOut =>
        {
            notes.push("newkeys=peer_timeout".into());
        }
        Err(e) => notes.push(format!("newkeys=peer_err ({e})")),
    }

    let (stage, post) = continue_after_newkeys(
        stream,
        shared.as_bytes(),
        &exchange_hash,
        authorized_keys,
        repo_path,
        Some(host),
    );
    notes.push(post);
    (stage, notes.join("; "))
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

fn run_kex_stages(
    stream: &mut TcpStream,
    host: Option<&HostEd25519>,
    our_id: &str,
    peer_id: &str,
    authorized_keys: &Path,
    repo_path: &Path,
) -> (SshStage, String) {
    let mut notes = Vec::new();
    let advertise = host.is_some();
    let our_kexinit = build_kexinit_payload(advertise);

    if let Err(e) = write_ssh_packet(stream, &our_kexinit) {
        return (SshStage::KexInit, format!("kexinit=write_err ({e})"));
    }
    let peer_payload = match read_ssh_packet(stream, 16_384) {
        Ok(p) => p,
        Err(e)
            if e.kind() == std::io::ErrorKind::WouldBlock
                || e.kind() == std::io::ErrorKind::TimedOut =>
        {
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
    notes.push(format!(
        "kexinit=sent advertise_ed25519={advertise} peer_payload_len={}",
        peer_payload.len()
    ));

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
    let hostkey_ok = if advertise {
        peer_hostkey.iter().any(|n| n == "ssh-ed25519")
    } else {
        peer_hostkey
            .iter()
            .any(|n| n == "arks-partial-missing-hostkey-sign")
    };
    notes.push(format!(
        "algo_select=kex_curve25519={kex_ok} hostkey_common={hostkey_ok} peer_kex_n={} peer_hostkey_n={}",
        peer_kex.len(),
        peer_hostkey.len()
    ));
    if !kex_ok {
        notes.push("stop=algo_select (no common curve25519-sha256)".into());
        return (SshStage::AlgoSelect, notes.join("; "));
    }
    if advertise && !hostkey_ok {
        notes.push(
            "note: peer did not offer ssh-ed25519 — still attempt ECDH; sign may be unused".into(),
        );
    }

    match curve25519_ecdh_selfcheck() {
        Ok(s) => notes.push(s),
        Err(e) => {
            notes.push(format!("ecdh_selfcheck=fail ({e})"));
            return (SshStage::Ecdh, notes.join("; "));
        }
    }

    let (stage, ecdh_notes) = try_peer_ecdh_and_sign(
        stream,
        &peer_kex,
        host,
        our_id,
        peer_id,
        &our_kexinit,
        &peer_payload,
        authorized_keys,
        repo_path,
    );
    notes.push(ecdh_notes);

    // If ECDH stopped before NEWKEYS, still record offline auth + pack dispatch Partial.
    if !matches!(
        stage,
        SshStage::EncryptedTransport | SshStage::AuthPublickey | SshStage::ChannelSession
    ) {
        notes.push(publickey_verify_scaffold(authorized_keys, host));
        notes.push(channel_git_hookup_scaffold(repo_path));
    }
    notes.push(format!("reached_stage={}", stage.as_str()));
    (stage, notes.join("; "))
}

/// Run an in-process SSH listener (Partial durable accept per SoT §17(c)).
///
/// Advances structured KEX / sign / NEWKEYS / auth / channel stages as far as
/// honest Partial allows per connection. Stays up until `stop` is set.
/// Bind failure fail-closed. Never advertises Claimed OpenSSH / SSH git e2e.
///
/// Callers MUST NOT silent-degrade to HTTP-only when SSH was left enabled.
pub fn serve_ssh_listener(
    opts: ServeSshOptions,
    stop: Arc<AtomicBool>,
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
    let host_key = match load_host_ed25519(&opts.host_key) {
        Ok(h) => Some(h),
        Err(e) => {
            // Keep going — Partial without sign when key missing/unloadable.
            let _ = e;
            None
        }
    };
    let host_load = if host_key.is_some() {
        "ed25519_unencrypted_ok"
    } else {
        "unloadable_or_absent"
    };

    let mut listener = TcpListener::bind(opts.listen)?;
    let mut current = listener.local_addr().unwrap_or(opts.listen);
    listener.set_nonblocking(true)?;
    let our_id = "SSH-2.0-arks_partial_chacha_auth_channel";
    eprintln!(
        "arks serve SSH Partial durable listen on {current} \
         ([[sot:root/arks-self-host-server]] §17(c); design ≠ Claimed OpenSSH git e2e); \
         host_key={hk} ({host_st}; format={host_fmt}; load={host_load}); \
         authorized_keys={ak} ({auth_st}; {auth_probe}); path={path} ({repo_st})",
        hk = opts.host_key.display(),
        ak = opts.authorized_keys.display(),
        path = opts.path.display(),
    );

    let mut sessions: u64 = 0;
    while !stop.load(Ordering::SeqCst) {
        if let Some(slot) = &opts.rebind_desired {
            if let Ok(mut g) = slot.lock() {
                if let Some(new_addr) = g.take() {
                    if new_addr != current {
                        match TcpListener::bind(new_addr) {
                            Ok(l) => {
                                if let Err(e) = l.set_nonblocking(true) {
                                    eprintln!(
                                        "arks serve SSH: live rebind Partial bind ok but \
                                         nonblocking fail ({e}); keeping {current}"
                                    );
                                    *g = Some(new_addr);
                                } else {
                                    eprintln!(
                                        "arks serve SSH: live rebind Partial {current} → {new_addr} \
                                         (design ≠ Claimed zero-downtime hot-rebind)"
                                    );
                                    listener = l;
                                    current = new_addr;
                                }
                            }
                            Err(e) => {
                                eprintln!(
                                    "arks serve SSH: live rebind Partial fail {current} → \
                                     {new_addr} ({e}); keeping prior listen"
                                );
                            }
                        }
                    }
                }
            }
        }

        match listener.accept() {
            Ok((mut stream, peer)) => {
                sessions += 1;
                let _ = stream.set_nonblocking(false);
                let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
                let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
                let ours = format!("{our_id}\r\n");
                let _ = stream.write_all(ours.as_bytes());
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
                    match curve25519_ecdh_selfcheck() {
                        Ok(s) => {
                            let auth = publickey_verify_scaffold(
                                &opts.authorized_keys,
                                host_key.as_ref(),
                            );
                            let chan = channel_git_hookup_scaffold(&opts.path);
                            (
                                SshStage::HostKeySign,
                                format!(
                                    "kex=skipped_no_peer_id; {s}; host_load={host_load}; {auth}; {chan}; \
                                     stop=host_key_sign/wire Missing without peer"
                                ),
                            )
                        }
                        Err(e) => (
                            SshStage::Ecdh,
                            format!("kex=skipped_no_peer_id; ecdh_selfcheck=fail ({e})"),
                        ),
                    }
                } else {
                    run_kex_stages(
                        &mut stream,
                        host_key.as_ref(),
                        our_id,
                        &peer_id,
                        &opts.authorized_keys,
                        &opts.path,
                    )
                };
                let _ = stream.shutdown(Shutdown::Both);
                eprintln!(
                    "arks serve SSH Partial session#{sessions} reached_stage={} \
                     ({id_part}; {kex_part}) — Claimed OpenSSH git e2e Missing",
                    stage.as_str()
                );
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(20));
            }
            Err(e) => {
                return Err(ServerError::Msg(format!(
                    "arks serve SSH Partial durable accept err on {current}: {e}"
                )));
            }
        }
    }

    eprintln!(
        "arks serve SSH Partial durable listen stopped on {current} \
         (sessions={sessions}; design ≠ Claimed; ops SSHD + upload-pack/receive-pack still valid)"
    );
    Ok(())
}

type LoopbackSsh = (
    SocketAddr,
    Arc<AtomicBool>,
    thread::JoinHandle<Result<(), ServerError>>,
);

/// Ephemeral loopback SSH Partial listener for smokes (not Claimed OpenSSH e2e).
pub fn spawn_loopback_ssh(opts: ServeSshOptions) -> Result<LoopbackSsh, ServerError> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let addr = listener.local_addr()?;
    drop(listener);
    let mut opts = opts;
    opts.listen = addr;
    let stop = Arc::new(AtomicBool::new(false));
    let stop2 = stop.clone();
    let handle = thread::spawn(move || serve_ssh_listener(opts, stop2));
    // Brief settle so bind wins the ephemeral port before clients connect.
    thread::sleep(Duration::from_millis(30));
    Ok((addr, stop, handle))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;

    #[test]
    fn parse_git_exec_services() {
        assert_eq!(
            parse_git_exec_service("git-upload-pack '/tmp/r.git'"),
            Some("git-upload-pack")
        );
        assert_eq!(
            parse_git_exec_service("git-receive-pack '/tmp/r.git'"),
            Some("git-receive-pack")
        );
        assert_eq!(parse_git_exec_service("bash"), None);
    }

    #[test]
    fn chacha_aead_roundtrip_partial() {
        let shared = [7u8; 32];
        let h = Sha256::digest(b"exchange-hash-partial");
        let note = encrypted_transport_selfcheck(&shared, &h);
        assert!(note.contains("selfcheck=ok"), "{note}");
    }

    #[test]
    fn authorized_keys_verify_partial() {
        let dir = tempfile::tempdir().unwrap();
        let seed = [9u8; 32];
        let signing = SigningKey::from_bytes(&seed);
        let vk = signing.verifying_key();
        let blob = build_ssh_ed25519_public_blob(vk.as_bytes());
        let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &blob);
        let ak = dir.path().join("authorized_keys");
        std::fs::write(&ak, format!("ssh-ed25519 {b64} test@arks\n")).unwrap();
        let challenge = b"challenge-partial";
        let sig = signing.sign(challenge);
        let mut sig_blob = Vec::new();
        sig_blob.extend(ssh_string(b"ssh-ed25519"));
        sig_blob.extend(ssh_string(&sig.to_bytes()));
        verify_publickey_against_authorized(&ak, &blob, challenge, &sig_blob).unwrap();
    }
}
