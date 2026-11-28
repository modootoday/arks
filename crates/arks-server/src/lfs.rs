//! Native Git LFS server endpoints for `arks serve` (`info/lfs`, E7).
//!
//! Primary references:
//! - <https://github.com/git-lfs/git-lfs/blob/main/docs/api/batch.md>
//! - <https://github.com/git-lfs/git-lfs/blob/main/docs/api/basic-transfers.md>
//!
//! Spec: `sot:root/v0-2-0-scope-expansion` §2.7 (E7).

use crate::error::ServerError;
use crate::http::{HttpRequest, HttpResponse};
use arks_repo::Repository;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;

pub const LFS_MEDIA_TYPE: &str = "application/vnd.git-lfs+json";

/// Returns `true` if `path` targets a Git LFS endpoint under `/info/lfs/`.
pub fn is_lfs_path(path: &str) -> bool {
    path.contains("/info/lfs/")
}

fn is_valid_oid(oid: &str) -> bool {
    oid.len() == 64 && oid.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

fn lfs_object_path(repo: &Repository, oid: &str) -> Result<PathBuf, ServerError> {
    if !is_valid_oid(oid) {
        return Err(ServerError::Protocol(format!("invalid LFS oid: {oid}")));
    }
    Ok(repo
        .common_dir()
        .join("lfs")
        .join("objects")
        .join(&oid[0..2])
        .join(&oid[2..4])
        .join(oid))
}

fn hex_lower(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        use std::fmt::Write as _;
        let _ = write!(&mut s, "{b:02x}");
    }
    s
}

fn json_resp(status: u16, v: &Value) -> HttpResponse {
    HttpResponse {
        status,
        content_type: LFS_MEDIA_TYPE.into(),
        body: v.to_string().into_bytes(),
        www_authenticate: None,
        extra_headers: Vec::new(),
    }
}

/// Dispatch `POST .../info/lfs/objects/batch`, `PUT|GET .../info/lfs/objects/<oid>`,
/// and `POST .../info/lfs/verify` behind `arks-server` auth.
pub fn handle_lfs(
    repo: &Repository,
    req: &HttpRequest,
    scheme: &str,
) -> Result<HttpResponse, ServerError> {
    let Some(idx) = req.path.find("/info/lfs/") else {
        return Ok(json_resp(404, &json!({ "message": "not found" })));
    };
    let repo_prefix = &req.path[..idx];
    let sub = &req.path[idx + "/info/lfs/".len()..];

    if req.method == "POST" && sub == "objects/batch" {
        return handle_batch(repo, req, scheme, repo_prefix);
    }
    if req.method == "POST" && sub == "verify" {
        return handle_verify(repo, req);
    }
    if let Some(oid) = sub.strip_prefix("objects/") {
        if !is_valid_oid(oid) {
            return Ok(json_resp(422, &json!({ "message": "invalid oid" })));
        }
        return match req.method.as_str() {
            "PUT" => handle_put_object(repo, req, oid),
            "GET" => handle_get_object(repo, oid),
            _ => Ok(json_resp(405, &json!({ "message": "method not allowed" }))),
        };
    }

    Ok(json_resp(
        404,
        &json!({ "message": "unknown LFS endpoint" }),
    ))
}

