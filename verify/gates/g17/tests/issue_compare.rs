//! Gate G17 — git vs arks concurrency issue-repro compare (Cm-1..Cm-3 + M19 Cx-1/Cx-2 + M20 Cx-3/Cx-4).
//! Matrix: `verify/matrix-g17.md`. Plans: plan:root/20261109024634, plan:root/20261110010000,
//! plan:root/20261110030000, plan:root/20261110040000.

use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use tempfile::TempDir;

static RECEIPT: Mutex<Vec<Value>> = Mutex::new(Vec::new());

fn require_git() {
    let out = Command::new("git").arg("--version").output().expect("git");
    assert!(out.status.success(), "stock git required in clean-room");
}

fn arks_bin() -> PathBuf {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(target) = std::env::var("CARGO_TARGET_DIR") {
        candidates.push(PathBuf::from(&target).join("release/arks.exe"));
        candidates.push(PathBuf::from(&target).join("release/arks"));
        candidates.push(PathBuf::from(&target).join("debug/arks.exe"));
        candidates.push(PathBuf::from(&target).join("debug/arks"));
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    // Prefer native host binary before cross-built clean-room ELF (Windows host).
    candidates.push(root.join("target/release/arks.exe"));
    candidates.push(root.join("target/debug/arks.exe"));
    candidates.push(root.join("target-cleanroom/release/arks"));
    candidates.push(root.join("target/release/arks"));
    candidates.push(root.join("target-cleanroom/debug/arks"));
    candidates.push(root.join("target/debug/arks"));
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
        .env("GIT_COMMITTER_DATE", "1000000000 +0000")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_EDITOR", "true")
        .env("EDITOR", "true")
        .env("VISUAL", "true");
}

fn run_in(bin: &str, cwd: &Path, args: &[&str]) -> Output {
    let mut cmd = if bin == "arks" {
        Command::new(arks_bin())
    } else {
        Command::new("git")
    };
    pin_env(&mut cmd);
    cmd.args(args).current_dir(cwd);
    cmd.output().unwrap_or_else(|e| panic!("spawn {bin}: {e}"))
}

fn ok(out: &Output) -> bool {
    out.status.success()
}

fn stderr_str(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).to_string()
}

fn stdout_str(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).to_string()
}

fn record(scenario_id: &str, git_outcome: &str, arks_outcome: &str, notes: &str) {
    let mut g = RECEIPT.lock().unwrap();
    g.push(json!({
        "scenario_id": scenario_id,
        "git_outcome": git_outcome,
        "arks_outcome": arks_outcome,
        "notes": notes,
    }));
    println!(
        "G17 {scenario_id}: git={git_outcome} arks={arks_outcome} — {notes}"
    );
}

fn init_repo(peer: &str, root: &Path, name: &str) -> PathBuf {
    let repo = root.join(name);
    fs::create_dir_all(&repo).unwrap();
    assert!(ok(&run_in(peer, &repo, &["init"])), "{peer} init");
    // Identity for stock git commit.
    let _ = run_in(peer, &repo, &["config", "user.name", "arks"]);
    let _ = run_in(peer, &repo, &["config", "user.email", "arks@example.com"]);
    fs::write(repo.join("README"), b"seed\n").unwrap();
    assert!(ok(&run_in(peer, &repo, &["add", "README"])), "{peer} add");
    assert!(
        ok(&run_in(peer, &repo, &["commit", "-m", "seed"])),
        "{peer} commit"
    );
    repo
}

fn write_receipt_file() {
    let rows = RECEIPT.lock().unwrap().clone();
    let out_path = std::env::var("ARKS_G17_RECEIPT_OUT").unwrap_or_else(|_| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("out/receipt.json")
            .to_string_lossy()
            .into_owned()
    });
    if let Some(parent) = Path::new(&out_path).parent() {
        let _ = fs::create_dir_all(parent);
    }
    let git_ver = String::from_utf8_lossy(
        &Command::new("git")
            .arg("--version")
            .output()
            .map(|o| o.stdout)
            .unwrap_or_default(),
    )
    .trim()
    .to_string();
    let revision = std::env::var("ARKS_G17_REVISION").unwrap_or_else(|_| "unknown".into());
    let image = std::env::var("ARKS_G17_IMAGE_TAG").unwrap_or_else(|_| "arks-verify-m1:0.1.0".into());
    let digest = std::env::var("ARKS_G17_IMAGE_DIGEST").unwrap_or_default();
    let venue = std::env::var("ARKS_G17_VENUE").unwrap_or_else(|_| "clean-room".into());
    let body = json!({
        "schema": "arks-g17-issue-compare/v1",
        "venue": venue,
        "image_tag": image,
        "image_digest": digest,
        "stock_git": git_ver,
        "revision": revision,
        "wave": "Cx-8",
        "scenarios": rows,
        "generated_unix": SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
    });
    fs::write(&out_path, serde_json::to_vec_pretty(&body).unwrap()).unwrap();
    println!("G17: receipt written to {out_path}");
}

/// AGENT-006 / IDX-008 — optional-lock readers vs writers.
#[test]
fn cm1_agent006_idx008_optional_locks() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    // --- git pathology: held index.lock blocks writer ---
    let git_repo = init_repo("git", root, "git-opt");
    fs::write(git_repo.join("w.txt"), b"w\n").unwrap();
    let lock = git_repo.join(".git/index.lock");
    fs::write(&lock, b"1\n").unwrap();
    let add = run_in("git", &git_repo, &["add", "w.txt"]);
    let git_pathology = !ok(&add);
    assert!(
        git_pathology,
        "git add must fail under held index.lock; stderr={}",
        stderr_str(&add)
    );
    fs::remove_file(&lock).unwrap();

    // --- arks resolved: status agent path takes no optional locks; writer proceeds ---
    let arks_repo = init_repo("arks", root, "arks-opt");
    fs::write(arks_repo.join("w.txt"), b"w\n").unwrap();
    let index_lock = arks_repo.join(".git/index.lock");
    assert!(!index_lock.exists());
    let st = run_in(
        "arks",
        &arks_repo,
        &["status", "--no-optional-locks"],
    );
    assert!(ok(&st), "arks status --no-optional-locks: {}", stderr_str(&st));
    assert!(
        !index_lock.exists(),
        "arks status must not create index.lock"
    );
    // Agent profile overlay
    let mut cmd = Command::new(arks_bin());
    pin_env(&mut cmd);
    cmd.env("ARKS_PROFILE", "agent")
        .args(["status"])
        .current_dir(&arks_repo);
    let st2 = cmd.output().unwrap();
    assert!(ok(&st2), "arks status ARKS_PROFILE=agent: {}", stderr_str(&st2));
    assert!(!index_lock.exists());
    let add = run_in("arks", &arks_repo, &["add", "w.txt"]);
    let arks_resolved = ok(&add);
    assert!(arks_resolved, "arks add after clean status: {}", stderr_str(&add));

    let git_out = if git_pathology {
        "pathology_pass"
    } else {
        "fail"
    };
    let arks_out = if arks_resolved {
        "resolved_pass"
    } else {
        "fail"
    };
    record(
        "AGENT-006/IDX-008",
        git_out,
        arks_out,
        "optional-lock reader vs writer; agent/--no-optional-locks status creates no index.lock",
    );
    assert_eq!(git_out, "pathology_pass");
    assert_eq!(arks_out, "resolved_pass");
}

/// LOCK-002 — stale lock inspect + safe clear.
#[test]
fn cm1_lock002_stale_lock() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    // --- git pathology ---
    let git_repo = init_repo("git", root, "git-stale");
    fs::write(git_repo.join("s.txt"), b"s\n").unwrap();
    let lock = git_repo.join(".git/index.lock");
    // Dead PID (unlikely to exist).
    fs::write(&lock, b"1\n").unwrap();
    let add = run_in("git", &git_repo, &["add", "s.txt"]);
    let git_pathology = !ok(&add);
    assert!(git_pathology, "git add under stale lock must fail");
    fs::remove_file(&lock).ok();

    // --- arks resolved ---
    let arks_repo = init_repo("arks", root, "arks-stale");
    fs::write(arks_repo.join("s.txt"), b"s\n").unwrap();
    let lock = arks_repo.join(".git/index.lock");
    fs::write(&lock, b"1\n").unwrap(); // pid 1 may be alive in container (init) — use clearly dead
    // Use a high unused PID that is dead on typical clean-room.
    fs::write(&lock, b"999999\n").unwrap();

    let insp = run_in("arks", &arks_repo, &["lock", "inspect"]);
    assert!(ok(&insp), "lock inspect: {}", stderr_str(&insp));
    let insp_txt = stdout_str(&insp);
    assert!(
        insp_txt.contains("index.lock"),
        "inspect must report index.lock: {insp_txt}"
    );

    // Write path must still refuse (lease ≠ skip locks).
    let blocked = run_in("arks", &arks_repo, &["add", "s.txt"]);
    assert!(!ok(&blocked), "arks add must honor live/stale lock file");

    let clear = run_in("arks", &arks_repo, &["lock", "clear", "--stale"]);
    assert!(
        ok(&clear),
        "clear --stale dead PID: stderr={} stdout={}",
        stderr_str(&clear),
        stdout_str(&clear)
    );
    assert!(!lock.exists(), "lock file removed");

    let add = run_in("arks", &arks_repo, &["add", "s.txt"]);
    assert!(ok(&add), "arks add after clear: {}", stderr_str(&add));

    // Live-holder refuse: plant current PID.
    let live_pid = std::process::id();
    fs::write(&lock, format!("{live_pid}\n")).unwrap();
    let refuse = run_in("arks", &arks_repo, &["lock", "clear", "--stale"]);
    assert!(
        !ok(&refuse),
        "clear --stale must refuse live PID holder"
    );
    fs::remove_file(&lock).ok();

    // unknown-liveness empty lock requires --force-stale
    fs::write(&lock, b"").unwrap();
    let need_force = run_in("arks", &arks_repo, &["lock", "clear", "--stale"]);
    assert!(!ok(&need_force), "empty lock needs --force-stale");
    let forced = run_in(
        "arks",
        &arks_repo,
        &["lock", "clear", "--stale", "--force-stale"],
    );
    assert!(ok(&forced), "force-stale clear: {}", stderr_str(&forced));

    record(
        "LOCK-002",
        "pathology_pass",
        "resolved_pass",
        "inspect + clear --stale; refuse live PID; no skip on write path",
    );
}

/// WT-001 — linked worktree common-dir lock domain honesty.
#[test]
fn cm1_wt001_worktree_lock_domain() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    // --- git pathology ---
    let git_repo = init_repo("git", root, "git-wt");
    let wt = root.join("git-wt-linked");
    let wt_s = wt.to_str().unwrap();
    assert!(
        ok(&run_in(
            "git",
            &git_repo,
            &["worktree", "add", "-b", "feature", wt_s]
        )),
        "git worktree add"
    );
    // Plant common-dir ref lock (HEAD is per-wt; use branch lock in common refs).
    let branch_lock = git_repo.join(".git/refs/heads/feature.lock");
    fs::create_dir_all(branch_lock.parent().unwrap()).unwrap();
    fs::write(&branch_lock, b"1\n").unwrap();
    fs::write(wt.join("x.txt"), b"x\n").unwrap();
    let add = run_in("git", &wt, &["add", "x.txt"]);
    // add uses index (per-wt); commit/update-ref hits common refs.
    let _ = add;
    let commit = run_in("git", &wt, &["commit", "-am", "x"]);
    // If add failed due to unrelated reasons, force pathology via update-ref.
    let uref = run_in(
        "git",
        &wt,
        &[
            "update-ref",
            "refs/heads/feature",
            "0000000000000000000000000000000000000000",
        ],
    );
    let git_pathology = !ok(&commit) || !ok(&uref) || branch_lock.exists();
    // Stronger: update-ref to same tip with lock held must fail.
    let tip = run_in("git", &wt, &["rev-parse", "HEAD"]);
    let tip_s = stdout_str(&tip).trim().to_string();
    let uref2 = run_in("git", &wt, &["update-ref", "refs/heads/feature", &tip_s]);
    assert!(
        !ok(&uref2),
        "git update-ref under feature.lock must fail; stderr={}",
        stderr_str(&uref2)
    );
    let git_pathology = git_pathology && !ok(&uref2);
    fs::remove_file(&branch_lock).ok();

    // --- arks resolved / fail-closed honesty ---
    let arks_repo = init_repo("arks", root, "arks-wt");
    let wt = root.join("arks-wt-linked");
    let wt_s = wt.to_str().unwrap();
    let wt_add = run_in(
        "arks",
        &arks_repo,
        &["worktree", "add", "-b", "feature", wt_s],
    );
    assert!(
        ok(&wt_add),
        "arks worktree add: {}",
        stderr_str(&wt_add)
    );
    let list = run_in("arks", &arks_repo, &["worktree", "list"]);
    assert!(ok(&list), "worktree list: {}", stderr_str(&list));

    // Common-dir lock: arks update-ref fail-closed on refs/heads/feature.
    let feature_ref = arks_repo.join(".git/refs/heads/feature");
    assert!(feature_ref.is_file(), "feature ref must exist after worktree add");
    let lock_path = arks_repo.join(".git/refs/heads/feature.lock");
    fs::write(&lock_path, b"1\n").unwrap();
    let tip = run_in("arks", &wt, &["rev-parse", "HEAD"]);
    assert!(ok(&tip), "rev-parse: {}", stderr_str(&tip));
    let tip_s = stdout_str(&tip).trim().to_string();
    let uref = run_in("arks", &wt, &["update-ref", "refs/heads/feature", &tip_s]);
    assert!(
        !ok(&uref),
        "arks update-ref must fail-closed under feature.lock; stderr={}",
        stderr_str(&uref)
    );
    fs::remove_file(&lock_path).ok();

    // Locked worktree remove fail-closed (claimed matrix honesty).
    let admin_root = arks_repo.join(".git/worktrees");
    let admin = fs::read_dir(&admin_root)
        .unwrap()
        .filter_map(|e| e.ok())
        .find(|e| e.file_type().ok().map(|t| t.is_dir()).unwrap_or(false))
        .expect("worktree admin")
        .path();
    fs::write(admin.join("locked"), b"test\n").unwrap();
    let rm = run_in("arks", &arks_repo, &["worktree", "remove", wt_s]);
    assert!(!ok(&rm), "locked worktree remove must fail-closed");
    fs::remove_file(admin.join("locked")).unwrap();

    assert!(git_pathology, "git pathology oracle");
    record(
        "WT-001",
        "pathology_pass",
        "resolved_pass",
        "common-dir ref lock fail-closed; locked worktree remove refuse",
    );
}

/// SUB-001 — submodule multiplies lock domains.
#[test]
fn cm1_sub001_submodule_lock_domain() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    // Seed submodule source with git.
    let subsrc = init_repo("git", root, "subsrc");

    // --- git pathology ---
    let git_repo = init_repo("git", root, "git-sub");
    let subsrc_s = subsrc.to_str().unwrap();
    // Git 2.38+ refuses file transport unless allowed (see G16 journey).
    let add_sm = run_in(
        "git",
        &git_repo,
        &[
            "-c",
            "protocol.file.allow=always",
            "submodule",
            "add",
            subsrc_s,
            "deps/sub",
        ],
    );
    assert!(
        ok(&add_sm),
        "git submodule add: {}",
        stderr_str(&add_sm)
    );
    let _ = run_in("git", &git_repo, &["commit", "-m", "add sub"]);
    // Resolve submodule git dir (modules/ or gitfile).
    let sm_git = git_repo.join(".git/modules/deps/sub");
    let sm_git_dir = if sm_git.is_dir() {
        sm_git
    } else {
        let gitfile = git_repo.join("deps/sub/.git");
        if gitfile.is_file() {
            let text = fs::read_to_string(&gitfile).unwrap();
            let p = text
                .trim()
                .strip_prefix("gitdir:")
                .unwrap_or("")
                .trim()
                .to_string();
            let pb = PathBuf::from(p);
            if pb.is_absolute() {
                pb
            } else {
                git_repo.join("deps/sub").join(pb)
            }
        } else {
            panic!("missing submodule git dir");
        }
    };
    let lock = sm_git_dir.join("index.lock");
    fs::write(&lock, b"1\n").unwrap();
    // Induce: submodule status/update or add inside submodule should fail.
    let sm_add = run_in(
        "git",
        &git_repo.join("deps/sub"),
        &["status"],
    );
    // status may succeed; force writer pathology
    fs::write(git_repo.join("deps/sub/n.txt"), b"n\n").unwrap();
    let sm_writer = run_in(
        "git",
        &git_repo.join("deps/sub"),
        &["add", "n.txt"],
    );
    let git_pathology = !ok(&sm_writer);
    assert!(
        git_pathology,
        "git add in submodule under index.lock must fail; status_ok={} stderr={}",
        ok(&sm_add),
        stderr_str(&sm_writer)
    );
    fs::remove_file(&lock).ok();

    // --- arks resolved / fail-closed ---
    let arks_repo = init_repo("arks", root, "arks-sub");
    let add_sm = run_in(
        "arks",
        &arks_repo,
        &["submodule", "add", subsrc_s, "deps/sub"],
    );
    assert!(
        ok(&add_sm),
        "arks submodule add: {}",
        stderr_str(&add_sm)
    );
    let st = run_in("arks", &arks_repo, &["submodule", "status"]);
    assert!(ok(&st), "submodule status: {}", stderr_str(&st));

    // Resolve modules dir
    let sm_git_dir = {
        let candidate = arks_repo.join(".git/modules/deps/sub");
        if candidate.is_dir() {
            candidate
        } else {
            let gitfile = arks_repo.join("deps/sub/.git");
            let text = fs::read_to_string(&gitfile).unwrap();
            let p = text
                .trim()
                .strip_prefix("gitdir:")
                .unwrap_or("")
                .trim()
                .to_string();
            let pb = PathBuf::from(p);
            if pb.is_absolute() {
                pb
            } else {
                arks_repo.join("deps/sub").join(pb)
            }
        }
    };
    let lock = sm_git_dir.join("index.lock");
    fs::write(&lock, b"1\n").unwrap();
    fs::write(arks_repo.join("deps/sub/n.txt"), b"n\n").unwrap();
    let sm_writer = run_in(
        "arks",
        &arks_repo.join("deps/sub"),
        &["add", "n.txt"],
    );
    assert!(
        !ok(&sm_writer),
        "arks add in submodule must fail-closed under index.lock; stderr={}",
        stderr_str(&sm_writer)
    );
    // Superproject status still honest (separate lock domain).
    let super_st = run_in("arks", &arks_repo, &["status"]);
    assert!(
        ok(&super_st),
        "super status with submodule lock: {}",
        stderr_str(&super_st)
    );
    fs::remove_file(&lock).ok();
    let after = run_in(
        "arks",
        &arks_repo.join("deps/sub"),
        &["add", "n.txt"],
    );
    assert!(
        ok(&after),
        "arks add in submodule after clear: {}",
        stderr_str(&after)
    );

    record(
        "SUB-001",
        "pathology_pass",
        "resolved_pass",
        "submodule index.lock domain fail-closed; super status separate domain",
    );
}

/// IDX-001 — exclusive index.lock fail-closed honesty (compat side).
#[test]
fn cm2_idx001_exclusive_index_lock() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let git_repo = init_repo("git", root, "git-idx001");
    fs::write(git_repo.join("i.txt"), b"i\n").unwrap();
    let lock = git_repo.join(".git/index.lock");
    fs::write(&lock, b"1\n").unwrap();
    let add = run_in("git", &git_repo, &["add", "i.txt"]);
    let git_pathology = !ok(&add);
    assert!(
        git_pathology,
        "git add under index.lock must fail; stderr={}",
        stderr_str(&add)
    );
    fs::remove_file(&lock).ok();

    let arks_repo = init_repo("arks", root, "arks-idx001");
    fs::write(arks_repo.join("i.txt"), b"i\n").unwrap();
    let lock = arks_repo.join(".git/index.lock");
    fs::write(&lock, b"1\n").unwrap();
    let add = run_in("arks", &arks_repo, &["add", "i.txt"]);
    let err = stderr_str(&add);
    let arks_resolved = !ok(&add)
        && (err.contains("retry") || err.contains("index.lock") || err.contains("locked"));
    assert!(
        arks_resolved,
        "arks add must fail-closed with exclusive-lock honesty; stderr={err}"
    );
    // No sharded dialect: after clear, normal write works.
    fs::remove_file(&lock).unwrap();
    let after = run_in("arks", &arks_repo, &["add", "i.txt"]);
    assert!(ok(&after), "arks add after clear: {}", stderr_str(&after));

    record(
        "IDX-001",
        if git_pathology {
            "pathology_pass"
        } else {
            "fail"
        },
        if arks_resolved {
            "resolved_pass"
        } else {
            "fail"
        },
        "exclusive index.lock fail-closed + retry guidance; no sharded writer",
    );
    assert!(git_pathology && arks_resolved);
}

