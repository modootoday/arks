//! Working-tree / index path ops: restore, reset, diff, rm, mv, clean.

use crate::cmd::branch::restore_paths;
use crate::cmd::shared::{collect_untracked, head_blob_map, resolve_rev, tree_blob_map};
use crate::error::{CliError, CliResult};
use crate::util::{
    open_repo, read_index, refuse_unknown_flags, rel_path_from_worktree, work_tree, write_index,
};
use arks_index::{Index, IndexEntry, sort_entries};
use arks_object::{ObjectKind, RawObject};
use arks_oid::Oid;
use arks_refs::RefTarget;
use arks_repo::Repository;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{self, Write};
use std::path::Path;
pub fn cmd_restore(args: &[String]) -> CliResult<()> {
    // Cite: https://git-scm.com/docs/git-restore (`-s`/`--source`, `-S`/`--staged`,
    // `-W`/`--worktree`, `--` separator).
    let args = crate::util::split_short_bundles(args, "SWq", "s");
    let mut staged = false;
    let mut wt_flag = false;
    let mut source: Option<String> = None;
    let mut paths = Vec::new();
    let mut i = 0;
    let mut dashdash = false;
    while i < args.len() {
        let a = &args[i];
        i += 1;
        if dashdash || !a.starts_with('-') || a == "-" {
            paths.push(a);
            continue;
        }
        match a.as_str() {
            "--" => dashdash = true,
            "--staged" | "-S" => staged = true,
            "--worktree" | "-W" => wt_flag = true,
            "-q" | "--quiet" => {}
            "--source" | "-s" => {
                source = Some(
                    args.get(i)
                        .ok_or_else(|| CliError::Usage("restore: --source <tree-ish>".into()))?
                        .clone(),
                );
                i += 1;
            }
            x => {
                if let Some(v) = x.strip_prefix("--source=") {
                    source = Some(v.to_string());
                } else {
                    return Err(CliError::NotImplemented(format!("unsupported option: {x}")));
                }
            }
        }
    }
    let worktree = wt_flag || !staged;
    if paths.is_empty() {
        return Err(CliError::Usage("restore: path required".into()));
    }
    restore_paths(&paths, staged, worktree, source.as_deref())
}

