//! Smart HTTP v1 ref advertisement (protocol v1, not v2).
//!
//! Shape from gitprotocol-http `smart_reply` plus gitprotocol-pack ref advertisement:
//! pkt-line `# service=$name`, flush-pkt, ref lines, flush-pkt.
//! The first ref line is `oid SP name NUL capability-list LF`.

use crate::error::RemoteError;
use arks_oid::Oid;
use arks_protocol::{
    Packet, decode_packet, decode_text_line, encode_packet, encode_text_line,
    parse_v1_advertisement_payload, v1_advertisement_payload,
};

pub(crate) struct AdvertisedRef {
    pub name: String,
    pub oid: Oid,
}

pub(crate) struct Advertisement {
    pub capabilities: Vec<String>,
    pub refs: Vec<AdvertisedRef>,
}

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn encode_advertisement(
    service: &str,
    caps: &[String],
    refs: &[(String, Oid)],
) -> Result<Vec<u8>, RemoteError> {
    let mut out = encode_text_line(&format!("# service={service}"))
        .map_err(|e| RemoteError::Protocol(e.to_string()))?;
    out.extend(encode_packet(&Packet::Flush).map_err(|e| RemoteError::Protocol(e.to_string()))?);
    if refs.is_empty() {
        let head = format!(
            "{} capabilities^{{}}",
            "0000000000000000000000000000000000000000"
        );
        let payload = v1_advertisement_payload(&head, caps);
        out.extend(
            encode_packet(&Packet::Data(payload))
                .map_err(|e| RemoteError::Protocol(e.to_string()))?,
        );
    } else {
        for (i, (name, oid)) in refs.iter().enumerate() {
            if i == 0 {
                let head = format!("{} {name}", oid.to_hex());
                let payload = v1_advertisement_payload(&head, caps);
                out.extend(
                    encode_packet(&Packet::Data(payload))
                        .map_err(|e| RemoteError::Protocol(e.to_string()))?,
                );
            } else {
                out.extend(
                    encode_text_line(&format!("{} {name}", oid.to_hex()))
                        .map_err(|e| RemoteError::Protocol(e.to_string()))?,
                );
            }
        }
    }
    out.extend(encode_packet(&Packet::Flush).map_err(|e| RemoteError::Protocol(e.to_string()))?);
    Ok(out)
}

pub(crate) fn parse_advertisement(
    body: &[u8],
    expected_service: &str,
) -> Result<Advertisement, RemoteError> {
    let (packet, n) = decode_packet(body).map_err(|e| RemoteError::Protocol(e.to_string()))?;
    let Packet::Data(payload) = packet else {
        return Err(RemoteError::Protocol(
            "smart HTTP reply missing service pkt-line".into(),
        ));
    };
    let text = decode_text_line(&payload).map_err(|e| RemoteError::Protocol(e.to_string()))?;
    let expect = format!("# service={expected_service}");
    if text != expect {
        return Err(RemoteError::Protocol(format!(
            "expected {expect}, got {text}"
        )));
    }
    let (flush, n2) =
        decode_packet(&body[n..]).map_err(|e| RemoteError::Protocol(e.to_string()))?;
    if flush != Packet::Flush {
        return Err(RemoteError::Protocol(
            "smart HTTP service line was not followed by flush-pkt".into(),
        ));
    }
    parse_pack_advertisement(&body[n + n2..])
}

/// Pack-protocol ref advertisement (SSH / `git` / file — no HTTP `# service=` wrapper).
///
/// Cite: <https://git-scm.com/docs/gitprotocol-pack>
pub(crate) fn parse_pack_advertisement(body: &[u8]) -> Result<Advertisement, RemoteError> {
    let mut offset = 0;
    let mut caps = Vec::new();
    let mut refs = Vec::new();
    let mut saw_flush = false;
    while offset < body.len() {
        let (packet, n) =
            decode_packet(&body[offset..]).map_err(|e| RemoteError::Protocol(e.to_string()))?;
        offset += n;
        match packet {
            Packet::Flush => {
                saw_flush = true;
                break;
            }
            Packet::Data(payload) => {
                let (head, tokens) = parse_v1_advertisement_payload(&payload)
                    .map_err(|e| RemoteError::Protocol(e.to_string()))?;
                if caps.is_empty() && !tokens.is_empty() {
                    caps = tokens;
                }
                let head = String::from_utf8(head)
                    .map_err(|_| RemoteError::Protocol("ref line is not UTF-8".into()))?;
                let Some((hex, name)) = head.split_once(' ') else {
                    return Err(RemoteError::Protocol(format!("bad ref line: {head}")));
                };
                if name == "capabilities^{}" || name.ends_with("^{}") {
                    continue;
                }
                let oid = Oid::from_hex(hex)
                    .map_err(|e| RemoteError::Protocol(format!("ref oid: {e}")))?;
                if oid.as_bytes().iter().all(|b| *b == 0) {
                    continue;
                }
                // Homogeneous sha1 or sha256 tips are parsed; mixed-format
                // remotes are refused later via `ensure_remote_object_format`
                // against the local repo's `extensions.objectFormat`.
                refs.push(AdvertisedRef {
                    name: name.to_string(),
                    oid,
                });
            }
            Packet::Delim | Packet::ResponseEnd => {
                return Err(RemoteError::NotImplemented(
                    "protocol v2 advertisement is not claimed; pack v1 only".into(),
                ));
            }
        }
    }
    if !saw_flush {
        return Err(RemoteError::Protocol(
            "ref advertisement missing flush-pkt".into(),
        ));
    }
    Ok(Advertisement {
        capabilities: caps,
        refs,
    })
}

