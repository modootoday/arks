//! Local forge git storage: hosted bare repositories under `<forge-data>/git/`.
//!
//! Read-only object/tree access for the contents API, ref resolution for the
//! linked-branch GraphQL operations, and branch creation. In-process arks
//! crates only — never spawns git/gh.
//!
//! Layout (local forge convention, not a GitHub contract):
//! `<forge-data>/git/<owner>/<name>.git` is the bare repository backing the
//! forge record `owner/name`. `clone_url` is `$ARKS_HUB_CLONE_BASE/<owner>/<name>.git`
//! when that variable is set on the forge host, else the bare repository path
//! (local path transport).
//!
//! Contents shape: <https://docs.github.com/en/rest/repos/contents#get-repository-content>.

use arks_object::{Commit, ObjectKind, Tree, TreeMode};
use arks_oid::Oid;
use arks_refs::RefTarget;
use arks_repo::Repository;
use base64::Engine as _;
use serde_json::{Map, Value, json};
use std::path::{Path, PathBuf};

/// Bare repository backing `owner/name`, when the forge hosts one.
pub fn hosted_git_dir(root: &Path, owner: &str, name: &str) -> Option<PathBuf> {
    if !safe_segment(owner) || !safe_segment(name) {
        return None;
    }
    let p = root.join("git").join(owner).join(format!("{name}.git"));
    p.join("HEAD").is_file().then_some(p)
}

fn safe_segment(s: &str) -> bool {
    !s.is_empty() && s != "." && s != ".." && !s.contains(['/', '\\'])
}

/// Extra repository fields (`clone_url`, `default_branch`) for a hosted repo.
pub fn repo_git_fields(root: &Path, owner: &str, name: &str) -> Map<String, Value> {
    let mut m = Map::new();
    let Some(dir) = hosted_git_dir(root, owner, name) else {
        return m;
    };
    let clone_url = match std::env::var("ARKS_HUB_CLONE_BASE") {
        Ok(base) if !base.trim().is_empty() => {
            format!("{}/{owner}/{name}.git", base.trim_end_matches('/'))
        }
        _ => dir.display().to_string(),
    };
    m.insert("clone_url".into(), json!(clone_url));
    if let Ok(repo) = Repository::open(&dir)
        && let Ok(RefTarget::Symbolic(t)) = repo.head_target()
    {
        m.insert(
            "default_branch".into(),
            json!(t.trim_start_matches("refs/heads/")),
        );
    }
    m.insert("fork".into(), json!(false));
    m
}

/// Resolve a branch, tag, full ref or hex object id to a commit id.
pub fn resolve_commitish(repo: &Repository, rev: Option<&str>) -> Option<Oid> {
    let Some(rev) = rev.filter(|r| !r.is_empty()) else {
        return repo.head_oid().ok();
    };
    for cand in [
        rev.to_string(),
        format!("refs/heads/{rev}"),
        format!("refs/tags/{rev}"),
    ] {
        if let Ok(oid) = repo.refs().resolve(&cand) {
            return peel_to_commit(repo, oid);
        }
    }
    if rev.len() == 40
        && let Ok(oid) = Oid::from_hex(rev)
    {
        return peel_to_commit(repo, oid);
    }
    None
}

fn peel_to_commit(repo: &Repository, mut oid: Oid) -> Option<Oid> {
    for _ in 0..8 {
        let obj = repo.get_object(&oid).ok()?;
        match obj.kind {
            ObjectKind::Commit => return Some(oid),
            ObjectKind::Tag => {
                let text = String::from_utf8_lossy(&obj.payload);
                let target = text.lines().next()?.strip_prefix("object ")?;
                oid = Oid::from_hex(target.trim()).ok()?;
            }
            _ => return None,
        }
    }
    None
}

fn type_of(mode: TreeMode) -> &'static str {
    match mode {
        TreeMode::Tree => "dir",
        TreeMode::Symlink => "symlink",
        TreeMode::Gitlink => "submodule",
        _ => "file",
    }
}

