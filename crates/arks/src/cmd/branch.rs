//! Branch / checkout / switch.

use crate::cmd::opt_parse::{Mode, Parsed, parse, read_pathspec_file};
use crate::cmd::shared::resolve_rev;
use crate::error::{CliError, CliResult};
use crate::util::{open_repo, read_index, work_tree, write_index};
use arks_index::{IndexEntry, sort_entries};
use arks_refs::RefTarget;
use std::collections::HashMap;
use std::fs;
/// Options shared by `checkout` and `switch` once parsed.
struct SwitchOpts {
    quiet: bool,
    force: bool,
    detach: bool,
    /// `-t` / `--track[=direct|inherit]` (`Some(None)` = direct), `--no-track` = `Some(Some("no"))`.
    track: Option<Option<String>>,
    guess: bool,
    /// Options that make `-b <new>` without a start point update the tree
    /// (and report carried changes) instead of taking git's fast path.
    no_fast_path: bool,
}

fn switch_opts(p: &Parsed, force_key: &str) -> CliResult<SwitchOpts> {
    if let Some(style) = p.value("conflict") {
        if !matches!(style.as_str(), "merge" | "diff3" | "zdiff3") {
            return Err(CliError::fatal(format!("unknown conflict style '{style}'")));
        }
    }
    let track = match p.last("track") {
        None if p.flag("track") == Some(false) => Some(Some("no".to_string())),
        None => None,
        Some(v) => {
            if let Some(m) = &v {
                if !matches!(m.as_str(), "direct" | "inherit") {
                    return Err(CliError::Exit(
                        129,
                        "error: option `track' expects \"direct\" or \"inherit\"".to_string(),
                    ));
                }
            }
            Some(v)
        }
    };
    let quiet = p.on("quiet");
    if quiet {
        crate::util::set_quiet_from(&["-q".to_string()]);
    }
    Ok(SwitchOpts {
        quiet,
        force: p.on(force_key) || p.on("force"),
        detach: p.on("detach"),
        // `checkout -b <new>` leaves the fast path for an explicit tracking
        // choice, `-l`, `-m` or `--conflict`; `switch -c` keeps it.
        no_fast_path: force_key == "force"
            && (track.is_some() || p.on("-l") || p.on("merge") || p.value("conflict").is_some()),
        track,
        guess: p.flag("guess") != Some(false),
    })
}

/// The branch `@{-n}` names (n-th previous `checkout: moving from X to Y`).
fn previous_branch(repo: &arks_repo::Repository, n: usize) -> Option<String> {
    crate::reflog::read_entries(repo, "HEAD")
        .iter()
        .rev()
        .filter_map(|(_, _, msg)| {
            msg.strip_prefix("checkout: moving from ")
                .and_then(|r| r.split_once(" to "))
                .map(|(from, _)| from.to_string())
        })
        .nth(n - 1)
}

/// `-` / `@{-N}`: the previously checked-out branch name (or commit).
fn expand_previous(name: &str) -> CliResult<String> {
    let n = if name == "-" {
        1
    } else if let Some(n) = name
        .strip_prefix("@{-")
        .and_then(|r| r.strip_suffix('}'))
        .and_then(|n| n.parse::<usize>().ok())
    {
        n
    } else {
        return Ok(name.to_string());
    };
    let repo = open_repo()?;
    let spec = format!("@{{-{n}}}");
    match previous_branch(&repo, n) {
        Some(b)
            if repo.refs().resolve(&branch_ref(&b)).is_ok() || resolve_rev(&repo, &b).is_ok() =>
        {
            Ok(b)
        }
        _ => Err(CliError::fatal(format!("invalid reference: {spec}"))),
    }
}

