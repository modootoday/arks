//! Plumbing diffs: `diff-tree`, `diff-index`, `diff-files` in git's raw
//! format (`:<mode> <mode> <oid> <oid> <status>\t<path>`) plus
//! `--name-only`, `--name-status`, `--stat`, `--numstat`, `-p`, `-s`,
//! `--quiet`/`--exit-code` and pathspecs.
//!
//! As in git, `diff-files` and `diff-index` (without `--cached`) do not
//! refresh the index: a work-tree file whose stat data differs from its
//! index entry is reported with the all-zero object id instead of being
//! hashed.
//!
//! Primary: <https://git-scm.com/docs/git-diff-tree>,
//! <https://git-scm.com/docs/git-diff-index>, <https://git-scm.com/docs/git-diff-files>,
//! <https://git-scm.com/docs/diff-format>.
//!
//! Partial: rename/copy detection, combined (`-c`/`--cc`) merge diffs and
//! `--stdin` are not claimed.

use crate::cmd::diff_render::{Change, DiffFormat, render};
use crate::cmd::log_format::path_in_specs;
use crate::cmd::shared::{ObjKind, peel_to_kind, resolve_rev};
use crate::error::{CliError, CliResult};
use crate::util::{open_repo, read_index, work_tree};
use arks_object::{Tree, TreeMode};
use arks_oid::Oid;
use arks_repo::Repository;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

/// One side of a raw diff entry.
#[derive(Clone, Copy, PartialEq, Eq)]
struct Side {
    mode: u32,
    /// `None` = the all-zero id (work-tree content not hashed).
    oid: Option<Oid>,
}

struct Entry {
    path: String,
    old: Option<Side>,
    new: Option<Side>,
}

#[derive(Default)]
struct Opts {
    recursive: bool,
    show_trees: bool,
    root: bool,
    no_commit_id: bool,
    cached: bool,
    quiet: bool,
    exit_code: bool,
    fmt: DiffFormat,
    raw: bool,
    revs: Vec<String>,
    specs: Vec<String>,
}

fn parse(args: &[String]) -> CliResult<Opts> {
    let mut o = Opts::default();
    let mut paths_only = false;
    for a in args {
        if paths_only {
            o.specs.push(a.clone());
            continue;
        }
        if o.fmt.take_flag(a)? {
            continue;
        }
        match a.as_str() {
            "--" => paths_only = true,
            "-r" => o.recursive = true,
            "-t" => {
                o.recursive = true;
                o.show_trees = true;
            }
            "--root" => o.root = true,
            "--no-commit-id" => o.no_commit_id = true,
            "--cached" | "--staged" => o.cached = true,
            "--quiet" => o.quiet = true,
            "--exit-code" => o.exit_code = true,
            "--raw" => o.raw = true,
            "-m" | "--no-renames" | "--full-index" => {}
            _ if a.starts_with('-') => {
                return Err(CliError::NotImplemented(format!("unsupported option: {a}")));
            }
            _ => o.revs.push(a.clone()),
        }
    }
    Ok(o)
}

fn tree_mode(m: TreeMode) -> u32 {
    match m {
        TreeMode::Tree => 0o040000,
        TreeMode::Executable => 0o100755,
        TreeMode::Symlink => 0o120000,
        TreeMode::Gitlink => 0o160000,
        TreeMode::File => 0o100644,
    }
}

fn tree_entries(repo: &Repository, tree: Option<&Oid>) -> CliResult<BTreeMap<String, Side>> {
    let mut out = BTreeMap::new();
    if let Some(t) = tree {
        let obj = repo.get_object(t)?;
        for e in Tree::decode(&obj.payload, repo.oid_kind())?.entries {
            out.insert(
                e.name.clone(),
                Side {
                    mode: tree_mode(e.mode),
                    oid: Some(e.oid),
                },
            );
        }
    }
    Ok(out)
}

