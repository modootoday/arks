//! Gate G40 — dumb HTTP push create + WebDAV DELETE (S3 / S6).
//! Cite: verify/matrices/matrix-g040-dumb-http-push-deletes.md, git-http-push, RFC 4918.

use std::fs;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
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
    panic!("missing arks binary");
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

fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

struct DumbServer {
    child: Child,
    url: String,
}

impl Drop for DumbServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn start_dumb_server(bare: &Path) -> DumbServer {
    let port = free_port();
    let listen = format!("127.0.0.1:{port}");
    let child = Command::new(arks_bin())
        .args([
            "serve-http",
            "--listen",
            &listen,
            "--dumb-only",
            bare.to_str().unwrap(),
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("serve-http");
    // Brief settle for accept loop.
    thread::sleep(Duration::from_millis(80));
    DumbServer {
        child,
        url: format!("http://{listen}/"),
    }
}

fn git(cwd: &Path, args: &[&str]) -> std::process::Output {
    Command::new("git")
        .args(args)
        .current_dir(cwd)
        .env("GIT_AUTHOR_NAME", "arks")
        .env("GIT_AUTHOR_EMAIL", "arks@example.com")
        .env("GIT_COMMITTER_NAME", "arks")
        .env("GIT_COMMITTER_EMAIL", "arks@example.com")
        .env("GIT_AUTHOR_DATE", "1000000000 +0000")
        .env("GIT_COMMITTER_DATE", "1000000000 +0000")
        .output()
        .expect("git")
}

fn seed_bare(parent: &Path) -> PathBuf {
    let work = parent.join("seed");
    fs::create_dir_all(&work).unwrap();
    assert_ok(&arks(&work, &["init"]), "init seed");
    fs::write(work.join("README"), b"hello dumb\n").unwrap();
    assert_ok(&arks(&work, &["add", "README"]), "add");
    assert_ok(&arks(&work, &["commit", "-m", "c1"]), "commit");
    let bare = parent.join("repo.git");
    // Stock git creates the bare peer; arks claims dumb push/delete against it.
    assert_ok(
        &git(
            parent,
            &[
                "clone",
                "--bare",
                work.to_str().unwrap(),
                bare.to_str().unwrap(),
            ],
        ),
        "git bare clone",
    );
    assert_ok(&arks(&bare, &["update-server-info"]), "update-server-info");
    bare
}

#[test]
fn g040-dumb-http-push-deletes_dumb_push_topic_then_delete_refspec() {
    require_git();
    let root = TempDir::new().unwrap();
    let bare = seed_bare(root.path());
    let server = start_dumb_server(&bare);
    let clone = root.path().join("clone");
    assert_ok(
        &arks(
            root.path(),
            &["clone", &server.url, clone.to_str().unwrap()],
        ),
        "dumb clone",
    );
    // Create topic tip locally and push.
    fs::write(clone.join("topic.txt"), b"topic\n").unwrap();
    assert_ok(&arks(&clone, &["add", "topic.txt"]), "add topic");
    assert_ok(&arks(&clone, &["commit", "-m", "topic"]), "commit topic");
    assert_ok(
        &arks(&clone, &["checkout", "-b", "topic"]),
        "branch topic",
    );
    assert_ok(&arks(&clone, &["push", "origin", "topic"]), "push topic");
    assert!(
        bare.join("refs/heads/topic").is_file()
            || Command::new(arks_bin())
                .args(["rev-parse", "refs/heads/topic"])
                .current_dir(&bare)
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false),
        "topic must exist on bare after push"
    );
    // Empty-source delete refspec (S3).
    assert_ok(
        &arks(&clone, &["push", "origin", ":topic"]),
        "push :topic delete",
    );
    let gone = Command::new(arks_bin())
        .args(["rev-parse", "refs/heads/topic"])
        .current_dir(&bare)
        .output()
        .expect("rev-parse topic");
    assert!(
        !gone.status.success(),
        "topic must be deleted on bare after WebDAV DELETE"
    );
    // Refuse deleting current HEAD target.
    let del_main = arks(&clone, &["push", "origin", ":main"]);
    assert!(
        !del_main.status.success(),
        "deleting remote HEAD target must fail"
    );
}
