//! Sync Git hook runner (M15 Fs-1).
//!
//! Cite: <https://git-scm.com/docs/githooks>
//! Sync hooks only — no async / out-of-band scheduler (no L3).

use crate::error::{CliError, CliResult};
use crate::util::work_tree;
use arks_repo::Repository;
use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};

/// Run named sync hooks from `$GIT_DIR/hooks` unless `no_verify`.
///
/// `.sample` files are ignored (stock Git install samples). Non-zero hook
/// exit aborts the porcelain command (Git semantics).
pub(crate) fn run_sync_hooks(
    repo: &Repository,
    no_verify: bool,
    names: &[&str],
    stdin: Option<&[u8]>,
) -> CliResult<()> {
    if no_verify {
        return Ok(());
    }
    let hooks_dir = repo.git_dir().join("hooks");
    if !hooks_dir.is_dir() {
        return Ok(());
    }
    let work = work_tree(repo).ok();
    for name in names {
        let path = hooks_dir.join(name);
        if !path.is_file() {
            continue;
        }
        // Ignore installer samples (not executable product hooks).
        if path
            .file_name()
            .and_then(|s| s.to_str())
            .is_some_and(|n| n.ends_with(".sample"))
        {
            continue;
        }
        let meta = fs::metadata(&path).map_err(|e| CliError::Msg(e.to_string()))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if meta.permissions().mode() & 0o111 == 0 {
                continue;
            }
        }
        #[cfg(not(unix))]
        {
            let _ = meta;
        }
        // Unix: execute hook path. Windows host advisory: prefer bash (Git for
        // Windows) so #!/bin/sh hooks match clean-room Linux behavior.
        let mut cmd = {
            #[cfg(windows)]
            {
                let mut c = Command::new("bash");
                c.arg(&path);
                c
            }
            #[cfg(not(windows))]
            {
                Command::new(&path)
            }
        };
        cmd.env("GIT_DIR", repo.git_dir());
        if let Some(ref wt) = work {
            cmd.current_dir(wt);
            cmd.env("GIT_WORK_TREE", wt);
        }
        if stdin.is_some() {
            cmd.stdin(Stdio::piped());
        } else {
            cmd.stdin(Stdio::null());
        }
        cmd.stdout(Stdio::inherit());
        cmd.stderr(Stdio::inherit());
        let mut child = cmd
            .spawn()
            .map_err(|e| CliError::Msg(format!("hook {name}: {e}")))?;
        if let Some(bytes) = stdin {
            if let Some(mut hin) = child.stdin.take() {
                hin.write_all(bytes)
                    .map_err(|e| CliError::Msg(format!("hook {name} stdin: {e}")))?;
            }
        }
        let status = child
            .wait()
            .map_err(|e| CliError::Msg(format!("hook {name}: {e}")))?;
        if !status.success() {
            return Err(CliError::Msg(format!(
                "hook {name} failed with {}",
                status
                    .code()
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "signal".into())
            )));
        }
    }
    Ok(())
}
