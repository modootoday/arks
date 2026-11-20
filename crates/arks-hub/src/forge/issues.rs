//! Issues, conversation comments, repository labels and milestones, linked
//! branches (`issue develop`) and issue transfer (arks-owned; GitHub-shaped REST).
//!
//! arks numbers issues and pull requests separately; a pull request's
//! conversation lives under `/repos/{o}/{r}/pulls/{n}/comments`, an issue's under
//! `/repos/{o}/{r}/issues/{n}/comments`, and both share
//! `/repos/{o}/{r}/issues/comments/{id}` for edit and delete.

use super::git::GitRepo;
use super::{
    Req, Resp, RouteResult, invalid, iso8601, next_id, no_content, not_found, segments, str_field,
};
use crate::store::{Comment, HubStore, Issue, Label, LinkedBranch, Milestone, now_secs};
use serde_json::{Value, json};

pub(crate) fn label_json(l: &Label) -> Value {
    json!({
        "id": l.id,
        "node_id": format!("LA_{}", l.id),
        "name": l.name,
        "color": l.color,
        "description": l.description,
        "default": false,
        "created_at": iso8601(l.created_at),
        "updated_at": iso8601(l.updated_at.max(l.created_at)),
    })
}

pub(crate) fn labels_json(st: &HubStore, owner: &str, repo: &str, names: &[String]) -> Value {
    Value::Array(
        names
            .iter()
            .map(|n| {
                st.db
                    .labels
                    .iter()
                    .find(|l| l.owner == owner && l.repo == repo && l.name == *n)
                    .map(label_json)
                    .unwrap_or_else(|| json!({"name": n, "color": "ededed", "description": ""}))
            })
            .collect(),
    )
}

pub(crate) fn milestone_json(st: &HubStore, owner: &str, repo: &str, n: Option<u64>) -> Value {
    n.and_then(|n| {
        st.db
            .milestones
            .iter()
            .find(|m| m.owner == owner && m.repo == repo && m.number == n)
    })
    .map(|m| json!({"number": m.number, "title": m.title, "description": m.description, "state": m.state}))
    .unwrap_or(Value::Null)
}

/// Create any missing labels (default colour), keeping order and dropping repeats.
pub(crate) fn ensure_labels(
    st: &mut HubStore,
    owner: &str,
    repo: &str,
    names: &[String],
) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for n in names {
        if out.contains(n) {
            continue;
        }
        if !st
            .db
            .labels
            .iter()
            .any(|l| l.owner == owner && l.repo == repo && l.name == *n)
        {
            let now = now_secs();
            let id = next_id(&mut st.db.next_label);
            st.db.labels.push(Label {
                id,
                owner: owner.into(),
                repo: repo.into(),
                name: n.clone(),
                color: "ededed".into(),
                description: String::new(),
                created_at: now,
                updated_at: now,
            });
        }
        out.push(n.clone());
    }
    out
}

pub(crate) fn strings(v: &Value, key: &str) -> Option<Vec<String>> {
    v.get(key).and_then(Value::as_array).map(|a| {
        a.iter()
            .filter_map(|x| x.as_str().or_else(|| x.get("name").and_then(Value::as_str)))
            .map(str::to_string)
            .collect()
    })
}

/// Milestone from `number` or title; `Some(None)` clears it.
pub(crate) fn milestone_arg(
    st: &HubStore,
    owner: &str,
    repo: &str,
    v: &Value,
) -> Result<Option<Option<u64>>, String> {
    match v.get("milestone") {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::Number(n)) => Ok(Some(n.as_u64())),
        Some(Value::String(t)) => st
            .db
            .milestones
            .iter()
            .find(|m| {
                m.owner == owner && m.repo == repo && (m.title == *t || m.number.to_string() == *t)
            })
            .map(|m| Some(Some(m.number)))
            .ok_or_else(|| format!("milestone {t:?} not found")),
        Some(_) => Err("invalid milestone".into()),
    }
}

pub(crate) fn comment_count(st: &HubStore, owner: &str, repo: &str, n: u64, kind: &str) -> usize {
    st.db
        .comments
        .iter()
        .filter(|c| c.owner == owner && c.repo == repo && c.number == n && c.kind == kind)
        .count()
}

