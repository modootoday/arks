//! Pull requests: metadata, reviews, requested reviewers, conversation, diff and
//! files, merge (merge / squash / rebase), auto-merge, update-branch and revert.
//!
//! With Git content the forge really merges: trees combine by the path-level
//! 3-way rule of [`super::git::merge_flat`] (a path changed on both sides is a
//! conflict → not mergeable) and the base branch moves. Without Git content a
//! merge is metadata-only, as before. Active rulesets gate merges
//! ([`super::rules::merge_blockers`]) unless the admin bypass is requested.

use super::git::{self, GitRepo};
use super::issues::{
    comments_route, ensure_labels, labels_json, milestone_arg, milestone_json, strings,
};
use super::{
    Req, Resp, RouteResult, invalid, iso8601, next_id, no_content, not_found, segments, str_field,
};
use crate::store::{HubStore, Pull, Review, now_secs};
use arks_oid::Oid;
use serde_json::{Value, json};

/// Head repository and branch of a pull (`owner:branch` names a fork's branch).
fn head_of(p: &Pull) -> (String, String, String) {
    match p.head.split_once(':') {
        Some((o, b)) => (o.to_string(), p.repo.clone(), b.to_string()),
        None => (p.owner.clone(), p.repo.clone(), p.head.clone()),
    }
}

struct Tips {
    base_dir: std::path::PathBuf,
    base: Option<Oid>,
    head: Option<Oid>,
}

fn tips(st: &HubStore, p: &Pull) -> Option<Tips> {
    let base_dir = st.git_dir(&p.owner, &p.repo)?;
    let g = GitRepo::open(&base_dir).ok()?;
    let (ho, hr, hb) = head_of(p);
    let head = if ho == p.owner {
        g.resolve(&format!("refs/heads/{hb}"))
    } else {
        let hd = st.git_dir(&ho, &hr)?;
        GitRepo::open(&hd)
            .ok()?
            .resolve(&format!("refs/heads/{hb}"))
    };
    Some(Tips {
        base: g.resolve(&format!("refs/heads/{}", p.base)),
        head,
        base_dir,
    })
}

/// Ensure a fork head's objects are present in the base repository.
fn import_head(st: &HubStore, p: &Pull, base_dir: &std::path::Path) -> Result<(), String> {
    let (ho, hr, _) = head_of(p);
    if ho != p.owner {
        if let Some(hd) = st.git_dir(&ho, &hr) {
            git::union_objects(&hd, base_dir)?;
        }
    }
    Ok(())
}

struct Stats {
    commits: usize,
    additions: usize,
    deletions: usize,
    files: Vec<Value>,
    mergeable: Option<bool>,
}

fn stats(st: &HubStore, p: &Pull) -> Option<Stats> {
    let t = tips(st, p)?;
    let (base, head) = (t.base?, t.head?);
    import_head(st, p, &t.base_dir).ok()?;
    let g = GitRepo::open(&t.base_dir).ok()?;
    let mb = g.merge_base(&base, &head).unwrap_or(base);
    let (old, new) = (g.flatten(&mb).ok()?, g.flatten(&head).ok()?);
    let cur = g.flatten(&base).ok()?;
    let mut files = Vec::new();
    let (mut adds, mut dels) = (0, 0);
    for c in git::changes(&old, &new) {
        let patch = git::unified(&g, std::slice::from_ref(&c)).unwrap_or_default();
        let a = patch
            .lines()
            .filter(|l| l.starts_with('+') && !l.starts_with("+++"))
            .count();
        let d = patch
            .lines()
            .filter(|l| l.starts_with('-') && !l.starts_with("---"))
            .count();
        adds += a;
        dels += d;
        let status = match (c.old.is_some(), c.new.is_some()) {
            (false, true) => "added",
            (true, false) => "removed",
            _ => "modified",
        };
        let body = patch
            .split_once("@@")
            .map(|(_, r)| format!("@@{r}"))
            .unwrap_or_default();
        files.push(
            json!({"filename": c.path, "status": status, "additions": a, "deletions": d,
            "changes": a + d, "patch": body, "sha": c.new.map(|n| n.1.to_hex())}),
        );
    }
    Some(Stats {
        commits: g.range(&base, &head).len(),
        additions: adds,
        deletions: dels,
        files,
        mergeable: Some(git::merge_flat(&old, &cur, &new).is_ok()),
    })
}

