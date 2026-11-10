//! Git bundle v2 header encode/decode (claimed subset).
//!
//! Primary: <https://git-scm.com/docs/gitformat-bundle>
//! Ownership: Ring 2 pack artifacts — bundle wraps a pack payload.
//! Prerequisites (`-` lines) are refused (not claimed).

use crate::PackError;
use arks_oid::Oid;

const MAGIC: &[u8] = b"# v2 git bundle\n";

/// Parsed v2 bundle: advertised refs + trailing pack bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleFile {
    pub refs: Vec<(Oid, String)>,
    pub pack: Vec<u8>,
}

/// Encode a v2 bundle header + pack body.
pub fn encode_bundle(refs: &[(String, Oid)], pack: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(MAGIC);
    for (name, oid) in refs {
        out.extend_from_slice(oid.to_hex().as_bytes());
        out.push(b' ');
        out.extend_from_slice(name.as_bytes());
        out.push(b'\n');
    }
    out.push(b'\n');
    out.extend_from_slice(pack);
    out
}

/// Parse a v2 git bundle. Refuses prerequisite lines.
pub fn parse_bundle(bytes: &[u8]) -> Result<BundleFile, PackError> {
    if !bytes.starts_with(MAGIC) {
        return Err(PackError::Bundle("not a v2 git bundle".into()));
    }
    let mut i = MAGIC.len();
    let mut refs = Vec::new();
    loop {
        if i >= bytes.len() {
            return Err(PackError::Bundle("truncated bundle header".into()));
        }
        let Some(nl) = bytes[i..].iter().position(|b| *b == b'\n') else {
            return Err(PackError::Bundle("truncated bundle header".into()));
        };
        let line = &bytes[i..i + nl];
        i += nl + 1;
        if line.is_empty() {
            break;
        }
        let line = std::str::from_utf8(line)
            .map_err(|_| PackError::Bundle("bundle header is not utf-8".into()))?;
        if line.starts_with('-') {
            return Err(PackError::Bundle(
                "bundle prerequisites are not claimed".into(),
            ));
        }
        let Some((hex, name)) = line.split_once(' ') else {
            return Err(PackError::Bundle(format!("bad bundle ref line: {line}")));
        };
        if name.is_empty() {
            return Err(PackError::Bundle("bundle ref name is empty".into()));
        }
        refs.push((Oid::from_hex(hex)?, name.to_owned()));
    }
    Ok(BundleFile {
        refs,
        pack: bytes[i..].to_vec(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use arks_oid::OidKind;

    #[test]
    fn roundtrip_empty_pack() {
        let oid = Oid::from_bytes(OidKind::Sha1, &[0u8; 20]).unwrap();
        let refs = vec![("refs/heads/main".to_owned(), oid)];
        let bytes = encode_bundle(&refs, b"PACK");
        let parsed = parse_bundle(&bytes).unwrap();
        assert_eq!(parsed.refs.len(), 1);
        assert_eq!(parsed.refs[0].1, "refs/heads/main");
        assert_eq!(parsed.pack, b"PACK");
    }

    #[test]
    fn refuse_non_v2() {
        assert!(parse_bundle(b"# v1 git bundle\n").is_err());
    }
}
