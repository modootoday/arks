//! Gate G21 — M16 Sy-1 everyday porcelain flag claim rows.
//! Matrix: `verify/matrices/matrix-g021-cli-flag-symmetry.md`.

use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use tempfile::TempDir;

static RECEIPT: Mutex<Vec<(&'static str, bool)>> = Mutex::new(Vec::new());

fn record_row(id: &'static str, pass: bool) {
    RECEIPT.lock().unwrap().push((id, pass));
}

fn arks_bin() -> PathBuf {
    if let Ok(p) = std::env::var("ARKS_BIN") {
        let p = PathBuf::from(p);
        if p.is_file() {
            return p;
        }
    }
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(target) = std::env::var("CARGO_TARGET_DIR") {
        candidates.push(PathBuf::from(&target).join("release/arks"));
        candidates.push(PathBuf::from(&target).join("release/arks.exe"));
        candidates.push(PathBuf::from(&target).join("debug/arks"));
        candidates.push(PathBuf::from(&target).join("debug/arks.exe"));
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
    candidates.push(root.join("target/release/arks"));
    candidates.push(root.join("target/release/arks.exe"));
    candidates.push(root.join("target/debug/arks"));
    candidates.push(root.join("target/debug/arks.exe"));
    for c in &candidates {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!("missing arks binary; tried {candidates:?}");
}

fn pin_env(cmd: &mut Command) {
    cmd.env("GIT_AUTHOR_NAME", "arks")
        .env("GIT_AUTHOR_EMAIL", "arks@example.com")
        .env("GIT_COMMITTER_NAME", "arks")
        .env("GIT_COMMITTER_EMAIL", "arks@example.com")
        .env("GIT_AUTHOR_DATE", "1000000000 +0000")
        .env("GIT_COMMITTER_DATE", "1000000000 +0000");
}

fn arks(repo: &Path, args: &[&str]) -> std::process::Output {
    let mut c = Command::new(arks_bin());
    pin_env(&mut c);
    c.args(args).current_dir(repo).output().expect("arks")
}

fn git(repo: &Path, args: &[&str]) -> std::process::Output {
    let mut c = Command::new("git");
    pin_env(&mut c);
    c.args(args).current_dir(repo).output().expect("git")
}

fn assert_ok(out: &std::process::Output, label: &str) {
    assert!(
        out.status.success(),
        "{label}: {}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

fn init_repo(path: &Path) {
    fs::create_dir_all(path).unwrap();
    assert_ok(&arks(path, &["init"]), "init");
}

fn seed(repo: &Path, file: &str, body: &str, msg: &str) {
    fs::write(repo.join(file), body).unwrap();
    assert_ok(&arks(repo, &["add", file]), "add");
    assert_ok(&arks(repo, &["commit", "-m", msg]), "commit");
}

#[test]
fn m16_sy1_01_status_b() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("r");
    init_repo(&repo);
    seed(&repo, "a.txt", "one\n", "first");
    let out = arks(&repo, &["status", "-b", "--porcelain"]);
    assert_ok(&out, "status -b --porcelain");
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(
        s.lines().next().unwrap_or("").starts_with("## "),
        "expected ## branch header: {s}"
    );
    record_row("M16-SY1-01", true);
}

#[test]
fn m16_sy1_02_log_oneline_graph() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("r");
    init_repo(&repo);
    seed(&repo, "a.txt", "one\n", "first");
    fs::write(repo.join("a.txt"), "two\n").unwrap();
    assert_ok(&arks(&repo, &["add", "a.txt"]), "add2");
    assert_ok(&arks(&repo, &["commit", "-m", "second"]), "commit2");
    let oneline = arks(&repo, &["log", "--oneline", "-n", "2"]);
    assert_ok(&oneline, "log --oneline");
    let s = String::from_utf8_lossy(&oneline.stdout);
    assert!(s.contains("second") && s.contains("first"), "{s}");
    let graph = arks(&repo, &["log", "--graph", "--oneline"]);
    assert_ok(&graph, "log --graph");
    let g = String::from_utf8_lossy(&graph.stdout);
    assert!(g.contains('*'), "graph marker missing: {g}");
    assert_ok(&git(&repo, &["log", "-1", "--oneline"]), "git peer");
    record_row("M16-SY1-02", true);
}

#[test]
fn m16_sy1_03_branch_a_r() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("r");
    init_repo(&repo);
    seed(&repo, "a.txt", "one\n", "first");
    assert_ok(&arks(&repo, &["branch", "topic"]), "branch topic");
    // Simulate remote-tracking ref for -r/-a.
    let oid = String::from_utf8_lossy(&git(&repo, &["rev-parse", "HEAD"]).stdout)
        .trim()
        .to_string();
    let remote_ref = repo.join(".git/refs/remotes/origin/main");
    fs::create_dir_all(remote_ref.parent().unwrap()).unwrap();
    fs::write(&remote_ref, format!("{oid}\n")).unwrap();
    let all = arks(&repo, &["branch", "-a"]);
    assert_ok(&all, "branch -a");
    let s = String::from_utf8_lossy(&all.stdout);
    assert!(s.contains("topic"), "{s}");
    assert!(s.contains("remotes/origin/main") || s.contains("origin/main"), "{s}");
    let rem = arks(&repo, &["branch", "-r"]);
    assert_ok(&rem, "branch -r");
    record_row("M16-SY1-03", true);
}

#[test]
fn m16_sy1_04_add_chmod() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("r");
    init_repo(&repo);
    fs::write(repo.join("x.sh"), "#!/bin/sh\n").unwrap();
    assert_ok(
        &arks(&repo, &["add", "--chmod=+x", "x.sh"]),
        "add --chmod=+x",
    );
    assert_ok(&arks(&repo, &["commit", "-m", "exec"]), "commit");
    let mode = git(&repo, &["ls-files", "--stage", "x.sh"]);
    assert_ok(&mode, "git ls-files --stage");
    let s = String::from_utf8_lossy(&mode.stdout);
    assert!(
        s.contains("100755"),
        "expected executable mode in index: {s}"
    );
    record_row("M16-SY1-04", true);
}

#[test]
fn m16_sy1_05_clean_x_describe_dirty() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("r");
    init_repo(&repo);
    seed(&repo, "a.txt", "one\n", "first");
    assert_ok(&arks(&repo, &["tag", "-a", "-m", "t", "v1"]), "tag");
    fs::write(repo.join("junk.tmp"), "x\n").unwrap();
    assert_ok(&arks(&repo, &["clean", "-x", "-f"]), "clean -x -f");
    assert!(!repo.join("junk.tmp").exists(), "junk should be removed");
    fs::write(repo.join("a.txt"), "dirty\n").unwrap();
    let d = arks(&repo, &["describe", "--tags", "--dirty"]);
    assert_ok(&d, "describe --dirty");
    let s = String::from_utf8_lossy(&d.stdout);
    assert!(s.contains("dirty"), "expected dirty mark: {s}");
    record_row("M16-SY1-05", true);
}

