//! Gate G21 — M16 Sy-3 plumbing breadth claim rows.
//! Matrix: `verify/matrix-g21.md`. Cite: git-show-ref / git-pack-objects / git-index-pack 2.39.0.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tempfile::TempDir;

fn arks_bin() -> PathBuf {
    if let Ok(p) = std::env::var("ARKS_BIN") {
        let p = PathBuf::from(p);
        if p.is_file() {
            return p;
        }
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    for c in [
        root.join("target/release/arks.exe"),
        root.join("target/release/arks"),
        root.join("target/debug/arks.exe"),
        root.join("target/debug/arks"),
    ] {
        if c.is_file() {
            return c;
        }
    }
    if let Ok(target) = std::env::var("CARGO_TARGET_DIR") {
        for c in [
            PathBuf::from(&target).join("release/arks"),
            PathBuf::from(&target).join("release/arks.exe"),
        ] {
            if c.is_file() {
                return c;
            }
        }
    }
    panic!("missing arks");
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
        "{label}: {}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

fn seed_repo(repo: &Path) {
    fs::create_dir_all(repo).unwrap();
    assert_ok(&arks(repo, &["init"]), "init");
    fs::write(repo.join("a.txt"), "one\n").unwrap();
    assert_ok(&arks(repo, &["add", "a.txt"]), "add");
    assert_ok(&arks(repo, &["commit", "-m", "first"]), "commit");
}

#[test]
fn m16_sy3_01_plumbing_refs_and_lists() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("r");
    seed_repo(&repo);
    assert_ok(&arks(&repo, &["show-ref", "--heads"]), "show-ref");
    assert_ok(
        &arks(&repo, &["for-each-ref", "refs/heads/"]),
        "for-each-ref",
    );
    assert_ok(
        &arks(&repo, &["rev-list", "--max-count", "1", "HEAD"]),
        "rev-list",
    );
    assert_ok(&arks(&repo, &["ls-files", "-c"]), "ls-files -c");
    assert_ok(
        &arks(&repo, &["ls-tree", "--name-only", "HEAD"]),
        "ls-tree",
    );
    assert_ok(
        &arks(&repo, &["merge-base", "--all", "HEAD", "HEAD"]),
        "merge-base",
    );
    assert_ok(&arks(&repo, &["write-tree"]), "write-tree");
    assert_ok(&arks(&repo, &["read-tree", "HEAD"]), "read-tree");
    assert_ok(
        &arks(&repo, &["diff-tree", "--name-status", "HEAD", "HEAD"]),
        "diff-tree",
    );
    let oid = String::from_utf8_lossy(&arks(&repo, &["hash-object", "-w", "a.txt"]).stdout)
        .trim()
        .to_string();
    assert_ok(
        &arks(
            &repo,
            &["update-index", "--cacheinfo", "100644", &oid, "b.txt"],
        ),
        "update-index --cacheinfo",
    );
}

#[test]
fn m16_sy3_02_pack_objects_index_pack() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("r");
    seed_repo(&repo);
    let blob = String::from_utf8_lossy(&arks(&repo, &["hash-object", "-w", "a.txt"]).stdout)
        .trim()
        .to_string();
    let tree = String::from_utf8_lossy(&arks(&repo, &["write-tree"]).stdout)
        .trim()
        .to_string();
    let commit = String::from_utf8_lossy(&arks(&repo, &["rev-parse", "HEAD"]).stdout)
        .trim()
        .to_string();

    let mut child = Command::new(arks_bin());
    pin_env(&mut child);
    let mut child = child
        .args(["pack-objects"])
        .current_dir(&repo)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn pack-objects");
    {
        let mut stdin = child.stdin.take().expect("stdin");
        writeln!(stdin, "{blob}").unwrap();
        writeln!(stdin, "{tree}").unwrap();
        writeln!(stdin, "{commit}").unwrap();
    }
    let out = child.wait_with_output().expect("pack-objects wait");
    assert_ok(&out, "pack-objects");
    let hex = String::from_utf8_lossy(&out.stdout).trim().to_string();
    assert_eq!(hex.len(), 40, "expected sha1 pack checksum hex, got {hex}");
    // Default base name is `pack` → `pack-<hex>.{pack,idx}` in cwd.
    let pack_path = repo.join(format!("pack-{hex}.pack"));
    let idx_path = repo.join(format!("pack-{hex}.idx"));
    assert!(pack_path.is_file(), "missing {}", pack_path.display());
    assert!(idx_path.is_file(), "missing {}", idx_path.display());

    // Remove idx and rebuild via index-pack.
    fs::remove_file(&idx_path).unwrap();
    assert_ok(
        &arks(&repo, &["index-pack", pack_path.to_str().unwrap()]),
        "index-pack",
    );
    assert!(
        idx_path.is_file(),
        "index-pack should recreate {}",
        idx_path.display()
    );

    let git_verify = Command::new("git")
        .args(["index-pack", "--verify", pack_path.to_str().unwrap()])
        .current_dir(&repo)
        .output()
        .expect("git index-pack");
    assert!(
        git_verify.status.success(),
        "git peer index-pack --verify: {}\n{}",
        String::from_utf8_lossy(&git_verify.stdout),
        String::from_utf8_lossy(&git_verify.stderr)
    );
}
