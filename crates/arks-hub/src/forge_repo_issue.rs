//! Local forge routes for the C8 repo/issue rows (Partial; host advisory).
//!
//! REST (GitHub reference shapes):
//! - `GET /gitignore/templates[/{name}]` — <https://docs.github.com/en/rest/gitignore/gitignore>
//! - `GET /licenses[/{license}]` — <https://docs.github.com/en/rest/licenses/licenses>
//! - `GET|POST /repos/{o}/{r}/autolinks`, `GET|DELETE …/autolinks/{id}` —
//!   <https://docs.github.com/en/rest/repos/autolinks>
//! - `GET|POST /repos/{o}/{r}/keys`, `GET|DELETE …/keys/{id}` —
//!   <https://docs.github.com/en/rest/deploy-keys/deploy-keys>
//! - `GET /repos/{o}/{r}/contents/{path}` — <https://docs.github.com/en/rest/repos/contents>
//! - `GET|POST /repos/{o}/{r}/issues/{n}/artifacts`, `GET|PATCH|DELETE …/artifacts/{k}` —
//!   <https://docs.github.com/en/rest/issues/artifacts>
//!
//! GraphQL (`POST /graphql`): only the three named operations the arks client
//! sends for `issue develop` (`IssueLinkedBranches`, `IssueDevelopContext`,
//! `CreateLinkedBranch`); fields follow `createLinkedBranch` / `Issue.linkedBranches`
//! (<https://docs.github.com/en/graphql/reference/issues>). This is not a general
//! GraphQL server. Never spawns git/gh.

use crate::forge_catalog;
use crate::forge_git;
use crate::store::HubStore;
use crate::store_repo_issue::{ARTIFACT_TYPES, IssueArtifact};
use arks_oid::Oid;
use arks_repo::Repository;
use serde_json::{Value, json};

pub type Routed = Option<(u16, Value)>;

fn msg(status: u16, m: &str) -> Routed {
    Some((status, json!({ "message": m })))
}

fn query_param(query: &str, key: &str) -> Option<String> {
    query.split('&').find_map(|pair| {
        let (k, v) = pair.split_once('=')?;
        (k == key).then(|| v.to_string())
    })
}

/// Route a request if it belongs to the C8 repo/issue surface; `Ok(None)` otherwise.
pub fn route(
    st: &mut HubStore,
    method: &str,
    path: &str,
    query: &str,
    body: &[u8],
) -> Result<Routed, String> {
    if method == "GET" && path == "/gitignore/templates" {
        return Ok(Some((200, forge_catalog::gitignore_names())));
    }
    if method == "GET"
        && let Some(name) = path.strip_prefix("/gitignore/templates/")
    {
        return Ok(match forge_catalog::gitignore_template(name) {
            Some(v) => Some((200, v)),
            None => msg(404, "Not Found"),
        });
    }
    if method == "GET" && path == "/licenses" {
        return Ok(Some((200, forge_catalog::licenses())));
    }
    if method == "GET"
        && let Some(key) = path.strip_prefix("/licenses/")
    {
        return Ok(match forge_catalog::license(key) {
            Some(v) => Some((200, v)),
            None => msg(404, "Not Found"),
        });
    }
    if method == "POST" && path == "/graphql" {
        return graphql(st, body).map(Some);
    }
    let Some(rest) = path.strip_prefix("/repos/") else {
        return Ok(None);
    };
    let parts: Vec<&str> = rest.split('/').collect();
    if parts.len() < 3 {
        return Ok(None);
    }
    let (owner, repo) = (parts[0], parts[1]);
    match parts[2] {
        "autolinks" => autolinks(st, method, owner, repo, &parts[3..], body),
        "keys" => deploy_keys(st, method, owner, repo, &parts[3..], body),
        "contents" if method == "GET" => {
            if !st.repo_exists(owner, repo) {
                return Ok(msg(404, "Not Found"));
            }
            let rel = parts[3..].join("/");
            let rev = query_param(query, "ref");
            Ok(Some(
                match forge_git::contents(&st.data_root(), owner, repo, &rel, rev.as_deref()) {
                    Ok(v) => (200, v),
                    Err((code, m)) => (code, json!({ "message": m })),
                },
            ))
        }
        "issues" if parts.len() >= 5 && parts[4] == "artifacts" => {
            let Ok(issue) = parts[3].parse::<u64>() else {
                return Ok(None);
            };
            artifacts(st, method, owner, repo, issue, &parts[5..], query, body)
        }
        _ => Ok(None),
    }
}

