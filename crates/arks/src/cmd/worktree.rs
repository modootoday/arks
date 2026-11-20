//! Linked worktrees (add/list/remove/prune/move).
//!
//! Cite: https://git-scm.com/docs/git-worktree
//! Cite: https://git-scm.com/docs/gitrepository-layout
//! Plan: `.spec/plan/20261109022956-worktree-submodule-residual.plan.md`

use crate::cmd::shared::resolve_rev;
use crate::error::{CliError, CliResult};
use crate::util::{open_repo, work_tree};
use arks_oid::Oid;
use arks_refs::RefTarget;
use arks_repo::Repository;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

const USAGE: &str =
    "usage: git worktree add [-f] [--detach] [--checkout] [--lock [--reason <string>]]
                        [--orphan] [(-b | -B) <new-branch>] <path> [<commit-ish>]
   or: git worktree list [-v | --porcelain [-z]]
   or: git worktree lock [--reason <string>] <worktree>
   or: git worktree move <worktree> <new-path>
   or: git worktree prune [-n] [-v] [--expire <expire>]
   or: git worktree remove [-f] <worktree>
   or: git worktree repair [<path>...]
   or: git worktree unlock <worktree>";

/// Parse a subcommand's options against its table.
fn sub_opts(
    sub: &str,
    args: &[String],
    defs: &[crate::cmd::opt_parse::OptDef],
) -> CliResult<(crate::cmd::opt_parse::Parsed, Vec<String>)> {
    let p = crate::cmd::opt_parse::parse(
        &format!("worktree {sub}"),
        args,
        defs,
        crate::cmd::opt_parse::Mode::default(),
    )?;
    let mut pos = p.args();
    pos.extend(p.after_dashdash());
    Ok((p, pos))
}

pub fn cmd_worktree(args: &[String]) -> CliResult<()> {
    let sub = args
        .first()
        .map(String::as_str)
        .ok_or_else(|| CliError::Exit(129, format!("error: need a subcommand\n{USAGE}")))?;
    let rest = &args[1..];
    match sub {
        "add" => wt_add(rest),
        "list" => wt_list(rest),
        "remove" => wt_remove(rest),
        "prune" => wt_prune(rest),
        "move" => wt_move(rest),
        // M16 Sy-2: lock/unlock claimed (git-worktree). Cite: https://git-scm.com/docs/git-worktree/2.39.0
        "lock" => wt_lock(rest),
        "unlock" => wt_unlock(rest),
        "repair" => wt_repair(rest),
        other => Err(CliError::Exit(
            129,
            format!("error: unknown subcommand: `{other}'\n{USAGE}"),
        )),
    }
}

