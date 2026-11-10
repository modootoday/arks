//! Offline client proofs: recorded script, in-memory peer, and loopback HTTP.

use crate::clone::{clone_http, clone_http_with, clone_local};
use crate::error::RemoteError;
use crate::fetch::{FetchOptions, FetchOutcome, fetch_with};
use crate::mock::MemoryPeer;
use crate::push::{PushUpdate, push_http, push_with};
use crate::transport::{HttpRequest, HttpResponse, RecordedExchange, RecordedTransport};
use arks_object::{Commit, ObjectKind, RawObject, Signature, Tree, TreeEntry, TreeMode};
use arks_oid::Oid;
use arks_repo::Repository;
use arks_store::{ObjectFormat, ObjectStore};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

const FORMAT: ObjectFormat = ObjectFormat::Sha1;

fn empty_repo(dir: &Path) -> Repository {
    let git = dir.join(".git");
    std::fs::create_dir_all(git.join("objects")).unwrap();
    std::fs::create_dir_all(git.join("refs/heads")).unwrap();
    std::fs::write(git.join("HEAD"), b"ref: refs/heads/main\n").unwrap();
    std::fs::write(
        git.join("config"),
        b"[core]\n\trepositoryformatversion = 0\n\tbare = false\n",
    )
    .unwrap();
    Repository::open(git).unwrap()
}

fn req(method: &str, url: &str) -> HttpRequest {
    HttpRequest {
        method: method.into(),
        url: url.into(),
        headers: Vec::new(),
        body: Vec::new(),
    }
}

#[test]
fn recorded_script_fetch_stores_advertised_commit() {
    let mut peer = MemoryPeer::new();
    let oid = peer
        .seed_file("refs/heads/main", "README", b"hello arks m6\n", "init")
        .unwrap();
    let base = "http://recorded.test/repo.git";
    let info = peer
        .handle(&req(
            "GET",
            &format!("{base}/info/refs?service=git-upload-pack"),
        ))
        .unwrap();
    let post = peer
        .handle(&req("POST", &format!("{base}/git-upload-pack")))
        .unwrap();
    let mut recorded = RecordedTransport::new(vec![
        RecordedExchange {
            method: "GET".into(),
            url_contains: "git-upload-pack".into(),
            status: info.status,
            headers: info.headers,
            body: info.body,
        },
        RecordedExchange {
            method: "POST".into(),
            url_contains: "/git-upload-pack".into(),
            status: post.status,
            headers: post.headers,
            body: post.body,
        },
    ]);
    let tmp = tempfile::TempDir::new().unwrap();
    let mut repo = empty_repo(tmp.path());
    let outcome: FetchOutcome = fetch_with(
        &mut recorded,
        &mut repo,
        base,
        "origin",
        &FetchOptions::default(),
    )
    .unwrap();
    assert_eq!(outcome.head_oid, Some(oid));
    assert_eq!(outcome.head_branch.as_deref(), Some("refs/heads/main"));
    let stored = repo.get_object(&oid).unwrap();
    assert_eq!(stored.kind, ObjectKind::Commit);
    assert_eq!(recorded.requests.len(), 2);
    let tracking = repo.refs().resolve("refs/remotes/origin/main").unwrap();
    assert_eq!(tracking, oid);
}

