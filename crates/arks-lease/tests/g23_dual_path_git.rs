//! G23 — CELIR dual-path Git verified (honest Done; not CELIR ObjectStore).
//!
//! Enabled with `--features celir-index-stress`.
//! SoT: `.spec/sot/celir-dual-path-git-verified.sot.md`
//! ADR: `.spec/adr/20261109204916-celir-dual-path-git-verified-done.adr.md`

#![cfg(feature = "celir-index-stress")]

use arks_lease::objects_dir;
use arks_store::celir_index::{lookup_oid_kind, open_sidecar, rebuild_from_git};
use arks_store::{FilesystemGitStore, ObjectStore};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

fn arks_bin() -> PathBuf {
    if let Ok(p) = std::env::var("ARKS_BIN") {
        let p = PathBuf::from(p);
        if p.is_file() {
            return p;
        }
    }
    panic!("ARKS_BIN missing or not a file (run-g023-celir-dual-path.sh must build arks first)");
}

fn arks(repo: &Path, args: &[&str]) -> std::process::Output {
    Command::new(arks_bin())
        .args(args)
        .current_dir(repo)
        .env("GIT_AUTHOR_NAME", "arks")
        .env("GIT_AUTHOR_EMAIL", "arks@example.com")
        .env("GIT_COMMITTER_NAME", "arks")
        .env("GIT_COMMITTER_EMAIL", "arks@example.com")
        .env("GIT_AUTHOR_DATE", "1000000000")
        .env("GIT_COMMITTER_DATE", "1000000000")
        .output()
        .expect("arks")
}

fn git(repo: &Path, args: &[&str]) -> std::process::Output {
    Command::new("git")
        .args(args)
        .current_dir(repo)
        .env("GIT_AUTHOR_NAME", "git")
        .env("GIT_AUTHOR_EMAIL", "git@example.com")
        .env("GIT_COMMITTER_NAME", "git")
        .env("GIT_COMMITTER_EMAIL", "git@example.com")
        .env("GIT_AUTHOR_DATE", "1000000001 +0000")
        .env("GIT_COMMITTER_DATE", "1000000001 +0000")
        .output()
        .expect("git")
}

fn assert_ok(out: &std::process::Output, label: &str) {
    assert!(
        out.status.success(),
        "{label} failed: status={:?}\nstdout={}\nstderr={}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

/// G23-C01 + G23-C02 — porcelain on FS `.git` while celir-index build/publish/open/query works.
#[test]
fn g023-celir-dual-path_c01_c02_porcelain_and_celir_index_same_repo() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path();

    assert_ok(&arks(repo, &["init"]), "arks init");
    fs::write(repo.join("dual.txt"), b"dual-path-seed").unwrap();
    assert_ok(&arks(repo, &["add", "dual.txt"]), "arks add");
    assert_ok(&arks(repo, &["commit", "-m", "g023-celir-dual-path-dual"]), "arks commit");
    assert_ok(&arks(repo, &["status"]), "arks status");
    let log = arks(repo, &["log"]);
    assert_ok(&log, "arks log");
    assert!(
        String::from_utf8_lossy(&log.stdout).contains("g023-celir-dual-path-dual"),
        "arks log missing subject"
    );

    let git_log = git(repo, &["log", "--oneline"]);
    assert_ok(&git_log, "git log");
    assert!(
        String::from_utf8_lossy(&git_log.stdout).contains("g023-celir-dual-path-dual"),
        "stock git cannot open/read tip"
    );
    let tip = git(repo, &["rev-parse", "HEAD"]);
    assert_ok(&tip, "git rev-parse HEAD");
    let tip_hex = String::from_utf8_lossy(&tip.stdout).trim().to_string();
    let cat = git(repo, &["cat-file", "-t", &tip_hex]);
    assert_ok(&cat, "git cat-file -t HEAD");
    assert_eq!(String::from_utf8_lossy(&cat.stdout).trim(), "commit");

    let git_dir = repo.join(".git");
    let store = FilesystemGitStore::new(objects_dir(&git_dir)).unwrap();
    let report = rebuild_from_git(&store, &git_dir).expect("celir rebuild");
    assert!(report.rows >= 1, "index must see committed objects");
    let store_ro = open_sidecar(&report.sidecar_root).expect("open sidecar");
    let mut snap = store_ro.snapshot().expect("snapshot");
    // Blob payload length for dual.txt content written via porcelain.
    let blob_oid = git(repo, &["rev-parse", "HEAD:dual.txt"]);
    assert_ok(&blob_oid, "git rev-parse HEAD:dual.txt");
    let blob_hex = String::from_utf8_lossy(&blob_oid.stdout).trim().to_string();
    let effect = lookup_oid_kind(&mut snap, &blob_hex, "blob").expect("lookup");
    assert_eq!(effect.as_deref(), Some("14")); // "dual-path-seed".len()
    assert!(
        store
            .exists(&arks_store::Oid::from_hex(&blob_hex).expect("oid"))
            .unwrap()
    );
}

/// G23-C03 — ObjectStore remains filesystem; index hint ≠ membership.
#[test]
fn g023-celir-dual-path_c03_object_store_filesystem_only() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path();
    assert_ok(&arks(repo, &["init"]), "arks init");
    fs::write(repo.join("fs.txt"), b"fs-only").unwrap();
    assert_ok(&arks(repo, &["add", "fs.txt"]), "arks add");
    assert_ok(&arks(repo, &["commit", "-m", "fs"]), "arks commit");

    let git_dir = repo.join(".git");
    let store = FilesystemGitStore::new(objects_dir(&git_dir)).unwrap();
    let blob = git(repo, &["rev-parse", "HEAD:fs.txt"]);
    assert_ok(&blob, "rev-parse blob");
    let hex = String::from_utf8_lossy(&blob.stdout).trim().to_string();
    let oid = arks_store::Oid::from_hex(&hex).unwrap();
    assert!(store.exists(&oid).unwrap());
    let obj = store.get(&oid).unwrap();
    assert_eq!(obj.payload, b"fs-only");
    let _ = rebuild_from_git(&store, &git_dir).unwrap();
}
