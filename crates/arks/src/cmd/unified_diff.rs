//! Unified diff emitter (git `diff --git` patch shape).
//!
//! Myers O((N+M)·D) line diff, 3 lines of context, hunks merged when the gap is
//! at most twice the context, default funcname heuristic (last preceding line
//! starting with a letter, `_` or `$`, trimmed, 80 bytes), `index` header,
//! new/deleted file headers, binary detection, `\ No newline at end of file`.
//!
//! Primary: <https://git-scm.com/docs/git-diff> (§GENERATING PATCH TEXT WITH -P),
//! <https://git-scm.com/docs/diff-format>.
//! Cite: `.spec/sot/cloud-agent-dropin.sot.md` §O.
//!
//! Partial: file modes are reported as `100644`; rename detection is not done.

use crate::error::CliResult;
use arks_object::{ObjectKind, RawObject};
use arks_store::ObjectFormat;
use std::io::Write;

const CONTEXT: usize = 3;
/// Above this many lines on both sides combined, fall back to one replace hunk.
const MAX_LINES: usize = 400_000;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Op {
    Eq,
    Del,
    Ins,
}

/// Emit one file's patch. `None` = side absent (new / deleted file).
pub fn write_file_patch(
    out: &mut impl Write,
    format: ObjectFormat,
    path: &str,
    old: Option<&[u8]>,
    new: Option<&[u8]>,
) -> CliResult<()> {
    let abbrev = |b: Option<&[u8]>| -> CliResult<String> {
        Ok(match b {
            Some(bytes) => RawObject::new(ObjectKind::Blob, bytes.to_vec())
                .oid(format)?
                .to_hex()[..7]
                .to_string(),
            None => "0000000".to_string(),
        })
    };
    let (ao, bo) = (abbrev(old)?, abbrev(new)?);
    writeln!(out, "diff --git a/{path} b/{path}")?;
    match (old, new) {
        (None, Some(_)) => {
            writeln!(out, "new file mode 100644")?;
            writeln!(out, "index {ao}..{bo}")?;
        }
        (Some(_), None) => {
            writeln!(out, "deleted file mode 100644")?;
            writeln!(out, "index {ao}..{bo}")?;
        }
        _ => writeln!(out, "index {ao}..{bo} 100644")?,
    }
    let a_name = if old.is_some() {
        format!("a/{path}")
    } else {
        "/dev/null".into()
    };
    let b_name = if new.is_some() {
        format!("b/{path}")
    } else {
        "/dev/null".into()
    };
    let a = old.unwrap_or(b"");
    let b = new.unwrap_or(b"");
    if is_binary(a) || is_binary(b) {
        writeln!(out, "Binary files {a_name} and {b_name} differ")?;
        return Ok(());
    }
    let al = split_lines(a);
    let bl = split_lines(b);
    if al.is_empty() && bl.is_empty() {
        return Ok(());
    }
    writeln!(out, "--- {a_name}")?;
    writeln!(out, "+++ {b_name}")?;
    let script = if al.len() + bl.len() > MAX_LINES {
        let mut s = vec![Op::Del; al.len()];
        s.extend(std::iter::repeat_n(Op::Ins, bl.len()));
        s
    } else {
        myers(&al, &bl)
    };
    emit_hunks(out, &al, &bl, &script)
}

/// One `--stat` row: path plus `(insertions, deletions)`, or binary sizes.
pub struct StatRow {
    pub path: String,
    pub change: StatChange,
}

pub enum StatChange {
    Lines(usize, usize),
    Binary(usize, usize),
}

/// Count changed lines (`None` side = absent file).
pub fn stat_change(old: Option<&[u8]>, new: Option<&[u8]>) -> StatChange {
    let a = old.unwrap_or(b"");
    let b = new.unwrap_or(b"");
    if is_binary(a) || is_binary(b) {
        return StatChange::Binary(a.len(), b.len());
    }
    let al = split_lines(a);
    let bl = split_lines(b);
    if al.len() + bl.len() > MAX_LINES {
        return StatChange::Lines(bl.len(), al.len());
    }
    let ops = myers(&al, &bl);
    let ins = ops.iter().filter(|o| **o == Op::Ins).count();
    let del = ops.iter().filter(|o| **o == Op::Del).count();
    StatChange::Lines(ins, del)
}

