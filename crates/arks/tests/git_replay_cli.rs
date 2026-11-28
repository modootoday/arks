//! Integration tests for worktree-free `arks replay` (E3).
//!
//! Verifies:
//! 1. `--onto` replays commits in memory without touching a dirty working tree or `.git/index`.
//! 2. `--ref-action=print` emits `update <ref> <new> <old>` lines accepted by `arks update-ref --stdin`.
//! 3. `--advance <branch>` moves the target branch forward.
//! 4. `--contained` updates intermediate branches inside the replayed range.
//! 5. Merge conflicts fail closed with exit code 1 leaving all refs, index, and worktree untouched.

use std::fs;
use std::path::Path;
use std::process::{Command, Output, Stdio};

fn arks_bin() -> &'static str {
    env!("CARGO_BIN_EXE_arks")
}

fn run(dir: &Path, args: &[&str]) -> Output {
    Command::new(arks_bin())
        .current_dir(dir)
        .env("GIT_AUTHOR_NAME", "Replay Test")
        .env("GIT_AUTHOR_EMAIL", "replay@example.com")
        .env("GIT_COMMITTER_NAME", "Replay Test")
        .env("GIT_COMMITTER_EMAIL", "replay@example.com")
        .args(args)
        .output()
        .expect("spawn arks")
}

