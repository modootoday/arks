//! Simple reflog append/show helpers (Wave Pc).
//!
//! Format matches stock Git log lines enough for interchange:
//! `oldhex newhex Name <email> timestamp tz\tmessage\n`
//! Cite: git-update-ref(1) / gitrepository-layout `logs/`.

use crate::cmd::shared::commit_signature;
use crate::error::{CliError, CliResult};
use arks_oid::Oid;
use arks_repo::Repository;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

fn zero_oid_hex() -> String {
    "0".repeat(40)
}

fn log_path_for_ref(repo: &Repository, refname: &str) -> PathBuf {
    let rel = if refname == "HEAD" {
        "HEAD".to_owned()
    } else if let Some(rest) = refname.strip_prefix("refs/") {
        rest.to_owned()
    } else {
        refname.to_owned()
    };
    repo.git_dir().join("logs").join(rel)
}

/// Lock sibling for a reflog path (`logs/HEAD` → `logs/HEAD.lock`).
pub fn reflog_lock_path(repo: &Repository, refname: &str) -> PathBuf {
    let mut s = log_path_for_ref(repo, refname).into_os_string();
    s.push(".lock");
    PathBuf::from(s)
}

/// REF-009 (M20 Cx-4): refuse when a reflog lock is held (lease ≠ skip).
pub fn refuse_if_reflog_locked(repo: &Repository, refname: &str) -> CliResult<()> {
    let lock = reflog_lock_path(repo, refname);
    if lock.is_file() {
        return Err(CliError::Msg(format!(
            "Unable to create '{}' (REF-009 — concurrent reflog append / logs lock held; lease ≠ skip)",
            lock.display()
        )));
    }
    Ok(())
}

/// Append one reflog line for `refname` (e.g. `HEAD` or `refs/heads/main`).
pub fn append_reflog(
    repo: &Repository,
    refname: &str,
    old: Option<&Oid>,
    new: &Oid,
    message: &str,
) -> CliResult<()> {
    // REF-009: honor exclusive logs/*.lock (api-lockfile / update-ref LOGGING).
    refuse_if_reflog_locked(repo, refname)?;
    let sig = commit_signature()?;
    let old_hex = old.map(Oid::to_hex).unwrap_or_else(zero_oid_hex);
    let line = format!(
        "{} {} {} <{}> {} {}\t{}\n",
        old_hex,
        new.to_hex(),
        sig.name,
        sig.email,
        sig.timestamp,
        sig.timezone,
        message
    );
    let path = log_path_for_ref(repo, refname);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut f = OpenOptions::new().create(true).append(true).open(&path)?;
    f.write_all(line.as_bytes())?;
    Ok(())
}

/// After a branch tip update that also moves HEAD, append both logs.
pub fn append_head_and_branch(
    repo: &Repository,
    branch_ref: &str,
    old: Option<&Oid>,
    new: &Oid,
    message: &str,
) -> CliResult<()> {
    append_reflog(repo, branch_ref, old, new, message)?;
    append_reflog(repo, "HEAD", old, new, message)?;
    Ok(())
}

pub fn show_reflog(repo: &Repository, refname: &str) -> CliResult<()> {
    let path = log_path_for_ref(repo, refname);
    if !path.is_file() {
        return Ok(());
    }
    let text = fs::read_to_string(&path)?;
    print!("{text}");
    Ok(())
}

pub fn resolve_reflog_name(args: &[String]) -> CliResult<String> {
    // reflog [show] [<ref>]
    let mut positional = Vec::new();
    for a in args {
        if a.starts_with('-') {
            return Err(CliError::NotImplemented(format!(
                "unsupported reflog option: {a}"
            )));
        }
        positional.push(a.as_str());
    }
    let mut i = 0;
    if positional.first() == Some(&"show") {
        i = 1;
    }
    let name = positional.get(i).copied().unwrap_or("HEAD");
    if name == "HEAD" {
        return Ok("HEAD".into());
    }
    if name.starts_with("refs/") {
        return Ok(name.to_owned());
    }
    Ok(format!("refs/heads/{name}"))
}
