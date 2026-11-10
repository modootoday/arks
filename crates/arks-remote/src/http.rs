//! HTTP client for claimed smart HTTP (`http://` cleartext and `https://` via rustls).
//!
//! `ureq` is the TLS-capable client for `https://`. Cleartext `http://` uses a
//! per-request TcpStream so J-CLONE-LOCAL (loopback CGI) never pays Agent/rustls
//! init. No keep-alive: git http-backend replies `Connection: close`, and reusing
//! the socket caused `cleartext EOF before headers` on the upload-pack round-trip.
//!
//! Protocol bytes stay in this crate; the library only moves request and response bodies.
//!
//! On HTTP 401 without an Authorization header, fill credentials via helper/askpass
//! and retry once ([[sot:root/credential-helper-askpass]]).
//!
//! Pf5: defer `CredentialSource::discover` until a 401 (or explicit `with_source`) so
//! unauthenticated smart-HTTP clone/fetch skips `.gitconfig` / helper walks.

use crate::credential::{CredentialSource, resolve_authorization};
use crate::error::RemoteError;
use crate::transport::{HttpRequest, HttpResponse, Transport};
use crate::url::prepare_url_with_source;
use std::io::{IoSlice, Read, Write};
use std::net::TcpStream;
use std::time::Duration;

pub struct UreqTransport {
    /// Built lazily on first `https://` exchange only.
    agent: Option<ureq::Agent>,
    source: CredentialSource,
    discovered: bool,
}

impl UreqTransport {
    pub fn new() -> Self {
        // Lazy discover: empty until 401 forces a fill (J-CLONE-LOCAL hot path).
        Self::with_source_lazy(CredentialSource::empty(), false)
    }

    /// Inject a pre-built credential source (tests / callers that already discovered).
    #[allow(dead_code)] // intentional API for injected CredentialSource (tests / auth)
    pub fn with_source(source: CredentialSource) -> Self {
        Self::with_source_lazy(source, true)
    }

    fn with_source_lazy(source: CredentialSource, discovered: bool) -> Self {
        Self {
            agent: None,
            source,
            discovered,
        }
    }

    fn ensure_discovered(&mut self) {
        if self.discovered {
            return;
        }
        self.source = CredentialSource::discover(&[]);
        self.discovered = true;
    }

    fn https_agent(&mut self) -> &ureq::Agent {
        self.agent.get_or_insert_with(|| {
            ureq::AgentBuilder::new()
                .redirects(0)
                .timeout_connect(Duration::from_secs(15))
                .timeout_read(Duration::from_secs(120))
                .timeout_write(Duration::from_secs(120))
                .user_agent("arks/0.0.0")
                .build()
        })
    }
}

impl Default for UreqTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl Transport for UreqTransport {
    fn exchange(&mut self, request: &HttpRequest) -> Result<HttpResponse, RemoteError> {
        let prepared = prepare_url_with_source(&request.url, &self.source)?;
        match self.exchange_once(request, &prepared.url, prepared.authorization.as_deref()) {
            Ok(response) => Ok(response),
            Err(RemoteError::Http(msg))
                if msg.starts_with("HTTP 401") && prepared.authorization.is_none() =>
            {
                self.ensure_discovered();
                let auth = resolve_authorization(&request.url, &self.source)?;
                if auth.is_none() {
                    return Err(RemoteError::Http(msg));
                }
                self.exchange_once(request, &prepared.url, auth.as_deref())
            }
            Err(err) => Err(err),
        }
    }
}

