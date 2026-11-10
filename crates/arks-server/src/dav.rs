//! Additive WebDAV write path for dumb HTTP push.
//!
//! Primary: [RFC 4918](https://datatracker.ietf.org/doc/html/rfc4918) (PROPFIND /
//! LOCK / UNLOCK / MKCOL / PUT / MOVE / DELETE), [git-http-push(1)](https://git-scm.com/docs/git-http-push),
//! object URL layout from [gitprotocol-http](https://git-scm.com/docs/gitprotocol-http),
//! ref file shape from [gitrepository-layout](https://git-scm.com/docs/gitrepository-layout),
//! [git-update-server-info](https://git-scm.com/docs/git-update-server-info).
//!
//! Dumb-HTTP-only as the sole claimed server path remains forbidden.

use crate::dumb::update_server_info;
use crate::error::ServerError;
use crate::http::{HttpRequest, HttpResponse};
use arks_oid::Oid;
use arks_refs::RefTarget;
use arks_repo::Repository;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const DAV_XML: &str = "application/xml; charset=\"utf-8\"";

/// True when the HTTP method is a claimed WebDAV write/discovery verb.
pub fn is_dav_method(method: &str) -> bool {
    matches!(
        method.to_ascii_uppercase().as_str(),
        "PROPFIND" | "LOCK" | "UNLOCK" | "MKCOL" | "PUT" | "MOVE" | "DELETE"
    )
}

/// Handle one WebDAV request against an open repository.
pub fn handle_dav(repo: &mut Repository, req: &HttpRequest) -> Result<HttpResponse, ServerError> {
    let method = req.method.to_ascii_uppercase();
    // PROPFIND is discovery-only (RFC 4918); path need not be a writable Git rel.
    if method == "PROPFIND" {
        return Ok(propfind_supported_lock());
    }
    let rel = repo_rel_path(&req.path)?;
    match method.as_str() {
        "LOCK" => lock_path(repo.git_dir(), &rel),
        "UNLOCK" => unlock_path(repo.git_dir(), &rel, req.lock_token.as_deref()),
        "MKCOL" => mkcol(repo.git_dir(), &rel),
        "PUT" => put_resource(repo, &rel, &req.body, req.lock_token.as_deref()),
        "MOVE" => move_resource(repo.git_dir(), &rel, req.destination.as_deref()),
        "DELETE" => delete_resource(repo, &rel, req.lock_token.as_deref()),
        _ => Ok(text(405, b"method not allowed")),
    }
}

fn propfind_supported_lock() -> HttpResponse {
    // Minimal multistatus so git-http-push-class clients see exclusive write locks
    // (RFC 4918 §9.1 / §15; git-http-push locking_available).
    let body = br#"<?xml version="1.0" encoding="utf-8"?>
<D:multistatus xmlns:D="DAV:">
  <D:response>
    <D:href>/</D:href>
    <D:propstat>
      <D:prop>
        <D:supportedlock>
          <D:lockentry>
            <D:lockscope><D:exclusive/></D:lockscope>
            <D:locktype><D:write/></D:locktype>
          </D:lockentry>
        </D:supportedlock>
      </D:prop>
      <D:status>HTTP/1.1 200 OK</D:status>
    </D:propstat>
  </D:response>
</D:multistatus>
"#;
    HttpResponse {
        status: 207,
        content_type: DAV_XML.into(),
        body: body.to_vec(),
        www_authenticate: None,
        extra_headers: Vec::new(),
    }
}

fn lock_path(git_dir: &Path, rel: &str) -> Result<HttpResponse, ServerError> {
    ensure_writable_rel(rel)?;
    let token = new_lock_token();
    let lock_file = lock_file_path(git_dir, rel)?;
    if let Some(parent) = lock_file.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&lock_file, format!("{token}\n"))?;
    let body = format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n\
         <D:prop xmlns:D=\"DAV:\">\n\
           <D:lockdiscovery>\n\
             <D:activelock>\n\
               <D:locktype><D:write/></D:locktype>\n\
               <D:lockscope><D:exclusive/></D:lockscope>\n\
               <D:depth>0</D:depth>\n\
               <D:timeout>Second-600</D:timeout>\n\
               <D:locktoken><D:href>{token}</D:href></D:locktoken>\n\
             </D:activelock>\n\
           </D:lockdiscovery>\n\
         </D:prop>\n"
    );
    Ok(HttpResponse {
        status: 200,
        content_type: DAV_XML.into(),
        body: body.into_bytes(),
        www_authenticate: None,
        extra_headers: vec![("Lock-Token".into(), format!("<{token}>"))],
    })
}

