//! Repository autolinks: a key prefix plus a URL template with `<num>`
//! (`/repos/{owner}/{repo}/autolinks[/{id}]`).

use super::{
    Req, RouteResult, invalid, method_not_allowed, next_id, no_content, not_found, repo_exists,
    segments,
};
use crate::store::HubStore;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Autolink {
    pub id: u64,
    pub owner: String,
    pub repo: String,
    pub key_prefix: String,
    pub url_template: String,
    pub is_alphanumeric: bool,
}

fn autolink_json(a: &Autolink) -> Value {
    json!({
        "id": a.id,
        "key_prefix": a.key_prefix,
        "url_template": a.url_template,
        "is_alphanumeric": a.is_alphanumeric,
    })
}

/// Validate prefix + template; returns a human message on refusal.
pub(crate) fn validate(key_prefix: &str, url_template: &str) -> Result<(), String> {
    if key_prefix.is_empty() {
        return Err("key_prefix can't be blank".into());
    }
    if key_prefix.chars().any(char::is_whitespace) {
        return Err("key_prefix must not contain whitespace".into());
    }
    if !(url_template.starts_with("https://") || url_template.starts_with("http://")) {
        return Err("url_template must be an http(s) URL".into());
    }
    if !url_template.contains("<num>") {
        return Err("url_template must contain <num>".into());
    }
    Ok(())
}

pub(crate) fn route(st: &mut HubStore, req: &Req<'_>) -> RouteResult {
    let Some(seg) = segments(req.path, "/repos/") else {
        return Ok(None);
    };
    if seg.len() < 3 || seg[2] != "autolinks" {
        return Ok(None);
    }
    let (owner, repo) = (seg[0], seg[1]);
    if !repo_exists(st, owner, repo) {
        return Ok(Some(not_found()));
    }
    let id = seg.get(3).and_then(|s| s.parse::<u64>().ok());
    let in_repo = |a: &Autolink| a.owner == owner && a.repo == repo;
    match (req.method, seg.len()) {
        ("GET", 3) => {
            let arr = st
                .db
                .autolinks
                .iter()
                .filter(|a| in_repo(a))
                .map(autolink_json)
                .collect();
            Ok(Some((200, Value::Array(arr))))
        }
        ("POST", 3) => {
            let v = req.json();
            let prefix = v.get("key_prefix").and_then(Value::as_str).unwrap_or("");
            let template = v.get("url_template").and_then(Value::as_str).unwrap_or("");
            if let Err(e) = validate(prefix, template) {
                return Ok(Some(invalid(e)));
            }
            if st
                .db
                .autolinks
                .iter()
                .any(|a| in_repo(a) && a.key_prefix.eq_ignore_ascii_case(prefix))
            {
                return Ok(Some(invalid("key_prefix already exists")));
            }
            let a = Autolink {
                id: next_id(&mut st.db.next_autolink),
                owner: owner.into(),
                repo: repo.into(),
                key_prefix: prefix.into(),
                url_template: template.into(),
                is_alphanumeric: v
                    .get("is_alphanumeric")
                    .and_then(Value::as_bool)
                    .unwrap_or(true),
            };
            st.db.autolinks.push(a.clone());
            st.save()?;
            Ok(Some((201, autolink_json(&a))))
        }
        ("GET", 4) => Ok(Some(
            st.db
                .autolinks
                .iter()
                .find(|a| in_repo(a) && Some(a.id) == id)
                .map(|a| (200, autolink_json(a)))
                .unwrap_or_else(not_found),
        )),
        ("DELETE", 4) => {
            let before = st.db.autolinks.len();
            st.db
                .autolinks
                .retain(|a| !(a.owner == owner && a.repo == repo && Some(a.id) == id));
            if st.db.autolinks.len() == before {
                return Ok(Some(not_found()));
            }
            st.save()?;
            Ok(Some(no_content()))
        }
        _ => Ok(Some(method_not_allowed())),
    }
}

#[cfg(test)]
mod tests {
    use super::validate;

    #[test]
    fn validate_prefix_and_template() {
        assert!(validate("JIRA-", "https://jira.example/browse/JIRA-<num>").is_ok());
        assert!(validate("", "https://x/<num>").is_err());
        assert!(validate("A B", "https://x/<num>").is_err());
        assert!(validate("T-", "ftp://x/<num>").is_err());
        assert!(validate("T-", "https://x/").is_err());
    }
}