fn autolink_json(a: &crate::store_repo_issue::Autolink) -> Value {
    json!({
        "id": a.id,
        "key_prefix": a.key_prefix,
        "url_template": a.url_template,
        "is_alphanumeric": a.is_alphanumeric,
    })
}

fn autolinks(
    st: &mut HubStore,
    method: &str,
    owner: &str,
    repo: &str,
    tail: &[&str],
    body: &[u8],
) -> Result<Routed, String> {
    if !st.repo_exists(owner, repo) {
        return Ok(msg(404, "Not Found"));
    }
    match (method, tail) {
        ("GET", []) => {
            let items: Vec<Value> = st
                .db
                .autolinks
                .iter()
                .filter(|a| a.owner == owner && a.repo == repo)
                .map(autolink_json)
                .collect();
            Ok(Some((200, Value::Array(items))))
        }
        ("POST", []) => {
            let v: Value = serde_json::from_slice(body).unwrap_or(json!({}));
            let key = v.get("key_prefix").and_then(Value::as_str).unwrap_or("");
            let tmpl = v.get("url_template").and_then(Value::as_str).unwrap_or("");
            let alnum = v
                .get("is_alphanumeric")
                .and_then(Value::as_bool)
                .unwrap_or(true);
            if key.is_empty() || !tmpl.contains("<num>") {
                return Ok(msg(
                    422,
                    "Validation Failed: key_prefix required and url_template must contain <num>",
                ));
            }
            match st.create_autolink(owner, repo, key, tmpl, alnum) {
                Ok(a) => {
                    st.save()?;
                    Ok(Some((201, autolink_json(&a))))
                }
                Err(e) => Ok(msg(422, &format!("Validation Failed: {e}"))),
            }
        }
        ("GET", [id]) => {
            let id: u64 = id.parse().unwrap_or(0);
            Ok(
                match st
                    .db
                    .autolinks
                    .iter()
                    .find(|a| a.owner == owner && a.repo == repo && a.id == id)
                {
                    Some(a) => Some((200, autolink_json(a))),
                    None => msg(404, "Not Found"),
                },
            )
        }
        ("DELETE", [id]) => {
            let id: u64 = id.parse().unwrap_or(0);
            if st.delete_autolink(owner, repo, id) {
                st.save()?;
                Ok(Some((204, json!({}))))
            } else {
                Ok(msg(404, "Not Found"))
            }
        }
        _ => Ok(None),
    }
}

fn deploy_key_json(k: &crate::store_repo_issue::DeployKey) -> Value {
    json!({
        "id": k.id,
        "key": k.key,
        "url": format!("/repos/{}/{}/keys/{}", k.owner, k.repo, k.id),
        "title": k.title,
        "verified": true,
        "created_at": iso8601(k.created_at),
        "read_only": k.read_only,
    })
}

fn deploy_keys(
    st: &mut HubStore,
    method: &str,
    owner: &str,
    repo: &str,
    tail: &[&str],
    body: &[u8],
) -> Result<Routed, String> {
    if !st.repo_exists(owner, repo) {
        return Ok(msg(404, "Not Found"));
    }
    match (method, tail) {
        ("GET", []) => {
            let items: Vec<Value> = st
                .db
                .deploy_keys
                .iter()
                .filter(|k| k.owner == owner && k.repo == repo)
                .map(deploy_key_json)
                .collect();
            Ok(Some((200, Value::Array(items))))
        }
        ("POST", []) => {
            let v: Value = serde_json::from_slice(body).unwrap_or(json!({}));
            let key = v.get("key").and_then(Value::as_str).unwrap_or("").trim();
            let title = v.get("title").and_then(Value::as_str).unwrap_or("");
            let read_only = v.get("read_only").and_then(Value::as_bool).unwrap_or(false);
            if !looks_like_public_key(key) {
                return Ok(msg(422, "Validation Failed: key is invalid"));
            }
            match st.create_deploy_key(owner, repo, title, key, read_only) {
                Ok(k) => {
                    st.save()?;
                    Ok(Some((201, deploy_key_json(&k))))
                }
                Err(e) => Ok(msg(422, &format!("Validation Failed: {e}"))),
            }
        }
        ("GET", [id]) => {
            let id: u64 = id.parse().unwrap_or(0);
            Ok(
                match st
                    .db
                    .deploy_keys
                    .iter()
                    .find(|k| k.owner == owner && k.repo == repo && k.id == id)
                {
                    Some(k) => Some((200, deploy_key_json(k))),
                    None => msg(404, "Not Found"),
                },
            )
        }
        ("DELETE", [id]) => {
            let id: u64 = id.parse().unwrap_or(0);
            if st.delete_deploy_key(owner, repo, id) {
                st.save()?;
                Ok(Some((204, json!({}))))
            } else {
                Ok(msg(404, "Not Found"))
            }
        }
        _ => Ok(None),
    }
}

