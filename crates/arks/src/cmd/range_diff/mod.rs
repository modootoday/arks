//! `arks range-diff` — compare two versions of a patch series.
//!
//! Primary: <https://git-scm.com/docs/git-range-diff>. Each side is the
//! non-merge commits of a range, oldest first (`<base>..<tip>`, or the
//! `<rev1>...<rev2>` and `<base> <rev1> <rev2>` forms). Following the
//! documented algorithm, every commit becomes a patch text
//! ([`patch_text`]); commits whose file diffs are equal pair up first; the
//! rest are paired by a least-cost assignment ([`assign`]) over a matrix
//! whose entries are the line counts of the diffs between patch texts, with
//! fixed costs (`--creation-factor` percent of a patch's size) for leaving a
//! commit unpaired. Pairs print `=` (identical patch text) or `!` followed
//! by the diff of the two patch texts; unpaired commits print `<` / `>`.
//! Diff options shape that diff of diffs; `--notes` / `--no-notes` choose
//! the notes included in the patch texts.

pub mod assign;
pub mod patch_text;

use crate::cmd::diffcore::emit::{EmitCtx, emit};
use crate::cmd::diffcore::emit_rd::RangeDiffStyle;
use crate::cmd::diffcore::options::DiffOptions;
use crate::cmd::diffcore::{Content, Pair};
use crate::cmd::opt_parse::{Mode, OptDef, Takes::*, o, parse};
use crate::error::{CliError, CliResult};
use crate::util::open_repo;
use arks_object::Commit;
use arks_oid::Oid;
use arks_repo::Repository;
use patch_text::{Notes, PatchText};
use std::collections::HashMap;
use std::io::Write;

/// Options of `git range-diff -h` that are not diff options.
const OPTS: &[OptDef] = &[
    o("creation-factor", ' ', Req, true),
    o("no-dual-color", ' ', No, true),
    o("notes", ' ', Opt, true),
    o("left-only", ' ', No, true),
    o("right-only", ' ', No, true),
];

const USAGE: &str = "usage: git range-diff [<options>] <old-base>..<old-tip> <new-base>..<new-tip>
   or: git range-diff [<options>] <old-tip>...<new-tip>
   or: git range-diff [<options>] <base> <old-tip> <new-tip>";

const DEFAULT_CREATION_FACTOR: i64 = 60;

/// Which notes the patch texts carry (`git log --notes` semantics).
#[derive(Clone, Debug)]
pub struct NotesChoice {
    /// The default notes ref is shown.
    pub default: bool,
    /// Further notes refs given with `--notes=<ref>`.
    pub refs: Vec<String>,
}

impl Default for NotesChoice {
    fn default() -> Self {
        Self {
            default: true,
            refs: Vec::new(),
        }
    }
}

/// Everything that shapes one range-diff besides the two commit lists.
#[derive(Clone, Debug)]
pub struct RangeDiffOpts {
    pub creation_factor: i64,
    pub dual_color: bool,
    pub left_only: bool,
    pub right_only: bool,
    pub notes: NotesChoice,
    /// Options of the diff of diffs (line prefix, colour, formats).
    pub diff: DiffOptions,
    /// Pathspecs limiting both ranges.
    pub specs: Vec<String>,
}

fn usage_exit(msg: &str) -> CliError {
    CliError::Exit(129, format!("fatal: {msg}\n\n{USAGE}"))
}

