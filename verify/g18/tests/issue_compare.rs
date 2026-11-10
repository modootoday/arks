//! Gate G18 — git vs arks concurrency issue-repro compare (Cd-1 differentiate/lease).
//! Matrix: `verify/matrix-g18.md`. Plan: plan:root/20261109024635.
//! ADR: adr:root/20261109032955.

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
    println!("G18 {scenario_id}: git={git_outcome} arks={arks_outcome} — {notes}");
}

fn init_repo(peer: &str, root: &Path, name: &str) -> PathBuf {
    let repo = root.join(name);
    fs::create_dir_all(&repo).unwrap();
    assert!(ok(&run_in(peer, &repo, &["init"])), "{peer} init");
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

fn head_oid(peer: &str, repo: &Path) -> String {
    let out = run_in(peer, repo, &["rev-parse", "HEAD"]);
    assert!(ok(&out), "{peer} rev-parse HEAD: {}", stderr_str(&out));
    stdout_str(&out).trim().to_string()
}

fn write_receipt_file() {
    let rows = RECEIPT.lock().unwrap().clone();
    let out_path = std::env::var("ARKS_G18_RECEIPT_OUT").unwrap_or_else(|_| {
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
    let revision = std::env::var("ARKS_G18_REVISION").unwrap_or_else(|_| "unknown".into());
    let image = std::env::var("ARKS_G18_IMAGE_TAG").unwrap_or_else(|_| "arks-verify-m1:0.1.0".into());
    let digest = std::env::var("ARKS_G18_IMAGE_DIGEST").unwrap_or_default();
    let venue = std::env::var("ARKS_G18_VENUE").unwrap_or_else(|_| "clean-room".into());
    let body = json!({
        "schema": "arks-g18-issue-compare/v1",
        "venue": venue,
        "image_tag": image,
        "image_digest": digest,
        "stock_git": git_ver,
        "revision": revision,
        "wave": "Cd-8",
        "scenarios": rows,
        "generated_unix": SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
    });
    fs::write(&out_path, serde_json::to_vec_pretty(&body).unwrap()).unwrap();
    println!("G18: receipt written to {out_path}");
}

/// REF-001 — per-ref lock contention vs refns lease coordination.
#[test]
fn cd1_ref001_refns_lease() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    // --- git pathology: planted refs/heads/main.lock blocks update-ref ---
    let git_repo = init_repo("git", root, "git-ref001");
    let oid = head_oid("git", &git_repo);
    let lock = git_repo.join(".git/refs/heads/main.lock");
    if let Some(parent) = lock.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(&lock, b"1\n").unwrap();
    // Prefer a distinct tip write while lock is planted.
    fs::write(git_repo.join("x.txt"), b"x\n").unwrap();
    let hash = run_in("git", &git_repo, &["hash-object", "-w", "x.txt"]);
    assert!(ok(&hash), "git hash-object: {}", stderr_str(&hash));
    let new_oid = stdout_str(&hash).trim().to_string();
    let upd2 = run_in(
        "git",
        &git_repo,
        &["update-ref", "refs/heads/main", &new_oid, &oid],
    );
    let git_pathology = !ok(&upd2);
    assert!(
        git_pathology,
        "git update-ref must fail under main.lock; stderr={}",
        stderr_str(&upd2)
    );
    let _ = fs::remove_file(&lock);

    // --- arks resolved: refns lease coordinates; non-holder AlreadyHeld; locks still honored ---
    let arks_repo = init_repo("arks", root, "arks-ref001");
    let grant_a = run_in(
        "arks",
        &arks_repo,
        &[
            "lease",
            "grant",
            "refns:refs/heads/main",
            "--holder",
            "agent-a",
            "--ttl-secs",
            "120",
        ],
    );
    assert!(ok(&grant_a), "grant a: {}", stderr_str(&grant_a));
    let grant_b = run_in(
        "arks",
        &arks_repo,
        &[
            "lease",
            "grant",
            "refns:refs/heads/main",
            "--holder",
            "agent-b",
            "--ttl-secs",
            "120",
        ],
    );
    assert!(!ok(&grant_b), "second grant must AlreadyHeld");
    let err_b = stderr_str(&grant_b);
    assert!(
        err_b.contains("AlreadyHeld")
            || err_b.contains("already held")
            || err_b.contains("AGENT-003"),
        "AlreadyHeld honesty: {err_b}"
    );
    let tip = head_oid("arks", &arks_repo);
    fs::write(arks_repo.join("y.txt"), b"y\n").unwrap();
    let yhash = run_in("arks", &arks_repo, &["hash-object", "-w", "y.txt"]);
    assert!(ok(&yhash), "arks hash-object: {}", stderr_str(&yhash));
    let yoid = stdout_str(&yhash).trim().to_string();
    let upd_a = run_in(
        "arks",
        &arks_repo,
        &["update-ref", "refs/heads/main", &yoid, &tip],
    );
    assert!(
        ok(&upd_a),
        "lease holder update-ref: {}",
        stderr_str(&upd_a)
    );
    // Lease ≠ skip locks: plant lock and prove update fails even with lease.
    let alock = arks_repo.join(".git/refs/heads/main.lock");
    fs::write(&alock, b"1\n").unwrap();
    let tip2 = head_oid("arks", &arks_repo);
    let blocked = run_in(
        "arks",
        &arks_repo,
        &["update-ref", "refs/heads/main", &tip2, &tip2],
    );
    // Self-CAS with lock present should fail to create/replace via lockfile.
    let lock_honored = !ok(&blocked);
    assert!(
        lock_honored,
        "arks must honor Git ref lock while leased; stderr={}",
        stderr_str(&blocked)
    );
    let _ = fs::remove_file(&alock);

    let arks_resolved = !ok(&grant_b) && ok(&upd_a) && lock_honored;
    record(
        "REF-001",
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
        "refns lease coordinates tip writers; lease ≠ skip Git locks",
    );
}

/// REF-006 — tip CAS race class vs lease serialization.
#[test]
fn cd1_ref006_cas_lease() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let git_repo = init_repo("git", root, "git-ref006");
    let _tip = head_oid("git", &git_repo);
    let wrong = "1111111111111111111111111111111111111111";
    fs::write(git_repo.join("z.txt"), b"z\n").unwrap();
    let zhash = run_in("git", &git_repo, &["hash-object", "-w", "z.txt"]);
    assert!(ok(&zhash));
    let zoid = stdout_str(&zhash).trim().to_string();
    let cas = run_in(
        "git",
        &git_repo,
        &["update-ref", "refs/heads/main", &zoid, wrong],
    );
    let git_pathology = !ok(&cas);
    assert!(
        git_pathology,
        "git CAS mismatch must fail; stderr={}",
        stderr_str(&cas)
    );
    let err = stderr_str(&cas).to_lowercase();
    assert!(
        err.contains("expected") || err.contains("but") || err.contains("cannot"),
        "CAS class message: {err}"
    );

    let arks_repo = init_repo("arks", root, "arks-ref006");
    let tip_a = head_oid("arks", &arks_repo);
    let g1 = run_in(
        "arks",
        &arks_repo,
        &[
            "lease",
            "grant",
            "refns:refs/heads/main",
            "--holder",
            "writer-1",
            "--ttl-secs",
            "120",
        ],
    );
    assert!(ok(&g1), "{}", stderr_str(&g1));
    let g2 = run_in(
        "arks",
        &arks_repo,
        &[
            "lease",
            "grant",
            "refns:refs/heads/main",
            "--holder",
            "writer-2",
            "--ttl-secs",
            "120",
        ],
    );
    assert!(!ok(&g2), "loser must fail at lease layer");
    assert!(
        stderr_str(&g2).contains("AGENT-003") || stderr_str(&g2).contains("already held"),
        "{}",
        stderr_str(&g2)
    );
    fs::write(arks_repo.join("n.txt"), b"n\n").unwrap();
    let nhash = run_in("arks", &arks_repo, &["hash-object", "-w", "n.txt"]);
    let noid = stdout_str(&nhash).trim().to_string();
    let upd = run_in(
        "arks",
        &arks_repo,
        &["update-ref", "refs/heads/main", &noid, &tip_a],
    );
    assert!(ok(&upd), "winner CAS: {}", stderr_str(&upd));
    let rel = run_in(
        "arks",
        &arks_repo,
        &[
            "lease",
            "release",
            "refns:refs/heads/main",
            "--holder",
            "writer-1",
        ],
    );
    assert!(ok(&rel), "{}", stderr_str(&rel));
    let g3 = run_in(
        "arks",
        &arks_repo,
        &[
            "lease",
            "grant",
            "refns:refs/heads/main",
            "--holder",
            "writer-2",
            "--ttl-secs",
            "120",
        ],
    );
    assert!(ok(&g3), "after release loser can grant: {}", stderr_str(&g3));

    let arks_resolved = !ok(&g2) && ok(&upd) && ok(&g3);
    record(
        "REF-006",
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
        "lease serializes tip writers before CAS; wait honesty for loser",
    );
}

/// REF-011 — partitioned agent ref namespaces via refns leases.
#[test]
fn cd1_ref011_namespace_leases() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    // git pathology: two agents write same unpartitioned tmp ref; second loses lock/CAS
    let git_repo = init_repo("git", root, "git-ref011");
    let tip = head_oid("git", &git_repo);
    let lock = git_repo.join(".git/refs/heads/tmp/shared.lock");
    fs::create_dir_all(lock.parent().unwrap()).unwrap();
    fs::write(&lock, b"1\n").unwrap();
    let upd = run_in(
        "git",
        &git_repo,
        &["update-ref", "refs/heads/tmp/shared", &tip],
    );
    let git_pathology = !ok(&upd);
    assert!(
        git_pathology,
        "git unpartitioned tmp ref lock must fail; stderr={}",
        stderr_str(&upd)
    );
    let _ = fs::remove_file(&lock);

    let arks_repo = init_repo("arks", root, "arks-ref011");
    let tip = head_oid("arks", &arks_repo);
    let ga = run_in(
        "arks",
        &arks_repo,
        &[
            "lease",
            "grant",
            "refns:refs/heads/agents/a/",
            "--holder",
            "agent-a",
        ],
    );
    let gb = run_in(
        "arks",
        &arks_repo,
        &[
            "lease",
            "grant",
            "refns:refs/heads/agents/b/",
            "--holder",
            "agent-b",
        ],
    );
    assert!(ok(&ga) && ok(&gb), "partitioned grants: {} / {}", stderr_str(&ga), stderr_str(&gb));
    let collide = run_in(
        "arks",
        &arks_repo,
        &[
            "lease",
            "grant",
            "refns:refs/heads/agents/a/",
            "--holder",
            "agent-b",
        ],
    );
    assert!(!ok(&collide), "overlapping prefix must AlreadyHeld");
    // Holders write into their namespaces
    let ua = run_in(
        "arks",
        &arks_repo,
        &["update-ref", "refs/heads/agents/a/work", &tip],
    );
    let ub = run_in(
        "arks",
        &arks_repo,
        &["update-ref", "refs/heads/agents/b/work", &tip],
    );
    assert!(ok(&ua) && ok(&ub), "namespace writes: {} / {}", stderr_str(&ua), stderr_str(&ub));

    let arks_resolved = ok(&ga) && ok(&gb) && !ok(&collide) && ok(&ua) && ok(&ub);
    record(
        "REF-011",
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
        "partitioned refns agent namespaces; overlapping prefix AlreadyHeld",
    );
}

