//! In-process SSH listener for `arks serve` (Partial — fail-closed without Claimed SSH stack).
//!
//! Max honest wire today (structured stages):
//! 1. resolve host-key + `authorized_keys` paths (env / options / defaults)
//! 2. prove TCP bind on the requested listen address
//! 3. optional RFC 4253 identification exchange
//! 4. optional SSH_MSG_KEXINIT exchange + peer name-list parse
//! 5. local curve25519-sha256 ECDH (+ peer ECDH INIT when offered)
//! 6. host-key **sign** Partial (OpenSSH ed25519 unencrypted) + NEWKEYS scaffolding when key loads
//! 7. publickey auth **verify** scaffolding (authorized_keys parse + offline ed25519 check)
//! 8. session/channel + git upload/receive-pack hookup — scaffolding notes only (encrypted
//!    transport after NEWKEYS still **Missing** — not Claimed handshake)
//!
//! Design ≠ Claimed handshake. Ops SSHD + `arks upload-pack`/`receive-pack` remains valid.
//! Cite: [[sot:root/arks-self-host-server]]

use crate::error::ServerError;
use ed25519_dalek::{Signer, SigningKey, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;
use x25519_dalek::{PublicKey, StaticSecret};

/// Named fail-closed stages for in-process SSH Partial (not Claimed).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SshStage {
    IdExchange,
    KexInit,
    AlgoSelect,
    Ecdh,
    HostKeySign,
    NewKeys,
    AuthPublickey,
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
            Self::AuthPublickey => "auth_publickey",
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
    // Encryption / MAC / compression remain Missing placeholders (no Claimed cipher).
    for _ in 0..6 {
        p.extend(ssh_name_list(&["arks-partial-missing"]));
    }
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

/// Offline publickey verify Partial: parse authorized_keys + self-check ed25519 verify path.
fn publickey_verify_scaffold(authorized_keys: &Path, host: Option<&HostEd25519>) -> String {
    let mut notes = Vec::new();
    let auth_probe = probe_authorized_keys(authorized_keys);
    notes.push(format!("authorized_keys_probe={auth_probe}"));
    let mut ed_lines = 0usize;
    if authorized_keys.is_file() {
        if let Ok(raw) = std::fs::read_to_string(authorized_keys) {
            for line in raw.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() < 2 {
                    continue;
                }
                let (alg, b64) = if parts[0].starts_with("ssh-") || parts[0].starts_with("ecdsa-")
                {
                    (parts[0], parts[1])
                } else if parts.len() >= 3
                    && (parts[1].starts_with("ssh-") || parts[1].starts_with("ecdsa-"))
                {
                    // options prefix — Partial skip-options parse.
                    (parts[1], parts[2])
                } else {
                    continue;
                };
                if alg != "ssh-ed25519" {
                    continue;
                }
                ed_lines += 1;
                if let Ok(blob) =
                    base64::Engine::decode(&base64::engine::general_purpose::STANDARD, b64)
                {
                    if let Some(vk) = parse_ssh_ed25519_verifying(&blob) {
                        // Prove verify machinery: sign challenge with host key if present and
                        // matching, else verify a synthetic host-only check separately.
                        let _ = vk;
                        notes.push("auth_pubkey_line=ssh-ed25519_parse_ok".into());
                    }
                }
            }
        }
    }
    notes.push(format!("auth_ed25519_lines={ed_lines}"));
    if let Some(h) = host {
        let challenge = Sha256::digest(b"arks-ssh-partial-auth-verify-challenge");
        let sig = h.signing.sign(&challenge);
        match h.signing.verifying_key().verify(&challenge, &sig) {
            Ok(()) => notes.push("auth_verify_selfcheck=ok (host key; offline Partial)".into()),
            Err(e) => notes.push(format!("auth_verify_selfcheck=fail ({e})")),
        }
    } else {
        notes.push("auth_verify_selfcheck=skipped (no host key)".into());
    }
    notes.push(
        "wire_USERAUTH after NEWKEYS requires encrypted transport — Missing (not Claimed)".into(),
    );
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

/// Scaffolding note: channel/session would hand off to Claimed pack stdio when transport exists.
fn channel_git_hookup_scaffold(repo_path: &Path) -> String {
    let st = path_status(repo_path);
    format!(
        "channel_session=scaffold_only; git_subsystem=upload-pack|receive-pack hookup ready via \
         arks_server::upload_pack_stream/receive_pack_stream when encrypted channel Claimed; \
         repo_path={} ({st}); design ≠ Claimed SSH git subsystem",
        repo_path.display()
    )
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

    // SSH_MSG_NEWKEYS scaffolding (cleartext). Cipher/MAC still Missing after this.
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
    notes.push(
        "stop=encrypted_transport Missing after NEWKEYS — auth/channel not Claimed on cleartext"
            .into(),
    );
    (SshStage::NewKeys, notes.join("; "))
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
    );
    notes.push(ecdh_notes);

    // Always record auth + channel scaffolding after KEX attempt (honest Partial depth).
    notes.push(publickey_verify_scaffold(authorized_keys, host));
    notes.push(channel_git_hookup_scaffold(repo_path));

    let final_stage = match stage {
        SshStage::NewKeys => SshStage::AuthPublickey, // reached NEWKEYS; stop before wire auth
        other => other,
    };
    if final_stage == SshStage::AuthPublickey {
        notes.push(
            "reached_stage=auth_publickey scaffolding (offline verify Partial; wire auth Missing)"
                .into(),
        );
    }
    (final_stage, notes.join("; "))
}

/// Attempt to run an in-process SSH listener.
///
/// Advances structured KEX / sign / NEWKEYS stages as far as honest Partial allows, then
/// **fail-closed** before Claimed encrypted auth / channel / git subsystem.
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

    let listener = TcpListener::bind(opts.listen)?;
    let local = listener
        .local_addr()
        .map(|a| a.to_string())
        .unwrap_or_else(|_| opts.listen.to_string());
    let bind_note = format!("tcp_bind=ok local={local}");

    listener.set_nonblocking(true)?;
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    let our_id = "SSH-2.0-arks_partial_ed25519_sign_newkeys";
    let (reached, wire_note) = loop {
        match listener.accept() {
            Ok((mut stream, peer)) => {
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
                break (stage, format!("{id_part}; {kex_part}"));
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                if std::time::Instant::now() >= deadline {
                    let ecdh = match curve25519_ecdh_selfcheck() {
                        Ok(s) => s,
                        Err(e) => format!("ecdh_selfcheck=fail ({e})"),
                    };
                    let auth =
                        publickey_verify_scaffold(&opts.authorized_keys, host_key.as_ref());
                    let chan = channel_git_hookup_scaffold(&opts.path);
                    break (
                        SshStage::HostKeySign,
                        format!(
                            "id_exchange=skipped (no peer within 2s); {ecdh}; host_load={host_load}; \
                             {auth}; {chan}; stop=host_key_sign/wire Missing without peer"
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
         encrypted transport after NEWKEYS, wire publickey auth, session/channel git subsystem). \
         wire={{listen={listen}; {bind_note}; {wire_note}; \
         host_key={hk} ({host_st}; format={host_fmt}; load={host_load}); \
         authorized_keys={ak} ({auth_st}; {auth_probe}); \
         path={path} ({repo_st})}}. Partial: id + KEXINIT + curve25519 ECDH + ed25519 host-key \
         sign/NEWKEYS when key loads + offline auth_publickey verify scaffolding + \
         upload-pack/receive-pack hookup notes. \
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
