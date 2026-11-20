//! `git am`: every option of `git am -h`.
//!
//! The mailbox is split into `.git/rebase-apply/0001`, `0002`, ... with
//! `next` / `last` counters; each message is applied to HEAD and committed
//! with the mail's author and date. A patch that does not apply stops the
//! run with the state on disk for `--continue` (`--resolved`), `--skip`,
//! `--abort`, `--quit` and `--show-current-patch`; with `--3way` the patch
//! is merged with its pre-image blob and conflicts land in the index.
//! Cite: <https://git-scm.com/docs/git-am> (behaviour, not code).

use crate::cmd::opt_parse::{Item, Mode, Parsed, parse};
use crate::cmd::patch::{parse_rfc2822_date, three_way_tip};
use crate::cmd::replay::{PathDelta, apply_checked, put_blob};
use crate::cmd::shared::{commit_signature, committer_signature};
use crate::error::{CliError, CliResult};
use crate::util::{open_repo, read_index, work_tree, write_index};
use arks_object::{
    MailMessage, ObjectKind, RawObject, apply_hunks, parse_mailbox, parse_unified_diff,
};
use arks_repo::Repository;
use arks_store::ObjectStore;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Default)]
struct Opts {
    three_way: bool,
    quiet: bool,
    signoff: bool,
    keep: bool,
    keep_non_patch: bool,
    message_id: bool,
    scissors: bool,
    no_verify: bool,
    strip: usize,
    directory: Option<String>,
    rules: Vec<(String, bool)>,
    committer_date_is_author_date: bool,
    ignore_date: bool,
    empty: String,
    sign_args: Vec<String>,
    whitespace: Option<String>,
    reject: bool,
}

fn options(p: &Parsed) -> CliResult<Opts> {
    let mut o = Opts {
        three_way: p.on("3way"),
        quiet: p.on("quiet"),
        signoff: p.on("signoff"),
        keep: p.on("keep"),
        keep_non_patch: p.on("keep-non-patch"),
        message_id: p.on("message-id"),
        scissors: p.on("scissors"),
        no_verify: p.on("no-verify"),
        strip: 1,
        directory: p.value("directory"),
        rules: Vec::new(),
        committer_date_is_author_date: p.on("committer-date-is-author-date"),
        ignore_date: p.on("ignore-date"),
        empty: p.value("empty").unwrap_or_else(|| "stop".into()),
        sign_args: crate::cmd::commit::sign_args(p),
        whitespace: p.value("whitespace"),
        reject: p.on("reject"),
    };
    if !matches!(o.empty.as_str(), "stop" | "drop" | "keep" | "die" | "asis") {
        return Err(CliError::Exit(
            129,
            format!("error: invalid value for '--empty': '{}'", o.empty),
        ));
    }
    if let Some(q) = p.value("quoted-cr") {
        if !matches!(q.as_str(), "nowarn" | "warn" | "strip") {
            return Err(CliError::Exit(
                129,
                format!("error: bad action '{q}' for '--quoted-cr'"),
            ));
        }
    }
    if let Some(f) = p.value("patch-format") {
        match f.as_str() {
            "mbox" | "mboxrd" => {}
            "stgit" | "stgit-series" | "hg" => {
                return Err(CliError::NotImplemented(format!(
                    "am --patch-format={f} is not implemented"
                )));
            }
            other => {
                return Err(CliError::Exit(
                    129,
                    format!("error: invalid value for '--patch-format': '{other}'"),
                ));
            }
        }
    }
    if let Some(n) = p.value("-p") {
        o.strip = n.parse().map_err(|_| {
            CliError::Exit(129, "error: switch `p' expects a numerical value".into())
        })?;
    }
    for it in &p.items {
        if let Item::Opt {
            key: k @ ("include" | "exclude"),
            val: Some(v),
            neg: false,
        } = it
        {
            o.rules.push((v.clone(), *k == "include"));
        }
    }
    Ok(o)
}

/// A `--3way` conflict: path, base, ours, theirs.
type Conflict = (String, Vec<u8>, Vec<u8>, Vec<u8>);

fn state_dir(repo: &Repository) -> PathBuf {
    repo.git_dir().join("rebase-apply")
}

