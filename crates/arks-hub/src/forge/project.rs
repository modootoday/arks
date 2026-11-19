//! Owner-scoped projects with custom fields and items (issues, pull requests,
//! draft issues).
//!
//! Routes (owner = user or organization login; `/orgs/…` is accepted as an alias):
//! - `/users/{owner}/projectsV2` GET (`?state=open|closed|all`), POST
//! - `/users/{owner}/projectsV2/{number}` GET, PATCH, DELETE
//! - `/users/{owner}/projectsV2/{number}/copy` POST
//! - `/users/{owner}/projectsV2/{number}/repositories/{o}/{r}` PUT (link), DELETE (unlink)
//! - `/users/{owner}/projectsV2/{number}/fields` GET, POST
//! - `/users/{owner}/projectsV2/{number}/items` GET (`?archived=true`), POST
//! - `/users/{owner}/projectsV2/{number}/items/{id}` PATCH (`archived`), DELETE
//! - `/projectsV2/fields/{id}` DELETE
//! - `/projectsV2/items/{id}` PATCH (field value / draft title·body)
//!
//! A project number is unique forge-wide. Field data types: TEXT, NUMBER, DATE,
//! SINGLE_SELECT. arks has no iteration primitive.

use super::{
    Req, RouteResult, invalid, iso8601, method_not_allowed, next_id, no_content, not_found,
    repo_exists, segments, str_field,
};
use crate::store::{HubStore, now_secs};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;

pub const FIELD_TYPES: &[&str] = &["TEXT", "NUMBER", "DATE", "SINGLE_SELECT"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    /// Project number (unique forge-wide).
    pub id: u64,
    pub owner: String,
    /// Legacy repo scope from older db files (treated as a linked repository).
    #[serde(default)]
    pub repo: String,
    pub title: String,
    /// Short description.
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub readme: String,
    pub state: String,
    #[serde(default)]
    pub public: bool,
    #[serde(default)]
    pub template: bool,
    #[serde(default)]
    pub linked_repos: Vec<String>,
    #[serde(default)]
    pub created_at: u64,
}

