//! Shared CLI wiring helpers (composition root helpers — not domain owners).
//! Pure tree/ancestry/patch/bundle logic lives in library rings.

use crate::error::{CliError, CliResult};
use arks_object::Signature;
use arks_oid::Oid;
use arks_refs::RefTarget;
use arks_repo::Repository;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) fn flatten_tree(
    repo: &Repository,
    tree_oid: &Oid,
    prefix: &str,
    out: &mut HashMap<String, Oid>,
) -> CliResult<()> {
    Ok(arks_repo::flatten_tree(repo, tree_oid, prefix, out)?)
}

pub(crate) fn head_blob_map(repo: &Repository) -> CliResult<HashMap<String, Oid>> {
    Ok(arks_repo::head_blob_map(repo)?)
}

pub(crate) fn tree_blob_map(repo: &Repository, rev_oid: &Oid) -> CliResult<HashMap<String, Oid>> {
    Ok(arks_repo::tree_blob_map(repo, rev_oid)?)
}

pub(crate) fn is_ancestor(repo: &Repository, ancestor: &Oid, tip: &Oid) -> CliResult<bool> {
    Ok(arks_repo::is_ancestor_first_parent(repo, ancestor, tip)?)
}

pub(crate) fn resolve_rev(repo: &Repository, rev: &str) -> CliResult<Oid> {
    // Minimal first-parent peel: `<rev>~N` / `<rev>~` (N=1). Cite: gitrevisions(7) 2.39.
    if let Some((base, rest)) = rev.rsplit_once('~') {
        if !base.is_empty() {
            let n: usize = if rest.is_empty() {
                1
            } else {
                rest.parse().map_err(|_| {
                    CliError::Msg(format!("unknown revision: {rev}"))
                })?
            };
            let mut oid = resolve_rev_base(repo, base)?;
            for _ in 0..n {
                let obj = repo.get_object(&oid)?;
                if obj.kind != arks_object::ObjectKind::Commit {
                    return Err(CliError::Msg(format!("{rev}: not a commit")));
                }
                let c = arks_object::Commit::decode(&obj.payload)?;
                oid = *c.parents.first().ok_or_else(|| {
                    CliError::Msg(format!("{rev}: no first parent"))
                })?;
            }
            return Ok(oid);
        }
    }
    resolve_rev_base(repo, rev)
}

fn resolve_rev_base(repo: &Repository, rev: &str) -> CliResult<Oid> {
    if let Ok(oid) = Oid::from_hex(rev) {
        return Ok(oid);
    }
    if rev == "HEAD" {
        return Ok(repo.head_oid()?);
    }
    // Try as ref name; also refs/heads/<name> and refs/remotes/<name>.
    if let Ok(oid) = repo.refs().resolve(rev) {
        return Ok(oid);
    }
    let as_heads = format!("refs/heads/{rev}");
    if let Ok(oid) = repo.refs().resolve(&as_heads) {
        return Ok(oid);
    }
    let as_remotes = format!("refs/remotes/{rev}");
    if let Ok(oid) = repo.refs().resolve(&as_remotes) {
        return Ok(oid);
    }
    let as_tags = format!("refs/tags/{rev}");
    if let Ok(oid) = repo.refs().resolve(&as_tags) {
        return Ok(oid);
    }
    Err(CliError::Msg(format!("unknown revision: {rev}")))
}

pub(crate) fn current_branch_name(repo: &Repository) -> CliResult<String> {
    match repo.head_target()? {
        RefTarget::Symbolic(s) => Ok(s
            .strip_prefix("refs/heads/")
            .unwrap_or(s.as_str())
            .to_owned()),
        RefTarget::Direct(oid) => Ok(format!("(detached at {})", &oid.to_hex()[..7])),
    }
}

pub(crate) fn collect_untracked(
    work: &Path,
    dir: &Path,
    indexed: &std::collections::HashSet<String>,
    out: &mut Vec<String>,
) -> CliResult<()> {
    // Pf4-1: recurse without per-directory sort; sort once at the top call.
    let before = out.len();
    collect_untracked_rec(work, dir, indexed, out)?;
    out[before..].sort();
    Ok(())
}

fn collect_untracked_rec(
    work: &Path,
    dir: &Path,
    indexed: &std::collections::HashSet<String>,
    out: &mut Vec<String>,
) -> CliResult<()> {
    let rd = fs::read_dir(dir)?;
    for ent in rd {
        let ent = ent?;
        let name = ent.file_name();
        if name == ".git" {
            continue;
        }
        let path = ent.path();
        let rel = path
            .strip_prefix(work)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        if path.is_dir() {
            collect_untracked_rec(work, &path, indexed, out)?;
        } else if !indexed.contains(&rel) {
            out.push(rel);
        }
    }
    Ok(())
}

pub(crate) fn file_times(meta: &fs::Metadata) -> (u32, u32, u32, u32) {
    let to_parts = |t: SystemTime| {
        let dur = t.duration_since(UNIX_EPOCH).unwrap_or_default();
        (dur.as_secs() as u32, dur.subsec_nanos())
    };
    let (mtime_sec, mtime_nsec) = meta.modified().ok().map(to_parts).unwrap_or((0, 0));
    let (ctime_sec, ctime_nsec) = meta
        .created()
        .ok()
        .map(to_parts)
        .unwrap_or((mtime_sec, mtime_nsec));
    (ctime_sec, ctime_nsec, mtime_sec, mtime_nsec)
}

/// Git-style index "up to date" short-circuit: matching size + mtime skips
/// re-hash (racy-git class; claimed subset for status/diff hot paths).
/// Cite: gitformat-index / `ce_match_stat` family in stock Git.
pub(crate) fn index_stat_matches(entry: &arks_index::IndexEntry, meta: &fs::Metadata) -> bool {
    let size = meta.len().min(u32::MAX as u64) as u32;
    if entry.size != size {
        return false;
    }
    let (_c_s, _c_n, mtime_sec, mtime_nsec) = file_times(meta);
    entry.mtime_sec == mtime_sec && entry.mtime_nsec == mtime_nsec
}

pub(crate) fn file_mode(meta: &fs::Metadata) -> u32 {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = meta.permissions().mode();
        if mode & 0o111 != 0 {
            0o100755
        } else {
            0o100644
        }
    }
    #[cfg(not(unix))]
    {
        let _ = meta;
        0o100644
    }
}

pub(crate) fn commit_signature() -> CliResult<Signature> {
    let name = std::env::var("GIT_AUTHOR_NAME")
        .or_else(|_| std::env::var("GIT_COMMITTER_NAME"))
        .unwrap_or_else(|_| "arks".into());
    let email = std::env::var("GIT_AUTHOR_EMAIL")
        .or_else(|_| std::env::var("GIT_COMMITTER_EMAIL"))
        .unwrap_or_else(|_| "arks@example.com".into());
    let timestamp = std::env::var("GIT_AUTHOR_DATE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0)
        });
    Ok(Signature {
        name,
        email,
        timestamp,
        timezone: "+0000".into(),
    })
}
