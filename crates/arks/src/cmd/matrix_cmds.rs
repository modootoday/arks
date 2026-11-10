//! High-traffic Missing pin commands (matrix fill toward [[sot:root/cli-git-help-matrix]]).
//!
//! Cite: pin git 2.39.5 man pages; survey [[research:root/20261111034805]].

use crate::cmd::replay::put_blob;
use crate::cmd::shared::{resolve_rev, tree_blob_map};
use crate::cmd::working_tree::{cmd_diff, print_maps_diff, print_maps_summary};
use crate::error::{CliError, CliResult};
use crate::util::{open_repo, read_index, refuse_unknown_flags, work_tree, write_index};
use arks_index::{IndexEntry, sort_entries};
use arks_object::{ObjectKind, RawObject, Tree, TreeEntry, TreeMode, parse_patch};
use arks_oid::Oid;
use arks_repo::Repository;
use arks_store::ObjectStore;
use std::collections::HashSet;
use std::fs;
use std::io::{self, Read};
use std::path::Path;

/// `arks apply [<patch>]` — apply a format-patch mailbox to the worktree (+ optional index).
pub fn cmd_apply(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &["--index", "--cached"])?;
    let to_index = args.iter().any(|a| a == "--index" || a == "--cached");
    let file = args
        .iter()
        .find(|a| !a.starts_with('-'))
        .ok_or_else(|| CliError::Usage("apply [--index] <patch>".into()))?;
    let bytes = fs::read(file)?;
    let text = std::str::from_utf8(&bytes)
        .map_err(|_| CliError::NotImplemented("apply: non-utf8 patch not claimed".into()))?;
    let patch = parse_patch(text).map_err(|e| CliError::Msg(e.to_string()))?;
    let mut repo = open_repo()?;
    let work = work_tree(&repo)?;
    let mut index = read_index(&repo)?;
    for file in &patch.files {
        let abs = work.join(&file.path);
        match &file.tip {
            Some(payload) => {
                if let Some(parent) = abs.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(&abs, payload)?;
                if to_index {
                    let oid = put_blob(&mut repo, payload)?;
                    upsert_index(&mut index, &file.path, oid, payload.len() as u32);
                }
            }
            None => {
                if abs.is_file() {
                    fs::remove_file(&abs)?;
                }
                if to_index {
                    index.entries.retain(|e| e.path != file.path);
                }
            }
        }
    }
    if to_index {
        sort_entries(&mut index.entries);
        write_index(&repo, &index)?;
    }
    Ok(())
}

fn upsert_index(index: &mut arks_index::Index, path: &str, oid: Oid, size: u32) {
    index.entries.retain(|e| e.path != path);
    index.entries.push(IndexEntry {
        ctime_sec: 0,
        ctime_nsec: 0,
        mtime_sec: 0,
        mtime_nsec: 0,
        dev: 0,
        ino: 0,
        mode: 0o100644,
        uid: 0,
        gid: 0,
        size,
        oid,
        flags: (path.len().min(0x0fff)) as u16,
        flags_extended: None,
        path: path.to_owned(),
    });
}

/// `arks ls-remote [<options>] [<remote> [<patterns>…]]`
pub fn cmd_ls_remote(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &["--heads", "--tags", "-h", "-t"])?;
    let heads = args.iter().any(|a| a == "--heads" || a == "-h");
    let tags = args.iter().any(|a| a == "--tags" || a == "-t");
    let positional: Vec<&String> = args.iter().filter(|a| !a.starts_with('-')).collect();
    let remote_or_url = positional
        .first()
        .map(|s| s.as_str())
        .unwrap_or("origin");
    let patterns: Vec<&str> = positional.iter().skip(1).map(|s| s.as_str()).collect();
    let url = resolve_remote_url(remote_or_url)?;
    let refs = arks_remote::ls_remote(&url).map_err(|e| CliError::Msg(e.to_string()))?;
    for r in refs {
        if heads && !r.name.starts_with("refs/heads/") {
            continue;
        }
        if tags && !r.name.starts_with("refs/tags/") {
            continue;
        }
        if !patterns.is_empty()
            && !patterns.iter().any(|p| {
                r.name == *p
                    || r.name.strip_prefix("refs/heads/") == Some(*p)
                    || r.name.strip_prefix("refs/tags/") == Some(*p)
                    || r.name.contains(p)
            })
        {
            continue;
        }
        println!("{}\t{}", r.oid.to_hex(), r.name);
    }
    Ok(())
}

