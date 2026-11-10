//! Stock `git` in the clean-room image is the compatibility oracle (no gix).

use arks_hash::ObjectFormat;
use arks_object::{ObjectKind, RawObject, read_loose, write_loose};
use arks_oid::Oid;
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
    // Empty template avoids host template-hook copy races on Windows temp dirs.
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

#[test]
fn empty_blob_oid_matches_git_hash_object() {
    let obj = RawObject::new(ObjectKind::Blob, b"");
    let arks = obj.oid(ObjectFormat::Sha1).unwrap().to_hex();
    let git = git_hash_object_blob(b"");
    assert_eq!(arks, git);
}

#[test]
fn hello_blob_oid_matches_git_hash_object() {
    let obj = RawObject::new(ObjectKind::Blob, b"hello");
    let arks = obj.oid(ObjectFormat::Sha1).unwrap().to_hex();
    let git = git_hash_object_blob(b"hello");
    assert_eq!(arks, git, "known hello oid b6fc4c62…");
}

#[test]
fn arks_write_git_cat_file_roundtrip() {
    let dir = TempDir::new().unwrap();
    git_init_repo(dir.path());
    let objects = dir.path().join(".git").join("objects");
    let obj = RawObject::new(ObjectKind::Blob, b"hello");
    let oid = write_loose(&objects, ObjectFormat::Sha1, &obj).unwrap();
    let payload = git_cat_file_blob(dir.path(), &oid.to_hex());
    assert_eq!(payload, b"hello");
}

#[test]
fn git_hash_object_write_arks_read_roundtrip() {
    let dir = TempDir::new().unwrap();
    git_init_repo(dir.path());
    let git_dir = dir.path().join(".git");
    let objects = git_dir.join("objects");

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
    let got = read_loose(&objects, &oid).unwrap();
    assert_eq!(got.kind, ObjectKind::Blob);
    assert_eq!(got.payload, payload);
}

#[test]
fn binary_nul_blob_oid_matches_git() {
    let payload = b"x\0y\0z";
    let obj = RawObject::new(ObjectKind::Blob, payload);
    let arks = obj.oid(ObjectFormat::Sha1).unwrap().to_hex();
    let git = git_hash_object_blob(payload);
    assert_eq!(arks, git);
}

#[test]
fn empty_blob_arks_write_git_cat_file() {
    let dir = TempDir::new().unwrap();
    git_init_repo(dir.path());
    let objects = dir.path().join(".git").join("objects");
    let obj = RawObject::new(ObjectKind::Blob, b"");
    let oid = write_loose(&objects, ObjectFormat::Sha1, &obj).unwrap();
    assert_eq!(git_cat_file_blob(dir.path(), &oid.to_hex()), b"");
}
