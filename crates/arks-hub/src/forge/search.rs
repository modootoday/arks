//! Search over the forge: repositories, issues, pull requests, code and commits
//! (`/search/{repositories|issues|code|commits}?q=…`), with gh's search qualifier
//! syntax (`repo:`, `is:`, `label:`, `language:`, ranges like `comments:>5`,
//! `created:2026-01-01..2026-02-01`, negation with `-`). Every qualifier is
//! evaluated against what the arks forge records; counts arks does not keep
//! (stars, reactions, followers) are 0, so ranges over them are honest.
//! Code and commit search read the repositories' Git content in-process.

use super::git::GitRepo;
use super::{Req, RouteResult, iso8601, segments};
use crate::store::{HubStore, Issue, Pull, Repo};
use serde_json::{Value, json};

pub(crate) fn decode(s: &str) -> String {
    crate::serve::urlencoding_lite_decode(s)
}

/// One parsed query: free-text terms and (negated?, key, value) qualifiers.
pub(crate) struct Query {
    pub terms: Vec<String>,
    pub quals: Vec<(bool, String, String)>,
}

pub(crate) fn parse_query(q: &str) -> Query {
    let mut toks = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    for c in q.chars() {
        match c {
            '"' => quoted = !quoted,
            c if c.is_whitespace() && !quoted => {
                if !cur.is_empty() {
                    toks.push(std::mem::take(&mut cur));
                }
            }
            c => cur.push(c),
        }
    }
    if !cur.is_empty() {
        toks.push(cur);
    }
    let mut out = Query {
        terms: Vec::new(),
        quals: Vec::new(),
    };
    for t in toks {
        let (neg, body) = match t.strip_prefix('-') {
            Some(b) if b.contains(':') => (true, b.to_string()),
            _ => (false, t.clone()),
        };
        match body.split_once(':') {
            Some((k, v))
                if !k.is_empty()
                    && k.chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') =>
            {
                out.quals.push((neg, k.to_ascii_lowercase(), v.to_string()));
            }
            _ => {
                if t != "AND" && t != "OR" && t != "NOT" {
                    out.terms.push(t.to_ascii_lowercase());
                }
            }
        }
    }
    out
}

/// `N`, `>N`, `>=N`, `<N`, `<=N`, `a..b`, `a..*`, `*..b` over numbers.
pub(crate) fn num_match(spec: &str, v: f64) -> bool {
    let p = |s: &str| s.trim().parse::<f64>().ok();
    if let Some((a, b)) = spec.split_once("..") {
        return (a == "*" || p(a).is_some_and(|a| v >= a))
            && (b == "*" || p(b).is_some_and(|b| v <= b));
    }
    for (op, f) in [(">=", 0), ("<=", 1), (">", 2), ("<", 3)] {
        if let Some(r) = spec.strip_prefix(op) {
            return p(r).is_some_and(|x| match f {
                0 => v >= x,
                1 => v <= x,
                2 => v > x,
                _ => v < x,
            });
        }
    }
    p(spec).is_some_and(|x| v == x)
}

/// The same forms over RFC 3339 dates (compared on the qualifier's precision).
pub(crate) fn date_match(spec: &str, iso: &str) -> bool {
    let cmp = |bound: &str| -> std::cmp::Ordering {
        let n = bound.len().min(iso.len());
        iso[..n].cmp(&bound[..n])
    };
    if let Some((a, b)) = spec.split_once("..") {
        return (a == "*" || cmp(a).is_ge()) && (b == "*" || cmp(b).is_le());
    }
    for (op, f) in [(">=", 0), ("<=", 1), (">", 2), ("<", 3)] {
        if let Some(r) = spec.strip_prefix(op) {
            return match f {
                0 => cmp(r).is_ge(),
                1 => cmp(r).is_le(),
                2 => cmp(r).is_gt(),
                _ => cmp(r).is_lt(),
            };
        }
    }
    cmp(spec).is_eq()
}

