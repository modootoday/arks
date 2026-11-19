//! Local forge records for repo autolinks, deploy keys, issue artifacts and
//! issue linked branches (Partial; host advisory). Never spawns git/gh.
//!
//! Shapes follow the GitHub REST reference:
//! - autolinks: <https://docs.github.com/en/rest/repos/autolinks>
//! - deploy keys: <https://docs.github.com/en/rest/deploy-keys/deploy-keys>
//! - issue artifacts: <https://docs.github.com/en/rest/issues/artifacts>
//!
//! Linked branches follow the GraphQL `createLinkedBranch` mutation and
//! `Issue.linkedBranches` connection
//! (<https://docs.github.com/en/graphql/reference/issues>).

use crate::store::{HubStore, now_secs};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Autolink {
    pub id: u64,
    pub owner: String,
    pub repo: String,
    pub key_prefix: String,
    pub url_template: String,
    pub is_alphanumeric: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployKey {
    pub id: u64,
    pub owner: String,
    pub repo: String,
    pub title: String,
    pub key: String,
    pub read_only: bool,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactVersion {
    pub version: u64,
    pub name: String,
    pub body: String,
    pub created_at: u64,
    pub actor: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueArtifact {
    pub id: u64,
    pub owner: String,
    pub repo: String,
    pub issue: u64,
    pub number: u64,
    pub kind: String,
    pub name: String,
    pub body: String,
    pub creator: String,
    pub created_at: u64,
    pub updated_at: u64,
    /// Newest last (REST view orders newest first).
    pub versions: Vec<ArtifactVersion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkedBranch {
    pub id: u64,
    pub owner: String,
    pub repo: String,
    pub issue: u64,
    /// Repository that holds the branch (`owner/name`).
    pub branch_repo: String,
    pub name: String,
}

/// Valid `type` values for issue artifacts (REST reference).
pub const ARTIFACT_TYPES: &[&str] = &["generic", "link", "plan", "embedded_html"];

impl HubStore {
    pub fn repo_exists(&self, owner: &str, name: &str) -> bool {
        self.db
            .repos
            .iter()
            .any(|r| r.owner == owner && r.name == name)
    }

    pub fn issue_exists(&self, owner: &str, repo: &str, number: u64) -> bool {
        self.db
            .issues
            .iter()
            .any(|i| i.owner == owner && i.repo == repo && i.number == number)
    }

    pub fn create_autolink(
        &mut self,
        owner: &str,
        repo: &str,
        key_prefix: &str,
        url_template: &str,
        is_alphanumeric: bool,
    ) -> Result<Autolink, String> {
        if self
            .db
            .autolinks
            .iter()
            .any(|a| a.owner == owner && a.repo == repo && a.key_prefix == key_prefix)
        {
            return Err("key_prefix already exists".into());
        }
        let id = self.db.next_autolink.max(1);
        self.db.next_autolink = id + 1;
        let a = Autolink {
            id,
            owner: owner.into(),
            repo: repo.into(),
            key_prefix: key_prefix.into(),
            url_template: url_template.into(),
            is_alphanumeric,
        };
        self.db.autolinks.push(a.clone());
        Ok(a)
    }

    pub fn delete_autolink(&mut self, owner: &str, repo: &str, id: u64) -> bool {
        let before = self.db.autolinks.len();
        self.db
            .autolinks
            .retain(|a| !(a.owner == owner && a.repo == repo && a.id == id));
        self.db.autolinks.len() != before
    }

    pub fn create_deploy_key(
        &mut self,
        owner: &str,
        repo: &str,
        title: &str,
        key: &str,
        read_only: bool,
    ) -> Result<DeployKey, String> {
        if self
            .db
            .deploy_keys
            .iter()
            .any(|k| k.owner == owner && k.repo == repo && k.key == key)
        {
            return Err("key is already in use".into());
        }
        let id = self.db.next_deploy_key.max(1);
        self.db.next_deploy_key = id + 1;
        let k = DeployKey {
            id,
            owner: owner.into(),
            repo: repo.into(),
            title: title.into(),
            key: key.into(),
            read_only,
            created_at: now_secs(),
        };
        self.db.deploy_keys.push(k.clone());
        Ok(k)
    }

    pub fn delete_deploy_key(&mut self, owner: &str, repo: &str, id: u64) -> bool {
        let before = self.db.deploy_keys.len();
        self.db
            .deploy_keys
            .retain(|k| !(k.owner == owner && k.repo == repo && k.id == id));
        self.db.deploy_keys.len() != before
    }

    pub fn create_issue_artifact(
        &mut self,
        owner: &str,
        repo: &str,
        issue: u64,
        kind: &str,
        name: &str,
        body: &str,
    ) -> IssueArtifact {
        let id = self.db.next_issue_artifact.max(1);
        self.db.next_issue_artifact = id + 1;
        let number = self
            .db
            .issue_artifacts
            .iter()
            .filter(|a| a.owner == owner && a.repo == repo && a.issue == issue)
            .map(|a| a.number)
            .max()
            .unwrap_or(0)
            + 1;
        let now = now_secs();
        let actor = self.db.user_login.clone();
        let a = IssueArtifact {
            id,
            owner: owner.into(),
            repo: repo.into(),
            issue,
            number,
            kind: kind.into(),
            name: name.into(),
            body: body.into(),
            creator: actor.clone(),
            created_at: now,
            updated_at: now,
            versions: vec![ArtifactVersion {
                version: 1,
                name: name.into(),
                body: body.into(),
                created_at: now,
                actor,
            }],
        };
        self.db.issue_artifacts.push(a.clone());
        a
    }

    pub fn find_issue_artifact(
        &self,
        owner: &str,
        repo: &str,
        issue: u64,
        number: u64,
    ) -> Option<&IssueArtifact> {
        self.db
            .issue_artifacts
            .iter()
            .find(|a| a.owner == owner && a.repo == repo && a.issue == issue && a.number == number)
    }

    pub fn update_issue_artifact(
        &mut self,
        owner: &str,
        repo: &str,
        issue: u64,
        number: u64,
        name: Option<&str>,
        body: Option<&str>,
    ) -> Option<IssueArtifact> {
        let actor = self.db.user_login.clone();
        let a = self.db.issue_artifacts.iter_mut().find(|a| {
            a.owner == owner && a.repo == repo && a.issue == issue && a.number == number
        })?;
        if let Some(n) = name {
            a.name = n.into();
        }
        if let Some(b) = body {
            a.body = b.into();
        }
        let now = now_secs();
        a.updated_at = now;
        let version = a.versions.iter().map(|v| v.version).max().unwrap_or(0) + 1;
        a.versions.push(ArtifactVersion {
            version,
            name: a.name.clone(),
            body: a.body.clone(),
            created_at: now,
            actor,
        });
        Some(a.clone())
    }

    pub fn delete_issue_artifact(
        &mut self,
        owner: &str,
        repo: &str,
        issue: u64,
        number: u64,
    ) -> bool {
        let before = self.db.issue_artifacts.len();
        self.db.issue_artifacts.retain(|a| {
            !(a.owner == owner && a.repo == repo && a.issue == issue && a.number == number)
        });
        self.db.issue_artifacts.len() != before
    }

    pub fn add_linked_branch(
        &mut self,
        owner: &str,
        repo: &str,
        issue: u64,
        branch_repo: &str,
        name: &str,
    ) -> LinkedBranch {
        let id = self.db.next_linked_branch.max(1);
        self.db.next_linked_branch = id + 1;
        let lb = LinkedBranch {
            id,
            owner: owner.into(),
            repo: repo.into(),
            issue,
            branch_repo: branch_repo.into(),
            name: name.into(),
        };
        self.db.linked_branches.push(lb.clone());
        lb
    }
}