fn not_resuming() -> CliError {
    CliError::fatal("Resolve operation not in progress, we are not resuming.")
}

pub fn cmd_am(args: &[String]) -> CliResult<()> {
    let p = parse("am", args, crate::cmd::porcelain_opts::AM, Mode::default())?;
    let repo = open_repo()?;
    let dir = state_dir(&repo);
    let in_progress = dir.join("applying").exists() || dir.join("last").exists();
    let action = [
        "continue",
        "resolved",
        "skip",
        "abort",
        "quit",
        "show-current-patch",
        "allow-empty",
    ]
    .into_iter()
    .find(|a| p.flag(a).is_some() || p.last(a).is_some());
    if let Some(a) = action {
        if !in_progress {
            return Err(not_resuming());
        }
        drop(repo);
        return match a {
            "continue" | "resolved" => resume(&dir, true),
            "allow-empty" => {
                // Record the stopped (empty) patch as an empty commit.
                let repo = open_repo()?;
                let n = read_num(&dir, "next")?;
                let raw = fs::read_to_string(dir.join(format!("{n:04}")))?;
                let msgs = parse_mailbox(&raw).map_err(|e| CliError::fatal(e.to_string()))?;
                let o = read_opts(&dir);
                let head_map = crate::cmd::shared::head_blob_map(&repo)?;
                drop(repo);
                if let Some(m) = msgs.first() {
                    commit_message(m, &o, &head_map, &[])?;
                }
                bump(&dir)?;
                resume(&dir, false)
            }
            "skip" => {
                let repo = open_repo()?;
                let work = work_tree(&repo)?;
                crate::cmd::working_tree::hard_reset_tree(&repo, &work, &repo.head_oid()?)?;
                bump(&dir)?;
                drop(repo);
                resume(&dir, false)
            }
            "abort" => {
                let repo = open_repo()?;
                let orig = fs::read_to_string(dir.join("orig-head"))?;
                let orig = crate::cmd::shared::resolve_rev(&repo, orig.trim())?;
                let work = work_tree(&repo)?;
                crate::cmd::working_tree::hard_reset_tree(&repo, &work, &orig)?;
                let head = repo.head_oid()?;
                if head != orig {
                    let mut r = open_repo()?;
                    match r.head_target()? {
                        arks_refs::RefTarget::Symbolic(b) => {
                            r.refs_mut().update(&b, &orig, Some(&head))?
                        }
                        arks_refs::RefTarget::Direct(_) => {
                            r.refs_mut().update("HEAD", &orig, None)?
                        }
                    }
                }
                fs::remove_dir_all(&dir)?;
                Ok(())
            }
            "quit" => {
                fs::remove_dir_all(&dir)?;
                Ok(())
            }
            _ => {
                let mode = p
                    .value("show-current-patch")
                    .unwrap_or_else(|| "raw".into());
                let next = read_num(&dir, "next")?;
                let raw = fs::read_to_string(dir.join(format!("{next:04}")))?;
                match mode.as_str() {
                    "diff" => {
                        // Everything after the message: the `---` line onwards.
                        let start = raw
                            .find("\n---\n")
                            .map(|i| i + 1)
                            .or_else(|| raw.find("\ndiff --git ").map(|i| i + 1))
                            .unwrap_or(0);
                        let mut text = raw[start..].to_string();
                        if !text.ends_with('\n') {
                            text.push('\n');
                        }
                        print!("{text}");
                    }
                    "raw" => print!("{raw}"),
                    other => {
                        return Err(CliError::Exit(
                            129,
                            format!("error: invalid value for '--show-current-patch': '{other}'"),
                        ));
                    }
                }
                Ok(())
            }
        };
    }
    let o = options(&p)?;
    let interactive = p.on("interactive");
    let files = {
        let mut f = p.args();
        f.extend(p.after_dashdash());
        f
    };
    if in_progress {
        return Err(CliError::fatal(
            "previous rebase directory .git/rebase-apply still exists but mbox given.",
        ));
    }
    {
        let git = repo.git_dir();
        if git.join("rebase-merge").is_dir() {
            return Err(CliError::Msg(
                "am refused: sequencer/apply state present (SEQ-007 / MISC-007); no dual-am dialect"
                    .into(),
            ));
        }
    }
    // Read the mailbox(es).
    let mut text = String::new();
    if files.is_empty() {
        std::io::Read::read_to_string(&mut std::io::stdin(), &mut text)?;
    }
    for f in &files {
        let path = PathBuf::from(f);
        if path.is_dir() {
            return Err(CliError::NotImplemented(
                "am of a Maildir directory is not implemented".into(),
            ));
        }
        let bytes = fs::read(&path)
            .map_err(|e| CliError::fatal(format!("could not open '{f}' for reading: {e}")))?;
        text.push_str(&String::from_utf8_lossy(&bytes));
    }
    if text.trim().is_empty() {
        // Nothing to apply: git records ORIG_HEAD and succeeds.
        if let Ok(h) = repo.head_oid() {
            let mut r = open_repo()?;
            let _ = r.refs_mut().update("ORIG_HEAD", &h, None);
        }
        return Ok(());
    }
    let messages =
        parse_mailbox(&text).map_err(|_| CliError::fatal("Patch format detection failed."))?;
    // Split into the state directory first, as git does.
    fs::create_dir_all(&dir)?;
    let head = repo.head_oid()?;
    fs::write(dir.join("orig-head"), format!("{}\n", head.to_hex()))?;
    for (k, m) in messages.iter().enumerate() {
        fs::write(dir.join(format!("{:04}", k + 1)), &m.raw)?;
    }
    fs::write(dir.join("next"), "1\n")?;
    fs::write(dir.join("last"), format!("{}\n", messages.len()))?;
    fs::write(dir.join("applying"), "")?;
    write_opts(&dir, &o)?;
    {
        let mut r = open_repo()?;
        let _ = r.refs_mut().update("ORIG_HEAD", &head, None);
    }
    drop(repo);
    if interactive {
        let first = &messages[0];
        println!(
            "Commit Body is:\n--------------------------\n{}\n--------------------------",
            first.message.trim_end()
        );
        print!("Apply? [y]es/[n]o/[e]dit/[v]iew patch/[a]ccept all: ");
        use std::io::Write;
        std::io::stdout().flush()?;
        let mut line = String::new();
        if std::io::stdin().read_line(&mut line)? == 0 {
            return Err(CliError::fatal("unable to read from stdin; aborting"));
        }
        if line.trim().starts_with('n') {
            bump(&dir)?;
        }
    }
    resume(&dir, false)
}