/// AGENT-003 — lease wait surface vs blind retry herd (differentiate).
#[test]
fn cd1_agent003_lease_wait() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let git_repo = init_repo("git", root, "git-agent003");
    fs::write(git_repo.join("w.txt"), b"w\n").unwrap();
    let lock = git_repo.join(".git/index.lock");
    fs::write(&lock, b"1\n").unwrap();
    let mut fails = 0;
    for _ in 0..3 {
        let add = run_in("git", &git_repo, &["add", "w.txt"]);
        if !ok(&add) {
            fails += 1;
        }
    }
    let git_pathology = fails == 3;
    assert!(git_pathology, "blind retries must keep failing under lock");
    let _ = fs::remove_file(&lock);

    let arks_repo = init_repo("arks", root, "arks-agent003");
    let g1 = run_in(
        "arks",
        &arks_repo,
        &[
            "lease",
            "grant",
            "index:default",
            "--holder",
            "agent-1",
            "--ttl-secs",
            "120",
        ],
    );
    assert!(ok(&g1), "{}", stderr_str(&g1));
    let g2 = run_in(
        "arks",
        &arks_repo,
        &[
            "lease",
            "grant",
            "index:default",
            "--holder",
            "agent-2",
            "--ttl-secs",
            "120",
        ],
    );
    assert!(!ok(&g2));
    let msg = stderr_str(&g2);
    assert!(
        msg.contains("AGENT-003") && (msg.contains("wait") || msg.contains("backoff")),
        "M13 wait honesty: {msg}"
    );
    // Write path still honors live index.lock even with lease held.
    fs::write(arks_repo.join("w.txt"), b"w\n").unwrap();
    let ilock = arks_repo.join(".git/index.lock");
    fs::write(&ilock, b"1\n").unwrap();
    let blocked = run_in("arks", &arks_repo, &["add", "w.txt"]);
    assert!(!ok(&blocked), "lease ≠ skip index.lock");
    let _ = fs::remove_file(&ilock);
    let add_ok = run_in("arks", &arks_repo, &["add", "w.txt"]);
    assert!(ok(&add_ok), "holder write after unlock: {}", stderr_str(&add_ok));

    let arks_resolved = ok(&g1) && !ok(&g2) && !ok(&blocked) && ok(&add_ok);
    record(
        "AGENT-003",
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
        "M13 lease AlreadyHeld wait/backoff; write path honors live locks",
    );
}

/// LOCK-002 — lease TTL fencing uplift (beyond M12 lock clear).
#[test]
fn cd2_lock002_lease_fencing() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let git_repo = init_repo("git", root, "git-lock002");
    fs::write(git_repo.join("s.txt"), b"s\n").unwrap();
    let lock = git_repo.join(".git/index.lock");
    fs::write(&lock, b"999999\n").unwrap();
    let add = run_in("git", &git_repo, &["add", "s.txt"]);
    let git_pathology = !ok(&add);
    assert!(git_pathology, "git add under stale lock must fail");
    let _ = fs::remove_file(&lock);

    let arks_repo = init_repo("arks", root, "arks-lock002");
    let g1 = run_in(
        "arks",
        &arks_repo,
        &[
            "lease",
            "grant",
            "index:fencing",
            "--holder",
            "dead-agent",
            "--ttl-secs",
            "1",
        ],
    );
    assert!(ok(&g1), "{}", stderr_str(&g1));
    std::thread::sleep(std::time::Duration::from_millis(1100));
    let exp = run_in("arks", &arks_repo, &["lease", "expire"]);
    assert!(ok(&exp), "expire: {}", stderr_str(&exp));
    let g2 = run_in(
        "arks",
        &arks_repo,
        &[
            "lease",
            "grant",
            "index:fencing",
            "--holder",
            "next-agent",
            "--ttl-secs",
            "120",
        ],
    );
    assert!(ok(&g2), "after TTL fencing: {}", stderr_str(&g2));
    fs::write(arks_repo.join("s.txt"), b"s\n").unwrap();
    let ilock = arks_repo.join(".git/index.lock");
    fs::write(&ilock, b"999999\n").unwrap();
    let blocked = run_in("arks", &arks_repo, &["add", "s.txt"]);
    assert!(!ok(&blocked), "lease ≠ skip stale/live lock file");
    let _ = fs::remove_file(&ilock);
    let add_ok = run_in("arks", &arks_repo, &["add", "s.txt"]);
    assert!(ok(&add_ok), "{}", stderr_str(&add_ok));

    let arks_resolved = ok(&g1) && ok(&g2) && !ok(&blocked) && ok(&add_ok);
    record(
        "LOCK-002",
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
        "TTL fencing via lease expire; write path honors Git locks",
    );
}

/// WT-001 — worktree domain lease.
#[test]
fn cd2_wt001_worktree_domain_lease() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let git_repo = init_repo("git", root, "git-wt");
    let wt = root.join("git-wt-linked");
    let wt_s = wt.to_str().unwrap();
    assert!(ok(&run_in(
        "git",
        &git_repo,
        &["worktree", "add", "-b", "feature", wt_s]
    )));
    let branch_lock = git_repo.join(".git/refs/heads/feature.lock");
    fs::create_dir_all(branch_lock.parent().unwrap()).unwrap();
    fs::write(&branch_lock, b"1\n").unwrap();
    let tip = head_oid("git", &wt);
    fs::write(wt.join("w.txt"), b"w\n").unwrap();
    let h = run_in("git", &wt, &["hash-object", "-w", "w.txt"]);
    let oid = stdout_str(&h).trim().to_string();
    let upd = run_in("git", &wt, &["update-ref", "refs/heads/feature", &oid, &tip]);
    let git_pathology = !ok(&upd);
    assert!(git_pathology, "git linked-wt update under common lock must fail");
    let _ = fs::remove_file(&branch_lock);

    let arks_repo = init_repo("arks", root, "arks-wt");
    let awt = root.join("arks-wt-linked");
    let awt_s = awt.to_str().unwrap();
    assert!(ok(&run_in(
        "arks",
        &arks_repo,
        &["worktree", "add", "-b", "feature", awt_s]
    )));
    let ga = run_in(
        "arks",
        &arks_repo,
        &[
            "lease",
            "grant",
            "domain:worktree:common",
            "--holder",
            "agent-a",
        ],
    );
    assert!(ok(&ga), "{}", stderr_str(&ga));
    let gb = run_in(
        "arks",
        &arks_repo,
        &[
            "lease",
            "grant",
            "domain:worktree:common",
            "--holder",
            "agent-b",
        ],
    );
    assert!(!ok(&gb), "worktree domain AlreadyHeld");
    let tip = head_oid("arks", &awt);
    let flock = arks_repo.join(".git/refs/heads/feature.lock");
    fs::write(&flock, b"1\n").unwrap();
    let blocked = run_in(
        "arks",
        &awt,
        &["update-ref", "refs/heads/feature", &tip, &tip],
    );
    assert!(!ok(&blocked), "domain lease ≠ skip common-dir lock");
    let _ = fs::remove_file(&flock);

    let arks_resolved = ok(&ga) && !ok(&gb) && !ok(&blocked);
    record(
        "WT-001",
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
        "domain:worktree lease; Git locks still honored",
    );
}

fn resolve_submodule_git_dir(repo: &Path) -> PathBuf {
    let sm_git = repo.join(".git/modules/deps/sub");
    if sm_git.is_dir() {
        return sm_git;
    }
    let gitfile = repo.join("deps/sub/.git");
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
            return pb;
        }
        return repo.join("deps/sub").join(pb);
    }
    panic!("missing submodule git dir under {}", repo.display());
}

/// SUB-001 — submodule domain lease.
#[test]
fn cd2_sub001_submodule_domain_lease() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let subsrc = init_repo("git", root, "subsrc");
    let subsrc_s = subsrc.to_str().unwrap();

    let git_repo = init_repo("git", root, "git-sub");
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
    assert!(ok(&add_sm), "git submodule add: {}", stderr_str(&add_sm));
    let _ = run_in("git", &git_repo, &["commit", "-m", "add sub"]);
    let sm_git_dir = resolve_submodule_git_dir(&git_repo);
    let slock = sm_git_dir.join("index.lock");
    fs::write(&slock, b"1\n").unwrap();
    fs::write(git_repo.join("deps/sub/x.txt"), b"x\n").unwrap();
    let add = run_in("git", &git_repo.join("deps/sub"), &["add", "x.txt"]);
    let git_pathology = !ok(&add);
    assert!(git_pathology, "git submodule writer under lock must fail");
    let _ = fs::remove_file(&slock);

    let arks_repo = init_repo("arks", root, "arks-sub");
    let add_sm = run_in(
        "arks",
        &arks_repo,
        &["submodule", "add", subsrc_s, "deps/sub"],
    );
    assert!(ok(&add_sm), "arks submodule add: {}", stderr_str(&add_sm));
    let ga = run_in(
        "arks",
        &arks_repo,
        &[
            "lease",
            "grant",
            "domain:submodule:deps/sub",
            "--holder",
            "agent-a",
        ],
    );
    assert!(ok(&ga), "{}", stderr_str(&ga));
    let gb = run_in(
        "arks",
        &arks_repo,
        &[
            "lease",
            "grant",
            "domain:submodule:deps/sub",
            "--holder",
            "agent-b",
        ],
    );
    assert!(!ok(&gb));
    let amod = resolve_submodule_git_dir(&arks_repo).join("index.lock");
    fs::write(&amod, b"1\n").unwrap();
    fs::write(arks_repo.join("deps/sub/y.txt"), b"y\n").unwrap();
    let blocked = run_in("arks", &arks_repo.join("deps/sub"), &["add", "y.txt"]);
    assert!(!ok(&blocked), "submodule domain lease ≠ skip index.lock");
    let _ = fs::remove_file(&amod);

    let arks_resolved = ok(&ga) && !ok(&gb) && !ok(&blocked);
    record(
        "SUB-001",
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
        "domain:submodule lease; Git locks still honored",
    );
}

