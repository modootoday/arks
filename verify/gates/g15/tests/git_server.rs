//! Gate G15 — production-server: stock Git clone/fetch/push vs arks smart HTTP.
//!
//! Cite: verify/matrices/matrix-m11.md, gitprotocol-http, gitprotocol-pack.
//! No forge. No CELIR. Lease not required.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

fn require_git() {
    let out = Command::new("git").arg("--version").output().expect("git");
    assert!(out.status.success(), "stock git required for G15");
}

fn arks_bin() -> PathBuf {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(target) = std::env::var("CARGO_TARGET_DIR") {
        // Prefer native extension first (Windows .exe vs Linux ELF in shared dirs).
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
    url: String,
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
            .expect("spawn serve-http");
        let url = format!("http://{listen}/");
        // Wait until info/refs answers.
        for _ in 0..50 {
            if let Ok(mut s) = std::net::TcpStream::connect(format!("127.0.0.1:{port}")) {
                use std::io::Write;
                let _ = write!(
                    s,
                    "GET /info/refs?service=git-upload-pack HTTP/1.0\r\nHost: 127.0.0.1\r\n\r\n"
                );
                thread::sleep(Duration::from_millis(30));
                break;
            }
            thread::sleep(Duration::from_millis(40));
        }
        Self { child, url }
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
    fs::write(work.join("hello.txt"), b"hello g15\n").unwrap();
    assert_ok(&arks(&work, &["add", "hello.txt"]), "add");
    assert_ok(&arks(&work, &["commit", "-m", "initial"]), "commit");
    let bare = root.join("server.git");
    fs::create_dir_all(&bare).unwrap();
    // Clone to bare via file protocol using stock git for fixture setup is OK
    // (oracle). Product server under test is arks.
    assert_ok(
        &git(root, &["clone", "--bare", work.join(".git").to_str().unwrap(), bare.to_str().unwrap()]),
        "bare clone fixture",
    );
    bare
}

#[test]
fn g15_help_wave_sv() {
    require_git();
    let out = Command::new(arks_bin())
        .arg("--help")
        .output()
        .expect("help");
    assert_ok(&out, "help");
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("upload-pack"));
    assert!(text.contains("receive-pack"));
    assert!(text.contains("serve-http"));
}

#[test]
fn g15_stock_git_clone_fetch_push() {
    require_git();
    let root = TempDir::new().unwrap();
    let bare = seed_bare_with_arks(root.path());
    let server = ArksHttp::start(&bare);

    let clone = root.path().join("clone");
    let out = git(
        root.path(),
        &["clone", &server.url, clone.to_str().unwrap()],
    );
    assert_ok(&out, "git clone via arks smart HTTP");
    let body = fs::read_to_string(clone.join("hello.txt")).unwrap();
    assert!(
        body.replace("\r\n", "\n").starts_with("hello g15"),
        "unexpected clone content: {body:?}"
    );

    // Commit in clone and push back to arks receive-pack.
    fs::write(clone.join("hello.txt"), b"hello push\n").unwrap();
    assert_ok(&git(&clone, &["add", "hello.txt"]), "git add");
    assert_ok(&git(&clone, &["commit", "-m", "push-me"]), "git commit");
    let push = git(&clone, &["push", "origin", "HEAD:refs/heads/main"]);
    assert_ok(&push, "git push to arks receive-pack");

    let clone2 = root.path().join("clone2");
    assert_ok(
        &git(
            root.path(),
            &["clone", &server.url, clone2.to_str().unwrap()],
        ),
        "git clone after push",
    );
    let body2 = fs::read_to_string(clone2.join("hello.txt")).unwrap();
    assert!(
        body2.replace("\r\n", "\n").starts_with("hello push"),
        "unexpected post-push content: {body2:?}"
    );

    assert_ok(&git(&clone2, &["fetch", "origin"]), "git fetch");
}

#[test]
fn g15_capability_honesty_upload_ad() {
    require_git();
    let root = TempDir::new().unwrap();
    let bare = seed_bare_with_arks(root.path());
    let out = arks(root.path(), &["upload-pack", bare.to_str().unwrap()]);
    assert_ok(&out, "upload-pack ad");
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("# service=git-upload-pack") || text.contains("refs/heads/main"));
    assert!(!text.contains("version 2"));
    let recv = arks(root.path(), &["receive-pack", bare.to_str().unwrap()]);
    assert_ok(&recv, "receive-pack ad");
    let rtext = String::from_utf8_lossy(&recv.stdout);
    assert!(rtext.contains("report-status"));
    assert!(rtext.contains("report-status-v2"));
    assert!(rtext.contains("delete-refs"));
    assert!(!rtext.contains("push-options"));
}

