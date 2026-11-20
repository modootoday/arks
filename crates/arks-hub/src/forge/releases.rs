//! Releases and release assets (arks-owned; GitHub-shaped REST).
//!
//! With Git content a release tag is created at its target when absent (a
//! lightweight `refs/tags/<tag>`), `verify_tag` refuses a missing tag, generated
//! notes list the commits and merged pull requests since the previous release,
//! and `notes_from_tag` takes an annotated tag's message. Asset bytes live under
//! `<data>/assets/<id>.bin`.

use super::git::GitRepo;
use super::{
    Req, Resp, RouteResult, invalid, iso8601, next_id, no_content, not_found, segments, str_field,
};
use crate::store::{HubStore, Release, ReleaseAsset, now_secs};
use base64::Engine as _;
use serde_json::{Value, json};
use std::fs;

fn latest_id(st: &HubStore, owner: &str, repo: &str) -> Option<u64> {
    let mine: Vec<&Release> = st
        .db
        .releases
        .iter()
        .filter(|r| r.owner == owner && r.repo == repo && !r.draft)
        .collect();
    if let Some(r) = mine.iter().rev().find(|r| r.make_latest == Some(true)) {
        return Some(r.id);
    }
    mine.iter()
        .filter(|r| !r.prerelease && r.make_latest != Some(false))
        .max_by_key(|r| (r.published_at.unwrap_or(r.created_at), r.id))
        .map(|r| r.id)
}

pub(crate) fn asset_json(st: &HubStore, a: &ReleaseAsset) -> Value {
    json!({
        "id": a.id,
        "name": a.name,
        "label": a.label,
        "size": a.size,
        "content_type": a.content_type,
        "state": "uploaded",
        "download_count": 0,
        "created_at": iso8601(a.created_at),
        "url": format!("{}/repos/{}/{}/releases/assets/{}", st.public_base, a.owner, a.repo, a.id),
        "browser_download_url": format!("{}/repos/{}/{}/releases/assets/{}", st.public_base, a.owner, a.repo, a.id),
    })
}

pub(crate) fn release_json(st: &HubStore, r: &Release) -> Value {
    let full = format!("{}/{}", r.owner, r.repo);
    let assets: Vec<Value> = st
        .db
        .assets
        .iter()
        .filter(|a| a.release_id == r.id)
        .map(|a| asset_json(st, a))
        .collect();
    json!({
        "id": r.id,
        "node_id": format!("RE_{}", r.id),
        "tag_name": r.tag_name,
        "name": r.name,
        "body": r.body,
        "draft": r.draft,
        "prerelease": r.prerelease,
        "immutable": false,
        "target_commitish": r.target_commitish,
        "author": {"login": if r.author.is_empty() { st.db.user_login.clone() } else { r.author.clone() }},
        "created_at": iso8601(r.created_at),
        "published_at": r.published_at.map(iso8601),
        "is_latest": latest_id(st, &r.owner, &r.repo) == Some(r.id),
        "discussion_category": r.discussion_category,
        "html_url": format!("{}/{full}/releases/tag/{}", st.public_base, r.tag_name),
        "url": format!("{}/repos/{full}/releases/{}", st.public_base, r.id),
        "upload_url": format!("{}/repos/{full}/releases/{}/assets{{?name,label}}", st.public_base, r.id),
        "tarball_url": format!("{}/repos/{full}/tarball/{}", st.public_base, r.tag_name),
        "zipball_url": format!("{}/repos/{full}/zipball/{}", st.public_base, r.tag_name),
        "assets": assets,
    })
}

fn find(st: &HubStore, owner: &str, repo: &str, key: &str) -> Option<usize> {
    st.db.releases.iter().position(|r| {
        r.owner == owner && r.repo == repo && (r.id.to_string() == key || r.tag_name == key)
    })
}

