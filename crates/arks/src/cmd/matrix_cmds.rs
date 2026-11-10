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
use std::path::{Path, PathBuf};

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
    let remote_or_url = positional.first().map(|s| s.as_str()).unwrap_or("origin");
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
        arks_refs::RefTarget::Symbolic(s) => {
            s.strip_prefix("refs/heads/")
                .map(|s| s.to_string())
                .ok_or_else(|| CliError::Msg("cherry: detached HEAD needs <upstream>".into()))?
        }
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
    let strip_comments = args.iter().any(|a| a == "-s" || a == "--strip-comments");
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

/// `arks credential-store` — native file-store helper (never spawns `git`).
pub fn cmd_credential_store(args: &[String]) -> CliResult<()> {
    let op = args
        .first()
        .map(String::as_str)
        .ok_or_else(|| CliError::Usage("credential-store <get|store|erase>".into()))?;
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;
    let desc = arks_remote::Credential::parse(&input).map_err(|e| CliError::Msg(e.to_string()))?;
    let mut source = arks_remote::CredentialSource::empty();
    source.helpers = vec!["store".into()];
    match op {
        "get" => {
            let filled = arks_remote::credential_fill(desc, &source)
                .map_err(|e| CliError::Msg(e.to_string()))?;
            if filled.password.is_some() || filled.username.is_some() {
                print!("{}", filled.format_attributes());
            }
            Ok(())
        }
        "store" => arks_remote::credential_approve(&desc, &source)
            .map_err(|e| CliError::Msg(e.to_string())),
        "erase" => {
            arks_remote::credential_reject(&desc, &source).map_err(|e| CliError::Msg(e.to_string()))
        }
        other => Err(CliError::Usage(format!(
            "credential-store: unknown op '{other}'"
        ))),
    }
}

/// `arks show-index` — list oid/offset from a pack `.idx` on stdin or path.
pub fn cmd_show_index(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &[])?;
    let bytes = if let Some(path) = args.first() {
        fs::read(path)?
    } else {
        let mut buf = Vec::new();
        io::stdin().read_to_end(&mut buf)?;
        buf
    };
    let format = open_repo()
        .map(|r| r.object_format())
        .unwrap_or(arks_store::ObjectFormat::Sha1);
    let idx = arks_pack::PackIndex::decode(&bytes, format)?;
    for e in &idx.entries {
        println!("{}\t{}", e.oid.to_hex(), e.offset);
    }
    Ok(())
}

/// `arks check-attr` — report attributes for paths from `.gitattributes` (subset).
pub fn cmd_check_attr(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &["-a", "--all", "-z"])?;
    let all = args.iter().any(|a| a == "-a" || a == "--all");
    let positional: Vec<&String> = args.iter().filter(|a| !a.starts_with('-')).collect();
    if positional.is_empty() {
        return Err(CliError::Usage(
            "check-attr [-a] <attr>… [--] <pathname>…".into(),
        ));
    }
    // Split attrs vs paths at `--` when present.
    let (attrs, paths) = if let Some(i) = positional.iter().position(|a| *a == "--") {
        (positional[..i].to_vec(), positional[i + 1..].to_vec())
    } else if all {
        (Vec::new(), positional)
    } else if positional.len() >= 2 {
        (vec![positional[0]], positional[1..].to_vec())
    } else {
        return Err(CliError::Usage(
            "check-attr: need attribute name(s) and path(s)".into(),
        ));
    };
    let repo = open_repo()?;
    let work = work_tree(&repo)?;
    let rules = load_gitattributes(&work)?;
    for path in paths {
        let matched = match_attrs(&rules, path);
        if all || attrs.is_empty() {
            for (k, v) in &matched {
                println!("{path}: {k}: {v}");
            }
        } else {
            for a in &attrs {
                let v = matched
                    .get(a.as_str())
                    .map(String::as_str)
                    .unwrap_or("unspecified");
                println!("{path}: {a}: {v}");
            }
        }
    }
    Ok(())
}

fn load_gitattributes(work: &Path) -> CliResult<Vec<(String, Vec<(String, String)>)>> {
    let path = work.join(".gitattributes");
    let text = match fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(CliError::Msg(e.to_string())),
    };
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split_whitespace();
        let Some(pat) = parts.next() else {
            continue;
        };
        let mut attrs = Vec::new();
        for a in parts {
            if let Some((k, v)) = a.split_once('=') {
                attrs.push((k.to_string(), v.to_string()));
            } else if let Some(k) = a.strip_prefix('-') {
                attrs.push((k.to_string(), "unset".into()));
            } else {
                attrs.push((a.to_string(), "set".into()));
            }
        }
        out.push((pat.to_string(), attrs));
    }
    Ok(out)
}