pub(crate) fn pull_json(st: &HubStore, p: &Pull) -> Value {
    let full = format!("{}/{}", p.owner, p.repo);
    let (ho, hr, hb) = head_of(p);
    let t = tips(st, p);
    let sha = |o: Option<Oid>| o.map(|o| o.to_hex());
    let s = if p.state == "open" {
        stats(st, p)
    } else {
        None
    };
    let reviews: Vec<&Review> = st
        .db
        .reviews
        .iter()
        .filter(|r| r.owner == p.owner && r.repo == p.repo && r.pull_number == p.number)
        .collect();
    json!({
        "id": p.number,
        "node_id": format!("PR_{}_{}", p.repo, p.number),
        "number": p.number,
        "title": p.title,
        "body": p.body,
        "state": p.state,
        "draft": p.draft,
        "locked": p.locked,
        "active_lock_reason": p.lock_reason,
        "merged": p.merged,
        "mergeable": s.as_ref().and_then(|s| s.mergeable),
        "mergeable_state": match s.as_ref().and_then(|s| s.mergeable) { Some(true) => "clean", Some(false) => "dirty", None => "unknown" },
        "head": {"ref": hb, "label": format!("{ho}:{hb}"), "sha": sha(t.as_ref().and_then(|t| t.head)),
                 "repo": {"full_name": format!("{ho}/{hr}"), "name": hr, "owner": {"login": ho}}, "user": {"login": ho}},
        "base": {"ref": p.base, "label": format!("{}:{}", p.owner, p.base), "sha": sha(t.as_ref().and_then(|t| t.base)),
                 "repo": {"full_name": full, "name": p.repo, "owner": {"login": p.owner}}},
        "user": {"login": p.user},
        "labels": labels_json(st, &p.owner, &p.repo, &p.labels),
        "assignees": p.assignees.iter().map(|a| json!({"login": a})).collect::<Vec<_>>(),
        "requested_reviewers": p.reviewers.iter().map(|a| json!({"login": a})).collect::<Vec<_>>(),
        "milestone": milestone_json(st, &p.owner, &p.repo, p.milestone),
        "projects": p.projects,
        "maintainer_can_modify": p.maintainer_can_modify,
        "auto_merge": p.auto_merge.as_ref().map(|m| json!({"merge_method": m, "enabled_by": {"login": p.user}})),
        "merge_commit_sha": p.merge_commit_sha,
        "merged_by": p.merged_by.as_ref().map(|m| json!({"login": m})),
        "commits": s.as_ref().map(|s| s.commits).unwrap_or(0),
        "additions": s.as_ref().map(|s| s.additions).unwrap_or(0),
        "deletions": s.as_ref().map(|s| s.deletions).unwrap_or(0),
        "changed_files": s.as_ref().map(|s| s.files.len()).unwrap_or(0),
        "comments": super::issues::comment_count(st, &p.owner, &p.repo, p.number, "pull"),
        "review_comments": 0,
        "reviews": reviews.iter().map(|r| review_json(r)).collect::<Vec<_>>(),
        "created_at": iso8601(p.created_at),
        "updated_at": iso8601(p.updated_at.max(p.created_at)),
        "closed_at": p.closed_at.map(iso8601),
        "merged_at": p.merged_at.map(iso8601),
        "html_url": format!("{}/{full}/pull/{}", st.public_base, p.number),
        "url": format!("{}/repos/{full}/pulls/{}", st.public_base, p.number),
        "diff_url": format!("{}/repos/{full}/pulls/{}.diff", st.public_base, p.number),
    })
}

fn review_json(r: &Review) -> Value {
    json!({"id": r.id, "body": r.body, "event": r.event, "state": r.state, "user": {"login": r.user},
        "submitted_at": iso8601(r.submitted_at)})
}

fn find(st: &HubStore, owner: &str, repo: &str, n: u64) -> Option<usize> {
    st.db
        .pulls
        .iter()
        .position(|p| p.owner == owner && p.repo == repo && p.number == n)
}