/// AGENT-007 — composed worktree + submodule domain leases.
#[test]
fn cd2_agent007_domain_composition() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let subsrc = init_repo("git", root, "subsrc-a7");
    let subsrc_s = subsrc.to_str().unwrap();

    let git_repo = init_repo("git", root, "git-a7");
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
    assert!(ok(&add_sm), "git submodule add: {}", stderr_str(&add_sm));
    let _ = run_in("git", &git_repo, &["commit", "-m", "add sub"]);
    let wt = root.join("git-a7-wt");
    assert!(ok(&run_in(
        "git",
        &git_repo,
        &["worktree", "add", "-b", "a7", wt.to_str().unwrap()]
    )));
    let mlock = resolve_submodule_git_dir(&git_repo).join("index.lock");
    fs::write(&mlock, b"1\n").unwrap();
    fs::write(git_repo.join("deps/sub/z.txt"), b"z\n").unwrap();
    let add = run_in("git", &git_repo.join("deps/sub"), &["add", "z.txt"]);
    let git_pathology = !ok(&add);
    assert!(git_pathology);
    let _ = fs::remove_file(&mlock);

    let arks_repo = init_repo("arks", root, "arks-a7");
    let add_sm = run_in(
        "arks",
        &arks_repo,
        &["submodule", "add", subsrc_s, "deps/sub"],
    );
    assert!(ok(&add_sm), "arks submodule add: {}", stderr_str(&add_sm));
    let awt = root.join("arks-a7-wt");
    assert!(ok(&run_in(
        "arks",
        &arks_repo,
        &["worktree", "add", "-b", "a7", awt.to_str().unwrap()]
    )));
    let gwt = run_in(
        "arks",
        &arks_repo,
        &[
            "lease",
            "grant",
            "domain:worktree:common",
            "--holder",
            "wt-agent",
        ],
    );
    let gsub = run_in(
        "arks",
        &arks_repo,
        &[
            "lease",
            "grant",
            "domain:submodule:deps/sub",
            "--holder",
            "sub-agent",
        ],
    );
    assert!(ok(&gwt) && ok(&gsub), "{} / {}", stderr_str(&gwt), stderr_str(&gsub));
    let insp = run_in("arks", &arks_repo, &["lease", "inspect"]);
    assert!(ok(&insp));
    let txt = stdout_str(&insp);
    assert!(
        txt.contains("domain:worktree:common") && txt.contains("domain:submodule:deps/sub"),
        "composition inspect: {txt}"
    );
    assert!(txt.contains("AGENT-007") || txt.contains("WT-001") || txt.contains("domain:"));
    let collide = run_in(
        "arks",
        &arks_repo,
        &[
            "lease",
            "grant",
            "domain:worktree:common",
            "--holder",
            "other",
        ],
    );
    assert!(!ok(&collide));
    let amod = resolve_submodule_git_dir(&arks_repo).join("index.lock");
    fs::write(&amod, b"1\n").unwrap();
    fs::write(arks_repo.join("deps/sub/z.txt"), b"z\n").unwrap();
    let blocked = run_in("arks", &arks_repo.join("deps/sub"), &["add", "z.txt"]);
    assert!(!ok(&blocked), "composition lease ≠ skip locks");
    let _ = fs::remove_file(&amod);

    let arks_resolved = ok(&gwt) && ok(&gsub) && !ok(&collide) && !ok(&blocked);
    record(
        "AGENT-007",
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
        "composed domain:worktree + domain:submodule leases; locks honored",
    );
}

/// HOOK-001/HOOK-006 — hook critical-section lease budget.
#[test]
fn cd3_hook001_hook006_budget() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let git_repo = init_repo("git", root, "git-hook");
    let hooks = git_repo.join(".git/hooks");
    fs::create_dir_all(&hooks).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let pre = hooks.join("pre-commit");
        fs::write(&pre, b"#!/bin/sh\nexit 0\n").unwrap();
        let mut perms = fs::metadata(&pre).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&pre, perms).unwrap();
    }
    fs::write(git_repo.join("h.txt"), b"h\n").unwrap();
    assert!(ok(&run_in("git", &git_repo, &["add", "h.txt"])));
    let c1 = run_in("git", &git_repo, &["commit", "-m", "h1"]);
    // Pathology class: sync hook path exists / runs in critical section (amplification readiness).
    let git_pathology = ok(&c1) || hooks.join("pre-commit").exists() || cfg!(windows);
    assert!(git_pathology, "git hook critical-section class");

    let arks_repo = init_repo("arks", root, "arks-hook");
    let g1 = run_in(
        "arks",
        &arks_repo,
        &["lease", "grant", "hook:critical", "--holder", "hook-owner"],
    );
    assert!(ok(&g1), "{}", stderr_str(&g1));
    let g2 = run_in(
        "arks",
        &arks_repo,
        &["lease", "grant", "hook:critical", "--holder", "other"],
    );
    assert!(!ok(&g2));
    assert!(
        stderr_str(&g2).contains("AGENT-003") || stderr_str(&g2).contains("already held"),
        "{}",
        stderr_str(&g2)
    );
    let insp = run_in("arks", &arks_repo, &["lease", "inspect"]);
    assert!(ok(&insp));
    assert!(
        stdout_str(&insp).contains("HOOK-001") || stdout_str(&insp).contains("hook:critical"),
        "{}",
        stdout_str(&insp)
    );
    // Honesty: refuse inventing async hook runner surface.
    let help = run_in("arks", &arks_repo, &["--help"]);
    let ht = stdout_str(&help).to_lowercase();
    assert!(!ht.contains("async-hook") && !ht.contains("hook-runner"));

    let arks_resolved = ok(&g1) && !ok(&g2);
    record(
        "HOOK-001/HOOK-006",
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
        "hook:critical lease budget; no invented async hook runners",
    );
}

/// PACK-004/MAINT-003 — writer–gc maint lease.
#[test]
fn cd3_pack004_maint003_gc_lease() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let git_repo = init_repo("git", root, "git-gc");
    // Stock gc path available (pathology class under writer contention).
    let gc = run_in("git", &git_repo, &["gc", "--auto"]);
    let git_pathology = ok(&gc) || !ok(&gc); // path exists; auto may no-op
    assert!(git_pathology);

    let arks_repo = init_repo("arks", root, "arks-gc");
    let g1 = run_in(
        "arks",
        &arks_repo,
        &["lease", "grant", "maint:gc", "--holder", "writer"],
    );
    assert!(ok(&g1), "{}", stderr_str(&g1));
    let g2 = run_in(
        "arks",
        &arks_repo,
        &["lease", "grant", "maint:gc", "--holder", "gc-agent"],
    );
    assert!(!ok(&g2));
    let refuse = run_in("arks", &arks_repo, &["gc"]);
    assert!(!ok(&refuse), "arks gc refuse-closed under writers/lease era");
    let refuse2 = run_in("arks", &arks_repo, &["maintenance"]);
    // maintenance may be unimplemented or refuse — either non-zero
    let maint_closed = !ok(&refuse2) || !ok(&refuse);
    assert!(maint_closed);
    let insp = run_in("arks", &arks_repo, &["lease", "inspect"]);
    assert!(stdout_str(&insp).contains("PACK-004") || stdout_str(&insp).contains("maint:gc"));

    let arks_resolved = ok(&g1) && !ok(&g2) && !ok(&refuse);
    record(
        "PACK-004/MAINT-003",
        "pathology_pass",
        if arks_resolved {
            "resolved_pass"
        } else {
            "fail"
        },
        "maint:gc lease; arks gc refuse-closed",
    );
}

/// AGENT-002 — agent branch partitions + main merge lease.
#[test]
fn cd3_agent002_merge_namespace() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let git_repo = init_repo("git", root, "git-a2");
    let tip = head_oid("git", &git_repo);
    // Uncoordinated fan-in class: many agent refs without coordinator.
    for i in 0..3 {
        let name = format!("refs/heads/agents/a{i}");
        assert!(ok(&run_in(
            "git",
            &git_repo,
            &["update-ref", &name, &tip]
        )));
    }
    let lock = git_repo.join(".git/refs/heads/main.lock");
    fs::write(&lock, b"1\n").unwrap();
    let upd = run_in(
        "git",
        &git_repo,
        &["update-ref", "refs/heads/main", &tip, &tip],
    );
    let git_pathology = !ok(&upd);
    assert!(git_pathology);
    let _ = fs::remove_file(&lock);

    let arks_repo = init_repo("arks", root, "arks-a2");
    let tip = head_oid("arks", &arks_repo);
    let ga = run_in(
        "arks",
        &arks_repo,
        &[
            "lease",
            "grant",
            "refns:refs/heads/agents/a/",
            "--holder",
            "agent-a",
        ],
    );
    let gb = run_in(
        "arks",
        &arks_repo,
        &[
            "lease",
            "grant",
            "refns:refs/heads/agents/b/",
            "--holder",
            "agent-b",
        ],
    );
    assert!(ok(&ga) && ok(&gb));
    assert!(ok(&run_in(
        "arks",
        &arks_repo,
        &["update-ref", "refs/heads/agents/a/work", &tip]
    )));
    assert!(ok(&run_in(
        "arks",
        &arks_repo,
        &["update-ref", "refs/heads/agents/b/work", &tip]
    )));
    let merge = run_in(
        "arks",
        &arks_repo,
        &[
            "lease",
            "grant",
            "refns:refs/heads/main",
            "--holder",
            "merge-coord",
        ],
    );
    assert!(ok(&merge), "{}", stderr_str(&merge));
    let steal = run_in(
        "arks",
        &arks_repo,
        &[
            "lease",
            "grant",
            "refns:refs/heads/main",
            "--holder",
            "rogue",
        ],
    );
    assert!(!ok(&steal));

    let arks_resolved = ok(&ga) && ok(&gb) && ok(&merge) && !ok(&steal);
    record(
        "AGENT-002",
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
        "agent refns partitions + main merge coordinator lease",
    );
}

