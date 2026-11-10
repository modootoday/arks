//! Capability advertise / parse helpers.
//!
//! v1 trailer prose: <https://git-scm.com/docs/gitprotocol-capabilities>
//! (first ref is followed by NUL and a space-delimited capability list).
//! v2 advertisement shape: <https://git-scm.com/docs/gitprotocol-v2>
//! (Capability Advertisement).
//!
//! This module does not invent capability names. Tokens are caller-supplied
//! and unknown peer tokens stay opaque strings.

use crate::error::ProtocolError;
use crate::pkt_line::{Packet, decode_packet, decode_text_line, encode_packets, encode_text_line};

/// Split a space-separated capability trailer into opaque tokens.
///
/// One trailing LF is removed when present (the v1 `capability-list LF` trailer
/// and the v2 per-line LF). Tokens are not checked against a name list.
/// Repeated SP separators do not produce empty tokens. Order is preserved,
/// including duplicates (documented forms such as repeated `symref` / `object-format`).
pub fn parse_capabilities(s: &str) -> Vec<String> {
    let s = s.strip_suffix('\n').unwrap_or(s);
    s.split(' ')
        .filter(|token| !token.is_empty())
        .map(str::to_string)
        .collect()
}

/// Join capability tokens with a single SP, in caller order.
///
/// Empty tokens are skipped so the result does not gain extra separators.
/// Names are not added or rewritten. A token that itself contains SP will
/// split again in [`parse_capabilities`]; the protocol list is SP-delimited
/// and this helper does not quote.
pub fn advertise_capabilities(caps: &[impl AsRef<str>]) -> String {
    caps.iter()
        .map(|cap| cap.as_ref())
        .filter(|cap| !cap.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Payload (not yet length-prefixed) of a v1 first-ref / `capabilities^{}` line.
///
/// Shape from gitprotocol-capabilities / gitprotocol-pack: `head NUL capability-list LF`.
/// `head` is caller-supplied (`obj-id SP refname`, or `zero-id SP capabilities^{}`).
/// The capability list is [`advertise_capabilities`]; this function adds no names.
pub fn v1_advertisement_payload(head: &str, caps: &[impl AsRef<str>]) -> Vec<u8> {
    let trailer = advertise_capabilities(caps);
    let mut out = Vec::with_capacity(head.len() + 1 + trailer.len() + 1);
    out.extend_from_slice(head.as_bytes());
    out.push(0);
    out.extend_from_slice(trailer.as_bytes());
    out.push(b'\n');
    out
}

/// Split a v1 advertisement payload at the first NUL.
///
/// Returns `(head, tokens)`. `head` is the bytes before NUL. Tokens come from
/// the trailer after NUL via [`parse_capabilities`]. A line with no NUL is a
/// non-capability ref line: head is the payload with one optional trailing LF
/// removed, and the token list is empty.
pub fn parse_v1_advertisement_payload(
    payload: &[u8],
) -> Result<(Vec<u8>, Vec<String>), ProtocolError> {
    match payload.iter().position(|&b| b == 0) {
        Some(nul) => {
            let head = payload[..nul].to_vec();
            let trailer = std::str::from_utf8(&payload[nul + 1..])
                .map_err(|_| ProtocolError::CapabilityUtf8)?;
            Ok((head, parse_capabilities(trailer)))
        }
        None => {
            let text = std::str::from_utf8(payload).map_err(|_| ProtocolError::CapabilityUtf8)?;
            let head = text.strip_suffix('\n').unwrap_or(text).as_bytes().to_vec();
            Ok((head, Vec::new()))
        }
    }
}

/// Encode a v2 capability advertisement.
///
/// gitprotocol-v2:
/// `capability-advertisement = protocol-version capability-list flush-pkt`,
/// `protocol-version = PKT-LINE("version 2" LF)`,
/// `capability = PKT-LINE(key[=value] LF)`.
///
/// `version 2` is the protocol-version line, not a caller capability.
/// Each entry of `caps` is one capability line. delim-pkt and response-end-pkt
/// are not part of this advertisement; callers append those packets themselves.
pub fn encode_v2_capability_advertisement(
    caps: &[impl AsRef<str>],
) -> Result<Vec<u8>, ProtocolError> {
    let mut packets = Vec::with_capacity(caps.len() + 2);
    packets.push(text_packet("version 2")?);
    for cap in caps {
        let cap = cap.as_ref();
        if cap.is_empty() {
            continue;
        }
        let line = cap.strip_suffix('\n').unwrap_or(cap);
        packets.push(text_packet(line)?);
    }
    packets.push(Packet::Flush);
    encode_packets(&packets)
}

/// Parse a v2 capability advertisement, stopping after its flush-pkt.
///
/// Returns `(capability tokens, bytes consumed)`. Bytes after the flush-pkt
/// are left unconsumed so a later delim-pkt or response-end-pkt can be decoded
/// separately. The `version 2` line is required and is not returned as a capability.
pub fn parse_v2_capability_advertisement(
    input: &[u8],
) -> Result<(Vec<String>, usize), ProtocolError> {
    let mut offset = 0;
    let mut caps = Vec::new();
    let mut saw_version = false;
    let mut saw_flush = false;
    while offset < input.len() {
        let (packet, n) = decode_packet(&input[offset..])?;
        offset += n;
        match packet {
            Packet::Data(payload) => {
                let text = decode_text_line(&payload)?;
                if !saw_version {
                    if text != "version 2" {
                        return Err(ProtocolError::MissingProtocolVersion);
                    }
                    saw_version = true;
                } else if !text.is_empty() {
                    caps.push(text);
                }
            }
            Packet::Flush => {
                saw_flush = true;
                break;
            }
            Packet::Delim | Packet::ResponseEnd => {
                return Err(ProtocolError::UnexpectedSpecial);
            }
        }
    }
    if !saw_version {
        return Err(ProtocolError::MissingProtocolVersion);
    }
    if !saw_flush {
        return Err(ProtocolError::MissingFlush);
    }
    Ok((caps, offset))
}

fn text_packet(text: &str) -> Result<Packet, ProtocolError> {
    let wire = encode_text_line(text)?;
    // encode_text_line writes one data pkt-line; the payload follows the 4 length bytes.
    Ok(Packet::Data(wire[4..].to_vec()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advertise_parse_roundtrip_preserves_order_and_unknowns() {
        // Names below are either documented (gitprotocol-capabilities) or an
        // opaque unknown used to prove there is no allow-list.
        let caps = [
            "ofs-delta",
            "side-band-64k",
            "agent=git/2.46.0",
            "not-a-documented-token",
            "symref=HEAD:refs/heads/master",
            "symref=refs/remotes/origin/HEAD:refs/heads/master",
        ];
        let advertised = advertise_capabilities(&caps);
        assert_eq!(advertised, caps.join(" "));
        assert!(!advertised.ends_with(' '));
        assert_eq!(
            parse_capabilities(&advertised),
            caps.iter().map(|s| (*s).to_string()).collect::<Vec<_>>()
        );
        assert_eq!(
            parse_capabilities(&format!("{advertised}\n")),
            parse_capabilities(&advertised)
        );
    }

    #[test]
    fn empty_and_duplicate_tokens() {
        assert_eq!(advertise_capabilities(&[] as &[&str]), "");
        assert!(parse_capabilities("").is_empty());
        assert!(parse_capabilities("\n").is_empty());
        assert_eq!(
            parse_capabilities("ofs-delta  side-band-64k "),
            vec!["ofs-delta".to_string(), "side-band-64k".to_string()]
        );
        let dup = advertise_capabilities(&["object-format=sha1", "object-format=sha256"]);
        assert_eq!(
            parse_capabilities(&dup),
            vec![
                "object-format=sha1".to_string(),
                "object-format=sha256".to_string()
            ]
        );
    }
}