fn read_num(dir: &Path, name: &str) -> CliResult<usize> {
    fs::read_to_string(dir.join(name))?
        .trim()
        .parse()
        .map_err(|_| CliError::Msg(format!("corrupt rebase-apply/{name}")))
}

fn bump(dir: &Path) -> CliResult<()> {
    let n = read_num(dir, "next")?;
    fs::write(dir.join("next"), format!("{}\n", n + 1))?;
    Ok(())
}

fn write_opts(dir: &Path, o: &Opts) -> CliResult<()> {
    let mut s = String::new();
    let mut flag = |k: &str, v: bool| {
        if v {
            s.push_str(&format!("{k}\n"));
        }
    };
    flag("3way", o.three_way);
    flag("quiet", o.quiet);
    flag("signoff", o.signoff);
    flag("keep", o.keep);
    flag("keep-non-patch", o.keep_non_patch);
    flag("message-id", o.message_id);
    flag("scissors", o.scissors);
    flag("no-verify", o.no_verify);
    flag(
        "committer-date-is-author-date",
        o.committer_date_is_author_date,
    );
    flag("ignore-date", o.ignore_date);
    flag("reject", o.reject);
    if let Some(w) = &o.whitespace {
        s.push_str(&format!("whitespace={w}\n"));
    }
    s.push_str(&format!("strip={}\n", o.strip));
    if let Some(d) = &o.directory {
        s.push_str(&format!("directory={d}\n"));
    }
    for (g, inc) in &o.rules {
        s.push_str(&format!(
            "{}={g}\n",
            if *inc { "include" } else { "exclude" }
        ));
    }
    s.push_str(&format!("empty={}\n", o.empty));
    for a in &o.sign_args {
        s.push_str(&format!("sign={a}\n"));
    }
    fs::write(dir.join("arks-opts"), s)?;
    Ok(())
}