fn approvals(st: &HubStore, p: &Pull) -> usize {
    let mut latest: std::collections::BTreeMap<&str, &str> = Default::default();
    for r in st
        .db
        .reviews
        .iter()
        .filter(|r| r.owner == p.owner && r.repo == p.repo && r.pull_number == p.number)
    {
        if r.state != "COMMENTED" {
            latest.insert(&r.user, &r.state);
        }
    }
    latest.values().filter(|s| **s == "APPROVED").count()
}

/// Merge `p` with `method`; `Ok(Err(msg))` is a refused merge (HTTP 405).
fn do_merge(
    st: &mut HubStore,
    idx: usize,
    method: &str,
    title: Option<&str>,
    message: Option<&str>,
    admin: bool,
    expect: Option<&str>,
) -> Result<Result<Value, String>, String> {
    let p = st.db.pulls[idx].clone();
    if p.state != "open" || p.merged {
        return Ok(Err("Pull Request is not open".into()));
    }
    if p.draft {
        return Ok(Err("Pull Request is still a draft".into()));
    }
    let repo = st.find_repo(&p.owner, &p.repo).cloned();
    if let Some(r) = &repo {
        let allowed = match method {
            "merge" => r.allow_merge_commit,
            "squash" => r.allow_squash_merge,
            "rebase" => r.allow_rebase_merge,
            _ => return Ok(Err(format!("invalid merge method {method:?}"))),
        };
        if !allowed {
            return Ok(Err(format!(
                "{method} merges are not allowed on this repository"
            )));
        }
    }
    let t = tips(st, &p);
    let head_sha = t.as_ref().and_then(|t| t.head).map(|o| o.to_hex());
    if let (Some(want), Some(have)) = (expect, &head_sha) {
        if want != have {
            return Ok(Err(format!(
                "Head branch was modified. Review and try the merge again ({have})"
            )));
        }
    }
    if !admin {
        let sha = head_sha
            .clone()
            .unwrap_or_else(|| super::status::key(st, &p.owner, &p.repo, &p.head));
        let latest = super::status::latest(st, &p.owner, &p.repo, &sha);
        let ok = |ctx: &str| {
            latest
                .iter()
                .any(|s| s.context == ctx && s.state == "success")
        };
        let why = super::rules::merge_blockers(
            st,
            &p.owner,
            &p.repo,
            &p.base,
            approvals(st, &p),
            &ok,
            method,
        );
        if !why.is_empty() {
            return Ok(Err(format!(
                "Repository rule violations found\n{}",
                why.join("\n")
            )));
        }
    }
    let mut merge_sha = format!("local-merge-{}", p.number);
    if let Some(Tips {
        base_dir,
        base: Some(base),
        head: Some(head),
    }) = t
    {
        import_head(st, &p, &base_dir)?;
        let mut g = GitRepo::open(&base_dir)?;
        let mb = g.merge_base(&base, &head).unwrap_or(base);
        let (fb, fo, fh) = (g.flatten(&mb)?, g.flatten(&base)?, g.flatten(&head)?);
        let Ok(merged) = git::merge_flat(&fb, &fo, &fh) else {
            return Ok(Err("Pull Request is not mergeable".into()));
        };
        let when = now_secs() as i64;
        let who = st.actor.clone();
        let subject = title.map(str::to_string).unwrap_or_else(|| match method {
            "squash" => format!("{} (#{})", p.title, p.number),
            _ => format!("Merge pull request #{} from {}", p.number, p.head),
        });
        let body = message.map(str::to_string).unwrap_or_else(|| {
            if method == "merge" {
                p.title.clone()
            } else {
                String::new()
            }
        });
        let msg = if body.is_empty() {
            subject.clone()
        } else {
            format!("{subject}\n\n{body}")
        };
        let new_tip = match method {
            "merge" => {
                let tree = g.write_flat(&merged)?;
                g.write_commit(tree, vec![base, head], &msg, &who, when)?
            }
            "squash" => {
                let tree = g.write_flat(&merged)?;
                g.write_commit(tree, vec![base], &msg, &who, when)?
            }
            _ => {
                let mut cur = base;
                let mut cur_flat = fo.clone();
                for (o, c) in g.range(&mb, &head) {
                    let parent = c.parents.first().copied().unwrap_or(mb);
                    let (pf, cf) = (g.flatten(&parent)?, g.flatten(&o)?);
                    let Ok(next) = git::merge_flat(&pf, &cur_flat, &cf) else {
                        return Ok(Err("Pull Request is not mergeable (rebase conflict)".into()));
                    };
                    let tree = g.write_flat(&next)?;
                    cur = g.write_commit(
                        tree,
                        vec![cur],
                        &c.message,
                        &c.author.name,
                        c.author.timestamp,
                    )?;
                    cur_flat = next;
                }
                cur
            }
        };
        g.set_branch(&p.base, &new_tip)?;
        merge_sha = new_tip.to_hex();
        let (ho, _, hb) = head_of(&p);
        if repo.as_ref().is_some_and(|r| r.delete_branch_on_merge) && ho == p.owner && hb != p.base
        {
            let _ = g.delete_branch(&hb);
        }
    }
    let now = now_secs();
    let pm = &mut st.db.pulls[idx];
    pm.merged = true;
    pm.state = "closed".into();
    pm.merged_at = Some(now);
    pm.closed_at = Some(now);
    pm.updated_at = now;
    pm.merged_by = Some(st.actor.clone());
    pm.merge_commit_sha = Some(merge_sha.clone());
    pm.auto_merge = None;
    Ok(Ok(json!({
        "sha": merge_sha,
        "merged": true,
        "message": "Pull Request successfully merged",
        "number": p.number,
        "state": "closed",
        "merge_method": method,
    })))
}

