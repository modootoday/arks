//! Core porcelain: init / objects / index / commit / log / show.

use crate::cmd::shared::{
    collect_untracked, commit_signature, current_branch_name, file_mode, file_times, head_blob_map,
    index_stat_matches, resolve_rev,
};
use crate::cmd::sign::{is_sign_flag, sign_commit, want_sign_commit};
use crate::error::{CliError, CliResult};
use crate::util::{
    FORMAT, cwd, open_repo, read_index, refuse_unknown_flags, rel_path_from_worktree, work_tree,
    write_index,
};
use crate::write_tree::write_tree_from_index;
use arks_index::{IndexEntry, sort_entries};
use arks_object::{Commit, ObjectKind, RawObject, write_loose};
use arks_oid::Oid;
use arks_refs::RefTarget;
use arks_repo::Repository;
use arks_store::ObjectStore;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::thread;
pub fn cmd_init(args: &[String]) -> CliResult<()> {
    // M16 Sy-1: `--bare` claimed (git-init). Cite: https://git-scm.com/docs/git-init
    // Sha256: `init --object-format=sha256` ([[adr:root/20261110214800]]).
    refuse_unknown_flags(args, &["--bare", "--object-format"])?;
    let bare = args.iter().any(|a| a == "--bare");
    let mut object_format = "sha1".to_string();
    let mut positional = Vec::new();
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--object-format" {
            i += 1;
            let v = args
                .get(i)
                .ok_or_else(|| CliError::Usage("init --object-format <sha1|sha256>".into()))?;
            object_format = v.to_ascii_lowercase();
        } else if let Some(rest) = args[i].strip_prefix("--object-format=") {
            object_format = rest.to_ascii_lowercase();
        } else if !args[i].starts_with('-') {
            positional.push(args[i].clone());
        }
        i += 1;
    }
    if object_format != "sha1" && object_format != "sha256" {
        return Err(CliError::Usage(format!(
            "init --object-format: unknown '{object_format}' (claimed: sha1, sha256)"
        )));
    }
    let dir = if let Some(p) = positional.first() {
        PathBuf::from(p)
    } else {
        cwd()?
    };
    fs::create_dir_all(&dir)?;
    let git_dir = if bare { dir.clone() } else { dir.join(".git") };
    if git_dir.join("HEAD").exists() || (!bare && git_dir.exists()) {
        return Err(CliError::Msg(format!(
            "repository already exists: {}",
            git_dir.display()
        )));
    }
    fs::create_dir_all(git_dir.join("objects"))?;
    fs::create_dir_all(git_dir.join("refs/heads"))?;
    fs::create_dir_all(git_dir.join("refs/tags"))?;
    fs::write(git_dir.join("HEAD"), b"ref: refs/heads/main\n")?;
    let bare_cfg = if bare { "true" } else { "false" };
    let cfg = if object_format == "sha256" {
        format!(
            "[core]\n\trepositoryformatversion = 1\n\tfilemode = true\n\tbare = {bare_cfg}\n\tlogallrefupdates = true\n[extensions]\n\tobjectFormat = sha256\n"
        )
    } else {
        format!(
            "[core]\n\trepositoryformatversion = 0\n\tfilemode = true\n\tbare = {bare_cfg}\n\tlogallrefupdates = true\n"
        )
    };
    fs::write(git_dir.join("config"), cfg)?;
    println!(
        "Initialized empty arks repository in {}",
        git_dir.canonicalize().unwrap_or(git_dir).display()
    );
    Ok(())
}

pub fn cmd_hash_object(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &["-w"])?;
    let write = args.iter().any(|a| a == "-w");
    let paths: Vec<&String> = args.iter().filter(|a| !a.starts_with('-')).collect();
    let payload = if paths.is_empty() {
        let mut buf = Vec::new();
        io::stdin().read_to_end(&mut buf)?;
        buf
    } else if paths.len() == 1 {
        fs::read(paths[0].as_str())?
    } else {
        return Err(CliError::Usage(
            "hash-object: at most one path (or stdin)".into(),
        ));
    };
    let obj = RawObject::new(ObjectKind::Blob, payload);
    let format = open_repo().map(|r| r.object_format()).unwrap_or(FORMAT);
    let oid = if write {
        let mut repo = open_repo()?;
        // OBJ-001 (M20 Cx-5): identical concurrent loose creates are idempotent; honesty note.
        let format = repo.object_format();
        let oid = repo.store_mut().put(format, &obj)?;
        eprintln!(
            "note: OBJ-001 — concurrent identical loose object creates are content-addressed/idempotent; do not invent unlock-by-delete under live writers"
        );
        eprintln!(
            "note: OBJ-008 — concurrent hash-object of large inputs saturates CPU/temp; serialize heavy hash jobs"
        );
        oid
    } else {
        obj.oid(format)?
    };
    println!("{}", oid.to_hex());
    Ok(())
}