/// `git diff --stat` block (default 80-column stat width, diff.c `show_stats`).
pub fn write_stat(out: &mut impl Write, rows: &[StatRow]) -> CliResult<()> {
    if rows.is_empty() {
        return Ok(());
    }
    const WIDTH: usize = 80;
    let max_change = rows
        .iter()
        .map(|r| match r.change {
            StatChange::Lines(i, d) => i + d,
            StatChange::Binary(..) => 0,
        })
        .max()
        .unwrap_or(0);
    let has_bin = rows
        .iter()
        .any(|r| matches!(r.change, StatChange::Binary(..)));
    let mut number_width = max_change.to_string().len();
    if has_bin {
        number_width = number_width.max(3);
    }
    let mut name_width = rows
        .iter()
        .map(|r| r.path.chars().count())
        .max()
        .unwrap_or(0);
    let mut graph_width = max_change;
    if name_width + number_width + 6 + graph_width > WIDTH {
        let gw_cap = (WIDTH * 3 / 8).saturating_sub(number_width + 6).max(6);
        if graph_width > gw_cap {
            graph_width = gw_cap;
        }
        let name_cap = WIDTH.saturating_sub(number_width + 6 + graph_width);
        if name_width > name_cap {
            name_width = name_cap;
        } else {
            // diff.c: names fit, so the graph takes the remaining columns.
            graph_width = WIDTH.saturating_sub(number_width + 6 + name_width);
        }
    }
    let scale = |it: usize| -> usize {
        if max_change <= graph_width || it == 0 {
            it
        } else {
            1 + it * (graph_width.saturating_sub(1)) / max_change
        }
    };
    let (mut files, mut tot_ins, mut tot_del) = (0usize, 0usize, 0usize);
    for r in rows {
        files += 1;
        let name = if r.path.chars().count() > name_width {
            let keep = name_width.saturating_sub(3);
            let tail: String = r
                .path
                .chars()
                .rev()
                .take(keep)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();
            // diff.c: prefer cutting at a directory boundary inside the kept tail.
            let tail = match tail.find('/') {
                Some(i) => tail[i..].to_string(),
                None => tail,
            };
            format!("...{tail}")
        } else {
            r.path.clone()
        };
        match r.change {
            StatChange::Lines(i, d) => {
                tot_ins += i;
                tot_del += d;
                let total = i + d;
                // diff.c show_stats: scale the total, then the smaller side.
                let (gi, gd) = if max_change <= graph_width {
                    (i, d)
                } else {
                    let mut t = scale(total);
                    if t < 2 && i > 0 && d > 0 {
                        t = 2;
                    }
                    if i < d {
                        let a = scale(i);
                        (a, t - a)
                    } else {
                        let dd = scale(d);
                        (t - dd, dd)
                    }
                };
                let bar = format!("{}{}", "+".repeat(gi), "-".repeat(gd));
                let sep = if bar.is_empty() { "" } else { " " };
                writeln!(
                    out,
                    " {name:<name_width$} | {total:>number_width$}{sep}{bar}"
                )?;
            }
            StatChange::Binary(a, b) => {
                writeln!(
                    out,
                    " {name:<name_width$} | {:>number_width$} {a} -> {b} bytes",
                    "Bin"
                )?;
            }
        }
    }
    let mut summary = format!(" {files} file{} changed", if files == 1 { "" } else { "s" });
    if tot_ins > 0 || tot_del == 0 {
        summary.push_str(&format!(
            ", {tot_ins} insertion{}(+)",
            if tot_ins == 1 { "" } else { "s" }
        ));
    }
    if tot_del > 0 || tot_ins == 0 {
        summary.push_str(&format!(
            ", {tot_del} deletion{}(-)",
            if tot_del == 1 { "" } else { "s" }
        ));
    }
    writeln!(out, "{summary}")?;
    Ok(())
}

fn is_binary(b: &[u8]) -> bool {
    b[..b.len().min(8000)].contains(&0)
}

/// Lines including their trailing `\n` (last line may lack it).
fn split_lines(b: &[u8]) -> Vec<&[u8]> {
    if b.is_empty() {
        return Vec::new();
    }
    b.split_inclusive(|c| *c == b'\n').collect()
}

/// Edit script (one op per consumed line) via Myers' greedy algorithm with trace.
fn myers(a: &[&[u8]], b: &[&[u8]]) -> Vec<Op> {
    let n = a.len() as isize;
    let m = b.len() as isize;
    let max = (n + m) as usize;
    let off = max as isize + 1;
    let mut v = vec![0isize; 2 * max + 3];
    let mut trace: Vec<Vec<isize>> = Vec::new();
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
    // Backtrack.
    let mut ops = Vec::with_capacity(max);
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
            ops.push(Op::Eq);
            x -= 1;
            y -= 1;
        }
        if x == prev_x {
            ops.push(Op::Ins);
        } else {
            ops.push(Op::Del);
        }
        x = prev_x;
        y = prev_y;
    }
    while x > 0 && y > 0 {
        ops.push(Op::Eq);
        x -= 1;
        y -= 1;
    }
    ops.reverse();
    // Git prints deletions before insertions within a change run.
    normalize_runs(&mut ops);
    ops
}

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