/// Merge every open pull of `owner/repo` with auto-merge enabled whose
/// requirements now pass (called after statuses and reviews change).
pub(crate) fn try_auto_merge(st: &mut HubStore, owner: &str, repo: &str) -> Result<(), String> {
    let pending: Vec<(usize, String)> = st
        .db
        .pulls
        .iter()
        .enumerate()
        .filter(|(_, p)| p.owner == owner && p.repo == repo && p.state == "open")
        .filter_map(|(i, p)| p.auto_merge.clone().map(|m| (i, m)))
        .collect();
    for (i, m) in pending {
        let _ = do_merge(st, i, &m, None, None, false, None)?;
    }
    Ok(())
}

fn update_branch(st: &mut HubStore, idx: usize, v: &Value) -> Result<Resp, String> {
    let p = st.db.pulls[idx].clone();
    let (ho, hr, hb) = head_of(&p);
    let Some(Tips {
        base_dir,
        base: Some(base),
        head: Some(head),
    }) = tips(st, &p)
    else {
        return Ok((
            422,
            json!({"message": "pull request has no Git content on this forge"}),
        ));
    };
    if let Some(want) = str_field(v, "expected_head_sha") {
        if want != head.to_hex() {
            return Ok((
                422,
                json!({"message": "expected head sha didn't match current head ref."}),
            ));
        }
    }
    let mut g = GitRepo::open(&base_dir)?;
    if g.is_ancestor(&base, &head) {
        return Ok((
            422,
            json!({"message": "There are no new commits on the base branch."}),
        ));
    }
    let mb = g.merge_base(&base, &head).unwrap_or(base);
    let who = st.actor.clone();
    let when = now_secs() as i64;
    let rebase = str_field(v, "update_method") == Some("rebase");
    let new_head = if rebase {
        let mut cur = base;
        let mut cur_flat = g.flatten(&base)?;
        for (o, c) in g.range(&mb, &head) {
            let parent = c.parents.first().copied().unwrap_or(mb);
            let Ok(next) = git::merge_flat(&g.flatten(&parent)?, &cur_flat, &g.flatten(&o)?) else {
                return Ok((
                    422,
                    json!({"message": "merge conflict between base and head"}),
                ));
            };
            let tree = g.write_flat(&next)?;
            cur = g.write_commit(
                tree,
                vec![cur],
                &c.message,
                &c.author.name,
                c.author.timestamp,
            )?;
            cur_flat = next;
        }
        cur
    } else {
        let Ok(merged) = git::merge_flat(&g.flatten(&mb)?, &g.flatten(&head)?, &g.flatten(&base)?)
        else {
            return Ok((
                422,
                json!({"message": "merge conflict between base and head"}),
            ));
        };
        let tree = g.write_flat(&merged)?;
        g.write_commit(
            tree,
            vec![head, base],
            &format!("Merge branch '{}' into {hb}", p.base),
            &who,
            when,
        )?
    };
    if ho == p.owner {
        g.set_branch(&hb, &new_head)?;
    } else if let Some(hd) = st.git_dir(&ho, &hr) {
        git::union_objects(&base_dir, &hd)?;
        GitRepo::open(&hd)?.set_branch(&hb, &new_head)?;
    }
    Ok((
        202,
        json!({"message": "Updating pull request branch.", "sha": new_head.to_hex()}),
    ))
}