pub(crate) fn issue_json(st: &HubStore, i: &Issue) -> Value {
    let full = format!("{}/{}", i.owner, i.repo);
    json!({
        "id": i.number,
        "node_id": format!("I_{}_{}", i.repo, i.number),
        "number": i.number,
        "title": i.title,
        "body": i.body,
        "state": i.state,
        "state_reason": i.state_reason,
        "locked": i.locked,
        "active_lock_reason": i.lock_reason,
        "pinned": i.pinned,
        "user": {"login": i.user},
        "labels": labels_json(st, &i.owner, &i.repo, &i.labels),
        "assignees": i.assignees.iter().map(|a| json!({"login": a})).collect::<Vec<_>>(),
        "milestone": milestone_json(st, &i.owner, &i.repo, i.milestone),
        "comments": comment_count(st, &i.owner, &i.repo, i.number, "issue"),
        "projects": i.projects,
        "linked_branches": i.linked_branches,
        "created_at": iso8601(i.created_at),
        "updated_at": iso8601(i.updated_at.max(i.created_at)),
        "closed_at": i.closed_at.map(iso8601),
        "html_url": format!("{}/{full}/issues/{}", st.public_base, i.number),
        "url": format!("{}/repos/{full}/issues/{}", st.public_base, i.number),
        "repository": {"full_name": full, "name": i.repo, "owner": {"login": i.owner}},
        "repository_url": format!("{}/repos/{full}", st.public_base),
        "owner": {"login": i.owner},
    })
}

pub(crate) fn comment_json(st: &HubStore, c: &Comment) -> Value {
    let seg = if c.kind == "pull" { "pull" } else { "issues" };
    json!({
        "id": c.id,
        "node_id": format!("IC_{}", c.id),
        "body": c.body,
        "user": {"login": c.user},
        "author_association": "OWNER",
        "created_at": iso8601(c.created_at),
        "updated_at": iso8601(c.updated_at.max(c.created_at)),
        "html_url": format!("{}/{}/{}/{seg}/{}#issuecomment-{}", st.public_base, c.owner, c.repo, c.number, c.id),
        "issue_url": format!("{}/repos/{}/{}/issues/{}", st.public_base, c.owner, c.repo, c.number),
    })
}

/// `/…/{n}/comments` shared by issues (`kind = issue`) and pull requests (`pull`).
pub(crate) fn comments_route(
    st: &mut HubStore,
    req: &Req<'_>,
    owner: &str,
    repo: &str,
    n: u64,
    kind: &str,
) -> Result<Resp, String> {
    match req.method {
        "GET" => {
            let items: Vec<Value> = st
                .db
                .comments
                .iter()
                .filter(|c| c.owner == owner && c.repo == repo && c.number == n && c.kind == kind)
                .map(|c| comment_json(st, c))
                .collect();
            Ok((200, Value::Array(items)))
        }
        "POST" => {
            let v = req.json();
            let Some(body) = str_field(&v, "body") else {
                return Ok(invalid("body is required"));
            };
            let now = now_secs();
            let c = Comment {
                id: next_id(&mut st.db.next_comment),
                owner: owner.into(),
                repo: repo.into(),
                number: n,
                body: body.into(),
                user: st.actor.clone(),
                kind: kind.into(),
                created_at: now,
                updated_at: now,
            };
            st.db.comments.push(c.clone());
            st.save()?;
            Ok((201, comment_json(st, &c)))
        }
        _ => Ok((405, json!({"message": "Method Not Allowed"}))),
    }
}

fn find(st: &HubStore, owner: &str, repo: &str, n: u64) -> Option<usize> {
    st.db
        .issues
        .iter()
        .position(|i| i.owner == owner && i.repo == repo && i.number == n)
}

