//! Issue artifacts: named documents and links attached to an issue, with an edit
//! history of versions and image/video attachments (`issue artifact *`).
//! arks-owned primitive; REST shape
//! `/repos/{o}/{r}/issues/{n}/artifacts[/{number}[/attachments/{id}]]`.
//!
//! Types: `generic`, `plan`, `embedded_html` (documents) and `link` (an http(s)
//! URL). Every edit appends a version; `restore_version` copies an old version's
//! name and body into a new one. Attachment bytes live under
//! `<data>/artifact-attachments/`.

use super::{
    Req, Resp, RouteResult, invalid, iso8601, next_id, no_content, not_found, segments, str_field,
};
use crate::store::{HubStore, now_secs};
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactVersion {
    pub version: u64,
    pub name: String,
    pub body: String,
    pub actor: String,
    pub at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub id: u64,
    pub name: String,
    #[serde(default)]
    pub alt: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub id: u64,
    pub owner: String,
    pub repo: String,
    pub issue: u64,
    pub number: u64,
    pub kind: String,
    pub creator: String,
    pub created_at: u64,
    pub versions: Vec<ArtifactVersion>,
    #[serde(default)]
    pub attachments: Vec<Attachment>,
}

pub(crate) const TYPES: &[&str] = &["generic", "plan", "embedded_html", "link"];

/// Characters an artifact name may not contain (they are dropped, like gh).
fn clean_name(s: &str) -> String {
    let kept: String = s
        .chars()
        .filter(|c| !"()[]{}<>\\|\"".contains(*c))
        .collect();
    kept.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn is_url(s: &str) -> bool {
    s.starts_with("https://") || s.starts_with("http://")
}

fn current(a: &Artifact) -> &ArtifactVersion {
    a.versions.last().expect("artifact has a version")
}

fn att_dir(st: &HubStore) -> std::path::PathBuf {
    st.data_root().join("artifact-attachments")
}

pub(crate) fn artifact_json(st: &HubStore, a: &Artifact, version: Option<u64>) -> Option<Value> {
    let v = match version {
        Some(n) if n > 0 => a.versions.iter().find(|x| x.version == n)?,
        _ => current(a),
    };
    let full = format!("{}/{}", a.owner, a.repo);
    let url = format!(
        "{}/{full}/issues/{}/artifacts/{}",
        st.public_base, a.issue, a.number
    );
    let html = if a.kind == "embedded_html" {
        v.body.clone()
    } else {
        String::new()
    };
    Some(json!({
        "id": a.id,
        "number": a.number,
        "name": v.name,
        "type": a.kind,
        "body": v.body,
        "bodyHtml": html,
        "description": "",
        "creator": {"login": a.creator},
        "updatedByActor": {"login": v.actor},
        "createdAt": iso8601(a.created_at),
        "updatedAt": iso8601(v.at),
        "url": url,
        "version": v.version,
        "attachments": a.attachments.iter().map(|t| json!({"id": t.id, "name": t.name, "alt": t.alt, "size": t.size,
            "url": format!("{}/repos/{full}/issues/{}/artifacts/{}/attachments/{}", st.public_base, a.issue, a.number, t.id)})).collect::<Vec<_>>(),
        "versions": a.versions.iter().rev().map(|x| json!({"version": x.version, "name": x.name, "actor": {"login": x.actor}, "createdAt": iso8601(x.at)})).collect::<Vec<_>>(),
    }))
}

fn add_attachments(st: &mut HubStore, a: &mut Artifact, v: &Value) -> Result<Vec<String>, String> {
    let mut refs = Vec::new();
    for item in v
        .get("attachments")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let name = str_field(item, "name").unwrap_or("attachment").to_string();
        let alt = item
            .get("alt")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(item.get("content").and_then(Value::as_str).unwrap_or(""))
            .map_err(|e| format!("attachment {name}: {e}"))?;
        let id = next_id(&mut st.db.next_asset);
        let dir = att_dir(st);
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        fs::write(dir.join(format!("{id}.bin")), &bytes).map_err(|e| e.to_string())?;
        refs.push(format!("![{alt}](attachments/{id}/{name})"));
        a.attachments.push(Attachment {
            id,
            name,
            alt,
            size: bytes.len() as u64,
        });
    }
    Ok(refs)
}

fn create(
    st: &mut HubStore,
    owner: &str,
    repo: &str,
    issue: u64,
    v: &Value,
) -> Result<Resp, String> {
    let body = v
        .get("body")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let kind = str_field(v, "type").map(str::to_string).unwrap_or_else(|| {
        if is_url(&body) {
            "link".into()
        } else {
            "generic".into()
        }
    });
    if !TYPES.contains(&kind.as_str()) {
        return Ok(invalid(format!("invalid artifact type {kind:?}")));
    }
    if kind == "link" && !is_url(body.trim()) {
        return Ok(invalid("a link artifact needs an http(s) URL"));
    }
    let name = clean_name(str_field(v, "name").unwrap_or_else(|| {
        if kind == "link" {
            body.split('/').nth(2).unwrap_or("link")
        } else {
            "document"
        }
    }));
    if name.is_empty() {
        return Ok(invalid("artifact name is empty"));
    }
    let number = st
        .db
        .artifacts
        .iter()
        .filter(|a| a.owner == owner && a.repo == repo && a.issue == issue)
        .map(|a| a.number)
        .max()
        .unwrap_or(0)
        + 1;
    let now = now_secs();
    let mut a = Artifact {
        id: next_id(&mut st.db.next_artifact),
        owner: owner.into(),
        repo: repo.into(),
        issue,
        number,
        kind,
        creator: st.actor.clone(),
        created_at: now,
        versions: Vec::new(),
        attachments: Vec::new(),
    };
    let refs = add_attachments(st, &mut a, v)?;
    let mut body = body.trim_end().to_string();
    for r in refs {
        body.push_str(&format!("\n\n{r}"));
    }
    a.versions.push(ArtifactVersion {
        version: 1,
        name,
        body,
        actor: st.actor.clone(),
        at: now,
    });
    st.db.artifacts.push(a.clone());
    st.save()?;
    Ok((201, artifact_json(st, &a, None).unwrap_or(Value::Null)))
}

