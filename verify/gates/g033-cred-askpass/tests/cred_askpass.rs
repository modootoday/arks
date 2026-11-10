//! Gate G33 — credential helper / GIT_ASKPASS against auth-required smart HTTP.
//! Cite: verify/matrices/matrix-g033-cred-askpass.md, gitcredentials, git-credential.
//! Secrets are generated at runtime into temp helpers — never committed.

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
    assert!(out.status.success(), "stock git required for G33");
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
        .env_remove("ARKS_REMOTE_USERNAME")
        .env_remove("ARKS_REMOTE_PASSWORD")
        .env_remove("GIT_ASKPASS")
        .env_remove("SSH_ASKPASS")
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .expect("arks")
}

fn arks_env(cwd: &Path, args: &[&str], envs: &[(&str, &str)]) -> std::process::Output {
    let mut cmd = Command::new(arks_bin());
    cmd.args(args)
        .current_dir(cwd)
        .env("GIT_AUTHOR_NAME", "arks")
        .env("GIT_AUTHOR_EMAIL", "arks@example.com")
        .env("GIT_COMMITTER_NAME", "arks")
        .env("GIT_COMMITTER_EMAIL", "arks@example.com")
        .env("GIT_AUTHOR_DATE", "1000000000 +0000")
        .env("GIT_COMMITTER_DATE", "1000000000 +0000")
        .env_remove("ARKS_REMOTE_USERNAME")
        .env_remove("ARKS_REMOTE_PASSWORD")
        .env_remove("GIT_ASKPASS")
        .env_remove("SSH_ASKPASS")
        .env("GIT_TERMINAL_PROMPT", "0");
    for (k, v) in envs {
        cmd.env(k, v);
    }
    cmd.output().expect("arks")
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

fn assert_fail(out: &std::process::Output, label: &str) {
    assert!(
        !out.status.success(),
        "{label} should refuse: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}

fn basic_header(user: &str, pass: &str) -> String {
    format!("Basic {}", b64(format!("{user}:{pass}").as_bytes()))
}

fn b64(data: &[u8]) -> String {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    let mut i = 0;
    while i + 3 <= data.len() {
        let n = (u32::from(data[i]) << 16) | (u32::from(data[i + 1]) << 8) | u32::from(data[i + 2]);
        out.push(T[((n >> 18) & 63) as usize] as char);
        out.push(T[((n >> 12) & 63) as usize] as char);
        out.push(T[((n >> 6) & 63) as usize] as char);
        out.push(T[(n & 63) as usize] as char);
        i += 3;
    }
    let rem = data.len() - i;
    if rem == 1 {
        let n = u32::from(data[i]) << 16;
        out.push(T[((n >> 18) & 63) as usize] as char);
        out.push(T[((n >> 12) & 63) as usize] as char);
        out.push('=');
        out.push('=');
    } else if rem == 2 {
        let n = (u32::from(data[i]) << 16) | (u32::from(data[i + 1]) << 8);
        out.push(T[((n >> 18) & 63) as usize] as char);
        out.push(T[((n >> 12) & 63) as usize] as char);
        out.push(T[((n >> 6) & 63) as usize] as char);
        out.push('=');
    }
    out
}

struct AuthHttpPeer {
    base_url: String,
    _join: thread::JoinHandle<()>,
    stop: Arc<Mutex<bool>>,
}

impl AuthHttpPeer {
    fn start(project_root: PathBuf, expected_auth: String) -> Self {
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
                        let expected = expected_auth.clone();
                        thread::spawn(move || {
                            if let Err(e) = handle_client(stream, &root, &expected) {
                                eprintln!("G33 peer handler: {e}");
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

impl Drop for AuthHttpPeer {
    fn drop(&mut self) {
        *self.stop.lock().unwrap() = true;
    }
}

fn handle_client(
    mut stream: TcpStream,
    project_root: &Path,
    expected_auth: &str,
) -> Result<(), String> {
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
    let mut auth = None;
    for line in header_text.split("\r\n").skip(1) {
        let lower = line.to_ascii_lowercase();
        if let Some(v) = lower.strip_prefix("authorization:") {
            auth = Some(v.trim().to_string());
            // Preserve original casing from header_text for exact Basic match.
            if let Some((_, rest)) = line.split_once(':') {
                auth = Some(rest.trim().to_string());
            }
        }
    }
    if auth.as_deref() != Some(expected_auth) {
        let body = b"unauthorized\n";
        let http = format!(
            "HTTP/1.1 401 Unauthorized\r\nWWW-Authenticate: Basic realm=\"g033-cred-askpass\"\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        stream.write_all(http.as_bytes()).map_err(|e| e.to_string())?;
        stream.write_all(body).map_err(|e| e.to_string())?;
        return Ok(());
    }

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
        content_type = if path.contains("receive-pack") {
            "application/x-git-receive-pack-request".into()
        } else {
            "application/x-git-upload-pack-request".into()
        };
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
    fs::write(work.join("README"), b"hello g033-cred-askpass cred\n").unwrap();
    assert_ok(&git(&work, &["add", "README"]), "add");
    assert_ok(&git(&work, &["commit", "-m", "init"]), "commit");
    assert_ok(
        &git(parent, &["clone", "--bare", work.to_str().unwrap(), name]),
        "bare clone",
    );
    bare
}

fn write_helper(path: &Path, user: &str, pass: &str) {
    let script = format!(
        "#!/bin/sh\nif [ \"$1\" = get ]; then\n  echo username={user}\n  echo password={pass}\n  echo\nfi\n"
    );
    fs::write(path, script).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms).unwrap();
    }
}

fn write_askpass(path: &Path, user: &str, pass: &str) {
    let script = format!(
        "#!/bin/sh\ncase \"$1\" in *sername*|*Username*) echo {user};; *) echo {pass};; esac\n"
    );
    fs::write(path, script).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms).unwrap();
    }
}

fn runtime_secret(tag: &str) -> (String, String) {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    (
        format!("g033-cred-askpassuser-{tag}"),
        format!("g033-cred-askpasspass-{tag}-{nonce}"),
    )
}

#[test]
fn g033-cred-askpass_c01_credential_fill_via_helper() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let (user, pass) = runtime_secret("fill");
    let helper = tmp.path().join("helper.sh");
    write_helper(&helper, &user, &pass);
    let cfg = tmp.path().join("gitconfig");
    fs::write(
        &cfg,
        format!("[credential]\n\thelper = \"!{}\"\n", helper.display()),
    )
    .unwrap();
    let input = "protocol=http\nhost=127.0.0.1\npath=r.git\n\n";
    let out = Command::new(arks_bin())
        .args(["credential", "fill"])
        .current_dir(tmp.path())
        .env("ARKS_CREDENTIAL_CONFIG", &cfg)
        .env_remove("ARKS_REMOTE_USERNAME")
        .env_remove("ARKS_REMOTE_PASSWORD")
        .env_remove("GIT_ASKPASS")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            child.stdin.take().unwrap().write_all(input.as_bytes())?;
            child.wait_with_output()
        })
        .expect("credential fill");
    assert_ok(&out, "credential fill");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains(&format!("username={user}")),
        "missing username"
    );
    assert!(
        stdout.contains(&format!("password={pass}")),
        "missing password"
    );
}

#[test]
fn g033-cred-askpass_c02_clone_with_credential_helper() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let (user, pass) = runtime_secret("helper");
    let expected = basic_header(&user, &pass);
    seed_bare_repo(tmp.path(), "auth.git");
    let peer = AuthHttpPeer::start(tmp.path().to_path_buf(), expected);
    let url = peer.repo_url("auth.git");

    let helper = tmp.path().join("helper.sh");
    write_helper(&helper, &user, &pass);
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    fs::write(
        home.join(".gitconfig"),
        format!("[credential]\n\thelper = \"!{}\"\n", helper.display()),
    )
    .unwrap();

    let dest = tmp.path().join("clone-helper");
    let out = arks_env(
        tmp.path(),
        &["clone", &url, dest.to_str().unwrap()],
        &[("HOME", home.to_str().unwrap())],
    );
    assert_ok(&out, "clone with helper");
    assert!(dest.join("README").is_file(), "workdir checkout");
}

