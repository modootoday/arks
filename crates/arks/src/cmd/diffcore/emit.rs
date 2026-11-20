//! Diff output for the diff core in git's `diff_flush` order: raw /
//! name-only / name-status / `--check` lines, then `--numstat`, `--stat`,
//! `--shortstat`, `--dirstat`, `--summary`, and (after a blank separator
//! line when anything was printed) the patch.
//!
//! Primary: <https://git-scm.com/docs/diff-format>,
//! <https://git-scm.com/docs/diff-options>, <https://git-scm.com/docs/git-diff>
//! (§GENERATING PATCH TEXT WITH -P, §COMBINED DIFF FORMAT not covered here).
//!
//! Partial: `--binary` emits literal (not delta) binary patches;
//! `--color-moved` colours moved lines as ordinary changes.

use super::options::{DiffOptions, DirstatMode, MAX_SCORE, WordDiff};
use super::xdiff::{self, HunkOpts, Op};
use super::{Content, MODE_GITLINK, Pair, Side};
use crate::error::CliResult;
use std::io::Write;

const RESET: &str = "\x1b[m";
const META: &str = "\x1b[1m";
const FRAG: &str = "\x1b[36m";
const OLD: &str = "\x1b[31m";
const NEW: &str = "\x1b[32m";
const WS: &str = "\x1b[41m";

/// Caller context that changes defaults between porcelain and plumbing.
#[derive(Clone, Copy, Debug, Default)]
pub struct EmitCtx {
    /// Raw output shows full object names unless `--abbrev` (plumbing).
    pub raw_full: bool,
}

/// git `quote_c_style` for path output (core.quotePath=true).
pub fn quote_path(p: &str) -> String {
    let needs = p
        .bytes()
        .any(|b| b < 0x20 || b == b'"' || b == b'\\' || b >= 0x80 || b == 0x7f);
    if !needs {
        return p.to_string();
    }
    let mut s = String::from("\"");
    for b in p.bytes() {
        match b {
            b'"' => s.push_str("\\\""),
            b'\\' => s.push_str("\\\\"),
            b'\n' => s.push_str("\\n"),
            b'\t' => s.push_str("\\t"),
            0x07 => s.push_str("\\a"),
            0x08 => s.push_str("\\b"),
            0x0c => s.push_str("\\f"),
            0x0d => s.push_str("\\r"),
            0x0b => s.push_str("\\v"),
            b if !(0x20..0x7f).contains(&b) => s.push_str(&format!("\\{b:03o}")),
            b => s.push(b as char),
        }
    }
    s.push('"');
    s
}

/// `a/{b => c}/d` style rename display (git `pprint_rename`).
pub fn pprint_rename(a: &str, b: &str) -> String {
    let (ab, bb) = (a.as_bytes(), b.as_bytes());
    // Common prefix up to and including the last '/'.
    let mut pfx = 0;
    let mut i = 0;
    while i < ab.len() && i < bb.len() && ab[i] == bb[i] {
        if ab[i] == b'/' {
            pfx = i + 1;
        }
        i += 1;
    }
    // Common suffix starting at a '/', not overlapping the prefix.
    let mut sfx = 0;
    let (mut x, mut y) = (ab.len(), bb.len());
    while x > pfx && y > pfx && ab[x - 1] == bb[y - 1] {
        if ab[x - 1] == b'/' {
            sfx = ab.len() - (x - 1);
        }
        x -= 1;
        y -= 1;
    }
    if pfx == 0 && sfx == 0 {
        return format!("{a} => {b}");
    }
    let mid_a = &a[pfx..a.len() - sfx];
    let mid_b = &b[pfx..b.len() - sfx];
    format!(
        "{}{{{} => {}}}{}",
        &a[..pfx],
        mid_a,
        mid_b,
        &a[a.len() - sfx..]
    )
}

struct Out<'a, W: Write> {
    w: &'a mut W,
    prefix: &'a str,
    color: bool,
}

impl<W: Write> Out<'_, W> {
    /// One full line: line prefix, optional colour, text, newline.
    fn line(&mut self, color: &str, text: &str) -> CliResult<()> {
        self.w.write_all(self.prefix.as_bytes())?;
        if self.color && !color.is_empty() {
            write!(self.w, "{color}{text}{RESET}")?;
        } else {
            self.w.write_all(text.as_bytes())?;
        }
        self.w.write_all(b"\n")?;
        Ok(())
    }

    fn raw(&mut self, b: &[u8]) -> CliResult<()> {
        self.w.write_all(b)?;
        Ok(())
    }
}

fn score_pct(score: u32) -> u32 {
    (score as u64 * 100 / MAX_SCORE as u64) as u32
}

fn status_with_score(p: &Pair) -> String {
    let st = p.status();
    if p.is_rename_or_copy() || is_rewrite(p) {
        format!("{st}{:03}", score_pct(p.score))
    } else {
        st.to_string()
    }
}

fn is_rewrite(p: &Pair) -> bool {
    p.status == Some('M') && p.score > 0
}

/// Changed line counts (or binary sizes) of one pair.
enum Stat {
    Lines(usize, usize),
    Binary(usize, usize),
    /// Unmerged entries show no counts.
    Unmerged,
}

fn pair_lines<'b>(
    c: &Content<'_>,
    o: &DiffOptions,
    p: &Pair,
) -> CliResult<(std::rc::Rc<Vec<u8>>, std::rc::Rc<Vec<u8>>, bool)> {
    let empty = || std::rc::Rc::new(Vec::new());
    let a = match &p.old {
        Some(s) if !s.is_tree() => c.bytes(s)?,
        _ => empty(),
    };
    let b = match &p.new {
        Some(s) if !s.is_tree() => c.bytes(s)?,
        _ => empty(),
    };
    let binary = !o.text && (xdiff::is_binary(&a) || xdiff::is_binary(&b));
    Ok((a, b, binary))
}

fn pair_stat(c: &Content<'_>, o: &DiffOptions, p: &Pair) -> CliResult<Stat> {
    if p.unmerged {
        return Ok(Stat::Unmerged);
    }
    let (a, b, _) = pair_lines(c, o, p)?;
    // Diffstat decides binary-ness from the content alone (`-a` is ignored).
    if xdiff::is_binary(&a) || xdiff::is_binary(&b) {
        return Ok(Stat::Binary(a.len(), b.len()));
    }
    let (al, bl) = (xdiff::split_lines(&a), xdiff::split_lines(&b));
    if o.irreversible_delete && p.new.is_none() {
        return Ok(Stat::Lines(0, al.len()));
    }
    let ops = xdiff::diff_lines(&al, &bl, &o.line);
    let (i, d) = xdiff::count(&ops);
    Ok(Stat::Lines(i, d))
}

