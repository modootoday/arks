//! `git://` daemon (`arks daemon`, peer of `git daemon`).
//!
//! Primary: <https://git-scm.com/docs/git-daemon>,
//! <https://git-scm.com/docs/gitprotocol-pack> (Git Transport initial request).
//!
//! Module map: [`options`] (option model), [`request`] (initial request),
//! [`path`] (path resolution and whitelist), [`session`] (per-request checks
//! and services), [`access_hook`], [`log`] (stderr / syslog / none),
//! [`listen`] (sockets, accept loop, `--inetd`), [`platform`] (OS lookups),
//! [`archive`] (upload-archive body).
//! Socket binding, privilege drop and detaching are driven by the CLI.
//! Never spawns `git` or `gh`.

mod access_hook;
mod archive;
mod listen;
mod log;
mod options;
mod path;
mod platform;
mod request;
mod session;

pub use listen::{
    DEFAULT_DAEMON_PORT, DaemonListeners, bind_daemon, serve_daemon, serve_daemon_inetd,
    spawn_loopback_daemon,
};
pub use log::{DaemonLog, LogDestination, SYSLOG_IDENT, SYSLOG_SOCKET};
pub use options::{DaemonOptions, DaemonService, ServicePolicy, ServiceTable};
pub use platform::{DaemonPlatform, HostInfo, StdPlatform};
pub use session::Outcome;

use crate::error::ServerError;
use arks_protocol::{Packet, encode_packet};
use std::io::Write;

use request::parse_pkt_len;

/// `ERR <msg>` pkt-line used inside the upload-archive body.
fn write_err_pkt<W: Write>(stream: &mut W, msg: &str) -> Result<(), ServerError> {
    let line = format!("ERR {msg}\n");
    let pkt = encode_packet(&Packet::Data(line.into_bytes()))
        .map_err(|e| ServerError::Protocol(e.to_string()))?;
    stream.write_all(&pkt)?;
    stream.flush()?;
    Ok(())
}
