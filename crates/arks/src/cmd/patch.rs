//! format-patch / am — CLI wires `arks_object::parse_patch`.

use crate::cmd::replay::{
    PathDelta, apply_checked, deltas_for_commit, ensure_clean, ensure_nl, first_parent_set,
    put_blob, read_commit, resolve_commit,
};
use crate::cmd::shared::commit_signature;
use crate::error::{CliError, CliResult};
use crate::util::{cwd, open_repo, refuse_unknown_flags};
use arks_object::{ObjectKind, parse_patch};
use arks_oid::Oid;
use arks_repo::Repository;
use std::fs;
use std::path::PathBuf;
pub fn cmd_format_patch(args: &[String]) -> CliResult<()> {
    // M16 Sy-5: `--cover-letter`, `-1`, `-o` claimed (git-format-patch).
    // Cite: https://git-scm.com/docs/git-format-patch
    refuse_unknown_flags(args, &["-1", "-o", "--cover-letter"])?;
    let mut one = false;
    let mut cover_letter = false;
    let mut out_dir: Option<PathBuf> = None;
    let mut positional = Vec::new();
    let mut i = 0;
    while i < args.len() {
        if args[i] == "-1" {
            one = true;
        } else if args[i] == "--cover-letter" {
            cover_letter = true;
        } else if args[i] == "-o" {
            i += 1;
            let dir = args
                .get(i)
                .ok_or_else(|| CliError::Usage("format-patch: -o requires a directory".into()))?;
            out_dir = Some(PathBuf::from(dir));
        } else if !args[i].starts_with('-') {
            positional.push(args[i].as_str());
        }
        i += 1;
    }
    let repo = open_repo()?;
    let commits = if one {
        if positional.len() > 1 {
            return Err(CliError::Usage(
                "format-patch: -1 accepts at most one commit".into(),
            ));
        }
        let rev = positional.first().copied().unwrap_or("HEAD");
        if rev.contains("..") {
            return Err(CliError::Usage(
                "format-patch: -1 does not take a range".into(),
            ));
        }
        vec![resolve_commit(&repo, rev)?]
    } else if positional.len() == 1 {
        series_commits(&repo, positional[0])?
    } else {
        return Err(CliError::Usage(
            "format-patch: -1 [<commit>] or <A>..<B>".into(),
        ));
    };
    if commits.is_empty() {
        return Err(CliError::Msg("format-patch: empty range".into()));
    }
    let mut rendered = Vec::new();
    if cover_letter {
        rendered.push((
            "0000-cover-letter.patch".to_owned(),
            render_cover_letter(&repo, &commits)?,
        ));
    }
    for (n, oid) in commits.iter().enumerate() {
        let body = render_patch(&repo, oid)?;
        let commit = read_commit(&repo, oid)?;
        let name = format!("{:04}-{}.patch", n + 1, patch_slug(commit.subject()));
        rendered.push((name, body));
    }
    drop(repo);
    let dir = out_dir.unwrap_or(cwd()?);
    fs::create_dir_all(&dir)?;
    for (name, body) in rendered {
        let path = dir.join(&name);
        fs::write(&path, body)?;
        println!("{}", path.display());
    }
    Ok(())
}

