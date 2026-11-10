//! Replay porcelain: cherry-pick / revert / rebase (clean path + Sy-5 sequencer).
//!
//! Cite: <https://git-scm.com/docs/git-cherry-pick/2.39.0>
//! Cite: <https://git-scm.com/docs/git-revert/2.39.0>
//! Cite: <https://git-scm.com/docs/git-rebase/2.39.0>
//! Ranges: `A..B` = first-parent commits reachable from B not from A (oldest first).
//! Conflicts: worktree markers + index stages + `.git/sequencer` + `CHERRY_PICK_HEAD`.
//!
//! M19 Cx-1: SEQ-001/002/003 + REF-018 — refuse concurrent sequencer / pseudoref
//! mutation when merge/rebase/cherry-pick state is already present (fail-closed;
//! no invented async multi-sequencer dialect). Catalog:
//! `.spec/sot/multi-agent-git-concurrency-scenarios.sot.md`.

use crate::cmd::shared::{
    commit_signature, current_branch_name, flatten_tree, head_blob_map, index_stat_matches,
    resolve_rev,
};
use crate::error::{CliError, CliResult};
use crate::util::{open_repo, read_index, refuse_unknown_flags, work_tree, write_index};
use crate::write_tree::write_tree_from_index;
use arks_index::{Index, IndexEntry, sort_entries};
use arks_object::{Commit, ObjectKind, RawObject, Signature};
use arks_oid::Oid;
use arks_refs::RefTarget;
use arks_repo::Repository;
use arks_store::ObjectStore;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
pub(crate) struct PathDelta {
    pub(crate) path: String,
    /// Blob in the base tree; absent when the path does not exist there.
    pub(crate) base: Option<Oid>,
    /// Blob in the tip tree; absent when the path is deleted.
    pub(crate) tip: Option<Oid>,
}

pub fn cmd_cherry_pick(args: &[String]) -> CliResult<()> {
    // M16 Sy-5: ranges, --no-commit, conflict markers, --continue/--abort.
    if args.iter().any(|a| a == "--continue") {
        refuse_unknown_flags(args, &["--continue"])?;
        return cherry_pick_continue();
    }
    if args.iter().any(|a| a == "--abort") {
        refuse_unknown_flags(args, &["--abort"])?;
        return cherry_pick_abort();
    }
    refuse_unknown_flags(args, &["--no-commit", "-n"])?;
    let no_commit = args.iter().any(|a| a == "--no-commit" || a == "-n");
    let rev = one_rev(args, "cherry-pick <commit>|A..B")?;
    let repo = open_repo()?;
    refuse_exclusive_sequencer_state(&repo, SequencerOp::CherryPick)?;
    let commits = resolve_pick_list(&repo, rev)?;
    if commits.is_empty() {
        return Err(CliError::Msg("cherry-pick: empty revision range".into()));
    }
    drop(repo);
    if commits.len() > 1 {
        write_sequencer_head()?;
    }
    for (i, oid) in commits.iter().enumerate() {
        // Todo stores commits after the one about to apply (continue must not re-pick).
        if commits.len() > 1 {
            advance_sequencer_todo(i + 1, &commits)?;
        }
        match cherry_pick_oid(oid, no_commit) {
            Ok(()) => {}
            Err(e) => {
                if is_conflict_error(&e) {
                    return Err(e);
                }
                clear_sequencer_state()?;
                return Err(e);
            }
        }
    }
    clear_sequencer_state()?;
    Ok(())
}

pub fn cmd_revert(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &[])?;
    let rev = one_rev(args, "revert <commit>|A..B")?;
    let repo = open_repo()?;
    let commits = resolve_pick_list(&repo, rev)?;
    if commits.is_empty() {
        return Err(CliError::Msg("revert: empty revision range".into()));
    }
    // Revert applies newest-first for ranges (git-revert).
    let mut commits = commits;
    commits.reverse();
    drop(repo);
    for oid in &commits {
        let repo = open_repo()?;
        let head_map = ensure_clean(&repo)?;
        let commit = read_commit(&repo, oid)?;
        if commit.parents.len() > 1 {
            return Err(CliError::NotImplemented(
                "revert of a merge commit is not claimed".into(),
            ));
        }
        let deltas = inverse_deltas(&repo, &commit)?;
        let message = format!(
            "Revert \"{}\"\n\nThis reverts commit {}.\n",
            commit.subject(),
            oid.to_hex()
        );
        let author = commit_signature()?;
        let committer = author.clone();
        drop(repo);
        apply_checked(&head_map, &deltas, message, author, committer)?;
    }
    Ok(())
}

pub fn cmd_rebase(args: &[String]) -> CliResult<()> {
    let interactive = args.iter().any(|a| a == "-i" || a == "--interactive");
    if interactive {
        refuse_unknown_flags(args, &["-i", "--interactive"])?;
        let filtered: Vec<String> = args
            .iter()
            .filter(|a| *a != "-i" && *a != "--interactive")
            .cloned()
            .collect();
        let upstream = one_rev(&filtered, "rebase -i <upstream>")?;
        if upstream.contains("..") {
            return Err(CliError::NotImplemented(
                "rebase -i ranges are not claimed; pass a single upstream".into(),
            ));
        }
        return rebase_interactive(upstream);
    }
    refuse_unknown_flags(args, &[])?;
    let upstream = one_rev(args, "rebase <upstream>")?;
    if upstream.contains("..") {
        return Err(CliError::NotImplemented(
            "rebase ranges are not claimed; pass a single upstream".into(),
        ));
    }
    rebase_onto(upstream)
}

