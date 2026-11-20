//! Forge repositories: metadata, settings, topics, forks, sync, branches, refs,
//! commits and the contents API (arks-owned; GitHub-shaped REST paths).
//!
//! Content routes read the repository's Git directory in-process
//! ([`super::git`]); a repository without one answers as empty (`409`, like an
//! empty GitHub repository) instead of inventing content.

use super::git::{self, GitRepo};
use super::{Req, Resp, RouteResult, invalid, iso8601, no_content, not_found, segments, str_field};
use crate::store::{HubStore, Repo, now_secs};
use base64::Engine as _;
use serde_json::{Value, json};

pub(crate) fn user_json(st: &HubStore, login: &str) -> Value {
    let is_org = !st.db.token_logins.values().any(|l| l == login)
        && login != st.db.user_login
        && st.db.repos.iter().any(|r| r.owner == login);
    json!({
        "login": login,
        "id": login.bytes().fold(0u64, |a, b| a.wrapping_mul(31).wrapping_add(u64::from(b))) % 1_000_000,
        "type": if is_org { "Organization" } else { "User" },
        "html_url": format!("{}/{login}", st.public_base),
    })
}

fn pushed_at(st: &HubStore, r: &Repo) -> Option<i64> {
    let g = GitRepo::open(&st.git_dir(&r.owner, &r.name)?).ok()?;
    g.branches()
        .iter()
        .filter_map(|(_, o)| g.commit(o).ok())
        .map(|c| c.committer.timestamp)
        .max()
}

pub(crate) fn repo_json(st: &HubStore, r: &Repo) -> Value {
    let full = r.full_name();
    let git_dir = st.git_dir(&r.owner, &r.name);
    let empty = git_dir
        .as_ref()
        .and_then(|d| GitRepo::open(d).ok())
        .is_none_or(|g| g.branches().is_empty());
    let clone_url = match (&st.git_url, &git_dir) {
        (Some(base), _) => Value::String(format!("{}/{full}.git", base.trim_end_matches('/'))),
        (None, Some(d)) => Value::String(d.display().to_string()),
        (None, None) => Value::Null,
    };
    let parent = r.parent.as_ref().map(|p| {
        let (o, n) = p.split_once('/').unwrap_or((p, ""));
        json!({"full_name": p, "name": n, "owner": {"login": o}})
    });
    let license = r.license.as_ref().map(|k| {
        let t = super::templates::license(k);
        json!({
            "key": k,
            "name": t.map(|t| t.name).unwrap_or(k),
            "spdx_id": t.map(|t| t.spdx).unwrap_or(k),
        })
    });
    let open_issues = st
        .db
        .issues
        .iter()
        .filter(|i| i.owner == r.owner && i.repo == r.name && i.state == "open")
        .count();
    let forks = st
        .db
        .repos
        .iter()
        .filter(|x| x.parent.as_deref() == Some(full.as_str()))
        .count();
    json!({
        "id": r.id,
        "node_id": format!("R_{}", r.id),
        "name": r.name,
        "full_name": full,
        "owner": user_json(st, &r.owner),
        "private": r.private || r.internal,
        "visibility": r.visibility(),
        "description": r.description,
        "homepage": r.homepage,
        "html_url": format!("{}/{full}", st.public_base),
        "url": format!("{}/repos/{full}", st.public_base),
        "clone_url": clone_url,
        "ssh_url": Value::Null,
        "archived": r.archived,
        "fork": r.parent.is_some(),
        "parent": parent,
        "is_template": r.is_template,
        "default_branch": r.default_branch,
        "topics": r.topics,
        "has_issues": r.has_issues,
        "has_wiki": r.has_wiki,
        "has_projects": r.has_projects,
        "has_discussions": r.has_discussions,
        "allow_merge_commit": r.allow_merge_commit,
        "allow_squash_merge": r.allow_squash_merge,
        "allow_rebase_merge": r.allow_rebase_merge,
        "allow_auto_merge": r.allow_auto_merge,
        "allow_update_branch": r.allow_update_branch,
        "allow_forking": r.allow_forking,
        "delete_branch_on_merge": r.delete_branch_on_merge,
        "squash_merge_commit_message": r.squash_merge_commit_message,
        "license": license,
        "forks_count": forks,
        "stargazers_count": 0,
        "watchers_count": 0,
        "open_issues_count": open_issues,
        "size": 0,
        "language": Value::Null,
        "empty": empty,
        "created_at": iso8601(r.created_at),
        "updated_at": iso8601(r.updated_at.max(r.created_at)),
        "pushed_at": pushed_at(st, r).map(|t| iso8601(t as u64)),
    })
}

