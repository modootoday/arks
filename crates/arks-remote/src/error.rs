//! Client errors for the claimed smart HTTP subset.

/// Failure while speaking the claimed remote-client subset.
#[derive(Debug, thiserror::Error)]
pub enum RemoteError {
    /// Caller asked for a behavior this slice does not implement.
    #[error("{0}")]
    NotImplemented(String),
    /// URL scheme is outside the claimed `http://` / `https://` subset.
    #[error("{0}")]
    UnsupportedUrl(String),
    /// `.git/config` remote section could not be read or updated.
    #[error("remote config: {0}")]
    Config(String),
    /// HTTP transport failure or non-success status.
    #[error("http: {0}")]
    Http(String),
    /// Smart HTTP / pkt-line / pack response was not a claimed shape.
    #[error("protocol: {0}")]
    Protocol(String),
    /// Push `report-status` rejected a ref or unpack.
    #[error("push rejected: {0}")]
    Rejected(String),
    /// Object, ref, or work-tree failure with context.
    #[error("{0}")]
    Msg(String),
}