pub fn cmd_range_diff(args: &[String]) -> CliResult<()> {
    let repo = open_repo()?;
    let mut diff = DiffOptions::default();
    crate::cmd::diff_cmd::porcelain_defaults(Some(&repo), &mut diff);
    let rest = diff.parse_all(args)?;
    diff.finish();
    let p = parse("range-diff", &rest, OPTS, Mode::default())?;
    let creation_factor = match (p.flag("creation-factor"), p.last("creation-factor")) {
        // `--no-creation-factor` resets the number to zero.
        (Some(false), _) => 0,
        (_, Some(v)) => parse_factor(v.as_deref())?,
        _ => DEFAULT_CREATION_FACTOR,
    };
    if diff.follow {
        return Err(CliError::fatal("--follow requires exactly one pathspec"));
    }
    let left_only = p.on("left-only");
    let right_only = p.on("right-only");
    if left_only && right_only {
        return Err(CliError::Exit(
            255,
            "error: options '--left-only' and '--right-only' cannot be used together".into(),
        ));
    }
    // git log's rules: the default notes show until `--no-notes` or a
    // `--notes=<ref>` that no bare `--notes` asked to keep them beside.
    let mut notes = NotesChoice::default();
    let mut implicit = true;
    for (key, val, neg) in p.opts() {
        if key != "notes" {
            continue;
        }
        match (neg, val) {
            (true, _) => {
                notes.default = false;
                notes.refs.clear();
                implicit = false;
            }
            (false, None) => {
                notes.default = true;
                implicit = false;
            }
            (false, Some(r)) => {
                if implicit {
                    notes.default = false;
                    implicit = false;
                }
                notes.refs.push(r.to_string());
            }
        }
    }
    let positional = p.args();
    let specs = p.after_dashdash();
    let (old, new) = ranges(&repo, &positional, &specs)?;
    // The diff of diffs: four-space indent, never reversed; `--quiet` and
    // `-s` keep only the commit headers.
    diff.line_prefix = "    ".into();
    diff.reverse = false;
    if diff.quiet {
        diff.no_output = true;
    } else {
        let others = diff.raw
            || diff.name_only
            || diff.name_status
            || diff.numstat
            || diff.stat
            || diff.shortstat
            || diff.dirstat.is_some()
            || diff.summary
            || diff.check;
        if diff.no_output {
            diff.patch = false;
            diff.no_output = !others;
        } else if !others {
            diff.patch = true;
        }
    }
    let output = diff.output.clone();
    let opts = RangeDiffOpts {
        creation_factor,
        dual_color: !p.on("no-dual-color"),
        left_only,
        right_only,
        notes,
        diff,
        specs,
    };
    let mut buf: Vec<u8> = Vec::new();
    render(&repo, &old, &new, &opts, &mut buf)?;
    match output {
        Some(path) => std::fs::write(&path, &buf)
            .map_err(|e| CliError::fatal(format!("could not open '{path}' for writing: {e}")))?,
        None => {
            let mut so = std::io::stdout().lock();
            so.write_all(&buf)?;
        }
    }
    Ok(())
}

/// `--creation-factor` takes an integer (git's OPT_INTEGER).
pub fn parse_factor(v: Option<&str>) -> CliResult<i64> {
    v.and_then(|s| s.trim().parse::<i64>().ok()).ok_or_else(|| {
        CliError::Exit(
            129,
            "error: option `creation-factor' expects a numerical value".into(),
        )
    })
}

/// Revisions selecting one side: (positive tips, negative tips).
type Side = (Vec<Oid>, Vec<Oid>);

/// Resolve the range forms of `git range-diff` into the two sides.
fn ranges(repo: &Repository, pos: &[String], specs: &[String]) -> CliResult<(Vec<Oid>, Vec<Oid>)> {
    let need = || usage_exit("need two commit ranges");
    let (old, new, extra): (Side, Side, &[String]) =
        if pos.first().is_some_and(|a| a.contains("...")) {
            let (l, r) = pos[0].split_once("...").expect("three dots");
            let l = if l.is_empty() { "HEAD" } else { l };
            let r = if r.is_empty() { "HEAD" } else { r };
            // A tip that does not resolve fails inside the log of that side.
            let lo = resolve_or(repo, l, 255, &format!("{r}..{l}"))?;
            let ro = resolve_or(repo, r, 255, &format!("{r}..{l}"))?;
            ((vec![lo], vec![ro]), (vec![ro], vec![lo]), &pos[1..])
        } else if pos.len() >= 2 && pos[0].contains("..") && pos[1].contains("..") {
            let a = range_side(repo, &pos[0])?.ok_or_else(need)?;
            let b = range_side(repo, &pos[1])?.ok_or_else(need)?;
            (a, b, &pos[2..])
        } else if pos.len() >= 3 {
            let mut tips = Vec::new();
            for r in &pos[..3] {
                if r.starts_with('^') || r.contains("..") || r.contains("^@") || r.contains("^!") {
                    return Err(need());
                }
                tips.push(resolve_or(repo, r, 128, r)?);
            }
            (
                (vec![tips[1]], vec![tips[0]]),
                (vec![tips[2]], vec![tips[0]]),
                &pos[3..],
            )
        } else {
            return Err(need());
        };
    let (mut old, mut new) = (old, new);
    for e in extra {
        // Further revisions join both logs (git passes them through).
        let (neg, name) = match e.strip_prefix('^') {
            Some(n) => (true, n),
            None => (false, e.as_str()),
        };
        let oid = crate::cmd::replay::resolve_commit(repo, name).map_err(|_| {
            CliError::Exit(
                255,
                format!(
                    "fatal: ambiguous argument '{e}': unknown revision or path not in the working tree."
                ),
            )
        })?;
        for side in [&mut old, &mut new] {
            if neg {
                side.1.push(oid);
            } else {
                side.0.push(oid);
            }
        }
    }
    Ok((series(repo, &old, specs)?, series(repo, &new, specs)?))
}

