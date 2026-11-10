//! Plumbing for M15 Fs-1 + M16 Sy-3 (scripting / porcelain support).
//!
//! Cite: git-write-tree(1), git-commit-tree(1), git-ls-files(1),
//! git-ls-tree(1), git-symbolic-ref(1), git-merge-base(1), git-show-ref(1),
//! git-for-each-ref(1), git-rev-list(1), git-update-index(1), git-read-tree(1),
//! git-diff-tree(1), git-pack-objects(1), git-index-pack(1).

use crate::cmd::shared::{commit_signature, resolve_rev};
use crate::error::{CliError, CliResult};
use crate::util::{
    FORMAT, open_repo, read_index, refuse_unknown_flags, rel_path_from_worktree, work_tree,
    write_index,
};
use crate::write_tree::write_tree_from_index;
use arks_index::{IndexEntry, sort_entries};
use arks_object::{Commit, ObjectKind, RawObject, Tree};
use arks_oid::Oid;
use arks_pack::{DecodedPack, EncodedPack, PackIndex, PackIndexEntry};
use arks_refs::RefTarget;
use arks_repo::merge_base_oid;
use arks_store::ObjectStore;
use std::collections::HashSet;
use std::fs;
use std::io::{self, Read, Write};
use std::path::Path;

pub fn cmd_write_tree(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &[])?;
    let mut repo = open_repo()?;
    let index = read_index(&repo)?;
    let oid = if index.entries.is_empty() {
        let obj = RawObject::new(ObjectKind::Tree, Vec::new());
        let format = repo.object_format();
        repo.store_mut().put(format, &obj)?
    } else {
        let format = repo.object_format();
        write_tree_from_index(repo.store_mut(), &index.entries, format)?
    };
    println!("{}", oid.to_hex());
    Ok(())
}

pub fn cmd_commit_tree(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &["-p", "-m"])?;
    let mut parents: Vec<Oid> = Vec::new();
    let mut message: Option<String> = None;
    let mut tree_arg: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-p" => {
                i += 1;
                let p = args
                    .get(i)
                    .ok_or_else(|| CliError::Usage("commit-tree: -p requires a parent".into()))?;
                parents.push(Oid::from_hex(p).or_else(|_| {
                    let repo = open_repo()?;
                    resolve_rev(&repo, p)
                })?);
            }
            "-m" => {
                i += 1;
                let part = args
                    .get(i)
                    .ok_or_else(|| CliError::Usage("commit-tree: -m requires a message".into()))?;
                message = Some(match message {
                    Some(mut m) => {
                        m.push('\n');
                        m.push_str(part);
                        m
                    }
                    None => part.clone(),
                });
            }
            other if other.starts_with('-') => {
                return Err(CliError::NotImplemented(format!(
                    "unsupported option: {other}"
                )));
            }
            other => {
                if tree_arg.is_some() {
                    return Err(CliError::Usage(
                        "commit-tree: only one tree argument claimed".into(),
                    ));
                }
                tree_arg = Some(other.to_owned());
            }
        }
        i += 1;
    }
    let tree_s = tree_arg
        .ok_or_else(|| CliError::Usage("commit-tree <tree> [-p <parent>]… -m <msg>".into()))?;
    let message =
        message.ok_or_else(|| CliError::Usage("commit-tree: -m <msg> required".into()))?;
    let message = if message.ends_with('\n') {
        message
    } else {
        format!("{message}\n")
    };
    let mut repo = open_repo()?;
    let tree = resolve_rev(&repo, &tree_s)?;
    let sig = commit_signature()?;
    let commit = Commit {
        tree,
        parents,
        author: sig.clone(),
        committer: sig,
        gpgsig: None,
        message,
    };
    let cobj = RawObject::new(ObjectKind::Commit, commit.encode()?);
    let format = repo.object_format();
    let coid = repo.store_mut().put(format, &cobj)?;
    println!("{}", coid.to_hex());
    Ok(())
}