pub fn cmd_cat_file(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &["-t", "-s", "-p"])?;
    let mode = args
        .iter()
        .find(|a| *a == "-t" || *a == "-s" || *a == "-p")
        .ok_or_else(|| CliError::Usage("cat-file: require -t, -s, or -p".into()))?;
    let object = args
        .iter()
        .find(|a| !a.starts_with('-'))
        .ok_or_else(|| CliError::Usage("cat-file: missing object".into()))?;
    let repo = open_repo()?;
    // OBJ-005 (M20 Cx-5): detect truncated/partial loose object files before decode.
    if object.len() == 40 && object.chars().all(|c| c.is_ascii_hexdigit()) {
        let loose = repo
            .common_dir()
            .join("objects")
            .join(&object[..2])
            .join(&object[2..]);
        if loose.is_file() {
            let meta = fs::metadata(&loose)?;
            if meta.len() < 2 {
                return Err(CliError::Msg(format!(
                    "corrupt loose object {} (OBJ-005 — partial/truncated loose write; remove only after crash recovery)",
                    object
                )));
            }
        }
    }
    let oid = resolve_rev(&repo, object)?;
    let obj = match repo.get_object(&oid) {
        Ok(o) => o,
        Err(e) => {
            return Err(CliError::Msg(format!(
                "{e} (OBJ-005 — refuse reading partial/corrupt object; no silent invent)"
            )));
        }
    };
    match mode.as_str() {
        "-t" => println!("{}", obj.kind.as_str()),
        "-s" => println!("{}", obj.payload.len()),
        "-p" => match obj.kind {
            ObjectKind::Blob => {
                io::Write::write_all(&mut io::stdout(), &obj.payload)?;
            }
            ObjectKind::Commit | ObjectKind::Tag => {
                print!("{}", String::from_utf8_lossy(&obj.payload));
            }
            ObjectKind::Tree => {
                let tree = arks_object::Tree::decode(&obj.payload, repo.oid_kind())?;
                for e in &tree.entries {
                    let kind = if e.mode.is_tree() { "tree" } else { "blob" };
                    println!(
                        "{:>6} {} {}\t{}",
                        e.mode.as_str(),
                        kind,
                        e.oid.to_hex(),
                        e.name
                    );
                }
            }
        },
        _ => unreachable!(),
    }
    Ok(())
}

/// M15 Fs-1: run sync hooks (githooks). No async/out-of-band scheduler (no L3).
pub(crate) fn run_claimed_hooks(
    repo: &Repository,
    no_verify: bool,
    names: &[&str],
    stdin: Option<&[u8]>,
) -> CliResult<()> {
    crate::cmd::hooks::run_sync_hooks(repo, no_verify, names, stdin)
}

pub fn cmd_update_ref(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &["-d", "--no-verify"])?;
    let delete = args.iter().any(|a| a == "-d");
    let no_verify = args.iter().any(|a| a == "--no-verify");
    let positional: Vec<&String> = args.iter().filter(|a| !a.starts_with('-')).collect();
    let mut repo = open_repo()?;
    // CORR-003 (M20 Cx-7): replace-refs concurrent honesty — refuse under replace.lock.
    // Cite: https://git-scm.com/docs/git-replace + lockfile (never delete live locks).
    let replace_lock = repo.common_dir().join("refs/replace.lock");
    let replace_lock_gd = repo.git_dir().join("refs/replace.lock");
    if replace_lock.is_file() || replace_lock_gd.is_file() {
        let p = if replace_lock.is_file() {
            &replace_lock
        } else {
            &replace_lock_gd
        };
        return Err(CliError::Msg(format!(
            "{} (CORR-003 — concurrent replace-refs / live lock delete anti-pattern refuse; lease ≠ skip)",
            p.display()
        )));
    }
    // REF-009 (M20 Cx-4): plant logs/*.lock → refuse update-ref class.
    crate::reflog::refuse_if_reflog_locked(&repo, "HEAD")?;
    if let Some(name) = positional.first() {
        crate::reflog::refuse_if_reflog_locked(&repo, name)?;
    }
    // REF-003 (M20 Cx-8): loose vs packed delete race — refuse delete under packed-refs.lock.
    let packed_lock = repo.common_dir().join("packed-refs.lock");
    if delete && packed_lock.is_file() {
        return Err(CliError::Msg(format!(
            "{} (REF-003 — loose vs packed-refs delete race refuse; serialize delete vs pack-refs; lease ≠ skip)",
            packed_lock.display()
        )));
    }
    run_claimed_hooks(&repo, no_verify, &["reference-transaction", "update"], None)?;
    if delete {
        let name = positional
            .first()
            .ok_or_else(|| CliError::Usage("update-ref -d <ref> [<old>]".into()))?;
        let old = positional.get(1).map(|s| Oid::from_hex(s)).transpose()?;
        repo.refs_mut().delete(name, old.as_ref())?;
    } else {
        if positional.len() < 2 {
            return Err(CliError::Usage("update-ref <ref> <new> [<old>]".into()));
        }
        let name = positional[0];
        let new = Oid::from_hex(positional[1])?;
        let old = positional.get(2).map(|s| Oid::from_hex(s)).transpose()?;
        repo.refs_mut().update(name, &new, old.as_ref())?;
    }
    Ok(())
}

pub fn cmd_rev_parse(args: &[String]) -> CliResult<()> {
    // Sy-1: `--abbrev-ref` / `--verify` / `--symbolic-full-name` (git-rev-parse).
    // Cite: https://git-scm.com/docs/git-rev-parse + [[sot:root/cli-flag-parity-complete]]
    refuse_unknown_flags(args, &["--abbrev-ref", "--verify", "--symbolic-full-name"])?;
    let abbrev_ref = args
        .iter()
        .any(|a| *a == "--abbrev-ref" || a.starts_with("--abbrev-ref="));
    let symbolic_full = args.iter().any(|a| a == "--symbolic-full-name");
    let verify = args.iter().any(|a| a == "--verify");
    let revs: Vec<&String> = args.iter().filter(|a| !a.starts_with('-')).collect();
    if revs.is_empty() {
        return Err(CliError::Usage("rev-parse: missing revision".into()));
    }
    let repo = open_repo()?;
    for rev in revs {
        if abbrev_ref {
            println!("{}", abbreviate_ref(&repo, rev)?);
            continue;
        }
        if symbolic_full {
            println!("{}", symbolic_full_name(&repo, rev)?);
            continue;
        }
        let oid = resolve_rev(&repo, rev)?;
        if verify {
            // Existence check only; still print the OID (git --verify default).
        }
        println!("{}", oid.to_hex());
    }
    Ok(())
}