fn new_pull(
    st: &mut HubStore,
    owner: &str,
    repo: &str,
    v: &Value,
) -> Result<Result<Pull, String>, String> {
    let (Some(title), Some(head)) = (str_field(v, "title"), str_field(v, "head")) else {
        return Ok(Err("title and head are required".into()));
    };
    let base = str_field(v, "base").map(str::to_string).unwrap_or_else(|| {
        st.find_repo(owner, repo)
            .map(|r| r.default_branch.clone())
            .unwrap_or_else(|| "main".into())
    });
    if head == base {
        return Ok(Err(format!("head and base are both {base:?}")));
    }
    if st.db.pulls.iter().any(|p| {
        p.owner == owner && p.repo == repo && p.state == "open" && p.head == head && p.base == base
    }) {
        return Ok(Err(format!(
            "a pull request already exists for {owner}:{head}"
        )));
    }
    let ms = milestone_arg(st, owner, repo, v)?;
    let labels = ensure_labels(st, owner, repo, &strings(v, "labels").unwrap_or_default());
    let now = now_secs();
    Ok(Ok(Pull {
        number: next_id(&mut st.db.next_pr),
        owner: owner.into(),
        repo: repo.into(),
        title: title.into(),
        body: v.get("body").and_then(Value::as_str).unwrap_or("").into(),
        state: "open".into(),
        head: head.into(),
        base,
        user: st.actor.clone(),
        merged: false,
        draft: v.get("draft").and_then(Value::as_bool).unwrap_or(false),
        locked: false,
        labels,
        assignees: strings(v, "assignees").unwrap_or_default(),
        reviewers: strings(v, "reviewers").unwrap_or_default(),
        milestone: ms.flatten(),
        projects: strings(v, "projects").unwrap_or_default(),
        maintainer_can_modify: v
            .get("maintainer_can_modify")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        lock_reason: None,
        auto_merge: None,
        merge_commit_sha: None,
        merged_by: None,
        created_at: now,
        updated_at: now,
        closed_at: None,
        merged_at: None,
    }))
}

fn revert(st: &mut HubStore, idx: usize, v: &Value) -> Result<Resp, String> {
    let p = st.db.pulls[idx].clone();
    if !p.merged {
        return Ok(invalid("only a merged pull request can be reverted"));
    }
    let (Some(dir), Some(sha)) = (st.git_dir(&p.owner, &p.repo), p.merge_commit_sha.clone()) else {
        return Ok(invalid("pull request has no Git content to revert"));
    };
    let mut g = GitRepo::open(&dir)?;
    let (Ok(m), Some(base_tip)) = (
        Oid::from_hex(&sha),
        g.resolve(&format!("refs/heads/{}", p.base)),
    ) else {
        return Ok(invalid("merge commit not found"));
    };
    let mc = g.commit(&m)?;
    let Some(before) = mc.parents.first().copied() else {
        return Ok(invalid("merge commit has no parent"));
    };
    let Ok(tree) = git::merge_flat(
        &g.flatten(&m)?,
        &g.flatten(&base_tip)?,
        &g.flatten(&before)?,
    ) else {
        return Ok(invalid(
            "revert conflicts with later changes on the base branch",
        ));
    };
    let branch = format!("revert-{}-{}", p.number, head_of(&p).2);
    let tree = g.write_flat(&tree)?;
    let title = str_field(v, "title")
        .map(str::to_string)
        .unwrap_or_else(|| format!("Revert \"{}\"", p.title));
    let msg = format!("{title}\n\nThis reverts commit {sha}.");
    let c = g.write_commit(
        tree,
        vec![base_tip],
        &msg,
        &st.actor.clone(),
        now_secs() as i64,
    )?;
    g.set_branch(&branch, &c)?;
    let body = str_field(v, "body")
        .map(str::to_string)
        .unwrap_or_else(|| format!("Reverts {}/{}#{}", p.owner, p.repo, p.number));
    let req = json!({"title": title, "body": body, "head": branch, "base": p.base, "draft": v.get("draft").and_then(Value::as_bool).unwrap_or(false)});
    match new_pull(st, &p.owner, &p.repo, &req)? {
        Ok(np) => {
            st.db.pulls.push(np.clone());
            st.save()?;
            Ok((201, pull_json(st, &np)))
        }
        Err(e) => Ok(invalid(e)),
    }
}

