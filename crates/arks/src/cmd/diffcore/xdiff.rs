//! Line diff engine for the diff core: edit scripts with git's diff
//! algorithm and whitespace options, and hunk grouping.
//!
//! Algorithms (git-diff `--diff-algorithm`): `myers` and `minimal` (the
//! greedy O((N+M)·D) shortest edit script; both are minimal here),
//! `patience` (unique common lines as anchors, recursing between them and
//! falling back to Myers), `histogram` (lowest-occurrence common regions,
//! falling back to Myers) and `--anchored=<text>` (patience whose anchors
//! must include the unique lines that start with `<text>`).
//!
//! Whitespace options compare normalised lines: `-w` drops all
//! whitespace, `-b` collapses runs and drops trailing whitespace,
//! `--ignore-space-at-eol` drops trailing whitespace and
//! `--ignore-cr-at-eol` drops one CR before the newline.
//!
//! Primary: <https://git-scm.com/docs/git-diff> (§Diff algorithm options),
//! <https://git-scm.com/docs/diff-options>.
//!
//! Partial: the indent heuristic (`--indent-heuristic`, git's default) is
//! not applied: ambiguous change groups keep the position the edit script
//! gives them.

use crate::cmd::bre::Bre;
use std::collections::HashMap;

/// Above this many lines on both sides combined, fall back to one replace.
pub const MAX_LINES: usize = 400_000;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Op {
    Eq,
    Del,
    Ins,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Algo {
    #[default]
    Myers,
    Minimal,
    Patience,
    Histogram,
}

impl Algo {
    pub fn parse(v: &str) -> Option<Self> {
        Some(match v {
            "myers" | "default" => Algo::Myers,
            "minimal" => Algo::Minimal,
            "patience" => Algo::Patience,
            "histogram" => Algo::Histogram,
            _ => return None,
        })
    }
}

/// Line comparison and algorithm options.
#[derive(Clone, Debug, Default)]
pub struct LineOpts {
    pub algo: Algo,
    pub anchors: Vec<String>,
    pub ignore_all_space: bool,
    pub ignore_space_change: bool,
    pub ignore_space_at_eol: bool,
    pub ignore_cr_at_eol: bool,
    pub ignore_blank_lines: bool,
    pub ignore_regex: Vec<Bre>,
}

impl LineOpts {
    /// Comparison key of one line (newline presence is kept).
    fn key(&self, line: &[u8]) -> Vec<u8> {
        let (body, nl) = match line.strip_suffix(b"\n") {
            Some(b) => (b, true),
            None => (line, false),
        };
        let mut body = body;
        if self.ignore_cr_at_eol || self.ignore_all_space || self.ignore_space_change {
            if let Some(b) = body.strip_suffix(b"\r") {
                body = b;
            }
        }
        let mut k: Vec<u8> = if self.ignore_all_space {
            body.iter()
                .copied()
                .filter(|c| !c.is_ascii_whitespace())
                .collect()
        } else if self.ignore_space_change {
            let trimmed = trim_end_ws(body);
            let mut v = Vec::with_capacity(trimmed.len());
            let mut in_ws = false;
            for &c in trimmed {
                if c.is_ascii_whitespace() {
                    if !in_ws {
                        v.push(b' ');
                    }
                    in_ws = true;
                } else {
                    v.push(c);
                    in_ws = false;
                }
            }
            v
        } else if self.ignore_space_at_eol {
            trim_end_ws(body).to_vec()
        } else {
            body.to_vec()
        };
        if nl {
            k.push(b'\n');
        }
        k
    }
}

fn trim_end_ws(b: &[u8]) -> &[u8] {
    let mut e = b.len();
    while e > 0 && b[e - 1].is_ascii_whitespace() {
        e -= 1;
    }
    &b[..e]
}

/// Lines including their trailing `\n` (last line may lack it).
pub fn split_lines(b: &[u8]) -> Vec<&[u8]> {
    if b.is_empty() {
        return Vec::new();
    }
    b.split_inclusive(|c| *c == b'\n').collect()
}

/// git's binary heuristic: a NUL byte in the first 8000 bytes.
pub fn is_binary(b: &[u8]) -> bool {
    b[..b.len().min(8000)].contains(&0)
}

/// Edit script turning `a` into `b` (one op per consumed line), with git's
/// order of deletions before insertions inside a change run.
pub fn diff_lines(a: &[&[u8]], b: &[&[u8]], o: &LineOpts) -> Vec<Op> {
    if a.len() + b.len() > MAX_LINES {
        let mut s = vec![Op::Del; a.len()];
        s.extend(std::iter::repeat_n(Op::Ins, b.len()));
        return s;
    }
    let mut ids: HashMap<Vec<u8>, u32> = HashMap::new();
    let mut intern = |l: &[u8]| -> u32 {
        let k = o.key(l);
        let n = ids.len() as u32;
        *ids.entry(k).or_insert(n)
    };
    let ai: Vec<u32> = a.iter().map(|l| intern(l)).collect();
    let bi: Vec<u32> = b.iter().map(|l| intern(l)).collect();
    let mut ops = Vec::with_capacity(a.len() + b.len());
    match o.algo {
        Algo::Myers | Algo::Minimal if o.anchors.is_empty() => {
            ops = myers(&ai, &bi);
        }
        Algo::Histogram if o.anchors.is_empty() => histogram(&ai, &bi, &mut ops),
        _ => {
            let anchored: Vec<bool> = a
                .iter()
                .map(|l| o.anchors.iter().any(|t| l.starts_with(t.as_bytes())))
                .collect();
            patience(&ai, &bi, &anchored, &mut ops)
        }
    }
    normalize_runs(&mut ops);
    ops
}

/// Myers' greedy shortest edit script with trace backtracking.
fn myers(a: &[u32], b: &[u32]) -> Vec<Op> {
    // Common prefix / suffix are trivially equal.
    let mut pre = 0;
    while pre < a.len() && pre < b.len() && a[pre] == b[pre] {
        pre += 1;
    }
    let mut suf = 0;
    while suf < a.len() - pre && suf < b.len() - pre && a[a.len() - 1 - suf] == b[b.len() - 1 - suf]
    {
        suf += 1;
    }
    let (a, b) = (&a[pre..a.len() - suf], &b[pre..b.len() - suf]);
    let mut ops = vec![Op::Eq; pre];
    let n = a.len() as isize;
    let m = b.len() as isize;
    let max = (n + m) as usize;
    let off = max as isize + 1;
    let mut v = vec![0isize; 2 * max + 3];
    let mut trace: Vec<Vec<isize>> = Vec::new();
    if n == 0 || m == 0 {
        ops.extend(std::iter::repeat_n(Op::Del, a.len()));
        ops.extend(std::iter::repeat_n(Op::Ins, b.len()));
        ops.extend(std::iter::repeat_n(Op::Eq, suf));
        return ops;
    }
    'outer: for d in 0..=max as isize {
        trace.push(v.clone());
        let mut k = -d;
        while k <= d {
            let idx = (k + off) as usize;
            let mut x = if k == -d || (k != d && v[idx - 1] < v[idx + 1]) {
                v[idx + 1]
            } else {
                v[idx - 1] + 1
            };
            let mut y = x - k;
            while x < n && y < m && a[x as usize] == b[y as usize] {
                x += 1;
                y += 1;
            }
            v[idx] = x;
            if x >= n && y >= m {
                trace.push(v.clone());
                break 'outer;
            }
            k += 2;
        }
    }
    let mut rev = Vec::with_capacity(max);
    let (mut x, mut y) = (n, m);
    for d in (1..trace.len() as isize - 1).rev() {
        let v = &trace[d as usize];
        let k = x - y;
        let prev_k = if k == -d || (k != d && v[(k - 1 + off) as usize] < v[(k + 1 + off) as usize])
        {
            k + 1
        } else {
            k - 1
        };
        let prev_x = v[(prev_k + off) as usize];
        let prev_y = prev_x - prev_k;
        while x > prev_x && y > prev_y {
            rev.push(Op::Eq);
            x -= 1;
            y -= 1;
        }
        rev.push(if x == prev_x { Op::Ins } else { Op::Del });
        x = prev_x;
        y = prev_y;
    }
    while x > 0 && y > 0 {
        rev.push(Op::Eq);
        x -= 1;
        y -= 1;
    }
    rev.reverse();
    ops.extend(rev);
    ops.extend(std::iter::repeat_n(Op::Eq, suf));
    ops
}