fn unlock_path(
    git_dir: &Path,
    rel: &str,
    token: Option<&str>,
) -> Result<HttpResponse, ServerError> {
    ensure_writable_rel(rel)?;
    let lock_file = lock_file_path(git_dir, rel)?;
    if !lock_file.is_file() {
        return Ok(text(409, b"not locked"));
    }
    let stored = fs::read_to_string(&lock_file).unwrap_or_default();
    let stored = stored.trim();
    let given = token.map(strip_lock_brackets).unwrap_or("");
    if given.is_empty() || given != stored {
        return Ok(text(409, b"lock token mismatch"));
    }
    let _ = fs::remove_file(&lock_file);
    Ok(text(204, b""))
}

fn mkcol(git_dir: &Path, rel: &str) -> Result<HttpResponse, ServerError> {
    ensure_writable_rel(rel)?;
    // MKCOL is for object fanout dirs (`objects/ab`) in the claimed subset.
    if !(rel.starts_with("objects/") && rel.matches('/').count() == 1) {
        return Ok(text(403, b"mkcol path not claimed"));
    }
    let path = git_dir.join(rel);
    if path.exists() {
        return Ok(text(405, b"exists"));
    }
    fs::create_dir_all(&path)?;
    Ok(text(201, b""))
}

fn put_resource(
    repo: &mut Repository,
    rel: &str,
    body: &[u8],
    token: Option<&str>,
) -> Result<HttpResponse, ServerError> {
    ensure_writable_rel(rel)?;
    if let Some(resp) = lock_conflict(repo.git_dir(), rel, token)? {
        return Ok(resp);
    }
    if let Some(rest) = rel.strip_prefix("objects/") {
        return put_loose_object(repo.git_dir(), rest, body);
    }
    if rel.starts_with("refs/") {
        return put_ref(repo, rel, body);
    }
    if rel == "info/refs" {
        let path = repo.git_dir().join("info/refs");
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        atomic_write(&path, body)?;
        return Ok(text(201, b""));
    }
    Ok(text(403, b"put path not claimed"))
}

fn put_loose_object(git_dir: &Path, rest: &str, body: &[u8]) -> Result<HttpResponse, ServerError> {
    // Final path `ab/cdef…` or temp `ab/cdef…_<suffix>` (git-http-push PUT+MOVE).
    let parts: Vec<&str> = rest.split('/').collect();
    if parts.len() != 2 || parts[0].len() != 2 {
        return Ok(text(403, b"bad loose path"));
    }
    let fanout = parts[0];
    let name = parts[1];
    if !fanout.chars().all(|c| c.is_ascii_hexdigit()) {
        return Ok(text(403, b"bad fanout"));
    }
    let dir = git_dir.join("objects").join(fanout);
    fs::create_dir_all(&dir)?;
    let path = dir.join(name);
    if path.is_file() {
        // Content-addressed: leave existing object in place.
        return Ok(text(200, b""));
    }
    atomic_write(&path, body)?;
    Ok(text(201, b""))
}

fn put_ref(repo: &mut Repository, rel: &str, body: &[u8]) -> Result<HttpResponse, ServerError> {
    let text_body = std::str::from_utf8(body)
        .map_err(|e| ServerError::Protocol(format!("ref body utf-8: {e}")))?;
    let oid_hex = text_body.trim();
    let oid = Oid::from_hex(oid_hex).map_err(|e| ServerError::Protocol(format!("ref oid: {e}")))?;
    repo.refs_mut()
        .update(rel, &oid, None)
        .map_err(|e| ServerError::Msg(e.to_string()))?;
    update_server_info(repo)?;
    Ok(text(201, b""))
}

