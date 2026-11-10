//! Smart + dumb HTTP request dispatch (info/refs + service POST + static dumb GETs)
//! plus additive WebDAV write for dumb HTTP push (R2).
//!
//! Auth: unauthenticated loopback when Basic unset (G15). Optional in-process HTTP
//! Basic when configured (G36 / ADR 20261109224236). Forge OAuth refused.
//! Ops reverse-proxy / external TLS MAY still front the listener.
//!
//! Cite: gitprotocol-http, gitprotocol-v2, RFC 7617 Basic (summarize),
//! RFC 4918 / git-http-push (dumb push).

use crate::auth::BasicAuthConfig;
use crate::dav::{handle_dav, is_dav_method};
use crate::dumb::serve_dumb_path;
use crate::error::ServerError;
use crate::receive::{receive_advertisement, receive_pack};
use crate::upload::{upload_advertisement, upload_pack_result};
use crate::v2::{handle_v2_request, v2_capability_advertisement};
use arks_repo::Repository;

const UPLOAD_AD: &str = "application/x-git-upload-pack-advertisement";
const UPLOAD_RES: &str = "application/x-git-upload-pack-result";
const RECEIVE_AD: &str = "application/x-git-receive-pack-advertisement";
const RECEIVE_RES: &str = "application/x-git-receive-pack-result";

#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: String,
    pub path: String,
    pub query: String,
    pub body: Vec<u8>,
    /// Raw `Git-Protocol` header value (e.g. `version=2`), if present.
    pub git_protocol: Option<String>,
    /// Raw `Authorization` header value (e.g. `Basic …`), if present.
    pub authorization: Option<String>,
    /// WebDAV `Destination` header (MOVE).
    pub destination: Option<String>,
    /// WebDAV lock token from `Lock-Token` or `If` header.
    pub lock_token: Option<String>,
    /// Raw `Host` header (for LFS batch href construction).
    pub host: Option<String>,
}

