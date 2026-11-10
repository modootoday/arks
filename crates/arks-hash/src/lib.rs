//! In-engine hashing for Git object identity.
//!
//! Contract (M1): compute digests of uncompressed object bytes
//! (`type SP size NUL` + payload) matching stock Git for the claimed
//! object formats. No I/O. No `gix`. No `git` subprocess.
//!
//! Primary: <https://git-scm.com/docs/gitformat-loose>
//! Cite: `.spec/research/20261108071913-git-objects-pack.research.md`

#![forbid(unsafe_code)]

/// Repository object-format hash algorithm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ObjectFormat {
    /// Classic SHA-1 (20-byte OID). M1 claimed interop format.
    Sha1,
    /// SHA-256 (32-byte OID). API present; G1 fixtures claim Sha1 first.
    Sha256,
}

impl ObjectFormat {
    /// Digest length in bytes for this format.
    pub const fn digest_len(self) -> usize {
        match self {
            Self::Sha1 => 20,
            Self::Sha256 => 32,
        }
    }
}

/// Fixed-capacity digest bytes for either object format.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Digest {
    format: ObjectFormat,
    bytes: [u8; 32],
    len: u8,
}

impl Digest {
    pub fn format(self) -> ObjectFormat {
        self.format
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len as usize]
    }

    pub fn from_bytes(format: ObjectFormat, bytes: &[u8]) -> Result<Self, HashError> {
        let need = format.digest_len();
        if bytes.len() != need {
            return Err(HashError::DigestLength {
                expected: need,
                actual: bytes.len(),
            });
        }
        let mut out = [0u8; 32];
        out[..need].copy_from_slice(bytes);
        Ok(Self {
            format,
            bytes: out,
            len: need as u8,
        })
    }
}

impl core::fmt::Debug for Digest {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Digest({:?}, ", self.format)?;
        for b in self.as_bytes() {
            write!(f, "{b:02x}")?;
        }
        write!(f, ")")
    }
}

/// Incremental hasher for a chosen object format.
#[derive(Clone)]
pub struct Hasher {
    inner: HasherInner,
}

#[derive(Clone)]
enum HasherInner {
    Sha1(sha1::Sha1),
    Sha256(sha2::Sha256),
}

impl Hasher {
    pub fn new(format: ObjectFormat) -> Self {
        let inner = match format {
            ObjectFormat::Sha1 => {
                use sha1::Digest as _;
                HasherInner::Sha1(sha1::Sha1::new())
            }
            ObjectFormat::Sha256 => {
                use sha2::Digest as _;
                HasherInner::Sha256(sha2::Sha256::new())
            }
        };
        Self { inner }
    }

    pub fn update(&mut self, data: &[u8]) {
        match &mut self.inner {
            HasherInner::Sha1(h) => {
                use sha1::Digest as _;
                h.update(data);
            }
            HasherInner::Sha256(h) => {
                use sha2::Digest as _;
                h.update(data);
            }
        }
    }

    pub fn finalize(self) -> Digest {
        match self.inner {
            HasherInner::Sha1(h) => {
                use sha1::Digest as _;
                let out = h.finalize();
                Digest::from_bytes(ObjectFormat::Sha1, out.as_slice()).expect("sha1 len")
            }
            HasherInner::Sha256(h) => {
                use sha2::Digest as _;
                let out = h.finalize();
                Digest::from_bytes(ObjectFormat::Sha256, out.as_slice()).expect("sha256 len")
            }
        }
    }
}

/// Hash arbitrary bytes with the given object format.
pub fn hash_bytes(format: ObjectFormat, data: &[u8]) -> Digest {
    // Reuse SHA-1 TLS when free (pack/idx trailers on J-CLONE-LOCAL).
    if format == ObjectFormat::Sha1 {
        let reused = SHA1_TLS.try_with(|cell| {
            let Ok(mut h) = cell.try_borrow_mut() else {
                return None;
            };
            use sha1::Digest as _;
            h.reset();
            h.update(data);
            let out = h.finalize_reset();
            Some(Digest::from_bytes(ObjectFormat::Sha1, out.as_slice()).expect("sha1 len"))
        });
        if let Ok(Some(d)) = reused {
            return d;
        }
    }
    let mut h = Hasher::new(format);
    h.update(data);
    h.finalize()
}

