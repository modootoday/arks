//! Shared discovery / index helpers for Wave L commands.

use crate::error::{CliError, CliResult};
use arks_index::Index;
use arks_repo::Repository;
use arks_store::ObjectFormat;
use std::env;
use std::path::{Path, PathBuf};

/// Default object format for non-repo contexts (e.g. standalone index-pack).
pub const FORMAT: ObjectFormat = ObjectFormat::Sha1;

/// Process-wide `-q` / `--quiet` for porcelain progress lines on stderr.
static QUIET: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Record `-q`/`--quiet` from `args` (call once per command).
pub fn set_quiet_from(args: &[String]) {
    if args.iter().any(|a| a == "-q" || a == "--quiet") {
        QUIET.store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

pub fn quiet() -> bool {
    QUIET.load(std::sync::atomic::Ordering::Relaxed)
}

pub fn cwd() -> CliResult<PathBuf> {
    env::current_dir().map_err(|e| CliError::Msg(e.to_string()))
}

pub fn open_repo() -> CliResult<Repository> {
    let repo = Repository::discover(cwd()?)?;
    // CFG-004 (M20 Cx-3): format v1 + unknown extensions.* MUST NOT proceed.
    // Cite: https://git-scm.com/docs/gitrepository-layout (Version 1)
    refuse_unknown_repo_extensions(&repo)?;
    Ok(repo)
}

/// Extensions arks understands enough to open a format-v1 repository.
/// Unknown keys under `[extensions]` with `core.repositoryformatversion=1` refuse.
const KNOWN_EXTENSIONS: &[&str] = &[
    "worktreeconfig",
    "partialclone",
    "preciousobjects",
    "refstorage",
    "objectformat",
    "relativeworktrees",
];

fn refuse_unknown_repo_extensions(repo: &Repository) -> CliResult<()> {
    let config_path = repo.common_dir().join("config");
    if !config_path.is_file() {
        return Ok(());
    }
    let text = std::fs::read_to_string(&config_path).unwrap_or_default();
    let mut format_v1 = false;
    let mut in_extensions = false;
    let mut in_core = false;
    let mut unknown = Vec::new();
    let mut object_format: Option<String> = None;
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            let sect = line[1..line.len() - 1].trim().to_ascii_lowercase();
            in_core = sect == "core";
            in_extensions = sect == "extensions";
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            let key = k.trim().to_ascii_lowercase();
            let val = v.trim().trim_matches('"');
            if in_core && key == "repositoryformatversion" {
                format_v1 = val == "1";
            }
            if in_extensions && key == "objectformat" {
                object_format = Some(val.to_ascii_lowercase());
            }
            if in_extensions && !KNOWN_EXTENSIONS.contains(&key.as_str()) {
                unknown.push(key);
            }
        }
    }
    if format_v1 && !unknown.is_empty() {
        return Err(CliError::Msg(format!(
            "unknown repository extension(s): {} (CFG-004 — format v1 unknown extensions.* hard refuse; MUST NOT proceed)",
            unknown.join(", ")
        )));
    }
    let _ = object_format; // Repository.object_format() is authoritative
    Ok(())
}

pub fn work_tree(repo: &Repository) -> CliResult<PathBuf> {
    if let Some(wt) = repo.work_tree() {
        return Ok(wt.to_path_buf());
    }
    match repo.kind() {
        arks_repo::GitDirKind::DotGitDir => {
            let git_dir = repo.git_dir();
            Ok(git_dir
                .parent()
                .ok_or_else(|| CliError::Msg("`.git` has no parent work tree".into()))?
                .to_path_buf())
        }
        arks_repo::GitDirKind::Bare => Err(CliError::Msg(
            "work tree commands require a non-bare repository".into(),
        )),
        arks_repo::        GitDirKind::DotGitFile => Err(CliError::Msg(
            "worktree path could not be resolved for .git file checkout (linked worktree or submodule)"
                .into(),
        )),
    }
}

pub fn read_index(repo: &Repository) -> CliResult<Index> {
    let path = repo.index_path();
    if !path.is_file() {
        return Ok(Index {
            version: 2,
            entries: Vec::new(),
            extensions: Vec::new(),
        });
    }
    Ok(Index::read_path(&path, repo.oid_kind())?)
}

pub fn write_index(repo: &Repository, index: &Index) -> CliResult<()> {
    Ok(index.write_path(&repo.index_path(), repo.oid_kind())?)
}

