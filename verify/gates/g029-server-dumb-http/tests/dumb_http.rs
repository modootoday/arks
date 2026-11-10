//! Gate G29 — dumb HTTP additive server path vs stock Git.
//! Cite: verify/matrices/matrix-g029-server-dumb-http.md, gitprotocol-http dumb section.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

fn require_git() {
    let out = Command::new("git").arg("--version").output().expect("git");
    assert!(out.status.success(), "stock git required for G29");
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
        candidates.push(root.join("target-cleanroom/debug/arks"));
    }
    for c in &candidates {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!("missing arks binary; tried {candidates:?}");
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
        "{label}: status={:?}\nstdout={}\nstderr={}",
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

struct ArksDumbHttp {
    child: Child,
    url: String,
}

impl ArksDumbHttp {
    fn start(bare: &Path) -> Self {
        let port = free_port();
        let listen = format!("127.0.0.1:{port}");
        let child = Command::new(arks_bin())
            .args([
                "serve-http",
                "--dumb-only",
                "--listen",
                &listen,
                bare.to_str().unwrap(),
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn serve-http --dumb-only");
        let url = format!("http://{listen}/");
        for _ in 0..50 {
            if let Ok(mut s) = std::net::TcpStream::connect(format!("127.0.0.1:{port}")) {
                use std::io::Write;
                let _ = write!(s, "GET /info/refs HTTP/1.0\r\nHost: 127.0.0.1\r\n\r\n");
                thread::sleep(Duration::from_millis(30));
                break;
            }
            thread::sleep(Duration::from_millis(40));
        }
        Self { child, url }
    }
}

impl Drop for ArksDumbHttp {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn seed_bare(root: &Path) -> PathBuf {
    let work = root.join("seed");
    fs::create_dir_all(&work).unwrap();
    assert_ok(&arks(&work, &["init"]), "init");
    fs::write(work.join("hello.txt"), b"hello g029-server-dumb-http\n").unwrap();
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
        "bare fixture",
    );
    assert_ok(
        &arks(root, &["update-server-info", bare.to_str().unwrap()]),
        "update-server-info",
    );
    bare
}

#[test]
fn g029-server-dumb-http_update_server_info_files() {
    require_git();
    let root = TempDir::new().unwrap();
    let bare = seed_bare(root.path());
    let refs = fs::read_to_string(bare.join("info/refs")).unwrap();
    assert!(refs.contains("refs/heads/"), "info/refs missing heads: {refs}");
    assert!(
        refs.contains('\t'),
        "info/refs must be TAB-separated: {refs}"
    );
    assert!(bare.join("objects/info/packs").is_file());
}

#[test]
fn g029-server-dumb-http_stock_git_clone_dumb_http() {
    require_git();
    let root = TempDir::new().unwrap();
    let bare = seed_bare(root.path());
    let server = ArksDumbHttp::start(&bare);
    let clone = root.path().join("clone");
    let out = git(
        root.path(),
        &["clone", &server.url, clone.to_str().unwrap()],
    );
    assert_ok(&out, "git clone via arks dumb HTTP");
    let body = fs::read_to_string(clone.join("hello.txt")).unwrap();
    assert!(
        body.replace("\r\n", "\n").starts_with("hello g029-server-dumb-http"),
        "unexpected: {body:?}"
    );
}