/// Short unambiguous ref name (`git rev-parse --abbrev-ref`).
fn abbreviate_ref(repo: &Repository, rev: &str) -> CliResult<String> {
    if rev == "HEAD" {
        return match repo.head_target()? {
            RefTarget::Symbolic(s) => Ok(shorten_ref_name(&s)),
            RefTarget::Direct(_) => Ok("HEAD".into()),
        };
    }
    if rev.starts_with("refs/") {
        return Ok(shorten_ref_name(rev));
    }
    // Already short, or resolve via DWIM then shorten if we find a ref form.
    let as_heads = format!("refs/heads/{rev}");
    if repo.refs().resolve(&as_heads).is_ok() {
        return Ok(rev.to_string());
    }
    let as_tags = format!("refs/tags/{rev}");
    if repo.refs().resolve(&as_tags).is_ok() {
        return Ok(rev.to_string());
    }
    let as_remotes = format!("refs/remotes/{rev}");
    if repo.refs().resolve(&as_remotes).is_ok() {
        return Ok(rev.to_string());
    }
    // Fall back: if resolve works as OID-ish name, still print the given token.
    let _ = resolve_rev(repo, rev)?;
    Ok(rev.to_string())
}

fn symbolic_full_name(repo: &Repository, rev: &str) -> CliResult<String> {
    if rev == "HEAD" {
        return match repo.head_target()? {
            RefTarget::Symbolic(s) => Ok(s),
            RefTarget::Direct(_) => Err(CliError::Msg(
                "rev-parse --symbolic-full-name: detached HEAD".into(),
            )),
        };
    }
    if rev.starts_with("refs/") {
        let _ = repo.refs().resolve(rev)?;
        return Ok(rev.to_string());
    }
    let as_heads = format!("refs/heads/{rev}");
    if repo.refs().resolve(&as_heads).is_ok() {
        return Ok(as_heads);
    }
    let as_tags = format!("refs/tags/{rev}");
    if repo.refs().resolve(&as_tags).is_ok() {
        return Ok(as_tags);
    }
    let as_remotes = format!("refs/remotes/{rev}");
    if repo.refs().resolve(&as_remotes).is_ok() {
        return Ok(as_remotes);
    }
    Err(CliError::Msg(format!(
        "rev-parse --symbolic-full-name: unknown ref {rev}"
    )))
}

fn shorten_ref_name(full: &str) -> String {
    full.strip_prefix("refs/heads/")
        .or_else(|| full.strip_prefix("refs/tags/"))
        .or_else(|| full.strip_prefix("refs/remotes/"))
        .unwrap_or(full)
        .to_string()
}

