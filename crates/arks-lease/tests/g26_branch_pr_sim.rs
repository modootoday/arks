//! G26 — local branch + synthetic PR-env scale + leased concurrent tip workers.
//!
//! SoT: `.spec/sot/local-branch-pr-sim-clean-room.sot.md`
//! Matrix: `verify/matrices/matrix-g26.md`
//! Honesty: local sim ≠ GitHub PR throughput.

use arks_hash::ObjectFormat;
use arks_lease::{
    LeaseError, LeaseManager, PrepItem, TipUpdate, apply_batch, materialize_objects, objects_dir,
    prep_hash_parallel, refns, shard_refns,
};
use arks_object::{ObjectKind, RawObject};
use arks_oid::Oid;
use arks_refs::{FilesystemRefStore, ref_lock_path};
use arks_store::{FilesystemGitStore, ObjectStore};
use std::path::Path;
use std::process::Command;
use std::sync::{Arc, Barrier, Mutex};
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

const N_BRANCHES: usize = 2048;
const N_PR: usize = 2048;
const N_WORKERS: usize = 32;

fn init_git(worktree: &Path) {
    let git = worktree.join(".git");
    std::fs::create_dir_all(git.join("refs/heads")).unwrap();
    std::fs::create_dir_all(git.join("objects")).unwrap();
    std::fs::write(git.join("HEAD"), b"ref: refs/heads/main\n").unwrap();
}

fn seed_blob(store: &mut FilesystemGitStore, payload: &[u8]) -> Oid {
    let blob = RawObject::new(ObjectKind::Blob, payload.to_vec());
    store.put(ObjectFormat::Sha1, &blob).unwrap()
}

fn stock_git(git_dir: &Path, args: &[&str]) -> std::process::Output {
    Command::new("git")
        .args(["--git-dir"])
        .arg(git_dir)
        .args(args)
        .output()
        .expect("stock git")
}

fn stock_for_each_ref_count(git_dir: &Path, pattern: &str) -> usize {
    let out = stock_git(git_dir, &["for-each-ref", "--format=%(refname)", pattern]);
    assert!(
        out.status.success(),
        "for-each-ref failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| !l.is_empty())
        .count()
}

fn write_scale_metrics(path: &Path, branches: usize, pr: usize, workers: usize) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let body = format!(
        "{{\n  \"n_branches\": {branches},\n  \"n_pr\": {pr},\n  \"n_workers\": {workers}\n}}\n"
    );
    let _ = std::fs::write(path, body);
}

/// G26-C01 — thousands-class branch heads.
#[test]
fn g26_c01_branch_scale_n2048() {
    let dir = TempDir::new().unwrap();
    init_git(dir.path());
    let git = dir.path().join(".git");
    let mut store = FilesystemGitStore::new(objects_dir(&git)).unwrap();
    let oid = seed_blob(&mut store, b"g26-branch-seed");

    let mut refs = FilesystemRefStore::open(&git).unwrap();
    refs.update("refs/heads/main", &oid, None).unwrap();
    for i in 0..N_BRANCHES {
        let name = format!("refs/heads/br-{i:04}");
        refs.update(&name, &oid, None).unwrap();
    }
    drop(refs);

    let listed = FilesystemRefStore::open(&git)
        .unwrap()
        .list_refs("refs/heads/br-")
        .unwrap();
    assert_eq!(listed.len(), N_BRANCHES);
    assert_eq!(
        stock_for_each_ref_count(&git, "refs/heads/br-*"),
        N_BRANCHES
    );

    if let Ok(out) = std::env::var("ARKS_G26_SCALE_OUT") {
        write_scale_metrics(Path::new(&out), N_BRANCHES, 0, 0);
    }
}