/// Interactive rebase (O3 scripted + FS1 editor fallback):
/// Prefer `ARKS_SEQUENCE_EDITOR` / `GIT_SEQUENCE_EDITOR`, else ordinary editor
/// (`ARKS_EDITOR` / `GIT_EDITOR` / `EDITOR`) — stock git `GIT_SEQUENCE_EDITOR` fallback.
/// Claimed todo verbs: `pick`, `drop` (or `#` comment), `reword` (message via editor env).
fn rebase_interactive(upstream_rev: &str) -> CliResult<()> {
    let editor = std::env::var("ARKS_SEQUENCE_EDITOR")
        .or_else(|_| std::env::var("GIT_SEQUENCE_EDITOR"))
        .or_else(|_| std::env::var("ARKS_EDITOR"))
        .or_else(|_| std::env::var("GIT_EDITOR"))
        .or_else(|_| std::env::var("EDITOR"))
        .map_err(|_| {
            CliError::Usage(
                "rebase -i requires a sequence/editor: set GIT_SEQUENCE_EDITOR / ARKS_SEQUENCE_EDITOR or GIT_EDITOR / ARKS_EDITOR / EDITOR (FS1)"
                    .into(),
            )
        })?;
    let repo = open_repo()?;
    refuse_exclusive_sequencer_state(&repo, SequencerOp::Rebase)?;
    require_branch(&repo)?;
    let head = repo.head_oid()?;
    let upstream = resolve_rev(&repo, upstream_rev)?;
    let _ = read_commit(&repo, &upstream)?;
    if head == upstream {
        println!("Already up to date.");
        return Ok(());
    }
    let commits = replay_list(&repo, head, upstream)?;
    if commits.is_empty() {
        drop(repo);
        reset_hard(&upstream)?;
        println!("Fast-forward");
        return Ok(());
    }
    let _ = ensure_clean(&repo)?;
    let git = repo.git_dir().to_path_buf();
    let merge_dir = git.join("rebase-merge");
    fs::create_dir_all(&merge_dir)?;
    fs::write(merge_dir.join("interactive"), b"")?;
    fs::write(merge_dir.join("onto"), format!("{}\n", upstream.to_hex()))?;
    fs::write(merge_dir.join("head-name"), b"attached")?;
    let mut todo =
        String::from("# arks rebase -i (O3/FS1)\n# Commands: pick | drop | reword | # comment\n");
    for oid in &commits {
        let c = read_commit(&repo, oid)?;
        let subj = c.message.lines().next().unwrap_or("").trim();
        todo.push_str(&format!("pick {} {}\n", oid.to_hex(), subj));
    }
    let todo_path = merge_dir.join("git-rebase-todo");
    fs::write(&todo_path, &todo)?;
    drop(repo);
    run_external_editor(&editor, &todo_path)?;
    let edited = fs::read_to_string(&todo_path)?;
    let steps = parse_rebase_todo(&edited)?;
    if steps.is_empty() {
        let _ = fs::remove_dir_all(&merge_dir);
        reset_hard(&upstream)?;
        println!("Successfully rebased and updated (empty todo).");
        return Ok(());
    }
    ensure_replay_applies(
        &open_repo()?,
        upstream,
        &steps.iter().map(|s| s.oid).collect::<Vec<_>>(),
    )?;
    reset_hard(&upstream)?;
    for step in &steps {
        match step.verb {
            RebaseVerb::Pick => cherry_pick_oid(&step.oid, false)?,
            RebaseVerb::Drop => {}
            RebaseVerb::Reword => {
                cherry_pick_oid(&step.oid, false)?;
                reword_head_message()?;
            }
        }
    }
    let _ = fs::remove_dir_all(&merge_dir);
    println!("Successfully rebased.");
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RebaseVerb {
    Pick,
    Drop,
    Reword,
}

#[derive(Debug, Clone)]
struct RebaseTodoStep {
    verb: RebaseVerb,
    oid: Oid,
}

fn parse_rebase_todo(text: &str) -> CliResult<Vec<RebaseTodoStep>> {
    let mut out = Vec::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split_whitespace();
        let verb_s = parts.next().unwrap_or("");
        let oid_s = parts
            .next()
            .ok_or_else(|| CliError::Usage(format!("rebase -i todo: missing oid in '{line}'")))?;
        let verb = match verb_s {
            "pick" | "p" => RebaseVerb::Pick,
            "drop" | "d" => RebaseVerb::Drop,
            "reword" | "r" => RebaseVerb::Reword,
            other => {
                return Err(CliError::NotImplemented(format!(
                    "rebase -i todo verb '{other}' not claimed (O3: pick|drop|reword)"
                )));
            }
        };
        if verb == RebaseVerb::Drop {
            continue;
        }
        let oid = Oid::from_hex(oid_s).map_err(|e| CliError::Msg(e.to_string()))?;
        out.push(RebaseTodoStep { verb, oid });
    }
    Ok(out)
}