fn diff_text(st: &HubStore, p: &Pull, patch: bool) -> Result<Option<String>, String> {
    let Some(Tips {
        base_dir,
        base: Some(base),
        head: Some(head),
    }) = tips(st, p)
    else {
        return Ok(None);
    };
    import_head(st, p, &base_dir)?;
    let g = GitRepo::open(&base_dir)?;
    let mb = g.merge_base(&base, &head).unwrap_or(base);
    if !patch {
        return Ok(Some(git::unified(
            &g,
            &git::changes(&g.flatten(&mb)?, &g.flatten(&head)?),
        )?));
    }
    let mut out = String::new();
    for (o, c) in g.range(&mb, &head) {
        let parent = c.parents.first().copied().unwrap_or(mb);
        let subject = c.message.lines().next().unwrap_or("");
        out.push_str(&format!(
            "From {} Mon Sep 17 00:00:00 2001\nFrom: {} <{}>\nDate: {}\nSubject: [PATCH] {subject}\n\n{}\n---\n",
            o.to_hex(), c.author.name, c.author.email, iso8601(c.author.timestamp.max(0) as u64),
            c.message.lines().skip(2).collect::<Vec<_>>().join("\n")
        ));
        out.push_str(&git::unified(
            &g,
            &git::changes(&g.flatten(&parent)?, &g.flatten(&o)?),
        )?);
        out.push('\n');
    }
    Ok(Some(out))
}