fn match_attrs(
    rules: &[(String, Vec<(String, String)>)],
    path: &str,
) -> std::collections::BTreeMap<String, String> {
    let mut map = std::collections::BTreeMap::new();
    let base = Path::new(path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(path);
    for (pat, attrs) in rules {
        let ok = if pat == "*" || pat == "**" {
            true
        } else if let Some(suf) = pat.strip_prefix('*') {
            path.ends_with(suf) || base.ends_with(suf)
        } else {
            path == pat || base == pat
        };
        if ok {
            for (k, v) in attrs {
                map.insert(k.clone(), v.clone());
            }
        }
    }
    map
}

/// `arks merge-file` — three-way line merge of `<base> <ours> <theirs>` files.
pub fn cmd_merge_file(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &["-p", "--stdout", "-q", "--quiet"])?;
    let to_stdout = args.iter().any(|a| a == "-p" || a == "--stdout");
    let quiet = args.iter().any(|a| a == "-q" || a == "--quiet");
    let positional: Vec<&String> = args.iter().filter(|a| !a.starts_with('-')).collect();
    if positional.len() != 3 {
        return Err(CliError::Usage(
            "merge-file [-p] <current> <base> <other>".into(),
        ));
    }
    let ours = fs::read_to_string(positional[0].as_str())?;
    let base = fs::read_to_string(positional[1].as_str())?;
    let theirs = fs::read_to_string(positional[2].as_str())?;
    let (merged, conflicts) = merge_lines(&base, &ours, &theirs);
    if to_stdout {
        print!("{merged}");
    } else {
        fs::write(positional[0].as_str(), &merged)?;
    }
    if conflicts > 0 {
        if !quiet {
            eprintln!("{conflicts} conflict(s) during merge-file");
        }
        return Err(CliError::Msg(format!("merge-file: {conflicts} conflicts")));
    }
    Ok(())
}

fn merge_lines(base: &str, ours: &str, theirs: &str) -> (String, usize) {
    // Minimal LCS-free three-way: if ours==theirs take that; if equals base take other;
    // else conflict markers. Line-granular whole-file when any side differs.
    if ours == theirs {
        return (ours.to_string(), 0);
    }
    if ours == base {
        return (theirs.to_string(), 0);
    }
    if theirs == base {
        return (ours.to_string(), 0);
    }
    let mut out = String::new();
    out.push_str("<<<<<<< ours\n");
    out.push_str(ours);
    if !ours.ends_with('\n') {
        out.push('\n');
    }
    out.push_str("=======\n");
    out.push_str(theirs);
    if !theirs.ends_with('\n') {
        out.push('\n');
    }
    out.push_str(">>>>>>> theirs\n");
    (out, 1)
}

/// `arks merge-tree <base> <ours> <theirs>` — print merged tree oid (conflict → err).
pub fn cmd_merge_tree(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &[])?;
    let positional: Vec<&String> = args.iter().filter(|a| !a.starts_with('-')).collect();
    if positional.len() != 3 {
        return Err(CliError::Usage(
            "merge-tree <base-tree> <our-tree> <their-tree>".into(),
        ));
    }
    // Accept commit-ish or tree-ish; resolve to commit tips when possible for three-way.
    let mut repo = open_repo()?;
    let work = work_tree(&repo)?;
    let ours = resolve_rev(&repo, positional[1])?;
    let theirs = resolve_rev(&repo, positional[2])?;
    // Prefer commit three-way; if args are pure trees, refuse for now.
    let ours_obj = repo
        .get_object(&ours)
        .map_err(|e| CliError::Msg(e.to_string()))?;
    let theirs_obj = repo
        .get_object(&theirs)
        .map_err(|e| CliError::Msg(e.to_string()))?;
    if ours_obj.kind != ObjectKind::Commit || theirs_obj.kind != ObjectKind::Commit {
        return Err(CliError::NotImplemented(
            "merge-tree: tree-only args not claimed; pass commit-ish tips".into(),
        ));
    }
    let result =
        crate::cmd::merge_engine::merge_commits_three_way(&mut repo, &work, &ours, &theirs)?;
    if !result.conflicts.is_empty() {
        for c in &result.conflicts {
            println!("CONFLICT {c}");
        }
        return Err(CliError::Msg(format!(
            "merge-tree: {} conflict(s)",
            result.conflicts.len()
        )));
    }
    let tree = result
        .tree
        .ok_or_else(|| CliError::Msg("merge-tree: no result tree".into()))?;
    println!("{}", tree.to_hex());
    Ok(())
}