fn run_external_editor(editor: &str, path: &Path) -> CliResult<()> {
    // Allow `true` / `:` no-op editors and `cmd args` forms used in scripted CI.
    let mut argv = shell_words(editor);
    if argv.is_empty() {
        return Err(CliError::Usage("empty sequence/editor command".into()));
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

fn shell_words(s: &str) -> Vec<String> {
    // Minimal split: whitespace; keep it deterministic for scripted env values.
    s.split_whitespace().map(|w| w.to_string()).collect()
}

fn reword_head_message() -> CliResult<()> {
    let editor = std::env::var("ARKS_EDITOR")
        .or_else(|_| std::env::var("GIT_EDITOR"))
        .or_else(|_| std::env::var("EDITOR"))
        .map_err(|_| {
            CliError::NotImplemented(
                "rebase -i reword requires GIT_EDITOR / ARKS_EDITOR / EDITOR (O3 scripted)".into(),
            )
        })?;
    let repo = open_repo()?;
    let head = repo.head_oid()?;
    let commit = read_commit(&repo, &head)?;
    let msg_path = repo.git_dir().join("COMMIT_EDITMSG");
    fs::write(&msg_path, ensure_nl(commit.message.clone()))?;
    drop(repo);
    run_external_editor(&editor, &msg_path)?;
    let new_msg = fs::read_to_string(&msg_path)?;
    let new_msg = strip_commit_editmsg_comments(&new_msg);
    if new_msg.trim().is_empty() {
        return Err(CliError::Msg("aborting reword due to empty message".into()));
    }
    // Amend HEAD message in place via existing commit --amend path.
    crate::cmd::core::cmd_commit(&[
        "--amend".into(),
        "-m".into(),
        new_msg.trim_end().to_string(),
    ])
}

fn strip_commit_editmsg_comments(text: &str) -> String {
    text.lines()
        .filter(|l| !l.starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Non-interactive rebase of `HEAD` onto `upstream_rev` (clean apply only).
pub(crate) fn rebase_onto(upstream_rev: &str) -> CliResult<()> {
    let repo = open_repo()?;
    refuse_exclusive_sequencer_state(&repo, SequencerOp::Rebase)?;
    require_branch(&repo)?;
    let head = repo.head_oid()?;
    let upstream = resolve_rev(&repo, upstream_rev)?;
    let _ = read_commit(&repo, &upstream)?;
    if head == upstream {
        println!("Already up to date.");
        return Ok(());
    }
    let commits = replay_list(&repo, head, upstream)?;
    if commits.is_empty() {
        drop(repo);
        reset_hard(&upstream)?;
        println!("Fast-forward");
        return Ok(());
    }
    let _ = ensure_clean(&repo)?;
    ensure_replay_applies(&repo, upstream, &commits)?;
    drop(repo);
    reset_hard(&upstream)?;
    for oid in &commits {
        cherry_pick_oid(oid, false)?;
    }
    println!("Successfully rebased.");
    Ok(())
}

pub(crate) fn cherry_pick_oid(oid: &Oid, no_commit: bool) -> CliResult<()> {
    let repo = open_repo()?;
    let head_map = ensure_clean(&repo)?;
    let commit = read_commit(&repo, oid)?;
    if commit.parents.len() > 1 {
        return Err(CliError::NotImplemented(
            "cherry-pick of a merge commit is not claimed".into(),
        ));
    }
    let deltas = deltas_for_commit(&repo, &commit)?;
    let message = ensure_nl(commit.message.clone());
    let author = commit.author.clone();
    let committer = commit_signature()?;
    drop(repo);
    apply_checked_opts(
        &head_map,
        &deltas,
        message,
        author,
        committer,
        Some(*oid),
        no_commit,
    )?;
    Ok(())
}

/// Apply deltas. When `picked` is `Some`, conflicts write sequencer + CHERRY_PICK_HEAD.
pub(crate) fn apply_checked(
    head_map: &HashMap<String, Oid>,
    deltas: &[PathDelta],
    message: String,
    author: Signature,
    committer: Signature,
) -> CliResult<Oid> {
    apply_checked_opts(head_map, deltas, message, author, committer, None, false)
}

fn apply_checked_opts(
    head_map: &HashMap<String, Oid>,
    deltas: &[PathDelta],
    message: String,
    author: Signature,
    committer: Signature,
    picked: Option<Oid>,
    no_commit: bool,
) -> CliResult<Oid> {
    let mut clean: Vec<&PathDelta> = Vec::new();
    let mut conflicts: Vec<&PathDelta> = Vec::new();
    for delta in deltas {
        let have = head_map.get(&delta.path).copied();
        if have == delta.base {
            clean.push(delta);
        } else {
            conflicts.push(delta);
        }
    }
    if !conflicts.is_empty() {
        let Some(picked_oid) = picked else {
            return Err(CliError::Msg(format!(
                "conflict on '{}': clean path apply refused",
                conflicts[0].path
            )));
        };
        write_cherry_pick_conflict(head_map, &clean, &conflicts, &picked_oid, &message)?;
        return Err(CliError::Msg(format!(
            "cherry-pick: conflicts on {}; fix conflicts then `arks cherry-pick --continue` or `--abort`",
            conflicts
                .iter()
                .map(|d| d.path.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )));
    }
    let repo = open_repo()?;
    refuse_untracked_overwrites(&repo, head_map, deltas)?;
    drop(repo);
    let result = apply_deltas(head_map, deltas);
    if &result == head_map {
        return Err(CliError::Msg("nothing to commit (tree unchanged)".into()));
    }
    if no_commit {
        materialize_tree_only(&result)?;
        return Ok(picked.unwrap_or_else(|| {
            result.values().next().copied().unwrap_or_else(|| {
                Oid::from_hex("0000000000000000000000000000000000000000").unwrap()
            })
        }));
    }
    commit_tree(&result, message, author, committer)
}

fn is_conflict_error(e: &CliError) -> bool {
    match e {
        CliError::Msg(m) => m.contains("cherry-pick: conflicts"),
        _ => false,
    }
}

fn write_cherry_pick_conflict(
    head_map: &HashMap<String, Oid>,
    clean: &[&PathDelta],
    conflicts: &[&PathDelta],
    picked: &Oid,
    message: &str,
) -> CliResult<()> {
    let mut repo = open_repo()?;
    let work = work_tree(&repo)?;
    // Preserve ORIG_HEAD / sequencer head once.
    if let Ok(head) = repo.head_oid() {
        let _ = fs::write(
            repo.git_dir().join("ORIG_HEAD"),
            format!("{}\n", head.to_hex()),
        );
        let seq = sequencer_dir(&repo);
        fs::create_dir_all(&seq)?;
        if !seq.join("head").is_file() {
            fs::write(seq.join("head"), format!("{}\n", head.to_hex()))?;
        }
        fs::write(seq.join("message"), message)?;
    }
    fs::write(
        repo.git_dir().join("CHERRY_PICK_HEAD"),
        format!("{}\n", picked.to_hex()),
    )?;

    let mut result = head_map.clone();
    for d in clean {
        match d.tip {
            Some(oid) => {
                result.insert(d.path.clone(), oid);
            }
            None => {
                result.remove(&d.path);
            }
        }
    }

    let mut entries: Vec<IndexEntry> = Vec::new();
    // Stage-0 for non-conflict paths (post-apply result ∪ unchanged).
    let conflict_paths: HashSet<&str> = conflicts.iter().map(|d| d.path.as_str()).collect();
    for (path, oid) in &result {
        if conflict_paths.contains(path.as_str()) {
            continue;
        }
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
            size: u32::try_from(blob.payload.len()).unwrap_or(u32::MAX),
            oid: *oid,
            flags: (path.len().min(0x0fff)) as u16,
            flags_extended: None,
            path: path.clone(),
        });
        let abs = work.join(path);
        if let Some(parent) = abs.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&abs, &blob.payload)?;
    }
    for d in conflicts {
        // Stage 1 = base (expected), 2 = ours (HEAD), 3 = theirs (tip).
        if let Some(base) = d.base {
            entries.push(stage_entry(&d.path, base, 1));
        }
        if let Some(ours) = head_map.get(&d.path).copied() {
            entries.push(stage_entry(&d.path, ours, 2));
        }
        if let Some(theirs) = d.tip {
            entries.push(stage_entry(&d.path, theirs, 3));
        }
        write_conflict_markers(&repo, &work, &d.path, head_map.get(&d.path).copied(), d.tip)?;
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
    let _ = &mut repo;
    Ok(())
}

fn stage_entry(path: &str, oid: Oid, stage: u16) -> IndexEntry {
    let flags = ((stage & 3) << 12) | ((path.len().min(0x0fff)) as u16);
    IndexEntry {
        ctime_sec: 0,
        ctime_nsec: 0,
        mtime_sec: 0,
        mtime_nsec: 0,
        dev: 0,
        ino: 0,
        mode: 0o100644,
        uid: 0,
        gid: 0,
        size: 0,
        oid,
        flags,
        flags_extended: None,
        path: path.to_owned(),
    }
}

fn write_conflict_markers(
    repo: &Repository,
    work: &Path,
    path: &str,
    ours: Option<Oid>,
    theirs: Option<Oid>,
) -> CliResult<()> {
    let mut body = String::new();
    body.push_str("<<<<<<< ours\n");
    if let Some(oid) = ours {
        let obj = repo.get_object(&oid)?;
        body.push_str(&String::from_utf8_lossy(&obj.payload));
        if !body.ends_with('\n') {
            body.push('\n');
        }
    }
    body.push_str("=======\n");
    if let Some(oid) = theirs {
        let obj = repo.get_object(&oid)?;
        body.push_str(&String::from_utf8_lossy(&obj.payload));
        if !body.ends_with('\n') {
            body.push('\n');
        }
    }
    body.push_str(">>>>>>> theirs\n");
    let abs = work.join(path);
    if let Some(parent) = abs.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(abs, body)?;
    Ok(())
}

fn materialize_tree_only(files: &HashMap<String, Oid>) -> CliResult<()> {
    let repo = open_repo()?;
    let work = work_tree(&repo)?;
    let prior = read_index(&repo).unwrap_or(Index {
        version: 2,
        entries: Vec::new(),
        extensions: Vec::new(),
    });
    let mut prior_map: HashMap<String, IndexEntry> = HashMap::new();
    for e in prior.entries.into_iter().filter(|e| e.stage() == 0) {
        prior_map.insert(e.path.clone(), e);
    }
    let mut paths: Vec<&String> = files.keys().collect();
    paths.sort();
    let mut entries = Vec::with_capacity(paths.len());
    let mut kept: HashSet<String> = HashSet::with_capacity(paths.len());
    for path in paths {
        let oid = files[path];
        kept.insert(path.clone());
        let abs = work.join(path);
        if let Some(prev) = prior_map.get(path) {
            if prev.oid == oid && abs.is_file() {
                let mut e = prev.clone();
                e.path = path.clone();
                entries.push(e);
                continue;
            }
        }
        let blob = repo.get_object(&oid)?;
        if blob.kind != ObjectKind::Blob {
            return Err(CliError::Msg(format!("path '{path}' is not a blob")));
        }
        if let Some(parent) = abs.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&abs, &blob.payload)?;
        let (ctime_sec, ctime_nsec, mtime_sec, mtime_nsec, size) =
            stamp_path(&abs, blob.payload.len());
        entries.push(IndexEntry {
            ctime_sec,
            ctime_nsec,
            mtime_sec,
            mtime_nsec,
            dev: 0,
            ino: 0,
            mode: 0o100644,
            uid: 0,
            gid: 0,
            size,
            oid,
            flags: (path.len().min(0x0fff)) as u16,
            flags_extended: None,
            path: path.clone(),
        });
    }
    for path in prior_map.keys() {
        if kept.contains(path) {
            continue;
        }
        let p = work.join(path);
        if p.is_file() || p.is_symlink() {
            let _ = fs::remove_file(&p);
        }
    }
    write_index(
        &repo,
        &Index {
            version: 2,
            entries,
            extensions: Vec::new(),
        },
    )?;
    Ok(())
}

fn stamp_path(path: &Path, len: usize) -> (u32, u32, u32, u32, u32) {
    use std::time::{SystemTime, UNIX_EPOCH};
    let size = u32::try_from(len).unwrap_or(u32::MAX);
    let to_parts = |t: SystemTime| {
        let dur = t.duration_since(UNIX_EPOCH).unwrap_or_default();
        (dur.as_secs() as u32, dur.subsec_nanos())
    };
    match fs::metadata(path) {
        Ok(meta) => {
            let (mtime_sec, mtime_nsec) = meta.modified().ok().map(to_parts).unwrap_or((0, 0));
            let (ctime_sec, ctime_nsec) = meta
                .created()
                .ok()
                .map(to_parts)
                .unwrap_or((mtime_sec, mtime_nsec));
            let size = meta.len().min(u32::MAX as u64) as u32;
            (ctime_sec, ctime_nsec, mtime_sec, mtime_nsec, size)
        }
        Err(_) => (0, 0, 0, 0, size),
    }
}

fn cherry_pick_continue() -> CliResult<()> {
    let repo = open_repo()?;
    let git_dir = repo.git_dir().to_path_buf();
    let cherry_path = git_dir.join("CHERRY_PICK_HEAD");
    if !cherry_path.is_file() && !sequencer_dir(&repo).join("todo").is_file() {
        return Err(CliError::Msg(
            "cherry-pick --continue: no cherry-pick in progress".into(),
        ));
    }
    let index = read_index(&repo)?;
    if index.entries.iter().any(|e| e.stage() != 0) {
        return Err(CliError::Msg(
            "cherry-pick --continue: unmerged paths remain; resolve and stage first".into(),
        ));
    }
    let message = if sequencer_dir(&repo).join("message").is_file() {
        fs::read_to_string(sequencer_dir(&repo).join("message"))?
    } else if cherry_path.is_file() {
        let oid = Oid::from_hex(fs::read_to_string(&cherry_path)?.trim())?;
        let commit = read_commit(&repo, &oid)?;
        ensure_nl(commit.message)
    } else {
        return Err(CliError::Msg(
            "cherry-pick --continue: missing commit message".into(),
        ));
    };
    let author = if cherry_path.is_file() {
        let oid = Oid::from_hex(fs::read_to_string(&cherry_path)?.trim())?;
        read_commit(&repo, &oid)?.author
    } else {
        commit_signature()?
    };
    let committer = commit_signature()?;
    let files = {
        let mut map = HashMap::new();
        for e in index.entries.iter().filter(|e| e.stage() == 0) {
            map.insert(e.path.clone(), e.oid);
        }
        map
    };
    drop(repo);
    commit_tree(&files, message, author, committer)?;
    let repo = open_repo()?;
    let _ = fs::remove_file(repo.git_dir().join("CHERRY_PICK_HEAD"));
    // Continue remaining todo lines.
    let todo_path = sequencer_dir(&repo).join("todo");
    if todo_path.is_file() {
        let text = fs::read_to_string(&todo_path)?;
        let remaining: Vec<Oid> = text
            .lines()
            .filter_map(|line| {
                let line = line.trim();
                if line.is_empty() {
                    return None;
                }
                let hex = line.strip_prefix("pick ").unwrap_or(line);
                let hex = hex.split_whitespace().next()?;
                Oid::from_hex(hex).ok()
            })
            .collect();
        drop(repo);
        if remaining.is_empty() {
            clear_sequencer_state()?;
            return Ok(());
        }
        for (i, oid) in remaining.iter().enumerate() {
            advance_sequencer_todo(i + 1, &remaining)?;
            match cherry_pick_oid(oid, false) {
                Ok(()) => {}
                Err(e) => {
                    if is_conflict_error(&e) {
                        return Err(e);
                    }
                    clear_sequencer_state()?;
                    return Err(e);
                }
            }
        }
        clear_sequencer_state()?;
    } else {
        drop(repo);
        clear_sequencer_state()?;
    }
    Ok(())
}

fn cherry_pick_abort() -> CliResult<()> {
    let mut repo = open_repo()?;
    let seq = sequencer_dir(&repo);
    let head_file = seq.join("head");
    let orig = repo.git_dir().join("ORIG_HEAD");
    let restore = if head_file.is_file() {
        Oid::from_hex(fs::read_to_string(&head_file)?.trim())?
    } else if orig.is_file() {
        Oid::from_hex(fs::read_to_string(&orig)?.trim())?
    } else {
        return Err(CliError::Msg(
            "cherry-pick --abort: no cherry-pick in progress".into(),
        ));
    };
    match repo.head_target()? {
        RefTarget::Symbolic(name) => {
            let old = repo.head_oid().ok();
            repo.refs_mut().update(&name, &restore, old.as_ref())?;
        }
        RefTarget::Direct(_) => {
            let old = repo.head_oid().ok();
            repo.refs_mut().update("HEAD", &restore, old.as_ref())?;
        }
    }
    let work = work_tree(&repo)?;
    arks_remote::checkout_commit(&repo, &work, &restore)?;
    drop(repo);
    clear_sequencer_state()?;
    println!("cherry-pick aborted");
    Ok(())
}

fn sequencer_dir(repo: &Repository) -> PathBuf {
    repo.git_dir().join("sequencer")
}

#[derive(Clone, Copy)]
enum SequencerOp {
    Rebase,
    CherryPick,
}

/// Fail-closed when another sequencer / pseudoref critical section is live.
/// Cite: gitrepository-layout (MERGE_HEAD, CHERRY_PICK_HEAD, rebase-merge).
/// Catalog: SEQ-001, SEQ-002, SEQ-003, REF-018 (M19 Cx-1); SEQ-006 (Cx-2 interactive todo).
fn refuse_exclusive_sequencer_state(repo: &Repository, op: SequencerOp) -> CliResult<()> {
    // SEQ-004 (M20 Cx-6): rerere shared rr-cache writes under lock.
    let rr_lock = repo.common_dir().join("rr-cache.lock");
    if rr_lock.is_file() {
        return Err(CliError::Msg(format!(
            "{} (SEQ-004 — concurrent rerere rr-cache writes refuse; serialize conflict resolution)",
            rr_lock.display()
        )));
    }
    let git = repo.git_dir();
    let merge = git.join("MERGE_HEAD").is_file();
    let cherry =
        git.join("CHERRY_PICK_HEAD").is_file() || sequencer_dir(repo).join("todo").is_file();
    let rebase = git.join("rebase-merge").is_dir() || git.join("rebase-apply").is_dir();
    let interactive_todo = git.join("rebase-merge").join("git-rebase-todo").is_file()
        || git.join("rebase-merge").join("interactive").is_file();

    if merge {
        return Err(CliError::Msg(
            "merge in progress (MERGE_HEAD); refuse concurrent sequencer (SEQ-002 / REF-018)"
                .into(),
        ));
    }
    match op {
        SequencerOp::Rebase if rebase => {
            let cite = if interactive_todo {
                "SEQ-001 / SEQ-006"
            } else {
                "SEQ-001"
            };
            Err(CliError::Msg(format!(
                "rebase in progress (rebase-merge/rebase-apply); refuse concurrent rebase ({cite})"
            )))
        }
        SequencerOp::Rebase if cherry => {
            Err(CliError::Msg(
                "cherry-pick in progress; refuse concurrent rebase (SEQ-003 / REF-018)".into(),
            ))
        }
        SequencerOp::CherryPick if cherry => {
            Err(CliError::Msg(
                "cherry-pick in progress (CHERRY_PICK_HEAD/sequencer); refuse concurrent cherry-pick (SEQ-003)"
                    .into(),
            ))
        }
        SequencerOp::CherryPick if rebase => {
            Err(CliError::Msg(
                "rebase in progress; refuse concurrent cherry-pick (SEQ-001 / REF-018)".into(),
            ))
        }
        _ => Ok(()),
    }
}

fn write_sequencer_head() -> CliResult<()> {
    let repo = open_repo()?;
    let seq = sequencer_dir(&repo);
    fs::create_dir_all(&seq)?;
    if let Ok(head) = repo.head_oid() {
        fs::write(seq.join("head"), format!("{}\n", head.to_hex()))?;
        fs::write(
            repo.git_dir().join("ORIG_HEAD"),
            format!("{}\n", head.to_hex()),
        )?;
    }
    Ok(())
}

fn advance_sequencer_todo(done: usize, commits: &[Oid]) -> CliResult<()> {
    let repo = open_repo()?;
    let seq = sequencer_dir(&repo);
    if !seq.is_dir() {
        return Ok(());
    }
    let mut body = String::new();
    for oid in commits.iter().skip(done) {
        body.push_str(&format!("pick {}\n", oid.to_hex()));
    }
    fs::write(seq.join("todo"), body)?;
    Ok(())
}

fn clear_sequencer_state() -> CliResult<()> {
    let repo = open_repo()?;
    let git = repo.git_dir();
    let _ = fs::remove_file(git.join("CHERRY_PICK_HEAD"));
    let seq = git.join("sequencer");
    if seq.is_dir() {
        let _ = fs::remove_dir_all(&seq);
    }
    Ok(())
}

fn resolve_pick_list(repo: &Repository, rev: &str) -> CliResult<Vec<Oid>> {
    if let Some((left, right)) = rev.split_once("..") {
        if left.contains('.') || right.contains('.') {
            // Reject `...` here; cherry-pick uses A..B.
            if rev.contains("...") {
                return Err(CliError::NotImplemented(
                    "cherry-pick A...B not claimed; use A..B".into(),
                ));
            }
        }
        let start = if left.is_empty() {
            return Err(CliError::Usage(
                "cherry-pick A..B requires both ends".into(),
            ));
        } else {
            resolve_commit(repo, left)?
        };
        let end = if right.is_empty() {
            repo.head_oid()?
        } else {
            resolve_commit(repo, right)?
        };
        return replay_list(repo, end, start);
    }
    Ok(vec![resolve_commit(repo, rev)?])
}

pub(crate) fn ensure_applicable(
    head: &HashMap<String, Oid>,
    deltas: &[PathDelta],
) -> CliResult<()> {
    for delta in deltas {
        let have = head.get(&delta.path).copied();
        if have != delta.base {
            return Err(CliError::Msg(format!(
                "conflict on '{}': clean path apply refused",
                delta.path
            )));
        }
    }
    Ok(())
}

pub(crate) fn apply_deltas(
    head: &HashMap<String, Oid>,
    deltas: &[PathDelta],
) -> HashMap<String, Oid> {
    let mut out = head.clone();
    for delta in deltas {
        match delta.tip {
            Some(oid) => {
                out.insert(delta.path.clone(), oid);
            }
            None => {
                out.remove(&delta.path);
            }
        }
    }
    out
}

pub(crate) fn refuse_untracked_overwrites(
    repo: &Repository,
    head: &HashMap<String, Oid>,
    deltas: &[PathDelta],
) -> CliResult<()> {
    let work = work_tree(repo)?;
    for delta in deltas {
        if delta.tip.is_none() || head.contains_key(&delta.path) {
            continue;
        }
        let path = work.join(&delta.path);
        if path.exists() {
            return Err(CliError::Msg(format!(
                "untracked path '{}' would be overwritten",
                delta.path
            )));
        }
    }
    Ok(())
}

pub(crate) fn commit_tree(
    files: &HashMap<String, Oid>,
    message: String,
    author: Signature,
    committer: Signature,
) -> CliResult<Oid> {
    let mut repo = open_repo()?;
    let parent = repo.head_oid().ok();
    let entries = index_entries(&repo, files)?;
    let format = repo.object_format();
    let tree = write_tree_from_index(repo.store_mut(), &entries, format)?;
    let message = ensure_nl(message);
    let subject = message.lines().next().unwrap_or("").to_owned();
    let commit = Commit {
        tree,
        parents: parent.iter().copied().collect(),
        author,
        committer,
        gpgsig: None,
        message,
    };
    let obj = RawObject::new(ObjectKind::Commit, commit.encode()?);
    let format = repo.object_format();
    let oid = repo.store_mut().put(format, &obj)?;
    // M16 Sy-5: allow detached HEAD updates (same as git cherry-pick / commit).
    match repo.head_target()? {
        RefTarget::Symbolic(branch) => {
            repo.refs_mut().update(&branch, &oid, parent.as_ref())?;
            let _ = crate::reflog::append_head_and_branch(
                &repo,
                &branch,
                parent.as_ref(),
                &oid,
                &format!("commit: {subject}"),
            );
        }
        RefTarget::Direct(_) => {
            repo.refs_mut().update("HEAD", &oid, parent.as_ref())?;
            let _ = crate::reflog::append_reflog(
                &repo,
                "HEAD",
                parent.as_ref(),
                &oid,
                &format!("commit: {subject}"),
            );
        }
    }
    // Incremental worktree/index update from the flat result map — avoids a full
    // tree walk on every cherry-pick/rebase step (M17 Pf3-2).
    drop(repo);
    materialize_tree_only(files)?;
    let repo = open_repo()?;
    let label = current_branch_name(&repo)?;
    println!("[{label}] {}", oid.to_hex());
    Ok(oid)
}

pub(crate) fn index_entries(
    _repo: &Repository,
    files: &HashMap<String, Oid>,
) -> CliResult<Vec<IndexEntry>> {
    // Pf4-2: tree write only needs path+oid; size is stamped in materialize_tree_only.
    // Skipping N× get_object here cuts cherry-pick/rebase object IO.
    let mut paths: Vec<&String> = files.keys().collect();
    paths.sort();
    let mut entries = Vec::with_capacity(paths.len());
    for path in paths {
        let oid = files[path];
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
            size: 0,
            oid,
            flags: (path.len().min(0x0fff)) as u16,
            flags_extended: None,
            path: path.clone(),
        });
    }
    Ok(entries)
}