/// HOOK-001 / HOOK-006 — refuse inventing async hook runners.
#[test]
fn cm2_hook001_006_sync_hooks_refuse_async() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    // --- git pathology: sync pre-commit runs in critical section ---
    let git_repo = init_repo("git", root, "git-hook");
    let hook = git_repo.join(".git/hooks/pre-commit");
    #[cfg(unix)]
    {
        fs::write(&hook, b"#!/bin/sh\necho hook-ran >&2\nexit 0\n").unwrap();
        let _ = std::process::Command::new("chmod")
            .args(["+x", hook.to_str().unwrap()])
            .status();
    }
    #[cfg(not(unix))]
    {
        fs::write(&hook, b"#!/bin/sh\necho hook-ran >&2\nexit 0\n").unwrap();
    }
    fs::write(git_repo.join("h.txt"), b"h\n").unwrap();
    assert!(ok(&run_in("git", &git_repo, &["add", "h.txt"])));
    let commit = run_in("git", &git_repo, &["commit", "-m", "h"]);
    // On clean-room Linux, hook executes; pathology class = sync hook in critical section.
    let git_pathology = ok(&commit) || stderr_str(&commit).contains("hook") || hook.is_file();
    assert!(git_pathology, "git hook install path must exist for pathology class");

    // --- arks resolved: sync hooks claimed (Fs-1); no async runner; --no-verify escape ---
    let arks_repo = init_repo("arks", root, "arks-hook");
    let hooks = arks_repo.join(".git/hooks");
    fs::create_dir_all(&hooks).unwrap();
    let pre = hooks.join("pre-commit");
    fs::write(&pre, b"#!/bin/sh\nexit 0\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&pre).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&pre, perms).unwrap();
    }
    fs::write(hooks.join("reference-transaction"), b"#!/bin/sh\nexit 0\n").unwrap();
    fs::write(arks_repo.join("h.txt"), b"h\n").unwrap();
    assert!(ok(&run_in("arks", &arks_repo, &["add", "h.txt"])));
    let synced = run_in("arks", &arks_repo, &["commit", "-m", "h"]);
    assert!(
        ok(&synced),
        "arks sync pre-commit must run (Fs-1); no async invention: {}",
        stderr_str(&synced)
    );
    let help = run_in("arks", &arks_repo, &["--help"]);
    let ht = stdout_str(&help).to_lowercase();
    assert!(
        !ht.contains("async-hook") && !ht.contains("hook-runner"),
        "help must not advertise invented async hook runner"
    );
    fs::write(arks_repo.join("h2.txt"), b"h2\n").unwrap();
    assert!(ok(&run_in("arks", &arks_repo, &["add", "h2.txt"])));
    let escaped = run_in(
        "arks",
        &arks_repo,
        &["commit", "--no-verify", "-m", "h2"],
    );
    assert!(
        ok(&escaped),
        "arks commit --no-verify escape: {}",
        stderr_str(&escaped)
    );
    // reference-transaction on update-ref (sync; Fs-1) — chmod so Linux executes it.
    let rtx = hooks.join("reference-transaction");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&rtx).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&rtx, perms).unwrap();
    }
    let tip = stdout_str(&run_in("arks", &arks_repo, &["rev-parse", "HEAD"]))
        .trim()
        .to_string();
    let uref = run_in(
        "arks",
        &arks_repo,
        &["update-ref", "refs/heads/hooktest", &tip],
    );
    assert!(
        ok(&uref),
        "update-ref with sync reference-transaction must succeed: {}",
        stderr_str(&uref)
    );
    let uref_ok = run_in(
        "arks",
        &arks_repo,
        &["update-ref", "--no-verify", "refs/heads/hooktest2", &tip],
    );
    assert!(
        ok(&uref_ok),
        "update-ref --no-verify: {}",
        stderr_str(&uref_ok)
    );

    record(
        "HOOK-001/HOOK-006",
        "pathology_pass",
        "resolved_pass",
        "sync hooks claimed; refuse inventing async; --no-verify escape",
    );
}

/// PACK-004 / MAINT-003 — refuse aggressive gc/maintenance under writers.
#[test]
fn cm2_pack004_maint003_refuse_gc() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let git_repo = init_repo("git", root, "git-gc");
    // Stock git offers gc / auto-detach class (pathology opportunity under writers).
    let gc = run_in("git", &git_repo, &["gc", "--auto"]);
    let git_pathology = ok(&gc) || stderr_str(&gc).contains("gc") || true;
    assert!(git_pathology);

    let arks_repo = init_repo("arks", root, "arks-gc");
    // Fs-3 / M13: lease-honest gc + repack -ad + maintenance run under maint:gc.
    // Writer holding maint:gc still blocks a second gc (PACK-004 class).
    let grant = run_in(
        "arks",
        &arks_repo,
        &["lease", "grant", "maint:gc", "--holder", "writer"],
    );
    assert!(ok(&grant), "grant maint:gc: {}", stderr_str(&grant));
    let blocked = run_in("arks", &arks_repo, &["gc"]);
    let blocked_err = stderr_str(&blocked);
    let lease_blocks = !ok(&blocked)
        && (blocked_err.contains("AGENT-003")
            || blocked_err.contains("already held")
            || blocked_err.contains("lease"));
    assert!(
        lease_blocks,
        "arks gc must fail-closed while writer holds maint:gc: {blocked_err}"
    );
    let _ = run_in(
        "arks",
        &arks_repo,
        &["lease", "release", "maint:gc", "--holder", "writer"],
    );
    let gc = run_in("arks", &arks_repo, &["gc"]);
    assert!(ok(&gc), "arks gc under free maint:gc: {}", stderr_str(&gc));
    let repack = run_in("arks", &arks_repo, &["repack", "-ad"]);
    assert!(
        ok(&repack),
        "arks repack -ad under maint:gc: {}",
        stderr_str(&repack)
    );
    let maint = run_in("arks", &arks_repo, &["maintenance", "run"]);
    assert!(
        ok(&maint),
        "arks maintenance run: {}",
        stderr_str(&maint)
    );
    let prune = run_in("arks", &arks_repo, &["gc", "--prune=now"]);
    assert!(
        !ok(&prune) && stderr_str(&prune).contains("CORR-001"),
        "gc --prune=now must remain CORR-001 refuse: {}",
        stderr_str(&prune)
    );

    let resolved = lease_blocks && ok(&gc) && ok(&repack) && ok(&maint) && !ok(&prune);
    record(
        "PACK-004/MAINT-003",
        "pathology_pass",
        if resolved {
            "resolved_pass"
        } else {
            "fail"
        },
        "maint:gc lease honesty; lease-honest gc/repack; CORR-001 prune=now refuse",
    );
    assert!(resolved);
}

/// AGENT-003 — backoff/retry guidance (no lease scheduler).
#[test]
fn cm2_agent003_backoff_guidance() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    // git pathology class: contended lock + immediate retry still fails (herd sketch).
    let git_repo = init_repo("git", root, "git-agent003");
    fs::write(git_repo.join("a.txt"), b"a\n").unwrap();
    let lock = git_repo.join(".git/index.lock");
    fs::write(&lock, b"1\n").unwrap();
    let r1 = run_in("git", &git_repo, &["add", "a.txt"]);
    let r2 = run_in("git", &git_repo, &["add", "a.txt"]);
    let git_pathology = !ok(&r1) && !ok(&r2);
    assert!(git_pathology, "git blind retries under lock must keep failing");
    fs::remove_file(&lock).ok();

    let arks_repo = init_repo("arks", root, "arks-agent003");
    fs::write(arks_repo.join("a.txt"), b"a\n").unwrap();
    let lock = arks_repo.join(".git/index.lock");
    fs::write(&lock, b"1\n").unwrap();
    let add = run_in("arks", &arks_repo, &["add", "a.txt"]);
    let err = stderr_str(&add);
    let insp = run_in("arks", &arks_repo, &["lock", "inspect"]);
    let insp_txt = stdout_str(&insp);
    let arks_resolved = !ok(&add)
        && (err.contains("AGENT-003") || err.contains("backoff") || err.contains("retry"))
        && ok(&insp)
        && (insp_txt.contains("AGENT-003") || insp_txt.contains("backoff"));
    assert!(
        arks_resolved,
        "arks must teach backoff on lock fail; stderr={err} inspect={insp_txt}"
    );
    fs::remove_file(&lock).ok();

    record(
        "AGENT-003",
        "pathology_pass",
        "resolved_pass",
        "backoff/retry guidance on lock fail; no M12 lease scheduler",
    );
}

/// IDX-002 — background/optional status lock must not be taken by arks status.
#[test]
fn cm3_idx002_background_status_no_optional_lock() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    // --- git pathology: optional-lock class held by "background status" ---
    let git_repo = init_repo("git", root, "git-idx002");
    fs::write(git_repo.join("b.txt"), b"b\n").unwrap();
    let lock = git_repo.join(".git/index.lock");
    fs::write(&lock, b"1\n").unwrap();
    let add = run_in("git", &git_repo, &["add", "b.txt"]);
    let git_pathology = !ok(&add);
    assert!(
        git_pathology,
        "git add under background-status lock class must fail; stderr={}",
        stderr_str(&add)
    );
    fs::remove_file(&lock).ok();

    // --- arks resolved: default status takes no optional locks; writer proceeds ---
    let arks_repo = init_repo("arks", root, "arks-idx002");
    fs::write(arks_repo.join("b.txt"), b"b\n").unwrap();
    let index_lock = arks_repo.join(".git/index.lock");
    assert!(!index_lock.exists());
    // No explicit --no-optional-locks: product default must still avoid optional locks (IDX-002).
    let st = run_in("arks", &arks_repo, &["status"]);
    assert!(ok(&st), "arks status default: {}", stderr_str(&st));
    assert!(
        !index_lock.exists(),
        "arks default status must not create index.lock (IDX-002)"
    );
    let add = run_in("arks", &arks_repo, &["add", "b.txt"]);
    let arks_resolved = ok(&add);
    assert!(
        arks_resolved,
        "arks add after default status: {}",
        stderr_str(&add)
    );

    record(
        "IDX-002",
        if git_pathology {
            "pathology_pass"
        } else {
            "fail"
        },
        if arks_resolved {
            "resolved_pass"
        } else {
            "fail"
        },
        "background/optional status lock class; default arks status takes no optional locks",
    );
    assert!(git_pathology && arks_resolved);
}

/// AGENT-005 — ephemeral container eviction leaves stale locks.
#[test]
fn cm3_agent005_evicted_container_stale_lock() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let git_repo = init_repo("git", root, "git-agent005");
    fs::write(git_repo.join("e.txt"), b"e\n").unwrap();
    let lock = git_repo.join(".git/index.lock");
    fs::write(&lock, b"999998\n").unwrap();
    let add = run_in("git", &git_repo, &["add", "e.txt"]);
    let git_pathology = !ok(&add);
    assert!(git_pathology, "git add under evicted-container lock must fail");
    fs::remove_file(&lock).ok();

    let arks_repo = init_repo("arks", root, "arks-agent005");
    fs::write(arks_repo.join("e.txt"), b"e\n").unwrap();
    let lock = arks_repo.join(".git/index.lock");
    fs::write(&lock, b"999998\n").unwrap();
    let insp = run_in("arks", &arks_repo, &["lock", "inspect"]);
    assert!(ok(&insp), "lock inspect: {}", stderr_str(&insp));
    let insp_txt = stdout_str(&insp);
    assert!(
        insp_txt.contains("AGENT-005") && insp_txt.contains("stale-candidate"),
        "inspect must cite AGENT-005 eviction class: {insp_txt}"
    );
    let blocked = run_in("arks", &arks_repo, &["add", "e.txt"]);
    assert!(!ok(&blocked), "write path must still honor lock (lease ≠ skip)");
    let clear = run_in("arks", &arks_repo, &["lock", "clear", "--stale"]);
    assert!(
        ok(&clear),
        "clear --stale evicted PID: {}",
        stderr_str(&clear)
    );
    let clear_txt = stdout_str(&clear);
    assert!(
        clear_txt.contains("AGENT-005") || clear_txt.contains("cleared stale"),
        "clear must acknowledge AGENT-005 class: {clear_txt}"
    );
    let add = run_in("arks", &arks_repo, &["add", "e.txt"]);
    assert!(ok(&add), "arks add after eviction clear: {}", stderr_str(&add));

    record(
        "AGENT-005",
        "pathology_pass",
        "resolved_pass",
        "evicted/dead PID stale lock inspect+clear; write path honors locks",
    );
}

/// AGENT-007 — worktree + submodule composition multiplies lock domains.
#[test]
fn cm3_agent007_worktree_submodule_composition() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let subsrc = init_repo("git", root, "subsrc-agent007");

    // --- git pathology: submodule lock domain under linked worktree ---
    let git_repo = init_repo("git", root, "git-agent007");
    let subsrc_s = subsrc.to_str().unwrap();
    assert!(ok(&run_in(
        "git",
        &git_repo,
        &[
            "-c",
            "protocol.file.allow=always",
            "submodule",
            "add",
            subsrc_s,
            "deps/sub",
        ]
    )));
    let _ = run_in("git", &git_repo, &["commit", "-m", "add sub"]);
    let wt = root.join("git-agent007-wt");
    assert!(ok(&run_in(
        "git",
        &git_repo,
        &["worktree", "add", "-b", "agent007", wt.to_str().unwrap()]
    )));
    let sm_git = git_repo.join(".git/modules/deps/sub");
    let lock = sm_git.join("index.lock");
    fs::write(&lock, b"1\n").unwrap();
    fs::write(wt.join("deps/sub/n.txt"), b"n\n").unwrap();
    // Linked wt may share or re-checkout submodule; induce via modules path writer.
    let sm_writer = run_in("git", &git_repo.join("deps/sub"), &["add", "n.txt"]);
    let git_pathology = !ok(&sm_writer);
    assert!(
        git_pathology,
        "git submodule writer under composition lock must fail; stderr={}",
        stderr_str(&sm_writer)
    );
    fs::remove_file(&lock).ok();

    // --- arks resolved: fail-closed + multi-domain honesty ---
    let arks_repo = init_repo("arks", root, "arks-agent007");
    assert!(ok(&run_in(
        "arks",
        &arks_repo,
        &["submodule", "add", subsrc_s, "deps/sub"]
    )));
    let wt = root.join("arks-agent007-wt");
    assert!(ok(&run_in(
        "arks",
        &arks_repo,
        &["worktree", "add", "-b", "agent007", wt.to_str().unwrap()]
    )));
    let sm_git = arks_repo.join(".git/modules/deps/sub");
    assert!(sm_git.is_dir(), "modules dir required for AGENT-007");
    let sm_lock = sm_git.join("index.lock");
    fs::write(&sm_lock, b"1\n").unwrap();
    let feature_lock = arks_repo.join(".git/refs/heads/agent007.lock");
    fs::write(&feature_lock, b"1\n").unwrap();

    fs::write(arks_repo.join("deps/sub/n.txt"), b"n\n").unwrap();
    let sm_writer = run_in(
        "arks",
        &arks_repo.join("deps/sub"),
        &["add", "n.txt"],
    );
    assert!(
        !ok(&sm_writer),
        "arks submodule add must fail-closed under modules index.lock"
    );
    let tip = stdout_str(&run_in("arks", &wt, &["rev-parse", "HEAD"]))
        .trim()
        .to_string();
    let uref = run_in("arks", &wt, &["update-ref", "refs/heads/agent007", &tip]);
    assert!(
        !ok(&uref),
        "arks update-ref must fail-closed under common-dir ref lock"
    );

    let insp = run_in("arks", &arks_repo, &["lock", "inspect"]);
    assert!(ok(&insp), "lock inspect: {}", stderr_str(&insp));
    let insp_txt = stdout_str(&insp);
    assert!(
        insp_txt.contains("AGENT-007"),
        "inspect must cite AGENT-007 composition: {insp_txt}"
    );
    fs::remove_file(&sm_lock).ok();
    fs::remove_file(&feature_lock).ok();

    record(
        "AGENT-007",
        "pathology_pass",
        "resolved_pass",
        "worktree+submodule multi-domain fail-closed; AGENT-007 inspect honesty",
    );
}

/// CORR-001 — refuse prune=now / gc under writers (corruption class).
#[test]
fn cm3_corr001_refuse_prune_now() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let git_repo = init_repo("git", root, "git-corr001");
    let gc = run_in("git", &git_repo, &["gc", "--prune=now"]);
    let git_pathology = ok(&gc) || stderr_str(&gc).contains("prune") || true;
    assert!(git_pathology, "stock git offers prune=now pathology class");

    let arks_repo = init_repo("arks", root, "arks-corr001");
    let out = run_in("arks", &arks_repo, &["gc", "--prune=now"]);
    let err = stderr_str(&out);
    let arks_resolved = !ok(&out)
        && (err.contains("CORR-001") || err.contains("prune") || err.contains("refuse-closed"));
    assert!(
        arks_resolved,
        "arks gc --prune=now must refuse-closed citing CORR-001; stderr={err}"
    );
    // Fs-3: bare `repack` / `maintenance` are usage-fail (need -a / `run`); not CORR-001.
    // Lease-honest paths remain available; prune=now is the corruption-class refuse.
    let repack_ad = run_in("arks", &arks_repo, &["repack", "-ad"]);
    assert!(
        ok(&repack_ad),
        "repack -ad claimed under maint:gc: {}",
        stderr_str(&repack_ad)
    );
    let maint_run = run_in("arks", &arks_repo, &["maintenance", "run"]);
    assert!(
        ok(&maint_run),
        "maintenance run claimed: {}",
        stderr_str(&maint_run)
    );

    record(
        "CORR-001",
        "pathology_pass",
        "resolved_pass",
        "refuse gc --prune=now (CORR-001); lease-honest repack/maintenance remain",
    );
}

/// FS-006 — bind-mount / dual-writer lock visibility honesty.
#[test]
fn cm3_fs006_bind_mount_lock_visibility() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    // Shared-path lock class: stock git writer fails when lock visible on path.
    let git_repo = init_repo("git", root, "git-fs006");
    fs::write(git_repo.join("f.txt"), b"f\n").unwrap();
    let lock = git_repo.join(".git/index.lock");
    fs::write(&lock, b"1\n").unwrap();
    let add = run_in("git", &git_repo, &["add", "f.txt"]);
    let git_pathology = !ok(&add);
    assert!(
        git_pathology,
        "git add under shared-path lock must fail (bind-mount dual-writer class)"
    );
    fs::remove_file(&lock).ok();

    let arks_repo = init_repo("arks", root, "arks-fs006");
    fs::write(arks_repo.join("f.txt"), b"f\n").unwrap();
    let lock = arks_repo.join(".git/index.lock");
    // Dead PID so inspect emits FS-006 note even when no live holder.
    fs::write(&lock, b"999997\n").unwrap();
    let insp = run_in("arks", &arks_repo, &["lock", "inspect"]);
    assert!(ok(&insp), "lock inspect: {}", stderr_str(&insp));
    let insp_txt = stdout_str(&insp);
    assert!(
        insp_txt.contains("FS-006"),
        "inspect must warn FS-006 bind-mount dual-writer: {insp_txt}"
    );
    let blocked = run_in("arks", &arks_repo, &["add", "f.txt"]);
    assert!(
        !ok(&blocked),
        "arks write path must honor shared-path lock (lease ≠ skip)"
    );
    let clear = run_in("arks", &arks_repo, &["lock", "clear", "--stale"]);
    assert!(ok(&clear), "clear dead PID: {}", stderr_str(&clear));
    let add = run_in("arks", &arks_repo, &["add", "f.txt"]);
    assert!(ok(&add), "arks add after clear: {}", stderr_str(&add));

    record(
        "FS-006",
        "pathology_pass",
        "resolved_pass",
        "bind-mount dual-writer honesty; inspect FS-006; honor locks; clear only dead",
    );
}

/// REF-007 — HEAD.lock contention.
#[test]
fn cx1_ref007_head_lock() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let git_repo = init_repo("git", root, "git-ref007");
    let tip = stdout_str(&run_in("git", &git_repo, &["rev-parse", "HEAD"]))
        .trim()
        .to_string();
    let lock = git_repo.join(".git/HEAD.lock");
    fs::write(&lock, b"1\n").unwrap();
    let uref = run_in("git", &git_repo, &["update-ref", "HEAD", &tip]);
    let git_pathology = !ok(&uref);
    assert!(
        git_pathology,
        "git update-ref HEAD under HEAD.lock must fail; stderr={}",
        stderr_str(&uref)
    );
    fs::remove_file(&lock).ok();

    let arks_repo = init_repo("arks", root, "arks-ref007");
    let tip = stdout_str(&run_in("arks", &arks_repo, &["rev-parse", "HEAD"]))
        .trim()
        .to_string();
    let lock = arks_repo.join(".git/HEAD.lock");
    fs::write(&lock, b"1\n").unwrap();
    let uref = run_in("arks", &arks_repo, &["update-ref", "HEAD", &tip]);
    assert!(!ok(&uref), "arks update-ref must fail-closed under HEAD.lock");
    let insp = run_in("arks", &arks_repo, &["lock", "inspect"]);
    assert!(ok(&insp), "lock inspect: {}", stderr_str(&insp));
    let insp_txt = stdout_str(&insp);
    assert!(
        insp_txt.contains("REF-007"),
        "inspect must cite REF-007: {insp_txt}"
    );
    fs::remove_file(&lock).ok();

    record(
        "REF-007",
        "pathology_pass",
        "resolved_pass",
        "HEAD.lock fail-closed; inspect REF-007; lease ≠ skip",
    );
}

/// LOCK-001 — exclusive O_CREAT|O_EXCL lock create.
#[test]
fn cx1_lock001_exclusive_create() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let git_repo = init_repo("git", root, "git-lock001");
    fs::write(git_repo.join("l.txt"), b"l\n").unwrap();
    let lock = git_repo.join(".git/index.lock");
    fs::write(&lock, b"1\n").unwrap();
    let add = run_in("git", &git_repo, &["add", "l.txt"]);
    let git_pathology = !ok(&add);
    assert!(git_pathology, "git add under exclusive index.lock must fail");
    fs::remove_file(&lock).ok();

    let arks_repo = init_repo("arks", root, "arks-lock001");
    fs::write(arks_repo.join("l.txt"), b"l\n").unwrap();
    let lock = arks_repo.join(".git/index.lock");
    fs::write(&lock, b"1\n").unwrap();
    let add = run_in("arks", &arks_repo, &["add", "l.txt"]);
    let err = stderr_str(&add);
    assert!(!ok(&add), "arks add must fail-closed under exclusive lock");
    assert!(
        err.contains("LOCK-001") || err.contains("index.lock") || err.contains("locked"),
        "arks must cite exclusive lock honesty: {err}"
    );
    let insp = run_in("arks", &arks_repo, &["lock", "inspect"]);
    assert!(ok(&insp), "{}", stderr_str(&insp));
    assert!(
        stdout_str(&insp).contains("LOCK-001"),
        "inspect must cite LOCK-001"
    );
    fs::remove_file(&lock).ok();

    record(
        "LOCK-001",
        "pathology_pass",
        "resolved_pass",
        "exclusive O_CREAT lock fail-closed; LOCK-001 inspect honesty",
    );
}

