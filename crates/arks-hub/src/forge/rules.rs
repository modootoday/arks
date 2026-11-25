//! Rulesets: branch protection rules stored by the arks forge (`ruleset` / `rs`).
//!
//! A ruleset belongs to a repository or to an owner ("organization level", which
//! applies to that owner's repositories). REST shape:
//! `/repos/{o}/{r}/rulesets[/{id}]` (`?includes_parents=`), `/orgs/{org}/rulesets[/{id}]`
//! and `/repos/{o}/{r}/rules/branches/{branch}`. Ref conditions use
//! `refs/heads/<glob>`, `~DEFAULT_BRANCH` and `~ALL`; repository conditions use
//! name globs and `~ALL`.
//!
//! Enforcement: active rulesets gate `pr merge` on the forge (required approving
//! reviews, required status contexts, linear history) unless the merge asks for
//! the admin bypass; `evaluate` rulesets are reported but never block. Pushes to
//! the Git server of the same `arks serve` are checked by [`super::push_rules`].

use super::{
    Req, RouteResult, invalid, iso8601, next_id, no_content, not_found, segments, str_field,
};
use crate::store::{HubStore, now_secs};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ruleset {
    pub id: u64,
    /// Repository owner, or the organization for an owner-level ruleset.
    pub owner: String,
    /// `None` = owner (organization) level.
    #[serde(default)]
    pub repo: Option<String>,
    pub name: String,
    #[serde(default = "branch")]
    pub target: String,
    #[serde(default = "active")]
    pub enforcement: String,
    #[serde(default)]
    pub include: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
    #[serde(default)]
    pub repo_include: Vec<String>,
    #[serde(default)]
    pub repo_exclude: Vec<String>,
    #[serde(default)]
    pub rules: Vec<Value>,
    #[serde(default)]
    pub bypass_actors: Vec<Value>,
    #[serde(default)]
    pub created_at: u64,
    #[serde(default)]
    pub updated_at: u64,
}

fn branch() -> String {
    "branch".into()
}

fn active() -> String {
    "active".into()
}

/// Shell-style glob: `*` matches within a path segment, `**` across segments.
pub(crate) fn glob(pat: &str, s: &str) -> bool {
    fn go(p: &[u8], s: &[u8]) -> bool {
        match p.first() {
            None => s.is_empty(),
            Some(b'*') if p.get(1) == Some(&b'*') => (0..=s.len()).any(|i| go(&p[2..], &s[i..])),
            Some(b'*') => (0..=s.len())
                .take_while(|i| *i == 0 || s[i - 1] != b'/')
                .any(|i| go(&p[1..], &s[i..])),
            Some(b'?') => !s.is_empty() && s[0] != b'/' && go(&p[1..], &s[1..]),
            Some(c) => s.first() == Some(c) && go(&p[1..], &s[1..]),
        }
    }
    go(pat.as_bytes(), s.as_bytes())
}

/// Does a ref condition pattern match the full ref name `full`?
fn ref_matches(pat: &str, full: &str, default_branch: &str) -> bool {
    let short = full
        .strip_prefix("refs/heads/")
        .or_else(|| full.strip_prefix("refs/tags/"))
        .unwrap_or(full);
    match pat {
        "~ALL" => true,
        "~DEFAULT_BRANCH" => full.strip_prefix("refs/heads/") == Some(default_branch),
        p if p.starts_with("refs/") => glob(p, full),
        p => glob(p, short),
    }
}

fn repo_matches(rs: &Ruleset, name: &str) -> bool {
    let inc =
        rs.repo_include.is_empty() || rs.repo_include.iter().any(|p| p == "~ALL" || glob(p, name));
    inc && !rs.repo_exclude.iter().any(|p| glob(p, name))
}

/// Rulesets that apply to `branch` of `owner/repo` (repository first, then owner level).
pub(crate) fn applicable(st: &HubStore, owner: &str, repo: &str, branch: &str) -> Vec<Ruleset> {
    applicable_ref(st, owner, repo, &format!("refs/heads/{branch}"))
}

/// Rulesets (any enforcement but `disabled`) that apply to the full ref name
/// `full`: `refs/heads/*` takes `branch` rulesets, `refs/tags/*` takes `tag`
/// rulesets, other refs have none.
pub(crate) fn applicable_ref(st: &HubStore, owner: &str, repo: &str, full: &str) -> Vec<Ruleset> {
    let target = if full.starts_with("refs/heads/") {
        "branch"
    } else if full.starts_with("refs/tags/") {
        "tag"
    } else {
        return Vec::new();
    };
    let default = st
        .find_repo(owner, repo)
        .map(|r| r.default_branch.clone())
        .unwrap_or_else(|| "main".into());
    st.db
        .rulesets
        .iter()
        .filter(|rs| rs.owner == owner && rs.target == target && rs.enforcement != "disabled")
        .filter(|rs| match &rs.repo {
            Some(r) => r == repo,
            None => repo_matches(rs, repo),
        })
        .filter(|rs| {
            let inc = rs.include.iter().any(|p| ref_matches(p, full, &default));
            inc && !rs.exclude.iter().any(|p| ref_matches(p, full, &default))
        })
        .cloned()
        .collect()
}