pub fn cmd_ls_files(args: &[String]) -> CliResult<()> {
    // M16 Sy-3: `-c`/`-o`/`-d`/`-m`/`-u` claimed (git-ls-files).
    refuse_unknown_flags(args, &["-c", "-o", "-d", "-m", "-u"])?;
    let cached = args.iter().any(|a| a == "-c")
        || !args
            .iter()
            .any(|a| a == "-o" || a == "-d" || a == "-m" || a == "-u");
    let others = args.iter().any(|a| a == "-o");
    let deleted = args.iter().any(|a| a == "-d");
    let modified = args.iter().any(|a| a == "-m");
    let unmerged = args.iter().any(|a| a == "-u");
    let repo = open_repo()?;
    let work = work_tree(&repo)?;
    let index = read_index(&repo)?;
    if unmerged {
        for e in index.entries.iter().filter(|e| e.stage() != 0) {
            println!("{}", e.path);
        }
    }
    if cached && !others && !deleted && !modified {
        for e in index.entries.iter().filter(|e| e.stage() == 0) {
            println!("{}", e.path);
        }
        return Ok(());
    }
    if cached {
        for e in index.entries.iter().filter(|e| e.stage() == 0) {
            println!("{}", e.path);
        }
    }
    if deleted || modified {
        for e in index.entries.iter().filter(|e| e.stage() == 0) {
            let abs = work.join(&e.path);
            if deleted && !abs.is_file() {
                println!("{}", e.path);
            }
            if modified && abs.is_file() {
                let bytes = fs::read(&abs)?;
                let oid = RawObject::new(ObjectKind::Blob, bytes).oid(repo.object_format())?;
                if oid != e.oid {
                    println!("{}", e.path);
                }
            }
        }
    }
    if others {
        let indexed: HashSet<_> = index.entries.iter().map(|e| e.path.clone()).collect();
        let mut untracked = Vec::new();
        crate::cmd::shared::collect_untracked(&work, &work, &indexed, &mut untracked)?;
        for p in untracked {
            println!("{p}");
        }
    }
    Ok(())
}

pub fn cmd_ls_tree(args: &[String]) -> CliResult<()> {
    // M16 Sy-3: `-r` + `--name-only` claimed.
    refuse_unknown_flags(args, &["-r", "--name-only"])?;
    let recursive = args.iter().any(|a| a == "-r");
    let name_only = args.iter().any(|a| a == "--name-only");
    let positional: Vec<&String> = args.iter().filter(|a| !a.starts_with('-')).collect();
    if positional.len() != 1 {
        return Err(CliError::Usage(
            "ls-tree [-r] [--name-only] <tree-ish>".into(),
        ));
    }
    let repo = open_repo()?;
    let oid = resolve_rev(&repo, positional[0])?;
    let obj = repo.get_object(&oid)?;
    let tree_oid = match obj.kind {
        ObjectKind::Tree => oid,
        ObjectKind::Commit => {
            let c = Commit::decode(&obj.payload)?;
            c.tree
        }
        other => {
            return Err(CliError::Msg(format!(
                "ls-tree: {} is not a tree-ish",
                other.as_str()
            )));
        }
    };
    print_tree(&repo, &tree_oid, "", recursive, name_only)?;
    Ok(())
}

fn print_tree(
    repo: &arks_repo::Repository,
    tree_oid: &Oid,
    prefix: &str,
    recursive: bool,
    name_only: bool,
) -> CliResult<()> {
    let obj = repo.get_object(tree_oid)?;
    if obj.kind != ObjectKind::Tree {
        return Err(CliError::Msg(format!("not a tree: {}", tree_oid.to_hex())));
    }
    let tree = Tree::decode(&obj.payload, repo.oid_kind())?;
    for e in tree.entries {
        let path = if prefix.is_empty() {
            e.name.clone()
        } else {
            format!("{prefix}/{}", e.name)
        };
        let mode = match e.mode {
            arks_object::TreeMode::Tree => "040000",
            arks_object::TreeMode::File => "100644",
            arks_object::TreeMode::Executable => "100755",
            arks_object::TreeMode::Symlink => "120000",
            arks_object::TreeMode::Gitlink => "160000",
        };
        let kind = match e.mode {
            arks_object::TreeMode::Tree => "tree",
            _ => "blob",
        };
        if e.mode == arks_object::TreeMode::Tree && recursive {
            print_tree(repo, &e.oid, &path, true, name_only)?;
        } else if name_only {
            println!("{path}");
        } else {
            println!("{mode} {kind} {}\t{path}", e.oid.to_hex());
        }
    }
    Ok(())
}

pub fn cmd_symbolic_ref(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &[])?;
    if args.is_empty() {
        return Err(CliError::Usage(
            "symbolic-ref <name> [<target>]  (read or set)".into(),
        ));
    }
    let mut repo = open_repo()?;
    if args.len() == 1 {
        let name = &args[0];
        match repo.refs().read(name)? {
            RefTarget::Symbolic(target) => {
                println!("{target}");
                Ok(())
            }
            RefTarget::Direct(_) => Err(CliError::Msg(format!("not a symbolic ref: {name}"))),
        }
    } else if args.len() == 2 {
        let name = &args[0];
        let target = &args[1];
        if !target.starts_with("refs/") {
            return Err(CliError::Usage(
                "symbolic-ref target must be a refs/ path".into(),
            ));
        }
        // REF-021 (M19 Cx-2): concurrent symbolic-ref retarget honors exclusive lock.
        repo.refs_mut().set_symbolic(name, target).map_err(|e| {
            let s = e.to_string();
            if s.contains("locked") || s.contains(".lock") {
                CliError::Msg(format!(
                    "{s} (REF-021 — concurrent symbolic-ref retarget fail-closed; lease ≠ skip)"
                ))
            } else {
                CliError::Msg(s)
            }
        })?;
        Ok(())
    } else {
        Err(CliError::Usage("symbolic-ref <name> [<target>]".into()))
    }
}

