//! Host-advisory unit tests for L1/L2/dual-path (G11 is authoritative).

use super::*;
use arks_oid::Oid;
use arks_refs::{FilesystemRefStore, RefsError};
use std::time::Duration;
use tempfile::TempDir;

#[test]
fn l1_grant_release_inspect_list() {
    let mut m = LeaseManager::new();
    let id = refns("refs/heads/main");
    let rec = m.grant(&id, "agent-a", Duration::from_secs(60)).unwrap();
    assert_eq!(rec.holder, "agent-a");
    assert!(m.inspect(&id).is_some());
    assert_eq!(m.list().len(), 1);
    m.release(&id, "agent-a").unwrap();
    assert!(m.inspect(&id).is_none());
}

#[test]
fn cd1_resource_prefix_helpers() {
    assert_eq!(refns("refs/heads/main"), "refns:refs/heads/main");
    assert_eq!(refns("refns:refs/heads/main"), "refns:refs/heads/main");
    assert_eq!(index_scope("default"), "index:default");
    assert_eq!(domain_worktree("common"), "domain:worktree:common");
    assert_eq!(domain_submodule("deps/sub"), "domain:submodule:deps/sub");
    assert!(is_typed("refns:refs/heads/a/"));
    assert!(is_typed("index:default"));
    assert!(is_typed("domain:worktree:common"));
    assert!(is_typed("domain:submodule:deps/sub"));
    assert_eq!(hook_scope("critical"), "hook:critical");
    assert_eq!(maint_scope("gc"), "maint:gc");
    assert!(is_typed("hook:critical"));
    assert!(is_typed("maint:gc"));
    assert!(!is_typed("opaque"));
}

#[test]
fn l1_conflict_and_renew_same_holder() {
    let mut m = LeaseManager::new();
    m.grant("r1", "a", Duration::from_secs(60)).unwrap();
    let err = m.grant("r1", "b", Duration::from_secs(60)).unwrap_err();
    assert!(matches!(err, LeaseError::AlreadyHeld { .. }));
    m.grant("r1", "a", Duration::from_secs(120)).unwrap();
    assert_eq!(m.inspect("r1").unwrap().holder, "a");
}

#[test]
fn l1_expire_due() {
    let mut m = LeaseManager::new();
    m.grant_at("r1", "a", Duration::from_millis(10), 1_000)
        .unwrap();
    assert!(m.expire_due_at(1_000).is_empty());
    let expired = m.expire_due_at(1_010);
    assert_eq!(expired.len(), 1);
    assert!(m.inspect("r1").is_none());
}

#[test]
fn l2_durable_roundtrip() {
    let dir = TempDir::new().unwrap();
    let git = dir.path().join(".git");
    std::fs::create_dir_all(&git).unwrap();
    {
        let mut store = DurableLeaseStore::open(&git).unwrap();
        store
            .manager_mut()
            .grant("res", "h1", Duration::from_secs(30))
            .unwrap();
        store.save().unwrap();
        assert!(store.path().exists());
    }
    let store2 = DurableLeaseStore::open(&git).unwrap();
    let rec = store2.inspect_via_manager("res");
    assert_eq!(rec.unwrap().holder, "h1");
}

trait InspectHelper {
    fn inspect_via_manager(&self, id: &str) -> Option<LeaseRecord>;
}
impl InspectHelper for DurableLeaseStore {
    fn inspect_via_manager(&self, id: &str) -> Option<LeaseRecord> {
        self.manager().inspect(id).cloned()
    }
}

#[test]
fn dual_path_gitcompat_default_high_concurrency_opt_in() {
    let s = WriterSession::default();
    assert_eq!(s.mode(), WriterMode::GitCompat);
    // M23 unlock: HighConcurrency selects claimed shape (still opt-in, not default).
    let hc = WriterSession::new(WriterMode::HighConcurrency).unwrap();
    assert_eq!(hc.mode(), WriterMode::HighConcurrency);
}

#[test]
fn lease_held_still_honors_git_ref_lock() {
    // Prove lease ≠ skip Git locks: hold a product lease, then show that
    // FilesystemRefStore update still fails when a competing *.lock exists.
    let dir = TempDir::new().unwrap();
    let git = dir.path().join(".git");
    std::fs::create_dir_all(git.join("refs/heads")).unwrap();
    std::fs::write(git.join("HEAD"), b"ref: refs/heads/main\n").unwrap();
    let oid = Oid::from_hex("1111111111111111111111111111111111111111").unwrap();
    std::fs::write(
        git.join("refs/heads/main"),
        format!("{}\n", oid.to_hex()).as_bytes(),
    )
    .unwrap();

    let mut leases = LeaseManager::new();
    leases
        .grant("refs/heads/main", "agent-a", Duration::from_secs(60))
        .unwrap();
    assert!(leases.inspect("refs/heads/main").is_some());

    // Competing Git lock present — update must fail even though lease is held.
    let lock_path = git.join("refs/heads/main.lock");
    std::fs::write(&lock_path, b"").unwrap();

    let mut refs = FilesystemRefStore::open(&git).unwrap();
    let new = Oid::from_hex("2222222222222222222222222222222222222222").unwrap();
    let err = refs
        .update("refs/heads/main", &new, Some(&oid))
        .unwrap_err();
    assert!(
        matches!(err, RefsError::LockFailed(_)),
        "expected LockFailed while lease held, got {err:?}"
    );

    // After removing the lock, update succeeds (lease still held — additive only).
    std::fs::remove_file(&lock_path).unwrap();
    refs.update("refs/heads/main", &new, Some(&oid)).unwrap();
    assert!(leases.inspect("refs/heads/main").is_some());
}
