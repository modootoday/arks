//! File-backed local forge store (`db.json`; Partial; not Claimed). Never spawns git/gh.
//!
//! Every field added after the first schema carries `#[serde(default…)]`, so older
//! `db.json` files keep loading. Feature logic lives in `crate::forge::*`; this
//! module owns the stored model, persistence, tokens and asset bytes.

use crate::forge::artifacts::Artifact;
use crate::forge::autolink::Autolink;
use crate::forge::discussion::{Discussion, DiscussionComment};
use crate::forge::gist::Gist;
use crate::forge::keys::{DeployKey, GpgKey, SshKey};
use crate::forge::project::{Project, ProjectField, ProjectItem};
use crate::forge::rules::Ruleset;
use crate::forge::status::CommitStatus;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn yes() -> bool {
    true
}

fn main_branch() -> String {
    "main".into()
}

fn issue_kind() -> String {
    "issue".into()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HubDb {
    pub next_issue: u64,
    pub next_pr: u64,
    pub next_repo: u64,
    pub next_label: u64,
    #[serde(default)]
    pub next_release: u64,
    #[serde(default)]
    pub next_milestone: u64,
    #[serde(default)]
    pub next_gist: u64,
    #[serde(default)]
    pub next_project: u64,
    #[serde(default)]
    pub next_ssh_key: u64,
    #[serde(default)]
    pub next_gpg_key: u64,
    #[serde(default)]
    pub next_discussion: u64,
    #[serde(default)]
    pub next_comment: u64,
    #[serde(default)]
    pub next_review: u64,
    #[serde(default)]
    pub next_asset: u64,
    #[serde(default)]
    pub next_deploy_key: u64,
    #[serde(default)]
    pub next_autolink: u64,
    #[serde(default)]
    pub next_project_field: u64,
    #[serde(default)]
    pub next_project_item: u64,
    #[serde(default)]
    pub next_discussion_comment: u64,
    #[serde(default)]
    pub next_status: u64,
    #[serde(default)]
    pub next_ruleset: u64,
    #[serde(default)]
    pub next_artifact: u64,
    pub tokens: Vec<String>,
    pub user_login: String,
    /// Token → account login (multi-account `auth login|switch`); unknown tokens act
    /// as `user_login`.
    #[serde(default)]
    pub token_logins: BTreeMap<String, String>,
    pub repos: Vec<Repo>,
    pub issues: Vec<Issue>,
    pub pulls: Vec<Pull>,
    #[serde(default)]
    pub labels: Vec<Label>,
    #[serde(default)]
    pub releases: Vec<Release>,
    #[serde(default)]
    pub assets: Vec<ReleaseAsset>,
    #[serde(default)]
    pub milestones: Vec<Milestone>,
    #[serde(default)]
    pub gists: Vec<Gist>,
    #[serde(default)]
    pub projects: Vec<Project>,
    #[serde(default)]
    pub ssh_keys: Vec<SshKey>,
    #[serde(default)]
    pub gpg_keys: Vec<GpgKey>,
    #[serde(default)]
    pub discussions: Vec<Discussion>,
    #[serde(default)]
    pub comments: Vec<Comment>,
    #[serde(default)]
    pub reviews: Vec<Review>,
    #[serde(default)]
    pub discussion_comments: Vec<DiscussionComment>,
    #[serde(default)]
    pub project_fields: Vec<ProjectField>,
    #[serde(default)]
    pub project_items: Vec<ProjectItem>,
    #[serde(default)]
    pub deploy_keys: Vec<DeployKey>,
    #[serde(default)]
    pub autolinks: Vec<Autolink>,
    #[serde(default)]
    pub statuses: Vec<CommitStatus>,
    #[serde(default)]
    pub rulesets: Vec<Ruleset>,
    #[serde(default, rename = "issue_artifacts")]
    pub artifacts: Vec<Artifact>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repo {
    pub id: u64,
    pub owner: String,
    pub name: String,
    pub description: String,
    pub private: bool,
    /// Local-forge metadata archive flag (≠ git gc; never spawn git).
    #[serde(default)]
    pub archived: bool,
    #[serde(default)]
    pub homepage: String,
    #[serde(default)]
    pub topics: Vec<String>,
    #[serde(default = "main_branch")]
    pub default_branch: String,
    /// `internal` when set; otherwise visibility follows `private`.
    #[serde(default)]
    pub internal: bool,
    #[serde(default = "yes")]
    pub has_issues: bool,
    #[serde(default = "yes")]
    pub has_wiki: bool,
    #[serde(default = "yes")]
    pub has_projects: bool,
    #[serde(default)]
    pub has_discussions: bool,
    #[serde(default)]
    pub is_template: bool,
    #[serde(default)]
    pub delete_branch_on_merge: bool,
    #[serde(default = "yes")]
    pub allow_merge_commit: bool,
    #[serde(default = "yes")]
    pub allow_squash_merge: bool,
    #[serde(default = "yes")]
    pub allow_rebase_merge: bool,
    #[serde(default)]
    pub allow_auto_merge: bool,
    #[serde(default)]
    pub allow_update_branch: bool,
    #[serde(default = "yes")]
    pub allow_forking: bool,
    #[serde(default)]
    pub squash_merge_commit_message: String,
    /// `owner/name` of the fork parent.
    #[serde(default)]
    pub parent: Option<String>,
    #[serde(default)]
    pub license: Option<String>,
    #[serde(default)]
    pub created_at: u64,
    #[serde(default)]
    pub updated_at: u64,
}

impl Repo {
    pub fn full_name(&self) -> String {
        format!("{}/{}", self.owner, self.name)
    }

    pub fn visibility(&self) -> &'static str {
        if self.internal {
            "internal"
        } else if self.private {
            "private"
        } else {
            "public"
        }
    }
}

/// A branch linked to an issue (`issue develop`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LinkedBranch {
    pub name: String,
    /// `owner/name` of the repository holding the branch.
    pub repo: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub number: u64,
    pub owner: String,
    pub repo: String,
    pub title: String,
    pub body: String,
    pub state: String,
    pub user: String,
    #[serde(default)]
    pub locked: bool,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default)]
    pub labels: Vec<String>,
    #[serde(default)]
    pub assignees: Vec<String>,
    /// Milestone number.
    #[serde(default)]
    pub milestone: Option<u64>,
    #[serde(default)]
    pub state_reason: Option<String>,
    #[serde(default)]
    pub lock_reason: Option<String>,
    #[serde(default)]
    pub projects: Vec<String>,
    #[serde(default)]
    pub linked_branches: Vec<LinkedBranch>,
    #[serde(default)]
    pub created_at: u64,
    #[serde(default)]
    pub updated_at: u64,
    #[serde(default)]
    pub closed_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pull {
    pub number: u64,
    pub owner: String,
    pub repo: String,
    pub title: String,
    pub body: String,
    pub state: String,
    pub head: String,
    pub base: String,
    pub user: String,
    /// Merged flag; with forge Git content the base branch really moves.
    #[serde(default)]
    pub merged: bool,
    /// Draft PR metadata (gh `pr ready` clears this).
    #[serde(default)]
    pub draft: bool,
    #[serde(default)]
    pub locked: bool,
    #[serde(default)]
    pub labels: Vec<String>,
    #[serde(default)]
    pub assignees: Vec<String>,
    /// Requested reviewers (logins).
    #[serde(default)]
    pub reviewers: Vec<String>,
    #[serde(default)]
    pub milestone: Option<u64>,
    #[serde(default)]
    pub projects: Vec<String>,
    #[serde(default = "yes")]
    pub maintainer_can_modify: bool,
    #[serde(default)]
    pub lock_reason: Option<String>,
    /// Auto-merge method (`merge` / `squash` / `rebase`) while enabled.
    #[serde(default)]
    pub auto_merge: Option<String>,
    #[serde(default)]
    pub merge_commit_sha: Option<String>,
    #[serde(default)]
    pub merged_by: Option<String>,
    #[serde(default)]
    pub created_at: u64,
    #[serde(default)]
    pub updated_at: u64,
    #[serde(default)]
    pub closed_at: Option<u64>,
    #[serde(default)]
    pub merged_at: Option<u64>,
}