#[test]
fn memory_peer_push_then_fetch() {
    let mut peer = MemoryPeer::new();
    let first = peer
        .seed_file("refs/heads/main", "README", b"one\n", "one")
        .unwrap();
    let tmp = tempfile::TempDir::new().unwrap();
    let mut repo = empty_repo(tmp.path());
    let outcome = fetch_with(
        &mut peer,
        &mut repo,
        "http://memory.test/repo.git",
        "origin",
        &FetchOptions::default(),
    )
    .unwrap();
    // fetch updates remote-tracking refs only; the local branch is clone's job.
    let tip = outcome.head_oid.unwrap();
    repo.refs_mut()
        .update("refs/heads/main", &tip, None)
        .unwrap();
    let second = commit_file(&mut repo, "README", b"two\n", "two").unwrap();
    assert_ne!(first, second);
    push_with(
        &mut peer,
        &repo,
        "http://memory.test/repo.git",
        &[PushUpdate {
            refname: "refs/heads/main".into(),
            new_oid: second,
        }],
    )
    .unwrap();
    assert_eq!(peer.ref_oid("refs/heads/main"), Some(second));
    let blob = peer.object(&readme_oid(&peer, second).unwrap()).unwrap();
    assert_eq!(blob.payload, b"two\n");

    peer.reject_push = true;
    let err = push_with(
        &mut peer,
        &repo,
        "http://memory.test/repo.git",
        &[PushUpdate {
            refname: "refs/heads/other".into(),
            new_oid: second,
        }],
    )
    .unwrap_err();
    assert!(matches!(err, RemoteError::Rejected(_)));
}

#[test]
fn loopback_clone_and_push() {
    let peer = Arc::new(Mutex::new(MemoryPeer::new()));
    peer.lock()
        .unwrap()
        .seed_file("refs/heads/main", "README", b"hello arks m6\n", "init")
        .unwrap();
    let server = TestServer::spawn(Arc::clone(&peer));
    let tmp = tempfile::TempDir::new().unwrap();
    let dest = tmp.path().join("clone");
    clone_http(&server.url, &dest).unwrap();
    let readme = std::fs::read(dest.join("README")).unwrap();
    assert_eq!(readme, b"hello arks m6\n");
    assert!(dest.join(".git/index").is_file());

    let mut repo = Repository::discover(&dest).unwrap();
    let new_oid = commit_file(&mut repo, "README", b"updated\n", "update").unwrap();
    push_http(
        &repo,
        &server.url,
        &[PushUpdate {
            refname: "refs/heads/main".into(),
            new_oid,
        }],
    )
    .unwrap();
    let peer = peer.lock().unwrap();
    assert_eq!(peer.ref_oid("refs/heads/main"), Some(new_oid));
}

#[test]
fn local_path_and_file_url_clone() {
    let tmp = tempfile::TempDir::new().unwrap();
    let src = tmp.path().join("src");
    std::fs::create_dir_all(&src).unwrap();
    let mut repo = empty_repo(&src);
    // Root commit: empty_repo has no tip yet — seed first commit without parent.
    let blob = RawObject::new(ObjectKind::Blob, b"hello local clone\n");
    let blob_oid = repo.store_mut().put(FORMAT, &blob).unwrap();
    let tree = Tree::new(vec![TreeEntry {
        mode: TreeMode::File,
        name: "README".into(),
        oid: blob_oid,
    }])
    .unwrap();
    let tree_oid = repo
        .store_mut()
        .put(
            FORMAT,
            &RawObject::new(ObjectKind::Tree, tree.encode().unwrap()),
        )
        .unwrap();
    let sig = Signature {
        name: "arks".into(),
        email: "arks@example.com".into(),
        timestamp: 1_000_000_000,
        timezone: "+0000".into(),
    };
    let commit = Commit {
        tree: tree_oid,
        parents: vec![],
        author: sig.clone(),
        committer: sig,
        gpgsig: None,
        message: "init\n".into(),
    };
    let tip = repo
        .store_mut()
        .put(
            FORMAT,
            &RawObject::new(ObjectKind::Commit, commit.encode().unwrap()),
        )
        .unwrap();
    repo.refs_mut()
        .update("refs/heads/main", &tip, None)
        .unwrap();

    let dest_path = tmp.path().join("clone-path");
    clone_local(src.to_str().unwrap(), &dest_path).unwrap();
    assert_eq!(
        std::fs::read(dest_path.join("README")).unwrap(),
        b"hello local clone\n"
    );

    let dest_file = tmp.path().join("clone-file");
    let abs = src.canonicalize().unwrap();
    let abs_s = abs.to_string_lossy().replace('\\', "/");
    let file_url = if abs_s.starts_with('/') {
        format!("file://{abs_s}")
    } else {
        format!("file:///{abs_s}")
    };
    clone_local(&file_url, &dest_file).unwrap();
    assert_eq!(
        std::fs::read(dest_file.join("README")).unwrap(),
        b"hello local clone\n"
    );
}