pub fn cmd_add(args: &[String]) -> CliResult<()> {
    // Fs-4 + M16 Sy-1: `-A`/`-u`/`--chmod=+x|-x` (git-add). Cite: https://git-scm.com/docs/git-add
    // FS3: hunk-level `-p`, `-e` edit, `-i` menu (plus FS1 scripted/TTY answers).
    if args.iter().any(|a| a == "-i" || a == "--interactive") {
        return crate::cmd::interactive_patch::cmd_add_interactive(args);
    }
    if args.iter().any(|a| a == "-e" || a == "--edit") {
        return crate::cmd::interactive_patch::cmd_add_edit(args);
    }
    if args.iter().any(|a| a == "-p" || a == "--patch") {
        return crate::cmd::interactive_patch::cmd_add_patch_hunks(args);
    }
    refuse_unknown_flags(args, &["-A", "--all", "-u", "--update", "--chmod"])?;
    // HOOK-005 (M20 Cx-4): nested index write under held lock (hook/agent) refuse-closed.
    // Keep LOCK-001/LOCK-004 + retry guidance so Cm-2/Cx-2 regressions stay honest.
    {
        let repo = open_repo()?;
        // FS-007 opt-in refuse when .git under sync root (ARKS_HOST_ENV_REFUSE_SYNC=1).
        // IDX-004 (M20 Cx-5): split-index sharedindex.lock race refuse.
        let shared = repo.git_dir().join("sharedindex.lock");
        if shared.is_file() {
            return Err(CliError::Msg(format!(
                "{} (IDX-004 — split-index / sharedindex.lock contention refuse; no parallel shared-index dialect)",
                shared.display()
            )));
        }
        // MISC-008 (M20 Cx-7): sparse cone rewrite vs concurrent materialize.
        let sparse_lock = repo.git_dir().join("info/sparse-checkout.lock");
        if sparse_lock.is_file() {
            return Err(CliError::Msg(format!(
                "{} (MISC-008 — parallel sparse-checkout cone vs materialize refuse; lease ≠ skip)",
                sparse_lock.display()
            )));
        }
        // MISC-019 (M20 Cx-5): concurrent gitattributes install.
        if let Ok(wt) = work_tree(&repo) {
            let attr_lock = wt.join(".gitattributes.lock");
            if attr_lock.is_file() {
                return Err(CliError::Msg(format!(
                    "{} (MISC-019 — concurrent .gitattributes install refuse; lease ≠ skip)",
                    attr_lock.display()
                )));
            }
        }
        let lock = arks_index::index_lock_path(&repo.index_path());
        if lock.is_file() {
            return Err(CliError::Msg(format!(
                "{} (index locked; LOCK-001 exclusive create; LOCK-004 porcelain die-class; HOOK-005 nested index write refuse; retry after the holder finishes — no skip)",
                lock.display()
            )));
        }
    }
    let all = args.iter().any(|a| a == "-A" || a == "--all");
    let update = args.iter().any(|a| a == "-u" || a == "--update");
    let mut chmod_mode: Option<u32> = None;
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--chmod" {
            i += 1;
            let v = args
                .get(i)
                .ok_or_else(|| CliError::Usage("add --chmod=+x|-x".into()))?;
            chmod_mode = Some(match v.as_str() {
                "+x" => 0o100755,
                "-x" => 0o100644,
                other => {
                    return Err(CliError::Usage(format!(
                        "add --chmod: unsupported mode '{other}' (claim +x|-x)"
                    )));
                }
            });
        } else if let Some(rest) = args[i].strip_prefix("--chmod=") {
            chmod_mode = Some(match rest {
                "+x" => 0o100755,
                "-x" => 0o100644,
                other => {
                    return Err(CliError::Usage(format!(
                        "add --chmod: unsupported mode '{other}' (claim +x|-x)"
                    )));
                }
            });
        }
        i += 1;
    }
    let positional: Vec<&String> = {
        let mut out = Vec::new();
        let mut skip_next = false;
        for a in args {
            if skip_next {
                skip_next = false;
                continue;
            }
            if a == "--chmod" {
                skip_next = true;
                continue;
            }
            if a.starts_with("--chmod=") || a.starts_with('-') {
                continue;
            }
            out.push(a);
        }
        out
    };
    if positional.is_empty() && !all && !update {
        return Err(CliError::Usage(
            "add: path required (or -A/--all / -u/--update)".into(),
        ));
    }
    let repo = open_repo()?;
    let work = work_tree(&repo)?;
    let mut index = read_index(&repo)?;
    let mut paths: Vec<String> = Vec::new();
    if all || update {
        let indexed: std::collections::HashSet<_> =
            index.entries.iter().map(|e| e.path.clone()).collect();
        if all {
            let mut untracked = Vec::new();
            collect_untracked(&work, &work, &indexed, &mut untracked)?;
            paths.extend(untracked);
        }
        for e in index.entries.iter().filter(|e| e.stage() == 0) {
            paths.push(e.path.clone());
        }
        paths.sort();
        paths.dedup();
    }
    for p in &positional {
        let rel = rel_path_from_worktree(&work, Path::new(p))?;
        let abs = work.join(&rel);
        if abs.is_dir() {
            let mut nested = Vec::new();
            let indexed: std::collections::HashSet<_> = std::collections::HashSet::new();
            collect_untracked(&work, &abs, &indexed, &mut nested)?;
            let prefix = format!("{rel}/");
            for e in index.entries.iter().filter(|e| e.stage() == 0) {
                if e.path == rel || e.path.starts_with(&prefix) {
                    paths.push(e.path.clone());
                }
            }
            paths.extend(nested);
        } else {
            paths.push(rel);
        }
    }
    paths.sort();
    paths.dedup();
    let mut updates: HashMap<String, IndexEntry> = HashMap::new();
    let mut removals: HashSet<String> = HashSet::new();
    // Pf4-2: classify dirty paths serially (stat short-circuit), then parallel
    // content-addressed loose writes (safe; OID path rename). Index lock stays serial.
    let mut dirty: Vec<String> = Vec::new();
    {
        let stage0: HashMap<&str, &IndexEntry> = index
            .entries
            .iter()
            .filter(|e| e.stage() == 0)
            .map(|e| (e.path.as_str(), e))
            .collect();
        for rel in paths {
            let abs = work.join(&rel);
            if !abs.is_file() {
                if all || update {
                    removals.insert(rel);
                }
                continue;
            }
            if update && !stage0.contains_key(rel.as_str()) {
                continue;
            }
            let meta = fs::metadata(&abs)?;
            // Git-style ce_match_stat short-circuit: unchanged stage-0 entries skip
            // re-hash/re-write (git-add / index refresh). Cite: gitformat-index.
            if chmod_mode.is_none() {
                if let Some(existing) = stage0.get(rel.as_str()) {
                    if index_stat_matches(existing, &meta) {
                        continue;
                    }
                }
            }
            dirty.push(rel);
        }
    }
    if !dirty.is_empty() {
        let objects_dir = repo.store().objects_dir().to_path_buf();
        let format = repo.object_format();
        let workers = parallel_workers(dirty.len());
        if workers <= 1 {
            for rel in dirty {
                let entry = stage_blob_entry(&work, &objects_dir, &rel, chmod_mode, format)?;
                updates.insert(rel, entry);
            }
        } else {
            let chunk = dirty.len().div_ceil(workers);
            let mut staged: Vec<IndexEntry> = Vec::with_capacity(dirty.len());
            let mut worker_err: Option<CliError> = None;
            thread::scope(|scope| {
                let mut handles = Vec::with_capacity(workers);
                for piece in dirty.chunks(chunk) {
                    let objects_dir = &objects_dir;
                    let work = &work;
                    handles.push(scope.spawn(move || -> CliResult<Vec<IndexEntry>> {
                        let mut local = Vec::with_capacity(piece.len());
                        for rel in piece {
                            local.push(stage_blob_entry(
                                work,
                                objects_dir,
                                rel,
                                chmod_mode,
                                format,
                            )?);
                        }
                        Ok(local)
                    }));
                }
                for h in handles {
                    match h.join() {
                        Ok(Ok(local)) => staged.extend(local),
                        Ok(Err(e)) => {
                            worker_err = Some(e);
                            break;
                        }
                        Err(_) => {
                            worker_err = Some(CliError::Msg("add: worker panicked".into()));
                            break;
                        }
                    }
                }
            });
            if let Some(e) = worker_err {
                return Err(e);
            }
            for entry in staged {
                updates.insert(entry.path.clone(), entry);
            }
        }
    }
    if !updates.is_empty() || !removals.is_empty() {
        index.entries.retain(|e| {
            if updates.contains_key(&e.path) {
                return false;
            }
            if removals.contains(&e.path) && e.stage() == 0 {
                return false;
            }
            true
        });
        index.entries.extend(updates.into_values());
        sort_entries(&mut index.entries);
        write_index(&repo, &index)?;
    }
    Ok(())
}

fn parallel_workers(n: usize) -> usize {
    if n < 32 {
        return 1;
    }
    thread::available_parallelism()
        .map(|p| p.get().clamp(2, 8))
        .unwrap_or(4)
        .min(n)
}

