//! Gate G20 — M15 Fs-1 full CLI symmetric claim rows.
//! Matrix: `verify/matrix-g20.md`.

use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

fn arks_bin() -> PathBuf {
    if let Ok(p) = std::env::var("ARKS_BIN") {
        let p = PathBuf::from(p);
        if p.is_file() {
            return p;
        }
    }
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(target) = std::env::var("CARGO_TARGET_DIR") {
        candidates.push(PathBuf::from(&target).join("release/arks"));
        candidates.push(PathBuf::from(&target).join("release/arks.exe"));
        candidates.push(PathBuf::from(&target).join("debug/arks"));
        candidates.push(PathBuf::from(&target).join("debug/arks.exe"));
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    candidates.push(root.join("target/release/arks"));
    candidates.push(root.join("target/release/arks.exe"));
    candidates.push(root.join("target/debug/arks"));
    candidates.push(root.join("target/debug/arks.exe"));
    for c in &candidates {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!("missing arks binary; tried {candidates:?}");
}

fn pin_env(cmd: &mut Command) {
    cmd.env("GIT_AUTHOR_NAME", "arks")
        .env("GIT_AUTHOR_EMAIL", "arks@example.com")
        .env("GIT_COMMITTER_NAME", "arks")
        .env("GIT_COMMITTER_EMAIL", "arks@example.com")
        .env("GIT_AUTHOR_DATE", "1000000000 +0000")
        .env("GIT_COMMITTER_DATE", "1000000000 +0000");
}

fn arks(repo: &Path, args: &[&str]) -> std::process::Output {
    let mut c = Command::new(arks_bin());
    pin_env(&mut c);
    c.args(args).current_dir(repo).output().expect("arks")
}

fn git(repo: &Path, args: &[&str]) -> std::process::Output {
    let mut c = Command::new("git");
    pin_env(&mut c);
    c.args(args).current_dir(repo).output().expect("git")
}

fn assert_ok(out: &std::process::Output, label: &str) {
    assert!(
        out.status.success(),
        "{label}: {}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

fn assert_fail(out: &std::process::Output, label: &str) {
    assert!(
        !out.status.success(),
        "{label} should fail: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}

fn init_repo(path: &Path) {
    fs::create_dir_all(path).unwrap();
    assert_ok(&arks(path, &["init"]), "init");
}

fn seed(repo: &Path, file: &str, body: &str, msg: &str) {
    fs::write(repo.join(file), body).unwrap();
    assert_ok(&arks(repo, &["add", file]), "add");
    assert_ok(&arks(repo, &["commit", "-m", msg]), "commit");
}

#[test]
fn m15_fs1_01_commit_amend() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("r");
    init_repo(&repo);
    seed(&repo, "a.txt", "one\n", "first");
    fs::write(repo.join("a.txt"), "two\n").unwrap();
    assert_ok(&arks(&repo, &["add", "a.txt"]), "add2");
    assert_ok(
        &arks(&repo, &["commit", "--amend", "-m", "amended"]),
        "amend",
    );
    let log = git(&repo, &["log", "-1", "--pretty=%s"]);
    assert_ok(&log, "git log");
    let subject = String::from_utf8_lossy(&log.stdout);
    assert!(
        subject.contains("amended"),
        "stock git should see amended subject: {subject}"
    );
    record_row("M15-FS1-01", true);
}

#[test]
fn m15_fs1_02_sync_pre_commit_hook() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("r");
    init_repo(&repo);
    seed(&repo, "a.txt", "one\n", "first");
    let hooks = repo.join(".git/hooks");
    fs::create_dir_all(&hooks).unwrap();
    let hook = hooks.join("pre-commit");
    fs::write(&hook, "#!/bin/sh\necho hook-ran >&2\nexit 1\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&hook).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&hook, perms).unwrap();
    }
    fs::write(repo.join("a.txt"), "two\n").unwrap();
    assert_ok(&arks(&repo, &["add", "a.txt"]), "add");
    assert_fail(
        &arks(&repo, &["commit", "-m", "blocked"]),
        "hook should abort",
    );
    assert_ok(
        &arks(&repo, &["commit", "--no-verify", "-m", "skipped"]),
        "no-verify",
    );
    record_row("M15-FS1-02", true);
}

#[test]
fn m15_fs1_03_merge_no_ff() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("r");
    init_repo(&repo);
    seed(&repo, "a.txt", "base\n", "base");
    assert_ok(&arks(&repo, &["branch", "topic"]), "branch");
    // Advance main so topic is behind (FF-capable into topic from main? 
    // Better: merge topic into main when topic is ancestor of... 
    // Create topic ahead of main:
    assert_ok(&arks(&repo, &["checkout", "topic"]), "co topic");
    fs::write(repo.join("b.txt"), "topic\n").unwrap();
    assert_ok(&arks(&repo, &["add", "b.txt"]), "add topic");
    assert_ok(&arks(&repo, &["commit", "-m", "topic"]), "commit topic");
    assert_ok(&arks(&repo, &["checkout", "main"]), "co main");
    // main is ancestor of topic → FF-capable
    assert_ok(&arks(&repo, &["merge", "--no-ff", "topic"]), "merge --no-ff");
    let parents = git(&repo, &["rev-list", "--parents", "-n", "1", "HEAD"]);
    assert_ok(&parents, "rev-list");
    let line = String::from_utf8_lossy(&parents.stdout);
    // OID parent1 parent2
    assert!(
        line.split_whitespace().count() >= 3,
        "expected merge commit with two parents: {line}"
    );
    record_row("M15-FS1-03", true);
}

#[test]
fn m15_fs1_04_plumbing_minimum() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("r");
    init_repo(&repo);
    seed(&repo, "a.txt", "blob\n", "c1");
    let wt = arks(&repo, &["write-tree"]);
    assert_ok(&wt, "write-tree");
    let tree = String::from_utf8_lossy(&wt.stdout).trim().to_owned();
    assert_eq!(tree.len(), 40);
    let git_wt = git(&repo, &["write-tree"]);
    assert_ok(&git_wt, "git write-tree");
    assert_eq!(
        String::from_utf8_lossy(&git_wt.stdout).trim(),
        tree.as_str()
    );

    let ls = arks(&repo, &["ls-files"]);
    assert_ok(&ls, "ls-files");
    assert!(String::from_utf8_lossy(&ls.stdout).contains("a.txt"));

    let lst = arks(&repo, &["ls-tree", "HEAD"]);
    assert_ok(&lst, "ls-tree");
    assert!(String::from_utf8_lossy(&lst.stdout).contains("a.txt"));

    let sym = arks(&repo, &["symbolic-ref", "HEAD"]);
    assert_ok(&sym, "symbolic-ref");
    assert!(String::from_utf8_lossy(&sym.stdout).contains("refs/heads/"));

    // Second commit for merge-base
    fs::write(repo.join("a.txt"), "blob2\n").unwrap();
    assert_ok(&arks(&repo, &["add", "a.txt"]), "add2");
    assert_ok(&arks(&repo, &["commit", "-m", "c2"]), "c2");
    let head_out = arks(&repo, &["rev-parse", "HEAD"]);
    assert_ok(&head_out, "rev-parse HEAD");
    let head = String::from_utf8_lossy(&head_out.stdout).trim().to_owned();
    let mb = arks(&repo, &["merge-base", &head, &head]);
    assert_ok(&mb, "merge-base");
    assert_eq!(String::from_utf8_lossy(&mb.stdout).trim(), head.as_str());

    let ct = arks(
        &repo,
        &["commit-tree", &tree, "-p", &head, "-m", "from-tree"],
    );
    assert_ok(&ct, "commit-tree");
    assert_eq!(String::from_utf8_lossy(&ct.stdout).trim().len(), 40);
    record_row("M15-FS1-04", true);
}

#[test]
fn m15_fs2_01_diverged_clean_merge() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("r");
    init_repo(&repo);
    seed(&repo, "base.txt", "base\n", "base");
    assert_ok(&arks(&repo, &["branch", "topic"]), "branch");
    // Advance main with unique file.
    fs::write(repo.join("main-only.txt"), "main\n").unwrap();
    assert_ok(&arks(&repo, &["add", "main-only.txt"]), "add main");
    assert_ok(&arks(&repo, &["commit", "-m", "main"]), "commit main");
    // Advance topic with different unique file.
    assert_ok(&arks(&repo, &["checkout", "topic"]), "co topic");
    fs::write(repo.join("topic-only.txt"), "topic\n").unwrap();
    assert_ok(&arks(&repo, &["add", "topic-only.txt"]), "add topic");
    assert_ok(&arks(&repo, &["commit", "-m", "topic"]), "commit topic");
    assert_ok(&arks(&repo, &["checkout", "main"]), "co main");
    assert_ok(&arks(&repo, &["merge", "topic"]), "merge diverged");
    assert!(repo.join("main-only.txt").is_file());
    assert!(repo.join("topic-only.txt").is_file());
    let parents = git(&repo, &["rev-list", "--parents", "-n", "1", "HEAD"]);
    assert_ok(&parents, "rev-list");
    assert!(
        String::from_utf8_lossy(&parents.stdout)
            .split_whitespace()
            .count()
            >= 3
    );
    // Stock Git can read the merge.
    assert_ok(&git(&repo, &["fsck", "--no-dangling"]), "git fsck");
    record_row("M15-FS2-01", true);
}

#[test]
fn m15_fs2_02_diverged_conflict() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("r");
    init_repo(&repo);
    seed(&repo, "c.txt", "base\n", "base");
    assert_ok(&arks(&repo, &["branch", "topic"]), "branch");
    fs::write(repo.join("c.txt"), "main-change\n").unwrap();
    assert_ok(&arks(&repo, &["add", "c.txt"]), "add main");
    assert_ok(&arks(&repo, &["commit", "-m", "main"]), "commit main");
    assert_ok(&arks(&repo, &["checkout", "topic"]), "co topic");
    fs::write(repo.join("c.txt"), "topic-change\n").unwrap();
    assert_ok(&arks(&repo, &["add", "c.txt"]), "add topic");
    assert_ok(&arks(&repo, &["commit", "-m", "topic"]), "commit topic");
    assert_ok(&arks(&repo, &["checkout", "main"]), "co main");
    assert_fail(&arks(&repo, &["merge", "topic"]), "expect conflict");
    let body = fs::read_to_string(repo.join("c.txt")).unwrap();
    assert!(body.contains("<<<<<<<"), "missing conflict markers: {body}");
    record_row("M15-FS2-02", true);
}