/// Display name for stat / numstat rows.
fn stat_name(p: &Pair) -> String {
    if p.is_rename_or_copy() {
        pprint_rename(&quote_path(p.old_path()), &quote_path(p.path()))
    } else {
        quote_path(p.path())
    }
}

/// Emit every selected format. Returns `true` when output was produced
/// for at least one pair.
pub fn emit(
    w: &mut impl Write,
    c: &Content<'_>,
    o: &DiffOptions,
    ctx: EmitCtx,
    pairs: &[Pair],
) -> CliResult<()> {
    if o.no_output || pairs.is_empty() {
        return Ok(());
    }
    let color = o.use_color();
    let mut out = Out {
        w,
        prefix: &o.line_prefix,
        color,
    };
    let mut separator = false;
    // name-only / name-status / check override the other formats.
    let exclusive = o.name_only || o.name_status || o.check;
    if o.check {
        check(&mut out, c, o, pairs)?;
        return Ok(());
    }
    if o.raw || o.name_only || o.name_status {
        for p in pairs {
            if o.name_only {
                write_name_only(&mut out, o, p)?;
            } else if o.name_status {
                write_name_status(&mut out, o, p)?;
            } else {
                write_raw(&mut out, c, o, ctx, p)?;
            }
        }
        separator = true;
    }
    if exclusive {
        return Ok(());
    }
    let wants_stats = o.numstat
        || o.stat
        || o.shortstat
        || o.dirstat
            .as_ref()
            .is_some_and(|d| d.mode == DirstatMode::Lines);
    let mut stats: Vec<(usize, Stat)> = Vec::new();
    if wants_stats {
        for (i, p) in pairs.iter().enumerate() {
            if p.old.as_ref().is_some_and(Side::is_tree)
                || p.new.as_ref().is_some_and(Side::is_tree)
            {
                continue;
            }
            // Pure renames / mode changes without content change still list.
            stats.push((i, pair_stat(c, o, p)?));
        }
        if o.numstat {
            for (i, s) in &stats {
                write_numstat(&mut out, o, &pairs[*i], s)?;
            }
        }
        if o.stat {
            write_stat(&mut out, o, pairs, &stats)?;
        }
        if o.shortstat {
            write_shortstat(&mut out, &stats)?;
        }
        if let Some(d) = &o.dirstat {
            if d.mode == DirstatMode::Lines {
                dirstat(&mut out, c, o, pairs, Some(&stats))?;
            }
        }
        separator = true;
    }
    if let Some(d) = &o.dirstat {
        if d.mode != DirstatMode::Lines {
            dirstat(&mut out, c, o, pairs, None)?;
            separator = true;
        }
    }
    if o.summary {
        let mut any = false;
        for p in pairs {
            any |= write_summary(&mut out, p)?;
        }
        separator |= any;
    }
    if o.patch {
        if separator {
            out.line("", "")?;
        }
        for p in pairs {
            write_patch(&mut out, c, o, p)?;
        }
    }
    Ok(())
}

fn write_name_only<W: Write>(out: &mut Out<'_, W>, o: &DiffOptions, p: &Pair) -> CliResult<()> {
    if o.nul {
        out.raw(o.line_prefix.as_bytes())?;
        out.raw(p.path().as_bytes())?;
        out.raw(b"\0")
    } else {
        out.line("", &quote_path(p.path()))
    }
}

fn write_name_status<W: Write>(out: &mut Out<'_, W>, o: &DiffOptions, p: &Pair) -> CliResult<()> {
    let st = status_with_score(p);
    if o.nul {
        out.raw(o.line_prefix.as_bytes())?;
        out.raw(st.as_bytes())?;
        out.raw(b"\0")?;
        if p.is_rename_or_copy() {
            out.raw(p.old_path().as_bytes())?;
            out.raw(b"\0")?;
        }
        out.raw(p.path().as_bytes())?;
        out.raw(b"\0")
    } else if p.is_rename_or_copy() {
        out.line(
            "",
            &format!(
                "{st}\t{}\t{}",
                quote_path(p.old_path()),
                quote_path(p.path())
            ),
        )
    } else {
        out.line("", &format!("{st}\t{}", quote_path(p.path())))
    }
}

/// Object name for raw output.
fn raw_oid(c: &Content<'_>, o: &DiffOptions, ctx: EmitCtx, s: Option<&Side>) -> String {
    let full = c.zero_hex();
    let n = match o.abbrev {
        Some(0) => full.len(),
        Some(n) => n,
        None if ctx.raw_full => full.len(),
        None => 7,
    };
    match s.and_then(|s| s.oid) {
        Some(oid) => {
            if n >= full.len() {
                oid.to_hex()
            } else {
                c.abbrev(&oid, n)
            }
        }
        None => full[..n.min(full.len())].to_string(),
    }
}

fn write_raw<W: Write>(
    out: &mut Out<'_, W>,
    c: &Content<'_>,
    o: &DiffOptions,
    ctx: EmitCtx,
    p: &Pair,
) -> CliResult<()> {
    let mode = |s: &Option<Side>| s.as_ref().map_or(0, |s| s.mode);
    let head = format!(
        ":{:06o} {:06o} {} {} {}",
        mode(&p.old),
        mode(&p.new),
        raw_oid(c, o, ctx, p.old.as_ref()),
        raw_oid(c, o, ctx, p.new.as_ref()),
        status_with_score(p)
    );
    if o.nul {
        out.raw(o.line_prefix.as_bytes())?;
        out.raw(head.as_bytes())?;
        out.raw(b"\0")?;
        if p.is_rename_or_copy() {
            out.raw(p.old_path().as_bytes())?;
            out.raw(b"\0")?;
        }
        out.raw(p.path().as_bytes())?;
        out.raw(b"\0")
    } else if p.is_rename_or_copy() {
        out.line(
            "",
            &format!(
                "{head}\t{}\t{}",
                quote_path(p.old_path()),
                quote_path(p.path())
            ),
        )
    } else {
        out.line("", &format!("{head}\t{}", quote_path(p.path())))
    }
}