pub fn cmd_reset(args: &[String]) -> CliResult<()> {
    // git-reset: `[--soft|--mixed|--hard] [<commit>]` and `[<tree-ish>] [--] <pathspec>...`.
    // Cloud drop-in §O: pathspec form, detached HEAD, tree modes, git output lines.
    let (opts, mut paths) = split_dashdash(args);
    refuse_unknown_flags(&opts, &["--soft", "--mixed", "--hard", "-q", "--quiet"])?;
    let quiet = opts.iter().any(|a| a == "-q" || a == "--quiet");
    let mode = if opts.iter().any(|a| a == "--soft") {
        "soft"
    } else if opts.iter().any(|a| a == "--hard") {
        "hard"
    } else {
        "mixed"
    };
    let mut repo = open_repo()?;
    let positional: Vec<&String> = opts.iter().filter(|a| !a.starts_with('-')).collect();
    let mut rev = "HEAD".to_string();
    for (i, p) in positional.iter().enumerate() {
        if i == 0 && resolve_rev(&repo, p).is_ok() {
            rev = (*p).clone();
        } else {
            paths.push((*p).clone());
        }
    }
    // MISC-005 (M19 Cx-2): parallel reset --hard contends on index.lock.
    if mode != "soft" {
        let mut lock = repo.index_path().into_os_string();
        lock.push(".lock");
        if std::path::PathBuf::from(lock).is_file() {
            return Err(CliError::Msg(
                "Unable to create index.lock (MISC-005 — parallel reset honors exclusive lock; no skip)"
                    .into(),
            ));
        }
    }
    let target = resolve_rev(&repo, &rev)?;
    let target_files = crate::cmd::switch_tree::commit_files(&repo, &target)?;
    let work = work_tree(&repo)?;
    if !paths.is_empty() {
        if mode != "mixed" {
            return Err(CliError::Msg(format!("Cannot do {mode} reset with paths.")));
        }
        let specs: Vec<String> = paths.iter().map(|p| pathspec(&work, p)).collect();
        let mut index = read_index(&repo)?;
        let mut kept: Vec<IndexEntry> = index
            .entries
            .drain(..)
            .filter(|e| !path_matches(&e.path, &specs))
            .collect();
        for (path, f) in &target_files {
            if path_matches(path, &specs) {
                kept.push(index_entry_for(&repo, path, f.oid, f.mode)?);
            }
        }
        sort_entries(&mut kept);
        index.entries = kept;
        index.extensions.clear();
        write_index(&repo, &index)?;
        if !quiet {
            print_unstaged(&repo, &work)?;
        }
        return Ok(());
    }
    let prior_paths: Vec<String> = read_index(&repo)
        .map(|i| i.entries.into_iter().map(|e| e.path).collect())
        .unwrap_or_default();
    let old = repo.head_oid().ok();
    match repo.head_target()? {
        RefTarget::Symbolic(name) => {
            repo.refs_mut().update(&name, &target, old.as_ref())?;
        }
        RefTarget::Direct(_) => {
            repo.refs_mut().update("HEAD", &target, old.as_ref())?;
        }
    }
    if let Some(o) = old {
        let _ = repo.refs_mut().update("ORIG_HEAD", &o, None);
    }
    if mode == "soft" {
        return Ok(());
    }
    if mode == "hard" {
        reset_tree_paths(&repo, &work, &target, &prior_paths)?;
        if !quiet {
            let subject = crate::cmd::log_format::read_commit(&repo, &target)
                .map(|c| c.subject().to_string())
                .unwrap_or_default();
            println!("HEAD is now at {} {subject}", &target.to_hex()[..7]);
        }
        return Ok(());
    }
    // mixed: index from target tree (modes kept); worktree untouched.
    let mut entries = Vec::new();
    for (path, f) in &target_files {
        entries.push(index_entry_for(&repo, path, f.oid, f.mode)?);
    }
    sort_entries(&mut entries);
    write_index(
        &repo,
        &Index {
            version: 2,
            entries,
            extensions: Vec::new(),
        },
    )?;
    if !quiet {
        print_unstaged(&repo, &work)?;
    }
    Ok(())
}

/// Make the index and working tree match `target` (the tree half of
/// `reset --hard`): tracked paths absent from `target` are removed, every
/// tracked file is rewritten, untracked files are left alone.
pub(crate) fn hard_reset_tree(repo: &Repository, work: &Path, target: &Oid) -> CliResult<()> {
    let prior_paths: Vec<String> = read_index(repo)
        .map(|i| i.entries.into_iter().map(|e| e.path).collect())
        .unwrap_or_default();
    reset_tree_paths(repo, work, target, &prior_paths)
}

fn reset_tree_paths(
    repo: &Repository,
    work: &Path,
    target: &Oid,
    prior_paths: &[String],
) -> CliResult<()> {
    let target_files = crate::cmd::switch_tree::commit_files(repo, target)?;
    // Prior tracked paths absent from the target are removed (incl. staged new files).
    for p in prior_paths {
        if !target_files.contains_key(p) {
            let abs = work.join(p);
            if abs.is_file() || abs.is_symlink() {
                let _ = fs::remove_file(&abs);
            }
        }
    }
    // Zero-stat index forces a rewrite of every tracked file below.
    let mut entries = Vec::new();
    for (path, f) in &target_files {
        let mut e = index_entry_for(repo, path, f.oid, f.mode)?;
        e.size = u32::MAX;
        entries.push(e);
    }
    sort_entries(&mut entries);
    write_index(
        repo,
        &Index {
            version: 2,
            entries,
            extensions: Vec::new(),
        },
    )?;
    arks_remote::checkout_commit(repo, work, target)?;
    Ok(())
}

