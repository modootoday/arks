//! Working-tree / index path ops: restore, reset, diff, rm, mv, clean.

use crate::cmd::branch::restore_paths;
use crate::cmd::shared::{
    collect_untracked, flatten_tree, head_blob_map, resolve_rev, tree_blob_map,
};
use crate::error::{CliError, CliResult};
use crate::util::{
    FORMAT, open_repo, read_index, refuse_unknown_flags, rel_path_from_worktree, work_tree,
    write_index,
};
use arks_index::{Index, IndexEntry, sort_entries};
use arks_object::{Commit, ObjectKind, RawObject};
use arks_oid::Oid;
use arks_refs::RefTarget;
use arks_repo::Repository;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{self, Write};
use std::path::Path;
pub fn cmd_restore(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &["--staged", "--worktree", "--source"])?;
    let staged = args.iter().any(|a| a == "--staged");
    let worktree = args.iter().any(|a| a == "--worktree") || !staged;
    let mut source: Option<String> = None;
    let mut paths = Vec::new();
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--source" {
            i += 1;
            source = Some(
                args.get(i)
                    .ok_or_else(|| CliError::Usage("restore: --source <tree-ish>".into()))?
                    .clone(),
            );
        } else if !args[i].starts_with('-') {
            paths.push(&args[i]);
        }
        i += 1;
    }
    if paths.is_empty() {
        return Err(CliError::Usage("restore: path required".into()));
    }
    restore_paths(&paths, staged, worktree, source.as_deref())
}

pub fn cmd_reset(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &["--soft", "--mixed", "--hard"])?;
    let mode = if args.iter().any(|a| a == "--soft") {
        "soft"
    } else if args.iter().any(|a| a == "--hard") {
        "hard"
    } else {
        "mixed"
    };
    let rev = args
        .iter()
        .find(|a| !a.starts_with('-'))
        .map(String::as_str)
        .unwrap_or("HEAD");
    let mut repo = open_repo()?;
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
    let target = resolve_rev(&repo, rev)?;
    match repo.head_target()? {
        RefTarget::Symbolic(name) => {
            let old = repo.head_oid().ok();
            repo.refs_mut().update(&name, &target, old.as_ref())?;
        }
        RefTarget::Direct(_) => {
            return Err(CliError::NotImplemented(
                "reset: detached HEAD not claimed in Wave Pa".into(),
            ));
        }
    }
    if mode == "soft" {
        return Ok(());
    }
    let obj = repo.get_object(&target)?;
    let c = Commit::decode(&obj.payload)?;
    if mode == "mixed" || mode == "hard" {
        // Rebuild index from tree without necessarily rewriting worktree for mixed.
        let mut map = HashMap::new();
        flatten_tree(&repo, &c.tree, "", &mut map)?;
        let mut entries = Vec::new();
        for (path, oid) in &map {
            let blob = repo.get_object(oid)?;
            entries.push(IndexEntry {
                ctime_sec: 0,
                ctime_nsec: 0,
                mtime_sec: 0,
                mtime_nsec: 0,
                dev: 0,
                ino: 0,
                mode: 0o100644,
                uid: 0,
                gid: 0,
                size: blob.payload.len().min(u32::MAX as usize) as u32,
                oid: *oid,
                flags: (path.len().min(0x0fff)) as u16,
                flags_extended: None,
                path: path.clone(),
            });
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
    }
    if mode == "hard" {
        let work = work_tree(&repo)?;
        arks_remote::checkout_commit(&repo, &work, &target)?;
    }
    Ok(())
}

