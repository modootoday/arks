//! Git LFS batch + basic transfer server (Claimed subset).
//!
//! Storage: `$GIT_DIR/lfs/objects/aa/bb/<oid>`
//! Endpoints:
//! - `POST …/info/lfs/objects/batch`
//! - `GET|PUT …/info/lfs/objects/{oid}`
//!
//! Cite: git-lfs batch + basic-transfers docs. ADR: [[adr:root/20261110214800]]

use crate::error::ServerError;
use crate::http::{HttpRequest, HttpResponse};
use arks_repo::Repository;
use std::fs;
use std::path::Path;

const LFS_JSON: &str = "application/vnd.git-lfs+json";

pub fn handle_lfs(repo: &Repository, req: &HttpRequest) -> Result<Option<HttpResponse>, ServerError> {
    let path = req.path.as_str();
    let lfs_idx = match path.find("info/lfs") {
        Some(i) => i,
        None => return Ok(None),
    };
    let rest = path[lfs_idx + "info/lfs".len()..].trim_start_matches('/');

    if rest == "objects/batch" || rest.ends_with("/objects/batch") {
        if !req.method.eq_ignore_ascii_case("POST") {
            return Ok(Some(text(405, b"method not allowed")));
        }
        return Ok(Some(handle_batch(repo, req)?));
    }

    if let Some(oid) = rest.strip_prefix("objects/") {
        let oid = oid.trim_matches('/');
        if oid.len() == 64 && oid.chars().all(|c| c.is_ascii_hexdigit()) {
            if req.method.eq_ignore_ascii_case("GET") || req.method.eq_ignore_ascii_case("HEAD") {
                return Ok(Some(handle_get_object(repo, oid)?));
            }
            if req.method.eq_ignore_ascii_case("PUT") {
                return Ok(Some(handle_put_object(repo, oid, &req.body)?));
            }
            return Ok(Some(text(405, b"method not allowed")));
        }
    }
    Ok(Some(text(404, b"lfs not found")))
}

fn handle_batch(repo: &Repository, req: &HttpRequest) -> Result<HttpResponse, ServerError> {
    let text_body = std::str::from_utf8(&req.body).unwrap_or("");
    let op = if text_body.contains("\"upload\"") {
        "upload"
    } else {
        "download"
    };
    let objects = parse_batch_objects(text_body);
    let base = public_lfs_base(req);

    let mut out = String::from(r#"{"transfer":"basic","objects":["#);
    for (i, (oid, size)) in objects.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        let path = object_path(repo.git_dir(), oid);
        let exists = path.is_file();
        if op == "download" {
            if exists {
                out.push_str(&format!(
                    r#"{{"oid":"{oid}","size":{size},"actions":{{"download":{{"href":"{base}/objects/{oid}"}}}}}}"#
                ));
            } else {
                out.push_str(&format!(
                    r#"{{"oid":"{oid}","size":{size},"error":{{"code":404,"message":"not found"}}}}"#
                ));
            }
        } else {
            // upload
            if exists {
                out.push_str(&format!(r#"{{"oid":"{oid}","size":{size}}}"#));
            } else {
                out.push_str(&format!(
                    r#"{{"oid":"{oid}","size":{size},"actions":{{"upload":{{"href":"{base}/objects/{oid}"}}}}}}"#
                ));
            }
        }
    }
    out.push_str("]}");
    Ok(typed(200, LFS_JSON, out.into_bytes()))
}

fn handle_get_object(repo: &Repository, oid: &str) -> Result<HttpResponse, ServerError> {
    let path = object_path(repo.git_dir(), oid);
    if !path.is_file() {
        return Ok(text(404, b"not found"));
    }
    let body = fs::read(&path).map_err(|e| ServerError::Msg(e.to_string()))?;
    Ok(typed(200, "application/octet-stream", body))
}

fn handle_put_object(repo: &Repository, oid: &str, body: &[u8]) -> Result<HttpResponse, ServerError> {
    let path = object_path(repo.git_dir(), oid);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| ServerError::Msg(e.to_string()))?;
    }
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, body).map_err(|e| ServerError::Msg(e.to_string()))?;
    fs::rename(&tmp, &path).map_err(|e| ServerError::Msg(e.to_string()))?;
    Ok(typed(201, "application/octet-stream", Vec::new()))
}

fn public_lfs_base(req: &HttpRequest) -> String {
    if let Ok(env) = std::env::var("ARKS_LFS_PUBLIC_BASE") {
        let t = env.trim().trim_end_matches('/');
        if !t.is_empty() {
            return t.to_string();
        }
    }
    let host = req.host.as_deref().unwrap_or("127.0.0.1");
    // Preserve path prefix before info/lfs (multi-repo mounts).
    let prefix = match req.path.find("info/lfs") {
        Some(i) => &req.path[..i],
        None => "/",
    };
    let prefix = if prefix.is_empty() { "/" } else { prefix };
    format!("http://{host}{prefix}info/lfs")
}

fn object_path(git_dir: &Path, oid: &str) -> std::path::PathBuf {
    git_dir
        .join("lfs")
        .join("objects")
        .join(&oid[..2])
        .join(&oid[2..4])
        .join(oid)
}

fn parse_batch_objects(body: &str) -> Vec<(String, u64)> {
    let mut out = Vec::new();
    // Scrape "oid":"...","size":N pairs
    let mut rest = body;
    while let Some(i) = rest.find("\"oid\"") {
        rest = &rest[i + 5..];
        let Some(colon) = rest.find(':') else { break };
        rest = rest[colon + 1..].trim_start();
        if !rest.starts_with('"') {
            continue;
        }
        rest = &rest[1..];
        let Some(end) = rest.find('"') else { break };
        let oid = rest[..end].to_string();
        rest = &rest[end + 1..];
        let Some(si) = rest.find("\"size\"") else {
            continue;
        };
        rest = &rest[si + 6..];
        let Some(colon) = rest.find(':') else { continue };
        rest = rest[colon + 1..].trim_start();
        let num: String = rest
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        if let Ok(size) = num.parse::<u64>() {
            if oid.len() == 64 {
                out.push((oid, size));
            }
        }
    }
    out
}

fn typed(status: u16, content_type: &str, body: Vec<u8>) -> HttpResponse {
    HttpResponse {
        status,
        content_type: content_type.into(),
        body,
        www_authenticate: None,
        extra_headers: Vec::new(),
    }
}

fn text(status: u16, body: &[u8]) -> HttpResponse {
    typed(status, "text/plain", body.to_vec())
}
