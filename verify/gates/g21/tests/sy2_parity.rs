//! Gate G21 — M16 Sy-2 worktree / submodule recursive / orphan claim rows.
//! Matrix: `verify/matrices/matrix-g21.md`. Cite: git-worktree / git-submodule / git-checkout 2.39.0.

use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use tempfile::TempDir;

static RECEIPT: Mutex<Vec<(&'static str, bool)>> = Mutex::new(Vec::new());

fn record_row(id: &'static str, pass: bool) {
    RECEIPT.lock().unwrap().push((id, pass));
}

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
fn m16_sy2_01_worktree_porcelain_lock() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("r");
    init_repo(&repo);
    seed(&repo, "a.txt", "one\n", "first");

    let wt = tmp.path().join("linked");
    let wt_s = wt.to_str().unwrap();
    assert_ok(
        &arks(&repo, &["worktree", "add", "-b", "feature", wt_s]),
        "worktree add -b",
    );

    let porcelain = arks(&repo, &["worktree", "list", "--porcelain"]);
    assert_ok(&porcelain, "worktree list --porcelain");
    let s = String::from_utf8_lossy(&porcelain.stdout);
    assert!(s.contains("worktree "), "missing worktree attr: {s}");
    assert!(s.contains("HEAD "), "missing HEAD attr: {s}");
    assert!(
        s.contains("branch refs/heads/") || s.contains("detached"),
        "missing branch/detached: {s}"
    );

    assert_ok(
        &arks(
            &repo,
            &["worktree", "lock", "--reason", "portable mount", wt_s],
        ),
        "worktree lock",
    );
    let locked = arks(&repo, &["worktree", "list", "--porcelain"]);
    assert_ok(&locked, "list after lock");
    let ls = String::from_utf8_lossy(&locked.stdout);
    assert!(ls.contains("locked"), "expected locked attr: {ls}");

    // Locked remove must fail-closed (lease ≠ skip locks / admin lock honor).
    let rm = arks(&repo, &["worktree", "remove", wt_s]);
    assert!(!rm.status.success(), "locked remove must fail");

    assert_ok(&arks(&repo, &["worktree", "unlock", wt_s]), "unlock");
    let unlocked = arks(&repo, &["worktree", "list", "--porcelain"]);
    assert_ok(&unlocked, "list after unlock");
    let us = String::from_utf8_lossy(&unlocked.stdout);
    assert!(!us.contains("locked"), "lock should be cleared: {us}");

    // add --detach claimed
    let det = tmp.path().join("detached-wt");
    assert_ok(
        &arks(
            &repo,
            &["worktree", "add", "--detach", det.to_str().unwrap()],
        ),
        "worktree add --detach",
    );
    let det_list = arks(&repo, &["worktree", "list", "--porcelain"]);
    assert_ok(&det_list, "list detach");
    let ds = String::from_utf8_lossy(&det_list.stdout);
    assert!(ds.contains("detached"), "expected detached worktree: {ds}");

    record_row("M16-SY2-01", true);
}

