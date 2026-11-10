//! Minimal HTTP/1.1 helpers for local hub serve/client (Partial). No TLS Claimed.

use std::io::{Read, Write};
use std::net::TcpStream;

#[derive(Debug)]
pub struct HttpRequest {
    pub method: String,
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

pub fn read_http_request(stream: &mut TcpStream) -> Result<HttpRequest, String> {
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
    stream: &mut TcpStream,
    status: u16,
    reason: &str,
    content_type: &str,
    body: &[u8],
) -> Result<(), String> {
    let head = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes()).map_err(|e| e.to_string())?;
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

/// Client GET/POST against `base` (e.g. http://127.0.0.1:3920).
pub fn http_exchange(
    base: &str,
    method: &str,
    path: &str,
    token: Option<&str>,
    body: Option<&str>,
) -> Result<(u16, String), String> {
    let base = base.trim_end_matches('/');
    let url = format!("{base}{path}");
    let (host, port, req_path) = parse_http_url(&url)?;
    let mut stream = TcpStream::connect((host.as_str(), port))
        .map_err(|e| format!("connect {host}:{port}: {e}"))?;
    let body_bytes = body.unwrap_or("").as_bytes();
    let mut req = format!(
        "{method} {req_path} HTTP/1.1\r\nHost: {host}:{port}\r\nAccept: application/json\r\nConnection: close\r\n"
    );
    if let Some(t) = token {
        req.push_str(&format!("Authorization: Bearer {t}\r\n"));
    }
    if !body_bytes.is_empty() {
        req.push_str("Content-Type: application/json\r\n");
        req.push_str(&format!("Content-Length: {}\r\n", body_bytes.len()));
    } else {
        req.push_str("Content-Length: 0\r\n");
    }
    req.push_str("\r\n");
    stream.write_all(req.as_bytes()).map_err(|e| e.to_string())?;
    if !body_bytes.is_empty() {
        stream.write_all(body_bytes).map_err(|e| e.to_string())?;
    }
    stream.flush().map_err(|e| e.to_string())?;

    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).map_err(|e| e.to_string())?;
    let text = String::from_utf8_lossy(&buf);
    let (status_line, rest) = text.split_once("\r\n").unwrap_or((&text, ""));
    let status: u16 = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let body = if let Some(i) = rest.find("\r\n\r\n") {
        rest[i + 4..].to_string()
    } else {
        rest.to_string()
    };
    Ok((status, body))
}

fn parse_http_url(url: &str) -> Result<(String, u16, String), String> {
    let rest = url
        .strip_prefix("http://")
        .ok_or_else(|| format!("only http:// local hub supported in Partial (got {url})"))?;
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
