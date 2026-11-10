//! `arks range-diff` — Fs-5 claimed subset (subject pairwise compare).
//!
//! Primary: <https://git-scm.com/docs/git-range-diff>
//! Claimed: `range-diff <oldA>..<oldB> <newA>..<newB>` first-parent linear series,
//! pairwise subject equality (`=` / `!`). Full dual patch emit remains Open.

use crate::cmd::replay::{first_parent_set, read_commit, resolve_commit};
use crate::error::{CliError, CliResult};
use crate::util::{open_repo, refuse_unknown_flags};
use arks_oid::Oid;
use arks_repo::Repository;

pub fn cmd_range_diff(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &[])?;
    let positional: Vec<&str> = args
        .iter()
        .filter(|a| !a.starts_with('-'))
        .map(String::as_str)
        .collect();
    if positional.len() != 2 {
        return Err(CliError::Usage(
            "range-diff <oldA>..<oldB>|<oldA>...<oldB> <newA>..<newB>|<newA>...<newB>".into(),
        ));
    }
    let repo = open_repo()?;
    let old_series = dual_dot_series(&repo, positional[0])?;
    let new_series = dual_dot_series(&repo, positional[1])?;
    if old_series.is_empty() && new_series.is_empty() {
        return Ok(());
    }
    let n = old_series.len().max(new_series.len());
    for i in 0..n {
        let old = old_series.get(i);
        let new = new_series.get(i);
        match (old, new) {
            (Some(o), Some(n_oid)) => {
                let oc = read_commit(&repo, o)?;
                let nc = read_commit(&repo, n_oid)?;
                let os = oc.subject();
                let ns = nc.subject();
                let op = if os == ns { '=' } else { '!' };
                println!("{}:  {} {} {}: {}", i + 1, short(o), op, short(n_oid), ns);
            }
            (Some(o), None) => {
                let oc = read_commit(&repo, o)?;
                println!("{}:  {} < -: {}", i + 1, short(o), oc.subject());
            }
            (None, Some(n_oid)) => {
                let nc = read_commit(&repo, n_oid)?;
                println!("{}:  - > {}: {}", i + 1, short(n_oid), nc.subject());
            }
            (None, None) => unreachable!(),
        }
    }
    Ok(())
}

fn dual_dot_series(repo: &Repository, spec: &str) -> CliResult<Vec<Oid>> {
    // M16 Sy-5: accept symmetric `A...B` as merge-base(A,B)..B (git-range-diff).
    let (left, right, three_dot) = if let Some((l, r)) = spec.split_once("...") {
        (l, r, true)
    } else if let Some((l, r)) = spec.split_once("..") {
        (l, r, false)
    } else {
        return Err(CliError::Usage(
            "range-diff: each side must be <A>..<B> or <A>...<B>".into(),
        ));
    };
    if left.is_empty() || right.is_empty() {
        return Err(CliError::Usage(
            "range-diff: each side must be <A>..<B> or <A>...<B>".into(),
        ));
    }
    let left_oid = resolve_commit(repo, left)?;
    let end = resolve_commit(repo, right)?;
    let start = if three_dot {
        arks_repo::merge_base_oid(repo, &left_oid, &end)?.unwrap_or(left_oid)
    } else {
        left_oid
    };
    let stop = first_parent_set(repo, start)?;
    let mut newest = Vec::new();
    let mut oid = end;
    loop {
        if stop.contains(&oid) {
            break;
        }
        let commit = read_commit(repo, &oid)?;
        if commit.parents.len() > 1 {
            return Err(CliError::NotImplemented(
                "range-diff of merge commits is not claimed".into(),
            ));
        }
        newest.push(oid);
        match commit.parents.first() {
            Some(parent) => oid = *parent,
            None => break,
        }
    }
    newest.reverse();
    Ok(newest)
}

fn short(oid: &Oid) -> String {
    oid.to_hex()[..7].to_owned()
}