pub(crate) fn ensure_clean(repo: &Repository) -> CliResult<HashMap<String, Oid>> {
    let head_map = if repo.head_oid().is_ok() {
        head_blob_map(repo)?
    } else {
        HashMap::new()
    };
    let index = read_index(repo)?;
    let mut index_map = HashMap::new();
    for entry in index.entries.iter().filter(|e| e.stage() == 0) {
        if index_map.insert(entry.path.clone(), entry.oid).is_some() {
            return Err(CliError::Msg(format!(
                "duplicate index path '{}'",
                entry.path
            )));
        }
    }
    // Allow conflicted index only when CHERRY_PICK_HEAD is absent (normal clean check).
    if index.entries.iter().any(|e| e.stage() != 0) {
        return Err(CliError::Msg(
            "index has unmerged entries; clean apply refused".into(),
        ));
    }
    if index_map != head_map {
        return Err(CliError::Msg(
            "index does not match HEAD; clean apply refused".into(),
        ));
    }
    if repo.head_oid().is_ok() {
        let work = work_tree(repo)?;
        let by_path: HashMap<&str, &IndexEntry> = index
            .entries
            .iter()
            .filter(|e| e.stage() == 0)
            .map(|e| (e.path.as_str(), e))
            .collect();
        for (path, oid) in &head_map {
            let abs = work.join(path);
            if let Some(entry) = by_path.get(path.as_str()) {
                if entry.oid == *oid {
                    if let Ok(meta) = fs::metadata(&abs) {
                        if index_stat_matches(entry, &meta) {
                            continue;
                        }
                    }
                }
            }
            let blob = repo.get_object(oid)?;
            let bytes = fs::read(&abs)
                .map_err(|_| CliError::Msg(format!("worktree missing tracked path '{path}'")))?;
            if bytes != blob.payload {
                return Err(CliError::Msg(format!(
                    "worktree path '{path}' is dirty; clean apply refused"
                )));
            }
        }
    }
    Ok(head_map)
}

