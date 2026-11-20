//! Repository discussions with categories and threaded-flat comments
//! (`/repos/{owner}/{repo}/discussions[/{number}[/comments]]`).

use super::{
    Req, RouteResult, invalid, iso8601, method_not_allowed, next_id, not_found, repo_exists,
    segments, str_field,
};
use crate::store::{HubStore, now_secs};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

fn default_open() -> String {
    "open".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Discussion {
    pub number: u64,
    pub owner: String,
    pub repo: String,
    pub title: String,
    pub body: String,
    pub category: String,
    #[serde(default = "default_open")]
    pub state: String,
    #[serde(default)]
    pub user: String,
    #[serde(default)]
    pub created_at: u64,
    #[serde(default)]
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscussionComment {
    pub id: u64,
    pub owner: String,
    pub repo: String,
    pub number: u64,
    pub body: String,
    pub user: String,
    pub created_at: u64,
}

fn comment_json(c: &DiscussionComment) -> Value {
    json!({
        "id": c.id,
        "body": c.body,
        "user": {"login": c.user},
        "created_at": iso8601(c.created_at),
    })
}

fn discussion_json(st: &HubStore, d: &Discussion) -> Value {
    let comments = st
        .db
        .discussion_comments
        .iter()
        .filter(|c| c.owner == d.owner && c.repo == d.repo && c.number == d.number)
        .count();
    let user = if d.user.is_empty() {
        &st.db.user_login
    } else {
        &d.user
    };
    json!({
        "number": d.number,
        "title": d.title,
        "body": d.body,
        "category": {"name": d.category},
        "state": d.state,
        "user": {"login": user},
        "comments": comments,
        "created_at": iso8601(d.created_at),
        "updated_at": iso8601(d.updated_at),
    })
}

/// Start a discussion announcing a published release (`release create --discussion-category`).
pub(crate) fn create_for_release(
    st: &mut HubStore,
    owner: &str,
    repo: &str,
    title: &str,
    body: &str,
    category: &str,
) {
    let now = now_secs();
    let d = Discussion {
        number: next_id(&mut st.db.next_discussion),
        owner: owner.into(),
        repo: repo.into(),
        title: title.into(),
        body: body.into(),
        category: category.into(),
        state: default_open(),
        user: st.actor.clone(),
        created_at: now,
        updated_at: now,
    };
    st.db.discussions.push(d);
}

pub(crate) fn route(st: &mut HubStore, req: &Req<'_>) -> RouteResult {
    let Some(seg) = segments(req.path, "/repos/") else {
        return Ok(None);
    };
    if seg.len() < 3 || seg[2] != "discussions" {
        return Ok(None);
    }
    let (owner, repo) = (seg[0].to_string(), seg[1].to_string());
    if !repo_exists(st, &owner, &repo) {
        return Ok(Some(not_found()));
    }
    let number = seg.get(3).and_then(|s| s.parse::<u64>().ok());
    if seg.len() >= 4 && number.is_none() {
        return Ok(Some(not_found()));
    }
    let find = |st: &HubStore, n: u64| {
        st.db
            .discussions
            .iter()
            .position(|d| d.owner == owner && d.repo == repo && d.number == n)
    };
    match (req.method, &seg[3..]) {
        ("GET", []) => {
            let state = req.query_param("state").unwrap_or_else(|| "open".into());
            let category = req.query_param("category");
            let mut arr = Vec::new();
            for d in st.db.discussions.iter().rev() {
                if d.owner != owner || d.repo != repo {
                    continue;
                }
                if state != "all" && d.state != state {
                    continue;
                }
                if let Some(c) = &category {
                    if !d.category.eq_ignore_ascii_case(c) {
                        continue;
                    }
                }
                arr.push(discussion_json(st, d));
            }
            Ok(Some((200, Value::Array(arr))))
        }
        ("POST", []) => {
            let v = req.json();
            let Some(title) = str_field(&v, "title") else {
                return Ok(Some(invalid("title can't be blank")));
            };
            let category = v
                .get("category")
                .and_then(|c| c.as_str().or_else(|| c.get("name").and_then(Value::as_str)))
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .unwrap_or("General");
            let now = now_secs();
            let d = Discussion {
                number: next_id(&mut st.db.next_discussion),
                owner: owner.clone(),
                repo: repo.clone(),
                title: title.to_string(),
                body: v.get("body").and_then(Value::as_str).unwrap_or("").into(),
                category: category.to_string(),
                state: default_open(),
                user: st.db.user_login.clone(),
                created_at: now,
                updated_at: now,
            };
            st.db.discussions.push(d.clone());
            st.save()?;
            Ok(Some((201, discussion_json(st, &d))))
        }
        ("GET", [_]) => {
            let Some(i) = find(st, number.unwrap_or(0)) else {
                return Ok(Some(not_found()));
            };
            Ok(Some((200, discussion_json(st, &st.db.discussions[i]))))
        }
        ("PATCH", [_]) => {
            let v = req.json();
            let Some(i) = find(st, number.unwrap_or(0)) else {
                return Ok(Some(not_found()));
            };
            if let Some(s) = v.get("state").and_then(Value::as_str) {
                if s != "open" && s != "closed" {
                    return Ok(Some(invalid("state must be open or closed")));
                }
            }
            if v.get("title").is_some() && str_field(&v, "title").is_none() {
                return Ok(Some(invalid("title can't be blank")));
            }
            let d = &mut st.db.discussions[i];
            if let Some(t) = str_field(&v, "title") {
                d.title = t.to_string();
            }
            if let Some(b) = v.get("body").and_then(Value::as_str) {
                d.body = b.to_string();
            }
            if let Some(c) = str_field(&v, "category") {
                d.category = c.to_string();
            }
            if let Some(s) = v.get("state").and_then(Value::as_str) {
                d.state = s.to_string();
            }
            d.updated_at = now_secs();
            st.save()?;
            Ok(Some((200, discussion_json(st, &st.db.discussions[i]))))
        }
        ("GET", [_, "comments"]) => {
            let n = number.unwrap_or(0);
            if find(st, n).is_none() {
                return Ok(Some(not_found()));
            }
            let arr = st
                .db
                .discussion_comments
                .iter()
                .filter(|c| c.owner == owner && c.repo == repo && c.number == n)
                .map(comment_json)
                .collect();
            Ok(Some((200, Value::Array(arr))))
        }
        ("POST", [_, "comments"]) => {
            let n = number.unwrap_or(0);
            if find(st, n).is_none() {
                return Ok(Some(not_found()));
            }
            let v = req.json();
            let Some(body) = str_field(&v, "body") else {
                return Ok(Some(invalid("body can't be blank")));
            };
            let c = DiscussionComment {
                id: next_id(&mut st.db.next_discussion_comment),
                owner: owner.clone(),
                repo: repo.clone(),
                number: n,
                body: body.to_string(),
                user: st.db.user_login.clone(),
                created_at: now_secs(),
            };
            st.db.discussion_comments.push(c.clone());
            st.save()?;
            Ok(Some((201, comment_json(&c))))
        }
        _ => Ok(Some(method_not_allowed())),
    }
}