/// Why a merge into `base` is blocked by active rulesets (empty = allowed).
pub(crate) fn merge_blockers(
    st: &HubStore,
    owner: &str,
    repo: &str,
    base: &str,
    approvals: usize,
    contexts_ok: &dyn Fn(&str) -> bool,
    method: &str,
) -> Vec<String> {
    let mut why = Vec::new();
    for rs in applicable(st, owner, repo, base) {
        if rs.enforcement != "active" {
            continue;
        }
        for rule in &rs.rules {
            let params = rule.get("parameters").cloned().unwrap_or(json!({}));
            match rule.get("type").and_then(Value::as_str).unwrap_or("") {
                "pull_request" => {
                    let need = params
                        .get("required_approving_review_count")
                        .and_then(Value::as_u64)
                        .unwrap_or(0) as usize;
                    if approvals < need {
                        why.push(format!(
                            "ruleset {:?}: {need} approving review(s) required, {approvals} given",
                            rs.name
                        ));
                    }
                }
                "required_status_checks" => {
                    for c in params
                        .get("required_status_checks")
                        .and_then(Value::as_array)
                        .into_iter()
                        .flatten()
                    {
                        let ctx = c.get("context").and_then(Value::as_str).unwrap_or("");
                        if !contexts_ok(ctx) {
                            why.push(format!(
                                "ruleset {:?}: required status check {ctx:?} has not passed",
                                rs.name
                            ));
                        }
                    }
                }
                "required_linear_history" if method == "merge" => why.push(format!(
                    "ruleset {:?}: linear history required (use --squash or --rebase)",
                    rs.name
                )),
                _ => {}
            }
        }
    }
    why
}