fn delete_resource(
    repo: &mut Repository,
    rel: &str,
    token: Option<&str>,
) -> Result<HttpResponse, ServerError> {
    ensure_writable_rel(rel)?;
    if !rel.starts_with("refs/") {
        return Ok(text(403, b"delete path not claimed"));
    }
    if let Some(resp) = lock_conflict(repo.git_dir(), rel, token)? {
        return Ok(resp);
    }
    // git-http-push refuses deleting the remote HEAD target.
    if let Ok(RefTarget::Symbolic(head_ref)) = repo.head_target() {
        if head_ref == rel {
            return Ok(text(403, b"cannot delete current HEAD"));
        }
    }
    match repo.refs_mut().delete(rel, None) {
        Ok(()) => {
            update_server_info(repo)?;
            Ok(text(204, b""))
        }
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("not found")
                || msg.contains("missing")
                || msg.contains("does not exist")
            {
                Ok(text(404, b"ref missing"))
            } else {
                Err(ServerError::Msg(msg))
            }
        }
    }
}

fn move_resource(
    git_dir: &Path,
    rel: &str,
    destination: Option<&str>,
) -> Result<HttpResponse, ServerError> {
    ensure_writable_rel(rel)?;
    let dest_url =
        destination.ok_or_else(|| ServerError::Protocol("MOVE missing Destination".into()))?;
    let dest_rel = destination_to_rel(dest_url)?;
    ensure_writable_rel(&dest_rel)?;
    let src = git_dir.join(rel);
    let dst = git_dir.join(&dest_rel);
    if !src.is_file() {
        return Ok(text(404, b"move source missing"));
    }
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)?;
    }
    if dst.exists() {
        // Loose objects are content-addressed; treat existing dest as success.
        let _ = fs::remove_file(&src);
        return Ok(text(204, b""));
    }
    fs::rename(&src, &dst)?;
    Ok(text(201, b""))
}

fn lock_conflict(
    git_dir: &Path,
    rel: &str,
    token: Option<&str>,
) -> Result<Option<HttpResponse>, ServerError> {
    let lock_file = lock_file_path(git_dir, rel)?;
    if !lock_file.is_file() {
        return Ok(None);
    }
    let stored = fs::read_to_string(&lock_file).unwrap_or_default();
    let stored = stored.trim();
    let given = token.map(strip_lock_brackets).unwrap_or("");
    if given.is_empty() || given != stored {
        return Ok(Some(text(423, b"locked")));
    }
    Ok(None)
}

fn repo_rel_path(path: &str) -> Result<String, ServerError> {
    let mut rel = path.trim_start_matches('/').to_string();
    // Strip an optional leading repo segment before known roots.
    for marker in ["objects/", "refs/", "info/"] {
        if let Some(idx) = rel.find(marker) {
            rel = rel[idx..].to_string();
            break;
        }
    }
    if rel.is_empty() {
        return Ok(String::new());
    }
    ensure_writable_rel(&rel)?;
    Ok(rel)
}

fn destination_to_rel(destination: &str) -> Result<String, ServerError> {
    // Destination may be an absolute URL or an absolute path.
    if let Some(idx) = destination.find("://") {
        let after = &destination[idx + 3..];
        if let Some(slash) = after.find('/') {
            return repo_rel_path(&after[slash..]);
        }
        return Err(ServerError::Protocol("bad Destination URL".into()));
    }
    repo_rel_path(destination)
}

fn ensure_writable_rel(rel: &str) -> Result<(), ServerError> {
    if rel.is_empty() {
        return Ok(());
    }
    let path = Path::new(rel);
    if path.components().any(|c| {
        matches!(
            c,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return Err(ServerError::Protocol("dav path escape".into()));
    }
    if !(rel.starts_with("objects/") || rel.starts_with("refs/") || rel.starts_with("info/")) {
        return Err(ServerError::Protocol(format!(
            "dav path not claimed: {rel}"
        )));
    }
    Ok(())
}

fn lock_file_path(git_dir: &Path, rel: &str) -> Result<PathBuf, ServerError> {
    let safe: String = rel
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    Ok(git_dir.join("arks-dav-locks").join(safe))
}

fn new_lock_token() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("opaquelocktoken:arks-{nanos:x}")
}

fn strip_lock_brackets(token: &str) -> &str {
    token.trim().trim_start_matches('<').trim_end_matches('>')
}

fn atomic_write(path: &Path, data: &[u8]) -> Result<(), ServerError> {
    let tmp = PathBuf::from(format!("{}.tmp-arks-dav", path.display()));
    fs::write(&tmp, data)?;
    fs::rename(&tmp, path)?;
    Ok(())
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