fn patch(st: &mut HubStore, idx: usize, v: &Value) -> Result<Option<String>, String> {
    let (owner, repo) = (
        st.db.issues[idx].owner.clone(),
        st.db.issues[idx].repo.clone(),
    );
    let ms = match milestone_arg(st, &owner, &repo, v) {
        Ok(m) => m,
        Err(e) => return Ok(Some(e)),
    };
    let labels = strings(v, "labels").map(|l| ensure_labels(st, &owner, &repo, &l));
    let i = &mut st.db.issues[idx];
    if let Some(t) = v.get("title").and_then(Value::as_str) {
        i.title = t.into();
    }
    if let Some(b) = v.get("body").and_then(Value::as_str) {
        i.body = b.into();
    }
    if let Some(s) = v.get("state").and_then(Value::as_str) {
        if !matches!(s, "open" | "closed") {
            return Ok(Some(format!("invalid state {s:?}")));
        }
        if s != i.state {
            i.closed_at = (s == "closed").then(now_secs);
        }
        i.state = s.into();
        i.state_reason = match (s, v.get("state_reason").and_then(Value::as_str)) {
            ("closed", Some(r)) => Some(r.into()),
            ("closed", None) => Some("completed".into()),
            (_, _) => Some("reopened".into()),
        };
    }
    if let Some(l) = labels {
        i.labels = l;
    }
    if let Some(a) = strings(v, "assignees") {
        i.assignees = a;
    }
    if let Some(m) = ms {
        i.milestone = m;
    }
    if let Some(l) = v.get("locked").and_then(Value::as_bool) {
        i.locked = l;
    }
    if let Some(p) = v.get("pinned").and_then(Value::as_bool) {
        i.pinned = p;
    }
    if let Some(p) = strings(v, "projects") {
        i.projects = p;
    }
    i.updated_at = now_secs();
    Ok(None)
}

fn transfer(st: &mut HubStore, idx: usize, to: &str) -> Result<Resp, String> {
    let Some((to_o, to_r)) = to.split_once('/') else {
        return Ok(invalid("destination must be owner/name"));
    };
    if st.find_repo(to_o, to_r).is_none() {
        return Ok(not_found());
    }
    let mut i = st.db.issues.remove(idx);
    let (old_o, old_r, old_n) = (i.owner.clone(), i.repo.clone(), i.number);
    let n = st
        .db
        .issues
        .iter()
        .filter(|x| x.owner == to_o && x.repo == to_r)
        .map(|x| x.number)
        .max()
        .unwrap_or(0)
        .max(st.db.next_issue.saturating_sub(1))
        + 1;
    st.db.next_issue = n + 1;
    i.owner = to_o.into();
    i.repo = to_r.into();
    i.number = n;
    i.milestone = None;
    i.labels = ensure_labels(st, to_o, to_r, &i.labels);
    for c in
        st.db.comments.iter_mut().filter(|c| {
            c.owner == old_o && c.repo == old_r && c.number == old_n && c.kind == "issue"
        })
    {
        c.owner = to_o.into();
        c.repo = to_r.into();
        c.number = n;
    }
    st.db.issues.push(i.clone());
    st.save()?;
    Ok((200, issue_json(st, &i)))
}

fn develop(st: &mut HubStore, idx: usize, v: &Value) -> Result<Resp, String> {
    let i = st.db.issues[idx].clone();
    let target = str_field(v, "repo")
        .map(str::to_string)
        .unwrap_or(format!("{}/{}", i.owner, i.repo));
    let Some((to, tr)) = target
        .split_once('/')
        .map(|(a, b)| (a.to_string(), b.to_string()))
    else {
        return Ok(invalid("repo must be owner/name"));
    };
    let Some(trepo) = st.find_repo(&to, &tr).cloned() else {
        return Ok(not_found());
    };
    let slug: String = i
        .title
        .to_ascii_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    let name = str_field(v, "name")
        .map(str::to_string)
        .unwrap_or_else(|| format!("{}-{slug}", i.number));
    let base = str_field(v, "base")
        .unwrap_or(&trepo.default_branch)
        .to_string();
    let mut sha = Value::Null;
    if let Some(d) = st.git_dir(&to, &tr) {
        let mut g = GitRepo::open(&d)?;
        if g.resolve(&format!("refs/heads/{name}")).is_some() {
            return Ok(invalid(format!("branch {name:?} already exists")));
        }
        let Some(tip) = g.resolve(&base) else {
            return Ok(invalid(format!("base branch {base:?} not found")));
        };
        g.set_branch(&name, &tip)?;
        sha = json!(tip.to_hex());
    }
    let link = LinkedBranch {
        name: name.clone(),
        repo: target.clone(),
    };
    if !st.db.issues[idx].linked_branches.contains(&link) {
        st.db.issues[idx].linked_branches.push(link);
    }
    st.save()?;
    Ok((
        201,
        json!({"name": name, "repository": {"full_name": target}, "base": base, "sha": sha,
        "url": format!("{}/{target}/tree/{name}", st.public_base)}),
    ))
}

