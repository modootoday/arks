//! Public-key format checks for forge key records (pure; no I/O, no crypto invention).
//!
//! - OpenSSH public keys: `<algorithm> <base64 blob> [comment]`, where the blob is the
//!   RFC 4253 §6.6 wire encoding whose first `string` names the same algorithm.
//!   Fingerprint follows OpenSSH (`SHA256:` + unpadded base64 of SHA-256 over the blob).
//! - OpenPGP public keys: ASCII armor (RFC 9580 §6.2) wrapping a Public-Key packet
//!   (tag 6). Fingerprint / key ID per RFC 9580 §5.5.4: v4 = SHA-1 over
//!   `0x99 || len16 || body` (key ID = low 64 bits); v6 = SHA-256 over
//!   `0x9B || len32 || body` (key ID = high 64 bits).
//!
//! These checks prove the input is a well-formed public key record. They do not
//! verify signatures and never touch private material (private blocks are refused).

use base64::Engine as _;
use base64::engine::general_purpose::{STANDARD, STANDARD_NO_PAD};

/// SSH public-key algorithms the arks forge accepts (OpenSSH names).
/// DSA (`ssh-dss`) is refused: OpenSSH 9.8 removed it.
pub const SSH_KEY_ALGORITHMS: &[&str] = &[
    "ssh-ed25519",
    "ssh-rsa",
    "ecdsa-sha2-nistp256",
    "ecdsa-sha2-nistp384",
    "ecdsa-sha2-nistp521",
    "sk-ssh-ed25519@openssh.com",
    "sk-ecdsa-sha2-nistp256@openssh.com",
];

/// Parsed OpenSSH public key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SshPublicKey {
    pub algorithm: String,
    /// Base64 blob exactly as supplied (validated).
    pub blob_b64: String,
    pub comment: String,
    /// `SHA256:…` fingerprint (OpenSSH display form).
    pub fingerprint: String,
}

impl SshPublicKey {
    /// Normalized stored form: `<algorithm> <blob>` (comment dropped).
    pub fn normalized(&self) -> String {
        format!("{} {}", self.algorithm, self.blob_b64)
    }
}

/// Parse and validate a single OpenSSH public key line.
pub fn parse_ssh_public_key(input: &str) -> Result<SshPublicKey, String> {
    let text = input.trim();
    if text.is_empty() {
        return Err("empty SSH public key".into());
    }
    if text.contains("PRIVATE KEY") {
        return Err("refusing a private key; supply the .pub public key".into());
    }
    let line = text
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty() && !l.starts_with('#'))
        .ok_or_else(|| "empty SSH public key".to_string())?;
    let mut fields = line.split_whitespace();
    let algorithm = fields.next().unwrap_or_default().to_string();
    if algorithm == "ssh-dss" {
        return Err("ssh-dss (DSA) keys are not accepted".into());
    }
    if !SSH_KEY_ALGORITHMS.contains(&algorithm.as_str()) {
        return Err(format!(
            "unsupported SSH key type `{algorithm}` (want one of: {})",
            SSH_KEY_ALGORITHMS.join(", ")
        ));
    }
    let blob_b64 = fields
        .next()
        .ok_or_else(|| "SSH public key is missing its base64 body".to_string())?
        .to_string();
    let comment = fields.collect::<Vec<_>>().join(" ");
    let blob = STANDARD
        .decode(blob_b64.as_bytes())
        .map_err(|e| format!("SSH public key body is not valid base64: {e}"))?;
    let inner =
        read_ssh_string(&blob).ok_or_else(|| "SSH public key blob is truncated".to_string())?;
    if inner != algorithm.as_bytes() {
        return Err(format!(
            "SSH public key blob names `{}` but the line says `{algorithm}`",
            String::from_utf8_lossy(inner)
        ));
    }
    let digest = sha256(&blob);
    Ok(SshPublicKey {
        algorithm,
        blob_b64,
        comment,
        fingerprint: format!("SHA256:{}", STANDARD_NO_PAD.encode(digest)),
    })
}

fn read_ssh_string(buf: &[u8]) -> Option<&[u8]> {
    let len = u32::from_be_bytes(buf.get(..4)?.try_into().ok()?) as usize;
    buf.get(4..4usize.checked_add(len)?)
}

/// Parsed OpenPGP public key (primary key packet + user IDs).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PgpPublicKey {
    pub version: u8,
    /// Uppercase hex fingerprint.
    pub fingerprint: String,
    /// Uppercase hex 64-bit key ID.
    pub key_id: String,
    /// Key creation time (seconds since epoch) from the packet.
    pub created: u32,
    /// Email addresses found in User ID packets (`Name <mail>`).
    pub emails: Vec<String>,
    /// Normalized armored block (BEGIN … END lines only).
    pub armored: String,
}