impl UreqTransport {
    fn exchange_once(
        &mut self,
        request: &HttpRequest,
        url: &str,
        authorization: Option<&str>,
    ) -> Result<HttpResponse, RemoteError> {
        if url.starts_with("http://") {
            return exchange_cleartext(request, url, authorization);
        }
        let agent = self.https_agent();
        let mut call = match request.method.as_str() {
            "GET" => agent.get(url),
            "POST" => agent.post(url),
            other => {
                return Err(RemoteError::Http(format!(
                    "unsupported HTTP method {other}"
                )));
            }
        };
        for (name, value) in &request.headers {
            call = call.set(name, value);
        }
        if let Some(auth) = authorization {
            call = call.set("Authorization", auth);
        }
        let result = if request.method == "POST" {
            call.send_bytes(&request.body)
        } else {
            call.call()
        };
        match result {
            Ok(response) => read_ureq_response(response),
            Err(ureq::Error::Status(code, response)) => {
                let mut snippet = String::new();
                let _ = response
                    .into_reader()
                    .take(512)
                    .read_to_string(&mut snippet);
                Err(RemoteError::Http(format!("HTTP {code}: {snippet}")))
            }
            Err(err) => Err(RemoteError::Http(err.to_string())),
        }
    }
}

/// One TCP connect per request — CGI peers close after each response.
fn exchange_cleartext(
    request: &HttpRequest,
    url: &str,
    authorization: Option<&str>,
) -> Result<HttpResponse, RemoteError> {
    let (host, port, path_query) = split_http_url(url)?;
    let mut stream = TcpStream::connect((host.as_str(), port))
        .map_err(|e| RemoteError::Http(format!("connect {host}:{port}: {e}")))?;
    stream
        .set_nodelay(true)
        .map_err(|e| RemoteError::Http(e.to_string()))?;
    tune_cleartext_socket(&stream);
    let _ = stream.set_read_timeout(Some(Duration::from_secs(120)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(120)));
    write_http_request(
        &mut stream,
        request,
        &path_query,
        &host,
        port,
        authorization,
    )?;
    read_http_response(&mut stream)
}

/// Loopback CGI (Linux clean-room): larger RCVBUF absorbs upload-pack in fewer
/// reads; QUICKACK cuts delayed-ACK stalls on info/refs → upload-pack RTs.
#[cfg(target_os = "linux")]
#[allow(unsafe_code)] // setsockopt for SOL_SOCKET / TCP_QUICKACK only.
fn tune_cleartext_socket(stream: &TcpStream) {
    use std::os::fd::AsRawFd;
    let fd = stream.as_raw_fd();
    let buf: libc::c_int = 512 * 1024;
    let on: libc::c_int = 1;
    unsafe {
        libc::setsockopt(
            fd,
            libc::SOL_SOCKET,
            libc::SO_RCVBUF,
            (&buf as *const libc::c_int).cast(),
            std::mem::size_of_val(&buf) as libc::socklen_t,
        );
        libc::setsockopt(
            fd,
            libc::SOL_SOCKET,
            libc::SO_SNDBUF,
            (&buf as *const libc::c_int).cast(),
            std::mem::size_of_val(&buf) as libc::socklen_t,
        );
        libc::setsockopt(
            fd,
            libc::IPPROTO_TCP,
            libc::TCP_QUICKACK,
            (&on as *const libc::c_int).cast(),
            std::mem::size_of_val(&on) as libc::socklen_t,
        );
    }
}

#[cfg(not(target_os = "linux"))]
fn tune_cleartext_socket(_stream: &TcpStream) {}

fn split_http_url(url: &str) -> Result<(String, u16, String), RemoteError> {
    let rest = url
        .strip_prefix("http://")
        .ok_or_else(|| RemoteError::Http(format!("not http:// URL: {url}")))?;
    let (authority, path) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, "/"),
    };
    if authority.is_empty() {
        return Err(RemoteError::Http(format!("missing host in {url}")));
    }
    let (host, port) = if let Some((h, p)) = authority.rsplit_once(':') {
        if h.contains(':') {
            return Err(RemoteError::Http(format!("unsupported host in {url}")));
        }
        let port: u16 = p
            .parse()
            .map_err(|_| RemoteError::Http(format!("bad port in {url}")))?;
        (h.to_string(), port)
    } else {
        (authority.to_string(), 80)
    };
    let path_query = if path.is_empty() {
        "/".to_string()
    } else {
        path.to_string()
    };
    Ok((host, port, path_query))
}