/// `arks hook run <name>` — run a sync hook from `$GIT_DIR/hooks` (subset).
pub fn cmd_hook(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &[])?;
    let sub = args
        .first()
        .map(String::as_str)
        .ok_or_else(|| CliError::Usage("hook run <hook-name>".into()))?;
    match sub {
        "run" => {
            let name = args
                .get(1)
                .ok_or_else(|| CliError::Usage("hook run <hook-name>".into()))?;
            let repo = open_repo()?;
            crate::cmd::hooks::run_sync_hooks(&repo, false, &[name.as_str()], None)
        }
        "list" => {
            let repo = open_repo()?;
            let dir = repo.git_dir().join("hooks");
            if let Ok(rd) = fs::read_dir(dir) {
                for e in rd.flatten() {
                    let n = e.file_name().to_string_lossy().to_string();
                    if n.ends_with(".sample") {
                        continue;
                    }
                    println!("{n}");
                }
            }
            Ok(())
        }
        other => Err(CliError::NotImplemented(format!(
            "hook '{other}' not claimed (run|list subset)"
        ))),
    }
}

/// `arks unpack-objects` — explode a pack from stdin into loose objects.
pub fn cmd_unpack_objects(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &["-q", "--quiet", "-n", "--dry-run"])?;
    let quiet = args.iter().any(|a| a == "-q" || a == "--quiet");
    let dry = args.iter().any(|a| a == "-n" || a == "--dry-run");
    let mut buf = Vec::new();
    io::stdin().read_to_end(&mut buf)?;
    let mut repo = open_repo()?;
    let format = repo.object_format();
    let decoded = arks_pack::DecodedPack::decode(&buf, format)?;
    let mut n = 0u64;
    for obj in &decoded.objects {
        if !dry {
            let _ = repo.store_mut().put(format, &obj.object)?;
        }
        n += 1;
    }
    if !quiet {
        eprintln!("Unpacked {n} object(s)");
    }
    Ok(())
}

/// `arks verify-pack [-v] <pack>` — validate pack + companion idx when present.
pub fn cmd_verify_pack(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &["-v", "--verbose", "-s", "--stat-only"])?;
    let verbose = args.iter().any(|a| a == "-v" || a == "--verbose");
    let pack_path = args
        .iter()
        .find(|a| !a.starts_with('-'))
        .ok_or_else(|| CliError::Usage("verify-pack [-v] <pack>".into()))?;
    let bytes = fs::read(pack_path)?;
    let format = open_repo()
        .map(|r| r.object_format())
        .unwrap_or(arks_store::ObjectFormat::Sha1);
    let decoded = arks_pack::DecodedPack::decode(&bytes, format)?;
    let idx_path = {
        let p = Path::new(pack_path);
        if let Some(stem) = p.file_stem() {
            p.with_file_name(format!("{}.idx", stem.to_string_lossy()))
        } else {
            Path::new(&format!("{pack_path}.idx")).to_path_buf()
        }
    };
    // pack-*.pack → pack-*.idx
    let idx_path = if pack_path.ends_with(".pack") {
        Path::new(&pack_path.replace(".pack", ".idx")).to_path_buf()
    } else {
        idx_path
    };
    if idx_path.is_file() {
        let idx_bytes = fs::read(&idx_path)?;
        let idx = arks_pack::PackIndex::decode(&idx_bytes, format)?;
        if idx.entries.len() != decoded.objects.len() {
            return Err(CliError::Msg(format!(
                "verify-pack: idx count {} != pack objects {}",
                idx.entries.len(),
                decoded.objects.len()
            )));
        }
        if verbose {
            for e in &idx.entries {
                println!("{}\t{}", e.oid.to_hex(), e.offset);
            }
        }
    } else if verbose {
        eprintln!(
            "note: no idx beside pack; pack decode ok ({} objects)",
            decoded.objects.len()
        );
    }
    println!("ok: {} object(s) in {}", decoded.objects.len(), pack_path);
    Ok(())
}

/// `arks show-branch` — list branch tips (subset; no --topo-order graph).
pub fn cmd_show_branch(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &["--all", "-a", "--list"])?;
    let all = args.iter().any(|a| a == "--all" || a == "-a");
    let repo = open_repo()?;
    let mut names: Vec<String> = args
        .iter()
        .filter(|a| !a.starts_with('-'))
        .cloned()
        .collect();
    if names.is_empty() || all {
        names.clear();
        for (name, _oid) in repo.refs().list_refs("refs/heads/")? {
            if let Some(short) = name.strip_prefix("refs/heads/") {
                names.push(short.to_string());
            }
        }
        names.sort();
    }
    for n in names {
        let full = if n.starts_with("refs/") {
            n.clone()
        } else {
            format!("refs/heads/{n}")
        };
        match repo.refs().resolve(&full) {
            Ok(oid) => println!("[{}] {}", &oid.to_hex()[..7.min(oid.to_hex().len())], n),
            Err(_) => println!("[gone] {n}"),
        }
    }
    Ok(())
}

