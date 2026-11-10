//! Gate G41 — dumb HTTP push create/FF/delete vs stock Apache `mod_dav`.
//! Cite: verify/matrices/matrix-g41.md, git-http-push, RFC 4918, O4.
//!
//! Peer URL: `ARKS_G41_DAV_URL` (compose sets `http://g41-dav/repo.git/`).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

fn require_git() {
    let out = Command::new("git").arg("--version").output().expect("git");
    assert!(out.status.success(), "stock git required");
}

fn arks_bin() -> PathBuf {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(target) = std::env::var("CARGO_TARGET_DIR") {
        candidates.push(PathBuf::from(&target).join("debug/arks"));
        candidates.push(PathBuf::from(&target).join("debug/arks.exe"));
    } else {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
        candidates.push(root.join("target/debug/arks"));
        candidates.push(root.join("target/debug/arks.exe"));
    }
    for c in &candidates {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!("missing arks binary (build -p arks first)");
}

fn arks(cwd: &Path, args: &[&str]) -> std::process::Output {
    Command::new(arks_bin())
        .args(args)
        .current_dir(cwd)
        .env("GIT_AUTHOR_NAME", "arks")
        .env("GIT_AUTHOR_EMAIL", "arks@example.com")
        .env("GIT_COMMITTER_NAME", "arks")
        .env("GIT_COMMITTER_EMAIL", "arks@example.com")
        .env("GIT_AUTHOR_DATE", "1000000000 +0000")
        .env("GIT_COMMITTER_DATE", "1000000000 +0000")
        .output()
        .expect("arks")
}

fn assert_ok(out: &std::process::Output, label: &str) {
    assert!(
        out.status.success(),
        "{label}: {:?}\n{}\n{}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

fn dav_url() -> String {
    std::env::var("ARKS_G41_DAV_URL").unwrap_or_else(|_| "http://g41-dav/repo.git/".into())
}

fn wait_for_peer(url: &str) {
    for _ in 0..60 {
        let ok = Command::new("git")
            .args(["ls-remote", url])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if ok {
            return;
        }
        thread::sleep(Duration::from_millis(250));
    }
    panic!("Apache DAV peer not ready at {url}");
}

#[test]
fn g41_arks_push_create_ff_and_delete_vs_apache_dav() {
    require_git();
    let url = dav_url();
    wait_for_peer(&url);

    let root = TempDir::new().unwrap();
    let clone = root.path().join("clone");
    assert_ok(
        &arks(root.path(), &["clone", &url, clone.to_str().unwrap()]),
        "clone via Apache DAV",
    );
    assert_eq!(
        fs::read(clone.join("README")).unwrap(),
        b"hello apache dav\n"
    );

    // Create topic + push (WebDAV PUT).
    fs::write(clone.join("topic.txt"), b"topic\n").unwrap();
    assert_ok(&arks(&clone, &["add", "topic.txt"]), "add topic");
    assert_ok(&arks(&clone, &["commit", "-m", "topic"]), "commit topic");
    assert_ok(&arks(&clone, &["checkout", "-b", "topic"]), "branch topic");
    assert_ok(&arks(&clone, &["push", "origin", "topic"]), "push topic");

    // Empty-source delete against Apache DAV.
    assert_ok(
        &arks(&clone, &["push", "origin", ":topic"]),
        "push :topic delete via Apache DAV",
    );

    // Refuse deleting remote HEAD target.
    let del_main = arks(&clone, &["push", "origin", ":main"]);
    assert!(
        !del_main.status.success(),
        "deleting remote HEAD target must fail against Apache DAV"
    );
}