fn next_repo_id(st: &mut HubStore) -> u64 {
    if st.db.next_repo == 0 {
        st.db.next_repo = st.db.repos.iter().map(|r| r.id).max().unwrap_or(0) + 1;
    }
    let id = st.db.next_repo;
    st.db.next_repo += 1;
    id
}

pub(crate) fn new_repo(st: &mut HubStore, owner: &str, name: &str) -> Repo {
    let now = now_secs();
    Repo {
        id: next_repo_id(st),
        owner: owner.into(),
        name: name.into(),
        description: String::new(),
        private: false,
        archived: false,
        homepage: String::new(),
        topics: Vec::new(),
        default_branch: "main".into(),
        internal: false,
        has_issues: true,
        has_wiki: true,
        has_projects: true,
        has_discussions: false,
        is_template: false,
        delete_branch_on_merge: false,
        allow_merge_commit: true,
        allow_squash_merge: true,
        allow_rebase_merge: true,
        allow_auto_merge: false,
        allow_update_branch: false,
        allow_forking: true,
        squash_merge_commit_message: String::new(),
        parent: None,
        license: None,
        created_at: now,
        updated_at: now,
    }
}

fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 100
        && name != "."
        && name != ".."
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
}

fn apply_settings(r: &mut Repo, v: &Value) -> Result<(), String> {
    if let Some(d) = v.get("description").and_then(Value::as_str) {
        r.description = d.into();
    }
    if let Some(h) = v.get("homepage").and_then(Value::as_str) {
        r.homepage = h.into();
    }
    if let Some(p) = v.get("private").and_then(Value::as_bool) {
        r.private = p;
        r.internal = false;
    }
    match v.get("visibility").and_then(Value::as_str) {
        None => {}
        Some("public") => (r.private, r.internal) = (false, false),
        Some("private") => (r.private, r.internal) = (true, false),
        Some("internal") => (r.private, r.internal) = (true, true),
        Some(o) => return Err(format!("invalid visibility {o:?}")),
    }
    let flags: [(&str, &mut bool); 13] = [
        ("archived", &mut r.archived),
        ("has_issues", &mut r.has_issues),
        ("has_wiki", &mut r.has_wiki),
        ("has_projects", &mut r.has_projects),
        ("has_discussions", &mut r.has_discussions),
        ("is_template", &mut r.is_template),
        ("delete_branch_on_merge", &mut r.delete_branch_on_merge),
        ("allow_merge_commit", &mut r.allow_merge_commit),
        ("allow_squash_merge", &mut r.allow_squash_merge),
        ("allow_rebase_merge", &mut r.allow_rebase_merge),
        ("allow_auto_merge", &mut r.allow_auto_merge),
        ("allow_update_branch", &mut r.allow_update_branch),
        ("allow_forking", &mut r.allow_forking),
    ];
    for (k, slot) in flags {
        if let Some(b) = v.get(k).and_then(Value::as_bool) {
            *slot = b;
        }
    }
    if let Some(m) = v.get("squash_merge_commit_message").and_then(Value::as_str) {
        r.squash_merge_commit_message = m.into();
    }
    if let Some(t) = v.get("topics").and_then(Value::as_array) {
        r.topics = t
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect();
    }
    r.updated_at = now_secs();
    Ok(())
}

