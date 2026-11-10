//! Loopback smart-HTTP listener for clean-room / local serve.

use crate::auth::BasicAuthConfig;
use crate::error::ServerError;
use crate::http::{handle_smart_http_auth, HttpRequest};
use arks_repo::Repository;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

#[derive(Debug, Clone, Default)]
pub struct ServeHttpOptions {
    /// When true, answer `info/refs` (even with `service=`) as dumb text/plain so
    /// smart clients fall back to dumb HTTP (gitprotocol-http). Used by G29.
    pub dumb_only: bool,
    /// Optional in-process HTTP Basic (G36). Unset = unauthenticated (G15).
    pub basic_auth: Option<BasicAuthConfig>,
}

/// Bind `addr` and serve `git_dir` over smart HTTP until `stop` is set or accept fails.
pub fn serve_http(
    git_dir: impl AsRef<Path>,
    addr: SocketAddr,
    stop: Arc<AtomicBool>,
) -> Result<(), ServerError> {
    serve_http_with_opts(git_dir, addr, stop, ServeHttpOptions::default())
}

pub fn serve_http_with_opts(
    git_dir: impl AsRef<Path>,
    addr: SocketAddr,
    stop: Arc<AtomicBool>,
    opts: ServeHttpOptions,
) -> Result<(), ServerError> {
    let git_dir = git_dir.as_ref().to_path_buf();
    let listener = TcpListener::bind(addr)?;
    listener.set_nonblocking(true)?;
    while !stop.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((stream, _)) => {
                let dir = git_dir.clone();
                let opts = opts.clone();
                thread::spawn(move || {
                    let _ = handle_connection(stream, &dir, &opts);
                });
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(std::time::Duration::from_millis(20));
            }
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}

/// Bind an ephemeral port; return (bound addr, join handle, stop flag).
pub fn spawn_loopback(git_dir: PathBuf) -> Result<(SocketAddr, Arc<AtomicBool>, thread::JoinHandle<Result<(), ServerError>>), ServerError> {
    spawn_loopback_with_opts(git_dir, ServeHttpOptions::default())
}

pub fn spawn_loopback_with_opts(
    git_dir: PathBuf,
    opts: ServeHttpOptions,
) -> Result<(SocketAddr, Arc<AtomicBool>, thread::JoinHandle<Result<(), ServerError>>), ServerError> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let addr = listener.local_addr()?;
    listener.set_nonblocking(true)?;
    let stop = Arc::new(AtomicBool::new(false));
    let stop2 = stop.clone();
    let handle = thread::spawn(move || {
        while !stop2.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((stream, _)) => {
                    let dir = git_dir.clone();
                    let opts = opts.clone();
                    thread::spawn(move || {
                        let _ = handle_connection(stream, &dir, &opts);
                    });
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(std::time::Duration::from_millis(20));
                }
                Err(e) => return Err(e.into()),
            }
        }
        Ok(())
    });
    Ok((addr, stop, handle))
}

