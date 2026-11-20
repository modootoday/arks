//! Merge helper programs: `merge-index` and `merge-one-file`, plus the
//! in-index three-way merge the scripted strategies (`resolve`, `octopus`)
//! build on.
//!
//! Cite: <https://git-scm.com/docs/git-merge-index>,
//! <https://git-scm.com/docs/git-merge-one-file>,
//! <https://git-scm.com/docs/git-read-tree#_3_way_merge> (behaviour, not code).
//!
//! `merge-index` runs `<merge-program>` once per unmerged path with seven
//! arguments (stage 1/2/3 blob ids, the path, stage 1/2/3 modes; empty for a
//! missing stage). The program `git-merge-one-file` runs in-process (arks never
//! spawns `git`); any other program is the user's and is run as given.
//! `merge-one-file` resolves one path from those seven arguments, printing the
//! lines stock git prints (`Adding`, `Removing`, `Auto-merging`, `ERROR: ...`).
//! Conflict markers are labelled `.merge_file_XXXXXX`, the temporary-file names
//! stock git shows there.

use crate::error::{CliError, CliResult};
use crate::util::{open_repo, read_index, work_tree, write_index};
use arks_index::{IndexEntry, sort_entries};
use arks_object::{ObjectKind, RawObject};
use arks_oid::Oid;
use arks_repo::Repository;
use arks_store::ObjectStore;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

const MERGE_INDEX_USAGE: &str =
    "usage: git merge-index [-o] [-q] <merge-program> (-a | [--] [<filename>...])";
const ONE_FILE_USAGE: &str = "usage: git merge-one-file <orig blob> <our blob> <their blob> <path> <orig mode> <our mode> <their mode>";

/// One side of a path in a three-way merge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Side {
    pub mode: u32,
    pub oid: Oid,
}

/// A path left unmerged by the trivial in-index merge: stages 1 (base),
/// 2 (ours) and 3 (theirs).
#[derive(Clone, Debug)]
pub(crate) struct Unmerged {
    pub path: String,
    pub stages: [Option<Side>; 3],
}

/// `arks merge-index [-o] [-q] <merge-program> (-a | [--] [<filename>...])`.
pub fn cmd_merge_index(args: &[String]) -> CliResult<()> {
    let usage = || CliError::Exit(129, MERGE_INDEX_USAGE.into());
    let (mut one_shot, mut quiet) = (false, false);
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-o" => one_shot = true,
            "-q" => quiet = true,
            _ => break,
        }
        i += 1;
    }
    if args.len() < i + 2 {
        return Err(usage());
    }
    let pgm = args[i].clone();
    i += 1;
    let repo = open_repo()?;
    let index = read_index(&repo)?;
    drop(repo);
    let mut failed = 0usize;
    let on_fail = |failed: &mut usize| -> CliResult<()> {
        *failed += 1;
        if one_shot {
            return Ok(());
        }
        if quiet {
            return Err(CliError::silent(1));
        }
        Err(CliError::fatal("merge program failed"))
    };
    let mut force_file = false;
    for a in &args[i..] {
        if !force_file && a.starts_with('-') {
            match a.as_str() {
                "--" => force_file = true,
                "-a" => {
                    for u in unmerged_paths(&index.entries) {
                        if !run_merge_program(&pgm, &u)? {
                            on_fail(&mut failed)?;
                        }
                    }
                }
                _ => {
                    return Err(CliError::fatal(format!(
                        "git merge-index: unknown option {a}"
                    )));
                }
            }
            continue;
        }
        let entries: Vec<&IndexEntry> = index.entries.iter().filter(|e| e.path == *a).collect();
        if entries.is_empty() {
            return Err(CliError::fatal(format!(
                "git merge-index: {a} not in the cache"
            )));
        }
        let owned: Vec<IndexEntry> = entries.into_iter().cloned().collect();
        if let Some(u) = unmerged_paths(&owned).into_iter().next() {
            if !run_merge_program(&pgm, &u)? {
                on_fail(&mut failed)?;
            }
        }
    }
    if failed > 0 {
        if quiet {
            return Err(CliError::silent(1));
        }
        return Err(CliError::fatal("merge program failed"));
    }
    Ok(())
}

