//! Server errors.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ServerError {
    #[error("{0}")]
    Msg(String),
    #[error("protocol: {0}")]
    Protocol(String),
    #[error("not implemented: {0}")]
    NotImplemented(String),
    #[error("io: {0}")]
    Io(String),
}

impl From<std::io::Error> for ServerError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e.to_string())
    }
}
