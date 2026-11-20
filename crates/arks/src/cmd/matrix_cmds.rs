//! High-traffic Missing pin commands (matrix fill toward [[sot:root/cli-git-help-matrix]]).
//!
//! Cite: pin git 2.39.5 man pages; survey [[research:root/20261111034805]].

use crate::cmd::shared::resolve_rev;
use crate::error::{CliError, CliResult};
use crate::util::{open_repo, refuse_unknown_flags, work_tree};
use arks_object::{ObjectKind, RawObject, Tree, TreeEntry, TreeMode};
use arks_oid::Oid;
use arks_repo::Repository;
use arks_store::ObjectStore;
use std::collections::HashSet;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

/// `arks prune [-n] [-v] [--progress] [--expire <time>] [--exclude-promisor-objects] [--] [<head>...]`
/// — delete loose objects unreachable from refs, reflogs, the index and the
/// given heads. Cite: https://git-scm.com/docs/git-prune
pub fn cmd_prune(args: &[String]) -> CliResult<()> {
    let mut dry = false;
    let mut verbose = false;
    let mut expire: Option<i64> = None;
    let mut heads: Vec<String> = Vec::new();
    let mut i = 0;
    let mut dashdash = false;
    while i < args.len() {
        let a = args[i].as_str();
        i += 1;
        if dashdash {
            heads.push(a.to_string());
            continue;
        }
        match a {
            "-n" | "--dry-run" => dry = true,
            "--no-dry-run" => dry = false,
            "-v" | "--verbose" => verbose = true,
            "--no-verbose" => verbose = false,
            "-nv" | "-vn" => {
                dry = true;
                verbose = true;
            }
            "--progress"
            | "--no-progress"
            | "--exclude-promisor-objects"
            | "--no-exclude-promisor-objects" => {}
            // `--no-expire` keeps everything, like `--expire=never`.
            "--no-expire" => expire = Some(0),
            "--expire" => {
                let v = args.get(i).ok_or_else(|| {
                    CliError::Exit(129, "error: option `expire' requires a value".into())
                })?;
                i += 1;
                expire = Some(parse_expire(v)?);
            }
            "--" => dashdash = true,
            _ if a.starts_with("--expire=") => expire = Some(parse_expire(&a[9..])?),
            _ if a.starts_with('-') => {
                return Err(CliError::Exit(
                    129,
                    format!(
                        "error: unknown option `{}'\nusage: git prune [-n] [-v] [--progress] [--expire <time>] [--] [<head>...]",
                        a.trim_start_matches('-')
                    ),
                ));
            }
            _ => heads.push(a.to_string()),
        }
    }
    let repo = open_repo()?;
    let mut tips = Vec::new();
    for h in &heads {
        tips.push(
            resolve_rev(&repo, h)
                .map_err(|_| CliError::fatal(format!("unrecognized argument: {h}")))?,
        );
    }
    let reachable = crate::cmd::plumbing_ext::reachable_all(&repo, &tips)?;
    let mut out = String::new();
    for (oid, path) in crate::cmd::plumbing_ext::loose_objects(&repo) {
        if reachable.contains(&oid) {
            continue;
        }
        if let Some(cut) = expire {
            let mtime = fs::metadata(&path)
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            if mtime > cut {
                continue;
            }
        }
        if dry || verbose {
            let kind = repo
                .get_object(&oid)
                .map(|o| o.kind.as_str().to_string())
                .unwrap_or_else(|_| "unknown".into());
            out.push_str(&format!("{} {kind}\n", oid.to_hex()));
        }
        if !dry {
            let _ = fs::remove_file(&path);
            if let Some(dir) = path.parent() {
                let _ = fs::remove_dir(dir);
            }
        }
    }
    // `prune` also runs prune-packed (dry-run lists `rm -f <path>`).
    let packed = packed_oids(&repo)?;
    for (oid, path) in crate::cmd::plumbing_ext::loose_objects(&repo) {
        if packed.contains(&oid) {
            if dry {
                out.push_str(&format!("rm -f {}\n", shown_path(&path)));
            } else {
                let _ = fs::remove_file(&path);
            }
        }
    }
    print!("{out}");
    Ok(())
}

/// `arks prune-packed [-n | --dry-run] [-q | --quiet]` — remove loose objects
/// that are also in a pack. Cite: https://git-scm.com/docs/git-prune-packed
pub fn cmd_prune_packed(args: &[String]) -> CliResult<()> {
    let mut dry = false;
    let mut quiet = false;
    for a in args {
        match a.as_str() {
            "-n" | "--dry-run" => dry = true,
            "--no-dry-run" => dry = false,
            "-q" | "--quiet" => quiet = true,
            "--no-quiet" => quiet = false,
            "-nq" | "-qn" => {
                dry = true;
                quiet = true;
            }
            _ => {
                return Err(CliError::Exit(
                    129,
                    "usage: git prune-packed [-n | --dry-run] [-q | --quiet]".into(),
                ));
            }
        }
    }
    let repo = open_repo()?;
    let pack_oids = packed_oids(&repo)?;
    let mut out = String::new();
    for (oid, path) in crate::cmd::plumbing_ext::loose_objects(&repo) {
        if !pack_oids.contains(&oid) {
            continue;
        }
        if dry {
            out.push_str(&format!("rm -f {}\n", shown_path(&path)));
        } else {
            let _ = fs::remove_file(&path);
            if let Some(dir) = path.parent() {
                let _ = fs::remove_dir(dir);
            }
        }
    }
    let _ = quiet;
    print!("{out}");
    Ok(())
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
    // git-cherry: `[-v] [<upstream> [<head> [<limit>]]]`.
    // Cite: https://git-scm.com/docs/git-cherry
    let mut verbose = false;
    let mut abbrev: Option<usize> = None;
    let mut positional: Vec<&String> = Vec::new();
    for a in args {
        match a.as_str() {
            "-v" | "--verbose" => verbose = true,
            "--no-verbose" => verbose = false,
            "--abbrev" => abbrev = Some(7),
            "--no-abbrev" => abbrev = None,
            x if x.starts_with("--abbrev=") => {
                let n: usize = x[9..].parse().map_err(|_| {
                    CliError::Exit(
                        129,
                        "error: option `abbrev' expects a numerical value".into(),
                    )
                })?;
                abbrev = Some(if n == 0 { 40 } else { n.clamp(4, 40) });
            }
            x if x.starts_with('-') && x.len() > 1 => {
                return Err(CliError::Exit(
                    129,
                    format!("error: unknown option `{}'", x.trim_start_matches('-')),
                ));
            }
            _ => positional.push(a),
        }
    }
    let repo = open_repo()?;
    let hex = |o: &Oid| match abbrev {
        Some(n) if n < 40 => crate::cmd::shared::abbrev_hex(&repo, o, n),
        _ => o.to_hex(),
    };
    let head = match positional.get(1) {
        Some(h) => resolve_rev(&repo, h)?,
        None => repo.head_oid()?,
    };
    let upstream = match positional.first() {
        Some(u) => resolve_rev(&repo, u)?,
        None => default_upstream(&repo)?,
    };
    // Commits in <upstream>..<head> (no merges, oldest first); `-` when an
    // equivalent change is already in <head>..<upstream>.
    let walk = |inc: Oid, exc: Oid| -> CliResult<Vec<Oid>> {
        Ok(crate::cmd::log_format::walk(&repo, &[inc], &[exc], false)?
            .into_iter()
            .filter(|(_, c)| c.parents.len() < 2)
            .map(|(o, _)| o)
            .collect())
    };
    let mut listed = walk(head, upstream)?;
    listed.reverse();
    let mut upstream_ids = HashSet::new();
    for o in walk(upstream, head)? {
        upstream_ids.insert(patch_identity(&repo, &o)?);
    }
    let limit = positional
        .get(2)
        .map(|l| resolve_rev(&repo, l))
        .transpose()?
        .map(|l| crate::cmd::shared::is_ancestor_set(&repo, l))
        .transpose()?;
    for oid in listed {
        if limit.as_ref().is_some_and(|set| set.contains(&oid)) {
            continue;
        }
        let mark = if upstream_ids.contains(&patch_identity(&repo, &oid)?) {
            '-'
        } else {
            '+'
        };
        if verbose {
            let c = arks_object::Commit::decode(&repo.get_object(&oid)?.payload)?;
            println!("{mark} {} {}", hex(&oid), c.subject());
        } else {
            println!("{mark} {}", hex(&oid));
        }
    }
    Ok(())
}

