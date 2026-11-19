//! File-backed local hub store (Partial; not Claimed). Never spawns git/gh.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

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
    pub tokens: Vec<String>,
    pub user_login: String,
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
    /// Local-forge metadata merge only (≠ git worktree merge; never spawn git).
    #[serde(default)]
    pub merged: bool,
    /// Draft PR metadata (gh `pr ready` clears this).
    #[serde(default)]
    pub draft: bool,
    #[serde(default)]
    pub locked: bool,
}

/// PR review metadata only (≠ CI checks; never spawn git/gh).
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub id: u64,
    pub owner: String,
    pub repo: String,
    /// Issue or PR number (GitHub comments API shares issues path).
    pub number: u64,
    pub body: String,
    pub user: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Label {
    pub id: u64,
    pub owner: String,
    pub repo: String,
    pub name: String,
    pub color: String,
    pub description: String,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gist {
    pub id: String,
    pub description: String,
    pub public: bool,
    pub files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: u64,
    pub owner: String,
    pub repo: String,
    pub title: String,
    pub body: String,
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshKey {
    pub id: u64,
    pub title: String,
    pub key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpgKey {
    pub id: u64,
    pub name: String,
    pub key_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Discussion {
    pub number: u64,
    pub owner: String,
    pub repo: String,
    pub title: String,
    pub body: String,
    pub category: String,
}

pub struct HubStore {
    path: PathBuf,
    pub db: HubDb,
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
            db.repos.push(Repo {
                id: 1,
                owner: "local".into(),
                name: "playground".into(),
                description: "local hub seed repo".into(),
                private: false,
                archived: false,
            });
            db.next_repo = 2;
            db
        };
        let store = Self { path, db };
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

    pub fn ensure_token(&mut self, token: &str) {
        if !self.db.tokens.iter().any(|t| t == token) {
            self.db.tokens.push(token.to_string());
        }
    }

    pub fn create_issue(&mut self, owner: &str, repo: &str, title: &str, body: &str) -> Issue {
        let number = self.db.next_issue;
        self.db.next_issue += 1;
        let issue = Issue {
            number,
            owner: owner.into(),
            repo: repo.into(),
            title: title.into(),
            body: body.into(),
            state: "open".into(),
            user: self.db.user_login.clone(),
            locked: false,
            pinned: false,
        };
        self.db.issues.push(issue.clone());
        issue
    }

    pub fn create_pull(
        &mut self,
        owner: &str,
        repo: &str,
        title: &str,
        body: &str,
        head: &str,
        base: &str,
    ) -> Pull {
        let number = self.db.next_pr;
        self.db.next_pr += 1;
        let pull = Pull {
            number,
            owner: owner.into(),
            repo: repo.into(),
            title: title.into(),
            body: body.into(),
            state: "open".into(),
            head: head.into(),
            base: base.into(),
            user: self.db.user_login.clone(),
            merged: false,
            draft: false,
            locked: false,
        };
        self.db.pulls.push(pull.clone());
        pull
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_pull_draft(
        &mut self,
        owner: &str,
        repo: &str,
        title: &str,
        body: &str,
        head: &str,
        base: &str,
        draft: bool,
    ) -> Pull {
        let mut pull = self.create_pull(owner, repo, title, body, head, base);
        if draft {
            if let Some(p) = self
                .db
                .pulls
                .iter_mut()
                .find(|p| p.owner == owner && p.repo == repo && p.number == pull.number)
            {
                p.draft = true;
                pull = p.clone();
            }
        }
        pull
    }

    #[allow(clippy::too_many_arguments)]
    pub fn patch_issue(
        &mut self,
        owner: &str,
        repo: &str,
        number: u64,
        title: Option<&str>,
        body: Option<&str>,
        state: Option<&str>,
        locked: Option<bool>,
        pinned: Option<bool>,
    ) -> Option<Issue> {
        let issue = self
            .db
            .issues
            .iter_mut()
            .find(|i| i.owner == owner && i.repo == repo && i.number == number)?;
        if let Some(t) = title {
            issue.title = t.into();
        }
        if let Some(b) = body {
            issue.body = b.into();
        }
        if let Some(s) = state {
            issue.state = s.into();
        }
        if let Some(l) = locked {
            issue.locked = l;
        }
        if let Some(p) = pinned {
            issue.pinned = p;
        }
        Some(issue.clone())
    }

    pub fn transfer_issue(
        &mut self,
        owner: &str,
        repo: &str,
        number: u64,
        to_owner: &str,
        to_repo: &str,
    ) -> Option<Issue> {
        // Ensure destination repo exists in local forge.
        if !self
            .db
            .repos
            .iter()
            .any(|r| r.owner == to_owner && r.name == to_repo)
        {
            return None;
        }
        let issue = self
            .db
            .issues
            .iter_mut()
            .find(|i| i.owner == owner && i.repo == repo && i.number == number)?;
        issue.owner = to_owner.into();
        issue.repo = to_repo.into();
        // Re-number into destination sequence.
        let new_number = self.db.next_issue;
        self.db.next_issue += 1;
        issue.number = new_number;
        Some(issue.clone())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn patch_pull(
        &mut self,
        owner: &str,
        repo: &str,
        number: u64,
        title: Option<&str>,
        body: Option<&str>,
        state: Option<&str>,
        draft: Option<bool>,
        locked: Option<bool>,
    ) -> Option<Pull> {
        let pull = self
            .db
            .pulls
            .iter_mut()
            .find(|p| p.owner == owner && p.repo == repo && p.number == number)?;
        if let Some(t) = title {
            pull.title = t.into();
        }
        if let Some(b) = body {
            pull.body = b.into();
        }
        if let Some(s) = state {
            pull.state = s.into();
        }
        if let Some(d) = draft {
            pull.draft = d;
        }
        if let Some(l) = locked {
            pull.locked = l;
        }
        Some(pull.clone())
    }

    pub fn create_review(
        &mut self,
        owner: &str,
        repo: &str,
        pull_number: u64,
        body: &str,
        event: &str,
    ) -> Option<Review> {
        if !self
            .db
            .pulls
            .iter()
            .any(|p| p.owner == owner && p.repo == repo && p.number == pull_number)
        {
            return None;
        }
        if self.db.next_review == 0 {
            self.db.next_review = 1;
        }
        let id = self.db.next_review;
        self.db.next_review += 1;
        let state = match event {
            "APPROVE" => "APPROVED",
            "REQUEST_CHANGES" => "CHANGES_REQUESTED",
            _ => "COMMENTED",
        };
        let review = Review {
            id,
            owner: owner.into(),
            repo: repo.into(),
            pull_number,
            body: body.into(),
            event: event.into(),
            user: self.db.user_login.clone(),
            state: state.into(),
        };
        self.db.reviews.push(review.clone());
        Some(review)
    }

    pub fn rename_repo(&mut self, owner: &str, name: &str, new_name: &str) -> Option<Repo> {
        if self
            .db
            .repos
            .iter()
            .any(|r| r.owner == owner && r.name == new_name)
        {
            return None;
        }
        let repo = self
            .db
            .repos
            .iter_mut()
            .find(|r| r.owner == owner && r.name == name)?;
        repo.name = new_name.into();
        let out = repo.clone();
        // Retarget issues/pulls/labels under old name.
        for i in self
            .db
            .issues
            .iter_mut()
            .filter(|i| i.owner == owner && i.repo == name)
        {
            i.repo = new_name.into();
        }
        for p in self
            .db
            .pulls
            .iter_mut()
            .filter(|p| p.owner == owner && p.repo == name)
        {
            p.repo = new_name.into();
        }
        for l in self
            .db
            .labels
            .iter_mut()
            .filter(|l| l.owner == owner && l.repo == name)
        {
            l.repo = new_name.into();
        }
        Some(out)
    }

    pub fn delete_repo(&mut self, owner: &str, name: &str) -> bool {
        let before = self.db.repos.len();
        self.db
            .repos
            .retain(|r| !(r.owner == owner && r.name == name));
        if self.db.repos.len() == before {
            return false;
        }
        self.db
            .issues
            .retain(|i| !(i.owner == owner && i.repo == name));
        self.db
            .pulls
            .retain(|p| !(p.owner == owner && p.repo == name));
        self.db
            .labels
            .retain(|l| !(l.owner == owner && l.repo == name));
        self.db
            .releases
            .retain(|r| !(r.owner == owner && r.repo == name));
        true
    }

    /// Metadata-only merge: marks pull merged+closed. Never touches git objects.
    pub fn merge_pull(&mut self, owner: &str, repo: &str, number: u64) -> Option<Pull> {
        let pull = self
            .db
            .pulls
            .iter_mut()
            .find(|p| p.owner == owner && p.repo == repo && p.number == number)?;
        pull.merged = true;
        pull.state = "closed".into();
        Some(pull.clone())
    }

    pub fn create_comment(&mut self, owner: &str, repo: &str, number: u64, body: &str) -> Comment {
        if self.db.next_comment == 0 {
            self.db.next_comment = 1;
        }
        let id = self.db.next_comment;
        self.db.next_comment += 1;
        let comment = Comment {
            id,
            owner: owner.into(),
            repo: repo.into(),
            number,
            body: body.into(),
            user: self.db.user_login.clone(),
        };
        self.db.comments.push(comment.clone());
        comment
    }

    pub fn patch_label(
        &mut self,
        owner: &str,
        repo: &str,
        name: &str,
        new_name: Option<&str>,
        color: Option<&str>,
        description: Option<&str>,
    ) -> Option<Label> {
        let label = self
            .db
            .labels
            .iter_mut()
            .find(|l| l.owner == owner && l.repo == repo && l.name == name)?;
        if let Some(n) = new_name {
            label.name = n.into();
        }
        if let Some(c) = color {
            label.color = c.into();
        }
        if let Some(d) = description {
            label.description = d.into();
        }
        Some(label.clone())
    }

    pub fn delete_label(&mut self, owner: &str, repo: &str, name: &str) -> bool {
        let before = self.db.labels.len();
        self.db
            .labels
            .retain(|l| !(l.owner == owner && l.repo == repo && l.name == name));
        self.db.labels.len() != before
    }

    pub fn find_release(&self, owner: &str, repo: &str, tag_or_id: &str) -> Option<&Release> {
        if let Ok(id) = tag_or_id.parse::<u64>() {
            if let Some(r) = self
                .db
                .releases
                .iter()
                .find(|r| r.owner == owner && r.repo == repo && r.id == id)
            {
                return Some(r);
            }
        }
        self.db
            .releases
            .iter()
            .find(|r| r.owner == owner && r.repo == repo && r.tag_name == tag_or_id)
    }

    pub fn create_repo(&mut self, owner: &str, name: &str, description: &str) -> Repo {
        let id = self.db.next_repo;
        self.db.next_repo += 1;
        let repo = Repo {
            id,
            owner: owner.into(),
            name: name.into(),
            description: description.into(),
            private: false,
            archived: false,
        };
        self.db.repos.push(repo.clone());
        repo
    }

    pub fn patch_repo(
        &mut self,
        owner: &str,
        name: &str,
        description: Option<&str>,
        private: Option<bool>,
        archived: Option<bool>,
    ) -> Option<Repo> {
        let repo = self
            .db
            .repos
            .iter_mut()
            .find(|r| r.owner == owner && r.name == name)?;
        if let Some(d) = description {
            repo.description = d.into();
        }
        if let Some(p) = private {
            repo.private = p;
        }
        if let Some(a) = archived {
            repo.archived = a;
        }
        Some(repo.clone())
    }

    pub fn delete_issue(&mut self, owner: &str, repo: &str, number: u64) -> bool {
        let before = self.db.issues.len();
        self.db
            .issues
            .retain(|i| !(i.owner == owner && i.repo == repo && i.number == number));
        self.db.issues.len() < before
    }

    pub fn create_label(
        &mut self,
        owner: &str,
        repo: &str,
        name: &str,
        color: &str,
        description: &str,
    ) -> Label {
        if self.db.next_label == 0 {
            self.db.next_label = 1;
        }
        let id = self.db.next_label;
        self.db.next_label += 1;
        let label = Label {
            id,
            owner: owner.into(),
            repo: repo.into(),
            name: name.into(),
            color: color.into(),
            description: description.into(),
        };
        self.db.labels.push(label.clone());
        label
    }

    pub fn create_release(
        &mut self,
        owner: &str,
        repo: &str,
        tag_name: &str,
        name: &str,
        body: &str,
    ) -> Release {
        if self.db.next_release == 0 {
            self.db.next_release = 1;
        }
        let id = self.db.next_release;
        self.db.next_release += 1;
        let release = Release {
            id,
            owner: owner.into(),
            repo: repo.into(),
            tag_name: tag_name.into(),
            name: if name.is_empty() {
                tag_name.into()
            } else {
                name.into()
            },
            body: body.into(),
            draft: false,
            prerelease: false,
        };
        self.db.releases.push(release.clone());
        release
    }

    fn data_root(&self) -> PathBuf {
        self.path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."))
    }

    fn assets_dir(&self) -> PathBuf {
        self.data_root().join("assets")
    }

    pub fn upload_release_asset(
        &mut self,
        owner: &str,
        repo: &str,
        release_id: u64,
        name: &str,
        content_type: &str,
        bytes: &[u8],
    ) -> Option<ReleaseAsset> {
        if !self
            .db
            .releases
            .iter()
            .any(|r| r.owner == owner && r.repo == repo && r.id == release_id)
        {
            return None;
        }
        if self.db.next_asset == 0 {
            self.db.next_asset = 1;
        }
        let id = self.db.next_asset;
        self.db.next_asset += 1;
        let dir = self.assets_dir();
        let _ = fs::create_dir_all(&dir);
        let path = dir.join(format!("{id}.bin"));
        fs::write(&path, bytes).ok()?;
        let asset = ReleaseAsset {
            id,
            release_id,
            owner: owner.into(),
            repo: repo.into(),
            name: name.into(),
            content_type: if content_type.is_empty() {
                "application/octet-stream".into()
            } else {
                content_type.into()
            },
            size: bytes.len() as u64,
        };
        self.db.assets.push(asset.clone());
        Some(asset)
    }

    pub fn find_asset(&self, owner: &str, repo: &str, id: u64) -> Option<&ReleaseAsset> {
        self.db
            .assets
            .iter()
            .find(|a| a.owner == owner && a.repo == repo && a.id == id)
    }

    pub fn read_asset_bytes(&self, id: u64) -> Option<Vec<u8>> {
        let path = self.assets_dir().join(format!("{id}.bin"));
        fs::read(path).ok()
    }

    pub fn delete_asset(&mut self, owner: &str, repo: &str, id: u64) -> bool {
        let before = self.db.assets.len();
        self.db
            .assets
            .retain(|a| !(a.owner == owner && a.repo == repo && a.id == id));
        let path = self.assets_dir().join(format!("{id}.bin"));
        let _ = fs::remove_file(path);
        self.db.assets.len() != before
    }

    pub fn update_release(
        &mut self,
        owner: &str,
        repo: &str,
        tag_or_id: &str,
        name: Option<&str>,
        body: Option<&str>,
        tag_name: Option<&str>,
    ) -> Option<Release> {
        let id = self.find_release(owner, repo, tag_or_id)?.id;
        let rel = self
            .db
            .releases
            .iter_mut()
            .find(|r| r.owner == owner && r.repo == repo && r.id == id)?;
        if let Some(n) = name {
            rel.name = n.into();
        }
        if let Some(b) = body {
            rel.body = b.into();
        }
        if let Some(t) = tag_name {
            rel.tag_name = t.into();
        }
        Some(rel.clone())
    }

    pub fn delete_release(&mut self, owner: &str, repo: &str, tag_or_id: &str) -> bool {
        let Some(id) = self.find_release(owner, repo, tag_or_id).map(|r| r.id) else {
            return false;
        };
        self.db
            .releases
            .retain(|r| !(r.owner == owner && r.repo == repo && r.id == id));
        let asset_ids: Vec<u64> = self
            .db
            .assets
            .iter()
            .filter(|a| a.owner == owner && a.repo == repo && a.release_id == id)
            .map(|a| a.id)
            .collect();
        for aid in asset_ids {
            let _ = self.delete_asset(owner, repo, aid);
        }
        true
    }

    pub fn create_milestone(
        &mut self,
        owner: &str,
        repo: &str,
        title: &str,
        description: &str,
    ) -> Milestone {
        if self.db.next_milestone == 0 {
            self.db.next_milestone = 1;
        }
        let number = self.db.next_milestone;
        self.db.next_milestone += 1;
        let m = Milestone {
            number,
            owner: owner.into(),
            repo: repo.into(),
            title: title.into(),
            description: description.into(),
            state: "open".into(),
        };
        self.db.milestones.push(m.clone());
        m
    }

    pub fn create_gist(&mut self, description: &str, public: bool, files: Vec<String>) -> Gist {
        if self.db.next_gist == 0 {
            self.db.next_gist = 1;
        }
        let id = format!("gist{}", self.db.next_gist);
        self.db.next_gist += 1;
        let g = Gist {
            id,
            description: description.into(),
            public,
            files,
        };
        self.db.gists.push(g.clone());
        g
    }

    pub fn create_project(&mut self, owner: &str, repo: &str, title: &str, body: &str) -> Project {
        if self.db.next_project == 0 {
            self.db.next_project = 1;
        }
        let id = self.db.next_project;
        self.db.next_project += 1;
        let p = Project {
            id,
            owner: owner.into(),
            repo: repo.into(),
            title: title.into(),
            body: body.into(),
            state: "open".into(),
        };
        self.db.projects.push(p.clone());
        p
    }

    pub fn create_ssh_key(&mut self, title: &str, key: &str) -> SshKey {
        if self.db.next_ssh_key == 0 {
            self.db.next_ssh_key = 1;
        }
        let id = self.db.next_ssh_key;
        self.db.next_ssh_key += 1;
        let k = SshKey {
            id,
            title: title.into(),
            key: key.into(),
        };
        self.db.ssh_keys.push(k.clone());
        k
    }

    pub fn create_gpg_key(&mut self, name: &str, key_id: &str) -> GpgKey {
        if self.db.next_gpg_key == 0 {
            self.db.next_gpg_key = 1;
        }
        let id = self.db.next_gpg_key;
        self.db.next_gpg_key += 1;
        let k = GpgKey {
            id,
            name: name.into(),
            key_id: key_id.into(),
        };
        self.db.gpg_keys.push(k.clone());
        k
    }

    pub fn create_discussion(
        &mut self,
        owner: &str,
        repo: &str,
        title: &str,
        body: &str,
        category: &str,
    ) -> Discussion {
        if self.db.next_discussion == 0 {
            self.db.next_discussion = 1;
        }
        let number = self.db.next_discussion;
        self.db.next_discussion += 1;
        let d = Discussion {
            number,
            owner: owner.into(),
            repo: repo.into(),
            title: title.into(),
            body: body.into(),
            category: category.into(),
        };
        self.db.discussions.push(d.clone());
        d
    }
}

#[allow(dead_code)] // reserved for future Partial timestamps on forge resources
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
        let out = serde_json::to_string(&db).unwrap();
        assert!(!out.contains("workflows"));
        assert!(!out.contains("next_run"));
    }
}
