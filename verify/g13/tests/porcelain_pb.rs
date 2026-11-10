//! Gate G13 — Wave Pb porcelain argv suite (clean-room). Not production (M9).
//!
//! Matrix: `verify/matrix-m8.md` M8-C01..C10. Stock Git is a peer oracle where noted.

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
    assert!(out.status.success());
}

fn arks_bin() -> PathBuf {
    // Clean-room compose sets CARGO_TARGET_DIR; host advisory uses workspace target.
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(target) = std::env::var("CARGO_TARGET_DIR") {
        candidates.push(PathBuf::from(&target).join("debug/arks"));
        candidates.push(PathBuf::from(&target).join("debug/arks.exe"));
    } else {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        candidates.push(root.join("target/debug/arks.exe"));
        candidates.push(root.join("target/debug/arks"));
        candidates.push(root.join("target-cleanroom/debug/arks"));
    }
    for c in &candidates {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!("missing arks binary; tried {candidates:?}");
}

fn arks(repo: &Path, args: &[&str]) -> std::process::Output {
    Command::new(arks_bin())
        .args(args)
        .current_dir(repo)
        .env("GIT_AUTHOR_NAME", "arks")
        .env("GIT_AUTHOR_EMAIL", "arks@example.com")
        .env("GIT_COMMITTER_NAME", "arks")
        .env("GIT_COMMITTER_EMAIL", "arks@example.com")
        .env("GIT_AUTHOR_DATE", "1000000000")
        .env("GIT_COMMITTER_DATE", "1000000000")
        .output()
        .expect("arks")
}

fn git(repo: &Path, args: &[&str]) -> std::process::Output {
    Command::new("git")
        .args(args)
        .current_dir(repo)
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
        "{label}: {}\n{}",
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

fn seed_commit(repo: &Path, file: &str, body: &str, msg: &str) {
    fs::write(repo.join(file), body).unwrap();
    assert_ok(&arks(repo, &["add", file]), "add");
    assert_ok(&arks(repo, &["commit", "-m", msg]), "commit");
}

fn rev(repo: &Path, name: &str) -> String {
    let out = arks(repo, &["rev-parse", name]);
    assert_ok(&out, "rev-parse");
    String::from_utf8_lossy(&out.stdout).trim().to_owned()
}

fn help_text() -> String {
    let out = Command::new(arks_bin()).arg("--help").output().unwrap();
    assert_ok(&out, "help");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn command_listed(help: &str, name: &str) -> bool {
    help.lines().any(|line| {
        let trimmed = line.trim_start();
        trimmed.starts_with(name) && trimmed[name.len()..].starts_with(char::is_whitespace)
    })
}

#[test]
fn help_lists_wave_pb_and_omits_deferred() {
    require_git();
    let s = help_text();
    for cmd in [
        "init",
        "commit",
        "pull",
        "branch",
        "checkout",
        "merge",
        "stash",
        "cherry-pick",
        "revert",
        "rebase",
        "format-patch",
        "am",
        "bisect",
        "bundle",
    ] {
        assert!(command_listed(&s, cmd), "help missing {cmd}");
    }
    // M15/M16 claimed residuals supersede Wave Pb deferred omit list.
    for cmd in ["repack", "range-diff", "notes", "gc", "maintenance"] {
        assert!(command_listed(&s, cmd), "help missing claimed residual {cmd}");
    }
    for bad in ["filter-branch", "backfill"] {
        assert!(!command_listed(&s, bad), "help must not list {bad}");
    }
    // `worktree` / `submodule` / `reflog` may appear once residual/Pc claims them.
}

#[test]
fn refuse_deferred_and_unclaimed_flags() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path();
    assert_ok(&arks(repo, &["init"]), "init");
    seed_commit(repo, "a.txt", "one\n", "c1");
    // Still refuse-closed / usage-fail under current claims.
    assert_fail(&arks(repo, &["filter-branch"]), "filter-branch");
    assert_fail(&arks(repo, &["backfill"]), "backfill");
    assert_fail(&arks(repo, &["rebase", "-i", "HEAD"]), "rebase -i");
    assert_fail(&arks(repo, &["cherry-pick", "--continue"]), "continue without sequencer");
    assert_fail(&arks(repo, &["bisect", "run", "true"]), "bisect run");
    // Missing bundle file still refuses non-zero (Pc may implement verify for real files).
    assert_fail(&arks(repo, &["bundle", "verify", "x.bundle"]), "bundle verify missing");
    // M16 Sy-5: cover-letter claimed (smoke under historical G13 regression).
    assert_ok(
        &arks(repo, &["format-patch", "--cover-letter", "-1"]),
        "cover letter",
    );
}

#[test]
fn cherry_pick_single_clean_commit() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path();
    assert_ok(&arks(repo, &["init"]), "init");
    seed_commit(repo, "a.txt", "one\n", "c1");
    assert_ok(&arks(repo, &["branch", "topic"]), "branch");
    assert_ok(&arks(repo, &["checkout", "topic"]), "checkout topic");
    seed_commit(repo, "b.txt", "two\n", "c2");
    let picked = rev(repo, "HEAD");
    assert_ok(&arks(repo, &["checkout", "main"]), "checkout main");
    assert!(!repo.join("b.txt").exists());
    assert_ok(&arks(repo, &["cherry-pick", &picked]), "cherry-pick");
    assert_eq!(fs::read_to_string(repo.join("b.txt")).unwrap(), "two\n");
    assert_eq!(fs::read_to_string(repo.join("a.txt")).unwrap(), "one\n");
    assert_ok(&git(repo, &["status"]), "git status");
    assert_ok(&git(repo, &["rev-parse", "HEAD"]), "git rev-parse");
    let log = arks(repo, &["log"]);
    assert_ok(&log, "log");
    assert!(String::from_utf8_lossy(&log.stdout).contains("c2"));
}

#[test]
fn revert_single_clean_commit() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path();
    assert_ok(&arks(repo, &["init"]), "init");
    seed_commit(repo, "a.txt", "one\n", "c1");
    seed_commit(repo, "a.txt", "two\n", "c2");
    assert_ok(&arks(repo, &["revert", "HEAD"]), "revert");
    assert_eq!(fs::read_to_string(repo.join("a.txt")).unwrap(), "one\n");
    assert_ok(&git(repo, &["status"]), "git status");
    assert_ok(&git(repo, &["rev-parse", "HEAD"]), "git rev-parse");
    let log = arks(repo, &["log"]);
    assert_ok(&log, "log");
    assert!(String::from_utf8_lossy(&log.stdout).contains("Revert \"c2\""));
}