fn stage_blob_entry(
    work: &Path,
    objects_dir: &Path,
    rel: &str,
    chmod_mode: Option<u32>,
    format: arks_store::ObjectFormat,
) -> CliResult<IndexEntry> {
    let abs = work.join(rel);
    let meta = fs::metadata(&abs)?;
    let mut payload = fs::read(&abs)?;
    // FS6 closest: honor configured `filter.<name>.clean` when `.gitattributes` sets filter=.
    if let Some(filtered) = apply_clean_filter(work, rel, &payload)? {
        payload = filtered;
    }
    let size = payload.len().min(u32::MAX as usize) as u32;
    let obj = RawObject::new(ObjectKind::Blob, payload);
    let oid = write_loose(objects_dir, format, &obj)?;
    let (ctime_sec, ctime_nsec, mtime_sec, mtime_nsec) = file_times(&meta);
    let mode = chmod_mode.unwrap_or_else(|| file_mode(&meta));
    Ok(IndexEntry {
        ctime_sec,
        ctime_nsec,
        mtime_sec,
        mtime_nsec,
        dev: 0,
        ino: 0,
        mode,
        uid: 0,
        gid: 0,
        size,
        oid,
        flags: (rel.len().min(0x0fff)) as u16,
        flags_extended: None,
        path: rel.to_owned(),
    })
}

pub fn cmd_status(args: &[String]) -> CliResult<()> {
    // Optional locks: stock Git may refresh the index under `index.lock` unless
    // `--no-optional-locks` / `GIT_OPTIONAL_LOCKS=0` (IDX-002 background status
    // vs writers; AGENT-006 / IDX-008 agent readers). arks status is read-only
    // on the index (product default = no optional locks). Cite: git-status.
    // M16 Sy-1: `-b` branch header claimed.
    refuse_unknown_flags(args, &["--no-optional-locks", "--porcelain", "-s", "-b"])?;
    let porcelain = args.iter().any(|a| a == "--porcelain" || a == "-s");
    let show_branch = args.iter().any(|a| a == "-b");
    let _no_optional = wants_no_optional_locks(args);
    let repo = open_repo()?;
    // FSM-002 (M20 Cx-7): concurrent FSM transition refuse.
    let fsm_transition = repo.git_dir().join(".arks-fsm-transition");
    if fsm_transition.is_file() {
        return Err(CliError::Msg(
            "status refused (FSM-002 — concurrent fsmonitor/state-machine transition in progress; serialize clients)".into(),
        ));
    }
    // FSM-004 (M20 Cx-7): incomplete FSM marker refuse.
    let fsm_incomplete = repo.git_dir().join(".fsmonitor-incomplete");
    if fsm_incomplete.is_file() {
        return Err(CliError::Msg(
            "status refused (FSM-004 — incomplete fsmonitor FSM marker; refuse stampede full-scan under crash recovery)".into(),
        ));
    }
    let index_path = repo.index_path();
    let lock_before = arks_index::index_lock_path(&index_path);
    let had_lock = lock_before.is_file();
    let work = work_tree(&repo)?;
    let index = read_index(&repo)?;
    let head_paths = head_blob_map(&repo)?;
    let branch = current_branch_name(&repo)?;

    let mut staged = Vec::new();
    let mut modified = Vec::new();
    let mut deleted = Vec::new();

    for e in index.entries.iter().filter(|e| e.stage() == 0) {
        let abs = work.join(&e.path);
        match head_paths.get(&e.path) {
            None => staged.push(e.path.clone()),
            Some(oid) if *oid != e.oid => staged.push(e.path.clone()),
            Some(_) => {}
        }
        if !abs.is_file() {
            deleted.push(e.path.clone());
            continue;
        }
        // Skip re-hash when index stat cache matches (git ce_match_stat class).
        let meta = fs::metadata(&abs)?;
        if index_stat_matches(e, &meta) {
            continue;
        }
        let bytes = fs::read(&abs)?;
        let blob = RawObject::new(ObjectKind::Blob, bytes);
        let oid = blob.oid(repo.object_format())?;
        if oid != e.oid {
            modified.push(e.path.clone());
        }
    }

    let indexed: std::collections::HashSet<_> =
        index.entries.iter().map(|e| e.path.clone()).collect();
    let mut untracked = Vec::new();
    collect_untracked(&work, &work, &indexed, &mut untracked)?;

    if porcelain {
        // v1 porcelain: optional `## branch` then XY PATH (claimed subset: M/A/D/?).
        if show_branch {
            println!("## {branch}");
        }
        for p in &staged {
            let xy = if head_paths.contains_key(p) {
                "M "
            } else {
                "A "
            };
            println!("{xy}{p}");
        }
        for p in &modified {
            if !staged.contains(p) {
                println!(" M{p}");
            }
        }
        for p in &deleted {
            println!(" D{p}");
        }
        for p in &untracked {
            println!("?? {p}");
        }
    } else {
        println!("On branch {branch}");
        if staged.is_empty() && modified.is_empty() && deleted.is_empty() && untracked.is_empty() {
            println!("nothing to commit, working tree clean");
        } else {
            if !staged.is_empty() {
                println!("Changes to be committed:");
                for p in &staged {
                    println!("\tnew file / modified: {p}");
                }
            }
            if !modified.is_empty() || !deleted.is_empty() {
                println!("Changes not staged for commit:");
                for p in &modified {
                    println!("\tmodified: {p}");
                }
                for p in &deleted {
                    println!("\tdeleted: {p}");
                }
            }
            if !untracked.is_empty() {
                println!("Untracked files:");
                for p in &untracked {
                    println!("\t{p}");
                }
            }
        }
    }
    // Honesty: status must not create optional index.lock (AGENT-006 / IDX-008 / IDX-002).
    if !had_lock && lock_before.is_file() {
        return Err(CliError::Msg(
            "internal error: status created index.lock (optional locks forbidden; IDX-002/AGENT-006)"
                .into(),
        ));
    }
    Ok(())
}