fn resolve_remote_url(name_or_url: &str) -> CliResult<String> {
    if name_or_url.contains("://")
        || name_or_url.starts_with("git@")
        || Path::new(name_or_url).exists()
    {
        return Ok(name_or_url.to_string());
    }
    let repo = open_repo()?;
    let cfg = repo.git_dir().join("config");
    let remotes = arks_remote::list_remotes(&cfg)?;
    remotes
        .into_iter()
        .find(|r| r.name == name_or_url)
        .map(|r| r.url)
        .ok_or_else(|| CliError::Msg(format!("No such remote: {name_or_url}")))
}

/// `arks prune` — delete unreachable loose objects.
pub fn cmd_prune(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &["-n", "--dry-run", "-v", "--verbose"])?;
    let dry = args.iter().any(|a| a == "-n" || a == "--dry-run");
    let verbose = args.iter().any(|a| a == "-v" || a == "--verbose");
    let repo = open_repo()?;
    let reachable = reachable_oids(&repo)?;
    let objects = repo.store().objects_dir();
    let mut removed = 0u64;
    if let Ok(rd) = fs::read_dir(objects) {
        for ent in rd.flatten() {
            let prefix = ent.file_name().to_string_lossy().to_string();
            if prefix.len() != 2 || !prefix.chars().all(|c| c.is_ascii_hexdigit()) {
                continue;
            }
            let dir = ent.path();
            if !dir.is_dir() {
                continue;
            }
            if let Ok(inner) = fs::read_dir(&dir) {
                for obj in inner.flatten() {
                    let hex = format!("{}{}", prefix, obj.file_name().to_string_lossy());
                    if hex.len() != 40 {
                        continue;
                    }
                    let Ok(oid) = Oid::from_hex(&hex) else {
                        continue;
                    };
                    if reachable.contains(&oid) {
                        continue;
                    }
                    if verbose || dry {
                        println!("{}", oid.to_hex());
                    }
                    if !dry {
                        let _ = fs::remove_file(obj.path());
                    }
                    removed += 1;
                }
            }
        }
    }
    if dry {
        eprintln!("Would remove {removed} loose object(s)");
    }
    Ok(())
}

/// `arks prune-packed` — remove loose objects also present in packs.
pub fn cmd_prune_packed(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &["-n", "--dry-run", "-q", "--quiet"])?;
    let dry = args.iter().any(|a| a == "-n" || a == "--dry-run");
    let quiet = args.iter().any(|a| a == "-q" || a == "--quiet");
    let repo = open_repo()?;
    let pack_oids = packed_oids(&repo)?;
    let objects = repo.store().objects_dir();
    let mut removed = 0u64;
    if let Ok(rd) = fs::read_dir(objects) {
        for ent in rd.flatten() {
            let prefix = ent.file_name().to_string_lossy().to_string();
            if prefix.len() != 2 || !prefix.chars().all(|c| c.is_ascii_hexdigit()) {
                continue;
            }
            let dir = ent.path();
            if !dir.is_dir() {
                continue;
            }
            if let Ok(inner) = fs::read_dir(&dir) {
                for obj in inner.flatten() {
                    let hex = format!("{}{}", prefix, obj.file_name().to_string_lossy());
                    let Ok(oid) = Oid::from_hex(&hex) else {
                        continue;
                    };
                    if !pack_oids.contains(&oid) {
                        continue;
                    }
                    if !quiet && !dry {
                        println!("{}", oid.to_hex());
                    }
                    if !dry {
                        let _ = fs::remove_file(obj.path());
                    }
                    removed += 1;
                }
            }
        }
    }
    if dry && !quiet {
        eprintln!("Would prune-packed {removed} loose object(s)");
    }
    Ok(())
}

