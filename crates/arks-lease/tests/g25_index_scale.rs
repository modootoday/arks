//! G25 — CELIR index scale (N=256 required; 384/512 stretch under OOM policy).
//!
//! Enabled with `--features celir-index-stress`.
//! SoT: `.spec/sot/celir-index-scale-clean-room.sot.md`
//!
//! Stretch tiers return soft HARD_BLOCK via worker Result aggregation (no thread
//! panic → catch_unwind holes). Container OOM (137) remains a shell-level hard-block.

#![cfg(feature = "celir-index-stress")]

use arks_hash::ObjectFormat;
use arks_lease::objects_dir;
use arks_object::{ObjectKind, RawObject};
use arks_store::celir_index::{lookup_oid_kind, open_sidecar, rebuild_from_git};
use arks_store::{FilesystemGitStore, ObjectStore};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Barrier, Mutex};
use std::thread;
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

fn stock_git_cat_file(git_dir: &Path, oid_hex: &str) -> Result<(), String> {
    let out = Command::new("git")
        .args(["--git-dir"])
        .arg(git_dir)
        .args(["cat-file", "-t"])
        .arg(oid_hex)
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).into_owned());
    }
    Ok(())
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

fn write_stretch_status(tier: &str, status: &str) {
    let root = std::env::var("ARKS_G25_ROOT").unwrap_or_else(|_| ".".into());
    let out = PathBuf::from(root).join("verify/g25/out");
    let _ = std::fs::create_dir_all(&out);
    let _ = std::fs::write(out.join(format!("{tier}.status")), status);
}

/// Returns Ok on identical effects; Err if any worker mismatch (no panics in workers).
fn concurrent_query_storm(n: usize) -> Result<(), String> {
    let dir = TempDir::new().map_err(|e| e.to_string())?;
    init_git(dir.path());
    let git = dir.path().join(".git");
    let mut store =
        FilesystemGitStore::new(objects_dir(&git)).map_err(|e| format!("store open: {e}"))?;
    let payload = format!("g25-query-seed-{n}").into_bytes();
    let oid = seed_blob(&mut store, &payload);
    let oid_hex = oid.to_hex();
    let want = payload.len().to_string();
    let sidecar = publish_once(&store, &git);

    let barrier = Arc::new(Barrier::new(n));
    let fails = Arc::new(Mutex::new(Vec::new()));
    let mut handles = Vec::with_capacity(n);
    for _ in 0..n {
        let barrier = Arc::clone(&barrier);
        let sidecar = sidecar.clone();
        let oid_hex = oid_hex.clone();
        let want = want.clone();
        let fails = Arc::clone(&fails);
        handles.push(thread::spawn(move || {
            barrier.wait();
            let got = query_effect(&sidecar, &oid_hex);
            if got.as_deref() != Some(want.as_str()) {
                fails
                    .lock()
                    .unwrap()
                    .push(format!("want={want} got={got:?}"));
            }
        }));
    }
    for h in handles {
        h.join().map_err(|_| "worker panicked".to_string())?;
    }
    let fails = fails.lock().unwrap();
    if !fails.is_empty() {
        return Err(fails.join("; "));
    }
    stock_git_cat_file(&git, &oid_hex)?;
    Ok(())
}

/// G25-C01 — required scale tier.
#[test]
fn g25_c01_concurrent_query_n256() {
    concurrent_query_storm(256).expect("N=256 required scale");
}

/// G25-C02 — stretch 384.
#[test]
fn g25_c02_stretch_n384() {
    if std::env::var("ARKS_G25_STRETCH").ok().as_deref() == Some("0") {
        write_stretch_status("n384", "SKIP");
        return;
    }
    match concurrent_query_storm(384) {
        Ok(()) => write_stretch_status("n384", "PASS"),
        Err(e) => {
            write_stretch_status("n384", "HARD_BLOCK");
            if std::env::var("ARKS_G25_REQUIRE_STRETCH").ok().as_deref() == Some("1") {
                panic!("stretch N=384 failed: {e}");
            }
            eprintln!(
                "G25: stretch N=384 HARD_BLOCK ({e}); required N=256 still authoritative"
            );
        }
    }
}

/// G25-C03 — stretch 512.
#[test]
fn g25_c03_stretch_n512() {
    if std::env::var("ARKS_G25_STRETCH").ok().as_deref() == Some("0") {
        write_stretch_status("n512", "SKIP");
        return;
    }
    match concurrent_query_storm(512) {
        Ok(()) => write_stretch_status("n512", "PASS"),
        Err(e) => {
            write_stretch_status("n512", "HARD_BLOCK");
            if std::env::var("ARKS_G25_REQUIRE_STRETCH").ok().as_deref() == Some("1") {
                panic!("stretch N=512 failed: {e}");
            }
            eprintln!(
                "G25: stretch N=512 HARD_BLOCK ({e}); required N=256 still authoritative"
            );
        }
    }
}