fn emit_hunks(out: &mut impl Write, a: &[&[u8]], b: &[&[u8]], ops: &[Op]) -> CliResult<()> {
    // Positions (ai, bi) before each op.
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
    let changes: Vec<usize> = (0..ops.len()).filter(|&i| ops[i] != Op::Eq).collect();
    if changes.is_empty() {
        return Ok(());
    }
    // Group change indices into hunks.
    let mut groups: Vec<(usize, usize)> = Vec::new();
    for &c in &changes {
        match groups.last_mut() {
            Some((_, end)) if c <= *end + 2 * CONTEXT + 1 => *end = c,
            _ => groups.push((c, c)),
        }
    }
    for (first, last) in groups {
        let start = first.saturating_sub(CONTEXT);
        let end = (last + CONTEXT + 1).min(ops.len());
        let (a0, b0) = pos[start];
        let a_len = ops[start..end].iter().filter(|o| **o != Op::Ins).count();
        let b_len = ops[start..end].iter().filter(|o| **o != Op::Del).count();
        let range = |s: usize, l: usize| -> String {
            let s = if l == 0 { s } else { s + 1 };
            if l == 1 {
                format!("{s}")
            } else {
                format!("{s},{l}")
            }
        };
        let func = funcname(a, a0);
        write!(out, "@@ -{} +{} @@", range(a0, a_len), range(b0, b_len))?;
        if let Some(f) = func {
            out.write_all(b" ")?;
            out.write_all(f)?;
        }
        writeln!(out)?;
        for (i, op) in ops.iter().enumerate().take(end).skip(start) {
            let (ai, bi) = pos[i];
            let (sigil, line) = match op {
                Op::Eq => (b' ', a[ai]),
                Op::Del => (b'-', a[ai]),
                Op::Ins => (b'+', b[bi]),
            };
            out.write_all(&[sigil])?;
            out.write_all(line)?;
            if !line.ends_with(b"\n") {
                out.write_all(b"\n\\ No newline at end of file\n")?;
            }
        }
    }
    Ok(())
}

/// Default xfuncname: nearest line before `before` starting with alpha, `_`, `$`.
fn funcname<'a>(a: &[&'a [u8]], before: usize) -> Option<&'a [u8]> {
    for line in a[..before].iter().rev() {
        if let Some(&c) = line.first() {
            if c.is_ascii_alphabetic() || c == b'_' || c == b'$' {
                let mut l = &line[..line.len().min(80)];
                while let Some((&last, rest)) = l.split_last() {
                    if last.is_ascii_whitespace() {
                        l = rest;
                    } else {
                        break;
                    }
                }
                return Some(l);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn patch(old: Option<&str>, new: Option<&str>) -> String {
        let mut v = Vec::new();
        write_file_patch(
            &mut v,
            ObjectFormat::Sha1,
            "f",
            old.map(str::as_bytes),
            new.map(str::as_bytes),
        )
        .unwrap();
        String::from_utf8(v).unwrap()
    }

    #[test]
    fn middle_change_has_context_and_index() {
        let old = "1\n2\n3\n4\n5\n6\n7\n8\n9\n";
        let new = "1\n2\n3\n4\nX\n6\n7\n8\n9\n";
        let p = patch(Some(old), Some(new));
        assert!(p.contains("index "), "{p}");
        assert!(
            p.contains("@@ -2,7 +2,7 @@\n 2\n 3\n 4\n-5\n+X\n 6\n 7\n 8\n"),
            "{p}"
        );
    }

    #[test]
    fn new_file_and_no_newline() {
        let p = patch(None, Some("a\nb"));
        assert!(p.contains("new file mode 100644\nindex 0000000.."), "{p}");
        assert!(
            p.contains(
                "--- /dev/null\n+++ b/f\n@@ -0,0 +1,2 @@\n+a\n+b\n\\ No newline at end of file\n"
            ),
            "{p}"
        );
    }

    #[test]
    fn single_line_ranges_omit_count() {
        let p = patch(Some("a\n"), Some("b\n"));
        assert!(p.contains("@@ -1 +1 @@\n-a\n+b\n"), "{p}");
    }

    #[test]
    fn distant_changes_split_hunks() {
        let old: String = (1..=30).map(|i| format!("L{i}\n")).collect();
        let new = old.replace("L2\n", "two\n").replace("L28\n", "x\n");
        let p = patch(Some(&old), Some(&new));
        assert_eq!(p.matches("@@ -").count(), 2, "{p}");
    }
}