/// G26-C02 — synthetic PR refs (head + optional merge).
#[test]
fn g26_c02_synthetic_pr_refs_n2048() {
    let dir = TempDir::new().unwrap();
    init_git(dir.path());
    let git = dir.path().join(".git");
    let mut store = FilesystemGitStore::new(objects_dir(&git)).unwrap();
    let head_oid = seed_blob(&mut store, b"g26-pr-head");
    let merge_oid = seed_blob(&mut store, b"g26-pr-merge");

    let mut refs = FilesystemRefStore::open(&git).unwrap();
    for i in 1..=N_PR {
        let head = format!("refs/pull/{i}/head");
        refs.update(&head, &head_oid, None).unwrap();
        if i % 2 == 0 {
            let merge = format!("refs/pull/{i}/merge");
            refs.update(&merge, &merge_oid, None).unwrap();
        }
    }
    drop(refs);

    let heads = FilesystemRefStore::open(&git)
        .unwrap()
        .list_refs("refs/pull/")
        .unwrap();
    // N_PR heads + N_PR/2 merge refs
    assert_eq!(heads.len(), N_PR + N_PR / 2);
    assert_eq!(stock_for_each_ref_count(&git, "refs/pull/*/head"), N_PR);
    assert_eq!(
        stock_for_each_ref_count(&git, "refs/pull/*/merge"),
        N_PR / 2
    );

    // Sample stock resolve
    let sample = stock_git(&git, &["rev-parse", "refs/pull/1/head"]);
    assert!(sample.status.success());
    assert_eq!(
        String::from_utf8_lossy(&sample.stdout).trim(),
        head_oid.to_hex()
    );

    if let Ok(out) = std::env::var("ARKS_G26_SCALE_OUT") {
        write_scale_metrics(Path::new(&out), 0, N_PR, 0);
    }
}

/// G26-C03 — leased concurrent tip workers (merge-style tip advances).
#[test]
fn g26_c03_leased_concurrent_tip_workers_n32() {
    let dir = TempDir::new().unwrap();
    init_git(dir.path());
    let git = dir.path().join(".git");
    let mut store = FilesystemGitStore::new(objects_dir(&git)).unwrap();
    let base = seed_blob(&mut store, b"g26-integ-base");
    let mut refs = FilesystemRefStore::open(&git).unwrap();
    refs.update("refs/heads/main", &base, None).unwrap();
    // Distinct PR-like heads (tips to merge into shard outs).
    let items: Vec<PrepItem> = (0..N_WORKERS)
        .map(|i| PrepItem {
            kind: ObjectKind::Blob,
            payload: format!("g26-pr-body-{i}").into_bytes(),
        })
        .collect();
    let prepared = prep_hash_parallel(&items, ObjectFormat::Sha1, 8).unwrap();
    materialize_objects(&mut store, ObjectFormat::Sha1, &prepared).unwrap();
    for (i, p) in prepared.iter().enumerate() {
        let pr = format!("refs/pull/{}/head", i + 1);
        refs.update(&pr, &p.oid, None).unwrap();
    }
    drop(refs);

    let barrier = Arc::new(Barrier::new(N_WORKERS));
    let git_arc = Arc::new(git.clone());
    let prepared = Arc::new(prepared);
    let mut handles = Vec::with_capacity(N_WORKERS);
    for i in 0..N_WORKERS {
        let barrier = Arc::clone(&barrier);
        let git = Arc::clone(&git_arc);
        let prepared = Arc::clone(&prepared);
        handles.push(thread::spawn(move || {
            barrier.wait();
            let refname = format!("refs/heads/integ-{i:03}");
            let update = TipUpdate {
                shard_resource: shard_refns(&refname),
                refname: refname.clone(),
                expected_old: None,
                new_oid: prepared[i].oid,
            };
            let mut leases = LeaseManager::new();
            apply_batch(
                git.as_path(),
                &mut leases,
                &format!("worker-{i}"),
                Duration::from_secs(60),
                &[update],
            )
            .expect("leased tip apply");
            // Stock peer readable
            let out = stock_git(git.as_path(), &["rev-parse", &refname]);
            assert!(out.status.success());
            assert_eq!(
                String::from_utf8_lossy(&out.stdout).trim(),
                prepared[i].oid.to_hex()
            );
        }));
    }
    for h in handles {
        h.join().expect("worker panicked");
    }
    assert_eq!(
        stock_for_each_ref_count(&git, "refs/heads/integ-*"),
        N_WORKERS
    );

    if let Ok(out) = std::env::var("ARKS_G26_SCALE_OUT") {
        write_scale_metrics(Path::new(&out), 0, 0, N_WORKERS);
    }
}