fn commit_file(
    repo: &mut Repository,
    path: &str,
    bytes: &[u8],
    message: &str,
) -> Result<Oid, RemoteError> {
    let blob = RawObject::new(ObjectKind::Blob, bytes.to_vec());
    let blob_oid = repo
        .store_mut()
        .put(FORMAT, &blob)
        .map_err(|e| RemoteError::Msg(e.to_string()))?;
    let tree = Tree::new(vec![TreeEntry {
        mode: TreeMode::File,
        name: path.to_string(),
        oid: blob_oid,
    }])
    .map_err(|e| RemoteError::Msg(e.to_string()))?;
    let tree_obj = RawObject::new(
        ObjectKind::Tree,
        tree.encode().map_err(|e| RemoteError::Msg(e.to_string()))?,
    );
    let tree_oid = repo
        .store_mut()
        .put(FORMAT, &tree_obj)
        .map_err(|e| RemoteError::Msg(e.to_string()))?;
    let parent = repo
        .head_oid()
        .map_err(|e| RemoteError::Msg(e.to_string()))?;
    let sig = Signature {
        name: "arks".into(),
        email: "arks@example.com".into(),
        timestamp: 1_000_000_000,
        timezone: "+0000".into(),
    };
    let message = if message.ends_with('\n') {
        message.to_string()
    } else {
        format!("{message}\n")
    };
    let commit = Commit {
        tree: tree_oid,
        parents: vec![parent],
        author: sig.clone(),
        committer: sig,
        gpgsig: None,
        message,
    };
    let obj = RawObject::new(
        ObjectKind::Commit,
        commit
            .encode()
            .map_err(|e| RemoteError::Msg(e.to_string()))?,
    );
    let oid = repo
        .store_mut()
        .put(FORMAT, &obj)
        .map_err(|e| RemoteError::Msg(e.to_string()))?;
    repo.refs_mut()
        .update("refs/heads/main", &oid, Some(&parent))
        .map_err(|e| RemoteError::Msg(e.to_string()))?;
    Ok(oid)
}

#[test]
fn stock_git_http_backend_shallow_clone_depth_1() {
    let git_ok = std::process::Command::new("git")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !git_ok {
        return;
    }
    let tmp = tempfile::TempDir::new().unwrap();
    let root = tmp.path().join("peer");
    std::fs::create_dir_all(&root).unwrap();
    let work = root.join("src");
    std::fs::create_dir_all(&work).unwrap();
    git_ok_at(&work, &["init", "-b", "main"]);
    git_ok_at(&work, &["config", "user.email", "git@example.com"]);
    git_ok_at(&work, &["config", "user.name", "Git"]);
    std::fs::write(work.join("README"), b"c1\n").unwrap();
    git_ok_at(&work, &["add", "README"]);
    git_ok_at(&work, &["commit", "-m", "c1"]);
    std::fs::write(work.join("README"), b"c2\n").unwrap();
    git_ok_at(&work, &["add", "README"]);
    git_ok_at(&work, &["commit", "-m", "c2"]);
    std::fs::write(work.join("README"), b"c3\n").unwrap();
    git_ok_at(&work, &["add", "README"]);
    git_ok_at(&work, &["commit", "-m", "c3"]);
    git_ok_at(
        &root,
        &["clone", "--bare", work.to_str().unwrap(), "app.git"],
    );
    let bare = root.join("app.git");
    git_ok_at(&bare, &["config", "http.receivepack", "true"]);

    let server = GitBackend::spawn(root.clone());
    let dest = tmp.path().join("shallow");
    clone_http_with(
        &format!("{}/app.git", server.url),
        &dest,
        &FetchOptions { depth: Some(1) },
    )
    .unwrap();
    assert_eq!(std::fs::read(dest.join("README")).unwrap(), b"c3\n");
    let shallow = dest.join(".git/shallow");
    assert!(
        shallow.is_file(),
        "expected .git/shallow after --depth 1 clone"
    );
    let shallow_text = std::fs::read_to_string(&shallow).unwrap();
    assert!(
        !shallow_text.trim().is_empty(),
        "shallow file must list boundary OID(s)"
    );
    // Depth-1 history: only tip; parent walk stops at shallow boundary.
    let tip = git_stdout(&dest, &["rev-parse", "HEAD"]);
    assert!(shallow_text.contains(&tip));
    let count = git_stdout(&dest, &["rev-list", "--count", "HEAD"]);
    assert_eq!(count, "1");
}

