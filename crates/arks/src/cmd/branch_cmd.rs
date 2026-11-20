//! `branch`: list (patterns, `-v`/`-vv`, `-a`/`-r`, `--merged`,
//! `--no-merged`, `--contains`, `--no-contains`), create, delete, rename,
//! `--show-current`.
//!
//! Primary: <https://git-scm.com/docs/git-branch>.
//!
//! `-c`/`-C` copy a branch (ref + reflog); `--sort=[-]<key>` orders the
//! list by `refname`, `committerdate`, `authordate` or `objectname`.
//!
//! `--format` lists through for-each-ref; `-u`, `--unset-upstream` and
//! `--track` write `branch.<name>.remote/merge`.
//!
//! Partial: `--column` keeps one branch per line (non-terminal output).

use crate::cmd::branch::branch_ref;
use crate::cmd::shared::{abbrev_hex, is_ancestor, resolve_rev};
use crate::error::{CliError, CliResult};
use crate::util::open_repo;
use arks_oid::Oid;
use arks_refs::RefTarget;
use arks_repo::Repository;
use std::collections::HashSet;

/// (display name, ref oid, is_current, upstream-capable short branch name, symref target)
type BranchRow = (String, Oid, bool, Option<String>, Option<String>);

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
    sort: Option<String>,
    points_at: Option<Oid>,
    ignore_case: bool,
    abbrev: usize,
    color: bool,
    column: bool,
}

/// `--merged`, `--no-merged`, `--contains` and `--no-contains` default to
/// HEAD when they are the last argument (parse-options LASTARG_DEFAULT).
pub(crate) fn lastarg_default(args: &[String], keys: &[&str]) -> Vec<String> {
    let mut out = args.to_vec();
    if let Some(last) = out.last() {
        let bare = last.trim_start_matches("--");
        if last.starts_with("--")
            && !last.contains('=')
            && keys
                .iter()
                .any(|k| *k == bare || (bare.len() >= 3 && k.starts_with(bare)))
        {
            out.push("HEAD".to_string());
        }
    }
    out
}

/// Ref listing through for-each-ref for `--format` (branch and tag).
pub(crate) fn list_with_format(
    p: &crate::cmd::opt_parse::Parsed,
    format: &str,
    prefixes: &[&str],
    patterns: &[String],
) -> CliResult<()> {
    let mut a = vec![format!("--format={format}")];
    for s in p.values("sort") {
        a.push(format!("--sort={s}"));
    }
    for k in [
        "merged",
        "no-merged",
        "contains",
        "no-contains",
        "points-at",
    ] {
        for v in p.values(k) {
            a.push(format!("--{k}"));
            a.push(v);
        }
    }
    if patterns.is_empty() {
        a.extend(prefixes.iter().map(|s| s.trim_end_matches('/').to_string()));
    } else {
        for pat in patterns {
            for pre in prefixes {
                a.push(format!("{pre}{pat}"));
            }
        }
    }
    crate::cmd::for_each_ref::cmd_for_each_ref(&a)
}

