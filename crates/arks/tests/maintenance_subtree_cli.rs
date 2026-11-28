//! Integration tests for `arks maintenance` and `arks subtree` (E4).

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn arks_bin() -> &'static str {
    env!("CARGO_BIN_EXE_arks")
}

fn run(dir: &Path, args: &[&str]) -> Output {
    Command::new(arks_bin())
        .current_dir(dir)
        .env("GIT_AUTHOR_NAME", "E4 Test")
        .env("GIT_AUTHOR_EMAIL", "e4test@example.com")
        .env("GIT_COMMITTER_NAME", "E4 Test")
        .env("GIT_COMMITTER_EMAIL", "e4test@example.com")
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

fn commit_file(dir: &Path, rel: &str, content: &str, msg: &str) -> String {
    let p = dir.join(rel);
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(&p, content).unwrap();
    ok(dir, &["add", rel]);
    ok(dir, &["commit", "-m", msg]);
    ok(dir, &["rev-parse", "HEAD"])
}

#[test]
fn maintenance_register_run_start_stop_and_lock_refuse() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("repo");
    fs::create_dir_all(&dir).unwrap();
    let cfg_file = tmp.path().join("global-maint.gitconfig");

    ok(&dir, &["init", "-b", "main"]);
    commit_file(&dir, "README.md", "hello\n", "init");
    commit_file(&dir, "a.txt", "a\n", "add a");

    // Register & verify entry in explicit config file.
    ok(
        &dir,
        &[
            "maintenance",
            "register",
            &format!("--config-file={}", cfg_file.display()),
        ],
    );
    let cfg_body = fs::read_to_string(&cfg_file).unwrap();
    assert!(
        cfg_body.contains("[maintenance]") && cfg_body.contains("repo = "),
        "missing maintenance.repo in config:\n{cfg_body}"
    );

    // Run targeted maintenance tasks (commit-graph, pack-refs, incremental-repack).
    ok(
        &dir,
        &[
            "maintenance",
            "run",
            "--task=commit-graph",
            "--task=pack-refs",
            "--task=incremental-repack",
        ],
    );
    assert!(
        dir.join(".git/objects/info/commit-graph").is_file(),
        "commit-graph must be written"
    );
    assert!(
        dir.join(".git/packed-refs").is_file(),
        "packed-refs must be written"
    );
    assert!(
        !dir.join(".git/maintenance.lock").exists(),
        "maintenance.lock must be cleaned up after run"
    );

    // MAINT-005: existing maintenance.lock must cause fail-closed refusal.
    fs::write(dir.join(".git/maintenance.lock"), "held\n").unwrap();
    let locked = run(&dir, &["maintenance", "run", "--task=pack-refs"]);
    assert!(
        !locked.status.success(),
        "must refuse when maintenance.lock exists"
    );
    let stderr = String::from_utf8_lossy(&locked.stderr);
    assert!(stderr.contains("MAINT-005"), "stderr={stderr}");
    fs::remove_file(dir.join(".git/maintenance.lock")).unwrap();

    // Unregister removes the repo entry.
    ok(
        &dir,
        &[
            "maintenance",
            "unregister",
            &format!("--config-file={}", cfg_file.display()),
        ],
    );
    let cfg_after = fs::read_to_string(&cfg_file).unwrap();
    assert!(
        !cfg_after.contains("repo = "),
        "repo entry should be removed:\n{cfg_after}"
    );

    // start / stop write and clear maintenance.auto in local repo config.
    ok(&dir, &["maintenance", "start", "--scheduler=crontab"]);
    let local_cfg = fs::read_to_string(dir.join(".git/config")).unwrap();
    assert!(
        local_cfg.contains("auto = false") || local_cfg.contains("strategy = incremental"),
        "local config after start:\n{local_cfg}"
    );
    ok(&dir, &["maintenance", "stop"]);
}

#[test]
fn subtree_add_merge_and_split_roundtrip() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();

    ok(dir, &["init", "-b", "main"]);
    commit_file(dir, "app.txt", "main app v1\n", "main init");

    // Create an orphan branch `lib` representing an independent library repo.
    ok(dir, &["checkout", "--orphan", "lib"]);
    ok(dir, &["rm", "-f", "app.txt"]);
    let lib_v1 = commit_file(dir, "lib.rs", "pub fn v() -> u32 { 1 }\n", "lib v1");
    let lib_v2 = commit_file(dir, "lib.rs", "pub fn v() -> u32 { 2 }\n", "lib v2");

    // Return to `main` and add `lib_v1` under `vendor/lib` with --squash.
    ok(dir, &["checkout", "main"]);
    ok(
        dir,
        &["subtree", "add", "-P", "vendor/lib", "--squash", &lib_v1],
    );
    assert_eq!(
        fs::read_to_string(dir.join("vendor/lib/lib.rs")).unwrap(),
        "pub fn v() -> u32 { 1 }\n"
    );
    assert_eq!(
        fs::read_to_string(dir.join("app.txt")).unwrap(),
        "main app v1\n"
    );
    let head_msg = ok(dir, &["log", "-1", "--format=%B"]);
    assert!(
        head_msg.contains("git-subtree-dir: vendor/lib"),
        "msg={head_msg}"
    );
    assert!(
        head_msg.contains(&format!("git-subtree-split: {lib_v1}")),
        "msg={head_msg}"
    );

    // Make an unrelated change on `main` and a local change inside `vendor/lib`.
    commit_file(dir, "app.txt", "main app v2\n", "unrelated main edit");
    commit_file(
        dir,
        "vendor/lib/local.txt",
        "local addition\n",
        "local vendor edit",
    );

    // Split `vendor/lib` into branch `split-lib`.
    let split_oid = ok(
        dir,
        &["subtree", "split", "-P", "vendor/lib", "-b", "split-lib"],
    );
    assert_eq!(ok(dir, &["rev-parse", "refs/heads/split-lib"]), split_oid);
    let split_tree = ok(dir, &["ls-tree", "--name-only", "split-lib"]);
    assert!(split_tree.contains("lib.rs"), "split_tree={split_tree}");
    assert!(split_tree.contains("local.txt"), "split_tree={split_tree}");
    assert!(
        !split_tree.contains("app.txt"),
        "split must not contain outside files: {split_tree}"
    );

    // Merge upstream `lib_v2` into `vendor/lib` and verify outside files + local addition stay intact.
    ok(dir, &["subtree", "merge", "-P", "vendor/lib", &lib_v2]);
    assert_eq!(
        fs::read_to_string(dir.join("vendor/lib/lib.rs")).unwrap(),
        "pub fn v() -> u32 { 2 }\n"
    );
    assert_eq!(
        fs::read_to_string(dir.join("vendor/lib/local.txt")).unwrap(),
        "local addition\n"
    );
    assert_eq!(
        fs::read_to_string(dir.join("app.txt")).unwrap(),
        "main app v2\n"
    );
}
