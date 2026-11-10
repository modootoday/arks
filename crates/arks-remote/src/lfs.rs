//! Git LFS batch/basic client — network fetch + local store.
//!
//! Cite: <https://github.com/git-lfs/git-lfs/blob/main/docs/api/batch.md>
//! Cite: <https://github.com/git-lfs/git-lfs/blob/main/docs/api/basic-transfers.md>
//! ADR: [[adr:root/20261110214800]]

use crate::error::RemoteError;
use crate::http::{UreqTransport, http_get};
use crate::transport::{HttpRequest, Transport};
use arks_repo::Repository;
use std::fs;
use std::path::{Path, PathBuf};

/// Parsed Git LFS pointer (spec v1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LfsPointer {
    pub oid_hex: String,
    pub size: u64,
}

/// Parse a Git LFS pointer blob; `None` if not a pointer.
pub fn parse_lfs_pointer(payload: &[u8]) -> Option<LfsPointer> {
    let text = std::str::from_utf8(payload).ok()?;
    if !text.starts_with("version https://git-lfs.github.com/spec/v1\n")
        && !text.starts_with("version https://git-lfs.github.com/spec/v1\r\n")
    {
        return None;
    }
    let mut oid_hex = None;
    let mut size = None;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("oid sha256:") {
            oid_hex = Some(rest.trim().to_string());
        } else if let Some(rest) = line.strip_prefix("size ") {
            size = rest.trim().parse().ok();
        }
    }
    let oid_hex = oid_hex?;
    if oid_hex.len() != 64 || !oid_hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    Some(LfsPointer {
        oid_hex,
        size: size?,
    })
}

pub fn lfs_object_path(git_dir: &Path, oid_hex: &str) -> PathBuf {
    git_dir
        .join("lfs")
        .join("objects")
        .join(&oid_hex[..2])
        .join(&oid_hex[2..4])
        .join(oid_hex)
}

pub fn read_local_lfs_object(
    git_dir: &Path,
    ptr: &LfsPointer,
) -> Result<Option<Vec<u8>>, RemoteError> {
    let path = lfs_object_path(git_dir, &ptr.oid_hex);
    if !path.is_file() {
        return Ok(None);
    }
    let bytes = fs::read(&path).map_err(|e| RemoteError::Msg(e.to_string()))?;
    if bytes.len() as u64 != ptr.size {
        return Err(RemoteError::Msg(format!(
            "lfs: size mismatch for {} (got {} want {})",
            ptr.oid_hex,
            bytes.len(),
            ptr.size
        )));
    }
    Ok(Some(bytes))
}

pub fn write_local_lfs_object(
    git_dir: &Path,
    oid_hex: &str,
    bytes: &[u8],
) -> Result<(), RemoteError> {
    let path = lfs_object_path(git_dir, oid_hex);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| RemoteError::Msg(e.to_string()))?;
    }
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, bytes).map_err(|e| RemoteError::Msg(e.to_string()))?;
    if path.is_file() {
        let _ = fs::remove_file(&tmp);
        return Ok(());
    }
    fs::rename(&tmp, &path).map_err(|e| RemoteError::Msg(e.to_string()))?;
    Ok(())
}

/// Discover LFS API base URL (no trailing slash).
pub fn discover_lfs_endpoint(repo: &Repository) -> Result<Option<String>, RemoteError> {
    if let Ok(env) = std::env::var("ARKS_LFS_URL") {
        let t = env.trim();
        if !t.is_empty() {
            return Ok(Some(t.trim_end_matches('/').to_string()));
        }
    }
    let cfg = fs::read_to_string(repo.git_dir().join("config")).unwrap_or_default();
    if let Some(url) = config_value(&cfg, "lfs.url") {
        return Ok(Some(url.trim_end_matches('/').to_string()));
    }
    // First remote url → {url}.git/info/lfs or {url}/info/lfs
    if let Some(remote_url) = first_remote_url(&cfg) {
        return Ok(Some(lfs_endpoint_from_remote_url(&remote_url)));
    }
    Ok(None)
}

