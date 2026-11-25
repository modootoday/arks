//! Public keys held by the forge: user SSH keys (authentication + signing), user
//! OpenPGP keys, and per-repository deploy keys.
//!
//! Routes: `/user/keys[/{id}]`, `/user/ssh_signing_keys[/{id}]`,
//! `/user/gpg_keys[/{id}]`, `/repos/{owner}/{repo}/keys[/{id}]`.
//! Every key is checked with [`crate::key_formats`] before it is stored.

use super::{
    Req, RouteResult, invalid, iso8601, method_not_allowed, next_id, no_content, not_found,
    repo_exists, segments, str_field,
};
use crate::key_formats::{parse_pgp_public_key, parse_ssh_public_key};
use crate::store::{HubStore, now_secs};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub const SSH_AUTH: &str = "authentication";
pub const SSH_SIGNING: &str = "signing";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshKey {
    pub id: u64,
    pub title: String,
    pub key: String,
    /// `authentication` or `signing` (empty in older db files = authentication).
    #[serde(default)]
    pub key_type: String,
    #[serde(default)]
    pub fingerprint: String,
    #[serde(default)]
    pub created_at: u64,
    /// Account holding the key (empty in older db files = the forge's default user).
    #[serde(default)]
    pub owner: String,
}

impl SshKey {
    fn kind(&self) -> &str {
        if self.key_type.is_empty() {
            SSH_AUTH
        } else {
            &self.key_type
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpgKey {
    pub id: u64,
    pub name: String,
    pub key_id: String,
    #[serde(default)]
    pub fingerprint: String,
    #[serde(default)]
    pub emails: Vec<String>,
    /// Armored public key block as supplied.
    #[serde(default)]
    pub raw_key: String,
    #[serde(default)]
    pub created_at: u64,
    /// Account holding the key (empty in older db files = the forge's default user).
    #[serde(default)]
    pub owner: String,
}

/// Whether a user key stored for `owner` belongs to the request's account.
fn mine(st: &HubStore, owner: &str) -> bool {
    let owner = if owner.is_empty() {
        st.db.user_login.as_str()
    } else {
        owner
    };
    owner == st.actor
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployKey {
    pub id: u64,
    pub owner: String,
    pub repo: String,
    pub title: String,
    pub key: String,
    pub read_only: bool,
    pub fingerprint: String,
    pub created_at: u64,
}

fn ssh_json(k: &SshKey) -> Value {
    json!({
        "id": k.id,
        "title": k.title,
        "key": k.key,
        "key_type": k.kind(),
        "fingerprint": k.fingerprint,
        "created_at": iso8601(k.created_at),
    })
}

fn gpg_json(k: &GpgKey) -> Value {
    let emails: Vec<Value> = k
        .emails
        .iter()
        .map(|e| json!({"email": e, "verified": false}))
        .collect();
    json!({
        "id": k.id,
        "name": k.name,
        "key_id": k.key_id,
        "fingerprint": k.fingerprint,
        "emails": emails,
        "raw_key": k.raw_key,
        "created_at": iso8601(k.created_at),
        // Expiry lives in self-signature subpackets, which arks does not evaluate.
        "expires_at": Value::Null,
    })
}

fn deploy_json(k: &DeployKey) -> Value {
    json!({
        "id": k.id,
        "title": k.title,
        "key": k.key,
        "read_only": k.read_only,
        "fingerprint": k.fingerprint,
        "created_at": iso8601(k.created_at),
        "verified": true,
    })
}

fn parse_id(s: &str) -> Option<u64> {
    s.parse().ok()
}

pub(crate) fn route(st: &mut HubStore, req: &Req<'_>) -> RouteResult {
    if let Some(seg) =
        segments(req.path, "/user/keys/").or_else(|| (req.path == "/user/keys").then(Vec::new))
    {
        return ssh_route(st, req, &seg, SSH_AUTH);
    }
    if let Some(seg) = segments(req.path, "/user/ssh_signing_keys/")
        .or_else(|| (req.path == "/user/ssh_signing_keys").then(Vec::new))
    {
        return ssh_route(st, req, &seg, SSH_SIGNING);
    }
    if let Some(seg) = segments(req.path, "/user/gpg_keys/")
        .or_else(|| (req.path == "/user/gpg_keys").then(Vec::new))
    {
        return gpg_route(st, req, &seg);
    }
    if let Some(seg) = segments(req.path, "/repos/") {
        if seg.len() >= 3 && seg[2] == "keys" {
            return deploy_route(st, req, seg[0], seg[1], &seg[3..]);
        }
    }
    Ok(None)
}

fn ssh_route(st: &mut HubStore, req: &Req<'_>, seg: &[&str], kind: &str) -> RouteResult {
    match (req.method, seg) {
        ("GET", []) => {
            let arr = st
                .db
                .ssh_keys
                .iter()
                .filter(|k| k.kind() == kind && mine(st, &k.owner))
                .map(ssh_json)
                .collect();
            Ok(Some((200, Value::Array(arr))))
        }
        ("POST", []) => {
            let v = req.json();
            let raw = v.get("key").and_then(Value::as_str).unwrap_or("");
            let parsed = match parse_ssh_public_key(raw) {
                Ok(p) => p,
                Err(e) => return Ok(Some(invalid(format!("key is invalid: {e}")))),
            };
            let normalized = parsed.normalized();
            if st
                .db
                .ssh_keys
                .iter()
                .any(|k| k.kind() == kind && k.key == normalized)
            {
                return Ok(Some(invalid("key is already in use")));
            }
            let title = str_field(&v, "title")
                .unwrap_or(&parsed.comment)
                .to_string();
            let k = SshKey {
                id: next_id(&mut st.db.next_ssh_key),
                title,
                key: normalized,
                key_type: kind.to_string(),
                fingerprint: parsed.fingerprint,
                created_at: now_secs(),
                owner: st.actor.clone(),
            };
            st.db.ssh_keys.push(k.clone());
            st.save()?;
            Ok(Some((201, ssh_json(&k))))
        }
        ("GET", [id]) => Ok(Some(
            st.db
                .ssh_keys
                .iter()
                .find(|k| k.kind() == kind && Some(k.id) == parse_id(id) && mine(st, &k.owner))
                .map(|k| (200, ssh_json(k)))
                .unwrap_or_else(not_found),
        )),
        ("DELETE", [id]) => {
            let Some(pos) =
                st.db.ssh_keys.iter().position(|k| {
                    k.kind() == kind && Some(k.id) == parse_id(id) && mine(st, &k.owner)
                })
            else {
                return Ok(Some(not_found()));
            };
            st.db.ssh_keys.remove(pos);
            st.save()?;
            Ok(Some(no_content()))
        }
        _ => Ok(Some(method_not_allowed())),
    }
}

fn gpg_route(st: &mut HubStore, req: &Req<'_>, seg: &[&str]) -> RouteResult {
    match (req.method, seg) {
        ("GET", []) => {
            let arr = st
                .db
                .gpg_keys
                .iter()
                .filter(|k| mine(st, &k.owner))
                .map(gpg_json)
                .collect();
            Ok(Some((200, Value::Array(arr))))
        }
        ("POST", []) => {
            let v = req.json();
            let raw = v
                .get("armored_public_key")
                .and_then(Value::as_str)
                .unwrap_or("");
            let parsed = match parse_pgp_public_key(raw) {
                Ok(p) => p,
                Err(e) => return Ok(Some(invalid(format!("armored_public_key is invalid: {e}")))),
            };
            if st.db.gpg_keys.iter().any(|k| k.key_id == parsed.key_id) {
                return Ok(Some(invalid("key_id already exists")));
            }
            let k = GpgKey {
                id: next_id(&mut st.db.next_gpg_key),
                name: str_field(&v, "name").unwrap_or("").to_string(),
                key_id: parsed.key_id,
                fingerprint: parsed.fingerprint,
                emails: parsed.emails,
                raw_key: parsed.armored,
                created_at: now_secs(),
                owner: st.actor.clone(),
            };
            st.db.gpg_keys.push(k.clone());
            st.save()?;
            Ok(Some((201, gpg_json(&k))))
        }
        ("GET", [id]) => Ok(Some(
            st.db
                .gpg_keys
                .iter()
                .find(|k| Some(k.id) == parse_id(id) && mine(st, &k.owner))
                .map(|k| (200, gpg_json(k)))
                .unwrap_or_else(not_found),
        )),
        ("DELETE", [id]) => {
            let Some(pos) = st
                .db
                .gpg_keys
                .iter()
                .position(|k| Some(k.id) == parse_id(id) && mine(st, &k.owner))
            else {
                return Ok(Some(not_found()));
            };
            st.db.gpg_keys.remove(pos);
            st.save()?;
            Ok(Some(no_content()))
        }
        _ => Ok(Some(method_not_allowed())),
    }
}

fn deploy_route(
    st: &mut HubStore,
    req: &Req<'_>,
    owner: &str,
    repo: &str,
    seg: &[&str],
) -> RouteResult {
    if !repo_exists(st, owner, repo) {
        return Ok(Some(not_found()));
    }
    let in_repo = |k: &DeployKey| k.owner == owner && k.repo == repo;
    match (req.method, seg) {
        ("GET", []) => {
            let arr = st
                .db
                .deploy_keys
                .iter()
                .filter(|k| in_repo(k))
                .map(deploy_json)
                .collect();
            Ok(Some((200, Value::Array(arr))))
        }
        ("POST", []) => {
            let v = req.json();
            let raw = v.get("key").and_then(Value::as_str).unwrap_or("");
            let parsed = match parse_ssh_public_key(raw) {
                Ok(p) => p,
                Err(e) => return Ok(Some(invalid(format!("key is invalid: {e}")))),
            };
            let normalized = parsed.normalized();
            // A deploy key identifies exactly one repository, so it is unique forge-wide.
            if st.db.deploy_keys.iter().any(|k| k.key == normalized) {
                return Ok(Some(invalid("key is already in use")));
            }
            let k = DeployKey {
                id: next_id(&mut st.db.next_deploy_key),
                owner: owner.into(),
                repo: repo.into(),
                title: str_field(&v, "title")
                    .unwrap_or(&parsed.comment)
                    .to_string(),
                key: normalized,
                read_only: v.get("read_only").and_then(Value::as_bool).unwrap_or(true),
                fingerprint: parsed.fingerprint,
                created_at: now_secs(),
            };
            st.db.deploy_keys.push(k.clone());
            st.save()?;
            Ok(Some((201, deploy_json(&k))))
        }
        ("GET", [id]) => Ok(Some(
            st.db
                .deploy_keys
                .iter()
                .find(|k| in_repo(k) && Some(k.id) == parse_id(id))
                .map(|k| (200, deploy_json(k)))
                .unwrap_or_else(not_found),
        )),
        ("DELETE", [id]) => {
            let before = st.db.deploy_keys.len();
            st.db
                .deploy_keys
                .retain(|k| !(k.owner == owner && k.repo == repo && Some(k.id) == parse_id(id)));
            if st.db.deploy_keys.len() == before {
                return Ok(Some(not_found()));
            }
            st.save()?;
            Ok(Some(no_content()))
        }
        _ => Ok(Some(method_not_allowed())),
    }
}