#[test]
fn stock_git_http_backend_clone_fetch_push() {
    let git_ok = std::process::Command::new("git")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !git_ok {
        return;
    }
    let tmp = tempfile::TempDir::new().unwrap();
    let root = tmp.path().join("peer");
    std::fs::create_dir_all(&root).unwrap();
    let work = root.join("src");
    std::fs::create_dir_all(&work).unwrap();
    git_ok_at(&work, &["init", "-b", "main"]);
    git_ok_at(&work, &["config", "user.email", "git@example.com"]);
    git_ok_at(&work, &["config", "user.name", "Git"]);
    std::fs::write(work.join("README"), b"hello arks m6\n").unwrap();
    git_ok_at(&work, &["add", "README"]);
    git_ok_at(&work, &["commit", "-m", "init"]);
    git_ok_at(
        &root,
        &["clone", "--bare", work.to_str().unwrap(), "app.git"],
    );
    let bare = root.join("app.git");
    git_ok_at(&bare, &["config", "http.receivepack", "true"]);

    let server = GitBackend::spawn(root.clone());
    let dest = tmp.path().join("clone");
    clone_http(&format!("{}/app.git", server.url), &dest).unwrap();
    assert_eq!(
        std::fs::read(dest.join("README")).unwrap(),
        b"hello arks m6\n"
    );

    let mut repo = Repository::discover(&dest).unwrap();
    crate::fetch_http(&mut repo, &format!("{}/app.git", server.url), "origin").unwrap();

    let new_oid = commit_file(&mut repo, "README", b"from arks\n", "from arks").unwrap();
    push_http(
        &repo,
        &format!("{}/app.git", server.url),
        &[PushUpdate {
            refname: "refs/heads/main".into(),
            new_oid,
        }],
    )
    .unwrap();
    let tip = git_stdout(&bare, &["rev-parse", "refs/heads/main"]);
    assert_eq!(tip, new_oid.to_hex());
    let blob = git_stdout(&bare, &["cat-file", "-p", "HEAD:README"]);
    assert_eq!(blob, "from arks");
}

fn git_ok_at(cwd: &Path, args: &[&str]) {
    let out = std::process::Command::new("git")
        .args(args)
        .current_dir(cwd)
        .env("GIT_AUTHOR_NAME", "Git")
        .env("GIT_AUTHOR_EMAIL", "git@example.com")
        .env("GIT_COMMITTER_NAME", "Git")
        .env("GIT_COMMITTER_EMAIL", "git@example.com")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn git_stdout(cwd: &Path, args: &[&str]) -> String {
    let out = std::process::Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

struct GitBackend {
    url: String,
    stop: Arc<AtomicBool>,
    join: Option<JoinHandle<()>>,
}

impl GitBackend {
    fn spawn(project_root: std::path::PathBuf) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        listener.set_nonblocking(true).unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let stop2 = Arc::clone(&stop);
        let join = thread::spawn(move || {
            while !stop2.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        // Windows: accepted sockets inherit nonblocking from the listener.
                        stream
                            .set_nonblocking(false)
                            .expect("blocking accepted stream");
                        let root = project_root.clone();
                        thread::spawn(move || {
                            let _ = git_http_one(stream, &root);
                        });
                    }
                    Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(_) => break,
                }
            }
        });
        Self {
            url: format!("http://127.0.0.1:{port}"),
            stop,
            join: Some(join),
        }
    }
}

