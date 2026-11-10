//! Git LFS batch + basic transfer + locks server (Claimed subset).
//!
//! Storage: `$GIT_DIR/lfs/objects/aa/bb/<oid>`; locks: `$GIT_DIR/lfs/locks.json`
//! Endpoints:
//! - `POST …/info/lfs/objects/batch`
//! - `GET|PUT …/info/lfs/objects/{oid}`
//! - `GET|POST …/info/lfs/locks` (list / create)
//! - `POST …/info/lfs/locks/verify`
//! - `POST …/info/lfs/locks/{id}/unlock`
//!
//! Non-claims: tus resumable transfer; multi-tenant S3 custom agents (handoff).
//! Cite: git-lfs batch + basic-transfers + locking docs. ADR: [[adr:root/20261110214800]]

use crate::error::ServerError;
use crate::http::{HttpRequest, HttpResponse};
use arks_repo::Repository;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const LFS_JSON: &str = "application/vnd.git-lfs+json";

pub fn handle_lfs(
    repo: &Repository,
    req: &HttpRequest,
) -> Result<Option<HttpResponse>, ServerError> {
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

    if rest == "locks/verify" || rest.ends_with("/locks/verify") {
        if !req.method.eq_ignore_ascii_case("POST") {
            return Ok(Some(text(405, b"method not allowed")));
        }
        return Ok(Some(handle_locks_verify(repo)?));
    }

    if let Some(id) = rest.strip_prefix("locks/") {
        let id = id.trim_matches('/');
        if let Some(lock_id) = id.strip_suffix("/unlock") {
            if req.method.eq_ignore_ascii_case("POST") {
                return Ok(Some(handle_lock_unlock(repo, lock_id.trim_matches('/'))?));
            }
            return Ok(Some(text(405, b"method not allowed")));
        }
    }

    if rest == "locks" || rest.ends_with("/locks") {
        if req.method.eq_ignore_ascii_case("GET") {
            return Ok(Some(handle_locks_list(repo, &req.query)?));
        }
        if req.method.eq_ignore_ascii_case("POST") {
            return Ok(Some(handle_lock_create(repo, req)?));
        }
        return Ok(Some(text(405, b"method not allowed")));
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

#[derive(Clone)]
struct LfsLock {
    id: String,
    path: String,
    owner: String,
    locked_at: String,
}

fn locks_path(git_dir: &Path) -> PathBuf {
    git_dir.join("lfs").join("locks.json")
}

fn load_locks(git_dir: &Path) -> Result<Vec<LfsLock>, ServerError> {
    let path = locks_path(git_dir);
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let text = fs::read_to_string(&path).map_err(|e| ServerError::Msg(e.to_string()))?;
    Ok(parse_locks_json(&text))
}

fn save_locks(git_dir: &Path, locks: &[LfsLock]) -> Result<(), ServerError> {
    let path = locks_path(git_dir);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| ServerError::Msg(e.to_string()))?;
    }
    let mut body = String::from("{\"locks\":[");
    for (i, l) in locks.iter().enumerate() {
        if i > 0 {
            body.push(',');
        }
        body.push_str(&format!(
            r#"{{"id":"{}","path":"{}","owner":{{"name":"{}"}},"locked_at":"{}"}}"#,
            json_escape(&l.id),
            json_escape(&l.path),
            json_escape(&l.owner),
            json_escape(&l.locked_at)
        ));
    }
    body.push_str("]}");
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, body).map_err(|e| ServerError::Msg(e.to_string()))?;
    fs::rename(&tmp, &path).map_err(|e| ServerError::Msg(e.to_string()))?;
    Ok(())
}

fn parse_locks_json(text: &str) -> Vec<LfsLock> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(i) = rest.find("\"id\"") {
        rest = &rest[i + 4..];
        let Some(id) = scrape_json_string(rest) else {
            break;
        };
        rest = skip_after_string(rest);
        let Some(pi) = rest.find("\"path\"") else {
            continue;
        };
        rest = &rest[pi + 6..];
        let Some(path) = scrape_json_string(rest) else {
            continue;
        };
        rest = skip_after_string(rest);
        let owner = scrape_named_owner(rest).unwrap_or_else(|| "arks".into());
        let locked_at = scrape_locked_at(rest).unwrap_or_else(|| "1970-01-01T00:00:00Z".into());
        out.push(LfsLock {
            id,
            path,
            owner,
            locked_at,
        });
    }
    out
}

fn scrape_json_string(rest: &str) -> Option<String> {
    let colon = rest.find(':')?;
    let mut s = rest[colon + 1..].trim_start();
    if !s.starts_with('"') {
        return None;
    }
    s = &s[1..];
    let end = s.find('"')?;
    Some(s[..end].to_string())
}

fn skip_after_string(rest: &str) -> &str {
    let Some(colon) = rest.find(':') else {
        return rest;
    };
    let mut s = rest[colon + 1..].trim_start();
    if s.starts_with('"') {
        s = &s[1..];
        if let Some(end) = s.find('"') {
            return &s[end + 1..];
        }
    }
    rest
}