/// Index entry for a working-tree file whose blob is `oid`, with stat data
/// from `meta` (so later status reads take the stat fast path).
pub(crate) fn stat_entry(path: &str, oid: Oid, meta: &fs::Metadata) -> IndexEntry {
    let (ctime_sec, ctime_nsec, mtime_sec, mtime_nsec) = crate::cmd::shared::file_times(meta);
    #[cfg(unix)]
    let (dev, ino, uid, gid) = {
        use std::os::unix::fs::MetadataExt;
        (meta.dev() as u32, meta.ino() as u32, meta.uid(), meta.gid())
    };
    #[cfg(not(unix))]
    let (dev, ino, uid, gid) = (0, 0, 0, 0);
    IndexEntry {
        ctime_sec,
        ctime_nsec,
        mtime_sec,
        mtime_nsec,
        dev,
        ino,
        mode: crate::cmd::shared::file_mode(meta),
        uid,
        gid,
        size: meta.len().min(u32::MAX as u64) as u32,
        oid,
        flags: (path.len().min(0x0fff)) as u16,
        flags_extended: None,
        path: path.to_owned(),
    }
}

/// Zero-stat index entry for a tree blob (stat refresh happens on later reads).
fn index_entry_for(repo: &Repository, path: &str, oid: Oid, mode: u32) -> CliResult<IndexEntry> {
    let size = repo.get_object(&oid)?.payload.len().min(u32::MAX as usize) as u32;
    Ok(IndexEntry {
        ctime_sec: 0,
        ctime_nsec: 0,
        mtime_sec: 0,
        mtime_nsec: 0,
        dev: 0,
        ino: 0,
        mode,
        uid: 0,
        gid: 0,
        size,
        oid,
        flags: (path.len().min(0x0fff)) as u16,
        flags_extended: None,
        path: path.to_owned(),
    })
}

/// git-reset (mixed) report: `Unstaged changes after reset:` + `M`/`D` lines.
fn print_unstaged(repo: &Repository, work: &Path) -> CliResult<()> {
    let index = read_index(repo)?;
    let mut lines = Vec::new();
    for e in index.entries.iter().filter(|e| e.stage() == 0) {
        let abs = work.join(&e.path);
        match fs::metadata(&abs) {
            Err(_) => lines.push(format!("D\t{}", e.path)),
            Ok(m) => {
                if crate::cmd::shared::index_stat_matches(e, &m) {
                    continue;
                }
                let oid =
                    RawObject::new(ObjectKind::Blob, fs::read(&abs)?).oid(repo.object_format())?;
                if oid != e.oid {
                    lines.push(format!("M\t{}", e.path));
                }
            }
        }
    }
    if !lines.is_empty() {
        println!("Unstaged changes after reset:");
        for l in lines {
            println!("{l}");
        }
    }
    Ok(())
}

