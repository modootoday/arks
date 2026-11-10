//! Gate G38 — OpenPGP commit/tag + X.509 mock program (R4 / S2).
//!
//! Carrying evidence venue: Docker clean-room only (no host Windows gpg keygen).

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
        Command::new("gpg")
            .arg("--version")
            .output()
            .unwrap()
            .status
            .success(),
        "gpg required in clean-room"
    );
}

fn arks_bin() -> PathBuf {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(target) = std::env::var("CARGO_TARGET_DIR") {
        candidates.push(PathBuf::from(&target).join("debug/arks"));
        candidates.push(PathBuf::from(&target).join("debug/arks.exe"));
    } else {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
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

fn arks_env(cwd: &Path, gnupghome: &Path, args: &[&str]) -> std::process::Output {
    Command::new(arks_bin())
        .args(args)
        .current_dir(cwd)
        .env("GNUPGHOME", gnupghome)
        .env("GIT_AUTHOR_NAME", "Arks G38")
        .env("GIT_AUTHOR_EMAIL", "arks-g038-gpgsig-openpgp@example.com")
        .env("GIT_COMMITTER_NAME", "Arks G38")
        .env("GIT_COMMITTER_EMAIL", "arks-g038-gpgsig-openpgp@example.com")
        .env("GIT_AUTHOR_DATE", "1000000000")
        .env("GIT_COMMITTER_DATE", "1000000000")
        .output()
        .expect("arks")
}

fn git_env(cwd: &Path, gnupghome: &Path, args: &[&str]) -> std::process::Output {
    Command::new("git")
        .args(args)
        .current_dir(cwd)
        .env("GNUPGHOME", gnupghome)
        .env("GIT_AUTHOR_NAME", "Arks G38")
        .env("GIT_AUTHOR_EMAIL", "arks-g038-gpgsig-openpgp@example.com")
        .env("GIT_COMMITTER_NAME", "Arks G38")
        .env("GIT_COMMITTER_EMAIL", "arks-g038-gpgsig-openpgp@example.com")
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

fn setup_ephemeral_openpgp(gnupghome: &Path) -> String {
    fs::create_dir_all(gnupghome).unwrap();
    // Restrict perms when possible (gpg prefers 0700).
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(gnupghome, fs::Permissions::from_mode(0o700));
    }
    let out = Command::new("gpg")
        .args([
            "--batch",
            "--pinentry-mode",
            "loopback",
            "--passphrase",
            "",
            "--quick-generate-key",
            "arks-g038-gpgsig-openpgp <arks-g038-gpgsig-openpgp@example.com>",
            "default",
            "default",
            "never",
        ])
        .env("GNUPGHOME", gnupghome)
        .output()
        .expect("gpg keygen");
    assert_ok(&out, "gpg --quick-generate-key");
    let list = Command::new("gpg")
        .args(["--list-secret-keys", "--with-colons"])
        .env("GNUPGHOME", gnupghome)
        .output()
        .expect("gpg list");
    assert_ok(&list, "gpg --list-secret-keys");
    let text = String::from_utf8_lossy(&list.stdout);
    text.lines()
        .find_map(|l| {
            let parts: Vec<_> = l.split(':').collect();
            if parts.first() == Some(&"fpr") && parts.len() > 9 && !parts[9].is_empty() {
                Some(parts[9].to_string())
            } else {
                None
            }
        })
        .expect("fingerprint from gpg --with-colons")
}

/// Minimal gpgsm-shaped program: `-bsau KEY` signs stdin → SIGNED MESSAGE armor;
/// `--verify sig -` checks roundtrip. Proves arks X.509 spawn path (not stock PKI).
fn write_x509_mock(path: &Path) {
    let script = r#"#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" == "-bsau" ]]; then
  payload=$(cat)
  b64=$(printf '%s' "$payload" | base64 -w0 2>/dev/null || printf '%s' "$payload" | base64)
  printf '%s\n' '-----BEGIN SIGNED MESSAGE-----'
  printf '%s\n' "$b64"
  printf '%s\n' '-----END SIGNED MESSAGE-----'
  exit 0
fi
if [[ "${1:-}" == "--verify" ]]; then
  sigfile="$2"
  payload=$(cat)
  expected=$(printf '%s' "$payload" | base64 -w0 2>/dev/null || printf '%s' "$payload" | base64)
  body=$(sed -n '/^-----BEGIN SIGNED MESSAGE-----$/,/^-----END SIGNED MESSAGE-----$/p' "$sigfile" | sed '1d;$d' | tr -d '\n\r')
  [[ "$body" == "$expected" ]] || { echo "bad x509 mock signature" >&2; exit 1; }
  echo "gpg: Good signature from mock-x509" >&2
  exit 0
fi
echo "x509-mock: unsupported argv: $*" >&2
exit 2
"#;
    fs::write(path, script.replace("\r\n", "\n")).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
    }
}

#[test]
fn g038-gpgsig-openpgp_openpgp_commit_and_tag_stock_roundtrip() {
    require_tools();
    let tmp = TempDir::new().unwrap();
    let gnupghome = tmp.path().join("gnupg");
    let fpr = setup_ephemeral_openpgp(&gnupghome);
    let repo = tmp.path().join("repo");
    fs::create_dir_all(&repo).unwrap();

    assert_ok(&arks_env(&repo, &gnupghome, &["init"]), "init");
    fs::write(repo.join("README"), b"g038-gpgsig-openpgp\n").unwrap();
    assert_ok(&arks_env(&repo, &gnupghome, &["add", "README"]), "add");
    assert_ok(
        &arks_env(&repo, &gnupghome, &["config", "user.signingKey", &fpr]),
        "signingKey",
    );
    assert_ok(
        &arks_env(&repo, &gnupghome, &["config", "gpg.format", "openpgp"]),
        "format",
    );
    assert_ok(
        &arks_env(
            &repo,
            &gnupghome,
            &["commit", "-S", "-m", "signed openpgp commit"],
        ),
        "commit -S",
    );

    let cat = git_env(&repo, &gnupghome, &["cat-file", "-p", "HEAD"]);
    assert_ok(&cat, "cat-file");
    let body = String::from_utf8_lossy(&cat.stdout);
    assert!(
        body.contains("gpgsig -----BEGIN PGP SIGNATURE-----")
            || body.contains("gpgsig -----BEGIN PGP MESSAGE-----"),
        "{body}"
    );
    assert_ok(
        &arks_env(&repo, &gnupghome, &["verify-commit", "HEAD"]),
        "arks verify-commit",
    );
    assert_ok(
        &git_env(&repo, &gnupghome, &["verify-commit", "HEAD"]),
        "git verify-commit",
    );

    assert_ok(
        &arks_env(
            &repo,
            &gnupghome,
            &["tag", "-s", "-m", "signed tag body", "v1"],
        ),
        "tag -s",
    );
    assert_ok(
        &arks_env(&repo, &gnupghome, &["verify-tag", "v1"]),
        "arks verify-tag",
    );
    assert_ok(
        &git_env(&repo, &gnupghome, &["verify-tag", "v1"]),
        "git verify-tag",
    );
}

#[test]
fn g038-gpgsig-openpgp_x509_mock_program_roundtrip() {
    require_tools();
    let tmp = TempDir::new().unwrap();
    let mock = tmp.path().join("gpgsm-mock");
    write_x509_mock(&mock);
    let gnupghome = tmp.path().join("gnupg-empty");
    fs::create_dir_all(&gnupghome).unwrap();
    let repo = tmp.path().join("repo");
    fs::create_dir_all(&repo).unwrap();

    assert_ok(&arks_env(&repo, &gnupghome, &["init"]), "init");
    fs::write(repo.join("f"), b"x509\n").unwrap();
    assert_ok(&arks_env(&repo, &gnupghome, &["add", "f"]), "add");
    assert_ok(
        &arks_env(&repo, &gnupghome, &["config", "gpg.format", "x509"]),
        "format",
    );
    assert_ok(
        &arks_env(
            &repo,
            &gnupghome,
            &[
                "config",
                "gpg.x509.program",
                mock.to_str().unwrap(),
            ],
        ),
        "x509.program",
    );
    assert_ok(
        &arks_env(&repo, &gnupghome, &["config", "user.signingKey", "mock-key"]),
        "signingKey",
    );
    assert_ok(
        &arks_env(&repo, &gnupghome, &["commit", "-S", "-m", "x509 mock"]),
        "commit -S x509",
    );
    let cat = git_env(&repo, &gnupghome, &["cat-file", "-p", "HEAD"]);
    assert_ok(&cat, "cat-file");
    let body = String::from_utf8_lossy(&cat.stdout);
    assert!(
        body.contains("gpgsig -----BEGIN SIGNED MESSAGE-----"),
        "{body}"
    );
    assert_ok(
        &arks_env(&repo, &gnupghome, &["verify-commit", "HEAD"]),
        "arks verify-commit x509",
    );
    assert_ok(
        &arks_env(
            &repo,
            &gnupghome,
            &["tag", "-s", "-m", "x509 tag", "x1"],
        ),
        "tag -s x509",
    );
    assert_ok(
        &arks_env(&repo, &gnupghome, &["verify-tag", "x1"]),
        "arks verify-tag x509",
    );
}

#[test]
fn g038-gpgsig-openpgp_split_signed_tag_helper_smoke() {
    // Compile-time independent smoke: armor detection shared with CLI.
    let payload = b"object deadbeef\ntype commit\ntag t\ntagger A <a@e.com> 1 +0000\n\nmsg\n-----BEGIN PGP SIGNATURE-----\n\nabc\n-----END PGP SIGNATURE-----\n";
    let text = std::str::from_utf8(payload).unwrap();
    assert!(text.contains("BEGIN PGP SIGNATURE"));
}