/// Patience diff: unique common lines (longest increasing run of them) as
/// anchors; ranges without anchors fall back to Myers.
fn patience(a: &[u32], b: &[u32], anchored: &[bool], out: &mut Vec<Op>) {
    let mut pre = 0;
    while pre < a.len() && pre < b.len() && a[pre] == b[pre] {
        pre += 1;
    }
    out.extend(std::iter::repeat_n(Op::Eq, pre));
    let mut suf = 0;
    while suf < a.len() - pre && suf < b.len() - pre && a[a.len() - 1 - suf] == b[b.len() - 1 - suf]
    {
        suf += 1;
    }
    let (ra, rb) = (&a[pre..a.len() - suf], &b[pre..b.len() - suf]);
    let anch = &anchored[pre..anchored.len() - suf];
    if ra.is_empty() || rb.is_empty() {
        out.extend(std::iter::repeat_n(Op::Del, ra.len()));
        out.extend(std::iter::repeat_n(Op::Ins, rb.len()));
    } else {
        let mut count: HashMap<u32, (usize, usize, usize)> = HashMap::new();
        for (i, x) in ra.iter().enumerate() {
            let e = count.entry(*x).or_insert((0, 0, i));
            e.0 += 1;
            e.2 = i;
        }
        let mut pairs: Vec<(usize, usize)> = Vec::new();
        let mut bcount: HashMap<u32, (usize, usize)> = HashMap::new();
        for (j, y) in rb.iter().enumerate() {
            let e = bcount.entry(*y).or_insert((0, j));
            e.0 += 1;
        }
        for (j, y) in rb.iter().enumerate() {
            if let (Some(&(1, _, i)), Some(&(1, _))) = (count.get(y), bcount.get(y)) {
                pairs.push((i, j));
            }
        }
        pairs.sort();
        // Anchored lines must stay: if any unique pair is anchored, take
        // the increasing run among the anchored pairs only.
        let anchored_pairs: Vec<(usize, usize)> =
            pairs.iter().copied().filter(|(i, _)| anch[*i]).collect();
        let chosen = if anchored_pairs.is_empty() {
            lis(&pairs)
        } else {
            lis(&anchored_pairs)
        };
        if chosen.is_empty() {
            out.extend(myers(ra, rb));
        } else {
            let (mut ai, mut bi) = (0, 0);
            for (i, j) in chosen {
                patience(&ra[ai..i], &rb[bi..j], &anch[ai..i], out);
                out.push(Op::Eq);
                ai = i + 1;
                bi = j + 1;
            }
            patience(&ra[ai..], &rb[bi..], &anch[ai..], out);
        }
    }
    out.extend(std::iter::repeat_n(Op::Eq, suf));
}