impl Project {
    fn linked(&self) -> Vec<String> {
        let mut out = self.linked_repos.clone();
        if !self.repo.is_empty() {
            let legacy = format!("{}/{}", self.owner, self.repo);
            if !out.contains(&legacy) {
                out.insert(0, legacy);
            }
        }
        out
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldOption {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectField {
    pub id: u64,
    pub project: u64,
    pub name: String,
    pub data_type: String,
    #[serde(default)]
    pub options: Vec<FieldOption>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectItem {
    pub id: u64,
    pub project: u64,
    /// `Issue`, `PullRequest` or `DraftIssue`.
    pub content_type: String,
    pub title: String,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub archived: bool,
    /// Field id → value.
    #[serde(default)]
    pub values: BTreeMap<u64, Value>,
}

fn project_json(st: &HubStore, p: &Project) -> Value {
    let items = st
        .db
        .project_items
        .iter()
        .filter(|i| i.project == p.id)
        .count();
    let fields = st
        .db
        .project_fields
        .iter()
        .filter(|f| f.project == p.id)
        .count();
    json!({
        "number": p.id,
        "title": p.title,
        "short_description": p.body,
        "readme": p.readme,
        "state": p.state,
        "closed": p.state == "closed",
        "public": p.public,
        "template": p.template,
        "owner": {"login": p.owner},
        "linked_repositories": p.linked(),
        "items_count": items,
        "fields_count": fields,
        "created_at": iso8601(p.created_at),
    })
}

fn field_json(f: &ProjectField) -> Value {
    let options: Vec<Value> = f
        .options
        .iter()
        .map(|o| json!({"id": o.id, "name": o.name}))
        .collect();
    json!({
        "id": f.id,
        "name": f.name,
        "data_type": f.data_type,
        "options": options,
    })
}

fn item_json(st: &HubStore, i: &ProjectItem) -> Value {
    let mut values = Map::new();
    for (fid, v) in &i.values {
        if let Some(f) = st.db.project_fields.iter().find(|f| f.id == *fid) {
            let shown = if f.data_type == "SINGLE_SELECT" {
                f.options
                    .iter()
                    .find(|o| Some(o.id.as_str()) == v.as_str())
                    .map(|o| Value::String(o.name.clone()))
                    .unwrap_or(Value::Null)
            } else {
                v.clone()
            };
            values.insert(f.name.clone(), shown);
        }
    }
    json!({
        "id": i.id,
        "content_type": i.content_type,
        "title": i.title,
        "body": i.body,
        "url": i.url,
        "archived": i.archived,
        "field_values": values,
    })
}

/// Validate a field value against its data type; returns the stored value.
fn coerce_value(f: &ProjectField, v: &Value) -> Result<Value, String> {
    match f.data_type.as_str() {
        "TEXT" => v
            .as_str()
            .map(|s| Value::String(s.to_string()))
            .ok_or_else(|| "TEXT field needs a string".to_string()),
        "NUMBER" => v
            .as_f64()
            .or_else(|| v.as_str().and_then(|s| s.trim().parse().ok()))
            .map(|n| json!(n))
            .ok_or_else(|| "NUMBER field needs a number".to_string()),
        "DATE" => {
            let s = v.as_str().unwrap_or("");
            let ok = s.len() == 10
                && s.as_bytes()[4] == b'-'
                && s.as_bytes()[7] == b'-'
                && s[..4].parse::<u32>().is_ok()
                && s[5..7].parse::<u32>().is_ok_and(|m| (1..=12).contains(&m))
                && s[8..].parse::<u32>().is_ok_and(|d| (1..=31).contains(&d));
            if ok {
                Ok(Value::String(s.to_string()))
            } else {
                Err("DATE field needs YYYY-MM-DD".into())
            }
        }
        "SINGLE_SELECT" => {
            let id = v.as_str().unwrap_or("");
            if f.options.iter().any(|o| o.id == id) {
                Ok(Value::String(id.to_string()))
            } else {
                Err(format!(
                    "option `{id}` is not defined on field `{}`",
                    f.name
                ))
            }
        }
        other => Err(format!("unsupported field type {other}")),
    }
}

/// Resolve `{owner}/{repo}/(issues|pull|pulls)/{n}` or `owner/repo#n`.
fn parse_content_ref(url: &str) -> Option<(String, String, u64)> {
    let clean = url.split(['?', '#']).next().unwrap_or("");
    if let Some((slug, n)) = url.split_once('#') {
        if let (Some((o, r)), Ok(n)) = (slug.split_once('/'), n.parse()) {
            if !o.contains(':') && !r.contains('/') {
                return Some((o.to_string(), r.to_string(), n));
            }
        }
    }
    let parts: Vec<&str> = clean.trim_end_matches('/').split('/').collect();
    let k = parts.len();
    if k >= 4 && matches!(parts[k - 2], "issues" | "pull" | "pulls") {
        let n = parts[k - 1].parse().ok()?;
        return Some((parts[k - 4].to_string(), parts[k - 3].to_string(), n));
    }
    None
}

impl HubStore {
    fn find_project(&self, owner: &str, number: u64) -> Option<usize> {
        self.db
            .projects
            .iter()
            .position(|p| p.owner == owner && p.id == number)
    }

    pub(crate) fn create_project(&mut self, owner: &str, title: &str) -> Project {
        let p = Project {
            id: next_id(&mut self.db.next_project),
            owner: owner.into(),
            repo: String::new(),
            title: title.into(),
            body: String::new(),
            readme: String::new(),
            state: "open".into(),
            public: false,
            template: false,
            linked_repos: Vec::new(),
            created_at: now_secs(),
        };
        self.db.projects.push(p.clone());
        p
    }

    fn copy_project(&mut self, src: &Project, owner: &str, title: &str, drafts: bool) -> Project {
        let mut p = self.create_project(owner, title);
        p.body = src.body.clone();
        p.readme = src.readme.clone();
        p.public = src.public;
        if let Some(last) = self.db.projects.last_mut() {
            *last = p.clone();
        }
        let mut remap = BTreeMap::new();
        let fields: Vec<ProjectField> = self
            .db
            .project_fields
            .iter()
            .filter(|f| f.project == src.id)
            .cloned()
            .collect();
        for f in fields {
            let id = next_id(&mut self.db.next_project_field);
            remap.insert(f.id, id);
            let options = f
                .options
                .iter()
                .enumerate()
                .map(|(i, o)| FieldOption {
                    id: format!("{id}_{}", i + 1),
                    name: o.name.clone(),
                })
                .collect();
            self.db.project_fields.push(ProjectField {
                id,
                project: p.id,
                name: f.name.clone(),
                data_type: f.data_type.clone(),
                options,
            });
        }
        if drafts {
            let items: Vec<ProjectItem> = self
                .db
                .project_items
                .iter()
                .filter(|i| i.project == src.id && i.content_type == "DraftIssue")
                .cloned()
                .collect();
            for it in items {
                let mut values = BTreeMap::new();
                for (fid, v) in &it.values {
                    let Some(nid) = remap.get(fid) else { continue };
                    let nv = match v.as_str().and_then(|s| s.rsplit_once('_')) {
                        // Single-select option ids are `{field}_{index}`.
                        Some((_, idx)) if self.field_type(*nid) == Some("SINGLE_SELECT") => {
                            Value::String(format!("{nid}_{idx}"))
                        }
                        _ => v.clone(),
                    };
                    values.insert(*nid, nv);
                }
                let id = next_id(&mut self.db.next_project_item);
                self.db.project_items.push(ProjectItem {
                    id,
                    project: p.id,
                    values,
                    archived: false,
                    ..it
                });
            }
        }
        p
    }

    fn field_type(&self, id: u64) -> Option<&str> {
        self.db
            .project_fields
            .iter()
            .find(|f| f.id == id)
            .map(|f| f.data_type.as_str())
    }

    fn delete_project(&mut self, idx: usize) {
        let p = self.db.projects.remove(idx);
        self.db.project_fields.retain(|f| f.project != p.id);
        self.db.project_items.retain(|i| i.project != p.id);
    }
}

pub(crate) fn route(st: &mut HubStore, req: &Req<'_>) -> RouteResult {
    if let Some(seg) = segments(req.path, "/projectsV2/") {
        return global_route(st, req, &seg);
    }
    let seg = match segments(req.path, "/users/").or_else(|| segments(req.path, "/orgs/")) {
        Some(s) if s.len() >= 2 && s[1] == "projectsV2" => s,
        _ => return Ok(None),
    };
    let owner = if seg[0] == "@me" {
        st.db.user_login.clone()
    } else {
        seg[0].to_string()
    };
    let rest = &seg[2..];
    if rest.is_empty() {
        return collection_route(st, req, &owner);
    }
    let Ok(number) = rest[0].parse::<u64>() else {
        return Ok(Some(not_found()));
    };
    let Some(idx) = st.find_project(&owner, number) else {
        return Ok(Some(not_found()));
    };
    match (req.method, &rest[1..]) {
        ("GET", []) => Ok(Some((200, project_json(st, &st.db.projects[idx])))),
        ("PATCH", []) => {
            let v = req.json();
            if v.get("title").is_some() && str_field(&v, "title").is_none() {
                return Ok(Some(invalid("title can't be blank")));
            }
            let p = &mut st.db.projects[idx];
            if let Some(t) = str_field(&v, "title") {
                p.title = t.to_string();
            }
            if let Some(d) = v.get("short_description").and_then(Value::as_str) {
                p.body = d.to_string();
            }
            if let Some(r) = v.get("readme").and_then(Value::as_str) {
                p.readme = r.to_string();
            }
            if let Some(b) = v.get("public").and_then(Value::as_bool) {
                p.public = b;
            }
            if let Some(b) = v.get("closed").and_then(Value::as_bool) {
                p.state = if b { "closed" } else { "open" }.into();
            }
            if let Some(b) = v.get("template").and_then(Value::as_bool) {
                p.template = b;
            }
            st.save()?;
            Ok(Some((200, project_json(st, &st.db.projects[idx]))))
        }
        ("DELETE", []) => {
            let out = project_json(st, &st.db.projects[idx]);
            st.delete_project(idx);
            st.save()?;
            Ok(Some((200, out)))
        }
        ("POST", ["copy"]) => {
            let v = req.json();
            let Some(title) = str_field(&v, "title") else {
                return Ok(Some(invalid("title can't be blank")));
            };
            let target = match str_field(&v, "owner") {
                Some("@me") | None => st.db.user_login.clone(),
                Some(o) => o.to_string(),
            };
            let drafts = v
                .get("include_draft_issues")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let src = st.db.projects[idx].clone();
            let p = st.copy_project(&src, &target, title, drafts);
            st.save()?;
            Ok(Some((201, project_json(st, &p))))
        }
        (m @ ("PUT" | "DELETE"), ["repositories", ro, rn]) => {
            if !repo_exists(st, ro, rn) {
                return Ok(Some(not_found()));
            }
            let slug = format!("{ro}/{rn}");
            let p = &mut st.db.projects[idx];
            if m == "PUT" {
                if !p.linked().contains(&slug) {
                    p.linked_repos.push(slug);
                }
            } else {
                let legacy = format!("{}/{}", p.owner, p.repo);
                if !p.repo.is_empty() && legacy == slug {
                    p.repo.clear();
                } else if p.linked_repos.contains(&slug) {
                    p.linked_repos.retain(|s| *s != slug);
                } else {
                    return Ok(Some(invalid(format!(
                        "{slug} is not linked to this project"
                    ))));
                }
            }
            st.save()?;
            Ok(Some((200, project_json(st, &st.db.projects[idx]))))
        }
        ("GET", ["fields"]) => {
            let arr = st
                .db
                .project_fields
                .iter()
                .filter(|f| f.project == number)
                .map(field_json)
                .collect();
            Ok(Some((200, Value::Array(arr))))
        }
        ("POST", ["fields"]) => {
            let v = req.json();
            let Some(name) = str_field(&v, "name") else {
                return Ok(Some(invalid("name can't be blank")));
            };
            let data_type = str_field(&v, "data_type")
                .unwrap_or("")
                .to_ascii_uppercase();
            if !FIELD_TYPES.contains(&data_type.as_str()) {
                return Ok(Some(invalid(format!(
                    "data_type must be one of {}",
                    FIELD_TYPES.join(", ")
                ))));
            }
            if st
                .db
                .project_fields
                .iter()
                .any(|f| f.project == number && f.name.eq_ignore_ascii_case(name))
            {
                return Ok(Some(invalid(format!("field `{name}` already exists"))));
            }
            let opts: Vec<String> = v
                .get("options")
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(Value::as_str)
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_default();
            if data_type == "SINGLE_SELECT" && opts.is_empty() {
                return Ok(Some(invalid("SINGLE_SELECT needs at least one option")));
            }
            if data_type != "SINGLE_SELECT" && !opts.is_empty() {
                return Ok(Some(invalid("options apply to SINGLE_SELECT only")));
            }
            let id = next_id(&mut st.db.next_project_field);
            let f = ProjectField {
                id,
                project: number,
                name: name.to_string(),
                data_type,
                options: opts
                    .into_iter()
                    .enumerate()
                    .map(|(i, name)| FieldOption {
                        id: format!("{id}_{}", i + 1),
                        name,
                    })
                    .collect(),
            };
            st.db.project_fields.push(f.clone());
            st.save()?;
            Ok(Some((201, field_json(&f))))
        }
        ("GET", ["items"]) => {
            let archived = req.query_param("archived").as_deref() == Some("true");
            let items: Vec<ProjectItem> = st
                .db
                .project_items
                .iter()
                .filter(|i| i.project == number && i.archived == archived)
                .cloned()
                .collect();
            let arr = items.iter().map(|i| item_json(st, i)).collect();
            Ok(Some((200, Value::Array(arr))))
        }
        ("POST", ["items"]) => {
            let v = req.json();
            let item = if let Some(url) = str_field(&v, "url") {
                let Some((o, r, n)) = parse_content_ref(url) else {
                    return Ok(Some(invalid(
                        "url must name an issue or pull request (…/owner/repo/issues/N)",
                    )));
                };
                if let Some(existing) = st
                    .db
                    .project_items
                    .iter()
                    .find(|i| i.project == number && i.url == url)
                {
                    let out = item_json(st, existing);
                    return Ok(Some((200, out)));
                }
                let found = st
                    .db
                    .issues
                    .iter()
                    .find(|i| i.owner == o && i.repo == r && i.number == n)
                    .map(|i| ("Issue", i.title.clone(), i.body.clone()))
                    .or_else(|| {
                        st.db
                            .pulls
                            .iter()
                            .find(|p| p.owner == o && p.repo == r && p.number == n)
                            .map(|p| ("PullRequest", p.title.clone(), p.body.clone()))
                    });
                let Some((kind, title, body)) = found else {
                    return Ok(Some(not_found()));
                };
                ProjectItem {
                    id: next_id(&mut st.db.next_project_item),
                    project: number,
                    content_type: kind.into(),
                    title,
                    body,
                    url: url.to_string(),
                    archived: false,
                    values: BTreeMap::new(),
                }
            } else {
                let Some(title) = str_field(&v, "title") else {
                    return Ok(Some(invalid("url or title is required")));
                };
                ProjectItem {
                    id: next_id(&mut st.db.next_project_item),
                    project: number,
                    content_type: "DraftIssue".into(),
                    title: title.to_string(),
                    body: v.get("body").and_then(Value::as_str).unwrap_or("").into(),
                    url: String::new(),
                    archived: false,
                    values: BTreeMap::new(),
                }
            };
            st.db.project_items.push(item.clone());
            st.save()?;
            Ok(Some((201, item_json(st, &item))))
        }
        (m @ ("PATCH" | "DELETE"), ["items", id]) => {
            let id = id.parse::<u64>().unwrap_or(0);
            let Some(pos) = st
                .db
                .project_items
                .iter()
                .position(|i| i.project == number && i.id == id)
            else {
                return Ok(Some(not_found()));
            };
            if m == "DELETE" {
                st.db.project_items.remove(pos);
                st.save()?;
                return Ok(Some(no_content()));
            }
            let v = req.json();
            if let Some(b) = v.get("archived").and_then(Value::as_bool) {
                st.db.project_items[pos].archived = b;
            }
            st.save()?;
            let out = item_json(st, &st.db.project_items[pos]);
            Ok(Some((200, out)))
        }
        _ => Ok(Some(method_not_allowed())),
    }
}

fn collection_route(st: &mut HubStore, req: &Req<'_>, owner: &str) -> RouteResult {
    match req.method {
        "GET" => {
            let state = req.query_param("state").unwrap_or_else(|| "open".into());
            let projects: Vec<Project> = st
                .db
                .projects
                .iter()
                .filter(|p| p.owner == owner && (state == "all" || p.state == state))
                .cloned()
                .collect();
            let arr = projects.iter().map(|p| project_json(st, p)).collect();
            Ok(Some((200, Value::Array(arr))))
        }
        "POST" => {
            let v = req.json();
            let Some(title) = str_field(&v, "title") else {
                return Ok(Some(invalid("title can't be blank")));
            };
            let p = st.create_project(owner, title);
            st.save()?;
            Ok(Some((201, project_json(st, &p))))
        }
        _ => Ok(Some(method_not_allowed())),
    }
}

fn global_route(st: &mut HubStore, req: &Req<'_>, seg: &[&str]) -> RouteResult {
    match (req.method, seg) {
        ("DELETE", ["fields", id]) => {
            let id = id.parse::<u64>().unwrap_or(0);
            let before = st.db.project_fields.len();
            st.db.project_fields.retain(|f| f.id != id);
            if st.db.project_fields.len() == before {
                return Ok(Some(not_found()));
            }
            for item in &mut st.db.project_items {
                item.values.remove(&id);
            }
            st.save()?;
            Ok(Some(no_content()))
        }
        ("PATCH", ["items", id]) => {
            let id = id.parse::<u64>().unwrap_or(0);
            let Some(pos) = st.db.project_items.iter().position(|i| i.id == id) else {
                return Ok(Some(not_found()));
            };
            let v = req.json();
            if let Some(p) = v.get("project").and_then(Value::as_u64) {
                if st.db.project_items[pos].project != p {
                    return Ok(Some(invalid("item does not belong to that project")));
                }
            }
            if v.get("title").is_some() || v.get("body").is_some() {
                if st.db.project_items[pos].content_type != "DraftIssue" {
                    return Ok(Some(invalid(
                        "title/body can be edited on draft issues only; edit the issue or pull request itself",
                    )));
                }
                if let Some(t) = str_field(&v, "title") {
                    st.db.project_items[pos].title = t.to_string();
                }
                if let Some(b) = v.get("body").and_then(Value::as_str) {
                    st.db.project_items[pos].body = b.to_string();
                }
            }
            if let Some(fid) = v.get("field_id").and_then(Value::as_u64) {
                let project = st.db.project_items[pos].project;
                let Some(field) = st
                    .db
                    .project_fields
                    .iter()
                    .find(|f| f.id == fid && f.project == project)
                    .cloned()
                else {
                    return Ok(Some(invalid(
                        "field_id is not a field of the item's project",
                    )));
                };
                if v.get("clear").and_then(Value::as_bool) == Some(true) {
                    st.db.project_items[pos].values.remove(&fid);
                } else {
                    let raw = v.get("value").cloned().unwrap_or(Value::Null);
                    match coerce_value(&field, &raw) {
                        Ok(val) => {
                            st.db.project_items[pos].values.insert(fid, val);
                        }
                        Err(e) => return Ok(Some(invalid(e))),
                    }
                }
            }
            st.save()?;
            let out = item_json(st, &st.db.project_items[pos]);
            Ok(Some((200, out)))
        }
        _ => Ok(Some(not_found())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_refs_parse() {
        assert_eq!(
            parse_content_ref("http://h:1/local/x/issues/3"),
            Some(("local".into(), "x".into(), 3))
        );
        assert_eq!(
            parse_content_ref("local/x/pull/9"),
            Some(("local".into(), "x".into(), 9))
        );
        assert_eq!(
            parse_content_ref("local/x#4"),
            Some(("local".into(), "x".into(), 4))
        );
        assert_eq!(parse_content_ref("nonsense"), None);
    }

    #[test]
    fn legacy_repo_project_loads_and_counts_as_linked() {
        let p: Project = serde_json::from_str(
            r#"{"id":1,"owner":"local","repo":"x","title":"t","body":"b","state":"open"}"#,
        )
        .expect("legacy project");
        assert_eq!(p.linked(), vec!["local/x".to_string()]);
    }

    #[test]
    fn coerce_values_by_type() {
        let mk = |t: &str| ProjectField {
            id: 1,
            project: 1,
            name: "f".into(),
            data_type: t.into(),
            options: vec![FieldOption {
                id: "1_1".into(),
                name: "Todo".into(),
            }],
        };
        assert!(coerce_value(&mk("TEXT"), &json!("x")).is_ok());
        assert!(coerce_value(&mk("NUMBER"), &json!("2.5")).is_ok());
        assert!(coerce_value(&mk("NUMBER"), &json!("x")).is_err());
        assert!(coerce_value(&mk("DATE"), &json!("2026-11-19")).is_ok());
        assert!(coerce_value(&mk("DATE"), &json!("2026-13-01")).is_err());
        assert!(coerce_value(&mk("SINGLE_SELECT"), &json!("1_1")).is_ok());
        assert!(coerce_value(&mk("SINGLE_SELECT"), &json!("9")).is_err());
    }
}
