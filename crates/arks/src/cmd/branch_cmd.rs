//! `branch`: list (patterns, `-v`/`-vv`, `-a`/`-r`, `--merged`,
//! `--no-merged`, `--contains`, `--no-contains`), create, delete, rename,
//! `--show-current`.
//!
//! Primary: <https://git-scm.com/docs/git-branch>.
//!
//! Partial: `--sort`, `--format`, `--column`, `-c` (copy) and setting
//! upstreams (`-u`, `--track`) are not claimed.

use crate::cmd::branch::branch_ref;
use crate::cmd::shared::{abbrev_hex, is_ancestor, resolve_rev};
use crate::error::{CliError, CliResult};
use crate::util::open_repo;
use arks_oid::Oid;
use arks_refs::RefTarget;
use arks_repo::Repository;
use std::collections::HashSet;

/// Glob match where `*` also matches `/` (git `wildmatch` without
/// `WM_PATHNAME`, as used by `branch --list` / `tag -l`).
pub(crate) fn fnmatch(p: &[u8], t: &[u8]) -> bool {
    match p.first() {
        None => t.is_empty(),
        Some(b'*') => (0..=t.len()).any(|k| fnmatch(&p[1..], &t[k..])),
        Some(b'?') => !t.is_empty() && fnmatch(&p[1..], &t[1..]),
        Some(b'[') => {
            let Some(&c) = t.first() else { return false };
            let mut j = 1;
            let neg = matches!(p.get(j), Some(b'!') | Some(b'^'));
            if neg {
                j += 1;
            }
            let mut hit = false;
            let mut first = true;
            while j < p.len() && (first || p[j] != b']') {
                first = false;
                if j + 2 < p.len() && p[j + 1] == b'-' && p[j + 2] != b']' {
                    hit |= (p[j]..=p[j + 2]).contains(&c);
                    j += 3;
                } else {
                    hit |= p[j] == c;
                    j += 1;
                }
            }
            j < p.len() && hit != neg && fnmatch(&p[j + 1..], &t[1..])
        }
        Some(b'\\') if p.len() > 1 => t.first() == Some(&p[1]) && fnmatch(&p[2..], &t[1..]),
        Some(c) => t.first() == Some(c) && fnmatch(&p[1..], &t[1..]),
    }
}

/// Commits reachable from `tip` but not from `base` (`base..tip` count).
fn count_only_in(repo: &Repository, tip: &Oid, base: &Oid) -> CliResult<usize> {
    let a = arks_repo::ancestor_set(repo, *tip)?;
    let b = arks_repo::ancestor_set(repo, *base)?;
    Ok(a.difference(&b).count())
}

/// `[<upstream>: ahead N, behind M]` for `-vv` (None without an upstream).
fn upstream_info(repo: &Repository, branch: &str, tip: &Oid) -> CliResult<Option<String>> {
    let cfg = arks_config::ConfigSet::load(Some(repo.common_dir()));
    let (Some(remote), Some(merge)) = (
        cfg.get(&format!("branch.{branch}.remote")),
        cfg.get(&format!("branch.{branch}.merge")),
    ) else {
        return Ok(None);
    };
    let short = merge.trim_start_matches("refs/heads/");
    let (full, shown) = if remote == "." {
        (format!("refs/heads/{short}"), short.to_string())
    } else {
        (
            format!("refs/remotes/{remote}/{short}"),
            format!("{remote}/{short}"),
        )
    };
    let Ok(up) = repo.refs().resolve(&full) else {
        return Ok(Some(format!("[{shown}: gone]")));
    };
    let ahead = count_only_in(repo, tip, &up)?;
    let behind = count_only_in(repo, &up, tip)?;
    Ok(Some(match (ahead, behind) {
        (0, 0) => format!("[{shown}]"),
        (a, 0) => format!("[{shown}: ahead {a}]"),
        (0, b) => format!("[{shown}: behind {b}]"),
        (a, b) => format!("[{shown}: ahead {a}, behind {b}]"),
    }))
}

struct ListOpts {
    all: bool,
    remotes: bool,
    verbose: u8,
    patterns: Vec<String>,
    merged: Option<Oid>,
    no_merged: Option<Oid>,
    contains: Option<Oid>,
    no_contains: Option<Oid>,
}