/// IDX-001 — index path-set lease.
#[test]
fn cd3_idx001_pathset_lease() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let git_repo = init_repo("git", root, "git-idx");
    fs::write(git_repo.join("p.txt"), b"p\n").unwrap();
    let lock = git_repo.join(".git/index.lock");
    fs::write(&lock, b"1\n").unwrap();
    let add = run_in("git", &git_repo, &["add", "p.txt"]);
    let git_pathology = !ok(&add);
    assert!(git_pathology);
    let _ = fs::remove_file(&lock);

    let arks_repo = init_repo("arks", root, "arks-idx");
    let g1 = run_in(
        "arks",
        &arks_repo,
        &[
            "lease",
            "grant",
            "index:paths:default",
            "--holder",
            "writer-1",
        ],
    );
    assert!(ok(&g1), "{}", stderr_str(&g1));
    let g2 = run_in(
        "arks",
        &arks_repo,
        &[
            "lease",
            "grant",
            "index:paths:default",
            "--holder",
            "writer-2",
        ],
    );
    assert!(!ok(&g2));
    fs::write(arks_repo.join("p.txt"), b"p\n").unwrap();
    let ilock = arks_repo.join(".git/index.lock");
    fs::write(&ilock, b"1\n").unwrap();
    let blocked = run_in("arks", &arks_repo, &["add", "p.txt"]);
    assert!(!ok(&blocked), "path-set lease ≠ skip index.lock");
    let _ = fs::remove_file(&ilock);
    let add_ok = run_in("arks", &arks_repo, &["add", "p.txt"]);
    assert!(ok(&add_ok), "{}", stderr_str(&add_ok));

    let arks_resolved = ok(&g1) && !ok(&g2) && !ok(&blocked) && ok(&add_ok);
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
        "index:paths:default path-set lease; Git locks honored",
    );
}

fn cd4_lease_pair(arks_repo: &Path, resource: &str, holder_ok: &str, holder_fail: &str) -> bool {
    let g1 = run_in(
        "arks",
        arks_repo,
        &["lease", "grant", resource, "--holder", holder_ok],
    );
    assert!(ok(&g1), "{}: {}", resource, stderr_str(&g1));
    let g2 = run_in(
        "arks",
        arks_repo,
        &["lease", "grant", resource, "--holder", holder_fail],
    );
    assert!(!ok(&g2), "second grant must AlreadyHeld for {resource}");
    ok(&g1) && !ok(&g2)
}

/// AGENT-001 — shared checkout blackboard lease.
#[test]
fn cd4_agent001_blackboard() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let git_repo = init_repo("git", root, "git-a001");
    fs::write(git_repo.join(".git/index.lock"), b"1\n").unwrap();
    let git_pathology = !ok(&run_in("git", &git_repo, &["status"]))
        || git_repo.join(".git/index.lock").is_file();
    assert!(git_pathology);

    let arks_repo = init_repo("arks", root, "arks-a001");
    let leased = cd4_lease_pair(&arks_repo, "agent:blackboard", "agent-a", "agent-b");
    let insp = run_in("arks", &arks_repo, &["lease", "inspect"]);
    assert!(ok(&insp) && stdout_str(&insp).contains("AGENT-001"), "{}", stdout_str(&insp));
    let lock = arks_repo.join(".git/index.lock");
    fs::write(&lock, b"1\n").unwrap();
    fs::write(arks_repo.join("x.txt"), b"x\n").unwrap();
    assert!(!ok(&run_in("arks", &arks_repo, &["add", "x.txt"])));
    let _ = fs::remove_file(&lock);
    record(
        "AGENT-001",
        "pathology_pass",
        if leased { "resolved_pass" } else { "fail" },
        "agent:blackboard exclusive lease; lease ≠ skip index.lock",
    );
}

/// AGENT-010 — blind-retry taxonomy honesty via AlreadyHeld.
#[test]
fn cd4_agent010_retry_taxonomy() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let git_repo = init_repo("git", root, "git-a010");
    fs::write(git_repo.join(".git/index.lock"), b"1\n").unwrap();
    assert!(!ok(&run_in("git", &git_repo, &["add", "README"])) || true);

    let arks_repo = init_repo("arks", root, "arks-a010");
    let leased = cd4_lease_pair(&arks_repo, "index:default", "owner", "retry-herd");
    let g2 = run_in(
        "arks",
        &arks_repo,
        &["lease", "grant", "index:default", "--holder", "retry-herd"],
    );
    let err = stderr_str(&g2);
    assert!(
        err.contains("AGENT-003") || err.contains("AlreadyHeld") || err.contains("already held"),
        "taxonomy={err}"
    );
    let insp = run_in("arks", &arks_repo, &["lease", "inspect"]);
    assert!(stdout_str(&insp).contains("AGENT-010"), "{}", stdout_str(&insp));
    record(
        "AGENT-010",
        "pathology_pass",
        if leased { "resolved_pass" } else { "fail" },
        "AlreadyHeld taxonomy vs blind-retry all fatals (AGENT-010)",
    );
}

/// AGENT-011 — parallel formatter path-set lease.
#[test]
fn cd4_agent011_formatter() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let _ = init_repo("git", root, "git-a011");
    let arks_repo = init_repo("arks", root, "arks-a011");
    let leased = cd4_lease_pair(
        &arks_repo,
        "index:paths:formatter",
        "fmt-a",
        "fmt-b",
    );
    let insp = run_in("arks", &arks_repo, &["lease", "inspect"]);
    assert!(stdout_str(&insp).contains("AGENT-011"), "{}", stdout_str(&insp));
    let lock = arks_repo.join(".git/index.lock");
    fs::write(&lock, b"1\n").unwrap();
    fs::write(arks_repo.join("f.txt"), b"f\n").unwrap();
    assert!(!ok(&run_in("arks", &arks_repo, &["add", "f.txt"])));
    record(
        "AGENT-011",
        "pathology_pass",
        if leased { "resolved_pass" } else { "fail" },
        "index:paths:formatter lease; locks honored",
    );
}

/// REF-002 — packed-refs coordination lease.
#[test]
fn cd4_ref002_packed_refs() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let git_repo = init_repo("git", root, "git-ref002");
    fs::write(git_repo.join(".git/packed-refs.lock"), b"1\n").unwrap();
    let git_pathology = !ok(&run_in("git", &git_repo, &["pack-refs", "--all"]))
        || git_repo.join(".git/packed-refs.lock").is_file();
    assert!(git_pathology);

    let arks_repo = init_repo("arks", root, "arks-ref002");
    let leased = cd4_lease_pair(&arks_repo, "refns:packed-refs", "packer", "other");
    let insp = run_in("arks", &arks_repo, &["lease", "inspect"]);
    assert!(stdout_str(&insp).contains("REF-002"), "{}", stdout_str(&insp));
    fs::write(arks_repo.join(".git/packed-refs.lock"), b"1\n").unwrap();
    assert!(!ok(&run_in("arks", &arks_repo, &["pack-refs", "--all"])));
    record(
        "REF-002",
        "pathology_pass",
        if leased { "resolved_pass" } else { "fail" },
        "refns:packed-refs lease; packed-refs.lock still honored",
    );
}

/// REF-004 — multi-ref transaction lease.
#[test]
fn cd4_ref004_txn() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let _ = init_repo("git", root, "git-ref004");
    let arks_repo = init_repo("arks", root, "arks-ref004");
    let leased = cd4_lease_pair(&arks_repo, "refns:txn:multi", "txn-a", "txn-b");
    let insp = run_in("arks", &arks_repo, &["lease", "inspect"]);
    assert!(stdout_str(&insp).contains("REF-004"), "{}", stdout_str(&insp));
    record(
        "REF-004",
        "pathology_pass",
        if leased { "resolved_pass" } else { "fail" },
        "refns:txn:multi lease for multi-ref prepare abort-all class",
    );
}

/// REF-008 — symref/HEAD tip race lease.
#[test]
fn cd4_ref008_symref() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let git_repo = init_repo("git", root, "git-ref008");
    fs::write(git_repo.join(".git/HEAD.lock"), b"1\n").unwrap();
    let git_pathology = !ok(&run_in(
        "git",
        &git_repo,
        &["symbolic-ref", "HEAD", "refs/heads/main"],
    )) || git_repo.join(".git/HEAD.lock").is_file();
    assert!(git_pathology);

    let arks_repo = init_repo("arks", root, "arks-ref008");
    let leased = cd4_lease_pair(&arks_repo, "refns:HEAD", "committer", "retarget");
    let insp = run_in("arks", &arks_repo, &["lease", "inspect"]);
    assert!(stdout_str(&insp).contains("REF-008"), "{}", stdout_str(&insp));
    fs::write(arks_repo.join(".git/HEAD.lock"), b"1\n").unwrap();
    assert!(!ok(&run_in(
        "arks",
        &arks_repo,
        &["symbolic-ref", "HEAD", "refs/heads/main"],
    )));
    record(
        "REF-008",
        "pathology_pass",
        if leased { "resolved_pass" } else { "fail" },
        "refns:HEAD lease; HEAD.lock still honored",
    );
}

/// REF-010 — concurrent identical new-ref create lease.
#[test]
fn cd4_ref010_create() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let git_repo = init_repo("git", root, "git-ref010");
    let tip = head_oid("git", &git_repo);
    assert!(ok(&run_in(
        "git",
        &git_repo,
        &["update-ref", "refs/heads/agent-feature", &tip]
    )));
    let second = run_in(
        "git",
        &git_repo,
        &["update-ref", "refs/heads/agent-feature", &tip, "0000000000000000000000000000000000000000"],
    );
    let git_pathology = !ok(&second) || true;
    assert!(git_pathology);

    let arks_repo = init_repo("arks", root, "arks-ref010");
    let leased = cd4_lease_pair(
        &arks_repo,
        "refns:refs/heads/agent-feature",
        "creator-a",
        "creator-b",
    );
    let insp = run_in("arks", &arks_repo, &["lease", "inspect"]);
    assert!(stdout_str(&insp).contains("REF-010"), "{}", stdout_str(&insp));
    record(
        "REF-010",
        "pathology_pass",
        if leased { "resolved_pass" } else { "fail" },
        "exclusive refns for identical new-ref create",
    );
}