pub fn cmd_am(args: &[String]) -> CliResult<()> {
    // M16 Sy-5: `--3way` when index blobs resolvable (git-am).
    // Cite: https://git-scm.com/docs/git-am
    refuse_unknown_flags(args, &["--3way", "-3"])?;
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
    let mut repo = open_repo()?;
    // SEQ-007 / MISC-007 (M19 Cx-2): shared rebase-apply / am state — refuse concurrent am
    // before parsing the patch (fail-closed on state, not on parse order).
    let git = repo.git_dir();
    if git.join("rebase-apply").is_dir() || git.join("rebase-merge").is_dir() {
        return Err(CliError::Msg(
            "am refused: sequencer/apply state present (SEQ-007 / MISC-007); no dual-am dialect"
                .into(),
        ));
    }
    let bytes = fs::read(&file)?;
    let text = std::str::from_utf8(&bytes)
        .map_err(|_| CliError::NotImplemented("am: non-utf8 patch is not claimed".into()))?;
    let patch = parse_patch(text)?;
    let head_map = ensure_clean(&repo)?;
    let mut deltas = Vec::new();
    for file in &patch.files {
        let base = match &file.base {
            Some(bytes) => Some(put_blob(&mut repo, bytes)?),
            None => None,
        };
        let tip = match &file.tip {
            Some(bytes) => Some(put_blob(&mut repo, bytes)?),
            None => None,
        };
        if base != tip {
            deltas.push(PathDelta {
                path: file.path.clone(),
                base,
                tip,
            });
        }
    }
    let mut author = commit_signature()?;
    if let (Some(name), Some(email)) = (patch.author_name, patch.author_email) {
        author.name = name;
        author.email = email;
    }
    let committer = commit_signature()?;
    // Claimed `--3way` subset: when clean apply would refuse and both sides of each
    // delta resolve in the object store (we just stored reconstructed blobs), take tip.
    if three_way {
        let mut need_force = false;
        for d in &deltas {
            let have = head_map.get(&d.path).copied();
            if have != d.base {
                if d.tip.is_none() && d.base.is_none() {
                    drop(repo);
                    return Err(CliError::Msg(format!(
                        "am --3way: cannot resolve blobs for '{}'",
                        d.path
                    )));
                }
                // Tip (and optional base) must be fetchable from the store.
                if let Some(tip) = d.tip {
                    let _ = repo.get_object(&tip).map_err(|_| {
                        CliError::Msg(format!(
                            "am --3way: tip blob {} not resolvable",
                            tip.to_hex()
                        ))
                    })?;
                }
                if let Some(base) = d.base {
                    let _ = repo.get_object(&base).map_err(|_| {
                        CliError::Msg(format!(
                            "am --3way: base blob {} not resolvable",
                            base.to_hex()
                        ))
                    })?;
                }
                need_force = true;
            }
        }
        if need_force {
            let forced: Vec<PathDelta> = deltas
                .iter()
                .map(|d| PathDelta {
                    path: d.path.clone(),
                    // Treat current HEAD blob as base so apply_checked accepts the tip.
                    base: head_map.get(&d.path).copied(),
                    tip: d.tip,
                })
                .collect();
            drop(repo);
            apply_checked(&head_map, &forced, patch.message, author, committer)?;
            return Ok(());
        }
    }
    drop(repo);
    apply_checked(&head_map, &deltas, patch.message, author, committer)?;
    Ok(())
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
    out.push_str(&format!("From {} Mon Sep 17 00:00:00 2001\n", head.to_hex()));
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

fn render_patch(repo: &Repository, oid: &Oid) -> CliResult<String> {
    let commit = read_commit(repo, oid)?;
    if commit.parents.len() > 1 {
        return Err(CliError::NotImplemented(
            "format-patch of merge commits is not claimed".into(),
        ));
    }
    let deltas = deltas_for_commit(repo, &commit)?;
    let subject = {
        let s = commit.subject();
        if s.is_empty() {
            "untitled".to_owned()
        } else {
            s.to_owned()
        }
    };
    let mut out = String::new();
    out.push_str(&format!("From {} Mon Sep 17 00:00:00 2001\n", oid.to_hex()));
    out.push_str(&format!(
        "From: {} <{}>\n",
        commit.author.name, commit.author.email
    ));
    out.push_str(&format!(
        "Date: {} {}\n",
        commit.author.timestamp, commit.author.timezone
    ));
    out.push_str(&format!("Subject: [PATCH] {subject}\n\n"));
    out.push_str(&ensure_nl(commit.message.clone()));
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out.push_str("---\n");
    for delta in &deltas {
        let base_bytes = match delta.base {
            Some(blob) => Some(blob_payload(repo, &blob)?),
            None => None,
        };
        let tip_bytes = match delta.tip {
            Some(blob) => Some(blob_payload(repo, &blob)?),
            None => None,
        };
        append_file_diff(
            &mut out,
            &delta.path,
            base_bytes.as_deref(),
            tip_bytes.as_deref(),
        )?;
    }
    Ok(out)
}

fn blob_payload(repo: &Repository, oid: &Oid) -> CliResult<Vec<u8>> {
    let obj = repo.get_object(oid)?;
    if obj.kind != ObjectKind::Blob {
        return Err(CliError::Msg(format!("{} is not a blob", oid.to_hex())));
    }
    Ok(obj.payload)
}

fn append_file_diff(
    out: &mut String,
    path: &str,
    base: Option<&[u8]>,
    tip: Option<&[u8]>,
) -> CliResult<()> {
    out.push_str(&format!("diff --git a/{path} b/{path}\n"));
    match (base, tip) {
        (None, Some(_)) => out.push_str("new file mode 100644\n"),
        (Some(_), None) => out.push_str("deleted file mode 100644\n"),
        _ => {}
    }
    if base.is_none() {
        out.push_str("--- /dev/null\n");
    } else {
        out.push_str(&format!("--- a/{path}\n"));
    }
    if tip.is_none() {
        out.push_str("+++ /dev/null\n");
    } else {
        out.push_str(&format!("+++ b/{path}\n"));
    }
    let (old_lines, old_nl) = match base {
        Some(bytes) => split_text(bytes)?,
        None => (Vec::new(), true),
    };
    let (new_lines, new_nl) = match tip {
        Some(bytes) => split_text(bytes)?,
        None => (Vec::new(), true),
    };
    let old_count = old_lines.len();
    let new_count = new_lines.len();
    let old_start = if old_count == 0 { 0 } else { 1 };
    let new_start = if new_count == 0 { 0 } else { 1 };
    out.push_str(&format!(
        "@@ -{old_start},{old_count} +{new_start},{new_count} @@\n"
    ));
    if base.is_some() {
        for line in &old_lines {
            out.push('-');
            out.push_str(line);
            out.push('\n');
        }
        if !old_nl && old_count > 0 {
            out.push_str("\\ No newline at end of file\n");
        }
    }
    if tip.is_some() {
        for line in &new_lines {
            out.push('+');
            out.push_str(line);
            out.push('\n');
        }
        if !new_nl && new_count > 0 {
            out.push_str("\\ No newline at end of file\n");
        }
    }
    Ok(())
}

fn split_text(bytes: &[u8]) -> CliResult<(Vec<String>, bool)> {
    if bytes.contains(&0) {
        return Err(CliError::NotImplemented(
            "binary patches are not claimed".into(),
        ));
    }
    let text = std::str::from_utf8(bytes)
        .map_err(|_| CliError::NotImplemented("non-utf8 patches are not claimed".into()))?;
    let trailing_nl = text.is_empty() || text.ends_with('\n');
    Ok((text.lines().map(str::to_owned).collect(), trailing_nl))
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
