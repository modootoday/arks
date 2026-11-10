//! arks-server — Git server role (M11 / M25 / Phase Sv + Sv2).
//! Claimed: smart HTTP (G15), dumb/SSH/v2/git:// (G29–G32), optional HTTP Basic (G36),
//! receive-pack githooks honor list (pre-receive/update/post-receive).
#![forbid(unsafe_code)]
mod advertise; mod auth; mod daemon; mod dumb; mod error; mod hooks; mod http;
mod receive; mod serve; mod stdio; mod upload; mod v2; mod walk;
pub use auth::{basic_authorization_header, base64_encode, BasicAuthConfig, BasicCredential, BASIC_REALM};
pub use daemon::{serve_daemon, spawn_loopback_daemon, DaemonOptions};
pub use dumb::{dumb_info_refs_body, serve_dumb_path, update_server_info};
pub use error::ServerError;
pub use http::{handle_smart_http, handle_smart_http_auth, HttpRequest, HttpResponse};
pub use receive::{receive_advertisement, receive_pack, receive_pack_advertisement};
pub use serve::{serve_http, serve_http_with_opts, spawn_loopback, spawn_loopback_with_opts, ServeHttpOptions};
pub use stdio::{receive_pack_session, receive_pack_stream, upload_pack_session, upload_pack_stream};
pub use upload::{upload_advertisement, upload_pack_advertisement, upload_pack_result};
pub use v2::{handle_v2_request, v2_capability_advertisement, v2_capability_lines};
#[cfg(test)]
mod tests;
