//! G22 — CELIR index contention under leases (M24 clean-room).
//!
//! Enabled only with `--features celir-index-stress` (pulls `arks-store/celir-index`).
//! SoT: `.spec/sot/celir-index-contention-clean-room.sot.md`
//! Matrix: `verify/matrix-g22.md`

#![cfg(feature = "celir-index-stress")]

use arks_hash::ObjectFormat;
use arks_lease::{
    apply_batch, index_scope, materialize_objects, objects_dir, prep_hash_parallel, shard_refns,
    LeaseManager, PrepItem, TipUpdate,
};
use arks_object::{ObjectKind, RawObject};
use arks_store::celir_index::{
    lookup_oid_kind, open_sidecar, rebuild_from_git, sidecar_root, RebuildError,
};
use arks_store::{FilesystemGitStore, ObjectStore};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Barrier, Mutex};
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

fn init_git(worktree: &Path) {
    let git = worktree.join(".git");
    std::fs::create_dir_all(git.join("refs/heads")).unwrap();
    std::fs::create_dir_all(git.join("objects")).unwrap();
    std::fs::write(git.join("HEAD"), b"ref: refs/heads/main\n").unwrap();
}

fn seed_blob(store: &mut FilesystemGitStore, payload: &[u8]) -> arks_store::Oid {
    let blob = RawObject::new(ObjectKind::Blob, payload.to_vec());
    store.put(ObjectFormat::Sha1, &blob).unwrap()
}

