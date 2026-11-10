//! Git LFS — in-process fetch/pull/push + handoff for other subcommands.
//!
//! Cite: <https://git-lfs.com/>, [[adr:root/20261110214800]]

use crate::error::{CliError, CliResult};
use crate::util::{open_repo, work_tree};
use arks_object::ObjectKind;
use arks_remote::{
    LfsPointer, lfs_batch_download, lfs_batch_upload, parse_lfs_pointer, read_local_lfs_object,
    write_local_lfs_object,
};
use std::process::Command;

pub fn cmd_lfs(args: &[String]) -> CliResult<()> {
    let sub = args.first().map(|s| s.as_str()).unwrap_or("");
    match sub {
        "fetch" | "pull" => lfs_fetch_pull(),
        "push" => lfs_push(),
        _ => handoff_git_lfs(args),
    }
}

fn handoff_git_lfs(args: &[String]) -> CliResult<()> {
    let status = Command::new("git-lfs").args(args).status().map_err(|e| {
        CliError::NotImplemented(format!(
            "lfs: external git-lfs not found ({e}); use `arks lfs fetch|pull|push` for claimed in-process subset or install git-lfs"
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

fn lfs_fetch_pull() -> CliResult<()> {
    let repo = open_repo()?;
    let work = work_tree(&repo)?;
    let index = crate::util::read_index(&repo)?;
    let mut fetched = 0usize;
    for e in index.entries.iter().filter(|e| e.stage() == 0) {
        let obj = repo.get_object(&e.oid).map_err(|err| CliError::Msg(err.to_string()))?;
        if obj.kind != ObjectKind::Blob {
            continue;
        }
        let Some(ptr) = parse_lfs_pointer(&obj.payload) else {
            continue;
        };
        if read_local_lfs_object(repo.git_dir(), &ptr)
            .map_err(|err| CliError::Msg(err.to_string()))?
            .is_some()
        {
            continue;
        }
        lfs_batch_download(&repo, &ptr).map_err(|err| CliError::Msg(err.to_string()))?;
        fetched += 1;
        let _ = work;
    }
    println!("lfs fetch: {fetched} object(s)");
    Ok(())
}

fn lfs_push() -> CliResult<()> {
    let repo = open_repo()?;
    let index = crate::util::read_index(&repo)?;
    let mut pushed = 0usize;
    for e in index.entries.iter().filter(|e| e.stage() == 0) {
        let obj = repo.get_object(&e.oid).map_err(|err| CliError::Msg(err.to_string()))?;
        if obj.kind != ObjectKind::Blob {
            continue;
        }
        let Some(ptr) = parse_lfs_pointer(&obj.payload) else {
            continue;
        };
        let Some(bytes) = read_local_lfs_object(repo.git_dir(), &ptr)
            .map_err(|err| CliError::Msg(err.to_string()))?
        else {
            return Err(CliError::Msg(format!(
                "lfs push: missing local object {}",
                ptr.oid_hex
            )));
        };
        lfs_batch_upload(&repo, &ptr, &bytes).map_err(|err| CliError::Msg(err.to_string()))?;
        pushed += 1;
    }
    println!("lfs push: {pushed} object(s)");
    Ok(())
}

/// Helper for tests / clean filter: write bytes into local LFS store + return pointer text.
#[allow(dead_code)]
pub fn pointer_for_bytes(oid_hex: &str, bytes: &[u8]) -> String {
    format!(
        "version https://git-lfs.github.com/spec/v1\noid sha256:{oid_hex}\nsize {}\n",
        bytes.len()
    )
}

#[allow(dead_code)]
pub fn store_bytes(repo_git: &std::path::Path, oid_hex: &str, bytes: &[u8]) -> CliResult<()> {
    write_local_lfs_object(repo_git, oid_hex, bytes).map_err(|e| CliError::Msg(e.to_string()))
}

#[allow(dead_code)]
pub fn make_ptr(oid_hex: &str, size: u64) -> LfsPointer {
    LfsPointer {
        oid_hex: oid_hex.to_string(),
        size,
    }
}
