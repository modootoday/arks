//! `arks serve` enforces forge rulesets on push (queue unit Q4; contract
//! [[sot:root/arks-hub-product-intent]] §17–22). Host-advisory end-to-end: one
//! `arks serve` process (smart HTTP + forge API on loopback), a ruleset created
//! through the forge REST API, and pushes from the arks client — plus stock git
//! as a second client when it is installed (skipped with a note otherwise).
//!
//! Primary: <https://git-scm.com/docs/git-push> (OUTPUT: `remote rejected`),
//! <https://git-scm.com/docs/gitprotocol-pack> (report-status),
//! <https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-rulesets/available-rules-for-rulesets>.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

fn client(bin: &str) -> Command {
    let mut c = Command::new(bin);
    c.env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("ARKS_PERSONALITY", "git")
        .env_remove("ARKS_HUB_DATA")
        .env_remove("ARKS_HUB_GIT_ROOT");
    c
}

fn arks_bin() -> &'static str {
    env!("CARGO_BIN_EXE_arks")
}

fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

fn wait_port(port: u16) {
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return;
        }
        thread::sleep(Duration::from_millis(50));
    }
    panic!("port {port} never opened");
}

struct Serve(Child);

impl Drop for Serve {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Minimal HTTP/1.1 request to the forge API; returns (status, body).
fn forge(port: u16, method: &str, path: &str, body: &str) -> (u16, String) {
    let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
    let req = format!(
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nAuthorization: token local-dev-token\r\n\
         Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    s.write_all(req.as_bytes()).unwrap();
    let mut raw = String::new();
    s.read_to_string(&mut raw).unwrap();
    let status = raw
        .split_whitespace()
        .nth(1)
        .and_then(|c| c.parse().ok())
        .unwrap_or(0);
    let body = raw.split_once("\r\n\r\n").map(|x| x.1).unwrap_or("");
    (status, body.to_string())
}

fn run(bin: &str, dir: &Path, args: &[&str]) -> Output {
    client(bin)
        .args(args)
        .current_dir(dir)
        .stdin(Stdio::null())
        .output()
        .unwrap()
}

fn ok(bin: &str, dir: &Path, args: &[&str]) -> String {
    let out = run(bin, dir, args);
    assert!(
        out.status.success(),
        "{bin} {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn commit(bin: &str, dir: &Path, file: &str, msg: &str) -> String {
    std::fs::write(dir.join(file), format!("{msg}\n")).unwrap();
    ok(bin, dir, &["add", file]);
    ok(
        bin,
        dir,
        &[
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@example.com",
            "commit",
            "-q",
            "-m",
            msg,
        ],
    );
    ok(bin, dir, &["rev-parse", "HEAD"])
}

fn tip(git_dir: &Path, refname: &str) -> Option<String> {
    let out = run(
        arks_bin(),
        git_dir,
        &["rev-parse", "--verify", "-q", refname],
    );
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

struct Forge {
    _serve: Serve,
    _tmp: PathBuf,
    root: PathBuf,
    bare: PathBuf,
    url: String,
}

impl Drop for Forge {
    fn drop(&mut self) {
        let _ = self._serve.0.kill();
        let _ = self._serve.0.wait();
        let _ = std::fs::remove_dir_all(&self._tmp);
    }
}

/// One `arks serve --no-ssh` with the forge module; the served Git directory is
/// the forge repository `local/playground` with a `protect main` ruleset
/// (deletion + non_fast_forward on the default branch).
fn start(tag: &str) -> Forge {
    let tmp = std::env::temp_dir().join(format!("arks-q4-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    let data = tmp.join("forge");
    let bare = data.join("git").join("local").join("playground.git");
    std::fs::create_dir_all(bare.parent().unwrap()).unwrap();
    ok(
        arks_bin(),
        &tmp,
        &["init", "-q", "--bare", "-b", "main", bare.to_str().unwrap()],
    );
    let (http, api) = (free_port(), free_port());
    let child = client(arks_bin())
        .arg("serve")
        .arg("--no-ssh")
        .arg(format!("--forge-listen=127.0.0.1:{api}"))
        .arg(format!("--forge-data={}", data.display()))
        .arg("--listen")
        .arg(format!("127.0.0.1:{http}"))
        .arg(&bare)
        .env("HOME", &tmp)
        .env("ARKS_SERVE_CONFIG", tmp.join("serve.config"))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn arks serve");
    let serve = Serve(child);
    wait_port(api);
    wait_port(http);
    let ruleset = r#"{"name":"protect main","target":"branch","enforcement":"active",
        "conditions":{"ref_name":{"include":["~DEFAULT_BRANCH"],"exclude":[]}},
        "rules":[{"type":"deletion"},{"type":"non_fast_forward"}]}"#;
    let (code, body) = forge(api, "POST", "/repos/local/playground/rulesets", ruleset);
    assert_eq!(code, 201, "create ruleset: {body}");
    let root = tmp.join("clients");
    std::fs::create_dir_all(&root).unwrap();
    Forge {
        _serve: serve,
        _tmp: tmp,
        root,
        bare,
        url: format!("http://127.0.0.1:{http}/local/playground.git"),
    }
}

fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

/// The client's report of the refused `main`: stock git prints the
/// report-status `ng` as `! [remote rejected] <src -> dst> (<reason>)` (`main`
/// alone for a deletion); the arks network push client reports it as
/// `push rejected: refs/heads/main <reason>`.
fn assert_rejected(label: &str, e: &str, git_ref: &str, rule: &str) {
    let reason = format!(
        "push declined due to repository rule violations (ruleset \"protect main\": {rule})"
    );
    let shown = if label == "git" {
        format!("! [remote rejected] {git_ref} ({reason})")
    } else {
        format!("push rejected: refs/heads/main {reason}")
    };
    assert!(e.contains(&shown), "{label}: want {shown:?} in:\n{e}");
}

/// The journey with one client binary: seed main, refused deletion, refused
/// force push (while another ref in the same push is created), allowed
/// fast-forward.
fn journey(bin: &str, f: &Forge, label: &str) {
    let work = f.root.join(label);
    std::fs::create_dir_all(&work).unwrap();
    ok(bin, &work, &["init", "-q", "-b", "main"]);
    ok(bin, &work, &["remote", "add", "origin", &f.url]);
    let c1 = commit(bin, &work, "a", "one");
    ok(bin, &work, &["push", "-q", "origin", "main"]);
    assert_eq!(
        tip(&f.bare, "refs/heads/main").as_deref(),
        Some(c1.as_str())
    );

    // Deletion of the protected default branch is refused. (Full refnames:
    // the arks client does not yet resolve a short deletion name against a
    // network remote's advertisement.)
    let del = run(bin, &work, &["push", "origin", ":refs/heads/main"]);
    assert!(!del.status.success(), "{label}: deletion must fail");
    assert_rejected(label, &stderr(&del), "main", "cannot delete this ref");
    assert_eq!(
        tip(&f.bare, "refs/heads/main").as_deref(),
        Some(c1.as_str())
    );

    // A force push to main is refused; `topic` in the same push is created.
    let c2 = commit(bin, &work, "a", "two");
    ok(bin, &work, &["push", "-q", "origin", "main"]);
    ok(bin, &work, &["branch", "topic"]);
    ok(bin, &work, &["reset", "-q", "--hard", &c1]);
    let side = commit(bin, &work, "b", "side");
    let force = run(bin, &work, &["push", "-f", "origin", "main", "topic"]);
    assert!(!force.status.success(), "{label}: force push must fail");
    let e = stderr(&force);
    assert_rejected(label, &e, "main -> main", "cannot force-push to this ref");
    if label == "git" {
        assert!(e.contains("* [new branch]      topic -> topic"), "{e}");
    }
    assert_eq!(
        tip(&f.bare, "refs/heads/main").as_deref(),
        Some(c2.as_str())
    );
    assert_eq!(
        tip(&f.bare, "refs/heads/topic").as_deref(),
        Some(c2.as_str())
    );
    assert_ne!(side, c2);

    // A fast-forward on main is allowed.
    ok(bin, &work, &["reset", "-q", "--hard", &c2]);
    let c3 = commit(bin, &work, "a", "three");
    ok(bin, &work, &["push", "-q", "origin", "main"]);
    assert_eq!(
        tip(&f.bare, "refs/heads/main").as_deref(),
        Some(c3.as_str())
    );
    // Unprotected refs may still be deleted.
    ok(bin, &work, &["push", "-q", "origin", ":refs/heads/topic"]);
    assert_eq!(tip(&f.bare, "refs/heads/topic"), None);
}

#[test]
fn serve_refuses_ruleset_violations_with_arks_client() {
    let f = start("arks");
    journey(arks_bin(), &f, "arks");
}

#[test]
fn serve_refuses_ruleset_violations_with_stock_git_when_present() {
    // Stock git is a peer client of the test harness only, never a product path.
    let present = Command::new("git")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !present {
        eprintln!("skipped: stock git is not installed (advisory peer client only)");
        return;
    }
    let f = start("git");
    journey("git", &f, "git");
}
