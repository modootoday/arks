//! `arks push` against a smart-HTTP remote (`arks serve-http`): short
//! destination DWIM from the receive-pack advertisement and the per-ref
//! status table when receive-pack accepts some refs and rejects others.
//!
//! Expected output follows <https://git-scm.com/docs/git-push> (`<refspec>`,
//! OUTPUT) and observed stock git 2.43 against the same server; stock git is
//! not needed to run these. Governing rules: [[sot:root/arks-remote]] 7d, 7e.
#![cfg(unix)]

use std::net::{TcpListener, TcpStream};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

fn arks() -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_arks"));
    c.env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("ARKS_PERSONALITY", "git")
        .env("GIT_AUTHOR_NAME", "A U Thor")
        .env("GIT_AUTHOR_EMAIL", "author@example.com")
        .env("GIT_COMMITTER_NAME", "C O Mitter")
        .env("GIT_COMMITTER_EMAIL", "committer@example.com")
        .env("GIT_AUTHOR_DATE", "1700000000 +0000")
        .env("GIT_COMMITTER_DATE", "1700000000 +0000");
    c
}

fn run(dir: &Path, args: &[&str]) -> Output {
    arks()
        .args(args)
        .current_dir(dir)
        .stdin(Stdio::null())
        .output()
        .expect("spawn arks")
}

fn ok(dir: &Path, args: &[&str]) -> String {
    let out = run(dir, args);
    assert!(out.status.success(), "{args:?}: {out:?}");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

struct Running(Child);
impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// A bare remote served over smart HTTP whose `update` hook declines
/// `refs/heads/main`, plus a clone `w` with one new commit on `main` and a new
/// branch `topic` at the same commit.
struct Fixture {
    _tmp: tempfile::TempDir,
    _server: Running,
    bare: PathBuf,
    work: PathBuf,
    url: String,
}

fn fixture() -> Fixture {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("src");
    std::fs::create_dir_all(&src).unwrap();
    ok(&src, &["init", "-q", "-b", "main"]);
    ok(&src, &["commit", "-q", "--allow-empty", "-m", "one"]);
    let bare = tmp.path().join("bare.git");
    ok(
        tmp.path(),
        &[
            "clone",
            "-q",
            "--bare",
            src.to_str().unwrap(),
            bare.to_str().unwrap(),
        ],
    );
    let hook = bare.join("hooks/update");
    std::fs::create_dir_all(hook.parent().unwrap()).unwrap();
    std::fs::write(
        &hook,
        "#!/bin/sh\n[ \"$1\" = refs/heads/main ] && exit 1\nexit 0\n",
    )
    .unwrap();
    std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755)).unwrap();

    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let listen = format!("127.0.0.1:{port}");
    let server = Running(
        arks()
            .args(["serve-http", "--listen", &listen, bare.to_str().unwrap()])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let deadline = Instant::now() + Duration::from_secs(20);
    while TcpStream::connect(&listen).is_err() {
        assert!(Instant::now() < deadline, "serve-http did not start");
        std::thread::sleep(Duration::from_millis(50));
    }
    let url = format!("http://{listen}/");
    ok(tmp.path(), &["clone", "-q", &url, "w"]);
    let work = tmp.path().join("w");
    ok(&work, &["commit", "-q", "--allow-empty", "-m", "two"]);
    ok(&work, &["branch", "topic"]);
    Fixture {
        _tmp: tmp,
        _server: server,
        bare,
        work,
        url,
    }
}

fn remote_has(f: &Fixture, name: &str) -> bool {
    run(&f.bare, &["rev-parse", "--verify", "-q", name])
        .status
        .success()
}

fn tracking(f: &Fixture) -> String {
    ok(
        &f.work,
        &["for-each-ref", "--format=%(refname)", "refs/remotes/"],
    )
}

#[test]
fn short_deletion_resolves_against_the_advertised_refs() {
    let f = fixture();
    ok(&f.work, &["push", "-q", "origin", "topic"]);
    assert!(remote_has(&f, "refs/heads/topic"));

    let out = run(&f.work, &["push", "origin", ":topic"]);
    assert!(out.status.success(), "{out:?}");
    assert!(!remote_has(&f, "refs/heads/topic"));
    assert!(
        !tracking(&f).contains("refs/remotes/origin/topic"),
        "{}",
        tracking(&f)
    );

    // A name the remote does not advertise still fails before any request.
    let out = run(&f.work, &["push", "origin", ":nope"]);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(
        stderr(&out),
        format!(
            "error: unable to delete 'nope': remote ref does not exist\nerror: failed to push some refs to '{}'\n",
            f.url
        )
    );
}

#[test]
fn mixed_report_status_prints_per_ref_table() {
    let f = fixture();
    let out = run(&f.work, &["push", "origin", "main", "topic"]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert_eq!(
        stderr(&out),
        format!(
            "To {url}\n * [new branch]      topic -> topic\n ! [remote rejected] main -> main (hook declined)\nerror: failed to push some refs to '{url}'\n",
            url = f.url
        )
    );
    // The accepted ref landed and its remote-tracking ref moved; the rejected
    // ref and its tracking ref stayed where they were.
    let topic = ok(&f.work, &["rev-parse", "topic"]);
    assert_eq!(ok(&f.bare, &["rev-parse", "refs/heads/topic"]), topic);
    assert_eq!(
        ok(&f.work, &["rev-parse", "refs/remotes/origin/topic"]),
        topic
    );
    let old_main = ok(&f.bare, &["rev-parse", "refs/heads/main"]);
    assert_ne!(old_main, topic);
    assert_eq!(
        ok(&f.work, &["rev-parse", "refs/remotes/origin/main"]),
        old_main
    );

    // A refused deletion names only the remote ref.
    let out = run(&f.work, &["push", "origin", ":main"]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert_eq!(
        stderr(&out),
        format!(
            "To {url}\n ! [remote rejected] main (hook declined)\nerror: failed to push some refs to '{url}'\n",
            url = f.url
        )
    );
    assert!(remote_has(&f, "refs/heads/main"));

    // An update the remote already has is not sent; with nothing else to do
    // the push succeeds and lists it only under --verbose.
    let out = run(&f.work, &["push", "-v", "origin", "topic"]);
    assert!(out.status.success(), "{out:?}");
    assert!(
        stderr(&out).contains(" = [up to date]      topic -> topic\n"),
        "{}",
        stderr(&out)
    );
}

/// The table prettifies the source side like the destination, on the local
/// path too (observed stock git: `refs/heads/t:refs/heads/w` prints `t -> w`).
#[test]
fn status_table_shortens_full_source_names() {
    let tmp = tempfile::tempdir().unwrap();
    let a = tmp.path().join("a");
    std::fs::create_dir_all(&a).unwrap();
    ok(&a, &["init", "-q", "-b", "main"]);
    ok(&a, &["commit", "-q", "--allow-empty", "-m", "one"]);
    ok(tmp.path(), &["init", "-q", "--bare", "b.git"]);
    let out = run(
        &a,
        &[
            "push",
            "../b.git",
            "main",
            "refs/heads/main:refs/heads/w",
            "HEAD:hh",
        ],
    );
    assert!(out.status.success(), "{out:?}");
    assert_eq!(
        stderr(&out),
        "To ../b.git\n * [new branch]      main -> main\n * [new branch]      main -> w\n * [new branch]      HEAD -> hh\n"
    );
}