#[test]
fn g033-cred-askpass_c03_clone_with_git_askpass() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let (user, pass) = runtime_secret("ask");
    let expected = basic_header(&user, &pass);
    seed_bare_repo(tmp.path(), "ask.git");
    let peer = AuthHttpPeer::start(tmp.path().to_path_buf(), expected);
    let url = peer.repo_url("ask.git");

    let ask = tmp.path().join("ask.sh");
    write_askpass(&ask, &user, &pass);
    let dest = tmp.path().join("clone-ask");
    let out = arks_env(
        tmp.path(),
        &["clone", &url, dest.to_str().unwrap()],
        &[("GIT_ASKPASS", ask.to_str().unwrap())],
    );
    assert_ok(&out, "clone with GIT_ASKPASS");
    assert!(dest.join("README").is_file());
}

#[test]
fn g033-cred-askpass_c04_fail_closed_without_creds() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let (user, pass) = runtime_secret("deny");
    let expected = basic_header(&user, &pass);
    seed_bare_repo(tmp.path(), "deny.git");
    let peer = AuthHttpPeer::start(tmp.path().to_path_buf(), expected);
    let url = peer.repo_url("deny.git");
    let dest = tmp.path().join("clone-deny");
    let out = arks(tmp.path(), &["clone", &url, dest.to_str().unwrap()]);
    assert_fail(&out, "clone without creds");
    assert!(!dest.join("README").is_file());
}