fn write_http_request(
    stream: &mut TcpStream,
    request: &HttpRequest,
    path_query: &str,
    host: &str,
    port: u16,
    authorization: Option<&str>,
) -> Result<(), RemoteError> {
    // Headers only — do not copy the POST body into the header buffer.
    let mut hdr = Vec::with_capacity(256);
    hdr.extend_from_slice(request.method.as_bytes());
    hdr.push(b' ');
    hdr.extend_from_slice(path_query.as_bytes());
    hdr.extend_from_slice(b" HTTP/1.1\r\nHost: ");
    hdr.extend_from_slice(host.as_bytes());
    if port != 80 {
        hdr.push(b':');
        let mut port_buf = [0u8; 5];
        hdr.extend_from_slice(write_decimal_u16(port, &mut port_buf));
    }
    // Explicit close: matches git http-backend CGI; avoids half-closed reuse.
    hdr.extend_from_slice(b"\r\nUser-Agent: arks/0.0.0\r\nConnection: close\r\n");
    for (name, value) in &request.headers {
        hdr.extend_from_slice(name.as_bytes());
        hdr.extend_from_slice(b": ");
        hdr.extend_from_slice(value.as_bytes());
        hdr.extend_from_slice(b"\r\n");
    }
    if let Some(auth) = authorization {
        hdr.extend_from_slice(b"Authorization: ");
        hdr.extend_from_slice(auth.as_bytes());
        hdr.extend_from_slice(b"\r\n");
    }
    if request.method == "POST" || !request.body.is_empty() {
        hdr.extend_from_slice(b"Content-Length: ");
        let mut len_buf = [0u8; 20];
        hdr.extend_from_slice(write_decimal_usize(request.body.len(), &mut len_buf));
        hdr.extend_from_slice(b"\r\n");
    }
    hdr.extend_from_slice(b"\r\n");
    if request.body.is_empty() {
        stream
            .write_all(&hdr)
            .map_err(|e| RemoteError::Http(e.to_string()))?;
        return Ok(());
    }
    // One syscall for headers+POST body when the kernel accepts both (loopback).
    let bufs = [IoSlice::new(&hdr), IoSlice::new(&request.body)];
    match stream.write_vectored(&bufs) {
        Ok(n) if n == hdr.len() + request.body.len() => Ok(()),
        Ok(n) if n >= hdr.len() => stream
            .write_all(&request.body[n - hdr.len()..])
            .map_err(|e| RemoteError::Http(e.to_string())),
        Ok(n) => {
            stream
                .write_all(&hdr[n..])
                .map_err(|e| RemoteError::Http(e.to_string()))?;
            stream
                .write_all(&request.body)
                .map_err(|e| RemoteError::Http(e.to_string()))
        }
        Err(e) => Err(RemoteError::Http(e.to_string())),
    }
}

fn write_decimal_u16(mut n: u16, buf: &mut [u8; 5]) -> &[u8] {
    if n == 0 {
        buf[4] = b'0';
        return &buf[4..];
    }
    let mut i = 5;
    while n > 0 {
        i -= 1;
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
    }
    &buf[i..]
}

fn write_decimal_usize(mut n: usize, buf: &mut [u8; 20]) -> &[u8] {
    if n == 0 {
        buf[19] = b'0';
        return &buf[19..];
    }
    let mut i = 20;
    while n > 0 {
        i -= 1;
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
    }
    &buf[i..]
}

