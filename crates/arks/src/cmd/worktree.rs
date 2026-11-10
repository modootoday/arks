//! Linked worktrees (add/list/remove/prune/move).
//!
//! Cite: https://git-scm.com/docs/git-worktree
//! Cite: https://git-scm.com/docs/gitrepository-layout
//! Plan: `.spec/plan/20261109022956-worktree-submodule-residual.plan.md`

use crate::cmd::shared::resolve_rev;
use crate::error::{CliError, CliResult};
use crate::util::{open_repo, refuse_unknown_flags, work_tree};
use arks_oid::Oid;
use arks_refs::RefTarget;
use arks_repo::Repository;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

pub fn cmd_worktree(args: &[String]) -> CliResult<()> {
    let sub = args
        .first()
        .map(String::as_str)
        .ok_or_else(|| CliError::Usage(
            "worktree <add|list|remove|prune|move|lock|unlock>".into(),
        ))?;
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
        other => Err(CliError::NotImplemented(format!(
            "worktree {other} not claimed"
        ))),
    }
}

fn wt_add(args: &[String]) -> CliResult<()> {
    // M16 Sy-2: `--detach` claimed; `-B`/`-f`/`--orphan` remain refuse. Cite: git-worktree 2.39.0
    refuse_unknown_flags(
        args,
        &["-b", "-B", "-f", "--force", "--detach", "-d", "--orphan"],
    )?;
    if args
        .iter()
        .any(|a| a == "--orphan" || a == "-f" || a == "--force" || a == "-B")
    {
        return Err(CliError::NotImplemented(
            "worktree add: --orphan/-f/-B not claimed".into(),
        ));
    }
    let mut new_branch: Option<String> = None;
    let mut detach = false;
    let mut positional = Vec::new();
    let mut i = 0;
    while i < args.len() {
        if args[i] == "-b" {
            i += 1;
            new_branch = Some(
                args.get(i)
                    .ok_or_else(|| CliError::Usage("worktree add -b <branch> <path>".into()))?
                    .clone(),
            );
        } else if args[i] == "--detach" || args[i] == "-d" {
            detach = true;
        } else if !args[i].starts_with('-') {
            positional.push(args[i].clone());
        } else {
            return Err(CliError::NotImplemented(format!(
                "unsupported option: {}",
                args[i]
            )));
        }
        i += 1;
    }
    if detach && new_branch.is_some() {
        return Err(CliError::Usage(
            "worktree add: -b and --detach are mutually exclusive".into(),
        ));
    }
    if positional.is_empty() {
        return Err(CliError::Usage(
            "worktree add [-b <branch>|--detach] <path> [<commit-ish>]".into(),
        ));
    }
    let path = PathBuf::from(&positional[0]);
    let commit_ish = positional.get(1).map(String::as_str);

    let mut repo = open_repo()?;
    let common = repo.common_dir().to_path_buf();
    let abs_path = absolutize(&path)?;
    if abs_path.exists() {
        return Err(CliError::Msg(format!(
            "'{}' already exists",
            abs_path.display()
        )));
    }

    let oid = match commit_ish {
        Some(rev) => resolve_rev(&repo, rev)?,
        None => repo.head_oid()?,
    };

    let head_payload = if detach {
        format!("{}\n", oid.to_hex())
    } else if let Some(name) = &new_branch {
        let full = branch_ref(name);
        if repo.refs().read(&full).is_ok() {
            return Err(CliError::Msg(format!(
                "a branch named '{name}' already exists"
            )));
        }
        repo.refs_mut().update(&full, &oid, None)?;
        // Fail-closed: branch already checked out elsewhere.
        for wt in list_worktrees(&repo)? {
            if wt.branch.as_deref() == Some(full.as_str()) {
                return Err(CliError::Msg(format!(
                    "'{name}' is already checked out at '{}'",
                    wt.path.display()
                )));
            }
        }
        format!("ref: {full}\n")
    } else if let Some(rev) = commit_ish {
        // Treat commit-ish as an existing branch when it resolves as refs/heads/*.
        let full = branch_ref(rev);
        match repo.refs().resolve(&full) {
            Ok(resolved) if resolved == oid => {
                for wt in list_worktrees(&repo)? {
                    if wt.branch.as_deref() == Some(full.as_str()) {
                        return Err(CliError::Msg(format!(
                            "'{}' is already checked out at '{}'",
                            full.strip_prefix("refs/heads/").unwrap_or(&full),
                            wt.path.display()
                        )));
                    }
                }
                format!("ref: {full}\n")
            }
            Ok(_) | Err(_) => {
                return Err(CliError::Usage(
                    "worktree add: detached commit-ish requires -b <branch> or --detach".into(),
                ));
            }
        }
    } else {
        match repo.head_target()? {
            RefTarget::Symbolic(name) if name.starts_with("refs/heads/") => {
                for wt in list_worktrees(&repo)? {
                    if wt.branch.as_deref() == Some(name.as_str()) {
                        return Err(CliError::Msg(format!(
                            "'{}' is already checked out at '{}'",
                            name.strip_prefix("refs/heads/").unwrap_or(&name),
                            wt.path.display()
                        )));
                    }
                }
                format!("ref: {name}\n")
            }
            _ => {
                return Err(CliError::Usage(
                    "worktree add: detached HEAD requires -b <branch> or --detach".into(),
                ));
            }
        }
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

    let linked = Repository::open(&admin)?;
    arks_remote::checkout_commit(&linked, &abs_path, &oid)
        .map_err(|e| CliError::Msg(e.to_string()))?;
    if detach {
        println!(
            "Preparing worktree (detached HEAD {})",
            &oid.to_hex()[..7]
        );
    } else {
        let label = head_payload
            .trim()
            .strip_prefix("ref: refs/heads/")
            .or_else(|| head_payload.trim().strip_prefix("ref: "))
            .unwrap_or(head_payload.trim());
        println!("Preparing worktree (checking out '{label}')");
    }
    Ok(())
}

fn wt_repair(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &[])?;
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
    println!("worktree repair: no stale gitdir pointers detected");
    Ok(())
}

