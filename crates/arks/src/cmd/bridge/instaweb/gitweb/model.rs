//! Repository reads behind the instaweb pages: the project's metadata,
//! revisions, the commit walk, refs, trees and per-commit changes.

use crate::cmd::shared::{peel_tags, resolve_rev, tag_target, tree_entry_at};
use arks_config::ConfigSet;
use arks_object::{Commit, ObjectKind, Signature, Tree, TreeEntry};
use arks_oid::Oid;
use arks_repo::Repository;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::fs;

/// One open project for the length of a request.
pub struct Ctx {
    pub repo: Repository,
    pub name: String,
    pub cfg: ConfigSet,
    /// The request's `Host` header, when it is a plain host[:port].
    pub host: Option<String>,
}

/// A ref row for the heads and tags lists.
pub struct RefRow {
    /// Short name (`main`, `v1`).
    pub name: String,
    /// The object the ref names.
    pub oid: Oid,
    /// Whether that object is an annotated tag.
    pub annotated: bool,
    /// The commit it peels to, with that commit.
    pub commit: Option<(Oid, Commit)>,
    /// The annotated tag's first message line.
    pub tag_subject: Option<String>,
}

/// One changed path between two commits.
pub struct Change {
    pub path: String,
    pub old: Option<Oid>,
    pub new: Option<Oid>,
}

impl Ctx {
    /// The project description: `$GIT_DIR/description`, else
    /// `gitweb.description`, else the documented placeholder.
    pub fn description(&self) -> String {
        fs::read_to_string(self.repo.git_dir().join("description"))
            .ok()
            .map(|s| s.lines().next().unwrap_or("").trim().to_string())
            .filter(|s| !s.is_empty())
            .or_else(|| self.cfg.get("gitweb.description"))
            .unwrap_or_else(|| "Unnamed repository; edit this file to name it for gitweb.".into())
    }

    /// `gitweb.owner`, when set.
    pub fn owner(&self) -> Option<String> {
        self.cfg.get("gitweb.owner")
    }

    /// Clone URLs: `$GIT_DIR/cloneurl` lines, else every `gitweb.url`.
    pub fn clone_urls(&self) -> Vec<String> {
        if let Ok(s) = fs::read_to_string(self.repo.git_dir().join("cloneurl")) {
            return s
                .lines()
                .map(str::trim)
                .filter(|l| !l.is_empty())
                .map(str::to_string)
                .collect();
        }
        self.cfg.get_all("gitweb.url")
    }

    /// Resolve a revision given in a URL; option-like, index (`:path`),
    /// range and whitespace-bearing names are refused.
    pub fn resolve(&self, rev: &str) -> Option<Oid> {
        if rev.is_empty()
            || rev.starts_with('-')
            || rev.starts_with(':')
            || rev.contains("..")
            || rev.chars().any(|c| c.is_whitespace() || c.is_control())
        {
            return None;
        }
        resolve_rev(&self.repo, rev).ok()
    }

    /// The commit a revision peels to.
    pub fn resolve_commit(&self, rev: &str) -> Option<(Oid, Commit)> {
        let oid = peel_tags(&self.repo, &self.resolve(rev)?).ok()?;
        Some((oid, self.commit(&oid)?))
    }

    pub fn kind(&self, oid: &Oid) -> Option<ObjectKind> {
        self.repo.get_object(oid).ok().map(|o| o.kind)
    }

    pub fn commit(&self, oid: &Oid) -> Option<Commit> {
        let obj = self.repo.get_object(oid).ok()?;
        (obj.kind == ObjectKind::Commit)
            .then(|| Commit::decode(&obj.payload).ok())
            .flatten()
    }

    pub fn blob(&self, oid: &Oid) -> Option<Vec<u8>> {
        let obj = self.repo.get_object(oid).ok()?;
        (obj.kind == ObjectKind::Blob).then_some(obj.payload)
    }

    pub fn tree(&self, oid: &Oid) -> Option<Vec<TreeEntry>> {
        let obj = self.repo.get_object(oid).ok()?;
        (obj.kind == ObjectKind::Tree)
            .then(|| Tree::decode(&obj.payload, self.repo.oid_kind()).ok())
            .flatten()
            .map(|t| t.entries)
    }

    /// The object at `path` in `commit`'s tree (empty path: the root tree).
    pub fn at_path(&self, commit: &Commit, path: &str) -> Option<Oid> {
        tree_entry_at(&self.repo, &commit.tree, path)
    }

    /// Commits reachable from `tip`, newest committer date first (git's
    /// default walk order), skipping `skip` and keeping at most `limit`.
    pub fn walk(&self, tip: Oid, skip: usize, limit: usize) -> Vec<(Oid, Commit)> {
        // Heap keys: (committer time, insertion order reversed) so equal
        // times come out first-in first-out; the commits wait in `queued`.
        let mut heap = BinaryHeap::new();
        let mut queued = HashMap::new();
        let mut seen = HashSet::new();
        let mut seq = 0u64;
        let mut out = Vec::new();
        if let Some(c) = self.commit(&tip) {
            seen.insert(tip);
            heap.push((c.committer.timestamp, u64::MAX));
            queued.insert(u64::MAX, (tip, c));
        }
        let mut n = 0;
        while let Some((_, key)) = heap.pop() {
            let Some((oid, c)) = queued.remove(&key) else {
                continue;
            };
            for p in &c.parents {
                if seen.insert(*p) {
                    if let Some(pc) = self.commit(p) {
                        seq += 1;
                        heap.push((pc.committer.timestamp, u64::MAX - seq));
                        queued.insert(u64::MAX - seq, (*p, pc));
                    }
                }
            }
            if n >= skip {
                out.push((oid, c));
                if out.len() >= limit {
                    break;
                }
            }
            n += 1;
        }
        out
    }

