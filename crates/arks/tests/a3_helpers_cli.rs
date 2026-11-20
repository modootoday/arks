//! A3 merge strategies and internal helpers: process-level behaviour of the
//! rows that stock git 2.43 (the host oracle of the flag corpus) no longer
//! or not yet ships (`env--helper`, `bisect--helper`, `add--interactive`,
//! `backfill`, `credential-helper-selector`), plus the stateful helpers the
//! corpus cannot drive with an empty stdin (`credential-cache` with a live
//! daemon, `remote-ext` / `remote-fd` / `remote-http` sessions against a
//! local `arks serve-http`, `upload-archive--writer`, `checkout--worker`,
//! `sh-i18n--envsubst`, octopus merges).
//!
//! Expected strings follow the git-scm documentation of each command and
//! observed stock git behaviour; stock git is not needed to run these.

use std::io::Write;
use std::net::{TcpListener, TcpStream};
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

fn run_in(dir: &Path, args: &[&str], stdin: &[u8]) -> Output {
    let mut child = arks()
        .args(args)
        .current_dir(dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn arks");
    child.stdin.take().unwrap().write_all(stdin).unwrap();
    child.wait_with_output().expect("wait")
}

fn ok(dir: &Path, args: &[&str]) -> String {
    let out = run_in(dir, args, b"");
    assert!(out.status.success(), "{args:?}: {out:?}");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

fn commit(dir: &Path, file: &str, body: &str, msg: &str) {
    std::fs::write(dir.join(file), body).unwrap();
    ok(dir, &["add", file]);
    ok(dir, &["commit", "-q", "-m", msg]);
}

/// main and three branches off `base`: topic and other touch new files,
/// clash edits the line main edits.
fn branchy_repo(root: &Path) -> PathBuf {
    let d = root.join("repo");
    std::fs::create_dir_all(&d).unwrap();
    ok(&d, &["init", "-q", "-b", "main"]);
    commit(&d, "a.txt", "one\ntwo\nthree\n", "base");
    ok(&d, &["tag", "base"]);
    ok(&d, &["checkout", "-q", "-b", "topic"]);
    commit(&d, "t.txt", "topic\n", "topic");
    ok(&d, &["checkout", "-q", "-b", "other", "base"]);
    commit(&d, "o.txt", "other\n", "other");
    ok(&d, &["checkout", "-q", "-b", "clash", "base"]);
    commit(&d, "a.txt", "one\ntwo\n3\n", "clash");
    ok(&d, &["checkout", "-q", "main"]);
    commit(&d, "a.txt", "one\ntwo\nTHREE\n", "main");
    d
}

#[test]
fn octopus_merge_through_git_merge_and_strategy_program() {
    let t = tempfile::tempdir().unwrap();
    let d = branchy_repo(t.path());
    let out = run_in(
        &d,
        &["merge-octopus", "base", "--", "HEAD", "topic", "other"],
        b"",
    );
    assert!(out.status.success(), "{out:?}");
    assert_eq!(
        stdout(&out),
        "Trying simple merge with topic\nTrying simple merge with other\n"
    );
    assert_eq!(ok(&d, &["status", "--porcelain"]), "A  o.txt\nA  t.txt\n");
    ok(&d, &["reset", "-q", "--hard"]);
    let out = run_in(&d, &["merge", "topic", "other"], b"");
    assert!(out.status.success(), "{out:?}");
    assert!(stdout(&out).contains("Merge made by the 'octopus' strategy."));
    let parents = ok(&d, &["log", "-1", "--format=%P"]);
    assert_eq!(parents.split_whitespace().count(), 3, "{parents}");
    let subject = ok(&d, &["log", "-1", "--format=%s"]);
    assert_eq!(subject, "Merge branches 'topic' and 'other'\n");
    ok(&d, &["reset", "-q", "--hard", "HEAD~1"]);
    let out = run_in(&d, &["merge", "topic", "clash"], b"");
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(stderr(&out).contains("ERROR: content conflict in a.txt"));
    let heads = std::fs::read_to_string(d.join(".git/MERGE_HEAD")).unwrap();
    assert_eq!(heads.lines().count(), 2);
}

#[test]
fn merge_ours_and_resolve_exit_codes() {
    let t = tempfile::tempdir().unwrap();
    let d = branchy_repo(t.path());
    assert!(
        run_in(&d, &["merge-ours", "base", "--", "HEAD", "topic"], b"")
            .status
            .success()
    );
    std::fs::write(d.join("a.txt"), "staged\n").unwrap();
    ok(&d, &["add", "a.txt"]);
    let out = run_in(&d, &["merge-ours", "base", "--", "HEAD", "topic"], b"");
    assert_eq!(out.status.code(), Some(2));
    ok(&d, &["reset", "-q", "--hard"]);
    let out = run_in(&d, &["merge-resolve", "base", "--", "HEAD", "clash"], b"");
    assert_eq!(out.status.code(), Some(1));
    let marked = std::fs::read_to_string(d.join("a.txt")).unwrap();
    assert!(marked.contains("<<<<<<< .merge_file_"), "{marked}");
    let out = run_in(&d, &["merge-index", "echo", "-a"], b"");
    assert!(out.status.success());
    assert!(stdout(&out).ends_with(" a.txt 100644 100644 100644\n"));
}

#[test]
fn env_helper_bool_ulong_and_exit_code() {
    let t = tempfile::tempdir().unwrap();
    let d = t.path();
    let env_run = |args: &[&str], var: Option<&str>| {
        let mut c = arks();
        c.arg("env--helper").args(args).current_dir(d);
        match var {
            Some(v) => c.env("A3_ENV_HELPER", v),
            None => c.env_remove("A3_ENV_HELPER"),
        };
        c.output().unwrap()
    };
    let o = env_run(&["--type=bool", "A3_ENV_HELPER"], Some("yes"));
    assert_eq!(stdout(&o), "true\n");
    let o = env_run(&["--type=bool", "--default=true", "A3_ENV_HELPER"], None);
    assert_eq!(stdout(&o), "true\n");
    let o = env_run(&["--type=bool", "A3_ENV_HELPER"], None);
    assert_eq!(stdout(&o), "false\n");
    let o = env_run(&["--type=ulong", "A3_ENV_HELPER"], Some("3k"));
    assert_eq!(stdout(&o), "3072\n");
    let o = env_run(
        &["--type=bool", "--exit-code", "A3_ENV_HELPER"],
        Some("off"),
    );
    assert_eq!((o.status.code(), stdout(&o)), (Some(1), String::new()));
    let o = env_run(&["--type=bool", "--exit-code", "A3_ENV_HELPER"], Some("1"));
    assert_eq!(o.status.code(), Some(0));
    let o = env_run(&["--type=bool", "A3_ENV_HELPER"], Some("maybe"));
    assert_eq!(o.status.code(), Some(128));
    assert!(stderr(&o).contains("bad boolean environment value 'maybe' for 'A3_ENV_HELPER'"));
    let o = env_run(&["--type=bool", "--default=xyz", "A3_ENV_HELPER"], None);
    assert_eq!(o.status.code(), Some(129));
    let o = env_run(&["A3_ENV_HELPER"], None);
    assert_eq!(o.status.code(), Some(129));
}

#[test]
fn sh_i18n_envsubst_substitutes_named_variables_only() {
    let t = tempfile::tempdir().unwrap();
    let mut child = arks()
        .args(["sh-i18n--envsubst", "x $A3_FOO $A3_UNSET"])
        .env("A3_FOO", "foo")
        .env("A3_BAR", "bar")
        .env_remove("A3_UNSET")
        .current_dir(t.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"a $A3_FOO b ${A3_BAR} c $A3_UNSET d $$ e ${A3_FOO\n")
        .unwrap();
    let o = child.wait_with_output().unwrap();
    assert_eq!(stdout(&o), "a foo b ${A3_BAR} c  d $$ e ${A3_FOO\n");
}

#[test]
fn bisect_and_add_interactive_helpers_route_to_their_porcelain() {
    let t = tempfile::tempdir().unwrap();
    let d = branchy_repo(t.path());
    let o = run_in(
        &d,
        &["bisect--helper", "--bisect-start", "main", "base"],
        b"",
    );
    assert!(o.status.success(), "{o:?}");
    // arks keeps its bisect state under .git/arks-bisect.
    assert!(d.join(".git/arks-bisect").exists());
    let o = run_in(&d, &["bisect--helper", "reset"], b"");
    assert!(o.status.success(), "{o:?}");
    assert!(!d.join(".git/arks-bisect").exists());
    let o = run_in(&d, &["bisect--helper", "--bogus"], b"");
    assert_eq!(o.status.code(), Some(129));
    let o = run_in(&d, &["add--interactive", "--patch=bogus"], b"");
    assert_eq!(o.status.code(), Some(128));
    assert!(stderr(&o).contains("unknown --patch mode: bogus"));
}

#[test]
fn backfill_without_promisor_has_nothing_to_fetch() {
    let t = tempfile::tempdir().unwrap();
    let d = branchy_repo(t.path());
    assert!(run_in(&d, &["backfill"], b"").status.success());
    assert!(
        run_in(&d, &["backfill", "--min-batch-size=10", "--no-sparse"], b"")
            .status
            .success()
    );
    let o = run_in(&d, &["backfill", "--min-batch-size=0"], b"");
    assert_eq!(o.status.code(), Some(128));
    let o = run_in(&d, &["backfill", "extra"], b"");
    assert_eq!(o.status.code(), Some(129));
}

fn pkt(payload: &str) -> String {
    format!("{:04x}{payload}", payload.len() + 4)
}

#[test]
fn upload_archive_writer_and_checkout_worker_streams() {
    let t = tempfile::tempdir().unwrap();
    let d = branchy_repo(t.path());
    let req = format!("{}0000", pkt("argument HEAD"));
    let o = run_in(&d, &["upload-archive--writer", "."], req.as_bytes());
    assert!(o.status.success(), "{o:?}");
    assert!(o.stdout.len() >= 1024 && o.stdout.len() % 512 == 0);
    assert_eq!(&o.stdout[257..262], b"ustar");
    let req = format!("{}{}0000", pkt("argument --remote=x"), pkt("argument HEAD"));
    let o = run_in(&d, &["upload-archive--writer", "."], req.as_bytes());
    assert_eq!(o.status.code(), Some(128));
    assert!(stderr(&o).contains("Unexpected option --remote"));
    let o = run_in(&d, &["checkout--worker"], b"0000");
    assert!(o.status.success(), "{o:?}");
    let o = run_in(&d, &["checkout--worker"], b"");
    assert_eq!(o.status.code(), Some(128));
}

#[cfg(unix)]
#[test]
fn credential_cache_daemon_round_trip() {
    let t = tempfile::tempdir().unwrap();
    let sock = t.path().join("cache").join("socket");
    let s = sock.to_str().unwrap();
    let cred = b"protocol=https\nhost=example.com\nusername=u\npassword=p\n\n";
    let o = run_in(
        t.path(),
        &[
            "credential-cache",
            "--socket",
            s,
            "--timeout",
            "30",
            "store",
        ],
        cred,
    );
    assert!(o.status.success(), "{o:?}");
    let q = b"protocol=https\nhost=example.com\n\n";
    let o = run_in(t.path(), &["credential-cache", "--socket", s, "get"], q);
    assert_eq!(stdout(&o), "username=u\npassword=p\n");
    let o = run_in(
        t.path(),
        &["credential-cache", "--socket", s, "get"],
        b"protocol=https\nhost=other.example\n\n",
    );
    assert_eq!(stdout(&o), "");
    let o = run_in(t.path(), &["credential-cache", "--socket", s, "erase"], q);
    assert!(o.status.success());
    // An empty cache makes the daemon exit and remove its socket.
    let deadline = Instant::now() + Duration::from_secs(10);
    while sock.exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(!sock.exists());
    let o = run_in(t.path(), &["credential-cache", "--socket", s, "get"], q);
    assert_eq!((o.status.code(), stdout(&o)), (Some(0), String::new()));
    let o = run_in(
        t.path(),
        &["credential-cache", "--socket", s, "store"],
        cred,
    );
    assert!(o.status.success());
    let o = run_in(t.path(), &["credential-cache", "--socket", s, "exit"], b"");
    assert!(o.status.success());
}

#[test]
fn credential_helper_selector_forwards_to_the_selected_helper() {
    let t = tempfile::tempdir().unwrap();
    let store = t.path().join("creds");
    let sel = |action: &str, input: &[u8]| {
        let mut child = arks()
            .args([
                "-c",
                "credential.helperselector.selected=store",
                "credential-helper-selector",
                action,
            ])
            .env("ARKS_CREDENTIAL_STORE", &store)
            .current_dir(t.path())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(input).unwrap();
        child.wait_with_output().unwrap()
    };
    let o = sel(
        "store",
        b"protocol=https\nhost=example.com\nusername=u\npassword=p\n\n",
    );
    assert!(o.status.success(), "{o:?}");
    let o = sel("get", b"protocol=https\nhost=example.com\n\n");
    assert!(stdout(&o).contains("password=p"), "{o:?}");
    let o = run_in(t.path(), &["credential-helper-selector"], b"");
    assert_eq!(o.status.code(), Some(129));
}

#[test]
fn remote_ext_and_fd_sessions() {
    let t = tempfile::tempdir().unwrap();
    let o = run_in(
        t.path(),
        &["remote-ext", "origin", "cat"],
        b"capabilities\n",
    );
    assert_eq!(stdout(&o), "*connect\n\n");
    let o = run_in(
        t.path(),
        &["remote-ext", "origin", "cat"],
        b"capabilities\n\n",
    );
    assert_eq!(o.status.code(), Some(1));
    assert_eq!(stderr(&o), "Bad command");
    let o = run_in(
        t.path(),
        &["remote-ext", "origin", "printenv GIT_EXT_SERVICE_NOPREFIX"],
        b"connect git-receive-pack\n",
    );
    assert_eq!(stdout(&o), "\nreceive-pack\n");
    let o = run_in(
        t.path(),
        &["remote-ext", "origin", "cat"],
        b"connect git-upload-pack\nping\n",
    );
    assert_eq!(stdout(&o), "\nping\n");
    let o = run_in(
        t.path(),
        &["remote-ext", "origin", "echo %q"],
        b"connect git-upload-pack\n",
    );
    assert_eq!(o.status.code(), Some(128));
    let o = run_in(t.path(), &["remote-fd", "origin", "0,1"], b"capabilities\n");
    assert_eq!(stdout(&o), "*connect\n\n");
    let o = run_in(
        t.path(),
        &["remote-fd", "origin", "0,1"],
        b"capabilities\n\n",
    );
    assert_eq!(o.status.code(), Some(128));
}

struct Running(Child);
impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn remote_http_lists_and_fetches_from_a_local_server() {
    let t = tempfile::tempdir().unwrap();
    let d = branchy_repo(t.path());
    let bare = t.path().join("bare.git");
    ok(
        t.path(),
        &[
            "clone",
            "-q",
            "--bare",
            d.to_str().unwrap(),
            bare.to_str().unwrap(),
        ],
    );
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let listen = format!("127.0.0.1:{port}");
    let _server = Running(
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
    let o = run_in(
        t.path(),
        &["remote-http", "origin", &url],
        b"capabilities\n\n",
    );
    assert_eq!(
        stdout(&o),
        "stateless-connect\nfetch\nget\noption\npush\ncheck-connectivity\nobject-format\n\n"
    );
    let o = run_in(
        t.path(),
        &["remote-http", "origin", &url],
        b"option verbosity 1\noption bogus x\noption depth z\n\n",
    );
    assert_eq!(stdout(&o), "ok\nunsupported\nerror invalid depth\n");
    let o = run_in(t.path(), &["remote-http", "origin", &url], b"list\n\n");
    assert!(o.status.success(), "{o:?}");
    let listing = stdout(&o);
    let main = ok(&d, &["rev-parse", "main"]);
    assert!(
        listing.contains(&format!("{} refs/heads/main\n", main.trim())),
        "{listing}"
    );
    // Fetch the clash tip into a fresh repository (objects only, no refs).
    let clash = ok(&d, &["rev-parse", "clash"]);
    let fresh = t.path().join("fresh");
    std::fs::create_dir_all(&fresh).unwrap();
    ok(&fresh, &["init", "-q"]);
    let req = format!("fetch {} refs/heads/clash\n\n\n", clash.trim());
    let o = run_in(&fresh, &["remote-http", "origin", &url], req.as_bytes());
    assert!(o.status.success(), "{o:?}");
    assert_eq!(stdout(&o), "\n");
    assert_eq!(ok(&fresh, &["cat-file", "-t", clash.trim()]), "commit\n");
    let o = run_in(
        t.path(),
        &["remote-ftp", "origin", "ftp://127.0.0.1:9/x"],
        b"list\n\n",
    );
    assert_eq!(o.status.code(), Some(128));
}
