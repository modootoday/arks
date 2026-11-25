//! Ruleset enforcement on push ([[sot:root/arks-hub-product-intent]] §17–22).
//!
//! The Git server of `arks serve` asks [`push_verdicts`] once per push, after the
//! pack is unpacked and before any ref moves. Each requested ref update is checked
//! against the active rulesets that target it; a refused update carries one
//! report-status reason. Rule meanings follow GitHub's documented ruleset rules
//! (<https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-rulesets/available-rules-for-rulesets>)
//! for the types the arks forge stores: `creation`, `update`, `deletion`,
//! `non_fast_forward`, `required_linear_history`, `pull_request` and
//! `required_status_checks`. Other rule types are not evaluated on push.

use super::git::repo_dir;
use super::rules::{Ruleset, applicable_ref};
use super::status;
use crate::store::HubStore;
use arks_object::{Commit, ObjectKind};
use arks_oid::Oid;
use arks_repo::Repository;
use base64::Engine;
use serde_json::{Value, json};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// One requested ref update (`None` = the all-zero id: create / delete).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PushRefUpdate {
    pub name: String,
    pub old: Option<Oid>,
    pub new: Option<Oid>,
}

/// Commit-graph facts the rules need about one update.
pub(crate) trait History {
    /// Is `old` an ancestor of (or equal to) `new`?
    fn is_fast_forward(&self, old: &Oid, new: &Oid) -> bool;
    /// Does `new` bring a merge commit not already reachable from `old`
    /// (or, on create, from any existing ref)?
    fn introduces_merge(&self, old: Option<&Oid>, new: &Oid) -> bool;
}

/// Prefix of every refusal; the details follow in parentheses.
pub(crate) const DECLINED: &str = "push declined due to repository rule violations";

/// Verdict per update for a push into `git_dir` (`Some(reason)` refuses the ref).
///
/// `data_dir` is the forge data directory and `git_root` the forge git root
/// (default `<data_dir>/git`). The pusher is resolved from the forge token in
/// `authorization` (HTTP Basic password or `Bearer`), else `verified_user`.
pub fn push_verdicts(
    data_dir: &Path,
    git_root: Option<&Path>,
    git_dir: &Path,
    verified_user: Option<&str>,
    authorization: Option<&str>,
    updates: &[PushRefUpdate],
) -> Vec<Option<String>> {
    let st = match load_store(data_dir) {
        Ok(Some(mut st)) => {
            if let Some(root) = git_root {
                st.git_root = root.to_path_buf();
            }
            st
        }
        Ok(None) => return vec![None; updates.len()],
        Err(e) => {
            let why = format!("{DECLINED} (forge ruleset store unreadable: {e})");
            return vec![Some(why); updates.len()];
        }
    };
    let Some((owner, repo)) = forge_repo_for(&st, git_dir) else {
        return vec![None; updates.len()];
    };
    let login = pusher_login(&st, verified_user, authorization);
    let history = RepoHistory::open(git_dir);
    updates
        .iter()
        .map(|u| refusal(&st, &owner, &repo, login.as_deref(), u, &history))
        .collect()
}

/// Read the store; one retry covers a concurrent rewrite by the forge API thread.
fn load_store(data_dir: &Path) -> Result<Option<HubStore>, String> {
    match HubStore::load_existing(data_dir) {
        Ok(v) => Ok(v),
        Err(_) => {
            std::thread::sleep(std::time::Duration::from_millis(50));
            HubStore::load_existing(data_dir)
        }
    }
}

fn canonical(p: &Path) -> PathBuf {
    p.canonicalize().unwrap_or_else(|_| p.to_path_buf())
}

/// The forge repository whose Git directory is `git_dir`.
fn forge_repo_for(st: &HubStore, git_dir: &Path) -> Option<(String, String)> {
    let want = canonical(git_dir);
    st.db.repos.iter().find_map(|r| {
        let dir = repo_dir(&st.git_root, &r.owner, &r.name)?;
        (canonical(&dir) == want).then(|| (r.owner.clone(), r.name.clone()))
    })
}