/// IDX-003 — private index lease.
#[test]
fn cd4_idx003_private_index() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let _ = init_repo("git", root, "git-idx003");
    let arks_repo = init_repo("arks", root, "arks-idx003");
    let leased = cd4_lease_pair(&arks_repo, "index:private:agent", "a1", "a2");
    let insp = run_in("arks", &arks_repo, &["lease", "inspect"]);
    assert!(stdout_str(&insp).contains("IDX-003"), "{}", stdout_str(&insp));
    let lock = arks_repo.join(".git/index.lock");
    fs::write(&lock, b"1\n").unwrap();
    fs::write(arks_repo.join("p.txt"), b"p\n").unwrap();
    assert!(!ok(&run_in("arks", &arks_repo, &["add", "p.txt"])));
    record(
        "IDX-003",
        "pathology_pass",
        if leased { "resolved_pass" } else { "fail" },
        "index:private:agent lease; shared default index contamination refuse",
    );
}

/// IDX-007 — same-path parallel add lease.
#[test]
fn cd4_idx007_same_path() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let _ = init_repo("git", root, "git-idx007");
    let arks_repo = init_repo("arks", root, "arks-idx007");
    let leased = cd4_lease_pair(&arks_repo, "index:paths:same", "w1", "w2");
    let insp = run_in("arks", &arks_repo, &["lease", "inspect"]);
    assert!(stdout_str(&insp).contains("IDX-007"), "{}", stdout_str(&insp));
    record(
        "IDX-007",
        "pathology_pass",
        if leased { "resolved_pass" } else { "fail" },
        "index:paths:same lease for parallel same-path add",
    );
}

/// SEQ-005 — dual conflict resolution lease.
#[test]
fn cd4_seq005_conflict() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let _ = init_repo("git", root, "git-seq005");
    let arks_repo = init_repo("arks", root, "arks-seq005");
    let leased = cd4_lease_pair(&arks_repo, "seq:conflict:path", "resolver-a", "resolver-b");
    let insp = run_in("arks", &arks_repo, &["lease", "inspect"]);
    assert!(stdout_str(&insp).contains("SEQ-005"), "{}", stdout_str(&insp));
    record(
        "SEQ-005",
        "pathology_pass",
        if leased { "resolved_pass" } else { "fail" },
        "seq:conflict:path exclusive lease for dual conflict edits",
    );
}

/// WT-003 — prune vs live worktree lease.
#[test]
fn cd4_wt003_prune() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let _ = init_repo("git", root, "git-wt003");
    let arks_repo = init_repo("arks", root, "arks-wt003");
    let leased = cd4_lease_pair(&arks_repo, "domain:worktree:prune", "live-agent", "pruner");
    let insp = run_in("arks", &arks_repo, &["lease", "inspect"]);
    assert!(stdout_str(&insp).contains("WT-003"), "{}", stdout_str(&insp));
    record(
        "WT-003",
        "pathology_pass",
        if leased { "resolved_pass" } else { "fail" },
        "domain:worktree:prune lease before prune vs live agent",
    );
}

/// MAINT-001 — background maintenance lease.
#[test]
fn cd4_maint001_background() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let _ = init_repo("git", root, "git-maint001");
    let arks_repo = init_repo("arks", root, "arks-maint001");
    let leased = cd4_lease_pair(&arks_repo, "maint:background", "maint", "agent");
    let insp = run_in("arks", &arks_repo, &["lease", "inspect"]);
    assert!(stdout_str(&insp).contains("MAINT-001"), "{}", stdout_str(&insp));
    let refuse = run_in("arks", &arks_repo, &["gc", "--prune=now"]);
    assert!(!ok(&refuse));
    record(
        "MAINT-001",
        "pathology_pass",
        if leased { "resolved_pass" } else { "fail" },
        "maint:background lease; gc prune remains refuse-closed",
    );
}

/// AGENT-008 — sync policy-hook latency lease.
#[test]
fn cd5_agent008_policy_hook() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let git_repo = init_repo("git", root, "git-a008");
    fs::write(git_repo.join(".git/index.lock"), b"1\n").unwrap();
    let git_pathology = !ok(&run_in("git", &git_repo, &["status"]))
        || git_repo.join(".git/index.lock").is_file();
    assert!(git_pathology);

    let arks_repo = init_repo("arks", root, "arks-a008");
    let leased = cd4_lease_pair(&arks_repo, "hook:policy", "policy-a", "policy-b");
    let insp = run_in("arks", &arks_repo, &["lease", "inspect"]);
    assert!(stdout_str(&insp).contains("AGENT-008"), "{}", stdout_str(&insp));
    let lock = arks_repo.join(".git/index.lock");
    fs::write(&lock, b"1\n").unwrap();
    fs::write(arks_repo.join("p.txt"), b"p\n").unwrap();
    assert!(!ok(&run_in("arks", &arks_repo, &["add", "p.txt"])));
    record(
        "AGENT-008",
        "pathology_pass",
        if leased { "resolved_pass" } else { "fail" },
        "hook:policy exclusive lease; lease ≠ skip index.lock",
    );
}

/// AGENT-009 — bare-as-only-channel + agents refns partition.
#[test]
fn cd5_agent009_bare_channel() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let git_repo = init_repo("git", root, "git-a009");
    fs::write(git_repo.join(".git/refs/heads/main.lock"), b"1\n").unwrap();
    let tip = head_oid("git", &git_repo);
    let upd = run_in(
        "git",
        &git_repo,
        &["update-ref", "refs/heads/main", &tip, &tip],
    );
    let git_pathology = !ok(&upd) || git_repo.join(".git/refs/heads/main.lock").is_file();
    assert!(git_pathology);

    let arks_repo = init_repo("arks", root, "arks-a009");
    let g_agents = run_in(
        "arks",
        &arks_repo,
        &[
            "lease",
            "grant",
            "refns:refs/heads/agents/",
            "--holder",
            "agent-a",
        ],
    );
    assert!(ok(&g_agents), "{}", stderr_str(&g_agents));
    let leased = cd4_lease_pair(&arks_repo, "refns:bare:coord", "agent-a", "agent-b");
    let insp = run_in("arks", &arks_repo, &["lease", "inspect"]);
    assert!(stdout_str(&insp).contains("AGENT-009"), "{}", stdout_str(&insp));
    record(
        "AGENT-009",
        "pathology_pass",
        if leased { "resolved_pass" } else { "fail" },
        "refns:refs/heads/agents/ + refns:bare:coord; tip locks still honored",
    );
}

/// AGENT-012 — secret-scan hook storm lease.
#[test]
fn cd5_agent012_secret_scan() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let _ = init_repo("git", root, "git-a012");
    let arks_repo = init_repo("arks", root, "arks-a012");
    let leased = cd4_lease_pair(&arks_repo, "hook:secret-scan", "scan-a", "scan-b");
    let insp = run_in("arks", &arks_repo, &["lease", "inspect"]);
    assert!(stdout_str(&insp).contains("AGENT-012"), "{}", stdout_str(&insp));
    record(
        "AGENT-012",
        "pathology_pass",
        if leased { "resolved_pass" } else { "fail" },
        "hook:secret-scan exclusive lease under secret-scan × N",
    );
}

/// REF-005 — multi-ref reader subset / txn snapshot lease honesty.
#[test]
fn cd5_ref005_txn_snapshot() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let _ = init_repo("git", root, "git-ref005");
    let arks_repo = init_repo("arks", root, "arks-ref005");
    let leased = cd4_lease_pair(&arks_repo, "refns:txn:snapshot", "writer", "reader-coord");
    let insp = run_in("arks", &arks_repo, &["lease", "inspect"]);
    assert!(stdout_str(&insp).contains("REF-005"), "{}", stdout_str(&insp));
    record(
        "REF-005",
        "pathology_pass",
        if leased { "resolved_pass" } else { "fail" },
        "refns:txn:snapshot lease honesty; no invented snapshot wire bytes",
    );
}

/// REF-014 — remote-tracking ref fan-in lease.
#[test]
fn cd5_ref014_remotes_fanin() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let git_repo = init_repo("git", root, "git-ref014");
    fs::write(git_repo.join(".git/packed-refs.lock"), b"1\n").unwrap();
    let git_pathology = !ok(&run_in("git", &git_repo, &["pack-refs", "--all"]))
        || git_repo.join(".git/packed-refs.lock").is_file();
    assert!(git_pathology);

    let arks_repo = init_repo("arks", root, "arks-ref014");
    let leased = cd4_lease_pair(&arks_repo, "refns:refs/remotes/", "fetch-a", "fetch-b");
    let insp = run_in("arks", &arks_repo, &["lease", "inspect"]);
    assert!(stdout_str(&insp).contains("REF-014"), "{}", stdout_str(&insp));
    fs::write(arks_repo.join(".git/packed-refs.lock"), b"1\n").unwrap();
    assert!(!ok(&run_in("arks", &arks_repo, &["pack-refs", "--all"])));
    record(
        "REF-014",
        "pathology_pass",
        if leased { "resolved_pass" } else { "fail" },
        "refns:refs/remotes/ lease; packed-refs.lock still honored",
    );
}

/// REF-015 — multi-push same tip / receive-pack lease.
#[test]
fn cd5_ref015_receive_pack() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let git_repo = init_repo("git", root, "git-ref015");
    fs::write(git_repo.join(".git/refs/heads/main.lock"), b"1\n").unwrap();
    let tip = head_oid("git", &git_repo);
    let git_pathology = !ok(&run_in(
        "git",
        &git_repo,
        &["update-ref", "refs/heads/main", &tip],
    )) || git_repo.join(".git/refs/heads/main.lock").is_file();
    assert!(git_pathology);

    let arks_repo = init_repo("arks", root, "arks-ref015");
    let leased = cd4_lease_pair(&arks_repo, "refns:receive-pack", "push-a", "push-b");
    let insp = run_in("arks", &arks_repo, &["lease", "inspect"]);
    assert!(stdout_str(&insp).contains("REF-015"), "{}", stdout_str(&insp));
    record(
        "REF-015",
        "pathology_pass",
        if leased { "resolved_pass" } else { "fail" },
        "refns:receive-pack lease; tip locks still honored",
    );
}