/// Initial commit with README / `.gitignore` / LICENSE (repo create bootstrap).
fn bootstrap(st: &HubStore, r: &Repo, v: &Value) -> Result<(), String> {
    let readme = v.get("auto_init").and_then(Value::as_bool).unwrap_or(false);
    let gi = str_field(v, "gitignore_template");
    let lic = str_field(v, "license_template");
    let dir = git::init_bare(&st.git_root, &r.owner, &r.name, &r.default_branch)?;
    if !readme && gi.is_none() && lic.is_none() {
        return Ok(());
    }
    let mut files = Vec::new();
    if readme {
        let mut text = format!("# {}\n", r.name);
        if !r.description.is_empty() {
            text.push_str(&format!("\n{}\n", r.description));
        }
        files.push(("README.md".to_string(), text.into_bytes()));
    }
    if let Some(name) = gi {
        let t = super::templates::gitignore(name)
            .ok_or(format!("unknown gitignore template {name:?}"))?;
        files.push((".gitignore".into(), t.body.as_bytes().to_vec()));
    }
    if let Some(key) = lic {
        let t = super::templates::license(key).ok_or(format!("unknown license {key:?}"))?;
        files.push((
            "LICENSE".into(),
            super::templates::license_text(t, &r.owner).into_bytes(),
        ));
    }
    let mut g = GitRepo::open(&dir)?;
    g.commit_files(
        &r.default_branch,
        &files,
        "Initial commit",
        &st.actor,
        now_secs() as i64,
    )?;
    Ok(())
}

fn create(st: &mut HubStore, owner: &str, v: &Value) -> Result<Resp, String> {
    let Some(name) = str_field(v, "name") else {
        return Ok(invalid("name is required"));
    };
    if !valid_name(name) {
        return Ok(invalid(format!("invalid repository name {name:?}")));
    }
    if st.find_repo(owner, name).is_some() {
        return Ok(invalid("name already exists on this account"));
    }
    let mut r = new_repo(st, owner, name);
    if let Err(e) = apply_settings(&mut r, v) {
        return Ok(invalid(e));
    }
    r.license = str_field(v, "license_template").map(str::to_string);
    if let Err(e) = bootstrap(st, &r, v) {
        return Ok(invalid(e));
    }
    st.db.repos.push(r.clone());
    st.save()?;
    Ok((201, repo_json(st, &r)))
}

fn rename(
    st: &mut HubStore,
    owner: &str,
    name: &str,
    new_name: &str,
) -> Result<Option<Repo>, String> {
    if new_name == name {
        return Ok(st.find_repo(owner, name).cloned());
    }
    if !valid_name(new_name) || st.find_repo(owner, new_name).is_some() {
        return Err(format!("cannot rename to {new_name:?}"));
    }
    let Some(r) = st.find_repo_mut(owner, name) else {
        return Ok(None);
    };
    r.name = new_name.into();
    let out = r.clone();
    let db = &mut st.db;
    db.issues
        .iter_mut()
        .filter(|i| i.owner == owner && i.repo == name)
        .for_each(|i| i.repo = new_name.into());
    db.pulls
        .iter_mut()
        .filter(|p| p.owner == owner && p.repo == name)
        .for_each(|p| p.repo = new_name.into());
    db.labels
        .iter_mut()
        .filter(|l| l.owner == owner && l.repo == name)
        .for_each(|l| l.repo = new_name.into());
    db.releases
        .iter_mut()
        .filter(|l| l.owner == owner && l.repo == name)
        .for_each(|l| l.repo = new_name.into());
    db.comments
        .iter_mut()
        .filter(|l| l.owner == owner && l.repo == name)
        .for_each(|l| l.repo = new_name.into());
    db.milestones
        .iter_mut()
        .filter(|l| l.owner == owner && l.repo == name)
        .for_each(|l| l.repo = new_name.into());
    db.statuses
        .iter_mut()
        .filter(|l| l.owner == owner && l.repo == name)
        .for_each(|l| l.repo = new_name.into());
    if let Some(d) = st.git_dir(owner, name) {
        let to = st.git_root.join(owner).join(format!("{new_name}.git"));
        if let Some(p) = to.parent() {
            let _ = std::fs::create_dir_all(p);
        }
        std::fs::rename(&d, &to).map_err(|e| e.to_string())?;
    }
    Ok(Some(out))
}

fn delete(st: &mut HubStore, owner: &str, name: &str) -> bool {
    let before = st.db.repos.len();
    st.db
        .repos
        .retain(|r| !(r.owner == owner && r.name == name));
    if st.db.repos.len() == before {
        return false;
    }
    let db = &mut st.db;
    db.issues.retain(|i| !(i.owner == owner && i.repo == name));
    db.pulls.retain(|p| !(p.owner == owner && p.repo == name));
    db.labels.retain(|l| !(l.owner == owner && l.repo == name));
    db.releases
        .retain(|r| !(r.owner == owner && r.repo == name));
    db.comments
        .retain(|r| !(r.owner == owner && r.repo == name));
    db.statuses
        .retain(|r| !(r.owner == owner && r.repo == name));
    if let Some(d) = st.git_dir(owner, name) {
        if d.starts_with(&st.git_root) {
            let _ = std::fs::remove_dir_all(d);
        }
    }
    true
}