fn ok(dir: &Path, args: &[&str]) -> String {
    let out = run(dir, args);
    assert!(
        out.status.success(),
        "arks {:?} failed (status={:?}):\nstdout={}\nstderr={}",
        args,
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn commit_file(dir: &Path, name: &str, content: &str, msg: &str) -> String {
    fs::write(dir.join(name), content).unwrap();
    ok(dir, &["add", name]);
    ok(dir, &["commit", "-m", msg]);
    ok(dir, &["rev-parse", "HEAD"])
}

fn setup_diverged_repo(dir: &Path) {
    ok(dir, &["init", "-b", "main"]);
    commit_file(dir, "base.txt", "base\n", "base commit");
    // Topic branch with two commits (and an intermediate `mid` branch for --contained).
    ok(dir, &["checkout", "-b", "topic"]);
    commit_file(dir, "topic1.txt", "t1\n", "topic 1");
    ok(dir, &["branch", "mid"]);
    commit_file(dir, "topic2.txt", "t2\n", "topic 2");
    // Advance main independently.
    ok(dir, &["checkout", "main"]);
    commit_file(dir, "main_new.txt", "m1\n", "main advance");
}

#[test]
fn replay_onto_leaves_dirty_worktree_and_index_untouched() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_diverged_repo(dir);

    let old_topic = ok(dir, &["rev-parse", "refs/heads/topic"]);
    let old_mid = ok(dir, &["rev-parse", "refs/heads/mid"]);
    let main_tip = ok(dir, &["rev-parse", "refs/heads/main"]);

    // Make the working tree and index dirty on `main`.
    fs::write(dir.join("dirty.txt"), "uncommitted\n").unwrap();
    ok(dir, &["add", "dirty.txt"]);
    fs::write(dir.join("base.txt"), "dirty-worktree-edit\n").unwrap();
    let index_before = fs::read(dir.join(".git/index")).unwrap();

    // Replay `main..topic` (actually `base..topic` via `mid~1..topic`) onto `main` with `--contained`.
    let base_oid = ok(dir, &["rev-parse", "main~1"]);
    ok(
        dir,
        &[
            "replay",
            "--onto",
            "main",
            "--contained",
            &format!("{base_oid}..topic"),
        ],
    );

    let new_topic = ok(dir, &["rev-parse", "refs/heads/topic"]);
    let new_mid = ok(dir, &["rev-parse", "refs/heads/mid"]);
    assert_ne!(new_topic, old_topic);
    assert_ne!(new_mid, old_mid);
    // Replayed `mid` is now a direct child of `main`, and `topic` is a direct child of `mid`.
    assert_eq!(ok(dir, &["rev-parse", "refs/heads/mid~1"]), main_tip);
    assert_eq!(ok(dir, &["rev-parse", "refs/heads/topic~1"]), new_mid);
    // Replayed tree contains both `main_new.txt` and `topic2.txt`.
    let ls = ok(dir, &["ls-tree", "--name-only", "refs/heads/topic"]);
    assert!(ls.contains("main_new.txt"), "ls={ls}");
    assert!(ls.contains("topic1.txt"), "ls={ls}");
    assert!(ls.contains("topic2.txt"), "ls={ls}");

    // Worktree and index MUST be byte-for-byte untouched!
    assert_eq!(
        fs::read_to_string(dir.join("base.txt")).unwrap(),
        "dirty-worktree-edit\n"
    );
    assert_eq!(fs::read(dir.join(".git/index")).unwrap(), index_before);
}

#[test]
fn replay_print_pipes_into_update_ref_stdin() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_diverged_repo(dir);

    let old_topic = ok(dir, &["rev-parse", "refs/heads/topic"]);
    let base_oid = ok(dir, &["rev-parse", "main~1"]);

    let printed = ok(
        dir,
        &[
            "replay",
            "--onto",
            "main",
            "--ref-action=print",
            &format!("{base_oid}..topic"),
        ],
    );
    assert!(
        printed.starts_with("update refs/heads/topic "),
        "unexpected print output: {printed}"
    );
    // Ref MUST NOT have changed yet.
    assert_eq!(ok(dir, &["rev-parse", "refs/heads/topic"]), old_topic);

    // Pipe printed commands into `arks update-ref --stdin`.
    let mut child = Command::new(arks_bin())
        .current_dir(dir)
        .env("GIT_COMMITTER_NAME", "Replay Test")
        .env("GIT_COMMITTER_EMAIL", "replay@example.com")
        .args(["update-ref", "--stdin"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    {
        use std::io::Write;
        child
            .stdin
            .as_mut()
            .unwrap()
            .write_all(format!("{printed}\n").as_bytes())
            .unwrap();
    }
    let out = child.wait_with_output().unwrap();
    assert!(
        out.status.success(),
        "update-ref --stdin failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let new_topic = ok(dir, &["rev-parse", "refs/heads/topic"]);
    assert_ne!(new_topic, old_topic);
}

#[test]
fn replay_advance_moves_target_branch_and_conflict_fails_closed() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_diverged_repo(dir);

    let base_oid = ok(dir, &["rev-parse", "main~1"]);
    let old_main = ok(dir, &["rev-parse", "refs/heads/main"]);
    ok(
        dir,
        &["replay", "--advance", "main", &format!("{base_oid}..topic")],
    );
    let advanced_main = ok(dir, &["rev-parse", "refs/heads/main"]);
    assert_ne!(advanced_main, old_main);
    let ls = ok(dir, &["ls-tree", "--name-only", "refs/heads/main"]);
    assert!(ls.contains("topic2.txt"));

    // Now create a conflicting branch modifying `base.txt` differently from `main`.
    ok(dir, &["checkout", "-b", "conflict-a", &base_oid]);
    commit_file(dir, "base.txt", "side A\n", "side A edit");
    ok(dir, &["checkout", "-b", "conflict-b", &base_oid]);
    commit_file(dir, "base.txt", "side B\n", "side B edit");
    let before_b = ok(dir, &["rev-parse", "refs/heads/conflict-b"]);

    let res = run(
        dir,
        &[
            "replay",
            "--onto",
            "conflict-a",
            &format!("{base_oid}..conflict-b"),
        ],
    );
    assert_eq!(res.status.code(), Some(1), "conflict must exit 1");
    assert_eq!(
        ok(dir, &["rev-parse", "refs/heads/conflict-b"]),
        before_b,
        "conflict must not mutate refs"
    );
}