/// "What's Changed" notes since `previous` (or the newest older release).
fn generate_notes(
    st: &HubStore,
    owner: &str,
    repo: &str,
    tag: &str,
    target: &str,
    previous: Option<&str>,
) -> (String, String) {
    let full = format!("{owner}/{repo}");
    let prev = previous.map(str::to_string).or_else(|| {
        st.db
            .releases
            .iter()
            .filter(|r| r.owner == owner && r.repo == repo && r.tag_name != tag && !r.draft)
            .max_by_key(|r| r.created_at)
            .map(|r| r.tag_name.clone())
    });
    let mut lines = vec!["## What's Changed".to_string()];
    let since = prev
        .as_ref()
        .and_then(|p| {
            st.db
                .releases
                .iter()
                .find(|r| r.owner == owner && r.repo == repo && r.tag_name == *p)
        })
        .map(|r| r.created_at)
        .unwrap_or(0);
    let mut prs = 0;
    for p in st.db.pulls.iter().filter(|p| {
        p.owner == owner && p.repo == repo && p.merged && p.merged_at.unwrap_or(0) >= since
    }) {
        lines.push(format!(
            "* {} by @{} in {}/{full}/pull/{}",
            p.title, p.user, st.public_base, p.number
        ));
        prs += 1;
    }
    if prs == 0 {
        if let Some(g) = st.git_dir(owner, repo).and_then(|d| GitRepo::open(&d).ok()) {
            if let Some(tip) = g.resolve(target).or_else(|| g.resolve(tag)) {
                let commits = match prev.as_deref().and_then(|p| g.resolve(p)) {
                    Some(base) => g.range(&base, &tip),
                    None => g.log(&[tip], 200).into_iter().rev().collect(),
                };
                for (o, c) in commits {
                    lines.push(format!(
                        "* {} ({}) by {}",
                        c.subject(),
                        &o.to_hex()[..7],
                        c.author.name
                    ));
                }
            }
        }
    }
    if lines.len() == 1 {
        lines.push("* No changes recorded on this forge".into());
    }
    match prev {
        Some(p) => lines.push(format!(
            "\n**Full Changelog**: {}/{full}/compare/{p}...{tag}",
            st.public_base
        )),
        None => lines.push(format!(
            "\n**Full Changelog**: {}/{full}/commits/{tag}",
            st.public_base
        )),
    }
    (tag.to_string(), lines.join("\n"))
}

/// Tag annotation (or commit message) for `notes_from_tag`.
fn tag_message(st: &HubStore, owner: &str, repo: &str, tag: &str) -> Option<String> {
    let dir = st.git_dir(owner, repo)?;
    let raw = fs::read_to_string(dir.join("refs/tags").join(tag)).ok();
    let g = GitRepo::open(&dir).ok()?;
    let oid = raw
        .and_then(|s| arks_oid::Oid::from_hex(s.trim()).ok())
        .or_else(|| g.resolve(&format!("refs/tags/{tag}")))?;
    let obj = g.blob(&oid).ok()?;
    let text = String::from_utf8_lossy(&obj).into_owned();
    if text.starts_with("object ") {
        return text.split_once("\n\n").map(|(_, m)| m.trim().to_string());
    }
    g.commit(&oid).ok().map(|c| c.message.trim().to_string())
}

/// Create the tag at `target` when it is missing (Git content only).
fn ensure_tag(
    st: &HubStore,
    owner: &str,
    repo: &str,
    tag: &str,
    target: &str,
    verify: bool,
) -> Result<Option<String>, String> {
    let Some(dir) = st.git_dir(owner, repo) else {
        return Ok(verify.then(|| format!("tag {tag} doesn't exist in the repository")));
    };
    let mut g = GitRepo::open(&dir)?;
    if g.resolve(&format!("refs/tags/{tag}")).is_some() {
        return Ok(None);
    }
    if verify {
        return Ok(Some(format!("tag {tag} doesn't exist in the repository")));
    }
    if let Some(oid) = g.resolve(target) {
        g.set_branch(&format!("refs/tags/{tag}"), &oid)?;
    }
    Ok(None)
}

fn apply(st: &HubStore, r: &mut Release, v: &Value) {
    if let Some(t) = str_field(v, "tag_name") {
        r.tag_name = t.into();
    }
    if let Some(n) = v.get("name").and_then(Value::as_str) {
        r.name = n.into();
    }
    if let Some(b) = v.get("body").and_then(Value::as_str) {
        r.body = b.into();
    }
    if let Some(d) = v.get("draft").and_then(Value::as_bool) {
        if r.draft && !d {
            r.published_at = Some(now_secs());
        }
        r.draft = d;
    }
    if let Some(p) = v.get("prerelease").and_then(Value::as_bool) {
        r.prerelease = p;
    }
    if let Some(t) = str_field(v, "target_commitish") {
        r.target_commitish = t.into();
    }
    match v.get("make_latest").and_then(|x| {
        x.as_str()
            .map(str::to_string)
            .or_else(|| x.as_bool().map(|b| b.to_string()))
    }) {
        Some(s) if s == "true" => r.make_latest = Some(true),
        Some(s) if s == "false" => r.make_latest = Some(false),
        Some(_) => r.make_latest = None,
        None => {}
    }
    if let Some(c) = str_field(v, "discussion_category_name") {
        r.discussion_category = Some(c.into());
    }
    let _ = st;
}

