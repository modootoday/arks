//! Sync receive-pack githooks (Claimed honor list).
//!
//! Honored: `pre-receive`, `update`, `post-receive`, `post-update`.
//! Cite: <https://git-scm.com/docs/githooks>,
//! <https://git-scm.com/docs/git-receive-pack>.
//! Sync only — no invented async runners. Per-push stdin isolation.
//! `proc-receive` / `push-options` remain unclaimed (fail-closed: not advertised).

use crate::error::ServerError;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Result of a fail-closed hook (`pre-receive` / `update`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HookGate {
    /// Hook absent, not executable, or exited 0.
    Allow,
    /// Hook exited non-zero — reject per githooks / git-receive-pack.
    Decline,
}

/// Optional env for hooks during a quarantined receive-pack.
#[derive(Debug, Clone, Default)]
pub(crate) struct HookEnv {
    /// Absolute path to the quarantine objects directory (`GIT_QUARANTINE_PATH`).
    pub quarantine_path: Option<PathBuf>,
}

/// Run `$GIT_DIR/hooks/pre-receive` once with stdin lines of proposed updates.
pub(crate) fn run_pre_receive(
    git_dir: &Path,
    stdin_lines: &[u8],
    env: &HookEnv,
) -> Result<HookGate, ServerError> {
    run_named_hook(git_dir, "pre-receive", &[], Some(stdin_lines), true, env)
}

/// Run `$GIT_DIR/hooks/update` once for a single ref (`refname old-oid new-oid`).
pub(crate) fn run_update(
    git_dir: &Path,
    refname: &str,
    old_hex: &str,
    new_hex: &str,
    env: &HookEnv,
) -> Result<HookGate, ServerError> {
    run_named_hook(
        git_dir,
        "update",
        &[refname, old_hex, new_hex],
        None,
        true,
        env,
    )
}

/// Run `$GIT_DIR/hooks/post-receive` once with stdin of successfully updated refs.
///
/// Exit status does not change push outcome ([git-receive-pack] POST-RECEIVE HOOK).
pub(crate) fn run_post_receive(git_dir: &Path, stdin_lines: &[u8], env: &HookEnv) {
    let _ = run_named_hook(git_dir, "post-receive", &[], Some(stdin_lines), false, env);
}

/// Run `$GIT_DIR/hooks/post-update` with argv = updated ref names.
///
/// Cite: [git-receive-pack](https://git-scm.com/docs/git-receive-pack) POST-UPDATE HOOK.
/// Exit status ignored.
pub(crate) fn run_post_update(git_dir: &Path, updated_refs: &[&str], env: &HookEnv) {
    if updated_refs.is_empty() {
        return;
    }
    let _ = run_named_hook(git_dir, "post-update", updated_refs, None, false, env);
}

fn hook_path(git_dir: &Path, name: &str) -> PathBuf {
    git_dir.join("hooks").join(name)
}

/// Prefer Git for Windows bash over WSL `System32\bash.exe` (host advisory).
#[cfg(windows)]
fn windows_bash() -> Command {
    const CANDIDATES: &[&str] = &[
        r"C:\Program Files\Git\bin\bash.exe",
        r"C:\Program Files\Git\usr\bin\bash.exe",
        r"C:\Program Files (x86)\Git\bin\bash.exe",
    ];
    for c in CANDIDATES {
        if Path::new(c).is_file() {
            return Command::new(c);
        }
    }
    Command::new("bash")
}

/// Convert a Windows path to a Git-Bash-friendly form (`C:\a\b` → `/c/a/b`).
#[cfg(windows)]
fn bash_style_path(path: &Path) -> String {
    let s = path.to_string_lossy().replace('\\', "/");
    if let Some((drive, rest)) = s.split_once(':') {
        if drive.len() == 1 {
            let d = drive.chars().next().unwrap().to_ascii_lowercase();
            return format!("/{d}{rest}");
        }
    }
    s
}

fn is_runnable_hook(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    if path
        .file_name()
        .and_then(|s| s.to_str())
        .is_some_and(|n| n.ends_with(".sample"))
    {
        return false;
    }
    let Ok(meta) = fs::metadata(path) else {
        return false;
    };
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if meta.permissions().mode() & 0o111 == 0 {
            return false;
        }
    }
    #[cfg(not(unix))]
    {
        let _ = meta;
    }
    true
}

fn run_named_hook(
    git_dir: &Path,
    name: &str,
    args: &[&str],
    stdin: Option<&[u8]>,
    fail_on_nonzero: bool,
    env: &HookEnv,
) -> Result<HookGate, ServerError> {
    let path = hook_path(git_dir, name);
    if !is_runnable_hook(&path) {
        return Ok(HookGate::Allow);
    }
    // Unix: exec the hook path. Windows host advisory: bash (Git for Windows)
    // with a POSIX-style path so backslashes are not eaten by the shell.
    let mut cmd = {
        #[cfg(windows)]
        {
            let mut c = windows_bash();
            c.arg(bash_style_path(&path));
            c.args(args);
            c
        }
        #[cfg(not(windows))]
        {
            let mut c = Command::new(&path);
            c.args(args);
            c
        }
    };
    #[cfg(windows)]
    {
        // Hooks that expand `$GIT_DIR/...` must see a POSIX path under bash.
        cmd.env("GIT_DIR", bash_style_path(git_dir));
        if let Some(q) = &env.quarantine_path {
            cmd.env("GIT_QUARANTINE_PATH", bash_style_path(q));
        }
    }
    #[cfg(not(windows))]
    {
        cmd.env("GIT_DIR", git_dir);
        if let Some(q) = &env.quarantine_path {
            cmd.env("GIT_QUARANTINE_PATH", q);
        }
    }
    cmd.current_dir(git_dir);
    if stdin.is_some() {
        cmd.stdin(Stdio::piped());
    } else {
        cmd.stdin(Stdio::null());
    }
    // Inherit so operators see hook messages on the server process streams
    // (stock Git forwards to send-pack; we do not invent side-band for hooks).
    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());
    let mut child = cmd
        .spawn()
        .map_err(|e| ServerError::Msg(format!("hook {name}: {e}")))?;
    if let Some(bytes) = stdin {
        if let Some(mut hin) = child.stdin.take() {
            // Hook may exit before reading stdin (e.g. `exit 1`); BrokenPipe is
            // not a server fault — wait for status and apply fail-closed Decline.
            match hin.write_all(bytes) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => {}
                Err(e) => {
                    return Err(ServerError::Msg(format!("hook {name} stdin: {e}")));
                }
            }
        }
    }
    let status = child
        .wait()
        .map_err(|e| ServerError::Msg(format!("hook {name}: {e}")))?;
    if fail_on_nonzero && !status.success() {
        return Ok(HookGate::Decline);
    }
    Ok(HookGate::Allow)
}