fn resolve_or(repo: &Repository, rev: &str, code: u8, shown: &str) -> CliResult<Oid> {
    crate::cmd::replay::resolve_commit(repo, rev).map_err(|_| {
        let msg = if code == 128 {
            format!("fatal: bad revision '{shown}'")
        } else {
            format!(
                "fatal: ambiguous argument '{shown}': unknown revision or path not in the working tree.\nerror: could not parse log for '{shown}'"
            )
        };
        CliError::Exit(code, msg)
    })
}

/// One range argument (`<a>..<b>` or `<a>...<b>`); `None` when it does not
/// name two distinct ends.
fn range_side(repo: &Repository, spec: &str) -> CliResult<Option<Side>> {
    let bad = || CliError::Exit(128, format!("fatal: bad revision '{spec}'"));
    if let Some((l, r)) = spec.split_once("...") {
        let l = if l.is_empty() { "HEAD" } else { l };
        let r = if r.is_empty() { "HEAD" } else { r };
        let lo = crate::cmd::replay::resolve_commit(repo, l).map_err(|_| bad())?;
        let ro = crate::cmd::replay::resolve_commit(repo, r).map_err(|_| bad())?;
        if lo == ro {
            return Ok(None);
        }
        let bases: Vec<Oid> = arks_repo::merge_base_oid(repo, &lo, &ro)?
            .into_iter()
            .collect();
        return Ok(Some((vec![lo, ro], bases)));
    }
    let Some((l, r)) = spec.split_once("..") else {
        return Ok(None);
    };
    let l = if l.is_empty() { "HEAD" } else { l };
    let r = if r.is_empty() { "HEAD" } else { r };
    let lo = crate::cmd::replay::resolve_commit(repo, l).map_err(|_| bad())?;
    let ro = crate::cmd::replay::resolve_commit(repo, r).map_err(|_| bad())?;
    if lo == ro {
        return Ok(None);
    }
    Ok(Some((vec![ro], vec![lo])))
}

/// Non-merge commits of one side, oldest first (`git log --no-merges
/// --reverse`), limited to `specs` when given.
pub fn series(repo: &Repository, side: &Side, specs: &[String]) -> CliResult<Vec<Oid>> {
    let walked = if specs.is_empty() {
        crate::cmd::log_format::walk(repo, &side.0, &side.1, false)?
    } else {
        crate::cmd::log_format::walk_paths(repo, &side.0, &side.1, false, specs)?
    };
    let mut v: Vec<Oid> = walked
        .into_iter()
        .filter(|(_, c)| c.parents.len() < 2)
        .map(|(o, _)| o)
        .collect();
    v.reverse();
    Ok(v)
}

struct Item {
    oid: Oid,
    commit: Commit,
    patch: PatchText,
    matching: Option<usize>,
    shown: bool,
}

/// Notes refs to read: (ref, heading).
fn notes_refs(repo: &Repository, choice: &NotesChoice) -> Vec<(String, String)> {
    let default_ref = crate::cmd::notes::default_ref(repo);
    let mut out = Vec::new();
    let mut push = |full: String| {
        if !out.iter().any(|(r, _): &(String, String)| *r == full) {
            let heading = if full == default_ref {
                "Notes".to_string()
            } else {
                format!(
                    "Notes ({})",
                    full.strip_prefix("refs/notes/").unwrap_or(&full)
                )
            };
            out.push((full, heading));
        }
    };
    if choice.default {
        push(default_ref.clone());
    }
    for r in &choice.refs {
        push(crate::cmd::notes::full_ref(r));
    }
    out
}