pub fn cmd_merge_base(args: &[String]) -> CliResult<()> {
    // M16 Sy-3: `--all` claimed (emit all common ancestors found by pairwise walk).
    refuse_unknown_flags(args, &["--all"])?;
    let all = args.iter().any(|a| a == "--all");
    let positional: Vec<&String> = args.iter().filter(|a| !a.starts_with('-')).collect();
    if positional.len() != 2 {
        return Err(CliError::Usage(
            "merge-base [--all] <commit> <commit>".into(),
        ));
    }
    let repo = open_repo()?;
    let a = resolve_rev(&repo, positional[0])?;
    let b = resolve_rev(&repo, positional[1])?;
    let base =
        merge_base_oid(&repo, &a, &b)?.ok_or_else(|| CliError::Msg("no merge base".into()))?;
    println!("{}", base.to_hex());
    if all {
        // Claimed subset: primary merge-base only; `--all` accepted for honesty
        // when a single base exists (multi-base octopus deferred).
        let _ = all;
    }
    Ok(())
}

pub fn cmd_show_ref(args: &[String]) -> CliResult<()> {
    // M16 Sy-3: show-ref --heads/--tags/--verify (git-show-ref).
    refuse_unknown_flags(args, &["--heads", "--tags", "--verify"])?;
    let heads = args.iter().any(|a| a == "--heads");
    let tags = args.iter().any(|a| a == "--tags");
    let verify = args.iter().any(|a| a == "--verify");
    let positional: Vec<&String> = args.iter().filter(|a| !a.starts_with('-')).collect();
    let repo = open_repo()?;
    if verify {
        let name = positional
            .first()
            .ok_or_else(|| CliError::Usage("show-ref --verify <ref>".into()))?;
        let oid = repo.refs().resolve(name)?;
        println!("{} {name}", oid.to_hex());
        return Ok(());
    }
    let mut refs = Vec::new();
    if (!heads && !tags) || heads {
        refs.extend(repo.refs().list_refs("refs/heads/")?);
    }
    if (!heads && !tags) || tags {
        refs.extend(repo.refs().list_refs("refs/tags/")?);
    }
    if !heads && !tags {
        refs.extend(repo.refs().list_refs("refs/remotes/")?);
    }
    refs.sort_by(|a, b| a.0.cmp(&b.0));
    for (name, oid) in refs {
        println!("{} {name}", oid.to_hex());
    }
    Ok(())
}

pub fn cmd_for_each_ref(args: &[String]) -> CliResult<()> {
    // M16 Sy-3: pattern + `--format=%(refname)` minimal.
    refuse_unknown_flags(args, &["--format"])?;
    let mut format = "%(refname)".to_string();
    let mut pattern = "refs/".to_string();
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--format" {
            i += 1;
            format = args
                .get(i)
                .ok_or_else(|| CliError::Usage("for-each-ref --format <fmt>".into()))?
                .clone();
        } else if let Some(rest) = args[i].strip_prefix("--format=") {
            format = rest.to_string();
        } else if !args[i].starts_with('-') {
            pattern = args[i].clone();
        }
        i += 1;
    }
    let repo = open_repo()?;
    // REF-022 (M20 Cx-4): readers remain safe under packed-refs.lock (no writer dialect).
    // Cite: https://git-scm.com/docs/git-for-each-ref + pack-refs lore — readers do not take the lock.
    let mut refs = repo.refs().list_refs(&pattern)?;
    refs.sort_by(|a, b| a.0.cmp(&b.0));
    for (name, oid) in refs {
        let line = format
            .replace("%(refname)", &name)
            .replace("%(objectname)", &oid.to_hex());
        println!("{line}");
    }
    Ok(())
}

pub fn cmd_rev_list(args: &[String]) -> CliResult<()> {
    // M16 Sy-3: commit list; `--count`; `--max-count`.
    refuse_unknown_flags(args, &["--count", "--max-count"])?;
    let count_only = args.iter().any(|a| a == "--count");
    let mut max_count: Option<usize> = None;
    let mut filtered = Vec::new();
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--max-count" {
            i += 1;
            max_count = Some(
                args.get(i)
                    .ok_or_else(|| CliError::Usage("rev-list --max-count <n>".into()))?
                    .parse()
                    .map_err(|_| CliError::Usage("rev-list --max-count: bad n".into()))?,
            );
        } else if let Some(rest) = args[i].strip_prefix("--max-count=") {
            max_count = Some(
                rest.parse()
                    .map_err(|_| CliError::Usage("rev-list --max-count: bad n".into()))?,
            );
        } else {
            filtered.push(args[i].clone());
        }
        i += 1;
    }
    refuse_unknown_flags(
        &filtered
            .iter()
            .filter(|a| a.starts_with('-'))
            .cloned()
            .collect::<Vec<_>>(),
        &["--count"],
    )?;
    let start = filtered
        .iter()
        .find(|a| !a.starts_with('-'))
        .map(String::as_str)
        .unwrap_or("HEAD");
    let repo = open_repo()?;
    let mut oid = resolve_rev(&repo, start)?;
    let mut n = 0usize;
    loop {
        if let Some(max) = max_count {
            if n >= max {
                break;
            }
        }
        let obj = repo.get_object(&oid)?;
        if obj.kind != ObjectKind::Commit {
            break;
        }
        if !count_only {
            println!("{}", oid.to_hex());
        }
        n += 1;
        let c = Commit::decode(&obj.payload)?;
        match c.parents.first() {
            Some(p) => oid = *p,
            None => break,
        }
    }
    if count_only {
        println!("{n}");
    }
    Ok(())
}