fn reachable_oids(repo: &Repository) -> CliResult<HashSet<Oid>> {
    let mut seen = HashSet::new();
    let refs = repo.refs().list_refs("")?;
    for (_name, oid) in refs {
        walk_reach(repo, &oid, &mut seen)?;
    }
    if let Ok(head) = repo.head_oid() {
        walk_reach(repo, &head, &mut seen)?;
    }
    Ok(seen)
}

fn walk_reach(repo: &Repository, oid: &Oid, seen: &mut HashSet<Oid>) -> CliResult<()> {
    if !seen.insert(*oid) {
        return Ok(());
    }
    let obj = match repo.get_object(oid) {
        Ok(o) => o,
        Err(_) => return Ok(()),
    };
    match obj.kind {
        ObjectKind::Commit => {
            let c = arks_object::Commit::decode(&obj.payload)?;
            walk_reach(repo, &c.tree, seen)?;
            for p in c.parents {
                walk_reach(repo, &p, seen)?;
            }
        }
        ObjectKind::Tree => {
            let t = Tree::decode(&obj.payload, repo.oid_kind())?;
            for e in t.entries {
                walk_reach(repo, &e.oid, seen)?;
            }
        }
        ObjectKind::Blob => {}
        ObjectKind::Tag => {
            if let Some(target) = tag_target(&obj.payload) {
                walk_reach(repo, &target, seen)?;
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

fn packed_oids(repo: &Repository) -> CliResult<HashSet<Oid>> {
    let mut out = HashSet::new();
    let pack_dir = repo.store().objects_dir().join("pack");
    if !pack_dir.is_dir() {
        return Ok(out);
    }
    for ent in fs::read_dir(&pack_dir)?.flatten() {
        let name = ent.file_name();
        let name = name.to_string_lossy();
        if !name.ends_with(".idx") {
            continue;
        }
        let bytes = fs::read(ent.path())?;
        for oid in parse_pack_idx_oids(&bytes)? {
            out.insert(oid);
        }
    }
    Ok(out)
}

fn parse_pack_idx_oids(bytes: &[u8]) -> CliResult<Vec<Oid>> {
    if bytes.len() < 8 + 256 * 4 {
        return Ok(Vec::new());
    }
    if &bytes[0..4] != b"\xfftOc" {
        return Ok(Vec::new());
    }
    let version = u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    if version != 2 {
        return Ok(Vec::new());
    }
    let fanout_end = 8 + 256 * 4;
    let n = u32::from_be_bytes([
        bytes[fanout_end - 4],
        bytes[fanout_end - 3],
        bytes[fanout_end - 2],
        bytes[fanout_end - 1],
    ]) as usize;
    let need = fanout_end + n * 20;
    if bytes.len() < need {
        return Ok(Vec::new());
    }
    let mut oids = Vec::with_capacity(n);
    for i in 0..n {
        let start = fanout_end + i * 20;
        let hex: String = bytes[start..start + 20]
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        if let Ok(oid) = Oid::from_hex(&hex) {
            oids.push(oid);
        }
    }
    Ok(oids)
}

/// `arks cherry [<upstream> [<head>]]`
pub fn cmd_cherry(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &["-v"])?;
    let verbose = args.iter().any(|a| a == "-v");
    let positional: Vec<&String> = args.iter().filter(|a| !a.starts_with('-')).collect();
    let repo = open_repo()?;
    let head = match positional.get(1) {
        Some(h) => resolve_rev(&repo, h)?,
        None => repo.head_oid()?,
    };
    let upstream = match positional.first() {
        Some(u) => resolve_rev(&repo, u)?,
        None => default_upstream(&repo)?,
    };
    let up_set = first_parent_set(&repo, upstream)?;
    let mut oid = head;
    let mut listed = Vec::new();
    loop {
        if up_set.contains(&oid) {
            break;
        }
        listed.push(oid);
        let obj = repo.get_object(&oid)?;
        let c = arks_object::Commit::decode(&obj.payload)?;
        match c.parents.first() {
            Some(p) => oid = *p,
            None => break,
        }
    }
    listed.reverse();
    for oid in listed {
        if verbose {
            let c = arks_object::Commit::decode(&repo.get_object(&oid)?.payload)?;
            println!("+ {} {}", oid.to_hex(), c.subject());
        } else {
            println!("+ {}", oid.to_hex());
        }
    }
    Ok(())
}

fn default_upstream(repo: &Repository) -> CliResult<Oid> {
    let branch = match repo.head_target()? {
        arks_refs::RefTarget::Symbolic(s) => s
            .strip_prefix("refs/heads/")
            .map(|s| s.to_string())
            .ok_or_else(|| CliError::Msg("cherry: detached HEAD needs <upstream>".into()))?,
        arks_refs::RefTarget::Direct(_) => {
            return Err(CliError::Msg(
                "cherry: detached HEAD needs <upstream>".into(),
            ));
        }
    };
    let cfg = fs::read_to_string(repo.git_dir().join("config")).unwrap_or_default();
    let remote =
        config_val(&cfg, &format!("branch.{branch}.remote")).unwrap_or_else(|| "origin".into());
    let merge = config_val(&cfg, &format!("branch.{branch}.merge")).ok_or_else(|| {
        CliError::Msg(format!(
            "cherry: no upstream configured for branch '{branch}'"
        ))
    })?;
    let short = merge.strip_prefix("refs/heads/").unwrap_or(&merge);
    let tracking = format!("refs/remotes/{remote}/{short}");
    repo.refs()
        .resolve(&tracking)
        .or_else(|_| repo.refs().resolve(&merge))
        .map_err(|_| CliError::Msg(format!("cherry: missing upstream ref {tracking}")))
}

fn config_val(cfg: &str, key: &str) -> Option<String> {
    let (section, rest) = key.split_once('.')?;
    let (sub, keyname) = if let Some((a, b)) = rest.split_once('.') {
        (Some(a), b)
    } else {
        (None, rest)
    };
    let mut in_sec = false;
    for raw in cfg.lines() {
        let line = raw.trim();
        if line.starts_with('[') && line.ends_with(']') {
            let inner = &line[1..line.len() - 1];
            in_sec = match sub {
                Some(s) => {
                    let want = format!("{section} \"{s}\"");
                    inner.eq_ignore_ascii_case(&want)
                        || inner.eq_ignore_ascii_case(&format!("{section} '{s}'"))
                }
                None => inner.eq_ignore_ascii_case(section),
            };
            continue;
        }
        if in_sec {
            if let Some((k, v)) = line.split_once('=') {
                if k.trim().eq_ignore_ascii_case(keyname) {
                    return Some(v.trim().trim_matches('"').to_string());
                }
            }
        }
    }
    None
}

fn first_parent_set(repo: &Repository, tip: Oid) -> CliResult<HashSet<Oid>> {
    let mut set = HashSet::new();
    let mut oid = tip;
    loop {
        if !set.insert(oid) {
            break;
        }
        let obj = repo.get_object(&oid)?;
        let c = arks_object::Commit::decode(&obj.payload)?;
        match c.parents.first() {
            Some(p) => oid = *p,
            None => break,
        }
    }
    Ok(set)
}

/// `arks check-ref-format <refname>`
pub fn cmd_check_ref_format(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &["--allow-onelevel", "--normalize", "--branch"])?;
    let allow_one = args.iter().any(|a| a == "--allow-onelevel");
    let name = args
        .iter()
        .find(|a| !a.starts_with('-'))
        .ok_or_else(|| CliError::Usage("check-ref-format [--allow-onelevel] <refname>".into()))?;
    if refname_ok(name, allow_one) {
        Ok(())
    } else {
        Err(CliError::Msg(format!("'{name}' is not a valid refname")))
    }
}

fn refname_ok(name: &str, allow_onelevel: bool) -> bool {
    if name.is_empty() || name == "@" {
        return false;
    }
    if name.contains("..") || name.contains("@{") || name.contains('\\') {
        return false;
    }
    if name.starts_with('/') || name.ends_with('/') || name.ends_with('.') {
        return false;
    }
    if name.contains("//") || name.contains(' ') {
        return false;
    }
    for ch in name.chars() {
        if ch.is_ascii_control() || "^:~?*[".contains(ch) {
            return false;
        }
    }
    let has_slash = name.contains('/');
    if !has_slash && !allow_onelevel {
        return false;
    }
    !name
        .split('/')
        .any(|p| p.is_empty() || p.starts_with('.') || p.ends_with(".lock"))
}

/// `arks stripspace` — trim trailing whitespace / blank lines (stdin→stdout).
pub fn cmd_stripspace(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &["-s", "--strip-comments", "-c", "--comment-lines"])?;
    let strip_comments = args
        .iter()
        .any(|a| a == "-s" || a == "--strip-comments");
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;
    let mut out = String::new();
    for line in input.lines() {
        let l = line.trim_end();
        if strip_comments && l.trim_start().starts_with('#') {
            continue;
        }
        out.push_str(l);
        out.push('\n');
    }
    while out.ends_with("\n\n") {
        out.pop();
    }
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    print!("{out}");
    Ok(())
}

/// `arks mktag` — read tag object from stdin, write if headers look valid.
pub fn cmd_mktag(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &[])?;
    let mut buf = Vec::new();
    io::stdin().read_to_end(&mut buf)?;
    let text = std::str::from_utf8(&buf)
        .map_err(|_| CliError::Msg("mktag: tag payload must be utf-8".into()))?;
    let mut has_object = false;
    let mut has_type = false;
    let mut has_tag = false;
    for line in text.lines() {
        if line.is_empty() {
            break;
        }
        if let Some(rest) = line.strip_prefix("object ") {
            has_object = Oid::from_hex(rest.trim()).is_ok();
        } else if line.starts_with("type ") {
            has_type = true;
        } else if line.starts_with("tag ") {
            has_tag = true;
        }
    }
    if !(has_object && has_type && has_tag) {
        return Err(CliError::Msg(
            "mktag: missing object/type/tag headers".into(),
        ));
    }
    let mut repo = open_repo()?;
    let format = repo.object_format();
    let obj = RawObject::new(ObjectKind::Tag, buf);
    let oid = repo.store_mut().put(format, &obj)?;
    println!("{}", oid.to_hex());
    Ok(())
}

