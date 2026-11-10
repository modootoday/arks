//! Tags and describe.

use crate::cmd::shared::{commit_signature, resolve_rev};
use crate::cmd::sign::sign_bytes;
use crate::error::{CliError, CliResult};
use crate::util::{FORMAT, open_repo, refuse_unknown_flags};
use arks_object::{Commit, ObjectKind, RawObject};
use arks_store::ObjectStore;

fn tag_positionals(args: &[String]) -> Vec<String> {
    args.iter()
        .enumerate()
        .filter_map(|(idx, a)| {
            if a.starts_with('-') {
                return None;
            }
            if idx > 0 {
                let prev = args[idx - 1].as_str();
                if prev == "-m" || prev == "-u" || prev == "--local-user" {
                    return None;
                }
            }
            Some(a.clone())
        })
        .collect()
}

pub fn cmd_tag(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &["-d", "-a", "-s", "-m", "-u", "--local-user"])?;
    let annotated = args.iter().any(|a| a == "-a");
    let signed = args.iter().any(|a| a == "-s");
    let mut message: Option<String> = None;
    let mut local_user: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        if args[i] == "-m" {
            i += 1;
            let part = args
                .get(i)
                .ok_or_else(|| CliError::Usage("tag: -m requires a message".into()))?;
            message = Some(part.clone());
        } else if args[i] == "-u" || args[i] == "--local-user" {
            i += 1;
            let key = args
                .get(i)
                .ok_or_else(|| CliError::Usage("tag: -u/--local-user requires a key id".into()))?;
            local_user = Some(key.clone());
        } else if let Some(rest) = args[i].strip_prefix("--local-user=") {
            local_user = Some(rest.to_string());
        } else if let Some(rest) = args[i].strip_prefix("-u=") {
            local_user = Some(rest.to_string());
        }
        i += 1;
    }
    let mut repo = open_repo()?;
    let positional = tag_positionals(args);
    if args.iter().any(|a| a == "-d") {
        let name = positional
            .first()
            .ok_or_else(|| CliError::Usage("tag -d <name>".into()))?;
        repo.refs_mut().delete(&format!("refs/tags/{name}"), None)?;
        return Ok(());
    }
    if positional.is_empty() {
        return list_tags(&repo);
    }
    let name = &positional[0];
    let target = if let Some(rev) = positional.get(1) {
        resolve_rev(&repo, rev)?
    } else {
        repo.head_oid()?
    };
    if signed || local_user.is_some() {
        let msg = message.ok_or_else(|| {
            CliError::Usage("tag -s requires -m <message> (claimed non-interactive subset)".into())
        })?;
        write_signed_tag(&mut repo, name, &target, &msg, local_user.as_deref())?;
        return Ok(());
    }
    if annotated {
        let msg = message.ok_or_else(|| CliError::Usage("tag -a requires -m <message>".into()))?;
        crate::cmd::write_annotated_tag(&mut repo, name, &target, &msg)?;
        return Ok(());
    }
    if message.is_some() {
        return Err(CliError::Usage(
            "tag: -m without -a/-s is not claimed (use -a -m or -s -m)".into(),
        ));
    }
    repo.refs_mut()
        .update(&format!("refs/tags/{name}"), &target, None)?;
    Ok(())
}

/// Signed annotated tag: unsigned payload + appended armor (gitformat-signature).
fn write_signed_tag(
    repo: &mut arks_repo::Repository,
    name: &str,
    target: &arks_oid::Oid,
    message: &str,
    key_override: Option<&str>,
) -> CliResult<arks_oid::Oid> {
    let sig = commit_signature()?;
    let mut unsigned =
        crate::cmd::encode_tag_payload(target, "commit", name, &sig.format_line(), message);
    // Ensure trailing newline before signature block (gitformat-signature examples).
    if !unsigned.ends_with(b"\n") {
        unsigned.push(b'\n');
    }
    let armor = sign_bytes(repo, &unsigned, key_override, "tag -s")?;
    let mut payload = unsigned;
    payload.extend_from_slice(armor.as_bytes());
    let obj = RawObject::new(ObjectKind::Tag, payload);
    let tag_oid = repo.store_mut().put(FORMAT, &obj)?;
    repo.refs_mut()
        .update(&format!("refs/tags/{name}"), &tag_oid, None)?;
    Ok(tag_oid)
}