/// A unique `refs/remotes/<remote>/<name>` for `--guess` (`None` when absent or ambiguous).
fn guess_remote_branch(repo: &arks_repo::Repository, name: &str) -> Option<String> {
    let dir = repo.common_dir().join("refs/remotes");
    let mut hits = Vec::new();
    if let Ok(rd) = fs::read_dir(&dir) {
        for r in rd.flatten() {
            let full = format!("refs/remotes/{}/{name}", r.file_name().to_string_lossy());
            if repo.refs().resolve(&full).is_ok() {
                hits.push(full);
            }
        }
    }
    let packed = fs::read_to_string(repo.common_dir().join("packed-refs")).unwrap_or_default();
    for l in packed.lines() {
        if let Some((_, r)) = l.split_once(' ') {
            if r.starts_with("refs/remotes/")
                && r.ends_with(&format!("/{name}"))
                && !hits.iter().any(|h| h == r)
            {
                hits.push(r.to_string());
            }
        }
    }
    (hits.len() == 1).then(|| hits.remove(0))
}

/// Record `branch.<name>.remote` / `.merge` for `start` (a local or
/// remote-tracking branch) and return git's "set up to track" line.
pub(crate) fn set_upstream(
    repo: &arks_repo::Repository,
    name: &str,
    start: &str,
    mode: Option<&str>,
) -> CliResult<Option<String>> {
    let short = name.trim_start_matches("refs/heads/");
    if mode == Some("inherit") {
        let cfg = arks_config::ConfigSet::load(Some(repo.common_dir()));
        let src = start.trim_start_matches("refs/heads/");
        let (Some(r), Some(m)) = (
            cfg.get(&format!("branch.{src}.remote")),
            cfg.get(&format!("branch.{src}.merge")),
        ) else {
            return Ok(None);
        };
        crate::cmd::config::cmd_config(&[format!("branch.{short}.remote"), r.clone()])?;
        crate::cmd::config::cmd_config(&[format!("branch.{short}.merge"), m.clone()])?;
        let shown = if r == "." {
            m.trim_start_matches("refs/heads/").to_string()
        } else {
            format!("{r}/{}", m.trim_start_matches("refs/heads/"))
        };
        return Ok(Some(format!("branch '{short}' set up to track '{shown}'.")));
    }
    let local = if start.starts_with("refs/heads/") {
        start.to_string()
    } else {
        format!("refs/heads/{start}")
    };
    let remote_full = if start.starts_with("refs/remotes/") {
        start.to_string()
    } else {
        format!("refs/remotes/{start}")
    };
    let (remote, merge, shown) = if repo.refs().resolve(&local).is_ok() {
        (
            ".".to_string(),
            local.clone(),
            local.trim_start_matches("refs/heads/").to_string(),
        )
    } else if repo.refs().resolve(&remote_full).is_ok() {
        let rest = remote_full.trim_start_matches("refs/remotes/");
        let (r, b) = rest
            .split_once('/')
            .ok_or_else(|| CliError::fatal(format!("not a valid upstream: {start}")))?;
        (r.to_string(), format!("refs/heads/{b}"), rest.to_string())
    } else {
        return Err(CliError::fatal(format!(
            "cannot set up tracking information; starting point '{start}' is not a branch"
        )));
    };
    crate::cmd::config::cmd_config(&[format!("branch.{short}.remote"), remote])?;
    crate::cmd::config::cmd_config(&[format!("branch.{short}.merge"), merge])?;
    Ok(Some(format!("branch '{short}' set up to track '{shown}'.")))
}

