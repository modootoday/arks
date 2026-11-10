//! pkt-line encode and decode.
//!
//! Primary: <https://git-scm.com/docs/gitprotocol-common>
//! v2 specials: <https://git-scm.com/docs/gitprotocol-v2> (Packet-Line Framing)

use crate::error::ProtocolError;

/// Maximum data payload in one pkt-line.
///
/// gitprotocol-common: "The maximum length of a pkt-line’s data component is 65516 bytes."
pub const MAX_DATA_LEN: usize = 65_516;

/// Maximum on-wire pkt-line length, including the 4-byte length.
///
/// gitprotocol-common: implementations MUST NOT send a pkt-line whose length exceeds 65520.
pub const MAX_PKT_LEN: usize = 65_520;

const HEX: &[u8; 16] = b"0123456789abcdef";

/// One pkt-line.
///
/// `Flush`, `Delim`, and `ResponseEnd` are length fields (`0000`, `0001`, `0002`),
/// not data payloads. `Data` is `pkt-len` + payload, with `payload.len() == pkt-len - 4`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Packet {
    /// Binary data pkt-line (8-bit clean). Payload does not include the 4 length bytes.
    Data(Vec<u8>),
    /// flush-pkt `0000` (gitprotocol-common). Distinct from an empty data line `0004`.
    Flush,
    /// delim-pkt `0001` (gitprotocol-v2). Separates sections of a message.
    Delim,
    /// response-end-pkt `0002` (gitprotocol-v2). Ends a response on a stateless connection.
    ResponseEnd,
}

/// Encode one pkt-line.
///
/// Length digits are lowercase hex and include themselves.
/// `Data` with an empty payload is refused (`0004` SHOULD NOT be sent).
/// `Data` longer than [`MAX_DATA_LEN`] is refused.
pub fn encode_packet(packet: &Packet) -> Result<Vec<u8>, ProtocolError> {
    match packet {
        Packet::Flush => Ok(b"0000".to_vec()),
        Packet::Delim => Ok(b"0001".to_vec()),
        Packet::ResponseEnd => Ok(b"0002".to_vec()),
        Packet::Data(payload) => encode_data(payload),
    }
}

/// Encode several pkt-lines in order.
pub fn encode_packets(packets: &[Packet]) -> Result<Vec<u8>, ProtocolError> {
    let mut out = Vec::new();
    for packet in packets {
        out.extend(encode_packet(packet)?);
    }
    Ok(out)
}

/// Decode the first pkt-line in `input`.
///
/// Returns the packet and the number of bytes consumed.
/// `0004` is accepted as [`Packet::Data`] with an empty payload.
/// Uppercase hex is rejected (gitprotocol-common `HEXDIG` is lowercase).
pub fn decode_packet(input: &[u8]) -> Result<(Packet, usize), ProtocolError> {
    if input.len() < 4 {
        return Err(ProtocolError::Truncated);
    }
    let len_bytes = [input[0], input[1], input[2], input[3]];
    let pkt_len = parse_hex4(&len_bytes)?;
    match pkt_len {
        0 => Ok((Packet::Flush, 4)),
        1 => Ok((Packet::Delim, 4)),
        2 => Ok((Packet::ResponseEnd, 4)),
        3 => Err(ProtocolError::ReservedLength { pkt_len }),
        pkt_len => {
            let pkt_len = usize::from(pkt_len);
            if pkt_len > MAX_PKT_LEN {
                return Err(ProtocolError::LineTooLong {
                    pkt_len,
                    max: MAX_PKT_LEN,
                });
            }
            if input.len() < pkt_len {
                return Err(ProtocolError::Truncated);
            }
            Ok((Packet::Data(input[4..pkt_len].to_vec()), pkt_len))
        }
    }
}

/// Decode a concatenated pkt-line sequence. Trailing bytes must form complete lines.
pub fn decode_packets(input: &[u8]) -> Result<Vec<Packet>, ProtocolError> {
    let mut out = Vec::new();
    let mut offset = 0;
    while offset < input.len() {
        let (packet, n) = decode_packet(&input[offset..])?;
        out.push(packet);
        offset += n;
    }
    Ok(out)
}

/// Encode a non-binary line as a data pkt-line with a trailing LF.
///
/// gitprotocol-common: a non-binary line SHOULD be terminated by an LF, and that LF
/// counts toward `pkt-len`. An empty `text` becomes a single LF (`0005\n`), not `0004`.
/// A `text` that already ends in LF is not given a second one.
pub fn encode_text_line(text: &str) -> Result<Vec<u8>, ProtocolError> {
    let mut payload = text.as_bytes().to_vec();
    if !payload.ends_with(b"\n") {
        payload.push(b'\n');
    }
    encode_packet(&Packet::Data(payload))
}

/// Decode a data payload as text, stripping one optional trailing LF.
///
/// gitprotocol-common: receivers MUST treat non-binary pkt-lines the same whether
/// or not the trailing LF is present (strip it if present; do not complain if missing).
pub fn decode_text_line(payload: &[u8]) -> Result<String, ProtocolError> {
    let bytes = match payload.split_last() {
        Some((b'\n', rest)) => rest,
        _ => payload,
    };
    String::from_utf8(bytes.to_vec()).map_err(|_| ProtocolError::TextUtf8)
}