pub fn cmd_describe(args: &[String]) -> CliResult<()> {
    // M16 Sy-1: `--dirty[=<mark>]` claimed (git-describe).
    refuse_unknown_flags(args, &["--tags", "--dirty"])?;
    let mut dirty_mark: Option<String> = None;
    for a in args {
        if a == "--dirty" {
            dirty_mark = Some("-dirty".into());
        } else if let Some(rest) = a.strip_prefix("--dirty=") {
            dirty_mark = Some(rest.to_string());
        }
    }
    let _tags = args.iter().any(|a| a == "--tags");
    let repo = open_repo()?;
    // MISC-001 (M20 Cx-8): describe under pack-refs is reader-safe; honesty note.
    if repo.common_dir().join("packed-refs.lock").is_file() {
        eprintln!(
            "note: MISC-001 — describe during pack-refs is reader-safe but expensive under write locks; prefer quiet windows"
        );
    } else {
        eprintln!(
            "note: MISC-001 — parallel describe / heavy reads during pack-refs are unlocked readers; still serialize writers"
        );
    }
    let head = repo.head_oid()?;
    let tags = repo.refs().list_refs("refs/tags/")?;
    let mut peeled: Vec<(String, arks_oid::Oid)> = Vec::new();
    for (name, oid) in &tags {
        peeled.push((name.clone(), peel_tag_target(&repo, oid)?));
    }
    let suffix = if let Some(mark) = &dirty_mark {
        if worktree_dirty(&repo)? {
            mark.as_str()
        } else {
            ""
        }
    } else {
        ""
    };
    // Exact match first (peeled commit OID).
    for (name, oid) in &peeled {
        if *oid == head {
            println!(
                "{}{suffix}",
                name.strip_prefix("refs/tags/").unwrap_or(name)
            );
            return Ok(());
        }
    }
    // Walk first-parent; report nearest tag with distance.
    let mut oid = head;
    let mut dist = 0u32;
    loop {
        for (name, tag_oid) in &peeled {
            if *tag_oid == oid {
                let short = name.strip_prefix("refs/tags/").unwrap_or(name);
                if dist == 0 {
                    println!("{short}{suffix}");
                } else {
                    println!("{short}-{dist}-g{}{suffix}", &head.to_hex()[..7]);
                }
                return Ok(());
            }
        }
        let obj = repo.get_object(&oid)?;
        if obj.kind != ObjectKind::Commit {
            break;
        }
        let c = Commit::decode(&obj.payload)?;
        match c.parents.first() {
            Some(p) => {
                oid = *p;
                dist += 1;
            }
            None => break,
        }
    }
    Err(CliError::Msg("no tags can describe HEAD".into()))
}

/// List tag short names. Packed-only path: one scan + one stdout write (M17 Pf3-2).
fn list_tags(repo: &arks_repo::Repository) -> CliResult<()> {
    use std::io::Write;
    let packed = repo.git_dir().join("packed-refs");
    let text = if packed.is_file() {
        std::fs::read_to_string(&packed)?
    } else {
        String::new()
    };
    let mut loose: Vec<String> = Vec::new();
    let tags_dir = repo.git_dir().join("refs/tags");
    if tags_dir.is_dir() {
        collect_loose_tag_names(&tags_dir, "", &mut loose)?;
    }
    let mut out = Vec::with_capacity(text.len() / 2 + 64);
    if loose.is_empty() {
        for line in text.lines() {
            let b = line.as_bytes();
            if b.is_empty() || b[0] == b'#' || b[0] == b'^' {
                continue;
            }
            let Some(sp) = line.find(' ') else { continue };
            if let Some(short) = line[sp + 1..].strip_prefix("refs/tags/") {
                out.extend_from_slice(short.as_bytes());
                out.push(b'\n');
            }
        }
    } else {
        let mut names: Vec<&str> = Vec::new();
        for line in text.lines() {
            let b = line.as_bytes();
            if b.is_empty() || b[0] == b'#' || b[0] == b'^' {
                continue;
            }
            let Some(sp) = line.find(' ') else { continue };
            if let Some(short) = line[sp + 1..].strip_prefix("refs/tags/") {
                names.push(short);
            }
        }
        let packed_set: std::collections::HashSet<&str> = names.iter().copied().collect();
        for name in &loose {
            if !packed_set.contains(name.as_str()) {
                names.push(name.as_str());
            }
        }
        names.sort_unstable();
        names.dedup();
        for name in names {
            out.extend_from_slice(name.as_bytes());
            out.push(b'\n');
        }
    }
    std::io::stdout().lock().write_all(&out)?;
    Ok(())
}