pub(crate) fn require_branch(repo: &Repository) -> CliResult<String> {
    match repo.head_target()? {
        RefTarget::Symbolic(name) => Ok(name),
        RefTarget::Direct(_) => Err(CliError::NotImplemented(
            "history edit on detached HEAD is not claimed".into(),
        )),
    }
}

pub(crate) fn reset_hard(oid: &Oid) -> CliResult<()> {
    let mut repo = open_repo()?;
    let branch = require_branch(&repo)?;
    let old = repo.head_oid().ok();
    repo.refs_mut().update(&branch, oid, old.as_ref())?;
    let work = work_tree(&repo)?;
    arks_remote::checkout_commit(&repo, &work, oid)?;
    Ok(())
}

pub(crate) fn read_commit(repo: &Repository, oid: &Oid) -> CliResult<Commit> {
    let obj = repo.get_object(oid)?;
    if obj.kind != ObjectKind::Commit {
        return Err(CliError::Msg(format!("{} is not a commit", oid.to_hex())));
    }
    Ok(Commit::decode(&obj.payload)?)
}

pub(crate) fn resolve_commit(repo: &Repository, rev: &str) -> CliResult<Oid> {
    let oid = resolve_rev(repo, rev)?;
    let _ = read_commit(repo, &oid)?;
    Ok(oid)
}

