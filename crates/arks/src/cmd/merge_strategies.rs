//! Merge strategy programs: `merge-octopus`, `merge-resolve`, `merge-ours`,
//! `merge-recursive` (and its `-ours` / `-theirs` names) and `merge-subtree`.
//!
//! Cite: <https://git-scm.com/docs/git-merge#_merge_strategies> (strategy
//! semantics) and the strategy-program calling convention
//! `<base>... -- <head> <remote>...` with exit 0 (merged), 1 (conflicts left
//! in the index and work tree) or 2 (strategy cannot handle the merge).
//! Behaviour and messages mirror stock git as observed (oracle only):
//! - `octopus` merges several remotes one by one (fast-forwarding while it
//!   can), refuses a staged index, and stops with exit 2 once a remote that
//!   is not the last one leaves conflicts.
//! - `resolve` handles exactly one remote with a trivial in-index merge plus
//!   `merge-one-file` for the rest.
//! - `ours` only checks that the index matches `HEAD`.
//! - `recursive` runs the three-way tree merge of `git merge`; called as
//!   `merge-recursive-ours` / `merge-recursive-theirs` it behaves like
//!   `merge-recursive` (as stock git does; pass `--ours` / `--theirs`), and
//!   as `merge-subtree` it shifts the remote tree first (see `merge_subtree`).
//!
//! The work tree and index change; nothing is committed (`git merge` does).

use crate::cmd::merge_engine::{Favor, MergeLabels, merge_trees_worktree};
use crate::cmd::merge_helpers::{
    Side, merge_all_one_file, tree_sides, trivial_merge, worktree_matches_index,
};
use crate::cmd::shared::{ObjKind, peel_to_kind, resolve_rev};
use crate::error::{CliError, CliResult};
use crate::util::{open_repo, read_index, work_tree};
use arks_oid::Oid;
use arks_repo::Repository;
use std::collections::{BTreeSet, HashSet};

/// `<base>... -- <head> <remote>...` split; `dashdash` = `--` was present.
struct StrategyArgs {
    bases: Vec<String>,
    dashdash: bool,
    head: Option<String>,
    remotes: Vec<String>,
}

fn split_args(args: &[String]) -> StrategyArgs {
    match args.iter().position(|a| a == "--") {
        Some(i) => StrategyArgs {
            bases: args[..i].to_vec(),
            dashdash: true,
            head: args.get(i + 1).cloned(),
            remotes: args
                .get(i + 2..)
                .map(<[String]>::to_vec)
                .unwrap_or_default(),
        },
        None => StrategyArgs {
            bases: args.to_vec(),
            dashdash: false,
            head: None,
            remotes: Vec::new(),
        },
    }
}

/// The name stock git shows for a merged commit: `GITHEAD_<oid>` from the
/// environment (set by `git merge`) or the argument as written.
pub(crate) fn pretty_name(arg: &str, oid: &Oid) -> String {
    std::env::var(format!("GITHEAD_{}", oid.to_hex()))
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| arg.to_string())
}

pub(crate) fn commit_of(repo: &Repository, rev: &str) -> CliResult<Oid> {
    let oid = resolve_rev(repo, rev)?;
    peel_to_kind(repo, &oid, ObjKind::Commit, rev)
}

pub(crate) fn tree_of(repo: &Repository, commit: &Oid) -> CliResult<Oid> {
    Ok(crate::cmd::log_format::read_commit(repo, commit)?.tree)
}

/// The empty tree, written to the object store.
pub(crate) fn empty_tree(repo: &mut Repository) -> CliResult<Oid> {
    use arks_object::{ObjectKind, RawObject};
    use arks_store::ObjectStore;
    let format = repo.object_format();
    Ok(repo
        .store_mut()
        .put(format, &RawObject::new(ObjectKind::Tree, Vec::new()))?)
}

