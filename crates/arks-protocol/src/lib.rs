//! pkt-line framing and capability string helpers (M4 claimed subset).
//!
//! Contract:
//! - pkt-line encode/decode, including flush, delim, and response-end
//! - text lines: trailing LF on encode; one optional LF stripped on decode
//! - capability advertise/parse from caller-supplied tokens only
//! - offline fixture builders for G5; no live transport
//!
//! Primary:
//! - <https://git-scm.com/docs/gitprotocol-common>
//! - <https://git-scm.com/docs/gitprotocol-capabilities>
//! - <https://git-scm.com/docs/gitprotocol-v2>
//!
//! Cite: `.spec/sot/arks-protocol.sot.md`,
//! `.spec/research/20261108071916-git-protocols.research.md`
//!
//! No `gix`. No product-path `git`. No HTTP/SSH.

#![forbid(unsafe_code)]

mod capabilities;
mod error;
mod pkt_line;

pub use capabilities::{
    advertise_capabilities, encode_v2_capability_advertisement, parse_capabilities,
    parse_v1_advertisement_payload, parse_v2_capability_advertisement, v1_advertisement_payload,
};
pub use error::ProtocolError;
pub use pkt_line::{
    MAX_DATA_LEN, MAX_PKT_LEN, Packet, decode_packet, decode_packets, decode_text_line,
    encode_packet, encode_packets, encode_text_line,
};