/// Hash uncompressed Git object bytes: `type SP size NUL` + `payload`.
///
/// `type_name` is the ASCII type token (`blob`, `tree`, `commit`, `tag`).
/// `payload` is the raw object body (not zlib-wrapped).
pub fn hash_object(format: ObjectFormat, type_name: &str, payload: &[u8]) -> Digest {
    // Reuse one SHA-1 state across pack ingest (220+ objects on J-CLONE-LOCAL).
    if format == ObjectFormat::Sha1 {
        return SHA1_TLS.with(|cell| {
            use sha1::Digest as _;
            let mut h = cell.borrow_mut();
            h.reset();
            h.update(type_name.as_bytes());
            h.update(b" ");
            let mut size_buf = [0u8; 20];
            let size = write_decimal(payload.len() as u64, &mut size_buf);
            h.update(size);
            h.update([0]);
            h.update(payload);
            let out = h.finalize_reset();
            Digest::from_bytes(ObjectFormat::Sha1, out.as_slice()).expect("sha1 len")
        });
    }
    let mut h = Hasher::new(format);
    h.update(type_name.as_bytes());
    h.update(b" ");
    // Stack decimal — pack ingest hashes every object; avoid `to_string` allocs.
    let mut size_buf = [0u8; 20];
    let size = write_decimal(payload.len() as u64, &mut size_buf);
    h.update(size);
    h.update(&[0]);
    h.update(payload);
    h.finalize()
}

std::thread_local! {
    static SHA1_TLS: std::cell::RefCell<sha1::Sha1> =
        std::cell::RefCell::new({
            use sha1::Digest as _;
            sha1::Sha1::new()
        });
}