pub fn cmd_diff(args: &[String]) -> CliResult<()> {
    // M16 Sy-1: `--stat`, `--name-only`, `--name-status` claimed (git-diff).
    refuse_unknown_flags(
        args,
        &[
            "--cached",
            "--staged",
            "--stat",
            "--name-only",
            "--name-status",
        ],
    )?;
    let cached = args.iter().any(|a| a == "--cached" || a == "--staged");
    let name_only = args.iter().any(|a| a == "--name-only");
    let name_status = args.iter().any(|a| a == "--name-status");
    let stat = args.iter().any(|a| a == "--stat");
    let positional: Vec<&String> = args.iter().filter(|a| !a.starts_with('-')).collect();
    let repo = open_repo()?;
    let work = work_tree(&repo)?;
    if positional.len() == 2 {
        let a = tree_blob_map(&repo, &resolve_rev(&repo, positional[0])?)?;
        let b = tree_blob_map(&repo, &resolve_rev(&repo, positional[1])?)?;
        if name_only || name_status || stat {
            return print_maps_summary(&a, &b, name_only, name_status, stat);
        }
        return print_maps_diff(&repo, &a, &b);
    }
    if !positional.is_empty() {
        return Err(CliError::Usage(
            "diff [<commit> <commit>] | diff [--cached] [--stat|--name-only|--name-status]".into(),
        ));
    }
    let index = read_index(&repo)?;
    if cached {
        let head = head_blob_map(&repo)?;
        let mut idx_map = HashMap::new();
        for e in index.entries.iter().filter(|e| e.stage() == 0) {
            idx_map.insert(e.path.clone(), e.oid);
        }
        if name_only || name_status || stat {
            return print_maps_summary(&head, &idx_map, name_only, name_status, stat);
        }
        return print_maps_diff(&repo, &head, &idx_map);
    }
    // WT vs index — Pf4-1: buffer unified emit (PERF stdout/syscall class).
    let mut out = io::BufWriter::with_capacity(256 * 1024, io::stdout().lock());
    let mut changed: Vec<(String, &'static str)> = Vec::new();
    for e in index.entries.iter().filter(|e| e.stage() == 0) {
        let abs = work.join(&e.path);
        if !abs.is_file() {
            if name_only || name_status || stat {
                changed.push((e.path.clone(), "D"));
            } else {
                let old = repo.get_object(&e.oid)?;
                print_unified_to(&mut out, &e.path, &old.payload, b"")?;
            }
            continue;
        }
        let meta = fs::metadata(&abs)?;
        // Stat short-circuit: unchanged blob, no re-hash / re-read (M17 Pf3-2).
        if crate::cmd::shared::index_stat_matches(e, &meta) {
            continue;
        }
        let size = meta.len().min(u32::MAX as u64) as u32;
        let new_obj = RawObject::new(ObjectKind::Blob, fs::read(&abs)?);
        // Size mismatch ⇒ content changed — skip SHA1 equality check (M17 Pf3-2).
        if size == e.size {
            let new_oid = new_obj.oid(FORMAT)?;
            if new_oid == e.oid {
                continue;
            }
        }
        if name_only || name_status || stat {
            changed.push((e.path.clone(), "M"));
        } else {
            let old = repo.get_object(&e.oid)?;
            print_unified_to(&mut out, &e.path, &old.payload, &new_obj.payload)?;
        }
    }
    if name_only || name_status || stat {
        for (path, status) in &changed {
            if name_only {
                writeln!(out, "{path}")?;
            } else if name_status {
                writeln!(out, "{status}\t{path}")?;
            } else {
                writeln!(out, " {path} | 1 +")?;
            }
        }
        if stat && !changed.is_empty() {
            writeln!(out, " {} files changed", changed.len())?;
        }
    }
    out.flush()?;
    Ok(())
}

fn print_maps_summary(
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
    refuse_unknown_flags(args, &["--cached"])?;
    let cached = args.iter().any(|a| a == "--cached");
    let paths: Vec<&String> = args.iter().filter(|a| !a.starts_with('-')).collect();
    if paths.is_empty() {
        return Err(CliError::Usage("rm [--cached] <path>...".into()));
    }
    let repo = open_repo()?;
    let work = work_tree(&repo)?;
    let mut index = read_index(&repo)?;
    for p in paths {
        let rel = rel_path_from_worktree(&work, Path::new(p)).unwrap_or_else(|_| p.clone());
        if !index
            .entries
            .iter()
            .any(|e| e.path == rel && e.stage() == 0)
        {
            return Err(CliError::Msg(format!("pathspec '{rel}' did not match")));
        }
        index.entries.retain(|e| !(e.path == rel && e.stage() == 0));
        if !cached {
            let abs = work.join(&rel);
            if abs.is_file() {
                fs::remove_file(&abs)?;
            }
        }
    }
    write_index(&repo, &index)?;
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
    // M16 Sy-1: `-x` claimed (ignore exclude rules). Without a gitignore engine yet,
    // untracked collection already includes all non-index paths — `-x` is accepted
    // and uses the same removal path (git-clean). Cite: https://git-scm.com/docs/git-clean
    refuse_unknown_flags(args, &["-f", "-d", "-x"])?;
    let _ignore_excludes = args.iter().any(|a| a == "-x");
    let remove_dirs = args.iter().any(|a| a == "-d");
    if !args.iter().any(|a| a == "-f") {
        return Err(CliError::Usage("clean: -f required".into()));
    }
    let repo = open_repo()?;
    let work = work_tree(&repo)?;
    let index = read_index(&repo)?;
    let indexed: HashSet<_> = index.entries.iter().map(|e| e.path.clone()).collect();
    let mut untracked = Vec::new();
    collect_untracked(&work, &work, &indexed, &mut untracked)?;
    for p in &untracked {
        let abs = work.join(p);
        if abs.is_file() {
            fs::remove_file(&abs)?;
            println!("Removing {p}");
        }
    }
    if remove_dirs {
        remove_untracked_dirs(&work, &work, &indexed)?;
    }
    Ok(())
}

fn remove_untracked_dirs(
    work: &std::path::Path,
    dir: &std::path::Path,
    indexed: &HashSet<String>,
) -> CliResult<()> {
    let entries: Vec<_> = fs::read_dir(dir)?.collect();
    for ent in entries {
        let ent = ent?;
        let name = ent.file_name();
        if name == ".git" {
            continue;
        }
        let path = ent.path();
        if !path.is_dir() {
            continue;
        }
        let rel = path
            .strip_prefix(work)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        // If any indexed path is under this directory, recurse instead of deleting.
        let has_tracked = indexed
            .iter()
            .any(|p| p == &rel || p.starts_with(&format!("{rel}/")));
        if has_tracked {
            remove_untracked_dirs(work, &path, indexed)?;
            continue;
        }
        // Empty of tracked content: remove directory tree if it has no tracked files.
        fs::remove_dir_all(&path)?;
        println!("Removing {rel}/");
    }
    Ok(())
}

fn print_maps_diff(
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
            Some(o) => repo.get_object(o)?.payload,
            None => Vec::new(),
        };
        let bp = match bo {
            Some(o) => repo.get_object(o)?.payload,
            None => Vec::new(),
        };
        print_unified_to(&mut out, k, &ap, &bp)?;
    }
    out.flush()?;
    Ok(())
}

fn print_unified_to(out: &mut impl Write, path: &str, old: &[u8], new: &[u8]) -> CliResult<()> {
    writeln!(out, "diff --git a/{path} b/{path}")?;
    writeln!(out, "--- a/{path}")?;
    writeln!(out, "+++ b/{path}")?;
    let old_s = String::from_utf8_lossy(old);
    let new_s = String::from_utf8_lossy(new);
    for line in old_s.lines() {
        writeln!(out, "-{line}")?;
    }
    for line in new_s.lines() {
        writeln!(out, "+{line}")?;
    }
    Ok(())
}