/// Paths whose stage-0 index entry differs from `commit`'s tree (or that are unmerged).
pub(crate) fn staged_changes(repo: &Repository, commit: &Oid) -> CliResult<Vec<String>> {
    let tree = tree_sides(repo, &tree_of(repo, commit)?)?;
    let index = read_index(repo)?;
    let mut seen = HashSet::new();
    let mut out = BTreeSet::new();
    for e in &index.entries {
        seen.insert(e.path.clone());
        let same = e.stage() == 0
            && tree.get(&e.path)
                == Some(&Side {
                    mode: e.mode,
                    oid: e.oid,
                });
        if !same {
            out.insert(e.path.clone());
        }
    }
    out.extend(tree.keys().filter(|k| !seen.contains(*k)).cloned());
    Ok(out.into_iter().collect())
}

/// Write the stage-0 index as a tree (`None` while unmerged entries remain).
pub(crate) fn index_tree(repo: &mut Repository) -> CliResult<Option<Oid>> {
    let index = read_index(repo)?;
    if index.entries.iter().any(|e| e.stage() != 0) {
        return Ok(None);
    }
    if index.entries.is_empty() {
        return empty_tree(repo).map(Some);
    }
    let format = repo.object_format();
    Ok(Some(crate::write_tree::write_tree_from_index(
        repo.store_mut(),
        &index.entries,
        format,
    )?))
}

/// Best common ancestor of `commit` and the union of `tips` (what
/// `git merge-base <commit> <tip>...` answers), newest first on ties.
pub(crate) fn octopus_base(
    repo: &Repository,
    commit: &Oid,
    tips: &[Oid],
) -> CliResult<Option<Oid>> {
    let mut union = HashSet::new();
    for t in tips {
        union.extend(arks_repo::ancestor_set(repo, *t)?);
    }
    let mine = arks_repo::ancestor_set(repo, *commit)?;
    let common: Vec<Oid> = mine.intersection(&union).copied().collect();
    let mut best: Vec<(i64, Oid)> = Vec::new();
    for c in &common {
        let dominated = common
            .iter()
            .any(|d| d != c && crate::cmd::shared::is_ancestor(repo, c, d).unwrap_or(false));
        if !dominated {
            let when = crate::cmd::log_format::read_commit(repo, c)?
                .committer
                .timestamp;
            best.push((when, *c));
        }
    }
    best.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.to_hex().cmp(&b.1.to_hex())));
    Ok(best.first().map(|x| x.1))
}

fn exit_code(code: u8) -> CliResult<()> {
    if code == 0 {
        Ok(())
    } else {
        Err(CliError::silent(code))
    }
}

/// `arks merge-octopus <base>... -- <head> <remote>...`.
pub fn cmd_merge_octopus(args: &[String]) -> CliResult<()> {
    let a = split_args(args);
    if !a.dashdash || a.remotes.len() < 2 {
        return exit_code(2);
    }
    let mut repo = open_repo()?;
    let head = commit_of(&repo, a.head.as_deref().unwrap_or("HEAD"))?;
    let mut remotes = Vec::new();
    for r in &a.remotes {
        let oid = commit_of(&repo, r)?;
        remotes.push(OctopusRemote {
            arg: r.clone(),
            name: pretty_name(r, &oid),
            oid,
        });
    }
    exit_code(octopus(&mut repo, &head, &remotes)?.code)
}

/// One remote of an octopus merge: the argument as written, the name to
/// show and the commit.
pub(crate) struct OctopusRemote {
    pub arg: String,
    pub name: String,
    pub oid: Oid,
}

/// What the octopus strategy did.
pub(crate) struct OctopusOutcome {
    /// Strategy exit code (0 merged, 1 conflicts, 2 cannot handle).
    pub code: u8,
    /// `HEAD` was fast-forwarded (it is not a parent of the merge).
    pub head_fast_forwarded: bool,
}

