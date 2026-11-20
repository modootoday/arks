//! `merge-tree` — merge without touching the index or the work tree.
//!
//! `--write-tree` (default for two arguments) prints the result tree, the
//! conflicted file info and the informational messages (`-z`, `--name-only`,
//! `--[no-]messages`, `--stdin`, `--merge-base`, `-X`,
//! `--allow-unrelated-histories`); `--trivial-merge` (default for three
//! arguments) prints the deprecated per-path merge report.
//! Cite: <https://git-scm.com/docs/git-merge-tree> (behaviour and output
//! format; observed stock git output for the message texts).

use crate::cmd::merge_engine::{Favor, MergeLabels, merge_text};
use crate::cmd::merge_strategies::RecursiveOpts;
use crate::cmd::opt_parse::{Mode, OptDef, Takes::*, o, parse};
use crate::cmd::shared::{ObjKind, peel_tags, peel_to_kind, resolve_rev};
use crate::cmd::unified_diff::{HunkStyle, write_hunks};
use crate::error::{CliError, CliResult};
use crate::util::open_repo;
use crate::write_tree::write_tree_from_index;
use arks_index::{IndexEntry, sort_entries};
use arks_object::{Commit, ObjectKind, RawObject, TreeMode};
use arks_oid::Oid;
use arks_repo::Repository;
use arks_store::ObjectStore;
use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, BufRead, Write};

const USAGE: &str = "usage: git merge-tree [--write-tree] [<options>] <branch1> <branch2>
   or: git merge-tree [--trivial-merge] <base-tree> <branch1> <branch2>

    --write-tree          do a real merge instead of a trivial merge
    --trivial-merge       do a trivial merge only
    --[no-]messages       also show informational/conflict messages
    -z                    separate paths with the NUL character
    --name-only           list filenames without modes/oids/stages
    --allow-unrelated-histories
                          allow merging unrelated histories
    --stdin               perform multiple merges, one per line of input
    --[no-]merge-base <commit>
                          specify a merge-base for the merge
    -X, --[no-]strategy-option <option=value>
                          option for selected merge strategy
";

const DEFS: &[OptDef] = &[
    o("write-tree", ' ', No, false),
    o("trivial-merge", ' ', No, false),
    o("messages", ' ', No, true),
    o("-z", 'z', No, false),
    o("name-only", ' ', No, false),
    o("allow-unrelated-histories", ' ', No, false),
    o("stdin", ' ', No, false),
    o("merge-base", ' ', Req, true),
    o("strategy-option", 'X', Req, true),
];

#[derive(Clone, Copy, PartialEq, Eq)]
struct Side {
    mode: u32,
    oid: Oid,
}

/// One informational or conflict message of a path.
struct Message {
    path: String,
    /// The stable short type printed with `-z`.
    kind: &'static str,
    text: String,
}

struct WriteTreeOpts {
    z: bool,
    name_only: bool,
    messages: Option<bool>,
    allow_unrelated: bool,
    recursive: RecursiveOpts,
}

fn usage() -> CliError {
    CliError::Exit(129, USAGE.to_string())
}

pub fn cmd_merge_tree(args: &[String]) -> CliResult<()> {
    let p = parse("merge-tree", args, DEFS, Mode::default())?;
    let mut mode: Option<&str> = None;
    for (key, _, _) in p.opts() {
        if matches!(key, "write-tree" | "trivial-merge") {
            match mode {
                Some(prev) if prev != key => {
                    return Err(CliError::Exit(
                        129,
                        format!("error: --{key} is incompatible with --{prev}"),
                    ));
                }
                _ => mode = Some(key),
            }
        }
    }
    let argv = p.args();
    let mode = mode.unwrap_or(if argv.len() == 3 && !p.on("stdin") {
        "trivial-merge"
    } else {
        "write-tree"
    });
    if mode == "trivial-merge" {
        let others = p.opts().any(|(k, _, _)| k != "trivial-merge");
        if others {
            return Err(CliError::fatal(
                "--trivial-merge is incompatible with all other options",
            ));
        }
        if argv.len() != 3 {
            return Err(usage());
        }
        return trivial_merge(&argv[0], &argv[1], &argv[2]);
    }
    let mut recursive = RecursiveOpts::default();
    for x in p.values("strategy-option") {
        if !recursive.apply(&x) {
            return Err(CliError::fatal(format!("unknown strategy option: -X{x}")));
        }
    }
    let opts = WriteTreeOpts {
        z: p.on("-z"),
        name_only: p.on("name-only"),
        messages: p.flag("messages"),
        allow_unrelated: p.on("allow-unrelated-histories"),
        recursive,
    };
    let merge_base = p.last("merge-base").and_then(|v| v);
    let mut repo = open_repo()?;
    if p.on("stdin") {
        let stdin = io::stdin();
        let mut out = io::stdout().lock();
        for line in stdin.lock().lines() {
            let line = line?;
            let words: Vec<&str> = line.split(' ').collect();
            let (base, b1, b2) = match words.as_slice() {
                [b1, b2] => (None, *b1, *b2),
                [base, "--", b1, b2] => (Some(*base), *b1, *b2),
                _ => {
                    return Err(CliError::fatal(format!("malformed input line: '{line}'.")));
                }
            };
            let mut buf = Vec::new();
            let clean = write_tree_merge(&mut repo, &opts, base, b1, b2, true, &mut buf)?;
            out.write_all(if clean { b"1\0" } else { b"0\0" })?;
            out.write_all(&buf)?;
            out.write_all(b"\0")?;
        }
        return Ok(());
    }
    if argv.len() != 2 {
        return Err(usage());
    }
    let mut buf = Vec::new();
    let clean = write_tree_merge(
        &mut repo,
        &opts,
        merge_base.as_deref(),
        &argv[0],
        &argv[1],
        false,
        &mut buf,
    )?;
    io::stdout().write_all(&buf)?;
    if clean {
        Ok(())
    } else {
        Err(CliError::silent(1))
    }
}