impl HttpRequest {
    pub fn new(
        method: impl Into<String>,
        path: impl Into<String>,
        query: impl Into<String>,
        body: Vec<u8>,
    ) -> Self {
        Self {
            method: method.into(),
            path: path.into(),
            query: query.into(),
            body,
            git_protocol: None,
            authorization: None,
            destination: None,
            lock_token: None,
            host: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub content_type: String,
    pub body: Vec<u8>,
    /// Set on 401 challenges (`Basic realm="arks"`).
    pub www_authenticate: Option<String>,
    /// Extra response headers (e.g. WebDAV `Lock-Token`).
    pub extra_headers: Vec<(String, String)>,
}

/// Handle one HTTP request against an open repository (smart + dumb + v2).
pub fn handle_smart_http(
    repo: &mut Repository,
    req: &HttpRequest,
) -> Result<HttpResponse, ServerError> {
    handle_smart_http_auth(repo, req, None)
}

/// Like [`handle_smart_http`], with optional Basic auth gate.
pub fn handle_smart_http_auth(
    repo: &mut Repository,
    req: &HttpRequest,
    basic: Option<&BasicAuthConfig>,
) -> Result<HttpResponse, ServerError> {
    if let Some(cfg) = basic {
        if !cfg.check_authorization(req.authorization.as_deref()) {
            return Ok(HttpResponse {
                status: 401,
                content_type: "text/plain".into(),
                body: b"unauthorized\n".to_vec(),
                www_authenticate: Some(cfg.challenge_header()),
                extra_headers: Vec::new(),
            });
        }
    }
    // LFS server productization (`info/lfs`) trimmed — keep plain git HTTP only.
    if is_dav_method(&req.method) {
        return handle_dav(repo, req);
    }
    let path = req.path.as_str();
    let wants_v2 = req
        .git_protocol
        .as_deref()
        .map(|v| {
            v.split(':')
                .any(|p| p.trim() == "version=2" || p.trim().starts_with("version=2"))
        })
        .unwrap_or(false)
        || req
            .git_protocol
            .as_deref()
            .map(|v| v.contains("version=2"))
            .unwrap_or(false);

    if path.ends_with("/info/refs") || path == "/info/refs" || path.ends_with("info/refs") {
        let service = query_param(&req.query, "service").unwrap_or("");
        if service.is_empty() {
            // Dumb HTTP discovery (gitprotocol-http dumb clients).
            return match serve_dumb_path(repo, "info/refs")? {
                Some((ct, body)) => Ok(typed(200, &ct, body)),
                None => Ok(text(404, b"not found")),
            };
        }
        if wants_v2 && service == "git-upload-pack" {
            // HTTP info/refs keeps `# service=` + flush, then v2 capability ad
            // (gitprotocol-http first-bytes `#` check + gitprotocol-v2).
            let mut body = arks_protocol::encode_text_line("# service=git-upload-pack")
                .map_err(|e| ServerError::Protocol(e.to_string()))?;
            body.extend(
                arks_protocol::encode_packet(&arks_protocol::Packet::Flush)
                    .map_err(|e| ServerError::Protocol(e.to_string()))?,
            );
            body.extend(v2_capability_advertisement()?);
            return Ok(typed(200, UPLOAD_AD, body));
        }
        // receive-pack stays v1 (push over v2 not claimed).
        return match service {
            "git-upload-pack" => Ok(typed(200, UPLOAD_AD, upload_advertisement(repo)?)),
            "git-receive-pack" => Ok(typed(200, RECEIVE_AD, receive_advertisement(repo)?)),
            _ => Ok(text(403, b"unknown service")),
        };
    }
    if req.method.eq_ignore_ascii_case("POST")
        && (path.ends_with("/git-upload-pack") || path.ends_with("git-upload-pack"))
    {
        if looks_like_v2_command(&req.body) {
            return Ok(typed(
                200,
                UPLOAD_RES,
                handle_v2_request(repo, &req.body, true)?,
            ));
        }
        return Ok(typed(200, UPLOAD_RES, upload_pack_result(repo, &req.body)?));
    }
    if req.method.eq_ignore_ascii_case("POST")
        && (path.ends_with("/git-receive-pack") || path.ends_with("git-receive-pack"))
    {
        return Ok(typed(200, RECEIVE_RES, receive_pack(repo, &req.body)?));
    }
    // Dumb static GETs: objects/…, objects/info/packs, …
    if req.method.eq_ignore_ascii_case("GET") || req.method.eq_ignore_ascii_case("HEAD") {
        let rel = path.trim_start_matches('/');
        if let Some((ct, body)) = serve_dumb_path(repo, rel)? {
            return Ok(typed(200, &ct, body));
        }
        // Also try stripping a leading repo path segment before objects/info.
        if let Some(idx) = rel.find("objects/") {
            if let Some((ct, body)) = serve_dumb_path(repo, &rel[idx..])? {
                return Ok(typed(200, &ct, body));
            }
        }
        if let Some(idx) = rel.find("info/refs") {
            if let Some((ct, body)) = serve_dumb_path(repo, &rel[idx..])? {
                return Ok(typed(200, &ct, body));
            }
        }
    }
    Ok(text(404, b"not found"))
}

fn looks_like_v2_command(body: &[u8]) -> bool {
    // v2 command requests start with pkt-line "command=…"
    body.len() >= 4 && {
        let s = String::from_utf8_lossy(body);
        s.contains("command=")
    }
}

fn query_param<'a>(query: &'a str, key: &str) -> Option<&'a str> {
    for part in query.split('&') {
        if let Some((k, v)) = part.split_once('=') {
            if k == key {
                return Some(v);
            }
        }
    }
    None
}

fn typed(status: u16, ct: &str, body: Vec<u8>) -> HttpResponse {
    HttpResponse {
        status,
        content_type: ct.into(),
        body,
        www_authenticate: None,
        extra_headers: Vec::new(),
    }
}

fn text(status: u16, body: &[u8]) -> HttpResponse {
    HttpResponse {
        status,
        content_type: "text/plain".into(),
        body: body.to_vec(),
        www_authenticate: None,
        extra_headers: Vec::new(),
    }
}