/// Tree-to-tree diff, descending into changed subtrees when `recursive`.
fn diff_trees(
    repo: &Repository,
    a: Option<&Oid>,
    b: Option<&Oid>,
    prefix: &str,
    o: &Opts,
    out: &mut Vec<Entry>,
) -> CliResult<()> {
    let (ea, eb) = (tree_entries(repo, a)?, tree_entries(repo, b)?);
    let mut names: Vec<&String> = ea.keys().chain(eb.keys()).collect();
    names.sort();
    names.dedup();
    for name in names {
        let (x, y) = (ea.get(name).copied(), eb.get(name).copied());
        if x == y {
            continue;
        }
        let path = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}/{name}")
        };
        let is_tree = |s: Option<Side>| s.is_some_and(|s| s.mode == 0o040000);
        if o.recursive && (is_tree(x) || is_tree(y)) {
            if o.show_trees && is_tree(x) && is_tree(y) {
                out.push(Entry {
                    path: path.clone(),
                    old: x,
                    new: y,
                });
            }
            let sub = |s: Option<Side>| s.filter(|s| s.mode == 0o040000).and_then(|s| s.oid);
            // A tree replaced by a blob (or the reverse) is a delete plus an add.
            if !is_tree(x) && x.is_some() {
                out.push(Entry {
                    path: path.clone(),
                    old: x,
                    new: None,
                });
            }
            diff_trees(repo, sub(x).as_ref(), sub(y).as_ref(), &path, o, out)?;
            if !is_tree(y) && y.is_some() {
                out.push(Entry {
                    path,
                    old: None,
                    new: y,
                });
            }
            continue;
        }
        out.push(Entry {
            path,
            old: x,
            new: y,
        });
    }
    Ok(())
}

/// git `ie_match_stat` (core.checkStat=default): size, mtime, ctime,
/// inode and owner must all match the index entry.
fn stat_clean(e: &arks_index::IndexEntry, meta: &fs::Metadata) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        e.size == meta.len().min(u32::MAX as u64) as u32
            && e.mtime_sec == meta.mtime() as u32
            && e.mtime_nsec == meta.mtime_nsec() as u32
            && e.ctime_sec == meta.ctime() as u32
            && e.ctime_nsec == meta.ctime_nsec() as u32
            && e.ino == meta.ino() as u32
            && e.uid == meta.uid()
            && e.gid == meta.gid()
    }
    #[cfg(not(unix))]
    {
        crate::cmd::shared::index_stat_matches(e, meta)
    }
}

/// Work-tree side for an index entry: `None` when the file is gone, the
/// index side when stat-clean, else the zero id with the file's mode.
fn worktree_side(work: &Path, e: &arks_index::IndexEntry) -> Option<Side> {
    let meta = fs::symlink_metadata(work.join(&e.path)).ok()?;
    if stat_clean(e, &meta) {
        return Some(Side {
            mode: e.mode,
            oid: Some(e.oid),
        });
    }
    let mode = if meta.file_type().is_symlink() {
        0o120000
    } else if e.mode == 0o160000 {
        0o160000
    } else {
        crate::cmd::shared::file_mode(&meta)
    };
    Some(Side { mode, oid: None })
}

fn index_sides(repo: &Repository) -> CliResult<BTreeMap<String, arks_index::IndexEntry>> {
    Ok(read_index(repo)?
        .entries
        .into_iter()
        .filter(|e| e.stage() == 0)
        .map(|e| (e.path.clone(), e))
        .collect())
}

fn flat_tree(repo: &Repository, tree: &Oid) -> CliResult<BTreeMap<String, Side>> {
    let mut v = Vec::new();
    crate::cmd::plumbing::flatten_tree_to_index(repo, tree, "", &mut v)?;
    Ok(v.into_iter()
        .map(|e| {
            (
                e.path,
                Side {
                    mode: e.mode,
                    oid: Some(e.oid),
                },
            )
        })
        .collect())
}

fn zero_hex(repo: &Repository) -> String {
    "0".repeat(repo.oid_kind().hex_len())
}

