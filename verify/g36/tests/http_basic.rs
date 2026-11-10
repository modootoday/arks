//! Gate G36 — optional HTTP Basic on arks smart HTTP (FW-AUTH-PROXY).
//!
//! Cite: verify/matrix-g36.md, gitprotocol-http, RFC 7617 Basic (summarize),
//! ADR 20261109224236. No forge OAuth.

use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

fn require_git() {
    let out = Command::new("git").arg("--version").output().expect("git");
    assert!(out.status.success(), "stock git required for G36");
}

fn arks_bin() -> PathBuf {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(target) = std::env::var("CARGO_TARGET_DIR") {
        candidates.push(PathBuf::from(&target).join("debug/arks.exe"));
        candidates.push(PathBuf::from(&target).join("debug/arks"));
    } else {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        candidates.push(root.join("target/debug/arks.exe"));
        candidates.push(root.join("target/debug/arks"));
        candidates.push(root.join("target-cleanroom/debug/arks.exe"));
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
        .env("GIT_TERMINAL_PROMPT", "0")
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
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.local_addr().unwrap().port()
}

struct ArksHttp {
    child: Child,
    hostport: String,
    url: String,
}

impl ArksHttp {
    fn start(bare: &Path, extra: &[&str]) -> Self {
        let port = free_port();
        let listen = format!("127.0.0.1:{port}");
        let mut args = vec!["serve-http", "--listen", &listen];
        args.extend_from_slice(extra);
        args.push(bare.to_str().unwrap());
        let child = Command::new(arks_bin())
            .args(&args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn serve-http");
        let hostport = listen.clone();
        let url = format!("http://{listen}/");
        for _ in 0..80 {
            if TcpStream::connect(&listen).is_ok() {
                break;
            }
            thread::sleep(Duration::from_millis(40));
        }
        Self {
            child,
            hostport,
            url,
        }
    }
}

impl Drop for ArksHttp {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn seed_bare_with_arks(root: &Path) -> PathBuf {
    let work = root.join("seed");
    fs::create_dir_all(&work).unwrap();
    assert_ok(&arks(&work, &["init"]), "init");
    fs::write(work.join("hello.txt"), b"hello g36\n").unwrap();
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
        "bare clone fixture",
    );
    bare
}

fn raw_get_info_refs(hostport: &str, authorization: Option<&str>) -> String {
    let mut stream = TcpStream::connect(hostport).expect("connect");
    let mut req = String::from(
        "GET /info/refs?service=git-upload-pack HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n",
    );
    if let Some(a) = authorization {
        req.push_str(&format!("Authorization: {a}\r\n"));
    }
    req.push_str("\r\n");
    stream.write_all(req.as_bytes()).unwrap();
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).unwrap();
    String::from_utf8_lossy(&buf).into_owned()
}

#[test]
fn g36_c01_challenge_without_auth() {
    require_git();
    let root = TempDir::new().unwrap();
    let bare = seed_bare_with_arks(root.path());
    let server = ArksHttp::start(&bare, &["--basic-auth", "alice:s3cret"]);
    let resp = raw_get_info_refs(&server.hostport, None);
    assert!(
        resp.starts_with("HTTP/1.1 401") || resp.contains(" 401 "),
        "expected 401, got: {resp}"
    );
    assert!(
        resp.to_ascii_lowercase()
            .contains("www-authenticate: basic realm=\"arks\""),
        "missing WWW-Authenticate realm=arks: {resp}"
    );
}

#[test]
fn g36_c02_clone_with_url_userinfo() {
    require_git();
    let root = TempDir::new().unwrap();
    let bare = seed_bare_with_arks(root.path());
    let server = ArksHttp::start(&bare, &["--basic-auth", "alice:s3cret"]);
    let clone = root.path().join("clone");
    let url = format!("http://alice:s3cret@{}/", server.hostport);
    let out = git(root.path(), &["clone", &url, clone.to_str().unwrap()]);
    assert_ok(&out, "git clone with URL userinfo");
    let body = fs::read_to_string(clone.join("hello.txt")).unwrap();
    assert!(
        body.replace("\r\n", "\n").starts_with("hello g36"),
        "unexpected clone content: {body:?}"
    );
}

#[test]
fn g36_c03_wrong_then_right_credential() {
    require_git();
    let root = TempDir::new().unwrap();
    let bare = seed_bare_with_arks(root.path());
    let server = ArksHttp::start(&bare, &["--basic-auth", "alice:s3cret"]);

    let bad = root.path().join("bad");
    let bad_url = format!("http://alice:wrong@{}/", server.hostport);
    let out = git(root.path(), &["clone", &bad_url, bad.to_str().unwrap()]);
    assert!(
        !out.status.success(),
        "wrong password must fail clone"
    );

    // Success path: URL userinfo on all hosts; clean-room Linux also exercises
    // a credential.helper script (gitprotocol-http / stock git credential).
    #[cfg(unix)]
    {
        let helper = root.path().join("cred-helper.sh");
        fs::write(
            &helper,
            "#!/bin/sh\necho username=alice\necho password=s3cret\n",
        )
        .unwrap();
        let _ = Command::new("chmod")
            .args(["+x", helper.to_str().unwrap()])
            .status();
        let ok = root.path().join("ok");
        let mut cmd = Command::new("git");
        cmd.args(["clone", &server.url, ok.to_str().unwrap()])
            .current_dir(root.path())
            .env("GIT_AUTHOR_NAME", "arks")
            .env("GIT_AUTHOR_EMAIL", "arks@example.com")
            .env("GIT_COMMITTER_NAME", "arks")
            .env("GIT_COMMITTER_EMAIL", "arks@example.com")
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_CONFIG_COUNT", "1")
            .env("GIT_CONFIG_KEY_0", "credential.helper")
            .env(
                "GIT_CONFIG_VALUE_0",
                format!("!{}", helper.to_str().unwrap()),
            );
        let out = cmd.output().expect("git clone helper");
        assert_ok(&out, "git clone via credential helper");
        let body = fs::read_to_string(ok.join("hello.txt")).unwrap();
        assert!(body.replace("\r\n", "\n").starts_with("hello g36"));
    }
    #[cfg(not(unix))]
    {
        let ok = root.path().join("ok");
        let ok_url = format!("http://alice:s3cret@{}/", server.hostport);
        assert_ok(
            &git(root.path(), &["clone", &ok_url, ok.to_str().unwrap()]),
            "git clone correct userinfo",
        );
    }
}

#[test]
fn g36_c04_unauthenticated_default_still_works() {
    require_git();
    let root = TempDir::new().unwrap();
    let bare = seed_bare_with_arks(root.path());
    let server = ArksHttp::start(&bare, &[]);
    let clone = root.path().join("clone");
    assert_ok(
        &git(
            root.path(),
            &["clone", &server.url, clone.to_str().unwrap()],
        ),
        "unauthenticated clone (G15 shape)",
    );
}