/// `arks check-mailmap` — map identities via `.mailmap` (subset).
pub fn cmd_check_mailmap(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &[])?;
    if args.is_empty() {
        return Err(CliError::Usage("check-mailmap <identity>…".into()));
    }
    let repo = open_repo()?;
    let work = work_tree(&repo).unwrap_or_else(|_| repo.git_dir().to_path_buf());
    let map = load_mailmap(&work.join(".mailmap"))?;
    for id in args {
        println!("{}", map_identity(&map, id));
    }
    Ok(())
}

fn load_mailmap(path: &Path) -> CliResult<Vec<(String, String)>> {
    let text = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(CliError::Msg(e.to_string())),
    };
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        // Proper Name <proper@email> Commit Name <commit@email>
        // or Proper Name <proper@email>
        if let Some((left, right)) = line.split_once('>') {
            let proper = format!("{left}>").trim().to_string();
            let rest = right.trim();
            if rest.is_empty() {
                out.push((proper.clone(), proper));
            } else if let Some((cn, _)) = rest.split_once('<') {
                let _ = cn;
                let commit = rest.trim().to_string();
                // Prefer matching on email in commit side.
                out.push((commit, proper));
            }
        }
    }
    Ok(out)
}

fn map_identity(map: &[(String, String)], id: &str) -> String {
    let id_l = id.to_ascii_lowercase();
    for (from, to) in map {
        if id == from || id_l.contains(&email_of(from).to_ascii_lowercase()) {
            return to.clone();
        }
    }
    id.to_string()
}

fn email_of(id: &str) -> String {
    if let (Some(a), Some(b)) = (id.find('<'), id.find('>')) {
        if b > a {
            return id[a + 1..b].to_string();
        }
    }
    id.to_string()
}

/// `arks column` — simple columnar layout of stdin lines (subset).
pub fn cmd_column(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &["--mode", "--width", "--padding"])?;
    let mut cols = 2usize;
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--mode" {
            i += 1;
            // ignore mode token (plain|dense|…)
        } else if let Some(rest) = args[i].strip_prefix("--mode=") {
            let _ = rest;
        } else if args[i] == "--width" {
            i += 1;
        } else if args[i].starts_with("--width=") {
        } else if args[i] == "--padding" {
            i += 1;
            if let Some(p) = args.get(i) {
                let _ = p;
            }
        } else if !args[i].starts_with('-') {
            if let Ok(n) = args[i].parse::<usize>() {
                cols = n.max(1);
            }
        }
        i += 1;
    }
    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf)?;
    let lines: Vec<&str> = buf.lines().filter(|l| !l.is_empty()).collect();
    if lines.is_empty() {
        return Ok(());
    }
    let width = lines.iter().map(|l| l.len()).max().unwrap_or(0) + 2;
    for chunk in lines.chunks(cols) {
        let mut row = String::new();
        for (i, cell) in chunk.iter().enumerate() {
            if i + 1 == chunk.len() {
                row.push_str(cell);
            } else {
                row.push_str(&format!("{cell:<width$}"));
            }
        }
        println!("{row}");
    }
    Ok(())
}

/// `arks for-each-repo -- <cmd>…` — run arks subcommand in each configured repo (subset).
pub fn cmd_for_each_repo(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &["-c", "--config"])?;
    let dash = args.iter().position(|a| a == "--");
    let (cfg_key, cmd_args) = if let Some(i) = dash {
        let key = args[..i]
            .iter()
            .find(|a| !a.starts_with('-'))
            .map(|s| s.as_str())
            .unwrap_or("arks.repo");
        (key, &args[i + 1..])
    } else {
        return Err(CliError::Usage(
            "for-each-repo [--] <arks-subcommand>… (needs --)".into(),
        ));
    };
    if cmd_args.is_empty() {
        return Err(CliError::Usage("for-each-repo -- <cmd>…".into()));
    }
    // Discover paths from env ARKS_FOR_EACH_REPOS (pathsep-separated) — no git config multi-value depth yet.
    let paths = if let Ok(v) = std::env::var("ARKS_FOR_EACH_REPOS") {
        #[cfg(windows)]
        let sep = ';';
        #[cfg(not(windows))]
        let sep = ':';
        v.split(sep)
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
    } else {
        let _ = cfg_key;
        return Err(CliError::NotImplemented(
            "for-each-repo: set ARKS_FOR_EACH_REPOS to a path list (config multi-value not claimed)"
                .into(),
        ));
    };
    let exe = std::env::current_exe().map_err(|e| CliError::Msg(e.to_string()))?;
    for p in paths {
        println!("Entering '{p}'");
        let status = std::process::Command::new(&exe)
            .args(cmd_args)
            .current_dir(&p)
            .status()
            .map_err(|e| CliError::Msg(e.to_string()))?;
        if !status.success() {
            return Err(CliError::Msg(format!(
                "for-each-repo: {p}: exit {}",
                status.code().unwrap_or(-1)
            )));
        }
    }
    Ok(())
}

