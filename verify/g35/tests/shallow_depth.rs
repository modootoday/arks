//! Gate G35 — arks `--depth` shallow clone/fetch vs stock `git http-backend`.
//! Cite: verify/matrix-g35.md, git-clone --depth, gitprotocol-capabilities shallow.
//! Coordination: G35 (not G33/G34). Filter/promisor not claimed.

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
    assert!(out.status.success(), "stock git required for G35 peer");
}

fn arks_bin() -> PathBuf {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(target) = std::env::var("CARGO_TARGET_DIR") {
        candidates.push(PathBuf::from(&target).join("debug/arks"));
        candidates.push(PathBuf::from(&target).join("debug/arks.exe"));
    } else {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
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
                            let _ = handle_client(stream, &root);
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
        if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            header_end = pos + 4;
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
        content_type = "application/x-git-upload-pack-request".into();
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

fn seed_chain_bare(parent: &Path, name: &str) -> PathBuf {
    let work = parent.join(format!("{name}-work"));
    fs::create_dir_all(&work).unwrap();
    assert_ok(&git(&work, &["init", "-b", "main"]), "init");
    for (i, body) in ["c1\n", "c2\n", "c3\n"].iter().enumerate() {
        fs::write(work.join("README"), body.as_bytes()).unwrap();
        assert_ok(&git(&work, &["add", "README"]), "add");
        assert_ok(
            &git(&work, &["commit", "-m", &format!("c{}", i + 1)]),
            "commit",
        );
    }
    assert_ok(
        &git(
            parent,
            &["clone", "--bare", work.to_str().unwrap(), name],
        ),
        "bare",
    );
    let bare = parent.join(name);
    assert_ok(
        &git(&bare, &["config", "http.receivepack", "true"]),
        "receivepack",
    );
    bare
}

#[test]
fn g35_clone_depth_1_writes_shallow() {
    require_git();
    let root = TempDir::new().unwrap();
    let _bare = seed_chain_bare(root.path(), "app.git");
    let peer = GitHttpPeer::start(root.path().to_path_buf());
    let dest = root.path().join("shallow");
    let url = peer.repo_url("app.git");
    let out = arks(
        root.path(),
        &["clone", "--depth", "1", &url, dest.to_str().unwrap()],
    );
    assert_ok(&out, "arks clone --depth 1");
    assert_eq!(fs::read(dest.join("README")).unwrap(), b"c3\n");
    let shallow = fs::read_to_string(dest.join(".git/shallow")).expect(".git/shallow");
    assert!(!shallow.trim().is_empty());
    let tip = String::from_utf8_lossy(
        &git(&dest, &["rev-parse", "HEAD"]).stdout,
    )
    .trim()
    .to_string();
    assert!(shallow.contains(&tip), "shallow should list tip {tip}");
    let count = String::from_utf8_lossy(&git(&dest, &["rev-list", "--count", "HEAD"]).stdout)
        .trim()
        .to_string();
    assert_eq!(count, "1");
}

#[test]
fn g35_fetch_depth_1_into_tracking_repo() {
    require_git();
    let root = TempDir::new().unwrap();
    let _bare = seed_chain_bare(root.path(), "app.git");
    let peer = GitHttpPeer::start(root.path().to_path_buf());
    let dest = root.path().join("fetched");
    assert_ok(&arks(root.path(), &["init", dest.to_str().unwrap()]), "init");
    let url = peer.repo_url("app.git");
    assert_ok(
        &arks(
            &dest,
            &["remote", "add", "origin", &url],
        ),
        "remote add",
    );
    let out = arks(&dest, &["fetch", "--depth", "1", "origin"]);
    assert_ok(&out, "arks fetch --depth 1");
    let shallow = dest.join(".git/shallow");
    assert!(shallow.is_file(), "fetch --depth must write .git/shallow");
}

#[test]
fn g35_local_clone_depth_refuses() {
    require_git();
    let root = TempDir::new().unwrap();
    let work = root.path().join("src");
    fs::create_dir_all(&work).unwrap();
    assert_ok(&git(&work, &["init", "-b", "main"]), "init");
    fs::write(work.join("README"), b"local\n").unwrap();
    assert_ok(&git(&work, &["add", "README"]), "add");
    assert_ok(&git(&work, &["commit", "-m", "c1"]), "commit");
    let dest = root.path().join("out");
    let out = arks(
        root.path(),
        &[
            "clone",
            "--depth",
            "1",
            work.to_str().unwrap(),
            dest.to_str().unwrap(),
        ],
    );
    assert!(
        !out.status.success(),
        "local --depth must refuse: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}