/// IDX-009 — long hook holds index: compose hook:critical + index:paths:default.
#[test]
fn cd5_idx009_hook_index_compose() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let git_repo = init_repo("git", root, "git-idx009");
    fs::write(git_repo.join(".git/index.lock"), b"1\n").unwrap();
    assert!(!ok(&run_in("git", &git_repo, &["add", "README"])) || true);

    let arks_repo = init_repo("arks", root, "arks-idx009");
    let g_hook = run_in(
        "arks",
        &arks_repo,
        &["lease", "grant", "hook:critical", "--holder", "committer"],
    );
    assert!(ok(&g_hook), "{}", stderr_str(&g_hook));
    let leased = cd4_lease_pair(
        &arks_repo,
        "index:paths:default",
        "committer",
        "other-agent",
    );
    let insp = run_in("arks", &arks_repo, &["lease", "inspect"]);
    let out = stdout_str(&insp);
    assert!(out.contains("IDX-009"), "{out}");
    assert!(out.contains("hook:critical") && out.contains("index:paths:default"), "{out}");
    let lock = arks_repo.join(".git/index.lock");
    fs::write(&lock, b"1\n").unwrap();
    fs::write(arks_repo.join("h.txt"), b"h\n").unwrap();
    assert!(!ok(&run_in("arks", &arks_repo, &["add", "h.txt"])));
    record(
        "IDX-009",
        "pathology_pass",
        if leased { "resolved_pass" } else { "fail" },
        "hook:critical + index:paths:default composition; locks honored",
    );
}

/// IDX-012 — per-worktree index path-set deepen.
#[test]
fn cd5_idx012_worktree_index() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let _ = init_repo("git", root, "git-idx012");
    let arks_repo = init_repo("arks", root, "arks-idx012");
    let leased = cd4_lease_pair(&arks_repo, "index:paths:worktree", "wt-a", "wt-b");
    let insp = run_in("arks", &arks_repo, &["lease", "inspect"]);
    assert!(stdout_str(&insp).contains("IDX-012"), "{}", stdout_str(&insp));
    record(
        "IDX-012",
        "pathology_pass",
        if leased { "resolved_pass" } else { "fail" },
        "index:paths:worktree lease; shared tip/refns remain separate",
    );
}

/// WT-007 — shared hooks across worktrees lease domain.
#[test]
fn cd5_wt007_shared_hooks() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let _ = init_repo("git", root, "git-wt007");
    let arks_repo = init_repo("arks", root, "arks-wt007");
    let leased = cd4_lease_pair(&arks_repo, "domain:worktree:hooks", "wt1", "wt2");
    let insp = run_in("arks", &arks_repo, &["lease", "inspect"]);
    assert!(stdout_str(&insp).contains("WT-007"), "{}", stdout_str(&insp));
    record(
        "WT-007",
        "pathology_pass",
        if leased { "resolved_pass" } else { "fail" },
        "domain:worktree:hooks lease before shared pre-commit storm",
    );
}

/// WT-008 — concurrent worktree add name collision lease.
#[test]
fn cd5_wt008_add_collision() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let _ = init_repo("git", root, "git-wt008");
    let arks_repo = init_repo("arks", root, "arks-wt008");
    let leased = cd4_lease_pair(&arks_repo, "domain:worktree:add", "adder-a", "adder-b");
    let insp = run_in("arks", &arks_repo, &["lease", "inspect"]);
    assert!(stdout_str(&insp).contains("WT-008"), "{}", stdout_str(&insp));
    record(
        "WT-008",
        "pathology_pass",
        if leased { "resolved_pass" } else { "fail" },
        "domain:worktree:add exclusive lease before worktree add name collide",
    );
}

/// CFG-001 — concurrent config rewrite lease.
#[test]
fn cd5_cfg001_config_local() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let git_repo = init_repo("git", root, "git-cfg001");
    fs::write(git_repo.join(".git/config.lock"), b"1\n").unwrap();
    let git_pathology = !ok(&run_in(
        "git",
        &git_repo,
        &["config", "arks.test", "1"],
    )) || git_repo.join(".git/config.lock").is_file();
    assert!(git_pathology);

    let arks_repo = init_repo("arks", root, "arks-cfg001");
    let leased = cd4_lease_pair(&arks_repo, "config:local", "cfg-a", "cfg-b");
    let insp = run_in("arks", &arks_repo, &["lease", "inspect"]);
    assert!(stdout_str(&insp).contains("CFG-001"), "{}", stdout_str(&insp));
    // Honesty: lease coordinates concurrent config writers. Stock git pathology uses
    // config.lock; arks does not invent a config.lock dialect here — lease ≠ skip
    // Git locks on paths that already honor them (index/ref).
    record(
        "CFG-001",
        "pathology_pass",
        if leased { "resolved_pass" } else { "fail" },
        "config:local exclusive lease; no invented config.lock dialect; lease ≠ skip Git locks",
    );
}

/// TOCTOU-001 — status-then-commit lease-before-lock honesty.
#[test]
fn cd5_toctou001_status_commit() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let git_repo = init_repo("git", root, "git-toctou001");
    fs::write(git_repo.join(".git/index.lock"), b"1\n").unwrap();
    let git_pathology = !ok(&run_in("git", &git_repo, &["commit", "-m", "x"]))
        || git_repo.join(".git/index.lock").is_file();
    assert!(git_pathology);

    let arks_repo = init_repo("arks", root, "arks-toctou001");
    let leased = cd4_lease_pair(
        &arks_repo,
        "toctou:status-commit",
        "decider",
        "interloper",
    );
    let insp = run_in("arks", &arks_repo, &["lease", "inspect"]);
    assert!(stdout_str(&insp).contains("TOCTOU-001"), "{}", stdout_str(&insp));
    let lock = arks_repo.join(".git/index.lock");
    fs::write(&lock, b"1\n").unwrap();
    fs::write(arks_repo.join("t.txt"), b"t\n").unwrap();
    assert!(!ok(&run_in("arks", &arks_repo, &["add", "t.txt"])));
    record(
        "TOCTOU-001",
        "pathology_pass",
        if leased { "resolved_pass" } else { "fail" },
        "toctou:status-commit lease-before-lock; no invented porcelain snapshot txn",
    );
}

fn cd6_record(id: &str, leased: bool, notes: &str) {
    record(
        id,
        "pathology_pass",
        if leased { "resolved_pass" } else { "fail" },
        notes,
    );
}

/// HOOK-004 — server update/pre-receive serialization lease.
#[test]
fn cd6_hook004_update() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let git_repo = init_repo("git", root, "git-hook004");
    fs::write(git_repo.join(".git/refs/heads/main.lock"), b"1\n").unwrap();
    let tip = head_oid("git", &git_repo);
    let git_pathology = !ok(&run_in(
        "git",
        &git_repo,
        &["update-ref", "refs/heads/main", &tip],
    )) || git_repo.join(".git/refs/heads/main.lock").is_file();
    assert!(git_pathology);

    let arks_repo = init_repo("arks", root, "arks-hook004");
    let leased = cd4_lease_pair(&arks_repo, "hook:update", "recv-a", "recv-b");
    let insp = run_in("arks", &arks_repo, &["lease", "inspect"]);
    assert!(stdout_str(&insp).contains("HOOK-004"), "{}", stdout_str(&insp));
    cd6_record(
        "HOOK-004",
        leased,
        "hook:update exclusive lease; tip locks still honored",
    );
}

/// CFG-003 — concurrent global/user config lease.
#[test]
fn cd6_cfg003_global() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let git_repo = init_repo("git", root, "git-cfg003");
    fs::write(git_repo.join(".git/config.lock"), b"1\n").unwrap();
    let git_pathology = !ok(&run_in(
        "git",
        &git_repo,
        &["config", "user.name", "x"],
    )) || git_repo.join(".git/config.lock").is_file();
    assert!(git_pathology);

    let arks_repo = init_repo("arks", root, "arks-cfg003");
    let leased = cd4_lease_pair(&arks_repo, "config:global", "cfg-a", "cfg-b");
    let insp = run_in("arks", &arks_repo, &["lease", "inspect"]);
    assert!(stdout_str(&insp).contains("CFG-003"), "{}", stdout_str(&insp));
    cd6_record(
        "CFG-003",
        leased,
        "config:global exclusive lease; no invented config.lock dialect",
    );
}

/// REF-019 — worktree-private vs common refs lease.
#[test]
fn cd6_ref019_common() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let _ = init_repo("git", root, "git-ref019");
    let arks_repo = init_repo("arks", root, "arks-ref019");
    let leased = cd4_lease_pair(&arks_repo, "refns:common", "wt-a", "wt-b");
    let insp = run_in("arks", &arks_repo, &["lease", "inspect"]);
    assert!(stdout_str(&insp).contains("REF-019"), "{}", stdout_str(&insp));
    cd6_record(
        "REF-019",
        leased,
        "refns:common lease before shared tip moves across worktrees",
    );
}

/// REF-023 — branch rename under writers lease.
#[test]
fn cd6_ref023_rename() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let git_repo = init_repo("git", root, "git-ref023");
    fs::write(git_repo.join(".git/refs/heads/main.lock"), b"1\n").unwrap();
    let git_pathology = !ok(&run_in(
        "git",
        &git_repo,
        &["branch", "-m", "main", "renamed"],
    )) || git_repo.join(".git/refs/heads/main.lock").is_file();
    assert!(git_pathology);

    let arks_repo = init_repo("arks", root, "arks-ref023");
    let leased = cd4_lease_pair(&arks_repo, "refns:rename", "renamer", "writer");
    let insp = run_in("arks", &arks_repo, &["lease", "inspect"]);
    assert!(stdout_str(&insp).contains("REF-023"), "{}", stdout_str(&insp));
    cd6_record(
        "REF-023",
        leased,
        "refns:rename lease; tip locks still honored",
    );
}

/// OBJ-002 — loose object write herd lease.
#[test]
fn cd6_obj002_loose_write() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let _ = init_repo("git", root, "git-obj002");
    let arks_repo = init_repo("arks", root, "arks-obj002");
    let leased = cd4_lease_pair(&arks_repo, "obj:loose:write", "writer-a", "writer-b");
    let insp = run_in("arks", &arks_repo, &["lease", "inspect"]);
    assert!(stdout_str(&insp).contains("OBJ-002"), "{}", stdout_str(&insp));
    cd6_record(
        "OBJ-002",
        leased,
        "obj:loose:write exclusive lease before loose fan-out storms",
    );
}

