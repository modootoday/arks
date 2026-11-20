//! `--access-hook=<path>`: external allow/decline decision per request.
//!
//! Primary: <https://git-scm.com/docs/git-daemon> (`--access-hook`): the hook
//! gets the service name, the repository path, `%H`, `%CH`, `%IP` and `%P` as
//! arguments and `REMOTE_ADDR` / `REMOTE_PORT` in its environment; exit 0
//! allows, non-zero declines, and an optional first stdout line is the
//! message sent to the client. It runs inside the repository directory (as
//! observed with stock `git daemon`). Runs the operator's program, never `git`.

use super::path::HostContext;
use std::io::Read;
use std::net::SocketAddr;
use std::path::Path;
use std::process::{Command, Stdio};

/// Outcome of a declined hook: optional client message.
pub(super) type Declined = Option<String>;

pub(super) struct HookRequest<'a> {
    pub service: &'a str,
    pub validated: &'a str,
    pub git_dir: &'a Path,
    pub host: Option<&'a HostContext>,
    pub peer: Option<SocketAddr>,
}

pub(super) fn run_access_hook(hook: &Path, req: &HookRequest<'_>) -> Result<(), Declined> {
    let empty = HostContext::default();
    let host = req.host.unwrap_or(&empty);
    let mut cmd = Command::new(hook);
    cmd.arg(req.service)
        .arg(req.validated)
        .arg(&host.hostname)
        .arg(&host.canonical)
        .arg(&host.ip)
        .arg(&host.port)
        .current_dir(req.git_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped());
    if let Some(peer) = req.peer {
        cmd.env("REMOTE_ADDR", peer.ip().to_string())
            .env("REMOTE_PORT", peer.port().to_string());
    }
    let mut child = cmd.spawn().map_err(|_| None)?;
    let mut out = Vec::new();
    if let Some(mut stdout) = child.stdout.take() {
        let _ = stdout.read_to_end(&mut out);
    }
    let status = child.wait().map_err(|_| None)?;
    if status.success() {
        return Ok(());
    }
    let text = String::from_utf8_lossy(&out);
    let first = text.lines().next().unwrap_or("").trim_end();
    Err((!first.is_empty()).then(|| first.to_string()))
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use tempfile::TempDir;

    fn hook(dir: &Path, body: &str) -> std::path::PathBuf {
        let p = dir.join("hook.sh");
        std::fs::write(&p, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
        p
    }

    #[test]
    fn hook_receives_documented_arguments_env_and_cwd() {
        let dir = TempDir::new().unwrap();
        let repo = dir.path().join("repo.git");
        std::fs::create_dir_all(&repo).unwrap();
        let log = dir.path().join("hook.log");
        let h = hook(
            dir.path(),
            &format!(
                "echo \"$*|$REMOTE_ADDR|$REMOTE_PORT|$(pwd)\" > {}",
                log.display()
            ),
        );
        let host = HostContext {
            hostname: "localhost".into(),
            canonical: "localhost".into(),
            ip: "127.0.0.1".into(),
            port: "9999".into(),
        };
        let req = HookRequest {
            service: "upload-pack",
            validated: "/srv/repo.git",
            git_dir: &repo,
            host: Some(&host),
            peer: Some("127.0.0.1:4242".parse().unwrap()),
        };
        run_access_hook(&h, &req).unwrap();
        let got = std::fs::read_to_string(&log).unwrap();
        let cwd = repo.canonicalize().unwrap();
        assert_eq!(
            got.trim_end(),
            format!(
                "upload-pack /srv/repo.git localhost localhost 127.0.0.1 9999|127.0.0.1|4242|{}",
                cwd.display()
            )
        );
    }

    #[test]
    fn declined_hook_reports_first_stdout_line() {
        let dir = TempDir::new().unwrap();
        let req = HookRequest {
            service: "upload-pack",
            validated: "/x",
            git_dir: dir.path(),
            host: None,
            peer: None,
        };
        let h = hook(dir.path(), "echo 'nope from hook'; echo second; exit 1");
        assert_eq!(
            run_access_hook(&h, &req),
            Err(Some("nope from hook".to_string()))
        );
        let h = hook(dir.path(), "exit 3");
        assert_eq!(run_access_hook(&h, &req), Err(None));
        assert_eq!(
            run_access_hook(&dir.path().join("missing"), &req),
            Err(None)
        );
    }
}