impl Drop for GitBackend {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

fn git_http_one(mut stream: TcpStream, project_root: &Path) -> Result<(), String> {
    stream.set_read_timeout(Some(Duration::from_secs(30))).ok();
    stream.set_write_timeout(Some(Duration::from_secs(30))).ok();
    let mut buf = Vec::new();
    let mut tmp = [0u8; 8192];
    let header_end = loop {
        let n = stream.read(&mut tmp).map_err(|e| e.to_string())?;
        if n == 0 {
            return Ok(());
        }
        buf.extend_from_slice(&tmp[..n]);
        if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break pos + 4;
        }
    };
    let header = String::from_utf8_lossy(&buf[..header_end]).to_string();
    let mut lines = header.split("\r\n");
    let request = lines.next().unwrap_or("");
    let mut parts = request.split_whitespace();
    let method = parts.next().unwrap_or("GET");
    let path_q = parts.next().unwrap_or("/");
    let (path, query) = path_q.split_once('?').unwrap_or((path_q, ""));
    let mut content_length = 0usize;
    let mut content_type = String::new();
    for line in lines {
        let lower = line.to_ascii_lowercase();
        if let Some(v) = lower.strip_prefix("content-length:") {
            content_length = v.trim().parse().unwrap_or(0);
        }
        if let Some(v) = line.split_once(':') {
            if v.0.eq_ignore_ascii_case("content-type") {
                content_type = v.1.trim().to_string();
            }
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
    let mut child = std::process::Command::new("git")
        .arg("http-backend")
        .env("GIT_PROJECT_ROOT", project_root)
        .env("GIT_HTTP_EXPORT_ALL", "1")
        .env("REQUEST_METHOD", method)
        .env("PATH_INFO", path)
        .env("QUERY_STRING", query)
        .env("CONTENT_TYPE", &content_type)
        .env("CONTENT_LENGTH", content_length.to_string())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&body)
        .map_err(|e| e.to_string())?;
    let output = child.wait_with_output().map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(format!(
            "http-backend: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let cgi = output.stdout;
    let (cgi_headers, cgi_body) = if let Some(pos) = cgi.windows(4).position(|w| w == b"\r\n\r\n") {
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
        (String::new(), cgi.as_slice())
    };
    let mut status = 200u16;
    let mut resp_headers = Vec::new();
    for line in cgi_headers.lines() {
        if let Some(rest) = line.strip_prefix("Status:") {
            status = rest
                .split_whitespace()
                .next()
                .unwrap_or("200")
                .parse()
                .unwrap_or(200);
        } else if !line.is_empty() && !line.to_ascii_lowercase().starts_with("content-length") {
            resp_headers.push(line.to_string());
        }
    }
    let mut http = format!(
        "HTTP/1.1 {status} OK\r\nContent-Length: {}\r\n",
        cgi_body.len()
    );
    for h in resp_headers {
        http.push_str(&h);
        http.push_str("\r\n");
    }
    http.push_str("Connection: close\r\n\r\n");
    stream
        .write_all(http.as_bytes())
        .map_err(|e| e.to_string())?;
    stream.write_all(cgi_body).map_err(|e| e.to_string())?;
    Ok(())
}

fn readme_oid(peer: &MemoryPeer, commit: Oid) -> Option<Oid> {
    let obj = peer.object(&commit)?;
    let commit = Commit::decode(&obj.payload).ok()?;
    let tree = peer.object(&commit.tree)?;
    let tree = Tree::decode(&tree.payload, arks_oid::OidKind::Sha1).ok()?;
    tree.entries
        .iter()
        .find(|e| e.name == "README")
        .map(|e| e.oid)
}

struct TestServer {
    url: String,
    stop: Arc<AtomicBool>,
    join: Option<JoinHandle<()>>,
}

impl TestServer {
    fn spawn(peer: Arc<Mutex<MemoryPeer>>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        // Keep the accept loop blocking with a short accept timeout so Drop can
        // unblock via connecting a dummy client after setting `stop`. Nonblocking
        // accept + inherited nonblocking streams caused hung ureq reads on Windows.
        listener
            .set_nonblocking(false)
            .expect("blocking listener");
        let stop = Arc::new(AtomicBool::new(false));
        let stop2 = Arc::clone(&stop);
        let join = thread::spawn(move || {
            while !stop2.load(Ordering::SeqCst) {
                // Wake periodically so stop can be observed without a client.
                let _ = listener.set_nonblocking(true);
                thread::sleep(Duration::from_millis(20));
                let _ = listener.set_nonblocking(false);
                match listener.accept() {
                    Ok((stream, _)) => {
                        // Accepted sockets may inherit nonblocking from the
                        // listener's brief nonblocking poll window (Windows).
                        stream
                            .set_nonblocking(false)
                            .expect("blocking accepted stream");
                        let peer = Arc::clone(&peer);
                        let _ = serve_one(stream, &peer);
                    }
                    Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => continue,
                    Err(err) if err.kind() == std::io::ErrorKind::TimedOut => continue,
                    Err(err) if err.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(_) => break,
                }
            }
        });
        Self {
            url: format!("http://127.0.0.1:{port}/repo.git"),
            stop,
            join: Some(join),
        }
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        // Unblock a stuck accept: connect then drop.
        if let Ok(addr) = self.url.trim_start_matches("http://").split('/').next().unwrap_or("").parse::<std::net::SocketAddr>() {
            let _ = TcpStream::connect_timeout(&addr, Duration::from_millis(100));
        } else if let Some(port_s) = self.url.rsplit(':').next().and_then(|s| s.split('/').next()) {
            if let Ok(port) = port_s.parse::<u16>() {
                let _ = TcpStream::connect_timeout(
                    &std::net::SocketAddr::from(([127, 0, 0, 1], port)),
                    Duration::from_millis(100),
                );
            }
        }
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

fn serve_one(mut stream: TcpStream, peer: &Mutex<MemoryPeer>) -> Result<(), String> {
    stream.set_read_timeout(Some(Duration::from_secs(30))).ok();
    stream.set_write_timeout(Some(Duration::from_secs(30))).ok();
    let mut buf = Vec::new();
    let mut tmp = [0u8; 8192];
    let header_end = loop {
        let n = stream.read(&mut tmp).map_err(|e| e.to_string())?;
        if n == 0 {
            return Ok(());
        }
        buf.extend_from_slice(&tmp[..n]);
        if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break pos + 4;
        }
        if buf.len() > 1024 * 1024 {
            return Err("headers too large".into());
        }
    };
    let header = String::from_utf8_lossy(&buf[..header_end]).to_string();
    let mut lines = header.split("\r\n");
    let request = lines.next().unwrap_or("");
    let mut parts = request.split_whitespace();
    let method = parts.next().unwrap_or("GET").to_string();
    let target = parts.next().unwrap_or("/").to_string();
    let mut content_length = 0usize;
    for line in lines {
        let lower = line.to_ascii_lowercase();
        if let Some(v) = lower.strip_prefix("content-length:") {
            content_length = v.trim().parse().unwrap_or(0);
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
    let response: HttpResponse = peer
        .lock()
        .map_err(|e| e.to_string())?
        .handle(&HttpRequest {
            method,
            url: target,
            headers: Vec::new(),
            body,
        })
        .map_err(|e| e.to_string())?;
    let mut head = format!("HTTP/1.1 {} OK\r\n", response.status);
    for (k, v) in &response.headers {
        head.push_str(k);
        head.push_str(": ");
        head.push_str(v);
        head.push_str("\r\n");
    }
    head.push_str(&format!("Content-Length: {}\r\n", response.body.len()));
    head.push_str("Connection: close\r\n\r\n");
    stream
        .write_all(head.as_bytes())
        .map_err(|e| e.to_string())?;
    stream
        .write_all(&response.body)
        .map_err(|e| e.to_string())?;
    Ok(())
}