/// `arks fmt-merge-msg` — synthesize a merge message from stdin MERGE_HEAD lines (subset).
pub fn cmd_fmt_merge_msg(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &["-m", "--message", "--log"])?;
    let mut message = None;
    let mut i = 0;
    while i < args.len() {
        if args[i] == "-m" || args[i] == "--message" {
            i += 1;
            message = args.get(i).cloned();
        } else if let Some(rest) = args[i].strip_prefix("--message=") {
            message = Some(rest.to_string());
        }
        i += 1;
    }
    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf)?;
    if let Some(m) = message {
        println!("{m}");
        return Ok(());
    }
    let mut branches = Vec::new();
    for line in buf.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        // Accept "branch 'x' of url" or raw ref names.
        if let Some(rest) = t.strip_prefix('\'') {
            if let Some(end) = rest.find('\'') {
                branches.push(rest[..end].to_string());
                continue;
            }
        }
        branches.push(t.to_string());
    }
    if branches.is_empty() {
        println!("Merge");
    } else if branches.len() == 1 {
        println!("Merge branch '{}'", branches[0]);
    } else {
        println!(
            "Merge branches {}",
            branches
                .iter()
                .map(|b| format!("'{b}'"))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    Ok(())
}

/// `arks interpret-trailers` — add/trim trailers on stdin message (subset).
pub fn cmd_interpret_trailers(args: &[String]) -> CliResult<()> {
    if args.iter().any(|a| a == "-h" || a == "--help") {
        print!(
            "\
arks interpret-trailers — add trailers to a message on stdin (subset)

Usage:
  arks interpret-trailers [--trailer <token>=<value>]… [--trim-empty]
"
        );
        return Ok(());
    }
    refuse_unknown_flags(
        args,
        &["--trailer", "--trim-empty", "--if-exists", "--if-missing"],
    )?;
    let mut trailers: Vec<(String, String)> = Vec::new();
    let mut trim_empty = false;
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--trailer" {
            i += 1;
            let spec = args.get(i).ok_or_else(|| {
                CliError::Usage("interpret-trailers --trailer <token>=<value>".into())
            })?;
            let (k, v) = split_trailer(spec)?;
            trailers.push((k, v));
        } else if let Some(rest) = args[i].strip_prefix("--trailer=") {
            let (k, v) = split_trailer(rest)?;
            trailers.push((k, v));
        } else if args[i] == "--trim-empty" {
            trim_empty = true;
        }
        i += 1;
    }
    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf)?;
    let mut body = buf;
    if !body.ends_with('\n') && !body.is_empty() {
        body.push('\n');
    }
    // Ensure blank line before trailer block when body has content.
    if !trailers.is_empty() && !body.trim().is_empty() && !body.ends_with("\n\n") {
        if !body.ends_with('\n') {
            body.push('\n');
        }
        body.push('\n');
    }
    for (k, v) in trailers {
        if trim_empty && v.is_empty() {
            continue;
        }
        body.push_str(&format!("{k}: {v}\n"));
    }
    print!("{body}");
    Ok(())
}

fn split_trailer(spec: &str) -> CliResult<(String, String)> {
    let (k, v) = spec
        .split_once('=')
        .or_else(|| spec.split_once(':'))
        .ok_or_else(|| CliError::Usage("trailer needs key=value or key:value".into()))?;
    Ok((k.trim().to_string(), v.trim().to_string()))
}

/// `arks mailinfo [<msg>] [<patch>]` — extract subject/author + patch from email (subset).
pub fn cmd_mailinfo(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &["-k", "-b", "-u", "-n", "--encoding", "--scissors"])?;
    let positional: Vec<&String> = args.iter().filter(|a| !a.starts_with('-')).collect();
    let msg_out = positional
        .first()
        .ok_or_else(|| CliError::Usage("mailinfo <msg> <patch>".into()))?;
    let patch_out = positional
        .get(1)
        .ok_or_else(|| CliError::Usage("mailinfo <msg> <patch>".into()))?;
    let mut raw = String::new();
    io::stdin().read_to_string(&mut raw)?;
    let (headers, body) = split_mail_headers(&raw);
    let subject = header_value(&headers, "Subject").unwrap_or_default();
    let author = header_value(&headers, "From").unwrap_or_default();
    let date = header_value(&headers, "Date").unwrap_or_default();
    let (msg_body, patch_body) = split_mail_body_and_patch(body);
    let mut msg = String::new();
    if !author.is_empty() {
        // Author Name <email> — best-effort
        if let Some((name, email)) = parse_from(&author) {
            msg.push_str(&format!("Author: {name}\n"));
            msg.push_str(&format!("Email: {email}\n"));
        } else {
            msg.push_str(&format!("Author: {author}\n"));
        }
    }
    if !date.is_empty() {
        msg.push_str(&format!("Date: {date}\n"));
    }
    msg.push_str(&format!("Subject: {subject}\n"));
    msg.push('\n');
    msg.push_str(msg_body.trim());
    msg.push('\n');
    fs::write(msg_out, msg)?;
    fs::write(patch_out, patch_body)?;
    // git mailinfo prints Author/Email/Subject on stdout
    if let Some((name, email)) = parse_from(&author) {
        println!("Author: {name}");
        println!("Email: {email}");
    }
    println!("Subject: {subject}");
    Ok(())
}