/// Pull-request review (≠ CI checks; never spawn git/gh).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Review {
    pub id: u64,
    pub owner: String,
    pub repo: String,
    pub pull_number: u64,
    pub body: String,
    pub event: String,
    pub user: String,
    pub state: String,
    #[serde(default)]
    pub submitted_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub id: u64,
    pub owner: String,
    pub repo: String,
    /// Issue or pull-request number (see `kind`: arks numbers them separately).
    pub number: u64,
    pub body: String,
    pub user: String,
    /// `issue` or `pull`.
    #[serde(default = "issue_kind")]
    pub kind: String,
    #[serde(default)]
    pub created_at: u64,
    #[serde(default)]
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Label {
    pub id: u64,
    pub owner: String,
    pub repo: String,
    pub name: String,
    pub color: String,
    pub description: String,
    #[serde(default)]
    pub created_at: u64,
    #[serde(default)]
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Release {
    pub id: u64,
    pub owner: String,
    pub repo: String,
    pub tag_name: String,
    pub name: String,
    pub body: String,
    pub draft: bool,
    pub prerelease: bool,
    #[serde(default)]
    pub target_commitish: String,
    /// Explicit `make_latest` choice; `None` = automatic (newest published).
    #[serde(default)]
    pub make_latest: Option<bool>,
    #[serde(default)]
    pub discussion_category: Option<String>,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub created_at: u64,
    #[serde(default)]
    pub published_at: Option<u64>,
}

/// Release asset metadata; bytes live under `data_dir/assets/{id}.bin`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseAsset {
    pub id: u64,
    pub release_id: u64,
    pub owner: String,
    pub repo: String,
    pub name: String,
    pub content_type: String,
    pub size: u64,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Milestone {
    pub number: u64,
    pub owner: String,
    pub repo: String,
    pub title: String,
    pub description: String,
    pub state: String,
}

pub struct HubStore {
    path: PathBuf,
    pub db: HubDb,
    /// Root holding forge repositories' Git directories (`<data>/git` by default).
    pub git_root: PathBuf,
    /// Optional clone URL base (`<base>/<owner>/<name>.git`).
    pub git_url: Option<String>,
    /// Login of the account behind the current request's token.
    pub actor: String,
    /// External base URL of this forge (`http://127.0.0.1:3920`) for `html_url`s.
    pub public_base: String,
}

impl HubStore {
    pub fn data_dir() -> PathBuf {
        if let Ok(p) = std::env::var("ARKS_HUB_DATA") {
            return PathBuf::from(p);
        }
        let home = std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        home.join(".config").join("arks").join("hub-data")
    }

    pub fn open(dir: &Path) -> Result<Self, String> {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        let path = dir.join("db.json");
        let db = if path.is_file() {
            let raw = fs::read_to_string(&path).map_err(|e| e.to_string())?;
            serde_json::from_str(&raw).unwrap_or_default()
        } else {
            let mut db = HubDb {
                next_issue: 1,
                next_pr: 1,
                next_repo: 1,
                next_label: 1,
                next_release: 1,
                next_milestone: 1,
                next_gist: 1,
                next_project: 1,
                next_ssh_key: 1,
                next_gpg_key: 1,
                next_discussion: 1,
                tokens: vec!["local-dev-token".into()],
                user_login: "local".into(),
                ..HubDb::default()
            };
            // Seed a default repo for local smoke.
            let now = now_secs();
            db.repos.push(Repo {
                id: 1,
                owner: "local".into(),
                name: "playground".into(),
                description: "local hub seed repo".into(),
                private: false,
                archived: false,
                homepage: String::new(),
                topics: Vec::new(),
                default_branch: main_branch(),
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
            });
            db.next_repo = 2;
            db
        };
        let actor = db.user_login.clone();
        let store = Self {
            path,
            db,
            git_root: dir.join("git"),
            git_url: None,
            actor,
            public_base: "http://127.0.0.1:3920".into(),
        };
        store.save()?;
        Ok(store)
    }

    pub fn save(&self) -> Result<(), String> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let raw = serde_json::to_string_pretty(&self.db).map_err(|e| e.to_string())?;
        fs::write(&self.path, raw).map_err(|e| e.to_string())
    }

    pub fn auth_ok(&self, token: Option<&str>) -> bool {
        match token {
            Some(t) if !t.is_empty() => {
                self.db.tokens.iter().any(|x| x == t) || t == "local-dev-token"
            }
            _ => false,
        }
    }

    /// Account login for a token (falls back to the forge's default user).
    pub fn login_for(&self, token: Option<&str>) -> String {
        token
            .and_then(|t| self.db.token_logins.get(t))
            .cloned()
            .unwrap_or_else(|| self.db.user_login.clone())
    }

    pub fn ensure_token(&mut self, token: &str) {
        if !self.db.tokens.iter().any(|t| t == token) {
            self.db.tokens.push(token.to_string());
        }
    }

    pub fn data_root(&self) -> PathBuf {
        self.path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."))
    }

    pub fn assets_dir(&self) -> PathBuf {
        self.data_root().join("assets")
    }

    pub fn read_asset_bytes(&self, id: u64) -> Option<Vec<u8>> {
        fs::read(self.assets_dir().join(format!("{id}.bin"))).ok()
    }

    pub fn find_repo(&self, owner: &str, name: &str) -> Option<&Repo> {
        self.db
            .repos
            .iter()
            .find(|r| r.owner == owner && r.name == name)
    }

    pub fn find_repo_mut(&mut self, owner: &str, name: &str) -> Option<&mut Repo> {
        self.db
            .repos
            .iter_mut()
            .find(|r| r.owner == owner && r.name == name)
    }

    /// Git directory for a forge repository, when it has content.
    pub fn git_dir(&self, owner: &str, name: &str) -> Option<PathBuf> {
        crate::forge::git::repo_dir(&self.git_root, owner, name)
    }
}

pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// DB files written before ADR 20261119230000 carry Actions keys; they must still load.
    #[test]
    fn legacy_db_with_actions_keys_loads() {
        let raw = r#"{
            "next_issue": 3, "next_pr": 1, "next_repo": 2, "next_label": 1,
            "next_run": 4, "next_workflow": 2, "next_secret": 2, "next_variable": 2,
            "next_artifact": 3,
            "tokens": ["t"], "user_login": "local",
            "repos": [], "issues": [], "pulls": [],
            "runs": [{"id": 1, "owner": "o", "repo": "r", "name": "ci", "status": "completed",
                      "conclusion": "success", "workflow_id": 1, "head_branch": "main"}],
            "artifacts": [{"id": 1, "run_id": 1, "owner": "o", "repo": "r", "name": "l", "size": 1}],
            "workflows": [{"id": 1, "owner": "o", "repo": "r", "name": "ci",
                           "path": ".arks/workflows/ci.yml", "state": "active"}],
            "secrets": [{"id": 1, "owner": "o", "repo": "r", "name": "S"}],
            "variables": [{"id": 1, "owner": "o", "repo": "r", "name": "V", "value": "x"}]
        }"#;
        let db: HubDb = serde_json::from_str(raw).expect("legacy db loads");
        assert_eq!(db.next_issue, 3);
        assert_eq!(db.user_login, "local");
        assert!(
            db.artifacts.is_empty(),
            "old Actions artifacts are not issue artifacts"
        );
        let out = serde_json::to_string(&db).unwrap();
        assert!(!out.contains("workflows"));
        assert!(!out.contains("next_run"));
    }

    /// DB files written before the arks-owned forge groups landed must still load,
    /// including list-only gists/projects/keys/discussions from the old Partial leaves.
    #[test]
    fn legacy_db_with_old_group_records_loads() {
        let raw = r#"{
            "next_issue": 1, "next_pr": 1, "next_repo": 2, "next_label": 1,
            "next_gist": 2, "next_project": 2, "next_ssh_key": 2, "next_gpg_key": 2,
            "next_discussion": 2,
            "tokens": ["t"], "user_login": "local",
            "repos": [], "issues": [], "pulls": [],
            "gists": [{"id": "gist1", "description": "d", "public": true, "files": ["a.txt"]}],
            "projects": [{"id": 1, "owner": "local", "repo": "x", "title": "p", "body": "", "state": "open"}],
            "ssh_keys": [{"id": 1, "title": "k", "key": "ssh-ed25519 AAAA Partial"}],
            "gpg_keys": [{"id": 1, "name": "g", "key_id": "00000000"}],
            "discussions": [{"number": 1, "owner": "local", "repo": "x", "title": "t",
                             "body": "b", "category": "General"}]
        }"#;
        let db: HubDb = serde_json::from_str(raw).expect("legacy group records load");
        assert_eq!(db.gists[0].files.len(), 1);
        assert_eq!(db.projects[0].owner, "local");
        assert!(db.ssh_keys[0].key_type.is_empty());
        assert_eq!(db.gpg_keys[0].key_id, "00000000");
        assert_eq!(db.discussions[0].state, "open");
        assert!(db.deploy_keys.is_empty() && db.autolinks.is_empty());
    }

    /// Core records written before the gh-argv wave (no labels, timestamps, repo
    /// settings) load with gh-like defaults.
    #[test]
    fn legacy_core_records_load_with_defaults() {
        let raw = r#"{
            "next_issue": 2, "next_pr": 2, "next_repo": 2, "next_label": 1,
            "tokens": ["t"], "user_login": "local",
            "repos": [{"id": 1, "owner": "o", "name": "r", "description": "", "private": false}],
            "issues": [{"number": 1, "owner": "o", "repo": "r", "title": "t", "body": "",
                        "state": "open", "user": "local"}],
            "pulls": [{"number": 1, "owner": "o", "repo": "r", "title": "p", "body": "",
                       "state": "open", "head": "f", "base": "main", "user": "local"}],
            "comments": [{"id": 1, "owner": "o", "repo": "r", "number": 1, "body": "c",
                          "user": "local"}]
        }"#;
        let db: HubDb = serde_json::from_str(raw).expect("legacy core loads");
        let r = &db.repos[0];
        assert_eq!(r.default_branch, "main");
        assert!(r.has_issues && r.allow_merge_commit && !r.is_template);
        assert!(db.issues[0].labels.is_empty());
        assert!(db.pulls[0].maintainer_can_modify);
        assert_eq!(db.comments[0].kind, "issue");
        assert!(db.statuses.is_empty() && db.rulesets.is_empty());
    }
}