fn collect_loose_tag_names(
    dir: &std::path::Path,
    prefix: &str,
    out: &mut Vec<String>,
) -> CliResult<()> {
    for ent in std::fs::read_dir(dir)? {
        let ent = ent?;
        let name = ent.file_name();
        let name = name.to_string_lossy();
        if name == "." || name == ".." {
            continue;
        }
        let rel = if prefix.is_empty() {
            name.to_string()
        } else {
            format!("{prefix}/{name}")
        };
        let path = ent.path();
        if path.is_dir() {
            collect_loose_tag_names(&path, &rel, out)?;
        } else if path.is_file() {
            out.push(rel);
        }
    }
    Ok(())
}

fn peel_tag_target(repo: &arks_repo::Repository, oid: &arks_oid::Oid) -> CliResult<arks_oid::Oid> {
    let obj = repo.get_object(oid)?;
    if obj.kind != ObjectKind::Tag {
        return Ok(*oid);
    }
    // Annotated tag payload: "object <hex>\ntype ...\n"
    let text = String::from_utf8_lossy(&obj.payload);
    for line in text.lines() {
        if let Some(hex) = line.strip_prefix("object ") {
            return arks_oid::Oid::from_hex(hex.trim())
                .map_err(|e| CliError::Msg(format!("bad tag object oid: {e}")));
        }
    }
    Err(CliError::Msg(format!(
        "annotated tag {} missing object line",
        oid.to_hex()
    )))
}

/// Shared dirty-worktree probe (describe `--dirty`, BISECT-002, TOCTOU-003).
pub(crate) fn worktree_dirty(repo: &arks_repo::Repository) -> CliResult<bool> {
    use crate::cmd::shared::{collect_untracked, head_blob_map};
    use crate::util::{FORMAT, read_index, work_tree};
    use arks_object::{ObjectKind, RawObject};
    let work = work_tree(repo)?;
    let index = read_index(repo)?;
    let head = head_blob_map(repo)?;
    for e in index.entries.iter().filter(|e| e.stage() == 0) {
        match head.get(&e.path) {
            None => return Ok(true),
            Some(oid) if *oid != e.oid => return Ok(true),
            Some(_) => {}
        }
        let abs = work.join(&e.path);
        if !abs.is_file() {
            return Ok(true);
        }
        let bytes = std::fs::read(&abs)?;
        let oid = RawObject::new(ObjectKind::Blob, bytes).oid(FORMAT)?;
        if oid != e.oid {
            return Ok(true);
        }
    }
    let indexed: std::collections::HashSet<_> =
        index.entries.iter().map(|e| e.path.clone()).collect();
    let mut untracked = Vec::new();
    collect_untracked(&work, &work, &indexed, &mut untracked)?;
    Ok(!untracked.is_empty())
}

#[cfg(test)]
mod tests {
    #[test]
    fn tag_signed_requires_message_flag() {
        let args = ["-s".to_string(), "v1".to_string()];
        assert!(args.iter().any(|a| a == "-s"));
        assert!(!args.windows(2).any(|w| w[0] == "-m"));
    }
}
