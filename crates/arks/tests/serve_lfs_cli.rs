//! End-to-end integration tests for `arks serve` native Git LFS server endpoint (`info/lfs`, E7).

use arks_server::{ServeHttpOptions, spawn_loopback_with_opts};
use std::fs;
use std::path::Path;
use std::process::{Command, Output};
use std::sync::atomic::Ordering;

fn arks_bin() -> &'static str {
    env!("CARGO_BIN_EXE_arks")
}

fn ok(dir: &Path, args: &[&str]) -> String {
    let out: Output = Command::new(arks_bin())
        .current_dir(dir)
        .env("GIT_AUTHOR_NAME", "LFS Server Test")
        .env("GIT_AUTHOR_EMAIL", "lfs-server@example.com")
        .env("GIT_COMMITTER_NAME", "LFS Server Test")
        .env("GIT_COMMITTER_EMAIL", "lfs-server@example.com")
        .args(args)
        .output()
        .expect("spawn arks");
    assert!(
        out.status.success(),
        "arks {:?} failed:\nstdout={}\nstderr={}",
        args,
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

#[test]
fn serve_lfs_push_pull_verify_and_no_lfs_opt_out() {
    let tmp = tempfile::tempdir().unwrap();
    let bare = tmp.path().join("remote.git");
    let client = tmp.path().join("client");
    fs::create_dir_all(&bare).unwrap();
    fs::create_dir_all(&client).unwrap();

    ok(&bare, &["init", "--bare", "-b", "main"]);
    ok(&client, &["init", "-b", "main"]);

    // Start real `arks-server` serving `bare` with LFS enabled (default).
    let (addr, stop, handle) =
        spawn_loopback_with_opts(bare.clone(), ServeHttpOptions::default()).unwrap();
    let endpoint = format!("http://{addr}/remote.git/info/lfs");

    // Track `*.gz` (starts with gzip magic 0x1f 0x8b to prove server does NOT corrupt binary PUTs!).
    ok(&client, &["lfs", "track", "*.gz"]);
    let payload: Vec<u8> = vec![0x1f, 0x8b, 0x08, 0x00, 0xde, 0xad, 0xbe, 0xef, 0x42, 0x99];
    fs::write(client.join("archive.gz"), &payload).unwrap();
    ok(&client, &["add", ".gitattributes", "archive.gz"]);
    ok(&client, &["commit", "-m", "add gzip lfs asset"]);

    // Push LFS object to live `arks-server` (`POST batch` -> `PUT object` -> `POST verify`).
    let push = Command::new(arks_bin())
        .current_dir(&client)
        .env("ARKS_LFS_ENDPOINT", &endpoint)
        .args(["lfs", "push"])
        .output()
        .unwrap();
    assert!(
        push.status.success(),
        "lfs push to arks-server failed: {}",
        String::from_utf8_lossy(&push.stderr)
    );
    let push_stdout = String::from_utf8_lossy(&push.stdout);
    assert!(
        push_stdout.contains("Uploaded 1 LFS object(s)"),
        "push_stdout={push_stdout}"
    );

    // Second push must be a no-op (server omits upload action for existing object).
    let push2 = Command::new(arks_bin())
        .current_dir(&client)
        .env("ARKS_LFS_ENDPOINT", &endpoint)
        .args(["lfs", "push"])
        .output()
        .unwrap();
    assert!(push2.status.success());
    assert!(
        String::from_utf8_lossy(&push2.stdout).contains("Uploaded 0 LFS object(s)"),
        "second push should skip existing object"
    );

    // Wipe client LFS cache, write pointer back to worktree, and `arks lfs pull` from `arks-server`!
    fs::remove_dir_all(client.join(".git/lfs")).unwrap();
    let ptr_content = ok(&client, &["show", "HEAD:archive.gz"]);
    fs::write(client.join("archive.gz"), format!("{ptr_content}\n")).unwrap();
    let pull = Command::new(arks_bin())
        .current_dir(&client)
        .env("ARKS_LFS_ENDPOINT", &endpoint)
        .args(["lfs", "pull"])
        .output()
        .unwrap();
    assert!(
        pull.status.success(),
        "lfs pull from arks-server failed: {}",
        String::from_utf8_lossy(&pull.stderr)
    );
    assert_eq!(fs::read(client.join("archive.gz")).unwrap(), payload);

    stop.store(true, Ordering::SeqCst);
    let _ = handle.join();

    // Now start `arks-server` with `disable_lfs: true` (`--no-lfs`) and confirm LFS fails closed!
    let (addr2, stop2, handle2) = spawn_loopback_with_opts(
        bare,
        ServeHttpOptions {
            disable_lfs: true,
            ..ServeHttpOptions::default()
        },
    )
    .unwrap();
    let endpoint2 = format!("http://{addr2}/remote.git/info/lfs");
    fs::remove_dir_all(client.join(".git/lfs")).unwrap();
    let blocked = Command::new(arks_bin())
        .current_dir(&client)
        .env("ARKS_LFS_ENDPOINT", &endpoint2)
        .args(["lfs", "fetch"])
        .output()
        .unwrap();
    assert!(!blocked.status.success(), "--no-lfs must fail closed");
    assert!(
        String::from_utf8_lossy(&blocked.stderr).contains("404"),
        "stderr={}",
        String::from_utf8_lossy(&blocked.stderr)
    );

    stop2.store(true, Ordering::SeqCst);
    let _ = handle2.join();
}