/// Change identity of a non-merge commit for `cherry`: its patch text with
/// `index` lines and hunk positions dropped (the `git patch-id --stable`
/// idea: equal changes on different bases compare equal).
fn patch_identity(repo: &Repository, oid: &Oid) -> CliResult<String> {
    let c = crate::cmd::log_format::read_commit(repo, oid)?;
    let old = match c.parents.first() {
        Some(p) => crate::cmd::shared::tree_blob_map(repo, p)?,
        None => Default::default(),
    };
    let new = crate::cmd::shared::tree_blob_map(repo, oid)?;
    let changes = crate::cmd::diff_render::changes_between(&old, &new, None);
    let fmt = crate::cmd::diff_render::DiffFormat {
        patch: true,
        ..Default::default()
    };
    let mut buf = Vec::new();
    crate::cmd::diff_render::render(&mut buf, repo, &fmt, &changes)?;
    Ok(String::from_utf8_lossy(&buf)
        .lines()
        .filter(|l| !l.starts_with("index "))
        .map(|l| if l.starts_with("@@") { "@@" } else { l })
        .collect::<Vec<_>>()
        .join("\n"))
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

/// `arks check-ref-format [--normalize] [--[no-]allow-onelevel] [--refspec-pattern] <refname>`
/// and `--branch <name>`. Exit 1 (silent) for an invalid name.
/// Cite: https://git-scm.com/docs/git-check-ref-format
pub fn cmd_check_ref_format(args: &[String]) -> CliResult<()> {
    let usage = || {
        CliError::Exit(
            129,
            "usage: git check-ref-format [--normalize] [<options>] <refname>\n   or: git check-ref-format --branch <branchname-shorthand>".into(),
        )
    };
    let mut normalize = false;
    let mut allow_one = false;
    let mut pattern = false;
    let mut branch = false;
    let mut rest: Vec<&String> = Vec::new();
    for a in args {
        match a.as_str() {
            "--normalize" | "--print" => normalize = true,
            "--allow-onelevel" => allow_one = true,
            "--no-allow-onelevel" => allow_one = false,
            "--refspec-pattern" => pattern = true,
            "--branch" => branch = true,
            _ if a.starts_with("--") => return Err(usage()),
            _ => rest.push(a),
        }
    }
    if branch {
        let [name] = rest.as_slice() else {
            return Err(usage());
        };
        let repo = open_repo().ok();
        let resolved = match repo.as_ref() {
            Some(r) => previous_branch(r, name).unwrap_or_else(|| (*name).clone()),
            None => (*name).clone(),
        };
        let full = format!("refs/heads/{resolved}");
        if resolved.starts_with('-') || !refname_ok(&full, false, false) {
            return Err(CliError::fatal(format!(
                "'{name}' is not a valid branch name"
            )));
        }
        println!("{resolved}");
        return Ok(());
    }
    let [name] = rest.as_slice() else {
        return Err(usage());
    };
    let mut name = (*name).clone();
    if normalize {
        let lead = name.starts_with('/');
        let trail = name.ends_with('/') && name.len() > 1;
        let collapsed: Vec<String> = name
            .split('/')
            .filter(|p| !p.is_empty())
            .map(str::to_string)
            .collect();
        if trail || (lead && collapsed.is_empty()) {
            return Err(CliError::silent(1));
        }
        name = collapsed.join("/");
    }
    if !refname_ok(&name, allow_one, pattern) {
        return Err(CliError::silent(1));
    }
    if normalize {
        println!("{name}");
    }
    Ok(())
}

fn refname_ok(name: &str, allow_onelevel: bool, pattern: bool) -> bool {
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
    let stars = name.matches('*').count();
    if stars > usize::from(pattern) {
        return false;
    }
    for ch in name.chars() {
        if ch.is_ascii_control() || "^:~?[".contains(ch) {
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

/// `arks stripspace [-s | --strip-comments | -c | --comment-lines]` (stdin→stdout):
/// trailing whitespace removed, blank-line runs collapsed, leading/trailing
/// blank lines dropped. Cite: https://git-scm.com/docs/git-stripspace
pub fn cmd_stripspace(args: &[String]) -> CliResult<()> {
    let mut strip_comments = false;
    let mut comment_lines = false;
    for a in args {
        match a.as_str() {
            "-s" | "--strip-comments" => strip_comments = true,
            "-c" | "--comment-lines" => comment_lines = true,
            "-sc" | "-cs" => {
                strip_comments = true;
                comment_lines = true;
            }
            _ => {
                return Err(CliError::Exit(
                    129,
                    "usage: git stripspace [-s | --strip-comments]\n   or: git stripspace [-c | --comment-lines]".into(),
                ));
            }
        }
    }
    if strip_comments && comment_lines {
        return Err(CliError::Exit(
            129,
            "error: --comment-lines is incompatible with --strip-comments".into(),
        ));
    }
    let cfg = arks_config::ConfigSet::load_discovered(&crate::util::cwd()?);
    let cc = cfg
        .get("core.commentchar")
        .filter(|c| !c.is_empty() && c != "auto")
        .unwrap_or_else(|| "#".into());
    let mut input = Vec::new();
    io::stdin().read_to_end(&mut input)?;
    let input = String::from_utf8_lossy(&input).into_owned();
    let mut out = String::new();
    if comment_lines {
        let body = input.strip_suffix('\n').unwrap_or(&input);
        if !input.is_empty() {
            for line in body.split('\n') {
                if line.is_empty() {
                    out.push_str(&cc);
                } else {
                    out.push_str(&format!("{cc} {line}"));
                }
                out.push('\n');
            }
        }
        print!("{out}");
        return Ok(());
    }
    let mut blank_run = false;
    for line in input.lines() {
        if strip_comments && line.starts_with(cc.as_str()) {
            continue;
        }
        let l = line.trim_end();
        if l.is_empty() {
            blank_run = !out.is_empty();
            continue;
        }
        if blank_run {
            out.push('\n');
            blank_run = false;
        }
        out.push_str(l);
        out.push('\n');
    }
    print!("{out}");
    Ok(())
}

/// `arks mktag [--[no-]strict]` — validate a tag object on stdin and write it.
/// Cite: https://git-scm.com/docs/git-mktag
pub fn cmd_mktag(args: &[String]) -> CliResult<()> {
    let mut strict = true;
    for a in args {
        match a.as_str() {
            "--strict" => strict = true,
            "--no-strict" => strict = false,
            _ if a.starts_with('-') => return Err(CliError::Exit(129, "usage: git mktag".into())),
            _ => {}
        }
    }
    let mut buf = Vec::new();
    io::stdin().read_to_end(&mut buf)?;
    let bad = || {
        CliError::Exit(
            128,
            "error: tag input does not pass fsck\nfatal: tag on stdin did not pass our strict fsck check".into(),
        )
    };
    let text = std::str::from_utf8(&buf).map_err(|_| bad())?;
    let mut lines = text.split('\n');
    let object = lines
        .next()
        .and_then(|l| l.strip_prefix("object "))
        .and_then(|h| Oid::from_hex(h).ok())
        .ok_or_else(bad)?;
    let ty = lines
        .next()
        .and_then(|l| l.strip_prefix("type "))
        .ok_or_else(bad)?
        .to_string();
    let tag = lines
        .next()
        .and_then(|l| l.strip_prefix("tag "))
        .ok_or_else(bad)?;
    if tag.is_empty() || (strict && !refname_ok(&format!("refs/tags/{tag}"), false, false)) {
        return Err(CliError::Exit(
            128,
            format!(
                "error: tag input does not pass fsck: badTagName: invalid 'tag' name: {tag}\nfatal: tag on stdin did not pass our strict fsck check"
            ),
        ));
    }
    let tagger = lines.next().unwrap_or("");
    if strict && !tagger.starts_with("tagger ") {
        return Err(bad());
    }
    let mut repo = open_repo()?;
    let target = repo.get_object(&object).map_err(|_| {
        CliError::fatal(format!(
            "could not read tagged object '{}'",
            object.to_hex()
        ))
    })?;
    if target.kind.as_str() != ty {
        return Err(CliError::fatal(format!(
            "object '{}' tagged as '{ty}', but is a '{}' type",
            object.to_hex(),
            target.kind.as_str()
        )));
    }
    let format = repo.object_format();
    let obj = RawObject::new(ObjectKind::Tag, buf);
    let oid = repo.store_mut().put(format, &obj)?;
    println!("{}", oid.to_hex());
    Ok(())
}

/// `arks mktree [-z] [--missing] [--batch]` — build trees from ls-tree lines.
/// Cite: https://git-scm.com/docs/git-mktree
pub fn cmd_mktree(args: &[String]) -> CliResult<()> {
    let (mut nul, mut missing, mut batch) = (false, false, false);
    for a in args {
        match a.as_str() {
            "-z" => nul = true,
            "--missing" => missing = true,
            "--no-missing" => missing = false,
            "--batch" => batch = true,
            "--no-batch" => batch = false,
            // Positional arguments are accepted and ignored, as in stock git.
            _ if a == "-" || !a.starts_with('-') || a == "--" => {}
            _ => {
                return Err(CliError::Exit(
                    129,
                    "usage: git mktree [-z] [--missing] [--batch]".into(),
                ));
            }
        }
    }
    let mut buf = Vec::new();
    io::stdin().read_to_end(&mut buf)?;
    let sep = if nul { b'\0' } else { b'\n' };
    let mut repo = open_repo()?;
    let format = repo.object_format();
    let mut entries: Vec<TreeEntry> = Vec::new();
    let mut seen_any = false;
    let mut out = String::new();
    let flush =
        |entries: &mut Vec<TreeEntry>, repo: &mut Repository, out: &mut String| -> CliResult<()> {
            let tree =
                Tree::new(std::mem::take(entries)).map_err(|e| CliError::fatal(e.to_string()))?;
            let payload = tree.encode().map_err(|e| CliError::Msg(e.to_string()))?;
            let obj = RawObject::new(ObjectKind::Tree, payload);
            let oid = repo.store_mut().put(format, &obj)?;
            out.push_str(&oid.to_hex());
            out.push('\n');
            Ok(())
        };
    let recs: Vec<&[u8]> = buf.split(|b| *b == sep).collect();
    let n = recs.len();
    for (i, rec) in recs.into_iter().enumerate() {
        let line = String::from_utf8_lossy(rec);
        if line.is_empty() {
            if i + 1 == n {
                break;
            }
            if batch {
                flush(&mut entries, &mut repo, &mut out)?;
                seen_any = false;
                continue;
            }
            return Err(CliError::fatal(
                "input format error: (blank line only valid in batch mode)",
            ));
        }
        seen_any = true;
        let (meta, name) = line
            .split_once('\t')
            .ok_or_else(|| CliError::fatal(format!("input format error: {line}")))?;
        let parts: Vec<&str> = meta.split(' ').collect();
        if parts.len() != 3 {
            return Err(CliError::fatal(format!("input format error: {line}")));
        }
        let mode = TreeMode::parse(parts[0])
            .map_err(|_| CliError::fatal(format!("input format error: {line}")))?;
        let oid = Oid::from_hex(parts[2])
            .map_err(|_| CliError::fatal(format!("input format error: {line}")))?;
        let want = if mode.is_tree() {
            "tree"
        } else if parts[0] == "160000" {
            "commit"
        } else {
            "blob"
        };
        if parts[1] != want {
            return Err(CliError::fatal(format!(
                "entry '{name}' object type ({}) doesn't match mode type ({want})",
                parts[1]
            )));
        }
        if !missing && want != "commit" {
            match repo.get_object(&oid) {
                Ok(o) if o.kind.as_str() == want => {}
                Ok(o) => {
                    return Err(CliError::fatal(format!(
                        "entry '{name}' object {} is a {} but specified type was ({want})",
                        oid.to_hex(),
                        o.kind.as_str()
                    )));
                }
                Err(_) => {
                    return Err(CliError::fatal(format!(
                        "entry '{name}' object {} is unavailable",
                        oid.to_hex()
                    )));
                }
            }
        }
        entries.push(TreeEntry {
            mode,
            name: name.to_string(),
            oid,
        });
    }
    if seen_any || !batch || (out.is_empty() && !buf.is_empty()) {
        flush(&mut entries, &mut repo, &mut out)?;
    }
    print!("{out}");
    Ok(())
}

/// `arks var (-l | <variable>)` — logical variables of git-var (2.43 set).
/// Cite: https://git-scm.com/docs/git-var
pub fn cmd_var(args: &[String]) -> CliResult<()> {
    let usage = || CliError::Exit(129, "usage: git var (-l | <variable>)".into());
    let [arg] = args else {
        return Err(usage());
    };
    let cwd = crate::util::cwd()?;
    let cfg = arks_config::ConfigSet::load_discovered(&cwd);
    let env = |k: &str| std::env::var(k).ok().filter(|v| !v.is_empty());
    let home = env("HOME");
    let xdg = env("XDG_CONFIG_HOME").or_else(|| home.as_ref().map(|h| format!("{h}/.config")));
    let ident = |author: bool| -> Option<String> {
        let s = if author {
            crate::cmd::shared::commit_signature().ok()?
        } else {
            crate::cmd::shared::committer_signature().ok()?
        };
        Some(format!(
            "{} <{}> {} {}",
            s.name, s.email, s.timestamp, s.timezone
        ))
    };
    let editor = || {
        env("GIT_EDITOR")
            .or_else(|| cfg.get("core.editor"))
            .or_else(|| env("VISUAL"))
            .or_else(|| env("EDITOR"))
            .unwrap_or_else(|| "vi".into())
    };
    let nosystem = |k: &str| env(k).is_some_and(|v| arks_config::parse_bool(&v).unwrap_or(true));
    let vars: Vec<(&str, Vec<String>)> = vec![
        ("GIT_COMMITTER_IDENT", ident(false).into_iter().collect()),
        ("GIT_AUTHOR_IDENT", ident(true).into_iter().collect()),
        ("GIT_EDITOR", vec![editor()]),
        (
            "GIT_SEQUENCE_EDITOR",
            vec![
                env("GIT_SEQUENCE_EDITOR")
                    .or_else(|| cfg.get("sequence.editor"))
                    .unwrap_or_else(editor),
            ],
        ),
        (
            "GIT_PAGER",
            vec![
                env("GIT_PAGER")
                    .or_else(|| cfg.get("core.pager"))
                    .or_else(|| env("PAGER"))
                    .unwrap_or_else(|| "less".into()),
            ],
        ),
        (
            "GIT_DEFAULT_BRANCH",
            vec![
                cfg.get("init.defaultbranch")
                    .unwrap_or_else(|| "master".into()),
            ],
        ),
        ("GIT_SHELL_PATH", vec!["/bin/sh".into()]),
        (
            "GIT_ATTR_SYSTEM",
            if nosystem("GIT_ATTR_NOSYSTEM") {
                Vec::new()
            } else {
                vec!["/etc/gitattributes".into()]
            },
        ),
        (
            "GIT_ATTR_GLOBAL",
            cfg.get("core.attributesfile")
                .or_else(|| xdg.as_ref().map(|x| format!("{x}/git/attributes")))
                .into_iter()
                .collect(),
        ),
        (
            "GIT_CONFIG_SYSTEM",
            if nosystem("GIT_CONFIG_NOSYSTEM") {
                Vec::new()
            } else {
                vec![env("GIT_CONFIG_SYSTEM").unwrap_or_else(|| "/etc/gitconfig".into())]
            },
        ),
        (
            "GIT_CONFIG_GLOBAL",
            match env("GIT_CONFIG_GLOBAL") {
                Some(g) => vec![g],
                None => xdg
                    .iter()
                    .map(|x| format!("{x}/git/config"))
                    .chain(home.iter().map(|h| format!("{h}/.gitconfig")))
                    .collect(),
            },
        ),
    ];
    if arg == "-l" {
        let mut out = String::new();
        for e in cfg.entries() {
            match &e.value {
                Some(v) => out.push_str(&format!("{}={v}\n", e.key)),
                None => out.push_str(&format!("{}\n", e.key)),
            }
        }
        for (k, vs) in &vars {
            for v in vs {
                out.push_str(&format!("{k}={v}\n"));
            }
        }
        print!("{out}");
        return Ok(());
    }
    match vars.iter().find(|(k, _)| *k == arg.as_str()) {
        Some((_, vs)) if vs.is_empty() => Err(CliError::silent(1)),
        Some((_, vs)) => {
            for v in vs {
                println!("{v}");
            }
            Ok(())
        }
        None => Err(usage()),
    }
}

/// `arks credential-store [--file=<path>] (get|store|erase)` — the stock
/// `~/.git-credentials` file format (`<proto>://<user>:<pass>@<host>[/path]`),
/// natively. Cite: https://git-scm.com/docs/git-credential-store
pub fn cmd_credential_store(args: &[String]) -> CliResult<()> {
    let mut file: Option<String> = None;
    let mut op: Option<&str> = None;
    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        i += 1;
        if let Some(v) = a.strip_prefix("--file=") {
            file = Some(v.to_string());
        } else if a == "--file" {
            file = args.get(i).cloned();
            i += 1;
        } else if a == "--no-file" {
            file = None;
        } else if a.starts_with('-') {
            return Err(CliError::Exit(
                129,
                format!(
                    "error: unknown option `{}'\nusage: git credential-store [<options>] <action>",
                    a.trim_start_matches('-')
                ),
            ));
        } else if op.is_none() {
            op = Some(a);
        }
    }
    let op = op.ok_or_else(|| {
        CliError::Exit(
            129,
            "usage: git credential-store [<options>] <action>".into(),
        )
    })?;
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;
    let desc = arks_remote::Credential::parse_attributes(&input);
    let home = std::env::var("HOME").unwrap_or_default();
    let paths: Vec<PathBuf> = match &file {
        Some(f) => vec![PathBuf::from(crate::cmd::plumbing_ext::expand_home(f))],
        None => {
            let xdg = std::env::var("XDG_CONFIG_HOME")
                .ok()
                .filter(|x| !x.is_empty())
                .unwrap_or_else(|| format!("{home}/.config"));
            vec![
                PathBuf::from(format!("{home}/.git-credentials")),
                PathBuf::from(format!("{xdg}/git/credentials")),
            ]
        }
    };
    crate::cmd::plumbing_ext::credential_store(op, &desc, &paths)
}

/// `arks show-index [--object-format=<hash>]` — dump a pack `.idx` from stdin
/// as `<offset> <oid> (<crc32>)`. Cite: https://git-scm.com/docs/git-show-index
pub fn cmd_show_index(args: &[String]) -> CliResult<()> {
    let mut format = open_repo()
        .map(|r| r.object_format())
        .unwrap_or(arks_store::ObjectFormat::Sha1);
    for a in args {
        match a.strip_prefix("--object-format=") {
            Some("sha1") => format = arks_store::ObjectFormat::Sha1,
            Some("sha256") => format = arks_store::ObjectFormat::Sha256,
            Some(other) => {
                return Err(CliError::fatal(format!("Unknown hash algorithm '{other}'")));
            }
            None if a == "--no-object-format" => {}
            None => {
                return Err(CliError::Exit(
                    129,
                    "usage: git show-index [--object-format=<hash-algorithm>]".into(),
                ));
            }
        }
    }
    let mut bytes = Vec::new();
    io::stdin().read_to_end(&mut bytes)?;
    if bytes.is_empty() {
        return Err(CliError::fatal("unable to read header"));
    }
    let idx = arks_pack::PackIndex::decode(&bytes, format)
        .map_err(|_| CliError::fatal("unable to read index"))?;
    let mut out = String::new();
    for e in &idx.entries {
        out.push_str(&format!(
            "{} {} ({:08x})\n",
            e.offset,
            e.oid.to_hex(),
            e.crc32
        ));
    }
    print!("{out}");
    Ok(())
}

/// `arks check-attr [--source <tree-ish>] [--cached] [--stdin [-z]] [-a | --all | <attr>...] [--] <pathname>...`
/// over the gitattributes(5) sources (see `cmd::attr`).
/// Cite: https://git-scm.com/docs/git-check-attr
pub fn cmd_check_attr(args: &[String]) -> CliResult<()> {
    let mut all = false;
    let mut cached = false;
    let mut stdin = false;
    let mut nul = false;
    let mut source: Option<String> = None;
    let mut before: Vec<&String> = Vec::new();
    let mut after: Option<Vec<&String>> = None;
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        i += 1;
        if let Some(v) = after.as_mut() {
            v.push(a);
            continue;
        }
        match a.as_str() {
            "-a" | "--all" => all = true,
            "--no-all" => all = false,
            "--cached" => cached = true,
            "--no-cached" => cached = false,
            "--stdin" => stdin = true,
            "--no-stdin" => stdin = false,
            "-z" => nul = true,
            "--source" => {
                source = args.get(i).cloned();
                i += 1;
            }
            "--no-source" => source = None,
            "--" => after = Some(Vec::new()),
            _ if a.starts_with("--source=") => source = Some(a["--source=".len()..].to_string()),
            _ if a.starts_with('-') && a.len() > 1 => {
                return Err(CliError::Exit(
                    129,
                    format!(
                        "error: unknown option `{}'\nusage: git check-attr [--source <tree-ish>] [-a | --all | <attr>...] [--] <pathname>...",
                        a.trim_start_matches('-')
                    ),
                ));
            }
            _ => before.push(a),
        }
    }
    let (attrs, mut paths): (Vec<String>, Vec<String>) = match after {
        Some(p) => (
            before.iter().map(|s| s.to_string()).collect(),
            p.iter().map(|s| s.to_string()).collect(),
        ),
        None if all || stdin => (
            if all {
                Vec::new()
            } else {
                before.iter().map(|s| s.to_string()).collect()
            },
            if all {
                before.iter().map(|s| s.to_string()).collect()
            } else {
                Vec::new()
            },
        ),
        None if before.len() >= 2 => (
            vec![before[0].to_string()],
            before[1..].iter().map(|s| s.to_string()).collect(),
        ),
        None => {
            return Err(CliError::Exit(129, "error: No attribute specified".into()));
        }
    };
    if all && !attrs.is_empty() {
        return Err(CliError::Exit(
            129,
            "error: Attributes and --all both specified".into(),
        ));
    }
    if stdin {
        if !paths.is_empty() {
            return Err(CliError::Exit(
                129,
                "error: Can't specify files with --stdin".into(),
            ));
        }
        let mut input = Vec::new();
        io::stdin().read_to_end(&mut input)?;
        let sep = if nul { b'\0' } else { b'\n' };
        paths = input
            .split(|b| *b == sep)
            .filter(|p| !p.is_empty())
            .map(|p| String::from_utf8_lossy(p).into_owned())
            .collect();
    } else if paths.is_empty() {
        return Err(CliError::Exit(129, "error: No file specified".into()));
    }
    let repo = open_repo()?;
    let work = work_tree(&repo)?;
    // `--source` / `--cached`: materialize the .gitattributes files of that
    // tree (or the index) into a scratch tree and read rules from there.
    let scratch = match (&source, cached) {
        (Some(rev), _) => {
            let oid = resolve_rev(&repo, rev)
                .map_err(|_| CliError::fatal(format!("bad --source or --cached: {rev}")))?;
            let files = crate::cmd::switch_tree::commit_files(&repo, &oid)
                .map_err(|_| CliError::fatal(format!("{rev}: not a tree-ish")))?;
            Some(attr_scratch(
                &repo,
                files
                    .iter()
                    .filter(|(p, _)| p.ends_with(".gitattributes"))
                    .map(|(p, f)| (p.clone(), f.oid))
                    .collect(),
            )?)
        }
        (None, true) => {
            let index = crate::util::read_index(&repo)?;
            Some(attr_scratch(
                &repo,
                index
                    .entries
                    .iter()
                    .filter(|e| e.stage() == 0 && e.path.ends_with(".gitattributes"))
                    .map(|e| (e.path.clone(), e.oid))
                    .collect(),
            )?)
        }
        _ => None,
    };
    let base = scratch
        .as_ref()
        .map(|t| t.path().to_path_buf())
        .unwrap_or_else(|| work.clone());
    let mut out = Vec::new();
    for path in &paths {
        let rel = crate::util::rel_path_from_worktree(&work, Path::new(path))
            .unwrap_or_else(|_| path.to_string());
        let rules = crate::cmd::attr::Attributes::load(&base, repo.git_dir(), &rel);
        let found = rules.for_path(&rel);
        let mut emit = |k: &str, v: &str| {
            if nul {
                out.extend_from_slice(format!("{path}\0{k}\0{v}\0").as_bytes());
            } else {
                out.extend_from_slice(format!("{path}: {k}: {v}\n").as_bytes());
            }
        };
        if all {
            for k in rules.order() {
                match found.get(k) {
                    Some(v) if *v != crate::cmd::attr::AttrValue::Unspecified => {
                        emit(k, v.display());
                    }
                    _ => {}
                }
            }
        } else {
            for a in &attrs {
                let v = found
                    .get(a.as_str())
                    .map(|v| v.display())
                    .unwrap_or("unspecified");
                emit(a, v);
            }
        }
    }
    io::Write::write_all(&mut io::stdout(), &out)?;
    Ok(())
}

/// `arks merge-file` — three-way line merge of `<base> <ours> <theirs>` files.
pub fn cmd_merge_file(args: &[String]) -> CliResult<()> {
    use crate::cmd::optparse::{self, Item, Opt};
    use crate::cmd::plumbing_ext::{MergeFavor, MergeStyle};
    const SPEC: &[Opt] = &[
        Opt::flag("stdout").with_short('p'),
        Opt::flag("object-id"),
        Opt::flag("diff3"),
        Opt::flag("zdiff3"),
        Opt::flag("ours"),
        Opt::flag("theirs"),
        Opt::flag("union"),
        Opt::value("marker-size"),
        Opt::flag("quiet").with_short('q'),
        Opt::short_value('L', "L"),
    ];
    let usage = || {
        CliError::Exit(
            129,
            "usage: git merge-file [<options>] [-L <name1> [-L <orig> [-L <name2>]]] <file1> <orig-file> <file2>".into(),
        )
    };
    let items = optparse::parse("merge-file", args, SPEC, optparse::Mode::default())?;
    let mut style = MergeStyle::Merge;
    let mut favor = MergeFavor::None;
    let mut labels: Vec<String> = Vec::new();
    for it in &items {
        if let Item::Opt { name, value, neg } = it {
            match (*name, *neg) {
                ("diff3", false) => style = MergeStyle::Diff3,
                ("zdiff3", false) => style = MergeStyle::Zdiff3,
                ("diff3" | "zdiff3", true) => style = MergeStyle::Merge,
                ("ours", false) => favor = MergeFavor::Ours,
                ("theirs", false) => favor = MergeFavor::Theirs,
                ("union", false) => favor = MergeFavor::Union,
                ("ours" | "theirs" | "union", true) => favor = MergeFavor::None,
                ("L", _) => {
                    if labels.len() == 3 {
                        return Err(CliError::Exit(
                            129,
                            "error: too many labels on the command line".into(),
                        ));
                    }
                    labels.push(value.clone().unwrap_or_default());
                }
                _ => {}
            }
        }
    }
    let to_stdout = optparse::on(&items, "stdout");
    let object_id = optparse::on(&items, "object-id");
    let marker = match optparse::value(&items, "marker-size") {
        Some(v) => v.parse::<usize>().map_err(|_| {
            CliError::Exit(
                129,
                "error: option `marker-size' expects a numerical value".into(),
            )
        })?,
        None => 7,
    };
    let names = optparse::positionals(&items);
    if names.len() != 3 {
        return Err(usage());
    }
    let mut repo = if object_id { Some(open_repo()?) } else { None };
    let mut data: Vec<Vec<u8>> = Vec::new();
    for n in &names {
        match repo.as_ref() {
            Some(r) => {
                let oid = resolve_rev(r, n)
                    .map_err(|_| CliError::fatal(format!("object '{n}' does not exist")))?;
                data.push(r.get_object(&oid)?.payload.to_vec());
            }
            None => match fs::read(n) {
                Ok(b) => data.push(b),
                Err(e) => {
                    return Err(CliError::Exit(
                        255,
                        format!(
                            "error: Could not stat {n}: {}",
                            if e.kind() == io::ErrorKind::NotFound {
                                "No such file or directory".to_string()
                            } else {
                                e.to_string()
                            }
                        ),
                    ));
                }
            },
        }
    }
    let label = |i: usize| labels.get(i).cloned().unwrap_or_else(|| names[i].clone());
    let (l0, l1, l2) = (label(0), label(1), label(2));
    let (merged, conflicts) = crate::cmd::plumbing_ext::merge_three(
        &data[1],
        &data[0],
        &data[2],
        [&l0, &l1, &l2],
        style,
        favor,
        marker,
    );
    if to_stdout {
        use std::io::Write;
        io::stdout().write_all(&merged)?;
    } else if let Some(r) = repo.as_mut() {
        let format = r.object_format();
        let oid = r
            .store_mut()
            .put(format, &RawObject::new(ObjectKind::Blob, merged))?;
        println!("{}", oid.to_hex());
    } else {
        fs::write(&names[0], &merged)?;
    }
    if conflicts > 0 {
        return Err(CliError::silent(conflicts.min(127) as u8));
    }
    Ok(())
}

/// `arks merge-tree [--write-tree] [--merge-base=<c>] <branch1> <branch2>` and the
/// legacy `merge-tree <base> <ours> <theirs>` — print the merged tree oid
/// (conflict → exit 1). Never touches the index or the work tree.
/// Cite: https://git-scm.com/docs/git-merge-tree
pub fn cmd_merge_tree(args: &[String]) -> CliResult<()> {
    let mut write_tree = false;
    let mut name_only = false;
    let mut merge_base: Option<String> = None;
    let mut positional: Vec<&String> = Vec::new();
    for a in args {
        match a.as_str() {
            "--write-tree" => write_tree = true,
            "--name-only" => name_only = true,
            "--messages" | "--no-messages" | "--allow-unrelated-histories" => {}
            _ if a.starts_with("--merge-base=") => {
                merge_base = Some(a["--merge-base=".len()..].to_string());
            }
            _ if a.starts_with('-') => {
                return Err(CliError::NotImplemented(format!("unsupported option: {a}")));
            }
            _ => positional.push(a),
        }
    }
    let legacy = !write_tree && positional.len() == 3;
    if !(positional.len() == 2 || legacy) {
        return Err(CliError::Usage(
            "merge-tree [--write-tree] <branch1> <branch2> | merge-tree <base> <b1> <b2>".into(),
        ));
    }
    let mut repo = open_repo()?;
    let (base_rev, ours_rev, theirs_rev) = if legacy {
        (Some(positional[0].as_str()), positional[1], positional[2])
    } else {
        (merge_base.as_deref(), positional[0], positional[1])
    };
    let to_tree = |repo: &arks_repo::Repository, r: &str| -> CliResult<Oid> {
        let oid = resolve_rev(repo, r)
            .map_err(|_| CliError::fatal(format!("could not parse as tree '{r}'")))?;
        crate::cmd::shared::peel_to_kind(repo, &oid, crate::cmd::shared::ObjKind::Tree, r)
    };
    let ours = to_tree(&repo, ours_rev)?;
    let theirs = to_tree(&repo, theirs_rev)?;
    let base = match base_rev {
        Some(b) => to_tree(&repo, b)?,
        None => {
            let commit = |r: &str| -> CliResult<Oid> {
                let oid = resolve_rev(&repo, r)?;
                crate::cmd::shared::peel_to_kind(
                    &repo,
                    &oid,
                    crate::cmd::shared::ObjKind::Commit,
                    r,
                )
            };
            let (o, t) = (commit(ours_rev)?, commit(theirs_rev)?);
            let mb = arks_repo::merge_base_oid(&repo, &o, &t)?
                .ok_or_else(|| CliError::fatal("refusing to merge unrelated histories"))?;
            to_tree(&repo, &mb.to_hex())?
        }
    };
    let result = crate::cmd::merge_engine::merge_trees_in_memory(&mut repo, &base, &ours, &theirs)?;
    if !result.conflicts.is_empty() {
        for c in &result.conflicts {
            if name_only {
                println!("{c}");
            } else {
                println!("CONFLICT {c}");
            }
        }
        return Err(CliError::silent(1));
    }
    let tree = result
        .tree
        .ok_or_else(|| CliError::Msg("merge-tree: no result tree".into()))?;
    println!("{}", tree.to_hex());
    Ok(())
}

/// `arks hook run [--ignore-missing] [--to-stdin=<path>] <hook-name> [-- <hook-args>]`.
/// Cite: https://git-scm.com/docs/git-hook
pub fn cmd_hook(args: &[String]) -> CliResult<()> {
    let usage = || {
        CliError::Exit(
            129,
            "usage: git hook run [--ignore-missing] [--to-stdin=<path>] <hook-name> [-- <hook-args>]"
                .into(),
        )
    };
    let Some(sub) = args.first() else {
        return Err(usage());
    };
    if sub != "run" {
        return Err(CliError::Exit(
            129,
            format!(
                "error: unknown subcommand: `{sub}'\nusage: git hook run [--ignore-missing] [--to-stdin=<path>] <hook-name> [-- <hook-args>]"
            ),
        ));
    }
    let mut ignore_missing = false;
    let mut to_stdin: Option<String> = None;
    let mut name: Option<String> = None;
    let mut hook_args: Vec<String> = Vec::new();
    let mut i = 1;
    while i < args.len() {
        let a = &args[i];
        i += 1;
        if name.is_some() {
            if a == "--" && hook_args.is_empty() {
                hook_args.extend(args[i..].iter().cloned());
                break;
            }
            return Err(usage());
        }
        match a.as_str() {
            "--ignore-missing" => ignore_missing = true,
            "--no-ignore-missing" => ignore_missing = false,
            "--to-stdin" => {
                to_stdin = args.get(i).cloned();
                i += 1;
            }
            "--no-to-stdin" => to_stdin = None,
            _ if a.starts_with("--to-stdin=") => {
                to_stdin = Some(a["--to-stdin=".len()..].to_string())
            }
            _ if a.starts_with('-') => return Err(usage()),
            _ => name = Some(a.clone()),
        }
    }
    let name = name.ok_or_else(usage)?;
    let repo = open_repo()?;
    let path = repo.git_dir().join("hooks").join(&name);
    let runnable = path.is_file() && {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::metadata(&path)
                .map(|m| m.permissions().mode() & 0o111 != 0)
                .unwrap_or(false)
        }
        #[cfg(not(unix))]
        {
            true
        }
    };
    if !runnable {
        if ignore_missing {
            return Ok(());
        }
        return Err(CliError::Exit(
            1,
            format!("error: cannot find a hook named {name}"),
        ));
    }
    let mut cmd = std::process::Command::new(&path);
    cmd.args(&hook_args).env("GIT_DIR", repo.git_dir());
    if let Ok(wt) = work_tree(&repo) {
        cmd.current_dir(wt);
    }
    match &to_stdin {
        Some(p) => {
            cmd.stdin(fs::File::open(p).map_err(|e| CliError::fatal(format!("{p}: {e}")))?);
        }
        None => {
            cmd.stdin(std::process::Stdio::null());
        }
    }
    // Hook stdout goes to stderr like run_hooks_opt (stdout_to_stderr).
    let out = cmd
        .stdout(std::process::Stdio::piped())
        .output()
        .map_err(|e| CliError::fatal(format!("cannot exec '{}': {e}", path.display())))?;
    io::Write::write_all(&mut io::stderr(), &out.stdout)?;
    match out.status.code() {
        Some(0) => Ok(()),
        Some(c) => Err(CliError::silent(c.clamp(1, 255) as u8)),
        None => Err(CliError::silent(1)),
    }
}

/// `arks unpack-objects [-n] [-q] [-r] [--strict] [--max-input-size=<n>]` —
/// explode a pack from stdin into loose objects.
/// Cite: https://git-scm.com/docs/git-unpack-objects
pub fn cmd_unpack_objects(args: &[String]) -> CliResult<()> {
    let mut dry = false;
    let mut max: Option<u64> = None;
    for a in args {
        match a.as_str() {
            "-n" => dry = true,
            "-q" | "-r" | "--strict" => {}
            _ if a.starts_with("--strict=") || a.starts_with("--pack_header=") => {}
            _ if a.starts_with("--max-input-size=") => {
                max = a["--max-input-size=".len()..].parse().ok();
            }
            _ => {
                return Err(CliError::Exit(
                    129,
                    "usage: git unpack-objects [-n] [-q] [-r] [--strict]".into(),
                ));
            }
        }
    }
    let mut buf = Vec::new();
    io::stdin().read_to_end(&mut buf)?;
    if let Some(m) = max {
        if m > 0 && buf.len() as u64 > m {
            return Err(CliError::fatal("pack exceeds maximum allowed size"));
        }
    }
    let mut repo = open_repo()?;
    let format = repo.object_format();
    let decoded = arks_pack::DecodedPack::decode(&buf, format)
        .map_err(|e| CliError::fatal(format!("{e}")))?;
    if !dry {
        for obj in &decoded.objects {
            if !repo.store().exists(&obj.oid).unwrap_or(false) {
                repo.store_mut().put(format, &obj.object)?;
            }
        }
    }
    Ok(())
}

/// `arks verify-pack [-v | --verbose] [-s | --stat-only] [--object-format=<hash>] <pack>.idx...`
/// Cite: https://git-scm.com/docs/git-verify-pack
pub fn cmd_verify_pack(args: &[String]) -> CliResult<()> {
    let mut verbose = false;
    let mut stat_only = false;
    let mut format = open_repo()
        .map(|r| r.object_format())
        .unwrap_or(arks_store::ObjectFormat::Sha1);
    let usage = || {
        CliError::Exit(
            129,
            "usage: git verify-pack [-v | --verbose] [-s | --stat-only] [--] <pack>.idx...".into(),
        )
    };
    let mut packs: Vec<&String> = Vec::new();
    let mut dashdash = false;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        if dashdash {
            packs.push(a);
            continue;
        }
        if a == "--object-format" {
            format = match it.next().map(String::as_str) {
                Some("sha1") => arks_store::ObjectFormat::Sha1,
                Some("sha256") => arks_store::ObjectFormat::Sha256,
                _ => return Err(usage()),
            };
            continue;
        }
        match a.as_str() {
            "-v" | "--verbose" => verbose = true,
            "--no-verbose" => verbose = false,
            "-s" | "--stat-only" => stat_only = true,
            "--no-stat-only" => stat_only = false,
            "-vs" | "-sv" => {
                verbose = true;
                stat_only = true;
            }
            "--" => dashdash = true,
            "--object-format=sha256" => format = arks_store::ObjectFormat::Sha256,
            "--object-format=sha1" | "--no-object-format" => {
                if a != "--no-object-format" {
                    format = arks_store::ObjectFormat::Sha1;
                }
            }
            _ if a.starts_with('-') => return Err(usage()),
            _ => packs.push(a),
        }
    }
    if packs.is_empty() {
        return Err(usage());
    }
    let mut failed = false;
    let mut out = String::new();
    for p in packs {
        let base = p
            .strip_suffix(".idx")
            .or_else(|| p.strip_suffix(".pack"))
            .unwrap_or(p);
        let pack_path = format!("{base}.pack");
        let idx_path = format!("{base}.idx");
        let res = crate::cmd::plumbing_ext::verify_one_pack(
            Path::new(&pack_path),
            Path::new(&idx_path),
            format,
            verbose,
            stat_only,
        );
        match res {
            Ok(text) => {
                out.push_str(&text);
                if verbose && !stat_only {
                    out.push_str(&format!("{pack_path}: ok\n"));
                }
            }
            Err(e) => {
                print!("{out}");
                out.clear();
                eprintln!("error: {e}");
                if verbose {
                    println!("{pack_path}: bad");
                }
                failed = true;
            }
        }
    }
    print!("{out}");
    if failed {
        return Err(CliError::silent(1));
    }
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

/// `arks check-mailmap [--stdin] <contact>...` — map `Name <email>` through
/// `.mailmap`, `mailmap.file` and `mailmap.blob`.
/// Cite: https://git-scm.com/docs/git-check-mailmap, gitmailmap(5)
pub fn cmd_check_mailmap(args: &[String]) -> CliResult<()> {
    let mut stdin = false;
    let mut contacts: Vec<&String> = Vec::new();
    for a in args {
        match a.as_str() {
            "--stdin" => stdin = true,
            "--no-stdin" => stdin = false,
            _ if a.starts_with('-') => {
                return Err(CliError::Exit(
                    129,
                    "usage: git check-mailmap [<options>] <contact>...".into(),
                ));
            }
            _ => contacts.push(a),
        }
    }
    if contacts.is_empty() && !stdin {
        return Err(CliError::fatal("no contacts specified"));
    }
    let repo = open_repo()?;
    let map = crate::cmd::plumbing_ext::Mailmap::load(&repo);
    let mut out = String::new();
    for c in contacts {
        out.push_str(&map.check(c)?);
        out.push('\n');
    }
    print!("{out}");
    if stdin {
        let mut input = String::new();
        io::stdin().read_to_string(&mut input)?;
        let mut o = io::stdout().lock();
        for line in input.lines() {
            let mapped = map.check(line)?;
            io::Write::write_all(&mut o, format!("{mapped}\n").as_bytes())?;
            io::Write::flush(&mut o)?;
        }
    }
    Ok(())
}

/// `arks column [--command=<name>] [--mode=<mode>] [--raw-mode=<n>] [--width=<w>]
/// [--indent=<s>] [--nl=<s>] [--padding=<n>]` — lay out stdin lines in columns.
/// Cite: https://git-scm.com/docs/git-column, git-config `column.ui`
pub fn cmd_column(args: &[String]) -> CliResult<()> {
    let mut opts = crate::cmd::plumbing_ext::ColumnOpts::default();
    let mut command: Option<String> = None;
    let mut mode_specs: Vec<String> = Vec::new();
    let mut raw_mode: Option<u32> = None;
    let mut i = 0;
    let usage = |m: &str| CliError::Exit(129, format!("error: {m}\nusage: git column [<options>]"));
    let value = |i: &mut usize, a: &str, name: &str| -> CliResult<Option<String>> {
        if let Some(v) = a.strip_prefix(&format!("--{name}=")) {
            return Ok(Some(v.to_string()));
        }
        if a == format!("--{name}") {
            let v = args.get(*i).cloned().ok_or_else(|| {
                CliError::Exit(129, format!("error: option `{name}' requires a value"))
            })?;
            *i += 1;
            return Ok(Some(v));
        }
        Ok(None)
    };
    while i < args.len() {
        let a = args[i].clone();
        i += 1;
        if a == "--mode" {
            mode_specs.push("column".into());
            continue;
        }
        if let Some(v) = a.strip_prefix("--mode=") {
            mode_specs.push(v.to_string());
            continue;
        }
        if a == "--no-mode" {
            mode_specs.clear();
            raw_mode = Some(0);
            continue;
        }
        let at = i - 1;
        if let Some(v) = value(&mut i, &a, "command")? {
            if at != 0 {
                return Err(CliError::fatal("--command must be the first argument"));
            }
            command = Some(v);
            continue;
        }
        if let Some(v) = value(&mut i, &a, "raw-mode")? {
            raw_mode = Some(
                v.parse()
                    .map_err(|_| usage("option `raw-mode' expects a numerical value"))?,
            );
            continue;
        }
        if let Some(v) = value(&mut i, &a, "width")? {
            opts.width = Some(
                v.parse()
                    .map_err(|_| usage("option `width' expects a numerical value"))?,
            );
            continue;
        }
        if let Some(v) = value(&mut i, &a, "indent")? {
            opts.indent = v;
            continue;
        }
        if let Some(v) = value(&mut i, &a, "nl")? {
            opts.nl = Some(v);
            continue;
        }
        if let Some(v) = value(&mut i, &a, "padding")? {
            opts.padding = v
                .parse()
                .map_err(|_| usage("option `padding' expects a numerical value"))?;
            continue;
        }
        match a.as_str() {
            "--no-command" => command = None,
            "--no-raw-mode" => raw_mode = None,
            "--no-width" => opts.width = None,
            "--no-indent" => opts.indent.clear(),
            "--no-nl" => opts.nl = None,
            "--no-padding" => opts.padding = 1,
            _ => {
                if a.starts_with('-') {
                    return Err(CliError::Exit(
                        129,
                        format!(
                            "error: unknown option `{}'\nusage: git column [<options>]",
                            a.trim_start_matches('-')
                        ),
                    ));
                }
                return Err(CliError::Exit(129, "usage: git column [<options>]".into()));
            }
        }
    }
    let mut layout = crate::cmd::plumbing_ext::ColumnLayout::default();
    if let Some(cmd) = &command {
        let cwd = crate::util::cwd()?;
        let cfg = arks_config::ConfigSet::load_discovered(&cwd);
        for key in ["column.ui".to_string(), format!("column.{cmd}")] {
            if let Some(v) = cfg.get(&key) {
                layout.apply(&v).map_err(|bad| {
                    CliError::Exit(129, format!("error: invalid column.{cmd} mode {bad}"))
                })?;
            }
        }
    }
    for spec in &mode_specs {
        layout
            .apply(spec)
            .map_err(|bad| CliError::Exit(129, format!("error: unsupported option '{bad}'")))?;
    }
    if let Some(r) = raw_mode {
        layout = crate::cmd::plumbing_ext::ColumnLayout::from_raw(r);
    }
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;
    let items: Vec<&str> = input.lines().collect();
    print!(
        "{}",
        crate::cmd::plumbing_ext::render_columns(&items, &layout, &opts)
    );
    Ok(())
}

/// `arks for-each-repo --config=<key> [--] <arguments>` — run the command in
/// every repository listed under the multi-valued config `<key>`.
/// Cite: https://git-scm.com/docs/git-for-each-repo
pub fn cmd_for_each_repo(args: &[String]) -> CliResult<()> {
    let mut key: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        if let Some(v) = a.strip_prefix("--config=") {
            key = Some(v.to_string());
        } else if a == "--config" {
            i += 1;
            key = args.get(i).cloned();
        } else if a == "--no-config" {
            key = None;
        } else if a == "--" {
            i += 1;
            break;
        } else if a.starts_with('-') {
            return Err(CliError::Exit(
                129,
                format!(
                    "error: unknown option `{}'\nusage: git for-each-repo --config=<config> [--] <arguments>",
                    a.trim_start_matches('-')
                ),
            ));
        } else {
            break;
        }
        i += 1;
    }
    let key = key.ok_or_else(|| CliError::fatal("missing --config=<config>"))?;
    let cmd_args = &args[i.min(args.len())..];
    let cwd = crate::util::cwd()?;
    let cfg = arks_config::ConfigSet::load_discovered(&cwd);
    let paths = cfg.get_all(&key);
    let exe = std::env::current_exe().map_err(|e| CliError::Msg(e.to_string()))?;
    for p in paths {
        let dir = crate::cmd::plumbing_ext::expand_home(&p);
        let status = std::process::Command::new(&exe)
            .arg("-C")
            .arg(&dir)
            .args(cmd_args)
            .status()
            .map_err(|e| CliError::Msg(e.to_string()))?;
        if !status.success() {
            // The first failing repository's status ends the loop.
            return Err(CliError::silent(
                status.code().unwrap_or(1).clamp(1, 255) as u8
            ));
        }
    }
    Ok(())
}