fn commit_tree(repo: &Repository, oid: &Oid) -> CliResult<Oid> {
    let obj = repo.get_object(oid)?;
    Ok(Commit::decode(&obj.payload)?.tree)
}

/// Resolve a branch argument to a commit, or fail like git's
/// "not something we can merge" (exit 1).
fn merge_commit(repo: &Repository, name: &str) -> CliResult<Oid> {
    let fail = |extra: String| {
        CliError::Exit(
            1,
            format!("{extra}merge-tree: {name} - not something we can merge"),
        )
    };
    let oid = resolve_rev(repo, name).map_err(|_| fail(String::new()))?;
    let peeled = peel_tags(repo, &oid).map_err(|_| fail(String::new()))?;
    let kind = repo
        .get_object(&peeled)
        .map_err(|_| fail(String::new()))?
        .kind;
    if kind != ObjectKind::Commit {
        return Err(fail(format!(
            "error: {name}: expected commit type, but the object dereferences to {} type\n",
            kind.as_str()
        )));
    }
    Ok(peeled)
}

fn lookup_commit(repo: &Repository, name: &str) -> CliResult<Oid> {
    let fatal = |extra: String| {
        CliError::Exit(
            128,
            format!("{extra}fatal: could not lookup commit '{name}'"),
        )
    };
    let oid = resolve_rev(repo, name).map_err(|_| fatal(String::new()))?;
    let peeled = peel_tags(repo, &oid).map_err(|_| fatal(String::new()))?;
    let kind = repo
        .get_object(&peeled)
        .map_err(|_| fatal(String::new()))?
        .kind;
    if kind != ObjectKind::Commit {
        return Err(fatal(format!(
            "error: object {} is a {}, not a commit\n",
            peeled.to_hex(),
            kind.as_str()
        )));
    }
    Ok(peeled)
}

fn empty_tree(repo: &mut Repository) -> CliResult<Oid> {
    let format = repo.object_format();
    Ok(repo
        .store_mut()
        .put(format, &RawObject::new(ObjectKind::Tree, Vec::new()))?)
}

/// Every non-tree entry below `tree`, keyed by full path.
fn tree_sides(repo: &Repository, tree: &Oid) -> CliResult<BTreeMap<String, Side>> {
    let mut out = BTreeMap::new();
    walk(repo, tree, "", &mut out)?;
    Ok(out)
}

fn walk(
    repo: &Repository,
    tree: &Oid,
    prefix: &str,
    out: &mut BTreeMap<String, Side>,
) -> CliResult<()> {
    let obj = repo.get_object(tree)?;
    let t = arks_object::Tree::decode(&obj.payload, repo.oid_kind())?;
    for e in t.entries {
        let path = if prefix.is_empty() {
            e.name.clone()
        } else {
            format!("{prefix}/{}", e.name)
        };
        let mode = match e.mode {
            TreeMode::Tree => {
                walk(repo, &e.oid, &path, out)?;
                continue;
            }
            TreeMode::Executable => 0o100755,
            TreeMode::Symlink => 0o120000,
            TreeMode::Gitlink => 0o160000,
            TreeMode::File => 0o100644,
        };
        out.insert(path, Side { mode, oid: e.oid });
    }
    Ok(())
}

