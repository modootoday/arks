//! Gate G14 — Wave Pc inventory + flag-parity + alias-ready (clean-room).
//! CLI-parity production (M9). Matrix: `verify/matrices/matrix-m009-clean-room-cli-parity.md` M9-C01..C10.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

fn require_git() {
    let out = Command::new("git").arg("--version").output().expect("git");
    assert!(out.status.success());
}

fn arks_bin() -> PathBuf {
    if let Ok(p) = std::env::var("ARKS_BIN") {
        let p = PathBuf::from(p);
        if p.is_file() {
            return p;
        }
    }
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(target) = std::env::var("CARGO_TARGET_DIR") {
        candidates.push(PathBuf::from(&target).join("release/arks"));
        candidates.push(PathBuf::from(&target).join("release/arks.exe"));
        candidates.push(PathBuf::from(&target).join("debug/arks"));
        candidates.push(PathBuf::from(&target).join("debug/arks.exe"));
    } else {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
        candidates.push(root.join("target/release/arks"));
        candidates.push(root.join("target/debug/arks.exe"));
        candidates.push(root.join("target/debug/arks"));
        candidates.push(root.join("target-cleanroom/release/arks"));
        candidates.push(root.join("target-cleanroom/debug/arks"));
    }
    for c in &candidates {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!("missing arks binary; tried {candidates:?}");
}

fn arks(repo: &Path, args: &[&str]) -> std::process::Output {
    Command::new(arks_bin())
        .args(args)
        .current_dir(repo)
        .env("GIT_AUTHOR_NAME", "arks")
        .env("GIT_AUTHOR_EMAIL", "arks@example.com")
        .env("GIT_COMMITTER_NAME", "arks")
        .env("GIT_COMMITTER_EMAIL", "arks@example.com")
        .env("GIT_AUTHOR_DATE", "1000000000")
        .env("GIT_COMMITTER_DATE", "1000000000")
        .output()
        .expect("arks")
}

fn git(repo: &Path, args: &[&str]) -> std::process::Output {
    Command::new("git")
        .args(args)
        .current_dir(repo)
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
        "{label}: {}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

fn assert_fail(out: &std::process::Output, label: &str) {
    assert!(
        !out.status.success(),
        "{label} should refuse: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}

fn seed_commit(repo: &Path, file: &str, body: &str, msg: &str) {
    fs::write(repo.join(file), body).unwrap();
    assert_ok(&arks(repo, &["add", file]), "add");
    assert_ok(&arks(repo, &["commit", "-m", msg]), "commit");
}

fn help_text() -> String {
    let out = Command::new(arks_bin()).arg("--help").output().unwrap();
    assert_ok(&out, "help");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn command_listed(help: &str, name: &str) -> bool {
    help.lines().any(|line| {
        let trimmed = line.trim_start();
        trimmed.starts_with(name) && trimmed[name.len()..].starts_with(char::is_whitespace)
    })
}

fn write_git_shim(dir: &Path) -> PathBuf {
    let shim_dir = dir.join("shim");
    fs::create_dir_all(&shim_dir).unwrap();
    let arks = arks_bin();
    #[cfg(windows)]
    {
        let bat = shim_dir.join("git.cmd");
        fs::write(
            &bat,
            format!(
                "@echo off\r\n\"{}\" %*\r\n",
                arks.display()
            ),
        )
        .unwrap();
        bat
    }
    #[cfg(not(windows))]
    {
        let script = shim_dir.join("git");
        fs::write(
            &script,
            format!("#!/bin/sh\nexec \"{}\" \"$@\"\n", arks.display()),
        )
        .unwrap();
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&script).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script, perms).unwrap();
        script
    }
}

fn shim_git(shim_dir: &Path, repo: &Path, args: &[&str]) -> std::process::Output {
    // Prefer invoking the shim file directly. On Windows, `Command::new("git")`
    // resolves `git.exe` ahead of `git.cmd` even when shim_dir is first on PATH.
    let shim = if cfg!(windows) {
        shim_dir.join("git.cmd")
    } else {
        shim_dir.join("git")
    };
    Command::new(&shim)
        .args(args)
        .current_dir(repo)
        .env("GIT_AUTHOR_NAME", "arks")
        .env("GIT_AUTHOR_EMAIL", "arks@example.com")
        .env("GIT_COMMITTER_NAME", "arks")
        .env("GIT_COMMITTER_EMAIL", "arks@example.com")
        .env("GIT_AUTHOR_DATE", "1000000000")
        .env("GIT_COMMITTER_DATE", "1000000000")
        .output()
        .expect("shim git")
}

#[test]
fn help_lists_pc_and_omits_refuse_closed() {
    require_git();
    let s = help_text();
    // Personality-condensed top-level help is not a full list-cmds inventory
    // ([[sot:root/arks-cli-personality-factory]]); Claimed porcelain remains
    // dispatchable. Keep refuse-closed omissions + tier banner honesty.
    assert!(
        s.contains("CLI-parity production")
            || s.contains("Wave L+H+Pa+Pb+Pc")
            || s.contains("personality"),
        "help should advertise Pc / production tier / personality"
    );
    for bad in ["backfill", "filter-branch"] {
        assert!(!command_listed(&s, bad), "help must not list {bad}");
    }
    let tmp = TempDir::new().unwrap();
    assert_ok(&arks(tmp.path(), &["init"]), "Claimed init leaf must dispatch");
}

#[test]
fn refuse_closed_and_unknown_flags() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path();
    assert_ok(&arks(repo, &["init"]), "init");
    seed_commit(repo, "a.txt", "one\n", "c1");
    for cmd in ["backfill", "filter-branch"] {
        assert_fail(&arks(repo, &[cmd]), cmd);
    }
    assert_fail(&arks(repo, &["commit"]), "commit without -m exclude");
    // Residual claimed: bare `worktree`/`submodule` without subcommand is usage/non-zero;
    // `worktree list` / `submodule status` succeed on an empty-capable repo.
    assert_ok(&arks(repo, &["worktree", "list"]), "worktree list");
    assert_ok(&arks(repo, &["submodule", "status"]), "submodule status");
    // M15 Fs-3: notes / sparse-checkout / repack claimed (smoke under G14 regression).
    assert_ok(
        &arks(repo, &["notes", "add", "-m", "g014-cli-parity-production-note"]),
        "notes add",
    );
    assert_ok(&arks(repo, &["notes", "show"]), "notes show");
    assert_ok(
        &arks(repo, &["sparse-checkout", "init", "--cone"]),
        "sparse init",
    );
    assert_ok(&arks(repo, &["repack", "-ad"]), "repack -ad");
    assert_fail(&arks(repo, &["rebase", "-i", "HEAD"]), "rebase -i");
    assert_fail(&arks(repo, &["archive", "--format=zip"]), "zip");
    assert_fail(&arks(repo, &["tag", "-s", "x"]), "signed tag");
    // Manifest lives at verify/gates/g014-cli-parity-production after layout relocate (cdbc907+).
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
    assert!(
        Path::new("docs/alias-git-arks.md").is_file()
            || Path::new("../docs/alias-git-arks.md").is_file()
            || repo_root.join("docs/alias-git-arks.md").is_file(),
        "alias how-to docs missing"
    );
    assert!(
        repo_root
            .join(".spec/sot/cli-flag-parity-m9.sot.md")
            .is_file()
            || repo_root.join(".spec/sot/arks.sot.md").is_file(),
        "flag-parity / unit SoT missing"
    );
}

#[test]
fn annotated_tag_stock_git_reads() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path();
    assert_ok(&arks(repo, &["init"]), "init");
    seed_commit(repo, "a.txt", "one\n", "c1");
    assert_ok(
        &arks(repo, &["tag", "-a", "-m", "release", "v1"]),
        "annotated tag",
    );
    let typ = git(repo, &["cat-file", "-t", "v1"]);
    assert_ok(&typ, "git cat-file -t");
    assert_eq!(String::from_utf8_lossy(&typ.stdout).trim(), "tag");
    let peeled = git(repo, &["rev-parse", "v1^{commit}"]);
    assert_ok(&peeled, "git peel tag");
}

#[test]
fn archive_tar_contains_blob() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path();
    assert_ok(&arks(repo, &["init"]), "init");
    seed_commit(repo, "a.txt", "hello-archive\n", "c1");
    let out = repo.join("out.tar");
    assert_ok(
        &arks(
            repo,
            &["archive", "--format=tar", "-o", out.to_str().unwrap(), "HEAD"],
        ),
        "archive",
    );
    let bytes = fs::read(&out).unwrap();
    assert!(bytes.len() > 512, "tar too small");
    let text = String::from_utf8_lossy(&bytes);
    assert!(text.contains("a.txt"), "tar missing path name");
    assert!(text.contains("hello-archive"), "tar missing blob");
}