/// Unmerged paths of `entries` (index order), with their stages.
pub(crate) fn unmerged_paths(entries: &[IndexEntry]) -> Vec<Unmerged> {
    let mut out: Vec<Unmerged> = Vec::new();
    for e in entries.iter().filter(|e| e.stage() > 0) {
        let side = Side {
            mode: e.mode,
            oid: e.oid,
        };
        let slot = usize::from(e.stage() - 1).min(2);
        match out.last_mut() {
            Some(u) if u.path == e.path => u.stages[slot] = Some(side),
            _ => {
                let mut stages = [None; 3];
                stages[slot] = Some(side);
                out.push(Unmerged {
                    path: e.path.clone(),
                    stages,
                });
            }
        }
    }
    out
}

/// The seven `merge-one-file` arguments for an unmerged path.
fn program_args(u: &Unmerged) -> [String; 7] {
    let oid = |i: usize| u.stages[i].map(|s| s.oid.to_hex()).unwrap_or_default();
    let mode = |i: usize| {
        u.stages[i]
            .map(|s| format!("{:o}", s.mode))
            .unwrap_or_default()
    };
    [
        oid(0),
        oid(1),
        oid(2),
        u.path.clone(),
        mode(0),
        mode(1),
        mode(2),
    ]
}

/// Whether `pgm` names the stock one-file merge program (run in-process).
fn is_one_file_program(pgm: &str) -> bool {
    let base = pgm.rsplit(['/', '\\']).next().unwrap_or(pgm);
    let base = base.strip_suffix(".exe").unwrap_or(base);
    base == "git-merge-one-file"
}

/// Run the merge program for one path; `true` when it succeeded.
fn run_merge_program(pgm: &str, u: &Unmerged) -> CliResult<bool> {
    use std::io::Write;
    let argv = program_args(u);
    if is_one_file_program(pgm) {
        let mut repo = open_repo()?;
        let code = merge_one_file(&mut repo, &argv)?;
        return Ok(code == 0);
    }
    std::io::stdout().flush()?;
    match std::process::Command::new(pgm).args(&argv).status() {
        Ok(st) => Ok(st.success()),
        Err(e) => {
            let why = if e.kind() == std::io::ErrorKind::NotFound {
                "No such file or directory".to_string()
            } else {
                e.to_string()
            };
            eprintln!("error: cannot run {pgm}: {why}");
            Ok(false)
        }
    }
}

/// `arks merge-one-file <orig> <ours> <theirs> <path> <orig-mode> <our-mode> <their-mode>`.
pub fn cmd_merge_one_file(args: &[String]) -> CliResult<()> {
    if args.len() != 7 {
        // Stock git prints the usage twice plus a note on stdout.
        println!("{ONE_FILE_USAGE}\n\n{ONE_FILE_USAGE}\n\nBlob ids and modes should be empty for missing files.");
        let help = args.len() == 1 && args[0] == "-h";
        return if help { Ok(()) } else { Err(CliError::silent(1)) };
    }
    let mut repo = open_repo()?;
    match merge_one_file(&mut repo, args)? {
        0 => Ok(()),
        c => Err(CliError::silent(c)),
    }
}

