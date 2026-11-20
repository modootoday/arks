//! Combined diffs of merge commits (`-c`, `--cc`): paths that differ from
//! every parent, in raw (`::` + one mode / id per parent), name-only,
//! name-status and the combined patch format with one column per parent.
//!
//! The patch keeps every result line and attaches to each position the
//! lines each parent lost there; lines lost identically from several
//! parents share one row. `--cc` drops hunks in which the result matches
//! one parent entirely.
//!
//! Primary: <https://git-scm.com/docs/git-diff-tree> (`-c`, `--cc`,
//! `--combined-all-paths`), <https://git-scm.com/docs/diff-format>
//! (§COMBINED DIFF FORMAT, §diff format for merges).

use super::emit::quote_path;
use super::options::DiffOptions;
use super::xdiff::{self, Op};
use super::{Content, Side};
use crate::cmd::log_format::read_commit;
use crate::error::CliResult;
use arks_oid::Oid;
use std::collections::BTreeSet;
use std::io::Write;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// `-c`
    Combined,
    /// `--cc`
    Dense,
}

/// One path that differs from every parent.
pub struct Entry {
    pub path: String,
    pub parents: Vec<Option<Side>>,
    pub result: Option<Side>,
}

impl Entry {
    fn status(&self, i: usize) -> char {
        match (&self.parents[i], &self.result) {
            (None, _) => 'A',
            (_, None) => 'D',
            _ => 'M',
        }
    }
}

/// Paths of `commit` that differ from all `parents` (pathspec-limited).
pub fn entries(
    c: &Content<'_>,
    _o: &DiffOptions,
    parents: &[Oid],
    commit: &Oid,
    specs: &[String],
) -> CliResult<Vec<Entry>> {
    let repo = c.repo;
    let tree = |oid: &Oid| -> CliResult<std::collections::BTreeMap<String, (u32, Oid)>> {
        super::flat_tree(repo, &read_commit(repo, oid)?.tree)
    };
    let result = tree(commit)?;
    let ptrees: Vec<_> = parents.iter().map(tree).collect::<CliResult<_>>()?;
    let mut paths: BTreeSet<&String> = result.keys().collect();
    for t in &ptrees {
        paths.extend(t.keys());
    }
    let mut out = Vec::new();
    for p in paths {
        if !super::spec_match(p, specs) {
            continue;
        }
        let r = result.get(p).copied();
        if ptrees.iter().any(|t| t.get(p).copied() == r) {
            continue;
        }
        let side = |e: Option<(u32, Oid)>| e.map(|(m, o)| Side::blob(p, m, o));
        out.push(Entry {
            path: p.clone(),
            parents: ptrees.iter().map(|t| side(t.get(p).copied())).collect(),
            result: side(r),
        });
    }
    Ok(out)
}

fn hex(c: &Content<'_>, o: &DiffOptions, s: &Option<Side>, raw: bool) -> String {
    let full = c.zero_hex();
    let n = match o.abbrev {
        Some(0) => full.len(),
        Some(n) => n,
        None if raw => full.len(),
        None if o.full_index => full.len(),
        None => 7,
    };
    match s.as_ref().and_then(|s| s.oid) {
        Some(oid) if n >= full.len() => oid.to_hex(),
        Some(oid) => c.abbrev(&oid, n),
        None => full[..n.min(full.len())].to_string(),
    }
}

/// Emit combined output for `entries`.
pub fn emit(
    w: &mut impl Write,
    c: &Content<'_>,
    o: &DiffOptions,
    mode: Mode,
    all_paths: bool,
    entries: &[Entry],
) -> CliResult<()> {
    if o.no_output {
        return Ok(());
    }
    let lp = &o.line_prefix;
    let term = if o.nul { "\0" } else { "\n" };
    if o.name_only {
        for e in entries {
            write!(
                w,
                "{lp}{}{term}",
                if o.nul {
                    e.path.clone()
                } else {
                    quote_path(&e.path)
                }
            )?;
        }
        return Ok(());
    }
    if o.raw || o.name_status {
        for e in entries {
            let st: String = (0..e.parents.len()).map(|i| e.status(i)).collect();
            if o.name_status {
                write!(w, "{lp}{st}")?;
            } else {
                let colons = ":".repeat(e.parents.len());
                let modes: Vec<String> = e
                    .parents
                    .iter()
                    .chain(std::iter::once(&e.result))
                    .map(|s| format!("{:06o}", s.as_ref().map_or(0, |s| s.mode)))
                    .collect();
                let ids: Vec<String> = e
                    .parents
                    .iter()
                    .chain(std::iter::once(&e.result))
                    .map(|s| hex(c, o, s, true))
                    .collect();
                write!(w, "{lp}{colons}{} {} {st}", modes.join(" "), ids.join(" "))?;
            }
            let sep = if o.nul { "\0" } else { "\t" };
            if all_paths {
                for _ in &e.parents {
                    write!(
                        w,
                        "{sep}{}",
                        if o.nul {
                            e.path.clone()
                        } else {
                            quote_path(&e.path)
                        }
                    )?;
                }
            }
            write!(
                w,
                "{sep}{}{term}",
                if o.nul {
                    e.path.clone()
                } else {
                    quote_path(&e.path)
                }
            )?;
        }
        if !o.patch {
            return Ok(());
        }
    }
    if o.patch {
        for e in entries {
            patch(w, c, o, mode, e)?;
        }
    }
    Ok(())
}

