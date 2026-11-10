//! Gate G6 — Wave L CLI argv suite vs stock Git peer (clean-room).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

fn require_git() {
    let out = Command::new("git").arg("--version").output().expect("git");
    assert!(out.status.success());
}

fn arks_bin() -> PathBuf {
    let target = std::env::var("CARGO_TARGET_DIR").unwrap_or_else(|_| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target-cleanroom")
            .display()
            .to_string()
    });
    let cand = PathBuf::from(&target).join("debug/arks");
    assert!(cand.is_file(), "missing {}", cand.display());
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
        .expect("arks")
}

fn git(repo: &Path, args: &[&str]) -> std::process::Output {
    Command::new("git")
        .args(args)
        .current_dir(repo)
        .output()
        .expect("git")
}

fn assert_ok(out: &std::process::Output, label: &str) {
    assert!(
        out.status.success(),
        "{label}: {}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

fn assert_fail(out: &std::process::Output, label: &str) {
    assert!(
        !out.status.success(),
        "{label} should refuse: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}

#[test]
fn help_lists_wave_l_and_h() {
    require_git();
    let out = Command::new(arks_bin()).arg("--help").output().unwrap();
    assert_ok(&out, "help");
    let s = String::from_utf8_lossy(&out.stdout);
    for cmd in [
        "init",
        "hash-object",
        "cat-file",
        "update-ref",
        "rev-parse",
        "add",
        "status",
        "commit",
        "log",
        "show",
        "remote",
        "fetch",
        "push",
        "clone",
        "pull",
        "branch",
        "checkout",
        "merge",
    ] {
        assert!(s.contains(cmd), "help missing {cmd}");
    }
    // `worktree` may appear once residual/Pc claims it; G6 no longer requires omission.
}

#[test]
fn refuse_unimplemented_command_and_flag() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path();
    assert_ok(&arks(repo, &["init"]), "init");
    // Product exclusions remain refuse-closed (historical G6 honesty).
    assert_fail(&arks(repo, &["filter-branch"]), "filter-branch");
    assert_fail(&arks(repo, &["backfill"]), "backfill");
    // M15 Fs-1/Fs-4 claimed amend + add --all (smoke under historical G6 regression).
    fs::write(repo.join("a.txt"), "one\n").unwrap();
    assert_ok(&arks(repo, &["add", "--all"]), "add --all");
    assert_ok(&arks(repo, &["commit", "-m", "c1"]), "commit");
    assert_ok(&arks(repo, &["commit", "--amend", "-m", "c1b"]), "amend");
}

#[test]
fn init_opens_with_stock_git() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path();
    assert_ok(&arks(repo, &["init"]), "arks init");
    let gd = git(repo, &["rev-parse", "--git-dir"]);
    assert_ok(&gd, "git rev-parse --git-dir");
    let st = git(repo, &["status"]);
    assert_ok(&st, "git status on arks-init repo");
}

#[test]
fn hash_object_and_cat_file_parity() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path();
    assert_ok(&arks(repo, &["init"]), "init");
    fs::write(repo.join("blob.txt"), b"hello").unwrap();
    let a = arks(repo, &["hash-object", "-w", "blob.txt"]);
    assert_ok(&a, "arks hash-object -w");
    let a_oid = String::from_utf8_lossy(&a.stdout).trim().to_string();
    let g = git(repo, &["hash-object", "-w", "blob.txt"]);
    assert_ok(&g, "git hash-object -w");
    let g_oid = String::from_utf8_lossy(&g.stdout).trim().to_string();
    assert_eq!(a_oid, g_oid);

    let t = arks(repo, &["cat-file", "-t", &a_oid]);
    assert_ok(&t, "arks cat-file -t");
    assert_eq!(String::from_utf8_lossy(&t.stdout).trim(), "blob");
    let s = arks(repo, &["cat-file", "-s", &a_oid]);
    assert_ok(&s, "arks cat-file -s");
    assert_eq!(String::from_utf8_lossy(&s.stdout).trim(), "5");
}

#[test]
fn update_ref_and_rev_parse() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path();
    assert_ok(&arks(repo, &["init"]), "init");
    fs::write(repo.join("f"), b"x").unwrap();
    assert_ok(&arks(repo, &["add", "f"]), "add");
    assert_ok(&arks(repo, &["commit", "-m", "c"]), "commit");
    let tip = arks(repo, &["rev-parse", "HEAD"]);
    assert_ok(&tip, "arks rev-parse HEAD");
    let tip_hex = String::from_utf8_lossy(&tip.stdout).trim().to_string();
    let g = git(repo, &["rev-parse", "HEAD"]);
    assert_ok(&g, "git rev-parse");
    assert_eq!(tip_hex, String::from_utf8_lossy(&g.stdout).trim());

    let show_ref = git(repo, &["show-ref", "--verify", "refs/heads/main"]);
    assert_ok(&show_ref, "git show-ref");
    assert!(String::from_utf8_lossy(&show_ref.stdout).contains(&tip_hex));
}

#[test]
fn add_status_commit_log_show_suite() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path();
    assert_ok(&arks(repo, &["init"]), "init");
    fs::write(repo.join("readme"), b"wave-l").unwrap();
    let st0 = arks(repo, &["status"]);
    assert_ok(&st0, "status untracked");
    assert!(String::from_utf8_lossy(&st0.stdout).contains("readme"));

    assert_ok(&arks(repo, &["add", "readme"]), "add");
    assert_ok(&arks(repo, &["commit", "-m", "wave-l"]), "commit");
    let log = arks(repo, &["log"]);
    assert_ok(&log, "log");
    assert!(String::from_utf8_lossy(&log.stdout).contains("wave-l"));
    let show = arks(repo, &["show"]);
    assert_ok(&show, "show");
    assert!(String::from_utf8_lossy(&show.stdout).contains("wave-l"));

    let gl = git(repo, &["log", "--oneline"]);
    assert_ok(&gl, "git log peer");
    assert!(String::from_utf8_lossy(&gl.stdout).contains("wave-l"));
}
