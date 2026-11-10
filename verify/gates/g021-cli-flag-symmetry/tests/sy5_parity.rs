//! Gate G21 — M16 Sy-5 remaining depth + ancillary interrogators.
//! Matrix: `verify/matrices/matrix-g021-cli-flag-symmetry.md`. Cite: git-stash / git-cherry-pick / git-bisect /
//! git-notes / git-blame / git-fsck / git-count-objects / git-pack-refs / git-commit 2.39.0.

use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use tempfile::TempDir;

static RECEIPT: Mutex<Vec<(&'static str, bool)>> = Mutex::new(Vec::new());

fn record_row(id: &'static str, pass: bool) {
    let mut rows = RECEIPT.lock().unwrap();
    rows.push((id, pass));
    if let Ok(path) = std::env::var("ARKS_G21_RECEIPT_OUT") {
        let body = json!({
            "gate": "G21",
            "wave": "Sy-5",
            "pass": rows.iter().all(|(_, p)| *p),
            "compose_service": "g021-cli-flag-symmetry",
            "rows": rows.iter().map(|(i, p)| json!({"id": i, "pass": p})).collect::<Vec<_>>(),
        });
        if let Some(parent) = Path::new(&path).parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(&path, serde_json::to_string_pretty(&body).unwrap());
    }
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
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
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

fn assert_ok(out: &std::process::Output, label: &str) {
    assert!(
        out.status.success(),
        "{label}: status={:?}\nstdout:\n{}\nstderr:\n{}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

fn assert_fail(out: &std::process::Output, label: &str) {
    assert!(
        !out.status.success(),
        "{label}: expected non-zero\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

fn seed_repo(repo: &Path) {
    fs::create_dir_all(repo).unwrap();
    assert_ok(&arks(repo, &["init"]), "init");
    fs::write(repo.join("a.txt"), "one\n").unwrap();
    assert_ok(&arks(repo, &["add", "a.txt"]), "add");
    assert_ok(&arks(repo, &["commit", "-m", "c1"]), "commit c1");
}

#[test]
fn m16_sy5_01_stash_and_cherry_pick_range() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("r");
    seed_repo(&repo);

    // stash show / apply / drop
    fs::write(repo.join("a.txt"), "dirty\n").unwrap();
    assert_ok(&arks(&repo, &["stash", "push"]), "stash push");
    let show = arks(&repo, &["stash", "show"]);
    assert_ok(&show, "stash show");
    assert!(
        String::from_utf8_lossy(&show.stdout).contains("stash@{0}"),
        "show output"
    );
    assert_ok(&arks(&repo, &["stash", "apply"]), "stash apply");
    assert_ok(&arks(&repo, &["stash", "drop"]), "stash drop");

    // cherry-pick A..B clean range on a side branch
    fs::write(repo.join("a.txt"), "base\n").unwrap();
    assert_ok(&arks(&repo, &["add", "a.txt"]), "add base");
    assert_ok(&arks(&repo, &["commit", "-m", "base"]), "commit base");
    let base = String::from_utf8_lossy(&arks(&repo, &["rev-parse", "HEAD"]).stdout)
        .trim()
        .to_string();
    assert_ok(&arks(&repo, &["checkout", "-b", "topic"]), "branch topic");
    fs::write(repo.join("b.txt"), "b1\n").unwrap();
    assert_ok(&arks(&repo, &["add", "b.txt"]), "add b1");
    assert_ok(&arks(&repo, &["commit", "-m", "t1"]), "commit t1");
    fs::write(repo.join("c.txt"), "c1\n").unwrap();
    assert_ok(&arks(&repo, &["add", "c.txt"]), "add c1");
    assert_ok(&arks(&repo, &["commit", "-m", "t2"]), "commit t2");
    let tip = String::from_utf8_lossy(&arks(&repo, &["rev-parse", "HEAD"]).stdout)
        .trim()
        .to_string();
    assert_ok(&arks(&repo, &["checkout", "main"]), "checkout main");
    assert_ok(&arks(&repo, &["reset", "--hard", &base]), "reset to base");
    let range = format!("{base}..{tip}");
    assert_ok(
        &arks(&repo, &["cherry-pick", &range]),
        "cherry-pick range",
    );
    assert!(repo.join("b.txt").is_file(), "b.txt present after range");
    assert!(repo.join("c.txt").is_file(), "c.txt present after range");

    record_row("M16-SY5-01", true);
}

#[test]
fn m16_sy5_02_bisect_notes_blame_fsck_count_packrefs() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("r");
    seed_repo(&repo);

    // Linear history for bisect: good at c1, bad at tip.
    for i in 2..=5 {
        fs::write(repo.join("a.txt"), format!("v{i}\n")).unwrap();
        assert_ok(&arks(&repo, &["add", "a.txt"]), &format!("add v{i}"));
        assert_ok(
            &arks(&repo, &["commit", "-m", &format!("c{i}")]),
            &format!("commit c{i}"),
        );
    }
    let good = String::from_utf8_lossy(
        &arks(&repo, &["rev-parse", "HEAD~4"]).stdout,
    )
    .trim()
    .to_string();
    let bad = String::from_utf8_lossy(&arks(&repo, &["rev-parse", "HEAD"]).stdout)
        .trim()
        .to_string();
    assert_ok(
        &arks(&repo, &["bisect", "start", &bad, &good]),
        "bisect start",
    );
    // Script: treat content containing v5 as bad, else good — written as a tiny shell/cmd.
    // Use arks-compatible portable check via rustc-less: compare file.
    #[cfg(unix)]
    {
        // Keep the probe script *outside* the worktree — an in-repo `check.sh`
        // is untracked dirty and trips BISECT-002 on the first mark (honest).
        let script = tmp.path().join("check.sh");
        fs::write(
            &script,
            "#!/bin/sh\ngrep -q v5 a.txt && exit 1 || exit 0\n",
        )
        .unwrap();
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&script).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script, perms).unwrap();
        assert_ok(
            &arks(&repo, &["bisect", "run", script.to_str().unwrap()]),
            "bisect run",
        );
    }
    #[cfg(windows)]
    {
        // On Windows host advisory runs, approximate with explicit good/bad marks
        // then a trivial `cmd /C exit 1` once mid-bisect — Docker G21 is Linux.
        let mid = arks(&repo, &["rev-parse", "HEAD"]);
        assert_ok(&mid, "bisect mid");
        assert_ok(&arks(&repo, &["bisect", "bad"]), "bisect bad mid");
        let _ = arks(&repo, &["bisect", "reset"]);
    }
    let _ = arks(&repo, &["bisect", "reset"]);

    // notes append
    assert_ok(
        &arks(&repo, &["notes", "add", "-m", "first"]),
        "notes add",
    );
    assert_ok(
        &arks(&repo, &["notes", "append", "-m", "second"]),
        "notes append",
    );
    let shown = arks(&repo, &["notes", "show"]);
    assert_ok(&shown, "notes show");
    let text = String::from_utf8_lossy(&shown.stdout);
    assert!(text.contains("first"), "notes has first");
    assert!(text.contains("second"), "notes has second");

    // blame path
    let blame = arks(&repo, &["blame", "a.txt"]);
    assert_ok(&blame, "blame");
    assert!(
        !String::from_utf8_lossy(&blame.stdout).trim().is_empty(),
        "blame lines"
    );

    // fsck / count-objects -v / pack-refs --all
    assert_ok(&arks(&repo, &["fsck"]), "fsck");
    let co = arks(&repo, &["count-objects", "-v"]);
    assert_ok(&co, "count-objects -v");
    assert!(
        String::from_utf8_lossy(&co.stdout).contains("count:"),
        "verbose count"
    );
    assert_ok(&arks(&repo, &["pack-refs", "--all"]), "pack-refs --all");
    assert!(
        repo.join(".git/packed-refs").is_file(),
        "packed-refs written"
    );

    record_row("M16-SY5-02", true);
}

#[test]
fn m16_sy5_03_detached_commit_and_cherry_pick_sequencer() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("r");
    seed_repo(&repo);

    // Detached commit + amend
    let tip = String::from_utf8_lossy(&arks(&repo, &["rev-parse", "HEAD"]).stdout)
        .trim()
        .to_string();
    assert_ok(&arks(&repo, &["checkout", "--detach", &tip]), "detach");
    fs::write(repo.join("d.txt"), "detached\n").unwrap();
    assert_ok(&arks(&repo, &["add", "d.txt"]), "add detached");
    assert_ok(
        &arks(&repo, &["commit", "-m", "detached commit"]),
        "detached commit",
    );
    assert_ok(
        &arks(&repo, &["commit", "--amend", "-m", "detached amend"]),
        "detached amend",
    );

    // Conflicted cherry-pick → markers + non-zero; --abort restores.
    // Reattach to main at current detached tip first.
    let det = String::from_utf8_lossy(&arks(&repo, &["rev-parse", "HEAD"]).stdout)
        .trim()
        .to_string();
    assert_ok(&arks(&repo, &["checkout", "-b", "tmp-reattach"]), "temp branch");
    assert_ok(&arks(&repo, &["checkout", "main"]), "back main");
    assert_ok(&arks(&repo, &["reset", "--hard", &det]), "main at detached tip");
    fs::write(repo.join("x.txt"), "ours\n").unwrap();
    assert_ok(&arks(&repo, &["add", "x.txt"]), "add ours");
    assert_ok(&arks(&repo, &["commit", "-m", "ours"]), "commit ours");
    let parent = String::from_utf8_lossy(&arks(&repo, &["rev-parse", "HEAD~1"]).stdout)
        .trim()
        .to_string();
    assert_ok(&arks(&repo, &["checkout", "-b", "other"]), "branch other");
    assert_ok(&arks(&repo, &["reset", "--hard", &parent]), "other at parent");
    fs::write(repo.join("x.txt"), "theirs\n").unwrap();
    assert_ok(&arks(&repo, &["add", "x.txt"]), "add theirs");
    assert_ok(&arks(&repo, &["commit", "-m", "theirs"]), "commit theirs");
    let theirs = String::from_utf8_lossy(&arks(&repo, &["rev-parse", "HEAD"]).stdout)
        .trim()
        .to_string();
    assert_ok(&arks(&repo, &["checkout", "main"]), "back to main");
    let before = String::from_utf8_lossy(&arks(&repo, &["rev-parse", "HEAD"]).stdout)
        .trim()
        .to_string();
    let conflict = arks(&repo, &["cherry-pick", &theirs]);
    assert_fail(&conflict, "cherry-pick conflict");
    let body = fs::read_to_string(repo.join("x.txt")).unwrap_or_default();
    assert!(
        body.contains("<<<<<<<") && body.contains(">>>>>>>"),
        "conflict markers present: {body}"
    );
    assert!(
        repo.join(".git/CHERRY_PICK_HEAD").is_file(),
        "CHERRY_PICK_HEAD"
    );
    assert_ok(&arks(&repo, &["cherry-pick", "--abort"]), "cherry-pick --abort");
    let after = String::from_utf8_lossy(&arks(&repo, &["rev-parse", "HEAD"]).stdout)
        .trim()
        .to_string();
    assert_eq!(before, after, "abort restored HEAD");
    assert!(
        !repo.join(".git/CHERRY_PICK_HEAD").is_file(),
        "CHERRY_PICK_HEAD cleared"
    );

    // Conflict then --continue after resolving.
    let conflict2 = arks(&repo, &["cherry-pick", &theirs]);
    assert_fail(&conflict2, "cherry-pick conflict again");
    fs::write(repo.join("x.txt"), "resolved\n").unwrap();
    assert_ok(&arks(&repo, &["add", "x.txt"]), "stage resolution");
    assert_ok(
        &arks(&repo, &["cherry-pick", "--continue"]),
        "cherry-pick --continue",
    );
    let resolved = fs::read_to_string(repo.join("x.txt")).unwrap();
    assert_eq!(resolved, "resolved\n");

    record_row("M16-SY5-03", true);
}

#[test]
fn m16_sy5_04_format_patch_cover_letter_and_am_3way() {
    // SoT Sy-5 claimed rows not previously G21-gated: --cover-letter / am --3way.
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("r");
    seed_repo(&repo);
    let base = String::from_utf8_lossy(&arks(&repo, &["rev-parse", "HEAD"]).stdout)
        .trim()
        .to_string();
    fs::write(repo.join("a.txt"), "two\n").unwrap();
    assert_ok(&arks(&repo, &["add", "a.txt"]), "add two");
    assert_ok(&arks(&repo, &["commit", "-m", "c2"]), "commit c2");
    let tip = String::from_utf8_lossy(&arks(&repo, &["rev-parse", "HEAD"]).stdout)
        .trim()
        .to_string();
    let patches = tmp.path().join("patches");
    let range = format!("{base}..{tip}");
    assert_ok(
        &arks(
            &repo,
            &[
                "format-patch",
                "--cover-letter",
                "-o",
                patches.to_str().unwrap(),
                &range,
            ],
        ),
        "format-patch --cover-letter",
    );
    assert!(
        patches.join("0000-cover-letter.patch").is_file(),
        "cover letter written"
    );
    let cover = fs::read_to_string(patches.join("0000-cover-letter.patch")).unwrap();
    assert!(
        cover.contains("Cover Letter") && cover.contains("c2"),
        "cover letter content"
    );

    // Reset to base and diverge so clean am would refuse; --3way takes tip.
    assert_ok(&arks(&repo, &["reset", "--hard", &base]), "reset base");
    fs::write(repo.join("a.txt"), "diverged\n").unwrap();
    assert_ok(&arks(&repo, &["add", "a.txt"]), "add diverged");
    assert_ok(&arks(&repo, &["commit", "-m", "diverged"]), "commit diverged");
    let patch_file = fs::read_dir(&patches)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .find(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("0001-"))
                .unwrap_or(false)
        })
        .expect("0001 patch");
    let plain = arks(&repo, &["am", patch_file.to_str().unwrap()]);
    assert_fail(&plain, "am without --3way on mismatch");
    assert_ok(
        &arks(&repo, &["am", "--3way", patch_file.to_str().unwrap()]),
        "am --3way",
    );
    let body = fs::read_to_string(repo.join("a.txt")).unwrap();
    assert_eq!(body, "two\n");

    record_row("M16-SY5-04", true);
}

#[test]
fn m16_sy5_05_version_and_global_version() {
    // Sy-5 ledger: `version` / `--version` print version string (git-version).
    let out_cmd = Command::new(arks_bin())
        .arg("version")
        .output()
        .expect("arks version");
    assert_ok(&out_cmd, "arks version");
    let cmd_text = String::from_utf8_lossy(&out_cmd.stdout);
    assert!(
        cmd_text.starts_with("arks version "),
        "expected 'arks version …', got {cmd_text:?}"
    );
    assert!(
        cmd_text.trim().split_whitespace().count() >= 3,
        "expected 'arks version <semver>', got {cmd_text:?}"
    );

    let out_opt = Command::new(arks_bin())
        .arg("--version")
        .output()
        .expect("arks --version");
    assert_ok(&out_opt, "arks --version");
    assert_eq!(
        String::from_utf8_lossy(&out_cmd.stdout).trim(),
        String::from_utf8_lossy(&out_opt.stdout).trim(),
        "version and --version must match"
    );

    record_row("M16-SY5-05", true);
}