const PGP_BEGIN: &str = "-----BEGIN PGP PUBLIC KEY BLOCK-----";
const PGP_END: &str = "-----END PGP PUBLIC KEY BLOCK-----";

/// Parse and validate an ASCII-armored OpenPGP public key block.
pub fn parse_pgp_public_key(input: &str) -> Result<PgpPublicKey, String> {
    if input.contains("PRIVATE KEY BLOCK") {
        return Err("refusing a private key block; export the public key".into());
    }
    let start = input
        .find(PGP_BEGIN)
        .ok_or_else(|| format!("missing `{PGP_BEGIN}` armor header"))?;
    let end_rel = input[start..]
        .find(PGP_END)
        .ok_or_else(|| format!("missing `{PGP_END}` armor tail"))?;
    let armored = input[start..start + end_rel + PGP_END.len()].to_string();
    let inner = &input[start + PGP_BEGIN.len()..start + end_rel];
    let mut lines = inner.lines().map(str::trim).skip_while(|l| l.is_empty());
    // Armor headers (`Key: Value`) end at the first blank line.
    let mut data = String::new();
    let mut crc: Option<String> = None;
    let mut in_headers = true;
    for line in lines.by_ref() {
        if in_headers {
            if line.is_empty() {
                in_headers = false;
                continue;
            }
            if line.contains(": ") {
                continue;
            }
            in_headers = false;
        }
        if line.is_empty() {
            continue;
        }
        if line.len() == 5 && line.starts_with('=') {
            crc = Some(line[1..].to_string());
            continue;
        }
        data.push_str(line);
    }
    let bytes = STANDARD
        .decode(data.as_bytes())
        .map_err(|e| format!("armored body is not valid base64: {e}"))?;
    if let Some(c) = crc {
        let want = STANDARD
            .decode(c.as_bytes())
            .map_err(|e| format!("armor checksum is not valid base64: {e}"))?;
        let got = crc24(&bytes).to_be_bytes();
        if want.as_slice() != &got[1..] {
            return Err("armor checksum (CRC-24) mismatch".into());
        }
    }
    let packets = split_packets(&bytes)?;
    let (tag, body) = packets
        .first()
        .copied()
        .ok_or_else(|| "armored block holds no packets".to_string())?;
    if tag != 6 {
        return Err(format!(
            "first packet has tag {tag}; an OpenPGP public key starts with a Public-Key packet (tag 6)"
        ));
    }
    let version = *body
        .first()
        .ok_or_else(|| "empty Public-Key packet".to_string())?;
    let created = body
        .get(1..5)
        .map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
        .ok_or_else(|| "truncated Public-Key packet".to_string())?;
    let (fingerprint, key_id) = match version {
        4 => {
            let len = u16::try_from(body.len())
                .map_err(|_| "v4 Public-Key packet too long".to_string())?;
            let mut pre = vec![0x99];
            pre.extend_from_slice(&len.to_be_bytes());
            pre.extend_from_slice(body);
            let fp = sha1(&pre)?;
            let id = hex_upper(&fp[fp.len() - 8..]);
            (hex_upper(&fp), id)
        }
        6 => {
            let len = u32::try_from(body.len())
                .map_err(|_| "v6 Public-Key packet too long".to_string())?;
            let mut pre = vec![0x9B];
            pre.extend_from_slice(&len.to_be_bytes());
            pre.extend_from_slice(body);
            let fp = sha256(&pre);
            let id = hex_upper(&fp[..8]);
            (hex_upper(&fp), id)
        }
        v => {
            return Err(format!(
                "unsupported OpenPGP key version {v} (RFC 9580 defines v4 and v6)"
            ));
        }
    };
    let emails = packets
        .iter()
        .filter(|(t, _)| *t == 13)
        .filter_map(|(_, b)| {
            let uid = String::from_utf8_lossy(b);
            let open = uid.rfind('<')?;
            let close = uid[open..].find('>')? + open;
            Some(uid[open + 1..close].to_string())
        })
        .collect();
    Ok(PgpPublicKey {
        version,
        fingerprint,
        key_id,
        created,
        emails,
        armored,
    })
}