/// Print `entries` in the selected format; returns whether anything differed.
fn emit(repo: &Repository, o: &Opts, entries: &[Entry], work: Option<&Path>) -> CliResult<bool> {
    let entries: Vec<&Entry> = entries
        .iter()
        .filter(|e| o.specs.is_empty() || path_in_specs(&e.path, &o.specs))
        .collect();
    if o.quiet {
        return Ok(!entries.is_empty());
    }
    let zero = zero_hex(repo);
    let hex = |s: Option<Side>| s.and_then(|s| s.oid).map_or(zero.clone(), |x| x.to_hex());
    let mode = |s: Option<Side>| s.map_or(0, |s| s.mode);
    let status = |e: &Entry| match (e.old, e.new) {
        (None, _) => 'A',
        (_, None) => 'D',
        (Some(a), Some(b)) if a.mode & 0o170000 != b.mode & 0o170000 => 'T',
        _ => 'M',
    };
    let mut out = std::io::stdout().lock();
    use std::io::Write;
    let fmt = &o.fmt;
    let wants_render = fmt.patch || fmt.stat || fmt.numstat || fmt.shortstat;
    if fmt.no_output {
        return Ok(!entries.is_empty());
    }
    if fmt.name_only || fmt.name_status {
        for e in &entries {
            if fmt.name_status {
                writeln!(out, "{}\t{}", status(e), e.path)?;
            } else {
                writeln!(out, "{}", e.path)?;
            }
        }
    } else if o.raw || !wants_render {
        for e in &entries {
            writeln!(
                out,
                ":{:06o} {:06o} {} {} {}\t{}",
                mode(e.old),
                mode(e.new),
                hex(e.old),
                hex(e.new),
                status(e),
                e.path
            )?;
        }
    }
    if wants_render {
        let changes: Vec<Change> = entries
            .iter()
            .filter(|e| {
                e.old.is_some_and(|s| s.mode != 0o040000)
                    || e.new.is_some_and(|s| s.mode != 0o040000)
            })
            .map(|e| Change {
                path: e.path.clone(),
                old: e.old.and_then(|s| s.oid),
                // Work-tree content: a placeholder id plus the file to read.
                new: e.new.map(|s| {
                    s.oid.unwrap_or_else(|| {
                        e.old
                            .and_then(|o| o.oid)
                            .unwrap_or(Oid::from_hex(&zero).expect("zero oid"))
                    })
                }),
                work: match (e.new, work) {
                    (Some(Side { oid: None, .. }), Some(w)) => Some(w.join(&e.path)),
                    _ => None,
                },
            })
            .collect();
        // Patch/stat output reads the content: a stat-dirty file whose bytes
        // still hash to the index blob has no diff.
        let changes: Vec<Change> = changes
            .into_iter()
            .filter(|c| match (&c.work, &c.old) {
                (Some(p), Some(old)) => fs::read(p)
                    .ok()
                    .and_then(|b| {
                        arks_object::RawObject::new(arks_object::ObjectKind::Blob, b)
                            .oid(repo.object_format())
                            .ok()
                    })
                    .is_none_or(|h| h != *old),
                _ => true,
            })
            .collect();
        let mut f = fmt.clone();
        f.name_only = false;
        f.name_status = false;
        drop(out);
        let mut out = std::io::stdout().lock();
        render(&mut out, repo, &f, &changes)?;
    }
    Ok(!entries.is_empty())
}

fn finish(o: &Opts, differs: bool) -> CliResult<()> {
    if (o.quiet || o.exit_code) && differs {
        return Err(CliError::silent(1));
    }
    Ok(())
}