fn split_mail_headers(raw: &str) -> (Vec<(String, String)>, &str) {
    let mut headers = Vec::new();
    let mut lines = raw.split_inclusive('\n');
    let mut body_start = 0usize;
    let bytes = raw.as_bytes();
    let mut offset = 0usize;
    while let Some(line) = lines.next() {
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            offset += line.len();
            body_start = offset;
            break;
        }
        if let Some((k, v)) = trimmed.split_once(':') {
            headers.push((k.to_ascii_lowercase(), v.trim().to_string()));
        }
        offset += line.len();
        if offset >= bytes.len() {
            body_start = offset;
            break;
        }
    }
    (headers, &raw[body_start.min(raw.len())..])
}

fn header_value<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
    let want = name.to_ascii_lowercase();
    headers
        .iter()
        .find(|(k, _)| *k == want)
        .map(|(_, v)| v.as_str())
}

fn parse_from(from: &str) -> Option<(String, String)> {
    if let Some(start) = from.find('<') {
        if let Some(end) = from[start..].find('>') {
            let email = from[start + 1..start + end].trim().to_string();
            let name = from[..start].trim().trim_matches('"').to_string();
            return Some((name, email));
        }
    }
    if from.contains('@') {
        return Some((from.to_string(), from.to_string()));
    }
    None
}

fn split_mail_body_and_patch(body: &str) -> (&str, String) {
    // Prefer unified-diff start; else scissors; else whole body as message.
    if let Some(idx) = body.find("\ndiff --git ") {
        return (&body[..idx], body[idx + 1..].to_string());
    }
    if let Some(idx) = body.find("\n--- \n") {
        return (&body[..idx], body[idx + 1..].to_string());
    }
    if let Some(idx) = body.find("\n-- \n") {
        // signature separator — treat rest as non-patch
        return (&body[..idx], String::new());
    }
    (body, String::new())
}

/// `arks mailsplit [-o <dir>] [-b] [<mbox>]` — split mbox into numbered files (subset).
pub fn cmd_mailsplit(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &["-o", "-b", "-d", "--keep-cr"])?;
    let mut out_dir = Path::new(".").to_path_buf();
    let mut mbox: Option<PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        if args[i] == "-o" {
            i += 1;
            out_dir = PathBuf::from(
                args.get(i)
                    .ok_or_else(|| CliError::Usage("mailsplit -o <dir>".into()))?,
            );
        } else if let Some(rest) = args[i].strip_prefix("-o") {
            if !rest.is_empty() {
                out_dir = PathBuf::from(rest);
            }
        } else if args[i] == "-b" || args[i] == "--keep-cr" {
        } else if args[i] == "-d" {
            i += 1; // digit width ignored in subset
        } else if !args[i].starts_with('-') {
            mbox = Some(PathBuf::from(&args[i]));
        }
        i += 1;
    }
    let text = if let Some(p) = mbox {
        fs::read_to_string(p)?
    } else {
        let mut s = String::new();
        io::stdin().read_to_string(&mut s)?;
        s
    };
    fs::create_dir_all(&out_dir)?;
    let mut n = 0usize;
    let mut cur = String::new();
    for line in text.lines() {
        if line.starts_with("From ") && !cur.is_empty() {
            n += 1;
            let path = out_dir.join(format!("{n:04}"));
            fs::write(&path, &cur)?;
            cur.clear();
        }
        cur.push_str(line);
        cur.push('\n');
    }
    if !cur.trim().is_empty() {
        n += 1;
        let path = out_dir.join(format!("{n:04}"));
        fs::write(&path, &cur)?;
    }
    println!("{n}");
    Ok(())
}

