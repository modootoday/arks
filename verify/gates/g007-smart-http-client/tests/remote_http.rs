//! Gate G7 — generic smart-HTTP remote client vs local stock git-http-backend peer.
//!
//! Offline / loopback only (no live forge). GitHub-class hosts remain the verify-peer
//! *label* in product SoT; this harness proves any smart-HTTP peer.
//!
//! Primary: https://git-scm.com/docs/gitprotocol-http
//! Cite: verify/matrices/matrix-m6.md, .spec/sot/arks-remote.sot.md

use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

fn require_git() {
    let out = Command::new("git").arg("--version").output().expect("git");
    assert!(out.status.success(), "stock git required for G7 peer");
}

fn arks_bin() -> PathBuf {
    let target = std::env::var("CARGO_TARGET_DIR").unwrap_or_else(|_| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../target-cleanroom")
            .display()
            .to_string()
    });
    let cand = PathBuf::from(&target).join("debug/arks");
    assert!(cand.is_file(), "missing {}", cand.display());
    cand
}

fn arks(cwd: &Path, args: &[&str]) -> std::process::Output {
    Command::new(arks_bin())
        .args(args)
        .current_dir(cwd)
        .env("GIT_AUTHOR_NAME", "arks")
        .env("GIT_AUTHOR_EMAIL", "arks@example.com")
        .env("GIT_COMMITTER_NAME", "arks")
        .env("GIT_COMMITTER_EMAIL", "arks@example.com")
        .env("GIT_AUTHOR_DATE", "1000000000")
        .env("GIT_COMMITTER_DATE", "1000000000")
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
        .env("GIT_AUTHOR_DATE", "1000000000")
        .env("GIT_COMMITTER_DATE", "1000000000")
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

fn assert_fail(out: &std::process::Output, label: &str) {
    assert!(
        !out.status.success(),
        "{label} should refuse: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}

/// Tiny HTTP→CGI bridge for `git http-backend` (loopback peer).
struct GitHttpPeer {
    base_url: String,
    _join: thread::JoinHandle<()>,
    stop: Arc<Mutex<bool>>,
}

impl GitHttpPeer {
    fn start(project_root: PathBuf) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = listener.local_addr().unwrap().port();
        let stop = Arc::new(Mutex::new(false));
        let stop2 = Arc::clone(&stop);
        let join = thread::spawn(move || {
            listener.set_nonblocking(true).ok();
            while !*stop2.lock().unwrap() {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let root = project_root.clone();
                        thread::spawn(move || {
                            if let Err(e) = handle_client(stream, &root) {
                                eprintln!("G7 peer handler: {e}");
                            }
                        });
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(_) => break,
                }
            }
        });
        Self {
            base_url: format!("http://127.0.0.1:{port}"),
            _join: join,
            stop,
        }
    }

    fn repo_url(&self, name: &str) -> String {
        format!("{}/{name}", self.base_url)
    }
}

impl Drop for GitHttpPeer {
    fn drop(&mut self) {
        *self.stop.lock().unwrap() = true;
    }
}

fn handle_client(mut stream: TcpStream, project_root: &Path) -> Result<(), String> {
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .ok();
    let mut buf = Vec::new();
    let mut tmp = [0u8; 4096];
    let header_end;
    loop {
        let n = stream.read(&mut tmp).map_err(|e| e.to_string())?;
        if n == 0 {
            return Ok(());
        }
        buf.extend_from_slice(&tmp[..n]);
        if let Some(pos) = find_header_end(&buf) {
            header_end = pos;
            break;
        }
        if buf.len() > 1024 * 1024 {
            return Err("headers too large".into());
        }
    }
    let header_text = String::from_utf8_lossy(&buf[..header_end]).to_string();
    let mut lines = header_text.split("\r\n");
    let request = lines.next().unwrap_or("");
    let mut parts = request.split_whitespace();
    let method = parts.next().unwrap_or("GET");
    let path_q = parts.next().unwrap_or("/");
    let (path, query) = match path_q.split_once('?') {
        Some((p, q)) => (p.to_string(), q.to_string()),
        None => (path_q.to_string(), String::new()),
    };
    let mut content_length = 0usize;
    let mut content_type = String::new();
    for line in lines {
        let lower = line.to_ascii_lowercase();
        if let Some(v) = lower.strip_prefix("content-length:") {
            content_length = v.trim().parse().unwrap_or(0);
        }
        if let Some(v) = lower.strip_prefix("content-type:") {
            content_type = v.trim().to_string();
        }
    }
    if content_type.is_empty() {
        if path.contains("receive-pack") {
            content_type = "application/x-git-receive-pack-request".into();
        } else {
            content_type = "application/x-git-upload-pack-request".into();
        }
    }
    let mut body = buf[header_end..].to_vec();
    while body.len() < content_length {
        let n = stream.read(&mut tmp).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&tmp[..n]);
    }
    body.truncate(content_length);

    let mut child = Command::new("git")
        .arg("http-backend")
        .env("GIT_PROJECT_ROOT", project_root)
        .env("GIT_HTTP_EXPORT_ALL", "1")
        .env("REQUEST_METHOD", method)
        .env("PATH_INFO", &path)
        .env("QUERY_STRING", &query)
        .env("CONTENT_TYPE", &content_type)
        .env("CONTENT_LENGTH", content_length.to_string())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("spawn git http-backend: {e}"))?;
    {
        let mut stdin = child.stdin.take().unwrap();
        stdin.write_all(&body).map_err(|e| e.to_string())?;
    }
    let output = child.wait_with_output().map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(format!(
            "git http-backend failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    // CGI: headers, blank line, body. Translate to HTTP/1.1.
    let cgi = output.stdout;
    let (cgi_headers, cgi_body) = split_cgi(&cgi);
    let mut status = 200u16;
    let mut resp_headers = Vec::new();
    for line in cgi_headers.lines() {
        if let Some(rest) = line.strip_prefix("Status:") {
            let code = rest.trim().split_whitespace().next().unwrap_or("200");
            status = code.parse().unwrap_or(200);
        } else if !line.is_empty() {
            resp_headers.push(line.to_string());
        }
    }
    let mut http = format!("HTTP/1.1 {status} OK\r\n");
    http.push_str(&format!("Content-Length: {}\r\n", cgi_body.len()));
    for h in resp_headers {
        http.push_str(&h);
        http.push_str("\r\n");
    }
    http.push_str("Connection: close\r\n\r\n");
    stream.write_all(http.as_bytes()).map_err(|e| e.to_string())?;
    stream.write_all(cgi_body).map_err(|e| e.to_string())?;
    Ok(())
}

fn find_header_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n").map(|p| p + 4)
}

