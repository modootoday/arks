//! Git LFS Batch API + Basic Transfer client over `ureq` 2.x + `arks-net::HttpsPolicy`.
//!
//! Primary reference: <https://github.com/git-lfs/git-lfs/blob/main/docs/api/batch.md>
//! and <https://github.com/git-lfs/git-lfs/blob/main/docs/api/basic-transfers.md>.
//! Spec: `sot:root/v0-2-0-scope-expansion` §2.7 (E6).

use crate::lfs::LfsError;
use crate::lfs::pointer::LfsPointer;
use crate::lfs::store::LfsStore;
use arks_net::HttpsPolicy;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::io::{Cursor, Read};

pub const LFS_MEDIA_TYPE: &str = "application/vnd.git-lfs+json";

/// Derive the canonical Git LFS HTTP(S) endpoint URL from a Git remote URL.
///
/// Per Git LFS spec: `<remote>.git/info/lfs` (or `<remote>/info/lfs` if already `.git`).
pub fn endpoint_from_remote_url(remote_url: &str) -> String {
    let trimmed = remote_url.trim_end_matches('/');
    if trimmed.ends_with("/info/lfs") {
        trimmed.to_string()
    } else if trimmed.ends_with(".git") {
        format!("{trimmed}/info/lfs")
    } else {
        format!("{trimmed}.git/info/lfs")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatchOperation {
    Download,
    Upload,
}

impl BatchOperation {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Download => "download",
            Self::Upload => "upload",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferAction {
    pub href: String,
    pub headers: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchObjectResponse {
    pub oid: String,
    pub size: u64,
    pub upload: Option<TransferAction>,
    pub download: Option<TransferAction>,
    pub verify: Option<TransferAction>,
    pub error: Option<(i64, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchResponse {
    pub transfer: String,
    pub objects: Vec<BatchObjectResponse>,
}

fn build_agent(endpoint: &str, policy: &HttpsPolicy) -> Result<ureq::Agent, LfsError> {
    let host = extract_host(endpoint);
    let builder = policy
        .apply(ureq::AgentBuilder::new(), &host)
        .map_err(LfsError::Http)?;
    Ok(builder.build())
}

fn extract_host(url: &str) -> String {
    let rest = url.split_once("://").map(|(_, r)| r).unwrap_or(url);
    let authority = rest.split('/').next().unwrap_or("");
    let host_port = authority.rsplit('@').next().unwrap_or(authority);
    host_port.split(':').next().unwrap_or("").to_string()
}

fn map_ureq_response(
    res: Result<ureq::Response, ureq::Error>,
    label: &str,
) -> Result<(u16, ureq::Response), LfsError> {
    match res {
        Ok(r) => Ok((r.status(), r)),
        Err(ureq::Error::Status(code, r)) => Ok((code, r)),
        Err(ureq::Error::Transport(t)) => Err(LfsError::Http(format!("{label}: {t}"))),
    }
}

/// Issue a `POST <endpoint>/objects/batch` request using the Basic transfer adapter.
pub fn batch_request(
    endpoint: &str,
    operation: BatchOperation,
    objects: &[LfsPointer],
    auth_header: Option<&str>,
    policy: &HttpsPolicy,
) -> Result<BatchResponse, LfsError> {
    let url = format!("{}/objects/batch", endpoint.trim_end_matches('/'));
    let agent = build_agent(&url, policy)?;
    let objs_json: Vec<Value> = objects
        .iter()
        .map(|p| json!({ "oid": p.oid, "size": p.size }))
        .collect();
    let payload = json!({
        "operation": operation.as_str(),
        "transfers": ["basic"],
        "objects": objs_json,
        "hash_algo": "sha256",
    })
    .to_string();

    let mut req = agent
        .post(&url)
        .set("Accept", LFS_MEDIA_TYPE)
        .set("Content-Type", LFS_MEDIA_TYPE);
    if let Some(auth) = auth_header {
        req = req.set("Authorization", auth);
    }
    let (status, resp) = map_ureq_response(
        req.send_bytes(payload.as_bytes()),
        &format!("LFS batch POST {url}"),
    )?;
    let body = resp
        .into_string()
        .map_err(|e| LfsError::Http(format!("LFS batch read body: {e}")))?;
    if !(200..300).contains(&status) {
        return Err(LfsError::Http(format!(
            "LFS batch POST {url} failed with HTTP {status}: {body}"
        )));
    }
    let v: Value = serde_json::from_str(&body)
        .map_err(|e| LfsError::Protocol(format!("LFS batch invalid JSON: {e}")))?;
    let transfer = v
        .get("transfer")
        .and_then(Value::as_str)
        .unwrap_or("basic")
        .to_string();
    if transfer != "basic" {
        return Err(LfsError::UnsupportedTransfer(transfer));
    }
    let arr = v
        .get("objects")
        .and_then(Value::as_array)
        .ok_or_else(|| LfsError::Protocol("LFS batch response missing objects array".into()))?;
    let mut parsed = Vec::with_capacity(arr.len());
    for item in arr {
        let oid = item
            .get("oid")
            .and_then(Value::as_str)
            .ok_or_else(|| LfsError::Protocol("LFS batch object missing oid".into()))?
            .to_string();
        let size = item
            .get("size")
            .and_then(Value::as_u64)
            .ok_or_else(|| LfsError::Protocol("LFS batch object missing size".into()))?;
        let actions = item.get("actions");
        let upload = actions.and_then(|a| parse_action(a.get("upload")));
        let download = actions.and_then(|a| parse_action(a.get("download")));
        let verify = actions.and_then(|a| parse_action(a.get("verify")));
        let error = item.get("error").map(|e| {
            let code = e.get("code").and_then(Value::as_i64).unwrap_or(500);
            let msg = e
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("LFS object error")
                .to_string();
            (code, msg)
        });
        parsed.push(BatchObjectResponse {
            oid,
            size,
            upload,
            download,
            verify,
            error,
        });
    }
    Ok(BatchResponse {
        transfer,
        objects: parsed,
    })
}

fn parse_action(v: Option<&Value>) -> Option<TransferAction> {
    let v = v?;
    let href = v.get("href")?.as_str()?.to_string();
    let mut headers = BTreeMap::new();
    if let Some(map) = v.get("header").and_then(Value::as_object) {
        for (k, val) in map {
            if let Some(s) = val.as_str() {
                headers.insert(k.clone(), s.to_string());
            }
        }
    }
    Some(TransferAction { href, headers })
}

/// Upload missing objects from `store` to `endpoint` via Batch API + Basic Transfer (`PUT` + optional `verify`).
/// Returns the count of objects actually uploaded.
pub fn upload_objects(
    store: &LfsStore,
    endpoint: &str,
    objects: &[LfsPointer],
    auth_header: Option<&str>,
    policy: &HttpsPolicy,
) -> Result<usize, LfsError> {
    if objects.is_empty() {
        return Ok(0);
    }
    let batch = batch_request(
        endpoint,
        BatchOperation::Upload,
        objects,
        auth_header,
        policy,
    )?;
    let agent = build_agent(endpoint, policy)?;
    let mut uploaded = 0usize;

    for obj in &batch.objects {
        if let Some((code, msg)) = &obj.error {
            return Err(LfsError::Http(format!(
                "LFS upload error for {} (HTTP {code}): {msg}",
                obj.oid
            )));
        }
        if let Some(act) = &obj.upload {
            let bytes = store.read_verified(&obj.oid, Some(obj.size))?;
            let mut req = agent
                .put(&act.href)
                .set("Content-Type", "application/octet-stream");
            let mut has_auth = false;
            for (k, v) in &act.headers {
                if k.eq_ignore_ascii_case("authorization") {
                    has_auth = true;
                }
                req = req.set(k.as_str(), v.as_str());
            }
            if !has_auth {
                if let Some(auth) = auth_header {
                    req = req.set("Authorization", auth);
                }
            }
            let (status, _) =
                map_ureq_response(req.send_bytes(&bytes), &format!("LFS PUT {}", act.href))?;
            if !(200..300).contains(&status) {
                return Err(LfsError::Http(format!(
                    "LFS PUT {} failed with HTTP {status}",
                    act.href
                )));
            }
            uploaded += 1;
        }
        if let Some(ver) = &obj.verify {
            let body = json!({ "oid": obj.oid, "size": obj.size }).to_string();
            let mut req = agent
                .post(&ver.href)
                .set("Content-Type", LFS_MEDIA_TYPE)
                .set("Accept", LFS_MEDIA_TYPE);
            for (k, v) in &ver.headers {
                req = req.set(k.as_str(), v.as_str());
            }
            let (status, _) = map_ureq_response(
                req.send_bytes(body.as_bytes()),
                &format!("LFS verify {}", ver.href),
            )?;
            if !(200..300).contains(&status) {
                return Err(LfsError::Http(format!(
                    "LFS verify {} failed with HTTP {status}",
                    ver.href
                )));
            }
        }
    }
    Ok(uploaded)
}

/// Download missing objects from `endpoint` into `store` with streaming SHA-256 verification.
/// Returns the count of newly downloaded objects.
pub fn download_objects(
    store: &LfsStore,
    endpoint: &str,
    objects: &[LfsPointer],
    auth_header: Option<&str>,
    policy: &HttpsPolicy,
) -> Result<usize, LfsError> {
    let missing: Vec<LfsPointer> = objects
        .iter()
        .filter(|p| !store.exists(&p.oid))
        .cloned()
        .collect();
    if missing.is_empty() {
        return Ok(0);
    }
    let batch = batch_request(
        endpoint,
        BatchOperation::Download,
        &missing,
        auth_header,
        policy,
    )?;
    let agent = build_agent(endpoint, policy)?;
    let mut downloaded = 0usize;

    for obj in &batch.objects {
        if let Some((code, msg)) = &obj.error {
            return Err(LfsError::Http(format!(
                "LFS download error for {} (HTTP {code}): {msg}",
                obj.oid
            )));
        }
        let Some(act) = &obj.download else {
            continue;
        };
        let mut req = agent.get(&act.href);
        let mut has_auth = false;
        for (k, v) in &act.headers {
            if k.eq_ignore_ascii_case("authorization") {
                has_auth = true;
            }
            req = req.set(k.as_str(), v.as_str());
        }
        if !has_auth {
            if let Some(auth) = auth_header {
                req = req.set("Authorization", auth);
            }
        }
        let (status, resp) = map_ureq_response(req.call(), &format!("LFS GET {}", act.href))?;
        if !(200..300).contains(&status) {
            return Err(LfsError::Http(format!(
                "LFS GET {} failed with HTTP {status}",
                act.href
            )));
        }
        let mut bytes = Vec::new();
        resp.into_reader()
            .read_to_end(&mut bytes)
            .map_err(|e| LfsError::Http(format!("LFS GET read body: {e}")))?;
        store.store_verified_reader(&obj.oid, Some(obj.size), Cursor::new(bytes))?;
        downloaded += 1;
    }
    Ok(downloaded)
}