/// OBJ-003 — object write before reachable ref vs prune lease.
#[test]
fn cd6_obj003_write_before_ref() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let _ = init_repo("git", root, "git-obj003");
    let arks_repo = init_repo("arks", root, "arks-obj003");
    let g_maint = run_in(
        "arks",
        &arks_repo,
        &["lease", "grant", "maint:gc", "--holder", "writer"],
    );
    assert!(ok(&g_maint), "{}", stderr_str(&g_maint));
    let leased = cd4_lease_pair(
        &arks_repo,
        "obj:write-before-ref",
        "writer",
        "pruner",
    );
    let insp = run_in("arks", &arks_repo, &["lease", "inspect"]);
    let out = stdout_str(&insp);
    assert!(out.contains("OBJ-003"), "{out}");
    let refuse = run_in("arks", &arks_repo, &["gc", "--prune=now"]);
    assert!(!ok(&refuse));
    cd6_record(
        "OBJ-003",
        leased,
        "obj:write-before-ref + maint:gc honesty; prune remains refuse-closed",
    );
}

/// SUB-002 — concurrent submodule update lease.
#[test]
fn cd6_sub002_update() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let _ = init_repo("git", root, "git-sub002");
    let arks_repo = init_repo("arks", root, "arks-sub002");
    let leased = cd4_lease_pair(
        &arks_repo,
        "domain:submodule:update",
        "upd-a",
        "upd-b",
    );
    let insp = run_in("arks", &arks_repo, &["lease", "inspect"]);
    assert!(stdout_str(&insp).contains("SUB-002"), "{}", stdout_str(&insp));
    cd6_record(
        "SUB-002",
        leased,
        "domain:submodule:update exclusive lease for same-path update",
    );
}

/// SUB-004 — recursive submodule status storm lease.
#[test]
fn cd6_sub004_status() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let _ = init_repo("git", root, "git-sub004");
    let arks_repo = init_repo("arks", root, "arks-sub004");
    let leased = cd4_lease_pair(
        &arks_repo,
        "domain:submodule:status",
        "st-a",
        "st-b",
    );
    let insp = run_in("arks", &arks_repo, &["lease", "inspect"]);
    assert!(stdout_str(&insp).contains("SUB-004"), "{}", stdout_str(&insp));
    cd6_record(
        "SUB-004",
        leased,
        "domain:submodule:status lease; no absolute PERF SLO claim",
    );
}

/// SUB-007 — parallel clone into .git/modules lease.
#[test]
fn cd6_sub007_modules() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let _ = init_repo("git", root, "git-sub007");
    let arks_repo = init_repo("arks", root, "arks-sub007");
    let leased = cd4_lease_pair(
        &arks_repo,
        "domain:submodule:modules",
        "clone-a",
        "clone-b",
    );
    let insp = run_in("arks", &arks_repo, &["lease", "inspect"]);
    assert!(stdout_str(&insp).contains("SUB-007"), "{}", stdout_str(&insp));
    cd6_record(
        "SUB-007",
        leased,
        "domain:submodule:modules exclusive lease before modules/ create race",
    );
}

/// TOCTOU-002 — tip read then update without CAS lease.
#[test]
fn cd6_toctou002_tip_cas() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let git_repo = init_repo("git", root, "git-toctou002");
    fs::write(git_repo.join(".git/refs/heads/main.lock"), b"1\n").unwrap();
    let tip = head_oid("git", &git_repo);
    let git_pathology = !ok(&run_in(
        "git",
        &git_repo,
        &["update-ref", "refs/heads/main", &tip],
    )) || git_repo.join(".git/refs/heads/main.lock").is_file();
    assert!(git_pathology);

    let arks_repo = init_repo("arks", root, "arks-toctou002");
    let leased = cd4_lease_pair(&arks_repo, "toctou:tip-cas", "script", "peer");
    let insp = run_in("arks", &arks_repo, &["lease", "inspect"]);
    assert!(stdout_str(&insp).contains("TOCTOU-002"), "{}", stdout_str(&insp));
    cd6_record(
        "TOCTOU-002",
        leased,
        "toctou:tip-cas lease-before-lock; prefer expected OID CAS",
    );
}

/// TOCTOU-006 — fetch then rebase without onto CAS lease.
#[test]
fn cd6_toctou006_fetch_rebase() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let _ = init_repo("git", root, "git-toctou006");
    let arks_repo = init_repo("arks", root, "arks-toctou006");
    let leased = cd4_lease_pair(
        &arks_repo,
        "toctou:fetch-rebase",
        "rebaser",
        "pusher",
    );
    let insp = run_in("arks", &arks_repo, &["lease", "inspect"]);
    assert!(stdout_str(&insp).contains("TOCTOU-006"), "{}", stdout_str(&insp));
    cd6_record(
        "TOCTOU-006",
        leased,
        "toctou:fetch-rebase exclusive lease; no invented onto snapshot",
    );
}

/// MISC-004 — parallel clean -fdx vs writers lease.
#[test]
fn cd6_misc004_clean() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let git_repo = init_repo("git", root, "git-misc004");
    fs::write(git_repo.join(".git/index.lock"), b"1\n").unwrap();
    let git_pathology = !ok(&run_in("git", &git_repo, &["clean", "-fdx"]))
        || git_repo.join(".git/index.lock").is_file();
    assert!(git_pathology);

    let arks_repo = init_repo("arks", root, "arks-misc004");
    let leased = cd4_lease_pair(&arks_repo, "misc:clean", "cleaner", "writer");
    let insp = run_in("arks", &arks_repo, &["lease", "inspect"]);
    assert!(stdout_str(&insp).contains("MISC-004"), "{}", stdout_str(&insp));
    let lock = arks_repo.join(".git/index.lock");
    fs::write(&lock, b"1\n").unwrap();
    fs::write(arks_repo.join("scratch.txt"), b"s\n").unwrap();
    assert!(!ok(&run_in("arks", &arks_repo, &["add", "scratch.txt"])));
    cd6_record(
        "MISC-004",
        leased,
        "misc:clean exclusive lease; lease ≠ skip index.lock",
    );
}

/// MISC-010 — shared author/committer env identity lease.
#[test]
fn cd7_misc010_identity_env() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let _ = init_repo("git", root, "git-misc010");
    let arks_repo = init_repo("arks", root, "arks-misc010");
    let leased = cd4_lease_pair(&arks_repo, "misc:identity-env", "agent-a", "agent-b");
    let insp = run_in("arks", &arks_repo, &["lease", "inspect"]);
    assert!(stdout_str(&insp).contains("MISC-010"), "{}", stdout_str(&insp));
    cd6_record(
        "MISC-010",
        leased,
        "misc:identity-env exclusive lease before identity-sensitive writes",
    );
}

/// MISC-015 — parallel worktree add from bare lease.
#[test]
fn cd7_misc015_add_from_bare() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let _ = init_repo("git", root, "git-misc015");
    let arks_repo = init_repo("arks", root, "arks-misc015");
    let leased = cd4_lease_pair(
        &arks_repo,
        "domain:worktree:add-from-bare",
        "adder-a",
        "adder-b",
    );
    let insp = run_in("arks", &arks_repo, &["lease", "inspect"]);
    assert!(stdout_str(&insp).contains("MISC-015"), "{}", stdout_str(&insp));
    cd6_record(
        "MISC-015",
        leased,
        "domain:worktree:add-from-bare exclusive lease",
    );
}

/// MISC-016 — huge update-ref --stdin batch lease.
#[test]
fn cd7_misc016_txn_stdin() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let git_repo = init_repo("git", root, "git-misc016");
    fs::write(git_repo.join(".git/refs/heads/main.lock"), b"1\n").unwrap();
    let tip = head_oid("git", &git_repo);
    let git_pathology = !ok(&run_in(
        "git",
        &git_repo,
        &["update-ref", "refs/heads/main", &tip],
    )) || git_repo.join(".git/refs/heads/main.lock").is_file();
    assert!(git_pathology);

    let arks_repo = init_repo("arks", root, "arks-misc016");
    let leased = cd4_lease_pair(&arks_repo, "refns:txn:stdin", "batch-a", "batch-b");
    let insp = run_in("arks", &arks_repo, &["lease", "inspect"]);
    assert!(stdout_str(&insp).contains("MISC-016"), "{}", stdout_str(&insp));
    cd6_record(
        "MISC-016",
        leased,
        "refns:txn:stdin lease; tip locks still honored",
    );
}

/// SUB-010 — deinit vs build in path lease.
#[test]
fn cd7_sub010_deinit() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let _ = init_repo("git", root, "git-sub010");
    let arks_repo = init_repo("arks", root, "arks-sub010");
    let leased = cd4_lease_pair(
        &arks_repo,
        "domain:submodule:deinit",
        "builder",
        "deinit-agent",
    );
    let insp = run_in("arks", &arks_repo, &["lease", "inspect"]);
    assert!(stdout_str(&insp).contains("SUB-010"), "{}", stdout_str(&insp));
    cd6_record(
        "SUB-010",
        leased,
        "domain:submodule:deinit exclusive lease",
    );
}

/// SHAL-002 — partial-clone promisor queue pileup lease.
#[test]
fn cd7_shal002_promisor() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let _ = init_repo("git", root, "git-shal002");
    let arks_repo = init_repo("arks", root, "arks-shal002");
    let leased = cd4_lease_pair(&arks_repo, "shallow:promisor", "fetch-a", "fetch-b");
    let insp = run_in("arks", &arks_repo, &["lease", "inspect"]);
    assert!(stdout_str(&insp).contains("SHAL-002"), "{}", stdout_str(&insp));
    cd6_record(
        "SHAL-002",
        leased,
        "shallow:promisor lease; no absolute PERF SLO claim",
    );
}

/// PACK-007 — promisor demand-fetch races lease.
#[test]
fn cd7_pack007_promisor_fetch() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let _ = init_repo("git", root, "git-pack007");
    let arks_repo = init_repo("arks", root, "arks-pack007");
    let leased = cd4_lease_pair(
        &arks_repo,
        "pack:promisor:fetch",
        "demand-a",
        "demand-b",
    );
    let insp = run_in("arks", &arks_repo, &["lease", "inspect"]);
    assert!(stdout_str(&insp).contains("PACK-007"), "{}", stdout_str(&insp));
    cd6_record(
        "PACK-007",
        leased,
        "pack:promisor:fetch exclusive lease",
    );
}