/// `arks mktree` — build tree from ls-tree style stdin lines.
pub fn cmd_mktree(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &["-z", "--missing"])?;
    let nul = args.iter().any(|a| a == "-z");
    let mut buf = Vec::new();
    io::stdin().read_to_end(&mut buf)?;
    if nul {
        for b in &mut buf {
            if *b == 0 {
                *b = b'\n';
            }
        }
    }
    let text = String::from_utf8(buf).map_err(|_| CliError::Msg("mktree: invalid utf-8".into()))?;
    let mut entries = Vec::new();
    for line in text.lines() {
        if line.is_empty() {
            continue;
        }
        let (meta, name) = line
            .split_once('\t')
            .ok_or_else(|| CliError::Usage("mktree: expected mode type oid\\tname".into()))?;
        let parts: Vec<&str> = meta.split_whitespace().collect();
        if parts.len() != 3 {
            return Err(CliError::Usage(
                "mktree: expected '<mode> <type> <oid>\\t<name>'".into(),
            ));
        }
        let mode = TreeMode::parse(parts[0])
            .map_err(|e| CliError::Msg(format!("mktree: bad mode {}: {e}", parts[0])))?;
        let oid = Oid::from_hex(parts[2]).map_err(|e| CliError::Msg(e.to_string()))?;
        entries.push(TreeEntry {
            mode,
            name: name.to_string(),
            oid,
        });
    }
    let tree = Tree::new(entries).map_err(|e| CliError::Msg(e.to_string()))?;
    let payload = tree.encode().map_err(|e| CliError::Msg(e.to_string()))?;
    let mut repo = open_repo()?;
    let format = repo.object_format();
    let obj = RawObject::new(ObjectKind::Tree, payload);
    let oid = repo.store_mut().put(format, &obj)?;
    println!("{}", oid.to_hex());
    Ok(())
}

