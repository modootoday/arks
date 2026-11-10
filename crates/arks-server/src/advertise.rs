//! Ref advertisement helpers (HTTP smart vs pack-protocol duplex).
//!
//! HTTP: `# service=$name` + flush + refs (gitprotocol-http).
//! SSH / git:// / file duplex: refs only (gitprotocol-pack) — no HTTP service pkt-line.
//! Cite: gitprotocol-http / gitprotocol-pack.

use crate::error::ServerError;
use arks_oid::Oid;
use arks_protocol::{Packet, encode_packet, encode_text_line, v1_advertisement_payload};

/// Encode a smart-HTTP service advertisement (`# service=` prefix).
pub fn encode_http_advertisement(
    service: &str,
    caps: &[String],
    refs: &[(String, Oid)],
) -> Result<Vec<u8>, ServerError> {
    let mut out = encode_text_line(&format!("# service={service}"))
        .map_err(|e| ServerError::Protocol(e.to_string()))?;
    out.extend(encode_packet(&Packet::Flush).map_err(|e| ServerError::Protocol(e.to_string()))?);
    out.extend(encode_ref_list(caps, refs)?);
    Ok(out)
}

/// Encode a pack-protocol (SSH / `git://` / stdio) ref advertisement — no HTTP service line.
pub fn encode_pack_advertisement(
    caps: &[String],
    refs: &[(String, Oid)],
) -> Result<Vec<u8>, ServerError> {
    encode_ref_list(caps, refs)
}

fn encode_ref_list(caps: &[String], refs: &[(String, Oid)]) -> Result<Vec<u8>, ServerError> {
    let mut out = Vec::new();
    if refs.is_empty() {
        let head = format!(
            "{} capabilities^{{}}",
            "0000000000000000000000000000000000000000"
        );
        let payload = v1_advertisement_payload(&head, caps);
        out.extend(
            encode_packet(&Packet::Data(payload))
                .map_err(|e| ServerError::Protocol(e.to_string()))?,
        );
    } else {
        for (i, (name, oid)) in refs.iter().enumerate() {
            if i == 0 {
                let head = format!("{} {name}", oid.to_hex());
                let payload = v1_advertisement_payload(&head, caps);
                out.extend(
                    encode_packet(&Packet::Data(payload))
                        .map_err(|e| ServerError::Protocol(e.to_string()))?,
                );
            } else {
                out.extend(
                    encode_text_line(&format!("{} {name}", oid.to_hex()))
                        .map_err(|e| ServerError::Protocol(e.to_string()))?,
                );
            }
        }
    }
    out.extend(encode_packet(&Packet::Flush).map_err(|e| ServerError::Protocol(e.to_string()))?);
    Ok(out)
}
