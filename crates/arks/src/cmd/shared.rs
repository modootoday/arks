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

/// `ancestor` is reachable from `tip` through any parents (git-merge-base --is-ancestor).
pub(crate) fn is_ancestor(repo: &Repository, ancestor: &Oid, tip: &Oid) -> CliResult<bool> {
    Ok(ancestor == tip || arks_repo::ancestor_set(repo, *tip)?.contains(ancestor))
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
    let (timestamp, timezone) = match std::env::var(date_var) {
        Ok(v) if !v.trim().is_empty() => {
            parse_git_date(&v).ok_or_else(|| CliError::Msg(format!("invalid date format: {v}")))?
        }
        _ => (
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0),
            "+0000".to_string(),
        ),
    };
    Ok(Signature {
        name: ident.name,
        email: ident.email,
        timestamp,
        timezone,
    })
}

/// git date forms (git-commit §DATE FORMATS, claimed subset): Git internal
/// `<unix> <+hhmm>`, `@<unix> [<+hhmm>]`, bare `<unix>`, and ISO 8601
/// `YYYY-MM-DD[T ]HH:MM[:SS][Z|<+hh:mm>|<+hhmm>]`.
pub(crate) fn parse_git_date(v: &str) -> Option<(i64, String)> {
    let v = v.trim();
    let norm_tz = |tz: &str| -> Option<String> {
        let tz = tz.replace(':', "");
        if tz == "Z" {
            return Some("+0000".into());
        }
        let ok = tz.len() == 5
            && (tz.starts_with('+') || tz.starts_with('-'))
            && tz[1..].bytes().all(|b| b.is_ascii_digit());
        ok.then_some(tz)
    };
    let raw = v.strip_prefix('@').unwrap_or(v);
    let mut parts = raw.split_whitespace();
    if let Some(first) = parts.next() {
        if let Ok(secs) = first.parse::<i64>() {
            let tz = match parts.next() {
                Some(t) => norm_tz(t)?,
                None => "+0000".into(),
            };
            return Some((secs, tz));
        }
    }
    // ISO 8601.
    let (date, rest) = v.split_at(v.find(['T', ' '])?);
    let rest = &rest[1..];
    let mut d = date.split('-').map(|x| x.parse::<i64>().ok());
    let (y, mo, dd) = (d.next()??, d.next()??, d.next()??);
    let tz_at = rest.find(['Z', '+', '-', ' ']).unwrap_or(rest.len());
    let (time, tz) = rest.split_at(tz_at);
    let tz = tz.trim();
    let mut t = time.split(':').map(|x| x.parse::<i64>().ok());
    let (h, mi) = (t.next()??, t.next()??);
    let sec = t.next().flatten().unwrap_or(0);
    let tz = if tz.is_empty() {
        "+0000".to_string()
    } else {
        norm_tz(tz)?
    };
    let off = {
        let sign = if tz.starts_with('-') { -1 } else { 1 };
        sign * (tz[1..3].parse::<i64>().ok()? * 3600 + tz[3..5].parse::<i64>().ok()? * 60)
    };
    // days_from_civil (Howard Hinnant).
    let yy = if mo <= 2 { y - 1 } else { y };
    let era = yy.div_euclid(400);
    let yoe = yy - era * 400;
    let mp = (mo + 9) % 12;
    let doy = (153 * mp + 2) / 5 + dd - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    Some((days * 86_400 + h * 3600 + mi * 60 + sec - off, tz))
}

#[cfg(test)]
mod date_tests {
    use super::parse_git_date;

    #[test]
    fn git_date_forms() {
        assert_eq!(
            parse_git_date("1700000000 +0000"),
            Some((1_700_000_000, "+0000".into()))
        );
        assert_eq!(
            parse_git_date("@1112911993 -0700"),
            Some((1_112_911_993, "-0700".into()))
        );
        assert_eq!(
            parse_git_date("1112911993"),
            Some((1_112_911_993, "+0000".into()))
        );
        assert_eq!(
            parse_git_date("2005-04-07T15:13:13-07:00"),
            Some((1_112_911_993, "-0700".into()))
        );
        assert_eq!(
            parse_git_date("2005-04-07 22:13:13Z"),
            Some((1_112_911_993, "+0000".into()))
        );
        assert_eq!(parse_git_date("garbage"), None);
    }
}