/// `checkout -b/-B` and `switch -c/-C`: create (or reset) `name` at `start`
/// and switch to it.
fn create_and_switch(
    name: &str,
    start: Option<&str>,
    force_create: bool,
    o: &SwitchOpts,
) -> CliResult<()> {
    let mut repo = open_repo()?;
    let head = repo.head_oid().ok();
    let oid = match start {
        Some(s) => crate::cmd::shared::peel_to_kind(
            &repo,
            &resolve_rev(&repo, s)?,
            crate::cmd::shared::ObjKind::Commit,
            s,
        )?,
        None => head.ok_or_else(|| CliError::Msg("HEAD has no commit".into()))?,
    };
    let full = branch_ref(name);
    let existed = repo.refs().resolve(&full).is_ok();
    if existed && !force_create {
        return Err(CliError::fatal(format!(
            "a branch named '{name}' already exists"
        )));
    }
    // Two-way switch first: refuse without moving refs when local changes conflict;
    // git skips the tree switch only for `-b <new>` without a start point.
    let work = work_tree(&repo)?;
    let carried = if o.force {
        crate::cmd::working_tree::hard_reset_tree(&repo, &work, &oid)?;
        Vec::new()
    } else if head != Some(oid) || start.is_some() || o.no_fast_path {
        crate::cmd::switch_tree::two_way_switch(&repo, &work, head.as_ref(), &oid)?
    } else {
        Vec::new()
    };
    let from = crate::reflog::head_label(&repo);
    let from_branch = match repo.head_target() {
        Ok(RefTarget::Symbolic(s)) => Some(s),
        _ => None,
    };
    let old_tip = repo.refs().resolve(&full).ok();
    repo.refs_mut().update(&full, &oid, None)?;
    repo.refs_mut().set_symbolic("HEAD", &full)?;
    let start_label = start.unwrap_or("HEAD");
    let _ = crate::reflog::append_reflog(
        &repo,
        &full,
        old_tip.as_ref(),
        &oid,
        &format!("branch: Created from {start_label}"),
    );
    let _ = crate::reflog::append_reflog(
        &repo,
        "HEAD",
        head.as_ref(),
        &oid,
        &format!("checkout: moving from {from} to {name}"),
    );
    // Tracking: explicit `--track`, or a remote-tracking start point
    // (branch.autoSetupMerge default).
    let track_line = match (&o.track, start) {
        (Some(Some(m)), _) if m == "no" => None,
        (Some(m), Some(s)) => set_upstream(&repo, name, s, m.as_deref())?,
        (Some(m), None) => match &from_branch {
            Some(b) => set_upstream(&repo, name, b, m.as_deref())?,
            None => set_upstream(&repo, name, "HEAD", None)?,
        },
        (None, Some(s))
            if s.starts_with("refs/remotes/") || {
                let rf = format!("refs/remotes/{s}");
                repo.refs().resolve(&branch_ref(s)).is_err() && repo.refs().resolve(&rf).is_ok()
            } =>
        {
            set_upstream(&repo, name, s, None)?
        }
        _ => None,
    };
    if !o.quiet {
        for l in &carried {
            println!("{l}");
        }
        if let Some(l) = track_line {
            println!("{l}");
        }
        if existed {
            eprintln!("Switched to and reset branch '{name}'");
        } else {
            eprintln!("Switched to a new branch '{name}'");
        }
    }
    Ok(())
}

/// Restore paths for `checkout [<tree-ish>] [--] <pathspec>...` and its
/// `--ours` / `--theirs` stage picks.
fn checkout_paths(
    paths: &[String],
    source: Option<&str>,
    overlay: bool,
    stage: Option<u16>,
) -> CliResult<()> {
    let refs: Vec<&String> = paths.iter().collect();
    if let Some(st) = stage {
        if checkout_stage_paths(&refs, st)? {
            return Ok(());
        }
    }
    match source {
        Some(rev) => restore_paths_mode(&refs, true, true, Some(rev), overlay),
        None => restore_paths_mode(&refs, false, true, None, true),
    }
}

/// `--ours` (stage 2) / `--theirs` (stage 3): write that stage of unmerged
/// paths to the work tree. Returns `false` when no path is unmerged.
pub(crate) fn checkout_stage_paths(paths: &[&String], stage: u16) -> CliResult<bool> {
    let repo = open_repo()?;
    let work = work_tree(&repo)?;
    let index = read_index(&repo)?;
    let mut any = false;
    for p in paths {
        let rel = crate::cmd::core::pathspec_rel(&work, p)?;
        let prefix = format!("{rel}/");
        for e in index.entries.iter().filter(|e| {
            e.stage() == stage && (rel.is_empty() || e.path == rel || e.path.starts_with(&prefix))
        }) {
            any = true;
            let abs = work.join(&e.path);
            if let Some(parent) = abs.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&abs, repo.get_object(&e.oid)?.payload)?;
        }
    }
    Ok(any)
}