pub(crate) fn route(st: &mut HubStore, req: &Req<'_>) -> RouteResult {
    let Some(seg) = segments(req.path, "/repos/") else {
        return Ok(None);
    };
    if seg.len() < 3 || seg[2] != "pulls" {
        return Ok(None);
    }
    let (owner, repo) = (seg[0].to_string(), seg[1].to_string());
    if st.find_repo(&owner, &repo).is_none() {
        return Ok(Some(not_found()));
    }
    let v = req.json();
    let rest = &seg[3..];
    match (req.method, rest) {
        ("GET", []) => {
            let state = req.query_param("state").unwrap_or_else(|| "open".into());
            let head = req.query_param("head");
            let base = req.query_param("base");
            let limit = req
                .query_param("per_page")
                .and_then(|p| p.parse().ok())
                .unwrap_or(usize::MAX);
            let mut items: Vec<&Pull> = st
                .db
                .pulls
                .iter()
                .filter(|p| p.owner == owner && p.repo == repo)
                .filter(|p| state == "all" || p.state == state)
                .filter(|p| {
                    head.as_deref().is_none_or(|h| {
                        p.head == h
                            || head_of(p).2 == h
                            || format!("{}:{}", head_of(p).0, head_of(p).2) == h
                    })
                })
                .filter(|p| base.as_deref().is_none_or(|b| p.base == b))
                .collect();
            items.sort_by_key(|x| std::cmp::Reverse(x.number));
            let out: Vec<Value> = items
                .into_iter()
                .take(limit)
                .map(|p| pull_json(st, p))
                .collect();
            Ok(Some((200, Value::Array(out))))
        }
        ("POST", []) => match new_pull(st, &owner, &repo, &v)? {
            Ok(p) => {
                st.db.pulls.push(p.clone());
                st.save()?;
                Ok(Some((201, pull_json(st, &p))))
            }
            Err(e) => Ok(Some(invalid(e))),
        },
        ("PATCH" | "DELETE", ["comments", id]) => {
            let path = format!("/repos/{owner}/{repo}/issues/comments/{id}");
            let r2 = Req {
                method: req.method,
                path: &path,
                query: req.query,
                body: req.body,
                accept: req.accept,
                content_type: req.content_type,
            };
            super::issues::route(st, &r2)
        }
        (_, [n, tail @ ..]) => {
            let (num, suffix) = match n.split_once('.') {
                Some((a, b)) => (a, Some(b)),
                None => (*n, None),
            };
            let Some(idx) = num
                .parse::<u64>()
                .ok()
                .and_then(|n| find(st, &owner, &repo, n))
            else {
                return Ok(Some(not_found()));
            };
            let p = st.db.pulls[idx].clone();
            let want_diff = suffix.or(if req.accept.contains("diff") {
                Some("diff")
            } else if req.accept.contains("patch") {
                Some("patch")
            } else {
                None
            });
            match (req.method, tail) {
                ("GET", []) if want_diff.is_some() => {
                    Ok(Some(match diff_text(st, &p, want_diff == Some("patch"))? {
                        Some(t) => (200, json!({"__raw_text": t})),
                        None => (
                            406,
                            json!({"message": "pull request has no Git content on this forge"}),
                        ),
                    }))
                }
                ("GET", []) => Ok(Some((200, pull_json(st, &p)))),
                ("PATCH", []) => {
                    let ms = match milestone_arg(st, &owner, &repo, &v) {
                        Ok(m) => m,
                        Err(e) => return Ok(Some(invalid(e))),
                    };
                    let labels =
                        strings(&v, "labels").map(|l| ensure_labels(st, &owner, &repo, &l));
                    let pm = &mut st.db.pulls[idx];
                    if let Some(t) = v.get("title").and_then(Value::as_str) {
                        pm.title = t.into();
                    }
                    if let Some(b) = v.get("body").and_then(Value::as_str) {
                        pm.body = b.into();
                    }
                    if let Some(s) = v.get("state").and_then(Value::as_str) {
                        if pm.merged && s == "open" {
                            return Ok(Some(invalid("a merged pull request cannot be reopened")));
                        }
                        if s != pm.state {
                            pm.closed_at = (s == "closed").then(now_secs);
                        }
                        pm.state = s.into();
                    }
                    if let Some(b) = str_field(&v, "base") {
                        pm.base = b.into();
                    }
                    if let Some(d) = v.get("draft").and_then(Value::as_bool) {
                        pm.draft = d;
                    }
                    if let Some(l) = v.get("locked").and_then(Value::as_bool) {
                        pm.locked = l;
                    }
                    if let Some(l) = labels {
                        pm.labels = l;
                    }
                    if let Some(a) = strings(&v, "assignees") {
                        pm.assignees = a;
                    }
                    if let Some(r) = strings(&v, "reviewers") {
                        pm.reviewers = r;
                    }
                    if let Some(p) = strings(&v, "projects") {
                        pm.projects = p;
                    }
                    if let Some(m) = ms {
                        pm.milestone = m;
                    }
                    if let Some(m) = v.get("maintainer_can_modify").and_then(Value::as_bool) {
                        pm.maintainer_can_modify = m;
                    }
                    pm.updated_at = now_secs();
                    let out = pm.clone();
                    st.save()?;
                    Ok(Some((200, pull_json(st, &out))))
                }
                (m @ ("PUT" | "DELETE"), ["lock"]) => {
                    let pm = &mut st.db.pulls[idx];
                    pm.locked = m == "PUT";
                    pm.lock_reason = if pm.locked {
                        str_field(&v, "lock_reason").map(str::to_string)
                    } else {
                        None
                    };
                    st.save()?;
                    Ok(Some(no_content()))
                }
                (_, ["comments"]) => {
                    comments_route(st, req, &owner, &repo, p.number, "pull").map(Some)
                }
                ("GET", ["files"]) => Ok(Some((
                    200,
                    Value::Array(stats(st, &p).map(|s| s.files).unwrap_or_default()),
                ))),
                ("GET", ["commits"]) => {
                    let Some(Tips {
                        base_dir,
                        base: Some(base),
                        head: Some(head),
                    }) = tips(st, &p)
                    else {
                        return Ok(Some((200, json!([]))));
                    };
                    import_head(st, &p, &base_dir)?;
                    let g = GitRepo::open(&base_dir)?;
                    let full = format!("{owner}/{repo}");
                    let items: Vec<Value> = g
                        .range(&base, &head)
                        .iter()
                        .map(|(o, c)| super::repos::commit_json(st, &full, o, c))
                        .collect();
                    Ok(Some((200, Value::Array(items))))
                }
                ("GET", ["reviews"]) => {
                    let items: Vec<Value> = st
                        .db
                        .reviews
                        .iter()
                        .filter(|r| r.owner == owner && r.repo == repo && r.pull_number == p.number)
                        .map(review_json)
                        .collect();
                    Ok(Some((200, Value::Array(items))))
                }
                ("POST", ["reviews"]) => {
                    let event = str_field(&v, "event")
                        .unwrap_or("COMMENT")
                        .to_ascii_uppercase();
                    let state = match event.as_str() {
                        "APPROVE" => "APPROVED",
                        "REQUEST_CHANGES" => "CHANGES_REQUESTED",
                        "COMMENT" => "COMMENTED",
                        _ => return Ok(Some(invalid(format!("invalid review event {event:?}")))),
                    };
                    let body = v.get("body").and_then(Value::as_str).unwrap_or("");
                    if event != "APPROVE" && body.trim().is_empty() {
                        return Ok(Some(invalid("body is required for this review event")));
                    }
                    let r = Review {
                        id: next_id(&mut st.db.next_review),
                        owner: owner.clone(),
                        repo: repo.clone(),
                        pull_number: p.number,
                        body: body.into(),
                        event,
                        user: st.actor.clone(),
                        state: state.into(),
                        submitted_at: now_secs(),
                    };
                    st.db.reviews.push(r.clone());
                    let actor = st.actor.clone();
                    st.db.pulls[idx].reviewers.retain(|x| *x != actor);
                    try_auto_merge(st, &owner, &repo)?;
                    st.save()?;
                    Ok(Some((200, review_json(&r))))
                }
                (m @ ("POST" | "DELETE"), ["requested_reviewers"]) => {
                    let who = strings(&v, "reviewers").unwrap_or_default();
                    let pm = &mut st.db.pulls[idx];
                    if m == "POST" {
                        for w in who {
                            if !pm.reviewers.contains(&w) {
                                pm.reviewers.push(w);
                            }
                        }
                    } else {
                        pm.reviewers.retain(|r| !who.contains(r));
                    }
                    let out = pm.clone();
                    st.save()?;
                    Ok(Some((
                        if m == "POST" { 201 } else { 200 },
                        pull_json(st, &out),
                    )))
                }
                ("PUT", ["merge"]) => {
                    let method = str_field(&v, "merge_method").unwrap_or("merge").to_string();
                    let admin = v.get("admin").and_then(Value::as_bool).unwrap_or(false);
                    match do_merge(
                        st,
                        idx,
                        &method,
                        str_field(&v, "commit_title"),
                        str_field(&v, "commit_message"),
                        admin,
                        str_field(&v, "sha"),
                    )? {
                        Ok(body) => {
                            st.save()?;
                            Ok(Some((200, body)))
                        }
                        Err(why) => Ok(Some((405, json!({"message": why})))),
                    }
                }
                ("PUT", ["auto-merge"]) => {
                    let method = str_field(&v, "merge_method").unwrap_or("merge").to_string();
                    st.db.pulls[idx].auto_merge = Some(method);
                    try_auto_merge(st, &owner, &repo)?;
                    st.save()?;
                    Ok(Some((200, pull_json(st, &st.db.pulls[idx].clone()))))
                }
                ("DELETE", ["auto-merge"]) => {
                    st.db.pulls[idx].auto_merge = None;
                    st.save()?;
                    Ok(Some((200, pull_json(st, &st.db.pulls[idx].clone()))))
                }
                ("PUT", ["update-branch"]) => update_branch(st, idx, &v).map(Some),
                ("POST", ["revert"]) => revert(st, idx, &v).map(Some),
                _ => Ok(None),
            }
        }
        _ => Ok(None),
    }
}