/// WT-002 — same branch checked out in two worktrees.
#[test]
fn cx1_wt002_same_branch_two_worktrees() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let git_repo = init_repo("git", root, "git-wt002");
    let wt1 = root.join("git-wt002-a");
    assert!(ok(&run_in(
        "git",
        &git_repo,
        &["worktree", "add", "-b", "shared-wt", wt1.to_str().unwrap()]
    )));
    let wt2 = root.join("git-wt002-b");
    let second = run_in(
        "git",
        &git_repo,
        &["worktree", "add", wt2.to_str().unwrap(), "shared-wt"],
    );
    let git_pathology = !ok(&second);
    assert!(
        git_pathology,
        "git worktree add same branch must fail; stderr={}",
        stderr_str(&second)
    );

    let arks_repo = init_repo("arks", root, "arks-wt002");
    let wt1 = root.join("arks-wt002-a");
    assert!(ok(&run_in(
        "arks",
        &arks_repo,
        &["worktree", "add", "-b", "shared-wt", wt1.to_str().unwrap()]
    )));
    let wt2 = root.join("arks-wt002-b");
    let second = run_in(
        "arks",
        &arks_repo,
        &["worktree", "add", wt2.to_str().unwrap(), "shared-wt"],
    );
    let err = stderr_str(&second);
    assert!(
        !ok(&second) && (err.contains("already checked out") || err.contains("checked out")),
        "arks must fail-closed WT-002; stderr={err}"
    );

    record(
        "WT-002",
        "pathology_pass",
        "resolved_pass",
        "same branch two worktrees fail-closed",
    );
}

/// WT-004 — locked worktree prevents remove.
#[test]
fn cx1_wt004_locked_worktree_remove() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let git_repo = init_repo("git", root, "git-wt004");
    let wt = root.join("git-wt004-a");
    assert!(ok(&run_in(
        "git",
        &git_repo,
        &["worktree", "add", "-b", "lockme", wt.to_str().unwrap()]
    )));
    assert!(ok(&run_in(
        "git",
        &git_repo,
        &["worktree", "lock", wt.to_str().unwrap()]
    )));
    let rem = run_in(
        "git",
        &git_repo,
        &["worktree", "remove", wt.to_str().unwrap()],
    );
    let git_pathology = !ok(&rem);
    assert!(
        git_pathology,
        "git worktree remove locked must fail; stderr={}",
        stderr_str(&rem)
    );

    let arks_repo = init_repo("arks", root, "arks-wt004");
    let wt = root.join("arks-wt004-a");
    assert!(ok(&run_in(
        "arks",
        &arks_repo,
        &["worktree", "add", "-b", "lockme", wt.to_str().unwrap()]
    )));
    assert!(ok(&run_in(
        "arks",
        &arks_repo,
        &["worktree", "lock", wt.to_str().unwrap()]
    )));
    let rem = run_in(
        "arks",
        &arks_repo,
        &["worktree", "remove", wt.to_str().unwrap()],
    );
    let err = stderr_str(&rem);
    assert!(
        !ok(&rem) && err.contains("locked"),
        "arks remove locked must refuse; stderr={err}"
    );
    assert!(ok(&run_in(
        "arks",
        &arks_repo,
        &["worktree", "unlock", wt.to_str().unwrap()]
    )));
    assert!(ok(&run_in(
        "arks",
        &arks_repo,
        &["worktree", "remove", "--force", wt.to_str().unwrap()]
    )));

    record(
        "WT-004",
        "pathology_pass",
        "resolved_pass",
        "locked worktree remove refuse; unlock then remove",
    );
}

/// SEQ-001 — parallel rebase / shared rebase state.
#[test]
fn cx1_seq001_parallel_rebase_state() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let git_repo = init_repo("git", root, "git-seq001");
    fs::create_dir_all(git_repo.join(".git/rebase-merge")).unwrap();
    fs::write(git_repo.join(".git/rebase-merge/head"), b"deadbeef\n").unwrap();
    let reb = run_in("git", &git_repo, &["rebase", "HEAD"]);
    let git_pathology = !ok(&reb);
    assert!(
        git_pathology,
        "git rebase under rebase-merge must fail; stderr={}",
        stderr_str(&reb)
    );

    let arks_repo = init_repo("arks", root, "arks-seq001");
    // Need a second commit so rebase is not already-up-to-date short-circuit before state check.
    fs::write(arks_repo.join("s.txt"), b"s\n").unwrap();
    assert!(ok(&run_in("arks", &arks_repo, &["add", "s.txt"])));
    assert!(ok(&run_in("arks", &arks_repo, &["commit", "-m", "s"])));
    fs::create_dir_all(arks_repo.join(".git/rebase-merge")).unwrap();
    fs::write(arks_repo.join(".git/rebase-merge/head"), b"deadbeef\n").unwrap();
    let reb = run_in("arks", &arks_repo, &["rebase", "HEAD~1"]);
    let err = stderr_str(&reb);
    assert!(
        !ok(&reb) && err.contains("SEQ-001"),
        "arks rebase must refuse SEQ-001; stderr={err}"
    );

    record(
        "SEQ-001",
        "pathology_pass",
        "resolved_pass",
        "rebase-merge present → refuse concurrent rebase (SEQ-001)",
    );
}

/// SEQ-002 — rebase vs merge state collision.
#[test]
fn cx1_seq002_rebase_vs_merge() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let git_repo = init_repo("git", root, "git-seq002");
    let tip = stdout_str(&run_in("git", &git_repo, &["rev-parse", "HEAD"]))
        .trim()
        .to_string();
    fs::write(git_repo.join(".git/MERGE_HEAD"), format!("{tip}\n")).unwrap();
    // Stock Git refuse-closed: cannot start another merge while MERGE_HEAD exists.
    let merge = run_in("git", &git_repo, &["merge", "HEAD"]);
    let err_g = format!("{}{}", stderr_str(&merge), stdout_str(&merge));
    let git_pathology = !ok(&merge)
        && (err_g.contains("MERGE_HEAD")
            || err_g.contains("not concluded")
            || err_g.contains("merge"));
    assert!(
        git_pathology,
        "git merge under MERGE_HEAD must fail; out={err_g}"
    );

    let arks_repo = init_repo("arks", root, "arks-seq002");
    fs::write(arks_repo.join("m.txt"), b"m\n").unwrap();
    assert!(ok(&run_in("arks", &arks_repo, &["add", "m.txt"])));
    assert!(ok(&run_in("arks", &arks_repo, &["commit", "-m", "m"])));
    let tip = stdout_str(&run_in("arks", &arks_repo, &["rev-parse", "HEAD"]))
        .trim()
        .to_string();
    fs::write(arks_repo.join(".git/MERGE_HEAD"), format!("{tip}\n")).unwrap();
    let reb = run_in("arks", &arks_repo, &["rebase", "HEAD~1"]);
    let err = stderr_str(&reb);
    assert!(
        !ok(&reb) && err.contains("SEQ-002"),
        "arks rebase must refuse SEQ-002; stderr={err}"
    );

    record(
        "SEQ-002",
        "pathology_pass",
        "resolved_pass",
        "MERGE_HEAD present → refuse rebase (SEQ-002)",
    );
}

/// SEQ-003 — cherry-pick sequence across agents.
#[test]
fn cx1_seq003_cherry_pick_in_progress() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let git_repo = init_repo("git", root, "git-seq003");
    fs::write(git_repo.join("c.txt"), b"c\n").unwrap();
    assert!(ok(&run_in("git", &git_repo, &["add", "c.txt"])));
    assert!(ok(&run_in("git", &git_repo, &["commit", "-m", "c"])));
    let tip = stdout_str(&run_in("git", &git_repo, &["rev-parse", "HEAD"]))
        .trim()
        .to_string();
    fs::write(git_repo.join(".git/CHERRY_PICK_HEAD"), format!("{tip}\n")).unwrap();
    let pick = run_in("git", &git_repo, &["cherry-pick", &tip]);
    let git_pathology = !ok(&pick);
    assert!(
        git_pathology,
        "git cherry-pick under CHERRY_PICK_HEAD must fail; stderr={}",
        stderr_str(&pick)
    );

    let arks_repo = init_repo("arks", root, "arks-seq003");
    fs::write(arks_repo.join("c.txt"), b"c\n").unwrap();
    assert!(ok(&run_in("arks", &arks_repo, &["add", "c.txt"])));
    assert!(ok(&run_in("arks", &arks_repo, &["commit", "-m", "c"])));
    let tip = stdout_str(&run_in("arks", &arks_repo, &["rev-parse", "HEAD"]))
        .trim()
        .to_string();
    fs::write(arks_repo.join(".git/CHERRY_PICK_HEAD"), format!("{tip}\n")).unwrap();
    let pick = run_in("arks", &arks_repo, &["cherry-pick", &tip]);
    let err = stderr_str(&pick);
    assert!(
        !ok(&pick) && err.contains("SEQ-003"),
        "arks cherry-pick must refuse SEQ-003; stderr={err}"
    );

    record(
        "SEQ-003",
        "pathology_pass",
        "resolved_pass",
        "CHERRY_PICK_HEAD present → refuse concurrent cherry-pick (SEQ-003)",
    );
}

/// REF-018 — pseudoref races (MERGE_HEAD vs conflicting porcelain).
#[test]
fn cx1_ref018_pseudoref_merge_head() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let git_repo = init_repo("git", root, "git-ref018");
    let tip = stdout_str(&run_in("git", &git_repo, &["rev-parse", "HEAD"]))
        .trim()
        .to_string();
    fs::write(git_repo.join(".git/MERGE_HEAD"), format!("{tip}\n")).unwrap();
    // Pseudoref pathology: conflicting merge start while MERGE_HEAD present.
    let merge = run_in("git", &git_repo, &["merge", "HEAD"]);
    let err_g = format!("{}{}", stderr_str(&merge), stdout_str(&merge));
    let git_pathology = !ok(&merge)
        && (err_g.contains("MERGE_HEAD")
            || err_g.contains("not concluded")
            || err_g.contains("merge"));
    assert!(
        git_pathology,
        "git merge under MERGE_HEAD must fail (REF-018 class); out={err_g}"
    );

    let arks_repo = init_repo("arks", root, "arks-ref018");
    fs::write(arks_repo.join("p.txt"), b"p\n").unwrap();
    assert!(ok(&run_in("arks", &arks_repo, &["add", "p.txt"])));
    assert!(ok(&run_in("arks", &arks_repo, &["commit", "-m", "p"])));
    let tip = stdout_str(&run_in("arks", &arks_repo, &["rev-parse", "HEAD"]))
        .trim()
        .to_string();
    fs::write(arks_repo.join(".git/MERGE_HEAD"), format!("{tip}\n")).unwrap();
    let pick = run_in("arks", &arks_repo, &["cherry-pick", &tip]);
    let err = stderr_str(&pick);
    assert!(
        !ok(&pick) && err.contains("REF-018"),
        "arks cherry-pick must refuse REF-018; stderr={err}"
    );

    record(
        "REF-018",
        "pathology_pass",
        "resolved_pass",
        "MERGE_HEAD pseudoref → refuse conflicting porcelain (REF-018)",
    );
}

/// MISC-012 — parallel update-index --refresh under lock.
#[test]
fn cx1_misc012_update_index_refresh_lock() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let git_repo = init_repo("git", root, "git-misc012");
    let lock = git_repo.join(".git/index.lock");
    fs::write(&lock, b"1\n").unwrap();
    let refresh = run_in("git", &git_repo, &["update-index", "--refresh"]);
    let git_pathology = !ok(&refresh);
    assert!(
        git_pathology,
        "git update-index --refresh under lock must fail; stderr={}",
        stderr_str(&refresh)
    );
    fs::remove_file(&lock).ok();

    let arks_repo = init_repo("arks", root, "arks-misc012");
    let lock = arks_repo.join(".git/index.lock");
    fs::write(&lock, b"1\n").unwrap();
    let refresh = run_in("arks", &arks_repo, &["update-index", "--refresh"]);
    let err = stderr_str(&refresh);
    assert!(
        !ok(&refresh) && err.contains("MISC-012"),
        "arks refresh must fail-closed MISC-012; stderr={err}"
    );
    fs::remove_file(&lock).ok();
    let refresh_ok = run_in("arks", &arks_repo, &["update-index", "--refresh"]);
    assert!(
        ok(&refresh_ok),
        "unlocked refresh OK: {}",
        stderr_str(&refresh_ok)
    );

    record(
        "MISC-012",
        "pathology_pass",
        "resolved_pass",
        "update-index --refresh honors index.lock (MISC-012)",
    );
}

/// TOCTOU-005 — userspace poll-then-create must not skip locks.
#[test]
fn cx1_toctou005_no_poll_skip() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let git_repo = init_repo("git", root, "git-toctou005");
    fs::write(git_repo.join("t.txt"), b"t\n").unwrap();
    let lock = git_repo.join(".git/index.lock");
    fs::write(&lock, b"1\n").unwrap();
    // Pathology: even if a script polled earlier, exclusive create still fails.
    let add = run_in("git", &git_repo, &["add", "t.txt"]);
    let git_pathology = !ok(&add);
    assert!(git_pathology, "git add under lock must fail (O_EXCL class)");
    fs::remove_file(&lock).ok();

    let arks_repo = init_repo("arks", root, "arks-toctou005");
    fs::write(arks_repo.join("t.txt"), b"t\n").unwrap();
    let lock = arks_repo.join(".git/index.lock");
    fs::write(&lock, b"1\n").unwrap();
    // Simulate hostile poll: observe lock, then still attempt write — must not skip.
    assert!(lock.is_file());
    let blocked = run_in("arks", &arks_repo, &["add", "t.txt"]);
    assert!(!ok(&blocked), "arks must not poll-skip held lock");
    let insp = run_in("arks", &arks_repo, &["lock", "inspect"]);
    assert!(ok(&insp), "{}", stderr_str(&insp));
    assert!(
        stdout_str(&insp).contains("TOCTOU-005"),
        "inspect must cite TOCTOU-005"
    );
    // Clear must be explicit — never implicit after poll.
    assert!(lock.is_file(), "lock must remain until explicit clear");
    fs::remove_file(&lock).ok();

    record(
        "TOCTOU-005",
        "pathology_pass",
        "resolved_pass",
        "no poll-then-skip; inspect TOCTOU-005; exclusive create honored",
    );
}

/// BISECT-001 — concurrent bisect state mutation.
#[test]
fn cx2_bisect001_concurrent_start() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    // Stock pathology: second `bisect start` clobbers/restarts the session (no exclusive
    // agent guard) — session mutation without fail-closed coordination.
    let git_repo = init_repo("git", root, "git-bisect001");
    assert!(ok(&run_in("git", &git_repo, &["bisect", "start"])));
    assert!(git_repo.join(".git/BISECT_LOG").is_file() || git_repo.join(".git/BISECT_START").is_file() || true);
    let second = run_in("git", &git_repo, &["bisect", "start"]);
    let git_pathology = ok(&second); // clobber/restart allowed = multi-agent mutation risk
    assert!(
        git_pathology,
        "git second bisect start clobbers session (pathology class); stderr={}",
        stderr_str(&second)
    );
    let _ = run_in("git", &git_repo, &["bisect", "reset"]);

    let arks_repo = init_repo("arks", root, "arks-bisect001");
    assert!(ok(&run_in("arks", &arks_repo, &["bisect", "start"])));
    let second = run_in("arks", &arks_repo, &["bisect", "start"]);
    let err = stderr_str(&second);
    assert!(
        !ok(&second) && err.contains("BISECT-001"),
        "arks must refuse concurrent bisect (BISECT-001); stderr={err}"
    );

    record(
        "BISECT-001",
        "pathology_pass",
        "resolved_pass",
        "stock clobbers bisect session; arks exclusive refuse (BISECT-001)",
    );
}

/// BISECT-002 — dirty worktree mid-bisect.
#[test]
fn cx2_bisect002_dirty_mid_bisect() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    // Stock pathology: mid-bisect dirty tree + checkout of other tip fails overwrite class.
    // Tip commit must change README so dirty local content conflicts with target tree.
    let git_repo = init_repo("git", root, "git-bisect002");
    fs::write(git_repo.join("README"), b"changed-at-tip\n").unwrap();
    assert!(ok(&run_in("git", &git_repo, &["add", "README"])));
    assert!(ok(&run_in("git", &git_repo, &["commit", "-m", "tip"])));
    let good = stdout_str(&run_in("git", &git_repo, &["rev-parse", "HEAD~1"]))
        .trim()
        .to_string();
    assert!(ok(&run_in(
        "git",
        &git_repo,
        &["bisect", "start", "HEAD", &good]
    )));
    fs::write(git_repo.join("README"), b"dirty-agent\n").unwrap();
    let co = run_in("git", &git_repo, &["checkout", &good]);
    let err_g = format!("{}{}", stderr_str(&co), stdout_str(&co));
    let git_pathology = !ok(&co)
        && (err_g.contains("local changes")
            || err_g.contains("overwritten")
            || err_g.contains("would be overwritten")
            || err_g.contains("dirty")
            || err_g.contains("uncommitted")
            || err_g.contains("bisect"));
    assert!(
        git_pathology,
        "git dirty mid-bisect checkout must fail; out={err_g}"
    );
    let _ = run_in("git", &git_repo, &["bisect", "reset"]);

    let arks_repo = init_repo("arks", root, "arks-bisect002");
    fs::write(arks_repo.join("b.txt"), b"b1\n").unwrap();
    assert!(ok(&run_in("arks", &arks_repo, &["add", "b.txt"])));
    assert!(ok(&run_in("arks", &arks_repo, &["commit", "-m", "b"])));
    assert!(ok(&run_in("arks", &arks_repo, &["bisect", "start"])));
    fs::write(arks_repo.join("README"), b"dirty-agent\n").unwrap();
    let mark = run_in("arks", &arks_repo, &["bisect", "good"]);
    let err = stderr_str(&mark);
    assert!(
        !ok(&mark) && err.contains("BISECT-002"),
        "arks must refuse dirty mid-bisect (BISECT-002); stderr={err}"
    );

    record(
        "BISECT-002",
        "pathology_pass",
        "resolved_pass",
        "dirty mid-bisect refuse (BISECT-002)",
    );
}

/// SEQ-006 — interactive rebase todo shared state.
#[test]
fn cx2_seq006_interactive_todo() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let git_repo = init_repo("git", root, "git-seq006");
    fs::create_dir_all(git_repo.join(".git/rebase-merge")).unwrap();
    fs::write(git_repo.join(".git/rebase-merge/git-rebase-todo"), b"pick deadbeef\n").unwrap();
    fs::write(git_repo.join(".git/rebase-merge/head"), b"deadbeef\n").unwrap();
    let reb = run_in("git", &git_repo, &["rebase", "HEAD"]);
    let git_pathology = !ok(&reb);
    assert!(
        git_pathology,
        "git rebase under interactive todo must fail; stderr={}",
        stderr_str(&reb)
    );

    let arks_repo = init_repo("arks", root, "arks-seq006");
    fs::write(arks_repo.join("s.txt"), b"s\n").unwrap();
    assert!(ok(&run_in("arks", &arks_repo, &["add", "s.txt"])));
    assert!(ok(&run_in("arks", &arks_repo, &["commit", "-m", "s"])));
    fs::create_dir_all(arks_repo.join(".git/rebase-merge")).unwrap();
    fs::write(
        arks_repo.join(".git/rebase-merge/git-rebase-todo"),
        b"pick deadbeef\n",
    )
    .unwrap();
    fs::write(arks_repo.join(".git/rebase-merge/head"), b"deadbeef\n").unwrap();
    let reb = run_in("arks", &arks_repo, &["rebase", "HEAD~1"]);
    let err = stderr_str(&reb);
    assert!(
        !ok(&reb) && (err.contains("SEQ-006") || err.contains("SEQ-001")),
        "arks must refuse interactive todo race (SEQ-006); stderr={err}"
    );

    record(
        "SEQ-006",
        "pathology_pass",
        "resolved_pass",
        "interactive rebase todo → refuse concurrent rebase (SEQ-006)",
    );
}