fn pathspecs_from(p: &Parsed, mut paths: Vec<String>) -> CliResult<Vec<String>> {
    if let Some(f) = p.value("pathspec-from-file") {
        if !paths.is_empty() {
            return Err(CliError::fatal(
                "'--pathspec-from-file' and pathspec arguments cannot be used together",
            ));
        }
        paths = read_pathspec_file(&f, p.on("pathspec-file-nul"))?;
    } else if p.on("pathspec-file-nul") {
        return Err(CliError::fatal(
            "the option '--pathspec-file-nul' requires '--pathspec-from-file'",
        ));
    }
    Ok(paths)
}

pub fn cmd_checkout(args: &[String]) -> CliResult<()> {
    // Every option of `git checkout -h`; branch switching shares the two-way
    // switch with `switch`, path mode shares `restore`.
    // Cite: https://git-scm.com/docs/git-checkout
    let p = parse(
        "checkout",
        args,
        crate::cmd::porcelain_opts::CHECKOUT,
        Mode::default(),
    )?;
    let o = switch_opts(&p, "force")?;
    let stage = if p.on("ours") {
        Some(2)
    } else if p.on("theirs") {
        Some(3)
    } else {
        None
    };
    let mut positional = p.args();
    let dashdash = args.iter().any(|a| a == "--");
    let from_file = p.value("pathspec-from-file").is_some();
    if p.on("patch") {
        let mut a = vec!["-p".to_string()];
        a.extend(positional);
        a.extend(p.after_dashdash());
        return crate::cmd::interactive_patch::cmd_checkout_patch(&a);
    }
    let overlay = p.flag("overlay") != Some(false);
    if dashdash || from_file || stage.is_some() {
        let source = if dashdash || from_file {
            positional.first().cloned()
        } else {
            None
        };
        let mut paths: Vec<String> = if dashdash || from_file {
            positional.iter().skip(1).cloned().collect()
        } else {
            positional.clone()
        };
        paths.extend(p.after_dashdash());
        let paths = pathspecs_from(&p, paths)?;
        if paths.is_empty() {
            return Err(CliError::fatal("you must specify path(s) to restore"));
        }
        return checkout_paths(&paths, source.as_deref(), overlay, stage);
    }
    if let Some(name) = p.value("orphan") {
        return checkout_orphan(&name, positional.first().map(String::as_str));
    }
    let create = p
        .value("-b")
        .map(|n| (n, false))
        .or(p.value("-B").map(|n| (n, true)));
    if let Some((name, force_create)) = create {
        if positional.len() > 1 {
            return Err(CliError::fatal(format!(
                "'{}' is not a commit and a branch '{name}' cannot be created from it",
                positional[1]
            )));
        }
        let start = positional.first().cloned();
        return create_and_switch(&name, start.as_deref(), force_create, &o);
    }
    if o.track.as_ref().is_some_and(|t| t.as_deref() != Some("no")) {
        // `checkout --track <remote>/<branch>` creates the local branch.
        let Some(start) = positional.first().cloned() else {
            return Err(CliError::fatal("--track needs a branch name"));
        };
        let short = start
            .trim_start_matches("refs/remotes/")
            .split_once('/')
            .map(|(_, b)| b.to_string())
            .ok_or_else(|| CliError::fatal("missing branch name; try -b"))?;
        return create_and_switch(&short, Some(&start), false, &o);
    }
    if o.detach {
        return match positional.as_slice() {
            [] => detach_to("HEAD", o.force),
            [rev] => detach_to(rev, o.force),
            _ => Err(CliError::fatal("'--detach' does not take a path argument")),
        };
    }
    if positional.is_empty() {
        if o.force {
            return detach_or_force_head();
        }
        return Ok(());
    }
    if positional[0] == "-" || positional[0].starts_with("@{-") {
        match expand_previous(&positional[0]) {
            Ok(name) => positional[0] = name,
            Err(_) => {
                return Err(CliError::Exit(
                    1,
                    format!(
                        "error: pathspec '{}' did not match any file(s) known to git",
                        positional[0]
                    ),
                ));
            }
        }
    }
    if positional.len() == 1 && positional[0] == "HEAD" {
        // `checkout [-f] HEAD`: stay on the current branch.
        return if o.force {
            detach_or_force_head()
        } else {
            Ok(())
        };
    }
    // git-checkout: an argument that names no branch or commit but a path is a
    // pathspec (`checkout <path>` == `checkout -- <path>`).
    let repo = open_repo()?;
    let first = positional[0].clone();
    let is_rev =
        repo.refs().resolve(&branch_ref(&first)).is_ok() || resolve_rev(&repo, &first).is_ok();
    if !is_rev {
        let work = work_tree(&repo)?;
        let rel = crate::cmd::core::pathspec_rel(&work, &first).unwrap_or_default();
        let tracked = read_index(&repo)?
            .entries
            .iter()
            .any(|e| e.path == rel || e.path.starts_with(&format!("{rel}/")));
        if tracked {
            return checkout_paths(&positional, None, overlay, None);
        }
        if o.guess && positional.len() == 1 {
            if let Some(remote) = guess_remote_branch(&repo, &first) {
                drop(repo);
                return create_and_switch(&first, Some(&remote), false, &o);
            }
        }
        return Err(CliError::Exit(
            1,
            format!("error: pathspec '{first}' did not match any file(s) known to git"),
        ));
    } else if positional.len() > 1 {
        return checkout_paths(&positional[1..], Some(&first), overlay, None);
    }
    let is_branch = repo.refs().resolve(&branch_ref(&first)).is_ok();
    drop(repo);
    // A commit-ish that is not a branch name detaches HEAD (git-checkout).
    if !is_branch {
        return detach_to(&first, o.force);
    }
    switch_branch(&first, o.force)
}