/// OpenSSH public key line: `<type> <base64> [comment]` (authorized_keys format).
fn looks_like_public_key(key: &str) -> bool {
    let mut it = key.split_whitespace();
    let (Some(kind), Some(blob)) = (it.next(), it.next()) else {
        return false;
    };
    (kind.starts_with("ssh-") || kind.starts_with("ecdsa-") || kind.starts_with("sk-"))
        && blob.len() > 16
        && blob
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'+' || b == b'/' || b == b'=')
}

fn artifact_json(a: &IssueArtifact) -> Value {
    json!({
        "id": a.id,
        "html_url": format!("/{}/{}/issues/{}/artifacts/{}", a.owner, a.repo, a.issue, a.number),
        "number": a.number,
        "type": a.kind,
        "name": a.name,
        "body": a.body,
        "body_html": Value::Null,
        "description": "",
        "creator": {"login": a.creator},
        "updated_by_actor": {"login": a.versions.last().map(|v| v.actor.as_str()).unwrap_or("")},
        "created_at": iso8601(a.created_at),
        "updated_at": iso8601(a.updated_at),
    })
}

#[allow(clippy::too_many_arguments)]
fn artifacts(
    st: &mut HubStore,
    method: &str,
    owner: &str,
    repo: &str,
    issue: u64,
    tail: &[&str],
    query: &str,
    body: &[u8],
) -> Result<Routed, String> {
    if !st.issue_exists(owner, repo, issue) {
        return Ok(msg(404, "Not Found"));
    }
    match (method, tail) {
        ("GET", []) => {
            let want = query_param(query, "type");
            let mut items: Vec<&IssueArtifact> = st
                .db
                .issue_artifacts
                .iter()
                .filter(|a| a.owner == owner && a.repo == repo && a.issue == issue)
                .filter(|a| want.as_deref().is_none_or(|t| a.kind == t))
                .collect();
            items.sort_by_key(|a| a.number);
            Ok(Some((
                200,
                Value::Array(items.into_iter().map(artifact_json).collect()),
            )))
        }
        ("POST", []) => {
            let v: Value = serde_json::from_slice(body).unwrap_or(json!({}));
            let name = v.get("name").and_then(Value::as_str);
            let text = v.get("body").and_then(Value::as_str);
            let kind = v.get("type").and_then(Value::as_str).unwrap_or("generic");
            let (Some(name), Some(text)) = (name, text) else {
                return Ok(msg(422, "Validation Failed: name and body are required"));
            };
            if name.is_empty() || !ARTIFACT_TYPES.contains(&kind) {
                return Ok(msg(422, "Validation Failed: invalid name or type"));
            }
            let a = st.create_issue_artifact(owner, repo, issue, kind, name, text);
            st.save()?;
            Ok(Some((201, artifact_json(&a))))
        }
        (_, [num]) => {
            let Ok(num) = num.parse::<u64>() else {
                return Ok(msg(404, "Not Found"));
            };
            match method {
                "GET" => Ok(match st.find_issue_artifact(owner, repo, issue, num) {
                    Some(a) => {
                        let versions: Vec<Value> = a
                            .versions
                            .iter()
                            .rev()
                            .map(|ver| {
                                json!({
                                    "version": ver.version,
                                    "name": ver.name,
                                    "body": ver.body,
                                    "body_html": Value::Null,
                                    "created_at": iso8601(ver.created_at),
                                    "actor": {"login": ver.actor},
                                })
                            })
                            .collect();
                        Some((
                            200,
                            json!({
                                "artifact": artifact_json(a),
                                "has_next_page": false,
                                "total_count": versions.len(),
                                "versions": versions,
                            }),
                        ))
                    }
                    None => msg(404, "Not Found"),
                }),
                "PATCH" => {
                    let v: Value = serde_json::from_slice(body).unwrap_or(json!({}));
                    let name = v.get("name").and_then(Value::as_str);
                    let text = v.get("body").and_then(Value::as_str);
                    match st.update_issue_artifact(owner, repo, issue, num, name, text) {
                        Some(a) => {
                            st.save()?;
                            Ok(Some((200, artifact_json(&a))))
                        }
                        None => Ok(msg(404, "Not Found")),
                    }
                }
                "DELETE" => {
                    if st.delete_issue_artifact(owner, repo, issue, num) {
                        st.save()?;
                        Ok(Some((204, json!({}))))
                    } else {
                        Ok(msg(404, "Not Found"))
                    }
                }
                _ => Ok(None),
            }
        }
        _ => Ok(None),
    }
}