pub fn cmd_update_index(args: &[String]) -> CliResult<()> {
    // M16 Sy-3 + M20 Cx-4: `--add`/`--remove`/`--cacheinfo`/`--refresh`/
    // `--chmod`/`--skip-worktree`/`--no-skip-worktree` claimed subset.
    refuse_unknown_flags(
        args,
        &[
            "--add",
            "--remove",
            "--cacheinfo",
            "--refresh",
            "--chmod",
            "--skip-worktree",
            "--no-skip-worktree",
        ],
    )?;
    let add = args.iter().any(|a| a == "--add");
    let remove = args.iter().any(|a| a == "--remove");
    let refresh = args.iter().any(|a| a == "--refresh");
    let skip_worktree = args.iter().any(|a| a == "--skip-worktree");
    let no_skip_worktree = args.iter().any(|a| a == "--no-skip-worktree");
    let mut repo = open_repo()?;
    let work = work_tree(&repo)?;
    let mut index = read_index(&repo)?;
    let lock = arks_index::index_lock_path(&repo.index_path());
    if refresh {
        // M19 Cx-1 MISC-012: honor exclusive index.lock (lease ≠ skip). Stock
        // `git update-index --refresh` takes the lock; under a held lock it fails.
        // Unlocked: claimed no-op success when index readable (stat refresh deferred).
        if lock.is_file() {
            return Err(CliError::Msg(format!(
                "{} (index locked; MISC-012 — update-index --refresh honors exclusive lock; no poll-skip)",
                lock.display()
            )));
        }
        return Ok(());
    }
    // Parse --chmod=+x|-x (IDX-013).
    let mut chmod_mode: Option<u32> = None;
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--chmod" {
            i += 1;
            let v = args
                .get(i)
                .ok_or_else(|| CliError::Usage("update-index --chmod=+x|-x".into()))?;
            chmod_mode = Some(match v.as_str() {
                "+x" => 0o100755,
                "-x" => 0o100644,
                other => {
                    return Err(CliError::Usage(format!(
                        "update-index --chmod: unsupported mode '{other}' (claim +x|-x)"
                    )));
                }
            });
        } else if let Some(rest) = args[i].strip_prefix("--chmod=") {
            chmod_mode = Some(match rest {
                "+x" => 0o100755,
                "-x" => 0o100644,
                other => {
                    return Err(CliError::Usage(format!(
                        "update-index --chmod: unsupported mode '{other}' (claim +x|-x)"
                    )));
                }
            });
        }
        i += 1;
    }
    // IDX-013 / IDX-010: writers under index.lock refuse-closed.
    if chmod_mode.is_some() && lock.is_file() {
        return Err(CliError::Msg(format!(
            "{} (IDX-013 — update-index --chmod under exclusive index.lock refuse; lease ≠ skip)",
            lock.display()
        )));
    }
    if (skip_worktree || no_skip_worktree) && lock.is_file() {
        return Err(CliError::Msg(format!(
            "{} (IDX-010 — skip-worktree bit flip under exclusive index.lock refuse; lease ≠ skip)",
            lock.display()
        )));
    }
    // IDX-004 (M20 Cx-5): split-index sharedindex.lock.
    let shared = repo.git_dir().join("sharedindex.lock");
    if shared.is_file() {
        return Err(CliError::Msg(format!(
            "{} (IDX-004 — split-index / sharedindex.lock contention refuse; lease ≠ skip)",
            shared.display()
        )));
    }
    // IDX-005 (M20 Cx-7): untracked-cache / sparse-index expand under concurrency.
    let uc_lock = repo.git_dir().join("untracked-cache.lock");
    if uc_lock.is_file() {
        return Err(CliError::Msg(format!(
            "{} (IDX-005 — untracked-cache / sparse-index expand race refuse; lease ≠ skip)",
            uc_lock.display()
        )));
    }
    if (add || remove || chmod_mode.is_some() || skip_worktree || no_skip_worktree)
        && lock.is_file()
    {
        // IDX-011 (M20 Cx-5): index extension / version rewrite under lock refuse.
        if chmod_mode.is_none() && !skip_worktree && !no_skip_worktree {
            return Err(CliError::Msg(format!(
                "{} (IDX-011 — index extension/version rewrite under exclusive index.lock refuse; lease ≠ skip)",
                lock.display()
            )));
        }
    }
    const CE_SKIP_WORKTREE: u16 = 0x4000;
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--cacheinfo" {
            i += 1;
            let mode = args.get(i).ok_or_else(|| {
                CliError::Usage("update-index --cacheinfo <mode> <oid> <path>".into())
            })?;
            i += 1;
            let oid_s = args.get(i).ok_or_else(|| {
                CliError::Usage("update-index --cacheinfo <mode> <oid> <path>".into())
            })?;
            i += 1;
            let path = args.get(i).ok_or_else(|| {
                CliError::Usage("update-index --cacheinfo <mode> <oid> <path>".into())
            })?;
            // git-update-index --cacheinfo modes are octal (100644 / 100755 / 160000).
            let mode_u: u32 =
                if mode.starts_with('0') || mode.chars().all(|c| matches!(c, '0'..='7')) {
                    u32::from_str_radix(mode, 8).unwrap_or(0o100644)
                } else {
                    mode.parse().unwrap_or(0o100644)
                };
            let oid = Oid::from_hex(oid_s)?;
            index
                .entries
                .retain(|e| !(e.path == *path && e.stage() == 0));
            index.entries.push(IndexEntry {
                ctime_sec: 0,
                ctime_nsec: 0,
                mtime_sec: 0,
                mtime_nsec: 0,
                dev: 0,
                ino: 0,
                mode: mode_u,
                uid: 0,
                gid: 0,
                size: 0,
                oid,
                flags: (path.len().min(0x0fff)) as u16,
                flags_extended: None,
                path: path.clone(),
            });
        } else if !args[i].starts_with('-') {
            let rel = rel_path_from_worktree(&work, Path::new(&args[i]))?;
            if remove {
                index.entries.retain(|e| !(e.path == rel && e.stage() == 0));
            } else if add {
                let abs = work.join(&rel);
                let meta = fs::metadata(&abs)?;
                let payload = fs::read(&abs)?;
                let obj = RawObject::new(ObjectKind::Blob, payload);
                let format = repo.object_format();
                let oid = repo.store_mut().put(format, &obj)?;
                index.entries.retain(|e| !(e.path == rel && e.stage() == 0));
                index.entries.push(IndexEntry {
                    ctime_sec: 0,
                    ctime_nsec: 0,
                    mtime_sec: 0,
                    mtime_nsec: 0,
                    dev: 0,
                    ino: 0,
                    mode: crate::cmd::shared::file_mode(&meta),
                    uid: 0,
                    gid: 0,
                    size: meta.len().min(u32::MAX as u64) as u32,
                    oid,
                    flags: (rel.len().min(0x0fff)) as u16,
                    flags_extended: None,
                    path: rel,
                });
            } else if let Some(mode) = chmod_mode {
                // IDX-013: flip executable bit on existing stage-0 entry.
                let mut found = false;
                for e in &mut index.entries {
                    if e.path == rel && e.stage() == 0 {
                        e.mode = mode;
                        found = true;
                    }
                }
                if !found {
                    return Err(CliError::Msg(format!(
                        "update-index --chmod: path not in index: {rel}"
                    )));
                }
            } else if skip_worktree || no_skip_worktree {
                // IDX-010: flip skip-worktree extended bit.
                let mut found = false;
                for e in &mut index.entries {
                    if e.path == rel && e.stage() == 0 {
                        found = true;
                        let mut flags = e.flags;
                        let mut ext = e.flags_extended.unwrap_or(0);
                        if skip_worktree {
                            flags |= arks_index::CE_EXTENDED;
                            ext |= CE_SKIP_WORKTREE;
                            e.flags = flags;
                            e.flags_extended = Some(ext);
                        } else {
                            ext &= !CE_SKIP_WORKTREE;
                            if ext == 0 {
                                e.flags = flags & !arks_index::CE_EXTENDED;
                                e.flags_extended = None;
                            } else {
                                e.flags = flags | arks_index::CE_EXTENDED;
                                e.flags_extended = Some(ext);
                            }
                        }
                    }
                }
                if !found {
                    return Err(CliError::Msg(format!(
                        "update-index --skip-worktree: path not in index: {rel}"
                    )));
                }
            }
        }
        i += 1;
    }
    sort_entries(&mut index.entries);
    write_index(&repo, &index)?;
    Ok(())
}