fn encode_data(payload: &[u8]) -> Result<Vec<u8>, ProtocolError> {
    if payload.is_empty() {
        return Err(ProtocolError::EmptyDataPkt);
    }
    if payload.len() > MAX_DATA_LEN {
        return Err(ProtocolError::PayloadTooLarge {
            len: payload.len(),
            max: MAX_DATA_LEN,
        });
    }
    let total = payload.len() + 4;
    let total_u16 = u16::try_from(total).map_err(|_| ProtocolError::PayloadTooLarge {
        len: payload.len(),
        max: MAX_DATA_LEN,
    })?;
    let mut out = Vec::with_capacity(total);
    push_hex4(&mut out, total_u16);
    out.extend_from_slice(payload);
    debug_assert_eq!(out.len(), total);
    Ok(out)
}

fn push_hex4(out: &mut Vec<u8>, n: u16) {
    out.push(HEX[usize::from((n >> 12) & 0xf)]);
    out.push(HEX[usize::from((n >> 8) & 0xf)]);
    out.push(HEX[usize::from((n >> 4) & 0xf)]);
    out.push(HEX[usize::from(n & 0xf)]);
}

fn parse_hex4(bytes: &[u8; 4]) -> Result<u16, ProtocolError> {
    let mut n: u16 = 0;
    for &b in bytes {
        let digit = match b {
            b'0'..=b'9' => b - b'0',
            b'a'..=b'f' => b - b'a' + 10,
            _ => return Err(ProtocolError::BadHexLength),
        };
        n = (n << 4) | u16::from(digit);
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn common_doc_examples() {
        // gitprotocol-common pkt-line examples.
        let cases: &[(&[u8], &[u8])] = &[
            (b"0006a\n", b"a\n"),
            (b"0005a", b"a"),
            (b"000bfoobar\n", b"foobar\n"),
        ];
        for (wire, payload) in cases {
            let encoded = encode_packet(&Packet::Data(payload.to_vec())).unwrap();
            assert_eq!(encoded, *wire);
            let (decoded, n) = decode_packet(wire).unwrap();
            assert_eq!(n, wire.len());
            assert_eq!(decoded, Packet::Data(payload.to_vec()));
        }
    }

    #[test]
    fn specials_are_length_fields_not_payloads() {
        assert_eq!(encode_packet(&Packet::Flush).unwrap(), b"0000");
        assert_eq!(encode_packet(&Packet::Delim).unwrap(), b"0001");
        assert_eq!(encode_packet(&Packet::ResponseEnd).unwrap(), b"0002");
        assert_eq!(decode_packet(b"0000").unwrap(), (Packet::Flush, 4));
        assert_eq!(decode_packet(b"0001").unwrap(), (Packet::Delim, 4));
        assert_eq!(decode_packet(b"0002").unwrap(), (Packet::ResponseEnd, 4));
    }

    #[test]
    fn empty_data_0004_parse_only() {
        assert_eq!(
            decode_packet(b"0004").unwrap(),
            (Packet::Data(Vec::new()), 4)
        );
        assert_eq!(
            encode_packet(&Packet::Data(Vec::new())),
            Err(ProtocolError::EmptyDataPkt)
        );
        // Text helper emits a single LF, not 0004.
        assert_eq!(encode_text_line("").unwrap(), b"0005\n");
    }

    #[test]
    fn binary_roundtrip_is_8bit_clean() {
        let payload = vec![0x00, 0xff, 0x0a, 0x7f, b'A'];
        let wire = encode_packet(&Packet::Data(payload.clone())).unwrap();
        let (decoded, n) = decode_packet(&wire).unwrap();
        assert_eq!(n, wire.len());
        assert_eq!(decoded, Packet::Data(payload));
    }

    #[test]
    fn text_helper_lf_rules() {
        assert_eq!(encode_text_line("a").unwrap(), b"0006a\n");
        assert_eq!(encode_text_line("a\n").unwrap(), b"0006a\n");
        assert_eq!(decode_text_line(b"a\n").unwrap(), "a");
        assert_eq!(decode_text_line(b"a").unwrap(), "a");
        // Only one trailing LF is stripped.
        assert_eq!(decode_text_line(b"a\n\n").unwrap(), "a\n");
        assert_eq!(
            decode_text_line(&[0xff, 0xfe]),
            Err(ProtocolError::TextUtf8)
        );
    }

    #[test]
    fn refuse_bad_lengths() {
        assert_eq!(decode_packet(b"000"), Err(ProtocolError::Truncated));
        assert_eq!(decode_packet(b"000G"), Err(ProtocolError::BadHexLength));
        assert_eq!(decode_packet(b"000A"), Err(ProtocolError::BadHexLength));
        assert_eq!(
            decode_packet(b"0003"),
            Err(ProtocolError::ReservedLength { pkt_len: 3 })
        );
        assert_eq!(decode_packet(b"0006a"), Err(ProtocolError::Truncated));
        // 0xfff1 = 65521, one past the documented maximum of 65520.
        assert_eq!(
            decode_packet(b"fff1"),
            Err(ProtocolError::LineTooLong {
                pkt_len: 65521,
                max: MAX_PKT_LEN,
            })
        );
        let too_big = vec![b'x'; MAX_DATA_LEN + 1];
        assert_eq!(
            encode_packet(&Packet::Data(too_big)),
            Err(ProtocolError::PayloadTooLarge {
                len: MAX_DATA_LEN + 1,
                max: MAX_DATA_LEN,
            })
        );
    }

    #[test]
    fn max_payload_encodes() {
        let payload = vec![b'x'; MAX_DATA_LEN];
        let wire = encode_packet(&Packet::Data(payload.clone())).unwrap();
        assert_eq!(&wire[..4], b"fff0");
        assert_eq!(wire.len(), MAX_PKT_LEN);
        let (decoded, n) = decode_packet(&wire).unwrap();
        assert_eq!(n, MAX_PKT_LEN);
        assert_eq!(decoded, Packet::Data(payload));
    }
}