fn wt_add(args: &[String]) -> CliResult<()> {
    // Every option of `git worktree add -h`. Cite: https://git-scm.com/docs/git-worktree
    use crate::cmd::opt_parse::{Mode, parse};
    let p = parse(
        "worktree add",
        args,
        crate::cmd::porcelain_opts::WORKTREE__ADD,
        Mode::default(),
    )?;
    let new_b = p.value("-b");
    let reset_b = p.value("-B");
    let detach = p.on("detach");
    let orphan = p.on("orphan");
    let force = p.count("force") > 0;
    let quiet = p.on("quiet");
    let checkout = p.flag("checkout") != Some(false);
    let lock = p.on("lock");
    let reason = p.value("reason");
    let track = p.flag("track");
    if [new_b.is_some(), reset_b.is_some(), detach]
        .iter()
        .filter(|x| **x)
        .count()
        > 1
    {
        return Err(CliError::fatal(
            "options '-b', '-B', and '--detach' cannot be used together",
        ));
    }
    if reason.is_some() && !lock {
        return Err(CliError::fatal("the option '--reason' requires '--lock'"));
    }
    if orphan && detach {
        return Err(CliError::fatal(
            "options '--orphan' and '--detach' cannot be used together",
        ));
    }
    if orphan && !checkout {
        return Err(CliError::fatal(
            "options '--orphan' and '--no-checkout' cannot be used together",
        ));
    }
    let mut positional = p.args();
    positional.extend(p.after_dashdash());
    if positional.is_empty() || positional.len() > 2 {
        return Err(CliError::Exit(
            129,
            "usage: git worktree add [-f] [--detach] [--checkout] [--lock [--reason <string>]]\n                        [--orphan] [(-b | -B) <new-branch>] <path> [<commit-ish>]".into(),
        ));
    }
    if orphan && positional.len() > 1 {
        return Err(CliError::fatal(
            "'--orphan' and '<commit-ish>' cannot be used together",
        ));
    }
    let path = PathBuf::from(&positional[0]);
    let commit_ish = positional.get(1).cloned();

    let mut repo = open_repo()?;
    let common = repo.common_dir().to_path_buf();
    let abs_path = absolutize(&path)?;
    if abs_path.exists()
        && fs::read_dir(&abs_path)
            .map(|mut d| d.next().is_some())
            .unwrap_or(true)
    {
        return Err(CliError::fatal(format!(
            "'{}' already exists",
            positional[0]
        )));
    }
    let basename = abs_path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    // What HEAD of the new worktree will be: a branch (created, reset or
    // existing), a detached commit, or an unborn orphan branch.
    let mut branch: Option<(String, bool)> = None; // (name, create)
    let mut oid: Option<Oid> = None;
    let checked_out_elsewhere = |repo: &Repository, full: &str| -> CliResult<Option<PathBuf>> {
        Ok(list_worktrees(repo)?
            .into_iter()
            .find(|wt| wt.branch.as_deref() == Some(full))
            .map(|wt| wt.path))
    };
    let resolve_commit = |repo: &Repository, rev: &str| -> CliResult<Oid> {
        let o = resolve_rev(repo, rev)
            .map_err(|_| CliError::fatal(format!("invalid reference: {rev}")))?;
        crate::cmd::shared::peel_to_kind(repo, &o, crate::cmd::shared::ObjKind::Commit, rev)
    };
    if orphan {
        let name = new_b
            .clone()
            .or(reset_b.clone())
            .unwrap_or(basename.clone());
        branch = Some((name, false));
    } else if let Some(name) = new_b.clone().or(reset_b.clone()) {
        let start = commit_ish.clone().unwrap_or_else(|| "HEAD".into());
        oid = Some(resolve_commit(&repo, &start)?);
        if new_b.is_some() && repo.refs().resolve(&branch_ref(&name)).is_ok() {
            eprintln!("Preparing worktree (new branch '{name}')");
            return Err(CliError::Exit(
                255,
                format!("fatal: a branch named '{name}' already exists"),
            ));
        }
        if reset_b.is_some() && !force {
            if let Some(at) = checked_out_elsewhere(&repo, &branch_ref(&name))? {
                return Err(CliError::fatal(format!(
                    "'{name}' is already used by worktree at '{}'",
                    at.display()
                )));
            }
        }
        branch = Some((name, true));
    } else if detach {
        let start = commit_ish.clone().unwrap_or_else(|| "HEAD".into());
        oid = Some(resolve_commit(&repo, &start)?);
    } else if let Some(rev) = &commit_ish {
        let full = branch_ref(rev);
        if repo.refs().resolve(&full).is_ok() && !rev.starts_with("refs/") {
            oid = Some(repo.refs().resolve(&full)?);
            branch = Some((rev.clone(), false));
        } else {
            oid = Some(resolve_commit(&repo, rev)?);
        }
    } else {
        // DWIM: a branch named after the last path component.
        let full = branch_ref(&basename);
        match repo.refs().resolve(&full) {
            Ok(o) => {
                oid = Some(o);
                branch = Some((basename.clone(), false));
            }
            Err(_) => {
                oid = Some(repo.head_oid()?);
                branch = Some((basename.clone(), true));
            }
        }
    }
    if let Some((name, false)) = &branch {
        if !orphan && !force {
            if let Some(at) = checked_out_elsewhere(&repo, &branch_ref(name))? {
                return Err(CliError::fatal(format!(
                    "'{name}' is already used by worktree at '{}'",
                    at.display()
                )));
            }
        }
    }
    if !quiet {
        match (&branch, orphan, oid) {
            (Some((name, _)), true, _) => eprintln!("Preparing worktree (new branch '{name}')"),
            (Some((name, true)), _, _) if reset_b.is_some() => {
                if repo.refs().resolve(&branch_ref(name)).is_ok() {
                    let old = repo.refs().resolve(&branch_ref(name))?;
                    eprintln!(
                        "Preparing worktree (resetting branch '{name}'; was at {})",
                        crate::cmd::shared::abbrev_hex(&repo, &old, 7)
                    );
                } else {
                    eprintln!("Preparing worktree (new branch '{name}')");
                }
            }
            (Some((name, true)), _, _) => eprintln!("Preparing worktree (new branch '{name}')"),
            (Some((name, false)), _, _) => {
                eprintln!("Preparing worktree (checking out '{name}')")
            }
            (None, _, Some(o)) => eprintln!(
                "Preparing worktree (detached HEAD {})",
                crate::cmd::shared::abbrev_hex(&repo, &o, 7)
            ),
            _ => {}
        }
    }
    if let (Some((name, true)), Some(o)) = (&branch, oid) {
        let full = branch_ref(name);
        let old = repo.refs().resolve(&full).ok();
        repo.refs_mut().update(&full, &o, old.as_ref())?;
        let start = commit_ish.clone().unwrap_or_else(|| "HEAD".into());
        let _ = crate::reflog::append_reflog(
            &repo,
            &full,
            old.as_ref(),
            &o,
            &format!("branch: Created from {start}"),
        );
        let auto = commit_ish
            .as_deref()
            .is_some_and(|c| repo.refs().resolve(&format!("refs/remotes/{c}")).is_ok());
        if track == Some(true) || (track.is_none() && auto) {
            if let Some(line) = crate::cmd::branch::set_upstream(
                &repo,
                &full,
                commit_ish.as_deref().unwrap_or("HEAD"),
                None,
            )? {
                if !quiet {
                    println!("{line}");
                }
            }
        }
    }
    let head_payload = match &branch {
        Some((name, _)) => format!("ref: {}\n", branch_ref(name)),
        None => format!("{}\n", oid.map(|o| o.to_hex()).unwrap_or_default()),
    };

    let id = worktree_id_from_path(&abs_path);
    let admin = common.join("worktrees").join(&id);
    if admin.exists() {
        return Err(CliError::Msg(format!(
            "worktree admin '{}' already exists",
            admin.display()
        )));
    }
    fs::create_dir_all(&admin)?;
    if lock {
        fs::write(admin.join("locked"), reason.clone().unwrap_or_default())?;
    }
    // commondir relative to admin → common
    let rel_common = pathdiff_to(&admin, &common)?;
    fs::write(
        admin.join("commondir"),
        format!("{}\n", path_to_unix(&rel_common)),
    )?;
    fs::write(admin.join("HEAD"), head_payload.as_bytes())?;

    fs::create_dir_all(&abs_path)?;
    let gitfile = abs_path.join(".git");
    let admin_abs = fs::canonicalize(&admin).unwrap_or(admin.clone());
    {
        let mut f = fs::File::create(&gitfile)?;
        writeln!(f, "gitdir: {}", admin_abs.display())?;
    }
    let gitfile_abs = strip_verbatim(fs::canonicalize(&gitfile).unwrap_or(gitfile.clone()));
    fs::write(admin.join("gitdir"), format!("{}\n", gitfile_abs.display()))?;

    let Some(oid) = oid else {
        // --orphan: unborn branch, empty index and work tree.
        return Ok(());
    };
    if !checkout {
        return Ok(());
    }
    let linked = Repository::open(&admin)?;
    arks_remote::checkout_commit(&linked, &abs_path, &oid)
        .map_err(|e| CliError::Msg(e.to_string()))?;
    if !quiet {
        let subject = crate::cmd::log_format::read_commit(&repo, &oid)
            .map(|c| c.subject().to_string())
            .unwrap_or_default();
        println!(
            "HEAD is now at {} {subject}",
            crate::cmd::shared::abbrev_hex(&repo, &oid, 7)
        );
    }
    Ok(())
}

