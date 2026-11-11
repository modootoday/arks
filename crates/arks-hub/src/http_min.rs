//! Minimal HTTP/1.1 helpers for hub serve/client (Partial).
//!
//! - `http://` loopback: raw `TcpStream` (local forge).
//! - `https://` remote: `ureq` TLS path; operator CA via `ARKS_HUB_CA_FILE` (B3 harness).
//! Never spawns `gh` / `git`.

use rustls::pki_types::CertificateDer;
use std::env;
use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug)]
pub struct HttpRequest {
    pub method: String,
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

pub fn read_http_request(stream: &mut impl Read) -> Result<HttpRequest, String> {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 1024];
    loop {
        let n = stream.read(&mut tmp).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&tmp[..n]);
        if let Some(i) = find_headers_end(&buf) {
            let (head, rest) = buf.split_at(i + 4);
            let head_str = String::from_utf8_lossy(head);
            let mut lines = head_str.split("\r\n");
            let start = lines.next().unwrap_or("");
            let mut parts = start.split_whitespace();
            let method = parts.next().unwrap_or("GET").to_string();
            let path = parts.next().unwrap_or("/").to_string();
            let mut headers = Vec::new();
            let mut content_length = 0usize;
            for line in lines {
                if line.is_empty() {
                    break;
                }
                if let Some((k, v)) = line.split_once(':') {
                    let key = k.trim().to_string();
                    let val = v.trim().to_string();
                    if key.eq_ignore_ascii_case("content-length") {
                        content_length = val.parse().unwrap_or(0);
                    }
                    headers.push((key, val));
                }
            }
            let mut body = rest.to_vec();
            while body.len() < content_length {
                let n = stream.read(&mut tmp).map_err(|e| e.to_string())?;
                if n == 0 {
                    break;
                }
                body.extend_from_slice(&tmp[..n]);
            }
            body.truncate(content_length);
            return Ok(HttpRequest {
                method,
                path,
                headers,
                body,
            });
        }
        if buf.len() > 1024 * 1024 {
            return Err("HTTP request too large".into());
        }
    }
    Err("incomplete HTTP request".into())
}

fn find_headers_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n")
}

pub fn write_http_response(
    stream: &mut impl Write,
    status: u16,
    reason: &str,
    content_type: &str,
    body: &[u8],
) -> Result<(), String> {
    let head = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream
        .write_all(head.as_bytes())
        .map_err(|e| e.to_string())?;
    stream.write_all(body).map_err(|e| e.to_string())?;
    stream.flush().map_err(|e| e.to_string())?;
    Ok(())
}

pub fn bearer_token(headers: &[(String, String)]) -> Option<String> {
    for (k, v) in headers {
        if k.eq_ignore_ascii_case("authorization") {
            let v = v.trim();
            if let Some(t) = v
                .strip_prefix("Bearer ")
                .or_else(|| v.strip_prefix("token "))
            {
                return Some(t.trim().to_string());
            }
        }
    }
    None
}

/// Client GET/POST against `base` (`http://127.0.0.1:3920` or `https://…` Partial).
pub fn http_exchange(
    base: &str,
    method: &str,
    path: &str,
    token: Option<&str>,
    body: Option<&str>,
) -> Result<(u16, String), String> {
    let (status, bytes) = http_exchange_bytes(
        base,
        method,
        path,
        token,
        body.map(|s| s.as_bytes()),
        "application/json",
        "application/json",
    )?;
    Ok((status, String::from_utf8_lossy(&bytes).into_owned()))
}