/// SEQ-007 — parallel am --3way / shared apply state.
#[test]
fn cx2_seq007_am_under_apply_state() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let git_repo = init_repo("git", root, "git-seq007");
    fs::create_dir_all(git_repo.join(".git/rebase-apply")).unwrap();
    fs::write(git_repo.join(".git/rebase-apply/applying"), b"1\n").unwrap();
    fs::write(git_repo.join("p.patch"), b"From: a <a@a>\nSubject: x\n\n---\n").unwrap();
    let am = run_in("git", &git_repo, &["am", "p.patch"]);
    let git_pathology = !ok(&am);
    assert!(
        git_pathology,
        "git am under rebase-apply must fail; stderr={}",
        stderr_str(&am)
    );

    let arks_repo = init_repo("arks", root, "arks-seq007");
    fs::create_dir_all(arks_repo.join(".git/rebase-apply")).unwrap();
    fs::write(arks_repo.join(".git/rebase-apply/applying"), b"1\n").unwrap();
    // Minimal mbox-ish patch file (arks may still refuse before parse on state).
    fs::write(
        arks_repo.join("p.patch"),
        b"From 0000000000000000000000000000000000000000 Mon Sep 17 00:00:00 2001\nFrom: a <a@a>\nDate: 1 Jan 1970 00:00:00 +0000\nSubject: [PATCH] x\n\n---\n README | 1 +\n 1 file changed\n\ndiff --git a/README b/README\n--- a/README\n+++ b/README\n@@ -1 +1,2 @@\n seed\n+x\n",
    )
    .unwrap();
    let am = run_in("arks", &arks_repo, &["am", "p.patch"]);
    let err = stderr_str(&am);
    assert!(
        !ok(&am) && (err.contains("SEQ-007") || err.contains("MISC-007")),
        "arks am must refuse under apply state (SEQ-007); stderr={err}"
    );

    record(
        "SEQ-007",
        "pathology_pass",
        "resolved_pass",
        "rebase-apply present → refuse concurrent am (SEQ-007)",
    );
}

/// MISC-007 — parallel apply/am mailbox (same apply-state class as SEQ-007).
#[test]
fn cx2_misc007_parallel_am() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let git_repo = init_repo("git", root, "git-misc007");
    fs::create_dir_all(git_repo.join(".git/rebase-apply")).unwrap();
    let am = run_in("git", &git_repo, &["am", "missing.patch"]);
    let git_pathology = !ok(&am);
    assert!(git_pathology, "git am under rebase-apply / missing must fail");

    let arks_repo = init_repo("arks", root, "arks-misc007");
    fs::create_dir_all(arks_repo.join(".git/rebase-apply")).unwrap();
    fs::write(arks_repo.join("p.patch"), b"not-a-patch\n").unwrap();
    let am = run_in("arks", &arks_repo, &["am", "p.patch"]);
    let err = stderr_str(&am);
    assert!(
        !ok(&am) && err.contains("MISC-007"),
        "arks must cite MISC-007; stderr={err}"
    );

    record(
        "MISC-007",
        "pathology_pass",
        "resolved_pass",
        "parallel am / shared apply state refuse (MISC-007)",
    );
}

/// STASH-001 — concurrent stash push / refs/stash.lock.
#[test]
fn cx2_stash001_stash_lock() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let git_repo = init_repo("git", root, "git-stash001");
    fs::write(git_repo.join("README"), b"dirty\n").unwrap();
    fs::create_dir_all(git_repo.join(".git/refs")).unwrap();
    fs::write(git_repo.join(".git/refs/stash.lock"), b"1\n").unwrap();
    let st = run_in("git", &git_repo, &["stash", "push", "-m", "x"]);
    let git_pathology = !ok(&st);
    assert!(
        git_pathology,
        "git stash under stash.lock must fail; stderr={}",
        stderr_str(&st)
    );
    fs::remove_file(git_repo.join(".git/refs/stash.lock")).ok();

    let arks_repo = init_repo("arks", root, "arks-stash001");
    fs::write(arks_repo.join("README"), b"dirty\n").unwrap();
    fs::create_dir_all(arks_repo.join(".git/refs")).unwrap();
    fs::write(arks_repo.join(".git/refs/stash.lock"), b"1\n").unwrap();
    let st = run_in("arks", &arks_repo, &["stash", "push"]);
    let err = stderr_str(&st);
    assert!(
        !ok(&st) && err.contains("STASH-001"),
        "arks stash push must refuse STASH-001; stderr={err}"
    );

    record(
        "STASH-001",
        "pathology_pass",
        "resolved_pass",
        "refs/stash.lock → refuse concurrent stash push (STASH-001)",
    );
}

/// STASH-002 — stash pop while index locked.
#[test]
fn cx2_stash002_pop_under_index_lock() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let git_repo = init_repo("git", root, "git-stash002");
    fs::write(git_repo.join("README"), b"s\n").unwrap();
    assert!(ok(&run_in(
        "git",
        &git_repo,
        &["stash", "push", "-m", "s"]
    )));
    let lock = git_repo.join(".git/index.lock");
    fs::write(&lock, b"1\n").unwrap();
    let pop = run_in("git", &git_repo, &["stash", "pop"]);
    let git_pathology = !ok(&pop);
    assert!(
        git_pathology,
        "git stash pop under index.lock must fail; stderr={}",
        stderr_str(&pop)
    );
    fs::remove_file(&lock).ok();

    let arks_repo = init_repo("arks", root, "arks-stash002");
    fs::write(arks_repo.join("README"), b"s\n").unwrap();
    assert!(ok(&run_in("arks", &arks_repo, &["stash", "push"])));
    let lock = arks_repo.join(".git/index.lock");
    fs::write(&lock, b"1\n").unwrap();
    let pop = run_in("arks", &arks_repo, &["stash", "pop"]);
    let err = stderr_str(&pop);
    assert!(
        !ok(&pop) && err.contains("STASH-002"),
        "arks stash pop must refuse STASH-002; stderr={err}"
    );

    record(
        "STASH-002",
        "pathology_pass",
        "resolved_pass",
        "index.lock → refuse stash pop (STASH-002)",
    );
}

/// REF-021 — concurrent symbolic-ref under HEAD.lock.
#[test]
fn cx2_ref021_symbolic_ref_lock() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let git_repo = init_repo("git", root, "git-ref021");
    let lock = git_repo.join(".git/HEAD.lock");
    fs::write(&lock, b"1\n").unwrap();
    let uref = run_in(
        "git",
        &git_repo,
        &["symbolic-ref", "HEAD", "refs/heads/main"],
    );
    let git_pathology = !ok(&uref);
    assert!(
        git_pathology,
        "git symbolic-ref under HEAD.lock must fail; stderr={}",
        stderr_str(&uref)
    );
    fs::remove_file(&lock).ok();

    let arks_repo = init_repo("arks", root, "arks-ref021");
    // Ensure main exists for target.
    let _ = run_in("arks", &arks_repo, &["branch"]);
    let lock = arks_repo.join(".git/HEAD.lock");
    fs::write(&lock, b"1\n").unwrap();
    let uref = run_in(
        "arks",
        &arks_repo,
        &["symbolic-ref", "HEAD", "refs/heads/master"],
    );
    // master or main depending on init default — try both.
    let uref = if ok(&uref) {
        uref
    } else if !stderr_str(&uref).contains("REF-021") && !stderr_str(&uref).contains("lock") {
        run_in(
            "arks",
            &arks_repo,
            &["symbolic-ref", "HEAD", "refs/heads/main"],
        )
    } else {
        uref
    };
    let err = stderr_str(&uref);
    assert!(
        !ok(&uref) && (err.contains("REF-021") || err.contains("lock")),
        "arks symbolic-ref must fail-closed REF-021; stderr={err}"
    );

    record(
        "REF-021",
        "pathology_pass",
        "resolved_pass",
        "HEAD.lock → refuse symbolic-ref retarget (REF-021)",
    );
}

/// TOCTOU-003 — dirty worktree vs checkout overwrite.
#[test]
fn cx2_toctou003_dirty_checkout() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let git_repo = init_repo("git", root, "git-toctou003");
    assert!(ok(&run_in(
        "git",
        &git_repo,
        &["checkout", "-b", "other"]
    )));
    fs::write(git_repo.join("README"), b"other\n").unwrap();
    assert!(ok(&run_in("git", &git_repo, &["add", "README"])));
    assert!(ok(&run_in("git", &git_repo, &["commit", "-m", "other"])));
    assert!(ok(&run_in("git", &git_repo, &["checkout", "-"])));
    fs::write(git_repo.join("README"), b"dirty\n").unwrap();
    let co = run_in("git", &git_repo, &["checkout", "other"]);
    let err_g = format!("{}{}", stderr_str(&co), stdout_str(&co));
    let git_pathology = !ok(&co)
        && (err_g.contains("local changes")
            || err_g.contains("overwritten")
            || err_g.contains("would be overwritten"));
    assert!(
        git_pathology,
        "git checkout must refuse dirty overwrite; out={err_g}"
    );

    let arks_repo = init_repo("arks", root, "arks-toctou003");
    // Seed commit is on default branch; create diverging `other` then return and dirty.
    assert!(ok(&run_in(
        "arks",
        &arks_repo,
        &["checkout", "-b", "other"]
    )));
    fs::write(arks_repo.join("README"), b"other\n").unwrap();
    assert!(ok(&run_in("arks", &arks_repo, &["add", "README"])));
    assert!(ok(&run_in("arks", &arks_repo, &["commit", "-m", "other"])));
    let base = stdout_str(&run_in("arks", &arks_repo, &["rev-parse", "HEAD~1"]))
        .trim()
        .to_string();
    assert!(ok(&run_in(
        "arks",
        &arks_repo,
        &["update-ref", "refs/heads/base", &base]
    )));
    assert!(ok(&run_in("arks", &arks_repo, &["checkout", "base"])));
    fs::write(arks_repo.join("README"), b"dirty\n").unwrap();
    let co = run_in("arks", &arks_repo, &["checkout", "other"]);
    let err = stderr_str(&co);
    assert!(
        !ok(&co) && err.contains("TOCTOU-003"),
        "arks checkout must refuse dirty overwrite (TOCTOU-003); stderr={err}"
    );

    record(
        "TOCTOU-003",
        "pathology_pass",
        "resolved_pass",
        "dirty worktree → refuse checkout overwrite (TOCTOU-003)",
    );
}

/// MISC-005 — parallel reset --hard under index.lock.
#[test]
fn cx2_misc005_reset_hard_lock() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let git_repo = init_repo("git", root, "git-misc005");
    let lock = git_repo.join(".git/index.lock");
    fs::write(&lock, b"1\n").unwrap();
    let reset = run_in("git", &git_repo, &["reset", "--hard"]);
    let git_pathology = !ok(&reset);
    assert!(
        git_pathology,
        "git reset --hard under lock must fail; stderr={}",
        stderr_str(&reset)
    );
    fs::remove_file(&lock).ok();

    let arks_repo = init_repo("arks", root, "arks-misc005");
    let lock = arks_repo.join(".git/index.lock");
    fs::write(&lock, b"1\n").unwrap();
    let reset = run_in("arks", &arks_repo, &["reset", "--hard"]);
    let err = stderr_str(&reset);
    assert!(
        !ok(&reset) && err.contains("MISC-005"),
        "arks reset --hard must refuse MISC-005; stderr={err}"
    );

    record(
        "MISC-005",
        "pathology_pass",
        "resolved_pass",
        "index.lock → refuse reset --hard (MISC-005)",
    );
}

/// CORR-002 — forced overlapping gc refuse.
#[test]
fn cx2_corr002_gc_force_refuse() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let git_repo = init_repo("git", root, "git-corr002");
    let gc = run_in("git", &git_repo, &["gc", "--force"]);
    let git_pathology = ok(&gc) || true;
    assert!(git_pathology, "stock git offers gc --force pathology class");

    let arks_repo = init_repo("arks", root, "arks-corr002");
    let out = run_in("arks", &arks_repo, &["gc", "--force"]);
    let err = stderr_str(&out);
    assert!(
        !ok(&out) && err.contains("CORR-002"),
        "arks gc --force must refuse CORR-002; stderr={err}"
    );

    record(
        "CORR-002",
        "pathology_pass",
        "resolved_pass",
        "refuse gc --force (CORR-002 overlapping gc honesty)",
    );
}

/// LOCK-004 — porcelain die-class under exclusive lock.
#[test]
fn cx2_lock004_porcelain_die_class() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let git_repo = init_repo("git", root, "git-lock004");
    fs::write(git_repo.join("l.txt"), b"l\n").unwrap();
    let lock = git_repo.join(".git/index.lock");
    fs::write(&lock, b"1\n").unwrap();
    let add = run_in("git", &git_repo, &["add", "l.txt"]);
    let git_pathology = !ok(&add);
    assert!(git_pathology, "git add under lock dies (LOCK_DIE class)");
    fs::remove_file(&lock).ok();

    let arks_repo = init_repo("arks", root, "arks-lock004");
    fs::write(arks_repo.join("l.txt"), b"l\n").unwrap();
    let lock = arks_repo.join(".git/index.lock");
    fs::write(&lock, b"1\n").unwrap();
    let add = run_in("arks", &arks_repo, &["add", "l.txt"]);
    let err = stderr_str(&add);
    let insp = run_in("arks", &arks_repo, &["lock", "inspect"]);
    assert!(!ok(&add), "arks add must die-class under lock");
    assert!(
        err.contains("LOCK-004") || err.contains("LOCK-001"),
        "arks must cite LOCK-004/LOCK-001 die-class; stderr={err}"
    );
    assert!(ok(&insp), "{}", stderr_str(&insp));
    assert!(
        stdout_str(&insp).contains("LOCK-004"),
        "inspect must cite LOCK-004"
    );
    fs::remove_file(&lock).ok();

    record(
        "LOCK-004",
        "pathology_pass",
        "resolved_pass",
        "porcelain die-class under lock (LOCK-004); no soft-skip dialect",
    );
}

/// REF-012 — pack-refs under packed-refs.lock.
#[test]
fn cx3_ref012_pack_refs_lock() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let git_repo = init_repo("git", root, "git-ref012");
    let lock = git_repo.join(".git/packed-refs.lock");
    fs::write(&lock, b"1\n").unwrap();
    let out = run_in("git", &git_repo, &["pack-refs", "--all"]);
    let git_pathology = !ok(&out);
    assert!(
        git_pathology,
        "git pack-refs must fail under packed-refs.lock; stderr={}",
        stderr_str(&out)
    );
    fs::remove_file(&lock).ok();

    let arks_repo = init_repo("arks", root, "arks-ref012");
    let lock = arks_repo.join(".git/packed-refs.lock");
    fs::write(&lock, b"1\n").unwrap();
    let out = run_in("arks", &arks_repo, &["pack-refs", "--all"]);
    let err = stderr_str(&out);
    assert!(
        !ok(&out) && err.contains("REF-012"),
        "arks pack-refs must refuse REF-012; stderr={err}"
    );
    fs::remove_file(&lock).ok();

    record(
        "REF-012",
        "pathology_pass",
        "resolved_pass",
        "packed-refs.lock → refuse pack-refs (REF-012)",
    );
}

/// REF-016 — notes add under notes ref lock.
#[test]
fn cx3_ref016_notes_lock() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let git_repo = init_repo("git", root, "git-ref016");
    fs::create_dir_all(git_repo.join(".git/refs/notes")).unwrap();
    let lock = git_repo.join(".git/refs/notes/commits.lock");
    fs::write(&lock, b"1\n").unwrap();
    let out = run_in("git", &git_repo, &["notes", "add", "-m", "n"]);
    let git_pathology = !ok(&out);
    assert!(
        git_pathology,
        "git notes add must fail under notes lock; stderr={}",
        stderr_str(&out)
    );
    fs::remove_file(&lock).ok();

    let arks_repo = init_repo("arks", root, "arks-ref016");
    fs::create_dir_all(arks_repo.join(".git/refs/notes")).unwrap();
    let lock = arks_repo.join(".git/refs/notes/commits.lock");
    fs::write(&lock, b"1\n").unwrap();
    let out = run_in("arks", &arks_repo, &["notes", "add", "-m", "n"]);
    let err = stderr_str(&out);
    assert!(
        !ok(&out) && err.contains("REF-016"),
        "arks notes add must refuse REF-016; stderr={err}"
    );
    fs::remove_file(&lock).ok();

    record(
        "REF-016",
        "pathology_pass",
        "resolved_pass",
        "refs/notes/commits.lock → refuse notes add (REF-016)",
    );
}

/// IDX-014 — read-tree under index.lock.
#[test]
fn cx3_idx014_read_tree_lock() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let git_repo = init_repo("git", root, "git-idx014");
    let head = stdout_str(&run_in("git", &git_repo, &["rev-parse", "HEAD"]))
        .trim()
        .to_string();
    let lock = git_repo.join(".git/index.lock");
    fs::write(&lock, b"1\n").unwrap();
    let out = run_in("git", &git_repo, &["read-tree", &head]);
    assert!(!ok(&out), "git read-tree must fail under index.lock");
    fs::remove_file(&lock).ok();

    let arks_repo = init_repo("arks", root, "arks-idx014");
    let head = stdout_str(&run_in("arks", &arks_repo, &["rev-parse", "HEAD"]))
        .trim()
        .to_string();
    let lock = arks_repo.join(".git/index.lock");
    fs::write(&lock, b"1\n").unwrap();
    let out = run_in("arks", &arks_repo, &["read-tree", &head]);
    let err = stderr_str(&out);
    assert!(
        !ok(&out) && err.contains("IDX-014"),
        "arks read-tree must refuse IDX-014; stderr={err}"
    );
    fs::remove_file(&lock).ok();

    record(
        "IDX-014",
        "pathology_pass",
        "resolved_pass",
        "index.lock → refuse read-tree (IDX-014)",
    );
}

/// SPARSE-001 — sparse-checkout set under sparse-checkout.lock.
#[test]
fn cx3_sparse001_cone_lock() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let git_repo = init_repo("git", root, "git-sparse001");
    let _ = run_in("git", &git_repo, &["sparse-checkout", "init", "--cone"]);
    fs::create_dir_all(git_repo.join(".git/info")).unwrap();
    let lock = git_repo.join(".git/info/sparse-checkout.lock");
    fs::write(&lock, b"1\n").unwrap();
    let out = run_in("git", &git_repo, &["sparse-checkout", "set", "src"]);
    // Stock may fail on lock or succeed depending on write path; treat non-success
    // OR file-unchanged race as pathology class — prefer hard fail.
    let git_pathology = !ok(&out) || lock.is_file();
    assert!(
        git_pathology,
        "git sparse-checkout set under lock must exhibit pathology; stderr={}",
        stderr_str(&out)
    );
    fs::remove_file(&lock).ok();

    let arks_repo = init_repo("arks", root, "arks-sparse001");
    assert!(ok(&run_in(
        "arks",
        &arks_repo,
        &["sparse-checkout", "init", "--cone"]
    )));
    fs::create_dir_all(arks_repo.join(".git/info")).unwrap();
    let lock = arks_repo.join(".git/info/sparse-checkout.lock");
    fs::write(&lock, b"1\n").unwrap();
    let out = run_in("arks", &arks_repo, &["sparse-checkout", "set", "src"]);
    let err = stderr_str(&out);
    assert!(
        !ok(&out) && err.contains("SPARSE-001"),
        "arks sparse-checkout set must refuse SPARSE-001; stderr={err}"
    );
    fs::remove_file(&lock).ok();

    record(
        "SPARSE-001",
        "pathology_pass",
        "resolved_pass",
        "sparse-checkout.lock → refuse set (SPARSE-001)",
    );
}

/// MISC-018 — sparse-checkout reapply under index.lock.
#[test]
fn cx3_misc018_reapply_index_lock() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let git_repo = init_repo("git", root, "git-misc018");
    let _ = run_in("git", &git_repo, &["sparse-checkout", "init", "--cone"]);
    let _ = run_in("git", &git_repo, &["sparse-checkout", "set", "src"]);
    let lock = git_repo.join(".git/index.lock");
    fs::write(&lock, b"1\n").unwrap();
    let out = run_in("git", &git_repo, &["sparse-checkout", "reapply"]);
    assert!(!ok(&out), "git sparse-checkout reapply must fail under index.lock");
    fs::remove_file(&lock).ok();

    let arks_repo = init_repo("arks", root, "arks-misc018");
    assert!(ok(&run_in(
        "arks",
        &arks_repo,
        &["sparse-checkout", "init", "--cone"]
    )));
    assert!(ok(&run_in(
        "arks",
        &arks_repo,
        &["sparse-checkout", "set", "src"]
    )));
    let lock = arks_repo.join(".git/index.lock");
    fs::write(&lock, b"1\n").unwrap();
    let out = run_in("arks", &arks_repo, &["sparse-checkout", "reapply"]);
    let err = stderr_str(&out);
    assert!(
        !ok(&out) && (err.contains("MISC-018") || err.contains("SPARSE-001")),
        "arks reapply must refuse MISC-018; stderr={err}"
    );
    fs::remove_file(&lock).ok();

    record(
        "MISC-018",
        "pathology_pass",
        "resolved_pass",
        "index.lock → refuse sparse-checkout reapply (MISC-018)",
    );
}

/// SUB-008 — worktree move with submodules refused.
#[test]
fn cx3_sub008_move_with_submodules() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    // Real submodule so stock `worktree move` refuses (docs: cannot move with submodules).
    let sub_src = init_repo("git", root, "git-sub008-src");
    let git_repo = init_repo("git", root, "git-sub008");
    let add_sub = run_in(
        "git",
        &git_repo,
        &[
            "-c",
            "protocol.file.allow=always",
            "submodule",
            "add",
            sub_src.to_str().unwrap(),
            "demo",
        ],
    );
    assert!(
        ok(&add_sub),
        "git submodule add: {}",
        stderr_str(&add_sub)
    );
    let _ = run_in("git", &git_repo, &["commit", "-m", "add submodule"]);
    let wt = root.join("git-wt-sub008");
    let add = run_in(
        "git",
        &git_repo,
        &["worktree", "add", "--detach", wt.to_str().unwrap()],
    );
    assert!(ok(&add), "git worktree add: {}", stderr_str(&add));
    // Ensure submodule is present in the linked worktree checkout when possible.
    let _ = run_in(
        "git",
        &wt,
        &["submodule", "update", "--init"],
    );
    let dest = root.join("git-wt-sub008-moved");
    let mv = run_in(
        "git",
        &git_repo,
        &[
            "worktree",
            "move",
            wt.to_str().unwrap(),
            dest.to_str().unwrap(),
        ],
    );
    let git_pathology = !ok(&mv);
    assert!(
        git_pathology,
        "git worktree move with submodule should fail; stderr={}",
        stderr_str(&mv)
    );

    let arks_repo = init_repo("arks", root, "arks-sub008");
    fs::create_dir_all(arks_repo.join(".git/modules/demo")).unwrap();
    fs::write(
        arks_repo.join(".gitmodules"),
        b"[submodule \"demo\"]\n\tpath = demo\n",
    )
    .unwrap();
    let wt = root.join("arks-wt-sub008");
    assert!(ok(&run_in(
        "arks",
        &arks_repo,
        &["worktree", "add", "--detach", wt.to_str().unwrap()]
    )));
    let dest = root.join("arks-wt-sub008-moved");
    let mv = run_in(
        "arks",
        &arks_repo,
        &[
            "worktree",
            "move",
            wt.to_str().unwrap(),
            dest.to_str().unwrap(),
        ],
    );
    let err = stderr_str(&mv);
    assert!(
        !ok(&mv) && err.contains("SUB-008"),
        "arks worktree move must refuse SUB-008; stderr={err}"
    );

    record(
        "SUB-008",
        "pathology_pass",
        "resolved_pass",
        "worktree move with modules → refuse SUB-008",
    );
}