fn write_numstat<W: Write>(
    out: &mut Out<'_, W>,
    o: &DiffOptions,
    p: &Pair,
    s: &Stat,
) -> CliResult<()> {
    let counts = match s {
        Stat::Lines(i, d) => format!("{i}\t{d}\t"),
        Stat::Binary(..) | Stat::Unmerged => "-\t-\t".to_string(),
    };
    if o.nul {
        out.raw(o.line_prefix.as_bytes())?;
        out.raw(counts.as_bytes())?;
        if p.is_rename_or_copy() {
            out.raw(b"\0")?;
            out.raw(p.old_path().as_bytes())?;
            out.raw(b"\0")?;
        }
        out.raw(p.path().as_bytes())?;
        out.raw(b"\0")
    } else {
        out.line("", &format!("{counts}{}", stat_name(p)))
    }
}

fn summary_line(files: usize, ins: usize, del: usize) -> String {
    let mut s = format!(" {files} file{} changed", if files == 1 { "" } else { "s" });
    if ins > 0 || del == 0 {
        s.push_str(&format!(
            ", {ins} insertion{}(+)",
            if ins == 1 { "" } else { "s" }
        ));
    }
    if del > 0 || ins == 0 {
        s.push_str(&format!(
            ", {del} deletion{}(-)",
            if del == 1 { "" } else { "s" }
        ));
    }
    s
}

fn totals(stats: &[(usize, Stat)]) -> (usize, usize, usize) {
    let (mut f, mut i, mut d) = (0, 0, 0);
    for (_, s) in stats {
        f += 1;
        if let Stat::Lines(a, b) = s {
            i += a;
            d += b;
        }
    }
    (f, i, d)
}

fn write_shortstat<W: Write>(out: &mut Out<'_, W>, stats: &[(usize, Stat)]) -> CliResult<()> {
    if stats.is_empty() {
        return Ok(());
    }
    let (f, i, d) = totals(stats);
    out.line("", &summary_line(f, i, d))
}

fn compact_suffix(p: &Pair) -> String {
    let exec = |s: &Option<Side>| s.as_ref().is_some_and(|s| s.mode == 0o100755);
    match (&p.old, &p.new) {
        (None, Some(_)) => {
            if exec(&p.new) {
                " (new +x)".into()
            } else if p.new.as_ref().is_some_and(|s| s.mode == super::MODE_LINK) {
                " (new +l)".into()
            } else {
                " (new)".into()
            }
        }
        (Some(_), None) => " (gone)".into(),
        (Some(a), Some(b)) if a.mode != b.mode => {
            if b.mode == super::MODE_LINK {
                " (mode +l)".into()
            } else if a.mode == super::MODE_LINK {
                " (mode -l)".into()
            } else if b.mode == 0o100755 && a.mode == 0o100644 {
                " (mode +x)".into()
            } else if a.mode == 0o100755 && b.mode == 0o100644 {
                " (mode -x)".into()
            } else {
                String::new()
            }
        }
        _ => String::new(),
    }
}

fn write_stat<W: Write>(
    out: &mut Out<'_, W>,
    o: &DiffOptions,
    pairs: &[Pair],
    stats: &[(usize, Stat)],
) -> CliResult<()> {
    if stats.is_empty() {
        return Ok(());
    }
    let shown = o.stat_opts.count.unwrap_or(stats.len()).min(stats.len());
    let names: Vec<String> = stats
        .iter()
        .map(|(i, _)| {
            let mut n = stat_name(&pairs[*i]);
            if o.compact_summary {
                n.push_str(&compact_suffix(&pairs[*i]));
            }
            n
        })
        .collect();
    let max_change = stats[..shown]
        .iter()
        .map(|(_, s)| match s {
            Stat::Lines(i, d) => i + d,
            _ => 0,
        })
        .max()
        .unwrap_or(0);
    let bin_width = stats[..shown]
        .iter()
        .map(|(_, s)| match s {
            Stat::Binary(a, b) => format!("Bin {a} -> {b} bytes").len(),
            Stat::Unmerged => "Unmerged".len(),
            _ => 0,
        })
        .max()
        .unwrap_or(0);
    let has_bin = stats[..shown]
        .iter()
        .any(|(_, s)| matches!(s, Stat::Binary(..) | Stat::Unmerged));
    let max_len = names[..shown]
        .iter()
        .map(|n| n.chars().count())
        .max()
        .unwrap_or(0);
    let mut number_width = max_change.to_string().len();
    if has_bin {
        number_width = number_width.max(3);
    }
    let mut width = o.stat_opts.width.unwrap_or(80);
    if width < 16 + 6 + number_width {
        width = 16 + 6 + number_width;
    }
    let mut graph_width = if max_change + 4 > bin_width {
        max_change
    } else {
        bin_width - 4
    };
    if let Some(g) = o.stat_opts.graph_width {
        if g < graph_width {
            graph_width = g;
        }
    }
    let mut name_width = match o.stat_opts.name_width {
        Some(n) if n > 0 && n < max_len => n,
        _ => max_len,
    };
    if name_width + number_width + 6 + graph_width > width {
        let cap = (width * 3 / 8).saturating_sub(number_width + 6);
        if graph_width > cap {
            graph_width = cap.max(6);
        }
        if let Some(g) = o.stat_opts.graph_width {
            if graph_width > g {
                graph_width = g;
            }
        }
        let room = width.saturating_sub(number_width + 6 + graph_width);
        if name_width > room {
            name_width = room;
        } else {
            graph_width = width.saturating_sub(number_width + 6 + name_width);
        }
    }
    let scale = |it: usize| -> usize {
        if it == 0 {
            0
        } else {
            1 + it * graph_width.saturating_sub(1) / max_change.max(1)
        }
    };
    for (k, (_, s)) in stats.iter().enumerate().take(shown) {
        let name = &names[k];
        let len = name.chars().count();
        let shown_name = if len > name_width {
            let keep = name_width.saturating_sub(3);
            let tail: String = name.chars().skip(len - keep).collect();
            let tail = match tail.find('/') {
                Some(i) => tail[i..].to_string(),
                None => tail,
            };
            format!("...{tail}")
        } else {
            name.clone()
        };
        let pad = name_width.saturating_sub(shown_name.chars().count());
        let lead = format!(" {shown_name}{} |", " ".repeat(pad));
        match s {
            Stat::Lines(i, d) => {
                let total = i + d;
                let (gi, gd) = if max_change <= graph_width {
                    (*i, *d)
                } else {
                    let mut t = scale(total);
                    if t < 2 && *i > 0 && *d > 0 {
                        t = 2;
                    }
                    if i < d {
                        let a = scale(*i);
                        (a, t - a)
                    } else {
                        let dd = scale(*d);
                        (t - dd, dd)
                    }
                };
                let num = format!("{total:>number_width$}");
                out.raw(o.line_prefix.as_bytes())?;
                out.raw(format!("{lead} {num}").as_bytes())?;
                if gi + gd > 0 {
                    out.raw(b" ")?;
                    let plus = "+".repeat(gi);
                    let minus = "-".repeat(gd);
                    if out.color {
                        if gi > 0 {
                            out.raw(format!("{NEW}{plus}{RESET}").as_bytes())?;
                        }
                        if gd > 0 {
                            out.raw(format!("{OLD}{minus}{RESET}").as_bytes())?;
                        }
                    } else {
                        out.raw(plus.as_bytes())?;
                        out.raw(minus.as_bytes())?;
                    }
                }
                out.raw(b"\n")?;
            }
            Stat::Binary(a, b) => {
                let bin = format!("{:>number_width$}", "Bin");
                if out.color {
                    out.line(
                        "",
                        &format!("{lead} {bin} {OLD}{a}{RESET} -> {NEW}{b}{RESET} bytes"),
                    )?;
                } else {
                    out.line("", &format!("{lead} {bin} {a} -> {b} bytes"))?;
                }
            }
            Stat::Unmerged => {
                out.line("", &format!("{lead} Unmerged"))?;
            }
        }
    }
    if shown < stats.len() {
        out.line("", " ...")?;
    }
    let (f, i, d) = totals(stats);
    out.line("", &summary_line(f, i, d))
}

