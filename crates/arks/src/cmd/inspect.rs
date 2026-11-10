//! Ancillary interrogators: blame / annotate / fsck / count-objects / pack-refs.
//!
//! Cite: <https://git-scm.com/docs/git-blame/2.39.0>
//! Cite: <https://git-scm.com/docs/git-fsck/2.39.0>
//! Cite: <https://git-scm.com/docs/git-count-objects/2.39.0>
//! Cite: <https://git-scm.com/docs/git-pack-refs/2.39.0>
//! Connectivity fsck is a claimed subset (reachable objects must load).

use crate::cmd::replay::{read_commit, tree_map};
use crate::error::{CliError, CliResult};
use crate::util::{open_repo, refuse_unknown_flags, work_tree};
use arks_object::{Commit, ObjectKind, Tree};
use arks_oid::Oid;
use arks_repo::Repository;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

pub fn cmd_blame(args: &[String]) -> CliResult<()> {
    blame_path(args)
}

pub fn cmd_annotate(args: &[String]) -> CliResult<()> {
    // git-annotate is a synonym for blame (claimed minimal path form).
    blame_path(args)
}

fn blame_path(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &[])?;
    let path = args
        .iter()
        .find(|a| !a.starts_with('-'))
        .ok_or_else(|| CliError::Usage("blame <path>".into()))?;
    let repo = open_repo()?;
    let mut oid = repo.head_oid()?;
    let work = work_tree(&repo)?;
    let abs = work.join(path);
    let current = fs::read(&abs).map_err(|_| CliError::Msg(format!("no such path: {path}")))?;
    let lines: Vec<&[u8]> = split_lines(&current);
    // Walk first-parent; attribute each line to the newest commit whose blob still matches.
    let mut attrs: Vec<(Oid, String)> = Vec::with_capacity(lines.len());
    for _ in &lines {
        attrs.push((oid, String::new()));
    }
    loop {
        let commit = read_commit(&repo, &oid)?;
        let map = tree_map(&repo, &oid)?;
        if let Some(blob_oid) = map.get(path.as_str()) {
            let blob = repo.get_object(blob_oid)?;
            let blob_lines = split_lines(&blob.payload);
            for (i, line) in lines.iter().enumerate() {
                if i < blob_lines.len() && blob_lines[i] == *line {
                    attrs[i] = (oid, commit.author.name.clone());
                }
            }
        }
        match commit.parents.first() {
            Some(p) => oid = *p,
            None => break,
        }
    }
    for (i, line) in lines.iter().enumerate() {
        let (coid, author) = &attrs[i];
        let short = &coid.to_hex()[..7];
        let text = String::from_utf8_lossy(line);
        println!("{short} ({author} {}) {text}", i + 1);
    }
    Ok(())
}

fn split_lines(bytes: &[u8]) -> Vec<&[u8]> {
    if bytes.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut start = 0;
    for (i, b) in bytes.iter().enumerate() {
        if *b == b'\n' {
            out.push(&bytes[start..i]);
            start = i + 1;
        }
    }
    if start < bytes.len() {
        out.push(&bytes[start..]);
    } else if bytes.ends_with(b"\n") {
        // trailing newline → no extra empty line (git blame style for final \n)
    }
    out
}

pub fn cmd_fsck(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &[])?;
    let repo = open_repo()?;
    let refs = repo.refs().list_refs("")?;
    let mut seen = HashSet::new();
    let mut missing = Vec::new();
    for (_name, oid) in &refs {
        walk_check(&repo, oid, &mut seen, &mut missing)?;
    }
    if let Ok(head) = repo.head_oid() {
        walk_check(&repo, &head, &mut seen, &mut missing)?;
    }
    if missing.is_empty() {
        println!("fsck: {} reachable object(s) ok", seen.len());
        Ok(())
    } else {
        for m in &missing {
            eprintln!("missing {}", m.to_hex());
        }
        Err(CliError::Msg(format!(
            "fsck: {} missing object(s)",
            missing.len()
        )))
    }
}

