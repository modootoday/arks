//! OpenSSH client helper for pack protocol (M16 Sy-4).
//!
//! Spawns OpenSSH `ssh` (or `GIT_SSH_COMMAND` / `ARKS_SSH_COMMAND`) and speaks
//! gitprotocol-pack upload-pack / receive-pack over the duplex session.
//! Does **not** invoke a product-path `git` binary on the client — the remote
//! peer command is stock `git-upload-pack` / `git-receive-pack` as Git documents
//! for SSH transport.
//!
//! Primary: <https://git-scm.com/docs/gitprotocol-pack>,
//! <https://git-scm.com/docs/git-clone> (SSH URL forms),
//! <https://git-scm.com/docs/git-upload-pack>.

use crate::advertise::{Advertisement, parse_pack_advertisement};
use crate::error::RemoteError;
use crate::url::SshRemoteUrl;
use std::io::{Read, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

/// Remote pack service invoked over SSH.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SshService {
    UploadPack,
    ReceivePack,
}

impl SshService {
    fn remote_program(self) -> &'static str {
        match self {
            Self::UploadPack => "git-upload-pack",
            Self::ReceivePack => "git-receive-pack",
        }
    }
}

/// Live OpenSSH child with stdin/stdout connected to the pack peer.
pub struct SshSession {
    child: Child,
    stdin: Option<ChildStdin>,
    stdout: ChildStdout,
}

impl SshSession {
    /// Connect and start `git-upload-pack` / `git-receive-pack` on the peer.
    pub fn connect(url: &SshRemoteUrl, service: SshService) -> Result<Self, RemoteError> {
        let host = url.destination();
        let remote_cmd = format!(
            "{} {}",
            service.remote_program(),
            shell_single_quote(&url.path)
        );
        let mut child = spawn_ssh(url.port, &host, &remote_cmd)?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| RemoteError::Msg("ssh stdin missing".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| RemoteError::Msg("ssh stdout missing".into()))?;
        Ok(Self {
            child,
            stdin: Some(stdin),
            stdout,
        })
    }

    /// Read the v1 pack-protocol ref advertisement (through flush-pkt).
    pub fn read_advertisement(&mut self) -> Result<Advertisement, RemoteError> {
        let body = read_until_flush(&mut self.stdout)?;
        parse_pack_advertisement(&body)
    }

    pub fn write_all(&mut self, data: &[u8]) -> Result<(), RemoteError> {
        let stdin = self
            .stdin
            .as_mut()
            .ok_or_else(|| RemoteError::Msg("ssh stdin already closed".into()))?;
        stdin
            .write_all(data)
            .map_err(|e| RemoteError::Msg(format!("ssh write: {e}")))?;
        stdin
            .flush()
            .map_err(|e| RemoteError::Msg(format!("ssh flush: {e}")))?;
        Ok(())
    }

    /// Close the write half so the peer sees EOF after the client payload.
    pub fn close_stdin(&mut self) {
        self.stdin.take();
    }

    /// Read remaining stdout until EOF (upload-pack exits after the pack).
    pub fn read_to_end(&mut self) -> Result<Vec<u8>, RemoteError> {
        let mut buf = Vec::new();
        self.stdout
            .read_to_end(&mut buf)
            .map_err(|e| RemoteError::Msg(format!("ssh read: {e}")))?;
        Ok(buf)
    }

    /// Read pkt-lines until a flush-pkt (report-status).
    pub fn read_until_flush(&mut self) -> Result<Vec<u8>, RemoteError> {
        read_until_flush(&mut self.stdout)
    }

    /// Close stdin, wait for the ssh child, surface non-zero as error.
    pub fn finish(mut self) -> Result<(), RemoteError> {
        self.stdin.take();
        let status = self
            .child
            .wait()
            .map_err(|e| RemoteError::Msg(format!("ssh wait: {e}")))?;
        if status.success() {
            Ok(())
        } else {
            Err(RemoteError::Msg(format!(
                "ssh exited with status {status}"
            )))
        }
    }
}

fn spawn_ssh(port: Option<u16>, host: &str, remote_cmd: &str) -> Result<Child, RemoteError> {
    // Prefer GIT_SSH_COMMAND (Git-compatible), then ARKS_SSH_COMMAND, else `ssh`.
    // Port from `ssh://host:port/...` is passed as `-p` in `"$@"` even under
    // GIT_SSH_COMMAND (same shape as Git's connect.c).
    let ssh_command = std::env::var("GIT_SSH_COMMAND")
        .or_else(|_| std::env::var("ARKS_SSH_COMMAND"))
        .ok();
    let mut cmd = if let Some(wrapper) = ssh_command {
        // Match Git: sh -c '$GIT_SSH_COMMAND "$@"' 'sh' [-p port] host remote_cmd
        let mut c = Command::new("sh");
        c.arg("-c")
            .arg(format!("{wrapper} \"$@\""))
            .arg("sh");
        if let Some(p) = port {
            c.arg("-p").arg(p.to_string());
        }
        c.arg(host).arg(remote_cmd);
        c
    } else {
        let mut c = Command::new("ssh");
        c.arg("-o").arg("BatchMode=yes");
        if let Some(p) = port {
            c.arg("-p").arg(p.to_string());
        }
        c.arg(host).arg(remote_cmd);
        c
    };
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    cmd.spawn()
        .map_err(|e| RemoteError::Msg(format!("failed to spawn ssh: {e}")))
}

fn shell_single_quote(path: &str) -> String {
    // Wrap in single quotes; escape embedded ' as '\'' (POSIX).
    let mut out = String::from("'");
    for (i, part) in path.split('\'').enumerate() {
        if i > 0 {
            out.push_str("'\\''");
        }
        out.push_str(part);
    }
    out.push('\'');
    out
}

fn read_until_flush(reader: &mut impl Read) -> Result<Vec<u8>, RemoteError> {
    let mut out = Vec::new();
    loop {
        let mut len_bytes = [0u8; 4];
        reader
            .read_exact(&mut len_bytes)
            .map_err(|e| RemoteError::Protocol(format!("ssh pkt-len: {e}")))?;
        out.extend_from_slice(&len_bytes);
        let len = parse_hex4(&len_bytes)?;
        if len == 0 {
            break;
        }
        if len < 4 {
            return Err(RemoteError::Protocol(format!(
                "invalid pkt-len {len} on ssh stream"
            )));
        }
        let mut payload = vec![0u8; (len as usize) - 4];
        reader
            .read_exact(&mut payload)
            .map_err(|e| RemoteError::Protocol(format!("ssh pkt payload: {e}")))?;
        out.extend_from_slice(&payload);
    }
    Ok(out)
}

fn parse_hex4(bytes: &[u8; 4]) -> Result<u16, RemoteError> {
    let mut n = 0u16;
    for &b in bytes {
        let v = match b {
            b'0'..=b'9' => b - b'0',
            b'a'..=b'f' => b - b'a' + 10,
            _ => {
                return Err(RemoteError::Protocol(
                    "pkt-len is not 4 lowercase hex digits".into(),
                ));
            }
        };
        n = (n << 4) | u16::from(v);
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_quote_escapes_apostrophe() {
        assert_eq!(shell_single_quote("/tmp/r.git"), "'/tmp/r.git'");
        assert_eq!(shell_single_quote("a'b"), "'a'\\''b'");
    }
}