fn wt_repair(args: &[String]) -> CliResult<()> {
    let (_, paths) = sub_opts("repair", args, &[])?;
    let mut bad = false;
    for p in &paths {
        if !Path::new(p).join(".git").exists() {
            eprintln!("error: not a valid path: {p}");
            bad = true;
        }
    }
    let repo = open_repo()?;
    // WT-005 (M20 Cx-5): main move leaves stale absolute gitdir pointers.
    // WT-009 (M20 Cx-6): concurrent dual repair — refuse when repair.lock held.
    // Cite: https://git-scm.com/docs/git-worktree (repair)
    let repair_lock = repo.common_dir().join("worktrees/.repair.lock");
    if repair_lock.is_file() {
        return Err(CliError::Msg(format!(
            "{} (WT-009 — concurrent worktree repair refuse; serialize repair after path moves)",
            repair_lock.display()
        )));
    }
    let root = repo.common_dir().join("worktrees");
    if root.is_dir() {
        for entry in fs::read_dir(&root)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let gitdir = entry.path().join("gitdir");
            if !gitdir.is_file() {
                continue;
            }
            let text = fs::read_to_string(&gitdir)?;
            let target = text.trim();
            if target.is_empty() {
                continue;
            }
            let pb = PathBuf::from(target);
            if !pb.exists() {
                return Err(CliError::Msg(format!(
                    "worktree repair refused: stale gitdir '{}' (WT-005 — main worktree move broke linked pointers; fix paths manually or re-add worktree; no invented auto-relocate dialect)",
                    pb.display()
                )));
            }
        }
    }
    // WT-010: surface mixed worktreeConfig honesty when extension present.
    let cfg = fs::read_to_string(repo.git_dir().join("config")).unwrap_or_default();
    if cfg.to_ascii_lowercase().contains("worktreeconfig") {
        eprintln!(
            "note: WT-010 — extensions.worktreeConfig present; mixed true/false clients diverge effective config"
        );
    }
    if bad {
        return Err(CliError::silent(1));
    }
    Ok(())
}

