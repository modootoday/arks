//! `git merge-base`: best common ancestors of one commit and a hypothetical
//! merge of the others (`-a` / `--all` for every one), `--octopus` (common
//! ancestors of all), `--independent` (commits not reachable from any
//! other), `--is-ancestor` and `--fork-point <ref> [<commit>]` (the newest
//! reflog entry of `<ref>` that is an ancestor of `<commit>`).
//!
//! Primary: <https://git-scm.com/docs/git-merge-base>.

use crate::cmd::log_format::read_commit;
use crate::cmd::shared::{ObjKind, peel_to_kind, resolve_rev};
use crate::error::{CliError, CliResult};
use crate::util::open_repo;
use arks_oid::Oid;
use arks_repo::Repository;
use std::collections::HashSet;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Bases,
    Octopus,
    Independent,
    IsAncestor,
    ForkPoint,
}

fn usage() -> CliError {
    CliError::Exit(
        129,
        "usage: git merge-base [-a | --all] <commit> <commit>...\n   or: git merge-base [-a | --all] --octopus <commit>...\n   or: git merge-base --is-ancestor <commit> <commit>\n   or: git merge-base --independent <commit>...\n   or: git merge-base --fork-point <ref> [<commit>]".into(),
    )
}

fn commit_of(repo: &Repository, r: &str) -> CliResult<Oid> {
    let oid = resolve_rev(repo, r)
        .map_err(|_| CliError::fatal(format!("Not a valid object name {r}")))?;
    peel_to_kind(repo, &oid, ObjKind::Commit, r)
        .map_err(|_| CliError::fatal(format!("Not a valid commit name {r}")))
}

/// The members of `set` not reachable from another member.
fn best(repo: &Repository, set: &HashSet<Oid>) -> CliResult<Vec<Oid>> {
    let mut out = Vec::new();
    for c in set {
        let mut dominated = false;
        for d in set {
            if d != c && arks_repo::ancestor_set(repo, *d)?.contains(c) {
                dominated = true;
                break;
            }
        }
        if !dominated {
            out.push(*c);
        }
    }
    // Newest first, as git lists several bases.
    let mut dated: Vec<(i64, Oid)> = Vec::new();
    for o in out {
        dated.push((read_commit(repo, &o)?.committer.timestamp, o));
    }
    dated.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.to_hex().cmp(&b.1.to_hex())));
    Ok(dated.into_iter().map(|(_, o)| o).collect())
}

/// Best common ancestors of `one` and a merge of `others`.
fn bases(repo: &Repository, one: &Oid, others: &[Oid]) -> CliResult<Vec<Oid>> {
    let a = arks_repo::ancestor_set(repo, *one)?;
    let mut b: HashSet<Oid> = HashSet::new();
    for o in others {
        b.extend(arks_repo::ancestor_set(repo, *o)?);
    }
    let common: HashSet<Oid> = a.intersection(&b).copied().collect();
    best(repo, &common)
}