/// `GET /repos/{o}/{r}/contents/{path}?ref=` against a hosted bare repo.
pub fn contents(
    root: &Path,
    owner: &str,
    name: &str,
    path: &str,
    rev: Option<&str>,
) -> Result<Value, (u16, String)> {
    let not_found = || (404u16, "Not Found".to_string());
    let dir = hosted_git_dir(root, owner, name).ok_or_else(not_found)?;
    let repo = Repository::open(&dir).map_err(|e| (500, e.to_string()))?;
    let commit_oid =
        resolve_commitish(&repo, rev).ok_or((404, "No commit found for the ref".into()))?;
    let commit = repo
        .get_object(&commit_oid)
        .map_err(|e| (500, e.to_string()))?;
    let commit = Commit::decode(&commit.payload).map_err(|e| (500, e.to_string()))?;
    let path = path.trim_matches('/');
    let mut cur_oid = commit.tree;
    let mut cur_mode = TreeMode::Tree;
    let mut walked = String::new();
    if !path.is_empty() {
        for seg in path.split('/') {
            if cur_mode != TreeMode::Tree {
                return Err(not_found());
            }
            let obj = repo
                .get_object(&cur_oid)
                .map_err(|e| (500, e.to_string()))?;
            let tree =
                Tree::decode(&obj.payload, repo.oid_kind()).map_err(|e| (500, e.to_string()))?;
            let e = tree
                .entries
                .into_iter()
                .find(|e| e.name == seg)
                .ok_or_else(not_found)?;
            cur_oid = e.oid;
            cur_mode = e.mode;
            if !walked.is_empty() {
                walked.push('/');
            }
            walked.push_str(seg);
        }
    }
    let ref_q = rev.map(|r| format!("?ref={r}")).unwrap_or_default();
    let api = |p: &str| format!("/repos/{owner}/{name}/contents/{p}{ref_q}");
    let git_url =
        |kind: &str, oid: &Oid| format!("/repos/{owner}/{name}/git/{kind}/{}", oid.to_hex());
    match cur_mode {
        TreeMode::Tree => {
            let obj = repo
                .get_object(&cur_oid)
                .map_err(|e| (500, e.to_string()))?;
            let tree =
                Tree::decode(&obj.payload, repo.oid_kind()).map_err(|e| (500, e.to_string()))?;
            let mut out = Vec::with_capacity(tree.entries.len());
            for e in tree.entries {
                let p = if walked.is_empty() {
                    e.name.clone()
                } else {
                    format!("{walked}/{}", e.name)
                };
                let size = if e.mode.is_tree() || e.mode == TreeMode::Gitlink {
                    0
                } else {
                    repo.get_object(&e.oid)
                        .map(|o| o.payload.len())
                        .unwrap_or(0)
                };
                let kind = if e.mode.is_tree() { "trees" } else { "blobs" };
                out.push(json!({
                    "type": type_of(e.mode),
                    "size": size,
                    "name": e.name,
                    "path": p,
                    "sha": e.oid.to_hex(),
                    "url": api(&p),
                    "git_url": git_url(kind, &e.oid),
                    "html_url": Value::Null,
                    "download_url": Value::Null,
                }));
            }
            Ok(Value::Array(out))
        }
        TreeMode::Gitlink => Ok(json!({
            "type": "submodule",
            "size": 0,
            "name": walked.rsplit('/').next().unwrap_or(""),
            "path": walked,
            "sha": cur_oid.to_hex(),
            "url": api(&walked),
            "git_url": Value::Null,
            "html_url": Value::Null,
            "download_url": Value::Null,
        })),
        _ => {
            let blob = repo
                .get_object(&cur_oid)
                .map_err(|e| (500, e.to_string()))?;
            let b64 = base64::engine::general_purpose::STANDARD.encode(&blob.payload);
            let mut v = json!({
                "type": type_of(cur_mode),
                "encoding": "base64",
                "size": blob.payload.len(),
                "name": walked.rsplit('/').next().unwrap_or(""),
                "path": walked,
                "content": b64,
                "sha": cur_oid.to_hex(),
                "url": api(&walked),
                "git_url": git_url("blobs", &cur_oid),
                "html_url": Value::Null,
                "download_url": Value::Null,
            });
            if cur_mode == TreeMode::Symlink {
                v["target"] = json!(String::from_utf8_lossy(&blob.payload));
            }
            Ok(v)
        }
    }
}

/// Create `refs/heads/<branch>` at `oid` in the hosted repo; refuse when it exists.
pub fn create_branch(
    root: &Path,
    owner: &str,
    name: &str,
    branch: &str,
    oid: &Oid,
) -> Result<(), String> {
    let dir = hosted_git_dir(root, owner, name)
        .ok_or_else(|| format!("{owner}/{name} has no hosted git repository"))?;
    let mut repo = Repository::open(&dir).map_err(|e| e.to_string())?;
    let full = format!("refs/heads/{branch}");
    if !arks_refs::is_valid_refname(&full) {
        return Err(format!("invalid branch name `{branch}`"));
    }
    if repo.refs().resolve(&full).is_ok() {
        return Err(format!("branch `{branch}` already exists"));
    }
    if repo
        .get_object(oid)
        .map(|o| o.kind != ObjectKind::Commit)
        .unwrap_or(true)
    {
        return Err(format!("commit {} not found", oid.to_hex()));
    }
    repo.refs_mut()
        .update(&full, oid, None)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_traversal_segments() {
        let root = Path::new("/nonexistent");
        assert!(hosted_git_dir(root, "..", "x").is_none());
        assert!(hosted_git_dir(root, "a/b", "x").is_none());
        assert!(hosted_git_dir(root, "a", "").is_none());
    }
}