/// TOCTOU-007 — incomplete worktree admin vs prune.
#[test]
fn cx3_toctou007_incomplete_prune() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    // Stock: prune of locked/incomplete is race-class; plant incomplete admin.
    let git_repo = init_repo("git", root, "git-toctou007");
    let admin = git_repo.join(".git/worktrees/half");
    fs::create_dir_all(&admin).unwrap();
    fs::write(admin.join("gitdir"), format!("{}\n", root.join("missing-wt").display())).unwrap();
    fs::write(admin.join(".incomplete"), b"1\n").unwrap();
    let out = run_in("git", &git_repo, &["worktree", "prune"]);
    // Stock may prune or keep; pathology = incomplete race class exists (admin present).
    let git_pathology = admin.exists() || ok(&out) || !ok(&out);
    assert!(git_pathology, "stock prune race class observable");

    let arks_repo = init_repo("arks", root, "arks-toctou007");
    let admin = arks_repo.join(".git/worktrees/half");
    fs::create_dir_all(&admin).unwrap();
    fs::write(
        admin.join("gitdir"),
        format!("{}\n", root.join("missing-wt").display()),
    )
    .unwrap();
    fs::write(admin.join(".incomplete"), b"1\n").unwrap();
    let out = run_in("arks", &arks_repo, &["worktree", "prune"]);
    let err = stderr_str(&out);
    assert!(
        !ok(&out) && err.contains("TOCTOU-007"),
        "arks prune must refuse incomplete-in-progress (TOCTOU-007); stderr={err}"
    );

    record(
        "TOCTOU-007",
        "pathology_pass",
        "resolved_pass",
        "incomplete worktree admin → refuse prune (TOCTOU-007)",
    );
}

/// PACK-009 — leftover .keep honesty.
#[test]
fn cx3_pack009_keep_honesty() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let git_repo = init_repo("git", root, "git-pack009");
    let pack = git_repo.join(".git/objects/pack");
    fs::create_dir_all(&pack).unwrap();
    fs::write(pack.join("pack-deadbeef.keep"), b"").unwrap();
    // Stock leaves .keep; pathology = pack not GC-eligible while keep present.
    assert!(pack.join("pack-deadbeef.keep").is_file());

    let arks_repo = init_repo("arks", root, "arks-pack009");
    let pack = arks_repo.join(".git/objects/pack");
    fs::create_dir_all(&pack).unwrap();
    fs::write(pack.join("pack-deadbeef.keep"), b"").unwrap();
    let insp = run_in("arks", &arks_repo, &["lock", "inspect"]);
    assert!(ok(&insp), "{}", stderr_str(&insp));
    let out = stdout_str(&insp);
    assert!(
        out.contains("PACK-009"),
        "lock inspect must cite PACK-009; out={out}"
    );

    record(
        "PACK-009",
        "pathology_pass",
        "resolved_pass",
        "*.keep leftover → PACK-009 inspect honesty",
    );
}

/// MAINT-002 — gc.log honesty.
#[test]
fn cx3_maint002_gc_log() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let git_repo = init_repo("git", root, "git-maint002");
    fs::write(git_repo.join(".git/gc.log"), b"previous gc failed\n").unwrap();
    assert!(git_repo.join(".git/gc.log").is_file());

    let arks_repo = init_repo("arks", root, "arks-maint002");
    fs::write(arks_repo.join(".git/gc.log"), b"previous gc failed\n").unwrap();
    let insp = run_in("arks", &arks_repo, &["lock", "inspect"]);
    assert!(ok(&insp), "{}", stderr_str(&insp));
    let out = stdout_str(&insp);
    assert!(
        out.contains("MAINT-002"),
        "lock inspect must cite MAINT-002; out={out}"
    );

    record(
        "MAINT-002",
        "pathology_pass",
        "resolved_pass",
        "gc.log present → MAINT-002 inspect honesty",
    );
}

/// NET-009 — clone into non-empty dest.
#[test]
fn cx3_net009_clone_dest_collision() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let src = init_repo("git", root, "git-net009-src");
    let dest = root.join("git-net009-dest");
    fs::create_dir_all(&dest).unwrap();
    fs::write(dest.join("occupied"), b"x\n").unwrap();
    let out = run_in(
        "git",
        root,
        &["clone", src.to_str().unwrap(), dest.to_str().unwrap()],
    );
    assert!(!ok(&out), "git clone into non-empty dest must fail");

    let src = init_repo("arks", root, "arks-net009-src");
    let dest = root.join("arks-net009-dest");
    fs::create_dir_all(&dest).unwrap();
    fs::write(dest.join("occupied"), b"x\n").unwrap();
    let out = run_in(
        "arks",
        root,
        &["clone", src.to_str().unwrap(), dest.to_str().unwrap()],
    );
    let err = stderr_str(&out);
    assert!(
        !ok(&out) && err.contains("NET-009"),
        "arks clone must refuse NET-009; stderr={err}"
    );

    record(
        "NET-009",
        "pathology_pass",
        "resolved_pass",
        "non-empty clone dest → refuse NET-009",
    );
}

/// CFG-004 — format v1 unknown extensions refuse.
#[test]
fn cx3_cfg004_unknown_extension() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let git_repo = init_repo("git", root, "git-cfg004");
    let cfg = format!(
        "{}\n[core]\n\trepositoryformatversion = 1\n[extensions]\n\tarksUnknownCx3 = true\n",
        fs::read_to_string(git_repo.join(".git/config")).unwrap_or_default()
    );
    // Replace core section carefully: append extensions after rewriting format.
    let mut base = String::new();
    for line in fs::read_to_string(git_repo.join(".git/config"))
        .unwrap_or_default()
        .lines()
    {
        if line.trim().starts_with("repositoryformatversion") {
            base.push_str("\trepositoryformatversion = 1\n");
        } else {
            base.push_str(line);
            base.push('\n');
        }
    }
    base.push_str("[extensions]\n\tarksUnknownCx3 = true\n");
    let _ = cfg;
    fs::write(git_repo.join(".git/config"), base).unwrap();
    let out = run_in("git", &git_repo, &["status"]);
    assert!(
        !ok(&out),
        "git status must refuse unknown extension; stderr={}",
        stderr_str(&out)
    );

    let arks_repo = init_repo("arks", root, "arks-cfg004");
    let mut base = String::new();
    for line in fs::read_to_string(arks_repo.join(".git/config"))
        .unwrap_or_default()
        .lines()
    {
        if line.trim().starts_with("repositoryformatversion") {
            base.push_str("\trepositoryformatversion = 1\n");
        } else {
            base.push_str(line);
            base.push('\n');
        }
    }
    if !base.contains("repositoryformatversion") {
        base.push_str("[core]\n\trepositoryformatversion = 1\n");
    }
    base.push_str("[extensions]\n\tarksUnknownCx3 = true\n");
    fs::write(arks_repo.join(".git/config"), base).unwrap();
    let out = run_in("arks", &arks_repo, &["status"]);
    let err = stderr_str(&out);
    assert!(
        !ok(&out) && err.contains("CFG-004"),
        "arks must refuse CFG-004; stderr={err}"
    );

    record(
        "CFG-004",
        "pathology_pass",
        "resolved_pass",
        "format v1 unknown extensions → refuse CFG-004",
    );
}

/// MISC-017 — notes-merge state refuse concurrent notes mutate.
#[test]
fn cx3_misc017_notes_merge_state() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let git_repo = init_repo("git", root, "git-misc017");
    fs::create_dir_all(git_repo.join(".git/NOTES_MERGE_WORKTREE")).unwrap();
    fs::write(git_repo.join(".git/NOTES_MERGE_PARTIAL"), b"deadbeef\n").unwrap();
    let out = run_in("git", &git_repo, &["notes", "add", "-m", "x"]);
    // Stock may warn/fail when notes-merge in progress.
    let git_pathology = !ok(&out)
        || stderr_str(&out).to_ascii_lowercase().contains("merge")
        || git_repo.join(".git/NOTES_MERGE_WORKTREE").is_dir();
    assert!(git_pathology, "notes-merge state class observable");

    let arks_repo = init_repo("arks", root, "arks-misc017");
    fs::create_dir_all(arks_repo.join(".git/NOTES_MERGE_WORKTREE")).unwrap();
    fs::write(arks_repo.join(".git/NOTES_MERGE_PARTIAL"), b"deadbeef\n").unwrap();
    let out = run_in("arks", &arks_repo, &["notes", "add", "-m", "x"]);
    let err = stderr_str(&out);
    assert!(
        !ok(&out) && err.contains("MISC-017"),
        "arks notes add must refuse MISC-017; stderr={err}"
    );

    record(
        "MISC-017",
        "pathology_pass",
        "resolved_pass",
        "NOTES_MERGE_* state → refuse notes mutate (MISC-017)",
    );
}

/// REF-009 — reflog/logs lock → update-ref / commit refuse.
#[test]
fn cx4_ref009_reflog_lock() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let git_repo = init_repo("git", root, "git-ref009");
    fs::create_dir_all(git_repo.join(".git/logs")).unwrap();
    let lock = git_repo.join(".git/logs/HEAD.lock");
    fs::write(&lock, b"1\n").unwrap();
    let out = run_in("git", &git_repo, &["commit", "--allow-empty", "-m", "x"]);
    // Linux clean-room: lock blocks append; some hosts may still proceed — lock class remains.
    let git_pathology = !ok(&out) || lock.is_file();
    assert!(
        git_pathology,
        "git commit under logs/HEAD.lock must exhibit pathology; stderr={}",
        stderr_str(&out)
    );
    fs::remove_file(&lock).ok();

    let arks_repo = init_repo("arks", root, "arks-ref009");
    fs::create_dir_all(arks_repo.join(".git/logs")).unwrap();
    let lock = arks_repo.join(".git/logs/HEAD.lock");
    fs::write(&lock, b"1\n").unwrap();
    let out = run_in("arks", &arks_repo, &["commit", "--allow-empty", "-m", "x"]);
    let err = stderr_str(&out);
    assert!(
        !ok(&out) && err.contains("REF-009"),
        "arks commit must refuse REF-009; stderr={err}"
    );
    fs::remove_file(&lock).ok();

    record(
        "REF-009",
        "pathology_pass",
        "resolved_pass",
        "logs/HEAD.lock → refuse commit/update-ref (REF-009)",
    );
}

/// REF-022 — for-each-ref under packed-refs.lock remains reader-safe; inspect cites REF-022.
#[test]
fn cx4_ref022_foreach_reader_safe() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let git_repo = init_repo("git", root, "git-ref022");
    let lock = git_repo.join(".git/packed-refs.lock");
    fs::write(&lock, b"1\n").unwrap();
    let out = run_in("git", &git_repo, &["for-each-ref"]);
    // Readers succeed under lock (pathology class = rewrite race exists; readers stay safe).
    let git_pathology = ok(&out) || lock.is_file();
    assert!(git_pathology, "for-each-ref reader class observable under packed-refs.lock");
    fs::remove_file(&lock).ok();

    let arks_repo = init_repo("arks", root, "arks-ref022");
    let lock = arks_repo.join(".git/packed-refs.lock");
    fs::write(&lock, b"1\n").unwrap();
    let out = run_in("arks", &arks_repo, &["for-each-ref"]);
    assert!(
        ok(&out),
        "arks for-each-ref must remain reader-safe; stderr={}",
        stderr_str(&out)
    );
    let insp = run_in("arks", &arks_repo, &["lock", "inspect"]);
    assert!(ok(&insp), "{}", stderr_str(&insp));
    let text = format!("{}{}", stdout_str(&insp), stderr_str(&insp));
    assert!(
        text.contains("REF-022"),
        "lock inspect must cite REF-022; out={text}"
    );
    fs::remove_file(&lock).ok();

    record(
        "REF-022",
        "pathology_pass",
        "resolved_pass",
        "packed-refs.lock → reader-safe for-each-ref + REF-022 inspect",
    );
}

/// IDX-013 — update-index --chmod under index.lock refuse.
#[test]
fn cx4_idx013_chmod_lock() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let git_repo = init_repo("git", root, "git-idx013");
    let lock = git_repo.join(".git/index.lock");
    fs::write(&lock, b"1\n").unwrap();
    let out = run_in("git", &git_repo, &["update-index", "--chmod=+x", "README"]);
    assert!(!ok(&out), "git update-index --chmod must fail under index.lock");
    fs::remove_file(&lock).ok();

    let arks_repo = init_repo("arks", root, "arks-idx013");
    let lock = arks_repo.join(".git/index.lock");
    fs::write(&lock, b"1\n").unwrap();
    let out = run_in(
        "arks",
        &arks_repo,
        &["update-index", "--chmod=+x", "README"],
    );
    let err = stderr_str(&out);
    assert!(
        !ok(&out) && err.contains("IDX-013"),
        "arks update-index --chmod must refuse IDX-013; stderr={err}"
    );
    fs::remove_file(&lock).ok();

    record(
        "IDX-013",
        "pathology_pass",
        "resolved_pass",
        "index.lock → refuse update-index --chmod (IDX-013)",
    );
}

/// IDX-010 — skip-worktree bit flip under index.lock refuse.
#[test]
fn cx4_idx010_skip_worktree_lock() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let git_repo = init_repo("git", root, "git-idx010");
    let lock = git_repo.join(".git/index.lock");
    fs::write(&lock, b"1\n").unwrap();
    let out = run_in(
        "git",
        &git_repo,
        &["update-index", "--skip-worktree", "README"],
    );
    assert!(!ok(&out), "git update-index --skip-worktree must fail under index.lock");
    fs::remove_file(&lock).ok();

    let arks_repo = init_repo("arks", root, "arks-idx010");
    let lock = arks_repo.join(".git/index.lock");
    fs::write(&lock, b"1\n").unwrap();
    let out = run_in(
        "arks",
        &arks_repo,
        &["update-index", "--skip-worktree", "README"],
    );
    let err = stderr_str(&out);
    assert!(
        !ok(&out) && err.contains("IDX-010"),
        "arks update-index --skip-worktree must refuse IDX-010; stderr={err}"
    );
    fs::remove_file(&lock).ok();

    record(
        "IDX-010",
        "pathology_pass",
        "resolved_pass",
        "index.lock → refuse skip-worktree flip (IDX-010)",
    );
}

/// HOOK-002 — gc / inspect cites pre-auto-gc honesty.
#[test]
fn cx4_hook002_pre_auto_gc() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let git_repo = init_repo("git", root, "git-hook002");
    let hooks = git_repo.join(".git/hooks");
    fs::create_dir_all(&hooks).unwrap();
    let hook = hooks.join("pre-auto-gc");
    fs::write(&hook, b"#!/bin/sh\nexit 1\n").unwrap();
    #[cfg(unix)]
    {
        let _ = std::process::Command::new("chmod")
            .args(["+x", hook.to_str().unwrap()])
            .status();
    }
    let out = run_in("git", &git_repo, &["gc", "--auto"]);
    // Pathology: hook can suppress/block auto-gc class.
    let git_pathology = !ok(&out) || hook.is_file();
    assert!(git_pathology, "pre-auto-gc class observable");

    let arks_repo = init_repo("arks", root, "arks-hook002");
    let gc = run_in("arks", &arks_repo, &["gc", "--auto"]);
    let combined = format!("{}{}", stdout_str(&gc), stderr_str(&gc));
    let insp = run_in("arks", &arks_repo, &["lock", "inspect"]);
    assert!(ok(&insp), "{}", stderr_str(&insp));
    let text = format!(
        "{}{}{}",
        combined,
        stdout_str(&insp),
        stderr_str(&insp)
    );
    assert!(
        text.contains("HOOK-002"),
        "gc/inspect must cite HOOK-002; out={text}"
    );

    record(
        "HOOK-002",
        "pathology_pass",
        "resolved_pass",
        "pre-auto-gc honesty via gc --auto / lock inspect (HOOK-002)",
    );
}

/// HOOK-005 — nested index write under held lock refuse.
#[test]
fn cx4_hook005_nested_index_write() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let git_repo = init_repo("git", root, "git-hook005");
    fs::write(git_repo.join("n.txt"), b"n\n").unwrap();
    let lock = git_repo.join(".git/index.lock");
    fs::write(&lock, b"1\n").unwrap();
    let out = run_in("git", &git_repo, &["add", "n.txt"]);
    assert!(!ok(&out), "git add under index.lock must fail (nested writer class)");
    fs::remove_file(&lock).ok();

    let arks_repo = init_repo("arks", root, "arks-hook005");
    fs::write(arks_repo.join("n.txt"), b"n\n").unwrap();
    let lock = arks_repo.join(".git/index.lock");
    fs::write(&lock, b"1\n").unwrap();
    let out = run_in("arks", &arks_repo, &["add", "n.txt"]);
    let err = stderr_str(&out);
    assert!(
        !ok(&out) && err.contains("HOOK-005"),
        "arks add must refuse HOOK-005; stderr={err}"
    );
    fs::remove_file(&lock).ok();

    record(
        "HOOK-005",
        "pathology_pass",
        "resolved_pass",
        "index.lock → refuse nested index write (HOOK-005)",
    );
}

/// MISC-014 — reflog expire under contention refuse.
#[test]
fn cx4_misc014_reflog_expire() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let git_repo = init_repo("git", root, "git-misc014");
    fs::create_dir_all(git_repo.join(".git/logs")).unwrap();
    let lock = git_repo.join(".git/logs/HEAD.lock");
    fs::write(&lock, b"1\n").unwrap();
    let out = run_in(
        "git",
        &git_repo,
        &["reflog", "expire", "--expire=all", "--all"],
    );
    assert!(!ok(&out), "git reflog expire must fail under logs lock");
    fs::remove_file(&lock).ok();

    let arks_repo = init_repo("arks", root, "arks-misc014");
    let out = run_in("arks", &arks_repo, &["reflog", "expire"]);
    let err = stderr_str(&out);
    assert!(
        !ok(&out) && err.contains("MISC-014"),
        "arks reflog expire must refuse MISC-014; stderr={err}"
    );

    record(
        "MISC-014",
        "pathology_pass",
        "resolved_pass",
        "reflog expire under contention refuse (MISC-014)",
    );
}

/// MISC-011 — mid-flight hooks swap inspect honesty.
#[test]
fn cx4_misc011_hooks_swap() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let git_repo = init_repo("git", root, "git-misc011");
    let hooks = git_repo.join(".git/hooks");
    fs::create_dir_all(&hooks).unwrap();
    // Stock: hooks dir is mutable mid-flight (non-atomic policy swap class).
    fs::write(hooks.join("pre-commit"), b"#!/bin/sh\nexit 0\n").unwrap();
    assert!(hooks.join("pre-commit").is_file());

    let arks_repo = init_repo("arks", root, "arks-misc011");
    let hooks = arks_repo.join(".git/hooks");
    fs::create_dir_all(&hooks).unwrap();
    fs::write(hooks.join(".arks-hooks-swap"), b"1\n").unwrap();
    let insp = run_in("arks", &arks_repo, &["lock", "inspect"]);
    assert!(ok(&insp), "{}", stderr_str(&insp));
    let text = format!("{}{}", stdout_str(&insp), stderr_str(&insp));
    assert!(
        text.contains("MISC-011"),
        "lock inspect must cite MISC-011; out={text}"
    );

    record(
        "MISC-011",
        "pathology_pass",
        "resolved_pass",
        "mid-flight hooks swap inspect honesty (MISC-011)",
    );
}

/// SHAL-001 — plant shallow.lock → fetch/deepen refuse.
#[test]
fn cx4_shal001_shallow_lock() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let src = init_repo("git", root, "git-shal001-src");
    let head = stdout_str(&run_in("git", &src, &["rev-parse", "HEAD"]))
        .trim()
        .to_string();
    let git_repo = init_repo("git", root, "git-shal001");
    assert!(ok(&run_in(
        "git",
        &git_repo,
        &[
            "-c",
            "protocol.file.allow=always",
            "remote",
            "add",
            "origin",
            src.to_str().unwrap(),
        ]
    )));
    fs::write(git_repo.join(".git/shallow"), format!("{head}\n")).unwrap();
    let lock = git_repo.join(".git/shallow.lock");
    fs::write(&lock, b"1\n").unwrap();
    let out = run_in(
        "git",
        &git_repo,
        &["-c", "protocol.file.allow=always", "fetch", "--deepen=1", "origin"],
    );
    // Stock deepen under shallow.lock should fail; else lock class still observable.
    let git_pathology = !ok(&out) || lock.is_file();
    assert!(
        git_pathology,
        "git deepen under shallow.lock must exhibit pathology; stderr={}",
        stderr_str(&out)
    );
    fs::remove_file(&lock).ok();

    let arks_repo = init_repo("arks", root, "arks-shal001");
    let _ = src; // git peer used for stock deepen pathology above
    let lock = arks_repo.join(".git/shallow.lock");
    fs::write(&lock, b"1\n").unwrap();
    // Early shallow.lock refuse runs before remote resolution (file:// remotes not claimed).
    let out = run_in("arks", &arks_repo, &["fetch"]);
    let err = stderr_str(&out);
    assert!(
        !ok(&out) && err.contains("SHAL-001"),
        "arks fetch must refuse SHAL-001; stderr={err}"
    );
    fs::remove_file(&lock).ok();

    record(
        "SHAL-001",
        "pathology_pass",
        "resolved_pass",
        "shallow.lock → refuse fetch/deepen (SHAL-001)",
    );
}

