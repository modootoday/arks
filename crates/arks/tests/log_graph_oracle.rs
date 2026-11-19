//! `log --graph` byte-for-byte against stock git on small branchy histories.
//!
//! Stock git is the oracle only: it builds the fixture repositories and
//! renders the expected output. The comparison is skipped when `git` is not
//! on PATH.

use std::path::Path;
use std::process::Command;

fn git_available() -> bool {
    Command::new("git")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}

fn run(dir: &Path, prog: &str, args: &[&str]) -> (bool, Vec<u8>) {
    let out = Command::new(prog)
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_PAGER", "cat")
        .env("TZ", "UTC")
        .env("ARKS_PERSONALITY", "git")
        .output()
        .expect("spawn");
    (out.status.success(), out.stdout)
}

fn git(dir: &Path, args: &[&str]) -> String {
    let (ok, out) = run(dir, "git", args);
    assert!(ok, "git {args:?} failed");
    String::from_utf8(out).unwrap().trim().to_string()
}

/// Repository whose history is a DAG of empty-tree commits, created in the
/// listed order with increasing dates. Each entry: (name, parent names).
/// Every name becomes a branch `refs/heads/<name>`; `head` is checked out.
fn build(dag: &[(&str, &[&str])], head: &str) -> tempfile::TempDir {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    git(dir, &["init", "-q", "-b", "main", "."]);
    let tree = git(dir, &["write-tree"]);
    let mut oids: Vec<(String, String)> = Vec::new();
    for (i, (name, parents)) in dag.iter().enumerate() {
        let mut args = vec!["commit-tree".to_string(), tree.clone()];
        for p in *parents {
            let oid = &oids.iter().find(|(n, _)| n == p).expect("parent").1;
            args.push("-p".into());
            args.push(oid.clone());
        }
        args.push("-m".into());
        args.push(name.to_string());
        let date = format!("{} +0000", 1_700_000_000 + 60 * i as i64);
        let out = Command::new("git")
            .args(&args)
            .current_dir(dir)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_AUTHOR_NAME", "A U Thor")
            .env("GIT_AUTHOR_EMAIL", "author@example.com")
            .env("GIT_COMMITTER_NAME", "C O Mitter")
            .env("GIT_COMMITTER_EMAIL", "committer@example.com")
            .env("GIT_AUTHOR_DATE", &date)
            .env("GIT_COMMITTER_DATE", &date)
            .output()
            .unwrap();
        assert!(out.status.success(), "commit-tree {name}");
        let oid = String::from_utf8(out.stdout).unwrap().trim().to_string();
        git(dir, &["update-ref", &format!("refs/heads/{name}"), &oid]);
        oids.push((name.to_string(), oid));
    }
    git(
        dir,
        &["symbolic-ref", "HEAD", &format!("refs/heads/{head}")],
    );
    tmp
}

const ARGSETS: &[&[&str]] = &[
    &["log", "--graph", "--oneline", "--all"],
    &["log", "--graph", "--format=%s", "--all"],
    &["log", "--graph", "--all"],
    &["log", "--graph", "--oneline", "--decorate", "--all"],
    &["log", "--graph", "--format=%h %s", "-5", "--all"],
    &["log", "--graph", "--oneline"],
    &["log", "--graph", "--date-order", "--oneline", "--all"],
    &["log", "--graph", "--first-parent", "--oneline", "--all"],
];

fn assert_same_as_git(dag: &[(&str, &[&str])], head: &str) {
    if !git_available() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let repo = build(dag, head);
    for args in ARGSETS {
        let (gok, want) = run(repo.path(), "git", args);
        let (aok, got) = run(repo.path(), env!("CARGO_BIN_EXE_arks"), args);
        assert_eq!(aok, gok, "exit status differs for {args:?}");
        assert_eq!(
            String::from_utf8_lossy(&got),
            String::from_utf8_lossy(&want),
            "output differs for {args:?}"
        );
    }
}

#[test]
fn two_branches_with_several_commits_each() {
    assert_same_as_git(
        &[
            ("root", &[]),
            ("a1", &["root"]),
            ("b1", &["root"]),
            ("a2", &["a1"]),
            ("b2", &["b1"]),
            ("a3", &["a2"]),
            ("b3", &["b2"]),
            ("main", &["a3", "b3"]),
            ("tip", &["main"]),
        ],
        "tip",
    );
}

#[test]
fn criss_cross_merges() {
    assert_same_as_git(
        &[
            ("root", &[]),
            ("x1", &["root"]),
            ("y1", &["root"]),
            ("x2", &["x1", "y1"]),
            ("y2", &["y1", "x1"]),
            ("x3", &["x2"]),
            ("y3", &["y2"]),
            ("main", &["x3", "y3"]),
        ],
        "main",
    );
}

#[test]
fn octopus_merges() {
    assert_same_as_git(
        &[
            ("root", &[]),
            ("a", &["root"]),
            ("b", &["root"]),
            ("c", &["root"]),
            ("d", &["root"]),
            ("side", &["a"]),
            ("m1", &["root"]),
            ("oct3", &["m1", "a", "b"]),
            ("m2", &["oct3"]),
            ("oct4", &["m2", "b", "c", "d"]),
            ("main", &["oct4"]),
        ],
        "main",
    );
}

#[test]
fn merge_whose_first_parent_lies_left() {
    // `feature` merges `main` back in while `main` keeps going: the merge's
    // first parent is in a lane left of the merge itself.
    assert_same_as_git(
        &[
            ("root", &[]),
            ("m1", &["root"]),
            ("f1", &["root"]),
            ("m2", &["m1"]),
            ("f2", &["f1", "m2"]),
            ("m3", &["m2"]),
            ("f3", &["m3", "f2"]),
            ("main", &["m3"]),
        ],
        "main",
    );
}