/// Octopus strategy body. Stock git compares the merge base with the
/// remote *as written* (git merge passes full object names), so "Already up
/// to date with" and further fast-forwards only happen for full hex names;
/// `head` is the checked-out commit the merge starts from.
pub(crate) fn octopus(
    repo: &mut Repository,
    head: &Oid,
    remotes: &[OctopusRemote],
) -> CliResult<OctopusOutcome> {
    let done = |code: u8, ff: bool| OctopusOutcome {
        code,
        head_fast_forwarded: ff,
    };
    let current = repo.head_oid()?;
    let staged = staged_changes(repo, &current)?;
    if !staged.is_empty() {
        println!("Error: Your local changes to the following files would be overwritten by merge");
        for p in staged {
            println!("    {p}");
        }
        return Ok(done(2, false));
    }
    let work = work_tree(repo)?;
    // Merged commits so far, as (written name, commit).
    let mut mrc: Vec<(String, Oid)> = vec![(head.to_hex(), *head)];
    let mut mrt = tree_of(repo, head)?;
    let mut non_ff = false;
    let mut failed = false;
    let ff = |mrc: &[(String, Oid)]| mrc[0].1 != *head;
    for r in remotes {
        if failed {
            println!("Automated merge did not work.");
            println!("Should not be doing an octopus.");
            return Ok(done(2, ff(&mrc)));
        }
        let tips: Vec<Oid> = mrc.iter().map(|m| m.1).collect();
        let Some(common) = octopus_base(repo, &r.oid, &tips)? else {
            eprintln!("Unable to find common commit with {}", r.name);
            return Ok(done(2, ff(&mrc)));
        };
        let common_hex = common.to_hex();
        if common_hex == r.arg {
            println!("Already up to date with {}", r.name);
            continue;
        }
        if !non_ff && mrc.len() == 1 && mrc[0].0 == common_hex {
            println!("Fast-forwarding to: {}", r.name);
            crate::cmd::switch_tree::two_way_switch(repo, &work, Some(&mrc[0].1), &r.oid)
                .map_err(|e| CliError::Exit(2, format!("error: {e}")))?;
            mrc = vec![(r.arg.clone(), r.oid)];
            mrt = tree_of(repo, &r.oid)?;
            continue;
        }
        non_ff = true;
        println!("Trying simple merge with {}", r.name);
        let base_tree = tree_of(repo, &common)?;
        let their_tree = tree_of(repo, &r.oid)?;
        let tm = match trivial_merge(repo, &base_tree, &mrt, &their_tree) {
            Ok(t) => t,
            Err(CliError::Exit(2, m)) => {
                eprintln!("{m}");
                return Ok(done(2, ff(&mrc)));
            }
            Err(e) => return Err(e),
        };
        if !tm.unmerged.is_empty() {
            println!("Simple merge did not work, trying automatic merge.");
            if !merge_all_one_file(&tm.unmerged)? {
                eprintln!("fatal: merge program failed");
                failed = true;
            }
        }
        mrc.push((r.arg.clone(), r.oid));
        if !failed {
            if let Some(t) = index_tree(repo)? {
                mrt = t;
            }
        }
    }
    Ok(done(u8::from(failed), ff(&mrc)))
}

/// `arks merge-resolve <base>... -- <head> <remote>`.
pub fn cmd_merge_resolve(args: &[String]) -> CliResult<()> {
    let a = split_args(args);
    if !a.dashdash || a.bases.is_empty() || a.remotes.len() != 1 {
        return exit_code(2);
    }
    if let Some(opt) = a.bases.iter().find(|b| b.starts_with('-')) {
        eprintln!(
            "error: unknown option `{}'",
            opt.trim_start_matches('-').split('=').next().unwrap_or("")
        );
        return exit_code(2);
    }
    let mut repo = open_repo()?;
    let Ok(base) = commit_of(&repo, &a.bases[0]) else {
        return exit_code(2);
    };
    let head = commit_of(&repo, a.head.as_deref().unwrap_or("HEAD"))?;
    let remote = commit_of(&repo, &a.remotes[0])?;
    exit_code(resolve(&mut repo, &base, &head, &remote)?)
}

/// Resolve strategy body; returns the strategy exit code.
pub(crate) fn resolve(
    repo: &mut Repository,
    base: &Oid,
    head: &Oid,
    remote: &Oid,
) -> CliResult<u8> {
    let (bt, ht, rt) = (
        tree_of(repo, base)?,
        tree_of(repo, head)?,
        tree_of(repo, remote)?,
    );
    let tm = match trivial_merge(repo, &bt, &ht, &rt) {
        Ok(t) => t,
        Err(CliError::Exit(2, m)) => {
            eprintln!("{m}");
            return Ok(2);
        }
        Err(e) => return Err(e),
    };
    println!("Trying simple merge.");
    if tm.unmerged.is_empty() {
        return Ok(0);
    }
    println!("Simple merge failed, trying Automatic merge.");
    if merge_all_one_file(&tm.unmerged)? {
        Ok(0)
    } else {
        eprintln!("fatal: merge program failed");
        Ok(1)
    }
}

