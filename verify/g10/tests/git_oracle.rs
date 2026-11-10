//! Stock `git` in the clean-room image is the compatibility oracle (no gix).
//! Facade under test: `arks_store::FilesystemGitStore` at a Git `objects/` root.

use arks_hash::ObjectFormat;
use arks_object::{ObjectKind, RawObject};
use arks_oid::Oid;
use arks_store::{FilesystemGitStore, ObjectStore};
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use tempfile::TempDir;

fn require_git() -> &'static str {
    let out = Command::new("git")
        .arg("--version")
        .output()
        .expect("stock git must exist in clean-room image");
    assert!(
        out.status.success(),
        "git --version failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    "git"
}

fn git_hash_object_blob(payload: &[u8]) -> String {
    require_git();
    let mut child = Command::new("git")
        .args(["hash-object", "-t", "blob", "--stdin"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("git hash-object");
    child.stdin.as_mut().unwrap().write_all(payload).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(
        out.status.success(),
        "git hash-object: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap().trim().to_string()
}

fn git_init_repo(path: &Path) {
    require_git();
    let out = Command::new("git")
        .args(["-c", "init.templateDir=", "init", path.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(out.status.success());
}

fn git_cat_file_blob(repo: &Path, oid_hex: &str) -> Vec<u8> {
    require_git();
    let out = Command::new("git")
        .args([
            "--git-dir",
            &repo.join(".git").to_string_lossy(),
            "cat-file",
            "-p",
            oid_hex,
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git cat-file -p {oid_hex}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    out.stdout
}

fn open_store(dir: &TempDir) -> FilesystemGitStore {
    let objects = dir.path().join(".git").join("objects");
    FilesystemGitStore::new(&objects).expect("FilesystemGitStore::new")
}

#[test]
fn empty_blob_oid_matches_git_hash_object() {
    let dir = TempDir::new().unwrap();
    git_init_repo(dir.path());
    let mut store = open_store(&dir);
    let obj = RawObject::new(ObjectKind::Blob, b"");
    let oid = store.put(ObjectFormat::Sha1, &obj).unwrap();
    let git = git_hash_object_blob(b"");
    assert_eq!(oid.to_hex(), git);
}

#[test]
fn hello_blob_oid_matches_git_hash_object() {
    let dir = TempDir::new().unwrap();
    git_init_repo(dir.path());
    let mut store = open_store(&dir);
    let obj = RawObject::new(ObjectKind::Blob, b"hello");
    let oid = store.put(ObjectFormat::Sha1, &obj).unwrap();
    let git = git_hash_object_blob(b"hello");
    assert_eq!(oid.to_hex(), git, "known hello oid b6fc4c62…");
}

#[test]
fn facade_put_get_roundtrip() {
    let dir = TempDir::new().unwrap();
    git_init_repo(dir.path());
    let mut store = open_store(&dir);
    let obj = RawObject::new(ObjectKind::Blob, b"facade-roundtrip");
    let oid = store.put(ObjectFormat::Sha1, &obj).unwrap();
    let got = store.get(&oid).unwrap();
    assert_eq!(got.kind, ObjectKind::Blob);
    assert_eq!(got.payload, b"facade-roundtrip");
}

#[test]
fn facade_exists_true_after_put() {
    let dir = TempDir::new().unwrap();
    git_init_repo(dir.path());
    let mut store = open_store(&dir);
    let obj = RawObject::new(ObjectKind::Blob, b"x");
    let oid = store.put(ObjectFormat::Sha1, &obj).unwrap();
    assert!(store.exists(&oid).unwrap());
}

#[test]
fn facade_exists_false_absent() {
    let dir = TempDir::new().unwrap();
    git_init_repo(dir.path());
    let store = open_store(&dir);
    let absent = Oid::from_hex("0000000000000000000000000000000000000000").unwrap();
    assert!(!store.exists(&absent).unwrap());
}

#[test]
fn facade_write_git_cat_file_roundtrip() {
    let dir = TempDir::new().unwrap();
    git_init_repo(dir.path());
    let mut store = open_store(&dir);
    let obj = RawObject::new(ObjectKind::Blob, b"hello");
    let oid = store.put(ObjectFormat::Sha1, &obj).unwrap();
    let payload = git_cat_file_blob(dir.path(), &oid.to_hex());
    assert_eq!(payload, b"hello");
}

#[test]
fn git_hash_object_write_facade_get_roundtrip() {
    let dir = TempDir::new().unwrap();
    git_init_repo(dir.path());
    let git_dir = dir.path().join(".git");
    let store = open_store(&dir);

    let payload = b"roundtrip";
    let hex = git_hash_object_blob(payload);

    let mut child = Command::new("git")
        .args([
            "--git-dir",
            &git_dir.to_string_lossy(),
            "hash-object",
            "-w",
            "-t",
            "blob",
            "--stdin",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.as_mut().unwrap().write_all(payload).unwrap();
    let written = child.wait_with_output().unwrap();
    assert!(written.status.success());
    let written_hex = String::from_utf8(written.stdout).unwrap().trim().to_string();
    assert_eq!(written_hex, hex);

    let oid = Oid::from_hex(&hex).unwrap();
    let got = store.get(&oid).unwrap();
    assert_eq!(got.kind, ObjectKind::Blob);
    assert_eq!(got.payload, payload);
}