fn eqi(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

fn owner_q(k: &str) -> bool {
    matches!(k, "user" | "org" | "owner")
}

fn repo_of(st: &HubStore, owner: &str, name: &str) -> Option<Repo> {
    st.find_repo(owner, name).cloned()
}

/// Text match helper: all terms appear in one of `fields`.
fn terms_hit(q: &Query, fields: &[&str]) -> bool {
    q.terms.iter().all(|t| {
        fields
            .iter()
            .any(|f| f.to_ascii_lowercase().contains(t.as_str()))
    })
}

enum Item<'a> {
    Issue(&'a Issue),
    Pull(&'a Pull),
}

fn issue_like_match(st: &HubStore, q: &Query, it: &Item<'_>) -> bool {
    let (
        owner,
        repo,
        number,
        title,
        body,
        state,
        user,
        labels,
        assignees,
        milestone,
        locked,
        created,
        updated,
        closed,
        kind,
    ) = match it {
        Item::Issue(i) => (
            &i.owner,
            &i.repo,
            i.number,
            &i.title,
            &i.body,
            i.state.as_str(),
            &i.user,
            &i.labels,
            &i.assignees,
            i.milestone,
            i.locked,
            i.created_at,
            i.updated_at,
            i.closed_at,
            "issue",
        ),
        Item::Pull(p) => (
            &p.owner,
            &p.repo,
            p.number,
            &p.title,
            &p.body,
            p.state.as_str(),
            &p.user,
            &p.labels,
            &p.assignees,
            p.milestone,
            p.locked,
            p.created_at,
            p.updated_at,
            p.closed_at,
            "pull",
        ),
    };
    let comments: Vec<&crate::store::Comment> = st
        .db
        .comments
        .iter()
        .filter(|c| c.owner == *owner && c.repo == *repo && c.number == number && c.kind == kind)
        .collect();
    let r = repo_of(st, owner, repo);
    let me = |s: &str| {
        if s == "@me" {
            st.actor.clone()
        } else {
            s.to_string()
        }
    };
    let mut in_fields: Vec<&str> = Vec::new();
    for (neg, k, v) in &q.quals {
        let v = v.as_str();
        let hit = match k.as_str() {
            "repo" => eqi(&format!("{owner}/{repo}"), v),
            k if owner_q(k) => eqi(owner, v),
            "is" => match v {
                "open" | "closed" => state == v,
                "issue" => kind == "issue",
                "pr" => kind == "pull",
                "merged" => matches!(it, Item::Pull(p) if p.merged),
                "unmerged" => matches!(it, Item::Pull(p) if !p.merged && p.state == "closed"),
                "draft" => matches!(it, Item::Pull(p) if p.draft),
                "locked" => locked,
                "unlocked" => !locked,
                "public" => r.as_ref().is_some_and(|r| !r.private),
                "private" => r.as_ref().is_some_and(|r| r.private),
                "archived" => r.as_ref().is_some_and(|r| r.archived),
                _ => false,
            },
            "type" => (v == "pr") == (kind == "pull"),
            "state" => state == v,
            "label" => v.split(',').any(|l| labels.iter().any(|x| eqi(x, l))),
            "no" => match v {
                "label" => labels.is_empty(),
                "milestone" => milestone.is_none(),
                "assignee" => assignees.is_empty(),
                "project" => match it {
                    Item::Issue(i) => i.projects.is_empty(),
                    Item::Pull(p) => p.projects.is_empty(),
                },
                _ => false,
            },
            "author" => eqi(user, &me(v)),
            "assignee" => assignees.iter().any(|a| eqi(a, &me(v))),
            "mentions" => {
                body.contains(&format!("@{}", me(v)))
                    || comments
                        .iter()
                        .any(|c| c.body.contains(&format!("@{}", me(v))))
            }
            "commenter" => comments.iter().any(|c| eqi(&c.user, &me(v))),
            "involves" => {
                let w = me(v);
                eqi(user, &w)
                    || assignees.iter().any(|a| eqi(a, &w))
                    || comments.iter().any(|c| eqi(&c.user, &w))
                    || body.contains(&format!("@{w}"))
            }
            "milestone" => milestone.is_some_and(|n| {
                st.db.milestones.iter().any(|m| {
                    m.owner == *owner && m.repo == *repo && m.number == n && eqi(&m.title, v)
                })
            }),
            "project" => match it {
                Item::Issue(i) => i.projects.iter().any(|p| eqi(p, v)),
                Item::Pull(p) => p.projects.iter().any(|x| eqi(x, v)),
            },
            "comments" => num_match(v, comments.len() as f64),
            "interactions" => num_match(v, comments.len() as f64),
            "reactions" => num_match(v, 0.0),
            "created" => date_match(v, &iso8601(created)),
            "updated" => date_match(v, &iso8601(updated.max(created))),
            "closed" => closed.is_some_and(|c| date_match(v, &iso8601(c))),
            "merged" => {
                matches!(it, Item::Pull(p) if p.merged_at.is_some_and(|m| date_match(v, &iso8601(m))))
            }
            "base" => matches!(it, Item::Pull(p) if eqi(&p.base, v)),
            "head" => {
                matches!(it, Item::Pull(p) if eqi(&p.head, v) || p.head.ends_with(&format!(":{v}")))
            }
            "draft" => matches!(it, Item::Pull(p) if p.draft == (v == "true")),
            "review" => match it {
                Item::Pull(p) => {
                    let states: Vec<&str> = st
                        .db
                        .reviews
                        .iter()
                        .filter(|r| {
                            r.owner == p.owner && r.repo == p.repo && r.pull_number == p.number
                        })
                        .map(|r| r.state.as_str())
                        .collect();
                    match v {
                        "none" => states.is_empty(),
                        "required" => states.is_empty() && !p.reviewers.is_empty(),
                        "approved" => states.last() == Some(&"APPROVED"),
                        "changes_requested" => states.last() == Some(&"CHANGES_REQUESTED"),
                        _ => false,
                    }
                }
                Item::Issue(_) => false,
            },
            "reviewed-by" => {
                matches!(it, Item::Pull(p) if st.db.reviews.iter().any(|r| r.owner == p.owner && r.repo == p.repo && r.pull_number == p.number && eqi(&r.user, &me(v))))
            }
            "review-requested" => {
                matches!(it, Item::Pull(p) if p.reviewers.iter().any(|r| eqi(r, &me(v))))
            }
            "status" => match it {
                Item::Pull(p) => {
                    let sha = super::status::key(st, &p.owner, &p.repo, &p.head);
                    super::status::combined_state(&super::status::latest(
                        st, &p.owner, &p.repo, &sha,
                    )) == v
                }
                Item::Issue(_) => false,
            },
            "archived" => r.as_ref().is_some_and(|r| r.archived == (v == "true")),
            "language" | "team" | "app" => false,
            "in" => {
                for f in v.split(',') {
                    match f {
                        "title" => in_fields.push(title),
                        "body" => in_fields.push(body),
                        "comments" => in_fields.extend(comments.iter().map(|c| c.body.as_str())),
                        _ => {}
                    }
                }
                true
            }
            "sort" => true,
            _ => false,
        };
        if hit == *neg {
            return false;
        }
    }
    if in_fields.is_empty() {
        in_fields = vec![title, body];
        in_fields.extend(comments.iter().map(|c| c.body.as_str()));
    }
    terms_hit(q, &in_fields)
}

fn repo_match(st: &HubStore, q: &Query, r: &Repo) -> bool {
    let full = r.full_name();
    let readme = st
        .git_dir(&r.owner, &r.name)
        .and_then(|d| GitRepo::open(&d).ok())
        .and_then(|g| {
            let tip = g.resolve(&r.default_branch)?;
            let (_, o) = g.lookup(&tip, "README.md").ok()??;
            g.blob(&o).ok()
        })
        .map(|b| String::from_utf8_lossy(&b).into_owned())
        .unwrap_or_default();
    let forks = st
        .db
        .repos
        .iter()
        .filter(|x| x.parent.as_deref() == Some(full.as_str()))
        .count() as f64;
    let mut in_fields: Vec<&str> = Vec::new();
    for (neg, k, v) in &q.quals {
        let v = v.as_str();
        let hit = match k.as_str() {
            "repo" => eqi(&full, v),
            k if owner_q(k) => eqi(&r.owner, v),
            "is" => match v {
                "public" => r.visibility() == "public",
                "private" => r.visibility() == "private",
                "internal" => r.visibility() == "internal",
                "archived" => r.archived,
                "template" => r.is_template,
                "mirror" => false,
                "fork" => r.parent.is_some(),
                _ => false,
            },
            "archived" => r.archived == (v == "true"),
            "fork" => match v {
                "only" => r.parent.is_some(),
                "true" => true,
                _ => r.parent.is_none(),
            },
            "mirror" => v == "false",
            "template" => r.is_template == (v == "true"),
            "topic" => r.topics.iter().any(|t| eqi(t, v)),
            "topics" => num_match(v, r.topics.len() as f64),
            "license" => r.license.as_deref().is_some_and(|l| eqi(l, v)),
            "language" => false,
            "stars" | "followers" | "good-first-issues" | "help-wanted-issues" | "size" => {
                num_match(v, 0.0)
            }
            "forks" => num_match(v, forks),
            "created" => date_match(v, &iso8601(r.created_at)),
            "pushed" => date_match(v, &iso8601(r.updated_at.max(r.created_at))),
            "in" => {
                for f in v.split(',') {
                    match f {
                        "name" => in_fields.push(&r.name),
                        "description" => in_fields.push(&r.description),
                        "readme" => in_fields.push(&readme),
                        _ => {}
                    }
                }
                true
            }
            "sort" => true,
            _ => false,
        };
        if hit == *neg {
            return false;
        }
    }
    if in_fields.is_empty() {
        in_fields = vec![&r.name, &r.description, &r.owner];
        in_fields.extend(r.topics.iter().map(String::as_str));
    }
    terms_hit(q, &in_fields)
}

fn language_of(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or("") {
        "rs" => "Rust",
        "py" => "Python",
        "go" => "Go",
        "js" | "mjs" | "cjs" => "JavaScript",
        "ts" => "TypeScript",
        "c" | "h" => "C",
        "cc" | "cpp" | "hpp" => "C++",
        "java" => "Java",
        "rb" => "Ruby",
        "sh" => "Shell",
        "md" => "Markdown",
        "toml" => "TOML",
        "yml" | "yaml" => "YAML",
        "json" => "JSON",
        _ => "",
    }
}

fn code_search(st: &HubStore, q: &Query, limit: usize) -> Vec<Value> {
    let mut out = Vec::new();
    for r in &st.db.repos {
        let full = r.full_name();
        let scoped = q.quals.iter().all(|(neg, k, v)| match k.as_str() {
            "repo" => eqi(&full, v) != *neg,
            k if owner_q(k) => eqi(&r.owner, v) != *neg,
            _ => true,
        });
        if !scoped {
            continue;
        }
        let Some(g) = st
            .git_dir(&r.owner, &r.name)
            .and_then(|d| GitRepo::open(&d).ok())
        else {
            continue;
        };
        let Some(tip) = g.resolve(&r.default_branch) else {
            continue;
        };
        let Ok(files) = g.flatten(&tip) else {
            continue;
        };
        for (path, (mode, oid)) in files {
            if mode == arks_object::TreeMode::Gitlink {
                continue;
            }
            let Ok(bytes) = g.blob(&oid) else {
                continue;
            };
            if bytes.contains(&0) {
                continue;
            }
            let text = String::from_utf8_lossy(&bytes).into_owned();
            let name = path.rsplit('/').next().unwrap_or(&path).to_string();
            let mut in_file = true;
            let mut in_path = true;
            let ok = q.quals.iter().all(|(neg, k, v)| {
                let hit = match k.as_str() {
                    "repo" | "user" | "org" | "owner" => true,
                    "extension" => path
                        .rsplit('.')
                        .next()
                        .is_some_and(|e| eqi(e, v.trim_start_matches('.'))),
                    "filename" => eqi(&name, v) || crate::forge::rules::glob(v, &name),
                    "path" => {
                        path.starts_with(v.trim_start_matches('/'))
                            || crate::forge::rules::glob(v, &path)
                    }
                    "language" => eqi(language_of(&path), v),
                    "size" => num_match(v, bytes.len() as f64 / 1024.0),
                    "in" => {
                        in_file = v.split(',').any(|f| f == "file");
                        in_path = v.split(',').any(|f| f == "path");
                        true
                    }
                    _ => false,
                };
                hit != *neg
            });
            if !ok {
                continue;
            }
            let lower = text.to_ascii_lowercase();
            let path_l = path.to_ascii_lowercase();
            let hit = q.terms.iter().all(|t| {
                (in_file && lower.contains(t.as_str())) || (in_path && path_l.contains(t.as_str()))
            });
            if !hit
                || q.terms.is_empty()
                    && !q.quals.iter().any(|(_, k, _)| {
                        matches!(k.as_str(), "filename" | "extension" | "path" | "language")
                    })
            {
                continue;
            }
            let fragments: Vec<Value> = text
                .lines()
                .filter(|l| q.terms.iter().any(|t| l.to_ascii_lowercase().contains(t.as_str())))
                .take(3)
                .map(|l| {
                    let ll = l.to_ascii_lowercase();
                    let matches: Vec<Value> = q.terms.iter().filter_map(|t| ll.find(t.as_str()).map(|i| json!({"text": &l[i..i + t.len()], "indices": [i, i + t.len()]}))).collect();
                    json!({"fragment": l, "matches": matches, "property": "content"})
                })
                .collect();
            out.push(json!({
                "name": name,
                "path": path,
                "sha": oid.to_hex(),
                "url": format!("{}/repos/{full}/contents/{path}?ref={}", st.public_base, tip.to_hex()),
                "html_url": format!("{}/{full}/blob/{}/{path}", st.public_base, tip.to_hex()),
                "repository": {"full_name": full, "name": r.name, "owner": {"login": r.owner}, "private": r.private},
                "text_matches": fragments,
            }));
            if out.len() >= limit {
                return out;
            }
        }
    }
    out
}

fn commit_search(st: &HubStore, q: &Query, limit: usize) -> Vec<Value> {
    let mut out = Vec::new();
    for r in &st.db.repos {
        let full = r.full_name();
        let Some(g) = st
            .git_dir(&r.owner, &r.name)
            .and_then(|d| GitRepo::open(&d).ok())
        else {
            continue;
        };
        let tips: Vec<arks_oid::Oid> = g.branches().into_iter().map(|(_, o)| o).collect();
        for (oid, c) in g.log(&tips, 10_000) {
            let hex = oid.to_hex();
            let ok = q.quals.iter().all(|(neg, k, v)| {
                let v = v.as_str();
                let hit = match k.as_str() {
                    "repo" => eqi(&full, v),
                    k if owner_q(k) => eqi(&r.owner, v),
                    "author" => eqi(&c.author.name, v),
                    "author-name" => c
                        .author
                        .name
                        .to_ascii_lowercase()
                        .contains(&v.to_ascii_lowercase()),
                    "author-email" => eqi(&c.author.email, v),
                    "committer" => eqi(&c.committer.name, v),
                    "committer-name" => c
                        .committer
                        .name
                        .to_ascii_lowercase()
                        .contains(&v.to_ascii_lowercase()),
                    "committer-email" => eqi(&c.committer.email, v),
                    "author-date" => date_match(v, &iso8601(c.author.timestamp.max(0) as u64)),
                    "committer-date" => {
                        date_match(v, &iso8601(c.committer.timestamp.max(0) as u64))
                    }
                    "merge" => (c.parents.len() > 1) == (v == "true"),
                    "hash" => hex.starts_with(&v.to_ascii_lowercase()),
                    "parent" => c
                        .parents
                        .iter()
                        .any(|p| p.to_hex().starts_with(&v.to_ascii_lowercase())),
                    "tree" => c.tree.to_hex().starts_with(&v.to_ascii_lowercase()),
                    "is" => match v {
                        "public" => !r.private,
                        "private" => r.private,
                        _ => false,
                    },
                    "sort" => true,
                    _ => false,
                };
                hit != *neg
            });
            if !ok || !terms_hit(q, &[&c.message]) {
                continue;
            }
            if q.terms.is_empty()
                && q.quals
                    .iter()
                    .all(|(_, k, _)| matches!(k.as_str(), "repo" | "user" | "org" | "owner" | "is"))
            {
                continue;
            }
            out.push(super::repos::commit_json(st, &full, &oid, &c));
            if out.len() >= limit {
                return out;
            }
        }
    }
    out
}

pub(crate) fn route(st: &mut HubStore, req: &Req<'_>) -> RouteResult {
    if req.method != "GET" {
        return Ok(None);
    }
    let Some(seg) = segments(req.path, "/search/") else {
        return Ok(None);
    };
    let [kind] = seg.as_slice() else {
        return Ok(None);
    };
    let q = parse_query(&req.query_param("q").unwrap_or_default());
    let limit = req
        .query_param("per_page")
        .and_then(|p| p.parse().ok())
        .unwrap_or(100usize);
    let order_asc = req.query_param("order").as_deref() == Some("asc");
    let sort = req.query_param("sort");
    let items: Vec<Value> = match *kind {
        "repositories" | "repos" => st
            .db
            .repos
            .iter()
            .filter(|r| repo_match(st, &q, r))
            .take(limit)
            .map(|r| super::repos::repo_json(st, r))
            .collect(),
        "issues" | "pulls" | "prs" => {
            let want_pr = *kind != "issues"
                || q.quals
                    .iter()
                    .any(|(n, k, v)| !n && (k == "is" || k == "type") && v == "pr");
            let want_issue = *kind == "issues"
                && !q
                    .quals
                    .iter()
                    .any(|(n, k, v)| !n && (k == "is" || k == "type") && v == "pr");
            let mut v: Vec<(u64, u64, Value)> = Vec::new();
            if want_issue {
                for i in st
                    .db
                    .issues
                    .iter()
                    .filter(|i| issue_like_match(st, &q, &Item::Issue(i)))
                {
                    let mut j = super::issues::issue_json(st, i);
                    j["is_pull_request"] = json!(false);
                    v.push((i.created_at, i.updated_at, j));
                }
            }
            if want_pr {
                for p in st
                    .db
                    .pulls
                    .iter()
                    .filter(|p| issue_like_match(st, &q, &Item::Pull(p)))
                {
                    let mut j = super::pulls::pull_json(st, p);
                    j["pull_request"] = json!({"url": j["url"], "merged_at": j["merged_at"]});
                    j["is_pull_request"] = json!(true);
                    j["repository"] = json!({"full_name": format!("{}/{}", p.owner, p.repo), "name": p.repo, "owner": {"login": p.owner}});
                    v.push((p.created_at, p.updated_at, j));
                }
            }
            match sort.as_deref() {
                Some("created") => v.sort_by_key(|x| x.0),
                Some("updated") => v.sort_by_key(|x| x.1),
                Some("comments") => v.sort_by_key(|x| x.2["comments"].as_u64().unwrap_or(0)),
                _ => v.sort_by_key(|x| x.0),
            }
            if !order_asc {
                v.reverse();
            }
            v.into_iter().take(limit).map(|x| x.2).collect()
        }
        "code" => code_search(st, &q, limit),
        "commits" => {
            let mut v = commit_search(st, &q, limit);
            if order_asc && sort.is_some() {
                v.reverse();
            }
            v
        }
        _ => {
            return Ok(Some((
                422,
                json!({"message": format!("unknown search type {kind:?}")}),
            )));
        }
    };
    Ok(Some((
        200,
        json!({"total_count": items.len(), "incomplete_results": false, "items": items}),
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qualifiers_and_ranges() {
        let q = parse_query(r#"fix "two words" repo:o/r -label:bug comments:>2"#);
        assert_eq!(q.terms, ["fix", "two words"]);
        assert_eq!(q.quals.len(), 3);
        assert!(q.quals[1].0, "negated label");
        assert!(num_match(">2", 3.0) && !num_match(">2", 2.0));
        assert!(num_match("1..3", 3.0) && num_match("*..3", 0.0));
        assert!(date_match(">=2026-01-01", "2026-03-04T00:00:00Z"));
        assert!(date_match("2026-01-01..2026-02-01", "2026-01-15T10:00:00Z"));
        assert!(!date_match("<2026", "2026-01-15T10:00:00Z"));
    }
}
