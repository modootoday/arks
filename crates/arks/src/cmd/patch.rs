//! format-patch / am — CLI wires `arks_object` mailbox and unified-diff parsing.

use crate::cmd::replay::{
    PathDelta, apply_checked, ensure_clean, first_parent_set, put_blob, read_commit, resolve_commit,
};
use crate::cmd::shared::{commit_signature, committer_signature};
use crate::error::{CliError, CliResult};
use crate::util::{cwd, open_repo, refuse_unknown_flags};
use arks_object::{
    FilePatch, MailMessage, ObjectKind, apply_hunks, parse_mailbox, parse_unified_diff,
};
use arks_oid::Oid;
use arks_repo::Repository;
use std::fs;
use std::path::PathBuf;
pub fn cmd_format_patch(args: &[String]) -> CliResult<()> {
    // M16 Sy-5: `--cover-letter`, `-1`, `-o` claimed (git-format-patch).
    // C7b: `-<n>`, `--stdout`, `<since>` (= `<since>..HEAD`), `-n`/`-N`.
    // Cite: https://git-scm.com/docs/git-format-patch
    let mut count: Option<usize> = None;
    let mut cover_letter = false;
    let mut stdout = false;
    let mut quiet = false;
    let mut numbered: Option<bool> = None;
    let mut out_dir: Option<PathBuf> = None;
    let mut positional = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        if let Some(n) = a
            .strip_prefix('-')
            .filter(|d| !d.is_empty() && d.bytes().all(|b| b.is_ascii_digit()))
        {
            count = Some(
                n.parse()
                    .map_err(|_| CliError::fatal(format!("bad count {a}")))?,
            );
        } else if a == "--cover-letter" {
            cover_letter = true;
        } else if a == "--stdout" {
            stdout = true;
        } else if a == "-n" || a == "--numbered" {
            numbered = Some(true);
        } else if a == "-N" || a == "--no-numbered" {
            numbered = Some(false);
        } else if a == "-q" || a == "--quiet" {
            quiet = true;
        } else if a == "-o" || a == "--output-directory" {
            i += 1;
            let dir = args
                .get(i)
                .ok_or_else(|| CliError::Usage("format-patch: -o requires a directory".into()))?;
            out_dir = Some(PathBuf::from(dir));
        } else if let Some(dir) = a.strip_prefix("--output-directory=") {
            out_dir = Some(PathBuf::from(dir));
        } else if a.starts_with('-') {
            return Err(CliError::NotImplemented(format!("unsupported option: {a}")));
        } else {
            positional.push(a);
        }
        i += 1;
    }
    let repo = open_repo()?;
    let commits = if let Some(n) = count {
        if positional.len() > 1 {
            return Err(CliError::Usage(
                "format-patch: -<n> accepts at most one commit".into(),
            ));
        }
        let rev = positional.first().copied().unwrap_or("HEAD");
        if rev.contains("..") {
            return Err(CliError::Usage(
                "format-patch: -<n> does not take a range".into(),
            ));
        }
        // The last <n> non-merge commits reachable from <rev> (rev-list order).
        let tip = resolve_commit(&repo, rev)?;
        let mut picked: Vec<Oid> = crate::cmd::log_format::walk(&repo, &[tip], &[], false)?
            .into_iter()
            .filter(|(_, c)| c.parents.len() <= 1)
            .take(n)
            .map(|(o, _)| o)
            .collect();
        picked.reverse();
        picked
    } else if positional.len() == 1 {
        let spec = if positional[0].contains("..") {
            positional[0].to_string()
        } else {
            format!("{}..HEAD", positional[0])
        };
        series_commits(&repo, &spec)?
    } else {
        return Err(CliError::Usage(
            "format-patch: -<n> [<commit>] or <since> or <A>..<B>".into(),
        ));
    };
    if commits.is_empty() {
        return Ok(());
    }
    let total = commits.len();
    let numbered = numbered.unwrap_or(total > 1 || cover_letter);
    let mut rendered = Vec::new();
    if cover_letter {
        rendered.push((
            "0000-cover-letter.patch".to_owned(),
            render_cover_letter(&repo, &commits)?,
        ));
    }
    for (n, oid) in commits.iter().enumerate() {
        let prefix = if numbered {
            format!("[PATCH {}/{total}]", n + 1)
        } else {
            "[PATCH]".to_string()
        };
        let body = render_patch(&repo, oid, &prefix)?;
        let commit = read_commit(&repo, oid)?;
        let name = format!("{:04}-{}.patch", n + 1, patch_slug(commit.subject()));
        rendered.push((name, body));
    }
    drop(repo);
    if stdout {
        for (_, body) in rendered {
            print!("{body}");
        }
        return Ok(());
    }
    if let Some(dir) = &out_dir {
        fs::create_dir_all(dir)?;
    }
    let base = match &out_dir {
        Some(d) => d.clone(),
        None => cwd()?,
    };
    for (name, body) in rendered {
        fs::write(base.join(&name), body)?;
        // git prints the file name relative to where it was asked for.
        match &out_dir {
            _ if quiet => {}
            Some(d) => println!("{}", d.join(&name).display()),
            None => println!("{name}"),
        }
    }
    Ok(())
}

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

