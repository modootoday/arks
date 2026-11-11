//! Gate G32 — `git://` daemon vs stock Git.
//! Cite: verify/matrices/matrix-g032-git-daemon-wire.md, git-daemon, gitprotocol-pack.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

fn require_git() {
    assert!(
        Command::new("git")
            .arg("--version")
            .output()
            .unwrap()
            .status
            .success()
    );
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
    panic!("missing arks");
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
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

struct ArksDaemon {
    child: Child,
    url: String,
}

impl ArksDaemon {
    fn start(base: &Path, export_all: bool) -> Self {
        let port = free_port();
        let listen = format!("127.0.0.1:{port}");
        let mut args = vec![
            "daemon".to_string(),
            "--listen".into(),
            listen.clone(),
        ];
        if export_all {
            args.push("--export-all".into());
        }
        args.push(base.to_str().unwrap().into());
        let child = Command::new(arks_bin())
            .args(&args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("daemon");
        for _ in 0..50 {
            if std::net::TcpStream::connect(format!("127.0.0.1:{port}")).is_ok() {
                break;
            }
            thread::sleep(Duration::from_millis(40));
        }
        Self {
            child,
            url: format!("git://{listen}/server.git"),
        }
    }
}

impl Drop for ArksDaemon {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn seed_bare_under(base: &Path) -> PathBuf {
    let work = base.join("seed");
    fs::create_dir_all(&work).unwrap();
    assert_ok(&arks(&work, &["init"]), "init");
    fs::write(work.join("hello.txt"), b"hello g032-git-daemon-wire\n").unwrap();
    assert_ok(&arks(&work, &["add", "hello.txt"]), "add");
    assert_ok(&arks(&work, &["commit", "-m", "initial"]), "commit");
    let bare = base.join("server.git");
    assert_ok(
        &git(
            base,
            &[
                "clone",
                "--bare",
                work.join(".git").to_str().unwrap(),
                bare.to_str().unwrap(),
            ],
        ),
        "bare",
    );
    bare
}

#[test]
fn g032_git_daemon_wire_clone_with_export_all() {
    require_git();
    let root = TempDir::new().unwrap();
    let _bare = seed_bare_under(root.path());
    let daemon = ArksDaemon::start(root.path(), true);
    let clone = root.path().join("clone");
    let out = git(
        root.path(),
        &["clone", &daemon.url, clone.to_str().unwrap()],
    );
    assert_ok(&out, "git clone git:// via arks daemon");
    let body = fs::read_to_string(clone.join("hello.txt")).unwrap();
    assert!(body.replace("\r\n", "\n").starts_with("hello g032-git-daemon-wire"));
}

#[test]
fn g032_git_daemon_wire_refuse_without_export_marker() {
    require_git();
    let root = TempDir::new().unwrap();
    let _bare = seed_bare_under(root.path());
    let daemon = ArksDaemon::start(root.path(), false);
    let clone = root.path().join("clone");
    let out = git(
        root.path(),
        &["clone", &daemon.url, clone.to_str().unwrap()],
    );
    assert!(
        !out.status.success(),
        "clone must fail without export-ok / --export-all"
    );
}