fn labels_route(
    st: &mut HubStore,
    req: &Req<'_>,
    owner: &str,
    repo: &str,
    rest: &[&str],
) -> Result<Option<Resp>, String> {
    let v = req.json();
    match (req.method, rest) {
        ("GET", []) => {
            let items: Vec<Value> = st
                .db
                .labels
                .iter()
                .filter(|l| l.owner == owner && l.repo == repo)
                .map(label_json)
                .collect();
            Ok(Some((200, Value::Array(items))))
        }
        ("POST", []) => {
            let Some(name) = str_field(&v, "name") else {
                return Ok(Some(invalid("name is required")));
            };
            if st
                .db
                .labels
                .iter()
                .any(|l| l.owner == owner && l.repo == repo && l.name.eq_ignore_ascii_case(name))
            {
                return Ok(Some((
                    422,
                    json!({"message": "Validation Failed", "errors": [{"resource": "Label", "code": "already_exists", "field": "name"}]}),
                )));
            }
            let now = now_secs();
            let l = Label {
                id: next_id(&mut st.db.next_label),
                owner: owner.into(),
                repo: repo.into(),
                name: name.into(),
                color: str_field(&v, "color")
                    .unwrap_or("ededed")
                    .trim_start_matches('#')
                    .to_ascii_lowercase(),
                description: v
                    .get("description")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .into(),
                created_at: now,
                updated_at: now,
            };
            st.db.labels.push(l.clone());
            st.save()?;
            Ok(Some((201, label_json(&l))))
        }
        (m @ ("GET" | "PATCH" | "DELETE"), name) if !name.is_empty() => {
            let name = super::search::decode(&name.join("/"));
            let Some(idx) = st.db.labels.iter().position(|l| {
                l.owner == owner && l.repo == repo && l.name.eq_ignore_ascii_case(&name)
            }) else {
                return Ok(Some(not_found()));
            };
            match m {
                "GET" => Ok(Some((200, label_json(&st.db.labels[idx])))),
                "DELETE" => {
                    let old = st.db.labels.remove(idx).name;
                    for i in st
                        .db
                        .issues
                        .iter_mut()
                        .filter(|i| i.owner == owner && i.repo == repo)
                    {
                        i.labels.retain(|l| *l != old);
                    }
                    for p in st
                        .db
                        .pulls
                        .iter_mut()
                        .filter(|p| p.owner == owner && p.repo == repo)
                    {
                        p.labels.retain(|l| *l != old);
                    }
                    st.save()?;
                    Ok(Some(no_content()))
                }
                _ => {
                    let old = st.db.labels[idx].name.clone();
                    let new_name = str_field(&v, "new_name").map(str::to_string);
                    if let Some(nn) = &new_name {
                        if !nn.eq_ignore_ascii_case(&old)
                            && st.db.labels.iter().any(|l| {
                                l.owner == owner
                                    && l.repo == repo
                                    && l.name.eq_ignore_ascii_case(nn)
                            })
                        {
                            return Ok(Some(invalid(format!("label {nn:?} already exists"))));
                        }
                    }
                    let l = &mut st.db.labels[idx];
                    if let Some(nn) = &new_name {
                        l.name = nn.clone();
                    }
                    if let Some(c) = str_field(&v, "color") {
                        l.color = c.trim_start_matches('#').to_ascii_lowercase();
                    }
                    if let Some(d) = v.get("description").and_then(Value::as_str) {
                        l.description = d.into();
                    }
                    l.updated_at = now_secs();
                    let out = l.clone();
                    if let Some(nn) = new_name {
                        for i in st
                            .db
                            .issues
                            .iter_mut()
                            .filter(|i| i.owner == owner && i.repo == repo)
                        {
                            i.labels
                                .iter_mut()
                                .filter(|x| **x == old)
                                .for_each(|x| *x = nn.clone());
                        }
                        for p in st
                            .db
                            .pulls
                            .iter_mut()
                            .filter(|p| p.owner == owner && p.repo == repo)
                        {
                            p.labels
                                .iter_mut()
                                .filter(|x| **x == old)
                                .for_each(|x| *x = nn.clone());
                        }
                    }
                    st.save()?;
                    Ok(Some((200, label_json(&out))))
                }
            }
        }
        _ => Ok(None),
    }
}