/// Resolve one path from the seven `merge-one-file` arguments; returns the
/// exit code (0 resolved, 1 left for the user).
pub(crate) fn merge_one_file(repo: &mut Repository, a: &[String]) -> CliResult<u8> {
    let (o, ours, theirs, path) = (&a[0], &a[1], &a[2], a[3].as_str());
    let (om, am, bm) = (&a[4], &a[5], &a[6]);
    let work = work_tree(repo)?;
    let abs = work.join(path);
    let parse = |hex: &str| -> CliResult<Oid> {
        Oid::from_hex(hex).map_err(|_| CliError::fatal(format!("not a valid object name {hex}")))
    };
    let mode = |m: &str| u32::from_str_radix(m, 8).unwrap_or(0o100644);
    // Deleted on one side (or both), the other side unchanged.
    if !o.is_empty()
        && (ours.is_empty() || theirs.is_empty())
        && ((ours.is_empty() && theirs.is_empty())
            || (ours.is_empty() && theirs == o)
            || (theirs.is_empty() && ours == o))
    {
        if (am.is_empty() && om != bm) || (bm.is_empty() && om != am) {
            eprintln!("ERROR: File {path} deleted on one branch but had its");
            eprintln!("ERROR: permissions changed on the other.");
            return Ok(1);
        }
        if !ours.is_empty() {
            println!("Removing {path}");
            if abs.symlink_metadata().is_ok() {
                let _ = fs::remove_file(&abs);
                remove_empty_parents(&work, &abs);
            }
        }
        if abs.symlink_metadata().is_err() {
            set_index_path(repo, path, None)?;
        }
        return Ok(0);
    }
    // Added on one side only.
    if o.is_empty() && ours.is_empty() && !theirs.is_empty() {
        if abs.symlink_metadata().is_ok() {
            eprintln!("ERROR: untracked {path} is overwritten by the merge.");
            return Ok(1);
        }
        println!("Adding {path}");
        let side = Side {
            mode: mode(bm),
            oid: parse(theirs)?,
        };
        checkout_side(repo, &work, path, &side)?;
        set_index_path(repo, path, Some(side))?;
        return Ok(0);
    }
    if o.is_empty() && !ours.is_empty() && theirs.is_empty() {
        let side = Side {
            mode: mode(am),
            oid: parse(ours)?,
        };
        set_index_path(repo, path, Some(side))?;
        return Ok(0);
    }
    // Added identically on both sides.
    if o.is_empty() && !ours.is_empty() && ours == theirs {
        if am != bm {
            eprintln!("ERROR: File {path} added identically in both branches,");
            eprintln!("ERROR: but permissions conflict {am}->{bm}.");
            return Ok(1);
        }
        println!("Adding {path}");
        let side = Side {
            mode: mode(am),
            oid: parse(ours)?,
        };
        checkout_side(repo, &work, path, &side)?;
        set_index_path(repo, path, Some(side))?;
        return Ok(0);
    }
    // Modified (or added differently) on both sides: content merge.
    if !ours.is_empty() && !theirs.is_empty() {
        if o.is_empty() {
            println!("Added {path} in both, but differently.");
        }
        println!("Auto-merging {path}");
        let base = if o.is_empty() {
            Vec::new()
        } else {
            repo.get_object(&parse(o)?)?.payload
        };
        let ob = repo.get_object(&parse(ours)?)?.payload;
        let tb = repo.get_object(&parse(theirs)?)?.payload;
        let labels = crate::cmd::merge_engine::MergeLabels {
            ours: temp_merge_label(),
            base: temp_merge_label(),
            theirs: temp_merge_label(),
            ..Default::default()
        };
        let (merged, conflict) = crate::cmd::merge_engine::merge_text(&base, &ob, &tb, &labels);
        let mut code = 0;
        let final_mode = if am == bm || om == bm {
            mode(am)
        } else if om == am {
            mode(bm)
        } else {
            eprintln!("ERROR: permissions conflict: {om}->{am},{bm}");
            code = 1;
            mode(am)
        };
        if let Some(parent) = abs.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&abs, &merged)?;
        set_exec(&abs, final_mode)?;
        if conflict {
            eprintln!("ERROR: content conflict in {path}");
            code = 1;
        }
        if code == 0 {
            let format = repo.object_format();
            let oid = repo
                .store_mut()
                .put(format, &RawObject::new(ObjectKind::Blob, merged))?;
            set_index_path(
                repo,
                path,
                Some(Side {
                    mode: final_mode,
                    oid,
                }),
            )?;
        }
        return Ok(code);
    }
    eprintln!("ERROR: {path}: Not handling case {o} -> {ours} -> {theirs}");
    Ok(1)
}

/// A `.merge_file_XXXXXX` label (stock git's temporary-file name shape).
pub(crate) fn temp_merge_label() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    const ALPHA: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let mut x = nanos
        ^ (u64::from(std::process::id()) << 32)
        ^ COUNTER
            .fetch_add(1, Ordering::Relaxed)
            .wrapping_mul(0x9e37_79b9_7f4a_7c15);
    let mut s = String::from(".merge_file_");
    for _ in 0..6 {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        s.push(ALPHA[(x % ALPHA.len() as u64) as usize] as char);
    }
    s
}