fn read_opts(dir: &Path) -> Opts {
    let mut o = Opts {
        strip: 1,
        empty: "stop".into(),
        ..Opts::default()
    };
    let text = fs::read_to_string(dir.join("arks-opts")).unwrap_or_default();
    for l in text.lines() {
        match l.split_once('=') {
            Some(("strip", v)) => o.strip = v.parse().unwrap_or(1),
            Some(("directory", v)) => o.directory = Some(v.to_string()),
            Some(("include", v)) => o.rules.push((v.to_string(), true)),
            Some(("exclude", v)) => o.rules.push((v.to_string(), false)),
            Some(("empty", v)) => o.empty = v.to_string(),
            Some(("sign", v)) => o.sign_args.push(v.to_string()),
            Some(("whitespace", v)) => o.whitespace = Some(v.to_string()),
            _ => match l {
                "3way" => o.three_way = true,
                "quiet" => o.quiet = true,
                "signoff" => o.signoff = true,
                "keep" => o.keep = true,
                "keep-non-patch" => o.keep_non_patch = true,
                "message-id" => o.message_id = true,
                "scissors" => o.scissors = true,
                "no-verify" => o.no_verify = true,
                "committer-date-is-author-date" => o.committer_date_is_author_date = true,
                "ignore-date" => o.ignore_date = true,
                "reject" => o.reject = true,
                _ => {}
            },
        }
    }
    o
}

/// Apply messages `next..=last`; `commit_stopped` first commits the index
/// for the stopped message (`--continue`).
fn resume(dir: &Path, commit_stopped: bool) -> CliResult<()> {
    let o = read_opts(dir);
    if commit_stopped {
        let repo = open_repo()?;
        let index = read_index(&repo)?;
        if index.entries.iter().any(|e| e.stage() != 0) {
            return Err(CliError::Exit(
                128,
                "You still have unmerged paths in your index.\nYou should 'git add' each file with resolved conflicts to mark them as such.\nYou might run `git rm` on a file to accept \"deleted by them\" for it.".into(),
            ));
        }
        let n = read_num(dir, "next")?;
        let raw = fs::read_to_string(dir.join(format!("{n:04}")))?;
        let msgs = parse_mailbox(&raw).map_err(|e| CliError::fatal(e.to_string()))?;
        let Some(msg) = msgs.first() else {
            return Err(CliError::fatal("corrupt rebase-apply patch"));
        };
        let head_map = crate::cmd::shared::head_blob_map(&repo)?;
        let staged: std::collections::HashMap<String, arks_oid::Oid> = index
            .entries
            .iter()
            .map(|e| (e.path.clone(), e.oid))
            .collect();
        if staged == head_map {
            return Err(CliError::Exit(
                128,
                format!(
                    "Applying: {}\nNo changes - did you forget to use 'git add'?\nIf there is nothing left to stage, chances are that something else\nalready introduced the same changes; you might want to skip this patch.",
                    message_subject(msg, &o)
                ),
            ));
        }
        let mut deltas = Vec::new();
        let mut paths: Vec<&String> = head_map.keys().chain(staged.keys()).collect();
        paths.sort();
        paths.dedup();
        for p in paths {
            if head_map.get(p) != staged.get(p) {
                deltas.push(PathDelta {
                    path: p.clone(),
                    base: head_map.get(p).copied(),
                    tip: staged.get(p).copied(),
                });
            }
        }
        drop(repo);
        if !o.quiet {
            println!("Applying: {}", message_subject(msg, &o));
        }
        commit_message(msg, &o, &head_map, &deltas)?;
        bump(dir)?;
    }
    let last = read_num(dir, "last")?;
    loop {
        let n = read_num(dir, "next")?;
        if n > last {
            break;
        }
        let raw = fs::read_to_string(dir.join(format!("{n:04}")))?;
        let msgs = parse_mailbox(&raw).map_err(|e| CliError::fatal(e.to_string()))?;
        if let Some(msg) = msgs.first() {
            am_one(msg, n, &o)?;
        }
        bump(dir)?;
    }
    fs::remove_dir_all(dir)?;
    Ok(())
}