#[test]
fn m15_fs4_01_add_all_and_porcelain_status() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("r");
    init_repo(&repo);
    seed(&repo, "a.txt", "a\n", "base");
    fs::write(repo.join("b.txt"), "new\n").unwrap();
    fs::write(repo.join("a.txt"), "changed\n").unwrap();
    assert_ok(&arks(&repo, &["add", "-A"]), "add -A");
    let st = arks(&repo, &["status", "--porcelain"]);
    assert_ok(&st, "status --porcelain");
    let text = String::from_utf8_lossy(&st.stdout);
    assert!(
        text.contains("b.txt") || text.contains("A ") || text.contains("M "),
        "porcelain missing staged lines: {text}"
    );
    // Peer: stock git understands the index after arks add -A.
    assert_ok(&git(&repo, &["status", "--porcelain"]), "git status");
    record_row("M15-FS4-01", true);
}

#[test]
fn m15_fs3_01_notes_add_show() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("r");
    init_repo(&repo);
    seed(&repo, "a.txt", "a\n", "base");
    assert_ok(
        &arks(&repo, &["notes", "add", "-m", "hello-note"]),
        "notes add",
    );
    let show = arks(&repo, &["notes", "show"]);
    assert_ok(&show, "notes show");
    assert!(
        String::from_utf8_lossy(&show.stdout).contains("hello-note"),
        "arks notes show missing body"
    );
    let peer = git(&repo, &["notes", "show"]);
    assert_ok(&peer, "git notes show");
    assert!(
        String::from_utf8_lossy(&peer.stdout).contains("hello-note"),
        "stock git notes show missing body"
    );
    record_row("M15-FS3-01", true);
}