fn scrape_named_owner(rest: &str) -> Option<String> {
    let i = rest.find("\"name\"")?;
    scrape_json_string(&rest[i + 6..])
}

fn scrape_locked_at(rest: &str) -> Option<String> {
    let i = rest.find("\"locked_at\"")?;
    scrape_json_string(&rest[i + 11..])
}

fn json_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn now_rfc3339() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{secs}Z")
}

fn next_lock_id(locks: &[LfsLock]) -> String {
    let max = locks
        .iter()
        .filter_map(|l| l.id.parse::<u64>().ok())
        .max()
        .unwrap_or(0);
    (max + 1).to_string()
}

fn lock_to_json(l: &LfsLock) -> String {
    format!(
        r#"{{"id":"{}","path":"{}","owner":{{"name":"{}"}},"locked_at":"{}"}}"#,
        json_escape(&l.id),
        json_escape(&l.path),
        json_escape(&l.owner),
        json_escape(&l.locked_at)
    )
}

fn handle_locks_list(repo: &Repository, query: &str) -> Result<HttpResponse, ServerError> {
    let locks = load_locks(repo.git_dir())?;
    let path_q = query_param(query, "path");
    let mut body = String::from(r#"{"locks":["#);
    let mut first = true;
    for l in &locks {
        if let Some(p) = path_q {
            if l.path != p {
                continue;
            }
        }
        if !first {
            body.push(',');
        }
        first = false;
        body.push_str(&lock_to_json(l));
    }
    body.push_str("]}");
    Ok(typed(200, LFS_JSON, body.into_bytes()))
}

fn handle_lock_create(repo: &Repository, req: &HttpRequest) -> Result<HttpResponse, ServerError> {
    let text_body = std::str::from_utf8(&req.body).unwrap_or("");
    let path = scrape_field(text_body, "path").unwrap_or_default();
    if path.is_empty() {
        return Ok(typed(
            400,
            LFS_JSON,
            br#"{"message":"path required"}"#.to_vec(),
        ));
    }
    let mut locks = load_locks(repo.git_dir())?;
    if let Some(existing) = locks.iter().find(|l| l.path == path) {
        let body = format!(
            r#"{{"lock":{},"message":"already locked"}}"#,
            lock_to_json(existing)
        );
        return Ok(typed(409, LFS_JSON, body.into_bytes()));
    }
    let owner = scrape_field(text_body, "name")
        .or_else(|| scrape_named_owner(text_body))
        .unwrap_or_else(|| "arks".into());
    let lock = LfsLock {
        id: next_lock_id(&locks),
        path,
        owner,
        locked_at: now_rfc3339(),
    };
    let json = lock_to_json(&lock);
    locks.push(lock);
    save_locks(repo.git_dir(), &locks)?;
    let body = format!(r#"{{"lock":{json}}}"#);
    Ok(typed(201, LFS_JSON, body.into_bytes()))
}

fn handle_locks_verify(repo: &Repository) -> Result<HttpResponse, ServerError> {
    let locks = load_locks(repo.git_dir())?;
    let mut ours = String::from("[");
    let mut theirs = String::from("[");
    // Single-tenant Claimed slice: treat all locks as "ours" for verify honesty.
    for (i, l) in locks.iter().enumerate() {
        if i > 0 {
            ours.push(',');
        }
        ours.push_str(&lock_to_json(l));
    }
    ours.push(']');
    theirs.push(']');
    let body = format!(r#"{{"ours":{ours},"theirs":{theirs}}}"#);
    Ok(typed(200, LFS_JSON, body.into_bytes()))
}

fn handle_lock_unlock(repo: &Repository, id: &str) -> Result<HttpResponse, ServerError> {
    let mut locks = load_locks(repo.git_dir())?;
    let Some(pos) = locks.iter().position(|l| l.id == id) else {
        return Ok(typed(
            404,
            LFS_JSON,
            br#"{"message":"lock not found"}"#.to_vec(),
        ));
    };
    let removed = locks.remove(pos);
    save_locks(repo.git_dir(), &locks)?;
    let body = format!(r#"{{"lock":{}}}"#, lock_to_json(&removed));
    Ok(typed(200, LFS_JSON, body.into_bytes()))
}

fn scrape_field(body: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let i = body.find(&needle)?;
    scrape_json_string(&body[i + needle.len()..])
}

fn query_param<'a>(query: &'a str, key: &str) -> Option<&'a str> {
    for part in query.split('&') {
        let mut it = part.splitn(2, '=');
        let k = it.next()?;
        if k == key {
            return it.next();
        }
    }
    None
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

fn handle_put_object(
    repo: &Repository,
    oid: &str,
    body: &[u8],
) -> Result<HttpResponse, ServerError> {
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
        let Some(colon) = rest.find(':') else {
            continue;
        };
        rest = rest[colon + 1..].trim_start();
        let num: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
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