pub fn cmd_branch(args: &[String]) -> CliResult<()> {
    let args = crate::util::split_short_bundles(args, "dDmMarvlfq", "");
    let mut delete = 0u8;
    let mut rename = false;
    let mut force = false;
    let mut quiet = false;
    let mut list = false;
    let mut show_current = false;
    let mut lo = ListOpts {
        all: false,
        remotes: false,
        verbose: 0,
        patterns: Vec::new(),
        merged: None,
        no_merged: None,
        contains: None,
        no_contains: None,
    };
    let mut positional: Vec<String> = Vec::new();
    // Filters take an optional commit argument (default HEAD).
    let mut pending_filter: Option<&'static str> = None;
    let repo = open_repo()?;
    let commit_arg = |r: &str| -> CliResult<Oid> {
        let o = resolve_rev(&repo, r)
            .map_err(|_| CliError::fatal(format!("malformed object name {r}")))?;
        crate::cmd::shared::peel_to_kind(&repo, &o, crate::cmd::shared::ObjKind::Commit, r)
    };
    let set_filter = |lo: &mut ListOpts, which: &str, oid: Oid| match which {
        "merged" => lo.merged = Some(oid),
        "no-merged" => lo.no_merged = Some(oid),
        "contains" => lo.contains = Some(oid),
        _ => lo.no_contains = Some(oid),
    };
    for a in &args {
        if let Some(which) = pending_filter.take() {
            if !a.starts_with('-') {
                if let Ok(o) = commit_arg(a) {
                    set_filter(&mut lo, which, o);
                    continue;
                }
            }
            set_filter(&mut lo, which, commit_arg("HEAD")?);
        }
        match a.as_str() {
            "-d" | "--delete" => delete = delete.max(1),
            "-D" => delete = 2,
            "-m" | "--move" => rename = true,
            "-M" => {
                rename = true;
                force = true;
            }
            "-f" | "--force" => force = true,
            "-q" | "--quiet" => quiet = true,
            "-a" | "--all" => lo.all = true,
            "-r" | "--remotes" => lo.remotes = true,
            "-v" | "--verbose" => lo.verbose += 1,
            "-vv" => lo.verbose += 2,
            "-l" | "--list" => list = true,
            "--show-current" => show_current = true,
            "--no-color" | "--color=never" => {}
            "--merged" | "--no-merged" | "--contains" | "--no-contains" => {
                pending_filter = Some(match a.as_str() {
                    "--merged" => "merged",
                    "--no-merged" => "no-merged",
                    "--contains" => "contains",
                    _ => "no-contains",
                });
                list = true;
            }
            x if x.starts_with('-') && x.len() > 1 => {
                let mut handled = false;
                for (flag, which) in [
                    ("--merged=", "merged"),
                    ("--no-merged=", "no-merged"),
                    ("--contains=", "contains"),
                    ("--no-contains=", "no-contains"),
                ] {
                    if let Some(v) = x.strip_prefix(flag) {
                        set_filter(&mut lo, which, commit_arg(v)?);
                        list = true;
                        handled = true;
                    }
                }
                if !handled {
                    return Err(CliError::NotImplemented(format!("unsupported option: {x}")));
                }
            }
            _ => positional.push(a.clone()),
        }
    }
    if let Some(which) = pending_filter.take() {
        set_filter(&mut lo, which, commit_arg("HEAD")?);
    }
    if show_current {
        if let Ok(RefTarget::Symbolic(s)) = repo.head_target() {
            println!("{}", s.strip_prefix("refs/heads/").unwrap_or(&s));
        }
        return Ok(());
    }
    if delete > 0 {
        return delete_branches(&positional, delete == 2 || force, lo.remotes, quiet);
    }
    if rename {
        return rename_branch(&positional, force);
    }
    if list || positional.is_empty() {
        lo.patterns = positional;
        return list_branches(&repo, &lo);
    }
    create_branch(&positional, force)
}