/// The commit subject: `[PATCH ...]` stripped unless `-k`.
fn message_subject(msg: &MailMessage, o: &Opts) -> String {
    if o.keep || o.keep_non_patch {
        if let Some(line) = raw_subject(&msg.raw) {
            if o.keep {
                return line;
            }
            // --keep-non-patch: strip only the `[PATCH ...]` bracket.
            let mut s = line.as_str();
            while let Some(rest) = s.strip_prefix('[') {
                match rest.split_once(']') {
                    Some((tag, after)) if tag.contains("PATCH") => s = after.trim_start(),
                    _ => break,
                }
            }
            return s.to_string();
        }
    }
    crate::cmd::log_format::subject_line(&msg.message)
}

fn raw_subject(raw: &str) -> Option<String> {
    let mut out: Option<String> = None;
    for l in raw.lines().skip(1) {
        if l.is_empty() {
            break;
        }
        if let Some(rest) = l.strip_prefix("Subject: ") {
            out = Some(rest.to_string());
        } else if let (Some(s), true) = (&mut out, l.starts_with(' ') || l.starts_with('\t')) {
            s.push(' ');
            s.push_str(l.trim());
        } else if out.is_some() {
            break;
        }
    }
    out
}

/// The full commit message for one mail.
fn full_message(msg: &MailMessage, o: &Opts) -> CliResult<String> {
    let subject = message_subject(msg, o);
    let mut body = match msg.message.split_once('\n') {
        Some((_, rest)) => rest.trim_start_matches('\n').to_string(),
        None => String::new(),
    };
    if o.scissors {
        if let Some(i) = body
            .lines()
            .position(|l| l.contains("-- >8 --") || l.contains("-- 8< --"))
        {
            body = body.lines().skip(i + 1).collect::<Vec<_>>().join("\n");
            body = body.trim_start_matches('\n').to_string();
        }
    }
    let mut m = if body.trim().is_empty() {
        format!("{subject}\n")
    } else {
        format!("{subject}\n\n{}\n", body.trim_end_matches('\n'))
    };
    if o.message_id {
        if let Some(id) = msg.raw.lines().take_while(|l| !l.is_empty()).find_map(|l| {
            l.strip_prefix("Message-ID: ")
                .or_else(|| l.strip_prefix("Message-Id: "))
        }) {
            m = format!("{}\n\nMessage-ID: {id}\n", m.trim_end_matches('\n'));
        }
    }
    if o.signoff {
        m = crate::cmd::core::add_signoff(&m);
    }
    Ok(m)
}

