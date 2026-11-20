//! `arks gui` / `arks citool` / `gui.tcl` / `gui--askpass` / `gui--askyesno`
//! as a terminal UI: argv of git-gui (`blame`, `browser`, `citool [--amend]
//! [--nocommit]`, `version`), the scripted line front-end used without a
//! terminal, the full-screen view driven with key bytes
//! (`ARKS_GUI_TERMINAL=force`), and the citool exit status rule (non-zero
//! unless a commit was made).
//!
//! Primary: <https://git-scm.com/docs/git-gui>, <https://git-scm.com/docs/git-citool>.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn arks(dir: &Path) -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_arks"));
    c.current_dir(dir)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("ARKS_PERSONALITY", "git")
        .env("GIT_AUTHOR_NAME", "A U Thor")
        .env("GIT_AUTHOR_EMAIL", "author@example.com")
        .env("GIT_COMMITTER_NAME", "C O Mitter")
        .env("GIT_COMMITTER_EMAIL", "committer@example.com")
        .env("GIT_AUTHOR_DATE", "1700000000 +0000")
        .env("GIT_COMMITTER_DATE", "1700000000 +0000")
        .env_remove("ARKS_GUI_TERMINAL");
    c
}

fn run(dir: &Path, args: &[&str], stdin: &[u8], envs: &[(&str, &str)]) -> Output {
    let mut c = arks(dir);
    c.args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (k, v) in envs {
        c.env(k, v);
    }
    let mut child = c.spawn().expect("spawn arks");
    child.stdin.take().unwrap().write_all(stdin).unwrap();
    child.wait_with_output().unwrap()
}

fn ok(dir: &Path, args: &[&str]) -> String {
    let o = run(dir, args, b"", &[]);
    assert!(o.status.success(), "{args:?}: {o:?}");
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn repo(root: &Path) -> PathBuf {
    let d = root.join("w");
    std::fs::create_dir_all(&d).unwrap();
    ok(&d, &["init", "-q", "-b", "main"]);
    std::fs::write(d.join("a.txt"), "one\n").unwrap();
    std::fs::write(d.join("b.txt"), "two\n").unwrap();
    d
}

fn has_head(d: &Path) -> bool {
    run(d, &["rev-parse", "--verify", "-q", "HEAD"], b"", &[])
        .status
        .success()
}

fn subjects(d: &Path) -> String {
    ok(d, &["log", "--format=%s"])
}

#[test]
fn version_line() {
    let t = tempfile::tempdir().unwrap();
    let o = run(t.path(), &["gui", "version"], b"", &[]);
    assert!(o.status.success());
    assert_eq!(
        String::from_utf8_lossy(&o.stdout),
        format!("arks gui version {}\n", env!("CARGO_PKG_VERSION"))
    );
    let o = run(t.path(), &["gui.tcl", "version"], b"", &[]);
    assert!(o.status.success());
}

#[test]
fn argv_errors_are_usage() {
    let t = tempfile::tempdir().unwrap();
    let d = repo(t.path());
    for args in [
        &["gui", "nosuch"][..],
        &["citool", "--bogus"],
        &["citool", "extra"],
        &["gui", "blame"],
        &["gui", "browser", "a", "b"],
        &["gui", "version", "x"],
    ] {
        let o = run(&d, args, b"", &[]);
        assert_eq!(o.status.code(), Some(129), "{args:?}: {o:?}");
    }
}

#[test]
fn citool_script_commits_once() {
    let t = tempfile::tempdir().unwrap();
    let d = repo(t.path());
    let o = run(
        &d,
        &["citool"],
        b"status\nstage a.txt\ndiff a.txt\nmessage add a\nsignoff\ncommit\nmessage ignored\n",
        &[],
    );
    assert!(o.status.success(), "{o:?}");
    let out = String::from_utf8_lossy(&o.stdout);
    assert!(
        out.contains("Unstaged Changes:\n  ? a.txt\n  ? b.txt\n"),
        "{out}"
    );
    assert!(out.contains("+one"), "{out}");
    assert!(out.contains("Created commit "), "{out}");
    assert_eq!(subjects(&d), "add a\n");
    let body = ok(&d, &["log", "-1", "--format=%b"]);
    assert_eq!(
        body,
        "Signed-off-by: C O Mitter <committer@example.com>\n\n"
    );
    // b.txt was never staged.
    assert_eq!(ok(&d, &["ls-files"]), "a.txt\n");
}

#[test]
fn citool_without_commit_exits_nonzero() {
    let t = tempfile::tempdir().unwrap();
    let d = repo(t.path());
    let o = run(&d, &["citool"], b"stage a.txt\nquit\n", &[]);
    assert_eq!(o.status.code(), Some(1));
    let o = run(&d, &["citool"], b"", &[]);
    assert_eq!(o.status.code(), Some(1));
    // Refused commits (no message) keep the window open; EOF then closes it.
    let o = run(&d, &["citool"], b"commit\n", &[]);
    assert_eq!(o.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&o.stderr).contains("commit message"));
    assert!(!has_head(&d), "no commit was made");
}

#[test]
fn citool_nocommit_and_amend() {
    let t = tempfile::tempdir().unwrap();
    let d = repo(t.path());
    let o = run(&d, &["citool", "--nocommit"], b"stage a.txt\ncommit\n", &[]);
    assert!(o.status.success(), "{o:?}");
    assert!(!has_head(&d));
    // Index changes made in the window stay.
    assert_eq!(ok(&d, &["ls-files"]), "a.txt\n");
    let o = run(&d, &["citool"], b"message first\ncommit\n", &[]);
    assert!(o.status.success(), "{o:?}");
    // --amend loads the last message; staging b.txt and committing rewrites HEAD.
    let o = run(
        &d,
        &["gui", "citool", "--amend"],
        b"stage b.txt\ncommit\n",
        &[],
    );
    assert!(o.status.success(), "{o:?}");
    assert_eq!(subjects(&d), "first\n");
    assert_eq!(ok(&d, &["ls-files"]), "a.txt\nb.txt\n");
}