/// `arks merge-ours <base>... -- HEAD <remote>...`: exit 0 when the index
/// matches `HEAD`, 2 otherwise.
pub fn cmd_merge_ours(_args: &[String]) -> CliResult<()> {
    let repo = open_repo()?;
    let head = repo.head_oid()?;
    exit_code(if staged_changes(&repo, &head)?.is_empty() {
        0
    } else {
        2
    })
}

/// `arks merge-recursive <base>... -- <head> <remote>`.
pub fn cmd_merge_recursive(args: &[String]) -> CliResult<()> {
    recursive_program("merge-recursive", args)
}

/// `arks merge-recursive-ours`: stock git runs plain `merge-recursive` here.
pub fn cmd_merge_recursive_ours(args: &[String]) -> CliResult<()> {
    recursive_program("merge-recursive-ours", args)
}

/// `arks merge-recursive-theirs`: stock git runs plain `merge-recursive` here.
pub fn cmd_merge_recursive_theirs(args: &[String]) -> CliResult<()> {
    recursive_program("merge-recursive-theirs", args)
}

/// `arks merge-subtree`: `merge-recursive` with the remote tree shifted.
pub fn cmd_merge_subtree(args: &[String]) -> CliResult<()> {
    recursive_program("merge-subtree", args)
}

/// Strategy options of the recursive family (`-X <option>` of git merge).
#[derive(Default)]
pub(crate) struct RecursiveOpts {
    pub favor: Favor,
    /// `Some(None)`: shift automatically; `Some(Some(p))`: shift to `p`.
    pub subtree: Option<Option<String>>,
}

impl RecursiveOpts {
    /// Apply one option (without the leading `--`); `false` when unknown.
    pub(crate) fn apply(&mut self, opt: &str) -> bool {
        let (name, val) = match opt.split_once('=') {
            Some((n, v)) => (n, Some(v)),
            None => (opt, None),
        };
        match (name, val) {
            ("ours", None) => self.favor = Favor::Ours,
            ("theirs", None) => self.favor = Favor::Theirs,
            ("subtree", v) => self.subtree = Some(v.map(str::to_string)),
            (
                "patience"
                | "histogram"
                | "minimal"
                | "renormalize"
                | "no-renormalize"
                | "ignore-space-change"
                | "ignore-all-space"
                | "ignore-space-at-eol"
                | "ignore-cr-at-eol"
                | "no-renames"
                | "find-renames",
                _,
            ) => {}
            ("diff-algorithm" | "rename-threshold" | "renames", Some(_)) => {}
            _ => return false,
        }
        true
    }
}

fn recursive_program(name: &str, args: &[String]) -> CliResult<()> {
    let usage = || {
        CliError::Exit(
            129,
            format!("usage: git {name} <base>... -- <head> <remote> ..."),
        )
    };
    let a = split_args(args);
    if !a.dashdash || args.len() < 3 {
        return Err(usage());
    }
    let mut opts = RecursiveOpts::default();
    if name == "merge-subtree" {
        opts.subtree = Some(None);
    }
    let mut bases = Vec::new();
    for b in &a.bases {
        if let Some(o) = b.strip_prefix("--") {
            if !opts.apply(o) {
                return Err(CliError::fatal(format!("unknown option {b}")));
            }
        } else {
            bases.push(b.clone());
        }
    }
    if a.head.is_none() || a.remotes.len() != 1 {
        return Err(CliError::fatal(
            "not handling anything other than two heads merge.",
        ));
    }
    let mut repo = open_repo()?;
    let mut base_oids = Vec::new();
    for b in &bases {
        base_oids.push(
            commit_of(&repo, b)
                .map_err(|_| CliError::fatal(format!("could not parse object '{b}'")))?,
        );
    }
    let head_arg = a.head.clone().unwrap_or_default();
    let head = commit_of(&repo, &head_arg)
        .map_err(|_| CliError::fatal(format!("could not parse object '{head_arg}'")))?;
    let remote_arg = a.remotes[0].clone();
    let remote = commit_of(&repo, &remote_arg)
        .map_err(|_| CliError::fatal(format!("could not parse object '{remote_arg}'")))?;
    let base_tree = match base_oids.first() {
        Some(b) => Some(tree_of(&repo, b)?),
        None => match arks_repo::merge_base_oid(&repo, &head, &remote)? {
            Some(b) => Some(tree_of(&repo, &b)?),
            None => None,
        },
    };
    let names = (
        pretty_name(&head_arg, &head),
        pretty_name(&remote_arg, &remote),
    );
    let code = recursive(&mut repo, base_tree, &head, &remote, &names, &opts)?;
    exit_code(code)
}