/// `--dirstat`: per-directory share of the damage (tenths of a percent).
fn dirstat<W: Write>(
    out: &mut Out<'_, W>,
    c: &Content<'_>,
    o: &DiffOptions,
    pairs: &[Pair],
    stats: Option<&[(usize, Stat)]>,
) -> CliResult<()> {
    let Some(d) = &o.dirstat else {
        return Ok(());
    };
    let mut files: Vec<(String, u64)> = Vec::new();
    match d.mode {
        DirstatMode::Lines => {
            for (i, s) in stats.unwrap_or(&[]) {
                let n = match s {
                    Stat::Lines(a, b) => (a + b) as u64,
                    // Binary files count 64-byte chunks of change.
                    Stat::Binary(a, b) => {
                        let p = &pairs[*i];
                        let d = if p.old.is_none() || p.new.is_none() {
                            (a + b) as u64
                        } else {
                            a.abs_diff(*b) as u64
                        };
                        d.div_ceil(64)
                    }
                    Stat::Unmerged => 0,
                };
                files.push((pairs[*i].path().to_string(), n));
            }
        }
        DirstatMode::Files => {
            for p in pairs {
                files.push((p.path().to_string(), 1));
            }
        }
        DirstatMode::Changes => {
            for p in pairs {
                let (a, b, _) = pair_lines(c, o, p)?;
                let damage = if p.old.is_none() || p.new.is_none() {
                    (a.len() + b.len()) as u64
                } else {
                    let (copied, added) = super::rename::count_changes(&a, &b);
                    let removed = a.len() as u64 - copied.min(a.len() as u64);
                    // A pure mode change still counts as one changed byte.
                    (removed + added).max(if a.as_slice() == b.as_slice() { 0 } else { 1 })
                };
                files.push((p.path().to_string(), damage));
            }
        }
    }
    files.sort_by(|a, b| a.0.cmp(&b.0));
    let total: u64 = files.iter().map(|f| f.1).sum();
    if total == 0 {
        return Ok(());
    }
    // Walk directories depth first, printing those above the limit; with
    // `noncumulative`, a parent excludes what its printed children took.
    fn gather<W: Write>(
        out: &mut Out<'_, W>,
        files: &[(String, u64)],
        base: &str,
        total: u64,
        d: &super::options::DirstatOpts,
    ) -> CliResult<u64> {
        let mut sum = 0u64;
        let mut i = 0;
        let mut sources = 0;
        while i < files.len() {
            let rest = &files[i].0[base.len()..];
            if let Some(slash) = rest.find('/') {
                let sub = format!("{base}{}", &rest[..=slash]);
                let mut j = i;
                while j < files.len() && files[j].0.starts_with(&sub) {
                    j += 1;
                }
                let got = gather(out, &files[i..j], &sub, total, d)?;
                sum += got;
                if got > 0 {
                    sources += 1;
                }
                i = j;
            } else {
                sum += files[i].1;
                sources += 2;
                i += 1;
            }
        }
        if !base.is_empty() && sum > 0 && (sources > 1 || d.cumulative) {
            let permille = sum * 1000 / total;
            if permille >= d.limit as u64 && permille > 0 {
                out.line(
                    "",
                    &format!("{:>4}.{}% {base}", permille / 10, permille % 10),
                )?;
                if !d.cumulative {
                    return Ok(0);
                }
            }
        }
        Ok(sum)
    }
    gather(out, &files, "", total, d)?;
    Ok(())
}

fn write_summary<W: Write>(out: &mut Out<'_, W>, p: &Pair) -> CliResult<bool> {
    let name = quote_path(p.path());
    match (&p.old, &p.new) {
        (None, Some(n)) => {
            out.line("", &format!(" create mode {:06o} {name}", n.mode))?;
            return Ok(true);
        }
        (Some(o), None) => {
            out.line("", &format!(" delete mode {:06o} {name}", o.mode))?;
            return Ok(true);
        }
        _ => {}
    }
    let mut any = false;
    if p.is_rename_or_copy() {
        let verb = if p.status() == 'R' { "rename" } else { "copy" };
        out.line(
            "",
            &format!(
                " {verb} {} ({}%)",
                pprint_rename(&quote_path(p.old_path()), &name),
                score_pct(p.score)
            ),
        )?;
        any = true;
    } else if is_rewrite(p) {
        out.line("", &format!(" rewrite {name} ({}%)", score_pct(p.score)))?;
        any = true;
    }
    if let (Some(a), Some(b)) = (&p.old, &p.new) {
        if a.mode != b.mode {
            if p.is_rename_or_copy() {
                out.line(
                    "",
                    &format!(" mode change {:06o} => {:06o}", a.mode, b.mode),
                )?;
            } else {
                out.line(
                    "",
                    &format!(" mode change {:06o} => {:06o} {name}", a.mode, b.mode),
                )?;
            }
            any = true;
        }
    }
    Ok(any)
}