#[test]
fn rebase_replays_linear_commits() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path();
    assert_ok(&arks(repo, &["init"]), "init");
    seed_commit(repo, "a.txt", "base\n", "base");
    assert_ok(&arks(repo, &["branch", "feature"]), "branch");
    assert_ok(&arks(repo, &["checkout", "feature"]), "checkout feature");
    seed_commit(repo, "b.txt", "feat\n", "feat");
    assert_ok(&arks(repo, &["checkout", "main"]), "checkout main");
    seed_commit(repo, "c.txt", "main\n", "mainline");
    assert_ok(&arks(repo, &["checkout", "feature"]), "back feature");
    assert_ok(&arks(repo, &["rebase", "main"]), "rebase");
    assert_eq!(fs::read_to_string(repo.join("a.txt")).unwrap(), "base\n");
    assert_eq!(fs::read_to_string(repo.join("b.txt")).unwrap(), "feat\n");
    assert_eq!(fs::read_to_string(repo.join("c.txt")).unwrap(), "main\n");
    let log = arks(repo, &["log"]);
    assert_ok(&log, "log");
    let text = String::from_utf8_lossy(&log.stdout);
    assert!(text.contains("feat"));
    assert!(text.contains("mainline"));
}

#[test]
fn format_patch_and_am_round_trip() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path();
    assert_ok(&arks(repo, &["init"]), "init");
    seed_commit(repo, "a.txt", "one\n", "base");
    assert_ok(&arks(repo, &["branch", "base"]), "branch base");
    seed_commit(repo, "b.txt", "two\n", "add-b");
    fs::write(repo.join("b.txt"), "three\n").unwrap();
    fs::write(repo.join("c.txt"), "c\n").unwrap();
    assert_ok(&arks(repo, &["add", "b.txt"]), "add b");
    assert_ok(&arks(repo, &["add", "c.txt"]), "add c");
    assert_ok(&arks(repo, &["commit", "-m", "edit-b"]), "commit edit");

    let one_dir = tmp.path().join("one");
    assert_ok(
        &arks(
            repo,
            &[
                "format-patch",
                "-1",
                "-o",
                one_dir.to_str().unwrap(),
            ],
        ),
        "format-patch -1",
    );
    let one_files = patch_files(&one_dir);
    assert_eq!(one_files.len(), 1, "expected a single -1 patch");

    let series_dir = tmp.path().join("series");
    assert_ok(
        &arks(
            repo,
            &[
                "format-patch",
                "-o",
                series_dir.to_str().unwrap(),
                "base..HEAD",
            ],
        ),
        "format-patch range",
    );
    let series = patch_files(&series_dir);
    assert_eq!(series.len(), 2, "expected two patches in A..B");

    let dest = tmp.path().join("dest");
    fs::create_dir_all(&dest).unwrap();
    assert_ok(&arks(&dest, &["init"]), "init dest");
    for patch in &series {
        assert_ok(
            &arks(&dest, &["am", patch.to_str().unwrap()]),
            "am",
        );
    }
    assert_eq!(fs::read_to_string(dest.join("b.txt")).unwrap(), "three\n");
    assert_eq!(fs::read_to_string(dest.join("c.txt")).unwrap(), "c\n");
    assert!(!dest.join("a.txt").exists());
}