/// Split an OpenPGP packet stream into `(tag, body)` pairs (RFC 9580 §4.2).
fn split_packets(mut buf: &[u8]) -> Result<Vec<(u8, &[u8])>, String> {
    let mut out = Vec::new();
    while !buf.is_empty() {
        let hdr = buf[0];
        if hdr & 0x80 == 0 {
            return Err("malformed OpenPGP packet header".into());
        }
        let (tag, len, hlen): (u8, usize, usize) = if hdr & 0x40 != 0 {
            let tag = hdr & 0x3f;
            let o1 = *buf.get(1).ok_or("truncated packet length")? as usize;
            match o1 {
                0..=191 => (tag, o1, 2),
                192..=223 => {
                    let o2 = *buf.get(2).ok_or("truncated packet length")? as usize;
                    (tag, ((o1 - 192) << 8) + o2 + 192, 3)
                }
                255 => {
                    let b = buf.get(2..6).ok_or("truncated packet length")?;
                    (
                        tag,
                        u32::from_be_bytes([b[0], b[1], b[2], b[3]]) as usize,
                        6,
                    )
                }
                _ => return Err("partial-length packets are not valid in a key".into()),
            }
        } else {
            let tag = (hdr >> 2) & 0x0f;
            match hdr & 0x03 {
                0 => (
                    tag,
                    *buf.get(1).ok_or("truncated packet length")? as usize,
                    2,
                ),
                1 => {
                    let b = buf.get(1..3).ok_or("truncated packet length")?;
                    (tag, u16::from_be_bytes([b[0], b[1]]) as usize, 3)
                }
                2 => {
                    let b = buf.get(1..5).ok_or("truncated packet length")?;
                    (
                        tag,
                        u32::from_be_bytes([b[0], b[1], b[2], b[3]]) as usize,
                        5,
                    )
                }
                _ => return Err("indeterminate-length packets are not valid in a key".into()),
            }
        };
        let end = hlen
            .checked_add(len)
            .filter(|e| *e <= buf.len())
            .ok_or_else(|| "truncated OpenPGP packet".to_string())?;
        out.push((tag, &buf[hlen..end]));
        buf = &buf[end..];
    }
    Ok(out)
}

/// CRC-24 of RFC 9580 §6.1.1.
fn crc24(data: &[u8]) -> u32 {
    let mut crc: u32 = 0x00B7_04CE;
    for b in data {
        crc ^= u32::from(*b) << 16;
        for _ in 0..8 {
            crc <<= 1;
            if crc & 0x0100_0000 != 0 {
                crc ^= 0x0186_4CFB;
            }
        }
    }
    crc & 0x00FF_FFFF
}

fn sha256(data: &[u8]) -> Vec<u8> {
    use sha2::Digest as _;
    sha2::Sha256::digest(data).to_vec()
}

fn sha1(data: &[u8]) -> Result<Vec<u8>, String> {
    let mut h = sha1collisiondetection::Sha1CD::default();
    sha1collisiondetection::Digest::update(&mut h, data);
    h.finalize_cd()
        .map(|d| d.to_vec())
        .map_err(|_| "SHA-1 collision attack detected in key packet".to_string())
}

fn hex_upper(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02X}")).collect()
}

#[cfg(test)]
pub(crate) mod fixtures {
    //! Test fixtures built from first principles (no external key material).

    use super::*;

    /// A syntactically valid ed25519 public key line (32 arbitrary key bytes).
    pub fn ssh_ed25519(seed: u8, comment: &str) -> String {
        let mut blob = Vec::new();
        let name = b"ssh-ed25519";
        blob.extend_from_slice(&(name.len() as u32).to_be_bytes());
        blob.extend_from_slice(name);
        blob.extend_from_slice(&32u32.to_be_bytes());
        blob.extend_from_slice(&[seed; 32]);
        format!("ssh-ed25519 {} {comment}", STANDARD.encode(blob))
    }

    /// Armored v4 public key: Public-Key packet (EdDSA-shaped body) + User ID packet.
    pub fn pgp_v4(seed: u8, uid: &str) -> String {
        let mut body = vec![4u8];
        body.extend_from_slice(&0x6000_0000u32.to_be_bytes());
        body.push(22); // EdDSALegacy algorithm id
        body.extend_from_slice(&[seed; 40]);
        let mut pkt = vec![0xC6, body.len() as u8];
        pkt.extend_from_slice(&body);
        pkt.push(0xCD);
        pkt.push(uid.len() as u8);
        pkt.extend_from_slice(uid.as_bytes());
        let b64 = STANDARD.encode(&pkt);
        let crc = crc24(&pkt).to_be_bytes();
        let crc_b64 = STANDARD.encode(&crc[1..]);
        let mut out = String::from("-----BEGIN PGP PUBLIC KEY BLOCK-----\nComment: fixture\n\n");
        for chunk in b64.as_bytes().chunks(64) {
            out.push_str(std::str::from_utf8(chunk).unwrap());
            out.push('\n');
        }
        out.push('=');
        out.push_str(&crc_b64);
        out.push_str("\n-----END PGP PUBLIC KEY BLOCK-----\n");
        out
    }
}

#[cfg(test)]
mod tests {
    use super::fixtures::*;
    use super::*;