pub fn cmd_merge_base(args: &[String]) -> CliResult<()> {
    // Cite: https://git-scm.com/docs/git-merge-base
    let mut all = false;
    let mut mode = Mode::Bases;
    let mut revs: Vec<String> = Vec::new();
    let mut dashdash = false;
    for a in args {
        if dashdash || !a.starts_with('-') || a == "-" {
            revs.push(a.clone());
            continue;
        }
        let set = |m: Mode, cur: &mut Mode| -> CliResult<()> {
            if *cur != Mode::Bases && *cur != m {
                return Err(CliError::Exit(
                    129,
                    "error: options are incompatible".into(),
                ));
            }
            *cur = m;
            Ok(())
        };
        match a.as_str() {
            "--" => dashdash = true,
            "-a" | "--all" => all = true,
            "--no-all" => all = false,
            "--octopus" => set(Mode::Octopus, &mut mode)?,
            "--independent" => set(Mode::Independent, &mut mode)?,
            "--is-ancestor" => set(Mode::IsAncestor, &mut mode)?,
            "--fork-point" => set(Mode::ForkPoint, &mut mode)?,
            x => {
                return Err(CliError::Exit(
                    129,
                    format!("error: unknown option `{}'", x.trim_start_matches('-')),
                ));
            }
        }
    }
    let repo = open_repo()?;
    match mode {
        Mode::IsAncestor => {
            if revs.len() != 2 {
                return Err(CliError::fatal("--is-ancestor takes exactly two commits"));
            }
            let (a, b) = (commit_of(&repo, &revs[0])?, commit_of(&repo, &revs[1])?);
            if crate::cmd::shared::is_ancestor(&repo, &a, &b)? {
                Ok(())
            } else {
                Err(CliError::silent(1))
            }
        }
        Mode::ForkPoint => {
            if revs.is_empty() || revs.len() > 2 {
                return Err(usage());
            }
            let commit = commit_of(&repo, revs.get(1).map_or("HEAD", String::as_str))?;
            let name = &revs[0];
            let full = ["", "refs/", "refs/tags/", "refs/heads/", "refs/remotes/"]
                .iter()
                .map(|p| format!("{p}{name}"))
                .find(|n| repo.refs().resolve(n).is_ok())
                .ok_or_else(|| CliError::fatal(format!("Not a valid object name: '{name}'")))?;
            let tip = commit_of(&repo, &full)?;
            let mut cands: Vec<Oid> = vec![tip];
            for (old, new, _) in crate::reflog::read_entries(&repo, &full).into_iter().rev() {
                cands.push(new);
                cands.extend(old);
            }
            let mut seen = HashSet::new();
            let in_log: Vec<Oid> = cands.into_iter().filter(|c| seen.insert(*c)).collect();
            let in_set: HashSet<Oid> = in_log.iter().copied().collect();
            // Best common ancestors of the commit and a merge of every
            // reflog entry; the fork point is one that is itself an entry.
            let found = bases(&repo, &commit, &in_log)?;
            let common: HashSet<Oid> = found.into_iter().filter(|b| in_set.contains(b)).collect();
            match best(&repo, &common)?.first() {
                Some(b) => {
                    println!("{}", b.to_hex());
                    Ok(())
                }
                None => Err(CliError::silent(1)),
            }
        }
        Mode::Independent => {
            if revs.is_empty() {
                return Err(usage());
            }
            let mut set = HashSet::new();
            let mut order = Vec::new();
            for r in &revs {
                let c = commit_of(&repo, r)?;
                if set.insert(c) {
                    order.push(c);
                }
            }
            // Listed in the order given.
            let keep: HashSet<Oid> = best(&repo, &set)?.into_iter().collect();
            for o in order.iter().filter(|o| keep.contains(o)) {
                println!("{}", o.to_hex());
            }
            Ok(())
        }
        Mode::Octopus => {
            if revs.is_empty() {
                return Err(usage());
            }
            let commits: Vec<Oid> = revs
                .iter()
                .map(|r| commit_of(&repo, r))
                .collect::<CliResult<_>>()?;
            // Fold pairwise: bases of the running set with the next commit.
            let mut cur: Vec<Oid> = vec![commits[0]];
            for c in &commits[1..] {
                let mut next: HashSet<Oid> = HashSet::new();
                for x in &cur {
                    next.extend(bases(&repo, x, std::slice::from_ref(c))?);
                }
                cur = best(&repo, &next)?;
            }
            if cur.is_empty() {
                return Err(CliError::silent(1));
            }
            for o in if all { &cur[..] } else { &cur[..1] } {
                println!("{}", o.to_hex());
            }
            Ok(())
        }
        Mode::Bases => {
            if revs.len() < 2 {
                return Err(usage());
            }
            let commits: Vec<Oid> = revs
                .iter()
                .map(|r| commit_of(&repo, r))
                .collect::<CliResult<_>>()?;
            let found = bases(&repo, &commits[0], &commits[1..])?;
            if found.is_empty() {
                return Err(CliError::silent(1));
            }
            for o in if all { &found[..] } else { &found[..1] } {
                println!("{}", o.to_hex());
            }
            Ok(())
        }
    }
}
