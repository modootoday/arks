//! Remaining pin Missing cmds → Specfold-honest Partial (or refuse-with-reason).
//!
//! Cite: [[sot:root/cli-git-help-matrix]]; never spawn `git`/`gh`.

use crate::cmd::shared::resolve_rev;
use crate::error::{CliError, CliResult};
use crate::util::{open_repo, refuse_unknown_flags};
use arks_object::{Commit, ObjectKind, RawObject, Signature, Tree, TreeEntry, TreeMode};
use arks_oid::Oid;
use arks_store::ObjectStore;
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::io::{self, Read, Write};
use std::path::Path;

/// `arks commit-graph` — verify/list subset (write format not claimed).
pub fn cmd_commit_graph(args: &[String]) -> CliResult<()> {
    if args.is_empty() || args.iter().any(|a| a == "-h" || a == "--help") {
        print!(
            "\
arks commit-graph — commit-graph file helpers (subset)

Usage:
  arks commit-graph verify
  arks commit-graph list

write / --append / split not claimed (format depth).
"
        );
        return Ok(());
    }
    refuse_unknown_flags(args, &["verify", "write", "list"])?;
    let sub = args
        .iter()
        .find(|a| !a.starts_with('-'))
        .map(String::as_str)
        .unwrap_or("verify");
    let repo = open_repo()?;
    let path = repo
        .git_dir()
        .join("objects/info/commit-graph")
        .canonicalize()
        .unwrap_or_else(|_| repo.git_dir().join("objects/info/commit-graph"));
    let alt = repo.git_dir().join("objects/info/commit-graphs");
    match sub {
        "verify" => {
            if path.is_file() {
                let n = fs::metadata(&path)?.len();
                println!("ok\t{}\t{n} bytes", path.display());
                Ok(())
            } else if alt.is_dir() {
                println!("ok\tchain dir\t{}", alt.display());
                Ok(())
            } else {
                Err(CliError::Msg(
                    "commit-graph verify: no commit-graph present".into(),
                ))
            }
        }
        "list" => {
            if path.is_file() {
                println!("{}", path.display());
            }
            if alt.is_dir() {
                for ent in fs::read_dir(&alt)? {
                    let ent = ent?;
                    println!("{}", ent.path().display());
                }
            }
            Ok(())
        }
        "write" => Err(CliError::NotImplemented(
            "commit-graph write: on-disk format writer not claimed (Partial; verify/list only)"
                .into(),
        )),
        other => Err(CliError::Usage(format!(
            "commit-graph: unknown subcommand {other}"
        ))),
    }
}

/// `arks multi-pack-index` — list/verify subset (write not claimed).
pub fn cmd_multi_pack_index(args: &[String]) -> CliResult<()> {
    if args.is_empty() || args.iter().any(|a| a == "-h" || a == "--help") {
        print!(
            "\
arks multi-pack-index — MIDX helpers (subset)

Usage:
  arks multi-pack-index list
  arks multi-pack-index verify
  arks multi-pack-index write   # not claimed
"
        );
        return Ok(());
    }
    refuse_unknown_flags(args, &["list", "verify", "write", "expire", "repack"])?;
    let sub = args
        .iter()
        .find(|a| !a.starts_with('-'))
        .map(String::as_str)
        .unwrap_or("list");
    let repo = open_repo()?;
    let pack_dir = repo.git_dir().join("objects/pack");
    let midx = pack_dir.join("multi-pack-index");
    match sub {
        "list" => {
            if midx.is_file() {
                println!("{}", midx.display());
            }
            if pack_dir.is_dir() {
                for ent in fs::read_dir(&pack_dir)? {
                    let ent = ent?;
                    let name = ent.file_name().to_string_lossy().into_owned();
                    if name.ends_with(".pack") {
                        println!("{}", ent.path().display());
                    }
                }
            }
            Ok(())
        }
        "verify" => {
            if !midx.is_file() {
                return Err(CliError::Msg(
                    "multi-pack-index verify: no multi-pack-index present".into(),
                ));
            }
            let n = fs::metadata(&midx)?.len();
            println!("ok\t{}\t{n} bytes (presence check; checksum algebra not claimed)", midx.display());
            Ok(())
        }
        "write" | "expire" | "repack" => Err(CliError::NotImplemented(format!(
            "multi-pack-index {sub}: not claimed (Partial; list/verify only)"
        ))),
        other => Err(CliError::Usage(format!(
            "multi-pack-index: unknown subcommand {other}"
        ))),
    }
}