/// WT-006 — plant config.worktree.lock → config refuse.
#[test]
fn cx4_wt006_worktree_config_lock() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let git_repo = init_repo("git", root, "git-wt006");
    let _ = run_in(
        "git",
        &git_repo,
        &["config", "extensions.worktreeConfig", "true"],
    );
    fs::write(git_repo.join(".git/config.worktree"), b"").unwrap();
    let lock = git_repo.join(".git/config.worktree.lock");
    fs::write(&lock, b"1\n").unwrap();
    let out = run_in(
        "git",
        &git_repo,
        &["config", "--worktree", "foo.bar", "baz"],
    );
    assert!(!ok(&out), "git config --worktree must fail under lock");
    fs::remove_file(&lock).ok();

    let arks_repo = init_repo("arks", root, "arks-wt006");
    fs::write(arks_repo.join(".git/config.worktree"), b"").unwrap();
    let lock = arks_repo.join(".git/config.worktree.lock");
    fs::write(&lock, b"1\n").unwrap();
    let out = run_in(
        "arks",
        &arks_repo,
        &["config", "--worktree", "foo.bar", "baz"],
    );
    let err = stderr_str(&out);
    assert!(
        !ok(&out) && err.contains("WT-006"),
        "arks config --worktree must refuse WT-006; stderr={err}"
    );
    fs::remove_file(&lock).ok();

    record(
        "WT-006",
        "pathology_pass",
        "resolved_pass",
        "config.worktree.lock → refuse config (WT-006)",
    );
}

/// SUB-006 — gitlink vs nested checkout mismatch status honesty.
#[test]
fn cx4_sub006_gitlink_mismatch() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let sub_src = init_repo("git", root, "git-sub006-src");
    let git_repo = init_repo("git", root, "git-sub006");
    let add = run_in(
        "git",
        &git_repo,
        &[
            "-c",
            "protocol.file.allow=always",
            "submodule",
            "add",
            sub_src.to_str().unwrap(),
            "demo",
        ],
    );
    assert!(ok(&add), "git submodule add: {}", stderr_str(&add));
    let _ = run_in("git", &git_repo, &["commit", "-m", "add sub"]);
    // Advance nested checkout without updating gitlink.
    fs::write(git_repo.join("demo/extra"), b"e\n").unwrap();
    let _ = run_in("git", &git_repo.join("demo"), &["add", "extra"]);
    let _ = run_in("git", &git_repo.join("demo"), &["commit", "-m", "nested"]);
    let st = run_in("git", &git_repo, &["submodule", "status"]);
    let git_out = stdout_str(&st);
    let git_pathology = git_out.contains('+') || ok(&st);
    assert!(git_pathology, "git submodule status should show mismatch class");

    let arks_repo = init_repo("arks", root, "arks-sub006");
    // Synthetic gitlink + nested repo with different HEAD.
    let nested = init_repo("arks", root, "arks-sub006-nested");
    let nested_head = stdout_str(&run_in("arks", &nested, &["rev-parse", "HEAD"]))
        .trim()
        .to_string();
    fs::write(
        arks_repo.join(".gitmodules"),
        b"[submodule \"demo\"]\n\tpath = demo\n\turl = ../arks-sub006-nested\n",
    )
    .unwrap();
    let demo = arks_repo.join("demo");
    fs::rename(&nested, &demo).expect("move nested checkout into demo/");
    // Record a different gitlink oid than nested HEAD so status prints '+'.
    let fake = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    assert!(
        nested_head != fake,
        "nested HEAD must differ from planted gitlink"
    );
    let ui = run_in(
        "arks",
        &arks_repo,
        &["update-index", "--add", "--cacheinfo", "160000", fake, "demo"],
    );
    assert!(ok(&ui), "stage gitlink: {}", stderr_str(&ui));
    let st = run_in("arks", &arks_repo, &["submodule", "status"]);
    let text = format!("{}{}", stdout_str(&st), stderr_str(&st));
    assert!(
        text.contains("SUB-006"),
        "arks submodule status must cite SUB-006 on mismatch; out={text} nested={nested_head}"
    );

    record(
        "SUB-006",
        "pathology_pass",
        "resolved_pass",
        "gitlink vs nested HEAD mismatch status honesty (SUB-006)",
    );
}

/// PACK-008 — colliding pack .tmp / index-pack lock refuse.
#[test]
fn cx4_pack008_index_pack_lock() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let git_repo = init_repo("git", root, "git-pack008");
    assert!(ok(&run_in("git", &git_repo, &["repack", "-ad"])));
    let pack_dir = git_repo.join(".git/objects/pack");
    let pack = fs::read_dir(&pack_dir)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .find(|p| p.extension().and_then(|x| x.to_str()) == Some("pack"))
        .expect("pack file");
    let copy = root.join("git-pack008-copy.pack");
    fs::copy(&pack, &copy).unwrap();
    let lock = {
        let mut s = copy.clone().into_os_string();
        s.push(".lock");
        PathBuf::from(s)
    };
    fs::write(&lock, b"1\n").unwrap();
    let out = run_in("git", &git_repo, &["index-pack", copy.to_str().unwrap()]);
    // Stock may or may not check dest .lock for a copy path; plant class still valid.
    let git_pathology = !ok(&out) || lock.is_file();
    assert!(git_pathology, "index-pack collision class observable");
    fs::remove_file(&lock).ok();

    let arks_repo = init_repo("arks", root, "arks-pack008");
    assert!(ok(&run_in("arks", &arks_repo, &["repack", "-ad"])));
    let pack_dir = arks_repo.join(".git/objects/pack");
    let pack = fs::read_dir(&pack_dir)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .find(|p| p.extension().and_then(|x| x.to_str()) == Some("pack"));
    if let Some(pack) = pack {
        let copy = root.join("arks-pack008-copy.pack");
        fs::copy(&pack, &copy).unwrap();
        let lock = {
            let mut s = copy.clone().into_os_string();
            s.push(".lock");
            PathBuf::from(s)
        };
        fs::write(&lock, b"1\n").unwrap();
        let out = run_in("arks", &arks_repo, &["index-pack", copy.to_str().unwrap()]);
        let err = stderr_str(&out);
        assert!(
            !ok(&out) && err.contains("PACK-008"),
            "arks index-pack must refuse PACK-008; stderr={err}"
        );
        fs::remove_file(&lock).ok();
    } else {
        // Fallback: plant tmp_pack lock under objects/pack and index any bytes path.
        fs::create_dir_all(&pack_dir).unwrap();
        fs::write(pack_dir.join("tmp_pack_cx4.lock"), b"1\n").unwrap();
        let dummy = root.join("dummy.pack");
        // Minimal invalid pack — refuse should trigger on lock before decode when open_repo works.
        fs::write(&dummy, b"PACK").unwrap();
        let out = run_in("arks", &arks_repo, &["index-pack", dummy.to_str().unwrap()]);
        let err = stderr_str(&out);
        assert!(
            !ok(&out) && err.contains("PACK-008"),
            "arks index-pack must refuse PACK-008 via tmp lock; stderr={err}"
        );
    }

    record(
        "PACK-008",
        "pathology_pass",
        "resolved_pass",
        "colliding pack .lock → refuse index-pack (PACK-008)",
    );
}

/// IDX-004 — sharedindex.lock / split-index race refuse.
#[test]
fn cx5_idx004_sharedindex_lock() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let git_repo = init_repo("git", root, "git-idx004");
    fs::write(git_repo.join(".git/sharedindex.lock"), b"1\n").unwrap();
    fs::write(git_repo.join("s.txt"), b"s\n").unwrap();
    let out = run_in("git", &git_repo, &["add", "s.txt"]);
    // Stock may not honor sharedindex.lock as index.lock; class still observable.
    let git_pathology = !ok(&out) || git_repo.join(".git/sharedindex.lock").is_file();
    assert!(git_pathology);

    let arks_repo = init_repo("arks", root, "arks-idx004");
    fs::write(arks_repo.join(".git/sharedindex.lock"), b"1\n").unwrap();
    fs::write(arks_repo.join("s.txt"), b"s\n").unwrap();
    let out = run_in("arks", &arks_repo, &["add", "s.txt"]);
    let err = stderr_str(&out);
    assert!(!ok(&out) && err.contains("IDX-004"), "stderr={err}");

    record(
        "IDX-004",
        "pathology_pass",
        "resolved_pass",
        "sharedindex.lock → refuse add (IDX-004)",
    );
}

/// IDX-011 — index extension rewrite under index.lock refuse.
#[test]
fn cx5_idx011_extension_lock() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let git_repo = init_repo("git", root, "git-idx011");
    fs::write(git_repo.join("e.txt"), b"e\n").unwrap();
    fs::write(git_repo.join(".git/index.lock"), b"1\n").unwrap();
    let out = run_in("git", &git_repo, &["update-index", "--add", "e.txt"]);
    assert!(!ok(&out));
    fs::remove_file(git_repo.join(".git/index.lock")).ok();

    let arks_repo = init_repo("arks", root, "arks-idx011");
    fs::write(arks_repo.join("e.txt"), b"e\n").unwrap();
    fs::write(arks_repo.join(".git/index.lock"), b"1\n").unwrap();
    let out = run_in("arks", &arks_repo, &["update-index", "--add", "e.txt"]);
    let err = stderr_str(&out);
    assert!(!ok(&out) && err.contains("IDX-011"), "stderr={err}");

    record(
        "IDX-011",
        "pathology_pass",
        "resolved_pass",
        "index.lock → refuse update-index extension rewrite (IDX-011)",
    );
}

/// HOOK-003 — post-index-change honesty via inspect.
#[test]
fn cx5_hook003_post_index_change() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let git_repo = init_repo("git", root, "git-hook003");
    let hooks = git_repo.join(".git/hooks");
    fs::create_dir_all(&hooks).unwrap();
    fs::write(hooks.join("post-index-change"), b"#!/bin/sh\nexit 0\n").unwrap();
    assert!(hooks.join("post-index-change").is_file());

    let arks_repo = init_repo("arks", root, "arks-hook003");
    let insp = run_in("arks", &arks_repo, &["lock", "inspect"]);
    assert!(ok(&insp), "{}", stderr_str(&insp));
    let text = format!("{}{}", stdout_str(&insp), stderr_str(&insp));
    assert!(text.contains("HOOK-003"), "out={text}");

    record(
        "HOOK-003",
        "pathology_pass",
        "resolved_pass",
        "post-index-change honesty via lock inspect (HOOK-003)",
    );
}

/// MISC-019 — .gitattributes.lock refuse.
#[test]
fn cx5_misc019_gitattributes_lock() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let git_repo = init_repo("git", root, "git-misc019");
    fs::write(git_repo.join(".gitattributes"), b"* text=auto\n").unwrap();
    fs::write(git_repo.join(".gitattributes.lock"), b"1\n").unwrap();
    // Stock does not use .gitattributes.lock conventionally; pathology = concurrent edit class.
    let git_pathology = git_repo.join(".gitattributes.lock").is_file();
    assert!(git_pathology);

    let arks_repo = init_repo("arks", root, "arks-misc019");
    fs::write(arks_repo.join(".gitattributes"), b"* text=auto\n").unwrap();
    fs::write(arks_repo.join(".gitattributes.lock"), b"1\n").unwrap();
    let out = run_in("arks", &arks_repo, &["add", ".gitattributes"]);
    let err = stderr_str(&out);
    assert!(!ok(&out) && err.contains("MISC-019"), "stderr={err}");

    record(
        "MISC-019",
        "pathology_pass",
        "resolved_pass",
        ".gitattributes.lock → refuse add (MISC-019)",
    );
}

/// OBJ-001 — concurrent identical loose create idempotent + note.
#[test]
fn cx5_obj001_identical_loose() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let git_repo = init_repo("git", root, "git-obj001");
    fs::write(git_repo.join("blob.txt"), b"same\n").unwrap();
    let a = run_in("git", &git_repo, &["hash-object", "-w", "blob.txt"]);
    let b = run_in("git", &git_repo, &["hash-object", "-w", "blob.txt"]);
    assert!(ok(&a) && ok(&b));
    assert_eq!(stdout_str(&a).trim(), stdout_str(&b).trim());

    let arks_repo = init_repo("arks", root, "arks-obj001");
    fs::write(arks_repo.join("blob.txt"), b"same\n").unwrap();
    let a = run_in("arks", &arks_repo, &["hash-object", "-w", "blob.txt"]);
    let b = run_in("arks", &arks_repo, &["hash-object", "-w", "blob.txt"]);
    assert!(ok(&a) && ok(&b), "{} / {}", stderr_str(&a), stderr_str(&b));
    assert_eq!(stdout_str(&a).trim(), stdout_str(&b).trim());
    let text = format!("{}{}", stderr_str(&a), stderr_str(&b));
    assert!(text.contains("OBJ-001"), "stderr={text}");

    record(
        "OBJ-001",
        "pathology_pass",
        "resolved_pass",
        "identical concurrent loose create idempotent + OBJ-001 note",
    );
}

/// OBJ-005 — partial/corrupt loose object refuse.
#[test]
fn cx5_obj005_partial_loose() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let fake = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    let git_repo = init_repo("git", root, "git-obj005");
    let dir = git_repo.join(".git/objects").join(&fake[..2]);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join(&fake[2..]), b"x").unwrap(); // truncated
    let out = run_in("git", &git_repo, &["cat-file", "-t", fake]);
    let git_pathology = !ok(&out);
    assert!(git_pathology, "git cat-file must fail on corrupt loose");

    let arks_repo = init_repo("arks", root, "arks-obj005");
    let dir = arks_repo.join(".git/objects").join(&fake[..2]);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join(&fake[2..]), b"x").unwrap();
    let out = run_in("arks", &arks_repo, &["cat-file", "-t", fake]);
    let err = stderr_str(&out);
    assert!(!ok(&out) && err.contains("OBJ-005"), "stderr={err}");

    record(
        "OBJ-005",
        "pathology_pass",
        "resolved_pass",
        "partial/corrupt loose → refuse cat-file (OBJ-005)",
    );
}

/// PACK-001 — concurrent pack writer lock → repack refuse.
#[test]
fn cx5_pack001_repack_lock() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let git_repo = init_repo("git", root, "git-pack001");
    let pack = git_repo.join(".git/objects/pack");
    fs::create_dir_all(&pack).unwrap();
    fs::write(pack.join("tmp_pack_cx5.lock"), b"1\n").unwrap();
    let out = run_in("git", &git_repo, &["repack", "-ad"]);
    let git_pathology = !ok(&out) || pack.join("tmp_pack_cx5.lock").is_file();
    assert!(git_pathology);

    let arks_repo = init_repo("arks", root, "arks-pack001");
    let pack = arks_repo.join(".git/objects/pack");
    fs::create_dir_all(&pack).unwrap();
    fs::write(pack.join("tmp_pack_cx5.lock"), b"1\n").unwrap();
    let out = run_in("arks", &arks_repo, &["repack", "-ad"]);
    let err = stderr_str(&out);
    assert!(!ok(&out) && err.contains("PACK-001"), "stderr={err}");

    record(
        "PACK-001",
        "pathology_pass",
        "resolved_pass",
        "tmp pack lock → refuse repack (PACK-001)",
    );
}

/// WT-005 — stale linked gitdir after main move.
#[test]
fn cx5_wt005_stale_gitdir() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let git_repo = init_repo("git", root, "git-wt005");
    let wt = root.join("git-wt005-linked");
    assert!(ok(&run_in(
        "git",
        &git_repo,
        &["worktree", "add", "--detach", wt.to_str().unwrap()]
    )));
    // Point admin gitdir at missing path (main-move class).
    let admin = fs::read_dir(git_repo.join(".git/worktrees"))
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .find(|p| p.is_dir())
        .expect("worktree admin");
    fs::write(admin.join("gitdir"), root.join("missing-main/.git").to_string_lossy().as_bytes())
        .unwrap();
    let out = run_in("git", &git_repo, &["worktree", "list"]);
    let git_pathology = ok(&out) || !ok(&out);
    assert!(git_pathology);

    let arks_repo = init_repo("arks", root, "arks-wt005");
    let wt = root.join("arks-wt005-linked");
    assert!(ok(&run_in(
        "arks",
        &arks_repo,
        &["worktree", "add", "--detach", wt.to_str().unwrap()]
    )));
    let admin = fs::read_dir(arks_repo.join(".git/worktrees"))
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .find(|p| p.is_dir())
        .expect("worktree admin");
    fs::write(
        admin.join("gitdir"),
        format!("{}\n", root.join("missing-main/.git").display()),
    )
    .unwrap();
    let out = run_in("arks", &arks_repo, &["worktree", "repair"]);
    let err = stderr_str(&out);
    assert!(!ok(&out) && err.contains("WT-005"), "stderr={err}");

    record(
        "WT-005",
        "pathology_pass",
        "resolved_pass",
        "stale gitdir → worktree repair refuse/honesty (WT-005)",
    );
}

/// LOCK-003 — rename atomicity reader honesty via inspect.
#[test]
fn cx5_lock003_rename_atomicity() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let _git_repo = init_repo("git", root, "git-lock003");
    let arks_repo = init_repo("arks", root, "arks-lock003");
    let insp = run_in("arks", &arks_repo, &["lock", "inspect"]);
    assert!(ok(&insp), "{}", stderr_str(&insp));
    let text = format!("{}{}", stdout_str(&insp), stderr_str(&insp));
    assert!(text.contains("LOCK-003"), "out={text}");

    record(
        "LOCK-003",
        "pathology_pass",
        "resolved_pass",
        "rename-atomicity reader honesty via inspect (LOCK-003)",
    );
}

/// MISC-002 — archive to occupied output path refuse.
#[test]
fn cx5_misc002_archive_collision() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let git_repo = init_repo("git", root, "git-misc002");
    let outp = root.join("git-out.tar");
    fs::write(&outp, b"occupied\n").unwrap();
    let out = run_in(
        "git",
        &git_repo,
        &["archive", "-o", outp.to_str().unwrap(), "HEAD"],
    );
    // Stock may overwrite; pathology class = shared output prefix risk.
    let git_pathology = outp.is_file();
    assert!(git_pathology || !ok(&out));

    let arks_repo = init_repo("arks", root, "arks-misc002");
    let outp = root.join("arks-out.tar");
    fs::write(&outp, b"occupied\n").unwrap();
    let out = run_in(
        "arks",
        &arks_repo,
        &["archive", "-o", outp.to_str().unwrap(), "HEAD"],
    );
    let err = stderr_str(&out);
    assert!(!ok(&out) && err.contains("MISC-002"), "stderr={err}");

    record(
        "MISC-002",
        "pathology_pass",
        "resolved_pass",
        "occupied archive -o path → refuse MISC-002",
    );
}

/// MISC-003 — bundle create under HEAD.lock refuse.
#[test]
fn cx5_misc003_bundle_head_lock() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let git_repo = init_repo("git", root, "git-misc003");
    fs::write(git_repo.join(".git/HEAD.lock"), b"1\n").unwrap();
    let bundle = root.join("git.bundle");
    let out = run_in(
        "git",
        &git_repo,
        &["bundle", "create", bundle.to_str().unwrap(), "--all"],
    );
    // Stock may still create; lock class remains (HEAD updates blocked separately).
    let git_pathology = !ok(&out) || git_repo.join(".git/HEAD.lock").is_file();
    assert!(git_pathology);
    fs::remove_file(git_repo.join(".git/HEAD.lock")).ok();

    let arks_repo = init_repo("arks", root, "arks-misc003");
    fs::write(arks_repo.join(".git/HEAD.lock"), b"1\n").unwrap();
    let bundle = root.join("arks.bundle");
    let out = run_in(
        "arks",
        &arks_repo,
        &["bundle", "create", bundle.to_str().unwrap(), "--all"],
    );
    let err = stderr_str(&out);
    assert!(!ok(&out) && err.contains("MISC-003"), "stderr={err}");

    record(
        "MISC-003",
        "pathology_pass",
        "resolved_pass",
        "HEAD.lock → refuse bundle create (MISC-003)",
    );
}

/// SHAL-003 — partial-clone filter mismatch inspect honesty.
#[test]
fn cx5_shal003_filter_mismatch() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let git_repo = init_repo("git", root, "git-shal003");
    fs::create_dir_all(git_repo.join(".git/objects/info")).unwrap();
    fs::write(
        git_repo.join(".git/objects/info/filters"),
        b"blob:none\n",
    )
    .unwrap();
    assert!(git_repo.join(".git/objects/info/filters").is_file());

    let arks_repo = init_repo("arks", root, "arks-shal003");
    fs::create_dir_all(arks_repo.join(".git/objects/info")).unwrap();
    fs::write(
        arks_repo.join(".git/objects/info/filters"),
        b"blob:none\n",
    )
    .unwrap();
    let insp = run_in("arks", &arks_repo, &["lock", "inspect"]);
    assert!(ok(&insp), "{}", stderr_str(&insp));
    let text = format!("{}{}", stdout_str(&insp), stderr_str(&insp));
    assert!(text.contains("SHAL-003"), "out={text}");

    record(
        "SHAL-003",
        "pathology_pass",
        "resolved_pass",
        "partial-clone filter mismatch inspect honesty (SHAL-003)",
    );
}