fn gql_error(m: &str) -> (u16, Value) {
    (
        200,
        json!({"data": Value::Null, "errors": [{"message": m}]}),
    )
}

fn repo_node_id(owner: &str, name: &str) -> String {
    format!("R_{owner}/{name}")
}

fn graphql(st: &mut HubStore, body: &[u8]) -> Result<(u16, Value), String> {
    let v: Value = serde_json::from_slice(body).unwrap_or(json!({}));
    let op = v.get("operationName").and_then(Value::as_str).unwrap_or("");
    let vars = v.get("variables").cloned().unwrap_or(json!({}));
    let s = |k: &str| {
        vars.get(k)
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string()
    };
    match op {
        "IssueLinkedBranches" => {
            let (owner, name) = (s("owner"), s("name"));
            let number = vars.get("number").and_then(Value::as_u64).unwrap_or(0);
            if !st.issue_exists(&owner, &name, number) {
                return Ok(gql_error("Could not resolve to an issue"));
            }
            let nodes: Vec<Value> = st
                .db
                .linked_branches
                .iter()
                .filter(|b| b.owner == owner && b.repo == name && b.issue == number)
                .map(|b| {
                    json!({"ref": {"name": b.name, "repository": {
                        "nameWithOwner": b.branch_repo,
                        "url": format!("/{}", b.branch_repo),
                    }}})
                })
                .collect();
            Ok((
                200,
                json!({"data": {"repository": {"issue": {"linkedBranches": {"nodes": nodes}}}}}),
            ))
        }
        "IssueDevelopContext" => {
            let (owner, name) = (s("owner"), s("name"));
            let (bo, bn) = (s("branchOwner"), s("branchName"));
            let number = vars.get("number").and_then(Value::as_u64).unwrap_or(0);
            if !st.issue_exists(&owner, &name, number) {
                return Ok(gql_error("Could not resolve to an issue"));
            }
            let Some(dir) = forge_git::hosted_git_dir(&st.data_root(), &bo, &bn) else {
                return Ok(gql_error(
                    "Could not resolve to a repository with git storage",
                ));
            };
            let repo = Repository::open(&dir).map_err(|e| e.to_string())?;
            let default = match repo.head_target() {
                Ok(arks_refs::RefTarget::Symbolic(t)) => t,
                _ => "refs/heads/main".into(),
            };
            let default_oid = repo.refs().resolve(&default).ok();
            let base = s("baseRef");
            let base_oid = repo.refs().resolve(&base).ok();
            let target = |o: Option<Oid>| match o {
                Some(o) => json!({"target": {"oid": o.to_hex()}}),
                None => Value::Null,
            };
            let mut default_ref = target(default_oid);
            if let Some(m) = default_ref.as_object_mut() {
                m.insert(
                    "name".into(),
                    json!(default.trim_start_matches("refs/heads/")),
                );
            }
            Ok((
                200,
                json!({"data": {
                    "repository": {
                        "id": repo_node_id(&owner, &name),
                        "issue": {"id": format!("I_{owner}/{name}#{number}")},
                    },
                    "branchRepository": {
                        "id": repo_node_id(&bo, &bn),
                        "nameWithOwner": format!("{bo}/{bn}"),
                        "url": format!("/{bo}/{bn}"),
                        "defaultBranchRef": default_ref,
                        "baseRef": target(base_oid),
                    },
                }}),
            ))
        }
        "CreateLinkedBranch" => {
            let issue_id = s("issueId");
            let Some((owner, name, number)) = parse_issue_id(&issue_id) else {
                return Ok(gql_error("Could not resolve to a node with the global id"));
            };
            let issue = st
                .db
                .issues
                .iter()
                .find(|i| i.owner == owner && i.repo == name && i.number == number)
                .cloned();
            let Some(issue) = issue else {
                return Ok(gql_error("Could not resolve to an issue"));
            };
            let repo_id = s("repositoryId");
            let (bo, bn) = if repo_id.is_empty() {
                (owner.clone(), name.clone())
            } else {
                match repo_id.strip_prefix("R_").and_then(|r| r.split_once('/')) {
                    Some((o, n)) => (o.to_string(), n.to_string()),
                    None => return Ok(gql_error("Could not resolve to a repository")),
                }
            };
            let Ok(oid) = Oid::from_hex(&s("oid")) else {
                return Ok(gql_error("oid is not a valid commit id"));
            };
            let branch = match vars.get("name").and_then(Value::as_str) {
                Some(n) if !n.is_empty() => n.to_string(),
                _ => default_branch_name(number, &issue.title),
            };
            if let Err(e) = forge_git::create_branch(&st.data_root(), &bo, &bn, &branch, &oid) {
                return Ok(gql_error(&e));
            }
            let lb = st.add_linked_branch(&owner, &name, number, &format!("{bo}/{bn}"), &branch);
            st.save()?;
            Ok((
                200,
                json!({"data": {"createLinkedBranch": {"linkedBranch": {
                    "id": format!("LB_{}", lb.id),
                    "ref": {"name": lb.name},
                }}}}),
            ))
        }
        other => Ok((
            400,
            json!({"errors": [{"message": format!(
                "local forge GraphQL serves only IssueLinkedBranches, IssueDevelopContext, \
                 CreateLinkedBranch (got operationName `{other}`)"
            )}]}),
        )),
    }
}