pub fn cmd_diff(args: &[String]) -> CliResult<()> {
    // M16 Sy-1: `--stat`, `--name-only`, `--name-status` claimed (git-diff).
    // Cloud drop-in §O: `<rev>`, `A..B`, `A...B`, `--cached <rev>`, `-- <path>`,
    // `--numstat`, `--shortstat`, `-U<n>`, `--exit-code`, `--quiet`.
    // Cite: https://git-scm.com/docs/git-diff
    use crate::cmd::diff_render::{DiffFormat, changes_between, render};
    let (opts, rest) = split_dashdash(args);
    let mut fmt = DiffFormat::default();
    let mut cached = false;
    let mut exit_code = false;
    let mut quiet = false;
    let mut positional = Vec::new();
    for a in &opts {
        if fmt.take_flag(a)? {
            continue;
        }
        match a.as_str() {
            "--cached" | "--staged" => cached = true,
            "--exit-code" => exit_code = true,
            "--quiet" => quiet = true,
            "--no-color" | "--no-ext-diff" => {}
            x if x.starts_with('-') && x.len() > 1 => {
                return Err(CliError::NotImplemented(format!("unsupported option: {x}")));
            }
            _ => positional.push(a.clone()),
        }
    }
    if !(fmt.stat || fmt.numstat || fmt.shortstat || fmt.name_only || fmt.name_status) {
        fmt.patch = true;
    }
    let repo = open_repo()?;
    let work = work_tree(&repo)?;
    let mut revs: Vec<Oid> = Vec::new();
    let mut paths: Vec<String> = rest.iter().map(|p| pathspec(&work, p)).collect();
    for p in &positional {
        if let Some((l, r)) = p.split_once("...") {
            let lo = resolve_rev(&repo, if l.is_empty() { "HEAD" } else { l })?;
            let ro = resolve_rev(&repo, if r.is_empty() { "HEAD" } else { r })?;
            let base = arks_repo::merge_base_oid(&repo, &lo, &ro)?
                .ok_or_else(|| CliError::Msg(format!("{p}: no merge base")))?;
            revs.push(base);
            revs.push(ro);
        } else if let Some((l, r)) = p.split_once("..") {
            revs.push(resolve_rev(&repo, if l.is_empty() { "HEAD" } else { l })?);
            revs.push(resolve_rev(&repo, if r.is_empty() { "HEAD" } else { r })?);
        } else if let Ok(o) = resolve_rev(&repo, p) {
            revs.push(o);
        } else if work.join(p).exists() || std::path::Path::new(p).exists() {
            paths.push(pathspec(&work, p));
        } else {
            return Err(CliError::fatal(format!(
                "ambiguous argument '{p}': unknown revision or path not in the working tree.\n\
                 Use '--' to separate paths from revisions, like this:\n\
                 'git <command> [<revision>...] -- [<file>...]'"
            )));
        }
    }
    let index_map = || -> CliResult<HashMap<String, Oid>> {
        let index = read_index(&repo)?;
        Ok(index
            .entries
            .iter()
            .filter(|e| e.stage() == 0)
            .map(|e| (e.path.clone(), e.oid))
            .collect())
    };
    let (a, b, wt) = match (revs.len(), cached) {
        (0, true) => (head_blob_map(&repo)?, index_map()?, false),
        (0, false) => (index_map()?, worktree_map(&repo, &work)?, true),
        (1, true) => (tree_blob_map(&repo, &revs[0])?, index_map()?, false),
        (1, false) => (
            tree_blob_map(&repo, &revs[0])?,
            worktree_map(&repo, &work)?,
            true,
        ),
        (2, _) => (
            tree_blob_map(&repo, &revs[0])?,
            tree_blob_map(&repo, &revs[1])?,
            false,
        ),
        _ => {
            return Err(CliError::Usage(
                "diff [--cached] [<commit> [<commit>]] [-- <path>...]".into(),
            ));
        }
    };
    let a = filter_paths(a, &paths);
    let b = filter_paths(b, &paths);
    let changes = changes_between(&a, &b, wt.then_some(work.as_path()));
    if !quiet {
        // Pf4-1: buffer unified emit (PERF stdout/syscall class).
        let mut out = io::BufWriter::with_capacity(256 * 1024, io::stdout().lock());
        render(&mut out, &repo, &fmt, &changes)?;
        out.flush()?;
    }
    if (exit_code || quiet) && !changes.is_empty() {
        return Err(CliError::silent(1));
    }
    Ok(())
}

pub(crate) fn print_maps_summary(
    a: &HashMap<String, Oid>,
    b: &HashMap<String, Oid>,
    name_only: bool,
    name_status: bool,
    stat: bool,
) -> CliResult<()> {
    let mut keys: Vec<_> = a.keys().chain(b.keys()).cloned().collect();
    keys.sort();
    keys.dedup();
    let mut n = 0usize;
    for k in keys {
        let left = a.get(&k);
        let right = b.get(&k);
        let status = match (left, right) {
            (Some(x), Some(y)) if x == y => continue,
            (None, Some(_)) => "A",
            (Some(_), None) => "D",
            (Some(_), Some(_)) => "M",
            (None, None) => continue,
        };
        n += 1;
        if name_only {
            println!("{k}");
        } else if name_status {
            println!("{status}\t{k}");
        } else if stat {
            println!(" {k} | 1 +");
        }
    }
    if stat && n > 0 {
        println!(" {n} files changed");
    }
    Ok(())
}