fn list_of(v: &Value, ptr: &str) -> Vec<String> {
    v.pointer(ptr)
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn apply(rs: &mut Ruleset, v: &Value) -> Result<(), String> {
    if let Some(n) = str_field(v, "name") {
        rs.name = n.into();
    }
    if let Some(t) = str_field(v, "target") {
        if !matches!(t, "branch" | "tag" | "push") {
            return Err(format!("invalid target {t:?}"));
        }
        rs.target = t.into();
    }
    if let Some(e) = str_field(v, "enforcement") {
        if !matches!(e, "active" | "evaluate" | "disabled") {
            return Err(format!("invalid enforcement {e:?}"));
        }
        rs.enforcement = e.into();
    }
    if v.pointer("/conditions/ref_name").is_some() {
        rs.include = list_of(v, "/conditions/ref_name/include");
        rs.exclude = list_of(v, "/conditions/ref_name/exclude");
    }
    if v.pointer("/conditions/repository_name").is_some() {
        rs.repo_include = list_of(v, "/conditions/repository_name/include");
        rs.repo_exclude = list_of(v, "/conditions/repository_name/exclude");
    }
    if let Some(r) = v.get("rules").and_then(Value::as_array) {
        for rule in r {
            if rule.get("type").and_then(Value::as_str).is_none() {
                return Err("every rule needs a type".into());
            }
        }
        rs.rules = r.clone();
    }
    if let Some(b) = v.get("bypass_actors").and_then(Value::as_array) {
        rs.bypass_actors = b.clone();
    }
    rs.updated_at = now_secs();
    Ok(())
}

pub(crate) fn ruleset_json(st: &HubStore, rs: &Ruleset, full: bool) -> Value {
    let (stype, source) = match &rs.repo {
        Some(r) => ("Repository", format!("{}/{r}", rs.owner)),
        None => ("Organization", rs.owner.clone()),
    };
    let mut v = json!({
        "id": rs.id,
        "name": rs.name,
        "target": rs.target,
        "source_type": stype,
        "source": source,
        "enforcement": rs.enforcement,
        "node_id": format!("RRS_{}", rs.id),
        "created_at": iso8601(rs.created_at),
        "updated_at": iso8601(rs.updated_at),
        "_links": {"html": {"href": match &rs.repo {
            Some(r) => format!("{}/{}/{r}/rules/{}", st.public_base, rs.owner, rs.id),
            None => format!("{}/organizations/{}/settings/rules/{}", st.public_base, rs.owner, rs.id),
        }}},
    });
    if full {
        let mut cond = json!({"ref_name": {"include": rs.include, "exclude": rs.exclude}});
        if rs.repo.is_none() {
            cond["repository_name"] =
                json!({"include": rs.repo_include, "exclude": rs.repo_exclude});
        }
        v["conditions"] = cond;
        v["rules"] = json!(rs.rules);
        v["bypass_actors"] = json!(rs.bypass_actors);
        v["current_user_can_bypass"] = json!("never");
    }
    v
}

fn crud(
    st: &mut HubStore,
    req: &Req<'_>,
    owner: &str,
    repo: Option<&str>,
    rest: &[&str],
) -> RouteResult {
    let in_scope = |rs: &Ruleset| rs.owner == owner && rs.repo.as_deref() == repo;
    match (req.method, rest) {
        ("GET", []) => {
            let parents = req.query_param("includes_parents").as_deref() != Some("false");
            let items: Vec<Value> = st
                .db
                .rulesets
                .iter()
                .filter(|rs| {
                    in_scope(rs)
                        || (parents
                            && repo.is_some()
                            && rs.owner == owner
                            && rs.repo.is_none()
                            && repo_matches(rs, repo.unwrap_or("")))
                })
                .map(|rs| ruleset_json(st, rs, false))
                .collect();
            Ok(Some((200, Value::Array(items))))
        }
        ("POST", []) => {
            let v = req.json();
            if str_field(&v, "name").is_none() {
                return Ok(Some(invalid("name is required")));
            }
            let now = now_secs();
            let mut rs = Ruleset {
                id: next_id(&mut st.db.next_ruleset),
                owner: owner.into(),
                repo: repo.map(str::to_string),
                name: String::new(),
                target: branch(),
                enforcement: active(),
                include: Vec::new(),
                exclude: Vec::new(),
                repo_include: Vec::new(),
                repo_exclude: Vec::new(),
                rules: Vec::new(),
                bypass_actors: Vec::new(),
                created_at: now,
                updated_at: now,
            };
            if let Err(e) = apply(&mut rs, &v) {
                return Ok(Some(invalid(e)));
            }
            st.db.rulesets.push(rs.clone());
            st.save()?;
            Ok(Some((201, ruleset_json(st, &rs, true))))
        }
        (m, [id]) => {
            let Ok(id) = id.parse::<u64>() else {
                return Ok(Some(not_found()));
            };
            // A repository view may read an applicable owner-level ruleset.
            let Some(i) = st.db.rulesets.iter().position(|rs| {
                rs.id == id
                    && (in_scope(rs)
                        || (m == "GET" && repo.is_some() && rs.owner == owner && rs.repo.is_none()))
            }) else {
                return Ok(Some(not_found()));
            };
            match m {
                "GET" => Ok(Some((
                    200,
                    ruleset_json(st, &st.db.rulesets[i].clone(), true),
                ))),
                "PUT" | "PATCH" => {
                    let mut rs = st.db.rulesets[i].clone();
                    if let Err(e) = apply(&mut rs, &req.json()) {
                        return Ok(Some(invalid(e)));
                    }
                    st.db.rulesets[i] = rs.clone();
                    st.save()?;
                    Ok(Some((200, ruleset_json(st, &rs, true))))
                }
                "DELETE" => {
                    st.db.rulesets.remove(i);
                    st.save()?;
                    Ok(Some(no_content()))
                }
                _ => Ok(None),
            }
        }
        _ => Ok(None),
    }
}

pub(crate) fn route(st: &mut HubStore, req: &Req<'_>) -> RouteResult {
    if let Some(seg) = segments(req.path, "/orgs/") {
        if seg.len() >= 2 && seg[1] == "rulesets" {
            let owner = seg[0].to_string();
            return crud(st, req, &owner, None, &seg[2..]);
        }
        return Ok(None);
    }
    let Some(seg) = segments(req.path, "/repos/") else {
        return Ok(None);
    };
    if seg.len() < 3 {
        return Ok(None);
    }
    let (owner, repo) = (seg[0].to_string(), seg[1].to_string());
    match seg[2] {
        "rulesets" => {
            if st.find_repo(&owner, &repo).is_none() {
                return Ok(Some(not_found()));
            }
            crud(st, req, &owner, Some(&repo), &seg[3..])
        }
        "rules" if req.method == "GET" && seg.get(3) == Some(&"branches") && seg.len() > 4 => {
            let b = seg[4..].join("/");
            let mut out = Vec::new();
            for rs in applicable(st, &owner, &repo, &b) {
                let (stype, source) = match &rs.repo {
                    Some(r) => ("Repository", format!("{owner}/{r}")),
                    None => ("Organization", owner.clone()),
                };
                for rule in &rs.rules {
                    let mut r = rule.clone();
                    r["ruleset_source_type"] = json!(stype);
                    r["ruleset_source"] = json!(source);
                    r["ruleset_id"] = json!(rs.id);
                    out.push(r);
                }
            }
            Ok(Some((200, Value::Array(out))))
        }
        _ => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::glob;

    #[test]
    fn globs() {
        assert!(glob("refs/heads/release/*", "refs/heads/release/1.0"));
        assert!(!glob("refs/heads/release/*", "refs/heads/release/1/x"));
        assert!(glob("refs/heads/**", "refs/heads/a/b"));
        assert!(glob("ma?n", "main"));
    }
}