/// Forge account behind the push credentials, if any.
pub(crate) fn pusher_login(
    st: &HubStore,
    verified_user: Option<&str>,
    authorization: Option<&str>,
) -> Option<String> {
    let token = authorization.map(str::trim).and_then(|a| {
        let (scheme, rest) = a.split_once(' ')?;
        let rest = rest.trim();
        match scheme.to_ascii_lowercase().as_str() {
            "basic" => {
                let raw = base64::engine::general_purpose::STANDARD
                    .decode(rest)
                    .ok()?;
                let pair = String::from_utf8(raw).ok()?;
                pair.split_once(':').map(|(_, p)| p.to_string())
            }
            "bearer" | "token" => Some(rest.to_string()),
            _ => None,
        }
    });
    if let Some(t) = token.filter(|t| st.auth_ok(Some(t))) {
        return Some(st.login_for(Some(&t)));
    }
    verified_user.map(str::to_string)
}

/// Does `login` bypass ruleset `rs` on a push into `repo_owner`'s repository?
fn bypasses(rs: &Ruleset, repo_owner: &str, login: Option<&str>) -> bool {
    let Some(login) = login else {
        return false;
    };
    rs.bypass_actors.iter().any(|a| {
        let mode = a
            .get("bypass_mode")
            .and_then(Value::as_str)
            .unwrap_or("always");
        if !matches!(mode, "always" | "exempt") {
            return false;
        }
        match a.get("actor_type").and_then(Value::as_str) {
            // No collaborators on the arks forge: the owner holds every role.
            Some("OrganizationAdmin" | "RepositoryRole") => login == repo_owner,
            _ => false,
        }
    })
}

/// Rule violations of one update (empty = allowed).
pub(crate) fn violations(
    st: &HubStore,
    owner: &str,
    repo: &str,
    login: Option<&str>,
    u: &PushRefUpdate,
    h: &dyn History,
) -> Vec<String> {
    let mut why = Vec::new();
    for rs in applicable_ref(st, owner, repo, &u.name) {
        if rs.enforcement != "active" || bypasses(&rs, owner, login) {
            continue;
        }
        for rule in &rs.rules {
            let params = rule.get("parameters").cloned().unwrap_or(json!({}));
            let kind = rule.get("type").and_then(Value::as_str).unwrap_or("");
            if let Some(msg) = rule_violation(st, owner, repo, kind, &params, u, h) {
                why.push(format!("ruleset {:?}: {msg}", rs.name));
            }
        }
    }
    why
}

fn rule_violation(
    st: &HubStore,
    owner: &str,
    repo: &str,
    kind: &str,
    params: &Value,
    u: &PushRefUpdate,
    h: &dyn History,
) -> Option<String> {
    let Some(new) = u.new.as_ref() else {
        return (kind == "deletion").then(|| "cannot delete this ref".to_string());
    };
    let old = u.old.as_ref();
    match kind {
        "creation" if old.is_none() => Some("cannot create this ref".into()),
        "update" if old.is_some() => Some("cannot update this ref".into()),
        "non_fast_forward" => match old {
            Some(o) if !h.is_fast_forward(o, new) => Some("cannot force-push to this ref".into()),
            _ => None,
        },
        "required_linear_history" if h.introduces_merge(old, new) => {
            Some("merge commits are not allowed (linear history required)".into())
        }
        "pull_request" if old.is_some() => {
            Some("changes must be made through a pull request".into())
        }
        "required_status_checks" => {
            let skip_create = params
                .get("do_not_enforce_on_create")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            if old.is_none() && skip_create {
                return None;
            }
            let sha = new.to_hex();
            let latest = status::latest(st, owner, repo, &sha);
            let missing: Vec<String> = params
                .get("required_status_checks")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|c| c.get("context").and_then(Value::as_str))
                .filter(|ctx| {
                    !latest
                        .iter()
                        .any(|s| s.context == *ctx && s.state == "success")
                })
                .map(|ctx| format!("{ctx:?}"))
                .collect();
            (!missing.is_empty()).then(|| {
                format!(
                    "required status check {} has not passed for {}",
                    missing.join(", "),
                    &sha[..7.min(sha.len())]
                )
            })
        }
        _ => None,
    }
}

