//! FS6 closest parity for Git LFS — hand off to external `git-lfs` when present.
//!
//! arks does not embed an LFS server. When `git-lfs` is on PATH, `arks lfs …`
//! re-executes it with the same argv so muscle-memory `git lfs` → `arks lfs`
//! works. Pointer smudge/clean remains via configured `filter.lfs.*` helpers.
//!
//! Cite: <https://git-lfs.com/>, Full Symmetry FS6.

use crate::error::{CliError, CliResult};
use std::process::Command;

pub fn cmd_lfs(args: &[String]) -> CliResult<()> {
    let status = Command::new("git-lfs")
        .args(args)
        .status()
        .map_err(|e| {
            CliError::NotImplemented(format!(
                "lfs: external git-lfs not found ({e}); install git-lfs or configure filter.lfs clean/smudge (FS6 closest)"
            ))
        })?;
    if !status.success() {
        return Err(CliError::Msg(format!(
            "git-lfs exited {}",
            status.code().unwrap_or(-1)
        )));
    }
    Ok(())
}