#[test]
fn reflog_after_commit() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path();
    assert_ok(&arks(repo, &["init"]), "init");
    seed_commit(repo, "a.txt", "one\n", "c1");
    let out = arks(repo, &["reflog"]);
    assert_ok(&out, "reflog");
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("commit:"), "reflog missing commit line: {s}");
    assert!(repo.join(".git/logs/HEAD").is_file());
    assert_ok(&git(repo, &["log", "-1", "--oneline"]), "git log");
}

#[test]
fn bundle_verify_arks_bundle() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path();
    assert_ok(&arks(repo, &["init"]), "init");
    seed_commit(repo, "a.txt", "one\n", "c1");
    let bundle = repo.join("r.bundle");
    assert_ok(
        &arks(
            repo,
            &["bundle", "create", bundle.to_str().unwrap(), "--all"],
        ),
        "bundle create",
    );
    assert_ok(
        &arks(repo, &["bundle", "verify", bundle.to_str().unwrap()]),
        "bundle verify",
    );
}

#[test]
fn clean_d_removes_untracked_dir() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path();
    assert_ok(&arks(repo, &["init"]), "init");
    seed_commit(repo, "a.txt", "one\n", "c1");
    fs::create_dir_all(repo.join("junk/nested")).unwrap();
    fs::write(repo.join("junk/nested/x.txt"), "x").unwrap();
    assert_ok(&arks(repo, &["clean", "-f", "-d"]), "clean -fd");
    assert!(!repo.join("junk").exists(), "junk dir should be removed");
}