fn load_items(
    repo: &Repository,
    content: &Content<'_>,
    commits: &[Oid],
    opts: &RangeDiffOpts,
    notes: &[(String, HashMap<String, Oid>)],
) -> CliResult<Vec<Item>> {
    let mut inner = DiffOptions::default();
    crate::cmd::diff_cmd::porcelain_defaults(Some(repo), &mut inner);
    inner.context = None;
    inner.color = None;
    let mut out = Vec::with_capacity(commits.len());
    for oid in commits {
        let commit = crate::cmd::log_format::read_commit(repo, oid)?;
        let mut shown: Notes = Vec::new();
        for (heading, map) in notes {
            if let Some(blob) = map.get(&oid.to_hex()) {
                let text = repo.get_object(blob)?.payload;
                shown.push((heading.clone(), String::from_utf8_lossy(&text).into_owned()));
            }
        }
        let patch = patch_text::build(content, &inner, oid, &commit, &shown, &opts.specs)?;
        out.push(Item {
            oid: *oid,
            commit,
            patch,
            matching: None,
            shown: false,
        });
    }
    Ok(out)
}

/// Number of lines in the diff (three lines of context) between two texts:
/// the cost of pairing them.
fn diff_size(a: &str, b: &str) -> i64 {
    use crate::cmd::diffcore::xdiff::{self, HunkOpts};
    let al = xdiff::split_lines(a.as_bytes());
    let bl = xdiff::split_lines(b.as_bytes());
    let ops = xdiff::diff_lines(&al, &bl, &Default::default());
    let ho = HunkOpts {
        context: 3,
        inter: 0,
        function_context: false,
    };
    let hunks = xdiff::hunks(&ops, &al, ho, &|_, _| false, &|_| false);
    hunks.iter().map(|h| (h.end - h.start + 1) as i64).sum()
}

/// Pair old and new commits (exact diffs first, then least cost).
fn correspond(a: &mut [Item], b: &mut [Item], creation_factor: i64) {
    // Exact matches: equal file diffs pair in order.
    let mut by_diff: HashMap<&str, Vec<usize>> = HashMap::new();
    for (i, it) in a.iter().enumerate() {
        by_diff.entry(it.patch.diff()).or_default().push(i);
    }
    let mut exact: Vec<(usize, usize)> = Vec::new();
    for (j, it) in b.iter().enumerate() {
        if let Some(list) = by_diff.get_mut(it.patch.diff()) {
            if !list.is_empty() {
                exact.push((list.remove(0), j));
            }
        }
    }
    for (i, j) in exact {
        a[i].matching = Some(j);
        b[j].matching = Some(i);
    }
    let (na, nb) = (a.len(), b.len());
    let n = na + nb;
    const COST_MAX: i64 = 1 << 16;
    let mut cost = vec![0i64; n * n];
    for i in 0..na {
        for j in 0..nb {
            cost[i + n * j] = if a[i].matching == Some(j) {
                0
            } else if a[i].matching.is_none() && b[j].matching.is_none() {
                diff_size(a[i].patch.diff(), b[j].patch.diff())
            } else {
                COST_MAX
            };
        }
        let c = match a[i].matching {
            None => a[i].patch.diff_size as i64 * creation_factor / 100,
            Some(_) => COST_MAX,
        };
        for j in nb..n {
            cost[i + n * j] = c;
        }
    }
    for j in 0..nb {
        let c = match b[j].matching {
            None => b[j].patch.diff_size as i64 * creation_factor / 100,
            Some(_) => COST_MAX,
        };
        for i in na..n {
            cost[i + n * j] = c;
        }
    }
    // Columns are old commits, rows new ones (plus the "unpaired" slots).
    let a2b = assign::assign(n, &|col, row| cost[col + n * row]);
    for (i, &j) in a2b.iter().enumerate().take(na) {
        if j >= 0 && (j as usize) < nb {
            let j = j as usize;
            a[i].matching = Some(j);
            b[j].matching = Some(i);
        }
    }
}

