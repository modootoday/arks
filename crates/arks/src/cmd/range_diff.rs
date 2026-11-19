//! `arks range-diff` — Fs-5 claimed subset.
//!
//! Primary: <https://git-scm.com/docs/git-range-diff>
//! Each side is `<A>..<B>` / `<A>...<B>` (or the `<A>...<B>` and
//! `<base> <rev1> <rev2>` forms): the non-merge commits of the range in log
//! order, oldest first (git runs `log --no-merges`). Claimed: pairwise
//! comparison by position — `=` when the subjects are equal, `!` otherwise
//! — and `<` / `>` for the commits one side has beyond the other, printed
//! with range-diff's `<n>:  <abbrev>` columns.
//! Partial: git's patch-similarity matching (creation factor) and the
//! diff-of-diffs printed under `!` pairs are not claimed.

use crate::cmd::replay::{read_commit, resolve_commit};
use crate::error::{CliError, CliResult};
use crate::util::{open_repo, refuse_unknown_flags};
use arks_oid::Oid;
use arks_repo::Repository;

pub fn cmd_range_diff(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &["--no-patch", "-s"])?;
    let positional: Vec<&str> = args
        .iter()
        .filter(|a| !a.starts_with('-'))
        .map(String::as_str)
        .collect();
    let repo = open_repo()?;
    let (old, new) = match positional.as_slice() {
        [a, b] => (series(&repo, a)?, series(&repo, b)?),
        // `<A>...<B>`: compare B..A with A..B.
        [sym] if sym.contains("...") => {
            let (l, r) = sym.split_once("...").expect("three dots");
            let l = if l.is_empty() { "HEAD" } else { l };
            let r = if r.is_empty() { "HEAD" } else { r };
            (
                series(&repo, &format!("{r}..{l}"))?,
                series(&repo, &format!("{l}..{r}"))?,
            )
        }
        // `<base> <rev1> <rev2>`: compare base..rev1 with base..rev2.
        [base, r1, r2] => (
            series(&repo, &format!("{base}..{r1}"))?,
            series(&repo, &format!("{base}..{r2}"))?,
        ),
        _ => {
            return Err(CliError::Usage(
                "range-diff <range1> <range2> | <rev1>...<rev2> | <base> <rev1> <rev2>".into(),
            ));
        }
    };
    let subject =
        |o: &Oid| -> CliResult<String> { Ok(read_commit(&repo, o)?.subject().to_string()) };
    let wa = old.len().max(1).to_string().len();
    let wb = new.len().max(1).to_string().len();
    let short = |o: &Oid| o.to_hex()[..7].to_owned();
    let dashes = "-".repeat(7);
    let old_col = |i: Option<usize>| match i {
        Some(i) => format!("{:>wa$}:  {}", i + 1, short(&old[i])),
        None => format!("{:>wa$}:  {dashes}", "-"),
    };
    let new_col = |j: Option<usize>| match j {
        Some(j) => format!("{:>wb$}:  {}", j + 1, short(&new[j])),
        None => format!("{:>wb$}:  {dashes}", "-"),
    };
    for k in 0..old.len().max(new.len()) {
        match (old.get(k), new.get(k)) {
            (Some(o), Some(n)) => {
                let (so, sn) = (subject(o)?, subject(n)?);
                let mark = if so == sn { '=' } else { '!' };
                println!("{} {mark} {} {sn}", old_col(Some(k)), new_col(Some(k)));
            }
            (Some(o), None) => {
                println!("{} < {} {}", old_col(Some(k)), new_col(None), subject(o)?);
            }
            (None, Some(n)) => {
                println!("{} > {} {}", old_col(None), new_col(Some(k)), subject(n)?);
            }
            (None, None) => {}
        }
    }
    Ok(())
}

/// Non-merge commits of the revision range `<A>..<B>` (or the symmetric
/// difference `<A>...<B>`), oldest first.
fn series(repo: &Repository, spec: &str) -> CliResult<Vec<Oid>> {
    let (left, right, three_dot) = if let Some((l, r)) = spec.split_once("...") {
        (l, r, true)
    } else if let Some((l, r)) = spec.split_once("..") {
        (l, r, false)
    } else {
        return Err(CliError::Usage(
            "range-diff: each side must be <A>..<B> or <A>...<B>".into(),
        ));
    };
    let left = if left.is_empty() { "HEAD" } else { left };
    let right = if right.is_empty() { "HEAD" } else { right };
    let left_oid = resolve_commit(repo, left)?;
    let end = resolve_commit(repo, right)?;
    let (include, exclude) = if three_dot {
        let bases: Vec<Oid> = arks_repo::merge_base_oid(repo, &left_oid, &end)?
            .into_iter()
            .collect();
        (vec![left_oid, end], bases)
    } else {
        (vec![end], vec![left_oid])
    };
    let mut v: Vec<Oid> = crate::cmd::log_format::walk(repo, &include, &exclude, false)?
        .into_iter()
        .filter(|(_, c)| c.parents.len() < 2)
        .map(|(o, _)| o)
        .collect();
    v.reverse();
    Ok(v)
}