fn create(st: &mut HubStore, owner: &str, repo: &str, v: &Value) -> Result<Resp, String> {
    let Some(tag) = str_field(v, "tag_name").map(str::to_string) else {
        return Ok(invalid("tag_name is required"));
    };
    if find(st, owner, repo, &tag).is_some() {
        return Ok((
            422,
            json!({"message": "Validation Failed", "errors": [{"resource": "Release", "code": "already_exists", "field": "tag_name"}]}),
        ));
    }
    let default = st
        .find_repo(owner, repo)
        .map(|r| r.default_branch.clone())
        .unwrap_or_else(|| "main".into());
    let target = str_field(v, "target_commitish")
        .unwrap_or(&default)
        .to_string();
    let verify = v
        .get("verify_tag")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if let Some(e) = ensure_tag(st, owner, repo, &tag, &target, verify)? {
        return Ok(invalid(e));
    }
    let now = now_secs();
    let mut r = Release {
        id: next_id(&mut st.db.next_release),
        owner: owner.into(),
        repo: repo.into(),
        tag_name: tag.clone(),
        name: String::new(),
        body: String::new(),
        draft: false,
        prerelease: false,
        target_commitish: target.clone(),
        make_latest: None,
        discussion_category: None,
        author: st.actor.clone(),
        created_at: now,
        published_at: None,
    };
    apply(st, &mut r, v);
    if !r.draft {
        r.published_at = Some(now);
    }
    let want_notes = v
        .get("generate_release_notes")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if want_notes {
        let (name, notes) = generate_notes(st, owner, repo, &tag, &target, None);
        if r.name.is_empty() {
            r.name = name;
        }
        r.body = if r.body.is_empty() {
            notes
        } else {
            format!("{}\n\n{notes}", r.body)
        };
    }
    if v.get("notes_from_tag")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        r.body = tag_message(st, owner, repo, &tag).unwrap_or_default();
    }
    if r.name.is_empty() {
        r.name = tag.clone();
    }
    if let Some(cat) = r.discussion_category.clone() {
        if !r.draft {
            super::discussion::create_for_release(st, owner, repo, &r.name, &r.body, &cat);
        }
    }
    st.db.releases.push(r.clone());
    st.save()?;
    Ok((201, release_json(st, &r)))
}