/// `arks fmt-merge-msg [-m <message>] [--into-name <branch>] [--log[=<n>] | --no-log] [-F <file>]`
/// — FETCH_HEAD-style input → merge commit message.
/// Cite: https://git-scm.com/docs/git-fmt-merge-msg
pub fn cmd_fmt_merge_msg(args: &[String]) -> CliResult<()> {
    let mut message: Option<String> = None;
    let mut into: Option<String> = None;
    let mut log: Option<usize> = None;
    let mut log_set = false;
    let mut file: Option<String> = None;
    let mut i = 0;
    let usage = || {
        CliError::Exit(
            129,
            "usage: git fmt-merge-msg [-m <message>] [--log[=<n>] | --no-log] [--file <file>]"
                .into(),
        )
    };
    while i < args.len() {
        let a = args[i].as_str();
        i += 1;
        let take = |name: &str| -> CliResult<Option<String>> {
            if let Some(v) = a.strip_prefix(&format!("--{name}=")) {
                return Ok(Some(v.to_string()));
            }
            Ok(None)
        };
        if let Some(v) = take("message")? {
            message = Some(v);
        } else if let Some(v) = take("into-name")? {
            into = Some(v);
        } else if let Some(v) = take("file")? {
            file = Some(v);
        } else if let Some(v) = take("log")? {
            log = Some(v.parse().map_err(|_| usage())?);
            log_set = true;
        } else {
            match a {
                "-m" | "--message" => {
                    message = Some(args.get(i).cloned().ok_or_else(usage)?);
                    i += 1;
                }
                "--no-message" => message = None,
                "--into-name" => {
                    into = Some(args.get(i).cloned().ok_or_else(usage)?);
                    i += 1;
                }
                "--no-into-name" => into = None,
                "-F" | "--file" => {
                    file = Some(args.get(i).cloned().ok_or_else(usage)?);
                    i += 1;
                }
                "--no-file" => file = None,
                "--log" | "--summary" => {
                    log = Some(20);
                    log_set = true;
                }
                "--no-log" | "--no-summary" => {
                    log = None;
                    log_set = true;
                }
                _ if a.starts_with("-m") && a.len() > 2 => message = Some(a[2..].to_string()),
                _ if a.starts_with("-F") && a.len() > 2 => file = Some(a[2..].to_string()),
                _ => return Err(usage()),
            }
        }
    }
    let input = match &file {
        Some(f) if f != "-" => fs::read_to_string(f)
            .map_err(|e| CliError::fatal(format!("could not open '{f}' for reading: {e}")))?,
        _ => {
            let mut s = String::new();
            io::stdin().read_to_string(&mut s)?;
            s
        }
    };
    let repo = open_repo()?;
    if !log_set {
        let cwd = crate::util::cwd()?;
        let cfg = arks_config::ConfigSet::load_discovered(&cwd);
        if let Some(v) = cfg.get("merge.log") {
            log = match v.parse::<usize>() {
                Ok(0) => None,
                Ok(n) => Some(n),
                Err(_) => arks_config::parse_bool(&v).unwrap_or(false).then_some(20),
            };
        }
    }
    let text = crate::cmd::plumbing_ext::fmt_merge_msg(
        &repo,
        &input,
        message.as_deref(),
        into.as_deref(),
        log,
    )?;
    print!("{text}");
    Ok(())
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
        if let Some((name, email)) = parse_from(author) {
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
    if let Some((name, email)) = parse_from(author) {
        println!("Author: {name}");
        println!("Email: {email}");
    }
    println!("Subject: {subject}");
    Ok(())
}

fn split_mail_headers(raw: &str) -> (Vec<(String, String)>, &str) {
    let mut headers = Vec::new();
    let lines = raw.split_inclusive('\n');
    let mut body_start = 0usize;
    let bytes = raw.as_bytes();
    let mut offset = 0usize;
    for line in lines {
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

/// `arks get-tar-commit-id` — print the commit id stored in a `git archive`
/// tar's pax global header (`comment=`); exit 1 when absent.
/// Cite: https://git-scm.com/docs/git-get-tar-commit-id
pub fn cmd_get_tar_commit_id(args: &[String]) -> CliResult<()> {
    if !args.is_empty() {
        return Err(CliError::Exit(129, "usage: git get-tar-commit-id".into()));
    }
    let mut buf = vec![0u8; 1024];
    let mut got = 0;
    let mut stdin = io::stdin().lock();
    while got < 1024 {
        let n = stdin.read(&mut buf[got..])?;
        if n == 0 {
            break;
        }
        got += n;
    }
    if got < 1024 {
        return Err(CliError::fatal(
            "git get-tar-commit-id: EOF before reading tar header: Success",
        ));
    }
    // Header block: typeflag 'g' at 156, ustar magic at 257; record at 512.
    if buf[156] != b'g' || &buf[257..262] != b"ustar" {
        return Err(CliError::silent(1));
    }
    let rec = &buf[512..1024];
    let text = String::from_utf8_lossy(rec);
    let Some(rest) = text
        .strip_prefix("52 comment=")
        .or_else(|| text.strip_prefix("76 comment="))
    else {
        return Err(CliError::silent(1));
    };
    let oid: String = rest.chars().take_while(|c| c.is_ascii_hexdigit()).collect();
    if oid.len() != 40 && oid.len() != 64 {
        return Err(CliError::silent(1));
    }
    println!("{oid}");
    Ok(())
}

/// `arks unpack-file <blob>` — write the blob to `.merge_file_XXXXXX` in the
/// current directory and print that name. Cite: https://git-scm.com/docs/git-unpack-file
pub fn cmd_unpack_file(args: &[String]) -> CliResult<()> {
    let [rev] = args else {
        return Err(CliError::Exit(129, "usage: git unpack-file <blob>".into()));
    };
    let repo = open_repo()?;
    let oid = resolve_rev(&repo, rev)
        .map_err(|_| CliError::fatal(format!("Not a valid object name {rev}")))?;
    let obj = repo
        .get_object(&oid)
        .map_err(|_| CliError::fatal(format!("unable to read blob object {}", oid.to_hex())))?;
    if obj.kind != ObjectKind::Blob {
        return Err(CliError::fatal(format!(
            "unable to read blob object {}",
            oid.to_hex()
        )));
    }
    let name = crate::cmd::plumbing_ext::unique_merge_file_name(&crate::util::cwd()?);
    fs::write(&name, &obj.payload)?;
    println!(
        "{}",
        name.file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default()
    );
    Ok(())
}

/// `arks bugreport [-o <dir>] [-s <format>] [--diagnose[=<mode>]]` — write a
/// `git-bugreport-<suffix>.txt` system report (no network, no git spawn).
/// Cite: https://git-scm.com/docs/git-bugreport
pub fn cmd_bugreport(args: &[String]) -> CliResult<()> {
    let o = crate::cmd::plumbing_ext::ReportOpts::parse(args, "bugreport", true)?;
    let path = o.target("git-bugreport", "txt")?;
    let mut text = String::new();
    text.push_str("Thank you for filling out a Git bug report!\nPlease answer the following questions to help us understand your issue.\n\n");
    text.push_str("What did you do before the bug happened? (Steps to reproduce your issue)\n\n");
    text.push_str("What did you expect to happen? (Expected behavior)\n\n");
    text.push_str("What happened instead? (Actual behavior)\n\n");
    text.push_str("What's different between what you expected and what actually happened?\n\n");
    text.push_str("Anything else you want to add:\n\n");
    text.push_str("Please review the rest of the bug report below.\nYou can delete any lines you don't wish to share.\n\n\n");
    text.push_str("[System Info]\n");
    text.push_str(&format!(
        "arks version:\narks version {}\n",
        env!("CARGO_PKG_VERSION")
    ));
    text.push_str(&format!(
        "uname: {} {}\n",
        std::env::consts::OS,
        std::env::consts::ARCH
    ));
    text.push_str(&format!(
        "$SHELL (typically, interactive shell): {}\n\n\n",
        std::env::var("SHELL").unwrap_or_default()
    ));
    text.push_str("[Enabled Hooks]\n");
    if let Ok(repo) = open_repo() {
        if let Ok(rd) = fs::read_dir(repo.git_dir().join("hooks")) {
            let mut names: Vec<String> = rd
                .flatten()
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .filter(|n| !n.ends_with(".sample"))
                .collect();
            names.sort();
            for n in names {
                text.push_str(&n);
                text.push('\n');
            }
        }
    }
    fs::write(&path, text)?;
    eprintln!("Created new report at '{}'.", path.display());
    if let Some(mode) = &o.diagnose {
        let zip = o.target("git-diagnostics", "zip")?;
        crate::cmd::plumbing_ext::write_diagnostics(&zip, mode)?;
        eprintln!("Created new diagnostics archive at '{}'.", zip.display());
    }
    Ok(())
}

/// `arks diagnose [-o <dir>] [-s <format>] [--mode=stats|all]` — write a
/// `git-diagnostics-<suffix>.zip` archive of repository diagnostics.
/// Cite: https://git-scm.com/docs/git-diagnose
pub fn cmd_diagnose(args: &[String]) -> CliResult<()> {
    let o = crate::cmd::plumbing_ext::ReportOpts::parse(args, "diagnose", false)?;
    let zip = o.target("git-diagnostics", "zip")?;
    let mode = o.diagnose.clone().unwrap_or_else(|| "stats".into());
    crate::cmd::plumbing_ext::write_diagnostics(&zip, &mode)?;
    eprintln!(
        "Diagnostics complete.\nAll of the gathered info is captured in '{}'",
        zip.display()
    );
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

/// `arks upload-archive <directory>`: the server half of `archive --remote`.
/// Reads `argument <arg>` pkt-lines up to a flush, answers `ACK` + flush,
/// then streams the archive on side-band 1 (errors on bands 2 and 3).
/// Cite: <https://git-scm.com/docs/git-upload-archive>,
/// <https://git-scm.com/docs/gitprotocol-common> (pkt-line, side-band).
pub fn cmd_upload_archive(args: &[String]) -> CliResult<()> {
    use std::io::{Read, Write};
    let usage = || CliError::Exit(129, "usage: git upload-archive <repository>".into());
    if args.iter().any(|a| a == "-h") {
        return Err(usage());
    }
    let [dir] = args else {
        return Err(usage());
    };
    let repo = arks_repo::Repository::open(dir)
        .or_else(|_| arks_repo::Repository::open(std::path::Path::new(dir).join(".git")))
        .map_err(|_| CliError::fatal(format!("'{dir}' does not appear to be a git repository")))?;
    let mut input = std::io::stdin().lock();
    let mut argv: Vec<String> = Vec::new();
    let mut complete = false;
    loop {
        let mut len = [0u8; 4];
        if input.read_exact(&mut len).is_err() {
            break;
        }
        let Some(n) = std::str::from_utf8(&len)
            .ok()
            .and_then(|h| usize::from_str_radix(h, 16).ok())
        else {
            break;
        };
        if n == 0 {
            complete = true;
            break;
        }
        if n < 4 {
            break;
        }
        let mut buf = vec![0u8; n - 4];
        if input.read_exact(&mut buf).is_err() {
            break;
        }
        let line = String::from_utf8_lossy(&buf);
        let line = line.strip_suffix('\n').unwrap_or(&line);
        match line.strip_prefix("argument ") {
            Some(a) => argv.push(a.to_string()),
            None => break,
        }
    }
    let pkt = |band: Option<u8>, data: &[u8]| -> Vec<u8> {
        let extra = usize::from(band.is_some());
        let mut v = format!("{:04x}", data.len() + 4 + extra).into_bytes();
        v.extend(band);
        v.extend_from_slice(data);
        v
    };
    let mut out = std::io::stdout().lock();
    out.write_all(&pkt(None, b"ACK\n"))?;
    out.write_all(b"0000")?;
    let result = if complete {
        crate::cmd::archive::archive_bytes_for_args(&repo, &argv)
    } else {
        Err(CliError::fatal("the remote end hung up unexpectedly"))
    };
    match result {
        Ok(bytes) => {
            for chunk in bytes.chunks(65515) {
                out.write_all(&pkt(Some(1), chunk))?;
            }
            out.write_all(b"0000")?;
            out.flush()?;
            Ok(())
        }
        Err(e) => {
            let msg = e.to_string();
            let msg = msg.trim_end();
            let msg = if msg.starts_with("fatal: ") || msg.starts_with("error: ") {
                msg.to_string()
            } else {
                format!("fatal: {msg}")
            };
            out.write_all(&pkt(Some(2), format!("{msg}\n").as_bytes()))?;
            let died = "git upload-archive: archiver died with error";
            out.write_all(&pkt(Some(3), died.as_bytes()))?;
            out.flush()?;
            Err(CliError::fatal(format!("sent error to the client: {died}")))
        }
    }
}

/// `@{-N}` → the N-th previously checked out branch (HEAD reflog).
fn previous_branch(repo: &Repository, spec: &str) -> Option<String> {
    let n: usize = spec.strip_prefix("@{-")?.strip_suffix('}')?.parse().ok()?;
    let entries = crate::reflog::read_entries(repo, "HEAD");
    let mut seen = 0;
    for (_, _, msg) in entries.iter().rev() {
        if let Some(rest) = msg.strip_prefix("checkout: moving from ") {
            seen += 1;
            if seen == n {
                return rest.split(" to ").next().map(str::to_string);
            }
        }
    }
    None
}

fn attr_scratch(
    repo: &Repository,
    files: Vec<(String, Oid)>,
) -> CliResult<crate::cmd::plumbing_ext::ScratchDir> {
    let dir = std::env::temp_dir().join(format!(
        "arks-attr-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    fs::create_dir_all(&dir)?;
    let guard = crate::cmd::plumbing_ext::ScratchDir(dir);
    for (p, oid) in files {
        let obj = repo.get_object(&oid)?;
        let dest = guard.path().join(&p);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(dest, &obj.payload)?;
    }
    Ok(guard)
}

fn parse_expire(v: &str) -> CliResult<i64> {
    crate::cmd::config::expiry_date(v)
        .ok_or_else(|| CliError::fatal(format!("malformed expiration date '{v}'")))
}

/// A path as git prints it: relative to the current directory when below it.
fn shown_path(p: &Path) -> String {
    let cwd = std::env::current_dir().unwrap_or_default();
    p.strip_prefix(&cwd)
        .map(|r| r.display().to_string())
        .unwrap_or_else(|_| p.display().to_string())
}