fn wt_list(args: &[String]) -> CliResult<()> {
    // M16 Sy-2: `--porcelain` / `-v` claimed; `-z` remains refuse.
    // Cite: https://git-scm.com/docs/git-worktree/2.39.0
    let (p, pos) = sub_opts("list", args, crate::cmd::porcelain_opts::WORKTREE__LIST)?;
    if !pos.is_empty() {
        return Err(CliError::Exit(129, USAGE.to_string()));
    }
    let porcelain = p.on("porcelain");
    let verbose = p.on("verbose");
    let nul = p.on("-z");
    if nul && !porcelain {
        return Err(CliError::fatal("the option '-z' requires '--porcelain'"));
    }
    if porcelain && verbose {
        return Err(CliError::fatal(
            "options '--verbose' and '--porcelain' cannot be used together",
        ));
    }
    // `--expire` only changes which missing worktrees list as prunable.
    let _ = p.value("expire");
    let eol = if nul { '\0' } else { '\n' };
    let repo = open_repo()?;
    let trees = list_worktrees(&repo)?;
    if porcelain {
        for wt in trees {
            print!("worktree {}{eol}", path_to_unix(&wt.path));
            print!("HEAD {}{eol}", wt.oid.to_hex());
            if let Some(b) = &wt.branch {
                print!("branch {b}{eol}");
            } else {
                print!("detached{eol}");
            }
            if wt.locked {
                if let Some(reason) = &wt.lock_reason {
                    let reason = reason.trim();
                    if reason.is_empty() {
                        print!("locked{eol}");
                    } else {
                        print!("locked {reason}{eol}");
                    }
                } else {
                    print!("locked{eol}");
                }
            }
            print!("{eol}");
        }
        return Ok(());
    }
    // git worktree.c show_worktree: path padded to 1 + the longest path, a
    // space, the abbreviated HEAD, then `[branch]` / `(detached HEAD)`.
    let w = trees
        .iter()
        .map(|t| t.path.display().to_string().chars().count())
        .max()
        .unwrap_or(0)
        + 1;
    for wt in trees {
        let short = short_oid(&wt.oid);
        let trail = if let Some(b) = &wt.branch {
            let name = b.strip_prefix("refs/heads/").unwrap_or(b);
            format!("[{name}]")
        } else {
            "(detached HEAD)".to_string()
        };
        let lock_same_line = if wt.locked {
            if verbose {
                match wt.lock_reason.as_deref().map(str::trim) {
                    Some(r) if !r.is_empty() => {
                        println!("{:<w$} {} {}", wt.path.display().to_string(), short, trail);
                        println!("\tlocked: {r}");
                        continue;
                    }
                    _ => " locked",
                }
            } else {
                " locked"
            }
        } else {
            ""
        };
        println!(
            "{:<w$} {} {}{}",
            wt.path.display().to_string(),
            short,
            trail,
            lock_same_line
        );
    }
    Ok(())
}