/// `checkout -f` with no branch: throw away local changes to tracked files.
fn detach_or_force_head() -> CliResult<()> {
    let repo = open_repo()?;
    let work = work_tree(&repo)?;
    if let Ok(h) = repo.head_oid() {
        crate::cmd::working_tree::hard_reset_tree(&repo, &work, &h)?;
    }
    Ok(())
}

/// `checkout --orphan <new-branch> [<start-point>]`.
///
/// Points `HEAD` at an unborn branch (symbolic ref with no tip yet), and leaves
/// the index/worktree populated from `<start-point>` (default `HEAD`) so the
/// first commit has no parents. Cite: git-checkout(1) 2.39.0 `--orphan`.
fn checkout_orphan(name: &str, start_point: Option<&str>) -> CliResult<()> {
    let mut repo = open_repo()?;
    let start = start_point.unwrap_or("HEAD");
    let oid = resolve_rev(&repo, start)?;
    let full = branch_ref(name);
    if repo.refs().read(&full).is_ok() {
        return Err(CliError::fatal(format!(
            "a branch named '{name}' already exists"
        )));
    }
    // The index and work tree move to <start-point> like a branch switch.
    let work = work_tree(&repo)?;
    let head = repo.head_oid().ok();
    let carried = crate::cmd::switch_tree::two_way_switch(&repo, &work, head.as_ref(), &oid)?;
    // Unborn branch: symbolic HEAD only — do not create refs/heads/<name> yet.
    repo.refs_mut().set_symbolic("HEAD", &full)?;
    if !crate::util::quiet() {
        for l in &carried {
            println!("{l}");
        }
        eprintln!("Switched to a new branch '{name}'");
    }
    Ok(())
}