#[test]
fn bisect_start_bad_good_reset() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path();
    assert_ok(&arks(repo, &["init"]), "init");
    seed_commit(repo, "a.txt", "1\n", "c1");
    let good = rev(repo, "HEAD");
    seed_commit(repo, "a.txt", "2\n", "c2");
    seed_commit(repo, "a.txt", "3\n", "c3");
    seed_commit(repo, "a.txt", "4\n", "c4");
    assert_ok(&arks(repo, &["bisect", "start"]), "start");
    assert_ok(&arks(repo, &["bisect", "bad"]), "bad");
    assert_ok(&arks(repo, &["bisect", "good", &good]), "good");
    assert_eq!(fs::read_to_string(repo.join("a.txt")).unwrap(), "3\n");
    assert_ok(&arks(repo, &["bisect", "bad"]), "bad mid");
    assert_eq!(fs::read_to_string(repo.join("a.txt")).unwrap(), "2\n");
    let found = arks(repo, &["bisect", "good"]);
    assert_ok(&found, "good mid");
    assert!(
        String::from_utf8_lossy(&found.stdout).contains("first bad commit"),
        "{}",
        String::from_utf8_lossy(&found.stdout)
    );
    assert_eq!(fs::read_to_string(repo.join("a.txt")).unwrap(), "3\n");
    assert_ok(&arks(repo, &["bisect", "reset"]), "reset");
    assert_eq!(fs::read_to_string(repo.join("a.txt")).unwrap(), "4\n");
    assert!(!repo.join(".git/arks-bisect").exists());
}