fn wt_lock(args: &[String]) -> CliResult<()> {
    let (p, pos) = sub_opts("lock", args, crate::cmd::porcelain_opts::WORKTREE__LOCK)?;
    if pos.len() != 1 {
        return Err(CliError::Exit(129, USAGE.to_string()));
    }
    let reason = p.value("reason");
    let path_arg = &pos[0];
    let repo = open_repo()?;
    let target = find_worktree(&repo, path_arg)?;
    if target.is_main {
        return Err(CliError::Msg(
            "cannot lock the main working tree (no admin dir)".into(),
        ));
    }
    let admin = target
        .admin
        .ok_or_else(|| CliError::Msg("missing worktree admin dir".into()))?;
    let locked = admin.join("locked");
    if locked.is_file() {
        return Err(CliError::Msg(format!(
            "'{}' is already locked",
            target.path.display()
        )));
    }
    let body = reason.unwrap_or_default();
    fs::write(&locked, format!("{body}\n"))?;
    Ok(())
}

fn wt_unlock(args: &[String]) -> CliResult<()> {
    let (_, pos) = sub_opts("unlock", args, &[])?;
    if pos.len() != 1 {
        return Err(CliError::Exit(129, USAGE.to_string()));
    }
    let path_arg = &pos[0];
    let repo = open_repo()?;
    let target = find_worktree(&repo, path_arg)?;
    if target.is_main {
        return Err(CliError::Msg(
            "cannot unlock the main working tree (no admin dir)".into(),
        ));
    }
    let admin = target
        .admin
        .ok_or_else(|| CliError::Msg("missing worktree admin dir".into()))?;
    let locked = admin.join("locked");
    if !locked.is_file() {
        return Err(CliError::Msg(format!(
            "'{}' is not locked",
            target.path.display()
        )));
    }
    fs::remove_file(&locked)?;
    Ok(())
}

fn wt_remove(args: &[String]) -> CliResult<()> {
    let (p, pos) = sub_opts("remove", args, crate::cmd::porcelain_opts::WORKTREE__REMOVE)?;
    if pos.len() != 1 {
        return Err(CliError::Exit(129, USAGE.to_string()));
    }
    // `-f` removes a dirty worktree; `-f -f` also a locked one.
    let forces = p.count("force");
    let force = forces > 0;
    let path_arg = &pos[0];
    let repo = open_repo()?;
    let target = find_worktree(&repo, path_arg)?;
    if target.is_main {
        return Err(CliError::Msg("cannot remove the main working tree".into()));
    }
    if target.locked && forces < 2 {
        return Err(CliError::Msg(format!(
            "working tree '{}' is locked; unlock before remove",
            target.path.display()
        )));
    }
    if !force && !is_clean_worktree(&target.path)? {
        return Err(CliError::Msg(format!(
            "'{}' contains modified or untracked files, use --force to delete it",
            target.path.display()
        )));
    }
    let admin = target
        .admin
        .ok_or_else(|| CliError::Msg("missing worktree admin dir".into()))?;
    if target.path.exists() {
        fs::remove_dir_all(&target.path)?;
    }
    fs::remove_dir_all(&admin)?;
    Ok(())
}