/// Report-status reason for one update, or `None` when it may proceed.
pub(crate) fn refusal(
    st: &HubStore,
    owner: &str,
    repo: &str,
    login: Option<&str>,
    u: &PushRefUpdate,
    h: &dyn History,
) -> Option<String> {
    let why = violations(st, owner, repo, login, u, h);
    (!why.is_empty()).then(|| format!("{DECLINED} ({})", why.join("; ")))
}

/// [`History`] over the pushed repository (objects already unpacked).
struct RepoHistory {
    repo: Option<Repository>,
}

impl RepoHistory {
    fn open(git_dir: &Path) -> Self {
        Self {
            repo: Repository::open(git_dir).ok(),
        }
    }

    /// Peel tags down to a commit.
    fn peel(&self, oid: &Oid) -> Option<Oid> {
        let repo = self.repo.as_ref()?;
        let mut cur = *oid;
        for _ in 0..8 {
            let obj = repo.get_object(&cur).ok()?;
            match obj.kind {
                ObjectKind::Commit => return Some(cur),
                ObjectKind::Tag => {
                    let text = String::from_utf8_lossy(&obj.payload).into_owned();
                    cur = text
                        .lines()
                        .find_map(|l| l.strip_prefix("object "))
                        .and_then(|h| Oid::from_hex(h.trim()).ok())?;
                }
                _ => return None,
            }
        }
        None
    }

    fn ancestors(&self, tip: &Oid) -> HashSet<Oid> {
        match (self.repo.as_ref(), self.peel(tip)) {
            (Some(repo), Some(c)) => arks_repo::ancestor_set(repo, c).unwrap_or_default(),
            _ => HashSet::new(),
        }
    }
}

impl History for RepoHistory {
    fn is_fast_forward(&self, old: &Oid, new: &Oid) -> bool {
        if old == new {
            return true;
        }
        match (self.peel(old), self.peel(new)) {
            (Some(o), Some(_)) => self.ancestors(new).contains(&o),
            _ => false,
        }
    }

