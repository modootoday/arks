//! Packs, bitmaps, fast-export streams, commit-graphs and multi-pack-indexes
//! written by arks, read back by stock git.
//!
//! Stock git is the oracle only: it builds the fixture repositories and
//! checks what arks wrote (`index-pack`, `verify-pack`, `rev-list
//! --test-bitmap`, `fast-import`, `commit-graph verify`, `multi-pack-index
//! verify`). The tests are skipped when `git` is not on PATH.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

fn git_available() -> bool {
    Command::new("git")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}

fn cmd(dir: &Path, prog: &str, args: &[&str], stdin: &[u8]) -> (bool, Vec<u8>, String) {
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
        .env("ARKS_PERSONALITY", "git")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    child.stdin.take().unwrap().write_all(stdin).unwrap();
    let out = child.wait_with_output().unwrap();
    (
        out.status.success(),
        out.stdout,
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn git(dir: &Path, args: &[&str]) -> String {
    let (ok, out, err) = cmd(dir, "git", args, b"");
    assert!(ok, "git {args:?} failed: {err}");
    String::from_utf8(out).unwrap().trim().to_string()
}

fn arks(dir: &Path, args: &[&str], stdin: &[u8]) -> Vec<u8> {
    let (ok, out, err) = cmd(dir, env!("CARGO_BIN_EXE_arks"), args, stdin);
    assert!(ok, "arks {args:?} failed: {err}");
    out
}

/// A small history with similar file versions (deltas), a merge and tags.
fn fixture() -> tempfile::TempDir {
    let tmp = tempfile::tempdir().unwrap();
    let d = tmp.path();
    git(d, &["init", "-q", "-b", "main", "."]);
    let mut text = String::new();
    for i in 0..200 {
        text.push_str(&format!("line {i} of a file that changes a little\n"));
    }
    for round in 0..4 {
        text.push_str(&format!("round {round}\n"));
        std::fs::write(d.join("big.txt"), &text).unwrap();
        std::fs::write(d.join(format!("f{round}.txt")), format!("{round}\n")).unwrap();
        git(d, &["add", "."]);
        git(d, &["commit", "-q", "-m", &format!("round {round}")]);
    }
    git(d, &["tag", "-a", "-m", "tag", "v1", "HEAD~1"]);
    git(d, &["checkout", "-q", "-b", "side", "HEAD~2"]);
    std::fs::write(d.join("side.txt"), "side\n").unwrap();
    git(d, &["add", "."]);
    git(d, &["commit", "-q", "-m", "side"]);
    git(d, &["checkout", "-q", "main"]);
    git(d, &["merge", "-q", "--no-ff", "-m", "merge", "side"]);
    tmp
}

fn object_set(dir: &Path, idx: &str) -> Vec<String> {
    let out = git(dir, &["verify-pack", "-v", idx]);
    let mut v: Vec<String> = out
        .lines()
        .filter(|l| l.len() > 40 && l.as_bytes()[40] == b' ')
        .map(|l| l[..40].to_string())
        .collect();
    v.sort();
    v
}

#[test]
fn packs_read_back_by_stock_git() {
    if !git_available() {
        return;
    }
    let repo = fixture();
    let d = repo.path();
    let want = {
        let out = git(d, &["rev-list", "--objects", "--all"]);
        let mut v: Vec<String> = out.lines().map(|l| l[..40].to_string()).collect();
        v.sort();
        v
    };
    for (i, opts) in [
        vec!["--revs", "--all"],
        vec!["--revs", "--all", "--delta-base-offset"],
        vec!["--revs", "--all", "--window=0"],
        vec!["--revs", "--all", "--depth=1", "--compression=0"],
        vec!["--revs", "--all", "--index-version=1"],
        vec![
            "--all",
            "--include-tag",
            "--threads=1",
            "--window-memory=1m",
        ],
    ]
    .into_iter()
    .enumerate()
    {
        let base = format!("out{i}");
        let mut args = vec!["pack-objects"];
        args.extend(opts.iter().copied());
        args.push(&base);
        let hex = String::from_utf8(arks(d, &args, b"")).unwrap();
        let idx = format!("{base}-{}.idx", hex.trim());
        assert_eq!(object_set(d, &idx), want, "{opts:?}");
    }
    // --stdout packs go through index-pack.
    let pack = arks(d, &["pack-objects", "--revs", "--stdout"], b"main\n^v1\n");
    let (ok, out, err) = cmd(d, "git", &["index-pack", "--stdin"], &pack);
    assert!(ok, "index-pack: {err}");
    assert!(String::from_utf8_lossy(&out).starts_with("pack\t"));
    // An object-list pack holds exactly the listed objects.
    let head = git(d, &["rev-parse", "HEAD"]);
    let tree = git(d, &["rev-parse", "HEAD^{tree}"]);
    let pack = arks(
        d,
        &["pack-objects", "--stdout"],
        format!("{head}\n{tree} name\n").as_bytes(),
    );
    let (ok, _, err) = cmd(d, "git", &["index-pack", "--stdin"], &pack);
    assert!(ok, "index-pack: {err}");
}

#[test]
fn bitmaps_pass_stock_test_bitmap() {
    if !git_available() {
        return;
    }
    let repo = fixture();
    let d = repo.path();
    arks(
        d,
        &[
            "pack-objects",
            "--revs",
            "--all",
            "--write-bitmap-index",
            ".git/objects/pack/pack",
        ],
        b"",
    );
    let (ok, _, err) = cmd(d, "git", &["rev-list", "--test-bitmap", "main"], b"");
    assert!(ok, "test-bitmap: {err}");
    assert!(err.contains("OK!"), "{err}");
}

#[test]
fn fast_export_streams_import_into_stock_git() {
    if !git_available() {
        return;
    }
    let repo = fixture();
    let d = repo.path();
    let stream = arks(d, &["fast-export", "--all", "--signed-tags=strip"], b"");
    let target = tempfile::tempdir().unwrap();
    let t = target.path();
    git(t, &["init", "-q", "--bare", "."]);
    let (ok, _, err) = cmd(t, "git", &["fast-import", "--quiet"], &stream);
    assert!(ok, "fast-import: {err}");
    for r in ["refs/heads/main", "refs/heads/side", "refs/tags/v1"] {
        assert_eq!(git(d, &["rev-parse", r]), git(t, &["rev-parse", r]), "{r}");
    }
}

#[test]
fn commit_graph_and_midx_verify_with_stock_git() {
    if !git_available() {
        return;
    }
    let repo = fixture();
    let d = repo.path();
    arks(d, &["commit-graph", "write", "--reachable"], b"");
    let (ok, _, err) = cmd(d, "git", &["commit-graph", "verify"], b"");
    assert!(ok, "commit-graph verify: {err}");
    arks(d, &["commit-graph", "write", "--reachable", "--split"], b"");
    let (ok, _, err) = cmd(d, "git", &["commit-graph", "verify"], b"");
    assert!(ok, "split commit-graph verify: {err}");
    git(d, &["repack", "-adq"]);
    arks(d, &["multi-pack-index", "write"], b"");
    let (ok, _, err) = cmd(d, "git", &["multi-pack-index", "verify"], b"");
    assert!(ok, "multi-pack-index verify: {err}");
}