/// G26-C04 — contended lease on same integration ref fail-closed.
#[test]
fn g26_c04_contended_lease_fail_closed() {
    let resource = refns("refs/heads/main");
    let leases = Arc::new(Mutex::new(LeaseManager::new()));
    let n = N_WORKERS;
    let barrier = Arc::new(Barrier::new(n));
    let results = Arc::new(Mutex::new(Vec::with_capacity(n)));
    let mut handles = Vec::with_capacity(n);
    for i in 0..n {
        let leases = Arc::clone(&leases);
        let barrier = Arc::clone(&barrier);
        let results = Arc::clone(&results);
        let resource = resource.clone();
        handles.push(thread::spawn(move || {
            barrier.wait();
            let outcome = {
                let mut mgr = leases.lock().unwrap();
                mgr.grant(&resource, format!("w-{i}"), Duration::from_secs(30))
            };
            results
                .lock()
                .unwrap()
                .push(outcome.map(|_| ()).map_err(|e| match e {
                    LeaseError::AlreadyHeld { .. } => "already_held".to_string(),
                    other => format!("other:{other}"),
                }));
        }));
    }
    for h in handles {
        h.join().unwrap();
    }
    let outcomes = results.lock().unwrap();
    let oks = outcomes.iter().filter(|r| r.is_ok()).count();
    let held = outcomes
        .iter()
        .filter(|r| matches!(r, Err(s) if s == "already_held"))
        .count();
    assert_eq!(oks, 1, "exactly one lease grant must win");
    assert_eq!(held, n - 1, "others must AlreadyHeld");
}

/// G26-C05 — induce-compare: planted ref lock → stock pathology; arks errors (no skip).
#[test]
fn g26_c05_planted_lock_induce_compare() {
    let dir = TempDir::new().unwrap();
    init_git(dir.path());
    let git = dir.path().join(".git");
    let mut store = FilesystemGitStore::new(objects_dir(&git)).unwrap();
    let oid = seed_blob(&mut store, b"g26-lock-seed");
    let next = seed_blob(&mut store, b"g26-lock-next");
    let mut refs = FilesystemRefStore::open(&git).unwrap();
    refs.update("refs/heads/main", &oid, None).unwrap();
    drop(refs);

    let ref_path = git.join("refs/heads/main");
    let lock = ref_lock_path(&ref_path);
    std::fs::write(&lock, b"planted\n").unwrap();

    // Stock git pathology
    let git_out = stock_git(
        &git,
        &[
            "update-ref",
            "refs/heads/main",
            &next.to_hex(),
            &oid.to_hex(),
        ],
    );
    let git_pathology = !git_out.status.success();
    assert!(
        git_pathology,
        "stock git must fail under planted ref.lock; stderr={}",
        String::from_utf8_lossy(&git_out.stderr)
    );

    // arks must not skip the lock
    let mut refs = FilesystemRefStore::open(&git).unwrap();
    let arks_err = refs.update("refs/heads/main", &next, Some(&oid));
    assert!(arks_err.is_err(), "arks must error under planted lock");

    // Lease held does not authorize lock skip
    let mut leases = LeaseManager::new();
    leases
        .grant(refns("refs/heads/main"), "holder", Duration::from_secs(60))
        .unwrap();
    let update = TipUpdate {
        shard_resource: shard_refns("refs/heads/main"),
        refname: "refs/heads/main".into(),
        expected_old: Some(oid),
        new_oid: next,
    };
    let batch = apply_batch(
        &git,
        &mut leases,
        "holder2",
        Duration::from_secs(60),
        &[update],
    );
    // Either AlreadyHeld on shard or pipeline lock error — never success
    assert!(
        batch.is_err(),
        "leased apply must not skip planted Git lock"
    );

    std::fs::remove_file(&lock).ok();
}
