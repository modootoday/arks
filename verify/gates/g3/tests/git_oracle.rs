//! Gate G3 — full index + commit round-trip vs stock Git (clean-room).
//! arks add/commit → git log/ls-files/cat-file; reverse git → arks log/show/status.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

fn require_git() {
    let out = Command::new("git").arg("--version").output().expect("git");
    assert!(out.status.success(), "stock git missing in clean-room");
}

fn arks_bin() -> PathBuf {
    let target = std::env::var("CARGO_TARGET_DIR").unwrap_or_else(|_| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../target-cleanroom")
            .display()
            .to_string()
    });
    let cand = PathBuf::from(&target).join("debug/arks");
    assert!(
        cand.is_file(),
        "arks binary missing at {}; run `cargo build -p arks` first",
        cand.display()
    );
    cand
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
        .expect("run arks")
}

fn git(repo: &Path, args: &[&str]) -> std::process::Output {
    Command::new("git")
        .args(args)
        .current_dir(repo)
        .env("GIT_AUTHOR_NAME", "git")
        .env("GIT_AUTHOR_EMAIL", "git@example.com")
        .env("GIT_COMMITTER_NAME", "git")
        .env("GIT_COMMITTER_EMAIL", "git@example.com")
        .env("GIT_AUTHOR_DATE", "1000000001")
        .env("GIT_COMMITTER_DATE", "1000000001")
        .output()
        .expect("run git")
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

#[test]
fn arks_commit_readable_by_git() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path();
    assert_ok(&arks(repo, &["init"]), "arks init");
    fs::write(repo.join("a.txt"), b"alpha").unwrap();
    assert_ok(&arks(repo, &["add", "a.txt"]), "arks add");
    assert_ok(&arks(repo, &["commit", "-m", "arks-first"]), "arks commit");

    let log = git(repo, &["log", "--oneline"]);
    assert_ok(&log, "git log");
    let log_s = String::from_utf8_lossy(&log.stdout);
    assert!(log_s.contains("arks-first"), "git log missing subject: {log_s}");

    let ls = git(repo, &["ls-files", "-s"]);
    assert_ok(&ls, "git ls-files");
    let ls_s = String::from_utf8_lossy(&ls.stdout);
    assert!(ls_s.contains("a.txt"), "ls-files: {ls_s}");

    let tip = git(repo, &["rev-parse", "HEAD"]);
    assert_ok(&tip, "git rev-parse HEAD");
    let tip_hex = String::from_utf8_lossy(&tip.stdout).trim().to_string();
    let cat = git(repo, &["cat-file", "-t", &tip_hex]);
    assert_ok(&cat, "git cat-file -t");
    assert_eq!(String::from_utf8_lossy(&cat.stdout).trim(), "commit");
}

#[test]
fn git_commit_readable_by_arks() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path();
    let init = git(repo, &["-c", "init.defaultBranch=main", "init"]);
    assert_ok(&init, "git init");
    fs::write(repo.join("b.txt"), b"beta").unwrap();
    assert_ok(&git(repo, &["add", "b.txt"]), "git add");
    assert_ok(&git(repo, &["commit", "-m", "git-first"]), "git commit");

    let log = arks(repo, &["log"]);
    assert_ok(&log, "arks log");
    let log_s = String::from_utf8_lossy(&log.stdout);
    assert!(log_s.contains("git-first"), "arks log: {log_s}");

    let show = arks(repo, &["show", "HEAD"]);
    assert_ok(&show, "arks show");
    let show_s = String::from_utf8_lossy(&show.stdout);
    assert!(show_s.contains("git-first"), "arks show: {show_s}");

    let st = arks(repo, &["status"]);
    assert_ok(&st, "arks status");
    let st_s = String::from_utf8_lossy(&st.stdout);
    assert!(
        st_s.contains("working tree clean") || st_s.contains("nothing to commit"),
        "arks status: {st_s}"
    );
}

#[test]
fn nested_path_roundtrip() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path();
    assert_ok(&arks(repo, &["init"]), "init");
    fs::create_dir_all(repo.join("src")).unwrap();
    fs::write(repo.join("src/main.rs"), b"fn main() {}\n").unwrap();
    assert_ok(&arks(repo, &["add", "src/main.rs"]), "add nested");
    assert_ok(&arks(repo, &["commit", "-m", "nested"]), "commit nested");
    let ls = git(repo, &["ls-tree", "-r", "HEAD"]);
    assert_ok(&ls, "git ls-tree");
    let s = String::from_utf8_lossy(&ls.stdout);
    assert!(s.contains("src/main.rs"), "ls-tree: {s}");
}