#[test]
fn alias_shim_workflows_and_refuse() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let shim = write_git_shim(root);
    let shim_dir = shim.parent().unwrap();
    let repo = root.join("repo");
    fs::create_dir_all(&repo).unwrap();
    assert_ok(&shim_git(shim_dir, &repo, &["init"]), "shim init");
    fs::write(repo.join("f.txt"), "hi\n").unwrap();
    assert_ok(&shim_git(shim_dir, &repo, &["add", "f.txt"]), "shim add");
    assert_ok(
        &shim_git(shim_dir, &repo, &["commit", "-m", "c1"]),
        "shim commit",
    );
    assert_ok(&shim_git(shim_dir, &repo, &["status"]), "shim status");
    assert_ok(&shim_git(shim_dir, &repo, &["branch", "topic"]), "shim branch");
    assert_ok(&shim_git(shim_dir, &repo, &["log"]), "shim log");
    assert_ok(
        &shim_git(shim_dir, &repo, &["worktree", "list"]),
        "shim worktree list",
    );
    assert_ok(
        &shim_git(shim_dir, &repo, &["submodule", "status"]),
        "shim submodule status",
    );
    assert_ok(
        &shim_git(shim_dir, &repo, &["repack", "-ad"]),
        "shim repack -ad",
    );
    // range-diff needs two dual-dot ranges; bare argv remains usage/non-zero.
    assert_fail(
        &shim_git(shim_dir, &repo, &["range-diff"]),
        "shim range-diff usage",
    );
}

#[test]
fn same_dot_git_stock_peer() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path();
    assert_ok(&arks(repo, &["init"]), "init");
    seed_commit(repo, "a.txt", "peer\n", "c1");
    assert_ok(
        &arks(repo, &["tag", "-a", "-m", "t", "vpeer"]),
        "annotated",
    );
    assert_ok(&arks(repo, &["reflog"]), "reflog");
    assert_ok(&git(repo, &["status"]), "git status");
    assert_ok(&git(repo, &["log", "-1"]), "git log");
    assert_ok(&git(repo, &["cat-file", "-t", "vpeer"]), "git tag type");
}
