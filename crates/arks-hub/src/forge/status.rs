//! Commit statuses: the arks-owned check primitive behind `pr checks`
//! ([[adr:root/20261119230000]] decision 6 — not GitHub Apps check runs).
//!
//! `POST /repos/{o}/{r}/statuses/{sha}` records `{state, context, description,
//! target_url}`; `GET /repos/{o}/{r}/commits/{ref}/statuses` lists them newest
//! first and `GET …/commits/{ref}/status` folds them to the latest per context.
//! A branch or tag name resolves through the repository's Git content when there
//! is some; otherwise the literal ref string is the key.

use super::git::GitRepo;
use super::{Req, RouteResult, invalid, iso8601, next_id, not_found, segments, str_field};
use crate::store::{HubStore, now_secs};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitStatus {
    pub id: u64,
    pub owner: String,
    pub repo: String,
    pub sha: String,
    pub state: String,
    pub context: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub target_url: String,
    #[serde(default)]
    pub creator: String,
    pub created_at: u64,
}

pub(crate) const STATES: &[&str] = &["error", "failure", "pending", "success"];

/// Resolve `rf` to the status key (commit id when Git content resolves it).
pub(crate) fn key(st: &HubStore, owner: &str, repo: &str, rf: &str) -> String {
    st.git_dir(owner, repo)
        .and_then(|d| GitRepo::open(&d).ok())
        .and_then(|g| g.resolve(rf))
        .map(|o| o.to_hex())
        .unwrap_or_else(|| rf.to_string())
}

/// Latest status per context for `sha`, newest first.
pub(crate) fn latest(st: &HubStore, owner: &str, repo: &str, sha: &str) -> Vec<CommitStatus> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for s in st.db.statuses.iter().rev() {
        if s.owner == owner && s.repo == repo && s.sha == sha && seen.insert(s.context.clone()) {
            out.push(s.clone());
        }
    }
    out
}

pub(crate) fn combined_state(list: &[CommitStatus]) -> &'static str {
    if list
        .iter()
        .any(|s| s.state == "error" || s.state == "failure")
    {
        "failure"
    } else if list.is_empty() || list.iter().any(|s| s.state == "pending") {
        "pending"
    } else {
        "success"
    }
}

fn status_json(s: &CommitStatus, first_at: u64) -> Value {
    json!({
        "id": s.id,
        "state": s.state,
        "context": s.context,
        "description": s.description,
        "target_url": s.target_url,
        "creator": {"login": s.creator},
        "created_at": iso8601(first_at),
        "updated_at": iso8601(s.created_at),
        "sha": s.sha,
    })
}

/// When the context first appeared for this sha (gh shows elapsed = updated − created).
fn first_seen(st: &HubStore, s: &CommitStatus) -> u64 {
    st.db
        .statuses
        .iter()
        .filter(|x| {
            x.owner == s.owner && x.repo == s.repo && x.sha == s.sha && x.context == s.context
        })
        .map(|x| x.created_at)
        .min()
        .unwrap_or(s.created_at)
}

pub(crate) fn route(st: &mut HubStore, req: &Req<'_>) -> RouteResult {
    let Some(seg) = segments(req.path, "/repos/") else {
        return Ok(None);
    };
    let (owner, repo) = match seg.as_slice() {
        [o, r, ..] => (o.to_string(), r.to_string()),
        _ => return Ok(None),
    };
    let rest = &seg[2..];
    match (req.method, rest) {
        ("POST", ["statuses", rf @ ..]) if !rf.is_empty() => {
            if st.find_repo(&owner, &repo).is_none() {
                return Ok(Some(not_found()));
            }
            let v = req.json();
            let Some(state) = str_field(&v, "state").filter(|s| STATES.contains(s)) else {
                return Ok(Some(invalid(
                    "state must be one of error, failure, pending, success",
                )));
            };
            let sha = key(st, &owner, &repo, &rf.join("/"));
            let s = CommitStatus {
                id: next_id(&mut st.db.next_status),
                owner,
                repo,
                sha,
                state: state.into(),
                context: str_field(&v, "context").unwrap_or("default").into(),
                description: str_field(&v, "description").unwrap_or("").into(),
                target_url: str_field(&v, "target_url").unwrap_or("").into(),
                creator: st.actor.clone(),
                created_at: now_secs(),
            };
            st.db.statuses.push(s.clone());
            st.save()?;
            let first = first_seen(st, &s);
            Ok(Some((201, status_json(&s, first))))
        }
        ("GET", ["commits", rf @ .., last]) if *last == "statuses" || *last == "status" => {
            let sha = key(st, &owner, &repo, &rf.join("/"));
            if *last == "statuses" {
                let items: Vec<Value> = st
                    .db
                    .statuses
                    .iter()
                    .rev()
                    .filter(|s| s.owner == owner && s.repo == repo && s.sha == sha)
                    .map(|s| status_json(s, first_seen(st, s)))
                    .collect();
                return Ok(Some((200, Value::Array(items))));
            }
            let list = latest(st, &owner, &repo, &sha);
            let items: Vec<Value> = list
                .iter()
                .map(|s| status_json(s, first_seen(st, s)))
                .collect();
            Ok(Some((
                200,
                json!({
                    "state": combined_state(&list),
                    "sha": sha,
                    "total_count": items.len(),
                    "statuses": items,
                }),
            )))
        }
        _ => Ok(None),
    }
}
