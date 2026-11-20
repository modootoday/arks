//! format-patch / am — CLI wires `arks_object` mailbox and unified-diff parsing.

use crate::cmd::replay::{PathDelta, apply_checked, ensure_clean, put_blob};
use crate::cmd::shared::{commit_signature, committer_signature};
use crate::error::{CliError, CliResult};
use crate::util::{open_repo, refuse_unknown_flags};
use arks_object::{
    FilePatch, MailMessage, ObjectKind, apply_hunks, parse_mailbox, parse_unified_diff,
};
use arks_repo::Repository;
use std::fs;
use std::path::PathBuf;

pub fn cmd_am(args: &[String]) -> CliResult<()> {
    // M16 Sy-5: `--3way` when index blobs resolvable (git-am).
    // C7b: unified-diff hunks (any count, with offsets) applied to the HEAD
    // blobs; mailboxes with several messages commit one by one.
    // Cite: https://git-scm.com/docs/git-am
    refuse_unknown_flags(args, &["--3way", "-3", "-q", "--quiet"])?;
    let three_way = args.iter().any(|a| a == "--3way" || a == "-3");
    let path = args
        .iter()
        .find(|a| !a.starts_with('-'))
        .ok_or_else(|| CliError::Usage("am <file>".into()))?;
    let file = PathBuf::from(path);
    if file.is_dir() {
        return Err(CliError::NotImplemented(
            "am of a patch directory is not claimed".into(),
        ));
    }
    {
        let repo = open_repo()?;
        // SEQ-007 / MISC-007 (M19 Cx-2): shared rebase-apply / am state — refuse
        // concurrent am before parsing the patch (fail-closed on state, not on parse order).
        let git = repo.git_dir();
        if git.join("rebase-apply").is_dir() || git.join("rebase-merge").is_dir() {
            return Err(CliError::Msg(
                "am refused: sequencer/apply state present (SEQ-007 / MISC-007); no dual-am dialect"
                    .into(),
            ));
        }
    }
    let bytes = fs::read(&file)?;
    let text = std::str::from_utf8(&bytes)
        .map_err(|_| CliError::NotImplemented("am: non-utf8 patch is not claimed".into()))?;
    let messages = parse_mailbox(text)?;
    for (n, msg) in messages.iter().enumerate() {
        am_one(msg, n + 1, three_way)?;
    }
    Ok(())
}

/// Apply and commit one mailbox message on top of HEAD.
fn am_one(msg: &MailMessage, n: usize, three_way: bool) -> CliResult<()> {
    let mut repo = open_repo()?;
    let head_map = crate::cmd::shared::head_blob_map(&repo)?;
    let index = crate::util::read_index(&repo)?;
    let mut dirty: Vec<String> = index
        .entries
        .iter()
        .filter(|e| e.stage() != 0 || head_map.get(&e.path) != Some(&e.oid))
        .map(|e| e.path.clone())
        .collect();
    dirty.extend(
        head_map
            .keys()
            .filter(|p| !index.entries.iter().any(|e| &e.path == *p))
            .cloned(),
    );
    dirty.sort();
    dirty.dedup();
    if !dirty.is_empty() {
        return Err(CliError::fatal(format!(
            "Dirty index: cannot apply patches (dirty: {})",
            dirty.join(" ")
        )));
    }
    let head_map = ensure_clean(&repo)?;
    let subject = crate::cmd::log_format::subject_line(&msg.message);
    let files = parse_unified_diff(msg.raw.as_bytes())
        .map_err(|e| CliError::fatal(format!("{e}\nPatch failed at {n:04} {subject}")))?;
    let mut deltas = Vec::new();
    for fp in &files {
        let shown = fp.display_path().to_string();
        let failed = |line: usize| {
            CliError::Exit(
                128,
                format!(
                    "error: patch failed: {shown}:{line}\nerror: {shown}: patch does not apply\n\
                     Patch failed at {n:04} {subject}"
                ),
            )
        };
        if fp.binary {
            return Err(failed(0));
        }
        let base = fp.old_path.as_ref().and_then(|p| head_map.get(p)).copied();
        if fp.old_path.is_some() && base.is_none() && !three_way {
            return Err(failed(0));
        }
        let base_bytes = match base {
            Some(o) => repo.get_object(&o)?.payload,
            None => Vec::new(),
        };
        let new_bytes = match apply_hunks(&base_bytes, fp) {
            Ok(b) => b,
            Err(line) if three_way => three_way_tip(&repo, fp).ok_or_else(|| failed(line))?,
            Err(line) => return Err(failed(line)),
        };
        let tip = match &fp.new_path {
            Some(_) => Some(put_blob(&mut repo, &new_bytes)?),
            None => None,
        };
        match (&fp.old_path, &fp.new_path) {
            (Some(op), Some(np)) if op != np => {
                deltas.push(PathDelta {
                    path: op.clone(),
                    base,
                    tip: None,
                });
                deltas.push(PathDelta {
                    path: np.clone(),
                    base: head_map.get(np).copied(),
                    tip,
                });
            }
            _ => {
                if base != tip {
                    deltas.push(PathDelta {
                        path: shown.clone(),
                        base,
                        tip,
                    });
                }
            }
        }
    }
    let mut author = commit_signature()?;
    if let (Some(name), Some(email)) = (&msg.author_name, &msg.author_email) {
        author.name = name.clone();
        author.email = email.clone();
    }
    // git am records the mail's Date: header as the author date.
    if let Some((ts, tz)) = msg.date.as_deref().and_then(parse_rfc2822_date) {
        author.timestamp = ts;
        author.timezone = tz;
    }
    let committer = committer_signature()?;
    drop(repo);
    if !crate::util::quiet() {
        println!("Applying: {subject}");
    }
    // git am reports `Applying: <subject>` instead of a commit summary.
    crate::util::set_quiet_from(&["-q".to_string()]);
    apply_checked(&head_map, &deltas, msg.message.clone(), author, committer)?;
    Ok(())
}

/// RFC 2822 `[Wed, ]15 Nov 2023 05:13:20 +0000` → (unix seconds, `+hhmm`).
fn parse_rfc2822_date(v: &str) -> Option<(i64, String)> {
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let v = v.split_once(", ").map(|(_, r)| r).unwrap_or(v);
    let mut it = v.split_whitespace();
    let day: u32 = it.next()?.parse().ok()?;
    let mon_name = it.next()?;
    let mon = MONTHS.iter().position(|m| *m == mon_name)? + 1;
    let year: i64 = it.next()?.parse().ok()?;
    let time = it.next()?;
    let tz = it.next()?;
    crate::cmd::shared::parse_git_date(&format!("{year:04}-{mon:02}-{day:02} {time} {tz}"))
}

/// Claimed `--3way` subset: when the hunks do not apply to HEAD, take the
/// post-image blob named on the `index` line, or re-apply the hunks to the
/// pre-image blob, when that blob is in the object store.
fn three_way_tip(repo: &Repository, fp: &FilePatch) -> Option<Vec<u8>> {
    let (pre, post) = fp.index_oids.as_ref()?;
    let blob = |hex: &str| {
        crate::cmd::shared::resolve_rev(repo, hex)
            .ok()
            .and_then(|o| repo.get_object(&o).ok())
            .filter(|o| o.kind == ObjectKind::Blob)
            .map(|o| o.payload)
    };
    blob(post).or_else(|| apply_hunks(&blob(pre)?, fp).ok())
}
