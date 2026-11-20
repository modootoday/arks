//! Range-diff and interdiff sections that `git format-patch --range-diff` /
//! `--interdiff` add to a cover letter or a single patch.
//!
//! Primary: <https://git-scm.com/docs/git-format-patch> (`--interdiff`,
//! `--range-diff`, `--creation-factor`). `--interdiff=<previous>` is the
//! diff from `<previous>` to the tip of the series; `--range-diff=<previous>`
//! compares `<previous>` (a range, or a tip sharing the series' base) with
//! the series. Titles read `Interdiff:` / `Range-diff:`, or `... against
//! v<N-1>:` when the reroll count is a whole number `<N>` (observed stock
//! output).

use super::{NotesChoice, RangeDiffOpts, range_side, render, series};
use crate::cmd::diffcore::emit::{EmitCtx, emit};
use crate::cmd::diffcore::options::DiffOptions;
use crate::cmd::diffcore::{self, Content};
use crate::error::{CliError, CliResult};
use arks_oid::Oid;
use arks_repo::Repository;

/// format-patch's default creation factor (the same as range-diff's).
pub const DEFAULT_CREATION_FACTOR: i64 = 60;

/// `Interdiff` / `Range-diff` plus ` against v<N-1>` for a whole reroll count.
pub fn title(what: &str, reroll: Option<&str>) -> String {
    match reroll.and_then(|r| r.parse::<i64>().ok()) {
        Some(n) => format!("{what} against v{}:", n - 1),
        None => format!("{what}:"),
    }
}

fn resolve(repo: &Repository, rev: &str) -> CliResult<Oid> {
    crate::cmd::replay::resolve_commit(repo, rev)
        .map_err(|_| CliError::fatal(format!("bad revision '{rev}'")))
}

/// The patch from `previous` to `tip` (format-patch's diff options).
pub fn interdiff(repo: &Repository, previous: &str, tip: &Oid, binary: bool) -> CliResult<String> {
    let from = resolve(repo, previous)?;
    let tree =
        |o: &Oid| -> CliResult<Oid> { Ok(crate::cmd::log_format::read_commit(repo, o)?.tree) };
    let (a, b) = (tree(&from)?, tree(tip)?);
    let mut o = DiffOptions::default();
    crate::cmd::diff_cmd::porcelain_defaults(Some(repo), &mut o);
    o.patch = true;
    o.binary = binary;
    o.color = None;
    o.finish();
    let content = Content::new(repo);
    let mut pairs = diffcore::tree_pairs(repo, Some(&a), Some(&b), true, false, &[])?;
    diffcore::transform(&content, &o, &mut pairs)?;
    let mut buf = Vec::new();
    emit(&mut buf, &content, &o, EmitCtx::default(), &pairs)?;
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

/// The range-diff of `previous` against the series `commits` whose base is
/// `origin`.
pub fn range_diff(
    repo: &Repository,
    previous: &str,
    origin: Option<&Oid>,
    commits: &[Oid],
    creation_factor: i64,
) -> CliResult<String> {
    let old = if previous.contains("..") {
        match range_side(repo, previous)? {
            Some(side) => series(repo, &side, &[])?,
            None => Vec::new(),
        }
    } else {
        let tip = resolve(repo, previous)?;
        let base = origin.copied().ok_or_else(|| {
            CliError::fatal(format!(
                "could not determine the base of '{previous}' for --range-diff"
            ))
        })?;
        series(repo, &(vec![tip], vec![base]), &[])?
    };
    let diff = DiffOptions {
        patch: true,
        line_prefix: "    ".into(),
        ..Default::default()
    };
    let opts = RangeDiffOpts {
        creation_factor,
        dual_color: false,
        left_only: false,
        right_only: false,
        notes: NotesChoice::default(),
        diff,
        specs: Vec::new(),
    };
    let mut buf = Vec::new();
    render(repo, &old, commits, &opts, &mut buf)?;
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

/// Lines indented by two spaces (the interdiff inside a single patch).
pub fn indent(text: &str) -> String {
    text.lines().map(|l| format!("  {l}\n")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titles() {
        assert_eq!(title("Range-diff", None), "Range-diff:");
        assert_eq!(title("Interdiff", Some("3")), "Interdiff against v2:");
        assert_eq!(title("Range-diff", Some("1")), "Range-diff against v0:");
        assert_eq!(title("Range-diff", Some("2.5")), "Range-diff:");
    }
}