/// `arks rerere` — rr-cache status/clear subset.
pub fn cmd_rerere(args: &[String]) -> CliResult<()> {
    if args.iter().any(|a| a == "-h" || a == "--help") {
        print!(
            "\
arks rerere — reuse recorded resolution (subset)

Usage:
  arks rerere status
  arks rerere clear
  arks rerere diff     # not claimed
"
        );
        return Ok(());
    }
    refuse_unknown_flags(args, &["status", "clear", "diff", "gc", "remaining", "forget"])?;
    let sub = args
        .first()
        .map(String::as_str)
        .unwrap_or("status");
    let repo = open_repo()?;
    let rr = repo.git_dir().join("rr-cache");
    match sub {
        "status" | "remaining" => {
            if !rr.is_dir() {
                return Ok(());
            }
            let mut n = 0usize;
            for ent in fs::read_dir(&rr)? {
                let ent = ent?;
                if ent.file_type()?.is_dir() {
                    println!("{}", ent.file_name().to_string_lossy());
                    n += 1;
                }
            }
            eprintln!("rerere status: {n} recorded conflict(s) (subset)");
            Ok(())
        }
        "clear" => {
            if rr.is_dir() {
                fs::remove_dir_all(&rr)?;
            }
            Ok(())
        }
        "diff" | "gc" | "forget" => Err(CliError::NotImplemented(format!(
            "rerere {sub}: not claimed (Partial; status/clear only)"
        ))),
        other if other.starts_with('-') => Err(CliError::NotImplemented(format!(
            "rerere option not claimed: {other}"
        ))),
        other => Err(CliError::Usage(format!(
            "rerere: unknown subcommand {other}"
        ))),
    }
}

/// `arks subtree` — discoverability + honest refuse (full subtree not claimed).
pub fn cmd_subtree(args: &[String]) -> CliResult<()> {
    if args.is_empty() || args.iter().any(|a| a == "-h" || a == "--help") {
        print!(
            "\
arks subtree — git-subtree peer (not claimed beyond help)

Usage:
  arks subtree add|split|pull|push|merge …

Full prefix-history rewrite is not claimed. Prefer fetch + read-tree / merge
workflows, or external git-subtree until a native subset is evidenced.
"
        );
        return Ok(());
    }
    let sub = args[0].as_str();
    Err(CliError::NotImplemented(format!(
        "subtree {sub}: not claimed (Partial help only; no git/gh spawn fallback)"
    )))
}

/// `arks fast-export` — emit a simplified fast-export stream for commits (subset).
pub fn cmd_fast_export(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &["--all", "--progress"])?;
    let all = args.iter().any(|a| a == "--all");
    let repo = open_repo()?;
    let tips: Vec<Oid> = if all {
        let mut tips = Vec::new();
        for name in ["HEAD", "refs/heads", "refs/tags"] {
            collect_tips(&repo, name, &mut tips)?;
        }
        tips
    } else {
        let revs: Vec<&String> = args.iter().filter(|a| !a.starts_with('-')).collect();
        if revs.is_empty() {
            vec![resolve_rev(&repo, "HEAD")?]
        } else {
            revs.iter()
                .map(|r| resolve_rev(&repo, r))
                .collect::<CliResult<Vec<_>>>()?
        }
    };
    let mut seen = HashSet::new();
    let mut mark: u32 = 1;
    let mut marks: BTreeMap<String, u32> = BTreeMap::new();
    for tip in tips {
        export_commit_walk(&repo, tip, &mut seen, &mut mark, &mut marks)?;
    }
    println!("done");
    Ok(())
}

fn collect_tips(repo: &arks_repo::Repository, which: &str, out: &mut Vec<Oid>) -> CliResult<()> {
    if which == "HEAD" {
        if let Ok(oid) = repo.head_oid() {
            out.push(oid);
        }
        return Ok(());
    }
    let dir = repo.git_dir().join(which);
    if !dir.is_dir() {
        return Ok(());
    }
    walk_ref_dir(&dir, which, repo, out)?;
    Ok(())
}

fn walk_ref_dir(
    dir: &Path,
    prefix: &str,
    repo: &arks_repo::Repository,
    out: &mut Vec<Oid>,
) -> CliResult<()> {
    for ent in fs::read_dir(dir)? {
        let ent = ent?;
        let name = ent.file_name().to_string_lossy().into_owned();
        let path = ent.path();
        if path.is_dir() {
            walk_ref_dir(&path, &format!("{prefix}/{name}"), repo, out)?;
        } else if let Ok(oid) = repo.refs().resolve(&format!("{prefix}/{name}")) {
            out.push(oid);
        }
    }
    Ok(())
}