pub fn cmd_branch(args: &[String]) -> CliResult<()> {
    // Every option of `git branch -h`. Cite: https://git-scm.com/docs/git-branch
    use crate::cmd::opt_parse::{Item, Mode, parse};
    let args = lastarg_default(args, &["merged", "no-merged", "contains", "no-contains"]);
    let p = parse(
        "branch",
        &args,
        crate::cmd::porcelain_opts::BRANCH,
        Mode::default(),
    )?;
    let mut delete = u8::from(p.on("delete"));
    let mut rename = p.on("move");
    let mut copy = p.on("copy");
    let mut force = p.on("force");
    for it in &p.items {
        if let Item::Opt {
            key, neg: false, ..
        } = it
        {
            match *key {
                "-D" => delete = 2,
                "-M" => {
                    rename = true;
                    force = true;
                }
                "-C" => {
                    copy = true;
                    force = true;
                }
                _ => {}
            }
        }
    }
    if p.on("recurse-submodules") {
        let repo = open_repo()?;
        let cfg = arks_config::ConfigSet::load(Some(repo.common_dir()));
        if cfg.get_bool("submodule.propagateBranches") != Some(true) {
            return Err(CliError::fatal(
                "branch with --recurse-submodules can only be used if submodule.propagateBranches is enabled",
            ));
        }
    }
    let column = column_mode(&p)?;
    let quiet = p.on("quiet");
    let show_current = p.on("show-current");
    let mut list = p.on("list");
    let repo = open_repo()?;
    let commit_arg = |r: &str| -> CliResult<Oid> {
        let o = resolve_rev(&repo, r)
            .map_err(|_| CliError::fatal(format!("malformed object name {r}")))?;
        crate::cmd::shared::peel_to_kind(&repo, &o, crate::cmd::shared::ObjKind::Commit, r)
    };
    let one = |k: &str| -> CliResult<Option<Oid>> {
        match p.values(k).last() {
            Some(v) => commit_arg(v).map(Some),
            None => Ok(None),
        }
    };
    let mut lo = ListOpts {
        all: p.on("all"),
        remotes: p.on("remotes"),
        verbose: p.count("verbose").min(255) as u8,
        patterns: Vec::new(),
        merged: one("merged")?,
        no_merged: one("no-merged")?,
        contains: one("contains")?,
        no_contains: one("no-contains")?,
        sort: p.values("sort").last().cloned(),
        points_at: match p.value("points-at") {
            Some(v) => Some(
                resolve_rev(&repo, &v)
                    .map_err(|_| CliError::fatal(format!("malformed object name {v}")))?,
            ),
            None => None,
        },
        ignore_case: p.on("ignore-case"),
        abbrev: match p.last("abbrev") {
            None => 7,
            Some(None) => 7,
            Some(Some(n)) => n
                .parse::<usize>()
                .map_err(|_| CliError::fatal(format!("invalid --abbrev: {n}")))?
                .clamp(4, 40),
        },
        color: matches!(p.last("color"), Some(None))
            || p.value("color").as_deref() == Some("always"),
        column,
    };
    if p.flag("abbrev") == Some(false) {
        lo.abbrev = 40;
    }
    if lo.merged.is_some()
        || lo.no_merged.is_some()
        || lo.contains.is_some()
        || lo.no_contains.is_some()
        || lo.points_at.is_some()
        || p.value("format").is_some()
        || p.value("sort").is_some()
    {
        list = true;
    }
    let mut positional = p.args();
    positional.extend(p.after_dashdash());
    if show_current {
        if let Ok(RefTarget::Symbolic(s)) = repo.head_target() {
            println!("{}", s.strip_prefix("refs/heads/").unwrap_or(&s));
        }
        return Ok(());
    }
    if let Some(up) = p.value("set-upstream-to") {
        return set_upstream_cmd(&repo, &up, positional.first().map(String::as_str), quiet);
    }
    if p.on("unset-upstream") {
        return unset_upstream_cmd(&repo, positional.first().map(String::as_str));
    }
    if p.on("edit-description") {
        return edit_description(&repo, positional.first().map(String::as_str));
    }
    if delete > 0 {
        return delete_branches(&positional, delete == 2 || force, lo.remotes, quiet);
    }
    if rename || copy {
        return rename_branch(&positional, force, copy);
    }
    if list || positional.is_empty() {
        if let Some(f) = p.value("format") {
            if f.is_empty() && p.on("omit-empty") {
                return Ok(());
            }
            let mut prefixes = Vec::new();
            if !lo.remotes || lo.all {
                prefixes.push("refs/heads/");
            }
            if lo.remotes || lo.all {
                prefixes.push("refs/remotes/");
            }
            return list_with_format(&p, &f, &prefixes, &positional);
        }
        lo.patterns = positional;
        return list_branches(&repo, &lo);
    }
    drop(repo);
    create_branch(&positional, force)?;
    // `-t` / `--track[=direct|inherit]` (or a remote-tracking start point).
    let track = p.last("track");
    if p.flag("track") != Some(false) {
        let repo = open_repo()?;
        let start = positional.get(1).cloned();
        let explicit = track.is_some();
        let start_name = match &start {
            Some(s) => s.clone(),
            None => match repo.head_target()? {
                RefTarget::Symbolic(s) => s,
                RefTarget::Direct(_) => "HEAD".to_string(),
            },
        };
        let remote_start = repo
            .refs()
            .resolve(&format!("refs/remotes/{start_name}"))
            .is_ok()
            && repo.refs().resolve(&branch_ref(&start_name)).is_err();
        if explicit || remote_start {
            let mode = track.flatten();
            if let Some(line) = crate::cmd::branch::set_upstream(
                &repo,
                &positional[0],
                &start_name,
                mode.as_deref(),
            )? {
                if !quiet {
                    println!("{line}");
                }
            }
        }
    }
    Ok(())
}