pub fn cmd_switch(args: &[String]) -> CliResult<()> {
    // Every option of `git switch -h`. Cite: https://git-scm.com/docs/git-switch
    let p = parse(
        "switch",
        args,
        crate::cmd::porcelain_opts::SWITCH,
        Mode::default(),
    )?;
    let o = switch_opts(&p, "discard-changes")?;
    let mut positional = p.args();
    positional.extend(p.after_dashdash());
    if let Some(name) = p.value("orphan") {
        if !positional.is_empty() {
            return Err(CliError::fatal("'--orphan' cannot take <start-point>"));
        }
        return switch_orphan(&name);
    }
    let create = p
        .value("create")
        .map(|n| (n, false))
        .or(p.value("force-create").map(|n| (n, true)));
    if positional.len() > 1 {
        return Err(CliError::Exit(
            129,
            "usage: git switch [<options>] [<branch>]".into(),
        ));
    }
    if let Some((name, force_create)) = create {
        let start = positional.first().cloned();
        return create_and_switch(&name, start.as_deref(), force_create, &o);
    }
    let Some(first) = positional.first().cloned() else {
        return Err(CliError::fatal("missing branch or commit argument"));
    };
    let name = expand_previous(&first)?;
    if o.detach {
        return detach_to(&name, o.force);
    }
    let repo = open_repo()?;
    if o.track.as_ref().is_some_and(|t| t.as_deref() != Some("no")) {
        let short = name
            .trim_start_matches("refs/remotes/")
            .split_once('/')
            .map(|(_, b)| b.to_string())
            .ok_or_else(|| CliError::fatal("missing branch name; try -c"))?;
        drop(repo);
        return create_and_switch(&short, Some(&name), false, &o);
    }
    if repo.refs().resolve(&branch_ref(&name)).is_ok() {
        drop(repo);
        return switch_branch(&name, o.force);
    }
    if resolve_rev(&repo, &name).is_ok() {
        let kind = if repo.refs().resolve(&format!("refs/tags/{name}")).is_ok() {
            "tag"
        } else if repo.refs().resolve(&format!("refs/remotes/{name}")).is_ok() {
            "remote-tracking branch"
        } else {
            "commit"
        };
        return Err(CliError::fatal(format!(
            "a branch is expected, got {kind} '{name}'\nhint: If you want to detach HEAD at the commit, try again with the --detach option."
        )));
    }
    if o.guess {
        if let Some(remote) = guess_remote_branch(&repo, &name) {
            drop(repo);
            return create_and_switch(&name, Some(&remote), false, &o);
        }
    }
    Err(CliError::fatal(format!("invalid reference: {name}")))
}

fn switch_orphan(name: &str) -> CliResult<()> {
    // git-switch --orphan: unborn branch; the HEAD tree's files leave the index
    // and work tree (refused when they carry local changes), others stay.
    let mut repo = open_repo()?;
    let full = branch_ref(name);
    if repo.refs().read(&full).is_ok() {
        return Err(CliError::fatal(format!(
            "a branch named '{name}' already exists"
        )));
    }
    let work = work_tree(&repo)?;
    let head_files = match repo.head_oid() {
        Ok(h) => crate::cmd::switch_tree::commit_files(&repo, &h)?,
        Err(_) => HashMap::new(),
    };
    let mut index = read_index(&repo)?;
    let mut conflicts = Vec::new();
    let mut paths: Vec<&String> = head_files.keys().collect();
    paths.sort();
    for p in paths {
        let f = &head_files[p];
        let entry = index
            .entries
            .iter()
            .find(|e| e.path == *p && e.stage() == 0);
        let clean = match entry {
            Some(e) if e.oid == f.oid => fs::read(work.join(p)).ok().is_some_and(|b| {
                arks_object::RawObject::new(arks_object::ObjectKind::Blob, b)
                    .oid(repo.object_format())
                    .ok()
                    == Some(f.oid)
            }),
            _ => false,
        };
        if !clean {
            conflicts.push(p.clone());
        }
    }
    if !conflicts.is_empty() {
        let mut msg = String::from(
            "error: Your local changes to the following files would be overwritten by checkout:\n",
        );
        for c in &conflicts {
            msg.push_str(&format!("\t{c}\n"));
        }
        msg.push_str(
            "Please commit your changes or stash them before you switch branches.\nAborting",
        );
        return Err(CliError::Exit(1, msg));
    }
    for p in head_files.keys() {
        let abs = work.join(p);
        if abs.is_file() || abs.is_symlink() {
            let _ = fs::remove_file(&abs);
        }
    }
    index.entries.retain(|e| !head_files.contains_key(&e.path));
    write_index(&repo, &index)?;
    repo.refs_mut().set_symbolic("HEAD", &full)?;
    if !crate::util::quiet() {
        eprintln!("Switched to a new branch '{name}'");
    }
    Ok(())
}

