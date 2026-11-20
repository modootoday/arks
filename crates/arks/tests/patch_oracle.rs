//! Interactive hunk selection (`add -p`, `reset -p`, `checkout -p`,
//! `restore -p`, `stash push -p`) with scripted answers on stdin, compared
//! byte for byte with stock git: the transcript on stdout and the resulting
//! index, work tree and stash.
//!
//! Stock git is the oracle only: it builds each fixture and reads back the
//! state. The comparison is skipped when `git` is not on PATH.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

fn git_available() -> bool {
    Command::new("git")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}

/// The transcripts follow the prompt set of the pin (git 2.39.5). Later git
/// releases add hunk commands and change which letters a prompt lists, so the
/// stdout comparison applies only to oracles up to 2.43 (the pin and the
/// host-advisory oracle); exit code and resulting state are compared always.
fn oracle_prompts_match_pin() -> bool {
    let out = Command::new("git").arg("--version").output().unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    let mut nums = text
        .split_whitespace()
        .nth(2)
        .unwrap_or("")
        .split('.')
        .map(|n| n.parse::<u32>().unwrap_or(u32::MAX));
    let (major, minor) = (nums.next().unwrap_or(0), nums.next().unwrap_or(0));
    major == 2 && (39..=43).contains(&minor)
}

fn run(dir: &Path, prog: &str, args: &[&str], stdin: &[u8]) -> (i32, Vec<u8>) {
    let mut child = Command::new(prog)
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_AUTHOR_NAME", "A U Thor")
        .env("GIT_AUTHOR_EMAIL", "author@example.com")
        .env("GIT_COMMITTER_NAME", "C O Mitter")
        .env("GIT_COMMITTER_EMAIL", "committer@example.com")
        .env("GIT_AUTHOR_DATE", "1700000000 +0000")
        .env("GIT_COMMITTER_DATE", "1700000000 +0000")
        .env("GIT_EDITOR", "true")
        .env("GIT_PAGER", "cat")
        .env("ARKS_PERSONALITY", "git")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    child.stdin.take().unwrap().write_all(stdin).unwrap();
    let out = child.wait_with_output().unwrap();
    (out.status.code().unwrap_or(-1), out.stdout)
}

fn git(dir: &Path, args: &[&str]) -> String {
    let (code, out) = run(dir, "git", args, b"");
    assert_eq!(code, 0, "git {args:?}");
    String::from_utf8(out).unwrap()
}

/// History with a two-hunk file, a splittable hunk, a staged addition, a
/// mode change and a binary file; then unstaged edits on top.
fn fixture(dir: &Path) {
    git(dir, &["init", "-q", "-b", "main", "."]);
    let a: String = (1..=12).map(|i| format!("line {i}\n")).collect();
    std::fs::write(dir.join("a.txt"), &a).unwrap();
    std::fs::write(dir.join("s.txt"), "fn one\na\nb\nc\nd\ne\nf\ng\n").unwrap();
    std::fs::write(dir.join("m.sh"), "echo\n").unwrap();
    std::fs::write(dir.join("bin.dat"), b"bin\0one\n").unwrap();
    git(dir, &["add", "."]);
    git(dir, &["commit", "-q", "-m", "base"]);
    std::fs::write(
        dir.join("a.txt"),
        a.replace("line 2\n", "LINE 2\n")
            .replace("line 11\n", "LINE 11\nextra\n"),
    )
    .unwrap();
    git(dir, &["commit", "-q", "-am", "second"]);
    let a2 = a
        .replace("line 1\n", "line one\n")
        .replace("line 12\n", "line twelve\n");
    std::fs::write(dir.join("a.txt"), a2).unwrap();
    std::fs::write(dir.join("s.txt"), "fn one\nA\nb\nc\nd\nE\nf\ng\n").unwrap();
    std::fs::write(dir.join("new.txt"), "staged\n").unwrap();
    git(dir, &["add", "new.txt"]);
    std::fs::write(dir.join("bin.dat"), b"bin\0two\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(dir.join("m.sh"), std::fs::Permissions::from_mode(0o755)).unwrap();
    }
}

fn state(dir: &Path) -> String {
    let mut s = git(dir, &["status", "--porcelain=v1", "-uall"]);
    s.push_str(&git(dir, &["diff", "--cached"]));
    s.push_str(&git(dir, &["diff"]));
    s.push_str(&git(dir, &["stash", "list", "--format=%gs %T"]));
    s
}

fn compare(args: &[&str], answers: &str) {
    let g = tempfile::tempdir().unwrap();
    let a = tempfile::tempdir().unwrap();
    fixture(g.path());
    fixture(a.path());
    let (gc, go) = run(g.path(), "git", args, answers.as_bytes());
    let (ac, ao) = run(
        a.path(),
        env!("CARGO_BIN_EXE_arks"),
        args,
        answers.as_bytes(),
    );
    if oracle_prompts_match_pin() {
        assert_eq!(
            String::from_utf8_lossy(&ao),
            String::from_utf8_lossy(&go),
            "stdout of {args:?} with {answers:?}"
        );
    }
    assert_eq!(ac, gc, "exit of {args:?} with {answers:?}");
    assert_eq!(
        state(a.path()),
        state(g.path()),
        "state after {args:?} with {answers:?}"
    );
}

#[test]
fn add_patch_answers() {
    if !git_available() {
        return;
    }
    for answers in [
        "",
        "y\n",
        "n\ny\n",
        "y\nn\ny\nn\ny\n",
        "a\n",
        "d\na\n",
        "q\n",
        "s\ny\nn\n",
        "j\nk\nJ\nK\ny\ny\n",
        "g\n2\ny\nq\n",
        "/twelve\ny\nq\n",
        "?\nx\n\ny\n",
        "e\ny\n",
        "y\ny\ny\ny\ny\ny\n",
    ] {
        compare(&["add", "-p"], answers);
    }
    compare(&["add", "-p", "--", "s.txt"], "s\nn\ny\n");
}

#[test]
fn reset_checkout_restore_patch_answers() {
    if !git_available() {
        return;
    }
    for (args, answers) in [
        (vec!["reset", "-p"], "y\n"),
        (vec!["reset", "-p", "HEAD~1"], "y\nn\ny\n"),
        (vec!["reset", "-p", "HEAD~1", "--", "a.txt"], "s\ny\nn\n"),
        (vec!["checkout", "-p"], "y\nn\ny\ny\n"),
        (vec!["checkout", "-p", "HEAD"], "y\ny\n"),
        (vec!["checkout", "-p", "HEAD~1", "--", "a.txt"], "a\n"),
        (vec!["restore", "-p"], "n\ny\nq\n"),
        (vec!["restore", "-p", "--staged"], "y\n"),
        (vec!["restore", "-p", "--source=HEAD~1", "a.txt"], "y\nn\n"),
        (vec!["restore", "-p", "--staged", "--worktree"], "y\n"),
    ] {
        compare(&args, answers);
    }
}

#[test]
fn stash_patch_answers() {
    if !git_available() {
        return;
    }
    for answers in ["y\n", "n\ny\n", "n\nn\nn\nn\nn\n", "a\n"] {
        compare(&["stash", "push", "-p"], answers);
    }
    compare(
        &["stash", "push", "-p", "-m", "msg", "--", "a.txt"],
        "y\nn\n",
    );
}