/// True when agent/profile path or Git-compatible no-optional-locks is requested.
///
/// Defaults **on** for `ARKS_PROFILE=agent` (arks agent overlay). Also honors
/// `--no-optional-locks` and `GIT_OPTIONAL_LOCKS=0` (git-status).
/// True when agent/profile path or Git-compatible no-optional-locks is requested.
///
/// Defaults **on** for `ARKS_PROFILE=agent` (arks agent overlay). Also honors
/// `--no-optional-locks` and `GIT_OPTIONAL_LOCKS=0` (git-status).
pub fn wants_no_optional_locks(args: &[String]) -> bool {
    if args.iter().any(|a| a == "--no-optional-locks") {
        return true;
    }
    if std::env::var("GIT_OPTIONAL_LOCKS")
        .map(|v| v == "0")
        .unwrap_or(false)
    {
        return true;
    }
    if std::env::var("ARKS_PROFILE")
        .map(|v| v.eq_ignore_ascii_case("agent"))
        .unwrap_or(false)
    {
        return true;
    }
    // Default product path: status never takes optional locks (compat mitigate).
    true
}

pub fn cmd_commit(args: &[String]) -> CliResult<()> {
    // Filter sign flags (incl. `-S<keyid>`) before refuse_unknown_flags.
    let flag_args: Vec<String> = args.iter().filter(|a| !is_sign_flag(a)).cloned().collect();
    refuse_unknown_flags(
        &flag_args,
        &[
            "-m",
            "-F",
            "--file",
            "-t",
            "--template",
            "--allow-empty",
            "--no-verify",
            "--amend",
            "-a",
            "--all",
        ],
    )?;
    let allow_empty = args.iter().any(|a| a == "--allow-empty");
    let no_verify = args.iter().any(|a| a == "--no-verify");
    let amend = args.iter().any(|a| a == "--amend");
    let all = args.iter().any(|a| a == "-a" || a == "--all");
    // REF-009 (M20 Cx-4): early refuse when reflog lock held (commit appends logs).
    {
        let repo = open_repo()?;
        crate::reflog::refuse_if_reflog_locked(&repo, "HEAD")?;
        if let Ok(RefTarget::Symbolic(name)) = repo.head_target() {
            crate::reflog::refuse_if_reflog_locked(&repo, &name)?;
        }
    }
    let mut message: Option<String> = None;
    let mut file_msg: Option<PathBuf> = None;
    let mut template: Option<PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        if args[i] == "-m" {
            i += 1;
            let part = args
                .get(i)
                .ok_or_else(|| CliError::Usage("commit: -m requires a message".into()))?;
            message = Some(match message {
                Some(mut m) => {
                    m.push('\n');
                    m.push_str(part);
                    m
                }
                None => part.clone(),
            });
        } else if args[i] == "-F" || args[i] == "--file" {
            i += 1;
            let p = args
                .get(i)
                .ok_or_else(|| CliError::Usage("commit -F <file>".into()))?;
            file_msg = Some(PathBuf::from(p));
        } else if let Some(rest) = args[i].strip_prefix("--file=") {
            file_msg = Some(PathBuf::from(rest));
        } else if args[i] == "-t" || args[i] == "--template" {
            i += 1;
            let p = args
                .get(i)
                .ok_or_else(|| CliError::Usage("commit -t <file>".into()))?;
            template = Some(PathBuf::from(p));
        } else if let Some(rest) = args[i].strip_prefix("--template=") {
            template = Some(PathBuf::from(rest));
        }
        i += 1;
    }
    if message.is_none() {
        if let Some(path) = file_msg {
            let text = fs::read_to_string(&path)
                .map_err(|e| CliError::Msg(format!("commit -F {}: {e}", path.display())))?;
            message = Some(text);
        }
    }
    // O3: editor-driven commit when GIT_EDITOR / ARKS_EDITOR / EDITOR is set (scripted OK).
    if message.is_none() && !amend {
        message = Some(commit_message_via_editor(template.as_deref())?);
    }

    // Fs-5: `-a` / `--all` stages tracked updates then commits (git-commit).
    if all {
        cmd_add(&["-u".into()])?;
    }

    let mut repo = open_repo()?;
    run_claimed_hooks(&repo, no_verify, &["pre-commit"], None)?;

    let index = read_index(&repo)?;
    if index.entries.is_empty() && !allow_empty && !amend {
        return Err(CliError::Msg("nothing to commit (empty index)".into()));
    }

    let tree = if index.entries.is_empty() {
        let obj = RawObject::new(ObjectKind::Tree, Vec::new());
        let format = repo.object_format();
        repo.store_mut().put(format, &obj)?
    } else {
        let format = repo.object_format();
        write_tree_from_index(repo.store_mut(), &index.entries, format)?
    };

    if amend {
        let head = repo
            .head_oid()
            .map_err(|_| CliError::Msg("commit --amend: no HEAD to amend".into()))?;
        let pobj = repo.get_object(&head)?;
        if pobj.kind != ObjectKind::Commit {
            return Err(CliError::Msg("HEAD is not a commit".into()));
        }
        let prev = Commit::decode(&pobj.payload)?;
        let message = match message {
            Some(m) => {
                if m.ends_with('\n') {
                    m
                } else {
                    format!("{m}\n")
                }
            }
            None => prev.message.clone(),
        };
        run_claimed_hooks(&repo, no_verify, &["commit-msg"], Some(message.as_bytes()))?;
        if prev.tree == tree && message == prev.message && !allow_empty {
            return Err(CliError::Msg("nothing to commit (amend unchanged)".into()));
        }
        let sig = commit_signature()?;
        let subject = message.lines().next().unwrap_or("").to_owned();
        let (do_sign, sign_key) = want_sign_commit(args, &repo)?;
        let mut commit = Commit {
            tree,
            parents: prev.parents.clone(),
            author: prev.author.clone(),
            committer: sig,
            gpgsig: None,
            message,
        };
        if do_sign {
            sign_commit(&repo, &mut commit, sign_key.as_deref())?;
        }
        let cobj = RawObject::new(ObjectKind::Commit, commit.encode()?);
        let format = repo.object_format();
        let coid = repo.store_mut().put(format, &cobj)?;
        // M16 Sy-5: detached HEAD amend updates HEAD directly (git-commit).
        match repo.head_target()? {
            RefTarget::Symbolic(name) => {
                repo.refs_mut().update(&name, &coid, Some(&head))?;
                let _ = crate::reflog::append_head_and_branch(
                    &repo,
                    &name,
                    Some(&head),
                    &coid,
                    &format!("commit (amend): {subject}"),
                );
            }
            RefTarget::Direct(_) => {
                repo.refs_mut().update("HEAD", &coid, Some(&head))?;
                let _ = crate::reflog::append_reflog(
                    &repo,
                    "HEAD",
                    Some(&head),
                    &coid,
                    &format!("commit (amend): {subject}"),
                );
            }
        }
        println!("[{}] {}", current_branch_name(&repo)?, coid.to_hex());
        return Ok(());
    }

    let message = message.ok_or_else(|| CliError::Usage("commit: -m <msg> required".into()))?;
    let message = if message.ends_with('\n') {
        message
    } else {
        format!("{message}\n")
    };
    run_claimed_hooks(&repo, no_verify, &["commit-msg"], Some(message.as_bytes()))?;
    let parents = match repo.head_oid() {
        Ok(oid) => vec![oid],
        Err(_) => Vec::new(),
    };
    if let Some(parent) = parents.first() {
        let pobj = repo.get_object(parent)?;
        if pobj.kind == ObjectKind::Commit {
            let pc = Commit::decode(&pobj.payload)?;
            if pc.tree == tree && !allow_empty {
                return Err(CliError::Msg("nothing to commit (tree unchanged)".into()));
            }
        }
    }
    let sig = commit_signature()?;
    let subject = message.lines().next().unwrap_or("").to_owned();
    let (do_sign, sign_key) = want_sign_commit(args, &repo)?;
    let mut commit = Commit {
        tree,
        parents: parents.clone(),
        author: sig.clone(),
        committer: sig,
        gpgsig: None,
        message,
    };
    if do_sign {
        sign_commit(&repo, &mut commit, sign_key.as_deref())?;
    }
    let cobj = RawObject::new(ObjectKind::Commit, commit.encode()?);
    let format = repo.object_format();
    let coid = repo.store_mut().put(format, &cobj)?;
    // M16 Sy-5: detached HEAD commit updates HEAD directly (git-commit).
    match repo.head_target()? {
        RefTarget::Symbolic(name) => {
            let old = parents.first().copied();
            repo.refs_mut().update(&name, &coid, old.as_ref())?;
            let _ = crate::reflog::append_head_and_branch(
                &repo,
                &name,
                old.as_ref(),
                &coid,
                &format!("commit: {subject}"),
            );
        }
        RefTarget::Direct(_) => {
            let old = parents.first().copied();
            repo.refs_mut().update("HEAD", &coid, old.as_ref())?;
            let _ = crate::reflog::append_reflog(
                &repo,
                "HEAD",
                old.as_ref(),
                &coid,
                &format!("commit: {subject}"),
            );
        }
    }
    println!("[{}] {}", current_branch_name(&repo)?, coid.to_hex());
    Ok(())
}