/// `arks get-tar-commit-id` — read commit oid from `arks archive`/`git archive` tar (subset).
pub fn cmd_get_tar_commit_id(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &[])?;
    let mut buf = Vec::new();
    io::stdin().read_to_end(&mut buf)?;
    // git archive --prefix embeds pax/global header with `comment=<oid>` or `42 comment=<oid>`.
    let text = String::from_utf8_lossy(&buf);
    for token in ["comment=", "42 comment="].iter() {
        if let Some(idx) = text.find(token) {
            let rest = &text[idx + token.len()..];
            let oid: String = rest.chars().take_while(|c| c.is_ascii_hexdigit()).collect();
            if oid.len() == 40 || oid.len() == 64 {
                println!("{oid}");
                return Ok(());
            }
        }
    }
    Err(CliError::Msg(
        "get-tar-commit-id: no commit comment found in tar stream".into(),
    ))
}

/// `arks unpack-file <blob>` — write blob to temp file; print path (subset).
pub fn cmd_unpack_file(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &[])?;
    let rev = args
        .iter()
        .find(|a| !a.starts_with('-'))
        .ok_or_else(|| CliError::Usage("unpack-file <blob>".into()))?;
    let repo = open_repo()?;
    let oid = resolve_rev(&repo, rev)?;
    let obj = repo.get_object(&oid)?;
    if obj.kind != ObjectKind::Blob {
        return Err(CliError::Msg(format!("unpack-file: {rev} is not a blob")));
    }
    let path = std::env::temp_dir().join(format!(".merge_file_{}", &oid.to_hex()[..8]));
    fs::write(&path, &obj.payload)?;
    println!("{}", path.display());
    Ok(())
}

/// `arks request-pull <start> <url> [<end>]` — summarize commits for a pull request (subset).
pub fn cmd_request_pull(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &[])?;
    let positional: Vec<&String> = args.iter().filter(|a| !a.starts_with('-')).collect();
    if positional.len() < 2 {
        return Err(CliError::Usage("request-pull <start> <url> [<end>]".into()));
    }
    let start = positional[0].as_str();
    let url = positional[1].as_str();
    let end = positional.get(2).map(|s| s.as_str()).unwrap_or("HEAD");
    let repo = open_repo()?;
    let start_oid = resolve_rev(&repo, start)?;
    let end_oid = resolve_rev(&repo, end)?;
    println!(
        "The following changes since commit {}:\n",
        start_oid.to_hex()
    );
    println!("  {url}\n");
    println!("are available in the git repository at:\n");
    println!("  {url}\n");
    println!("for you to fetch changes up to {}:\n", end_oid.to_hex());
    // One-line summaries between start..end (exclusive start) — walk parents of end until start.
    let mut cur = end_oid;
    let mut lines = Vec::new();
    for _ in 0..128 {
        if cur == start_oid {
            break;
        }
        let obj = repo.get_object(&cur)?;
        if obj.kind != ObjectKind::Commit {
            break;
        }
        let text = String::from_utf8_lossy(&obj.payload);
        let subject = text
            .lines()
            .skip_while(|l| !l.is_empty())
            .nth(1)
            .or_else(|| {
                text.lines().find(|l| {
                    !l.starts_with("tree ")
                        && !l.starts_with("parent ")
                        && !l.starts_with("author ")
                        && !l.starts_with("committer ")
                        && !l.is_empty()
                })
            })
            .unwrap_or("")
            .to_string();
        lines.push(format!("  {} {}", &cur.to_hex()[..7], subject));
        let parent = text
            .lines()
            .find_map(|l| l.strip_prefix("parent "))
            .and_then(|h| Oid::from_hex(h.trim()).ok());
        match parent {
            Some(p) => cur = p,
            None => break,
        }
    }
    lines.reverse();
    for l in lines {
        println!("{l}");
    }
    Ok(())
}

/// `arks bugreport` — print environment / version diagnostics (subset).
pub fn cmd_bugreport(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &["--output-directory", "-s", "--suffix"])?;
    println!("arks version {}", env!("CARGO_PKG_VERSION"));
    println!("os={}", std::env::consts::OS);
    println!("arch={}", std::env::consts::ARCH);
    if let Ok(cwd) = std::env::current_dir() {
        println!("cwd={}", cwd.display());
    }
    if let Ok(exe) = std::env::current_exe() {
        println!("exe={}", exe.display());
    }
    match open_repo() {
        Ok(repo) => {
            println!("git_dir={}", repo.git_dir().display());
            if let Ok(oid) = repo.head_oid() {
                println!("HEAD={}", oid.to_hex());
            }
        }
        Err(_) => println!("git_dir=(not a repository)"),
    }
    println!("note: bugreport subset — no zip bundle; no git/gh spawn");
    Ok(())
}