fn handle_batch(
    repo: &Repository,
    req: &HttpRequest,
    scheme: &str,
    repo_prefix: &str,
) -> Result<HttpResponse, ServerError> {
    let v: Value = match serde_json::from_slice(&req.body) {
        Ok(v) => v,
        Err(e) => {
            return Ok(json_resp(
                422,
                &json!({ "message": format!("invalid batch JSON: {e}") }),
            ));
        }
    };
    let op = v.get("operation").and_then(Value::as_str).unwrap_or("");
    if op != "download" && op != "upload" {
        return Ok(json_resp(
            422,
            &json!({ "message": format!("unsupported operation: {op}") }),
        ));
    }
    let Some(objs) = v.get("objects").and_then(Value::as_array) else {
        return Ok(json_resp(
            422,
            &json!({ "message": "missing objects array" }),
        ));
    };

    let host = req.host.as_deref().unwrap_or("127.0.0.1");
    let base_lfs = format!("{scheme}://{host}{repo_prefix}/info/lfs");
    let auth_header = req.authorization.as_deref();

    let mut out_objs = Vec::with_capacity(objs.len());
    for item in objs {
        let Some(oid) = item.get("oid").and_then(Value::as_str) else {
            return Ok(json_resp(422, &json!({ "message": "object missing oid" })));
        };
        let Some(size) = item.get("size").and_then(Value::as_u64) else {
            return Ok(json_resp(422, &json!({ "message": "object missing size" })));
        };
        if !is_valid_oid(oid) {
            out_objs.push(json!({
                "oid": oid,
                "size": size,
                "error": { "code": 422, "message": "invalid oid" }
            }));
            continue;
        }
        let path = lfs_object_path(repo, oid)?;
        let exists = path.is_file();
        let make_action = |href: String| -> Value {
            if let Some(auth) = auth_header {
                json!({ "href": href, "header": { "Authorization": auth } })
            } else {
                json!({ "href": href })
            }
        };

        if op == "upload" {
            if exists {
                // Per Git LFS spec: omit actions when server already holds the object.
                out_objs.push(json!({ "oid": oid, "size": size }));
            } else {
                out_objs.push(json!({
                    "oid": oid,
                    "size": size,
                    "authenticated": true,
                    "actions": {
                        "upload": make_action(format!("{base_lfs}/objects/{oid}")),
                        "verify": make_action(format!("{base_lfs}/verify"))
                    }
                }));
            }
        } else if exists {
            out_objs.push(json!({
                "oid": oid,
                "size": size,
                "authenticated": true,
                "actions": {
                    "download": make_action(format!("{base_lfs}/objects/{oid}"))
                }
            }));
        } else {
            out_objs.push(json!({
                "oid": oid,
                "size": size,
                "error": { "code": 404, "message": "Object does not exist" }
            }));
        }
    }

    Ok(json_resp(
        200,
        &json!({
            "transfer": "basic",
            "objects": out_objs
        }),
    ))
}

fn handle_put_object(
    repo: &Repository,
    req: &HttpRequest,
    oid: &str,
) -> Result<HttpResponse, ServerError> {
    let actual = hex_lower(&Sha256::digest(&req.body));
    if actual != oid {
        return Ok(json_resp(
            422,
            &json!({
                "message": format!("SHA-256 mismatch: expected {oid}, got {actual}")
            }),
        ));
    }
    let dst = lfs_object_path(repo, oid)?;
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)?;
    }
    if !dst.is_file() {
        let tmp = dst.with_extension("tmp");
        fs::write(&tmp, &req.body)?;
        fs::rename(&tmp, &dst)?;
    }
    Ok(HttpResponse {
        status: 200,
        content_type: "application/octet-stream".into(),
        body: Vec::new(),
        www_authenticate: None,
        extra_headers: Vec::new(),
    })
}

fn handle_get_object(repo: &Repository, oid: &str) -> Result<HttpResponse, ServerError> {
    let path = lfs_object_path(repo, oid)?;
    if !path.is_file() {
        return Ok(json_resp(404, &json!({ "message": "Object not found" })));
    }
    let bytes = fs::read(&path)?;
    Ok(HttpResponse {
        status: 200,
        content_type: "application/octet-stream".into(),
        body: bytes,
        www_authenticate: None,
        extra_headers: Vec::new(),
    })
}

fn handle_verify(repo: &Repository, req: &HttpRequest) -> Result<HttpResponse, ServerError> {
    let v: Value = match serde_json::from_slice(&req.body) {
        Ok(v) => v,
        Err(e) => {
            return Ok(json_resp(
                422,
                &json!({ "message": format!("invalid verify JSON: {e}") }),
            ));
        }
    };
    let Some(oid) = v.get("oid").and_then(Value::as_str) else {
        return Ok(json_resp(422, &json!({ "message": "missing oid" })));
    };
    let Some(size) = v.get("size").and_then(Value::as_u64) else {
        return Ok(json_resp(422, &json!({ "message": "missing size" })));
    };
    if !is_valid_oid(oid) {
        return Ok(json_resp(422, &json!({ "message": "invalid oid" })));
    }
    let path = lfs_object_path(repo, oid)?;
    let Ok(meta) = fs::metadata(&path) else {
        return Ok(json_resp(404, &json!({ "message": "Object not found" })));
    };
    if meta.len() != size {
        return Ok(json_resp(
            422,
            &json!({ "message": "Object size mismatch" }),
        ));
    }
    Ok(HttpResponse {
        status: 200,
        content_type: LFS_MEDIA_TYPE.into(),
        body: Vec::new(),
        www_authenticate: None,
        extra_headers: Vec::new(),
    })
}
