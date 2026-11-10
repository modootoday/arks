//! Gate G5: offline pkt-line / capability fixture replay (no live network).
//!
//! Primary:
//! - https://git-scm.com/docs/gitprotocol-common
//! - https://git-scm.com/docs/gitprotocol-capabilities
//! - https://git-scm.com/docs/gitprotocol-v2
//! Cite: `.spec/sot/arks-protocol.sot.md`, `verify/matrices/matrix-m004-clean-room-pack-protocol.md`

use arks_protocol::{
    Packet, advertise_capabilities, decode_packets, encode_packet,
    encode_v2_capability_advertisement, parse_capabilities, parse_v1_advertisement_payload,
    parse_v2_capability_advertisement, v1_advertisement_payload,
};

#[test]
fn pkt_line_specials_and_data_offline() {
    let packets = vec![
        Packet::Data(b"want deadbeef\n".to_vec()),
        Packet::Flush,
        Packet::Delim,
        Packet::Data(b"section\n".to_vec()),
        Packet::ResponseEnd,
    ];
    let mut wire = Vec::new();
    for p in &packets {
        wire.extend(encode_packet(p).unwrap());
    }
    assert!(wire.windows(4).any(|w| w == b"0000"));
    assert!(wire.windows(4).any(|w| w == b"0001"));
    assert!(wire.windows(4).any(|w| w == b"0002"));
    assert_eq!(decode_packets(&wire).unwrap(), packets);
}

#[test]
fn capability_advertise_parse_offline() {
    let caps = ["multi_ack_detailed", "ofs-delta", "side-band-64k", "agent=arks/0.0.0"];
    let s = advertise_capabilities(&caps);
    assert_eq!(parse_capabilities(&s), caps);
}

#[test]
fn v1_advertisement_fragment_offline() {
    let zero = "0".repeat(40);
    let head = format!("{zero} capabilities^{{}}");
    let caps = ["multi_ack", "ofs-delta"];
    let payload = v1_advertisement_payload(&head, &caps);
    let (parsed_head, parsed_caps) = parse_v1_advertisement_payload(&payload).unwrap();
    assert_eq!(parsed_head, head.as_bytes());
    assert_eq!(parsed_caps, caps);
    let mut wire = encode_packet(&Packet::Data(payload)).unwrap();
    wire.extend(encode_packet(&Packet::Flush).unwrap());
    let decoded = decode_packets(&wire).unwrap();
    assert_eq!(decoded.len(), 2);
    assert!(matches!(decoded[1], Packet::Flush));
}

#[test]
fn v2_capability_advertisement_offline() {
    let caps = ["ls-refs", "fetch", "agent=arks/0.0.0"];
    let wire = encode_v2_capability_advertisement(&caps).unwrap();
    let (parsed, n) = parse_v2_capability_advertisement(&wire).unwrap();
    assert_eq!(parsed, caps);
    assert_eq!(n, wire.len());
    // Trailing delim / response-end after the advertisement flush stay outside.
    let mut extended = wire.clone();
    extended.extend(encode_packet(&Packet::Delim).unwrap());
    extended.extend(encode_packet(&Packet::ResponseEnd).unwrap());
    let (again, n2) = parse_v2_capability_advertisement(&extended).unwrap();
    assert_eq!(again, caps);
    assert_eq!(n2, wire.len());
}