pub fn cmd_rm(args: &[String]) -> CliResult<()> {
    // git-rm: `--cached`, `-r`, `-f`, `-q`, `-n`, directory pathspecs, the
    // up-to-date safety checks, and `rm '<path>'` output per removed entry.
    // Cite: https://git-scm.com/docs/git-rm
    let args = crate::util::split_short_bundles(args, "rfqn", "");
    let (opts, mut paths) = split_dashdash(&args);
    let mut cached = false;
    let mut recursive = false;
    let mut force = false;
    let mut quiet = false;
    let mut dry_run = false;
    for a in &opts {
        match a.as_str() {
            "--cached" => cached = true,
            "-r" => recursive = true,
            "-f" | "--force" => force = true,
            "-q" | "--quiet" => quiet = true,
            "-n" | "--dry-run" => dry_run = true,
            x if x.starts_with('-') && x.len() > 1 => {
                return Err(CliError::NotImplemented(format!("unsupported option: {x}")));
            }
            _ => paths.push(a.clone()),
        }
    }
    if paths.is_empty() {
        return Err(CliError::fatal(
            "No pathspec was given. Which files should I remove?",
        ));
    }
    let repo = open_repo()?;
    let work = work_tree(&repo)?;
    let mut index = read_index(&repo)?;
    let mut targets: Vec<String> = Vec::new();
    for p in &paths {
        let rel = crate::cmd::core::pathspec_rel(&work, p)?;
        let prefix = if rel.is_empty() {
            String::new()
        } else {
            format!("{rel}/")
        };
        let mut hits: Vec<String> = index
            .entries
            .iter()
            .filter(|e| e.path == rel || rel.is_empty() || e.path.starts_with(&prefix))
            .map(|e| e.path.clone())
            .collect();
        hits.dedup();
        if hits.is_empty() {
            return Err(CliError::fatal(format!(
                "pathspec '{p}' did not match any files"
            )));
        }
        if !recursive && !hits.iter().any(|h| *h == rel) {
            return Err(CliError::fatal(format!(
                "not removing '{p}' recursively without -r"
            )));
        }
        targets.extend(hits);
    }
    targets.sort();
    targets.dedup();
    if !force {
        let head = head_blob_map(&repo)?;
        let mut staged_diff = Vec::new();
        let mut local_mod = Vec::new();
        let mut both = Vec::new();
        for t in &targets {
            let Some(e) = index
                .entries
                .iter()
                .find(|e| &e.path == t && e.stage() == 0)
            else {
                continue;
            };
            let abs = work.join(t);
            let wt_differs = abs.is_file() && {
                let meta = fs::metadata(&abs)?;
                !crate::cmd::shared::index_stat_matches(e, &meta)
                    && RawObject::new(ObjectKind::Blob, fs::read(&abs)?)
                        .oid(repo.object_format())?
                        != e.oid
            };
            let idx_differs = head.get(t) != Some(&e.oid);
            if idx_differs && wt_differs {
                both.push(t.clone());
            } else if !cached && idx_differs {
                staged_diff.push(t.clone());
            } else if !cached && wt_differs {
                local_mod.push(t.clone());
            }
        }
        let report = |list: &[String], what: &str, hint: &str| -> Option<String> {
            if list.is_empty() {
                return None;
            }
            let (one, many) = (
                format!("the following file {what}"),
                format!("the following files {what}"),
            );
            let mut m = format!("error: {}:\n", if list.len() == 1 { one } else { many });
            for p in list {
                m.push_str(&format!("    {p}\n"));
            }
            m.push_str(hint);
            Some(m)
        };
        let msgs: Vec<String> = [
            report(
                &both,
                &format!(
                    "{} staged content different from both the\nfile and the HEAD",
                    if both.len() == 1 { "has" } else { "have" }
                ),
                "(use -f to force removal)",
            ),
            report(
                &staged_diff,
                &format!(
                    "{} changes staged in the index",
                    if staged_diff.len() == 1 {
                        "has"
                    } else {
                        "have"
                    }
                ),
                "(use --cached to keep the file, or -f to force removal)",
            ),
            report(
                &local_mod,
                &format!(
                    "{} local modifications",
                    if local_mod.len() == 1 { "has" } else { "have" }
                ),
                "(use --cached to keep the file, or -f to force removal)",
            ),
        ]
        .into_iter()
        .flatten()
        .collect();
        if !msgs.is_empty() {
            return Err(CliError::Exit(1, msgs.join("\n")));
        }
    }
    for t in &targets {
        if !quiet {
            println!("rm '{t}'");
        }
    }
    if dry_run {
        return Ok(());
    }
    index.entries.retain(|e| !targets.contains(&e.path));
    write_index(&repo, &index)?;
    if !cached {
        for t in &targets {
            let abs = work.join(t);
            if abs.is_file() || abs.is_symlink() {
                fs::remove_file(&abs)?;
            }
            // Remove now-empty leading directories (git `remove_dir_recursively` class).
            let mut dir = abs.parent().map(Path::to_path_buf);
            while let Some(d) = dir {
                if d == work || fs::remove_dir(&d).is_err() {
                    break;
                }
                dir = d.parent().map(Path::to_path_buf);
            }
        }
    }
    Ok(())
}