pub(crate) fn changed_paths(
    base: &HashMap<String, Oid>,
    tip: &HashMap<String, Oid>,
) -> Vec<PathDelta> {
    let mut names: Vec<String> = base.keys().cloned().collect();
    for key in tip.keys() {
        if !base.contains_key(key) {
            names.push(key.clone());
        }
    }
    names.sort();
    let mut out = Vec::new();
    for name in names {
        let base_oid = base.get(&name).copied();
        let tip_oid = tip.get(&name).copied();
        if base_oid != tip_oid {
            out.push(PathDelta {
                path: name,
                base: base_oid,
                tip: tip_oid,
            });
        }
    }
    out
}

pub(crate) fn deltas_for_commit(repo: &Repository, commit: &Commit) -> CliResult<Vec<PathDelta>> {
    let mut tip = HashMap::new();
    flatten_tree(repo, &commit.tree, "", &mut tip)?;
    let base = match commit.parents.first() {
        Some(parent) => tree_map(repo, parent)?,
        None => HashMap::new(),
    };
    Ok(changed_paths(&base, &tip))
}

pub(crate) fn inverse_deltas(repo: &Repository, commit: &Commit) -> CliResult<Vec<PathDelta>> {
    let mut base = HashMap::new();
    flatten_tree(repo, &commit.tree, "", &mut base)?;
    let tip = match commit.parents.first() {
        Some(parent) => tree_map(repo, parent)?,
        None => HashMap::new(),
    };
    Ok(changed_paths(&base, &tip))
}