/// Raw body exchange (release assets / run download). Loopback http only for binary Claimed.
pub fn http_exchange_bytes(
    base: &str,
    method: &str,
    path: &str,
    token: Option<&str>,
    body: Option<&[u8]>,
    content_type: &str,
    accept: &str,
) -> Result<(u16, Vec<u8>), String> {
    let base = base.trim_end_matches('/');
    let url = format!("{base}{path}");
    if url.starts_with("https://") {
        let (st, s) = http_exchange_https(
            &url,
            method,
            token,
            body.map(|b| String::from_utf8_lossy(b).into_owned())
                .as_deref(),
        )?;
        return Ok((st, s.into_bytes()));
    }
    let (host, port, req_path) = parse_http_url(&url)?;
    let mut stream = TcpStream::connect((host.as_str(), port))
        .map_err(|e| format!("connect {host}:{port}: {e}"))?;
    let body_bytes = body.unwrap_or(&[]);
    let mut req = format!(
        "{method} {req_path} HTTP/1.1\r\nHost: {host}:{port}\r\nAccept: {accept}\r\nConnection: close\r\n"
    );
    if let Some(t) = token {
        req.push_str(&format!("Authorization: Bearer {t}\r\n"));
    }
    if !body_bytes.is_empty() {
        req.push_str(&format!("Content-Type: {content_type}\r\n"));
        req.push_str(&format!("Content-Length: {}\r\n", body_bytes.len()));
    } else {
        req.push_str("Content-Length: 0\r\n");
    }
    req.push_str("\r\n");
    stream
        .write_all(req.as_bytes())
        .map_err(|e| e.to_string())?;
    if !body_bytes.is_empty() {
        stream.write_all(body_bytes).map_err(|e| e.to_string())?;
    }
    stream.flush().map_err(|e| e.to_string())?;

    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).map_err(|e| e.to_string())?;
    let sep = buf
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or_else(|| "http response missing header separator".to_string())?;
    let head = String::from_utf8_lossy(&buf[..sep]);
    let status: u16 = head
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let body = buf[sep + 4..].to_vec();
    Ok((status, body))
}

/// Partial HTTPS remote client via workspace `ureq` (TLS feature). Not Claimed e2e.
/// Hardening Partial: timeouts, redirect cap, User-Agent, size cap, transport retry.
fn http_exchange_https(
    url: &str,
    method: &str,
    token: Option<&str>,
    body: Option<&str>,
) -> Result<(u16, String), String> {
    const MAX_ATTEMPTS: u32 = 3;
    let mut last_err = None;
    for attempt in 1..=MAX_ATTEMPTS {
        match http_exchange_https_once(url, method, token, body) {
            Ok(v) => {
                if attempt > 1 {
                    eprintln!(
                        "note: https Partial retry succeeded on attempt {attempt}/{MAX_ATTEMPTS} \
                         (transport backoff; design ≠ Claimed forge remote e2e)"
                    );
                }
                return Ok(v);
            }
            Err(e) if is_https_transport_retryable(&e) && attempt < MAX_ATTEMPTS => {
                eprintln!("note: https Partial transport retry {attempt}/{MAX_ATTEMPTS}: {e}");
                last_err = Some(e);
                std::thread::sleep(Duration::from_millis(50 * u64::from(attempt)));
            }
            Err(e) => return Err(e),
        }
    }
    Err(last_err.unwrap_or_else(|| "https Partial retry exhausted".into()))
}

fn is_https_transport_retryable(err: &str) -> bool {
    let e = err.to_ascii_lowercase();
    e.contains("transport")
        || e.contains("timed out")
        || e.contains("timeout")
        || e.contains("connection reset")
        || e.contains("connection refused")
        || e.contains("broken pipe")
}

/// Operator trust anchor for `https://` hub client (B3 clean-room; PEM file path).
fn hub_https_client_tls_config() -> Result<Option<Arc<rustls::ClientConfig>>, String> {
    let ca_path = match env::var("ARKS_HUB_CA_FILE") {
        Ok(p) if !p.trim().is_empty() => p,
        _ => return Ok(None),
    };
    Ok(Some(load_operator_ca_tls_config(Path::new(&ca_path))?))
}

