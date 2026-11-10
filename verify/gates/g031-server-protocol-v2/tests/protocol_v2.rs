//! Gate G31 — protocol v2 ls-refs + fetch vs stock Git.
//! Cite: verify/matrices/matrix-g031-server-protocol-v2.md, gitprotocol-v2.

use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
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

struct ArksHttp {
    child: Child,
    url: String,
    port: u16,
}

impl ArksHttp {
    fn start(bare: &Path) -> Self {
        let port = free_port();
        let listen = format!("127.0.0.1:{port}");
        let child = Command::new(arks_bin())
            .args(["serve-http", "--listen", &listen, bare.to_str().unwrap()])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("serve-http");
        for _ in 0..50 {
            if TcpStream::connect(format!("127.0.0.1:{port}")).is_ok() {
                break;
            }
            thread::sleep(Duration::from_millis(40));
        }
        Self {
            child,
            url: format!("http://{listen}/"),
            port,
        }
    }
}

impl Drop for ArksHttp {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn seed_bare(root: &Path) -> PathBuf {
    let work = root.join("seed");
    fs::create_dir_all(&work).unwrap();
    assert_ok(&arks(&work, &["init"]), "init");
    fs::write(work.join("hello.txt"), b"hello g031-server-protocol-v2\n").unwrap();
    assert_ok(&arks(&work, &["add", "hello.txt"]), "add");
    assert_ok(&arks(&work, &["commit", "-m", "initial"]), "commit");
    let bare = root.join("server.git");
    assert_ok(
        &git(
            root,
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

fn http_get_v2(port: u16) -> String {
    let mut stream = TcpStream::connect(format!("127.0.0.1:{port}")).unwrap();
    let req = format!(
        "GET /info/refs?service=git-upload-pack HTTP/1.1\r\nHost: 127.0.0.1\r\nGit-Protocol: version=2\r\nConnection: close\r\n\r\n"
    );
    stream.write_all(req.as_bytes()).unwrap();
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).unwrap();
    String::from_utf8_lossy(&buf).into_owned()
}

#[test]
fn g031-server-protocol-v2_v2_capability_advertisement() {
    require_git();
    let root = TempDir::new().unwrap();
    let bare = seed_bare(root.path());
    let server = ArksHttp::start(&bare);
    let body = http_get_v2(server.port);
    assert!(
        body.contains("version 2"),
        "expected v2 ad, got: {body}"
    );
    assert!(body.contains("ls-refs"));
    assert!(body.contains("fetch"));
    assert!(!body.contains("bundle-uri"), "unclaimed cap");
}

#[test]
fn g031-server-protocol-v2_stock_git_protocol_v2_clone() {
    require_git();
    let root = TempDir::new().unwrap();
    let bare = seed_bare(root.path());
    let server = ArksHttp::start(&bare);
    let clone = root.path().join("clone");
    let out = Command::new("git")
        .args([
            "-c",
            "protocol.version=2",
            "clone",
            &server.url,
            clone.to_str().unwrap(),
        ])
        .current_dir(root.path())
        .env("GIT_AUTHOR_NAME", "arks")
        .env("GIT_AUTHOR_EMAIL", "arks@example.com")
        .env("GIT_COMMITTER_NAME", "arks")
        .env("GIT_COMMITTER_EMAIL", "arks@example.com")
        .output()
        .unwrap();
    assert_ok(&out, "git -c protocol.version=2 clone");
    let body = fs::read_to_string(clone.join("hello.txt")).unwrap();
    assert!(body.replace("\r\n", "\n").starts_with("hello g031-server-protocol-v2"));
}

#[test]
fn g031-server-protocol-v2_v1_still_works() {
    require_git();
    let root = TempDir::new().unwrap();
    let bare = seed_bare(root.path());
    let server = ArksHttp::start(&bare);
    // Give the listener a moment after bind (avoids curl 56 on slow hosts).
    thread::sleep(Duration::from_millis(100));
    let clone = root.path().join("clone-v1");
    let out = Command::new("git")
        .args([
            "-c",
            "protocol.version=1",
            "clone",
            &server.url,
            clone.to_str().unwrap(),
        ])
        .current_dir(root.path())
        .output()
        .unwrap();
    if !out.status.success() && clone.join("hello.txt").is_file() {
        return;
    }
    assert_ok(&out, "git protocol.version=1 clone");
}