pub(crate) fn route(st: &mut HubStore, req: &Req<'_>) -> RouteResult {
    let Some(seg) = segments(req.path, "/repos/") else {
        return Ok(None);
    };
    if seg.len() < 5 || seg[2] != "issues" || seg[4] != "artifacts" {
        return Ok(None);
    }
    let (owner, repo) = (seg[0].to_string(), seg[1].to_string());
    let Ok(issue) = seg[3].parse::<u64>() else {
        return Ok(Some(not_found()));
    };
    if !st
        .db
        .issues
        .iter()
        .any(|i| i.owner == owner && i.repo == repo && i.number == issue)
    {
        return Ok(Some((
            404,
            json!({"message": format!("issue #{issue} not found in {owner}/{repo}")}),
        )));
    }
    let v = req.json();
    match (req.method, &seg[5..]) {
        ("GET", []) => {
            let kind = req.query_param("type");
            let items: Vec<Value> = st
                .db
                .artifacts
                .iter()
                .filter(|a| a.owner == owner && a.repo == repo && a.issue == issue)
                .filter(|a| kind.as_deref().is_none_or(|k| a.kind == k))
                .filter_map(|a| artifact_json(st, a, None))
                .collect();
            Ok(Some((200, Value::Array(items))))
        }
        ("POST", []) => create(st, &owner, &repo, issue, &v).map(Some),
        (m, [num, tail @ ..]) => {
            let Some(idx) = num.parse::<u64>().ok().and_then(|n| {
                st.db.artifacts.iter().position(|a| {
                    a.owner == owner && a.repo == repo && a.issue == issue && a.number == n
                })
            }) else {
                return Ok(Some((
                    404,
                    json!({"message": format!("artifact {num} not found on issue #{issue}")}),
                )));
            };
            match (m, tail) {
                ("GET", []) => {
                    let version = req.query_param("version").and_then(|x| x.parse().ok());
                    Ok(Some(
                        match artifact_json(st, &st.db.artifacts[idx], version) {
                            Some(j) => (200, j),
                            None => (
                                404,
                                json!({"message": format!("version {} not found", version.unwrap_or(0))}),
                            ),
                        },
                    ))
                }
                ("PATCH", []) => {
                    let mut a = st.db.artifacts[idx].clone();
                    let cur = current(&a).clone();
                    let (mut name, mut body) = (cur.name.clone(), cur.body.clone());
                    if let Some(rv) = v
                        .get("restore_version")
                        .and_then(Value::as_u64)
                        .filter(|n| *n > 0)
                    {
                        let Some(old) = a.versions.iter().find(|x| x.version == rv) else {
                            return Ok(Some(invalid(format!("version {rv} not found"))));
                        };
                        name = old.name.clone();
                        body = old.body.clone();
                    }
                    if let Some(n) = str_field(&v, "name") {
                        name = clean_name(n);
                    }
                    if let Some(b) = v.get("body").and_then(Value::as_str) {
                        if a.kind == "link" && !is_url(b.trim()) {
                            return Ok(Some(invalid(
                                "a link's new content must be an http(s) URL",
                            )));
                        }
                        body = b.trim_end().to_string();
                    }
                    for r in add_attachments(st, &mut a, &v)? {
                        body.push_str(&format!("\n\n{r}"));
                    }
                    let version = cur.version + 1;
                    a.versions.push(ArtifactVersion {
                        version,
                        name,
                        body,
                        actor: st.actor.clone(),
                        at: now_secs(),
                    });
                    st.db.artifacts[idx] = a.clone();
                    st.save()?;
                    Ok(Some((
                        200,
                        artifact_json(st, &a, None).unwrap_or(Value::Null),
                    )))
                }
                ("DELETE", []) => {
                    let a = st.db.artifacts.remove(idx);
                    for t in a.attachments {
                        let _ = fs::remove_file(att_dir(st).join(format!("{}.bin", t.id)));
                    }
                    st.save()?;
                    Ok(Some(no_content()))
                }
                ("GET", ["attachments", id]) => {
                    let a = &st.db.artifacts[idx];
                    let Some(t) = id
                        .parse::<u64>()
                        .ok()
                        .and_then(|id| a.attachments.iter().find(|t| t.id == id))
                    else {
                        return Ok(Some(not_found()));
                    };
                    let bytes =
                        fs::read(att_dir(st).join(format!("{}.bin", t.id))).unwrap_or_default();
                    Ok(Some((
                        200,
                        json!({"__bytes_b64": base64::engine::general_purpose::STANDARD.encode(bytes), "__content_type": "application/octet-stream"}),
                    )))
                }
                _ => Ok(None),
            }
        }
        _ => Ok(None),
    }
}