fn parse_issue_id(id: &str) -> Option<(String, String, u64)> {
    let rest = id.strip_prefix("I_")?;
    let (slug, num) = rest.rsplit_once('#')?;
    let (o, n) = slug.split_once('/')?;
    Some((o.to_string(), n.to_string(), num.parse().ok()?))
}

/// Local-forge default for an omitted `name` ("issue number and title").
fn default_branch_name(number: u64, title: &str) -> String {
    let mut slug = String::new();
    for c in title.chars() {
        if c.is_ascii_alphanumeric() {
            slug.push(c.to_ascii_lowercase());
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }
    let slug = slug.trim_matches('-');
    if slug.is_empty() {
        number.to_string()
    } else {
        format!("{number}-{slug}")
    }
}

/// RFC 3339 UTC timestamp from Unix seconds (civil-from-days, proleptic Gregorian).
pub fn iso8601(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        rem / 3_600,
        (rem % 3_600) / 60,
        rem % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iso8601_known_points() {
        assert_eq!(iso8601(0), "1970-01-01T00:00:00Z");
        assert_eq!(iso8601(951_782_400), "2000-02-29T00:00:00Z");
        assert_eq!(iso8601(1_700_000_000), "2023-11-14T22:13:20Z");
    }

    #[test]
    fn public_key_shape() {
        assert!(looks_like_public_key(
            "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIExample0000000000 me@host"
        ));
        assert!(!looks_like_public_key("not a key"));
    }

    #[test]
    fn branch_name_default() {
        assert_eq!(default_branch_name(7, "Fix: the bug!"), "7-fix-the-bug");
        assert_eq!(default_branch_name(3, "!!"), "3");
        assert_eq!(
            parse_issue_id("I_local/x#12"),
            Some(("local".into(), "x".into(), 12))
        );
    }
}