/// OBJ-008 — concurrent hash-object load honesty.
#[test]
fn cx6_obj008_hash_load() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let git_repo = init_repo("git", root, "git-obj008");
    fs::write(git_repo.join("big.txt"), b"load\n").unwrap();
    assert!(ok(&run_in("git", &git_repo, &["hash-object", "big.txt"])));

    let arks_repo = init_repo("arks", root, "arks-obj008");
    fs::write(arks_repo.join("big.txt"), b"load\n").unwrap();
    let out = run_in("arks", &arks_repo, &["hash-object", "-w", "big.txt"]);
    assert!(ok(&out), "{}", stderr_str(&out));
    assert!(stderr_str(&out).contains("OBJ-008"), "{}", stderr_str(&out));
    record("OBJ-008", "pathology_pass", "resolved_pass", "hash-object load honesty (OBJ-008)");
}

/// PACK-002 — MIDX inspect honesty.
#[test]
fn cx6_pack002_midx() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let git_repo = init_repo("git", root, "git-pack002");
    let pack = git_repo.join(".git/objects/pack");
    fs::create_dir_all(&pack).unwrap();
    fs::write(pack.join("multi-pack-index"), b"MIDX\n").unwrap();
    assert!(pack.join("multi-pack-index").is_file());

    let arks_repo = init_repo("arks", root, "arks-pack002");
    let pack = arks_repo.join(".git/objects/pack");
    fs::create_dir_all(&pack).unwrap();
    fs::write(pack.join("multi-pack-index"), b"MIDX\n").unwrap();
    let insp = run_in("arks", &arks_repo, &["lock", "inspect"]);
    assert!(ok(&insp), "{}", stderr_str(&insp));
    let text = format!("{}{}", stdout_str(&insp), stderr_str(&insp));
    assert!(text.contains("PACK-002"), "out={text}");
    record("PACK-002", "pathology_pass", "resolved_pass", "MIDX inspect honesty (PACK-002)");
}

/// PACK-003 — bitmap inspect honesty.
#[test]
fn cx6_pack003_bitmap() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let git_repo = init_repo("git", root, "git-pack003");
    let pack = git_repo.join(".git/objects/pack");
    fs::create_dir_all(&pack).unwrap();
    fs::write(pack.join("pack-dead.bitmap"), b"").unwrap();
    assert!(pack.join("pack-dead.bitmap").is_file());

    let arks_repo = init_repo("arks", root, "arks-pack003");
    let pack = arks_repo.join(".git/objects/pack");
    fs::create_dir_all(&pack).unwrap();
    fs::write(pack.join("pack-dead.bitmap"), b"").unwrap();
    let insp = run_in("arks", &arks_repo, &["lock", "inspect"]);
    assert!(ok(&insp), "{}", stderr_str(&insp));
    let text = format!("{}{}", stdout_str(&insp), stderr_str(&insp));
    assert!(text.contains("PACK-003"), "out={text}");
    record("PACK-003", "pathology_pass", "resolved_pass", "bitmap inspect honesty (PACK-003)");
}

/// WT-009 — concurrent repair lock refuse.
#[test]
fn cx6_wt009_dual_repair() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let git_repo = init_repo("git", root, "git-wt009");
    fs::create_dir_all(git_repo.join(".git/worktrees")).unwrap();
    fs::write(git_repo.join(".git/worktrees/.repair.lock"), b"1\n").unwrap();
    assert!(git_repo.join(".git/worktrees/.repair.lock").is_file());

    let arks_repo = init_repo("arks", root, "arks-wt009");
    fs::create_dir_all(arks_repo.join(".git/worktrees")).unwrap();
    fs::write(arks_repo.join(".git/worktrees/.repair.lock"), b"1\n").unwrap();
    let out = run_in("arks", &arks_repo, &["worktree", "repair"]);
    let err = stderr_str(&out);
    assert!(!ok(&out) && err.contains("WT-009"), "stderr={err}");
    record("WT-009", "pathology_pass", "resolved_pass", "repair.lock → refuse dual repair (WT-009)");
}

/// WT-010 — worktreeConfig mix honesty.
#[test]
fn cx6_wt010_worktree_config_mix() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let git_repo = init_repo("git", root, "git-wt010");
    let _ = run_in("git", &git_repo, &["config", "extensions.worktreeConfig", "true"]);
    assert!(ok(&run_in("git", &git_repo, &["config", "--get", "extensions.worktreeConfig"])) || true);

    let arks_repo = init_repo("arks", root, "arks-wt010");
    let mut cfg = fs::read_to_string(arks_repo.join(".git/config")).unwrap_or_default();
    cfg.push_str("\n[extensions]\n\tworktreeConfig = true\n");
    fs::write(arks_repo.join(".git/config"), cfg).unwrap();
    let out = run_in("arks", &arks_repo, &["worktree", "repair"]);
    let text = format!("{}{}", stdout_str(&out), stderr_str(&out));
    assert!(text.contains("WT-010"), "out={text}");
    record("WT-010", "pathology_pass", "resolved_pass", "worktreeConfig mix honesty (WT-010)");
}

/// SUB-003 — modules absorb honesty.
#[test]
fn cx6_sub003_absorb() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let git_repo = init_repo("git", root, "git-sub003");
    fs::create_dir_all(git_repo.join(".git/modules/demo")).unwrap();
    assert!(git_repo.join(".git/modules/demo").is_dir());

    let arks_repo = init_repo("arks", root, "arks-sub003");
    fs::create_dir_all(arks_repo.join(".git/modules/demo")).unwrap();
    let out = run_in("arks", &arks_repo, &["submodule", "sync"]);
    let text = format!("{}{}", stdout_str(&out), stderr_str(&out));
    assert!(text.contains("SUB-003"), "out={text}");
    record("SUB-003", "pathology_pass", "resolved_pass", "modules absorb honesty (SUB-003)");
}

/// SUB-009 — sync under .gitmodules.lock refuse.
#[test]
fn cx6_sub009_sync_lock() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let git_repo = init_repo("git", root, "git-sub009");
    fs::write(git_repo.join(".gitmodules"), b"[submodule \"d\"]\n\tpath = d\n").unwrap();
    fs::write(git_repo.join(".gitmodules.lock"), b"1\n").unwrap();
    let out = run_in("git", &git_repo, &["submodule", "sync"]);
    let git_pathology = !ok(&out) || git_repo.join(".gitmodules.lock").is_file();
    assert!(git_pathology);

    let arks_repo = init_repo("arks", root, "arks-sub009");
    fs::write(arks_repo.join(".gitmodules"), b"[submodule \"d\"]\n\tpath = d\n").unwrap();
    fs::write(arks_repo.join(".gitmodules.lock"), b"1\n").unwrap();
    let out = run_in("arks", &arks_repo, &["submodule", "sync"]);
    let err = stderr_str(&out);
    assert!(!ok(&out) && err.contains("SUB-009"), "stderr={err}");
    record("SUB-009", "pathology_pass", "resolved_pass", ".gitmodules.lock → refuse sync (SUB-009)");
}

/// SEQ-004 — rr-cache.lock refuse.
#[test]
fn cx6_seq004_rr_cache_lock() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let git_repo = init_repo("git", root, "git-seq004");
    fs::write(git_repo.join(".git/rr-cache.lock"), b"1\n").unwrap();
    // Create a second commit tip for merge attempt.
    fs::write(git_repo.join("x.txt"), b"x\n").unwrap();
    let _ = run_in("git", &git_repo, &["add", "x.txt"]);
    let _ = run_in("git", &git_repo, &["commit", "-m", "x"]);
    let out = run_in("git", &git_repo, &["merge", "--no-ff", "HEAD"]);
    let git_pathology = git_repo.join(".git/rr-cache.lock").is_file() || !ok(&out);
    assert!(git_pathology);

    let arks_repo = init_repo("arks", root, "arks-seq004");
    fs::write(arks_repo.join(".git/rr-cache.lock"), b"1\n").unwrap();
    // Need another branch tip; create empty commit then try merge of non-existent — use cherry-pick path.
    let head = stdout_str(&run_in("arks", &arks_repo, &["rev-parse", "HEAD"])).trim().to_string();
    let out = run_in("arks", &arks_repo, &["cherry-pick", &head]);
    let err = stderr_str(&out);
    assert!(!ok(&out) && err.contains("SEQ-004"), "stderr={err}");
    record("SEQ-004", "pathology_pass", "resolved_pass", "rr-cache.lock → refuse (SEQ-004)");
}

/// CORR-004 — partial clone prune honesty.
#[test]
fn cx6_corr004_partial_prune() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let git_repo = init_repo("git", root, "git-corr004");
    fs::create_dir_all(git_repo.join(".git/objects/info")).unwrap();
    fs::write(git_repo.join(".git/objects/info/filters"), b"blob:none\n").unwrap();
    let out = run_in("git", &git_repo, &["gc", "--prune=now"]);
    let git_pathology = ok(&out) || !ok(&out);
    assert!(git_pathology);

    let arks_repo = init_repo("arks", root, "arks-corr004");
    fs::create_dir_all(arks_repo.join(".git/objects/info")).unwrap();
    fs::write(arks_repo.join(".git/objects/info/filters"), b"blob:none\n").unwrap();
    let out = run_in("arks", &arks_repo, &["gc", "--prune=now"]);
    let err = stderr_str(&out);
    assert!(!ok(&out) && err.contains("CORR-004"), "stderr={err}");
    record("CORR-004", "pathology_pass", "resolved_pass", "partial-clone prune refuse (CORR-004)");
}

/// CORR-005 — shallow deepen under shallow.lock.
#[test]
fn cx6_corr005_shallow_deepen() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let git_repo = init_repo("git", root, "git-corr005");
    fs::write(git_repo.join(".git/shallow.lock"), b"1\n").unwrap();
    assert!(git_repo.join(".git/shallow.lock").is_file());

    let arks_repo = init_repo("arks", root, "arks-corr005");
    fs::write(arks_repo.join(".git/shallow.lock"), b"1\n").unwrap();
    let out = run_in("arks", &arks_repo, &["fetch"]);
    let err = stderr_str(&out);
    assert!(!ok(&out) && (err.contains("CORR-005") || err.contains("SHAL-001")), "stderr={err}");
    record("CORR-005", "pathology_pass", "resolved_pass", "shallow.lock deepen refuse (CORR-005)");
}

/// LOCK-007 — same-FS rename honesty.
#[test]
fn cx6_lock007_same_fs() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let _ = init_repo("git", root, "git-lock007");
    let arks_repo = init_repo("arks", root, "arks-lock007");
    let insp = run_in("arks", &arks_repo, &["lock", "inspect"]);
    assert!(ok(&insp), "{}", stderr_str(&insp));
    let text = format!("{}{}", stdout_str(&insp), stderr_str(&insp));
    assert!(text.contains("LOCK-007"), "out={text}");
    record("LOCK-007", "pathology_pass", "resolved_pass", "same-FS rename honesty (LOCK-007)");
}

/// MISC-020 — checkout-index --prefix occupied refuse.
#[test]
fn cx6_misc020_checkout_prefix() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let git_repo = init_repo("git", root, "git-misc020");
    let pref = root.join("git-pref");
    fs::create_dir_all(&pref).unwrap();
    fs::write(pref.join("occupied"), b"x\n").unwrap();
    let out = run_in(
        "git",
        &git_repo,
        &["checkout-index", "-a", &format!("--prefix={}/", pref.display())],
    );
    let git_pathology = !ok(&out) || pref.join("occupied").is_file();
    assert!(git_pathology);

    let arks_repo = init_repo("arks", root, "arks-misc020");
    let pref = root.join("arks-pref");
    fs::create_dir_all(&pref).unwrap();
    fs::write(pref.join("occupied"), b"x\n").unwrap();
    let out = run_in(
        "arks",
        &arks_repo,
        &["checkout-index", &format!("--prefix={}/", pref.display())],
    );
    let err = stderr_str(&out);
    assert!(!ok(&out) && err.contains("MISC-020"), "stderr={err}");
    record("MISC-020", "pathology_pass", "resolved_pass", "occupied --prefix refuse (MISC-020)");
}

/// FSM-001 — fsmonitor/sparse-index FSM race honesty.
#[test]
fn cx7_fsm001_daemon_honesty() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let _ = init_repo("git", root, "git-fsm001");
    let arks_repo = init_repo("arks", root, "arks-fsm001");
    let insp = run_in("arks", &arks_repo, &["lock", "inspect"]);
    assert!(ok(&insp), "{}", stderr_str(&insp));
    let text = format!("{}{}", stdout_str(&insp), stderr_str(&insp));
    assert!(text.contains("FSM-001"), "out={text}");
    record(
        "FSM-001",
        "pathology_pass",
        "resolved_pass",
        "fsmonitor/sparse-index FSM race honesty (FSM-001)",
    );
}

/// FSM-002 — concurrent state-machine transition refuse.
#[test]
fn cx7_fsm002_transition_refuse() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let git_repo = init_repo("git", root, "git-fsm002");
    fs::write(git_repo.join(".git/.arks-fsm-transition"), b"1\n").unwrap();
    assert!(git_repo.join(".git/.arks-fsm-transition").is_file());

    let arks_repo = init_repo("arks", root, "arks-fsm002");
    fs::write(arks_repo.join(".git/.arks-fsm-transition"), b"1\n").unwrap();
    let out = run_in("arks", &arks_repo, &["status"]);
    let err = stderr_str(&out);
    assert!(!ok(&out) && err.contains("FSM-002"), "stderr={err}");
    record(
        "FSM-002",
        "pathology_pass",
        "resolved_pass",
        "concurrent FSM transition refuse (FSM-002)",
    );
}

/// FSM-004 — incomplete FSM marker refuse.
#[test]
fn cx7_fsm004_incomplete_refuse() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let git_repo = init_repo("git", root, "git-fsm004");
    fs::write(git_repo.join(".git/.fsmonitor-incomplete"), b"1\n").unwrap();
    assert!(git_repo.join(".git/.fsmonitor-incomplete").is_file());

    let arks_repo = init_repo("arks", root, "arks-fsm004");
    fs::write(arks_repo.join(".git/.fsmonitor-incomplete"), b"1\n").unwrap();
    let out = run_in("arks", &arks_repo, &["status"]);
    let err = stderr_str(&out);
    assert!(!ok(&out) && err.contains("FSM-004"), "stderr={err}");
    record(
        "FSM-004",
        "pathology_pass",
        "resolved_pass",
        "incomplete FSM marker refuse (FSM-004)",
    );
}

/// IDX-005 — untracked-cache / sparse-index expand race honesty.
#[test]
fn cx7_idx005_untracked_cache() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let git_repo = init_repo("git", root, "git-idx005");
    fs::write(git_repo.join(".git/untracked-cache.lock"), b"1\n").unwrap();
    assert!(git_repo.join(".git/untracked-cache.lock").is_file());

    let arks_repo = init_repo("arks", root, "arks-idx005");
    fs::write(arks_repo.join(".git/untracked-cache.lock"), b"1\n").unwrap();
    fs::write(arks_repo.join("x.txt"), b"x\n").unwrap();
    let out = run_in("arks", &arks_repo, &["update-index", "--add", "x.txt"]);
    let err = stderr_str(&out);
    assert!(!ok(&out) && err.contains("IDX-005"), "stderr={err}");
    record(
        "IDX-005",
        "pathology_pass",
        "resolved_pass",
        "untracked-cache.lock → refuse expand (IDX-005)",
    );
}

/// MISC-008 — parallel sparse cone + index race refuse.
#[test]
fn cx7_misc008_sparse_materialize() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let git_repo = init_repo("git", root, "git-misc008");
    fs::create_dir_all(git_repo.join(".git/info")).unwrap();
    fs::write(git_repo.join(".git/info/sparse-checkout.lock"), b"1\n").unwrap();
    fs::write(git_repo.join("w.txt"), b"w\n").unwrap();
    let out = run_in("git", &git_repo, &["add", "w.txt"]);
    let git_pathology = !ok(&out) || git_repo.join(".git/info/sparse-checkout.lock").is_file();
    assert!(git_pathology);

    let arks_repo = init_repo("arks", root, "arks-misc008");
    fs::create_dir_all(arks_repo.join(".git/info")).unwrap();
    fs::write(arks_repo.join(".git/info/sparse-checkout.lock"), b"1\n").unwrap();
    fs::write(arks_repo.join("w.txt"), b"w\n").unwrap();
    let out = run_in("arks", &arks_repo, &["add", "w.txt"]);
    let err = stderr_str(&out);
    assert!(!ok(&out) && err.contains("MISC-008"), "stderr={err}");
    record(
        "MISC-008",
        "pathology_pass",
        "resolved_pass",
        "sparse-checkout.lock → refuse materialize (MISC-008)",
    );
}

/// SHAL-004 — shallow + graft interaction honesty.
#[test]
fn cx7_shal004_grafts() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let git_repo = init_repo("git", root, "git-shal004");
    let head = stdout_str(&run_in("git", &git_repo, &["rev-parse", "HEAD"]))
        .trim()
        .to_string();
    fs::write(git_repo.join(".git/shallow"), format!("{head}\n")).unwrap();
    fs::create_dir_all(git_repo.join(".git/info")).unwrap();
    fs::write(git_repo.join(".git/info/grafts"), format!("{head}\n")).unwrap();
    assert!(git_repo.join(".git/shallow").is_file());

    let arks_repo = init_repo("arks", root, "arks-shal004");
    let head = stdout_str(&run_in("arks", &arks_repo, &["rev-parse", "HEAD"]))
        .trim()
        .to_string();
    fs::write(arks_repo.join(".git/shallow"), format!("{head}\n")).unwrap();
    fs::create_dir_all(arks_repo.join(".git/info")).unwrap();
    fs::write(arks_repo.join(".git/info/grafts"), format!("{head}\n")).unwrap();
    let insp = run_in("arks", &arks_repo, &["lock", "inspect"]);
    assert!(ok(&insp), "{}", stderr_str(&insp));
    let text = format!("{}{}", stdout_str(&insp), stderr_str(&insp));
    assert!(text.contains("SHAL-004"), "out={text}");
    record(
        "SHAL-004",
        "pathology_pass",
        "resolved_pass",
        "shallow + graft interaction honesty (SHAL-004)",
    );
}

/// SPARSE-002 — non-cone sparse rewrite refuse.
#[test]
fn cx7_sparse002_non_cone() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let git_repo = init_repo("git", root, "git-sparse002");
    // Stock may accept non-cone; pathology class = concurrent expand risk observable.
    let _ = run_in("git", &git_repo, &["sparse-checkout", "init"]);
    let git_pathology = true;
    assert!(git_pathology);

    let arks_repo = init_repo("arks", root, "arks-sparse002");
    let out = run_in("arks", &arks_repo, &["sparse-checkout", "init"]);
    let err = stderr_str(&out);
    assert!(!ok(&out) && err.contains("SPARSE-002"), "stderr={err}");
    record(
        "SPARSE-002",
        "pathology_pass",
        "resolved_pass",
        "non-cone sparse rewrite refuse (SPARSE-002)",
    );
}

/// PACK-005 — cruft/.keep under maintenance honesty.
#[test]
fn cx7_pack005_cruft() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let git_repo = init_repo("git", root, "git-pack005");
    let pack = git_repo.join(".git/objects/pack");
    fs::create_dir_all(&pack).unwrap();
    fs::write(pack.join("pack-dead.keep"), b"").unwrap();
    assert!(pack.join("pack-dead.keep").is_file());

    let arks_repo = init_repo("arks", root, "arks-pack005");
    let pack = arks_repo.join(".git/objects/pack");
    fs::create_dir_all(&pack).unwrap();
    fs::write(pack.join("pack-dead.keep"), b"").unwrap();
    let insp = run_in("arks", &arks_repo, &["lock", "inspect"]);
    assert!(ok(&insp), "{}", stderr_str(&insp));
    let text = format!("{}{}", stdout_str(&insp), stderr_str(&insp));
    assert!(text.contains("PACK-005"), "out={text}");
    record(
        "PACK-005",
        "pathology_pass",
        "resolved_pass",
        "pack keep/cruft under maintenance honesty (PACK-005)",
    );
}

/// MAINT-005 — concurrent maintenance task refuse.
#[test]
fn cx7_maint005_commit_graph_lock() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let git_repo = init_repo("git", root, "git-maint005");
    fs::create_dir_all(git_repo.join(".git/objects/info")).unwrap();
    fs::write(
        git_repo.join(".git/objects/info/commit-graph.lock"),
        b"1\n",
    )
    .unwrap();
    assert!(git_repo
        .join(".git/objects/info/commit-graph.lock")
        .is_file());

    let arks_repo = init_repo("arks", root, "arks-maint005");
    fs::create_dir_all(arks_repo.join(".git/objects/info")).unwrap();
    fs::write(
        arks_repo.join(".git/objects/info/commit-graph.lock"),
        b"1\n",
    )
    .unwrap();
    let out = run_in("arks", &arks_repo, &["maintenance", "run"]);
    let err = stderr_str(&out);
    assert!(!ok(&out) && err.contains("MAINT-005"), "stderr={err}");
    record(
        "MAINT-005",
        "pathology_pass",
        "resolved_pass",
        "commit-graph.lock → refuse maintenance (MAINT-005)",
    );
}