/// Recursive (three-way tree) merge of `remote` into the checked-out `head`;
/// returns 0 (clean) or 1 (conflicts). `base_tree = None` merges from the
/// empty tree (unrelated histories).
pub(crate) fn recursive(
    repo: &mut Repository,
    base_tree: Option<Oid>,
    head: &Oid,
    remote: &Oid,
    names: &(String, String),
    opts: &RecursiveOpts,
) -> CliResult<u8> {
    let staged = staged_changes(repo, head)?;
    if !staged.is_empty() {
        return Err(CliError::Exit(
            128,
            format!(
                "error: Your local changes to the following files would be overwritten by merge:\n  {}",
                staged.join(" ")
            ),
        ));
    }
    let base_tree = match base_tree {
        Some(t) => t,
        None => empty_tree(repo)?,
    };
    let ours_tree = tree_of(repo, head)?;
    let mut their_tree = tree_of(repo, remote)?;
    let mut base_tree = base_tree;
    if let Some(prefix) = &opts.subtree {
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
    let b = tree_sides(repo, &base_tree)?;
    let o = tree_sides(repo, &ours_tree)?;
    let t = tree_sides(repo, &their_tree)?;
    let work = work_tree(repo)?;
    let index = read_index(repo)?;
    let mut dirty: Vec<String> = o
        .keys()
        .chain(t.keys())
        .filter(|k| o.get(*k) != t.get(*k))
        .filter(|k| !worktree_matches_index(repo, &work, &index.entries, k))
        .cloned()
        .collect();
    dirty.sort();
    dirty.dedup();
    if !dirty.is_empty() {
        let list: String = dirty.iter().map(|p| format!("\t{p}\n")).collect();
        return Err(CliError::Exit(
            128,
            format!(
                "error: Your local changes to the following files would be overwritten by merge:\n{list}Please commit your changes or stash them before you merge.\nAborting"
            ),
        ));
    }
    let style = arks_config::ConfigSet::load(Some(repo.common_dir()))
        .get("merge.conflictStyle")
        .unwrap_or_default();
    let labels = MergeLabels {
        ours: names.0.clone(),
        base: "merged common ancestors".into(),
        theirs: names.1.clone(),
        diff3: matches!(style.as_str(), "diff3" | "zdiff3"),
        favor: opts.favor,
    };
    let result = merge_trees_worktree(
        repo,
        &work,
        &base_tree,
        &ours_tree,
        &their_tree,
        true,
        &labels,
    )?;
    let mut paths: Vec<&String> = result
        .auto_merged
        .iter()
        .chain(result.conflicts.iter())
        .collect();
    paths.sort();
    paths.dedup();
    for p in paths {
        let conflict = result.conflicts.contains(p);
        match (b.get(p), o.get(p), t.get(p)) {
            (_, Some(_), None) | (_, None, Some(_)) if conflict => {
                let (gone, kept) = if o.contains_key(p) {
                    (&names.1, &names.0)
                } else {
                    (&names.0, &names.1)
                };
                println!(
                    "CONFLICT (modify/delete): {p} deleted in {gone} and modified in {kept}. Version {kept} of {p} left in tree."
                );
            }
            (None, Some(_), Some(_)) if conflict => {
                println!("CONFLICT (add/add): Merge conflict in {p}");
                println!("Auto-merging {p}");
            }
            _ => {
                println!("Auto-merging {p}");
                if conflict {
                    println!("CONFLICT (content): Merge conflict in {p}");
                }
            }
        }
    }
    Ok(u8::from(!result.conflicts.is_empty()))
}