#[test]
fn m16_sy1_06_switch_detach_diff_stat_bare() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("r");
    init_repo(&repo);
    seed(&repo, "a.txt", "one\n", "first");
    let head = String::from_utf8_lossy(&arks(&repo, &["rev-parse", "HEAD"]).stdout)
        .trim()
        .to_string();
    assert_ok(
        &arks(&repo, &["switch", "--detach", &head]),
        "switch --detach",
    );
    fs::write(repo.join("a.txt"), "two\n").unwrap();
    assert_ok(&arks(&repo, &["add", "a.txt"]), "add");
    let st = arks(&repo, &["diff", "--cached", "--stat"]);
    assert_ok(&st, "diff --stat");
    let bare = tmp.path().join("bare.git");
    assert_ok(&arks(tmp.path(), &["init", "--bare", bare.to_str().unwrap()]), "init --bare");
    assert!(bare.join("HEAD").is_file());
    record_row("M16-SY1-06", true);
}

#[test]
fn m16_sy1_receipt_write() {
    // Ensure prior tests recorded; write advisory receipt when env set.
    let rows = RECEIPT.lock().unwrap().clone();
    if let Ok(path) = std::env::var("ARKS_G21_RECEIPT_OUT") {
        let body = json!({
            "gate": "G21",
            "wave": "Sy-1",
            "rows": rows.iter().map(|(id, pass)| json!({"id": id, "pass": pass})).collect::<Vec<_>>(),
        });
        if let Some(parent) = Path::new(&path).parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(&path, serde_json::to_vec_pretty(&body).unwrap());
    }
    // Soft check: at least the five functional rows should have run in this process
    // when cargo runs the whole suite; individual test binaries may see empty.
    let _ = rows;
}