fn wt_prune(args: &[String]) -> CliResult<()> {
    let (p, pos) = sub_opts("prune", args, crate::cmd::porcelain_opts::WORKTREE__PRUNE)?;
    if !pos.is_empty() {
        return Err(CliError::Exit(129, USAGE.to_string()));
    }
    let dry_run = p.on("dry-run");
    let verbose = p.on("verbose") || dry_run;
    // `--expire never` keeps everything; other times prune what is stale now.
    if p.value("expire").as_deref() == Some("never") {
        return Ok(());
    }
    let repo = open_repo()?;
    let common = repo.common_dir();
    let root = common.join("worktrees");
    if !root.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(&root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let admin = entry.path();
        if admin.join("locked").is_file() {
            continue;
        }
        // TOCTOU-007 (M20 Cx-3): refuse pruning incomplete-in-progress admin
        // (gitdir present + .incomplete marker) — add/prune race class.
        // Cite: https://git-scm.com/docs/git-worktree
        if admin.join(".incomplete").is_file() && admin.join("gitdir").is_file() {
            return Err(CliError::Msg(format!(
                "worktree prune refused: incomplete admin '{}' (TOCTOU-007 — worktree add/prune race; wait for add to finish)",
                admin.display()
            )));
        }
        let gitdir_path = admin.join("gitdir");
        let stale = if !gitdir_path.is_file() {
            true
        } else {
            let text = fs::read_to_string(&gitdir_path)?;
            let line = text.lines().map(str::trim).find(|l| !l.is_empty());
            match line {
                Some(p) => !Path::new(p).exists(),
                None => true,
            }
        };
        if stale {
            let id = entry.file_name().to_string_lossy().into_owned();
            if verbose {
                println!("Removing worktrees/{id}: gitdir file points to non-existent location");
            }
            if !dry_run {
                fs::remove_dir_all(&admin)?;
            }
        }
    }
    Ok(())
}

fn wt_move(args: &[String]) -> CliResult<()> {
    let (p, args) = sub_opts("move", args, crate::cmd::porcelain_opts::WORKTREE__MOVE)?;
    if args.len() != 2 {
        return Err(CliError::Exit(129, USAGE.to_string()));
    }
    let forces = p.count("force");
    let repo = open_repo()?;
    // SUB-008 (M20 Cx-3): stock refuses move when worktree contains submodules.
    // Cite: https://git-scm.com/docs/git-worktree (move notes)
    if repo.common_dir().join("modules").is_dir() || worktree_has_gitmodules(&repo, &args[0])? {
        return Err(CliError::Msg(
            "worktree move refused: submodules present (SUB-008 — cannot move worktree containing submodules)".into(),
        ));
    }
    let target = find_worktree(&repo, &args[0])?;
    if target.is_main {
        return Err(CliError::Msg("cannot move the main working tree".into()));
    }
    if target.locked && forces < 2 {
        return Err(CliError::Msg(format!(
            "working tree '{}' is locked",
            target.path.display()
        )));
    }
    let new_path = absolutize(Path::new(&args[1]))?;
    if new_path.exists() {
        return Err(CliError::Msg(format!(
            "'{}' already exists",
            new_path.display()
        )));
    }
    if let Some(parent) = new_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::rename(&target.path, &new_path)?;
    let admin = target
        .admin
        .ok_or_else(|| CliError::Msg("missing worktree admin dir".into()))?;
    let gitfile = new_path.join(".git");
    let admin_abs = fs::canonicalize(&admin).unwrap_or(admin.clone());
    fs::write(&gitfile, format!("gitdir: {}\n", admin_abs.display()))?;
    let gitfile_abs = strip_verbatim(fs::canonicalize(&gitfile).unwrap_or(gitfile));
    fs::write(admin.join("gitdir"), format!("{}\n", gitfile_abs.display()))?;
    Ok(())
}