/// `arks diff-files` — worktree vs index.
pub fn cmd_diff_files(args: &[String]) -> CliResult<()> {
    let mut forwarded = Vec::new();
    for a in args {
        if a == "--cached" || a == "--staged" {
            return Err(CliError::Usage(
                "diff-files: use diff-index --cached for index vs tree".into(),
            ));
        }
        forwarded.push(a.clone());
    }
    cmd_diff(&forwarded)
}

/// `arks diff-index [--cached] <tree-ish>`
pub fn cmd_diff_index(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(
        args,
        &["--cached", "--name-only", "--name-status", "--stat"],
    )?;
    let name_only = args.iter().any(|a| a == "--name-only");
    let name_status = args.iter().any(|a| a == "--name-status");
    let stat = args.iter().any(|a| a == "--stat");
    let treeish = args
        .iter()
        .find(|a| !a.starts_with('-'))
        .ok_or_else(|| CliError::Usage("diff-index [--cached] <tree-ish>".into()))?;
    let repo = open_repo()?;
    let tree_map = tree_blob_map(&repo, &resolve_rev(&repo, treeish)?)?;
    let index = read_index(&repo)?;
    let mut idx_map = std::collections::HashMap::new();
    for e in index.entries.iter().filter(|e| e.stage() == 0) {
        idx_map.insert(e.path.clone(), e.oid);
    }
    if name_only || name_status || stat {
        return print_maps_summary(&tree_map, &idx_map, name_only, name_status, stat);
    }
    print_maps_diff(&repo, &tree_map, &idx_map)
}