/// Replace every index entry of `path` with one stage-0 entry (or none).
pub(crate) fn set_index_path(repo: &Repository, path: &str, side: Option<Side>) -> CliResult<()> {
    let mut index = read_index(repo)?;
    index.entries.retain(|e| e.path != path);
    if let Some(s) = side {
        let work = work_tree(repo)?;
        let entry = match fs::symlink_metadata(work.join(path)) {
            Ok(meta) => {
                let mut e = crate::cmd::working_tree::stat_entry(path, s.oid, &meta);
                e.mode = s.mode;
                e
            }
            Err(_) => entry_for(path, &s, 0),
        };
        index.entries.push(entry);
    }
    sort_entries(&mut index.entries);
    write_index(repo, &index)
}

/// A zero-stat index entry at `stage`.
pub(crate) fn entry_for(path: &str, s: &Side, stage: u16) -> IndexEntry {
    IndexEntry {
        ctime_sec: 0,
        ctime_nsec: 0,
        mtime_sec: 0,
        mtime_nsec: 0,
        dev: 0,
        ino: 0,
        mode: s.mode,
        uid: 0,
        gid: 0,
        size: 0,
        oid: s.oid,
        flags: ((stage & 3) << 12) | (path.len().min(0x0fff) as u16),
        flags_extended: None,
        path: path.to_owned(),
    }
}

/// Write `side`'s blob to the work tree at `path`.
pub(crate) fn checkout_side(
    repo: &Repository,
    work: &Path,
    path: &str,
    side: &Side,
) -> CliResult<()> {
    let abs = work.join(path);
    if let Some(parent) = abs.parent() {
        fs::create_dir_all(parent)?;
    }
    if side.mode == 0o160000 {
        fs::create_dir_all(&abs)?;
        return Ok(());
    }
    let payload = repo.get_object(&side.oid)?.payload;
    if abs.symlink_metadata().is_ok() && !abs.is_dir() {
        let _ = fs::remove_file(&abs);
    }
    #[cfg(unix)]
    if side.mode == 0o120000 {
        let target = String::from_utf8_lossy(&payload).to_string();
        std::os::unix::fs::symlink(target, &abs)?;
        return Ok(());
    }
    fs::write(&abs, payload)?;
    set_exec(&abs, side.mode)
}

fn set_exec(abs: &Path, mode: u32) -> CliResult<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if mode == 0o100755 || mode == 0o100644 {
            let perm = if mode == 0o100755 { 0o755 } else { 0o644 };
            fs::set_permissions(abs, fs::Permissions::from_mode(perm))?;
        }
    }
    #[cfg(not(unix))]
    let _ = (abs, mode);
    Ok(())
}

/// Remove now-empty parent directories of `abs` up to (not including) `work`.
pub(crate) fn remove_empty_parents(work: &Path, abs: &Path) {
    let mut dir = abs.parent();
    while let Some(d) = dir {
        if d == work || !d.starts_with(work) || fs::remove_dir(d).is_err() {
            break;
        }
        dir = d.parent();
    }
}

/// Result of a trivial (read-tree style) three-way merge into the index.
pub(crate) struct TrivialMerge {
    /// Paths that need a file-level merge, in index order.
    pub unmerged: Vec<Unmerged>,
}

/// Blob map (`path -> mode/oid`) of a tree.
pub(crate) fn tree_sides(repo: &Repository, tree: &Oid) -> CliResult<BTreeMap<String, Side>> {
    Ok(crate::cmd::switch_tree::commit_files(repo, tree)?
        .into_iter()
        .map(|(path, f)| {
            (
                path,
                Side {
                    mode: f.mode,
                    oid: f.oid,
                },
            )
        })
        .collect())
}