fn milestones_route(
    st: &mut HubStore,
    req: &Req<'_>,
    owner: &str,
    repo: &str,
    rest: &[&str],
) -> Result<Option<Resp>, String> {
    let mj = |m: &Milestone| json!({"number": m.number, "title": m.title, "description": m.description, "state": m.state});
    let v = req.json();
    match (req.method, rest) {
        ("GET", []) => {
            let state = req.query_param("state").unwrap_or_else(|| "open".into());
            let items: Vec<Value> = st
                .db
                .milestones
                .iter()
                .filter(|m| {
                    m.owner == owner && m.repo == repo && (state == "all" || m.state == state)
                })
                .map(mj)
                .collect();
            Ok(Some((200, Value::Array(items))))
        }
        ("POST", []) => {
            let Some(title) = str_field(&v, "title") else {
                return Ok(Some(invalid("title is required")));
            };
            let m = Milestone {
                number: next_id(&mut st.db.next_milestone),
                owner: owner.into(),
                repo: repo.into(),
                title: title.into(),
                description: v
                    .get("description")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .into(),
                state: str_field(&v, "state").unwrap_or("open").into(),
            };
            st.db.milestones.push(m.clone());
            st.save()?;
            Ok(Some((201, mj(&m))))
        }
        (m, [n]) => {
            let Some(idx) = n.parse::<u64>().ok().and_then(|n| {
                st.db
                    .milestones
                    .iter()
                    .position(|x| x.owner == owner && x.repo == repo && x.number == n)
            }) else {
                return Ok(Some(not_found()));
            };
            match m {
                "GET" => Ok(Some((200, mj(&st.db.milestones[idx])))),
                "PATCH" => {
                    let x = &mut st.db.milestones[idx];
                    if let Some(t) = str_field(&v, "title") {
                        x.title = t.into();
                    }
                    if let Some(d) = v.get("description").and_then(Value::as_str) {
                        x.description = d.into();
                    }
                    if let Some(s) = str_field(&v, "state") {
                        x.state = s.into();
                    }
                    let out = mj(x);
                    st.save()?;
                    Ok(Some((200, out)))
                }
                "DELETE" => {
                    let num = st.db.milestones.remove(idx).number;
                    for i in
                        st.db.issues.iter_mut().filter(|i| {
                            i.owner == owner && i.repo == repo && i.milestone == Some(num)
                        })
                    {
                        i.milestone = None;
                    }
                    st.save()?;
                    Ok(Some(no_content()))
                }
                _ => Ok(None),
            }
        }
        _ => Ok(None),
    }
}

fn list_filter(st: &HubStore, req: &Req<'_>, i: &Issue) -> bool {
    let state = req.query_param("state").unwrap_or_else(|| "open".into());
    if state != "all" && i.state != state {
        return false;
    }
    if let Some(l) = req.query_param("labels") {
        if !l
            .split(',')
            .filter(|x| !x.is_empty())
            .all(|x| i.labels.iter().any(|y| y.eq_ignore_ascii_case(x)))
        {
            return false;
        }
    }
    let me = |s: String| if s == "@me" { st.actor.clone() } else { s };
    if let Some(a) = req.query_param("assignee").map(me) {
        if a == "none" && !i.assignees.is_empty()
            || a != "none" && a != "*" && !i.assignees.contains(&a)
        {
            return false;
        }
    }
    if let Some(c) = req.query_param("creator").map(me) {
        if i.user != c {
            return false;
        }
    }
    if let Some(m) = req.query_param("mentioned").map(me) {
        let tag = format!("@{m}");
        if !i.body.contains(&tag)
            && !st.db.comments.iter().any(|c| {
                c.owner == i.owner
                    && c.repo == i.repo
                    && c.number == i.number
                    && c.kind == "issue"
                    && c.body.contains(&tag)
            })
        {
            return false;
        }
    }
    if let Some(m) = req.query_param("milestone") {
        let ok = match m.as_str() {
            "*" => i.milestone.is_some(),
            "none" => i.milestone.is_none(),
            t => i.milestone.is_some_and(|n| {
                n.to_string() == t
                    || st.db.milestones.iter().any(|x| {
                        x.owner == i.owner && x.repo == i.repo && x.number == n && x.title == t
                    })
            }),
        };
        if !ok {
            return false;
        }
    }
    true
}