fn blob(repo: &Repository, s: Option<&Side>) -> CliResult<Vec<u8>> {
    Ok(match s {
        Some(s) => repo.get_object(&s.oid)?.payload,
        None => Vec::new(),
    })
}

fn is_binary(b: &[u8]) -> bool {
    b[..b.len().min(8000)].contains(&0)
}

fn entry(path: &str, s: &Side, stage: u16) -> IndexEntry {
    IndexEntry {
        ctime_sec: 0,
        ctime_nsec: 0,
        mtime_sec: 0,
        mtime_nsec: 0,
        dev: 0,
        ino: 0,
        mode: s.mode,
        uid: 0,
        gid: 0,
        size: 0,
        oid: s.oid,
        flags: ((stage & 3) << 12) | (path.len().min(0x0fff) as u16),
        flags_extended: None,
        path: path.to_owned(),
    }
}

/// One `--write-tree` merge into `out`; returns whether it was clean.
fn write_tree_merge(
    repo: &mut Repository,
    opts: &WriteTreeOpts,
    merge_base: Option<&str>,
    b1: &str,
    b2: &str,
    stdin_mode: bool,
    out: &mut Vec<u8>,
) -> CliResult<bool> {
    let ours_c = merge_commit(repo, b1)?;
    let theirs_c = merge_commit(repo, b2)?;
    let base_tree = match merge_base {
        Some(name) => {
            let c = lookup_commit(repo, name)?;
            commit_tree(repo, &c)?
        }
        None => match arks_repo::merge_base_oid(repo, &ours_c, &theirs_c)? {
            Some(c) => commit_tree(repo, &c)?,
            None if opts.allow_unrelated => empty_tree(repo)?,
            None => return Err(CliError::fatal("refusing to merge unrelated histories")),
        },
    };
    let ours_tree = commit_tree(repo, &ours_c)?;
    let mut their_tree = commit_tree(repo, &theirs_c)?;
    let mut base_tree = base_tree;
    if let Some(prefix) = &opts.recursive.subtree {
        let (b, t) = crate::cmd::merge_subtree::shift_trees(
            repo,
            &ours_tree,
            &base_tree,
            &their_tree,
            prefix.as_deref(),
        )?;
        base_tree = b;
        their_tree = t;
    }
    let labels = MergeLabels {
        ours: b1.to_string(),
        base: "merged common ancestors".into(),
        theirs: b2.to_string(),
        diff3: false,
        favor: opts.recursive.favor,
    };
    let (tree, conflicted, messages) =
        merge_trees(repo, &base_tree, &ours_tree, &their_tree, &labels)?;
    let z = opts.z || stdin_mode;
    let term: &[u8] = if z { b"\0" } else { b"\n" };
    out.extend_from_slice(tree.to_hex().as_bytes());
    out.extend_from_slice(term);
    let clean = conflicted.is_empty();
    let mut last_path: Option<&str> = None;
    for e in &conflicted {
        if opts.name_only {
            if last_path == Some(e.path.as_str()) {
                continue;
            }
            last_path = Some(&e.path);
            out.extend_from_slice(e.path.as_bytes());
        } else {
            out.extend_from_slice(
                format!(
                    "{:06o} {} {}\t{}",
                    e.mode,
                    e.oid.to_hex(),
                    e.stage(),
                    e.path
                )
                .as_bytes(),
            );
        }
        out.extend_from_slice(term);
    }
    let show = opts.messages.unwrap_or(!clean);
    if show {
        out.extend_from_slice(term);
        for m in &messages {
            if z {
                out.extend_from_slice(
                    format!("1\0{}\0{}\0{}\n\0", m.path, m.kind, m.text).as_bytes(),
                );
            } else {
                out.extend_from_slice(m.text.as_bytes());
                out.push(b'\n');
            }
        }
    }
    Ok(clean)
}