#[test]
fn gui_window_keeps_going_after_commit() {
    let t = tempfile::tempdir().unwrap();
    let d = repo(t.path());
    let o = run(
        &d,
        &["gui"],
        b"stage a.txt\nmessage one\ncommit\nstage b.txt\nmessage two\ncommit\nstatus\nunstage nosuch\n",
        &[],
    );
    assert!(o.status.success(), "{o:?}");
    assert_eq!(subjects(&d), "two\none\n");
    std::fs::write(d.join("a.txt"), "changed\n").unwrap();
    let o = run(&d, &["gui"], b"stage-changed\nunstage a.txt\nstatus\n", &[]);
    let out = String::from_utf8_lossy(&o.stdout);
    assert!(
        out.contains("Unstaged Changes:\n  M a.txt\nStaged Changes (Will Commit):\n"),
        "{out}"
    );
}

#[test]
fn blame_and_browser_print_without_terminal() {
    let t = tempfile::tempdir().unwrap();
    let d = repo(t.path());
    ok(&d, &["add", "a.txt", "b.txt"]);
    ok(&d, &["commit", "-q", "-m", "c1"]);
    let out = ok(&d, &["gui", "browser"]);
    assert_eq!(out, "a.txt\nb.txt\n");
    let out = ok(&d, &["gui", "browser", "main"]);
    assert_eq!(out, "a.txt\nb.txt\n");
    let blame = ok(&d, &["gui", "blame", "--line=1", "HEAD", "a.txt"]);
    assert_eq!(blame, ok(&d, &["blame", "HEAD", "--", "a.txt"]));
    assert!(blame.contains(") one"), "{blame}");
    let o = run(&d, &["gui", "browser", "nosuch"], b"", &[]);
    assert_eq!(o.status.code(), Some(128));
}

#[test]
fn screen_view_driven_by_keys() {
    let t = tempfile::tempdir().unwrap();
    let d = repo(t.path());
    let force = [("ARKS_GUI_TERMINAL", "force")];
    // Down to b.txt, stage it, edit the message, leave editing (Esc), commit.
    let o = run(&d, &["citool"], b"j\recommit b\x1bc", &force);
    assert!(o.status.success(), "{o:?}");
    let screen = String::from_utf8_lossy(&o.stdout);
    assert!(screen.contains("Unstaged Changes"), "{screen}");
    assert!(screen.contains("Staged Changes (Will Commit)"), "{screen}");
    assert_eq!(subjects(&d), "commit b\n");
    assert_eq!(ok(&d, &["ls-files"]), "b.txt\n");
    // Closing the window with q exits 1.
    let o = run(&d, &["citool"], b"q", &force);
    assert_eq!(o.status.code(), Some(1));
    // Blame view closes with q.
    let o = run(&d, &["gui", "blame", "b.txt"], b"jjq", &force);
    assert!(o.status.success(), "{o:?}");
}

#[test]
fn askpass_and_askyesno() {
    let t = tempfile::tempdir().unwrap();
    let o = run(
        t.path(),
        &["gui--askpass", "Password for 'https://x@h':"],
        b"s3cret\n",
        &[],
    );
    assert!(o.status.success());
    assert_eq!(String::from_utf8_lossy(&o.stdout), "s3cret\n");
    assert!(String::from_utf8_lossy(&o.stderr).contains("Password for 'https://x@h':"));
    let o = run(t.path(), &["gui--askpass"], b"", &[]);
    assert_eq!(o.status.code(), Some(1));
    for (input, code) in [
        (&b"y\n"[..], 0),
        (b"yes\n", 0),
        (b"n\n", 1),
        (b"", 1),
        (b"?\n", 1),
    ] {
        let o = run(t.path(), &["gui--askyesno", "Retry?"], input, &[]);
        assert_eq!(o.status.code(), Some(code), "{input:?}");
    }
}

/// Real pseudo-terminal through util-linux `script(1)` when it is installed.
#[test]
fn citool_on_a_pseudo_terminal() {
    let pty_ok = Command::new("script")
        .args(["-qec", "true", "/dev/null"])
        .stdin(Stdio::null())
        .output()
        .is_ok_and(|o| o.status.success());
    if !pty_ok {
        return;
    }
    let t = tempfile::tempdir().unwrap();
    let d = repo(t.path());
    let bin = env!("CARGO_BIN_EXE_arks");
    let sh = format!(
        "(sleep 1.5; printf '\\r'; sleep 0.5; printf 'epty'; sleep 0.2; printf '\\033'; sleep 0.3; printf c; sleep 1) | script -qec '{bin} citool' /dev/null >/dev/null"
    );
    let st = Command::new("sh")
        .arg("-c")
        .arg(&sh)
        .current_dir(&d)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("ARKS_PERSONALITY", "git")
        .env("GIT_AUTHOR_NAME", "A")
        .env("GIT_AUTHOR_EMAIL", "a@example.com")
        .env("GIT_COMMITTER_NAME", "C")
        .env("GIT_COMMITTER_EMAIL", "c@example.com")
        .status()
        .unwrap();
    assert!(st.success());
    assert_eq!(subjects(&d), "pty\n");
}