/// Three-way trivial merge of `base`/`ours`/`theirs` trees into the index and
/// the work tree (git-read-tree -m -u --aggressive): a path both sides agree
/// on, or that only one side changed, is taken; the rest stay as stages
/// 1/2/3 for a file-level merge. `ours` must be the checked-out tree; the
/// work tree is refused when a path this merge rewrites is not up to date.
pub(crate) fn trivial_merge(
    repo: &mut Repository,
    base: &Oid,
    ours: &Oid,
    theirs: &Oid,
) -> CliResult<TrivialMerge> {
    let b = tree_sides(repo, base)?;
    let o = tree_sides(repo, ours)?;
    let t = tree_sides(repo, theirs)?;
    let work = work_tree(repo)?;
    let index = read_index(repo)?;
    let mut keys: Vec<&String> = b.keys().chain(o.keys()).chain(t.keys()).collect();
    keys.sort();
    keys.dedup();
    let mut entries: Vec<IndexEntry> = Vec::new();
    let mut unmerged = Vec::new();
    let mut writes: Vec<(String, Option<Side>)> = Vec::new();
    for k in keys {
        let (bs, os, ts) = (b.get(k), o.get(k), t.get(k));
        let taken = if os == ts {
            Some(os)
        } else if bs == os {
            Some(ts)
        } else if bs == ts {
            Some(os)
        } else {
            None
        };
        match taken {
            Some(side) => {
                if side != os {
                    writes.push((k.clone(), side.copied()));
                }
                if let Some(s) = side {
                    let kept = index
                        .entries
                        .iter()
                        .find(|e| e.path == *k && e.stage() == 0 && e.oid == s.oid)
                        .cloned();
                    entries.push(kept.unwrap_or_else(|| entry_for(k, s, 0)));
                }
            }
            None => {
                let stages = [bs.copied(), os.copied(), ts.copied()];
                for (i, s) in stages.iter().enumerate() {
                    if let Some(s) = s {
                        entries.push(entry_for(k, s, i as u16 + 1));
                    }
                }
                unmerged.push(Unmerged {
                    path: k.clone(),
                    stages,
                });
            }
        }
    }
    for (path, _) in &writes {
        if !worktree_matches_index(repo, &work, &index.entries, path) {
            return Err(CliError::Exit(
                2,
                format!("error: Entry '{path}' not uptodate. Cannot merge."),
            ));
        }
    }
    for (path, side) in &writes {
        match side {
            Some(s) => checkout_side(repo, &work, path, s)?,
            None => {
                let abs = work.join(path);
                if abs.symlink_metadata().is_ok() {
                    let _ = fs::remove_file(&abs);
                    remove_empty_parents(&work, &abs);
                }
            }
        }
    }
    // Fresh stat data for rewritten paths keeps later status reads cheap.
    for e in entries.iter_mut().filter(|e| e.stage() == 0) {
        if writes.iter().any(|(p, _)| *p == e.path) {
            if let Ok(meta) = fs::symlink_metadata(work.join(&e.path)) {
                let mode = e.mode;
                *e = crate::cmd::working_tree::stat_entry(&e.path, e.oid, &meta);
                e.mode = mode;
            }
        }
    }
    sort_entries(&mut entries);
    let mut out = index;
    out.entries = entries;
    write_index(repo, &out)?;
    Ok(TrivialMerge { unmerged })
}

/// Whether the work-tree file at `path` matches its stage-0 index entry
/// (a missing file matches a missing entry).
pub(crate) fn worktree_matches_index(
    repo: &Repository,
    work: &Path,
    entries: &[IndexEntry],
    path: &str,
) -> bool {
    let abs = work.join(path);
    let entry = entries.iter().find(|e| e.path == path && e.stage() == 0);
    match (entry, fs::symlink_metadata(&abs)) {
        (None, Err(_)) => true,
        (None, Ok(_)) => false,
        (Some(_), Err(_)) => false,
        (Some(e), Ok(meta)) => {
            if meta.is_file() && crate::cmd::shared::index_stat_matches(e, &meta) {
                return true;
            }
            let bytes = if meta.file_type().is_symlink() {
                fs::read_link(&abs)
                    .map(|p| p.to_string_lossy().into_owned().into_bytes())
                    .unwrap_or_default()
            } else {
                fs::read(&abs).unwrap_or_default()
            };
            RawObject::new(ObjectKind::Blob, bytes)
                .oid(repo.object_format())
                .ok()
                == Some(e.oid)
        }
    }
}

/// Run the in-process one-file merge over every unmerged path (merge-index
/// `-o git-merge-one-file -a`); returns whether all of them resolved.
pub(crate) fn merge_all_one_file(unmerged: &[Unmerged]) -> CliResult<bool> {
    let mut ok = true;
    for u in unmerged {
        let mut repo = open_repo()?;
        if merge_one_file(&mut repo, &program_args(u))? != 0 {
            ok = false;
        }
    }
    Ok(ok)
}