/// Apply and commit one mailbox message on top of HEAD.
fn am_one(msg: &MailMessage, n: usize, o: &Opts) -> CliResult<()> {
    let mut repo = open_repo()?;
    let head_map = crate::cmd::shared::head_blob_map(&repo)?;
    let index = read_index(&repo)?;
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
        let _ = fs::remove_dir_all(state_dir(&repo));
        return Err(CliError::fatal(format!(
            "Dirty index: cannot apply patches (dirty: {})",
            dirty.join(" ")
        )));
    }
    let subject = message_subject(msg, o);
    if !o.quiet {
        println!("Applying: {subject}");
    }
    if let Some(w) = &o.whitespace {
        // git checks the whitespace action when it applies the first patch.
        if !matches!(
            w.as_str(),
            "nowarn" | "warn" | "fix" | "strip" | "error" | "error-all"
        ) {
            return Err(CliError::Exit(
                129,
                format!("error: unrecognized whitespace option '{w}'"),
            ));
        }
    }
    let filtered_out = |files: &Vec<arks_object::FilePatch>| files.is_empty();
    let raw = crate::cmd::apply::rewrite_patch_paths(
        msg.raw.as_bytes(),
        o.strip,
        o.directory.as_deref(),
    )?;
    let mut files = parse_unified_diff(&raw).unwrap_or_default();
    let had_files = !filtered_out(&files);
    files.retain(|fp| crate::cmd::apply::path_selected(fp.display_path(), &o.rules));
    let stop_hints = |out: &str| {
        format!(
            "{out}Patch failed at {n:04} {subject}\nWhen you have resolved this problem, run \"git am --continue\".\nIf you prefer to skip this patch, run \"git am --skip\" instead.\nTo restore the original branch and stop patching, run \"git am --abort\"."
        )
    };
    if files.is_empty() && !had_files {
        match o.empty.as_str() {
            "drop" => return Ok(()),
            "keep" => {}
            _ => {
                println!("Patch is empty.");
                return Err(CliError::Exit(
                    128,
                    "When you have resolved this problem, run \"git am --continue\".\nIf you prefer to skip this patch, run \"git am --skip\" instead.\nTo record the empty patch as an empty commit, run \"git am --allow-empty\".\nTo restore the original branch and stop patching, run \"git am --abort\".".into(),
                ));
            }
        }
    }
    let mut deltas = Vec::new();
    let mut conflicts: Vec<Conflict> = Vec::new();
    for fp in &files {
        let shown = fp.display_path().to_string();
        let base = fp.old_path.as_ref().and_then(|p| head_map.get(p)).copied();
        let base_bytes = match base {
            Some(o) => repo.get_object(&o)?.payload,
            None => Vec::new(),
        };
        let applied = if fp.binary || (fp.old_path.is_some() && base.is_none()) {
            Err(0)
        } else {
            apply_hunks(&base_bytes, fp)
        };
        let new_bytes = match applied {
            Ok(b) => b,
            Err(line) => {
                let pre = fp.index_oids.as_ref().and_then(|(a, _)| {
                    crate::cmd::shared::resolve_rev(&repo, a)
                        .ok()
                        .and_then(|oid| repo.get_object(&oid).ok())
                        .filter(|obj| obj.kind == ObjectKind::Blob)
                        .map(|obj| obj.payload)
                });
                match (o.three_way, pre, three_way_tip(&repo, fp)) {
                    (true, Some(pre), Some(theirs)) => {
                        let labels = crate::cmd::merge_engine::MergeLabels {
                            ours: "HEAD".into(),
                            base: "base".into(),
                            theirs: subject.clone(),
                            diff3: false,
                            favor: crate::cmd::merge_engine::Favor::None,
                        };
                        let (merged, conflicted) = crate::cmd::merge_engine::merge_text(
                            &pre,
                            &base_bytes,
                            &theirs,
                            &labels,
                        );
                        if conflicted {
                            conflicts.push((shown.clone(), pre, base_bytes.clone(), theirs));
                            continue;
                        }
                        merged
                    }
                    _ => {
                        if o.reject {
                            reject_file(&repo, fp, &base_bytes, &shown)?;
                        }
                        eprintln!(
                            "error: patch failed: {shown}:{line}\nerror: {shown}: patch does not apply"
                        );
                        println!("{}", stop_hints(""));
                        return Err(CliError::Exit(
                            128,
                            "hint: Use 'git am --show-current-patch=diff' to see the failed patch"
                                .into(),
                        ));
                    }
                }
            }
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
    if !conflicts.is_empty() {
        println!("Using index info to reconstruct a base tree...");
        for (p, _, _, _) in &conflicts {
            println!("M\t{p}");
        }
        println!("Falling back to patching base and 3-way merge...");
        let work = work_tree(&repo)?;
        let mut index = read_index(&repo)?;
        for (p, base, ours, theirs) in &conflicts {
            println!("Auto-merging {p}");
            println!("CONFLICT (content): Merge conflict in {p}");
            let labels = crate::cmd::merge_engine::MergeLabels {
                ours: "HEAD".into(),
                base: "base".into(),
                theirs: subject.clone(),
                diff3: false,
                favor: crate::cmd::merge_engine::Favor::None,
            };
            let (merged, _) = crate::cmd::merge_engine::merge_text(base, ours, theirs, &labels);
            fs::write(work.join(p), &merged)?;
            index.entries.retain(|e| &e.path != p);
            for (stage, bytes) in [(1u16, base), (2, ours), (3, theirs)] {
                let format = repo.object_format();
                let oid = repo
                    .store_mut()
                    .put(format, &RawObject::new(ObjectKind::Blob, bytes.clone()))?;
                index.entries.push(arks_index::IndexEntry {
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
                    flags: (p.len().min(0x0fff)) as u16 | (stage << 12),
                    flags_extended: None,
                    path: p.clone(),
                });
            }
        }
        arks_index::sort_entries(&mut index.entries);
        write_index(&repo, &index)?;
        println!("{}", stop_hints(""));
        return Err(CliError::Exit(
            128,
            "error: Failed to merge in the changes.\nhint: Use 'git am --show-current-patch=diff' to see the failed patch".into(),
        ));
    }
    drop(repo);
    commit_message(msg, o, &head_map, &deltas)
}

fn commit_message(
    msg: &MailMessage,
    o: &Opts,
    head_map: &std::collections::HashMap<String, arks_oid::Oid>,
    deltas: &[PathDelta],
) -> CliResult<()> {
    let mut author = commit_signature()?;
    if let (Some(name), Some(email)) = (&msg.author_name, &msg.author_email) {
        author.name = name.clone();
        author.email = email.clone();
    }
    let mut committer = committer_signature()?;
    // git am records the mail's Date: header as the author date.
    if !o.ignore_date {
        if let Some((ts, tz)) = msg.date.as_deref().and_then(parse_rfc2822_date) {
            author.timestamp = ts;
            author.timezone = tz;
        }
    } else {
        // git takes the current time here, not GIT_AUTHOR_DATE.
        author.timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(committer.timestamp);
        author.timezone = committer.timezone.clone();
    }
    if o.committer_date_is_author_date {
        committer.timestamp = author.timestamp;
        committer.timezone = author.timezone.clone();
    }
    let message = full_message(msg, o)?;
    let repo = open_repo()?;
    crate::cmd::core::run_claimed_hooks(
        &repo,
        o.no_verify,
        &["applypatch-msg"],
        Some(message.as_bytes()),
    )?;
    drop(repo);
    // git am reports `Applying: <subject>` instead of a commit summary.
    crate::util::set_quiet_from(&["-q".to_string()]);
    if !o.sign_args.is_empty() {
        let repo = open_repo()?;
        let (do_sign, _) = crate::cmd::sign::want_sign_commit(&o.sign_args, &repo)?;
        if do_sign {
            return Err(CliError::NotImplemented(
                "am -S: signing am commits is not implemented".into(),
            ));
        }
    }
    if deltas.is_empty() {
        // Every file was filtered out (--include / --exclude): an empty commit.
        crate::cmd::replay::commit_tree(head_map, message, author, committer)?;
        return Ok(());
    }
    apply_checked(head_map, deltas, message, author, committer)?;
    Ok(())
}

/// `--reject`: apply the hunks that apply to the work tree and write the
/// others to `<path>.rej`.
fn reject_file(
    repo: &Repository,
    fp: &arks_object::FilePatch,
    current: &[u8],
    path: &str,
) -> CliResult<()> {
    let work = work_tree(repo)?;
    let mut content = current.to_vec();
    let mut rej = format!("diff a/{path} b/{path}\t(rejected hunks)\n");
    for h in &fp.hunks {
        let single = arks_object::FilePatch {
            hunks: vec![h.clone()],
            ..fp.clone()
        };
        match apply_hunks(&content, &single) {
            Ok(n) => content = n,
            Err(_) => {
                rej.push_str(&format!(
                    "@@ -{},{} +{},{} @@\n",
                    h.old_start, h.old_count, h.new_start, h.new_count
                ));
                for l in &h.lines {
                    let (c, b) = match l {
                        arks_object::HunkLine::Context(b) => (' ', b),
                        arks_object::HunkLine::Del(b) => ('-', b),
                        arks_object::HunkLine::Add(b) => ('+', b),
                    };
                    rej.push(c);
                    rej.push_str(&String::from_utf8_lossy(b));
                    if !b.ends_with(b"\n") {
                        rej.push('\n');
                    }
                }
            }
        }
    }
    fs::write(work.join(path), content)?;
    fs::write(work.join(format!("{path}.rej")), rej)?;
    Ok(())
}
