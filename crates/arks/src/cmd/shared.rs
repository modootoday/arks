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

/// Resolve a revision per gitrevisions(7) (claimed subset): full or abbreviated
/// hex, `HEAD`/`@`, `FETCH_HEAD`, ref DWIM (`refs/`, heads, remotes, tags),
/// `<rev>@{u}` / `@{upstream}`, and any chain of `~N`, `^N`, `^` suffixes.
pub(crate) fn resolve_rev(repo: &Repository, rev: &str) -> CliResult<Oid> {
    let cut = rev
        .char_indices()
        .find(|(i, c)| (*c == '~' || *c == '^') && *i > 0)
        .map(|(i, _)| i)
        .unwrap_or(rev.len());
    let (base, mut ops) = rev.split_at(cut);
    let mut oid = resolve_rev_base(repo, base)?;
    while !ops.is_empty() {
        let op = ops.as_bytes()[0];
        let digits: String = ops[1..]
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        ops = &ops[1 + digits.len()..];
        let n: usize = if digits.is_empty() {
            1
        } else {
            digits
                .parse()
                .map_err(|_| CliError::Msg(format!("unknown revision: {rev}")))?
        };
        let parents = |o: &Oid| -> CliResult<Vec<Oid>> {
            let obj = repo.get_object(o)?;
            if obj.kind != arks_object::ObjectKind::Commit {
                return Err(CliError::Msg(format!("{rev}: not a commit")));
            }
            Ok(arks_object::Commit::decode(&obj.payload)?.parents)
        };
        if op == b'~' {
            for _ in 0..n {
                oid = *parents(&oid)?
                    .first()
                    .ok_or_else(|| CliError::Msg(format!("{rev}: no first parent")))?;
            }
        } else if n > 0 {
            oid = *parents(&oid)?
                .get(n - 1)
                .ok_or_else(|| CliError::Msg(format!("{rev}: no parent {n}")))?;
        }
    }
    Ok(oid)
}

fn resolve_rev_base(repo: &Repository, rev: &str) -> CliResult<Oid> {
    if let Ok(oid) = Oid::from_hex(rev) {
        return Ok(oid);
    }
    if rev == "HEAD" || rev == "@" {
        return Ok(repo.head_oid()?);
    }
    for suffix in ["@{upstream}", "@{u}", "@{UPSTREAM}", "@{U}"] {
        if let Some(branch) = rev.strip_suffix(suffix) {
            return resolve_upstream(repo, branch);
        }
    }
    if rev == "FETCH_HEAD" {
        let text = fs::read_to_string(repo.git_dir().join("FETCH_HEAD"))
            .map_err(|_| CliError::Msg("unknown revision: FETCH_HEAD".into()))?;
        let hex = text.split_whitespace().next().unwrap_or("");
        return Oid::from_hex(hex).map_err(|_| CliError::Msg("bad FETCH_HEAD".into()));
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
    if let Some(oid) = resolve_abbrev(repo, rev)? {
        return Ok(oid);
    }
    Err(CliError::Msg(format!("unknown revision: {rev}")))
}

/// `<branch>@{upstream}` → `refs/remotes/<remote>/<merge-branch>` from
/// `branch.<name>.remote` / `branch.<name>.merge`.
fn resolve_upstream(repo: &Repository, branch: &str) -> CliResult<Oid> {
    let name = if branch.is_empty() || branch == "HEAD" {
        match repo.head_target()? {
            RefTarget::Symbolic(s) => s.trim_start_matches("refs/heads/").to_string(),
            RefTarget::Direct(_) => {
                return Err(CliError::Msg("HEAD does not point to a branch".into()));
            }
        }
    } else {
        branch.to_string()
    };
    let cfg = arks_config::ConfigSet::load(Some(repo.common_dir()));
    let remote = cfg.get(&format!("branch.{name}.remote"));
    let merge = cfg.get(&format!("branch.{name}.merge"));
    let (Some(remote), Some(merge)) = (remote, merge) else {
        return Err(CliError::Msg(format!(
            "no upstream configured for branch '{name}'"
        )));
    };
    let short = merge.trim_start_matches("refs/heads/");
    let full = if remote == "." {
        format!("refs/heads/{short}")
    } else {
        format!("refs/remotes/{remote}/{short}")
    };
    Ok(repo.refs().resolve(&full)?)
}

/// Unique object whose hex starts with `prefix` (4+ hex digits), loose or packed.
fn resolve_abbrev(repo: &Repository, prefix: &str) -> CliResult<Option<Oid>> {
    let p = prefix.to_ascii_lowercase();
    if p.len() < 4 || p.len() > 64 || !p.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Ok(None);
    }
    let objects = repo.common_dir().join("objects");
    let mut hits: Vec<Oid> = Vec::new();
    if let Ok(rd) = fs::read_dir(objects.join(&p[..2])) {
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if name.starts_with(&p[2..]) {
                if let Ok(o) = Oid::from_hex(&format!("{}{name}", &p[..2])) {
                    hits.push(o);
                }
            }
        }
    }
    if let Ok(rd) = fs::read_dir(objects.join("pack")) {
        for e in rd.flatten() {
            let path = e.path();
            if path.extension().and_then(|x| x.to_str()) != Some("idx") {
                continue;
            }
            let Ok(bytes) = fs::read(&path) else { continue };
            let Ok(idx) = arks_pack::PackIndex::decode(&bytes, repo.object_format()) else {
                continue;
            };
            for ent in &idx.entries {
                if ent.oid.to_hex().starts_with(&p) && !hits.contains(&ent.oid) {
                    hits.push(ent.oid);
                }
            }
        }
    }
    match hits.len() {
        0 => Ok(None),
        1 => Ok(Some(hits[0])),
        _ => Err(CliError::Msg(format!(
            "short object ID {prefix} is ambiguous"
        ))),
    }
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

/// Author identity + date from env and merged config
/// ([[sot:root/cloud-agent-dropin]] §C). Missing identity is an error.
pub(crate) fn commit_signature() -> CliResult<Signature> {
    ident_signature(arks_config::IdentRole::Author)
}

/// Committer identity + date (`GIT_COMMITTER_*` / `committer.*` / `user.*`).
pub(crate) fn committer_signature() -> CliResult<Signature> {
    ident_signature(arks_config::IdentRole::Committer)
}

fn ident_signature(role: arks_config::IdentRole) -> CliResult<Signature> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let cfg = arks_config::ConfigSet::load_discovered(&cwd);
    let ident = arks_config::resolve_ident(&cfg, role).map_err(|e| CliError::Msg(e.to_string()))?;
    let date_var = match role {
        arks_config::IdentRole::Author => "GIT_AUTHOR_DATE",
        arks_config::IdentRole::Committer => "GIT_COMMITTER_DATE",
    };
    let timestamp = std::env::var(date_var)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0)
        });
    Ok(Signature {
        name: ident.name,
        email: ident.email,
        timestamp,
        timezone: "+0000".into(),
    })
}