pub fn rel_path_from_worktree(work: &Path, path: &Path) -> CliResult<String> {
    // Fast path when cwd is the work tree and argv is a relative path without
    // `..` — skip canonicalize (M17 / G19 short-porcelain hot path).
    let cwd = cwd()?;
    if path.is_relative()
        && path
            .components()
            .all(|c| !matches!(c, std::path::Component::ParentDir))
        && (cwd == work || cwd.as_os_str() == work.as_os_str())
    {
        let s = path
            .to_string_lossy()
            .replace('\\', "/")
            .trim_start_matches("./")
            .to_string();
        if s.is_empty() || s == "." {
            return Err(CliError::Msg("refusing to add work tree root".into()));
        }
        return Ok(s);
    }
    let abs = if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    };
    let abs = abs
        .canonicalize()
        .map_err(|e| CliError::Msg(format!("{}: {e}", path.display())))?;
    let work = work
        .canonicalize()
        .map_err(|e| CliError::Msg(format!("work tree: {e}")))?;
    let rel = abs
        .strip_prefix(&work)
        .map_err(|_| CliError::Msg(format!("path outside work tree: {}", path.display())))?;
    let s = rel.to_string_lossy().replace('\\', "/");
    if s.is_empty() {
        return Err(CliError::Msg("refusing to add work tree root".into()));
    }
    Ok(s)
}

pub fn refuse_unknown_flags(args: &[String], allowed: &[&str]) -> CliResult<()> {
    for a in args {
        if !a.starts_with('-') {
            continue;
        }
        // Exact match, or `flag=value` form when `flag` is allowed (e.g. `--chmod=+x`).
        let ok = allowed
            .iter()
            .any(|x| *a == *x || a.starts_with(&format!("{x}=")));
        if !ok {
            return Err(CliError::NotImplemented(format!("unsupported option: {a}")));
        }
    }
    Ok(())
}

/// Split bundled single-dash short options per gitcli (`-sb` → `-s -b`,
/// `-am msg` → `-a -m msg`, `-mmsg` → `-m msg`). `flags` are boolean letters;
/// `valued` take a value (rest of the token, else the next argument). Tokens
/// with any unknown letter, numeric tokens (`-5`), and everything after `--`
/// are left unchanged. Cite: <https://git-scm.com/docs/gitcli>.
pub fn split_short_bundles(args: &[String], flags: &str, valued: &str) -> Vec<String> {
    let mut out = Vec::with_capacity(args.len());
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        i += 1;
        if a == "--" {
            out.extend(args[i - 1..].iter().cloned());
            break;
        }
        let is_bundle = a.len() > 2
            && a.starts_with('-')
            && !a.starts_with("--")
            && !a[1..].chars().all(|c| c.is_ascii_digit());
        if !is_bundle {
            // A lone valued short flag consumes the next argument verbatim.
            let single_valued = a.len() == 2 && a.starts_with('-') && valued.contains(&a[1..]);
            out.push(a.clone());
            if single_valued && i < args.len() {
                out.push(args[i].clone());
                i += 1;
            }
            continue;
        }
        let body: Vec<char> = a[1..].chars().collect();
        let mut expanded = Vec::new();
        let mut ok = true;
        let mut j = 0;
        let mut takes_next = false;
        while j < body.len() {
            let c = body[j];
            if valued.contains(c) {
                expanded.push(format!("-{c}"));
                let rest: String = body[j + 1..].iter().collect();
                if rest.is_empty() {
                    takes_next = true;
                } else {
                    expanded.push(rest);
                }
                break;
            } else if flags.contains(c) {
                expanded.push(format!("-{c}"));
            } else {
                ok = false;
                break;
            }
            j += 1;
        }
        if !ok {
            out.push(a.clone());
            continue;
        }
        out.extend(expanded);
        if takes_next && i < args.len() {
            out.push(args[i].clone());
            i += 1;
        }
    }
    out
}

#[cfg(test)]
mod bundle_tests {
    use super::*;

    fn v(xs: &[&str]) -> Vec<String> {
        xs.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn splits_flags_and_values() {
        assert_eq!(
            split_short_bundles(&v(&["-sb"]), "sb", ""),
            v(&["-s", "-b"])
        );
        assert_eq!(
            split_short_bundles(&v(&["-am", "-sb msg"]), "a", "m"),
            v(&["-a", "-m", "-sb msg"])
        );
        assert_eq!(
            split_short_bundles(&v(&["-mhello"]), "a", "m"),
            v(&["-m", "hello"])
        );
        assert_eq!(
            split_short_bundles(&v(&["-m", "-xy"]), "xy", "m"),
            v(&["-m", "-xy"])
        );
        assert_eq!(
            split_short_bundles(&v(&["-5", "-zq"]), "s", ""),
            v(&["-5", "-zq"])
        );
        assert_eq!(
            split_short_bundles(&v(&["--", "-sb"]), "sb", ""),
            v(&["--", "-sb"])
        );
    }
}