fn list_branches(repo: &Repository, o: &ListOpts) -> CliResult<()> {
    let head = repo.head_target()?;
    // (display name, ref oid, is_current, upstream-capable short branch name, symref target)
    let mut rows: Vec<(String, Oid, bool, Option<String>, Option<String>)> = Vec::new();
    if let (RefTarget::Direct(h), false) = (&head, o.remotes) {
        rows.push((
            format!("(HEAD detached at {})", abbrev_hex(repo, h, 7)),
            *h,
            true,
            None,
            None,
        ));
    }
    let cur = match &head {
        RefTarget::Symbolic(s) => Some(s.clone()),
        RefTarget::Direct(_) => None,
    };
    if !o.remotes || o.all {
        let mut refs = repo.refs().list_refs("refs/heads/")?;
        refs.sort_by(|a, b| a.0.cmp(&b.0));
        for (name, oid) in refs {
            let short = name
                .strip_prefix("refs/heads/")
                .unwrap_or(&name)
                .to_string();
            let is_cur = cur.as_deref() == Some(name.as_str());
            rows.push((short.clone(), oid, is_cur, Some(short), None));
        }
    }
    if o.remotes || o.all {
        let mut refs = repo.refs().list_refs("refs/remotes/")?;
        refs.sort_by(|a, b| a.0.cmp(&b.0));
        for (name, oid) in refs {
            let short = name.strip_prefix("refs/remotes/").unwrap_or(&name);
            let shown = if o.all {
                format!("remotes/{short}")
            } else {
                short.to_string()
            };
            let sym = match repo.refs().read(&name) {
                Ok(RefTarget::Symbolic(t)) => {
                    Some(t.strip_prefix("refs/remotes/").unwrap_or(&t).to_string())
                }
                _ => None,
            };
            rows.push((shown, oid, false, None, sym));
        }
    }
    let mut kept = Vec::new();
    for row in rows {
        let (name, oid, _, _, _) = &row;
        if !o.patterns.is_empty() {
            let bare = name.strip_prefix("remotes/").unwrap_or(name);
            let hit = !name.starts_with('(')
                && o.patterns.iter().any(|p| {
                    fnmatch(p.as_bytes(), bare.as_bytes()) || fnmatch(p.as_bytes(), name.as_bytes())
                });
            if !hit {
                continue;
            }
        }
        if let Some(m) = &o.merged {
            if !is_ancestor(repo, oid, m)? {
                continue;
            }
        }
        if let Some(m) = &o.no_merged {
            if is_ancestor(repo, oid, m)? {
                continue;
            }
        }
        if let Some(c) = &o.contains {
            if !is_ancestor(repo, c, oid)? {
                continue;
            }
        }
        if let Some(c) = &o.no_contains {
            if is_ancestor(repo, c, oid)? {
                continue;
            }
        }
        kept.push(row);
    }
    let width = kept
        .iter()
        .filter(|r| r.4.is_none())
        .map(|r| r.0.chars().count())
        .max()
        .unwrap_or(0);
    let mut seen: HashSet<String> = HashSet::new();
    for (name, oid, is_cur, branch, sym) in &kept {
        if !seen.insert(name.clone()) {
            continue;
        }
        let mark = if *is_cur { '*' } else { ' ' };
        if let Some(t) = sym {
            println!("{mark} {name} -> {t}");
            continue;
        }
        if o.verbose == 0 {
            println!("{mark} {name}");
            continue;
        }
        let subject = crate::cmd::log_format::read_commit(repo, oid)
            .map(|c| crate::cmd::log_format::subject_line(&c.message))
            .unwrap_or_default();
        let up = match (o.verbose >= 2, branch) {
            (true, Some(b)) => upstream_info(repo, b, oid)?
                .map(|s| format!("{s} "))
                .unwrap_or_default(),
            _ => String::new(),
        };
        println!(
            "{mark} {name:<width$} {} {up}{subject}",
            abbrev_hex(repo, oid, 7)
        );
    }
    Ok(())
}