pub fn cmd_mv(args: &[String]) -> CliResult<()> {
    // git-mv(1): `mv <src> <dst>` or `mv <src>... <destination-dir>`.
    refuse_unknown_flags(args, &[])?;
    if args.len() < 2 {
        return Err(CliError::Usage("mv <src> <dst> | mv <src>... <dir>".into()));
    }
    let repo = open_repo()?;
    let work = work_tree(&repo)?;
    let mut index = read_index(&repo)?;
    let last = &args[args.len() - 1];
    let last_abs = {
        let p = Path::new(last);
        if p.is_absolute() {
            p.to_path_buf()
        } else {
            work.join(p)
        }
    };
    let sources: &[String] = if args.len() > 2 {
        if !last_abs.is_dir() {
            return Err(CliError::Usage(
                "mv: destination must be a directory when moving multiple sources".into(),
            ));
        }
        &args[..args.len() - 1]
    } else {
        &args[..1]
    };
    let dest_is_dir = args.len() > 2 || last_abs.is_dir();
    // Destination directory as worktree-relative (must not require the new
    // basename to exist yet — canonicalize would fail before rename).
    let dest_dir_rel = if dest_is_dir {
        let p = Path::new(last);
        if p.is_absolute() {
            Some(rel_path_from_worktree(&work, p)?)
        } else {
            Some(last.replace('\\', "/").trim_end_matches('/').to_string())
        }
    } else {
        None
    };
    for src_arg in sources {
        let src = rel_path_from_worktree(&work, Path::new(src_arg))?;
        let dst = if let Some(dir_rel) = &dest_dir_rel {
            let base = Path::new(&src)
                .file_name()
                .ok_or_else(|| CliError::Msg("mv: bad src".into()))?
                .to_string_lossy();
            format!("{dir_rel}/{base}")
        } else {
            let p = Path::new(last);
            if p.is_absolute() {
                rel_path_from_worktree(&work, p)?
            } else {
                last.replace('\\', "/")
            }
        };
        let abs_src = work.join(&src);
        let abs_dst = work.join(&dst);
        if let Some(parent) = abs_dst.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::rename(&abs_src, &abs_dst)?;
        if let Some(pos) = index
            .entries
            .iter()
            .position(|e| e.path == src && e.stage() == 0)
        {
            let mut e = index.entries.remove(pos);
            e.path = dst.clone();
            e.flags = (dst.len().min(0x0fff)) as u16;
            index.entries.push(e);
        }
    }
    sort_entries(&mut index.entries);
    write_index(&repo, &index)?;
    Ok(())
}