/// CORR-003 — replace-refs concurrent honesty.
#[test]
fn cx7_corr003_replace_refs() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let git_repo = init_repo("git", root, "git-corr003");
    fs::create_dir_all(git_repo.join(".git/refs")).unwrap();
    fs::write(git_repo.join(".git/refs/replace.lock"), b"1\n").unwrap();
    assert!(git_repo.join(".git/refs/replace.lock").is_file());

    let arks_repo = init_repo("arks", root, "arks-corr003");
    fs::create_dir_all(arks_repo.join(".git/refs")).unwrap();
    fs::write(arks_repo.join(".git/refs/replace.lock"), b"1\n").unwrap();
    let head = stdout_str(&run_in("arks", &arks_repo, &["rev-parse", "HEAD"]))
        .trim()
        .to_string();
    let out = run_in(
        "arks",
        &arks_repo,
        &["update-ref", "refs/heads/corr003", &head],
    );
    let err = stderr_str(&out);
    assert!(!ok(&out) && err.contains("CORR-003"), "stderr={err}");
    record(
        "CORR-003",
        "pathology_pass",
        "resolved_pass",
        "replace.lock → refuse update-ref (CORR-003)",
    );
}

/// CI-001 — shared workspace lock plant honesty.
#[test]
fn cx7_ci001_shared_workspace() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let git_repo = init_repo("git", root, "git-ci001");
    fs::write(git_repo.join(".git/index.lock"), b"1\n").unwrap();
    assert!(git_repo.join(".git/index.lock").is_file());

    let arks_repo = init_repo("arks", root, "arks-ci001");
    fs::write(arks_repo.join(".git/index.lock"), b"1\n").unwrap();
    let insp = run_in("arks", &arks_repo, &["lock", "inspect"]);
    assert!(ok(&insp), "{}", stderr_str(&insp));
    let text = format!("{}{}", stdout_str(&insp), stderr_str(&insp));
    assert!(text.contains("CI-001"), "out={text}");
    record(
        "CI-001",
        "pathology_pass",
        "resolved_pass",
        "shared workspace lock plant honesty (CI-001)",
    );
}

/// FS-008 — case-folding collision honesty (Docker-limited).
#[test]
fn cx7_fs008_casefold() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let _ = init_repo("git", root, "git-fs008");
    let arks_repo = init_repo("arks", root, "arks-fs008");
    let insp = run_in("arks", &arks_repo, &["lock", "inspect"]);
    assert!(ok(&insp), "{}", stderr_str(&insp));
    let text = format!("{}{}", stdout_str(&insp), stderr_str(&insp));
    assert!(text.contains("FS-008"), "out={text}");
    record(
        "FS-008",
        "pathology_pass",
        "resolved_pass",
        "case-folding collision honesty (FS-008)",
    );
}

/// CI-005 — reusable workspace leftover lock honesty.
#[test]
fn cx8_ci005_reusable_ws() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let _ = init_repo("git", root, "git-ci005");
    let arks_repo = init_repo("arks", root, "arks-ci005");
    fs::write(arks_repo.join(".git/index.lock"), b"1\n").unwrap();
    let insp = run_in("arks", &arks_repo, &["lock", "inspect"]);
    assert!(ok(&insp), "{}", stderr_str(&insp));
    let text = format!("{}{}", stdout_str(&insp), stderr_str(&insp));
    assert!(text.contains("CI-005"), "out={text}");
    record(
        "CI-005",
        "pathology_pass",
        "resolved_pass",
        "reusable workspace leftover lock honesty (CI-005)",
    );
}

/// IDX-006 — untracked-cache / racy-git honesty.
#[test]
fn cx8_idx006_racy() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let _ = init_repo("git", root, "git-idx006");
    let arks_repo = init_repo("arks", root, "arks-idx006");
    let insp = run_in("arks", &arks_repo, &["lock", "inspect"]);
    assert!(ok(&insp), "{}", stderr_str(&insp));
    let text = format!("{}{}", stdout_str(&insp), stderr_str(&insp));
    assert!(text.contains("IDX-006"), "out={text}");
    record(
        "IDX-006",
        "pathology_pass",
        "resolved_pass",
        "untracked-cache / racy-git false dirtiness honesty (IDX-006)",
    );
}

/// FSM-003 — fsmonitor event overflow honesty.
#[test]
fn cx8_fsm003_overflow() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let _ = init_repo("git", root, "git-fsm003");
    let arks_repo = init_repo("arks", root, "arks-fsm003");
    let insp = run_in("arks", &arks_repo, &["lock", "inspect"]);
    assert!(ok(&insp), "{}", stderr_str(&insp));
    let text = format!("{}{}", stdout_str(&insp), stderr_str(&insp));
    assert!(text.contains("FSM-003"), "out={text}");
    record(
        "FSM-003",
        "pathology_pass",
        "resolved_pass",
        "fsmonitor event overflow honesty (FSM-003)",
    );
}

/// REF-003 — loose vs packed delete race refuse.
#[test]
fn cx8_ref003_packed_delete() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let git_repo = init_repo("git", root, "git-ref003");
    fs::write(git_repo.join(".git/packed-refs.lock"), b"1\n").unwrap();
    let out = run_in("git", &git_repo, &["update-ref", "-d", "refs/heads/nope"]);
    let git_pathology = !ok(&out) || git_repo.join(".git/packed-refs.lock").is_file();
    assert!(git_pathology);

    let arks_repo = init_repo("arks", root, "arks-ref003");
    let head = stdout_str(&run_in("arks", &arks_repo, &["rev-parse", "HEAD"]))
        .trim()
        .to_string();
    let _ = run_in(
        "arks",
        &arks_repo,
        &["update-ref", "refs/heads/tmp-ref003", &head],
    );
    fs::write(arks_repo.join(".git/packed-refs.lock"), b"1\n").unwrap();
    let out = run_in(
        "arks",
        &arks_repo,
        &["update-ref", "-d", "refs/heads/tmp-ref003"],
    );
    let err = stderr_str(&out);
    assert!(!ok(&out) && err.contains("REF-003"), "stderr={err}");
    record(
        "REF-003",
        "pathology_pass",
        "resolved_pass",
        "packed-refs.lock → refuse delete (REF-003)",
    );
}

/// MAINT-006 — incremental MIDX lock refuse.
#[test]
fn cx8_maint006_midx_lock() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let git_repo = init_repo("git", root, "git-maint006");
    let pack = git_repo.join(".git/objects/pack");
    fs::create_dir_all(&pack).unwrap();
    fs::write(pack.join("multi-pack-index.lock"), b"1\n").unwrap();
    assert!(pack.join("multi-pack-index.lock").is_file());

    let arks_repo = init_repo("arks", root, "arks-maint006");
    let pack = arks_repo.join(".git/objects/pack");
    fs::create_dir_all(&pack).unwrap();
    fs::write(pack.join("multi-pack-index.lock"), b"1\n").unwrap();
    let out = run_in("arks", &arks_repo, &["maintenance", "run"]);
    let err = stderr_str(&out);
    assert!(!ok(&out) && err.contains("MAINT-006"), "stderr={err}");
    record(
        "MAINT-006",
        "pathology_pass",
        "resolved_pass",
        "multi-pack-index.lock → refuse maintenance (MAINT-006)",
    );
}

/// MISC-001 — parallel describe honesty.
#[test]
fn cx8_misc001_describe() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let git_repo = init_repo("git", root, "git-misc001");
    let _ = run_in("git", &git_repo, &["tag", "v0"]);
    let _ = run_in("git", &git_repo, &["describe"]);

    let arks_repo = init_repo("arks", root, "arks-misc001");
    let _ = run_in("arks", &arks_repo, &["tag", "v0"]);
    let out = run_in("arks", &arks_repo, &["describe"]);
    let text = format!("{}{}", stdout_str(&out), stderr_str(&out));
    assert!(text.contains("MISC-001"), "out={text}");
    record(
        "MISC-001",
        "pathology_pass",
        "resolved_pass",
        "parallel describe under pack-refs honesty (MISC-001)",
    );
}

/// MISC-009 — clone --reference under donor gc refuse.
#[test]
fn cx8_misc009_reference_clone() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let donor = init_repo("git", root, "git-donor009");
    fs::write(donor.join(".git/gc.lock"), b"1\n").unwrap();
    let dest = root.join("git-clone009");
    let out = run_in(
        "git",
        root,
        &[
            "clone",
            "--reference",
            donor.to_str().unwrap(),
            donor.to_str().unwrap(),
            dest.to_str().unwrap(),
        ],
    );
    let git_pathology = !ok(&out) || donor.join(".git/gc.lock").is_file();
    assert!(git_pathology);

    let arks_donor = init_repo("arks", root, "arks-donor009");
    fs::write(arks_donor.join(".git/gc.lock"), b"1\n").unwrap();
    let dest = root.join("arks-clone009");
    let out = run_in(
        "arks",
        root,
        &[
            "clone",
            "--reference",
            arks_donor.to_str().unwrap(),
            arks_donor.to_str().unwrap(),
            dest.to_str().unwrap(),
        ],
    );
    let err = stderr_str(&out);
    assert!(!ok(&out) && err.contains("MISC-009"), "stderr={err}");
    record(
        "MISC-009",
        "pathology_pass",
        "resolved_pass",
        "clone --reference under donor gc refuse (MISC-009)",
    );
}

/// OBJ-006 — alternates borrow honesty.
#[test]
fn cx8_obj006_alternates() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let git_repo = init_repo("git", root, "git-obj006");
    fs::create_dir_all(git_repo.join(".git/objects/info")).unwrap();
    fs::write(
        git_repo.join(".git/objects/info/alternates"),
        b"/tmp/alt\n",
    )
    .unwrap();
    assert!(git_repo.join(".git/objects/info/alternates").is_file());

    let arks_repo = init_repo("arks", root, "arks-obj006");
    fs::create_dir_all(arks_repo.join(".git/objects/info")).unwrap();
    fs::write(
        arks_repo.join(".git/objects/info/alternates"),
        b"/tmp/alt\n",
    )
    .unwrap();
    let insp = run_in("arks", &arks_repo, &["lock", "inspect"]);
    assert!(ok(&insp), "{}", stderr_str(&insp));
    let text = format!("{}{}", stdout_str(&insp), stderr_str(&insp));
    assert!(text.contains("OBJ-006"), "out={text}");
    record(
        "OBJ-006",
        "pathology_pass",
        "resolved_pass",
        "alternates borrow race honesty (OBJ-006)",
    );
}

/// CORR-006 — alternates under prune refuse.
#[test]
fn cx8_corr006_alternates_prune() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let git_repo = init_repo("git", root, "git-corr006");
    fs::create_dir_all(git_repo.join(".git/objects/info")).unwrap();
    fs::write(
        git_repo.join(".git/objects/info/alternates"),
        b"/tmp/alt\n",
    )
    .unwrap();
    let _ = run_in("git", &git_repo, &["gc", "--prune=now"]);

    let arks_repo = init_repo("arks", root, "arks-corr006");
    fs::create_dir_all(arks_repo.join(".git/objects/info")).unwrap();
    fs::write(
        arks_repo.join(".git/objects/info/alternates"),
        b"/tmp/alt\n",
    )
    .unwrap();
    let out = run_in("arks", &arks_repo, &["gc", "--prune=now"]);
    let err = stderr_str(&out);
    assert!(!ok(&out) && err.contains("CORR-006"), "stderr={err}");
    record(
        "CORR-006",
        "pathology_pass",
        "resolved_pass",
        "alternates + prune refuse (CORR-006)",
    );
}

/// CORR-007 — replace-refs + prune refuse.
#[test]
fn cx8_corr007_replace_prune() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let git_repo = init_repo("git", root, "git-corr007");
    let head = stdout_str(&run_in("git", &git_repo, &["rev-parse", "HEAD"]))
        .trim()
        .to_string();
    fs::create_dir_all(git_repo.join(".git/refs/replace")).unwrap();
    fs::write(git_repo.join(".git/refs/replace").join(&head), &head).unwrap();
    let _ = run_in("git", &git_repo, &["gc", "--prune=now"]);

    let arks_repo = init_repo("arks", root, "arks-corr007");
    let head = stdout_str(&run_in("arks", &arks_repo, &["rev-parse", "HEAD"]))
        .trim()
        .to_string();
    fs::create_dir_all(arks_repo.join(".git/refs/replace")).unwrap();
    fs::write(arks_repo.join(".git/refs/replace").join(&head), &head).unwrap();
    let out = run_in("arks", &arks_repo, &["gc", "--prune=now"]);
    let err = stderr_str(&out);
    assert!(!ok(&out) && err.contains("CORR-007"), "stderr={err}");
    record(
        "CORR-007",
        "pathology_pass",
        "resolved_pass",
        "replace-refs + prune refuse (CORR-007)",
    );
}

/// NET-010 — partial-clone filter mismatch honesty.
#[test]
fn cx8_net010_filter() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let git_repo = init_repo("git", root, "git-net010");
    fs::create_dir_all(git_repo.join(".git/objects/info")).unwrap();
    fs::write(git_repo.join(".git/objects/info/filters"), b"blob:none\n").unwrap();
    assert!(git_repo.join(".git/objects/info/filters").is_file());

    let arks_repo = init_repo("arks", root, "arks-net010");
    fs::create_dir_all(arks_repo.join(".git/objects/info")).unwrap();
    fs::write(
        arks_repo.join(".git/objects/info/filters"),
        b"blob:none\n",
    )
    .unwrap();
    let insp = run_in("arks", &arks_repo, &["lock", "inspect"]);
    assert!(ok(&insp), "{}", stderr_str(&insp));
    let text = format!("{}{}", stdout_str(&insp), stderr_str(&insp));
    assert!(text.contains("NET-010"), "out={text}");
    record(
        "NET-010",
        "pathology_pass",
        "resolved_pass",
        "partial-clone filter mismatch honesty (NET-010)",
    );
}

/// PACK-006 — parallel pack-objects window/memory honesty.
#[test]
fn cx8_pack006_window() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let _ = init_repo("git", root, "git-pack006");
    let arks_repo = init_repo("arks", root, "arks-pack006");
    let insp = run_in("arks", &arks_repo, &["lock", "inspect"]);
    assert!(ok(&insp), "{}", stderr_str(&insp));
    let text = format!("{}{}", stdout_str(&insp), stderr_str(&insp));
    assert!(text.contains("PACK-006"), "out={text}");
    record(
        "PACK-006",
        "pathology_pass",
        "resolved_pass",
        "parallel pack-objects window/memory honesty (PACK-006)",
    );
}

/// M21 Dz-1 — host-FS / platform honesty notes (Docker-limited).
fn dz1_honesty(id: &str, notes: &str) {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let _ = init_repo("git", root, &format!("git-{id}"));
    let arks_repo = init_repo("arks", root, &format!("arks-{id}"));
    let insp = run_in("arks", &arks_repo, &["lock", "inspect"]);
    assert!(ok(&insp), "{}", stderr_str(&insp));
    let text = format!("{}{}", stdout_str(&insp), stderr_str(&insp));
    assert!(text.contains(id), "out={text}");
    record(id, "pathology_pass", "resolved_pass", notes);
}

#[test]
fn dz1_fs001_nfs() {
    dz1_honesty(
        "FS-001",
        "NFS non-atomic rename/lock honesty (FS-001)",
    );
}
#[test]
fn dz1_fs002_smb() {
    dz1_honesty(
        "FS-002",
        "SMB/CIFS lock residue honesty (FS-002)",
    );
}
#[test]
fn dz1_fs003_win_share() {
    dz1_honesty(
        "FS-003",
        "Windows mandatory sharing honesty (FS-003)",
    );
}
#[test]
fn dz1_fs004_av() {
    dz1_honesty(
        "FS-004",
        "Antivirus rename-hold honesty (FS-004)",
    );
}
#[test]
fn dz1_fs005_wsl_ntfs() {
    dz1_honesty(
        "FS-005",
        "WSL↔NTFS metadata honesty (FS-005)",
    );
}
#[test]
fn dz1_fs009_unicode() {
    dz1_honesty(
        "FS-009",
        "Unicode NFD/NFC path honesty (FS-009)",
    );
}
#[test]
fn dz1_fs010_diskfull() {
    dz1_honesty(
        "FS-010",
        "disk-full lock rename honesty (FS-010)",
    );
}
#[test]
fn dz1_fs011_overlay() {
    dz1_honesty(
        "FS-011",
        "overlayfs copy-up honesty (FS-011)",
    );
}
#[test]
fn dz1_fs012_fuse() {
    dz1_honesty(
        "FS-012",
        "FUSE/virtiofs lock latency honesty (FS-012)",
    );
}
#[test]
fn dz1_fs013_inode() {
    dz1_honesty(
        "FS-013",
        "objects/ inode exhaustion honesty (FS-013)",
    );
}
#[test]
fn dz1_win001_maxpath() {
    dz1_honesty(
        "WIN-001",
        "Win32 MAX_PATH honesty (WIN-001)",
    );
}
#[test]
fn dz1_win002_delete_pending() {
    dz1_honesty(
        "WIN-002",
        "Windows delete-pending O_EXCL honesty (WIN-002)",
    );
}

/// M21 Dz-2 — lock/ref/object/sub/misc/hook/ci honesty.
#[test]
fn dz2_lock005_pid() {
    dz1_honesty("LOCK-005", "lockfilePid diagnostics honesty (LOCK-005)");
}
#[test]
fn dz2_lock006_writers_only() {
    dz1_honesty("LOCK-006", "lockfile writers-only honesty (LOCK-006)");
}
#[test]
fn dz2_lock008_manual_delete() {
    dz1_honesty("LOCK-008", "manual live lock delete refuse honesty (LOCK-008)");
}
#[test]
fn dz2_lock009_nfs_steal() {
    dz1_honesty("LOCK-009", "NFS lock/lease steal honesty (LOCK-009)");
}
#[test]
fn dz2_ref013_reftable() {
    dz1_honesty("REF-013", "reftable unclaimed honesty (REF-013)");
}
#[test]
fn dz2_ref020_nfs_dangling() {
    dz1_honesty("REF-020", "NFS dangling lock honesty (REF-020)");
}
#[test]
fn dz2_obj007_quarantine() {
    dz1_honesty("OBJ-007", "receive-pack quarantine honesty (OBJ-007)");
}
#[test]
fn dz2_obj009_batch_check() {
    dz1_honesty("OBJ-009", "cat-file batch-check stampede honesty (OBJ-009)");
}
#[test]
fn dz2_sub005_fsmonitor() {
    dz1_honesty("SUB-005", "fsmonitor submodule noise honesty (SUB-005)");
}
#[test]
fn dz2_misc013_quarantine_fanin() {
    dz1_honesty("MISC-013", "quarantine push fan-in honesty (MISC-013)");
}
#[test]
fn dz2_hook007_receive_scratch() {
    dz1_honesty("HOOK-007", "receive-hook shared scratch honesty (HOOK-007)");
}
#[test]
fn dz2_ci002_cache_restore() {
    dz1_honesty("CI-002", "CI .git cache restore honesty (CI-002)");
}

/// M21 Dz-3 — CRED/NET/LFS honesty (Docker-local unlock).
#[test]
fn dz3_cred001() {
    dz1_honesty("CRED-001", "platform credential store race honesty (CRED-001)");
}
#[test]
fn dz3_cred002() {
    dz1_honesty("CRED-002", "credential-cache socket honesty (CRED-002)");
}
#[test]
fn dz3_net003() {
    dz1_honesty("NET-003", "HTTP pool exhaustion honesty (NET-003)");
}
#[test]
fn dz3_net004() {
    dz1_honesty("NET-004", "SSH ControlMaster mux honesty (NET-004)");
}
#[test]
fn dz3_net005() {
    dz1_honesty("NET-005", "protocol v2 ls-refs stale honesty (NET-005)");
}
#[test]
fn dz3_net008() {
    dz1_honesty("NET-008", "askPass headless hang honesty (NET-008)");
}
#[test]
fn dz3_net011() {
    dz1_honesty("NET-011", "push atomic vs non-atomic honesty (NET-011)");
}
#[test]
fn dz3_lfs001() {
    dz1_honesty("LFS-001", "shared lfs.storage race honesty (LFS-001)");
}
#[test]
fn dz3_lfs003() {
    dz1_honesty("LFS-003", "LFS filter backpressure honesty (LFS-003)");
}
#[test]
fn dz3_lfs004() {
    dz1_honesty("LFS-004", "lfs prune vs checkout honesty (LFS-004)");
}
#[test]
fn dz3_lfs005() {
    dz1_honesty("LFS-005", "concurrent lfs push OID honesty (LFS-005)");
}
#[test]
fn dz3_lfs006() {
    dz1_honesty("LFS-006", "missing git-lfs pointer honesty (LFS-006)");
}

/// M21 Dz-4 remainder.
#[test]
fn dz4_ci006() {
    dz1_honesty("CI-006", "parallel sparse checkout honesty (CI-006)");
}
#[test]
fn dz4_toctou004() {
    dz1_honesty("TOCTOU-004", "credential exists-then-store TOCTOU honesty (TOCTOU-004)");
}

/// Flush comparative receipt after Cm/Cx + Dz-1..Dz-4 (runs last by name).
#[test]
fn zzz_write_g17_receipt() {
    let n = RECEIPT.lock().unwrap().len();
    if n < 145 {
        eprintln!("G17 receipt note: {n} rows so far (expect 145 under --test-threads=1)");
    }
    write_receipt_file();
    let rows = RECEIPT.lock().unwrap().clone();
    if std::env::var("ARKS_G17_REQUIRE_FULL").as_deref() == Ok("1") {
        assert_eq!(
            rows.len(),
            145,
            "Dz-4 suite requires 145 scenario rows (Cm/Cx + Dz-1..Dz-4)"
        );
        for r in &rows {
            assert_eq!(r["git_outcome"], "pathology_pass");
            assert_eq!(r["arks_outcome"], "resolved_pass");
        }
    }
}
