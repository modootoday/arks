//! pkt-line and capability-helper errors.

/// Failure while encoding or decoding a claimed protocol helper.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ProtocolError {
    /// Input ended before a 4-byte length or before `pkt-len` payload bytes.
    #[error("pkt-line truncated")]
    Truncated,
    /// Length field is not four lowercase hex digits (`0-9a-f`).
    ///
    /// gitprotocol-common replaces ABNF `HEXDIG` with `DIGIT / "a" / … / "f"`.
    #[error("pkt-len is not 4 lowercase hex digits")]
    BadHexLength,
    /// `0003` is neither a data pkt-line (`pkt-len >= 4`) nor a documented special.
    ///
    /// Specials are flush `0000` (gitprotocol-common), delim `0001`, and
    /// response-end `0002` (gitprotocol-v2).
    #[error("reserved pkt-len {pkt_len:#06x}")]
    ReservedLength { pkt_len: u16 },
    /// Encoded data payload exceeds 65516 bytes (gitprotocol-common).
    #[error("payload length {len} exceeds maximum {max}")]
    PayloadTooLarge { len: usize, max: usize },
    /// Decoded `pkt-len` exceeds 65520 (gitprotocol-common).
    #[error("pkt-len {pkt_len} exceeds maximum {max}")]
    LineTooLong { pkt_len: usize, max: usize },
    /// Helpers do not emit an empty data pkt-line (`0004`).
    ///
    /// gitprotocol-common: implementations SHOULD NOT send `"0004"`.
    /// Parsers still accept it as an empty data payload.
    #[error("refusing to encode empty data pkt-line 0004")]
    EmptyDataPkt,
    /// Text helper received a payload that is not UTF-8.
    #[error("text pkt-line payload is not valid UTF-8")]
    TextUtf8,
    /// v1 capability trailer after NUL is not UTF-8.
    #[error("v1 capability trailer is not valid UTF-8")]
    CapabilityUtf8,
    /// v2 capability advertisement did not start with `version 2`.
    #[error("v2 capability advertisement missing protocol version")]
    MissingProtocolVersion,
    /// v2 capability advertisement did not end with flush-pkt.
    #[error("v2 capability advertisement missing flush-pkt")]
    MissingFlush,
    /// delim-pkt or response-end-pkt appeared inside a v2 capability advertisement.
    ///
    /// gitprotocol-v2 `capability-advertisement` is version, capability lines, then flush-pkt.
    #[error("unexpected special pkt-line in v2 capability advertisement")]
    UnexpectedSpecial,
}