#[test]
fn bundle_create_list_heads_unbundle() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("src");
    fs::create_dir_all(&repo).unwrap();
    assert_ok(&arks(&repo, &["init"]), "init");
    seed_commit(&repo, "a.txt", "hello\n", "c1");
    assert_ok(&arks(&repo, &["branch", "other"]), "branch other");
    assert_ok(&arks(&repo, &["checkout", "other"]), "checkout other");
    seed_commit(&repo, "b.txt", "side\n", "c2");
    assert_ok(&arks(&repo, &["checkout", "main"]), "checkout main");

    let bundle = tmp.path().join("repo.bundle");
    assert_ok(
        &arks(
            &repo,
            &["bundle", "create", bundle.to_str().unwrap(), "--all"],
        ),
        "bundle create --all",
    );
    let heads = arks(&repo, &["bundle", "list-heads", bundle.to_str().unwrap()]);
    assert_ok(&heads, "list-heads");
    let listed = String::from_utf8_lossy(&heads.stdout);
    assert!(listed.contains("refs/heads/main"), "{listed}");
    assert!(listed.contains("refs/heads/other"), "{listed}");

    let only = tmp.path().join("main.bundle");
    assert_ok(
        &arks(
            &repo,
            &["bundle", "create", only.to_str().unwrap(), "main"],
        ),
        "bundle create main",
    );
    let only_heads = arks(&repo, &["bundle", "list-heads", only.to_str().unwrap()]);
    assert_ok(&only_heads, "list-heads main");
    let only_text = String::from_utf8_lossy(&only_heads.stdout);
    assert!(only_text.contains("refs/heads/main"), "{only_text}");
    assert!(
        !only_text.contains("refs/heads/other"),
        "{only_text}"
    );

    let dest = tmp.path().join("dest");
    fs::create_dir_all(&dest).unwrap();
    assert_ok(&arks(&dest, &["init"]), "init dest");
    let un = arks(&dest, &["bundle", "unbundle", bundle.to_str().unwrap()]);
    assert_ok(&un, "unbundle");
    assert!(String::from_utf8_lossy(&un.stdout).contains("refs/heads/main"));
    assert_ok(&arks(&dest, &["checkout", "main"]), "checkout main");
    assert_eq!(fs::read_to_string(dest.join("a.txt")).unwrap(), "hello\n");
    assert_ok(&arks(&dest, &["checkout", "other"]), "checkout other");
    assert_eq!(fs::read_to_string(dest.join("b.txt")).unwrap(), "side\n");
}

#[test]
fn conflicted_cherry_pick_and_rebase_refuse() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path();
    assert_ok(&arks(repo, &["init"]), "init");
    seed_commit(repo, "a.txt", "base\n", "base");
    assert_ok(&arks(repo, &["branch", "feature"]), "branch");
    assert_ok(&arks(repo, &["checkout", "feature"]), "checkout feature");
    seed_commit(repo, "a.txt", "feature\n", "feature");
    let feature = rev(repo, "HEAD");
    assert_ok(&arks(repo, &["checkout", "main"]), "checkout main");
    seed_commit(repo, "a.txt", "main\n", "mainline");
    let before = rev(repo, "HEAD");
    assert_fail(&arks(repo, &["cherry-pick", &feature]), "conflict cherry-pick");
    // M16 Sy-5: conflicted cherry-pick writes markers + keeps HEAD (sequencer).
    let body = fs::read_to_string(repo.join("a.txt")).unwrap();
    assert!(
        body.contains("<<<<<<<") && body.contains(">>>>>>>"),
        "conflict markers present: {body}"
    );
    assert_eq!(rev(repo, "HEAD"), before);
    assert_ok(&arks(repo, &["cherry-pick", "--abort"]), "cherry-pick --abort");
    assert_eq!(fs::read_to_string(repo.join("a.txt")).unwrap(), "main\n");

    assert_ok(&arks(repo, &["checkout", "feature"]), "back feature");
    let feature_tip = rev(repo, "HEAD");
    // Non-interactive rebase still refuse-closed on unclean apply (no markers).
    assert_fail(&arks(repo, &["rebase", "main"]), "conflict rebase");
    assert_eq!(rev(repo, "HEAD"), feature_tip);
    assert_eq!(fs::read_to_string(repo.join("a.txt")).unwrap(), "feature\n");
}