fn handle_connection(
    mut stream: TcpStream,
    git_dir: &Path,
    opts: &ServeHttpOptions,
) -> Result<(), ServerError> {
    // Listener is nonblocking; accepted sockets may inherit that on Windows.
    stream.set_nonblocking(false)?;
    stream.set_read_timeout(Some(std::time::Duration::from_secs(30)))?;
    stream.set_write_timeout(Some(std::time::Duration::from_secs(30)))?;
    let mut buf = Vec::new();
    let mut tmp = [0u8; 8192];
    // Read headers
    loop {
        let n = stream.read(&mut tmp)?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&tmp[..n]);
        if buf.windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }
        if buf.len() > 1024 * 1024 {
            return Err(ServerError::Protocol("headers too large".into()));
        }
    }
    let header_end = buf
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or_else(|| ServerError::Protocol("incomplete headers".into()))?;
    let header_bytes = &buf[..header_end];
    let header_text = std::str::from_utf8(header_bytes)
        .map_err(|_| ServerError::Protocol("headers not utf-8".into()))?;
    let mut lines = header_text.split("\r\n");
    let request_line = lines
        .next()
        .ok_or_else(|| ServerError::Protocol("missing request line".into()))?;
    let mut parts = request_line.split_whitespace();
    let method = parts
        .next()
        .ok_or_else(|| ServerError::Protocol("bad request line".into()))?
        .to_string();
    let target = parts
        .next()
        .ok_or_else(|| ServerError::Protocol("bad request line".into()))?;
    let (path, query) = match target.split_once('?') {
        Some((p, q)) => (p.to_string(), q.to_string()),
        None => (target.to_string(), String::new()),
    };
    let mut content_length = 0usize;
    let mut expect_continue = false;
    let mut content_encoding = String::new();
    let mut git_protocol = None;
    let mut authorization = None;
    for line in lines {
        let lower = line.to_ascii_lowercase();
        if let Some(v) = lower.strip_prefix("content-length:") {
            content_length = v.trim().parse().unwrap_or(0);
        }
        if lower.starts_with("expect:") && lower.contains("100-continue") {
            // git/curl POST often waits for 100 before sending the pack body
            // (gitprotocol-http / HTTP/1.1). Without this, clients see empty replies.
            expect_continue = true;
        }
        if let Some(v) = lower.strip_prefix("content-encoding:") {
            content_encoding = v.trim().to_string();
        }
        // gitprotocol-v2: clients send `Git-Protocol: version=2`.
        if lower.starts_with("git-protocol:") {
            if let Some((_, v)) = line.split_once(':') {
                git_protocol = Some(v.trim().to_string());
            }
        }
        if lower.starts_with("authorization:") {
            if let Some((_, v)) = line.split_once(':') {
                authorization = Some(v.trim().to_string());
            }
        }
    }
    if expect_continue && content_length > 0 {
        stream.write_all(b"HTTP/1.1 100 Continue\r\n\r\n")?;
        stream.flush()?;
    }
    let mut body = buf[header_end + 4..].to_vec();
    while body.len() < content_length {
        let n = stream.read(&mut tmp)?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&tmp[..n]);
    }
    body.truncate(content_length);
    // Stock git/curl may gzip the upload-pack / receive-pack POST once the prior
    // advertisement is large (observed ~32+ refs). Accept Content-Encoding: gzip
    // and gzip magic even if the header is omitted.
    let gzip_magic = body.len() >= 2 && body[0] == 0x1f && body[1] == 0x8b;
    if content_encoding.contains("gzip") || gzip_magic {
        use flate2::read::GzDecoder;
        use std::io::Read as _;
        let mut decoder = GzDecoder::new(body.as_slice());
        let mut decoded = Vec::new();
        decoder
            .read_to_end(&mut decoded)
            .map_err(|e| ServerError::Protocol(format!("gzip body: {e}")))?;
        body = decoded;
    }

    let mut repo = Repository::open(git_dir).map_err(|e| ServerError::Msg(e.to_string()))?;
    let mut req = HttpRequest {
        method,
        path: path.clone(),
        query: query.clone(),
        body,
        git_protocol,
        authorization,
    };
    // dumb_only: strip service= so discovery uses dumb info/refs (smart clients fall back).
    if opts.dumb_only
        && (req.path.ends_with("info/refs") || req.path.ends_with("/info/refs"))
    {
        req.query = String::new();
    }
    let resp = match handle_smart_http_auth(&mut repo, &req, opts.basic_auth.as_ref()) {
        Ok(r) => r,
        Err(e) => {
            // Fail closed with an HTTP error body (never silent empty close).
            let msg = e.to_string();
            let hex: String = req
                .body
                .iter()
                .take(32)
                .map(|b| format!("{b:02x}"))
                .collect::<Vec<_>>()
                .join(" ");
            eprintln!(
                "arks serve-http error path={path} query={query} body_len={} hex32=[{hex}] : {msg}",
                req.body.len()
            );
            crate::http::HttpResponse {
                status: 500,
                content_type: "text/plain".into(),
                body: msg.into_bytes(),
                www_authenticate: None,
            }
        }
    };
    let status_text = match resp.status {
        200 => "OK",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        500 => "Internal Server Error",
        _ => "Error",
    };
    let mut header = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n",
        resp.status,
        status_text,
        resp.content_type,
        resp.body.len()
    );
    if let Some(ref wa) = resp.www_authenticate {
        header.push_str(&format!("WWW-Authenticate: {wa}\r\n"));
    }
    header.push_str("\r\n");
    stream.write_all(header.as_bytes())?;
    stream.write_all(&resp.body)?;
    stream.flush()?;
    Ok(())
}
