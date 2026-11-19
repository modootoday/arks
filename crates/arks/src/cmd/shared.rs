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
/// `<rev>@{u}` / `@{upstream}`, any chain of `~N`, `^N`, `^`, `^{<type>}`,
/// `^{}` suffixes, `<rev>:<path>` (tree entry) and `:<path>` / `:<n>:<path>`
/// (index entry). `~`/`^N` peel annotated tags to their commit first.
/// Cite: <https://git-scm.com/docs/gitrevisions#_specifying_revisions>.
pub(crate) fn resolve_rev(repo: &Repository, rev: &str) -> CliResult<Oid> {
    if let Some(rest) = rev.strip_prefix(':') {
        return resolve_index_path(repo, rev, rest);
    }
    if let Some(colon) = rev_path_colon(rev) {
        let (r, path) = (&rev[..colon], &rev[colon + 1..]);
        let tree = peel_to_kind(repo, &resolve_rev(repo, r)?, ObjKind::Tree, rev)?;
        return tree_entry_at(repo, &tree, path)
            .ok_or_else(|| CliError::fatal(format!("path '{path}' does not exist in '{r}'")));
    }
    let cut = rev
        .char_indices()
        .find(|(i, c)| (*c == '~' || *c == '^') && *i > 0)
        .map(|(i, _)| i)
        .unwrap_or(rev.len());
    let (base, mut ops) = rev.split_at(cut);
    let mut oid = resolve_rev_base(repo, base)?;
    while !ops.is_empty() {
        let op = ops.as_bytes()[0];
        if op == b'^' && ops[1..].starts_with('{') {
            let close = ops
                .find('}')
                .ok_or_else(|| CliError::Msg(format!("unknown revision: {rev}")))?;
            let want = &ops[2..close];
            ops = &ops[close + 1..];
            oid = match want {
                "" => peel_tags(repo, &oid)?,
                "commit" => peel_to_kind(repo, &oid, ObjKind::Commit, rev)?,
                "tree" => peel_to_kind(repo, &oid, ObjKind::Tree, rev)?,
                "object" => oid,
                "blob" | "tag" => {
                    let k = repo.get_object(&oid)?.kind;
                    if k.as_str() != want {
                        return Err(CliError::Msg(format!("{rev}: not a {want}")));
                    }
                    oid
                }
                _ => return Err(CliError::Msg(format!("unknown revision: {rev}"))),
            };
            continue;
        }
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
        oid = peel_to_kind(repo, &oid, ObjKind::Commit, rev)?;
        let parents = |o: &Oid| -> CliResult<Vec<Oid>> {
            let obj = repo.get_object(o)?;
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

/// Peel target for `^{<type>}` and tree-ish contexts.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ObjKind {
    Commit,
    Tree,
}

/// Byte offset of the `<rev>:<path>` separator (outside any `@{...}`).
fn rev_path_colon(rev: &str) -> Option<usize> {
    let mut depth = 0usize;
    for (i, c) in rev.char_indices() {
        match c {
            '{' => depth += 1,
            '}' => depth = depth.saturating_sub(1),
            ':' if depth == 0 => return Some(i),
            _ => {}
        }
    }
    None
}

/// Target of an annotated tag object (`object <hex>` header line).
pub(crate) fn tag_target(payload: &[u8]) -> Option<Oid> {
    let text = String::from_utf8_lossy(payload);
    text.lines()
        .take_while(|l| !l.is_empty())
        .find_map(|l| l.strip_prefix("object "))
        .and_then(|h| Oid::from_hex(h.trim()).ok())
}

/// Recursively dereference annotated tags (`<rev>^{}`).
pub(crate) fn peel_tags(repo: &Repository, oid: &Oid) -> CliResult<Oid> {
    let mut cur = *oid;
    loop {
        let obj = repo.get_object(&cur)?;
        if obj.kind != arks_object::ObjectKind::Tag {
            return Ok(cur);
        }
        cur = tag_target(&obj.payload)
            .ok_or_else(|| CliError::Msg(format!("bad tag object {}", cur.to_hex())))?;
    }
}

/// Peel tags (and commit -> tree) until an object of `want` kind is reached.
pub(crate) fn peel_to_kind(
    repo: &Repository,
    oid: &Oid,
    want: ObjKind,
    rev: &str,
) -> CliResult<Oid> {
    let cur = peel_tags(repo, oid)?;
    let obj = repo.get_object(&cur)?;
    match (obj.kind, want) {
        (arks_object::ObjectKind::Commit, ObjKind::Commit)
        | (arks_object::ObjectKind::Tree, ObjKind::Tree) => Ok(cur),
        (arks_object::ObjectKind::Commit, ObjKind::Tree) => {
            Ok(arks_object::Commit::decode(&obj.payload)?.tree)
        }
        _ => Err(CliError::Msg(format!(
            "{rev}: expected {}",
            if want == ObjKind::Commit {
                "commit"
            } else {
                "tree"
            }
        ))),
    }
}

/// Object at `path` below `tree` (empty path = the tree itself).
pub(crate) fn tree_entry_at(repo: &Repository, tree: &Oid, path: &str) -> Option<Oid> {
    let mut cur = *tree;
    for comp in path.split('/').filter(|c| !c.is_empty() && *c != ".") {
        let obj = repo.get_object(&cur).ok()?;
        if obj.kind != arks_object::ObjectKind::Tree {
            return None;
        }
        let t = arks_object::Tree::decode(&obj.payload, repo.oid_kind()).ok()?;
        cur = t.entries.iter().find(|e| e.name == comp)?.oid;
    }
    Some(cur)
}

/// `:<path>` / `:<n>:<path>` -- blob recorded in the index at stage `n` (default 0).
fn resolve_index_path(repo: &Repository, rev: &str, rest: &str) -> CliResult<Oid> {
    let (stage, path) = match rest.split_once(':') {
        Some((n, p)) if n.len() == 1 && n.as_bytes()[0].is_ascii_digit() => {
            (u16::from(n.as_bytes()[0] - b'0'), p)
        }
        _ => (0, rest),
    };
    let index = crate::util::read_index(repo)?;
    index
        .entries
        .iter()
        .find(|e| e.path == path && e.stage() == stage)
        .map(|e| e.oid)
        .ok_or_else(|| {
            CliError::fatal(format!(
                "path '{path}' does not exist (neither on disk nor in the index) ({rev})"
            ))
        })
}

fn resolve_rev_base(repo: &Repository, rev: &str) -> CliResult<Oid> {
    if let Ok(oid) = Oid::from_hex(rev) {
        return Ok(oid);
    }
    if rev == "HEAD" || rev == "@" {
        return Ok(repo.head_oid()?);
    }
    if let Some(o) = resolve_reflog_spec(repo, rev)? {
        return Ok(o);
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

/// `<ref>@{<n>}` (n-th prior value from the ref's reflog; `@{n}` = current
/// branch) and `@{-<n>}` (n-th previously checked-out branch from the HEAD
/// reflog `checkout: moving from X to Y` entries).
/// Cite: <https://git-scm.com/docs/gitrevisions> (`@{<n>}`, `@{-<n>}`).
fn resolve_reflog_spec(repo: &Repository, rev: &str) -> CliResult<Option<Oid>> {
    let Some(open) = rev.find("@{") else {
        return Ok(None);
    };
    let Some(inner) = rev[open + 2..].strip_suffix('}') else {
        return Ok(None);
    };
    let base = &rev[..open];
    if let Some(n) = inner.strip_prefix('-') {
        let Ok(n) = n.parse::<usize>() else {
            return Ok(None);
        };
        if !base.is_empty() || n == 0 {
            return Ok(None);
        }
        let mut seen = 0;
        for (_, _, msg) in crate::reflog::read_entries(repo, "HEAD").iter().rev() {
            if let Some(rest) = msg.strip_prefix("checkout: moving from ") {
                if let Some((from, _)) = rest.split_once(" to ") {
                    seen += 1;
                    if seen == n {
                        return resolve_rev(repo, from).map(Some);
                    }
                }
            }
        }
        return Err(CliError::fatal(format!(
            "{rev}: not enough branch switches"
        )));
    }
    let Ok(n) = inner.parse::<usize>() else {
        return Ok(None);
    };
    let refname = match base {
        "" => match repo.head_target()? {
            RefTarget::Symbolic(s) => s,
            RefTarget::Direct(_) => "HEAD".to_string(),
        },
        "HEAD" => "HEAD".to_string(),
        b if b.starts_with("refs/") => b.to_string(),
        b => format!("refs/heads/{b}"),
    };
    let entries = crate::reflog::read_entries(repo, &refname);
    if n < entries.len() {
        return Ok(Some(entries[entries.len() - 1 - n].1));
    }
    if n == entries.len() {
        if let Some((Some(old), _, _)) = entries.first() {
            return Ok(Some(*old));
        }
    }
    Err(CliError::fatal(format!(
        "log for '{}' only has {} entries",
        base.trim_start_matches("refs/heads/"),
        entries.len()
    )))
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

/// Shortest unambiguous hex prefix of `oid`, at least `min` digits
/// (git `find_unique_abbrev`; default `core.abbrev` minimum is 7).
pub(crate) fn abbrev_hex(repo: &Repository, oid: &Oid, min: usize) -> String {
    let hex = oid.to_hex();
    let mut n = min.clamp(4, hex.len());
    while n < hex.len() {
        if matches!(resolve_abbrev(repo, &hex[..n]), Ok(Some(_))) {
            break;
        }
        n += 1;
    }
    hex[..n].to_string()
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

/// Untracked files below `dir` (work-tree relative, sorted), skipping paths
/// excluded by gitignore(5) rules; excluded directories are not descended.
pub(crate) fn collect_untracked(
    work: &Path,
    dir: &Path,
    indexed: &std::collections::HashSet<String>,
    out: &mut Vec<String>,
) -> CliResult<()> {
    let mut ignore = crate::cmd::ignore::Ignore::new(work);
    collect_untracked_with(work, dir, indexed, out, Some(&mut ignore))
}

/// [`collect_untracked`] with explicit exclude handling (`None` lists
/// ignored files too, e.g. `ls-files --others` without `--exclude-standard`).
pub(crate) fn collect_untracked_with(
    work: &Path,
    dir: &Path,
    indexed: &std::collections::HashSet<String>,
    out: &mut Vec<String>,
    mut ignore: Option<&mut crate::cmd::ignore::Ignore>,
) -> CliResult<()> {
    // Pf4-1: recurse without per-directory sort; sort once at the top call.
    let before = out.len();
    collect_untracked_rec(work, dir, indexed, out, &mut ignore)?;
    out[before..].sort();
    Ok(())
}

fn collect_untracked_rec(
    work: &Path,
    dir: &Path,
    indexed: &std::collections::HashSet<String>,
    out: &mut Vec<String>,
    ignore: &mut Option<&mut crate::cmd::ignore::Ignore>,
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
        let is_dir = path.is_dir();
        if !indexed.contains(&rel) {
            if let Some(ig) = ignore.as_deref_mut() {
                if ig.is_ignored(&rel, is_dir) {
                    continue;
                }
            }
        }
        if is_dir {
            collect_untracked_rec(work, &path, indexed, out, ignore)?;
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