fn delete_branches(names: &[String], force: bool, remotes: bool, quiet: bool) -> CliResult<()> {
    if names.is_empty() {
        return Err(CliError::fatal("branch name required"));
    }
    let mut repo = open_repo()?;
    let cur = match repo.head_target()? {
        RefTarget::Symbolic(s) => Some(s),
        RefTarget::Direct(_) => None,
    };
    let head = repo.head_oid().ok();
    let mut errors = Vec::new();
    for name in names {
        let full = if remotes {
            format!("refs/remotes/{name}")
        } else {
            branch_ref(name)
        };
        let Ok(oid) = repo.refs().resolve(&full) else {
            errors.push(if remotes {
                format!("error: remote-tracking branch '{name}' not found")
            } else {
                format!("error: branch '{name}' not found")
            });
            continue;
        };
        if !remotes && cur.as_deref() == Some(full.as_str()) {
            let work = crate::util::work_tree(&repo)
                .map(|w| w.display().to_string())
                .unwrap_or_default();
            errors.push(format!(
                "error: cannot delete branch '{name}' used by worktree at '{work}'"
            ));
            continue;
        }
        if !force && !remotes {
            // Merged into its upstream if set, else into HEAD (git-branch -d).
            let target = upstream_oid(&repo, name).or(head);
            let merged = match target {
                Some(t) => is_ancestor(&repo, &oid, &t)?,
                None => false,
            };
            if !merged {
                errors.push(format!(
                    "error: the branch '{name}' is not fully merged.\n\
                     If you are sure you want to delete it, run 'git branch -D {name}'"
                ));
                continue;
            }
        }
        repo.refs_mut().delete(&full, Some(&oid))?;
        let _ = std::fs::remove_file(repo.common_dir().join("logs").join(&full));
        if !quiet {
            if remotes {
                println!(
                    "Deleted remote-tracking branch {name} (was {}).",
                    abbrev_hex(&repo, &oid, 7)
                );
            } else {
                println!(
                    "Deleted branch {name} (was {}).",
                    abbrev_hex(&repo, &oid, 7)
                );
            }
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(CliError::Exit(1, errors.join("\n")))
    }
}

fn upstream_oid(repo: &Repository, branch: &str) -> Option<Oid> {
    let cfg = arks_config::ConfigSet::load(Some(repo.common_dir()));
    let remote = cfg.get(&format!("branch.{branch}.remote"))?;
    let merge = cfg.get(&format!("branch.{branch}.merge"))?;
    let short = merge.trim_start_matches("refs/heads/");
    let full = if remote == "." {
        format!("refs/heads/{short}")
    } else {
        format!("refs/remotes/{remote}/{short}")
    };
    repo.refs().resolve(&full).ok()
}

fn rename_branch(pos: &[String], force: bool) -> CliResult<()> {
    let mut repo = open_repo()?;
    let cur = match repo.head_target()? {
        RefTarget::Symbolic(s) => Some(s),
        RefTarget::Direct(_) => None,
    };
    let (old, new) = match pos {
        [new] => (
            cur.clone().ok_or_else(|| {
                CliError::fatal("cannot rename the current branch while not on any")
            })?,
            branch_ref(new),
        ),
        [old, new] => (branch_ref(old), branch_ref(new)),
        _ => return Err(CliError::fatal("too many arguments for a rename operation")),
    };
    let new_short = new.trim_start_matches("refs/heads/");
    if !arks_refs::is_valid_refname(&new) {
        return Err(CliError::fatal(format!(
            "'{new_short}' is not a valid branch name"
        )));
    }
    let old_short = old.trim_start_matches("refs/heads/").to_string();
    let oid = match repo.refs().resolve(&old) {
        Ok(o) => o,
        Err(_) if cur.as_deref() == Some(old.as_str()) => {
            // Unborn current branch: only HEAD moves.
            repo.refs_mut().set_symbolic("HEAD", &new)?;
            return Ok(());
        }
        Err(_) => {
            return Err(CliError::fatal(format!("no branch named '{old_short}'")));
        }
    };
    if old != new && !force && repo.refs().resolve(&new).is_ok() {
        return Err(CliError::fatal(format!(
            "a branch named '{new_short}' already exists"
        )));
    }
    repo.refs_mut().update(&new, &oid, None)?;
    if old != new {
        repo.refs_mut().delete(&old, Some(&oid))?;
        let logs = repo.common_dir().join("logs");
        if logs.join(&old).is_file() {
            if let Some(parent) = logs.join(&new).parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::rename(logs.join(&old), logs.join(&new));
        }
    }
    if cur.as_deref() == Some(old.as_str()) {
        repo.refs_mut().set_symbolic("HEAD", &new)?;
    }
    Ok(())
}

fn create_branch(pos: &[String], force: bool) -> CliResult<()> {
    if pos.len() > 2 {
        return Err(CliError::fatal("too many arguments"));
    }
    let mut repo = open_repo()?;
    let name = &pos[0];
    let full = branch_ref(name);
    if !arks_refs::is_valid_refname(&full) || name.starts_with('-') {
        return Err(CliError::fatal(format!(
            "'{name}' is not a valid branch name"
        )));
    }
    if repo.refs().resolve(&full).is_ok() {
        if !force {
            return Err(CliError::fatal(format!(
                "a branch named '{name}' already exists"
            )));
        }
        if let Ok(RefTarget::Symbolic(s)) = repo.head_target() {
            if s == full {
                return Err(CliError::fatal(format!(
                    "cannot force update the current branch."
                )));
            }
        }
    }
    let start = pos.get(1).map(String::as_str).unwrap_or("HEAD");
    let oid = resolve_rev(&repo, start)
        .map_err(|_| CliError::fatal(format!("not a valid object name: '{start}'")))?;
    let oid =
        crate::cmd::shared::peel_to_kind(&repo, &oid, crate::cmd::shared::ObjKind::Commit, start)
            .map_err(|_| CliError::fatal(format!("not a valid branch point: '{start}'")))?;
    let old = repo.refs().resolve(&full).ok();
    repo.refs_mut().update(&full, &oid, old.as_ref())?;
    let _ = crate::reflog::append_reflog(
        &repo,
        &full,
        old.as_ref(),
        &oid,
        &format!("branch: Created from {start}"),
    );
    Ok(())
}