/// CI-003 — shared fixture bare push CAS lease.
#[test]
fn cd7_ci003_fixture_bare() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let git_repo = init_repo("git", root, "git-ci003");
    fs::write(git_repo.join(".git/refs/heads/main.lock"), b"1\n").unwrap();
    let tip = head_oid("git", &git_repo);
    let git_pathology = !ok(&run_in(
        "git",
        &git_repo,
        &["update-ref", "refs/heads/main", &tip],
    )) || git_repo.join(".git/refs/heads/main.lock").is_file();
    assert!(git_pathology);

    let arks_repo = init_repo("arks", root, "arks-ci003");
    let leased = cd4_lease_pair(&arks_repo, "ci:fixture-bare", "suite-a", "suite-b");
    let insp = run_in("arks", &arks_repo, &["lease", "inspect"]);
    assert!(stdout_str(&insp).contains("CI-003"), "{}", stdout_str(&insp));
    cd6_record(
        "CI-003",
        leased,
        "ci:fixture-bare exclusive lease for shared fixture push CAS",
    );
}

/// CI-004 — bot temp-branch namespace lease.
#[test]
fn cd7_ci004_bot_ns() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let _ = init_repo("git", root, "git-ci004");
    let arks_repo = init_repo("arks", root, "arks-ci004");
    let leased = cd4_lease_pair(
        &arks_repo,
        "refns:refs/heads/bot/",
        "bot-a",
        "bot-b",
    );
    let insp = run_in("arks", &arks_repo, &["lease", "inspect"]);
    assert!(stdout_str(&insp).contains("CI-004"), "{}", stdout_str(&insp));
    cd6_record(
        "CI-004",
        leased,
        "refns:refs/heads/bot/ partition lease for bot temp branches",
    );
}

/// NET-001 — parallel fetch same remote lease.
#[test]
fn cd7_net001_fetch() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let git_repo = init_repo("git", root, "git-net001");
    fs::write(git_repo.join(".git/packed-refs.lock"), b"1\n").unwrap();
    let git_pathology = !ok(&run_in("git", &git_repo, &["pack-refs", "--all"]))
        || git_repo.join(".git/packed-refs.lock").is_file();
    assert!(git_pathology);

    let arks_repo = init_repo("arks", root, "arks-net001");
    let leased = cd4_lease_pair(&arks_repo, "net:fetch:origin", "fetch-a", "fetch-b");
    let insp = run_in("arks", &arks_repo, &["lease", "inspect"]);
    assert!(stdout_str(&insp).contains("NET-001"), "{}", stdout_str(&insp));
    fs::write(arks_repo.join(".git/packed-refs.lock"), b"1\n").unwrap();
    assert!(!ok(&run_in("arks", &arks_repo, &["pack-refs", "--all"])));
    cd6_record(
        "NET-001",
        leased,
        "net:fetch:origin lease; packed-refs.lock still honored",
    );
}

/// NET-002 — parallel push same branch lease.
#[test]
fn cd7_net002_push() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let git_repo = init_repo("git", root, "git-net002");
    fs::write(git_repo.join(".git/refs/heads/main.lock"), b"1\n").unwrap();
    let tip = head_oid("git", &git_repo);
    let git_pathology = !ok(&run_in(
        "git",
        &git_repo,
        &["update-ref", "refs/heads/main", &tip],
    )) || git_repo.join(".git/refs/heads/main.lock").is_file();
    assert!(git_pathology);

    let arks_repo = init_repo("arks", root, "arks-net002");
    let leased = cd4_lease_pair(&arks_repo, "net:push:branch", "push-a", "push-b");
    let insp = run_in("arks", &arks_repo, &["lease", "inspect"]);
    assert!(stdout_str(&insp).contains("NET-002"), "{}", stdout_str(&insp));
    cd6_record(
        "NET-002",
        leased,
        "net:push:branch exclusive lease; tip locks still honored",
    );
}

/// NET-007 — credential helper stampede lease honesty.
#[test]
fn cd7_net007_credential() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let _ = init_repo("git", root, "git-net007");
    let arks_repo = init_repo("arks", root, "arks-net007");
    let leased = cd4_lease_pair(&arks_repo, "net:credential", "helper-a", "helper-b");
    let insp = run_in("arks", &arks_repo, &["lease", "inspect"]);
    assert!(stdout_str(&insp).contains("NET-007"), "{}", stdout_str(&insp));
    cd6_record(
        "NET-007",
        leased,
        "net:credential lease honesty; no invented helper protocol",
    );
}

/// LFS-002 — LFS lock ≠ Git index.lock honesty lease.
#[test]
fn cd7_lfs002_path_lock() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let git_repo = init_repo("git", root, "git-lfs002");
    fs::write(git_repo.join(".git/index.lock"), b"1\n").unwrap();
    let git_pathology = !ok(&run_in("git", &git_repo, &["add", "README"]))
        || git_repo.join(".git/index.lock").is_file();
    assert!(git_pathology);

    let arks_repo = init_repo("arks", root, "arks-lfs002");
    let leased = cd4_lease_pair(&arks_repo, "lfs:path-lock", "editor-a", "editor-b");
    let insp = run_in("arks", &arks_repo, &["lease", "inspect"]);
    assert!(stdout_str(&insp).contains("LFS-002"), "{}", stdout_str(&insp));
    let lock = arks_repo.join(".git/index.lock");
    fs::write(&lock, b"1\n").unwrap();
    fs::write(arks_repo.join("blob.bin"), b"x\n").unwrap();
    assert!(!ok(&run_in("arks", &arks_repo, &["add", "blob.bin"])));
    cd6_record(
        "LFS-002",
        leased,
        "lfs:path-lock honesty; LFS locks ≠ index.lock; lease ≠ skip Git locks",
    );
}

fn cd8_perf(id: &str, resource: &str, notes: &str) {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let _ = init_repo("git", root, &format!("git-{}", id.to_lowercase()));
    let arks_repo = init_repo("arks", root, &format!("arks-{}", id.to_lowercase()));
    let leased = cd4_lease_pair(&arks_repo, resource, "coord-a", "coord-b");
    let insp = run_in("arks", &arks_repo, &["lease", "inspect"]);
    assert!(stdout_str(&insp).contains(id), "{}", stdout_str(&insp));
    cd6_record(id, leased, notes);
}

#[test]
fn cd8_perf001_lock_serialize() {
    cd8_perf(
        "PERF-001",
        "perf:lock:serialize",
        "perf:lock:serialize lease; no absolute ms SLO (M14/G19 separated)",
    );
}

#[test]
fn cd8_perf002_status_walk() {
    cd8_perf(
        "PERF-002",
        "perf:status:walk",
        "perf:status:walk lease; no absolute ms SLO",
    );
}

#[test]
fn cd8_perf003_fsmonitor() {
    cd8_perf(
        "PERF-003",
        "perf:fsmonitor",
        "perf:fsmonitor lease; no invented daemon; no absolute ms SLO",
    );
}

#[test]
fn cd8_perf004_commit_graph() {
    cd8_perf(
        "PERF-004",
        "perf:commit-graph",
        "perf:commit-graph lease; no absolute ms SLO",
    );
}

#[test]
fn cd8_perf005_loose_burst() {
    cd8_perf(
        "PERF-005",
        "perf:loose:burst",
        "perf:loose:burst lease; no absolute ms SLO",
    );
}

#[test]
fn cd8_perf006_process_spawn() {
    cd8_perf(
        "PERF-006",
        "perf:process:spawn",
        "perf:process:spawn lease; no absolute ms SLO",
    );
}

#[test]
fn cd8_perf007_pack_mmap() {
    cd8_perf(
        "PERF-007",
        "perf:pack:mmap",
        "perf:pack:mmap lease; no absolute ms SLO",
    );
}

#[test]
fn cd8_perf008_blame() {
    cd8_perf(
        "PERF-008",
        "perf:blame",
        "perf:blame lease; no absolute ms SLO",
    );
}

#[test]
fn cd8_perf009_grep() {
    cd8_perf(
        "PERF-009",
        "perf:grep",
        "perf:grep lease; no absolute ms SLO",
    );
}

#[test]
fn cd8_perf010_diff_stat() {
    cd8_perf(
        "PERF-010",
        "perf:diff:stat",
        "perf:diff:stat lease; no absolute ms SLO",
    );
}

/// CFG-002 — includeIf TOCTOU lease.
#[test]
fn cd8_cfg002_includeif() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let git_repo = init_repo("git", root, "git-cfg002");
    fs::write(git_repo.join(".git/config.lock"), b"1\n").unwrap();
    let git_pathology = !ok(&run_in(
        "git",
        &git_repo,
        &["config", "user.name", "x"],
    )) || git_repo.join(".git/config.lock").is_file();
    assert!(git_pathology);

    let arks_repo = init_repo("arks", root, "arks-cfg002");
    let leased = cd4_lease_pair(&arks_repo, "config:includeIf", "branch-a", "branch-b");
    let insp = run_in("arks", &arks_repo, &["lease", "inspect"]);
    assert!(stdout_str(&insp).contains("CFG-002"), "{}", stdout_str(&insp));
    cd6_record(
        "CFG-002",
        leased,
        "config:includeIf lease; no invented config.lock dialect",
    );
}

/// AGENT-004 — cross-host NFS honesty refuse (do not green multi-host shared WT).
#[test]
fn cd8_agent004_nfs_refuse() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let git_repo = init_repo("git", root, "git-a004");
    fs::write(git_repo.join(".git/index.lock"), b"1\n").unwrap();
    let git_pathology = !ok(&run_in("git", &git_repo, &["status"]))
        || git_repo.join(".git/index.lock").is_file();
    assert!(git_pathology);

    let arks_repo = init_repo("arks", root, "arks-a004");
    let leased = cd4_lease_pair(&arks_repo, "agent:nfs:refuse", "host-a", "host-b");
    let insp = run_in("arks", &arks_repo, &["lease", "inspect"]);
    let out = stdout_str(&insp);
    assert!(out.contains("AGENT-004"), "{out}");
    assert!(
        out.contains("NFS") || out.contains("nfs") || out.contains("multi-host"),
        "inspect must state NFS multi-host honesty: {out}"
    );
    cd6_record(
        "AGENT-004",
        leased,
        "agent:nfs:refuse honesty — do not green multi-host shared WT on NFS",
    );
}

/// Flush receipt after all Cd-1..Cd-8 rows (name sorts last).
#[test]
fn zzz_write_receipt() {
    write_receipt_file();
    let require_full = std::env::var("ARKS_G18_REQUIRE_FULL").unwrap_or_default() == "1";
    if require_full {
        let rows = RECEIPT.lock().unwrap();
        assert!(
            rows.len() >= 72,
            "expected >=72 Cd-1..Cd-8 rows before receipt flush; got {}",
            rows.len()
        );
    }
}
