//! G24 — L3 HighConcurrency + CELIR index joint clean-room.
//!
//! Enabled with `--features celir-index-stress`.
//! SoT: `.spec/sot/l3-celir-index-joint-clean-room.sot.md`

#![cfg(feature = "celir-index-stress")]

use arks_hash::ObjectFormat;
use arks_lease::{
    LeaseError, LeaseManager, PrepItem, TipUpdate, WriterMode, WriterSession, apply_batch,
    materialize_objects, objects_dir, prep_hash, prep_hash_parallel, shard_refns,
};
use arks_object::{ObjectKind, RawObject};
use arks_store::celir_index::{lookup_oid_kind, open_sidecar, rebuild_from_git};
use arks_store::{FilesystemGitStore, ObjectStore};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Barrier};
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
        "stock git cat-file failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_owned()
}

fn query_effect(sidecar: &Path, oid_hex: &str) -> Option<String> {
    let store_ro = open_sidecar(sidecar).ok()?;
    let mut snap = store_ro.snapshot().ok()?;
    lookup_oid_kind(&mut snap, oid_hex, "blob").ok()?
}

fn publish_once(store: &FilesystemGitStore, git_dir: &Path) -> PathBuf {
    let report = rebuild_from_git(store, git_dir).expect("rebuild");
    assert!(report.rows >= 1);
    report.sidecar_root
}

/// G24-C01 — HighConcurrency valve unlocks (opt-in).
#[test]
fn g024_l3_celir_joint_c01_highconcurrency_opt_in() {
    let hc = WriterSession::new(WriterMode::HighConcurrency).unwrap();
    assert_eq!(hc.mode(), WriterMode::HighConcurrency);
    let def = WriterSession::default();
    assert_eq!(def.mode(), WriterMode::GitCompat);
}

fn joint_storm(n: usize) {
    let _session = WriterSession::new(WriterMode::HighConcurrency).unwrap();

    let dir = TempDir::new().unwrap();
    init_git(dir.path());
    let git = dir.path().join(".git");
    let mut store = FilesystemGitStore::new(objects_dir(&git)).unwrap();
    let seed = seed_blob(&mut store, b"g024-l3-celir-joint-joint-seed");
    let seed_hex = seed.to_hex();
    let want = (b"g024-l3-celir-joint-joint-seed".len()).to_string();
    let sidecar = publish_once(&store, &git);

    let items: Vec<PrepItem> = (0..n)
        .map(|i| PrepItem {
            kind: ObjectKind::Blob,
            payload: format!("g024-l3-celir-joint-agent-{i}").into_bytes(),
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
                new_oid: p.oid.clone(),
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
        .expect("HC shard apply")
    });

    let sidecar_c = sidecar.clone();
    let seed_hex_c = seed_hex.clone();
    let want_c = want.clone();
    let barrier_r = Arc::clone(&barrier);
    let readers_n = n.min(32);
    let readers = thread::spawn(move || {
        barrier_r.wait();
        let inner = Arc::new(Barrier::new(readers_n));
        let mut handles = Vec::with_capacity(readers_n);
        for _ in 0..readers_n {
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
    // Post-storm index still consistent for seed.
    assert_eq!(
        query_effect(&sidecar, &seed_hex).as_deref(),
        Some(want.as_str())
    );
}

#[test]
fn g024_l3_celir_joint_c02_joint_n32() {
    joint_storm(32);
}

#[test]
fn g024_l3_celir_joint_c03_joint_n128() {
    joint_storm(128);
}

/// G24-C04 — competing Git lock still fails; lease ≠ skip locks.
#[test]
fn g024_l3_celir_joint_c04_competing_lock_still_fails() {
    let _session = WriterSession::new(WriterMode::HighConcurrency).unwrap();
    let dir = TempDir::new().unwrap();
    init_git(dir.path());
    let git = dir.path().join(".git");
    let mut store = FilesystemGitStore::new(objects_dir(&git)).unwrap();
    let prep = prep_hash(
        &[PrepItem {
            kind: ObjectKind::Blob,
            payload: b"x".to_vec(),
        }],
        ObjectFormat::Sha1,
    )
    .unwrap();
    materialize_objects(&mut store, ObjectFormat::Sha1, &prep).unwrap();
    let oid = prep[0].oid.clone();
    let refname = "refs/heads/shard-a";
    std::fs::write(git.join("refs/heads/shard-a.lock"), b"").unwrap();

    let mut leases = LeaseManager::new();
    let err = apply_batch(
        &git,
        &mut leases,
        "agent",
        Duration::from_secs(10),
        &[TipUpdate {
            refname: refname.into(),
            expected_old: None,
            new_oid: oid,
            shard_resource: shard_refns(refname),
        }],
    )
    .unwrap_err();
    assert!(
        matches!(err, LeaseError::Pipeline(_)),
        "expected Pipeline lock failure, got {err:?}"
    );
    assert!(leases.list().is_empty());
}

/// G24-C05 — ObjectStore honesty under joint feature graph.
#[test]
fn g024_l3_celir_joint_c05_object_store_filesystem() {
    let dir = TempDir::new().unwrap();
    init_git(dir.path());
    let git = dir.path().join(".git");
    let mut store = FilesystemGitStore::new(objects_dir(&git)).unwrap();
    let oid = seed_blob(&mut store, b"g024-l3-celir-joint-fs");
    assert!(store.exists(&oid).unwrap());
    let _ = rebuild_from_git(&store, &git).unwrap();
    assert_eq!(store.get(&oid).unwrap().kind, ObjectKind::Blob);
    assert_eq!(stock_git_cat_file(&git, &oid.to_hex()), "blob");
}