/// M11-C14 — stock Git push vs arks receive-pack honor list
/// (`pre-receive` / `update` / `post-receive`).
/// Cite: https://git-scm.com/docs/githooks , https://git-scm.com/docs/git-receive-pack
#[test]
fn g15_receive_pack_githooks_honor_list() {
    require_git();
    let root = TempDir::new().unwrap();
    let bare = seed_bare_with_arks(root.path());

    // pre-receive decline: push must fail; tip unchanged.
    let hooks = bare.join("hooks");
    fs::create_dir_all(&hooks).unwrap();
    let pre = hooks.join("pre-receive");
    fs::write(&pre, "#!/bin/sh\nexit 1\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut p = fs::metadata(&pre).unwrap().permissions();
        p.set_mode(0o755);
        fs::set_permissions(&pre, p).unwrap();
    }
    let server = ArksHttp::start(&bare);
    let clone = root.path().join("clone-hooks");
    assert_ok(
        &git(
            root.path(),
            &["clone", &server.url, clone.to_str().unwrap()],
        ),
        "clone for hooks",
    );
    fs::write(clone.join("hello.txt"), b"blocked\n").unwrap();
    assert_ok(&git(&clone, &["add", "hello.txt"]), "add blocked");
    assert_ok(&git(&clone, &["commit", "-m", "blocked"]), "commit blocked");
    let blocked = git(&clone, &["push", "origin", "HEAD:refs/heads/main"]);
    assert!(
        !blocked.status.success(),
        "pre-receive must reject push\nstdout={}\nstderr={}",
        String::from_utf8_lossy(&blocked.stdout),
        String::from_utf8_lossy(&blocked.stderr)
    );
    drop(server);

    // Allow pre-receive; post-receive marker; update allow.
    fs::write(&pre, "#!/bin/sh\nexit 0\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut p = fs::metadata(&pre).unwrap().permissions();
        p.set_mode(0o755);
        fs::set_permissions(&pre, p).unwrap();
    }
    let post = hooks.join("post-receive");
    fs::write(
        &post,
        "#!/bin/sh\ncat > \"$GIT_DIR/post-receive-ran\"\n",
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut p = fs::metadata(&post).unwrap().permissions();
        p.set_mode(0o755);
        fs::set_permissions(&post, p).unwrap();
    }
    let update = hooks.join("update");
    fs::write(&update, "#!/bin/sh\nexit 0\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut p = fs::metadata(&update).unwrap().permissions();
        p.set_mode(0o755);
        fs::set_permissions(&update, p).unwrap();
    }

    let server2 = ArksHttp::start(&bare);
    // Re-point origin in case port changed.
    assert_ok(
        &git(
            &clone,
            &["remote", "set-url", "origin", &server2.url],
        ),
        "retarget origin",
    );
    fs::write(clone.join("hello.txt"), b"hooks ok\n").unwrap();
    assert_ok(&git(&clone, &["add", "hello.txt"]), "add ok");
    assert_ok(&git(&clone, &["commit", "-m", "hooks-ok"]), "commit ok");
    let push_ok = git(&clone, &["push", "origin", "HEAD:refs/heads/main"]);
    assert_ok(&push_ok, "push after hooks allow");
    assert!(
        bare.join("post-receive-ran").is_file(),
        "post-receive should write marker under GIT_DIR"
    );

    // update decline for a new branch name.
    fs::write(&update, "#!/bin/sh\nexit 1\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut p = fs::metadata(&update).unwrap().permissions();
        p.set_mode(0o755);
        fs::set_permissions(&update, p).unwrap();
    }
    let denied = git(&clone, &["push", "origin", "HEAD:refs/heads/topic"]);
    assert!(
        !denied.status.success(),
        "update hook must reject topic\nstdout={}\nstderr={}",
        String::from_utf8_lossy(&denied.stdout),
        String::from_utf8_lossy(&denied.stderr)
    );
}