fn series_commits(repo: &Repository, spec: &str) -> CliResult<Vec<Oid>> {
    if spec.contains("...") {
        return Err(CliError::NotImplemented(
            "format-patch symmetric difference is not claimed".into(),
        ));
    }
    let Some((left, right)) = spec.split_once("..") else {
        return Err(CliError::Usage(
            "format-patch: -1 [<commit>] or <A>..<B>".into(),
        ));
    };
    if left.is_empty() || right.is_empty() {
        return Err(CliError::Usage(
            "format-patch: -1 [<commit>] or <A>..<B>".into(),
        ));
    }
    let start = resolve_commit(repo, left)?;
    let end = resolve_commit(repo, right)?;
    let stop = first_parent_set(repo, start)?;
    let mut newest = Vec::new();
    let mut oid = end;
    loop {
        if stop.contains(&oid) {
            break;
        }
        let commit = read_commit(repo, &oid)?;
        if commit.parents.len() > 1 {
            return Err(CliError::NotImplemented(
                "format-patch of merge commits is not claimed".into(),
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

fn render_cover_letter(repo: &Repository, commits: &[Oid]) -> CliResult<String> {
    // Cover letter: mailbox stub + shortlog of the series (claimed subset).
    let head = commits
        .last()
        .copied()
        .ok_or_else(|| CliError::Msg("format-patch: empty cover letter series".into()))?;
    let tip = read_commit(repo, &head)?;
    let mut out = String::new();
    out.push_str(&format!(
        "From {} Mon Sep 17 00:00:00 2001\n",
        head.to_hex()
    ));
    out.push_str(&format!(
        "From: {} <{}>\n",
        tip.author.name, tip.author.email
    ));
    out.push_str(&format!(
        "Date: {} {}\n",
        tip.author.timestamp, tip.author.timezone
    ));
    out.push_str(&format!(
        "Subject: [PATCH 0/{}] Cover Letter\n\n",
        commits.len()
    ));
    out.push_str("*** BLURB HERE ***\n\n");
    for oid in commits {
        let c = read_commit(repo, oid)?;
        let subj = c.subject();
        out.push_str(&format!("  {} {}\n", &oid.to_hex()[..7], subj));
    }
    out.push('\n');
    out.push_str("---\n");
    Ok(out)
}

/// One format-patch mailbox: `From <oid> <magic date>`, `From:`, RFC 2822
/// `Date:`, `Subject: <prefix> <subject>`, the message body, `---`, the
/// diffstat, the patch, and the `-- ` signature (the arks version).
fn render_patch(repo: &Repository, oid: &Oid, prefix: &str) -> CliResult<String> {
    use crate::cmd::diff_render::{DiffFormat, changes_between, render};
    use crate::cmd::log_format::DateMode;
    let commit = read_commit(repo, oid)?;
    if commit.parents.len() > 1 {
        return Err(CliError::NotImplemented(
            "format-patch of merge commits is not claimed".into(),
        ));
    }
    let subject = crate::cmd::log_format::subject_line(&commit.message);
    let body: String = {
        let msg = commit.message.trim_end_matches('\n');
        match msg.split_once("\n\n") {
            Some((_, rest)) => format!("{}\n", rest.trim_start_matches('\n')),
            None => String::new(),
        }
    };
    let mut out = String::new();
    out.push_str(&format!("From {} Mon Sep 17 00:00:00 2001\n", oid.to_hex()));
    out.push_str(&format!(
        "From: {} <{}>\n",
        commit.author.name, commit.author.email
    ));
    out.push_str(&format!(
        "Date: {}\n",
        DateMode::Rfc2822.format(&commit.author)
    ));
    out.push_str(&format!("Subject: {prefix} {subject}\n\n"));
    if !body.is_empty() {
        out.push_str(&body);
        out.push('\n');
    }
    out.push_str("---\n");
    let old = match commit.parents.first() {
        Some(p) => crate::cmd::shared::tree_blob_map(repo, p)?,
        None => Default::default(),
    };
    let new = crate::cmd::shared::tree_blob_map(repo, oid)?;
    let changes = changes_between(&old, &new, None);
    let fmt = DiffFormat {
        stat: true,
        patch: true,
        ..Default::default()
    };
    let mut buf = Vec::new();
    render(&mut buf, repo, &fmt, &changes)?;
    out.push_str(&String::from_utf8_lossy(&buf));
    out.push_str(&format!("-- \n{}\n\n", env!("CARGO_PKG_VERSION")));
    Ok(out)
}

fn patch_slug(subject: &str) -> String {
    let mut slug = String::new();
    let mut prev_dash = false;
    for ch in subject.chars() {
        if ch.is_ascii_alphanumeric() || ch == '.' || ch == '_' {
            slug.push(ch);
            prev_dash = false;
        } else if !prev_dash {
            slug.push('-');
            prev_dash = true;
        }
        if slug.len() >= 52 {
            break;
        }
    }
    let slug = slug.trim_matches('-');
    if slug.is_empty() {
        "patch".to_owned()
    } else {
        slug.to_owned()
    }
}