#[test]
fn g033-cred-askpass_c05_no_secret_in_tree_fixtures() {
    // Static check: committed verify/gates/g033-cred-askpass sources generate secrets at runtime.
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let src = fs::read_to_string(root.join("tests/cred_askpass.rs")).unwrap();
    assert!(
        src.contains("runtime_secret"),
        "must generate secrets at runtime"
    );
    // No hard-coded password assignment (runtime format! args only).
    let hardcoded = src
        .lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .any(|l| l.contains("password: \"") || l.contains("password = \""));
    assert!(!hardcoded, "no hard-coded password string literals");
}

#[test]
fn g033-cred-askpass_c06_tty_prompt_via_pty() {
    require_git();
    #[cfg(unix)]
    {
        let tmp = TempDir::new().unwrap();
        let (user, pass) = runtime_secret("tty");
        let expected = basic_header(&user, &pass);
        seed_bare_repo(tmp.path(), "tty.git");
        let peer = AuthHttpPeer::start(tmp.path().to_path_buf(), expected);
        let url = peer.repo_url("tty.git");
        let dest = tmp.path().join("clone-tty");
        let feeder = tmp.path().join("tty_feed.py");
        // Feed username/password on the PTY after prompts appear (or timed fallback).
        let script = format!(
            r#"#!/usr/bin/env python3
import os, pty, sys, select, time, signal
user = {user:?}
password = {pass:?}
cmd = sys.argv[1:]
pid, fd = pty.fork()
if pid == 0:
    os.environ.pop("GIT_ASKPASS", None)
    os.environ.pop("SSH_ASKPASS", None)
    os.environ.pop("ARKS_REMOTE_USERNAME", None)
    os.environ.pop("ARKS_REMOTE_PASSWORD", None)
    os.environ["GIT_TERMINAL_PROMPT"] = "1"
    os.execvp(cmd[0], cmd)
os.set_blocking(fd, False)
sent_user = False
sent_pass = False
buf = b""
deadline = time.time() + 25
fallback_at = time.time() + 1.0

def finish(code):
    sys.stderr.write(buf.decode("utf-8", "replace"))
    sys.exit(code)

while time.time() < deadline:
    r, _, _ = select.select([fd], [], [], 0.1)
    if fd in r:
        try:
            chunk = os.read(fd, 4096)
        except BlockingIOError:
            chunk = b""
        except OSError:
            chunk = b""
        if chunk:
            buf += chunk
            text = buf.decode("utf-8", "replace")
            if (not sent_user) and ("Username" in text):
                os.write(fd, (user + "\n").encode())
                sent_user = True
            if sent_user and (not sent_pass) and ("Password" in text):
                os.write(fd, (password + "\n").encode())
                sent_pass = True
        else:
            # PTY EOF — child likely exited.
            try:
                _, status = os.waitpid(pid, 0)
                finish(os.WEXITSTATUS(status) if os.WIFEXITED(status) else 1)
            except ChildProcessError:
                finish(0 if b"Cloned into" in buf else 1)
    if (not sent_user) and time.time() >= fallback_at:
        os.write(fd, (user + "\n").encode())
        sent_user = True
        fallback_at = time.time() + 0.3
    elif sent_user and (not sent_pass) and time.time() >= fallback_at:
        os.write(fd, (password + "\n").encode())
        sent_pass = True
    try:
        waited = os.waitpid(pid, os.WNOHANG)
        if waited[0] == pid:
            finish(os.WEXITSTATUS(waited[1]) if os.WIFEXITED(waited[1]) else 1)
    except ChildProcessError:
        finish(0 if b"Cloned into" in buf else 1)
try:
    os.kill(pid, signal.SIGKILL)
except OSError:
    pass
sys.stderr.write("G33-C06 timeout\n")
finish(0 if b"Cloned into" in buf else 1)
"#
        );
        fs::write(&feeder, script).unwrap();
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&feeder).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&feeder, perms).unwrap();
        }
        let arks = arks_bin();
        let out = Command::new("python3")
            .arg(&feeder)
            .arg(&arks)
            .arg("clone")
            .arg(&url)
            .arg(dest.to_str().unwrap())
            .current_dir(tmp.path())
            .env_remove("GIT_ASKPASS")
            .env_remove("SSH_ASKPASS")
            .env_remove("ARKS_REMOTE_USERNAME")
            .env_remove("ARKS_REMOTE_PASSWORD")
            .env("GIT_TERMINAL_PROMPT", "1")
            .output()
            .expect("pty feed");
        assert_ok(&out, "clone via TTY PTY");
        assert!(dest.join("README").is_file());
    }
    #[cfg(not(unix))]
    {
        eprintln!("G33-C06 skipped on non-unix host (clean-room is Linux)");
    }
}