pub(crate) fn branch_ref(name: &str) -> String {
    if name.starts_with("refs/heads/") {
        name.to_string()
    } else {
        format!("refs/heads/{name}")
    }
}

fn switch_branch(name: &str, force: bool) -> CliResult<()> {
    let mut repo = open_repo()?;
    let full = branch_ref(name);
    let oid = repo.refs().resolve(&full)?;
    // Same-commit switch (e.g. `branch topic` then `checkout topic`): update
    // HEAD only — skip worktree/index rewrite (git short-circuit class; M17).
    // Do **not** short-circuit when the index is empty: unbundle/fetch can advance
    // `refs/heads/<name>` while HEAD already points there, leaving an empty worktree
    // that still needs materialization (G13 bundle→checkout).
    let same_oid = repo.head_oid().ok() == Some(oid);
    let index_populated = read_index(&repo)
        .map(|idx| !idx.entries.is_empty())
        .unwrap_or(false);
    // TOCTOU-003 (M19 Cx-2): refuse checkouts that would overwrite local edits — now
    // per path via the two-way switch (git semantics: non-conflicting changes carry over).
    let mut carried = Vec::new();
    if force {
        let work = work_tree(&repo)?;
        crate::cmd::working_tree::hard_reset_tree(&repo, &work, &oid)?;
    } else if !(same_oid && index_populated) {
        let work = work_tree(&repo)?;
        let head = repo.head_oid().ok();
        if index_populated {
            carried = crate::cmd::switch_tree::two_way_switch(&repo, &work, head.as_ref(), &oid)
                .map_err(|e| CliError::Msg(format!("{e} (TOCTOU-003)")))?;
        } else {
            arks_remote::checkout_commit(&repo, &work, &oid)?;
        }
    }
    let was_current = matches!(repo.head_target(), Ok(RefTarget::Symbolic(ref s)) if *s == full);
    let from = crate::reflog::head_label(&repo);
    let old_head = repo.head_oid().ok();
    repo.refs_mut().set_symbolic("HEAD", &full)?;
    let _ = crate::reflog::append_reflog(
        &repo,
        "HEAD",
        old_head.as_ref(),
        &oid,
        &format!("checkout: moving from {from} to {name}"),
    );
    for l in carried.iter().filter(|_| !crate::util::quiet()) {
        println!("{l}");
    }
    if was_current {
        if !crate::util::quiet() {
            eprintln!("Already on '{name}'");
        }
    } else {
        if !crate::util::quiet() {
            eprintln!("Switched to branch '{name}'");
        }
    }
    Ok(())
}

/// `checkout --detach <rev>` / `switch --detach <rev>` with a two-way switch.
fn detach_to(rev: &str, force: bool) -> CliResult<()> {
    let mut repo = open_repo()?;
    let oid = crate::cmd::shared::peel_to_kind(
        &repo,
        &resolve_rev(&repo, rev)?,
        crate::cmd::shared::ObjKind::Commit,
        rev,
    )?;
    let head = repo.head_oid().ok();
    // The two-way switch also reports carried local changes for a same-commit detach.
    let work = work_tree(&repo)?;
    let carried = if force {
        crate::cmd::working_tree::hard_reset_tree(&repo, &work, &oid)?;
        Vec::new()
    } else {
        crate::cmd::switch_tree::two_way_switch(&repo, &work, head.as_ref(), &oid)?
    };
    let from = crate::reflog::head_label(&repo);
    repo.refs_mut().update("HEAD", &oid, None)?;
    let _ = crate::reflog::append_reflog(
        &repo,
        "HEAD",
        head.as_ref(),
        &oid,
        &format!("checkout: moving from {from} to {rev}"),
    );
    for l in carried.iter().filter(|_| !crate::util::quiet()) {
        println!("{l}");
    }
    let subject = crate::cmd::log_format::read_commit(&repo, &oid)
        .map(|c| c.subject().to_string())
        .unwrap_or_default();
    if !crate::util::quiet() {
        eprintln!("HEAD is now at {} {subject}", &oid.to_hex()[..7]);
    }
    Ok(())
}