    #[test]
    fn ssh_key_accepts_wellformed_and_fingerprints() {
        let k = parse_ssh_public_key(&ssh_ed25519(7, "me@host")).expect("valid");
        assert_eq!(k.algorithm, "ssh-ed25519");
        assert_eq!(k.comment, "me@host");
        assert!(k.fingerprint.starts_with("SHA256:"));
        assert!(!k.normalized().contains("me@host"));
    }

    /// Public keys exported by GnuPG 2.4.4 / OpenSSH (ephemeral fixtures, public halves only);
    /// expected fingerprints are what those tools printed for the same keys.
    const GPG_FIXTURE: &str = "-----BEGIN PGP PUBLIC KEY BLOCK-----

mDMEav9wixYJKwYBBAHaRw8BAQdAQjLeQBeoT4aDP0hMcp8V0S5KliR4Wo7OTC0l
qsx/yCK0JUZpeHR1cmUgS2V5IDxmaXh0dXJlQGV4YW1wbGUuaW52YWxpZD6IkwQT
FgoAOxYhBI7bByOhOpVqZC07Mi8wt6QweLGoBQJq/3CLAhsDBQsJCAcCAiICBhUK
CQgLAgQWAgMBAh4HAheAAAoJEC8wt6QweLGoQmcA/j1iFqpGmluCZTSL06ANxSXg
tb3A038eqruIHmAhLWu5AQDLo/2RiEm7W4TOdcPttVtdoGWKtQK53SBDHVsTh/EZ
AA==
=SSXi
-----END PGP PUBLIC KEY BLOCK-----
";
    const SSH_FIXTURE: &str = "ssh-ed25519 \
        AAAAC3NzaC1lZDI1NTE5AAAAIOeNU8Aa/AgRlkpxyZRX7JNqG5QCAhOXZdT1/GhUi3TM fixture@example.invalid";

    #[test]
    fn fingerprints_match_gnupg_and_openssh() {
        let k = parse_pgp_public_key(GPG_FIXTURE).expect("gpg fixture parses");
        assert_eq!(k.fingerprint, "8EDB0723A13A956A642D3B322F30B7A43078B1A8");
        assert_eq!(k.key_id, "2F30B7A43078B1A8");
        assert_eq!(k.emails, vec!["fixture@example.invalid".to_string()]);
        let s = parse_ssh_public_key(SSH_FIXTURE).expect("ssh fixture parses");
        assert_eq!(
            s.fingerprint,
            "SHA256:MO2X5WSxNRrSc36ZeuLqwBzZc44TWFE2llj7/mJ1s2w"
        );
    }

    #[test]
    fn ssh_key_rejects_bad_inputs() {
        for bad in [
            "",
            "ssh-dss AAAA",
            "ssh-foo AAAA",
            "ssh-ed25519",
            "ssh-ed25519 !!!notbase64",
            "-----BEGIN OPENSSH PRIVATE KEY-----\nabc\n-----END OPENSSH PRIVATE KEY-----",
        ] {
            assert!(parse_ssh_public_key(bad).is_err(), "{bad:?}");
        }
        // Algorithm mismatch between line and blob.
        let line = ssh_ed25519(1, "");
        let swapped = line.replacen("ssh-ed25519", "ssh-rsa", 1);
        assert!(parse_ssh_public_key(&swapped).is_err());
    }

    #[test]
    fn pgp_key_parses_v4_fingerprint_and_email() {
        let k = parse_pgp_public_key(&pgp_v4(3, "Dev <dev@example.org>")).expect("valid");
        assert_eq!(k.version, 4);
        assert_eq!(k.fingerprint.len(), 40);
        assert_eq!(k.key_id, k.fingerprint[24..]);
        assert_eq!(k.emails, vec!["dev@example.org".to_string()]);
        assert_eq!(k.created, 0x6000_0000);
    }

    #[test]
    fn pgp_key_rejects_bad_inputs() {
        assert!(parse_pgp_public_key("hello").is_err());
        assert!(
            parse_pgp_public_key(
                "-----BEGIN PGP PRIVATE KEY BLOCK-----\n\nAA==\n-----END PGP PRIVATE KEY BLOCK-----"
            )
            .is_err()
        );
        let good = pgp_v4(3, "x <x@y>");
        // Corrupt the checksum line.
        let idx = good.find("\n=").unwrap() + 2;
        let orig = good[idx..idx + 4].to_string();
        let repl = if orig == "AAAA" { "BBBB" } else { "AAAA" };
        let mut bad = good.clone();
        bad.replace_range(idx..idx + 4, repl);
        assert!(parse_pgp_public_key(&bad).unwrap_err().contains("CRC-24"));
    }
}