/// A row of the combined patch.
struct Row {
    text: Vec<u8>,
    /// Per parent: '+' (result line added vs that parent), '-' (lost), ' '.
    marks: Vec<u8>,
}

fn patch(
    w: &mut impl Write,
    c: &Content<'_>,
    o: &DiffOptions,
    mode: Mode,
    e: &Entry,
) -> CliResult<()> {
    let lp = &o.line_prefix;
    let np = e.parents.len();
    let bytes = |s: &Option<Side>| -> CliResult<Vec<u8>> {
        Ok(match s {
            Some(s) => c.bytes(s)?.to_vec(),
            None => Vec::new(),
        })
    };
    let rb = bytes(&e.result)?;
    let pbs: Vec<Vec<u8>> = e.parents.iter().map(bytes).collect::<CliResult<_>>()?;
    let rl = xdiff::split_lines(&rb);
    let tag = if mode == Mode::Dense {
        "cc"
    } else {
        "combined"
    };
    writeln!(w, "{lp}diff --{tag} {}", quote_path(&e.path))?;
    let rmode = e.result.as_ref().map_or(0, |s| s.mode);
    let pmodes: Vec<u32> = e
        .parents
        .iter()
        .map(|s| s.as_ref().map_or(0, |s| s.mode))
        .collect();
    let pm: Vec<String> = pmodes.iter().map(|m| format!("{m:06o}")).collect();
    if e.result.is_none() {
        writeln!(w, "{lp}deleted file mode {}", pm.join(","))?;
    } else if e.parents.iter().all(Option::is_none) {
        writeln!(w, "{lp}new file mode {rmode:06o}")?;
    } else if pmodes.iter().any(|m| *m != rmode) {
        writeln!(w, "{lp}mode {}..{rmode:06o}", pm.join(","))?;
    }
    let ids: Vec<String> = e.parents.iter().map(|s| hex(c, o, s, false)).collect();
    writeln!(
        w,
        "{lp}index {}..{}",
        ids.join(","),
        hex(c, o, &e.result, false)
    )?;
    if pbs.iter().any(|b| xdiff::is_binary(b)) || xdiff::is_binary(&rb) {
        writeln!(w, "{lp}Binary files differ")?;
        return Ok(());
    }
    writeln!(w, "{lp}--- a/{}", e.path)?;
    writeln!(w, "{lp}+++ b/{}", e.path)?;
    // lost[j]: rows lost before result line j (j == rl.len(): at end).
    let mut lost: Vec<Vec<Row>> = (0..=rl.len()).map(|_| Vec::new()).collect();
    let mut added: Vec<Vec<bool>> = vec![vec![false; np]; rl.len()];
    // pre[i][j]: parent i lines consumed before the lost rows at j.
    let mut pre: Vec<Vec<usize>> = vec![vec![0; rl.len() + 1]; np];
    let mut lost_count: Vec<Vec<usize>> = vec![vec![0; rl.len() + 1]; np];
    for (i, pb) in pbs.iter().enumerate() {
        let pl = xdiff::split_lines(pb);
        let ops = xdiff::diff_lines(&pl, &rl, &o.line);
        let (mut a, mut b) = (0usize, 0usize);
        let mut pending: Vec<&[u8]> = Vec::new();
        let mut run_start = 0usize;
        for op in &ops {
            match op {
                Op::Del => {
                    if pending.is_empty() {
                        run_start = a;
                    }
                    pending.push(pl[a]);
                    a += 1;
                }
                Op::Eq | Op::Ins => {
                    if pending.is_empty() {
                        pre[i][b] = a;
                    } else {
                        pre[i][b] = run_start;
                    }
                    lost_count[i][b] = pending.len();
                    merge_lost(&mut lost[b], &pending, i, np);
                    pending.clear();
                    if *op == Op::Ins {
                        added[b][i] = true;
                    } else {
                        a += 1;
                    }
                    b += 1;
                }
            }
        }
        pre[i][rl.len()] = if pending.is_empty() { a } else { run_start };
        lost_count[i][rl.len()] = pending.len();
        merge_lost(&mut lost[rl.len()], &pending, i, np);
    }
    // Interesting positions: a lost row or an added line.
    let n = rl.len();
    let interesting =
        |j: usize| -> bool { !lost[j].is_empty() || (j < n && added[j].iter().any(|x| *x)) };
    let ctx = o.context();
    // Hunks over positions 0..=n (position j covers lost[j] then line j).
    let mut hunks: Vec<(usize, usize)> = Vec::new();
    let mut j = 0;
    while j <= n {
        if !interesting(j) {
            j += 1;
            continue;
        }
        let mut s = j;
        let mut e2 = j;
        let mut k = j + 1;
        while k <= n {
            if interesting(k) {
                if k - e2 <= 2 * ctx + 1 {
                    e2 = k;
                } else {
                    break;
                }
            }
            k += 1;
        }
        s = s.saturating_sub(ctx);
        let end = (e2 + ctx + 1).min(n + 1);
        hunks.push((s, end));
        j = end;
    }
    // Merge overlapping hunks.
    let mut merged: Vec<(usize, usize)> = Vec::new();
    for h in hunks {
        match merged.last_mut() {
            Some(last) if h.0 <= last.1 => last.1 = last.1.max(h.1),
            _ => merged.push(h),
        }
    }
    for (s, end) in merged {
        // Positions s..end; line j included when j < n.
        let changed_vs = |i: usize| -> bool {
            (s..end).any(|j| (j < n && added[j][i]) || lost[j].iter().any(|r| r.marks[i] == b'-'))
        };
        if mode == Mode::Dense && (0..np).any(|i| !changed_vs(i)) {
            continue;
        }
        let last_line = end.min(n);
        let mut ranges = Vec::new();
        for i in 0..np {
            let start = pre[i][s];
            let mut stop = if end > n {
                pre[i][n] + lost_count[i][n]
            } else {
                pre[i][end]
            };
            if stop < start {
                stop = start;
            }
            ranges.push(format!("-{},{}", start + 1, stop - start));
        }
        let rs = s.min(n);
        let rlen = last_line.saturating_sub(rs);
        let ats = "@".repeat(np + 1);
        write!(
            w,
            "{lp}{ats} {} +{},{} {ats}",
            ranges.join(" "),
            rs + 1,
            rlen
        )?;
        if let Some(f) = combined_funcname(&rl, rs) {
            w.write_all(b" ")?;
            w.write_all(f)?;
        }
        writeln!(w)?;
        for j in s..end {
            for r in &lost[j] {
                w.write_all(lp.as_bytes())?;
                w.write_all(&r.marks)?;
                w.write_all(r.text.strip_suffix(b"\n").unwrap_or(&r.text))?;
                w.write_all(b"\n")?;
            }
            if j < n {
                w.write_all(lp.as_bytes())?;
                for &hit in added[j].iter().take(np) {
                    w.write_all(if hit { b"+" } else { b" " })?;
                }
                let l = rl[j];
                w.write_all(l.strip_suffix(b"\n").unwrap_or(l))?;
                w.write_all(b"\n")?;
            }
        }
    }
    Ok(())
}