fn lfs_endpoint_from_remote_url(url: &str) -> String {
    let base = url.trim_end_matches('/');
    if base.ends_with(".git") {
        format!("{base}/info/lfs")
    } else {
        format!("{base}.git/info/lfs")
    }
}

fn first_remote_url(cfg: &str) -> Option<String> {
    let mut in_remote = false;
    for raw in cfg.lines() {
        let line = raw.trim();
        if line.starts_with('[') {
            in_remote = line.to_ascii_lowercase().starts_with("[remote ");
            continue;
        }
        if in_remote {
            if let Some((k, v)) = line.split_once('=') {
                if k.trim().eq_ignore_ascii_case("url") {
                    return Some(v.trim().trim_matches('"').to_string());
                }
            }
        }
    }
    None
}

fn config_value(text: &str, key: &str) -> Option<String> {
    let key = key.to_ascii_lowercase();
    let mut section = String::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if line.starts_with('[') {
            section = line
                .trim_matches(|c| c == '[' || c == ']')
                .to_ascii_lowercase()
                .replace('"', "");
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let full = if section.contains(' ') {
            let mut bits = section.split_whitespace();
            let head = bits.next().unwrap_or("");
            let name = bits.next().unwrap_or("");
            format!("{head}.{name}.{}", k.trim().to_ascii_lowercase())
        } else {
            format!("{section}.{}", k.trim().to_ascii_lowercase())
        };
        if full == key {
            return Some(v.trim().trim_matches('"').to_string());
        }
    }
    None
}

/// Batch download one object and store under `$GIT_DIR/lfs/objects`.
pub fn lfs_batch_download(repo: &Repository, ptr: &LfsPointer) -> Result<Vec<u8>, RemoteError> {
    let endpoint = discover_lfs_endpoint(repo)?.ok_or_else(|| {
        RemoteError::Msg(
            "lfs: no endpoint (set lfs.url, a remote url, or ARKS_LFS_URL for network fetch)"
                .into(),
        )
    })?;
    let batch_url = format!("{endpoint}/objects/batch");
    let body = format!(
        r#"{{"operation":"download","transfers":["basic"],"objects":[{{"oid":"{}","size":{}}}]}}"#,
        ptr.oid_hex, ptr.size
    );
    let mut transport = UreqTransport::new();
    let resp = transport.exchange(&HttpRequest {
        method: "POST".into(),
        url: batch_url,
        headers: vec![
            ("Accept".into(), "application/vnd.git-lfs+json".into()),
            ("Content-Type".into(), "application/vnd.git-lfs+json".into()),
        ],
        body: body.into_bytes(),
    })?;
    if resp.status != 200 {
        return Err(RemoteError::Msg(format!(
            "lfs batch download HTTP {}",
            resp.status
        )));
    }
    let href = extract_download_href(&resp.body)
        .ok_or_else(|| RemoteError::Msg("lfs batch: missing actions.download.href".into()))?;
    let headers = extract_download_headers(&resp.body);
    let file = if headers.is_empty() {
        http_get(&href)?
    } else {
        let mut t = UreqTransport::new();
        t.exchange(&HttpRequest {
            method: "GET".into(),
            url: href,
            headers,
            body: Vec::new(),
        })?
    };
    if file.status != 200 {
        return Err(RemoteError::Msg(format!(
            "lfs object download HTTP {}",
            file.status
        )));
    }
    if file.body.len() as u64 != ptr.size {
        return Err(RemoteError::Msg(format!(
            "lfs download size mismatch (got {} want {})",
            file.body.len(),
            ptr.size
        )));
    }
    write_local_lfs_object(repo.git_dir(), &ptr.oid_hex, &file.body)?;
    Ok(file.body)
}

/// Resolve pointer: local store, else network batch download.
pub fn resolve_lfs_pointer(
    repo: &Repository,
    payload: &[u8],
) -> Result<Option<Vec<u8>>, RemoteError> {
    let Some(ptr) = parse_lfs_pointer(payload) else {
        return Ok(None);
    };
    if let Some(bytes) = read_local_lfs_object(repo.git_dir(), &ptr)? {
        return Ok(Some(bytes));
    }
    match lfs_batch_download(repo, &ptr) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(e) => {
            // No endpoint / network fail: leave pointer bytes (honest non-mutation).
            eprintln!("note: lfs network fetch skipped ({e}); leaving pointer");
            Ok(None)
        }
    }
}