/// `branch -u <upstream> [<branch>]`.
fn set_upstream_cmd(
    repo: &Repository,
    upstream: &str,
    branch: Option<&str>,
    quiet: bool,
) -> CliResult<()> {
    let name = match branch {
        Some(b) => b.to_string(),
        None => match repo.head_target()? {
            RefTarget::Symbolic(s) => s.trim_start_matches("refs/heads/").to_string(),
            RefTarget::Direct(_) => {
                return Err(CliError::fatal(
                    "could not set upstream of HEAD to {upstream} when it does not point to any branch.",
                ));
            }
        },
    };
    if repo.refs().resolve(&branch_ref(&name)).is_err() {
        return Err(CliError::fatal(format!("branch '{name}' does not exist")));
    }
    let up = upstream.trim_start_matches("refs/heads/");
    let known = repo.refs().resolve(&branch_ref(up)).is_ok()
        || repo.refs().resolve(&format!("refs/remotes/{up}")).is_ok();
    if !known {
        return Err(CliError::fatal(format!(
            "the requested upstream branch '{upstream}' does not exist"
        )));
    }
    if let Some(line) = crate::cmd::branch::set_upstream(repo, &name, up, None)? {
        if !quiet {
            println!("{line}");
        }
    }
    Ok(())
}

/// `branch --unset-upstream [<branch>]`.
fn unset_upstream_cmd(repo: &Repository, branch: Option<&str>) -> CliResult<()> {
    let name = match branch {
        Some(b) => b.to_string(),
        None => match repo.head_target()? {
            RefTarget::Symbolic(s) => s.trim_start_matches("refs/heads/").to_string(),
            RefTarget::Direct(_) => {
                return Err(CliError::fatal(
                    "could not unset upstream of HEAD when it does not point to any branch.",
                ));
            }
        },
    };
    let cfg = arks_config::ConfigSet::load(Some(repo.common_dir()));
    if cfg.get(&format!("branch.{name}.merge")).is_none() {
        return Err(CliError::fatal(format!(
            "branch '{name}' has no upstream information"
        )));
    }
    let _ = crate::cmd::config::cmd_config(&["--unset".into(), format!("branch.{name}.remote")]);
    let _ = crate::cmd::config::cmd_config(&["--unset".into(), format!("branch.{name}.merge")]);
    Ok(())
}

/// `branch --edit-description [<branch>]`: edit `branch.<name>.description`
/// in the editor; an empty result removes it.
fn edit_description(repo: &Repository, branch: Option<&str>) -> CliResult<()> {
    let name = match branch {
        Some(b) => b.to_string(),
        None => match repo.head_target()? {
            RefTarget::Symbolic(s) => s.trim_start_matches("refs/heads/").to_string(),
            RefTarget::Direct(_) => {
                return Err(CliError::fatal("Cannot give description to detached HEAD"));
            }
        },
    };
    if repo.refs().resolve(&branch_ref(&name)).is_err() {
        return Err(CliError::fatal(format!("No branch named '{name}'.")));
    }
    let cfg = arks_config::ConfigSet::load(Some(repo.common_dir()));
    let key = format!("branch.{name}.description");
    let mut seed = cfg.get(&key).unwrap_or_default();
    if !seed.is_empty() && !seed.ends_with('\n') {
        seed.push('\n');
    }
    seed.push_str(&format!(
        "# Please edit the description for the branch\n#   {name}\n# Lines starting with '#' will be stripped.\n"
    ));
    let path = repo.git_dir().join("EDIT_DESCRIPTION");
    std::fs::write(&path, &seed)?;
    if let Some(editor) = ["ARKS_EDITOR", "GIT_EDITOR", "EDITOR"]
        .iter()
        .find_map(|v| std::env::var(v).ok().filter(|s| !s.is_empty()))
    {
        let ok = std::process::Command::new("sh")
            .arg("-c")
            .arg(format!("{editor} \"$@\""))
            .arg(&editor)
            .arg(&path)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if !ok {
            return Err(CliError::Exit(1, "error: editor failed".into()));
        }
    }
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let desc = crate::cmd::trailers::stripspace(&text, true);
    if desc.is_empty() {
        if cfg.get(&key).is_some() {
            crate::cmd::config::cmd_config(&["--unset".into(), key])?;
        }
    } else {
        crate::cmd::config::cmd_config(&[key, desc])?;
    }
    Ok(())
}