/// Add parent `i`'s lost lines to the rows at one position: lines equal
/// to an existing row (in order) share it, others are inserted.
fn merge_lost(rows: &mut Vec<Row>, lines: &[&[u8]], i: usize, np: usize) {
    if lines.is_empty() {
        return;
    }
    let old: Vec<&[u8]> = rows.iter().map(|r| r.text.as_slice()).collect();
    let ops = xdiff::diff_lines(&old, lines, &Default::default());
    let mut out: Vec<Row> = Vec::with_capacity(rows.len() + lines.len());
    let mut it = std::mem::take(rows).into_iter();
    let mut li = 0;
    for op in ops {
        match op {
            Op::Eq => {
                let mut r = it.next().unwrap_or_else(|| unreachable!());
                r.marks[i] = b'-';
                out.push(r);
                li += 1;
            }
            Op::Del => out.push(it.next().unwrap_or_else(|| unreachable!())),
            Op::Ins => {
                let mut marks = vec![b' '; np];
                marks[i] = b'-';
                out.push(Row {
                    text: lines[li].to_vec(),
                    marks,
                });
                li += 1;
            }
        }
    }
    *rows = out;
}

/// Function header for a combined hunk: the last line before `before`
/// starting with a letter, `_` or `$` (shown without its final byte).
fn combined_funcname<'a>(lines: &[&'a [u8]], before: usize) -> Option<&'a [u8]> {
    for l in lines[..before.min(lines.len())].iter().rev() {
        if l.first()
            .is_some_and(|c| c.is_ascii_alphabetic() || *c == b'_' || *c == b'$')
        {
            let body = l.strip_suffix(b"\n").unwrap_or(l);
            let body = &body[..body.len().saturating_sub(1)];
            return Some(&body[..body.len().min(80)]);
        }
    }
    None
}