fn read_http_response(stream: &mut TcpStream) -> Result<HttpResponse, RemoteError> {
    // Headers: 8KiB stack (info/refs stays cheap). Large Content-Length bodies
    // (upload-pack PACK) switch to one 64KiB heap scratch — fewer syscalls
    // without zero-taxing the small advertisement round-trip.
    let mut tmp = [0u8; 8192];
    let mut buf = Vec::with_capacity(8192);
    let header_end;
    loop {
        let n = stream
            .read(&mut tmp)
            .map_err(|e| RemoteError::Http(e.to_string()))?;
        if n == 0 {
            return Err(RemoteError::Http("cleartext EOF before headers".into()));
        }
        buf.extend_from_slice(&tmp[..n]);
        if let Some(pos) = find_header_end(&buf) {
            header_end = pos;
            break;
        }
        if buf.len() > 1024 * 1024 {
            return Err(RemoteError::Http("cleartext headers too large".into()));
        }
    }
    let (status, content_length, content_type) = parse_cleartext_headers(&buf[..header_end])?;
    // Build the body Vec directly — Vec::split_off allocates+copies the whole
    // upload-pack PACK (multi-MB on J-CLONE-LOCAL).
    let body = if let Some(len) = content_length {
        let mut body = Vec::with_capacity(len);
        let already = &buf[header_end..];
        let take = already.len().min(len);
        body.extend_from_slice(&already[..take]);
        if len > 16 * 1024 {
            let mut big = vec![0u8; 64 * 1024];
            while body.len() < len {
                let n = stream
                    .read(&mut big)
                    .map_err(|e| RemoteError::Http(e.to_string()))?;
                if n == 0 {
                    break;
                }
                let room = len - body.len();
                body.extend_from_slice(&big[..n.min(room)]);
            }
        } else {
            while body.len() < len {
                let n = stream
                    .read(&mut tmp)
                    .map_err(|e| RemoteError::Http(e.to_string()))?;
                if n == 0 {
                    break;
                }
                let room = len - body.len();
                body.extend_from_slice(&tmp[..n.min(room)]);
            }
        }
        body.truncate(len);
        body
    } else {
        let mut body = buf.split_off(header_end);
        loop {
            let n = stream
                .read(&mut tmp)
                .map_err(|e| RemoteError::Http(e.to_string()))?;
            if n == 0 {
                break;
            }
            body.extend_from_slice(&tmp[..n]);
        }
        body
    };
    if !(200..300).contains(&status) {
        let snippet = String::from_utf8_lossy(&body[..body.len().min(512)]);
        return Err(RemoteError::Http(format!("HTTP {status}: {snippet}")));
    }
    let mut headers = Vec::new();
    if let Some(ct) = content_type {
        headers.push(("content-type".into(), ct));
    }
    Ok(HttpResponse {
        status,
        headers,
        body,
    })
}

fn parse_cleartext_headers(
    header_bytes: &[u8],
) -> Result<(u16, Option<usize>, Option<String>), RemoteError> {
    let header_text = String::from_utf8_lossy(header_bytes);
    let mut lines = header_text.split("\r\n");
    let status_line = lines.next().unwrap_or("");
    let status = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(0);
    let mut content_length = None;
    let mut content_type = None;
    for line in lines {
        if line.is_empty() {
            continue;
        }
        let (name, value) = match line.split_once(':') {
            Some(pair) => pair,
            None => continue,
        };
        if name.eq_ignore_ascii_case("content-length") {
            content_length = value.trim().parse().ok();
        } else if name.eq_ignore_ascii_case("content-type") {
            content_type = Some(value.trim().to_string());
        }
    }
    Ok((status, content_length, content_type))
}

fn find_header_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n").map(|i| i + 4)
}

fn read_ureq_response(response: ureq::Response) -> Result<HttpResponse, RemoteError> {
    let status = response.status();
    let mut headers = Vec::new();
    if let Some(ct) = response.header("content-type") {
        headers.push(("content-type".into(), ct.to_string()));
    }
    let mut body = Vec::new();
    response
        .into_reader()
        .read_to_end(&mut body)
        .map_err(|e| RemoteError::Http(e.to_string()))?;
    Ok(HttpResponse {
        status,
        headers,
        body,
    })
}