/// Longest run of pairs increasing in both coordinates (pairs sorted by a).
fn lis(pairs: &[(usize, usize)]) -> Vec<(usize, usize)> {
    let mut tails: Vec<usize> = Vec::new();
    let mut prev: Vec<Option<usize>> = vec![None; pairs.len()];
    for (k, &(_, j)) in pairs.iter().enumerate() {
        let pos = tails.partition_point(|&t| pairs[t].1 < j);
        if pos > 0 {
            prev[k] = Some(tails[pos - 1]);
        }
        if pos == tails.len() {
            tails.push(k);
        } else {
            tails[pos] = k;
        }
    }
    let mut out = Vec::new();
    let mut cur = tails.last().copied();
    while let Some(k) = cur {
        out.push(pairs[k]);
        cur = prev[k];
    }
    out.reverse();
    out
}

/// Histogram diff: split at the common region whose lines occur least
/// often in `a`; regions without a common line fall back to Myers.
fn histogram(a: &[u32], b: &[u32], out: &mut Vec<Op>) {
    const MAX_CHAIN: usize = 64;
    if a.is_empty() || b.is_empty() {
        out.extend(std::iter::repeat_n(Op::Del, a.len()));
        out.extend(std::iter::repeat_n(Op::Ins, b.len()));
        return;
    }
    let mut occ: HashMap<u32, Vec<usize>> = HashMap::new();
    for (i, x) in a.iter().enumerate() {
        occ.entry(*x).or_default().push(i);
    }
    let cnt = |x: u32| occ.get(&x).map_or(0, Vec::len);
    // (a_start, b_start, len, lowest count)
    let mut best: Option<(usize, usize, usize, usize)> = None;
    let mut bi = 0;
    while bi < b.len() {
        let mut next_b = bi + 1;
        if let Some(list) = occ.get(&b[bi]) {
            if list.len() <= MAX_CHAIN {
                for &ai in list {
                    let (mut as_, mut bs) = (ai, bi);
                    let mut rc = list.len();
                    while as_ > 0 && bs > 0 && a[as_ - 1] == b[bs - 1] {
                        as_ -= 1;
                        bs -= 1;
                        rc = rc.min(cnt(a[as_]));
                    }
                    let (mut ae, mut be) = (ai + 1, bi + 1);
                    while ae < a.len() && be < b.len() && a[ae] == b[be] {
                        rc = rc.min(cnt(a[ae]));
                        ae += 1;
                        be += 1;
                    }
                    if next_b < be {
                        next_b = be;
                    }
                    let len = ae - as_;
                    let better = match best {
                        None => true,
                        Some((_, _, blen, bcnt)) => len > blen || rc < bcnt,
                    };
                    if better {
                        best = Some((as_, bs, len, rc));
                    }
                }
            }
        }
        bi = next_b;
    }
    match best {
        None => out.extend(myers(a, b)),
        Some((as_, bs, len, _)) => {
            histogram(&a[..as_], &b[..bs], out);
            out.extend(std::iter::repeat_n(Op::Eq, len));
            histogram(&a[as_ + len..], &b[bs + len..], out);
        }
    }
}