pub fn cmd_clean(args: &[String]) -> CliResult<()> {
    // git-clean: `-n`/`--dry-run`, `-f`, `-d`, `-x`, `-q`, pathspecs; cleans
    // below the current directory; without `-d`, wholly untracked directories
    // are left alone. clean.requireForce defaults to true.
    // Cite: https://git-scm.com/docs/git-clean
    // Ignore rules (gitignore(5)) apply unless `-x`; `-X` removes only ignored files.
    let args = crate::util::split_short_bundles(args, "fndxqX", "");
    let (opts, mut paths) = split_dashdash(&args);
    let mut force = 0;
    let mut dry_run = false;
    let mut dirs = false;
    let mut quiet = false;
    // 0 = honor ignore rules, 1 = `-x` (also remove ignored), 2 = `-X` (only ignored).
    let mut exclude_mode = 0u8;
    for a in &opts {
        match a.as_str() {
            "-f" | "--force" => force += 1,
            "-n" | "--dry-run" => dry_run = true,
            "-d" => dirs = true,
            "-x" => exclude_mode = 1,
            "-X" => exclude_mode = 2,
            "-q" | "--quiet" => quiet = true,
            x if x.starts_with('-') && x.len() > 1 => {
                return Err(CliError::NotImplemented(format!("unsupported option: {x}")));
            }
            _ => paths.push(a.clone()),
        }
    }
    let repo = open_repo()?;
    let cfg = arks_config::ConfigSet::load(Some(repo.common_dir()));
    let require_force = cfg
        .get("clean.requireForce")
        .map(|v| {
            !matches!(
                v.to_ascii_lowercase().as_str(),
                "false" | "no" | "off" | "0"
            )
        })
        .unwrap_or(true);
    if force == 0 && !dry_run && require_force {
        return Err(CliError::fatal(
            "clean.requireForce defaults to true and neither -i, -n, nor -f given; refusing to clean",
        ));
    }
    let work = work_tree(&repo)?;
    let index = read_index(&repo)?;
    let indexed: HashSet<String> = index.entries.iter().map(|e| e.path.clone()).collect();
    let mut untracked = Vec::new();
    if exclude_mode == 0 {
        collect_untracked(&work, &work, &indexed, &mut untracked)?;
    } else {
        crate::cmd::shared::collect_untracked_with(&work, &work, &indexed, &mut untracked, None)?;
        if exclude_mode == 2 {
            let mut kept = Vec::new();
            collect_untracked(&work, &work, &indexed, &mut kept)?;
            let kept: HashSet<String> = kept.into_iter().collect();
            untracked.retain(|p| !kept.contains(p));
        }
    }
    let shown = crate::cmd::core::collapse_untracked_dirs(&untracked, &indexed);
    let canon = |p: &Path| p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
    let cwd_rel = crate::util::cwd()
        .ok()
        .and_then(|c| {
            canon(&c)
                .strip_prefix(canon(&work))
                .ok()
                .map(|r| r.to_string_lossy().replace('\\', "/"))
        })
        .unwrap_or_default();
    let mut specs: Vec<String> = paths
        .iter()
        .map(|p| crate::cmd::core::pathspec_rel(&work, p))
        .collect::<CliResult<_>>()?;
    if specs.is_empty() {
        specs.push(cwd_rel.clone());
    }
    let in_spec = |p: &str| {
        let p = p.trim_end_matches('/');
        specs
            .iter()
            .any(|s| s.is_empty() || p == s || p.starts_with(&format!("{s}/")))
    };
    let display = |p: &str| -> String {
        if cwd_rel.is_empty() {
            return p.to_string();
        }
        let pre: Vec<&str> = cwd_rel.split('/').collect();
        let comps: Vec<&str> = p.split('/').collect();
        let mut common = 0;
        while common < pre.len() && common + 1 < comps.len() && pre[common] == comps[common] {
            common += 1;
        }
        format!(
            "{}{}",
            "../".repeat(pre.len() - common),
            comps[common..].join("/")
        )
    };
    for p in shown {
        let is_dir = p.ends_with('/');
        if (is_dir && !dirs) || !in_spec(&p) {
            continue;
        }
        if dry_run {
            if !quiet {
                println!("Would remove {}", display(&p));
            }
            continue;
        }
        let abs = work.join(p.trim_end_matches('/'));
        if is_dir {
            fs::remove_dir_all(&abs)?;
        } else {
            fs::remove_file(&abs)?;
        }
        if !quiet {
            println!("Removing {}", display(&p));
        }
    }
    Ok(())
}