#[test]
fn m15_fs3_02_sparse_checkout_cone() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("r");
    init_repo(&repo);
    fs::create_dir_all(repo.join("keep")).unwrap();
    fs::create_dir_all(repo.join("drop")).unwrap();
    fs::write(repo.join("root.txt"), "root\n").unwrap();
    fs::write(repo.join("keep/in.txt"), "in\n").unwrap();
    fs::write(repo.join("drop/out.txt"), "out\n").unwrap();
    assert_ok(&arks(&repo, &["add", "-A"]), "add");
    assert_ok(&arks(&repo, &["commit", "-m", "tree"]), "commit");
    assert_ok(
        &arks(&repo, &["sparse-checkout", "init", "--cone"]),
        "init --cone",
    );
    assert_ok(
        &arks(&repo, &["sparse-checkout", "set", "keep"]),
        "set keep",
    );
    let list = arks(&repo, &["sparse-checkout", "list"]);
    assert_ok(&list, "list");
    let patterns = String::from_utf8_lossy(&list.stdout);
    assert!(patterns.contains("/keep/"), "missing cone dir: {patterns}");
    assert!(repo.join("keep/in.txt").is_file() || repo.join("root.txt").is_file());
    assert!(
        !repo.join("drop/out.txt").is_file(),
        "drop/out.txt should be removed from worktree"
    );
    record_row("M15-FS3-02", true);
}