pub fn cmd_log(args: &[String]) -> CliResult<()> {
    // M16 Sy-1: `--oneline`, `--graph`, `-n`/`-<n>` claimed (git-log).
    // Cite: https://git-scm.com/docs/git-log
    let oneline = args.iter().any(|a| a == "--oneline");
    let graph = args.iter().any(|a| a == "--graph");
    let mut max_count: Option<usize> = None;
    let mut filtered: Vec<String> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        if a == "-n" {
            i += 1;
            let n = args
                .get(i)
                .ok_or_else(|| CliError::Usage("log -n <count>".into()))?;
            max_count = Some(
                n.parse()
                    .map_err(|_| CliError::Usage("log -n: count must be integer".into()))?,
            );
        } else if let Some(rest) = a.strip_prefix("-n") {
            if !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_digit()) {
                max_count = Some(
                    rest.parse()
                        .map_err(|_| CliError::Usage("log -n: count must be integer".into()))?,
                );
            } else {
                filtered.push(a.clone());
            }
        } else if a.starts_with('-')
            && a.len() > 1
            && a.as_bytes()[1].is_ascii_digit()
            && !a.starts_with("--")
        {
            max_count = Some(
                a[1..]
                    .parse()
                    .map_err(|_| CliError::Usage("log -<n>: count must be integer".into()))?,
            );
        } else {
            filtered.push(a.clone());
        }
        i += 1;
    }
    refuse_unknown_flags(&filtered, &["--oneline", "--graph"])?;
    let repo = open_repo()?;
    let mut oid = match repo.head_oid() {
        Ok(o) => o,
        Err(_) => return Ok(()),
    };
    let mut shown = 0usize;
    loop {
        if let Some(max) = max_count {
            if shown >= max {
                break;
            }
        }
        let obj = repo.get_object(&oid)?;
        if obj.kind != ObjectKind::Commit {
            return Err(CliError::Msg(format!(
                "rev {} is not a commit",
                oid.to_hex()
            )));
        }
        let c = Commit::decode(&obj.payload)?;
        let short = &oid.to_hex()[..7];
        if oneline || graph {
            let prefix = if graph { "* " } else { "" };
            println!("{prefix}{short} {}", c.subject());
        } else {
            println!("{} {}", oid.to_hex(), c.subject());
        }
        shown += 1;
        match c.parents.first() {
            Some(p) => oid = *p,
            None => break,
        }
    }
    Ok(())
}