/// Fork `owner/name` into `to_owner/to_name`; the Git directory is copied as-is.
fn fork(st: &mut HubStore, owner: &str, name: &str, v: &Value) -> Result<Resp, String> {
    let Some(src) = st.find_repo(owner, name).cloned() else {
        return Ok(not_found());
    };
    if !src.allow_forking {
        return Ok(invalid("forking is disabled for this repository"));
    }
    let to_owner = str_field(v, "organization")
        .unwrap_or(&st.actor)
        .to_string();
    let to_name = str_field(v, "name").unwrap_or(name).to_string();
    if let Some(existing) = st.find_repo(&to_owner, &to_name) {
        if existing.parent.as_deref() == Some(src.full_name().as_str()) {
            return Ok((202, repo_json(st, &existing.clone())));
        }
        return Ok(invalid(format!("{to_owner}/{to_name} already exists")));
    }
    let mut r = new_repo(st, &to_owner, &to_name);
    r.description = src.description.clone();
    r.default_branch = src.default_branch.clone();
    r.parent = Some(src.full_name());
    r.private = src.private;
    r.license = src.license.clone();
    if let Some(d) = st.git_dir(owner, name) {
        let to = st.git_root.join(&to_owner).join(format!("{to_name}.git"));
        git::copy_dir(&d, &to)?;
        if v.get("default_branch_only")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            let mut g = GitRepo::open(&to)?;
            for (b, _) in g.branches() {
                if b != r.default_branch {
                    let _ = g.delete_branch(&b);
                }
            }
        }
    }
    st.db.repos.push(r.clone());
    st.save()?;
    Ok((202, repo_json(st, &r)))
}

/// Fast-forward a fork branch to its parent's branch (`repo sync` remote form).
fn merge_upstream(st: &mut HubStore, owner: &str, name: &str, v: &Value) -> Result<Resp, String> {
    let Some(r) = st.find_repo(owner, name).cloned() else {
        return Ok(not_found());
    };
    let branch = str_field(v, "branch")
        .unwrap_or(&r.default_branch)
        .to_string();
    let force = v.get("force").and_then(Value::as_bool).unwrap_or(false);
    let source = str_field(v, "source")
        .map(str::to_string)
        .or(r.parent.clone());
    let Some(source) = source else {
        return Ok(invalid(
            "repository is not a fork; pass a source repository",
        ));
    };
    let (so, sn) = source.split_once('/').unwrap_or((&source, ""));
    let (Some(src_dir), Some(dst_dir)) = (st.git_dir(so, sn), st.git_dir(owner, name)) else {
        return Ok((
            409,
            json!({"message": "repository has no Git content on this forge"}),
        ));
    };
    let src = GitRepo::open(&src_dir)?;
    let Some(tip) = src.resolve(&branch) else {
        return Ok(invalid(format!("branch {branch:?} not found in {source}")));
    };
    git::union_objects(&src_dir, &dst_dir)?;
    let mut dst = GitRepo::open(&dst_dir)?;
    let cur = dst.resolve(&branch);
    let kind = match cur {
        Some(c) if c == tip => "none",
        Some(c) if dst.is_ancestor(&c, &tip) => "fast-forward",
        Some(_) if !force => {
            return Ok((
                409,
                json!({"message": format!(
                "can't sync because there are diverging changes; use --force to reset {branch}")}),
            ));
        }
        Some(_) => "hard-reset",
        None => "create",
    };
    if kind != "none" {
        dst.set_branch(&branch, &tip)?;
    }
    Ok((
        200,
        json!({
            "message": format!("Successfully fetched and fast-forwarded from upstream {source}:{branch}."),
            "merge_type": kind,
            "base_branch": format!("{source}:{branch}"),
            "sha": tip.to_hex(),
        }),
    ))
}

