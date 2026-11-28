//! Atomic JSON-backed local forge store (`<git_dir>/arks-forge/store.json`) for
//! self-hosted Issues, PR review comments, and form parsing.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ForgeStore {
    pub next_id: u64,
    pub issues: Vec<ForgeIssue>,
    #[serde(default)]
    pub pr_comments: Vec<PrComment>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ForgeIssue {
    pub id: u64,
    pub title: String,
    pub body: String,
    pub state: String,
    pub author: String,
    pub created_at: i64,
    #[serde(default)]
    pub comments: Vec<ForgeComment>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ForgeComment {
    pub author: String,
    pub body: String,
    pub created_at: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrComment {
    pub branch: String,
    pub author: String,
    pub body: String,
    pub created_at: i64,
}

fn store_path(git_dir: &Path) -> PathBuf {
    git_dir.join("arks-forge").join("store.json")
}

pub fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

impl ForgeStore {
    pub fn load(git_dir: &Path) -> Self {
        let path = store_path(git_dir);
        let Ok(raw) = fs::read_to_string(&path) else {
            return Self {
                next_id: 1,
                issues: Vec::new(),
                pr_comments: Vec::new(),
            };
        };
        let mut s: Self = serde_json::from_str(&raw).unwrap_or_default();
        if s.next_id == 0 {
            s.next_id = s.issues.iter().map(|i| i.id).max().unwrap_or(0) + 1;
        }
        s
    }

    pub fn save(&self, git_dir: &Path) -> std::io::Result<()> {
        let path = store_path(git_dir);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let tmp = path.with_extension("json.tmp");
        let data = serde_json::to_vec_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        fs::write(&tmp, data)?;
        fs::rename(&tmp, &path)?;
        Ok(())
    }

    pub fn create_issue(&mut self, title: String, body: String, author: String) -> u64 {
        let id = self.next_id.max(1);
        self.next_id = id + 1;
        self.issues.push(ForgeIssue {
            id,
            title,
            body,
            state: "open".into(),
            author,
            created_at: now_unix(),
            comments: Vec::new(),
        });
        id
    }

    pub fn add_issue_comment(&mut self, id: u64, author: String, body: String) -> bool {
        let Some(issue) = self.issues.iter_mut().find(|i| i.id == id) else {
            return false;
        };
        issue.comments.push(ForgeComment {
            author,
            body,
            created_at: now_unix(),
        });
        true
    }

    pub fn toggle_issue_state(&mut self, id: u64) -> Option<&str> {
        let issue = self.issues.iter_mut().find(|i| i.id == id)?;
        issue.state = if issue.state == "open" {
            "closed".into()
        } else {
            "open".into()
        };
        Some(issue.state.as_str())
    }

    pub fn add_pr_comment(&mut self, branch: String, author: String, body: String) {
        self.pr_comments.push(PrComment {
            branch,
            author,
            body,
            created_at: now_unix(),
        });
    }
}

/// Parse `application/x-www-form-urlencoded` body into key-value map.
pub fn parse_form_urlencoded(body: &[u8]) -> BTreeMap<String, String> {
    let text = String::from_utf8_lossy(body);
    let mut map = BTreeMap::new();
    for pair in text.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            map.insert(percent_decode(k), percent_decode(v));
        }
    }
    map
}

fn percent_decode(input: &str) -> String {
    let mut bytes = Vec::with_capacity(input.len());
    let raw = input.as_bytes();
    let mut i = 0;
    while i < raw.len() {
        if raw[i] == b'+' {
            bytes.push(b' ');
            i += 1;
        } else if raw[i] == b'%' && i + 2 < raw.len() {
            if let Ok(val) = u8::from_str_radix(&input[i + 1..i + 3], 16) {
                bytes.push(val);
                i += 3;
                continue;
            }
            bytes.push(b'%');
            i += 1;
        } else {
            bytes.push(raw[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&bytes).into_owned()
}