/// `arks var [-l] <variable>`
pub fn cmd_var(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &["-l"])?;
    let list = args.iter().any(|a| a == "-l");
    let repo = open_repo().ok();
    let mut pairs = Vec::new();
    if let Some(repo) = &repo {
        pairs.push(("GIT_DIR", repo.git_dir().display().to_string()));
        pairs.push((
            "GIT_OBJECT_DIRECTORY",
            repo.store().objects_dir().display().to_string(),
        ));
        if let Ok(wt) = work_tree(repo) {
            pairs.push(("GIT_WORK_TREE", wt.display().to_string()));
        }
        pairs.push(("GIT_COMMON_DIR", repo.common_dir().display().to_string()));
    }
    if list {
        for (k, v) in &pairs {
            println!("{k}={v}");
        }
        return Ok(());
    }
    let key = args
        .iter()
        .find(|a| !a.starts_with('-'))
        .ok_or_else(|| CliError::Usage("var [-l] <variable>".into()))?;
    for (k, v) in pairs {
        if k == key.as_str() {
            println!("{v}");
            return Ok(());
        }
    }
    Err(CliError::Msg(format!("unknown variable: {key}")))
}

/// `arks patch-id` — stable id for a diff on stdin.
pub fn cmd_patch_id(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &["--stable", "--unstable"])?;
    let mut buf = Vec::new();
    io::stdin().read_to_end(&mut buf)?;
    let text = String::from_utf8_lossy(&buf);
    let mut filtered = String::new();
    for line in text.lines() {
        if line.starts_with("index ")
            || line.starts_with("diff --git ")
            || line.starts_with("--- ")
            || line.starts_with("+++ ")
        {
            continue;
        }
        filtered.push_str(line);
        filtered.push('\n');
    }
    let obj = RawObject::new(ObjectKind::Blob, filtered.into_bytes());
    let format = open_repo()
        .map(|r| r.object_format())
        .unwrap_or(arks_store::ObjectFormat::Sha1);
    let oid = obj.oid(format)?;
    println!("{} {}", oid.to_hex(), "0".repeat(40));
    Ok(())
}