/// `/repos/{o}/{r}/generate` from a template repository.
fn generate(st: &mut HubStore, owner: &str, name: &str, v: &Value) -> Result<Resp, String> {
    let Some(src) = st.find_repo(owner, name).cloned() else {
        return Ok(not_found());
    };
    if !src.is_template {
        return Ok(invalid(format!(
            "{} is not a template repository",
            src.full_name()
        )));
    }
    let to_owner = str_field(v, "owner").unwrap_or(&st.actor).to_string();
    let Some(to_name) = str_field(v, "name").map(str::to_string) else {
        return Ok(invalid("name is required"));
    };
    if st.find_repo(&to_owner, &to_name).is_some() {
        return Ok(invalid("name already exists on this account"));
    }
    let mut r = new_repo(st, &to_owner, &to_name);
    apply_settings(&mut r, v).map_err(|e| e.to_string())?;
    r.default_branch = src.default_branch.clone();
    if let Some(d) = st.git_dir(owner, name) {
        let to = git::init_bare(&st.git_root, &to_owner, &to_name, &src.default_branch)?;
        let s = GitRepo::open(&d)?;
        git::union_objects(&d, &to)?;
        let mut g = GitRepo::open(&to)?;
        let all = v
            .get("include_all_branches")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        for (b, tip) in s.branches() {
            if !all && b != src.default_branch {
                continue;
            }
            // A generated repository starts with fresh history: one commit per branch.
            let tree = s.commit(&tip)?.tree;
            let c = g.write_commit(
                tree,
                Vec::new(),
                "Initial commit",
                &st.actor,
                now_secs() as i64,
            )?;
            g.set_branch(&b, &c)?;
        }
    }
    st.db.repos.push(r.clone());
    st.save()?;
    Ok((201, repo_json(st, &r)))
}

fn branch_json(st: &HubStore, r: &Repo, g: &GitRepo, name: &str, oid: &arks_oid::Oid) -> Value {
    let protected = !super::rules::applicable(st, &r.owner, &r.name, name).is_empty();
    json!({
        "name": name,
        "commit": {"sha": oid.to_hex(), "url": format!("{}/repos/{}/commits/{}", st.public_base, r.full_name(), oid.to_hex())},
        "protected": protected,
        "default": g.default_branch() == name,
    })
}

pub(crate) fn commit_json(
    st: &HubStore,
    full: &str,
    oid: &arks_oid::Oid,
    c: &arks_object::Commit,
) -> Value {
    let sig = |s: &arks_object::Signature| json!({"name": s.name, "email": s.email, "date": iso8601(s.timestamp.max(0) as u64)});
    json!({
        "sha": oid.to_hex(),
        "node_id": format!("C_{}", &oid.to_hex()[..12]),
        "html_url": format!("{}/{full}/commit/{}", st.public_base, oid.to_hex()),
        "url": format!("{}/repos/{full}/commits/{}", st.public_base, oid.to_hex()),
        "commit": {
            "message": c.message.trim_end(),
            "author": sig(&c.author),
            "committer": sig(&c.committer),
            "tree": {"sha": c.tree.to_hex()},
            "comment_count": 0,
        },
        "author": {"login": c.author.name},
        "committer": {"login": c.committer.name},
        "parents": c.parents.iter().map(|p| json!({"sha": p.to_hex()})).collect::<Vec<_>>(),
        "repository": {"full_name": full, "name": full.split('/').nth(1).unwrap_or("")},
    })
}