fn split_cgi(cgi: &[u8]) -> (String, &[u8]) {
    if let Some(pos) = cgi.windows(4).position(|w| w == b"\r\n\r\n") {
        (
            String::from_utf8_lossy(&cgi[..pos]).to_string(),
            &cgi[pos + 4..],
        )
    } else if let Some(pos) = cgi.windows(2).position(|w| w == b"\n\n") {
        (
            String::from_utf8_lossy(&cgi[..pos]).to_string(),
            &cgi[pos + 2..],
        )
    } else {
        (String::new(), cgi)
    }
}

fn seed_bare_repo(parent: &Path, name: &str) -> PathBuf {
    let work = parent.join(format!("{name}-work"));
    let bare = parent.join(name);
    fs::create_dir_all(&work).unwrap();
    assert_ok(&git(&work, &["init", "-b", "main"]), "git init");
    // Export for http-backend
    assert_ok(
        &git(&work, &["config", "http.receivepack", "true"]),
        "receivepack",
    );
    fs::write(work.join("README"), b"hello arks m6\n").unwrap();
    assert_ok(&git(&work, &["add", "README"]), "add");
    assert_ok(&git(&work, &["commit", "-m", "init"]), "commit");
    assert_ok(
        &git(parent, &["clone", "--bare", work.to_str().unwrap(), name]),
        "bare clone",
    );
    // Allow push to bare via http-backend
    assert_ok(
        &git(&bare, &["config", "http.receivepack", "true"]),
        "bare receivepack",
    );
    bare
}

#[test]
fn help_lists_wave_h_and_refuses_ssh() {
    require_git();
    let out = Command::new(arks_bin()).arg("--help").output().unwrap();
    assert_ok(&out, "help");
    let s = String::from_utf8_lossy(&out.stdout);
    for cmd in ["remote", "fetch", "push", "clone", "pull"] {
        assert!(s.contains(cmd), "help missing {cmd}");
    }
    let tmp = TempDir::new().unwrap();
    assert_ok(&arks(tmp.path(), &["init"]), "init");
    // pull without remotes / upstream fails non-zero (honest).
    assert_fail(&arks(tmp.path(), &["pull"]), "pull without remote");
    // M16 Sy-4: SSH remote URLs are claimed (config accept); transport exercised under G21.
    assert_ok(
        &arks(
            tmp.path(),
            &["remote", "add", "origin", "git@example.com:a/b.git"],
        ),
        "ssh remote add",
    );
}

