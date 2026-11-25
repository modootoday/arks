//! Push policy seam for receive-pack: a host (the `arks serve` forge module) may
//! refuse individual ref updates before any ref moves.
//!
//! The server stays forge-agnostic: it hands the policy the repository, who is
//! pushing (as far as the transport knows) and the requested ref updates, and
//! reports each refusal as `ng <ref> <reason>` in report-status, so the client
//! prints `! [remote rejected] <ref> (<reason>)`. Cite:
//! <https://git-scm.com/docs/gitprotocol-pack> (report-status),
//! <https://git-scm.com/docs/git-push> (OUTPUT).

use arks_oid::Oid;
use arks_repo::Repository;
use std::sync::{Arc, OnceLock, RwLock};

/// One requested ref update; an all-zero `old` creates, an all-zero `new` deletes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefUpdate {
    pub name: String,
    pub old: Oid,
    pub new: Oid,
}

impl RefUpdate {
    pub fn is_create(&self) -> bool {
        self.old.as_bytes().iter().all(|b| *b == 0)
    }

    pub fn is_delete(&self) -> bool {
        self.new.as_bytes().iter().all(|b| *b == 0)
    }
}

/// What the transport knows about the pusher.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Pusher {
    /// `http`, `ssh` or `local`.
    pub transport: &'static str,
    /// User verified by the server's own HTTP Basic gate, if one is configured.
    pub verified_user: Option<String>,
    /// Raw `Authorization` header sent with the push (HTTP only).
    pub authorization: Option<String>,
}

impl Pusher {
    pub fn local() -> Self {
        Self {
            transport: "local",
            ..Self::default()
        }
    }

    pub fn ssh() -> Self {
        Self {
            transport: "ssh",
            ..Self::default()
        }
    }
}

/// Decides, per ref update, whether a push may proceed.
pub trait PushPolicy: Send + Sync {
    /// One entry per update, in order: `Some(reason)` refuses that ref.
    fn check(
        &self,
        repo: &Repository,
        pusher: &Pusher,
        updates: &[RefUpdate],
    ) -> Vec<Option<String>>;
}

fn slot() -> &'static RwLock<Option<Arc<dyn PushPolicy>>> {
    static SLOT: OnceLock<RwLock<Option<Arc<dyn PushPolicy>>>> = OnceLock::new();
    SLOT.get_or_init(|| RwLock::new(None))
}

/// Install (or clear) the process-wide policy used by the serving receive-pack paths.
pub fn install_push_policy(policy: Option<Arc<dyn PushPolicy>>) {
    if let Ok(mut g) = slot().write() {
        *g = policy;
    }
}

/// The installed policy, if any.
pub fn installed_push_policy() -> Option<Arc<dyn PushPolicy>> {
    slot().read().ok().and_then(|g| g.clone())
}

/// Run `policy` and normalise its answer to one verdict per update (fail closed).
pub(crate) fn verdicts(
    policy: Option<&dyn PushPolicy>,
    repo: &Repository,
    pusher: &Pusher,
    updates: &[RefUpdate],
) -> Vec<Option<String>> {
    let Some(policy) = policy else {
        return vec![None; updates.len()];
    };
    let out = policy.check(repo, pusher, updates);
    if out.len() != updates.len() {
        return vec![Some("push policy error".into()); updates.len()];
    }
    out.into_iter()
        .map(|v| v.map(|r| r.replace(['\n', '\r'], " ")))
        .collect()
}
