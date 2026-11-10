//! Gate G12 — Wave Pa porcelain argv suite vs stock Git peer (clean-room).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

fn require_git() {
    let out = Command::new("git").arg("--version").output().expect("git");
    assert!(out.status.success());
}

fn arks_bin() -> PathBuf {
    // Clean-room compose sets CARGO_TARGET_DIR; host advisory uses workspace target.
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(target) = std::env::var("CARGO_TARGET_DIR") {
        candidates.push(PathBuf::from(&target).join("debug/arks"));
        candidates.push(PathBuf::from(&target).join("debug/arks.exe"));
    } else {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
        candidates.push(root.join("target/debug/arks.exe"));
        candidates.push(root.join("target/debug/arks"));
        candidates.push(root.join("target-cleanroom/debug/arks"));
    }
    for c in &candidates {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!("missing arks binary; tried {candidates:?}");
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
        .env("GIT_AUTHOR_NAME", "arks")
        .env("GIT_AUTHOR_EMAIL", "arks@example.com")
        .env("GIT_COMMITTER_NAME", "arks")
        .env("GIT_COMMITTER_EMAIL", "arks@example.com")
        .env("GIT_AUTHOR_DATE", "1000000000 +0000")
        .env("GIT_COMMITTER_DATE", "1000000000 +0000")
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

fn seed_commit(repo: &Path, file: &str, body: &str, msg: &str) {
    fs::write(repo.join(file), body).unwrap();
    assert_ok(&arks(repo, &["add", file]), "add");
    assert_ok(&arks(repo, &["commit", "-m", msg]), "commit");
}

#[test]
fn help_lists_wave_pa_and_omits_deferred() {
    require_git();
    let out = Command::new(arks_bin()).arg("--help").output().unwrap();
    assert_ok(&out, "help");
    let s = String::from_utf8_lossy(&out.stdout);
    for cmd in [
        "branch", "checkout", "switch", "restore", "reset", "diff", "merge", "rm", "mv", "clean",
        "tag", "describe", "shortlog", "config", "grep", "stash", "pull",
    ] {
        assert!(s.contains(cmd), "help missing {cmd}");
    }
    // M15 Fs-3 claimed `repack`; still omit product exclusions.
    let listed_repack = s.lines().any(|l| {
        let t = l.trim_start();
        t.starts_with("repack") && t["repack".len()..].starts_with(char::is_whitespace)
    });
    assert!(listed_repack, "help missing claimed residual repack");
    for bad in ["filter-branch", "backfill"] {
        let listed = s.lines().any(|l| {
            let t = l.trim_start();
            t.starts_with(bad) && t[bad.len()..].starts_with(char::is_whitespace)
        });
        assert!(!listed, "help must not list excluded command {bad}");
    }
    // `worktree` / `reflog` / `clean -d` may be claimed at residual/Pc; Pa suite no longer requires omission/refuse.
}

#[test]
fn refuse_deferred_and_unclaimed_flags() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path();
    assert_ok(&arks(repo, &["init"]), "init");
    // Bare `repack` without -a remains usage-fail; -ad is claimed (Fs-3).
    assert_fail(&arks(repo, &["repack"]), "repack without -a");
    seed_commit(repo, "a.txt", "one\n", "c1");
    assert_ok(&arks(repo, &["repack", "-ad"]), "repack -ad");
    // Fs-1 `--no-ff` + Pc `clean -x` claimed; keep product exclusions refuse-closed.
    assert_ok(&arks(repo, &["merge", "--no-ff", "main"]), "merge --no-ff up-to-date");
    assert_ok(&arks(repo, &["clean", "-x", "-f"]), "clean -x");
    assert_fail(&arks(repo, &["filter-branch"]), "filter-branch");
}

#[test]
fn branch_checkout_switch_roundtrip() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path();
    assert_ok(&arks(repo, &["init"]), "init");
    seed_commit(repo, "a.txt", "one\n", "c1");
    assert_ok(&arks(repo, &["branch", "topic"]), "branch topic");
    assert_ok(&arks(repo, &["checkout", "topic"]), "checkout topic");
    seed_commit(repo, "b.txt", "two\n", "c2");
    assert_ok(&arks(repo, &["switch", "main"]), "switch main");
    assert!(!repo.join("b.txt").exists());
    let list = arks(repo, &["branch"]);
    assert_ok(&list, "branch list");
    let s = String::from_utf8_lossy(&list.stdout);
    assert!(s.contains("main"));
    assert!(s.contains("topic"));
    // Stock Git opens the repo.
    assert_ok(&git(repo, &["status"]), "git status peer");
}