pub(crate) fn cap_advertised(caps: &[String], name: &str) -> bool {
    let prefix = format!("{name}=");
    caps.iter().any(|c| c == name || c.starts_with(&prefix))
}

pub(crate) fn symref_target(caps: &[String], name: &str) -> Option<String> {
    let prefix = format!("symref={name}:");
    caps.iter()
        .find_map(|c| c.strip_prefix(&prefix).map(|target| target.to_string()))
}

pub(crate) fn content_type_is(value: Option<&str>, expected: &str) -> bool {
    match value {
        None => true,
        Some(v) => v.to_ascii_lowercase().contains(expected),
    }
}

/// Refuse mixed sha1↔sha256 remotes (honesty ladder slice).
///
/// Homogeneous sha1 or sha256 ads are allowed when they match the local
/// repo's `extensions.objectFormat`. Cross-format fetch/push remains
/// non-claim (no OID rewrite). Cite: [[adr:root/20261110214800]].
pub(crate) fn ensure_remote_object_format(
    repo: &arks_repo::Repository,
    ad: &Advertisement,
) -> Result<(), RemoteError> {
    let want = repo.oid_kind();
    for r in &ad.refs {
        if r.oid.as_bytes().iter().all(|b| *b == 0) {
            continue;
        }
        if r.oid.kind() != want {
            let local = match want {
                arks_oid::OidKind::Sha1 => "sha1",
                arks_oid::OidKind::Sha256 => "sha256",
            };
            let remote = match r.oid.kind() {
                arks_oid::OidKind::Sha1 => "sha1",
                arks_oid::OidKind::Sha256 => "sha256",
            };
            return Err(RemoteError::NotImplemented(format!(
                "mixed objectFormat remotes not claimed (local {local}, remote tip {} is {remote}); use a matching extensions.objectFormat peer — no sha1↔sha256 rewrite",
                r.name
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_line_matches_gitprotocol_http_example() {
        let body = encode_advertisement("git-upload-pack", &[], &[]).unwrap();
        assert!(body.starts_with(b"001e# service=git-upload-pack\n0000"));
        let ad = parse_advertisement(&body, "git-upload-pack").unwrap();
        assert!(ad.refs.is_empty());
    }

    #[test]
    fn roundtrip_refs_and_symref() {
        let oid = Oid::from_hex("0123456789abcdef0123456789abcdef01234567").unwrap();
        let caps = vec![
            "ofs-delta".into(),
            "side-band-64k".into(),
            "symref=HEAD:refs/heads/main".into(),
            "agent=git/2.39.0".into(),
        ];
        let body =
            encode_advertisement("git-upload-pack", &caps, &[("refs/heads/main".into(), oid)])
                .unwrap();
        let ad = parse_advertisement(&body, "git-upload-pack").unwrap();
        assert_eq!(ad.refs.len(), 1);
        assert_eq!(ad.refs[0].name, "refs/heads/main");
        assert_eq!(ad.refs[0].oid, oid);
        assert_eq!(
            symref_target(&ad.capabilities, "HEAD").as_deref(),
            Some("refs/heads/main")
        );
        assert!(cap_advertised(&ad.capabilities, "side-band-64k"));
        assert!(cap_advertised(&ad.capabilities, "agent"));
    }

    #[test]
    fn parse_accepts_sha256_ref_lines() {
        let oid = Oid::from_hex("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef")
            .unwrap();
        let body = encode_advertisement(
            "git-upload-pack",
            &["ofs-delta".into()],
            &[("refs/heads/main".into(), oid)],
        )
        .unwrap();
        let ad = parse_advertisement(&body, "git-upload-pack").unwrap();
        assert_eq!(ad.refs.len(), 1);
        assert_eq!(ad.refs[0].oid.kind(), arks_oid::OidKind::Sha256);
    }
}