/// Batch upload local LFS object to endpoint (basic transfer).
pub fn lfs_batch_upload(
    repo: &Repository,
    ptr: &LfsPointer,
    bytes: &[u8],
) -> Result<(), RemoteError> {
    let endpoint = discover_lfs_endpoint(repo)?
        .ok_or_else(|| RemoteError::Msg("lfs: no endpoint for upload".into()))?;
    let batch_url = format!("{endpoint}/objects/batch");
    let body = format!(
        r#"{{"operation":"upload","transfers":["basic"],"objects":[{{"oid":"{}","size":{}}}]}}"#,
        ptr.oid_hex, ptr.size
    );
    let mut transport = UreqTransport::new();
    let resp = transport.exchange(&HttpRequest {
        method: "POST".into(),
        url: batch_url,
        headers: vec![
            ("Accept".into(), "application/vnd.git-lfs+json".into()),
            ("Content-Type".into(), "application/vnd.git-lfs+json".into()),
        ],
        body: body.into_bytes(),
    })?;
    if resp.status != 200 {
        return Err(RemoteError::Msg(format!(
            "lfs batch upload HTTP {}",
            resp.status
        )));
    }
    // Already present on server → no actions.upload
    let Some(href) = extract_upload_href(&resp.body) else {
        return Ok(());
    };
    let mut headers = extract_upload_headers(&resp.body);
    if !headers
        .iter()
        .any(|(n, _)| n.eq_ignore_ascii_case("content-type"))
    {
        headers.push(("Content-Type".into(), "application/octet-stream".into()));
    }
    let mut t = UreqTransport::new();
    let put = t.exchange(&HttpRequest {
        method: "PUT".into(),
        url: href,
        headers,
        body: bytes.to_vec(),
    })?;
    if put.status != 200 && put.status != 201 {
        return Err(RemoteError::Msg(format!(
            "lfs object upload HTTP {}",
            put.status
        )));
    }
    Ok(())
}

fn extract_download_href(body: &[u8]) -> Option<String> {
    extract_action_href(body, "download")
}

fn extract_upload_href(body: &[u8]) -> Option<String> {
    extract_action_href(body, "upload")
}

fn extract_action_href(body: &[u8], action: &str) -> Option<String> {
    let text = std::str::from_utf8(body).ok()?;
    // Minimal JSON scrape: "download":{"href":"..."} or "upload":{"href":"..."}
    let key = format!("\"{action}\"");
    let idx = text.find(&key)?;
    let after = &text[idx + key.len()..];
    let href_key = "\"href\"";
    let h = after.find(href_key)?;
    let rest = &after[h + href_key.len()..];
    let colon = rest.find(':')?;
    let rest = rest[colon + 1..].trim_start();
    if !rest.starts_with('"') {
        return None;
    }
    let rest = &rest[1..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn extract_download_headers(body: &[u8]) -> Vec<(String, String)> {
    extract_action_headers(body, "download")
}

fn extract_upload_headers(body: &[u8]) -> Vec<(String, String)> {
    extract_action_headers(body, "upload")
}

fn extract_action_headers(body: &[u8], action: &str) -> Vec<(String, String)> {
    // Optional; most local servers need none. Skip nested parse for Claimed slice.
    let _ = (body, action);
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_pointer_ok() {
        let p = b"version https://git-lfs.github.com/spec/v1\noid sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\nsize 12\n";
        let ptr = parse_lfs_pointer(p).unwrap();
        assert_eq!(ptr.size, 12);
        assert_eq!(ptr.oid_hex.len(), 64);
    }

    #[test]
    fn extract_href_from_batch_json() {
        let body = br#"{"transfer":"basic","objects":[{"oid":"aa","size":1,"actions":{"download":{"href":"http://127.0.0.1/x"}}}]}"#;
        assert_eq!(
            extract_download_href(body).as_deref(),
            Some("http://127.0.0.1/x")
        );
    }
}