fn write_decimal(mut n: u64, buf: &mut [u8; 20]) -> &[u8] {
    if n == 0 {
        buf[19] = b'0';
        return &buf[19..];
    }
    let mut i = 20;
    while n > 0 {
        i -= 1;
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
    }
    &buf[i..]
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum HashError {
    #[error("digest length mismatch: expected {expected}, got {actual}")]
    DigestLength { expected: usize, actual: usize },
}

#[cfg(test)]
mod tests {
    use super::*;

    const EMPTY_BLOB_SHA1: [u8; 20] = [
        0xe6, 0x9d, 0xe2, 0x9b, 0xb2, 0xd1, 0xd6, 0x43, 0x4b, 0x8b, 0x29, 0xae, 0x77, 0x5a, 0xd8,
        0xc2, 0xe4, 0x8c, 0x53, 0x91,
    ];

    #[test]
    fn empty_blob_sha1_matches_known_oid() {
        // `git hash-object --stdin </dev/null` → e69de29…
        let d = hash_object(ObjectFormat::Sha1, "blob", b"");
        assert_eq!(d.as_bytes(), &EMPTY_BLOB_SHA1);
    }

    #[test]
    fn hello_blob_sha1() {
        // echo -n 'hello' | git hash-object --stdin
        // → b6fc4c620b67d95f953a5c1c1230aaab5db5a1b0
        let d = hash_object(ObjectFormat::Sha1, "blob", b"hello");
        assert_eq!(
            hex::encode_lower(d.as_bytes()),
            "b6fc4c620b67d95f953a5c1c1230aaab5db5a1b0"
        );
    }

    #[test]
    fn empty_blob_sha256_length_and_oid() {
        let d = hash_object(ObjectFormat::Sha256, "blob", b"");
        assert_eq!(d.format(), ObjectFormat::Sha256);
        assert_eq!(d.as_bytes().len(), 32);
        assert_eq!(
            hex::encode_lower(d.as_bytes()),
            "473a0f4c3be8a93681a267e3b1e9a7dcda1185436fe141f7749120a303721813"
        );
    }

    #[test]
    fn hasher_incremental_matches_one_shot() {
        let payload = b"incremental hashing must match one-shot";
        let one_shot = hash_bytes(ObjectFormat::Sha1, payload);

        let mut h = Hasher::new(ObjectFormat::Sha1);
        h.update(&payload[..10]);
        h.update(&payload[10..20]);
        h.update(&payload[20..]);
        assert_eq!(h.finalize(), one_shot);
    }

    #[test]
    fn hash_object_incremental_matches_one_shot() {
        let payload = b"split header and body updates";
        let whole = hash_object(ObjectFormat::Sha1, "blob", payload);

        let mut h = Hasher::new(ObjectFormat::Sha1);
        h.update(b"blob");
        h.update(b" ");
        h.update(payload.len().to_string().as_bytes());
        h.update(&[0]);
        h.update(&payload[..payload.len() / 2]);
        h.update(&payload[payload.len() / 2..]);
        assert_eq!(h.finalize(), whole);
    }

    #[test]
    fn digest_from_bytes_rejects_wrong_length() {
        assert_eq!(
            Digest::from_bytes(ObjectFormat::Sha1, &[0u8; 19]),
            Err(HashError::DigestLength {
                expected: 20,
                actual: 19,
            })
        );
        assert_eq!(
            Digest::from_bytes(ObjectFormat::Sha256, &[0u8; 31]),
            Err(HashError::DigestLength {
                expected: 32,
                actual: 31,
            })
        );
        assert!(Digest::from_bytes(ObjectFormat::Sha1, &EMPTY_BLOB_SHA1).is_ok());
    }

    #[test]
    fn nul_bytes_in_payload_are_hashed_as_payload() {
        let payload = b"before\x00after";
        let d = hash_object(ObjectFormat::Sha1, "blob", payload);
        let mut h = Hasher::new(ObjectFormat::Sha1);
        h.update(b"blob");
        h.update(b" ");
        h.update(payload.len().to_string().as_bytes());
        h.update(&[0]);
        h.update(payload);
        assert_eq!(d, h.finalize());
        assert_ne!(d, hash_object(ObjectFormat::Sha1, "blob", b"before"));
        assert_ne!(d, hash_object(ObjectFormat::Sha1, "blob", b"beforeafter"));
    }

    #[test]
    fn large_payload_stress() {
        let payload: Vec<u8> = (0..64 * 1024).map(|i| (i % 251) as u8).collect();
        let whole = hash_object(ObjectFormat::Sha1, "blob", &payload);

        let mut h = Hasher::new(ObjectFormat::Sha1);
        h.update(b"blob");
        h.update(b" ");
        h.update(payload.len().to_string().as_bytes());
        h.update(&[0]);
        for chunk in payload.chunks(4096) {
            h.update(chunk);
        }
        assert_eq!(h.finalize(), whole);
    }

    #[test]
    fn object_format_digest_len() {
        assert_eq!(ObjectFormat::Sha1.digest_len(), 20);
        assert_eq!(ObjectFormat::Sha256.digest_len(), 32);
    }

    #[test]
    fn type_token_changes_oid() {
        let payload = b"same-bytes";
        let blob = hash_object(ObjectFormat::Sha1, "blob", payload);
        let tree = hash_object(ObjectFormat::Sha1, "tree", payload);
        let commit = hash_object(ObjectFormat::Sha1, "commit", payload);
        let tag = hash_object(ObjectFormat::Sha1, "tag", payload);
        assert_ne!(blob, tree);
        assert_ne!(blob, commit);
        assert_ne!(blob, tag);
        assert_ne!(tree, commit);
    }

    #[test]
    fn sha256_incremental_matches_one_shot() {
        let payload = b"sha256 split updates";
        let one = hash_bytes(ObjectFormat::Sha256, payload);
        let mut h = Hasher::new(ObjectFormat::Sha256);
        h.update(&payload[..5]);
        h.update(&payload[5..]);
        assert_eq!(h.finalize(), one);
        assert_eq!(one.as_bytes().len(), 32);
    }

    #[test]
    fn digest_wrong_format_length_pairs() {
        assert!(Digest::from_bytes(ObjectFormat::Sha1, &[0u8; 32]).is_err());
        assert!(Digest::from_bytes(ObjectFormat::Sha256, &[0u8; 20]).is_err());
    }
}

// Tiny hex helper for tests only (avoid extra dep in unit tests of known vectors).
#[cfg(test)]
mod hex {
    pub fn encode_lower(bytes: &[u8]) -> String {
        let mut s = String::with_capacity(bytes.len() * 2);
        for b in bytes {
            s.push_str(&format!("{b:02x}"));
        }
        s
    }
}