fn list_branches(repo: &Repository, o: &ListOpts) -> CliResult<()> {
    let head = repo.head_target()?;
    // (display name, ref oid, is_current, upstream-capable short branch name, symref target)
    let mut rows: Vec<BranchRow> = Vec::new();
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
    if let Some(key) = &o.sort {
        let (desc, key) = match key.strip_prefix('-') {
            Some(k) => (true, k),
            None => (false, key.as_str()),
        };
        let date = |oid: &Oid, author: bool| -> i64 {
            crate::cmd::log_format::read_commit(repo, oid)
                .map(|c| {
                    if author {
                        c.author.timestamp
                    } else {
                        c.committer.timestamp
                    }
                })
                .unwrap_or(0)
        };
        // Detached HEAD row stays first; ties keep refname order (stable sort).
        let detached = rows.first().is_some_and(|r| r.0.starts_with('('));
        let tail = &mut rows[usize::from(detached)..];
        match key {
            "refname" => tail.sort_by(|a, b| a.0.cmp(&b.0)),
            "committerdate" | "creatordate" => tail.sort_by_key(|r| date(&r.1, false)),
            "authordate" => tail.sort_by_key(|r| date(&r.1, true)),
            "objectname" => tail.sort_by_key(|r| r.1.to_hex()),
            other => {
                return Err(CliError::NotImplemented(format!(
                    "branch --sort={other} not claimed"
                )));
            }
        }
        if desc {
            tail.reverse();
        }
    }
    let mut kept = Vec::new();
    for row in rows {
        let (name, oid, _, _, _) = &row;
        if !o.patterns.is_empty() {
            let bare = name.strip_prefix("remotes/").unwrap_or(name);
            let fold = |s: &str| {
                if o.ignore_case {
                    s.to_ascii_lowercase()
                } else {
                    s.to_string()
                }
            };
            let hit = !name.starts_with('(')
                && o.patterns.iter().any(|p| {
                    let p = fold(p);
                    fnmatch(p.as_bytes(), fold(bare).as_bytes())
                        || fnmatch(p.as_bytes(), fold(name).as_bytes())
                });
            if !hit {
                continue;
            }
        }
        if let Some(at) = &o.points_at {
            if oid != at {
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
    if o.column && o.verbose == 0 {
        let mut seen: HashSet<String> = HashSet::new();
        let items: Vec<String> = kept
            .iter()
            .filter(|r| seen.insert(r.0.clone()))
            .map(|(name, _, is_cur, _, sym)| {
                let mark = if *is_cur { '*' } else { ' ' };
                match sym {
                    Some(t) => format!("{mark} {name} -> {t}"),
                    None => format!("{mark} {name}"),
                }
            })
            .collect();
        print_columns(&items, "", 1);
        return Ok(());
    }
    let mut seen: HashSet<String> = HashSet::new();
    for (name, oid, is_cur, branch, sym) in &kept {
        if !seen.insert(name.clone()) {
            continue;
        }
        let mark = if *is_cur { '*' } else { ' ' };
        // `--color=always`: current branch green, remote-tracking red.
        let (on, off) = match (o.color, *is_cur, branch.is_none() && !name.starts_with('(')) {
            (true, true, _) => ("\x1b[32m", "\x1b[m"),
            (true, false, true) => ("\x1b[31m", "\x1b[m"),
            (true, false, false) => ("", "\x1b[m"),
            _ => ("", ""),
        };
        if let Some(t) = sym {
            println!("{mark} {on}{name}{off} -> {t}");
            continue;
        }
        if o.verbose == 0 {
            println!("{mark} {on}{name}{off}");
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
            "{mark} {on}{name:<width$}{off} {} {up}{subject}",
            abbrev_hex(repo, oid, o.abbrev)
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

/// `-m`/`-M` rename, or with `copy` the `-c`/`-C` copy (old branch, HEAD
/// and the old reflog stay; the new branch gets a copy of the reflog).
fn rename_branch(pos: &[String], force: bool, copy: bool) -> CliResult<()> {
    let mut repo = open_repo()?;
    let cur = match repo.head_target()? {
        RefTarget::Symbolic(s) => Some(s),
        RefTarget::Direct(_) => None,
    };
    let (old, new) = match pos {
        [new] => (
            cur.clone().ok_or_else(|| {
                CliError::fatal(if copy {
                    "cannot copy the current branch while not on any"
                } else {
                    "cannot rename the current branch while not on any"
                })
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
        Err(_) if cur.as_deref() == Some(old.as_str()) && !copy => {
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
    if old != new && cur.as_deref() == Some(new.as_str()) {
        let work = crate::util::work_tree(&repo)
            .map(|w| w.display().to_string())
            .unwrap_or_default();
        return Err(CliError::fatal(format!(
            "cannot force update the branch '{new_short}' used by worktree at '{work}'"
        )));
    }
    repo.refs_mut().update(&new, &oid, None)?;
    if copy {
        let logs = repo.common_dir().join("logs");
        if old != new && logs.join(&old).is_file() {
            if let Some(parent) = logs.join(&new).parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::copy(logs.join(&old), logs.join(&new));
        }
        return Ok(());
    }
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
                return Err(CliError::fatal(
                    "cannot force update the current branch.".to_string(),
                ));
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

/// `--column[=<options>]` / `column.ui`: `always` (or a bare `--column`)
/// lays the list out in columns; `auto` and `never` keep one item per
/// line because the output is not a terminal.
pub(crate) fn column_mode(p: &crate::cmd::opt_parse::Parsed) -> CliResult<bool> {
    let spec = match p.last("column") {
        None => {
            let repo = open_repo()?;
            let cfg = arks_config::ConfigSet::load(Some(repo.common_dir()));
            match cfg.get("column.ui") {
                Some(v) => v,
                None => return Ok(false),
            }
        }
        Some(None) => "always".to_string(),
        Some(Some(v)) => v,
    };
    let mut on = false;
    for tok in spec.split([',', ' ']).filter(|t| !t.is_empty()) {
        match tok {
            "always" => on = true,
            "never" | "auto" => on = false,
            "column" | "row" | "plain" | "dense" | "nodense" => {}
            other => {
                return Err(CliError::fatal(format!("unsupported option '{other}'")));
            }
        }
    }
    Ok(on)
}

/// git `column` layout (column-first, `COLUMNS` or 80 wide; branch pads 1,
/// tag pads 2).
pub(crate) fn print_columns(items: &[String], indent: &str, padding: usize) {
    if items.is_empty() {
        return;
    }
    let width: usize = std::env::var("COLUMNS")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|w| *w > 0)
        .unwrap_or(80);
    let cell = items.iter().map(|i| i.chars().count()).max().unwrap_or(0) + padding;
    let cols = ((width.saturating_sub(indent.len())) / cell).max(1);
    let rows = items.len().div_ceil(cols);
    let cols = items.len().div_ceil(rows);
    for r in 0..rows {
        let mut line = String::from(indent);
        for c in 0..cols {
            let Some(item) = items.get(r + c * rows) else {
                break;
            };
            let last = c + 1 == cols || r + (c + 1) * rows >= items.len();
            if last {
                line.push_str(item);
            } else {
                line.push_str(&format!("{item:<cell$}"));
            }
        }
        println!("{line}");
    }
}
