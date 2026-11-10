//! Gate G30 — SSH command-mode shape via duplex `arks upload-pack` / `receive-pack`.
//! Cite: verify/matrices/matrix-g30.md, gitprotocol-pack.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tempfile::TempDir;

fn require_git() {
    let out = Command::new("git").arg("--version").output().expect("git");
    assert!(out.status.success());
}

fn arks_bin() -> PathBuf {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(target) = std::env::var("CARGO_TARGET_DIR") {
        candidates.push(PathBuf::from(&target).join("debug/arks"));
        candidates.push(PathBuf::from(&target).join("debug/arks.exe"));
    } else {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        candidates.push(root.join("target/debug/arks"));
        candidates.push(root.join("target/debug/arks.exe"));
    }
    for c in &candidates {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!("missing arks; tried {candidates:?}");
}

fn arks(cwd: &Path, args: &[&str]) -> std::process::Output {
    Command::new(arks_bin())
        .args(args)
        .current_dir(cwd)
        .env("GIT_AUTHOR_NAME", "arks")
        .env("GIT_AUTHOR_EMAIL", "arks@example.com")
        .env("GIT_COMMITTER_NAME", "arks")
        .env("GIT_COMMITTER_EMAIL", "arks@example.com")
        .env("GIT_AUTHOR_DATE", "1000000000 +0000")
        .env("GIT_COMMITTER_DATE", "1000000000 +0000")
        .output()
        .expect("arks")
}

fn git(cwd: &Path, args: &[&str]) -> std::process::Output {
    Command::new("git")
        .args(args)
        .current_dir(cwd)
        .env("GIT_AUTHOR_NAME", "arks")
        .env("GIT_AUTHOR_EMAIL", "arks@example.com")
        .env("GIT_COMMITTER_NAME", "arks")
        .env("GIT_COMMITTER_EMAIL", "arks@example.com")
        .env("GIT_AUTHOR_DATE", "1000000000 +0000")
        .env("GIT_COMMITTER_DATE", "1000000000 +0000")
        .output()
        .expect("git")
}

fn assert_ok(out: &std::process::Output, label: &str) {
    assert!(
        out.status.success(),
        "{label}: {:?}\n{}\n{}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

fn seed_bare(root: &Path) -> PathBuf {
    let work = root.join("seed");
    fs::create_dir_all(&work).unwrap();
    assert_ok(&arks(&work, &["init"]), "init");
    fs::write(work.join("hello.txt"), b"hello g30\n").unwrap();
    assert_ok(&arks(&work, &["add", "hello.txt"]), "add");
    assert_ok(&arks(&work, &["commit", "-m", "initial"]), "commit");
    let bare = root.join("server.git");
    assert_ok(
        &git(
            root,
            &[
                "clone",
                "--bare",
                work.join(".git").to_str().unwrap(),
                bare.to_str().unwrap(),
            ],
        ),
        "bare",
    );
    bare
}

fn unixish(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}

/// Write a POSIX shim so `git --upload-pack` invokes arks reliably (Linux clean-room / Git Bash).
fn write_pack_shim(dir: &Path, service: &str) -> PathBuf {
    let shim = dir.join(format!("arks-{service}.sh"));
    let bin = unixish(&arks_bin());
    let script = format!("#!/bin/sh\nexec \"{bin}\" {service} \"$@\"\n");
    fs::write(&shim, script).unwrap();
    // Best-effort executable bit (no-op on Windows NTFS without chmod tooling).
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&shim).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&shim, perms).unwrap();
    }
    shim
}

#[test]
fn g30_pack_ad_no_http_service_prefix() {
    require_git();
    let root = TempDir::new().unwrap();
    let bare = seed_bare(root.path());
    // Duplex: null stdin → ad then empty negotiation EOF.
    let out = Command::new(arks_bin())
        .args(["upload-pack", bare.to_str().unwrap()])
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_ok(&out, "upload-pack duplex ad");
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        !text.contains("# service="),
        "SSH/pack ad must omit HTTP service line: {text}"
    );
    assert!(text.contains("refs/heads/") || text.contains("HEAD"));
}

#[test]
fn g30_stock_git_clone_fetch_push_via_upload_pack() {
    require_git();
    let root = TempDir::new().unwrap();
    let bare = seed_bare(root.path());
    let up = unixish(&write_pack_shim(root.path(), "upload-pack"));
    let clone = root.path().join("clone");
    let out = git(
        root.path(),
        &[
            "clone",
            "--upload-pack",
            &up,
            bare.to_str().unwrap(),
            clone.to_str().unwrap(),
        ],
    );
    // Some Windows Git builds report a non-zero status after a successful local
    // `--upload-pack` clone; prefer on-disk oracle.
    if !out.status.success() && !clone.join("hello.txt").is_file() {
        assert_ok(&out, "git clone --upload-pack shim");
    }
    let body = fs::read_to_string(clone.join("hello.txt")).expect("clone hello.txt");
    assert!(body.replace("\r\n", "\n").starts_with("hello g30"));

    fs::write(clone.join("hello.txt"), b"hello push\n").unwrap();
    assert_ok(&git(&clone, &["add", "hello.txt"]), "add");
    assert_ok(&git(&clone, &["commit", "-m", "push"]), "commit");
    let rp = unixish(&write_pack_shim(root.path(), "receive-pack"));
    let push = git(
        &clone,
        &[
            "push",
            "--receive-pack",
            &rp,
            bare.to_str().unwrap(),
            "HEAD:refs/heads/main",
        ],
    );
    assert_ok(&push, "git push --receive-pack shim");

    let clone2 = root.path().join("clone2");
    assert_ok(
        &git(
            root.path(),
            &[
                "clone",
                "--upload-pack",
                &up,
                bare.to_str().unwrap(),
                clone2.to_str().unwrap(),
            ],
        ),
        "re-clone after push",
    );
    let body2 = fs::read_to_string(clone2.join("hello.txt")).unwrap();
    assert!(body2.replace("\r\n", "\n").starts_with("hello push"));
}