/// Merge three trees: the result tree (conflicted paths carry marker
/// content or the surviving side), the conflicted index entries and the
/// per-path messages.
fn merge_trees(
    repo: &mut Repository,
    base: &Oid,
    ours: &Oid,
    theirs: &Oid,
    labels: &MergeLabels,
) -> CliResult<(Oid, Vec<IndexEntry>, Vec<Message>)> {
    let bs = tree_sides(repo, base)?;
    let os = tree_sides(repo, ours)?;
    let ts = tree_sides(repo, theirs)?;
    let paths: BTreeSet<&String> = bs.keys().chain(os.keys()).chain(ts.keys()).collect();
    let format = repo.object_format();
    let mut result: Vec<IndexEntry> = Vec::new();
    let mut conflicted: Vec<IndexEntry> = Vec::new();
    let mut messages: Vec<Message> = Vec::new();
    let (on, tn) = (labels.ours.as_str(), labels.theirs.as_str());
    for path in paths {
        let (b, o, t) = (bs.get(path), os.get(path), ts.get(path));
        let msg = |kind: &'static str, text: String| Message {
            path: path.clone(),
            kind,
            text,
        };
        match (b, o, t) {
            (_, Some(y), Some(z)) if y == z => result.push(entry(path, y, 0)),
            (Some(x), Some(y), _) if x == y => {
                if let Some(z) = t {
                    result.push(entry(path, z, 0));
                }
            }
            (Some(x), _, Some(z)) if x == z => {
                if let Some(y) = o {
                    result.push(entry(path, y, 0));
                }
            }
            (None, Some(y), None) => result.push(entry(path, y, 0)),
            (None, None, Some(z)) => result.push(entry(path, z, 0)),
            (Some(_), None, None) => {}
            (Some(x), Some(y), None) => {
                result.push(entry(path, y, 0));
                conflicted.push(entry(path, x, 1));
                conflicted.push(entry(path, y, 2));
                messages.push(msg(
                    "CONFLICT (modify/delete)",
                    format!(
                        "CONFLICT (modify/delete): {path} deleted in {tn} and modified in {on}.  Version {on} of {path} left in tree."
                    ),
                ));
            }
            (Some(x), None, Some(z)) => {
                result.push(entry(path, z, 0));
                conflicted.push(entry(path, x, 1));
                conflicted.push(entry(path, z, 3));
                messages.push(msg(
                    "CONFLICT (modify/delete)",
                    format!(
                        "CONFLICT (modify/delete): {path} deleted in {on} and modified in {tn}.  Version {tn} of {path} left in tree."
                    ),
                ));
            }
            (_, Some(y), Some(z)) => {
                // Both sides changed the path: mode from the side that changed it.
                let mode = match b {
                    Some(x) if x.mode == y.mode => z.mode,
                    _ => y.mode,
                };
                let regular = |m: u32| m & 0o170000 == 0o100000;
                let (bb, yb, zb) = (blob(repo, b)?, blob(repo, Some(y))?, blob(repo, Some(z))?);
                let content_kind = if b.is_some() {
                    "CONFLICT (content)"
                } else {
                    "CONFLICT (add/add)"
                };
                let push_conflict = |conflicted: &mut Vec<IndexEntry>| {
                    if let Some(x) = b {
                        conflicted.push(entry(path, x, 1));
                    }
                    conflicted.push(entry(path, y, 2));
                    conflicted.push(entry(path, z, 3));
                };
                if !(regular(y.mode) && regular(z.mode)) {
                    result.push(entry(path, y, 0));
                    push_conflict(&mut conflicted);
                    messages.push(msg(
                        "CONFLICT (contents)",
                        format!("{content_kind}: Merge conflict in {path}"),
                    ));
                    continue;
                }
                if y.oid == z.oid {
                    result.push(entry(path, &Side { mode, oid: y.oid }, 0));
                    continue;
                }
                if is_binary(&bb) || is_binary(&yb) || is_binary(&zb) {
                    let pick = match labels.favor {
                        Favor::Theirs => z,
                        _ => y,
                    };
                    result.push(entry(
                        path,
                        &Side {
                            mode,
                            oid: pick.oid,
                        },
                        0,
                    ));
                    if labels.favor == Favor::None {
                        messages.push(msg(
                            "CONFLICT (binary)",
                            format!("warning: Cannot merge binary files: {path} ({on} vs. {tn})"),
                        ));
                    }
                    messages.push(msg("Auto-merging", format!("Auto-merging {path}")));
                    if labels.favor == Favor::None {
                        push_conflict(&mut conflicted);
                        messages.push(msg(
                            "CONFLICT (contents)",
                            format!("{content_kind}: Merge conflict in {path}"),
                        ));
                    }
                    continue;
                }
                messages.push(msg("Auto-merging", format!("Auto-merging {path}")));
                let (merged, conflict) = merge_text(&bb, &yb, &zb, labels);
                let oid = repo
                    .store_mut()
                    .put(format, &RawObject::new(ObjectKind::Blob, merged))?;
                result.push(entry(path, &Side { mode, oid }, 0));
                if conflict {
                    push_conflict(&mut conflicted);
                    messages.push(msg(
                        "CONFLICT (contents)",
                        format!("{content_kind}: Merge conflict in {path}"),
                    ));
                }
            }
            (None, None, None) => {}
        }
    }
    sort_entries(&mut result);
    let tree = if result.is_empty() {
        empty_tree(repo)?
    } else {
        write_tree_from_index(repo.store_mut(), &result, format)?
    };
    Ok((tree, conflicted, messages))
}