    /// Refs under `refs/<kind>/`, newest commit first.
    pub fn refs(&self, kind: &str) -> Vec<RefRow> {
        let prefix = format!("refs/{kind}/");
        let mut rows: Vec<RefRow> = self
            .repo
            .refs()
            .list_refs(&prefix)
            .unwrap_or_default()
            .into_iter()
            .map(|(name, oid)| {
                let obj = self.repo.get_object(&oid).ok();
                let annotated = obj.as_ref().is_some_and(|o| o.kind == ObjectKind::Tag);
                let tag_subject = obj
                    .filter(|o| o.kind == ObjectKind::Tag)
                    .and_then(|o| tag_message(&o.payload))
                    .map(|m| m.lines().next().unwrap_or("").to_string());
                let commit = peel_tags(&self.repo, &oid)
                    .ok()
                    .and_then(|c| Some((c, self.commit(&c)?)));
                RefRow {
                    name: name[prefix.len()..].to_string(),
                    oid,
                    annotated,
                    commit,
                    tag_subject,
                }
            })
            .collect();
        rows.sort_by(|a, b| {
            let t = |r: &RefRow| r.commit.as_ref().map(|(_, c)| c.committer.timestamp);
            t(b).cmp(&t(a)).then_with(|| a.name.cmp(&b.name))
        });
        rows
    }

    /// The newest committer date across the branches.
    pub fn last_change(&self) -> Option<Signature> {
        self.refs("heads")
            .into_iter()
            .filter_map(|r| r.commit.map(|(_, c)| c.committer))
            .max_by_key(|s| s.timestamp)
    }

    /// Paths that differ between `parent` (none: the empty tree) and `commit`.
    pub fn changes(&self, parent: Option<&Commit>, commit: &Commit) -> Vec<Change> {
        let map = |c: Option<&Commit>| {
            let mut m = HashMap::new();
            if let Some(c) = c {
                let _ = arks_repo::flatten_tree(&self.repo, &c.tree, "", &mut m);
            }
            m
        };
        let (old, new) = (map(parent), map(Some(commit)));
        let mut paths: Vec<&String> = old.keys().chain(new.keys()).collect();
        paths.sort();
        paths.dedup();
        paths
            .into_iter()
            .filter(|p| old.get(*p) != new.get(*p))
            .map(|p| Change {
                path: p.clone(),
                old: old.get(p).copied(),
                new: new.get(p).copied(),
            })
            .collect()
    }

    /// Short object name for display.
    pub fn short(&self, oid: &Oid) -> String {
        crate::cmd::shared::abbrev_hex(&self.repo, oid, 7)
    }
}

/// Parsed annotated tag headers and message.
pub struct TagInfo {
    pub object: Option<Oid>,
    pub kind: String,
    pub name: String,
    pub tagger: Option<Signature>,
    pub message: String,
}

/// Decode an annotated tag payload (gitformat object `tag` headers).
pub fn parse_tag(payload: &[u8]) -> TagInfo {
    let text = String::from_utf8_lossy(payload);
    let (head, message) = text.split_once("\n\n").unwrap_or((&text, ""));
    let mut info = TagInfo {
        object: tag_target(payload),
        kind: String::new(),
        name: String::new(),
        tagger: None,
        message: strip_signature(message).to_string(),
    };
    for line in head.lines() {
        if let Some(v) = line.strip_prefix("type ") {
            info.kind = v.to_string();
        } else if let Some(v) = line.strip_prefix("tag ") {
            info.name = v.to_string();
        } else if let Some(v) = line.strip_prefix("tagger ") {
            info.tagger = parse_signature(v);
        }
    }
    info
}

fn tag_message(payload: &[u8]) -> Option<String> {
    let m = parse_tag(payload).message;
    (!m.trim().is_empty()).then_some(m)
}

fn strip_signature(message: &str) -> &str {
    message
        .find("-----BEGIN ")
        .map(|i| &message[..i])
        .unwrap_or(message)
}

/// `Name <email> <ts> <tz>`.
pub fn parse_signature(v: &str) -> Option<Signature> {
    let lt = v.find('<')?;
    let gt = v[lt..].find('>')? + lt;
    let mut rest = v[gt + 1..].split_whitespace();
    Some(Signature {
        name: v[..lt].trim().to_string(),
        email: v[lt + 1..gt].to_string(),
        timestamp: rest.next()?.parse().ok()?,
        timezone: rest.next().unwrap_or("+0000").to_string(),
    })
}