fn export_commit_walk(
    repo: &arks_repo::Repository,
    oid: Oid,
    seen: &mut HashSet<String>,
    mark: &mut u32,
    marks: &mut BTreeMap<String, u32>,
) -> CliResult<()> {
    let hex = oid.to_hex();
    if !seen.insert(hex.clone()) {
        return Ok(());
    }
    let obj = repo.get_object(&oid)?;
    if obj.kind != ObjectKind::Commit {
        return Ok(());
    }
    let c = Commit::decode(&obj.payload)?;
    for p in &c.parents {
        export_commit_walk(repo, *p, seen, mark, marks)?;
    }
    // Export tree blobs (non-recursive depth limited by flatten).
    let mut files: Vec<(String, Oid)> = Vec::new();
    arks_repo::flatten_tree_entries(repo, &c.tree, "", &mut files)
        .map_err(|e| CliError::Msg(e.to_string()))?;
    for (_path, blob) in &files {
        let bhex = blob.to_hex();
        if marks.contains_key(&bhex) {
            continue;
        }
        let blob_obj = repo.get_object(blob)?;
        if blob_obj.kind != ObjectKind::Blob {
            continue;
        }
        let m = *mark;
        *mark += 1;
        marks.insert(bhex.clone(), m);
        println!("blob");
        println!("mark :{m}");
        println!("data {}", blob_obj.payload.len());
        io::stdout().write_all(&blob_obj.payload)?;
        println!();
    }
    let m = *mark;
    *mark += 1;
    marks.insert(hex.clone(), m);
    println!("commit refs/arks/export/{hex}");
    println!("mark :{m}");
    let msg = c.message.as_bytes();
    println!("data {}", msg.len());
    io::stdout().write_all(msg)?;
    if !msg.ends_with(b"\n") {
        println!();
    }
    if let Some(p) = c.parents.first() {
        if let Some(pm) = marks.get(&p.to_hex()) {
            println!("from :{pm}");
        }
    }
    for (path, blob) in &files {
        if let Some(bm) = marks.get(&blob.to_hex()) {
            println!("M 100644 :{bm} {path}");
        }
    }
    println!();
    Ok(())
}

/// `arks fast-import` — consume a simplified fast-import stream (subset).
pub fn cmd_fast_import(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &["--quiet", "--date-format"])?;
    let mut repo = open_repo()?;
    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf)?;
    let mut marks: BTreeMap<u32, Oid> = BTreeMap::new();
    let mut lines = buf.lines().peekable();
    while let Some(line) = lines.next() {
        let line = line.trim_end();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line == "done" {
            break;
        }
        if line == "blob" {
            let mark = parse_mark_line(lines.next())?;
            let (n, data) = parse_data_block(&mut lines)?;
            if data.len() != n {
                return Err(CliError::Msg("fast-import: blob data length mismatch".into()));
            }
            let format = repo.object_format();
            let obj = RawObject::new(ObjectKind::Blob, data);
            let oid = repo
                .store_mut()
                .put(format, &obj)
                .map_err(|e| CliError::Msg(e.to_string()))?;
            marks.insert(mark, oid);
            continue;
        }
        if let Some(rest) = line.strip_prefix("commit ") {
            let refname = rest.trim().to_string();
            let mark = parse_mark_line(lines.next())?;
            while let Some(peek) = lines.peek() {
                let p = peek.trim();
                if p.starts_with("author ") || p.starts_with("committer ") {
                    lines.next();
                    continue;
                }
                break;
            }
            let (_n, msg) = parse_data_block(&mut lines)?;
            let mut parent: Option<Oid> = None;
            let mut entries: Vec<(String, Oid, u32)> = Vec::new();
            while let Some(peek) = lines.peek() {
                let p = peek.trim();
                if let Some(rest) = p.strip_prefix("from :") {
                    let m: u32 = rest
                        .parse()
                        .map_err(|_| CliError::Msg("fast-import: bad from mark".into()))?;
                    parent = Some(
                        *marks
                            .get(&m)
                            .ok_or_else(|| CliError::Msg(format!("fast-import: unknown mark :{m}")))?,
                    );
                    lines.next();
                } else if let Some(rest) = p.strip_prefix("M ") {
                    // M <mode> :<mark> <path>
                    let mut it = rest.splitn(3, ' ');
                    let mode_s = it.next().unwrap_or("100644");
                    let mark_tok = it.next().unwrap_or("");
                    let path = it.next().unwrap_or("").to_string();
                    let mode = u32::from_str_radix(mode_s, 8).unwrap_or(0o100644);
                    let m: u32 = mark_tok
                        .trim_start_matches(':')
                        .parse()
                        .map_err(|_| CliError::Msg("fast-import: bad M mark".into()))?;
                    let oid = *marks
                        .get(&m)
                        .ok_or_else(|| CliError::Msg(format!("fast-import: unknown mark :{m}")))?;
                    entries.push((path, oid, mode));
                    lines.next();
                } else {
                    break;
                }
            }
            let tree_oid = write_flat_tree(&mut repo, &entries)?;
            let mut parents = Vec::new();
            if let Some(p) = parent {
                parents.push(p);
            }
            let sig = Signature {
                name: "fast-import".into(),
                email: "arks@local".into(),
                timestamp: 0,
                timezone: "+0000".into(),
            };
            let commit = Commit {
                tree: tree_oid,
                parents,
                author: sig.clone(),
                committer: sig,
                gpgsig: None,
                message: String::from_utf8_lossy(&msg).into_owned(),
            };
            let payload = commit
                .encode()
                .map_err(|e| CliError::Msg(e.to_string()))?;
            let format = repo.object_format();
            let obj = RawObject::new(ObjectKind::Commit, payload);
            let oid = repo
                .store_mut()
                .put(format, &obj)
                .map_err(|e| CliError::Msg(e.to_string()))?;
            marks.insert(mark, oid);
            let _ = refname;
            continue;
        }
        return Err(CliError::NotImplemented(format!(
            "fast-import: command not claimed in subset: {line}"
        )));
    }
    Ok(())
}

