//! Patch output for `range-diff`'s "diff of diffs": the two patch texts of a
//! commit pair compared line by line, printed without file headers, with
//! `@@` hunk headers that carry no line numbers, every line indented by four
//! spaces, and (by default) dual colouring.
//!
//! Primary: <https://git-scm.com/docs/git-range-diff> (§DESCRIPTION,
//! `--no-dual-color`, §OUTPUT STABILITY). Layout and colours follow observed
//! stock git output: the hunk header names the nearest section above it (a
//! ` ## <name> ##` line or an inner `@@ <name>` line); with dual colouring a
//! removed or added outer line shows its marker in reverse video and the
//! inner line dimmed (removed) or bold (added), coloured by the inner
//! line's own `-` / `+` / `@@` marker.

use super::emit::{FRAG, NEW, OLD, Out, RESET};
use super::options::DiffOptions;
use super::xdiff::{self, HunkOpts, Op};
use super::{Content, Pair};
use crate::error::CliResult;
use std::io::Write;

const REVERSE: &str = "\x1b[7m";

/// How `range-diff` colours its diff of diffs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RangeDiffStyle {
    /// Colour the inner diff's markers too (`--dual-color`, the default).
    pub dual_color: bool,
}

/// Section name a hunk header shows for `line`.
fn section(line: &[u8]) -> Option<&[u8]> {
    let body = line.strip_suffix(b"\n").unwrap_or(line);
    if let Some(name) = body
        .strip_prefix(b" ## ")
        .and_then(|r| r.strip_suffix(b" ##"))
    {
        return Some(name);
    }
    body.strip_prefix(b"@@ ").filter(|r| !r.is_empty())
}

pub(super) fn write_patch<W: Write>(
    out: &mut Out<'_, W>,
    c: &Content<'_>,
    o: &DiffOptions,
    p: &Pair,
    style: RangeDiffStyle,
) -> CliResult<()> {
    let (a, b, _) = super::emit::pair_lines(c, o, p)?;
    if a == b {
        return Ok(());
    }
    let al = xdiff::split_lines(&a);
    let bl = xdiff::split_lines(&b);
    let ops = xdiff::diff_lines(&al, &bl, &o.line);
    let ho = HunkOpts {
        context: o.context(),
        inter: o.inter_hunk,
        function_context: o.function_context,
    };
    let ignorable =
        |s: usize, e: usize| -> bool { super::emit::run_ignorable(o, &ops, &al, &bl, s, e) };
    let hunks = xdiff::hunks(&ops, &al, ho, &ignorable, &|l| section(l).is_some());
    let pos = xdiff::positions(&ops);
    for h in hunks {
        out.raw(out.prefix.as_bytes())?;
        if out.color {
            if style.dual_color {
                out.raw(REVERSE.as_bytes())?;
            }
            out.raw(format!("{FRAG}@@{RESET}").as_bytes())?;
        } else {
            out.raw(b"@@")?;
        }
        if let Some(f) = super::emit::funcname_by(&al, h.a0, &section) {
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
            super::emit::word_hunk(
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
            let line = match op {
                Op::Eq => bl[bi],
                Op::Del => al[ai],
                Op::Ins => bl[bi],
            };
            write_line(out, o, *op, line, style)?;
        }
    }
    Ok(())
}

/// Colour of an inner line by its own marker: (context, dimmed, bold).
fn inner_colors(body: &[u8]) -> (&'static str, &'static str, &'static str) {
    match body.first() {
        Some(b'-') => (OLD, "\x1b[2;31m", "\x1b[1;31m"),
        Some(b'+') => (NEW, "\x1b[2;32m", "\x1b[1;32m"),
        Some(b'@') => (FRAG, FRAG, FRAG),
        _ => ("", "\x1b[2m", "\x1b[1m"),
    }
}

fn write_line<W: Write>(
    out: &mut Out<'_, W>,
    o: &DiffOptions,
    op: Op,
    line: &[u8],
    style: RangeDiffStyle,
) -> CliResult<()> {
    let (ind, color) = match op {
        Op::Eq => (&o.ind_ctx, ""),
        Op::Del => (&o.ind_old, OLD),
        Op::Ins => (&o.ind_new, NEW),
    };
    if !out.color || !style.dual_color {
        return super::emit::write_body_line(out, ind, color, line, op == Op::Ins);
    }
    let body = line.strip_suffix(b"\n").unwrap_or(line);
    let (ctx, dim, bold) = inner_colors(body);
    out.raw(out.prefix.as_bytes())?;
    match op {
        // With whitespace errors highlighted on context lines the marker
        // and the text are coloured separately.
        Op::Eq if o.ws_highlight_context => {
            out.raw(format!("{ctx}{ind}{RESET}").as_bytes())?;
            if !body.is_empty() {
                out.raw(ctx.as_bytes())?;
                out.raw(body)?;
                out.raw(RESET.as_bytes())?;
            }
        }
        Op::Eq => {
            out.raw(ctx.as_bytes())?;
            out.raw(ind.as_bytes())?;
            out.raw(body)?;
            out.raw(RESET.as_bytes())?;
        }
        Op::Del | Op::Ins => {
            out.raw(format!("{REVERSE}{color}{ind}{RESET}").as_bytes())?;
            if !body.is_empty() {
                let inner = if op == Op::Del { dim } else { bold };
                out.raw(inner.as_bytes())?;
                out.raw(body)?;
                out.raw(RESET.as_bytes())?;
            }
        }
    }
    out.raw(b"\n")
}

#[cfg(test)]
mod tests {
    use super::section;

    #[test]
    fn section_names() {
        assert_eq!(section(b" ## Metadata ##\n"), Some(&b"Metadata"[..]));
        assert_eq!(section(b"@@ a.c: main\n"), Some(&b"a.c: main"[..]));
        assert_eq!(section(b"@@\n"), None);
        assert_eq!(section(b"    text\n"), None);
    }
}
