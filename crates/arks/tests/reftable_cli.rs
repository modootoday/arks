//! Integration tests for Git Reftable v1/v2 backend and `arks refs migrate|verify` (E5).

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn arks_bin() -> &'static str {
    env!("CARGO_BIN_EXE_arks")
}

fn run(dir: &Path, args: &[&str]) -> Output {
    Command::new(arks_bin())
        .current_dir(dir)
        .env("GIT_AUTHOR_NAME", "Reftable Test")
        .env("GIT_AUTHOR_EMAIL", "reftable@example.com")
        .env("GIT_COMMITTER_NAME", "Reftable Test")
        .env("GIT_COMMITTER_EMAIL", "reftable@example.com")
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
    fs::write(dir.join(rel), content).unwrap();
    ok(dir, &["add", rel]);
    ok(dir, &["commit", "-m", msg]);
    ok(dir, &["rev-parse", "HEAD"])
}

#[test]
fn refs_migrate_files_to_reftable_live_ops_and_roundtrip_back() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();

    ok(dir, &["init", "-b", "main"]);
    let c1 = commit_file(dir, "a.txt", "v1\n", "commit 1");
    ok(dir, &["branch", "topic"]);
    let c2 = commit_file(dir, "b.txt", "v2\n", "commit 2");
    ok(dir, &["tag", "v0.1", &c1]);

    // Dry-run must not mutate on-disk layout.
    let dry = ok(
        dir,
        &["refs", "migrate", "--ref-format=reftable", "--dry-run"],
    );
    assert!(dry.contains("dry-run"), "dry={dry}");
    assert!(!dir.join(".git/reftable/tables.list").exists());

    // Migrate files -> reftable.
    let mig = ok(dir, &["refs", "migrate", "--ref-format=reftable"]);
    assert!(mig.contains("migrated"), "mig={mig}");
    assert!(dir.join(".git/reftable/tables.list").is_file());
    assert_eq!(
        fs::read_to_string(dir.join(".git/HEAD")).unwrap().trim(),
        "ref: refs/heads/.invalid"
    );
    assert!(
        dir.join(".git/refs/heads").is_file(),
        "refs/heads must be the reftable sentinel file"
    );
    let cfg = fs::read_to_string(dir.join(".git/config")).unwrap();
    assert!(cfg.contains("repositoryformatversion = 1"), "cfg={cfg}");
    assert!(cfg.contains("refStorage = reftable"), "cfg={cfg}");

    // Strict verification passes.
    let ver = ok(dir, &["refs", "verify", "--strict"]);
    assert!(ver.contains("format=reftable"), "ver={ver}");

    // Live Git commands work transparently on the reftable repo!
    assert_eq!(ok(dir, &["rev-parse", "HEAD"]), c2);
    assert_eq!(ok(dir, &["rev-parse", "refs/heads/topic"]), c1);
    assert_eq!(ok(dir, &["rev-parse", "refs/tags/v0.1"]), c1);

    let c3 = commit_file(dir, "c.txt", "v3\n", "commit 3 on reftable");
    assert_eq!(ok(dir, &["rev-parse", "HEAD"]), c3);
    assert_eq!(ok(dir, &["rev-parse", "refs/heads/main"]), c3);

    ok(dir, &["branch", "reftable-branch"]);
    assert_eq!(ok(dir, &["rev-parse", "refs/heads/reftable-branch"]), c3);
    ok(dir, &["branch", "-d", "topic"]);
    assert!(
        !run(dir, &["rev-parse", "refs/heads/topic"])
            .status
            .success()
    );

    // Compact via pack-refs --all and re-verify strictly.
    ok(dir, &["pack-refs", "--all"]);
    let ver2 = ok(dir, &["refs", "verify", "--strict"]);
    assert!(ver2.contains("tables=1"), "ver2={ver2}");

    // Migrate reftable -> files roundtrip.
    let back = ok(dir, &["refs", "migrate", "--ref-format=files"]);
    assert!(back.contains("migrated"), "back={back}");
    assert!(!dir.join(".git/reftable").exists());
    assert!(dir.join(".git/refs/heads").is_dir());
    assert_eq!(ok(dir, &["rev-parse", "HEAD"]), c3);
    assert_eq!(ok(dir, &["rev-parse", "refs/heads/reftable-branch"]), c3);
    assert_eq!(ok(dir, &["rev-parse", "refs/tags/v0.1"]), c1);
    let ver3 = ok(dir, &["refs", "verify", "--strict"]);
    assert!(ver3.contains("format=files"), "ver3={ver3}");
}