pub(crate) fn route(st: &mut HubStore, req: &Req<'_>) -> RouteResult {
    let Some(seg) = segments(req.path, "/repos/") else {
        return Ok(None);
    };
    if seg.len() < 3 || seg[2] != "releases" {
        return Ok(None);
    }
    let (owner, repo) = (seg[0].to_string(), seg[1].to_string());
    if st.find_repo(&owner, &repo).is_none() {
        return Ok(Some(not_found()));
    }
    let v = req.json();
    match (req.method, &seg[3..]) {
        ("GET", []) => {
            let mut items: Vec<&Release> = st
                .db
                .releases
                .iter()
                .filter(|r| r.owner == owner && r.repo == repo)
                .collect();
            items.sort_by(|a, b| b.created_at.cmp(&a.created_at).then(b.id.cmp(&a.id)));
            Ok(Some((
                200,
                Value::Array(items.into_iter().map(|r| release_json(st, r)).collect()),
            )))
        }
        ("POST", []) => create(st, &owner, &repo, &v).map(Some),
        ("POST", ["generate-notes"]) => {
            let Some(tag) = str_field(&v, "tag_name") else {
                return Ok(Some(invalid("tag_name is required")));
            };
            let default = st
                .find_repo(&owner, &repo)
                .map(|r| r.default_branch.clone())
                .unwrap_or_default();
            let target = str_field(&v, "target_commitish").unwrap_or(&default);
            let (name, body) = generate_notes(
                st,
                &owner,
                &repo,
                tag,
                target,
                str_field(&v, "previous_tag_name"),
            );
            Ok(Some((200, json!({"name": name, "body": body}))))
        }
        ("GET", ["latest"]) => Ok(Some(
            match latest_id(st, &owner, &repo)
                .and_then(|id| st.db.releases.iter().find(|r| r.id == id))
            {
                Some(r) => (200, release_json(st, r)),
                None => not_found(),
            },
        )),
        ("GET", ["tags", tag @ ..]) => {
            let tag = tag.join("/");
            Ok(Some(
                match st
                    .db
                    .releases
                    .iter()
                    .find(|r| r.owner == owner && r.repo == repo && r.tag_name == tag)
                {
                    Some(r) => (200, release_json(st, r)),
                    None => not_found(),
                },
            ))
        }
        (m, ["assets", id]) => {
            let Some(idx) = id.parse::<u64>().ok().and_then(|id| {
                st.db
                    .assets
                    .iter()
                    .position(|a| a.owner == owner && a.repo == repo && a.id == id)
            }) else {
                return Ok(Some(not_found()));
            };
            let a = st.db.assets[idx].clone();
            match m {
                "GET" if req.accept.contains("octet-stream") => {
                    let bytes = st.read_asset_bytes(a.id).unwrap_or_default();
                    Ok(Some((
                        200,
                        json!({"__bytes_b64": base64::engine::general_purpose::STANDARD.encode(bytes), "__content_type": a.content_type}),
                    )))
                }
                "GET" => Ok(Some((200, asset_json(st, &a)))),
                "PATCH" => {
                    if let Some(n) = str_field(&v, "name") {
                        st.db.assets[idx].name = n.into();
                    }
                    if let Some(l) = v.get("label").and_then(Value::as_str) {
                        st.db.assets[idx].label = l.into();
                    }
                    let out = asset_json(st, &st.db.assets[idx].clone());
                    st.save()?;
                    Ok(Some((200, out)))
                }
                "DELETE" => {
                    st.db.assets.remove(idx);
                    let _ = fs::remove_file(st.assets_dir().join(format!("{}.bin", a.id)));
                    st.save()?;
                    Ok(Some(no_content()))
                }
                _ => Ok(None),
            }
        }
        (m, [key, tail @ ..]) => {
            let Some(idx) = find(st, &owner, &repo, key) else {
                return Ok(Some(not_found()));
            };
            match (m, tail) {
                ("GET", []) => Ok(Some((200, release_json(st, &st.db.releases[idx].clone())))),
                ("PATCH", []) => {
                    let mut r = st.db.releases[idx].clone();
                    let was_draft = r.draft;
                    apply(st, &mut r, &v);
                    if let Some(t) = str_field(&v, "tag_name") {
                        let target = r.target_commitish.clone();
                        if let Some(e) = ensure_tag(
                            st,
                            &owner,
                            &repo,
                            t,
                            &target,
                            v.get("verify_tag")
                                .and_then(Value::as_bool)
                                .unwrap_or(false),
                        )? {
                            return Ok(Some(invalid(e)));
                        }
                    }
                    if was_draft && !r.draft {
                        if let Some(cat) = r.discussion_category.clone() {
                            super::discussion::create_for_release(
                                st, &owner, &repo, &r.name, &r.body, &cat,
                            );
                        }
                    }
                    st.db.releases[idx] = r.clone();
                    st.save()?;
                    Ok(Some((200, release_json(st, &r))))
                }
                ("DELETE", []) => {
                    let id = st.db.releases.remove(idx).id;
                    let gone: Vec<u64> = st
                        .db
                        .assets
                        .iter()
                        .filter(|a| a.release_id == id)
                        .map(|a| a.id)
                        .collect();
                    st.db.assets.retain(|a| a.release_id != id);
                    for a in gone {
                        let _ = fs::remove_file(st.assets_dir().join(format!("{a}.bin")));
                    }
                    st.save()?;
                    Ok(Some(no_content()))
                }
                ("GET", ["assets"]) => {
                    let id = st.db.releases[idx].id;
                    let items: Vec<Value> = st
                        .db
                        .assets
                        .iter()
                        .filter(|a| a.release_id == id)
                        .map(|a| asset_json(st, a))
                        .collect();
                    Ok(Some((200, Value::Array(items))))
                }
                ("POST", ["assets"]) => {
                    let id = st.db.releases[idx].id;
                    let name = req
                        .query_param("name")
                        .unwrap_or_else(|| "asset.bin".into());
                    if st
                        .db
                        .assets
                        .iter()
                        .any(|a| a.release_id == id && a.name == name)
                    {
                        return Ok(Some((
                            422,
                            json!({"message": "Validation Failed", "errors": [{"resource": "ReleaseAsset", "code": "already_exists", "field": "name"}]}),
                        )));
                    }
                    let aid = next_id(&mut st.db.next_asset);
                    let dir = st.assets_dir();
                    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
                    fs::write(dir.join(format!("{aid}.bin")), req.body)
                        .map_err(|e| e.to_string())?;
                    let a = ReleaseAsset {
                        id: aid,
                        release_id: id,
                        owner: owner.clone(),
                        repo: repo.clone(),
                        name,
                        content_type: if req.content_type.is_empty() {
                            "application/octet-stream".into()
                        } else {
                            req.content_type.to_string()
                        },
                        size: req.body.len() as u64,
                        label: req.query_param("label").unwrap_or_default(),
                        created_at: now_secs(),
                    };
                    st.db.assets.push(a.clone());
                    st.save()?;
                    Ok(Some((201, asset_json(st, &a))))
                }
                _ => Ok(None),
            }
        }
        _ => Ok(None),
    }
}