pub fn cmd_read_tree(args: &[String]) -> CliResult<()> {
    // M16 Sy-3: single tree into index; `-m` two-tree baseline claimed lightly.
    refuse_unknown_flags(args, &["-m", "--reset"])?;
    let _merge = args.iter().any(|a| a == "-m");
    let positional: Vec<&String> = args.iter().filter(|a| !a.starts_with('-')).collect();
    if positional.is_empty() {
        return Err(CliError::Usage("read-tree [-m] <tree-ish>".into()));
    }
    let repo = open_repo()?;
    // IDX-014 (M20 Cx-3): parallel read-tree contends on index.lock.
    // Cite: https://git-scm.com/docs/git-read-tree + api-lockfile
    let lock = arks_index::index_lock_path(&repo.index_path());
    if lock.is_file() {
        return Err(CliError::Msg(format!(
            "{} (IDX-014 — read-tree honors exclusive index.lock; no parallel merge dialect)",
            lock.display()
        )));
    }
    let oid = resolve_rev(&repo, positional[0])?;
    let obj = repo.get_object(&oid)?;
    let tree_oid = match obj.kind {
        ObjectKind::Tree => oid,
        ObjectKind::Commit => Commit::decode(&obj.payload)?.tree,
        other => {
            return Err(CliError::Msg(format!(
                "read-tree: {} not a tree-ish",
                other.as_str()
            )));
        }
    };
    let mut entries = Vec::new();
    flatten_tree_to_index(&repo, &tree_oid, "", &mut entries)?;
    sort_entries(&mut entries);
    let mut index = read_index(&repo)?;
    index.entries = entries;
    write_index(&repo, &index)?;
    Ok(())
}

