//! G1 oracle harness (advisory on host; authoritative inside clean-room).
//!
//! Requires in-PATH `git`. Compares arks loose encode/OID with stock Git
//! `hash-object` / `cat-file` for the M1 blob subset.
//!
//! Primary: https://git-scm.com/docs/gitformat-loose

use arks_hash::ObjectFormat;
use arks_object::{ObjectKind, RawObject, read_loose, write_loose};
use arks_oid::Oid;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

fn git_ok() -> bool {
    Command::new("git")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Drop inherited GIT_* so Actions/host wrappers cannot redirect stock-Git
/// into the workspace checkout (Windows flaky "not a git repository").
fn git_cmd() -> Command {
    let mut c = Command::new("git");
    c.env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_COMMON_DIR")
        .env_remove("GIT_OBJECT_DIRECTORY")
        .env_remove("GIT_INDEX_FILE");
    c
}

fn git_hash_object(payload: &[u8]) -> String {
    let mut child = git_cmd()
        .args(["hash-object", "--stdin"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn git hash-object");
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(payload)
        .expect("write stdin");
    let out = child.wait_with_output().expect("git hash-object");
    assert!(out.status.success(), "git hash-object failed: {out:?}");
    String::from_utf8(out.stdout).unwrap().trim().to_string()
}

fn abs_repo(repo: &Path) -> std::path::PathBuf {
    repo.canonicalize().unwrap_or_else(|_| repo.to_path_buf())
}

fn git_init(repo: &Path) {
    // Empty template avoids host template-hook copy races on Windows temp dirs.
    // Prefer absolute `git -C` — Windows Actions sometimes loses relative cwd.
    let repo_s = abs_repo(repo);
    let repo_s = repo_s.to_str().expect("repo utf-8 path");
    let st = git_cmd()
        .args(["-C", repo_s, "-c", "init.templateDir=", "init", "--quiet"])
        .status()
        .expect("git init");
    assert!(st.success());
}

fn git_hash_object_write(repo: &Path, payload: &[u8]) -> String {
    let repo_s = abs_repo(repo);
    let repo_s = repo_s.to_str().expect("repo utf-8 path");
    let mut child = git_cmd()
        .args(["-C", repo_s, "hash-object", "-w", "--stdin"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn git hash-object -w");
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(payload)
        .expect("write stdin");
    let out = child.wait_with_output().expect("git hash-object -w");
    assert!(out.status.success(), "git hash-object -w failed: {out:?}");
    String::from_utf8(out.stdout).unwrap().trim().to_string()
}

fn git_cat_file_blob(repo: &Path, oid: &str) -> Vec<u8> {
    let repo_s = abs_repo(repo);
    let repo_s = repo_s.to_str().expect("repo utf-8 path");
    let out = git_cmd()
        .args(["-C", repo_s, "cat-file", "blob", oid])
        .output()
        .expect("git cat-file");
    assert!(out.status.success(), "git cat-file failed: {out:?}");
    out.stdout
}

fn temp_repo() -> std::path::PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let tid = format!("{:?}", std::thread::current().id()).replace(['(', ')'], "");
    let root = std::env::temp_dir().join(format!("arks-g1-{stamp}-{tid}"));
    std::fs::create_dir_all(&root).unwrap();
    git_init(&root);
    root
}

#[test]
fn oid_parity_empty_and_hello() {
    if !git_ok() {
        eprintln!("skip: git not available (host advisory)");
        return;
    }
    for payload in [b"" as &[u8], b"hello"] {
        let expect = git_hash_object(payload);
        let obj = RawObject::new(ObjectKind::Blob, payload);
        let oid = obj.oid(ObjectFormat::Sha1).unwrap();
        assert_eq!(oid.to_hex(), expect, "payload={payload:?}");
    }
}

#[test]
fn arks_write_git_cat_file() {
    if !git_ok() {
        eprintln!("skip: git not available (host advisory)");
        return;
    }
    let repo = temp_repo();
    let objects = repo.join(".git/objects");
    let obj = RawObject::new(ObjectKind::Blob, b"hello");
    let oid = write_loose(&objects, ObjectFormat::Sha1, &obj).unwrap();
    let bytes = git_cat_file_blob(&repo, &oid.to_hex());
    assert_eq!(bytes, b"hello");
    let _ = std::fs::remove_dir_all(&repo);
}

#[test]
fn git_write_arks_read() {
    if !git_ok() {
        eprintln!("skip: git not available (host advisory)");
        return;
    }
    let repo = temp_repo();
    let hex = git_hash_object_write(&repo, b"hello");
    let oid = Oid::from_hex(&hex).unwrap();
    let objects = repo.join(".git/objects");
    let got = read_loose(&objects, &oid).unwrap();
    assert_eq!(got.kind, ObjectKind::Blob);
    assert_eq!(got.payload, b"hello");
    let _ = std::fs::remove_dir_all(&repo);
}

#[test]
fn binary_nul_blob_oid_and_roundtrip() {
    if !git_ok() {
        eprintln!("skip: git not available (host advisory)");
        return;
    }
    let payload = b"a\0b\0c";
    let expect = git_hash_object(payload);
    let obj = RawObject::new(ObjectKind::Blob, payload);
    assert_eq!(obj.oid(ObjectFormat::Sha1).unwrap().to_hex(), expect);

    let repo = temp_repo();
    let objects = repo.join(".git/objects");
    let oid = write_loose(&objects, ObjectFormat::Sha1, &obj).unwrap();
    let bytes = git_cat_file_blob(&repo, &oid.to_hex());
    assert_eq!(bytes, payload);
    let _ = std::fs::remove_dir_all(&repo);
}

#[test]
fn empty_blob_bidirectional_loose() {
    if !git_ok() {
        eprintln!("skip: git not available (host advisory)");
        return;
    }
    let repo = temp_repo();
    let objects = repo.join(".git/objects");
    let obj = RawObject::new(ObjectKind::Blob, b"");
    let oid = write_loose(&objects, ObjectFormat::Sha1, &obj).unwrap();
    assert_eq!(git_cat_file_blob(&repo, &oid.to_hex()), b"");

    let hex = git_hash_object_write(&repo, b"");
    let got = read_loose(&objects, &Oid::from_hex(&hex).unwrap()).unwrap();
    assert_eq!(got.payload, b"");
    let _ = std::fs::remove_dir_all(&repo);
}