fn parse_mark_line(line: Option<&str>) -> CliResult<u32> {
    let line = line.ok_or_else(|| CliError::Msg("fast-import: expected mark".into()))?;
    let rest = line
        .trim()
        .strip_prefix("mark :")
        .ok_or_else(|| CliError::Msg("fast-import: expected 'mark :N'".into()))?;
    rest.parse()
        .map_err(|_| CliError::Msg("fast-import: bad mark number".into()))
}

fn parse_data_block<'a>(
    lines: &mut std::iter::Peekable<impl Iterator<Item = &'a str>>,
) -> CliResult<(usize, Vec<u8>)> {
    let header = lines
        .next()
        .ok_or_else(|| CliError::Msg("fast-import: expected data".into()))?;
    let n: usize = header
        .trim()
        .strip_prefix("data ")
        .ok_or_else(|| CliError::Msg("fast-import: expected 'data <n>'".into()))?
        .parse()
        .map_err(|_| CliError::Msg("fast-import: bad data length".into()))?;
    // Collect exactly n bytes from subsequent content — line-oriented subset for text.
    let mut data = Vec::new();
    while data.len() < n {
        let Some(line) = lines.next() else {
            break;
        };
        if !data.is_empty() {
            data.push(b'\n');
        }
        data.extend_from_slice(line.as_bytes());
        if data.len() >= n {
            data.truncate(n);
            break;
        }
    }
    Ok((n, data))
}

fn write_flat_tree(
    repo: &mut arks_repo::Repository,
    entries: &[(String, Oid, u32)],
) -> CliResult<Oid> {
    let mut tree_entries = Vec::new();
    for (path, oid, mode) in entries {
        let name = Path::new(path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(path)
            .to_string();
        let tm = if *mode == 0o100755 {
            TreeMode::Executable
        } else {
            TreeMode::File
        };
        tree_entries.push(TreeEntry {
            mode: tm,
            name,
            oid: *oid,
        });
    }
    let tree = Tree::new(tree_entries).map_err(|e| CliError::Msg(e.to_string()))?;
    let payload = tree.encode().map_err(|e| CliError::Msg(e.to_string()))?;
    let format = repo.object_format();
    let obj = RawObject::new(ObjectKind::Tree, payload);
    repo.store_mut()
        .put(format, &obj)
        .map_err(|e| CliError::Msg(e.to_string()))
}

/// `arks fetch-pack <url> [<ref>…]` — pack-protocol fetch plumbing (subset).
pub fn cmd_fetch_pack(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &["--depth", "--all", "-k", "--keep", "-q", "--quiet"])?;
    let positional: Vec<&String> = args.iter().filter(|a| !a.starts_with('-')).collect();
    let url = positional
        .first()
        .ok_or_else(|| CliError::Usage("fetch-pack <url> [<ref>…]".into()))?;
    let mut depth = None;
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--depth" {
            i += 1;
            depth = Some(
                args.get(i)
                    .ok_or_else(|| CliError::Usage("fetch-pack --depth <n>".into()))?
                    .parse()
                    .map_err(|_| CliError::Usage("fetch-pack --depth <n>".into()))?,
            );
        } else if let Some(rest) = args[i].strip_prefix("--depth=") {
            depth = Some(
                rest.parse()
                    .map_err(|_| CliError::Usage("fetch-pack --depth=<n>".into()))?,
            );
        }
        i += 1;
    }
    let mut repo = open_repo()?;
    let opts = arks_remote::FetchOptions {
        depth,
        ..Default::default()
    };
    let out = arks_remote::fetch_remote_with(&mut repo, url, "fetch-pack", &opts)
        .map_err(|e| CliError::Msg(e.to_string()))?;
    for (name, oid) in out.updated {
        println!("{} {}", oid.to_hex(), name);
    }
    Ok(())
}

