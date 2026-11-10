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
    pub next_run: u64,
    #[serde(default)]
    pub next_workflow: u64,
    #[serde(default)]
    pub next_milestone: u64,
    #[serde(default)]
    pub next_gist: u64,
    #[serde(default)]
    pub next_project: u64,
    #[serde(default)]
    pub next_secret: u64,
    #[serde(default)]
    pub next_variable: u64,
    #[serde(default)]
    pub next_ssh_key: u64,
    #[serde(default)]
    pub next_gpg_key: u64,
    #[serde(default)]
    pub next_discussion: u64,
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
    pub runs: Vec<WorkflowRun>,
    #[serde(default)]
    pub workflows: Vec<Workflow>,
    #[serde(default)]
    pub milestones: Vec<Milestone>,
    #[serde(default)]
    pub gists: Vec<Gist>,
    #[serde(default)]
    pub projects: Vec<Project>,
    #[serde(default)]
    pub secrets: Vec<SecretMeta>,
    #[serde(default)]
    pub variables: Vec<Variable>,
    #[serde(default)]
    pub ssh_keys: Vec<SshKey>,
    #[serde(default)]
    pub gpg_keys: Vec<GpgKey>,
    #[serde(default)]
    pub discussions: Vec<Discussion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repo {
    pub id: u64,
    pub owner: String,
    pub name: String,
    pub description: String,
    pub private: bool,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: u64,
    pub owner: String,
    pub repo: String,
    pub name: String,
    pub path: String,
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRun {
    pub id: u64,
    pub owner: String,
    pub repo: String,
    pub name: String,
    pub status: String,
    pub conclusion: String,
    pub workflow_id: u64,
    pub head_branch: String,
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

/// Secret metadata only — never persist plaintext values (Partial).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretMeta {
    pub id: u64,
    pub owner: String,
    pub repo: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variable {
    pub id: u64,
    pub owner: String,
    pub repo: String,
    pub name: String,
    pub value: String,
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
                next_run: 1,
                next_workflow: 1,
                next_milestone: 1,
                next_gist: 1,
                next_project: 1,
                next_secret: 1,
                next_variable: 1,
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
        };
        self.db.pulls.push(pull.clone());
        pull
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
        };
        self.db.repos.push(repo.clone());
        repo
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

    pub fn ensure_default_workflow(&mut self, owner: &str, repo: &str) -> Workflow {
        if let Some(w) = self
            .db
            .workflows
            .iter()
            .find(|w| w.owner == owner && w.repo == repo)
        {
            return w.clone();
        }
        if self.db.next_workflow == 0 {
            self.db.next_workflow = 1;
        }
        let id = self.db.next_workflow;
        self.db.next_workflow += 1;
        let wf = Workflow {
            id,
            owner: owner.into(),
            repo: repo.into(),
            name: "ci".into(),
            path: ".arks/workflows/ci.yml".into(),
            state: "active".into(),
        };
        self.db.workflows.push(wf.clone());
        wf
    }

    pub fn create_run(
        &mut self,
        owner: &str,
        repo: &str,
        name: &str,
        head_branch: &str,
    ) -> WorkflowRun {
        let wf = self.ensure_default_workflow(owner, repo);
        if self.db.next_run == 0 {
            self.db.next_run = 1;
        }
        let id = self.db.next_run;
        self.db.next_run += 1;
        let run = WorkflowRun {
            id,
            owner: owner.into(),
            repo: repo.into(),
            name: name.into(),
            status: "completed".into(),
            conclusion: "success".into(),
            workflow_id: wf.id,
            head_branch: head_branch.into(),
        };
        self.db.runs.push(run.clone());
        run
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

    pub fn create_project(
        &mut self,
        owner: &str,
        repo: &str,
        title: &str,
        body: &str,
    ) -> Project {
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

    pub fn create_secret_meta(&mut self, owner: &str, repo: &str, name: &str) -> SecretMeta {
        if self.db.next_secret == 0 {
            self.db.next_secret = 1;
        }
        let id = self.db.next_secret;
        self.db.next_secret += 1;
        let s = SecretMeta {
            id,
            owner: owner.into(),
            repo: repo.into(),
            name: name.into(),
        };
        self.db.secrets.push(s.clone());
        s
    }

    pub fn create_variable(
        &mut self,
        owner: &str,
        repo: &str,
        name: &str,
        value: &str,
    ) -> Variable {
        if self.db.next_variable == 0 {
            self.db.next_variable = 1;
        }
        let id = self.db.next_variable;
        self.db.next_variable += 1;
        let v = Variable {
            id,
            owner: owner.into(),
            repo: repo.into(),
            name: name.into(),
            value: value.into(),
        };
        self.db.variables.push(v.clone());
        v
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