/// Split argv at the first `--` (options+revs, pathspecs).
fn split_dashdash(args: &[String]) -> (Vec<String>, Vec<String>) {
    match args.iter().position(|a| a == "--") {
        Some(i) => (args[..i].to_vec(), args[i + 1..].to_vec()),
        None => (args.to_vec(), Vec::new()),
    }
}

/// Worktree-relative pathspec (cwd-relative input), `/`-separated, no trailing slash.
fn pathspec(work: &Path, p: &str) -> String {
    let abs = std::env::current_dir()
        .map(|c| c.join(p))
        .unwrap_or_else(|_| std::path::PathBuf::from(p));
    let rel = abs
        .strip_prefix(work)
        .map(|r| r.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|_| p.replace('\\', "/"));
    let rel = rel
        .trim_start_matches("./")
        .trim_end_matches('/')
        .to_string();
    if rel == "." { String::new() } else { rel }
}

fn path_matches(path: &str, specs: &[String]) -> bool {
    specs.is_empty()
        || specs
            .iter()
            .any(|s| s.is_empty() || path == s || path.starts_with(&format!("{s}/")))
}

fn filter_paths(map: HashMap<String, Oid>, specs: &[String]) -> HashMap<String, Oid> {
    if specs.is_empty() {
        return map;
    }
    map.into_iter()
        .filter(|(k, _)| path_matches(k, specs))
        .collect()
}

/// Tracked paths as they are in the working tree (missing files omitted).
/// Hashes without writing objects; stat-clean entries reuse the index OID.
fn worktree_map(repo: &Repository, work: &Path) -> CliResult<HashMap<String, Oid>> {
    let index = read_index(repo)?;
    let mut out = HashMap::new();
    for e in index.entries.iter().filter(|e| e.stage() == 0) {
        let abs = work.join(&e.path);
        let Ok(meta) = fs::metadata(&abs) else {
            continue;
        };
        if !meta.is_file() {
            continue;
        }
        if crate::cmd::shared::index_stat_matches(e, &meta) {
            out.insert(e.path.clone(), e.oid);
            continue;
        }
        let obj = RawObject::new(ObjectKind::Blob, fs::read(&abs)?);
        out.insert(e.path.clone(), obj.oid(repo.object_format())?);
    }
    Ok(out)
}

pub(crate) fn print_maps_diff(
    repo: &Repository,
    a: &HashMap<String, Oid>,
    b: &HashMap<String, Oid>,
) -> CliResult<()> {
    // Pf4-1: BufWriter cuts syscall tax on bulk `diff --cached` (G19 DIFF hotspot).
    let mut out = io::BufWriter::with_capacity(256 * 1024, io::stdout().lock());
    let mut keys: HashSet<&String> = a.keys().collect();
    keys.extend(b.keys());
    let mut keys: Vec<_> = keys.into_iter().collect();
    keys.sort();
    for k in keys {
        let ao = a.get(k);
        let bo = b.get(k);
        if ao == bo {
            continue;
        }
        let ap = match ao {
            Some(o) => Some(repo.get_object(o)?.payload),
            None => None,
        };
        let bp = match bo {
            Some(o) => Some(repo.get_object(o)?.payload),
            None => None,
        };
        print_unified_to(
            &mut out,
            repo.object_format(),
            k,
            ap.as_deref(),
            bp.as_deref(),
        )?;
    }
    out.flush()?;
    Ok(())
}

fn print_unified_to(
    out: &mut impl Write,
    format: arks_store::ObjectFormat,
    path: &str,
    old: Option<&[u8]>,
    new: Option<&[u8]>,
) -> CliResult<()> {
    crate::cmd::unified_diff::write_file_patch(out, format, path, old, new)
}