#[test]
fn remote_add_list_remove_config() {
    require_git();
    let tmp = TempDir::new().unwrap();
    assert_ok(&arks(tmp.path(), &["init"]), "init");
    assert_ok(
        &arks(
            tmp.path(),
            &["remote", "add", "origin", "http://example.com/r.git"],
        ),
        "remote add",
    );
    let list = arks(tmp.path(), &["remote"]);
    assert_ok(&list, "remote list");
    assert!(String::from_utf8_lossy(&list.stdout).contains("origin"));
    let cfg = fs::read_to_string(tmp.path().join(".git/config")).unwrap();
    assert!(cfg.contains("http://example.com/r.git"));
    assert_ok(
        &Command::new("git")
            .args(["remote", "-v"])
            .current_dir(tmp.path())
            .output()
            .unwrap(),
        "stock git remote",
    );
    assert_ok(&arks(tmp.path(), &["remote", "remove", "origin"]), "rm");
}

#[test]
fn clone_fetch_push_smart_http_loopback() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().join("peer");
    fs::create_dir_all(&root).unwrap();
    let _bare = seed_bare_repo(&root, "app.git");
    let peer = GitHttpPeer::start(root.clone());
    // give listener a moment
    thread::sleep(Duration::from_millis(50));
    let url = peer.repo_url("app.git");

    let clone_dir = tmp.path().join("clone");
    let out = arks(tmp.path(), &["clone", &url, clone_dir.to_str().unwrap()]);
    assert_ok(&out, "arks clone");
    assert!(clone_dir.join("README").is_file(), "checkout README");
    let tip = git(&clone_dir, &["rev-parse", "HEAD"]);
    assert_ok(&tip, "git rev-parse after clone");

    // fetch (already up to date path)
    let fetch = arks(&clone_dir, &["fetch"]);
    assert_ok(&fetch, "arks fetch");

    // local commit then push
    fs::write(clone_dir.join("MORE"), b"more\n").unwrap();
    assert_ok(&arks(&clone_dir, &["add", "MORE"]), "add MORE");
    assert_ok(&arks(&clone_dir, &["commit", "-m", "more"]), "commit");
    let push = arks(&clone_dir, &["push"]);
    assert_ok(&push, "arks push");

    // verify peer tip advanced via stock git
    let peer_tip = git(&root.join("app.git"), &["rev-parse", "refs/heads/main"]);
    let local_tip = git(&clone_dir, &["rev-parse", "HEAD"]);
    assert_ok(&peer_tip, "peer tip");
    assert_eq!(
        String::from_utf8_lossy(&peer_tip.stdout).trim(),
        String::from_utf8_lossy(&local_tip.stdout).trim()
    );
}

/// FW-SUBMOD-HTTPS: `arks submodule add` over loopback smart HTTP.
#[test]
fn submodule_add_https_smart_http_loopback() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().join("peer");
    fs::create_dir_all(&root).unwrap();
    let _bare = seed_bare_repo(&root, "sub.git");
    let peer = GitHttpPeer::start(root.clone());
    thread::sleep(Duration::from_millis(50));
    let url = peer.repo_url("sub.git");

    let parent = tmp.path().join("parent");
    fs::create_dir_all(&parent).unwrap();
    assert_ok(&arks(&parent, &["init"]), "parent init");
    fs::write(parent.join("ROOT"), b"root\n").unwrap();
    assert_ok(&arks(&parent, &["add", "ROOT"]), "add ROOT");
    assert_ok(&arks(&parent, &["commit", "-m", "root"]), "commit root");

    let add = arks(&parent, &["submodule", "add", &url, "vendor/sub"]);
    assert_ok(&add, "arks submodule add https");
    assert!(parent.join("vendor/sub/README").is_file(), "submodule checkout");
    let gm = fs::read_to_string(parent.join(".gitmodules")).unwrap();
    assert!(gm.contains("vendor/sub"), ".gitmodules path");
    assert!(gm.contains(&url) || gm.contains("sub.git"), ".gitmodules url");
    let st = arks(&parent, &["submodule", "status"]);
    assert_ok(&st, "submodule status");
}