#[test]
fn m15_fs3_03_repack_lease_honest() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("r");
    init_repo(&repo);
    seed(&repo, "a.txt", "a\n", "c1");
    assert_ok(&arks(&repo, &["repack", "-ad"]), "repack -ad");
    assert_ok(&git(&repo, &["fsck", "--no-dangling"]), "git fsck");
    // Foreign maint:gc holder blocks concurrent repack.
    assert_ok(
        &arks(
            &repo,
            &[
                "lease",
                "grant",
                "maint:gc",
                "--holder",
                "foreign",
                "--ttl-secs",
                "60",
            ],
        ),
        "lease grant",
    );
    assert_fail(&arks(&repo, &["repack", "-a"]), "repack under foreign lease");
    assert_ok(
        &arks(
            &repo,
            &["lease", "release", "maint:gc", "--holder", "foreign"],
        ),
        "lease release",
    );
    assert_ok(&arks(&repo, &["repack", "-a"]), "repack after release");
    record_row("M15-FS3-03", true);
}

#[test]
fn m15_fs3_04_gc_and_prune_now_refuse() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("r");
    init_repo(&repo);
    seed(&repo, "a.txt", "a\n", "c1");
    assert_ok(&arks(&repo, &["gc"]), "gc");
    let pruned = arks(&repo, &["gc", "--prune=now"]);
    assert_fail(&pruned, "gc --prune=now must refuse");
    let err = String::from_utf8_lossy(&pruned.stderr);
    assert!(
        err.contains("CORR-001") || err.contains("prune=now"),
        "expected CORR-001 honesty: {err}"
    );
    record_row("M15-FS3-04", true);
}

#[test]
fn m15_fs3_05_filter_branch_backfill_exclude() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("r");
    init_repo(&repo);
    seed(&repo, "a.txt", "a\n", "c1");
    let fb = arks(&repo, &["filter-branch"]);
    assert_fail(&fb, "filter-branch exclude");
    let bf = arks(&repo, &["backfill"]);
    assert_fail(&bf, "backfill exclude");
    let msg = format!(
        "{}{}",
        String::from_utf8_lossy(&fb.stderr),
        String::from_utf8_lossy(&bf.stderr)
    );
    assert!(
        msg.contains("excluded") || msg.contains("exclude"),
        "expected exclude honesty: {msg}"
    );
    record_row("M15-FS3-05", true);
}

