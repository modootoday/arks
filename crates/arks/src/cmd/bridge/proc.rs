//! Child-process plumbing shared by the external tool bridges: running a
//! user-configured command line through `sh -c` (as git documents for tool,
//! filter and browser commands), passing `-c` overrides to children through
//! `GIT_CONFIG_PARAMETERS`, PATH lookups and one-line prompts on stdin.
//!
//! These helpers only ever start programs the user configured or named; they
//! never start `git` (rule 30).

use crate::error::{CliError, CliResult};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

/// Single-quote `s` for a POSIX shell.
pub fn sq(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// `GIT_CONFIG_PARAMETERS` for a child: the inherited value plus this
/// process's `-c` overrides, so configured tools see the same settings.
pub fn config_parameters() -> Option<String> {
    let mut out = std::env::var("GIT_CONFIG_PARAMETERS").unwrap_or_default();
    for (k, v) in arks_config::cli_overrides() {
        if !out.is_empty() {
            out.push(' ');
        }
        match v {
            Some(v) => out.push_str(&format!("{}={}", sq(&k), sq(&v))),
            None => out.push_str(&sq(&k)),
        }
    }
    (!out.is_empty()).then_some(out)
}

/// A `sh -c <script>` command with the configuration overrides exported.
/// Extra arguments become `$1…` (with `$0` set to `name`).
pub fn shell(script: &str, name: &str, args: &[String]) -> Command {
    let mut c = Command::new(shell_path());
    c.arg("-c").arg(script).arg(name).args(args);
    if let Some(p) = config_parameters() {
        c.env("GIT_CONFIG_PARAMETERS", p);
    }
    c
}

/// The shell used for configured command lines (`SHELL_PATH` build default
/// in git is `/bin/sh`; `sh` from PATH elsewhere).
pub fn shell_path() -> PathBuf {
    let bin = Path::new("/bin/sh");
    if bin.exists() {
        bin.to_path_buf()
    } else {
        PathBuf::from("sh")
    }
}

/// Run a command to completion, flushing our stdout first so output from the
/// child and from arks keeps its order.
pub fn run(mut c: Command) -> CliResult<ExitStatus> {
    let _ = std::io::stdout().flush();
    c.status()
        .map_err(|e| CliError::fatal(format!("cannot run {:?}: {e}", c.get_program())))
}

/// Exit status as a shell would report it (signals → 128 + n).
pub fn code_of(st: &ExitStatus) -> i32 {
    if let Some(c) = st.code() {
        return c;
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(s) = st.signal() {
            return 128 + s;
        }
    }
    1
}

/// Find `prog` on PATH (or check it directly when it contains a slash).
pub fn which(prog: &str) -> Option<PathBuf> {
    if prog.is_empty() {
        return None;
    }
    if prog.contains('/') {
        let p = PathBuf::from(prog);
        return is_executable(&p).then_some(p);
    }
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|d| d.join(prog))
        .find(|p| is_executable(p))
}

fn is_executable(p: &Path) -> bool {
    let Ok(meta) = std::fs::metadata(p) else {
        return false;
    };
    if !meta.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        meta.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

/// Print `prompt` on stdout and read one answer line from stdin (byte by
/// byte so later prompts still see the rest). `None` at end of input.
pub fn ask(prompt: &str) -> Option<String> {
    print!("{prompt}");
    let _ = std::io::stdout().flush();
    read_line_stdin()
}

/// Read one line from stdin without buffering ahead; `None` at end of input.
pub fn read_line_stdin() -> Option<String> {
    let mut buf = Vec::new();
    let mut byte = [0u8; 1];
    let mut stdin = std::io::stdin().lock();
    loop {
        match stdin.read(&mut byte) {
            Ok(0) => {
                if buf.is_empty() {
                    return None;
                }
                break;
            }
            Ok(_) => {
                if byte[0] == b'\n' {
                    break;
                }
                buf.push(byte[0]);
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => return None,
        }
    }
    let mut s = String::from_utf8_lossy(&buf).into_owned();
    if s.ends_with('\r') {
        s.pop();
    }
    Some(s)
}

/// A fresh private temporary directory `<tmp>/<prefix>XXXXXX`.
pub fn temp_dir(prefix: &str) -> CliResult<PathBuf> {
    let base = std::env::var_os("TMPDIR")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let seed = std::process::id() as u64
        ^ std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
    const ALPHA: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    for attempt in 0..64u64 {
        let mut n = seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(attempt);
        let mut suffix = String::new();
        for _ in 0..6 {
            suffix.push(ALPHA[(n % ALPHA.len() as u64) as usize] as char);
            n /= ALPHA.len() as u64;
        }
        let dir = base.join(format!("{prefix}{suffix}"));
        if std::fs::create_dir(&dir).is_ok() {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700));
            }
            return Ok(dir);
        }
    }
    Err(CliError::fatal(format!(
        "unable to create temporary directory {prefix}XXXXXX"
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quoting() {
        assert_eq!(sq("a'b"), "'a'\\''b'");
        assert_eq!(sq(""), "''");
    }

    #[test]
    fn temp_dirs_are_unique() {
        let a = temp_dir("arks-test-").unwrap();
        let b = temp_dir("arks-test-").unwrap();
        assert_ne!(a, b);
        let _ = std::fs::remove_dir(a);
        let _ = std::fs::remove_dir(b);
    }
}