fn wt_list(args: &[String]) -> CliResult<()> {
    // M16 Sy-2: `--porcelain` / `-v` claimed; `-z` remains refuse.
    // Cite: https://git-scm.com/docs/git-worktree/2.39.0
    refuse_unknown_flags(args, &["-v", "--verbose", "--porcelain", "-z"])?;
    if args.iter().any(|a| a == "-z") {
        return Err(CliError::NotImplemented(
            "worktree list: -z not claimed".into(),
        ));
    }
    let porcelain = args.iter().any(|a| a == "--porcelain");
    let verbose = args.iter().any(|a| a == "-v" || a == "--verbose");
    let repo = open_repo()?;
    let trees = list_worktrees(&repo)?;
    if porcelain {
        for wt in trees {
            println!("worktree {}", path_to_unix(&wt.path));
            println!("HEAD {}", wt.oid.to_hex());
            if let Some(b) = &wt.branch {
                println!("branch {b}");
            } else {
                println!("detached");
            }
            if wt.locked {
                if let Some(reason) = &wt.lock_reason {
                    let reason = reason.trim();
                    if reason.is_empty() {
                        println!("locked");
                    } else {
                        println!("locked {reason}");
                    }
                } else {
                    println!("locked");
                }
            }
            println!();
        }
        return Ok(());
    }
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
                        println!("{}\t{} {}", wt.path.display(), short, trail);
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
        println!("{}\t{} {}{}", wt.path.display(), short, trail, lock_same_line);
    }
    Ok(())
}

fn wt_lock(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &["--reason"])?;
    let mut reason: Option<String> = None;
    let mut positional = Vec::new();
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--reason" {
            i += 1;
            reason = Some(
                args.get(i)
                    .ok_or_else(|| CliError::Usage("worktree lock --reason <string> <worktree>".into()))?
                    .clone(),
            );
        } else if !args[i].starts_with('-') {
            positional.push(args[i].clone());
        } else {
            return Err(CliError::NotImplemented(format!(
                "unsupported option: {}",
                args[i]
            )));
        }
        i += 1;
    }
    let path_arg = positional
        .first()
        .ok_or_else(|| CliError::Usage("worktree lock [--reason <string>] <worktree>".into()))?;
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
    refuse_unknown_flags(args, &[])?;
    let path_arg = args
        .iter()
        .find(|a| !a.starts_with('-'))
        .ok_or_else(|| CliError::Usage("worktree unlock <worktree>".into()))?;
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
    refuse_unknown_flags(args, &["-f", "--force"])?;
    let force = args.iter().any(|a| a == "-f" || a == "--force");
    let path_arg = args
        .iter()
        .find(|a| !a.starts_with('-'))
        .ok_or_else(|| CliError::Usage("worktree remove [-f] <worktree>".into()))?;
    let repo = open_repo()?;
    let target = find_worktree(&repo, path_arg)?;
    if target.is_main {
        return Err(CliError::Msg("cannot remove the main working tree".into()));
    }
    if target.locked {
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
    refuse_unknown_flags(args, &["-n", "--dry-run", "-v", "--expire"])?;
    if args.iter().any(|a| a.starts_with('-')) {
        return Err(CliError::NotImplemented(
            "worktree prune options not claimed".into(),
        ));
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
            fs::remove_dir_all(&admin)?;
        }
    }
    Ok(())
}

fn wt_move(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &[])?;
    if args.len() != 2 {
        return Err(CliError::Usage(
            "worktree move <worktree> <new-path>".into(),
        ));
    }
    let repo = open_repo()?;
    // SUB-008 (M20 Cx-3): stock refuses move when worktree contains submodules.
    // Cite: https://git-scm.com/docs/git-worktree (move notes)
    if repo.common_dir().join("modules").is_dir()
        || worktree_has_gitmodules(&repo, &args[0])?
    {
        return Err(CliError::Msg(
            "worktree move refused: submodules present (SUB-008 — cannot move worktree containing submodules)".into(),
        ));
    }
    let target = find_worktree(&repo, &args[0])?;
    if target.is_main {
        return Err(CliError::Msg("cannot move the main working tree".into()));
    }
    if target.locked {
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
    Err(CliError::Msg(format!("'{arg}' is not a working tree")))
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
        let digest = arks_hash::hash_object(arks_hash::ObjectFormat::Sha1, "blob", &bytes);
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