#[test]
fn reset_hard_and_diff_dirty() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path();
    assert_ok(&arks(repo, &["init"]), "init");
    seed_commit(repo, "a.txt", "one\n", "c1");
    fs::write(repo.join("a.txt"), "two\n").unwrap();
    let d = arks(repo, &["diff"]);
    assert_ok(&d, "diff");
    assert!(String::from_utf8_lossy(&d.stdout).contains("two"));
    assert_ok(&arks(repo, &["reset", "--hard", "HEAD"]), "reset --hard");
    assert_eq!(fs::read_to_string(repo.join("a.txt")).unwrap(), "one\n");
}

#[test]
fn merge_ff_only() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path();
    assert_ok(&arks(repo, &["init"]), "init");
    seed_commit(repo, "a.txt", "one\n", "c1");
    assert_ok(&arks(repo, &["branch", "topic"]), "branch");
    assert_ok(&arks(repo, &["checkout", "topic"]), "checkout");
    seed_commit(repo, "b.txt", "two\n", "c2");
    assert_ok(&arks(repo, &["checkout", "main"]), "back main");
    assert_ok(&arks(repo, &["merge", "--ff-only", "topic"]), "ff merge");
    assert!(repo.join("b.txt").is_file());
}

#[test]
fn rm_mv_clean_tag_describe_config_grep_stash() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path();
    assert_ok(&arks(repo, &["init"]), "init");
    seed_commit(repo, "a.txt", "hello grepme\n", "c1");
    assert_ok(&arks(repo, &["tag", "v1"]), "tag");
    let desc = arks(repo, &["describe", "--tags"]);
    assert_ok(&desc, "describe");
    assert!(String::from_utf8_lossy(&desc.stdout).contains("v1"));
    assert_ok(&arks(repo, &["config", "--set", "user.localtest", "x"]), "config set");
    let g = arks(repo, &["config", "--get", "user.localtest"]);
    assert_ok(&g, "config get");
    assert!(String::from_utf8_lossy(&g.stdout).contains('x'));
    let gr = arks(repo, &["grep", "grepme"]);
    assert_ok(&gr, "grep");
    assert!(String::from_utf8_lossy(&gr.stdout).contains("grepme"));
    assert_ok(&arks(repo, &["mv", "a.txt", "c.txt"]), "mv");
    assert_ok(&arks(repo, &["commit", "-m", "mv"]), "commit mv");
    fs::write(repo.join("junk.txt"), "u").unwrap();
    assert_ok(&arks(repo, &["clean", "-f"]), "clean");
    assert!(!repo.join("junk.txt").exists());
    fs::write(repo.join("c.txt"), "dirty\n").unwrap();
    assert_ok(&arks(repo, &["stash", "push"]), "stash push");
    assert_ok(&arks(repo, &["stash", "list"]), "stash list");
    assert_ok(&arks(repo, &["stash", "pop"]), "stash pop");
    assert_ok(&arks(repo, &["rm", "c.txt"]), "rm");
    assert!(!repo.join("c.txt").exists());
}

#[test]
fn restore_and_shortlog() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path();
    assert_ok(&arks(repo, &["init"]), "init");
    seed_commit(repo, "a.txt", "one\n", "c1");
    fs::write(repo.join("a.txt"), "changed\n").unwrap();
    assert_ok(&arks(repo, &["restore", "a.txt"]), "restore");
    assert_eq!(fs::read_to_string(repo.join("a.txt")).unwrap(), "one\n");
    let sl = arks(repo, &["shortlog"]);
    assert_ok(&sl, "shortlog");
    assert!(String::from_utf8_lossy(&sl.stdout).contains("arks"));
}