/// `arks diagnose` — collect diagnostic paths for a repo (subset).
pub fn cmd_diagnose(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &["--mode", "--output-directory", "--suffix"])?;
    let repo = open_repo()?;
    let git = repo.git_dir();
    println!("arks diagnose (subset)");
    println!("git_dir={}", git.display());
    for rel in [
        "HEAD",
        "config",
        "index",
        "packed-refs",
        "objects/info/packs",
        "info/refs",
    ] {
        let p = git.join(rel);
        let st = if p.is_file() {
            "file"
        } else if p.is_dir() {
            "dir"
        } else {
            "missing"
        };
        println!("{st}\t{}", p.display());
    }
    let packs = git.join("objects/pack");
    if packs.is_dir() {
        for ent in fs::read_dir(&packs)? {
            let ent = ent?;
            let name = ent.file_name().to_string_lossy().into_owned();
            if name.ends_with(".pack") || name.ends_with(".idx") {
                println!("pack\t{}", ent.path().display());
            }
        }
    }
    println!("note: diagnose subset — listing only; no zip; no git/gh spawn");
    Ok(())
}

/// `arks pack-redundant` — list `.pack` files whose objects are covered by others (subset).
pub fn cmd_pack_redundant(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &["--all"])?;
    let all = args.iter().any(|a| a == "--all");
    let repo = open_repo()?;
    let pack_dir = repo.git_dir().join("objects/pack");
    if !pack_dir.is_dir() {
        return Ok(());
    }
    let mut packs: Vec<PathBuf> = Vec::new();
    for ent in fs::read_dir(&pack_dir)? {
        let ent = ent?;
        let path = ent.path();
        if path.extension().and_then(|s| s.to_str()) == Some("pack") {
            packs.push(path);
        }
    }
    packs.sort();
    if packs.len() <= 1 {
        return Ok(());
    }
    // Subset honesty: without full OID set algebra, only report candidates when --all
    // and more than one pack exists (operator must verify). Never invent "redundant".
    if all {
        for p in &packs {
            println!("{}", p.display());
        }
        eprintln!(
            "note: pack-redundant subset — listed all packs; full redundancy algebra not claimed"
        );
    } else {
        eprintln!("pack-redundant: pass --all to list packs (full redundancy scan not claimed)");
    }
    Ok(())
}

/// `arks shell` — restricted command shell (git-shell peer subset).
pub fn cmd_shell(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &["-c"])?;
    let mut cmd_line: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        if args[i] == "-c" {
            i += 1;
            cmd_line = Some(
                args.get(i)
                    .ok_or_else(|| CliError::Usage("shell -c <command>".into()))?
                    .clone(),
            );
        } else if !args[i].starts_with('-') && cmd_line.is_none() {
            cmd_line = Some(args[i].clone());
        }
        i += 1;
    }
    let Some(line) = cmd_line else {
        return Err(CliError::Usage(
            "shell -c 'git-upload-pack|git-receive-pack|git-upload-archive <dir>'".into(),
        ));
    };
    let line = line.trim();
    let (prog, rest) = line
        .split_once(char::is_whitespace)
        .map(|(a, b)| (a, b.trim()))
        .unwrap_or((line, ""));
    let prog = prog.trim_matches('\'').trim_matches('"');
    // git-shell allows only these; map to arks in-process dispatch (no git spawn).
    let arks_cmd = match prog {
        "git-upload-pack" | "upload-pack" => "upload-pack",
        "git-receive-pack" | "receive-pack" => "receive-pack",
        "git-upload-archive" | "upload-archive" => "upload-archive",
        other => {
            return Err(CliError::Msg(format!(
                "shell: unrecognized command '{other}' (allowed: git-upload-pack|git-receive-pack|git-upload-archive)"
            )));
        }
    };
    let dir = rest.trim().trim_matches('\'').trim_matches('"');
    if dir.is_empty() {
        return Err(CliError::Usage(format!("shell -c '{prog} <directory>'")));
    }
    match arks_cmd {
        "upload-pack" => crate::cmd::cmd_upload_pack(&[dir.to_string()]),
        "receive-pack" => crate::cmd::cmd_receive_pack(&[dir.to_string()]),
        "upload-archive" => cmd_upload_archive(&[dir.to_string()]),
        _ => unreachable!(),
    }
}

/// `arks upload-archive <directory>` — serve `git archive` over the upload-archive protocol (subset).
pub fn cmd_upload_archive(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &[])?;
    let dir = args
        .iter()
        .find(|a| !a.starts_with('-'))
        .ok_or_else(|| CliError::Usage("upload-archive <directory>".into()))?;
    // Protocol subset: without full pkt-line argument parse, materialize HEAD tar to stdout
    // when invoked as a local helper. SSH/git-daemon protocol depth remains Partial.
    let prev = std::env::current_dir()?;
    std::env::set_current_dir(dir)?;
    let result = crate::cmd::cmd_archive(&["--format=tar".into(), "HEAD".into()]);
    let _ = std::env::set_current_dir(prev);
    result.map_err(|e| CliError::Msg(format!("upload-archive: {e}")))
}
