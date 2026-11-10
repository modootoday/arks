//! G34 — shared-index N-way under arks-lease (M26).
//!
//! Host advisory + clean-room entry. Product Verified requires Docker G34.
//! Cite: api-lockfile, gitformat-index, adr:root/20261110074000.
//! ≠ G22 / G24 / M24 substitute.

use arks_index::{Index, IndexEntry, index_lock_path};
use arks_lease::{
    LeaseError, LeaseManager, SHARED_INDEX_RESOURCE, apply_shared_index_entries,
    shared_index_resource, try_grant_shared_index,
};
use arks_oid::{Oid, OidKind};
use std::fs;
use std::sync::{Arc, Barrier, Mutex};
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

fn empty_blob() -> Oid {
    Oid::from_hex("e69de29bb2d1d6434b8b29ae775ad8c2e48c5391").unwrap()
}

fn sample_entry(path: &str) -> IndexEntry {
    IndexEntry {
        ctime_sec: 1,
        ctime_nsec: 0,
        mtime_sec: 1,
        mtime_nsec: 0,
        dev: 0,
        ino: 0,
        mode: 0o100644,
        uid: 0,
        gid: 0,
        size: 0,
        oid: empty_blob(),
        flags: 0,
        flags_extended: None,
        path: path.into(),
    }
}

fn apply_with_retry(
    index_path: &std::path::Path,
    leases: &Mutex<LeaseManager>,
    holder: &str,
    entry: IndexEntry,
) -> arks_lease::SharedIndexWriteReceipt {
    let ttl = Duration::from_secs(30);
    for _ in 0..200 {
        match apply_shared_index_entries(
            index_path,
            leases,
            holder,
            ttl,
            std::slice::from_ref(&entry),
            OidKind::Sha1,
        ) {
            Ok(r) => return r,
            Err(LeaseError::AlreadyHeld { .. }) => {
                thread::sleep(Duration::from_millis(2));
            }
            Err(e) => panic!("unexpected shared-index error: {e}"),
        }
    }
    panic!("shared-index retry exhausted for {holder}");
}

/// G34-C01 — two writers, one shared index, both entries land; lease + index.lock.
#[test]
fn g34_c01_two_writer_shared_index() {
    assert_eq!(shared_index_resource(), SHARED_INDEX_RESOURCE);
    let dir = TempDir::new().unwrap();
    let git = dir.path().join(".git");
    fs::create_dir_all(&git).unwrap();
    let index_path = git.join("index");
    let leases = Arc::new(Mutex::new(LeaseManager::new()));
    let barrier = Arc::new(Barrier::new(2));

    let mut handles = Vec::new();
    for (holder, path) in [("w0", "a.txt"), ("w1", "b.txt")] {
        let leases = Arc::clone(&leases);
        let index_path = index_path.clone();
        let barrier = Arc::clone(&barrier);
        handles.push(thread::spawn(move || {
            barrier.wait();
            let r = apply_with_retry(&index_path, &leases, holder, sample_entry(path));
            assert!(r.index_lock_honored);
            assert_eq!(r.resource_id, SHARED_INDEX_RESOURCE);
            r
        }));
    }
    for h in handles {
        h.join().expect("writer panicked");
    }

    let idx = Index::read_path(&index_path, OidKind::Sha1).unwrap();
    let paths: Vec<_> = idx.entries.iter().map(|e| e.path.as_str()).collect();
    assert_eq!(paths, vec!["a.txt", "b.txt"]);
    assert!(!index_lock_path(&index_path).exists());
}

/// G34-C02 — second holder AlreadyHeld while first holds `index:shared`.
#[test]
fn g34_c02_second_holder_already_held() {
    let mut mgr = LeaseManager::new();
    try_grant_shared_index(&mut mgr, "holder-a", Duration::from_secs(60)).unwrap();
    match try_grant_shared_index(&mut mgr, "holder-b", Duration::from_secs(60)) {
        Err(LeaseError::AlreadyHeld {
            resource_id,
            holder,
        }) => {
            assert_eq!(resource_id, SHARED_INDEX_RESOURCE);
            assert_eq!(holder, "holder-a");
        }
        other => panic!("expected AlreadyHeld, got {other:?}"),
    }
}

/// G34-C03 — live `index.lock` still fails closed (lease ≠ skip).
#[test]
fn g34_c03_live_index_lock_refuses_write() {
    let dir = TempDir::new().unwrap();
    let index_path = dir.path().join("index");
    let lock = index_lock_path(&index_path);
    fs::write(&lock, b"999999\n").unwrap();
    let leases = Mutex::new(LeaseManager::new());
    match apply_shared_index_entries(
        &index_path,
        &leases,
        "w-lock",
        Duration::from_secs(30),
        &[sample_entry("x.txt")],
        OidKind::Sha1,
    ) {
        Err(LeaseError::Pipeline(msg)) => {
            assert!(
                msg.contains("index.lock") || msg.contains("locked") || msg.contains("Lock"),
                "expected lock failure class, got: {msg}"
            );
        }
        other => panic!("expected Pipeline lock failure, got {other:?}"),
    }
    assert!(!index_path.exists());
    assert!(lock.exists());
}
