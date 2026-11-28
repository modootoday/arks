//! Canonical Git LFS pointer v1 parser and formatter.
//!
//! Primary reference: <https://github.com/git-lfs/git-lfs/blob/main/docs/spec.md>
//! Spec: `sot:root/v0-2-0-scope-expansion` §2.7 (E6).

pub const LFS_POINTER_VERSION: &str = "https://git-lfs.github.com/spec/v1";
pub const MAX_POINTER_BYTES: usize = 1024;

/// Canonical Git LFS v1 pointer (`oid sha256:<64hex>`, `size <bytes>`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LfsPointer {
    pub oid: String,
    pub size: u64,
}

impl LfsPointer {
    /// Validate that `oid` is 64 lowercase hexadecimal characters (SHA-256).
    pub fn is_valid_oid(oid: &str) -> bool {
        oid.len() == 64 && oid.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
    }

    /// Compute a canonical Git LFS v1 pointer directly from an in-memory byte slice.
    pub fn from_bytes(bytes: &[u8]) -> Self {
        use sha2::{Digest, Sha256};
        let digest = Sha256::digest(bytes);
        let oid = crate::lfs::store::hex_lower(&digest);
        Self {
            oid,
            size: bytes.len() as u64,
        }
    }

    /// Encode this pointer in canonical Git LFS v1 format (LF-terminated, sorted keys).
    pub fn encode(&self) -> String {
        format!(
            "version {LFS_POINTER_VERSION}\noid sha256:{}\nsize {}\n",
            self.oid, self.size
        )
    }

    /// Parse a canonical Git LFS v1 pointer. Returns `None` if `bytes` is not a valid pointer.
    pub fn parse(bytes: &[u8]) -> Option<Self> {
        if bytes.is_empty() || bytes.len() > MAX_POINTER_BYTES {
            return None;
        }
        let text = std::str::from_utf8(bytes).ok()?;
        let mut saw_version = false;
        let mut oid: Option<String> = None;
        let mut size: Option<u64> = None;
        for raw_line in text.lines() {
            let line = raw_line.trim_end_matches('\r');
            if line.is_empty() {
                continue;
            }
            let (k, v) = line.split_once(' ')?;
            match k {
                "version" => {
                    if v != LFS_POINTER_VERSION {
                        return None;
                    }
                    saw_version = true;
                }
                "oid" => {
                    let hex = v.strip_prefix("sha256:")?;
                    if !Self::is_valid_oid(hex) {
                        return None;
                    }
                    oid = Some(hex.to_string());
                }
                "size" => {
                    size = Some(v.parse::<u64>().ok()?);
                }
                _ => {
                    // Extension keys (e.g. ext-0-...) are permitted by spec if well-formed.
                    if !k.starts_with("ext-") {
                        return None;
                    }
                }
            }
        }
        if !saw_version {
            return None;
        }
        Some(Self {
            oid: oid?,
            size: size?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pointer_roundtrip_and_reject_non_pointers() {
        let oid = "4d7a214614ab2935c943f9e0ff69d22eadbb8f32b1258daaa5e2ca24d17e2393";
        let ptr = LfsPointer {
            oid: oid.into(),
            size: 12345,
        };
        let encoded = ptr.encode();
        assert_eq!(LfsPointer::parse(encoded.as_bytes()), Some(ptr));
        assert_eq!(LfsPointer::parse(b"hello world\n"), None);
        assert_eq!(
            LfsPointer::parse(
                b"version https://git-lfs.github.com/spec/v1\noid sha256:bad\nsize 1\n"
            ),
            None
        );
    }
}