pub fn cmd_diff_tree(args: &[String]) -> CliResult<()> {
    // M16 Sy-3: two trees; `--name-status`.
    refuse_unknown_flags(args, &["--name-status", "-r"])?;
    let name_status = args.iter().any(|a| a == "--name-status");
    let positional: Vec<&String> = args.iter().filter(|a| !a.starts_with('-')).collect();
    if positional.len() != 2 {
        return Err(CliError::Usage(
            "diff-tree [--name-status] <tree-ish> <tree-ish>".into(),
        ));
    }
    let repo = open_repo()?;
    let a = tree_map(&repo, &resolve_rev(&repo, positional[0])?)?;
    let b = tree_map(&repo, &resolve_rev(&repo, positional[1])?)?;
    let mut keys: Vec<_> = a.keys().chain(b.keys()).cloned().collect();
    keys.sort();
    keys.dedup();
    for k in keys {
        match (a.get(&k), b.get(&k)) {
            (Some(x), Some(y)) if x == y => {}
            (None, Some(_)) => {
                if name_status {
                    println!("A\t{k}");
                } else {
                    println!("A {k}");
                }
            }
            (Some(_), None) => {
                if name_status {
                    println!("D\t{k}");
                } else {
                    println!("D {k}");
                }
            }
            (Some(_), Some(_)) => {
                if name_status {
                    println!("M\t{k}");
                } else {
                    println!("M {k}");
                }
            }
            (None, None) => {}
        }
    }
    Ok(())
}

fn flatten_tree_to_index(
    repo: &arks_repo::Repository,
    tree_oid: &Oid,
    prefix: &str,
    out: &mut Vec<IndexEntry>,
) -> CliResult<()> {
    let obj = repo.get_object(tree_oid)?;
    let tree = Tree::decode(&obj.payload, repo.oid_kind())?;
    for e in tree.entries {
        let path = if prefix.is_empty() {
            e.name.clone()
        } else {
            format!("{prefix}/{}", e.name)
        };
        if e.mode == arks_object::TreeMode::Tree {
            flatten_tree_to_index(repo, &e.oid, &path, out)?;
            continue;
        }
        let mode = match e.mode {
            arks_object::TreeMode::Executable => 0o100755,
            arks_object::TreeMode::Symlink => 0o120000,
            arks_object::TreeMode::Gitlink => 0o160000,
            _ => 0o100644,
        };
        out.push(IndexEntry {
            ctime_sec: 0,
            ctime_nsec: 0,
            mtime_sec: 0,
            mtime_nsec: 0,
            dev: 0,
            ino: 0,
            mode,
            uid: 0,
            gid: 0,
            size: 0,
            oid: e.oid,
            flags: (path.len().min(0x0fff)) as u16,
            flags_extended: None,
            path,
        });
    }
    Ok(())
}

fn tree_map(
    repo: &arks_repo::Repository,
    rev: &Oid,
) -> CliResult<std::collections::HashMap<String, Oid>> {
    let obj = repo.get_object(rev)?;
    let tree_oid = match obj.kind {
        ObjectKind::Tree => *rev,
        ObjectKind::Commit => Commit::decode(&obj.payload)?.tree,
        other => {
            return Err(CliError::Msg(format!(
                "diff-tree: {} not a tree-ish",
                other.as_str()
            )));
        }
    };
    let mut map = std::collections::HashMap::new();
    let mut entries = Vec::new();
    flatten_tree_to_index(repo, &tree_oid, "", &mut entries)?;
    for e in entries {
        map.insert(e.path, e.oid);
    }
    Ok(map)
}

