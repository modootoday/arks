//! Remaining pin Missing cmds → Specfold-honest Partial (or refuse-with-reason).
//!
//! Cite: [[sot:root/cli-git-help-matrix]]; never spawn `git`/`gh`.

use crate::cmd::shared::resolve_rev;
use crate::error::{CliError, CliResult};
use crate::util::{open_repo, refuse_unknown_flags};
use arks_object::{Commit, ObjectKind, RawObject, Signature, Tree, TreeEntry, TreeMode};
use arks_oid::Oid;
use arks_store::ObjectStore;
use std::collections::BTreeMap;
use std::io::{self, Read};
use std::path::Path;

/// `commit-graph`: see [`crate::cmd::commit_graph`].
pub fn cmd_commit_graph(args: &[String]) -> CliResult<()> {
    crate::cmd::commit_graph::cmd_commit_graph(args)
}

/// `multi-pack-index`: see [`crate::cmd::midx`].
pub fn cmd_multi_pack_index(args: &[String]) -> CliResult<()> {
    crate::cmd::midx::cmd_multi_pack_index(args)
}

/// `rerere`: see [`crate::cmd::rerere`].
pub fn cmd_rerere(args: &[String]) -> CliResult<()> {
    crate::cmd::rerere::cmd_rerere(args)
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

/// `fast-export`: see [`crate::cmd::fast_export`].
pub fn cmd_fast_export(args: &[String]) -> CliResult<()> {
    crate::cmd::fast_export::cmd_fast_export(args)
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
                return Err(CliError::Msg(
                    "fast-import: blob data length mismatch".into(),
                ));
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
                    parent =
                        Some(*marks.get(&m).ok_or_else(|| {
                            CliError::Msg(format!("fast-import: unknown mark :{m}"))
                        })?);
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
            let payload = commit.encode().map_err(|e| CliError::Msg(e.to_string()))?;
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

/// `arks http-fetch <commit-id> <url>` — legacy dumb/smart HTTP fetch (subset).
pub fn cmd_http_fetch(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &["-c", "-t", "-a", "-w", "--recover"])?;
    let positional: Vec<&String> = args.iter().filter(|a| !a.starts_with('-')).collect();
    if positional.len() < 2 {
        return Err(CliError::Usage("http-fetch <commit-id> <url>".into()));
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
    refuse_unknown_flags(
        args,
        &["--all", "--force", "-v", "--verbose", "-d", "--dry-run"],
    )?;
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