/// Inside each change run, deletions come before insertions.
fn normalize_runs(ops: &mut [Op]) {
    let mut i = 0;
    while i < ops.len() {
        if ops[i] == Op::Eq {
            i += 1;
            continue;
        }
        let start = i;
        while i < ops.len() && ops[i] != Op::Eq {
            i += 1;
        }
        let dels = ops[start..i].iter().filter(|o| **o == Op::Del).count();
        for (j, o) in ops[start..i].iter_mut().enumerate() {
            *o = if j < dels { Op::Del } else { Op::Ins };
        }
    }
}

/// Inserted / deleted line counts of an edit script.
pub fn count(ops: &[Op]) -> (usize, usize) {
    let ins = ops.iter().filter(|o| **o == Op::Ins).count();
    let del = ops.iter().filter(|o| **o == Op::Del).count();
    (ins, del)
}

/// One hunk: op index range plus the line positions where it starts.
#[derive(Debug, Clone, Copy)]
pub struct Hunk {
    pub start: usize,
    pub end: usize,
    pub a0: usize,
    pub b0: usize,
    pub a_len: usize,
    pub b_len: usize,
}

/// Hunk grouping knobs (`-U`, `--inter-hunk-context`, `-W`).
#[derive(Debug, Clone, Copy)]
pub struct HunkOpts {
    pub context: usize,
    pub inter: usize,
    pub function_context: bool,
}

/// Positions `(ai, bi)` before each op (plus the end position).
pub fn positions(ops: &[Op]) -> Vec<(usize, usize)> {
    let mut pos = Vec::with_capacity(ops.len() + 1);
    let (mut ai, mut bi) = (0usize, 0usize);
    for op in ops {
        pos.push((ai, bi));
        match op {
            Op::Eq => {
                ai += 1;
                bi += 1;
            }
            Op::Del => ai += 1,
            Op::Ins => bi += 1,
        }
    }
    pos.push((ai, bi));
    pos
}