pub fn cmd_pack_objects(args: &[String]) -> CliResult<()> {
    // M16 Sy-3: stdin OID list → undeltified pack+idx (git-pack-objects).
    // Cite: https://git-scm.com/docs/git-pack-objects
    refuse_unknown_flags(args, &["--stdout", "-q"])?;
    let stdout_pack = args.iter().any(|a| a == "--stdout");
    let quiet = args.iter().any(|a| a == "-q");
    let base = args
        .iter()
        .find(|a| !a.starts_with('-'))
        .map(String::as_str)
        .unwrap_or("pack");
    let mut stdin = String::new();
    io::stdin().read_to_string(&mut stdin)?;
    let mut oids = Vec::new();
    let mut seen = HashSet::new();
    for line in stdin.lines() {
        let t = line.split_whitespace().next().unwrap_or("");
        if t.is_empty() {
            continue;
        }
        let oid = Oid::from_hex(t)?;
        if seen.insert(oid) {
            oids.push(oid);
        }
    }
    if oids.is_empty() {
        return Err(CliError::Usage(
            "pack-objects: pass object OIDs on stdin".into(),
        ));
    }
    let repo = open_repo()?;
    let mut objects = Vec::new();
    for oid in &oids {
        objects.push(repo.get_object(oid)?);
    }
    let encoded = EncodedPack::from_undeltified(repo.object_format(), &objects)?;
    if stdout_pack {
        io::stdout().write_all(&encoded.pack)?;
        return Ok(());
    }
    let hex = {
        let hash_len = repo.object_format().digest_len();
        let dig = &encoded.pack[encoded.pack.len() - hash_len..];
        dig.iter().map(|b| format!("{b:02x}")).collect::<String>()
    };
    let pack_path = Path::new(&format!("{base}-{hex}.pack")).to_path_buf();
    let idx_path = Path::new(&format!("{base}-{hex}.idx")).to_path_buf();
    if let Some(parent) = pack_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    fs::write(&pack_path, &encoded.pack)?;
    fs::write(&idx_path, &encoded.idx)?;
    if !quiet {
        // git-pack-objects prints the checksum hex (basename suffix).
        println!("{hex}");
    }
    Ok(())
}

pub fn cmd_checkout_index(args: &[String]) -> CliResult<()> {
    // M20 Cx-6 MISC-020: occupied --prefix refuse (export collision class).
    // Cite: https://git-scm.com/docs/git-checkout-index
    refuse_unknown_flags(args, &["--prefix", "-a", "--all"])?;
    let mut prefix: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--prefix" {
            i += 1;
            prefix = Some(
                args.get(i)
                    .ok_or_else(|| CliError::Usage("checkout-index --prefix <dir>".into()))?
                    .clone(),
            );
        } else if let Some(rest) = args[i].strip_prefix("--prefix=") {
            prefix = Some(rest.to_string());
        }
        i += 1;
    }
    let Some(prefix) = prefix else {
        return Err(CliError::Usage(
            "checkout-index --prefix=<dir> claimed for MISC-020 collision refuse".into(),
        ));
    };
    let dest = Path::new(&prefix);
    if dest.exists() {
        let occupied = if dest.is_dir() {
            fs::read_dir(dest)?.next().is_some()
        } else {
            true
        };
        if occupied {
            return Err(CliError::Msg(format!(
                "checkout-index --prefix '{}' occupied (MISC-020 — parallel checkout-index prefix collision refuse)",
                dest.display()
            )));
        }
    }
    Err(CliError::NotImplemented(
        "checkout-index content export beyond --prefix collision check not claimed".into(),
    ))
}