/// `--check`: whitespace errors and conflict markers in added lines.
fn check<W: Write>(
    out: &mut Out<'_, W>,
    c: &Content<'_>,
    o: &DiffOptions,
    pairs: &[Pair],
) -> CliResult<()> {
    let mut bad = false;
    for p in pairs {
        if p.new.is_none() {
            continue;
        }
        let (a, b, binary) = pair_lines(c, o, p)?;
        if binary {
            continue;
        }
        let (al, bl) = (xdiff::split_lines(&a), xdiff::split_lines(&b));
        let ops = xdiff::diff_lines(&al, &bl, &o.line);
        let mut bi = 0;
        let mut last_added_blank_at_eof = false;
        for op in &ops {
            match op {
                Op::Eq => {
                    bi += 1;
                    last_added_blank_at_eof = false;
                }
                Op::Del => {}
                Op::Ins => {
                    let line = bl[bi];
                    bi += 1;
                    let body = line.strip_suffix(b"\n").unwrap_or(line);
                    let body = body.strip_suffix(b"\r").unwrap_or(body);
                    let mut errs = Vec::new();
                    if body.ends_with(b" ") || body.ends_with(b"\t") {
                        errs.push("trailing whitespace");
                    }
                    let indent: Vec<u8> = body
                        .iter()
                        .copied()
                        .take_while(|c| *c == b' ' || *c == b'\t')
                        .collect();
                    if let Some(t) = indent.iter().rposition(|c| *c == b'\t') {
                        if indent[..t].contains(&b' ') {
                            errs.push("space before tab in indent");
                        }
                    }
                    let marker = |m: &[u8]| {
                        body.len() >= 7 && body[..7] == *m && (body.len() == 7 || body[7] == b' ')
                    };
                    let conflict = marker(b"<<<<<<<") || marker(b">>>>>>>") || body == b"=======";
                    if !errs.is_empty() {
                        bad = true;
                        out.line(
                            "",
                            &format!("{}:{bi}: {}.", quote_path(p.path()), errs.join(", ")),
                        )?;
                        let mut l = String::from("+");
                        l.push_str(&String::from_utf8_lossy(
                            line.strip_suffix(b"\n").unwrap_or(line),
                        ));
                        out.line("", &l)?;
                    } else if conflict {
                        bad = true;
                        out.line(
                            "",
                            &format!("{}:{bi}: leftover conflict marker", quote_path(p.path())),
                        )?;
                    }
                    last_added_blank_at_eof = body.iter().all(|c| c.is_ascii_whitespace());
                }
            }
        }
        if last_added_blank_at_eof && bi == bl.len() && !bl.is_empty() {
            bad = true;
            out.line(
                "",
                &format!(
                    "{}:{}: new blank line at EOF.",
                    quote_path(p.path()),
                    bl.len()
                ),
            )?;
        }
    }
    if bad {
        CHECK_FAILED.with(|f| f.set(true));
    }
    Ok(())
}