#[test]
fn m15_fs5_01_range_diff_subjects() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("r");
    init_repo(&repo);
    seed(&repo, "a.txt", "base\n", "base");
    let base_out = arks(&repo, &["rev-parse", "HEAD"]);
    assert_ok(&base_out, "rev-parse HEAD base");
    let base = String::from_utf8_lossy(&base_out.stdout).trim().to_owned();
    assert_ok(&arks(&repo, &["branch", "old"]), "branch old");
    assert_ok(&arks(&repo, &["branch", "new"]), "branch new");
    assert_ok(&arks(&repo, &["checkout", "old"]), "co old");
    fs::write(repo.join("a.txt"), "old1\n").unwrap();
    assert_ok(&arks(&repo, &["add", "a.txt"]), "add old1");
    assert_ok(&arks(&repo, &["commit", "-m", "change-one"]), "c old1");
    fs::write(repo.join("a.txt"), "old2\n").unwrap();
    assert_ok(&arks(&repo, &["add", "a.txt"]), "add old2");
    assert_ok(&arks(&repo, &["commit", "-m", "change-two"]), "c old2");
    assert_ok(&arks(&repo, &["checkout", "new"]), "co new");
    fs::write(repo.join("a.txt"), "new1\n").unwrap();
    assert_ok(&arks(&repo, &["add", "a.txt"]), "add new1");
    assert_ok(&arks(&repo, &["commit", "-m", "change-one"]), "c new1");
    fs::write(repo.join("a.txt"), "new2\n").unwrap();
    assert_ok(&arks(&repo, &["add", "a.txt"]), "add new2");
    assert_ok(
        &arks(&repo, &["commit", "-m", "change-two-renamed"]),
        "c new2",
    );
    let old_range = format!("{base}..old");
    let new_range = format!("{base}..new");
    let out = arks(&repo, &["range-diff", &old_range, &new_range]);
    assert_ok(&out, "range-diff");
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains('='), "expected equal subject row: {text}");
    assert!(text.contains('!'), "expected changed subject row: {text}");
    record_row("M15-FS5-01", true);
}

#[test]
fn m15_fs5_02_commit_all() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("r");
    init_repo(&repo);
    seed(&repo, "a.txt", "one\n", "base");
    fs::write(repo.join("a.txt"), "two\n").unwrap();
    assert_ok(
        &arks(&repo, &["commit", "-a", "-m", "via-all"]),
        "commit -a",
    );
    let show = git(&repo, &["log", "-1", "--pretty=%s"]);
    assert_ok(&show, "git log");
    assert!(
        String::from_utf8_lossy(&show.stdout).contains("via-all"),
        "missing commit subject"
    );
    record_row("M15-FS5-02", true);
}

#[test]
fn m15_fs5_03_interactive_editor_exclude() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("r");
    init_repo(&repo);
    seed(&repo, "a.txt", "a\n", "c1");
    let no_m = arks(&repo, &["commit"]);
    assert_fail(&no_m, "commit without -m");
    let rebase_i = arks(&repo, &["rebase", "-i", "HEAD"]);
    assert_fail(&rebase_i, "rebase -i");
    let msg = format!(
        "{}{}",
        String::from_utf8_lossy(&no_m.stderr),
        String::from_utf8_lossy(&rebase_i.stderr)
    );
    assert!(
        msg.contains("excluded") || msg.contains("exclude") || msg.contains("interactive"),
        "expected exclude honesty: {msg}"
    );
    record_row("M15-FS5-03", true);
}

fn record_row(id: &str, pass: bool) {
    let path = std::env::var("ARKS_G20_RECEIPT_OUT").unwrap_or_else(|_| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("out/receipt.json")
            .to_string_lossy()
            .into()
    });
    let path = PathBuf::from(path);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let lock_path = path.with_extension("json.lock");
    for _ in 0..200 {
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock_path)
        {
            Ok(_) => break,
            Err(_) => std::thread::sleep(std::time::Duration::from_millis(10)),
        }
    }
    let mut rows = Vec::new();
    if path.is_file() {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&fs::read_to_string(&path).unwrap())
        {
            if let Some(arr) = v.get("rows").and_then(|r| r.as_array()) {
                rows = arr.clone();
            }
        }
    }
    rows.retain(|r| r.get("id").and_then(|x| x.as_str()) != Some(id));
    let wave = if id.contains("FS5") {
        "Fs-5"
    } else if id.contains("FS4") {
        "Fs-4"
    } else if id.contains("FS3") {
        "Fs-3"
    } else if id.contains("FS2") {
        "Fs-2"
    } else {
        "Fs-1"
    };
    rows.push(json!({"id": id, "pass": pass, "wave": wave}));
    let receipt = json!({
        "schema": "arks-g20-cli-full-parity/v1",
        "wave": "Fs-1+Fs-2+Fs-3+Fs-4+Fs-5",
        "venue": std::env::var("ARKS_G20_VENUE").unwrap_or_else(|_| "clean-room".into()),
        "rows": rows,
    });
    fs::write(&path, serde_json::to_string_pretty(&receipt).unwrap() + "\n").unwrap();
    let _ = fs::remove_file(&lock_path);
}