pub(crate) fn route(st: &mut HubStore, req: &Req<'_>) -> RouteResult {
    let Some(seg) = segments(req.path, "/repos/") else {
        return Ok(None);
    };
    if seg.len() < 3 || !matches!(seg[2], "issues" | "labels" | "milestones") {
        return Ok(None);
    }
    let (owner, repo) = (seg[0].to_string(), seg[1].to_string());
    if st.find_repo(&owner, &repo).is_none() {
        return Ok(Some(not_found()));
    }
    if seg[2] == "labels" {
        return labels_route(st, req, &owner, &repo, &seg[3..]);
    }
    if seg[2] == "milestones" {
        return milestones_route(st, req, &owner, &repo, &seg[3..]);
    }
    let rest = &seg[3..];
    let v = req.json();
    match (req.method, rest) {
        ("GET", []) => {
            let limit = req
                .query_param("per_page")
                .and_then(|p| p.parse().ok())
                .unwrap_or(usize::MAX);
            let mut items: Vec<&Issue> = st
                .db
                .issues
                .iter()
                .filter(|i| i.owner == owner && i.repo == repo && list_filter(st, req, i))
                .collect();
            items.sort_by(|a, b| b.number.cmp(&a.number));
            let out: Vec<Value> = items
                .into_iter()
                .take(limit)
                .map(|i| issue_json(st, i))
                .collect();
            Ok(Some((200, Value::Array(out))))
        }
        ("POST", []) => {
            let Some(title) = str_field(&v, "title") else {
                return Ok(Some(invalid("title is required")));
            };
            if !st.find_repo(&owner, &repo).is_some_and(|r| r.has_issues) {
                return Ok(Some((
                    410,
                    json!({"message": "Issues are disabled for this repository"}),
                )));
            }
            let ms = match milestone_arg(st, &owner, &repo, &v) {
                Ok(m) => m.flatten(),
                Err(e) => return Ok(Some(invalid(e))),
            };
            let labels = ensure_labels(
                st,
                &owner,
                &repo,
                &strings(&v, "labels").unwrap_or_default(),
            );
            let now = now_secs();
            let number = next_id(&mut st.db.next_issue);
            let i = Issue {
                number,
                owner: owner.clone(),
                repo: repo.clone(),
                title: title.into(),
                body: v.get("body").and_then(Value::as_str).unwrap_or("").into(),
                state: "open".into(),
                user: st.actor.clone(),
                locked: false,
                pinned: false,
                labels,
                assignees: strings(&v, "assignees").unwrap_or_default(),
                milestone: ms,
                state_reason: None,
                lock_reason: None,
                projects: strings(&v, "projects").unwrap_or_default(),
                linked_branches: Vec::new(),
                created_at: now,
                updated_at: now,
                closed_at: None,
            };
            st.db.issues.push(i.clone());
            st.save()?;
            Ok(Some((201, issue_json(st, &i))))
        }
        ("PATCH" | "DELETE", ["comments", id]) => {
            let Some(idx) = id.parse::<u64>().ok().and_then(|id| {
                st.db
                    .comments
                    .iter()
                    .position(|c| c.owner == owner && c.repo == repo && c.id == id)
            }) else {
                return Ok(Some(not_found()));
            };
            if req.method == "DELETE" {
                st.db.comments.remove(idx);
                st.save()?;
                return Ok(Some(no_content()));
            }
            let Some(body) = str_field(&v, "body") else {
                return Ok(Some(invalid("body is required")));
            };
            st.db.comments[idx].body = body.into();
            st.db.comments[idx].updated_at = now_secs();
            let c = st.db.comments[idx].clone();
            st.save()?;
            Ok(Some((200, comment_json(st, &c))))
        }
        (_, [n, tail @ ..]) => {
            let Ok(n) = n.parse::<u64>() else {
                return Ok(Some(not_found()));
            };
            let Some(idx) = find(st, &owner, &repo, n) else {
                return Ok(Some(not_found()));
            };
            match (req.method, tail) {
                ("GET", []) => Ok(Some((200, issue_json(st, &st.db.issues[idx].clone())))),
                ("PATCH", []) => {
                    if let Some(to) = str_field(&v, "transfer_to").map(str::to_string) {
                        return transfer(st, idx, &to).map(Some);
                    }
                    if let Some(e) = patch(st, idx, &v)? {
                        return Ok(Some(invalid(e)));
                    }
                    st.save()?;
                    Ok(Some((200, issue_json(st, &st.db.issues[idx].clone()))))
                }
                ("DELETE", []) => {
                    st.db.issues.remove(idx);
                    st.db.comments.retain(|c| {
                        !(c.owner == owner && c.repo == repo && c.number == n && c.kind == "issue")
                    });
                    st.db
                        .artifacts
                        .retain(|a| !(a.owner == owner && a.repo == repo && a.issue == n));
                    st.save()?;
                    Ok(Some(no_content()))
                }
                ("POST", ["transfer"]) => {
                    let to = str_field(&v, "new_repository")
                        .or(str_field(&v, "repository"))
                        .unwrap_or("")
                        .to_string();
                    transfer(st, idx, &to).map(Some)
                }
                (m @ ("PUT" | "DELETE"), ["lock"]) => {
                    let i = &mut st.db.issues[idx];
                    i.locked = m == "PUT";
                    i.lock_reason = if i.locked {
                        str_field(&v, "lock_reason").map(str::to_string)
                    } else {
                        None
                    };
                    st.save()?;
                    Ok(Some(no_content()))
                }
                (m @ ("PUT" | "DELETE"), ["pin"]) => {
                    st.db.issues[idx].pinned = m == "PUT";
                    st.save()?;
                    Ok(Some(no_content()))
                }
                ("POST", ["labels"]) => {
                    let add = ensure_labels(
                        st,
                        &owner,
                        &repo,
                        &strings(&v, "labels").unwrap_or_default(),
                    );
                    let i = &mut st.db.issues[idx];
                    for l in add {
                        if !i.labels.contains(&l) {
                            i.labels.push(l);
                        }
                    }
                    let out = labels_json(st, &owner, &repo, &st.db.issues[idx].labels.clone());
                    st.save()?;
                    Ok(Some((200, out)))
                }
                ("DELETE", ["labels", name @ ..]) => {
                    let name = super::search::decode(&name.join("/"));
                    st.db.issues[idx].labels.retain(|l| *l != name);
                    let out = labels_json(st, &owner, &repo, &st.db.issues[idx].labels.clone());
                    st.save()?;
                    Ok(Some((200, out)))
                }
                (m @ ("POST" | "DELETE"), ["assignees"]) => {
                    let who = strings(&v, "assignees").unwrap_or_default();
                    let i = &mut st.db.issues[idx];
                    if m == "POST" {
                        for w in who {
                            if !i.assignees.contains(&w) {
                                i.assignees.push(w);
                            }
                        }
                    } else {
                        i.assignees.retain(|a| !who.contains(a));
                    }
                    st.save()?;
                    Ok(Some((201, issue_json(st, &st.db.issues[idx].clone()))))
                }
                (_, ["comments"]) => comments_route(st, req, &owner, &repo, n, "issue").map(Some),
                ("GET", ["branches"]) => {
                    let items: Vec<Value> = st.db.issues[idx]
                        .linked_branches
                        .iter()
                        .map(|b| {
                            json!({"name": b.name, "repository": {"full_name": b.repo},
                            "url": format!("{}/{}/tree/{}", st.public_base, b.repo, b.name)})
                        })
                        .collect();
                    Ok(Some((200, Value::Array(items))))
                }
                ("POST", ["branches"]) => develop(st, idx, &v).map(Some),
                _ => Ok(None),
            }
        }
        _ => Ok(None),
    }
}