    fn introduces_merge(&self, old: Option<&Oid>, new: &Oid) -> bool {
        let Some(repo) = self.repo.as_ref() else {
            return false;
        };
        let Some(start) = self.peel(new) else {
            return false;
        };
        let exclude: HashSet<Oid> = match old {
            Some(o) => self.ancestors(o),
            None => repo
                .refs()
                .list_refs("refs/")
                .unwrap_or_default()
                .iter()
                .flat_map(|(_, tip)| self.ancestors(tip))
                .collect(),
        };
        let mut seen = HashSet::new();
        let mut stack = vec![start];
        while let Some(oid) = stack.pop() {
            if exclude.contains(&oid) || !seen.insert(oid) {
                continue;
            }
            let Ok(obj) = repo.get_object(&oid) else {
                continue;
            };
            let Ok(c) = Commit::decode(&obj.payload) else {
                continue;
            };
            if c.parents.len() > 1 {
                return true;
            }
            stack.extend(c.parents);
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::git::{GitRepo, init_bare};
    use crate::forge::status::CommitStatus;
    use crate::store::now_secs;

    fn oid(n: u8) -> Oid {
        Oid::from_hex(&format!("{n:02x}").repeat(20)).unwrap()
    }

    /// Fast-forward iff `old` < `new` by first byte; merges iff `new` is 0xee.
    struct Fake;

    impl History for Fake {
        fn is_fast_forward(&self, old: &Oid, new: &Oid) -> bool {
            old.as_bytes()[0] <= new.as_bytes()[0]
        }
        fn introduces_merge(&self, _: Option<&Oid>, new: &Oid) -> bool {
            new.as_bytes()[0] == 0xee
        }
    }

    fn store(tag: &str) -> HubStore {
        let dir = std::env::temp_dir().join(format!(
            "arks-push-rules-{tag}-{}-{}",
            std::process::id(),
            now_secs()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        HubStore::open(&dir).unwrap()
    }

    fn ruleset(st: &mut HubStore, name: &str, include: &[&str], rules: Value, bypass: Value) {
        let id = st.db.rulesets.len() as u64 + 1;
        st.db.rulesets.push(
            serde_json::from_value(json!({
                "id": id,
                "owner": "local",
                "repo": "playground",
                "name": name,
                "include": include,
                "rules": rules,
                "bypass_actors": bypass,
            }))
            .unwrap(),
        );
    }

    fn upd(name: &str, old: Option<u8>, new: Option<u8>) -> PushRefUpdate {
        PushRefUpdate {
            name: name.into(),
            old: old.map(oid),
            new: new.map(oid),
        }
    }

    fn check(st: &HubStore, login: Option<&str>, u: &PushRefUpdate) -> Option<String> {
        refusal(st, "local", "playground", login, u, &Fake)
    }

    #[test]
    fn deletion_and_force_push_refused_fast_forward_allowed() {
        let mut st = store("basic");
        ruleset(
            &mut st,
            "protect main",
            &["~DEFAULT_BRANCH"],
            json!([{"type": "deletion"}, {"type": "non_fast_forward"}]),
            json!([]),
        );
        let del = check(&st, None, &upd("refs/heads/main", Some(1), None)).unwrap();
        assert_eq!(
            del,
            format!("{DECLINED} (ruleset \"protect main\": cannot delete this ref)")
        );
        let force = check(&st, None, &upd("refs/heads/main", Some(5), Some(2))).unwrap();
        assert!(force.contains("cannot force-push to this ref"), "{force}");
        assert_eq!(
            check(&st, None, &upd("refs/heads/main", Some(1), Some(2))),
            None
        );
        // Other branches are untouched by a default-branch ruleset.
        assert_eq!(
            check(&st, None, &upd("refs/heads/topic", Some(1), None)),
            None
        );
    }

    #[test]
    fn creation_update_and_pull_request_apply_by_kind() {
        let mut st = store("kinds");
        ruleset(
            &mut st,
            "rel",
            &["refs/heads/release/*"],
            json!([{"type": "creation"}]),
            json!([]),
        );
        ruleset(
            &mut st,
            "frozen",
            &["refs/heads/frozen"],
            json!([{"type": "update"}]),
            json!([]),
        );
        ruleset(
            &mut st,
            "pr",
            &["refs/heads/main"],
            json!([{"type": "pull_request", "parameters": {"required_approving_review_count": 1}}]),
            json!([]),
        );
        let c = check(&st, None, &upd("refs/heads/release/1.0", None, Some(1))).unwrap();
        assert!(c.contains("cannot create this ref"), "{c}");
        assert_eq!(
            check(&st, None, &upd("refs/heads/release/1.0", Some(1), Some(2))),
            None
        );
        let f = check(&st, None, &upd("refs/heads/frozen", Some(1), Some(2))).unwrap();
        assert!(f.contains("cannot update this ref"), "{f}");
        let p = check(&st, None, &upd("refs/heads/main", Some(1), Some(2))).unwrap();
        assert!(p.contains("through a pull request"), "{p}");
        // Deletion is governed by `deletion` only.
        assert_eq!(
            check(&st, None, &upd("refs/heads/main", Some(1), None)),
            None
        );
    }

    #[test]
    fn linear_history_status_checks_and_tags() {
        let mut st = store("linear");
        ruleset(
            &mut st,
            "lin",
            &["~ALL"],
            json!([
                {"type": "required_linear_history"},
                {"type": "required_status_checks", "parameters": {
                    "do_not_enforce_on_create": true,
                    "required_status_checks": [{"context": "ci"}]}}
            ]),
            json!([]),
        );
        let m = check(&st, None, &upd("refs/heads/main", Some(1), Some(0xee))).unwrap();
        assert!(m.contains("merge commits are not allowed"), "{m}");
        let s = check(&st, None, &upd("refs/heads/main", Some(1), Some(2))).unwrap();
        assert!(
            s.contains("required status check \"ci\" has not passed"),
            "{s}"
        );
        // Creation is exempt with do_not_enforce_on_create.
        assert_eq!(
            check(&st, None, &upd("refs/heads/new", None, Some(2))),
            None
        );
        st.db.statuses.push(CommitStatus {
            id: 1,
            owner: "local".into(),
            repo: "playground".into(),
            sha: oid(2).to_hex(),
            state: "success".into(),
            context: "ci".into(),
            description: String::new(),
            target_url: String::new(),
            creator: String::new(),
            created_at: 0,
        });
        assert_eq!(
            check(&st, None, &upd("refs/heads/main", Some(1), Some(2))),
            None
        );
        // A branch ruleset never applies to tags or other refs.
        assert_eq!(check(&st, None, &upd("refs/tags/v1", Some(1), None)), None);
        assert_eq!(
            check(&st, None, &upd("refs/notes/x", Some(5), Some(1))),
            None
        );
    }

    #[test]
    fn tag_rulesets_target_tags() {
        let mut st = store("tags");
        st.db.rulesets.push(
            serde_json::from_value(json!({
                "id": 1, "owner": "local", "repo": "playground", "name": "tags",
                "target": "tag", "include": ["refs/tags/v*"],
                "rules": [{"type": "deletion"}, {"type": "update"}],
            }))
            .unwrap(),
        );
        assert!(check(&st, None, &upd("refs/tags/v1", Some(1), None)).is_some());
        assert!(check(&st, None, &upd("refs/tags/v1", Some(1), Some(2))).is_some());
        assert_eq!(check(&st, None, &upd("refs/heads/v1", Some(1), None)), None);
    }

    #[test]
    fn enforcement_evaluate_and_disabled_do_not_block() {
        let mut st = store("enf");
        for (i, e) in ["evaluate", "disabled"].iter().enumerate() {
            st.db.rulesets.push(
                serde_json::from_value(json!({
                    "id": i + 1, "owner": "local", "repo": "playground", "name": e,
                    "enforcement": e, "include": ["~ALL"], "rules": [{"type": "deletion"}],
                }))
                .unwrap(),
            );
        }
        assert_eq!(
            check(&st, None, &upd("refs/heads/main", Some(1), None)),
            None
        );
    }

    #[test]
    fn bypass_actors_honoured_by_mode_and_type() {
        let mut st = store("bypass");
        ruleset(
            &mut st,
            "always",
            &["refs/heads/a"],
            json!([{"type": "deletion"}]),
            json!([{"actor_id": 5, "actor_type": "RepositoryRole", "bypass_mode": "always"}]),
        );
        ruleset(
            &mut st,
            "pr-only",
            &["refs/heads/b"],
            json!([{"type": "deletion"}]),
            json!([{"actor_id": 1, "actor_type": "OrganizationAdmin", "bypass_mode": "pull_request"}]),
        );
        ruleset(
            &mut st,
            "team",
            &["refs/heads/c"],
            json!([{"type": "deletion"}]),
            json!([{"actor_id": 9, "actor_type": "Team", "bypass_mode": "always"}]),
        );
        let del = |b: &str| upd(&format!("refs/heads/{b}"), Some(1), None);
        // The repository owner bypasses an `always` role actor; others do not.
        assert_eq!(check(&st, Some("local"), &del("a")), None);
        assert!(check(&st, Some("someone"), &del("a")).is_some());
        assert!(check(&st, None, &del("a")).is_some());
        // `pull_request` mode never bypasses a push; Team never resolves.
        assert!(check(&st, Some("local"), &del("b")).is_some());
        assert!(check(&st, Some("local"), &del("c")).is_some());
    }

    #[test]
    fn pusher_login_from_forge_token() {
        let mut st = store("login");
        st.db.tokens.push("tok-b".into());
        st.db.token_logins.insert("tok-b".into(), "bob".into());
        let basic = format!(
            "Basic {}",
            base64::engine::general_purpose::STANDARD.encode("x-access-token:tok-b")
        );
        assert_eq!(
            pusher_login(&st, None, Some(&basic)).as_deref(),
            Some("bob")
        );
        assert_eq!(
            pusher_login(&st, None, Some("Bearer tok-b")).as_deref(),
            Some("bob")
        );
        assert_eq!(pusher_login(&st, None, Some("Bearer nope")), None);
        assert_eq!(
            pusher_login(&st, Some("carol"), Some("Bearer nope")).as_deref(),
            Some("carol")
        );
    }

    #[test]
    fn repo_history_and_push_verdicts_over_a_forge_repository() {
        let st = store("repo");
        let data = st.data_root();
        let git = init_bare(&st.git_root, "local", "playground", "main").unwrap();
        let mut g = GitRepo::open(&git).unwrap();
        let blob = g.write_blob(b"x\n").unwrap();
        let mut flat = crate::forge::git::Flat::new();
        flat.insert("x".into(), (arks_object::TreeMode::File, blob));
        let tree = g.write_flat(&flat).unwrap();
        let a = g.write_commit(tree, vec![], "a", "t", 1).unwrap();
        let b = g.write_commit(tree, vec![a], "b", "t", 2).unwrap();
        let side = g.write_commit(tree, vec![a], "side", "t", 3).unwrap();
        let merge = g
            .write_commit(tree, vec![b, side], "merge", "t", 4)
            .unwrap();
        g.set_branch("main", &b).unwrap();

        let h = RepoHistory::open(&git);
        assert!(h.is_fast_forward(&a, &b));
        assert!(!h.is_fast_forward(&b, &side));
        assert!(h.introduces_merge(Some(&b), &merge));
        assert!(!h.introduces_merge(Some(&a), &b));
        assert!(!h.introduces_merge(Some(&merge), &merge));

        let mut st = st;
        ruleset(
            &mut st,
            "main",
            &["~DEFAULT_BRANCH"],
            json!([{"type": "deletion"}, {"type": "non_fast_forward"}]),
            json!([]),
        );
        st.save().unwrap();
        let ups = [
            PushRefUpdate {
                name: "refs/heads/main".into(),
                old: Some(b),
                new: Some(side),
            },
            PushRefUpdate {
                name: "refs/heads/main".into(),
                old: Some(b),
                new: None,
            },
            PushRefUpdate {
                name: "refs/heads/main".into(),
                old: Some(b),
                new: Some(merge),
            },
            PushRefUpdate {
                name: "refs/heads/topic".into(),
                old: None,
                new: Some(side),
            },
        ];
        let v = push_verdicts(&data, None, &git, None, None, &ups);
        assert!(v[0].as_deref().unwrap().contains("force-push"), "{v:?}");
        assert!(v[1].as_deref().unwrap().contains("delete"), "{v:?}");
        assert_eq!(v[2], None);
        assert_eq!(v[3], None);
        // A Git directory that is no forge repository has no rulesets.
        let other = init_bare(&st.git_root, "elsewhere", "x", "main").unwrap();
        assert_eq!(
            push_verdicts(&data, None, &other, None, None, &ups[1..2]),
            vec![None]
        );
        // An unreadable store fails closed.
        std::fs::write(data.join("db.json"), b"{not json").unwrap();
        let v = push_verdicts(&data, None, &git, None, None, &ups[2..3]);
        assert!(v[0].as_deref().unwrap().contains("unreadable"), "{v:?}");
        let _ = std::fs::remove_dir_all(&data);
    }
}
