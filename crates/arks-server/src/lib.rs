//! arks-server — Git server role (M11 / M25 / Phase Sv + Sv2).
//! Claimed: smart HTTP (G15), dumb/SSH/v2/git:// (G29–G32), additive WebDAV
//! write for dumb HTTP push (R2), optional HTTP Basic (G36),
//! receive-pack githooks honor list (pre-receive/update/post-receive).
//! Trimmed: in-process `info/lfs` batch/locks server (not stock git-http-backend).
#![forbid(unsafe_code)]
mod advertise;
mod auth;
mod daemon;
mod dav;
mod dumb;
mod error;
mod hooks;
mod http;
mod receive;
mod serve;
mod ssh_serve;
mod stdio;
mod upload;
mod v2;
mod walk;
pub use auth::{
    BASIC_REALM, BasicAuthConfig, BasicCredential, base64_encode, basic_authorization_header,
};
pub use daemon::{DaemonOptions, serve_daemon, serve_daemon_inetd, spawn_loopback_daemon};
pub use dumb::{dumb_info_refs_body, serve_dumb_path, update_server_info};
pub use error::ServerError;
pub use http::{HttpRequest, HttpResponse, handle_smart_http, handle_smart_http_auth};
pub use receive::{receive_advertisement, receive_pack, receive_pack_advertisement};
pub use serve::{
    ListenRebindSlot, ServeHttpOptions, serve_http, serve_http_with_opts, spawn_loopback,
    spawn_loopback_with_opts,
};
pub use ssh_serve::{
    ServeSshOptions, default_authorized_keys_path, default_host_key_path, serve_ssh_listener,
    spawn_loopback_ssh,
};
pub use stdio::{
    receive_pack_session, receive_pack_stream, upload_pack_session, upload_pack_stream,
};
pub use upload::{upload_advertisement, upload_pack_advertisement, upload_pack_result};
pub use v2::{handle_v2_request, v2_capability_advertisement, v2_capability_lines};
#[cfg(test)]
mod tests;
