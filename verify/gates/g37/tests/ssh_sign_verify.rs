//! Gate G37 — SSH commit -S / verify-commit (`gpg.format=ssh`).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

fn require_tools() {
    assert!(
        Command::new("git")
            .arg("--version")
            .output()
            .unwrap()
            .status
            .success(),
        "stock git required"
    );
    assert!(
        Command::new("ssh-keygen").args(["-Y"]).output().is_ok(),
        "ssh-keygen -Y required"
    );
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
    panic!("missing arks binary");
}

fn arks(cwd: &Path, args: &[&str]) -> std::process::Output {
    Command::new(arks_bin())
        .args(args)
        .current_dir(cwd)
        .env("GIT_AUTHOR_NAME", "Arks Tester")
        .env("GIT_AUTHOR_EMAIL", "arks-g37@example.com")
        .env("GIT_COMMITTER_NAME", "Arks Tester")
        .env("GIT_COMMITTER_EMAIL", "arks-g37@example.com")
        .env("GIT_AUTHOR_DATE", "1000000000")
        .env("GIT_COMMITTER_DATE", "1000000000")
        .output()
        .expect("arks")
}

fn git(cwd: &Path, args: &[&str]) -> std::process::Output {
    Command::new("git")
        .args(args)
        .current_dir(cwd)
        .env("GIT_AUTHOR_NAME", "Arks Tester")
        .env("GIT_AUTHOR_EMAIL", "arks-g37@example.com")
        .env("GIT_COMMITTER_NAME", "Arks Tester")
        .env("GIT_COMMITTER_EMAIL", "arks-g37@example.com")
        .env("GIT_AUTHOR_DATE", "1000000000")
        .env("GIT_COMMITTER_DATE", "1000000000")
        .output()
        .expect("git")
}

fn assert_ok(out: &std::process::Output, label: &str) {
    assert!(
        out.status.success(),
        "{label}: {:?}\nstdout:\n{}\nstderr:\n{}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

fn assert_fail(out: &std::process::Output, label: &str) {
    assert!(!out.status.success(), "{label}: expected non-zero");
}

fn setup_ephemeral_ssh(dir: &Path) -> (PathBuf, PathBuf) {
    let priv_key = dir.join("id_ed25519");
    let out = Command::new("ssh-keygen")
        .args([
            "-t",
            "ed25519",
            "-f",
            priv_key.to_str().unwrap(),
            "-N",
            "",
            "-q",
            "-C",
            "arks-g37",
        ])
        .output()
        .expect("ssh-keygen");
    assert_ok(&out, "ssh-keygen");
    let pub_key = dir.join("id_ed25519.pub");
    let pub_line = fs::read_to_string(&pub_key).unwrap();
    let allowed = dir.join("allowed_signers");
    fs::write(
        &allowed,
        format!(
            "arks-g37@example.com namespaces=\"git\" {}\n",
            pub_line.trim()
        ),
    )
    .unwrap();
    (pub_key, allowed)
}

#[test]
fn g37_ssh_commit_sign_verify_stock_roundtrip() {
    require_tools();
    let tmp = TempDir::new().unwrap();
    let keys = tmp.path().join("keys");
    fs::create_dir_all(&keys).unwrap();
    let (pub_key, allowed) = setup_ephemeral_ssh(&keys);
    let repo = tmp.path().join("repo");
    fs::create_dir_all(&repo).unwrap();

    assert_ok(&arks(&repo, &["init"]), "init");
    fs::write(repo.join("README"), b"g37\n").unwrap();
    assert_ok(&arks(&repo, &["add", "README"]), "add");
    assert_ok(
        &arks(&repo, &["config", "user.signingKey", pub_key.to_str().unwrap()]),
        "signingKey",
    );
    assert_ok(&arks(&repo, &["config", "gpg.format", "ssh"]), "format");
    assert_ok(
        &arks(
            &repo,
            &[
                "config",
                "gpg.ssh.allowedSignersFile",
                allowed.to_str().unwrap(),
            ],
        ),
        "allowedSigners",
    );

    assert_ok(
        &arks(&repo, &["commit", "-S", "-m", "signed ssh commit"]),
        "commit -S",
    );
    let cat = git(&repo, &["cat-file", "-p", "HEAD"]);
    assert_ok(&cat, "cat-file");
    let body = String::from_utf8_lossy(&cat.stdout);
    assert!(body.contains("gpgsig -----BEGIN SSH SIGNATURE-----"), "{body}");
    assert!(body.contains("signed ssh commit"), "{body}");
    assert_ok(&arks(&repo, &["verify-commit", "HEAD"]), "arks verify");
    assert_ok(&git(&repo, &["verify-commit", "HEAD"]), "git verify");
}

#[test]
fn g37_refuse_openpgp_format() {
    require_tools();
    let tmp = TempDir::new().unwrap();
    let keys = tmp.path().join("keys");
    fs::create_dir_all(&keys).unwrap();
    let (pub_key, _) = setup_ephemeral_ssh(&keys);
    let repo = tmp.path().join("repo");
    fs::create_dir_all(&repo).unwrap();
    assert_ok(&arks(&repo, &["init"]), "init");
    fs::write(repo.join("f"), b"x\n").unwrap();
    assert_ok(&arks(&repo, &["add", "f"]), "add");
    assert_ok(
        &arks(&repo, &["config", "user.signingKey", pub_key.to_str().unwrap()]),
        "key",
    );
    assert_ok(&arks(&repo, &["config", "gpg.format", "openpgp"]), "openpgp");
    let out = arks(&repo, &["commit", "-S", "-m", "nope"]);
    assert_fail(&out, "must refuse openpgp");
}