/// Print the range-diff of `old` against `new` to `w`.
pub fn render(
    repo: &Repository,
    old: &[Oid],
    new: &[Oid],
    opts: &RangeDiffOpts,
    w: &mut impl Write,
) -> CliResult<()> {
    let content = Content::new(repo);
    let mut notes = Vec::new();
    for (r, heading) in notes_refs(repo, &opts.notes) {
        let map = crate::cmd::notes::note_map(repo, &r)?;
        notes.push((heading, map));
    }
    let mut a = load_items(repo, &content, old, opts, &notes)?;
    let mut b = load_items(repo, &content, new, opts, &notes)?;
    correspond(&mut a, &mut b, opts.creation_factor);
    let color = opts.diff.use_color();
    let width = (a.len().max(b.len()) + 1).to_string().len();
    // `--abbrev[=<n>]` also sets the width of the commit names.
    let abbrev = |o: &Oid| match opts.diff.abbrev {
        Some(0) => o.to_hex(),
        n => crate::cmd::shared::abbrev_hex(repo, o, n.unwrap_or(7)),
    };
    let dashes = |n: usize| "-".repeat(n);
    let header = |w: &mut dyn Write,
                  ai: Option<usize>,
                  bj: Option<usize>,
                  a: &[Item],
                  b: &[Item]|
     -> CliResult<()> {
        let ia = ai.map(|i| &a[i]);
        let ib = bj.map(|j| &b[j]);
        let ha = ia.map(|x| abbrev(&x.oid));
        let hb = ib.map(|x| abbrev(&x.oid));
        let len = ha.as_ref().or(hb.as_ref()).map_or(7, String::len);
        let left = match (ai, &ha) {
            (Some(i), Some(h)) => format!("{:>width$}:  {h}", i + 1),
            _ => format!("{:>width$}:  {}", "-", dashes(len)),
        };
        let right = match (bj, &hb) {
            (Some(j), Some(h)) => format!("{:>width$}:  {h}", j + 1),
            _ => format!("{:>width$}:  {}", "-", dashes(len)),
        };
        let mark = match (ia, ib) {
            (Some(x), Some(y)) if x.patch.text == y.patch.text => '=',
            (Some(_), Some(_)) => '!',
            (Some(_), None) => '<',
            _ => '>',
        };
        // The old commit's subject when there is one.
        let subject = crate::cmd::log_format::subject_line(
            &ia.or(ib)
                .map(|x| x.commit.message.clone())
                .unwrap_or_default(),
        );
        if !color {
            writeln!(w, "{left} {mark} {right} {subject}")?;
            return Ok(());
        }
        const RESET: &str = "\x1b[m";
        const OLD: &str = "\x1b[31m";
        const NEW: &str = "\x1b[32m";
        const COMMIT: &str = "\x1b[33m";
        match mark {
            '<' => writeln!(w, "{OLD}{left} {mark} {right} {subject}{RESET}")?,
            '>' => writeln!(w, "{NEW}{left} {mark} {right} {subject}{RESET}")?,
            '=' => writeln!(w, "{COMMIT}{left} {mark} {right} {subject}{RESET}")?,
            _ => writeln!(
                w,
                "{OLD}{left} {RESET}{COMMIT}{mark}{RESET}{NEW} {right}{RESET}{COMMIT} {subject}{RESET}"
            )?,
        }
        Ok(())
    };
    let ctx = EmitCtx {
        range_diff: Some(RangeDiffStyle {
            dual_color: opts.dual_color,
        }),
        ..Default::default()
    };
    let (mut i, mut j) = (0usize, 0usize);
    while i < a.len() || j < b.len() {
        if i < a.len() && a[i].shown {
            i += 1;
            continue;
        }
        if i < a.len() && a[i].matching.is_none() {
            if !opts.right_only {
                header(w, Some(i), None, &a, &b)?;
            }
            i += 1;
            continue;
        }
        while j < b.len() && b[j].matching.is_none() {
            if !opts.left_only {
                header(w, None, Some(j), &a, &b)?;
            }
            j += 1;
        }
        if j < b.len() {
            let ai = b[j].matching.expect("paired");
            header(w, Some(ai), Some(j), &a, &b)?;
            if !opts.diff.no_output {
                let pa = content.virtual_side("a", a[ai].patch.text.clone().into_bytes());
                let pb = content.virtual_side("b", b[j].patch.text.clone().into_bytes());
                // Pickaxe, `--diff-filter` and friends apply to the pair.
                let mut pairs = vec![Pair::new(Some(pa), Some(pb))];
                crate::cmd::diffcore::transform(&content, &opts.diff, &mut pairs)?;
                emit(w, &content, &opts.diff, ctx, &pairs)?;
            }
            a[ai].shown = true;
            j += 1;
        }
    }
    Ok(())
}