#[test]
fn g033-cred-askpass_c07_os_wallet_helpers_file_backend() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let wallet = tmp.path().join("wallet");
    fs::create_dir_all(&wallet).unwrap();
    let (user, pass) = runtime_secret("wallet");
    let expected = basic_header(&user, &pass);
    seed_bare_repo(tmp.path(), "wallet.git");
    let peer = AuthHttpPeer::start(tmp.path().to_path_buf(), expected);
    let url = peer.repo_url("wallet.git");
    // http://127.0.0.1:PORT/wallet.git → host includes port
    let hostport = url
        .trim_start_matches("http://")
        .split('/')
        .next()
        .unwrap()
        .to_string();

    // Store via first-party wincred into file wallet.
    let store_input = format!(
        "protocol=http\nhost={hostport}\nusername={user}\npassword={pass}\n\n"
    );
    let store = Command::new(arks_bin())
        .args(["credential-wincred", "store"])
        .env("ARKS_CREDENTIAL_WALLET_DIR", &wallet)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .and_then(|mut c| {
            c.stdin.take().unwrap().write_all(store_input.as_bytes())?;
            c.wait_with_output()
        })
        .expect("wallet store");
    assert_ok(&store, "credential-wincred store");

    // Fill via libsecret name (same file backend) proves cross-name Claimed path.
    let cfg = tmp.path().join("wallet-cfg");
    fs::write(&cfg, "[credential]\n\thelper = libsecret\n").unwrap();
    let fill = Command::new(arks_bin())
        .args(["credential", "fill"])
        .env("ARKS_CREDENTIAL_WALLET_DIR", &wallet)
        .env("ARKS_CREDENTIAL_CONFIG", &cfg)
        .env_remove("GIT_ASKPASS")
        .env_remove("ARKS_REMOTE_USERNAME")
        .env_remove("ARKS_REMOTE_PASSWORD")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .and_then(|mut c| {
            let input = format!("protocol=http\nhost={hostport}\n\n");
            c.stdin.take().unwrap().write_all(input.as_bytes())?;
            c.wait_with_output()
        })
        .expect("wallet fill");
    assert_ok(&fill, "credential fill via libsecret wallet");
    let stdout = String::from_utf8_lossy(&fill.stdout);
    assert!(stdout.contains(&format!("username={user}")));
    assert!(stdout.contains(&format!("password={pass}")));

    // Clone using osxkeychain helper name against same file wallet.
    let home = tmp.path().join("home-wallet");
    fs::create_dir_all(&home).unwrap();
    fs::write(
        home.join(".gitconfig"),
        "[credential]\n\thelper = osxkeychain\n",
    )
    .unwrap();
    let dest = tmp.path().join("clone-wallet");
    let out = arks_env(
        tmp.path(),
        &["clone", &url, dest.to_str().unwrap()],
        &[
            ("HOME", home.to_str().unwrap()),
            ("ARKS_CREDENTIAL_WALLET_DIR", wallet.to_str().unwrap()),
        ],
    );
    assert_ok(&out, "clone via osxkeychain wallet helper");
    assert!(dest.join("README").is_file());
}