#[test]
fn m16_sy2_02_submodule_recursive() {
    let tmp = TempDir::new().unwrap();

    // Nested leaf submodule source.
    let leaf_src = tmp.path().join("leaf-src");
    init_repo(&leaf_src);
    seed(&leaf_src, "leaf.txt", "leaf\n", "leaf");

    // Mid repo that contains the leaf as a submodule.
    let mid_src = tmp.path().join("mid-src");
    init_repo(&mid_src);
    seed(&mid_src, "mid.txt", "mid\n", "mid");
    assert_ok(
        &arks(
            &mid_src,
            &["submodule", "add", leaf_src.to_str().unwrap(), "deps/leaf"],
        ),
        "mid submodule add leaf",
    );
    assert_ok(&arks(&mid_src, &["commit", "-m", "add leaf"]), "mid commit");

    // Superproject with mid as submodule.
    let super_repo = tmp.path().join("super");
    init_repo(&super_repo);
    seed(&super_repo, "root.txt", "root\n", "root");
    assert_ok(
        &arks(
            &super_repo,
            &["submodule", "add", mid_src.to_str().unwrap(), "deps/mid"],
        ),
        "super submodule add mid",
    );
    assert_ok(
        &arks(&super_repo, &["commit", "-m", "add mid"]),
        "super commit",
    );

    // Deinit mid worktree contents then update --init --recursive.
    assert_ok(
        &arks(&super_repo, &["submodule", "deinit", "-f", "deps/mid"]),
        "deinit mid",
    );
    assert_ok(
        &arks(
            &super_repo,
            &["submodule", "update", "--init", "--recursive"],
        ),
        "update --init --recursive",
    );
    assert!(
        super_repo.join("deps/mid/deps/leaf/leaf.txt").is_file(),
        "nested leaf checkout missing"
    );

    let status = arks(&super_repo, &["submodule", "status", "--recursive"]);
    assert_ok(&status, "status --recursive");
    let st = String::from_utf8_lossy(&status.stdout);
    assert!(st.contains("deps/mid"), "mid missing: {st}");
    assert!(
        st.contains("deps/mid/deps/leaf") || st.contains("deps/leaf"),
        "nested leaf status missing: {st}"
    );

    // update --remote when tracking branch configured in .gitmodules.
    // Advance leaf-src tip, set branch=main on mid→leaf tracking, then remote update.
    fs::write(leaf_src.join("leaf.txt"), "leaf2\n").unwrap();
    assert_ok(&arks(&leaf_src, &["add", "leaf.txt"]), "leaf add2");
    assert_ok(&arks(&leaf_src, &["commit", "-m", "leaf2"]), "leaf commit2");
    let mid_abs = super_repo.join("deps/mid");
    let gm = fs::read_to_string(mid_abs.join(".gitmodules")).unwrap();
    let gm2 = if gm.contains("branch") {
        gm
    } else if gm.contains("[submodule \"deps/leaf\"]") {
        gm.replace(
            "[submodule \"deps/leaf\"]",
            "[submodule \"deps/leaf\"]\n\tbranch = main",
        )
    } else {
        // Fallback: append a branch key under whatever leaf section exists.
        let mut out = String::new();
        for line in gm.lines() {
            out.push_str(line);
            out.push('\n');
            let t = line.trim();
            if t.starts_with("[submodule ") && t.contains("leaf") {
                out.push_str("\tbranch = main\n");
            }
        }
        out
    };
    fs::write(mid_abs.join(".gitmodules"), gm2).unwrap();

    let remote_upd = arks(&mid_abs, &["submodule", "update", "--remote"]);
    assert_ok(&remote_upd, "update --remote");
    let leaf_body = fs::read_to_string(mid_abs.join("deps/leaf/leaf.txt")).unwrap();
    assert!(
        leaf_body.contains("leaf2"),
        "expected remote tip checked out, got {leaf_body}"
    );

    // Stock Git can still read the superproject object graph.
    assert_ok(&git(&super_repo, &["rev-parse", "HEAD"]), "git peer rev-parse");

    record_row("M16-SY2-02", true);
}

#[test]
fn m16_sy2_03_checkout_orphan() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("r");
    init_repo(&repo);
    seed(&repo, "a.txt", "one\n", "first");

    assert_ok(
        &arks(&repo, &["checkout", "--orphan", "orphan-branch"]),
        "checkout --orphan",
    );
    let head = fs::read_to_string(repo.join(".git/HEAD")).unwrap();
    assert!(
        head.trim() == "ref: refs/heads/orphan-branch",
        "HEAD should be unborn symbolic: {head}"
    );
    assert!(
        !repo.join(".git/refs/heads/orphan-branch").is_file(),
        "orphan branch tip must not exist yet"
    );
    // Index retains start-point paths (git-checkout --orphan).
    let ls = arks(&repo, &["ls-files"]);
    assert_ok(&ls, "ls-files");
    let files = String::from_utf8_lossy(&ls.stdout);
    assert!(files.contains("a.txt"), "index should retain a.txt: {files}");

    assert_ok(
        &arks(&repo, &["commit", "-m", "orphan root"]),
        "orphan first commit",
    );
    assert!(repo.join(".git/refs/heads/orphan-branch").is_file());
    // First commit has no parents — stock Git peer check.
    let parents = git(&repo, &["rev-list", "--parents", "-n", "1", "HEAD"]);
    assert_ok(&parents, "git rev-list --parents");
    let line = String::from_utf8_lossy(&parents.stdout);
    let parts: Vec<_> = line.split_whitespace().collect();
    assert_eq!(
        parts.len(),
        1,
        "orphan commit must have zero parents: {line}"
    );

    record_row("M16-SY2-03", true);
}

#[test]
fn m16_sy2_receipt_write() {
    let rows = RECEIPT.lock().unwrap().clone();
    if let Ok(path) = std::env::var("ARKS_G21_RECEIPT_OUT") {
        let body = json!({
            "gate": "G21",
            "wave": "Sy-2",
            "rows": rows.iter().map(|(id, pass)| json!({"id": id, "pass": pass})).collect::<Vec<_>>(),
        });
        if let Some(parent) = Path::new(&path).parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(&path, serde_json::to_vec_pretty(&body).unwrap());
    }
    let _ = rows;
}
