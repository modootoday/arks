//! Stock `git` in the clean-room image is the compatibility oracle (no gix).
//! Gate G2: refs round-trip + index r/w portion toward G3.

use arks_index::{sort_entries, Index, IndexEntry};
use arks_oid::{Oid, OidKind};
use arks_refs::{FilesystemRefStore, RefTarget};
use arks_repo::Repository;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

fn require_git() {
    let out = Command::new("git")
        .arg("--version")
        .output()
        .expect("stock git must exist in clean-room image");
    assert!(
        out.status.success(),
        "git --version failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn git_init(path: &Path) {
    require_git();
    let out = Command::new("git")
        .args(["-c", "init.templateDir=", "init", path.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git init: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn git_git_dir(repo: &Path) -> PathBuf {
    repo.join(".git")
}

fn git_show_ref(repo: &Path, name: &str) -> Option<String> {
    let out = Command::new("git")
        .args([
            "--git-dir",
            &git_git_dir(repo).to_string_lossy(),
            "show-ref",
            "--verify",
            "--hash",
            name,
        ])
        .output()
        .unwrap();
    if out.status.success() {
        Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        None
    }
}

fn git_rev_parse(repo: &Path, rev: &str) -> String {
    let out = Command::new("git")
        .args([
            "--git-dir",
            &git_git_dir(repo).to_string_lossy(),
            "rev-parse",
            rev,
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git rev-parse {rev}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn git_update_ref(repo: &Path, name: &str, oid: &str) {
    let out = Command::new("git")
        .args([
            "--git-dir",
            &git_git_dir(repo).to_string_lossy(),
            "update-ref",
            name,
            oid,
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git update-ref: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn git_hash_object_write(repo: &Path, payload: &[u8]) -> String {
    use std::io::Write;
    use std::process::Stdio;
    let mut child = Command::new("git")
        .args([
            "--git-dir",
            &git_git_dir(repo).to_string_lossy(),
            "hash-object",
            "-w",
            "--stdin",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.as_mut().unwrap().write_all(payload).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// Create an empty commit OID (branch tips must be commits for `git update-ref`).
fn git_empty_commit(repo: &Path, message: &str) -> String {
    use std::io::Write;
    use std::process::Stdio;
    require_git();
    let mut child = Command::new("git")
        .args([
            "--git-dir",
            &git_git_dir(repo).to_string_lossy(),
            "hash-object",
            "-t",
            "tree",
            "-w",
            "--stdin",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.as_mut().unwrap().write_all(b"").unwrap();
    let tree_out = child.wait_with_output().unwrap();
    assert!(
        tree_out.status.success(),
        "hash-object tree: {}",
        String::from_utf8_lossy(&tree_out.stderr)
    );
    let tree = String::from_utf8_lossy(&tree_out.stdout).trim().to_string();
    let out = Command::new("git")
        .args([
            "--git-dir",
            &git_git_dir(repo).to_string_lossy(),
            "-c",
            "user.name=arks",
            "-c",
            "user.email=arks@example.com",
            "commit-tree",
            &tree,
            "-m",
            message,
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "commit-tree: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn sample_oid(n: u8) -> Oid {
    Oid::from_hex(&format!("{n:040x}")).unwrap()
}

#[test]
fn arks_update_ref_readable_by_git_show_ref() {
    let dir = TempDir::new().unwrap();
    git_init(dir.path());
    let commit = git_empty_commit(dir.path(), "g2-ref-a");
    let oid = Oid::from_hex(&commit).unwrap();
    let mut refs = FilesystemRefStore::open(git_git_dir(dir.path())).unwrap();
    refs.update("refs/heads/feature", &oid, None).unwrap();
    let shown = git_show_ref(dir.path(), "refs/heads/feature").expect("show-ref");
    assert_eq!(shown, oid.to_hex());
}

#[test]
fn git_update_ref_readable_by_arks() {
    let dir = TempDir::new().unwrap();
    git_init(dir.path());
    let commit = git_empty_commit(dir.path(), "g2-ref-b");
    git_update_ref(dir.path(), "refs/heads/from-git", &commit);
    let refs = FilesystemRefStore::open(git_git_dir(dir.path())).unwrap();
    assert_eq!(
        refs.resolve("refs/heads/from-git").unwrap().to_hex(),
        commit
    );
}

#[test]
fn arks_head_symref_matches_git_rev_parse() {
    let dir = TempDir::new().unwrap();
    git_init(dir.path());
    let commit = git_empty_commit(dir.path(), "g2-head");
    let oid = Oid::from_hex(&commit).unwrap();
    let mut refs = FilesystemRefStore::open(git_git_dir(dir.path())).unwrap();
    refs.update("refs/heads/main", &oid, None).unwrap();
    refs.set_symbolic("HEAD", "refs/heads/main").unwrap();
    assert_eq!(
        refs.read("HEAD").unwrap(),
        RefTarget::Symbolic("refs/heads/main".into())
    );
    assert_eq!(git_rev_parse(dir.path(), "HEAD"), oid.to_hex());
    assert_eq!(git_rev_parse(dir.path(), "refs/heads/main"), oid.to_hex());
}

#[test]
fn arks_delete_ref_absent_from_git_show_ref() {
    let dir = TempDir::new().unwrap();
    git_init(dir.path());
    let commit = git_empty_commit(dir.path(), "g2-del");
    let oid = Oid::from_hex(&commit).unwrap();
    let mut refs = FilesystemRefStore::open(git_git_dir(dir.path())).unwrap();
    refs.update("refs/heads/temp", &oid, None).unwrap();
    assert!(git_show_ref(dir.path(), "refs/heads/temp").is_some());
    refs.delete("refs/heads/temp", Some(&oid)).unwrap();
    assert!(git_show_ref(dir.path(), "refs/heads/temp").is_none());
}

#[test]
fn l0_ref_lock_contention_fails() {
    let dir = TempDir::new().unwrap();
    git_init(dir.path());
    let git_dir = git_git_dir(dir.path());
    let name = "refs/heads/locked";
    let lock = git_dir.join(format!("{name}.lock"));
    fs::create_dir_all(lock.parent().unwrap()).unwrap();
    fs::write(&lock, b"").unwrap();
    let mut refs = FilesystemRefStore::open(&git_dir).unwrap();
    let err = refs.update(name, &sample_oid(1), None);
    assert!(err.is_err(), "expected LockFailed, got {err:?}");
}

#[test]
fn repo_compose_discovers_and_resolves_head() {
    let dir = TempDir::new().unwrap();
    git_init(dir.path());
    let commit = git_empty_commit(dir.path(), "g2-repo");
    let oid = Oid::from_hex(&commit).unwrap();
    {
        let mut refs = FilesystemRefStore::open(git_git_dir(dir.path())).unwrap();
        refs.update("refs/heads/main", &oid, None).unwrap();
        refs.set_symbolic("HEAD", "refs/heads/main").unwrap();
    }
    let repo = Repository::discover(dir.path()).unwrap();
    assert_eq!(repo.head_oid().unwrap().to_hex(), commit);
}

#[test]
fn arks_index_write_readable_by_git_ls_files() {
    let dir = TempDir::new().unwrap();
    git_init(dir.path());
    let blob = git_hash_object_write(dir.path(), b"hello-index\n");
    let oid = Oid::from_hex(&blob).unwrap();
    let entry = IndexEntry {
        ctime_sec: 1,
        ctime_nsec: 0,
        mtime_sec: 1,
        mtime_nsec: 0,
        dev: 0,
        ino: 0,
        mode: 0o100644,
        uid: 0,
        gid: 0,
        size: 12,
        oid,
        flags: 0,
        flags_extended: None,
        path: "hello.txt".into(),
    };
    let mut entries = vec![entry];
    sort_entries(&mut entries);
    let idx = Index {
        version: 2,
        entries,
        extensions: vec![],
    };
    let index_path = git_git_dir(dir.path()).join("index");
    idx.write_path(&index_path, OidKind::Sha1).unwrap();

    let out = Command::new("git")
        .args(["-C", dir.path().to_str().unwrap(), "ls-files", "--stage"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git ls-files: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.contains("hello.txt") && text.contains(&blob),
        "unexpected ls-files: {text}"
    );
}

#[test]
fn git_add_index_readable_by_arks() {
    let dir = TempDir::new().unwrap();
    git_init(dir.path());
    fs::write(dir.path().join("added.txt"), b"added\n").unwrap();
    let out = Command::new("git")
        .args(["-C", dir.path().to_str().unwrap(), "add", "added.txt"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git add: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let index_path = git_git_dir(dir.path()).join("index");
    let parsed = Index::read_path(&index_path, OidKind::Sha1).unwrap();
    assert!(
        parsed
            .entries
            .iter()
            .any(|e| e.path == "added.txt" && e.stage() == 0),
        "missing added.txt in {:?}",
        parsed.entries.iter().map(|e| &e.path).collect::<Vec<_>>()
    );
}

#[test]
fn l0_index_lock_contention_fails() {
    let dir = TempDir::new().unwrap();
    git_init(dir.path());
    let index_path = git_git_dir(dir.path()).join("index");
    let mut lock_os = index_path.as_os_str().to_owned();
    lock_os.push(".lock");
    let lock = PathBuf::from(lock_os);
    fs::write(&lock, b"").unwrap();
    let idx = Index {
        version: 2,
        entries: vec![],
        extensions: vec![],
    };
    let err = idx.write_path(&index_path, OidKind::Sha1);
    assert!(err.is_err(), "expected LockFailed, got {err:?}");
}