fn worktree_has_gitmodules(repo: &Repository, path_arg: &str) -> CliResult<bool> {
    let target = find_worktree(repo, path_arg)?;
    Ok(target.path.join(".gitmodules").is_file()
        || target.path.join(".git").join("modules").is_dir())
}

#[derive(Debug, Clone)]
struct WorktreeInfo {
    path: PathBuf,
    oid: Oid,
    branch: Option<String>,
    locked: bool,
    lock_reason: Option<String>,
    is_main: bool,
    admin: Option<PathBuf>,
}

fn list_worktrees(repo: &Repository) -> CliResult<Vec<WorktreeInfo>> {
    let mut out = Vec::new();
    let common = repo.common_dir();
    // Main worktree
    let main_path = if common
        .file_name()
        .is_some_and(|n| n == std::ffi::OsStr::new(".git"))
    {
        common.parent().map(|p| p.to_path_buf())
    } else {
        None
    };
    if let Some(path) = main_path {
        let main_repo = Repository::open(common)?;
        let (oid, branch) = head_info(&main_repo)?;
        out.push(WorktreeInfo {
            path: fs::canonicalize(&path).unwrap_or(path),
            oid,
            branch,
            locked: false,
            lock_reason: None,
            is_main: true,
            admin: None,
        });
    }
    let root = common.join("worktrees");
    if root.is_dir() {
        let mut ids: Vec<_> = fs::read_dir(&root)?
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
            .map(|e| e.path())
            .collect();
        ids.sort();
        for admin in ids {
            let locked_path = admin.join("locked");
            let (locked, lock_reason) = if locked_path.is_file() {
                let reason = fs::read_to_string(&locked_path).ok();
                (true, reason)
            } else {
                (false, None)
            };
            let path = match read_worktree_path(&admin) {
                Ok(p) => p,
                Err(_) => continue,
            };
            let linked = match Repository::open(&admin) {
                Ok(r) => r,
                Err(_) => continue,
            };
            let (oid, branch) = head_info(&linked)?;
            out.push(WorktreeInfo {
                path,
                oid,
                branch,
                locked,
                lock_reason,
                is_main: false,
                admin: Some(admin),
            });
        }
    }
    Ok(out)
}

fn find_worktree(repo: &Repository, arg: &str) -> CliResult<WorktreeInfo> {
    let want = absolutize(Path::new(arg)).unwrap_or_else(|_| PathBuf::from(arg));
    let want_canon = fs::canonicalize(&want).unwrap_or(want.clone());
    let name = Path::new(arg)
        .file_name()
        .map(|s| s.to_string_lossy().into_owned());
    for wt in list_worktrees(repo)? {
        let path_canon = fs::canonicalize(&wt.path).unwrap_or_else(|_| wt.path.clone());
        if path_canon == want_canon || wt.path == want {
            return Ok(wt);
        }
        if let (Some(n), Some(admin)) = (&name, &wt.admin) {
            if admin.file_name().is_some_and(|f| f == n.as_str()) {
                return Ok(wt);
            }
        }
    }
    Err(CliError::fatal(format!("'{arg}' is not a working tree")))
}

fn read_worktree_path(admin: &Path) -> CliResult<PathBuf> {
    let text = fs::read_to_string(admin.join("gitdir"))?;
    let line = text
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .ok_or_else(|| CliError::Msg("empty gitdir pointer".into()))?;
    let gitfile = PathBuf::from(line);
    Ok(gitfile
        .parent()
        .ok_or_else(|| CliError::Msg("gitdir has no parent".into()))?
        .to_path_buf())
}

fn head_info(repo: &Repository) -> CliResult<(Oid, Option<String>)> {
    let oid = repo.head_oid()?;
    let branch = match repo.head_target()? {
        RefTarget::Symbolic(name) if name.starts_with("refs/heads/") => Some(name),
        _ => None,
    };
    Ok((oid, branch))
}