/// First-parent commits reachable from `head` and not from `upstream`, oldest first.
pub(crate) fn replay_list(repo: &Repository, head: Oid, upstream: Oid) -> CliResult<Vec<Oid>> {
    let stop = first_parent_set(repo, upstream)?;
    let mut newest = Vec::new();
    let mut oid = head;
    loop {
        if stop.contains(&oid) {
            break;
        }
        let commit = read_commit(repo, &oid)?;
        if commit.parents.len() > 1 {
            return Err(CliError::NotImplemented(
                "rebase of merge commits is not claimed".into(),
            ));
        }
        newest.push(oid);
        match commit.parents.first() {
            Some(parent) => oid = *parent,
            None => break,
        }
    }
    newest.reverse();
    Ok(newest)
}

pub(crate) fn ensure_replay_applies(
    repo: &Repository,
    upstream: Oid,
    commits: &[Oid],
) -> CliResult<()> {
    let mut head_map = tree_map(repo, &upstream)?;
    for oid in commits {
        let commit = read_commit(repo, oid)?;
        if commit.parents.len() > 1 {
            return Err(CliError::NotImplemented(
                "rebase of merge commits is not claimed".into(),
            ));
        }
        let deltas = deltas_for_commit(repo, &commit)?;
        ensure_applicable(&head_map, &deltas)?;
        head_map = apply_deltas(&head_map, &deltas);
    }
    Ok(())
}

pub(crate) fn put_blob(repo: &mut Repository, bytes: &[u8]) -> CliResult<Oid> {
    let obj = RawObject::new(ObjectKind::Blob, bytes.to_vec());
    let format = repo.object_format();
    Ok(repo.store_mut().put(format, &obj)?)
}

pub(crate) fn ensure_nl(mut message: String) -> String {
    if !message.ends_with('\n') {
        message.push('\n');
    }
    message
}

pub(crate) fn one_rev<'a>(args: &'a [String], usage: &str) -> CliResult<&'a str> {
    let positional: Vec<&String> = args.iter().filter(|a| !a.starts_with('-')).collect();
    if positional.len() != 1 {
        return Err(CliError::Usage(usage.into()));
    }
    Ok(positional[0])
}

pub(crate) fn tree_map(repo: &Repository, commit_oid: &Oid) -> CliResult<HashMap<String, Oid>> {
    Ok(arks_repo::commit_tree_map(repo, commit_oid)?)
}

pub(crate) fn first_parent_set(repo: &Repository, tip: Oid) -> CliResult<HashSet<Oid>> {
    Ok(arks_repo::first_parent_set(repo, tip)?)
}