#[test]
fn pull_rebase_onto_tracking_branch() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().join("peer");
    fs::create_dir_all(&root).unwrap();
    let bare = seed_bare_repo(&root, "app.git");
    let peer = GitHttpPeer::start(root.clone());
    thread::sleep(Duration::from_millis(50));
    let url = peer.repo_url("app.git");
    let clone_dir = tmp.path().join("clone");
    assert_ok(
        &arks(tmp.path(), &["clone", &url, clone_dir.to_str().unwrap()]),
        "clone",
    );

    let work = root.join("app.git-work");
    fs::write(work.join("REMOTE.txt"), b"remote\n").unwrap();
    assert_ok(&git(&work, &["add", "REMOTE.txt"]), "add remote file");
    assert_ok(&git(&work, &["commit", "-m", "remote"]), "commit remote");
    assert_ok(
        &git(&work, &["push", bare.to_str().unwrap(), "HEAD:main"]),
        "push remote",
    );
    assert_ok(&arks(&clone_dir, &["pull"]), "ff pull");
    assert_eq!(
        fs::read_to_string(clone_dir.join("REMOTE.txt")).unwrap(),
        "remote\n"
    );

    fs::write(clone_dir.join("LOCAL.txt"), b"local\n").unwrap();
    assert_ok(&arks(&clone_dir, &["add", "LOCAL.txt"]), "add local");
    assert_ok(&arks(&clone_dir, &["commit", "-m", "local"]), "commit local");
    fs::write(work.join("OTHER.txt"), b"other\n").unwrap();
    assert_ok(&git(&work, &["add", "OTHER.txt"]), "add other");
    assert_ok(&git(&work, &["commit", "-m", "other"]), "commit other");
    assert_ok(
        &git(&work, &["push", bare.to_str().unwrap(), "HEAD:main"]),
        "push other",
    );
    assert_ok(&arks(&clone_dir, &["pull", "--rebase"]), "pull --rebase");
    assert_eq!(
        fs::read_to_string(clone_dir.join("LOCAL.txt")).unwrap(),
        "local\n"
    );
    assert_eq!(
        fs::read_to_string(clone_dir.join("OTHER.txt")).unwrap(),
        "other\n"
    );
    assert_eq!(
        fs::read_to_string(clone_dir.join("REMOTE.txt")).unwrap(),
        "remote\n"
    );
    let log = arks(&clone_dir, &["log"]);
    assert_ok(&log, "log");
    let text = String::from_utf8_lossy(&log.stdout);
    assert!(text.contains("local"));
    assert!(text.contains("other"));
}

fn patch_files(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap()
        .map(|ent| ent.unwrap().path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("patch"))
        .collect();
    files.sort();
    files
}

struct GitHttpPeer {
    base_url: String,
    stop: Arc<Mutex<bool>>,
}

impl GitHttpPeer {
    fn start(project_root: PathBuf) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = listener.local_addr().unwrap().port();
        let stop = Arc::new(Mutex::new(false));
        let stop2 = Arc::clone(&stop);
        thread::spawn(move || {
            listener.set_nonblocking(true).ok();
            while !*stop2.lock().unwrap() {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let root = project_root.clone();
                        thread::spawn(move || {
                            if let Err(e) = handle_client(stream, &root) {
                                eprintln!("G13 peer handler: {e}");
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
    fs::write(work.join("README"), b"hello arks m8\n").unwrap();
    assert_ok(&git(&work, &["add", "README"]), "add");
    assert_ok(&git(&work, &["commit", "-m", "init"]), "commit");
    assert_ok(
        &git(parent, &["clone", "--bare", work.to_str().unwrap(), name]),
        "bare clone",
    );
    bare
}