/// git-restore core (also `checkout -- <path>`): copy `<pathspec>` matches
/// from the source (`--source`, else HEAD for `--staged`, else the index)
/// into the index and/or working tree; `overlay` (git-checkout default) never removes paths
/// missing from the source.
pub(crate) fn restore_paths_mode(
    paths: &[&String],
    staged: bool,
    worktree: bool,
    source: Option<&str>,
    overlay: bool,
) -> CliResult<()> {
    let repo = open_repo()?;
    let work = work_tree(&repo)?;
    let mut index = read_index(&repo)?;
    let idx_map: HashMap<String, (arks_oid::Oid, u32)> = index
        .entries
        .iter()
        .filter(|e| e.stage() == 0)
        .map(|e| (e.path.clone(), (e.oid, e.mode)))
        .collect();
    let tree_files = |oid: &arks_oid::Oid| -> CliResult<HashMap<String, (arks_oid::Oid, u32)>> {
        let peeled = crate::cmd::shared::peel_tags(&repo, oid)?;
        Ok(crate::cmd::switch_tree::commit_files(&repo, &peeled)?
            .into_iter()
            .map(|(k, f)| (k, (f.oid, f.mode)))
            .collect())
    };
    let src_map = if let Some(rev) = source {
        tree_files(&resolve_rev(&repo, rev)?)?
    } else if staged {
        match repo.head_oid() {
            Ok(h) => tree_files(&h)?,
            Err(_) => HashMap::new(),
        }
    } else {
        idx_map.clone()
    };
    let mut matched: Vec<String> = Vec::new();
    for p in paths {
        let rel = crate::cmd::core::pathspec_rel(&work, p)?;
        let prefix = format!("{rel}/");
        let hit = |k: &String| rel.is_empty() || *k == rel || k.starts_with(&prefix);
        let before = matched.len();
        matched.extend(src_map.keys().filter(|k| hit(k)).cloned());
        let dest_known: Vec<String> = if overlay {
            Vec::new()
        } else if staged {
            idx_map.keys().filter(|k| hit(k)).cloned().collect()
        } else {
            // Work-tree restore from a tree also drops tracked paths absent there.
            idx_map
                .keys()
                .filter(|k| source.is_some() && hit(k))
                .cloned()
                .collect()
        };
        matched.extend(dest_known);
        if matched.len() == before {
            return Err(CliError::Exit(
                1,
                format!("error: pathspec '{p}' did not match any file(s) known to git"),
            ));
        }
    }
    matched.sort();
    matched.dedup();
    for rel in matched {
        let abs = work.join(&rel);
        match src_map.get(&rel) {
            Some((oid, mode)) => {
                if worktree {
                    let blob = repo.get_object(oid)?;
                    if let Some(parent) = abs.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    let payload =
                        arks_remote::smudge_worktree_blob(&repo, &work, &rel, &blob.payload)
                            .map_err(|e| CliError::Msg(e.to_string()))?;
                    fs::write(&abs, &payload)?;
                }
                if staged {
                    let size = repo.get_object(oid)?.payload.len().min(u32::MAX as usize) as u32;
                    index.entries.retain(|e| !(e.path == rel && e.stage() == 0));
                    index.entries.push(IndexEntry {
                        ctime_sec: 0,
                        ctime_nsec: 0,
                        mtime_sec: 0,
                        mtime_nsec: 0,
                        dev: 0,
                        ino: 0,
                        mode: *mode,
                        uid: 0,
                        gid: 0,
                        size,
                        oid: *oid,
                        flags: (rel.len().min(0x0fff)) as u16,
                        flags_extended: None,
                        path: rel,
                    });
                }
            }
            None => {
                if worktree && (abs.is_file() || abs.is_symlink()) {
                    fs::remove_file(&abs)?;
                }
                if staged {
                    index.entries.retain(|e| e.path != rel);
                }
            }
        }
    }
    if staged {
        sort_entries(&mut index.entries);
        write_index(&repo, &index)?;
    }
    Ok(())
}