fn walk_check(
    repo: &Repository,
    oid: &Oid,
    seen: &mut HashSet<Oid>,
    missing: &mut Vec<Oid>,
) -> CliResult<()> {
    if !seen.insert(*oid) {
        return Ok(());
    }
    let obj = match repo.get_object(oid) {
        Ok(o) => o,
        Err(_) => {
            missing.push(*oid);
            return Ok(());
        }
    };
    // OBJ-004: recompute content OID — refuse collision pretends / mismatched content /
    // sha1cd-detected cryptanalytic collision attacks (detect+refuse; immunity Impossible).
    match obj.oid(repo.object_format()) {
        Ok(got) if got == *oid => {}
        Ok(got) => {
            return Err(CliError::Msg(format!(
                "fsck: OID mismatch expected {} got {} (OBJ-004 — hostile/corrupt object refuse; sha1 collision pretend or mismatched content)",
                oid.to_hex(),
                got.to_hex()
            )));
        }
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("SHA-1 collision") || msg.contains("Sha1Collision") {
                return Err(CliError::Msg(format!(
                    "fsck: SHA-1 collision attack detected for {} (OBJ-004 detect+refuse; immunity Impossible)",
                    oid.to_hex()
                )));
            }
            return Err(CliError::Msg(format!(
                "fsck: cannot recompute OID for {}: {e} (OBJ-004)",
                oid.to_hex()
            )));
        }
    }
    match obj.kind {
        ObjectKind::Commit => {
            let c = Commit::decode(&obj.payload)?;
            walk_check(repo, &c.tree, seen, missing)?;
            for p in &c.parents {
                walk_check(repo, p, seen, missing)?;
            }
        }
        ObjectKind::Tree => {
            let tree = Tree::decode(&obj.payload, repo.oid_kind())?;
            for e in tree.entries {
                walk_check(repo, &e.oid, seen, missing)?;
            }
        }
        ObjectKind::Blob => {}
        ObjectKind::Tag => {
            if let Some(target) = tag_target(&obj.payload) {
                walk_check(repo, &target, seen, missing)?;
            }
        }
    }
    Ok(())
}

fn tag_target(payload: &[u8]) -> Option<Oid> {
    let text = std::str::from_utf8(payload).ok()?;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("object ") {
            return Oid::from_hex(rest.trim()).ok();
        }
        if line.is_empty() {
            break;
        }
    }
    None
}

pub fn cmd_count_objects(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &["-v", "--verbose"])?;
    let verbose = args.iter().any(|a| a == "-v" || a == "--verbose");
    let repo = open_repo()?;
    let objects = repo.store().objects_dir();
    let (count, size_kb) = count_loose(objects)?;
    let (packs, in_pack, size_pack_kb) = count_packs(objects)?;
    if verbose {
        println!("count: {count}");
        println!("size: {size_kb}");
        println!("in-pack: {in_pack}");
        println!("packs: {packs}");
        println!("size-pack: {size_pack_kb}");
        println!("prune-packable: 0");
        println!("garbage: 0");
        println!("size-garbage: 0");
    } else {
        println!("{count} objects, {size_kb} kilobytes");
    }
    Ok(())
}

fn count_loose(objects: &Path) -> CliResult<(u64, u64)> {
    let mut count = 0u64;
    let mut bytes = 0u64;
    if !objects.is_dir() {
        return Ok((0, 0));
    }
    for ent in fs::read_dir(objects)? {
        let ent = ent?;
        let name = ent.file_name();
        let name = name.to_string_lossy();
        if name.len() != 2 || name == "pack" || name == "info" {
            continue;
        }
        let dir = ent.path();
        if !dir.is_dir() {
            continue;
        }
        for obj in fs::read_dir(&dir)? {
            let obj = obj?;
            if obj.file_type()?.is_file() {
                count += 1;
                bytes += obj.metadata()?.len();
            }
        }
    }
    Ok((count, (bytes + 512) / 1024))
}

fn count_packs(objects: &Path) -> CliResult<(u64, u64, u64)> {
    let pack_dir = objects.join("pack");
    if !pack_dir.is_dir() {
        return Ok((0, 0, 0));
    }
    let mut packs = 0u64;
    let mut size = 0u64;
    for ent in fs::read_dir(&pack_dir)? {
        let ent = ent?;
        let name = ent.file_name();
        let name = name.to_string_lossy();
        if name.ends_with(".pack") {
            packs += 1;
            size += ent.metadata()?.len();
        }
    }
    // in-pack: best-effort — count idx entries if we can; else 0.
    let mut in_pack = 0u64;
    for ent in fs::read_dir(&pack_dir)? {
        let ent = ent?;
        let name = ent.file_name();
        let name = name.to_string_lossy();
        if name.ends_with(".idx") {
            // Fan-out table ends at byte 1028; object count is not trivial without full parse.
            // Claimed subset: report 0 when unknown; packs/size-pack still useful.
            let _ = ent;
            let _ = &mut in_pack;
        }
    }
    Ok((packs, in_pack, (size + 512) / 1024))
}

pub fn cmd_pack_refs(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &["--all"])?;
    if !args.iter().any(|a| a == "--all") {
        return Err(CliError::Usage(
            "pack-refs --all (Sy-5 claimed form)".into(),
        ));
    }
    let mut repo = open_repo()?;
    // REF-012 (M20 Cx-3): pack-refs rewrites packed-refs under one exclusive lock.
    // Cite: https://git-scm.com/docs/git-pack-refs + api-lockfile
    let packed_lock = repo.common_dir().join("packed-refs.lock");
    if packed_lock.is_file() {
        return Err(CliError::Msg(format!(
            "Unable to create '{}' (REF-012 — pack-refs while live updates / packed-refs.lock held; lease ≠ skip)",
            packed_lock.display()
        )));
    }
    let n = repo
        .refs_mut()
        .pack_refs_all()
        .map_err(|e| CliError::Msg(e.to_string()))?;
    println!("packed {n} ref(s)");
    Ok(())
}