/// Group change runs into hunks. `ignorable(i)` marks change ops that
/// `--ignore-blank-lines` / `-I` drop when no other change is near.
pub fn hunks(
    ops: &[Op],
    a: &[&[u8]],
    ho: HunkOpts,
    ignorable: &dyn Fn(usize, usize) -> bool,
    is_func: &dyn Fn(&[u8]) -> bool,
) -> Vec<Hunk> {
    let pos = positions(ops);
    // Change runs as (first op, last op).
    let mut runs: Vec<(usize, usize)> = Vec::new();
    let mut i = 0;
    while i < ops.len() {
        if ops[i] == Op::Eq {
            i += 1;
            continue;
        }
        let s = i;
        while i < ops.len() && ops[i] != Op::Eq {
            i += 1;
        }
        if !ignorable(s, i) {
            runs.push((s, i - 1));
        }
    }
    let ctx = ho.context;
    let mut groups: Vec<(usize, usize)> = Vec::new();
    for (s, e) in runs {
        match groups.last_mut() {
            Some((_, end)) if s <= *end + 2 * ctx + ho.inter + 1 => *end = e,
            _ => groups.push((s, e)),
        }
    }
    let mut out: Vec<(usize, usize)> = Vec::new();
    for (first, last) in groups {
        let mut start = first.saturating_sub(ctx);
        let mut end = (last + ctx + 1).min(ops.len());
        if ho.function_context {
            // Extend back to the function header line before the change
            // and forward to just before the next header.
            let (a_first, _) = pos[first];
            let mut fstart = a_first;
            loop {
                if fstart < a.len() && is_func(a[fstart]) && fstart <= a_first {
                    break;
                }
                if fstart == 0 {
                    break;
                }
                fstart -= 1;
            }
            while start > 0 && pos[start].0 > fstart {
                start -= 1;
            }
            let (a_last, _) = pos[last + 1];
            let mut fend = a_last;
            while fend < a.len() && !is_func(a[fend]) {
                fend += 1;
            }
            // Blank lines before the next function belong to neither.
            while fend > a_last && a[fend - 1].iter().all(|c| c.is_ascii_whitespace()) {
                fend -= 1;
            }
            while end < ops.len() && pos[end].0 < fend {
                end += 1;
            }
        }
        match out.last_mut() {
            Some((_, pend)) if start <= *pend => *pend = end.max(*pend),
            _ => out.push((start, end)),
        }
    }
    out.into_iter()
        .map(|(start, end)| {
            let (a0, b0) = pos[start];
            let a_len = ops[start..end].iter().filter(|o| **o != Op::Ins).count();
            let b_len = ops[start..end].iter().filter(|o| **o != Op::Del).count();
            Hunk {
                start,
                end,
                a0,
                b0,
                a_len,
                b_len,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(a: &str, b: &str, o: &LineOpts) -> Vec<Op> {
        diff_lines(&split_lines(a.as_bytes()), &split_lines(b.as_bytes()), o)
    }

    #[test]
    fn myers_basic() {
        let ops = run("a\nb\nc\n", "a\nx\nc\n", &LineOpts::default());
        assert_eq!(ops, vec![Op::Eq, Op::Del, Op::Ins, Op::Eq]);
    }

    #[test]
    fn whitespace_all_ignores_spaces() {
        let o = LineOpts {
            ignore_all_space: true,
            ..Default::default()
        };
        assert_eq!(run("a  b\n", "ab\n", &o), vec![Op::Eq]);
    }

    #[test]
    fn patience_uses_unique_anchor() {
        let o = LineOpts {
            algo: Algo::Patience,
            ..Default::default()
        };
        let ops = run("x\nu\ny\n", "y\nu\nx\n", &o);
        assert_eq!(count(&ops), (2, 2));
    }
}