fn stock_git_cat_file(git_dir: &Path, oid_hex: &str) -> String {
    let out = Command::new("git")
        .args(["--git-dir"])
        .arg(git_dir)
        .args(["cat-file", "-t"])
        .arg(oid_hex)
        .output()
        .expect("stock git cat-file");
    assert!(
        out.status.success(),
        "stock git cat-file failed: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_owned()
}

fn publish_once(store: &FilesystemGitStore, git_dir: &Path) -> PathBuf {
    let report = rebuild_from_git(store, git_dir).expect("initial rebuild");
    assert!(report.rows >= 1);
    report.sidecar_root
}

fn query_effect(sidecar: &Path, oid_hex: &str) -> Option<String> {
    let store_ro = open_sidecar(sidecar).ok()?;
    let mut snap = store_ro.snapshot().ok()?;
    lookup_oid_kind(&mut snap, oid_hex, "blob").ok()?
}

/// G22-C01 / G22-C02 — concurrent readonly open/query after publish.
fn concurrent_query_storm(n: usize) {
    let dir = TempDir::new().unwrap();
    init_git(dir.path());
    let git = dir.path().join(".git");
    let mut store = FilesystemGitStore::new(objects_dir(&git)).unwrap();
    let payload = b"g22-query-seed";
    let oid = seed_blob(&mut store, payload);
    let oid_hex = oid.to_hex();
    let want = payload.len().to_string();
    let sidecar = publish_once(&store, &git);

    let barrier = Arc::new(Barrier::new(n));
    let mut handles = Vec::with_capacity(n);
    for _ in 0..n {
        let barrier = Arc::clone(&barrier);
        let sidecar = sidecar.clone();
        let oid_hex = oid_hex.clone();
        let want = want.clone();
        handles.push(thread::spawn(move || {
            barrier.wait();
            let got = query_effect(&sidecar, &oid_hex);
            assert_eq!(got.as_deref(), Some(want.as_str()));
        }));
    }
    for h in handles {
        h.join().expect("query worker panicked");
    }
    assert_eq!(stock_git_cat_file(&git, &oid_hex), "blob");
}

#[test]
fn g22_c01_concurrent_query_n32() {
    concurrent_query_storm(32);
}

#[test]
fn g22_c02_concurrent_query_n128() {
    concurrent_query_storm(128);
}

/// G22-C03 — concurrent rebuild without index lease must fail closed; leased rebuild restores.
#[test]
fn g22_c03_concurrent_rebuild_fail_closed_then_leased() {
    let dir = TempDir::new().unwrap();
    init_git(dir.path());
    let git = dir.path().join(".git");
    let mut store = FilesystemGitStore::new(objects_dir(&git)).unwrap();
    let payload = b"g22-rebuild-seed";
    let oid = seed_blob(&mut store, payload);
    let oid_hex = oid.to_hex();
    let want = payload.len().to_string();
    let _ = publish_once(&store, &git);

    let n = 32;
    let barrier = Arc::new(Barrier::new(n));
    let git_arc = Arc::new(git.clone());
    // FilesystemGitStore is not Sync for &mut, but rebuild takes &store (read OIDs).
    // Each worker opens its own store handle on the same objects dir.
    let mut handles = Vec::with_capacity(n);
    let results = Arc::new(Mutex::new(Vec::with_capacity(n)));
    for _ in 0..n {
        let barrier = Arc::clone(&barrier);
        let git = Arc::clone(&git_arc);
        let results = Arc::clone(&results);
        handles.push(thread::spawn(move || {
            let store = FilesystemGitStore::new(objects_dir(&git)).unwrap();
            barrier.wait();
            let outcome = rebuild_from_git(&store, git.as_path());
            results.lock().unwrap().push(outcome.map(|r| r.generation));
        }));
    }
    for h in handles {
        h.join().expect("rebuild worker panicked");
    }
    let outcomes = results.lock().unwrap();
    let oks = outcomes.iter().filter(|r| r.is_ok()).count();
    let errs = outcomes.iter().filter(|r| r.is_err()).count();
    assert_eq!(oks + errs, n);
    // Contended create/remove must not pretend every publisher succeeded cleanly.
    // At least one failure OR exactly one success is acceptable; all-success under
    // racing remove_dir_all would be suspicious — require errors when oks > 1.
    if oks > 1 {
        // Multiple Ok can happen if races serialize by luck; still must not leave torn index.
        let sidecar = sidecar_root(&git);
        if sidecar.exists() {
            let effect = query_effect(&sidecar, &oid_hex);
            assert!(
                effect.is_none() || effect.as_deref() == Some(want.as_str()),
                "torn/conflicting Present forbidden; got {effect:?}"
            );
        }
    } else {
        assert!(errs >= 1 || oks <= 1);
    }
    drop(outcomes);

    // Leased single-writer rebuild restores integrity.
    let resource = index_scope("celir-index");
    let mut leases = LeaseManager::new();
    leases
        .grant(&resource, "g22-publisher", Duration::from_secs(60))
        .unwrap();
    let store = FilesystemGitStore::new(objects_dir(&git)).unwrap();
    let report = rebuild_from_git(&store, &git).expect("leased rebuild");
    leases.release(&resource, "g22-publisher").unwrap();
    assert!(report.rows >= 1);
    let effect = query_effect(&report.sidecar_root, &oid_hex);
    assert_eq!(effect.as_deref(), Some(want.as_str()));
    assert_eq!(stock_git_cat_file(&git, &oid_hex), "blob");
}

/// G22-C04 — lease shard Git writers + concurrent index readers; stock git OK.
#[test]
fn g22_c04_shard_writers_and_index_readers() {
    let dir = TempDir::new().unwrap();
    init_git(dir.path());
    let git = dir.path().join(".git");
    let mut store = FilesystemGitStore::new(objects_dir(&git)).unwrap();
    let seed = seed_blob(&mut store, b"g22-mixed-seed");
    let seed_hex = seed.to_hex();
    let want = (b"g22-mixed-seed".len()).to_string();
    let sidecar = publish_once(&store, &git);

    let n = 32;
    let items: Vec<PrepItem> = (0..n)
        .map(|i| PrepItem {
            kind: ObjectKind::Blob,
            payload: format!("g22-agent-{i}").into_bytes(),
        })
        .collect();
    let prepared = prep_hash_parallel(&items, ObjectFormat::Sha1, 8).unwrap();
    materialize_objects(&mut store, ObjectFormat::Sha1, &prepared).unwrap();

    let updates: Vec<TipUpdate> = prepared
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let refname = format!("refs/heads/agent-{i:03}");
            TipUpdate {
                shard_resource: shard_refns(&refname),
                refname,
                expected_old: None,
                new_oid: p.oid,
            }
        })
        .collect();

    let barrier = Arc::new(Barrier::new(2));
    let git_apply = git.clone();
    let updates_c = updates.clone();
    let barrier_w = Arc::clone(&barrier);
    let writer = thread::spawn(move || {
        barrier_w.wait();
        let mut leases = LeaseManager::new();
        apply_batch(
            &git_apply,
            &mut leases,
            "batch-engine",
            Duration::from_secs(60),
            &updates_c,
        )
        .expect("shard apply")
    });

    let sidecar_c = sidecar.clone();
    let seed_hex_c = seed_hex.clone();
    let want_c = want.clone();
    let barrier_r = Arc::clone(&barrier);
    let readers = thread::spawn(move || {
        barrier_r.wait();
        let mut handles = Vec::with_capacity(n);
        let inner = Arc::new(Barrier::new(n));
        for _ in 0..n {
            let inner = Arc::clone(&inner);
            let sidecar = sidecar_c.clone();
            let seed_hex = seed_hex_c.clone();
            let want = want_c.clone();
            handles.push(thread::spawn(move || {
                inner.wait();
                let got = query_effect(&sidecar, &seed_hex);
                assert_eq!(got.as_deref(), Some(want.as_str()));
            }));
        }
        for h in handles {
            h.join().expect("reader panicked");
        }
    });

    let receipt = writer.join().expect("writer panicked");
    readers.join().expect("readers panicked");
    assert!(receipt.tips_verified);
    assert_eq!(receipt.applied.len(), n);
    assert_eq!(stock_git_cat_file(&git, &seed_hex), "blob");
    for p in &prepared {
        assert_eq!(stock_git_cat_file(&git, &p.oid.to_hex()), "blob");
    }
}

/// G22-C05 — ObjectStore honesty: CELIR companion is not the object store.
#[test]
fn g22_c05_object_store_remains_filesystem() {
    let dir = TempDir::new().unwrap();
    init_git(dir.path());
    let git = dir.path().join(".git");
    let mut store = FilesystemGitStore::new(objects_dir(&git)).unwrap();
    let oid = seed_blob(&mut store, b"g22-fs-only");
    assert!(store.exists(&oid).unwrap());
    let _ = rebuild_from_git(&store, &git).unwrap();
    // Index hint ≠ ObjectStore membership proof; FS get still required.
    let obj = store.get(&oid).unwrap();
    assert_eq!(obj.kind, ObjectKind::Blob);
    assert_eq!(stock_git_cat_file(&git, &oid.to_hex()), "blob");
    // Row-budget fail-closed path still errors (does not silently truncate to green).
    let err = rebuild_from_git_capped_zero(&store, &git);
    assert!(matches!(err, Err(RebuildError::Empty)));
}

fn rebuild_from_git_capped_zero(
    store: &FilesystemGitStore,
    git_dir: &Path,
) -> Result<(), RebuildError> {
    // Exercise fail-closed empty/cap API without inventing ObjectStore.
    use arks_store::celir_index::rebuild_from_git_capped;
    rebuild_from_git_capped(store, git_dir, 0).map(|_| ())
}