fn contents(st: &HubStore, r: &Repo, path: &str, rev: Option<&str>) -> Result<Resp, String> {
    let Some(dir) = st.git_dir(&r.owner, &r.name) else {
        return Ok((404, json!({"message": "This repository is empty."})));
    };
    let g = GitRepo::open(&dir)?;
    let rev = rev.unwrap_or(&r.default_branch);
    let Some(commit) = g.resolve(rev) else {
        return Ok((
            404,
            json!({"message": format!("No commit found for the ref {rev}")}),
        ));
    };
    let path = path.trim_matches('/');
    let Some((mode, oid)) = g.lookup(&commit, path)? else {
        return Ok(not_found());
    };
    let full = r.full_name();
    let entry = |name: &str, p: &str, mode: arks_object::TreeMode, oid: &arks_oid::Oid| -> Value {
        let kind = match mode {
            arks_object::TreeMode::Tree => "dir",
            arks_object::TreeMode::Symlink => "symlink",
            arks_object::TreeMode::Gitlink => "submodule",
            _ => "file",
        };
        let size = if kind == "file" {
            g.blob(oid).map(|b| b.len()).unwrap_or(0)
        } else {
            0
        };
        json!({
            "type": kind,
            "name": name,
            "path": p,
            "sha": oid.to_hex(),
            "mode": mode.as_str(),
            "size": size,
            "url": format!("{}/repos/{full}/contents/{p}?ref={rev}", st.public_base),
            "html_url": format!("{}/{full}/blob/{rev}/{p}", st.public_base),
            "git_url": format!("{}/repos/{full}/git/blobs/{}", st.public_base, oid.to_hex()),
            "download_url": if kind == "file" { Value::String(format!("{}/repos/{full}/contents/{p}?ref={rev}&raw=1", st.public_base)) } else { Value::Null },
        })
    };
    if mode == arks_object::TreeMode::Tree {
        let t = g.tree(&oid)?;
        let items: Vec<Value> = t
            .entries
            .iter()
            .map(|e| {
                let p = if path.is_empty() {
                    e.name.clone()
                } else {
                    format!("{path}/{}", e.name)
                };
                entry(&e.name, &p, e.mode, &e.oid)
            })
            .collect();
        return Ok((200, Value::Array(items)));
    }
    let name = path.rsplit('/').next().unwrap_or(path);
    let mut v = entry(name, path, mode, &oid);
    if mode != arks_object::TreeMode::Gitlink {
        let bytes = g.blob(&oid)?;
        v["encoding"] = json!("base64");
        v["content"] = json!(base64::engine::general_purpose::STANDARD.encode(&bytes));
        v["size"] = json!(bytes.len());
    }
    Ok((200, v))
}