/// `arks send-pack <url> [<ref>]` — pack-protocol push plumbing (subset).
pub fn cmd_send_pack(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &["--all", "--force", "-v", "--verbose"])?;
    let positional: Vec<&String> = args.iter().filter(|a| !a.starts_with('-')).collect();
    let url = positional
        .first()
        .ok_or_else(|| CliError::Usage("send-pack <url> [<ref>]".into()))?;
    let refspec = positional.get(1).map(|s| s.as_str());
    let repo = open_repo()?;
    let (refname, oid) = match refspec {
        Some(rs) => {
            let full = if rs.starts_with("refs/") {
                rs.to_string()
            } else {
                format!("refs/heads/{rs}")
            };
            let oid = resolve_rev(&repo, rs)?;
            (full, oid)
        }
        None => {
            let oid = repo.head_oid()?;
            ("refs/heads/master".into(), oid)
        }
    };
    let updates = [arks_remote::PushUpdate {
        refname: refname.clone(),
        new_oid: oid,
        expected_old: None,
    }];
    let opts = arks_remote::PushOptions::default();
    arks_remote::push_remote_with(&repo, url, &updates, &opts)
        .map_err(|e| CliError::Msg(e.to_string()))?;
    println!("ok {refname}");
    Ok(())
}

/// `arks http-fetch <commit-id> <url>` — legacy dumb/smart HTTP fetch (subset).
pub fn cmd_http_fetch(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &["-c", "-t", "-a", "-w", "--recover"])?;
    let positional: Vec<&String> = args.iter().filter(|a| !a.starts_with('-')).collect();
    if positional.len() < 2 {
        return Err(CliError::Usage(
            "http-fetch <commit-id> <url>".into(),
        ));
    }
    let _want = &positional[0];
    let url = positional[1].as_str();
    let mut repo = open_repo()?;
    let out = arks_remote::fetch_http_with(&mut repo, url, "http-fetch", &Default::default())
        .map_err(|e| CliError::Msg(e.to_string()))?;
    for (name, oid) in out.updated {
        println!("{} {}", oid.to_hex(), name);
    }
    Ok(())
}

/// `arks http-push <remote-url> [<local-ref>]` — legacy HTTP push (subset).
pub fn cmd_http_push(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &["--all", "--force", "-v", "--verbose", "-d", "--dry-run"])?;
    let positional: Vec<&String> = args.iter().filter(|a| !a.starts_with('-')).collect();
    let url = positional
        .first()
        .ok_or_else(|| CliError::Usage("http-push <remote-url> [<ref>]".into()))?;
    let refspec = positional.get(1).map(|s| s.as_str()).unwrap_or("HEAD");
    let repo = open_repo()?;
    let oid = resolve_rev(&repo, refspec)?;
    let refname = if refspec.starts_with("refs/") {
        refspec.to_string()
    } else if refspec == "HEAD" {
        "refs/heads/master".into()
    } else {
        format!("refs/heads/{refspec}")
    };
    let updates = [arks_remote::PushUpdate {
        refname: refname.clone(),
        new_oid: oid,
        expected_old: None,
    }];
    arks_remote::push_http_with(&repo, url, &updates, &Default::default())
        .map_err(|e| CliError::Msg(e.to_string()))?;
    println!("ok {refname}");
    Ok(())
}