pub fn cmd_diff_tree(args: &[String]) -> CliResult<()> {
    let o = parse(args)?;
    let repo = open_repo()?;
    let tree_of = |r: &str| -> CliResult<Oid> {
        let oid = resolve_rev(&repo, r)
            .map_err(|_| CliError::fatal(format!("ambiguous argument '{r}': unknown revision")))?;
        peel_to_kind(&repo, &oid, ObjKind::Tree, r)
    };
    let mut entries = Vec::new();
    match o.revs.as_slice() {
        [one] => {
            // One commit: diff against its first parent, after a commit-id line.
            let oid = resolve_rev(&repo, one)?;
            let commit = peel_to_kind(&repo, &oid, ObjKind::Commit, one)?;
            let c = crate::cmd::log_format::read_commit(&repo, &commit)?;
            let parent = match c.parents.first() {
                Some(p) => Some(crate::cmd::log_format::read_commit(&repo, p)?.tree),
                None if o.root => None,
                None => return Ok(()),
            };
            diff_trees(&repo, parent.as_ref(), Some(&c.tree), "", &o, &mut entries)?;
            if !o.no_commit_id && (!entries.is_empty() || o.fmt.no_output) && !o.quiet {
                println!("{}", commit.to_hex());
            }
        }
        [a, b, ..] => {
            let (ta, tb) = (tree_of(a)?, tree_of(b)?);
            let mut specs = o.revs[2..].to_vec();
            specs.extend(o.specs.iter().cloned());
            let o2 = Opts {
                specs,
                ..parse(args)?
            };
            diff_trees(&repo, Some(&ta), Some(&tb), "", &o2, &mut entries)?;
            let d = emit(&repo, &o2, &entries, None)?;
            return finish(&o2, d);
        }
        [] => {
            return Err(CliError::Usage(
                "diff-tree <tree-ish> [<tree-ish>] [<path>...]".into(),
            ));
        }
    }
    let d = emit(&repo, &o, &entries, None)?;
    finish(&o, d)
}

pub fn cmd_diff_index(args: &[String]) -> CliResult<()> {
    let mut o = parse(args)?;
    let repo = open_repo()?;
    let rev =
        o.revs.first().cloned().ok_or_else(|| {
            CliError::Usage("diff-index [--cached] <tree-ish> [<path>...]".into())
        })?;
    o.specs.extend(o.revs.drain(1..));
    let oid = resolve_rev(&repo, &rev)
        .map_err(|_| CliError::fatal(format!("ambiguous argument '{rev}': unknown revision")))?;
    let tree = flat_tree(&repo, &peel_to_kind(&repo, &oid, ObjKind::Tree, &rev)?)?;
    let index = index_sides(&repo)?;
    let work = if o.cached {
        None
    } else {
        Some(work_tree(&repo)?)
    };
    let mut paths: Vec<&String> = tree.keys().chain(index.keys()).collect();
    paths.sort();
    paths.dedup();
    let mut entries = Vec::new();
    for p in paths {
        let t = tree.get(p).copied();
        let new = match (index.get(p), &work) {
            (None, _) => None,
            (Some(e), None) => Some(Side {
                mode: e.mode,
                oid: Some(e.oid),
            }),
            (Some(e), Some(w)) => worktree_side(w, e),
        };
        if t != new {
            entries.push(Entry {
                path: p.clone(),
                old: t,
                new,
            });
        }
    }
    let d = emit(&repo, &o, &entries, work.as_deref())?;
    finish(&o, d)
}

pub fn cmd_diff_files(args: &[String]) -> CliResult<()> {
    let mut o = parse(args)?;
    if o.cached {
        return Err(CliError::Usage(
            "diff-files: use diff-index --cached for index vs tree".into(),
        ));
    }
    let specs: Vec<String> = std::mem::take(&mut o.revs);
    o.specs.extend(specs);
    let repo = open_repo()?;
    let work = work_tree(&repo)?;
    let mut entries = Vec::new();
    for (p, e) in index_sides(&repo)? {
        let old = Some(Side {
            mode: e.mode,
            oid: Some(e.oid),
        });
        let new = worktree_side(&work, &e);
        if new != old {
            entries.push(Entry { path: p, old, new });
        }
    }
    let d = emit(&repo, &o, &entries, Some(&work))?;
    finish(&o, d)
}