pub(crate) fn route(st: &mut HubStore, req: &Req<'_>) -> RouteResult {
    let m = req.method;
    if req.path == "/user/repos"
        || req.path.starts_with("/users/")
        || req.path.starts_with("/orgs/")
    {
        let owner_filter = if req.path == "/user/repos" {
            None
        } else {
            let seg = segments(req.path, "/").unwrap_or_default();
            if seg.len() != 3 || seg[2] != "repos" {
                return Ok(None);
            }
            Some(seg[1].to_string())
        };
        if m == "POST" {
            let owner = owner_filter.clone().unwrap_or_else(|| st.actor.clone());
            return create(st, &owner, &req.json()).map(Some);
        }
        if m == "GET" {
            let vis = req.query_param("visibility");
            let items: Vec<Value> = st
                .db
                .repos
                .iter()
                .filter(|r| match &owner_filter {
                    Some(o) => &r.owner == o,
                    None => true,
                })
                .filter(|r| {
                    vis.as_deref()
                        .is_none_or(|v| v == "all" || r.visibility() == v)
                })
                .map(|r| repo_json(st, r))
                .collect();
            return Ok(Some((200, Value::Array(items))));
        }
        return Ok(None);
    }
    if req.path == "/orgs" && m == "GET" {
        let mut owners: Vec<String> = st.db.repos.iter().map(|r| r.owner.clone()).collect();
        owners.sort();
        owners.dedup();
        let items: Vec<Value> = owners
            .iter()
            .filter(|o| **o != st.actor)
            .map(|o| json!({"login": o, "type": "Organization"}))
            .collect();
        return Ok(Some((200, Value::Array(items))));
    }
    let Some(seg) = segments(req.path, "/repos/") else {
        return Ok(None);
    };
    if seg.len() < 2 {
        return Ok(None);
    }
    let (owner, name) = (seg[0].to_string(), seg[1].to_string());
    let rest: Vec<&str> = seg[2..].to_vec();
    // Routes below are owned only when the repository exists (else 404 here).
    let Some(r) = st.find_repo(&owner, &name).cloned() else {
        return Ok(match rest.first().copied() {
            None
            | Some(
                "topics" | "forks" | "merge-upstream" | "generate" | "branches" | "git" | "commits"
                | "contents" | "readme" | "tarball" | "zipball",
            ) => Some(not_found()),
            _ => None,
        });
    };
    let v = req.json();
    match (m, rest.as_slice()) {
        ("GET", []) => Ok(Some((200, repo_json(st, &r)))),
        ("PATCH", []) => {
            let mut cur = r.clone();
            if let Some(nn) = str_field(&v, "name") {
                match rename(st, &owner, &name, nn) {
                    Ok(Some(x)) => cur = x,
                    Ok(None) => return Ok(Some(not_found())),
                    Err(e) => return Ok(Some(invalid(e))),
                }
            }
            if let Some(b) = str_field(&v, "default_branch") {
                if let Some(d) = st.git_dir(&cur.owner, &cur.name) {
                    let g = GitRepo::open(&d)?;
                    if g.resolve(b).is_none() && !g.branches().is_empty() {
                        return Ok(Some(invalid(format!("branch {b:?} does not exist"))));
                    }
                    std::fs::write(d.join("HEAD"), format!("ref: refs/heads/{b}\n"))
                        .map_err(|e| e.to_string())?;
                }
                cur.default_branch = b.into();
            }
            if let Err(e) = apply_settings(&mut cur, &v) {
                return Ok(Some(invalid(e)));
            }
            if let Some(slot) = st.find_repo_mut(&cur.owner, &cur.name) {
                *slot = cur.clone();
            }
            st.save()?;
            Ok(Some((200, repo_json(st, &cur))))
        }
        ("DELETE", []) => {
            delete(st, &owner, &name);
            st.save()?;
            Ok(Some(no_content()))
        }
        ("GET", ["topics"]) => Ok(Some((200, json!({"names": r.topics})))),
        ("PUT", ["topics"]) => {
            let names: Vec<String> = v
                .get("names")
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(Value::as_str)
                        .map(|s| s.to_ascii_lowercase())
                        .collect()
                })
                .unwrap_or_default();
            if let Some(slot) = st.find_repo_mut(&owner, &name) {
                slot.topics = names.clone();
            }
            st.save()?;
            Ok(Some((200, json!({"names": names}))))
        }
        ("POST", ["forks"]) => fork(st, &owner, &name, &v).map(Some),
        ("GET", ["forks"]) => {
            let full = r.full_name();
            let items: Vec<Value> = st
                .db
                .repos
                .iter()
                .filter(|x| x.parent.as_deref() == Some(full.as_str()))
                .map(|x| repo_json(st, x))
                .collect();
            Ok(Some((200, Value::Array(items))))
        }
        ("POST", ["merge-upstream"]) => merge_upstream(st, &owner, &name, &v).map(Some),
        ("POST", ["generate"]) => generate(st, &owner, &name, &v).map(Some),
        ("GET", ["branches"]) => {
            let Some(d) = st.git_dir(&owner, &name) else {
                return Ok(Some((200, json!([]))));
            };
            let g = GitRepo::open(&d)?;
            let items: Vec<Value> = g
                .branches()
                .iter()
                .map(|(b, o)| branch_json(st, &r, &g, b, o))
                .collect();
            Ok(Some((200, Value::Array(items))))
        }
        ("GET", ["branches", b @ ..]) => {
            let b = b.join("/");
            let Some(d) = st.git_dir(&owner, &name) else {
                return Ok(Some(not_found()));
            };
            let g = GitRepo::open(&d)?;
            Ok(Some(
                match g.branches().into_iter().find(|(n, _)| *n == b) {
                    Some((n, o)) => (200, branch_json(st, &r, &g, &n, &o)),
                    None => (404, json!({"message": "Branch not found"})),
                },
            ))
        }
        ("POST", ["git", "refs"]) => {
            let (Some(rf), Some(sha)) = (str_field(&v, "ref"), str_field(&v, "sha")) else {
                return Ok(Some(invalid("ref and sha are required")));
            };
            let Some(d) = st.git_dir(&owner, &name) else {
                return Ok(Some((409, json!({"message": "Git Repository is empty."}))));
            };
            let mut g = GitRepo::open(&d)?;
            let Some(oid) = g.resolve(sha) else {
                return Ok(Some(invalid(format!("object {sha} does not exist"))));
            };
            if g.resolve(rf).is_some() {
                return Ok(Some(invalid("Reference already exists")));
            }
            g.set_branch(rf, &oid)?;
            Ok(Some((
                201,
                json!({"ref": rf, "object": {"sha": oid.to_hex(), "type": "commit"}}),
            )))
        }
        ("DELETE", ["git", "refs", kind @ ("heads" | "tags"), b @ ..]) => {
            let b = b.join("/");
            let Some(d) = st.git_dir(&owner, &name) else {
                return Ok(Some(not_found()));
            };
            let mut g = GitRepo::open(&d)?;
            let res = if *kind == "heads" {
                g.delete_branch(&b)
            } else {
                g.delete_tag(&b)
            };
            Ok(Some(match res {
                Ok(()) => no_content(),
                Err(_) => (422, json!({"message": "Reference does not exist"})),
            }))
        }
        ("GET", ["commits"]) => {
            let Some(d) = st.git_dir(&owner, &name) else {
                return Ok(Some((409, json!({"message": "Git Repository is empty."}))));
            };
            let g = GitRepo::open(&d)?;
            let rev = req
                .query_param("sha")
                .unwrap_or_else(|| r.default_branch.clone());
            let limit = req
                .query_param("per_page")
                .and_then(|p| p.parse().ok())
                .unwrap_or(30usize);
            let Some(tip) = g.resolve(&rev) else {
                return Ok(Some(not_found()));
            };
            let full = r.full_name();
            let items: Vec<Value> = g
                .log(&[tip], limit)
                .iter()
                .map(|(o, c)| commit_json(st, &full, o, c))
                .collect();
            Ok(Some((200, Value::Array(items))))
        }
        ("GET", ["commits", rf]) if !rf.is_empty() => {
            let Some(d) = st.git_dir(&owner, &name) else {
                return Ok(Some(not_found()));
            };
            let g = GitRepo::open(&d)?;
            let Some(oid) = g.resolve(rf) else {
                return Ok(Some((
                    422,
                    json!({"message": format!("No commit found for SHA: {rf}")}),
                )));
            };
            let c = g.commit(&oid)?;
            Ok(Some((200, commit_json(st, &r.full_name(), &oid, &c))))
        }
        ("GET", ["contents", p @ ..]) => {
            let path = p.join("/");
            let rev = req.query_param("ref");
            let resp = contents(st, &r, &path, rev.as_deref())?;
            if req.query_param("raw").is_some() && resp.0 == 200 {
                if let Some(c) = resp.1.get("content").and_then(Value::as_str) {
                    let bytes = base64::engine::general_purpose::STANDARD
                        .decode(c)
                        .unwrap_or_default();
                    return Ok(Some((200, json!({"raw": String::from_utf8_lossy(&bytes)}))));
                }
            }
            Ok(Some(resp))
        }
        ("GET", ["readme"]) => {
            for cand in ["README.md", "README", "readme.md", "README.txt"] {
                let resp = contents(st, &r, cand, req.query_param("ref").as_deref())?;
                if resp.0 == 200 {
                    return Ok(Some(resp));
                }
            }
            Ok(Some(not_found()))
        }
        ("GET", [kind @ ("tarball" | "zipball"), rf @ ..]) => {
            let rev = if rf.is_empty() {
                r.default_branch.clone()
            } else {
                rf.join("/")
            };
            let Some(d) = st.git_dir(&owner, &name) else {
                return Ok(Some(not_found()));
            };
            let g = GitRepo::open(&d)?;
            let Some(tip) = g.resolve(&rev) else {
                return Ok(Some(not_found()));
            };
            let prefix = format!(
                "{}-{}",
                r.name,
                rev.trim_start_matches('v').replace('/', "-")
            );
            let files = g.flatten(&tip)?;
            let mut entries = Vec::new();
            for (p, (mode, oid)) in &files {
                if *mode == arks_object::TreeMode::Gitlink {
                    continue;
                }
                entries.push((
                    format!("{prefix}/{p}"),
                    *mode == arks_object::TreeMode::Executable,
                    g.blob(oid)?,
                ));
            }
            let bytes = if *kind == "tarball" {
                super::archive::tar_gz(&entries)?
            } else {
                super::archive::zip(&entries)
            };
            Ok(Some((
                200,
                json!({"__bytes_b64": base64::engine::general_purpose::STANDARD.encode(bytes),
                "__content_type": if *kind == "tarball" { "application/x-gzip" } else { "application/zip" }}),
            )))
        }
        _ => Ok(None),
    }
}
