//! G5 offline pkt-line and capability fixtures (host-advisory).
//!
//! Replays documented framing. Not a live upload-pack / receive-pack session.
//!
//! Primary:
//! - https://git-scm.com/docs/gitprotocol-common
//! - https://git-scm.com/docs/gitprotocol-capabilities
//! - https://git-scm.com/docs/gitprotocol-v2
//! - https://git-scm.com/docs/gitprotocol-pack (`capabilities^{}` when no refs)

use arks_protocol::{
    Packet, advertise_capabilities, decode_packet, decode_packets, decode_text_line, encode_packet,
    encode_v2_capability_advertisement, parse_capabilities, parse_v1_advertisement_payload,
    parse_v2_capability_advertisement, v1_advertisement_payload,
};

#[test]
fn flush_data_delim_response_end_sequence() {
    let packets = vec![
        Packet::Data(b"hello\n".to_vec()),
        Packet::Flush,
        Packet::Data(vec![0x00, 0xff, 0x10]),
        Packet::Delim,
        Packet::Data(b"section\n".to_vec()),
        Packet::ResponseEnd,
        Packet::Flush,
    ];
    let mut wire = Vec::new();
    for packet in &packets {
        wire.extend(encode_packet(packet).unwrap());
    }
    assert!(wire.windows(4).any(|w| w == b"0000"));
    assert!(wire.windows(4).any(|w| w == b"0001"));
    assert!(wire.windows(4).any(|w| w == b"0002"));

    let decoded = decode_packets(&wire).unwrap();
    assert_eq!(decoded, packets);

    // Consumed lengths: specials are 4 bytes; data length includes those 4.
    let mut offset = 0;
    for packet in &packets {
        let (one, n) = decode_packet(&wire[offset..]).unwrap();
        assert_eq!(&one, packet);
        match packet {
            Packet::Flush | Packet::Delim | Packet::ResponseEnd => assert_eq!(n, 4),
            Packet::Data(payload) => assert_eq!(n, payload.len() + 4),
        }
        offset += n;
    }
    assert_eq!(offset, wire.len());
}

#[test]
fn capability_parse_advertise_roundtrip() {
    let caps = [
        "multi_ack_detailed",
        "no-done",
        "ofs-delta",
        "side-band-64k",
        "agent=arks/0.0.0",
        "not-a-documented-token",
    ];
    let advertised = advertise_capabilities(&caps);
    assert_eq!(
        advertised,
        "multi_ack_detailed no-done ofs-delta side-band-64k agent=arks/0.0.0 not-a-documented-token"
    );
    assert_eq!(parse_capabilities(&advertised), caps);
    assert_eq!(parse_capabilities(&format!("{advertised}\n")), caps);
    assert_eq!(advertise_capabilities(&[] as &[&str]), "");
    assert!(parse_capabilities("").is_empty());
}

#[test]
fn v1_style_advertisement_fragment() {
    // gitprotocol-common: zero-id = 40*"0" (SHA-1 width documented there).
    // gitprotocol-pack: no refs → `zero-id SP capabilities^{} NUL capability-list LF`,
    // then the advertisement ends with flush-pkt.
    let zero_id = "0".repeat(40);
    let head = format!("{zero_id} capabilities^{{}}");
    let caps = ["ofs-delta", "agent=arks/0.0.0", "not-a-documented-token"];
    let payload = v1_advertisement_payload(&head, &caps);
    assert!(payload.contains(&0));
    assert!(payload.ends_with(b"\n"));

    let wire = {
        let mut bytes = encode_packet(&Packet::Data(payload.clone())).unwrap();
        bytes.extend(encode_packet(&Packet::Flush).unwrap());
        bytes
    };
    let packets = decode_packets(&wire).unwrap();
    assert_eq!(packets.len(), 2);
    let Packet::Data(got_payload) = &packets[0] else {
        panic!("first packet is the advertisement line");
    };
    assert_eq!(packets[1], Packet::Flush);
    let (got_head, got_caps) = parse_v1_advertisement_payload(got_payload).unwrap();
    assert_eq!(got_head, head.as_bytes());
    assert_eq!(got_caps, caps);
    assert_eq!(got_payload, &payload);

    // First-ref form: obj-id SP refname NUL capability-list LF.
    // The obj-id is synthetic fixture hex, not a claimed object.
    let obj_id = "0123456789abcdef0123456789abcdef01234567";
    assert_eq!(obj_id.len(), 40);
    let ref_head = format!("{obj_id} HEAD");
    let ref_caps = ["symref=HEAD:refs/heads/master", "object-format=sha1"];
    let ref_payload = v1_advertisement_payload(&ref_head, &ref_caps);
    let (parsed_head, parsed_caps) = parse_v1_advertisement_payload(&ref_payload).unwrap();
    assert_eq!(parsed_head, ref_head.as_bytes());
    assert_eq!(parsed_caps, ref_caps);
}

#[test]
fn v2_style_capability_advertisement_with_delim_and_response_end() {
    // gitprotocol-v2 capability-advertisement ends with flush-pkt.
    // delim-pkt separates message sections; response-end-pkt ends a stateless
    // response. Those two are appended after the advertisement so this fixture
    // can tell every claimed special apart. The concatenation is not a live
    // server transcript.
    let caps = ["ls-refs", "fetch", "agent=arks/0.0.0"];
    let advertisement = encode_v2_capability_advertisement(&caps).unwrap();
    // Documented HTTP example prefix: PKT-LINE("version 2" LF) == 000eversion 2\n
    assert!(advertisement.starts_with(b"000eversion 2\n"));

    let (parsed, consumed) = parse_v2_capability_advertisement(&advertisement).unwrap();
    assert_eq!(consumed, advertisement.len());
    assert_eq!(parsed, caps);

    let mut stream = advertisement.clone();
    stream.extend(encode_packet(&Packet::Delim).unwrap());
    stream.extend(encode_packet(&Packet::ResponseEnd).unwrap());

    let (parsed, consumed) = parse_v2_capability_advertisement(&stream).unwrap();
    assert_eq!(parsed, caps);
    let rest = &stream[consumed..];
    assert_eq!(
        decode_packets(rest).unwrap(),
        vec![Packet::Delim, Packet::ResponseEnd]
    );

    let packets = decode_packets(&stream).unwrap();
    assert_eq!(
        packets,
        vec![
            Packet::Data(b"version 2\n".to_vec()),
            Packet::Data(b"ls-refs\n".to_vec()),
            Packet::Data(b"fetch\n".to_vec()),
            Packet::Data(b"agent=arks/0.0.0\n".to_vec()),
            Packet::Flush,
            Packet::Delim,
            Packet::ResponseEnd,
        ]
    );
    assert_eq!(decode_text_line(b"version 2\n").unwrap(), "version 2");
    assert_eq!(decode_text_line(b"ls-refs\n").unwrap(), "ls-refs");
}