pub fn cmd_show(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &[])?;
    let rev = args.first().map(String::as_str).unwrap_or("HEAD");
    let repo = open_repo()?;
    let oid = resolve_rev(&repo, rev)?;
    let obj = repo.get_object(&oid)?;
    match obj.kind {
        ObjectKind::Commit => {
            let c = Commit::decode(&obj.payload)?;
            println!("commit {}", oid.to_hex());
            println!("tree {}", c.tree.to_hex());
            for p in &c.parents {
                println!("parent {}", p.to_hex());
            }
            println!("author {}", c.author.format_line());
            println!("committer {}", c.committer.format_line());
            println!();
            print!("{}", c.message);
        }
        ObjectKind::Blob => {
            io::Write::write_all(&mut io::stdout(), &obj.payload)?;
        }
        other => {
            println!("{} {}", other.as_str(), oid.to_hex());
        }
    }
    Ok(())
}

/// O3: editor-required commit message via `-t` template + GIT_EDITOR / ARKS_EDITOR / EDITOR.
fn commit_message_via_editor(template: Option<&Path>) -> CliResult<String> {
    let editor = std::env::var("ARKS_EDITOR")
        .or_else(|_| std::env::var("GIT_EDITOR"))
        .or_else(|_| std::env::var("EDITOR"))
        .map_err(|_| {
            CliError::NotImplemented(
                "commit without -m/-F requires GIT_EDITOR / ARKS_EDITOR / EDITOR (O3 scripted); pass -m or set editor"
                    .into(),
            )
        })?;
    let repo = open_repo()?;
    let msg_path = repo.git_dir().join("COMMIT_EDITMSG");
    let mut seed = String::new();
    if let Some(t) = template {
        seed = fs::read_to_string(t)
            .map_err(|e| CliError::Msg(format!("commit -t {}: {e}", t.display())))?;
    }
    if !seed.ends_with('\n') && !seed.is_empty() {
        seed.push('\n');
    }
    seed.push_str(
        "\n# Please enter the commit message for your changes.\n# Lines starting with '#' are ignored (O3 scripted editor).\n",
    );
    fs::write(&msg_path, &seed)?;
    drop(repo);
    run_editor_cmd(&editor, &msg_path)?;
    let text = fs::read_to_string(&msg_path)?;
    let cleaned = text
        .lines()
        .filter(|l| !l.starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");
    if cleaned.trim().is_empty() {
        return Err(CliError::Msg(
            "aborting commit due to empty commit message".into(),
        ));
    }
    Ok(cleaned)
}

fn run_editor_cmd(editor: &str, path: &Path) -> CliResult<()> {
    let mut argv: Vec<String> = editor.split_whitespace().map(|s| s.to_string()).collect();
    if argv.is_empty() {
        return Err(CliError::Usage("empty editor command".into()));
    }
    argv.push(path.display().to_string());
    let status = std::process::Command::new(&argv[0])
        .args(&argv[1..])
        .status()
        .map_err(|e| CliError::Msg(format!("spawn editor '{editor}': {e}")))?;
    if !status.success() {
        return Err(CliError::Msg(format!(
            "editor '{editor}' exited {}",
            status.code().unwrap_or(-1)
        )));
    }
    Ok(())
}

/// FS6: run `filter.<name>.clean` when `.gitattributes` assigns `filter=<name>` (LFS closest).
fn apply_clean_filter(work: &Path, rel: &str, payload: &[u8]) -> CliResult<Option<Vec<u8>>> {
    let Some(name) = attribute_filter_name(work, rel)? else {
        return Ok(None);
    };
    let repo = open_repo()?;
    let cfg = fs::read_to_string(repo.git_dir().join("config")).unwrap_or_default();
    let key = format!("filter.{name}.clean");
    let Some(cmd) = config_value(&cfg, &key) else {
        return Ok(None);
    };
    let mut argv: Vec<String> = cmd.split_whitespace().map(|s| s.to_string()).collect();
    if argv.is_empty() {
        return Ok(None);
    }
    for a in &mut argv {
        *a = a.replace("%f", rel);
    }
    let mut child = std::process::Command::new(&argv[0])
        .args(&argv[1..])
        .current_dir(work)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::inherit())
        .spawn()
        .map_err(|e| CliError::Msg(format!("filter.{name}.clean: {e}")))?;
    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        stdin
            .write_all(payload)
            .map_err(|e| CliError::Msg(format!("filter clean stdin: {e}")))?;
    }
    let out = child
        .wait_with_output()
        .map_err(|e| CliError::Msg(format!("filter clean: {e}")))?;
    if !out.status.success() {
        return Err(CliError::Msg(format!(
            "filter.{name}.clean exited {}",
            out.status.code().unwrap_or(-1)
        )));
    }
    Ok(Some(out.stdout))
}

fn attribute_filter_name(work: &Path, rel: &str) -> CliResult<Option<String>> {
    let attr = work.join(".gitattributes");
    if !attr.is_file() {
        return Ok(None);
    }
    let text = fs::read_to_string(&attr)?;
    let file = Path::new(rel)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(rel);
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split_whitespace();
        let pattern = parts.next().unwrap_or("");
        let matches = pattern == "*"
            || pattern == rel
            || pattern == file
            || (pattern.starts_with("*.") && file.ends_with(&pattern[1..]));
        if !matches {
            continue;
        }
        for attr in parts {
            if let Some(name) = attr.strip_prefix("filter=") {
                return Ok(Some(name.to_string()));
            }
        }
    }
    Ok(None)
}

fn config_value(text: &str, key: &str) -> Option<String> {
    let key = key.to_ascii_lowercase();
    let mut section = String::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if line.starts_with('[') {
            section = line
                .trim_matches(|c| c == '[' || c == ']')
                .to_ascii_lowercase()
                .replace('"', "");
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let full = if section.contains(' ') {
            let mut bits = section.split_whitespace();
            let head = bits.next().unwrap_or("");
            let name = bits.next().unwrap_or("");
            format!("{head}.{name}.{}", k.trim().to_ascii_lowercase())
        } else {
            format!("{section}.{}", k.trim().to_ascii_lowercase())
        };
        if full == key {
            return Some(v.trim().trim_matches('"').to_string());
        }
    }
    None
}