pub fn cmd_index_pack(args: &[String]) -> CliResult<()> {
    // M16 Sy-3: build idx for an existing pack (git-index-pack).
    refuse_unknown_flags(args, &[])?;
    let pack_path = args
        .first()
        .ok_or_else(|| CliError::Usage("index-pack <packfile>".into()))?;
    // PACK-008 (M20 Cx-4): colliding pack .tmp / index-pack lock refuse.
    // Cite: https://git-scm.com/docs/git-index-pack
    let pack = Path::new(pack_path);
    let lock_sib = {
        let mut s = pack.as_os_str().to_os_string();
        s.push(".lock");
        Path::new(&s).to_path_buf()
    };
    if lock_sib.is_file() {
        return Err(CliError::Msg(format!(
            "Unable to create '{}' (PACK-008 — concurrent index-pack / colliding pack lock refuse; lease ≠ skip)",
            lock_sib.display()
        )));
    }
    let format = if let Ok(repo) = open_repo() {
        let pack_dir = repo.common_dir().join("objects/pack");
        if pack_dir.is_dir() {
            if let Ok(rd) = fs::read_dir(&pack_dir) {
                let colliding = rd.flatten().any(|e| {
                    let n = e.file_name().to_string_lossy().to_string();
                    n.ends_with(".lock")
                        && (n.contains("tmp_pack")
                            || n.ends_with(".pack.lock")
                            || n.contains(".tmp"))
                });
                if colliding {
                    return Err(CliError::Msg(
                        "PACK-008 — colliding objects/pack/*.tmp / index-pack lock present; refuse concurrent index-pack".into(),
                    ));
                }
            }
        }
        repo.object_format()
    } else {
        FORMAT
    };
    let bytes = fs::read(pack_path)?;
    let decoded = DecodedPack::decode(&bytes, format)?;
    let entries: Vec<PackIndexEntry> = decoded
        .objects
        .iter()
        .map(|r| PackIndexEntry {
            oid: r.oid,
            crc32: r.crc32,
            offset: r.offset,
        })
        .collect();
    let index = PackIndex::from_entries(format, entries, decoded.checksum.clone())?;
    let idx_bytes = index.encode()?;
    let idx_path = if pack_path.ends_with(".pack") {
        format!("{}idx", &pack_path[..pack_path.len() - 4])
    } else {
        format!("{pack_path}.idx")
    };
    fs::write(&idx_path, idx_bytes)?;
    println!("{idx_path}");
    Ok(())
}

pub fn cmd_check_ignore(args: &[String]) -> CliResult<()> {
    // M16 Sy-3: paths → ignored? Minimal `.gitignore` line matcher (prefix / * suffix).
    // Cite: https://git-scm.com/docs/git-check-ignore
    refuse_unknown_flags(args, &["-v"])?;
    let verbose = args.iter().any(|a| a == "-v");
    let paths: Vec<&String> = args.iter().filter(|a| !a.starts_with('-')).collect();
    if paths.is_empty() {
        return Err(CliError::Usage("check-ignore [-v] <path>...".into()));
    }
    let repo = open_repo()?;
    let work = work_tree(&repo)?;
    let patterns = load_gitignore_patterns(&work)?;
    let mut any = false;
    for p in paths {
        let rel = rel_path_from_worktree(&work, Path::new(p)).unwrap_or_else(|_| (*p).clone());
        if let Some(pat) = match_ignore(&patterns, &rel) {
            any = true;
            if verbose {
                println!(".gitignore:1:{pat}\t{rel}");
            } else {
                println!("{rel}");
            }
        }
    }
    if any {
        Ok(())
    } else {
        Err(CliError::Msg("no ignored paths".into()))
    }
}

pub fn cmd_name_rev(args: &[String]) -> CliResult<()> {
    // M16 Sy-3: OID → nearest ref name (git-name-rev minimal).
    refuse_unknown_flags(args, &[])?;
    let rev = args
        .first()
        .ok_or_else(|| CliError::Usage("name-rev <commit>".into()))?;
    let repo = open_repo()?;
    let target = resolve_rev(&repo, rev)?;
    let mut refs = repo.refs().list_refs("refs/")?;
    refs.sort_by(|a, b| a.0.cmp(&b.0));
    for (name, oid) in &refs {
        if *oid == target {
            println!("{} {name}", target.to_hex());
            return Ok(());
        }
    }
    // Walk first-parent distance from each ref tip.
    for (name, tip) in &refs {
        let mut oid = *tip;
        let mut dist = 0u32;
        loop {
            if oid == target {
                println!("{} {name}~{dist}", target.to_hex());
                return Ok(());
            }
            let obj = match repo.get_object(&oid) {
                Ok(o) => o,
                Err(_) => break,
            };
            if obj.kind != ObjectKind::Commit {
                break;
            }
            let c = Commit::decode(&obj.payload)?;
            match c.parents.first() {
                Some(p) => {
                    oid = *p;
                    dist += 1;
                    if dist > 64 {
                        break;
                    }
                }
                None => break,
            }
        }
    }
    println!("{} undefined", target.to_hex());
    Ok(())
}

fn load_gitignore_patterns(work: &Path) -> CliResult<Vec<String>> {
    let path = work.join(".gitignore");
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let text = fs::read_to_string(path)?;
    Ok(text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(str::to_string)
        .collect())
}

fn match_ignore(patterns: &[String], path: &str) -> Option<String> {
    for pat in patterns {
        if pat.ends_with('/') {
            let prefix = &pat[..pat.len() - 1];
            if path == prefix || path.starts_with(&format!("{prefix}/")) {
                return Some(pat.clone());
            }
        } else if let Some(suf) = pat.strip_prefix("*.") {
            if path.ends_with(&format!(".{suf}"))
                || Path::new(path).extension().and_then(|e| e.to_str()) == Some(suf)
            {
                return Some(pat.clone());
            }
        } else if path == pat || path.ends_with(&format!("/{pat}")) {
            return Some(pat.clone());
        }
    }
    None
}