/// Resolve a tree-ish argument of the trivial merge.
fn trivial_tree(repo: &Repository, name: &str) -> CliResult<Oid> {
    let oid =
        resolve_rev(repo, name).map_err(|_| CliError::fatal(format!("unknown rev {name}")))?;
    peel_to_kind(repo, &oid, ObjKind::Tree, name).map_err(|_| {
        let peeled = peel_tags(repo, &oid).unwrap_or(oid);
        CliError::fatal(format!("unable to read tree {}", peeled.to_hex()))
    })
}

/// The deprecated `--trivial-merge <base-tree> <branch1> <branch2>` report.
fn trivial_merge(base: &str, ours: &str, theirs: &str) -> CliResult<()> {
    let repo = open_repo()?;
    let bt = trivial_tree(&repo, base)?;
    let ot = trivial_tree(&repo, ours)?;
    let tt = trivial_tree(&repo, theirs)?;
    let bs = tree_sides(&repo, &bt)?;
    let os = tree_sides(&repo, &ot)?;
    let ts = tree_sides(&repo, &tt)?;
    let paths: BTreeSet<&String> = bs.keys().chain(os.keys()).chain(ts.keys()).collect();
    let mut out = io::stdout().lock();
    let style = HunkStyle {
        funcname: false,
        cut_nul: true,
    };
    let line = |out: &mut io::StdoutLock, label: &str, s: &Side, path: &str| -> CliResult<()> {
        writeln!(out, "  {label:<6} {:06o} {} {path}", s.mode, s.oid.to_hex())?;
        Ok(())
    };
    let labels = MergeLabels {
        ours: ".our".into(),
        base: String::new(),
        theirs: ".their".into(),
        diff3: false,
        favor: Favor::None,
    };
    for path in paths {
        let (b, o, t) = (bs.get(path), os.get(path), ts.get(path));
        match (b, o, t) {
            (_, Some(y), Some(z)) if y == z => {}
            (None, Some(_), None) | (Some(_), None, None) => {}
            (Some(x), None, Some(z)) => {
                if x != z {
                    writeln!(out, "removed in local")?;
                    line(&mut out, "base", x, path)?;
                    line(&mut out, "their", z, path)?;
                }
            }
            (Some(x), Some(y), None) => {
                writeln!(out, "removed in remote")?;
                line(&mut out, "base", x, path)?;
                line(&mut out, "our", y, path)?;
                write_hunks(&mut out, &blob(&repo, Some(y))?, b"", 3, style)?;
            }
            (None, None, Some(z)) => {
                writeln!(out, "added in remote")?;
                line(&mut out, "their", z, path)?;
                write_hunks(&mut out, b"", &blob(&repo, Some(z))?, 3, style)?;
            }
            (Some(x), Some(y), Some(z)) if x == y => {
                writeln!(out, "merged")?;
                line(&mut out, "result", z, path)?;
                line(&mut out, "our", y, path)?;
                if y.oid != z.oid {
                    write_hunks(
                        &mut out,
                        &blob(&repo, Some(y))?,
                        &blob(&repo, Some(z))?,
                        3,
                        style,
                    )?;
                }
            }
            (Some(x), Some(_), Some(z)) if x == z => {}
            (_, Some(y), Some(z)) => {
                if let Some(x) = b {
                    writeln!(out, "changed in both")?;
                    line(&mut out, "base", x, path)?;
                } else {
                    writeln!(out, "added in both")?;
                }
                line(&mut out, "our", y, path)?;
                line(&mut out, "their", z, path)?;
                let (bb, yb, zb) = (
                    blob(&repo, b)?,
                    blob(&repo, Some(y))?,
                    blob(&repo, Some(z))?,
                );
                if is_binary(&bb) || is_binary(&yb) || is_binary(&zb) {
                    eprintln!("warning: Cannot merge binary files: {path} (.our vs. .their)");
                    continue;
                }
                let (merged, _) = merge_text(&bb, &yb, &zb, &labels);
                write_hunks(&mut out, &yb, &merged, 3, style)?;
            }
            (None, None, None) => {}
        }
    }
    Ok(())
}
