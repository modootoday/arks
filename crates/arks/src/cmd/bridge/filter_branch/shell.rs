//! The filter shell: one long-lived `sh` that evaluates every filter, so
//! functions and variables from `--setup` (and from earlier filters) stay in
//! scope across commits, as git-filter-branch documents ("evaluated in the
//! shell context using the eval command").
//!
//! arks writes shell snippets to the shell's stdin. The shell's stdout is a
//! pipe back to arks: everything a filter prints is forwarded to arks' own
//! stdout unchanged, and each snippet ends by printing a random marker with
//! its exit status, which tells arks the step finished. Filters other than
//! the message / parent / tag-name filters read `/dev/null` on stdin.

use super::super::proc;
use crate::error::{CliError, CliResult};
use std::io::{BufReader, Read, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, ChildStdout, Stdio};

pub struct FilterShell {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    token: String,
}

impl FilterShell {
    /// Start `sh` in `cwd` with `env` exported and `preamble` evaluated.
    pub fn start(cwd: &Path, env: &[(String, String)], preamble: &str) -> CliResult<Self> {
        // `sh -s` reads its commands from stdin.
        let mut c = std::process::Command::new(proc::shell_path());
        c.arg("-s");
        if let Some(p) = proc::config_parameters() {
            c.env("GIT_CONFIG_PARAMETERS", p);
        }
        c.current_dir(cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
        for (k, v) in env {
            c.env(k, v);
        }
        let mut child = c
            .spawn()
            .map_err(|e| CliError::fatal(format!("cannot run the filter shell: {e}")))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| CliError::fatal("filter shell stdin"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| CliError::fatal("filter shell stdout"))?;
        let token = format!(
            "\u{1}arks-filter-branch-{}-{}\u{1}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        );
        let mut sh = Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
            token,
        };
        sh.run(preamble)?;
        Ok(sh)
    }

    /// Evaluate `snippet` and return its exit status. Output the snippet
    /// prints is copied to our stdout. A dead shell is an error.
    pub fn run(&mut self, snippet: &str) -> CliResult<i32> {
        let _ = std::io::stdout().flush();
        let cmd = format!(
            "{snippet}\n__arks_st=$?\nprintf '%s%s\\n' {} \"$__arks_st\"\n",
            proc::sq(&self.token)
        );
        self.stdin
            .write_all(cmd.as_bytes())
            .and_then(|_| self.stdin.flush())
            .map_err(|_| CliError::silent(1))?;
        let tok = self.token.as_bytes().to_vec();
        let mut out = std::io::stdout().lock();
        let mut pending: Vec<u8> = Vec::new();
        let mut byte = [0u8; 4096];
        loop {
            let n = self
                .stdout
                .read(&mut byte)
                .map_err(|_| CliError::silent(1))?;
            if n == 0 {
                out.write_all(&pending)?;
                out.flush()?;
                // The shell exited (a filter ran `exit`): report its status.
                let code = self
                    .child
                    .wait()
                    .ok()
                    .map(|st| proc::code_of(&st))
                    .unwrap_or(1);
                return Err(CliError::silent(code.clamp(1, 255) as u8));
            }
            pending.extend_from_slice(&byte[..n]);
            if let Some(pos) = find(&pending, &tok) {
                out.write_all(&pending[..pos])?;
                out.flush()?;
                let rest = &pending[pos + tok.len()..];
                // The status line follows the marker; read until its newline.
                let mut line = rest.to_vec();
                while !line.contains(&b'\n') {
                    let n = self
                        .stdout
                        .read(&mut byte)
                        .map_err(|_| CliError::silent(1))?;
                    if n == 0 {
                        break;
                    }
                    line.extend_from_slice(&byte[..n]);
                }
                let text = String::from_utf8_lossy(&line);
                let code = text.trim().parse().unwrap_or(1);
                return Ok(code);
            }
            // Forward what cannot be part of a marker yet.
            let keep = tok.len().saturating_sub(1).min(pending.len());
            let flush_upto = pending.len() - keep;
            if flush_upto > 0 {
                out.write_all(&pending[..flush_upto])?;
                pending.drain(..flush_upto);
            }
        }
    }

    pub fn finish(mut self) {
        let _ = self.stdin.write_all(b"exit 0\n");
        let _ = self.stdin.flush();
        drop(self.stdin);
        let _ = self.child.wait();
    }
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || hay.len() < needle.len() {
        return None;
    }
    hay.windows(needle.len()).position(|w| w == needle)
}
