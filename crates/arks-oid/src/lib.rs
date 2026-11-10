//! OID newtype for Git object identity.
//!
//! Contract (M1): fixed-width digest selected by object format length
//! (20 = SHA-1, 32 = SHA-256). Hex parse/format/compare. No I/O. No `gix`.
//! Independent of `arks-hash` (Ring 0; no workspace deps).
//!
//! Primary: <https://git-scm.com/docs/gitformat-loose>
//! Cite: `.spec/sot/core-object-model.sot.md`

#![forbid(unsafe_code)]

/// Object-format width for an OID (bytes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OidKind {
    Sha1,
    Sha256,
}

impl OidKind {
    pub const fn digest_len(self) -> usize {
        match self {
            Self::Sha1 => 20,
            Self::Sha256 => 32,
        }
    }

    pub const fn hex_len(self) -> usize {
        self.digest_len() * 2
    }
}

/// Fixed-width object id.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Oid {
    kind: OidKind,
    bytes: [u8; 32],
}

impl Oid {
    pub fn kind(self) -> OidKind {
        self.kind
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.kind.digest_len()]
    }

    pub fn from_bytes(kind: OidKind, bytes: &[u8]) -> Result<Self, OidError> {
        if bytes.len() != kind.digest_len() {
            return Err(OidError::Length {
                expected: kind.digest_len(),
                actual: bytes.len(),
            });
        }
        let mut buf = [0u8; 32];
        buf[..kind.digest_len()].copy_from_slice(bytes);
        Ok(Self { kind, bytes: buf })
    }

    pub fn from_hex(hex: &str) -> Result<Self, OidError> {
        let kind = match hex.len() {
            40 => OidKind::Sha1,
            64 => OidKind::Sha256,
            n => return Err(OidError::HexLength(n)),
        };
        if !hex.as_bytes().iter().all(u8::is_ascii_hexdigit) {
            return Err(OidError::HexCharset);
        }
        let mut buf = [0u8; 32];
        for (i, chunk) in hex.as_bytes().chunks(2).enumerate() {
            let hi = hex_val(chunk[0])?;
            let lo = hex_val(chunk[1])?;
            buf[i] = (hi << 4) | lo;
        }
        Ok(Self { kind, bytes: buf })
    }

    pub fn to_hex(&self) -> String {
        let mut s = String::with_capacity(self.kind.hex_len());
        for b in self.as_bytes() {
            s.push(HEX[(b >> 4) as usize] as char);
            s.push(HEX[(b & 0xf) as usize] as char);
        }
        s
    }
}

impl core::fmt::Debug for Oid {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Oid({})", self.to_hex())
    }
}

impl core::fmt::Display for Oid {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.to_hex())
    }
}

const HEX: &[u8; 16] = b"0123456789abcdef";

fn hex_val(c: u8) -> Result<u8, OidError> {
    match c {
        b'0'..=b'9' => Ok(c - b'0'),
        b'a'..=b'f' => Ok(c - b'a' + 10),
        b'A'..=b'F' => Ok(c - b'A' + 10),
        _ => Err(OidError::HexCharset),
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum OidError {
    #[error("OID byte length mismatch: expected {expected}, got {actual}")]
    Length { expected: usize, actual: usize },
    #[error("OID hex length invalid: {0} (want 40 or 64)")]
    HexLength(usize),
    #[error("OID hex contains non-hex characters")]
    HexCharset,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_blob_oid_roundtrip() {
        let hex = "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391";
        let oid = Oid::from_hex(hex).unwrap();
        assert_eq!(oid.kind(), OidKind::Sha1);
        assert_eq!(oid.to_hex(), hex);
    }

    #[test]
    fn reject_bad_hex_length() {
        assert!(matches!(Oid::from_hex("abcd"), Err(OidError::HexLength(4))));
    }

    #[test]
    fn hello_blob_oid_roundtrip() {
        let hex = "b6fc4c620b67d95f953a5c1c1230aaab5db5a1b0";
        let oid = Oid::from_hex(hex).unwrap();
        assert_eq!(oid.kind(), OidKind::Sha1);
        assert_eq!(oid.to_hex(), hex);
    }

    #[test]
    fn reject_bad_hex_charset() {
        assert!(matches!(
            Oid::from_hex("e69de29bb2d1d6434b8b29ae775ad8c2e48c539g"),
            Err(OidError::HexCharset)
        ));
        assert!(matches!(
            Oid::from_hex("e69de29bb2d1d6434b8b29ae775ad8c2e48c539 "),
            Err(OidError::HexCharset)
        ));
    }

    #[test]
    fn accept_uppercase_hex() {
        let lower = "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391";
        let upper = "E69DE29BB2D1D6434B8B29AE775AD8C2E48C5391";
        let oid = Oid::from_hex(upper).unwrap();
        assert_eq!(oid.to_hex(), lower);
        assert_eq!(oid.as_bytes(), Oid::from_hex(lower).unwrap().as_bytes());
    }

    #[test]
    fn reject_wrong_byte_length() {
        assert!(matches!(
            Oid::from_bytes(OidKind::Sha1, &[0u8; 19]),
            Err(OidError::Length {
                expected: 20,
                actual: 19
            })
        ));
        assert!(matches!(
            Oid::from_bytes(OidKind::Sha256, &[0u8; 20]),
            Err(OidError::Length {
                expected: 32,
                actual: 20
            })
        ));
    }

    #[test]
    fn display_and_debug_lowercase_hex() {
        let hex = "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391";
        let oid = Oid::from_hex(hex).unwrap();
        assert_eq!(oid.to_string(), hex);
        assert_eq!(format!("{oid:?}"), format!("Oid({hex})"));
    }

    #[test]
    fn from_bytes_roundtrip_sha1() {
        let bytes: [u8; 20] = [
            0xe6, 0x9d, 0xe2, 0x9b, 0xb2, 0xd1, 0xd6, 0x43, 0x4b, 0x8b, 0x29, 0xae, 0x77, 0x5a,
            0xd8, 0xc2, 0xe4, 0x8c, 0x53, 0x91,
        ];
        let oid = Oid::from_bytes(OidKind::Sha1, &bytes).unwrap();
        assert_eq!(oid.to_hex(), "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391");
    }

    #[test]
    fn sha256_hex_roundtrip() {
        let hex = "473a0f4c3be8a93681a267e3b1e9a7dcda1185436fe141f7749120a303721813";
        let oid = Oid::from_hex(hex).unwrap();
        assert_eq!(oid.kind(), OidKind::Sha256);
        assert_eq!(oid.as_bytes().len(), 32);
        assert_eq!(oid.to_hex(), hex);
    }

    #[test]
    fn reject_near_miss_hex_lengths() {
        for n in [39usize, 41, 63, 65] {
            let s = "a".repeat(n);
            assert!(
                matches!(Oid::from_hex(&s), Err(OidError::HexLength(got)) if got == n),
                "len {n}"
            );
        }
    }

    #[test]
    fn mixed_case_hex_normalizes_lower() {
        let mixed = "E69de29BB2d1D6434b8B29ae775aD8c2E48c5391";
        let oid = Oid::from_hex(mixed).unwrap();
        assert_eq!(oid.to_hex(), "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391");
    }

    #[test]
    fn oid_kinds_expose_widths() {
        assert_eq!(OidKind::Sha1.digest_len(), 20);
        assert_eq!(OidKind::Sha1.hex_len(), 40);
        assert_eq!(OidKind::Sha256.digest_len(), 32);
        assert_eq!(OidKind::Sha256.hex_len(), 64);
    }
}