thread_local! {
    static CHECK_FAILED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Did a `--check` run report problems (exit status 2)?
pub fn check_failed() -> bool {
    CHECK_FAILED.with(|f| f.get())
}

/// Default xfuncname: a line starting with a letter, `_` or `$`.
fn is_funcname(line: &[u8]) -> bool {
    line.first()
        .is_some_and(|c| c.is_ascii_alphabetic() || *c == b'_' || *c == b'$')
}

fn funcname<'a>(a: &[&'a [u8]], before: usize) -> Option<&'a [u8]> {
    for line in a[..before.min(a.len())].iter().rev() {
        if is_funcname(line) {
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
    None
}

fn index_hex(
    c: &Content<'_>,
    o: &DiffOptions,
    s: Option<&Side>,
    binary: bool,
) -> CliResult<String> {
    let full = o.full_index || (o.binary && binary);
    let n = match o.abbrev {
        Some(0) => usize::MAX,
        Some(n) => n,
        None => 7,
    };
    Ok(match s {
        Some(s) => {
            let oid = c.oid(s)?;
            if full || n >= oid.to_hex().len() {
                oid.to_hex()
            } else {
                c.abbrev(&oid, n)
            }
        }
        None => {
            let z = c.zero_hex();
            if full {
                z
            } else {
                z[..n.min(z.len())].to_string()
            }
        }
    })
}

fn write_patch<W: Write>(
    out: &mut Out<'_, W>,
    c: &Content<'_>,
    o: &DiffOptions,
    p: &Pair,
) -> CliResult<()> {
    if p.old.as_ref().is_some_and(Side::is_tree) || p.new.as_ref().is_some_and(Side::is_tree) {
        return Ok(());
    }
    if p.unmerged {
        out.line("", &format!("* Unmerged path {}", p.path()))?;
        return Ok(());
    }
    // A type change prints as a deletion followed by a creation.
    if p.status() == 'T' {
        let d = Pair::new(p.old.clone(), None);
        let a = Pair::new(None, p.new.clone());
        write_patch(out, c, o, &d)?;
        return write_patch(out, c, o, &a);
    }
    if all_changes_ignorable(c, o, p)? {
        return Ok(());
    }
    let (sp, dp) = (&o.src_prefix, &o.dst_prefix);
    let old_name = p.old_path();
    let new_name = p.path();
    let q = |pfx: &str, n: &str| quote_path(&format!("{pfx}{n}"));
    out.line(
        META,
        &format!("diff --git {} {}", q(sp, old_name), q(dp, new_name)),
    )?;
    match (&p.old, &p.new) {
        (None, Some(n)) => out.line(META, &format!("new file mode {:06o}", n.mode))?,
        (Some(od), None) => out.line(META, &format!("deleted file mode {:06o}", od.mode))?,
        (Some(a), Some(b)) => {
            if a.mode != b.mode {
                out.line(META, &format!("old mode {:06o}", a.mode))?;
                out.line(META, &format!("new mode {:06o}", b.mode))?;
            }
            if p.is_rename_or_copy() {
                let verb = if p.status() == 'R' { "rename" } else { "copy" };
                out.line(META, &format!("similarity index {}%", score_pct(p.score)))?;
                out.line(META, &format!("{verb} from {}", quote_path(old_name)))?;
                out.line(META, &format!("{verb} to {}", quote_path(new_name)))?;
            } else if is_rewrite(p) {
                out.line(
                    META,
                    &format!("dissimilarity index {}%", score_pct(p.score)),
                )?;
            }
        }
        (None, None) => return Ok(()),
    }
    let (ao, bo) = (
        p.old.as_ref().map(|s| c.oid(s)).transpose()?,
        p.new.as_ref().map(|s| c.oid(s)).transpose()?,
    );
    if ao == bo && p.old.is_some() && p.new.is_some() {
        return Ok(());
    }
    let is_bin = {
        let (a, b, _) = pair_lines(c, o, p)?;
        xdiff::is_binary(&a) || xdiff::is_binary(&b)
    };
    if p.new.as_ref().is_some_and(|s| s.mode == MODE_GITLINK)
        || p.old.as_ref().is_some_and(|s| s.mode == MODE_GITLINK)
    {
        let mut idx = format!(
            "index {}..{}",
            index_hex(c, o, p.old.as_ref(), is_bin)?,
            index_hex(c, o, p.new.as_ref(), is_bin)?
        );
        if let (Some(a), Some(b)) = (&p.old, &p.new) {
            if a.mode == b.mode {
                idx.push_str(&format!(" {:06o}", a.mode));
            }
        }
        out.line(META, &idx)?;
        let line = |s: &Option<Side>| -> CliResult<Option<String>> {
            Ok(s.as_ref()
                .map(|s| c.oid(s))
                .transpose()?
                .map(|o| format!("Subproject commit {}", o.to_hex())))
        };
        let (la, lb) = (line(&p.old)?, line(&p.new)?);
        out.line(
            META,
            &if p.old.is_some() {
                format!("--- {}", q(sp, old_name))
            } else {
                "--- /dev/null".into()
            },
        )?;
        out.line(
            META,
            &if p.new.is_some() {
                format!("+++ {}", q(dp, new_name))
            } else {
                "+++ /dev/null".into()
            },
        )?;
        let (an, bn) = (usize::from(la.is_some()), usize::from(lb.is_some()));
        let r = |s: usize, l: usize| {
            if l == 1 {
                format!("{s}")
            } else {
                format!("{s},{l}")
            }
        };
        out.line(FRAG, &format!("@@ -{} +{} @@", r(an, an), r(bn, bn)))?;
        if let Some(l) = la {
            out.line(OLD, &format!("{}{l}", o.ind_old))?;
        }
        if let Some(l) = lb {
            out.line(NEW, &format!("{}{l}", o.ind_new))?;
        }
        return Ok(());
    }
    let mut idx = format!(
        "index {}..{}",
        index_hex(c, o, p.old.as_ref(), is_bin)?,
        index_hex(c, o, p.new.as_ref(), is_bin)?
    );
    if let (Some(a), Some(b)) = (&p.old, &p.new) {
        if a.mode == b.mode {
            idx.push_str(&format!(" {:06o}", a.mode));
        }
    }
    out.line(META, &idx)?;
    let a_name = if p.old.is_some() {
        q(sp, old_name)
    } else {
        "/dev/null".into()
    };
    let b_name = if p.new.is_some() {
        q(dp, new_name)
    } else {
        "/dev/null".into()
    };
    let (a, b, binary) = pair_lines(c, o, p)?;
    if o.irreversible_delete && p.new.is_none() {
        return Ok(());
    }
    if binary {
        if o.binary {
            return write_binary(out, &a, &b);
        }
        return out.line("", &format!("Binary files {a_name} and {b_name} differ"));
    }
    let al = xdiff::split_lines(&a);
    let bl = xdiff::split_lines(&b);
    if al.is_empty() && bl.is_empty() {
        return Ok(());
    }
    // A rewrite shows the whole old file removed, then the new one.
    let ops = if is_rewrite(p) {
        let mut v = vec![Op::Del; al.len()];
        v.extend(std::iter::repeat_n(Op::Ins, bl.len()));
        v
    } else {
        xdiff::diff_lines(&al, &bl, &o.line)
    };
    let ho = HunkOpts {
        context: o.context(),
        inter: o.inter_hunk,
        function_context: o.function_context,
    };
    let ignorable = |s: usize, e: usize| -> bool { run_ignorable(o, &ops, &al, &bl, s, e) };
    let hunks = xdiff::hunks(&ops, &al, ho, &ignorable, &is_funcname);
    if hunks.is_empty() {
        // Only whitespace / ignorable changes: git prints the header only.
        return Ok(());
    }
    out.line(META, &format!("--- {a_name}"))?;
    out.line(META, &format!("+++ {b_name}"))?;
    let pos = xdiff::positions(&ops);
    for h in hunks {
        let range = |s: usize, l: usize| -> String {
            let s = if l == 0 { s } else { s + 1 };
            if l == 1 {
                format!("{s}")
            } else {
                format!("{s},{l}")
            }
        };
        let head = format!("@@ -{} +{} @@", range(h.a0, h.a_len), range(h.b0, h.b_len));
        out.raw(o.line_prefix.as_bytes())?;
        if out.color {
            out.raw(format!("{FRAG}{head}{RESET}").as_bytes())?;
        } else {
            out.raw(head.as_bytes())?;
        }
        if let Some(f) = funcname(&al, h.a0) {
            out.raw(b" ")?;
            if out.color {
                out.raw(RESET.as_bytes())?;
                out.raw(f)?;
                out.raw(RESET.as_bytes())?;
            } else {
                out.raw(f)?;
            }
        }
        out.raw(b"\n")?;
        if let Some(wd) = &o.word_diff {
            word_hunk(
                out,
                o,
                wd,
                &ops[h.start..h.end],
                &pos[h.start..h.end],
                &al,
                &bl,
            )?;
            continue;
        }
        for (i, op) in ops.iter().enumerate().take(h.end).skip(h.start) {
            let (ai, bi) = pos[i];
            let (ind, color, line) = match op {
                // Context lines come from the new side (whitespace options).
                Op::Eq => (&o.ind_ctx, "", bl[bi]),
                Op::Del => (&o.ind_old, OLD, al[ai]),
                Op::Ins => (&o.ind_new, NEW, bl[bi]),
            };
            write_body_line(out, ind, color, line, *op == Op::Ins)?;
            if !line.ends_with(b"\n") {
                out.raw(o.line_prefix.as_bytes())?;
                out.raw(b"\\ No newline at end of file")?;
                if out.color {
                    out.raw(RESET.as_bytes())?;
                }
                out.raw(b"\n")?;
            }
        }
    }
    Ok(())
}

/// `-I` / `--ignore-blank-lines`: a text pair whose every change is
/// ignorable produces no patch at all.
fn all_changes_ignorable(c: &Content<'_>, o: &DiffOptions, p: &Pair) -> CliResult<bool> {
    if !o.line.ignore_blank_lines && o.line.ignore_regex.is_empty() {
        return Ok(false);
    }
    if p.old.is_none() || p.new.is_none() {
        return Ok(false);
    }
    let (a, b, binary) = pair_lines(c, o, p)?;
    if binary {
        return Ok(false);
    }
    let (al, bl) = (xdiff::split_lines(&a), xdiff::split_lines(&b));
    let ops = xdiff::diff_lines(&al, &bl, &o.line);
    let mut i = 0;
    let mut any = false;
    while i < ops.len() {
        if ops[i] == Op::Eq {
            i += 1;
            continue;
        }
        let s = i;
        while i < ops.len() && ops[i] != Op::Eq {
            i += 1;
        }
        any = true;
        if !run_ignorable(o, &ops, &al, &bl, s, i) {
            return Ok(false);
        }
    }
    Ok(any)
}

fn run_ignorable(
    o: &DiffOptions,
    ops: &[Op],
    al: &[&[u8]],
    bl: &[&[u8]],
    s: usize,
    e: usize,
) -> bool {
    if !o.line.ignore_blank_lines && o.line.ignore_regex.is_empty() {
        return false;
    }
    let pos = xdiff::positions(&ops[..e]);
    (s..e).all(|i| {
        let (ai, bi) = pos[i];
        let line = match ops[i] {
            Op::Del => al[ai],
            Op::Ins => bl[bi],
            Op::Eq => return true,
        };
        let body = line.strip_suffix(b"\n").unwrap_or(line);
        (o.line.ignore_blank_lines && body.iter().all(|c| c.is_ascii_whitespace()))
            || o.line.ignore_regex.iter().any(|r| r.is_match(body))
    })
}

/// One `+`/`-`/context body line with git's colouring (added lines show
/// trailing whitespace highlighted).
fn write_body_line<W: Write>(
    out: &mut Out<'_, W>,
    ind: &str,
    color: &str,
    line: &[u8],
    added: bool,
) -> CliResult<()> {
    let body = line.strip_suffix(b"\n").unwrap_or(line);
    out.raw(out.prefix.as_bytes())?;
    if !out.color {
        out.raw(ind.as_bytes())?;
        out.raw(body)?;
        return out.raw(b"\n");
    }
    if added {
        let trimmed = {
            let mut e = body.len();
            while e > 0 && (body[e - 1] == b' ' || body[e - 1] == b'\t') {
                e -= 1;
            }
            e
        };
        out.raw(format!("{color}{ind}{RESET}").as_bytes())?;
        // Leading indentation up to each tab is written uncoloured
        // (spaces before a tab are a whitespace error).
        let mut written = 0;
        for i in 0..trimmed {
            match body[i] {
                b' ' => continue,
                b'\t' => {
                    if body[written..i].contains(&b' ') {
                        out.raw(WS.as_bytes())?;
                        out.raw(&body[written..i])?;
                        out.raw(RESET.as_bytes())?;
                        out.raw(b"\t")?;
                    } else {
                        out.raw(&body[written..=i])?;
                    }
                    written = i + 1;
                }
                _ => break,
            }
        }
        if trimmed > written {
            out.raw(color.as_bytes())?;
            out.raw(&body[written..trimmed])?;
            out.raw(RESET.as_bytes())?;
        }
        if trimmed < body.len() {
            out.raw(WS.as_bytes())?;
            out.raw(&body[trimmed.max(written)..])?;
            out.raw(RESET.as_bytes())?;
        }
        return out.raw(b"\n");
    }
    out.raw(color.as_bytes())?;
    out.raw(ind.as_bytes())?;
    out.raw(body)?;
    out.raw(RESET.as_bytes())?;
    out.raw(b"\n")
}

/// Words of a text: maximal runs of non-whitespace (default word regex)
/// as byte ranges.
fn words(t: &[u8], re: Option<&crate::cmd::bre::Bre>) -> Vec<(usize, usize)> {
    let mut v = Vec::new();
    match re {
        Some(r) => {
            let mut from = 0;
            while from < t.len() {
                match r.find_from(t, from) {
                    Some((s, e)) if e > s => {
                        // Words never span a newline.
                        let e = t[s..e]
                            .iter()
                            .position(|c| *c == b'\n')
                            .map_or(e, |k| s + k);
                        if e > s {
                            v.push((s, e));
                        }
                        from = e.max(s + 1);
                    }
                    Some((s, _)) => from = s + 1,
                    None => break,
                }
            }
        }
        None => {
            let mut i = 0;
            while i < t.len() {
                if t[i].is_ascii_whitespace() {
                    i += 1;
                    continue;
                }
                let s = i;
                while i < t.len() && !t[i].is_ascii_whitespace() {
                    i += 1;
                }
                v.push((s, i));
            }
        }
    }
    v
}

/// Word-diff rendering of one hunk (`--word-diff`, `--color-words`).
fn word_hunk<W: Write>(
    out: &mut Out<'_, W>,
    o: &DiffOptions,
    mode: &WordDiff,
    ops: &[Op],
    pos: &[(usize, usize)],
    al: &[&[u8]],
    bl: &[&[u8]],
) -> CliResult<()> {
    let re = o
        .word_regex
        .as_ref()
        .map(|r| crate::cmd::bre::Bre::with_mode(r, false, true));
    let mut i = 0;
    while i < ops.len() {
        if ops[i] == Op::Eq {
            let line = bl[pos[i].1];
            match mode {
                WordDiff::Porcelain => {
                    out.raw(out.prefix.as_bytes())?;
                    out.raw(b" ")?;
                    out.raw(line.strip_suffix(b"\n").unwrap_or(line))?;
                    out.raw(b"\n")?;
                    out.raw(out.prefix.as_bytes())?;
                    out.raw(b"~\n")?;
                }
                _ => {
                    out.raw(out.prefix.as_bytes())?;
                    out.raw(line.strip_suffix(b"\n").unwrap_or(line))?;
                    if out.color {
                        out.raw(RESET.as_bytes())?;
                    }
                    out.raw(b"\n")?;
                }
            }
            i += 1;
            continue;
        }
        let mut minus = Vec::new();
        let mut plus = Vec::new();
        while i < ops.len() && ops[i] != Op::Eq {
            match ops[i] {
                Op::Del => minus.extend_from_slice(al[pos[i].0]),
                Op::Ins => plus.extend_from_slice(bl[pos[i].1]),
                Op::Eq => {}
            }
            i += 1;
        }
        word_block(out, mode, &minus, &plus, re.as_ref())?;
    }
    Ok(())
}

fn word_block<W: Write>(
    out: &mut Out<'_, W>,
    mode: &WordDiff,
    minus: &[u8],
    plus: &[u8],
    re: Option<&crate::cmd::bre::Bre>,
) -> CliResult<()> {
    let mw = words(minus, re);
    let pw = words(plus, re);
    let mlines: Vec<&[u8]> = mw.iter().map(|(s, e)| &minus[*s..*e]).collect();
    let plines: Vec<&[u8]> = pw.iter().map(|(s, e)| &plus[*s..*e]).collect();
    let ops = xdiff::diff_lines(&mlines, &plines, &Default::default());
    let mut text: Vec<u8> = Vec::new();
    let color = out.color || *mode == WordDiff::Color;
    let porcelain = *mode == WordDiff::Porcelain;
    let emit_seg = |text: &mut Vec<u8>, kind: char, seg: &[u8]| {
        if seg.is_empty() {
            return;
        }
        for (k, part) in seg.split(|c| *c == b'\n').enumerate() {
            if k > 0 {
                text.extend_from_slice(if porcelain { b"~\n" } else { b"\n" });
            }
            if part.is_empty() {
                continue;
            }
            match (kind, porcelain, color) {
                (_, true, _) => {
                    text.push(kind as u8);
                    text.extend_from_slice(part);
                    text.push(b'\n');
                }
                (' ', false, _) => text.extend_from_slice(part),
                ('-', false, true) => {
                    text.extend_from_slice(OLD.as_bytes());
                    text.extend_from_slice(part);
                    text.extend_from_slice(RESET.as_bytes());
                }
                ('+', false, true) => {
                    text.extend_from_slice(NEW.as_bytes());
                    text.extend_from_slice(part);
                    text.extend_from_slice(RESET.as_bytes());
                }
                ('-', false, false) => {
                    text.extend_from_slice(b"[-");
                    text.extend_from_slice(part);
                    text.extend_from_slice(b"-]");
                }
                (_, false, false) => {
                    text.extend_from_slice(b"{+");
                    text.extend_from_slice(part);
                    text.extend_from_slice(b"+}");
                }
                _ => {}
            }
        }
    };
    // Walk the word script: common text is taken from the plus side.
    let (mut mi, mut pi) = (0usize, 0usize);
    let mut plus_at = 0usize;
    let mut k = 0;
    while k < ops.len() {
        if ops[k] == Op::Eq {
            mi += 1;
            pi += 1;
            k += 1;
            continue;
        }
        let (ms, ps) = (mi, pi);
        while k < ops.len() && ops[k] != Op::Eq {
            match ops[k] {
                Op::Del => mi += 1,
                Op::Ins => pi += 1,
                Op::Eq => {}
            }
            k += 1;
        }
        // Common text up to the change.
        let upto = if ps < pw.len() {
            pw[ps].0
        } else if ps > 0 {
            pw[ps - 1].1
        } else {
            0
        };
        emit_seg(&mut text, ' ', &plus[plus_at..upto.max(plus_at)]);
        plus_at = upto.max(plus_at);
        if mi > ms {
            emit_seg(&mut text, '-', &minus[mw[ms].0..mw[mi - 1].1]);
        }
        if pi > ps {
            emit_seg(&mut text, '+', &plus[pw[ps].0..pw[pi - 1].1]);
            plus_at = pw[pi - 1].1;
        }
    }
    let tail = &plus[plus_at.min(plus.len())..];
    if porcelain {
        if !tail.is_empty() {
            emit_seg(&mut text, ' ', tail);
        }
        if !text.ends_with(b"~\n") {
            text.extend_from_slice(b"~\n");
        }
        for line in text.split_inclusive(|c| *c == b'\n') {
            out.raw(out.prefix.as_bytes())?;
            out.raw(line)?;
        }
        return Ok(());
    }
    emit_seg(&mut text, ' ', tail);
    if plus.is_empty() && !text.ends_with(b"\n") {
        text.push(b'\n');
    }
    for line in text.split_inclusive(|c| *c == b'\n') {
        out.raw(out.prefix.as_bytes())?;
        out.raw(line)?;
    }
    if !text.ends_with(b"\n") {
        out.raw(b"\n")?;
    }
    Ok(())
}

/// git base85 alphabet (binary patches).
const B85: &[u8; 85] =
    b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz!#$%&()*+-;<=>?@^_`{|}~";

fn base85_line(chunk: &[u8]) -> String {
    let n = chunk.len();
    let mut s = String::new();
    s.push(if n <= 26 {
        (b'A' + n as u8 - 1) as char
    } else {
        (b'a' + n as u8 - 27) as char
    });
    for group in chunk.chunks(4) {
        let mut acc: u32 = 0;
        for k in 0..4 {
            acc = (acc << 8) | *group.get(k).unwrap_or(&0) as u32;
        }
        let mut enc = [0u8; 5];
        for e in enc.iter_mut().rev() {
            *e = B85[(acc % 85) as usize];
            acc /= 85;
        }
        s.push_str(std::str::from_utf8(&enc).unwrap_or(""));
    }
    s
}

fn write_binary<W: Write>(out: &mut Out<'_, W>, a: &[u8], b: &[u8]) -> CliResult<()> {
    out.line("", "GIT binary patch")?;
    for data in [b, a] {
        let z = arks_object::zlib_compress(data)
            .map_err(|e| crate::error::CliError::Msg(e.to_string()))?;
        out.line("", &format!("literal {}", data.len()))?;
        for chunk in z.chunks(52) {
            out.line("", &base85_line(chunk))?;
        }
        out.line("", "")?;
    }
    Ok(())
}