fn load_operator_ca_tls_config(ca_path: &Path) -> Result<Arc<rustls::ClientConfig>, String> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let f = File::open(ca_path)
        .map_err(|e| format!("ARKS_HUB_CA_FILE open {}: {e}", ca_path.display()))?;
    let mut reader = BufReader::new(f);
    let certs: Vec<CertificateDer<'static>> = rustls_pemfile::certs(&mut reader)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("ARKS_HUB_CA_FILE parse {}: {e}", ca_path.display()))?;
    if certs.is_empty() {
        return Err(format!("ARKS_HUB_CA_FILE PEM empty: {}", ca_path.display()));
    }
    let mut root_store = rustls::RootCertStore::empty();
    for cert in certs {
        root_store
            .add(cert)
            .map_err(|e| format!("ARKS_HUB_CA_FILE root add: {e}"))?;
    }
    let config = rustls::ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();
    Ok(Arc::new(config))
}

fn http_exchange_https_once(
    url: &str,
    method: &str,
    token: Option<&str>,
    body: Option<&str>,
) -> Result<(u16, String), String> {
    // Redirect cap keeps Partial client fail-closed on redirect loops (not Claimed cookie jar).
    let mut agent_builder = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(10))
        .timeout_read(Duration::from_secs(30))
        .timeout_write(Duration::from_secs(30))
        .redirects(4)
        .user_agent("arks-hub/0.0.0 (+Partial HTTPS client; design≠Claimed)");
    if let Some(tls_cfg) = hub_https_client_tls_config()? {
        agent_builder = agent_builder.tls_config(tls_cfg);
    }
    let agent = agent_builder.build();
    let mut req = agent
        .request(method, url)
        .set("Accept", "application/json")
        .set("Accept-Encoding", "identity")
        .set("X-Arks-Hub-Client", "partial-https");
    if let Some(t) = token {
        req = req.set("Authorization", &format!("Bearer {t}"));
    }
    let resp = if let Some(b) = body {
        if b.len() > 1024 * 1024 {
            return Err(format!(
                "https Partial refuse: request body {} > 1MiB (Partial size cap)",
                b.len()
            ));
        }
        req.set("Content-Type", "application/json")
            .send_string(b)
            .map_err(|e| format_https_err("request", e))?
    } else {
        req.call().map_err(|e| format_https_err("request", e))?
    };
    let status = resp.status();
    // Prefer charset-safe into_string; size-cap after read (Partial; not Claimed streaming).
    let text = resp
        .into_string()
        .map_err(|e| format!("https Partial body read: {e}"))?;
    if text.len() > 4 * 1024 * 1024 {
        return Err(format!(
            "https Partial refuse: response body {} > 4MiB (Partial size cap; not Claimed streaming)",
            text.len()
        ));
    }
    if (500..600).contains(&status) {
        // Surface server errors clearly; caller decides exit — not auto-retry (non-idempotent POST).
        eprintln!(
            "note: https Partial upstream HTTP {status} (no auto-retry on 5xx; design ≠ Claimed)"
        );
    }
    Ok((status, text))
}

fn format_https_err(kind: &str, err: ureq::Error) -> String {
    match &err {
        ureq::Error::Status(code, resp) => {
            let url = resp.get_url().to_string();
            format!(
                "https Partial {kind} HTTP {code} for {url} (design ≠ Claimed forge remote e2e)"
            )
        }
        ureq::Error::Transport(t) => {
            format!("https Partial {kind} transport: {t} (TLS/DNS/connect; never spawns gh)")
        }
    }
}

fn parse_http_url(url: &str) -> Result<(String, u16, String), String> {
    let rest = url
        .strip_prefix("http://")
        .ok_or_else(|| format!("expected http:// or https:// hub URL (got {url})"))?;
    let (hostport, path) = rest
        .split_once('/')
        .map(|(h, p)| (h, format!("/{p}")))
        .unwrap_or((rest, "/".into()));
    if let Some((h, p)) = hostport.rsplit_once(':') {
        let port: u16 = p.parse().map_err(|_| "bad port".to_string())?;
        Ok((h.to_string(), port, path))
    } else {
        Ok((hostport.to_string(), 80, path))
    }
}