fn is_clean_worktree(path: &Path) -> CliResult<bool> {
    // Advisory cleanliness: no untracked files besides `.git`; index dirty check via status-like walk.
    let repo = Repository::discover(path)?;
    let work = work_tree(&repo)?;
    let index = crate::util::read_index(&repo)?;
    for e in index.entries.iter().filter(|e| e.stage() == 0) {
        let p = work.join(&e.path);
        if !p.is_file() {
            return Ok(false);
        }
        let bytes = fs::read(&p)?;
        let digest = arks_hash::hash_object(arks_hash::ObjectFormat::Sha1, "blob", &bytes)
            .map_err(|err| CliError::Msg(err.to_string()))?;
        let oid = Oid::from_bytes(arks_oid::OidKind::Sha1, digest.as_bytes())
            .map_err(|err| CliError::Msg(err.to_string()))?;
        if oid != e.oid {
            return Ok(false);
        }
    }
    // Untracked (non-.git) files → dirty.
    if has_untracked(&work, &index)? {
        return Ok(false);
    }
    Ok(true)
}

fn has_untracked(work: &Path, index: &arks_index::Index) -> CliResult<bool> {
    use std::collections::HashSet;
    let tracked: HashSet<_> = index
        .entries
        .iter()
        .filter(|e| e.stage() == 0)
        .map(|e| e.path.clone())
        .collect();
    fn walk(dir: &Path, prefix: &str, tracked: &HashSet<String>) -> CliResult<bool> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let name = entry.file_name();
            let name_s = name.to_string_lossy();
            if name_s == ".git" {
                continue;
            }
            let rel = if prefix.is_empty() {
                name_s.to_string()
            } else {
                format!("{prefix}/{name_s}")
            };
            let path = entry.path();
            if path.is_dir() {
                if walk(&path, &rel, tracked)? {
                    return Ok(true);
                }
            } else if !tracked.contains(&rel) {
                return Ok(true);
            }
        }
        Ok(false)
    }
    walk(work, "", &tracked)
}

fn branch_ref(name: &str) -> String {
    if name.starts_with("refs/heads/") {
        name.to_string()
    } else {
        format!("refs/heads/{name}")
    }
}

fn worktree_id_from_path(path: &Path) -> String {
    path.file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "worktree".into())
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect()
}

fn short_oid(oid: &Oid) -> String {
    oid.to_hex()[..7].to_string()
}

fn absolutize(path: &Path) -> CliResult<PathBuf> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(std::env::current_dir()?.join(path))
    }
}

fn path_to_unix(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn strip_verbatim(path: PathBuf) -> PathBuf {
    let s = path.to_string_lossy();
    if let Some(rest) = s.strip_prefix(r"\\?\") {
        PathBuf::from(rest)
    } else {
        path
    }
}

fn pathdiff_to(from_dir: &Path, to: &Path) -> CliResult<PathBuf> {
    // Prefer relative path from from_dir to to when both share a root.
    let from_abs = fs::canonicalize(from_dir).unwrap_or_else(|_| from_dir.to_path_buf());
    let to_abs = fs::canonicalize(to).unwrap_or_else(|_| to.to_path_buf());
    pathdiff(&from_abs, &to_abs).ok_or_else(|| {
        CliError::Msg(format!(
            "cannot relativize {} from {}",
            to_abs.display(),
            from_abs.display()
        ))
    })
}

fn pathdiff(from_dir: &Path, to: &Path) -> Option<PathBuf> {
    let from_comps: Vec<_> = from_dir.components().collect();
    let to_comps: Vec<_> = to.components().collect();
    let mut i = 0;
    while i < from_comps.len() && i < to_comps.len() && from_comps[i] == to_comps[i] {
        i += 1;
    }
    let mut out = PathBuf::new();
    for _ in i..from_comps.len() {
        out.push("..");
    }
    for c in &to_comps[i..] {
        out.push(c.as_os_str());
    }
    if out.as_os_str().is_empty() {
        out.push(".");
    }
    Some(out)
}
