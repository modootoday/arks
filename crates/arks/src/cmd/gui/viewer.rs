//! `git gui blame` and `git gui browser` as terminal views.
//!
//! `blame [--line=<n>] [<rev>] <file>` annotates the file (the working copy
//! when no revision is given) and opens a scrolling view centred on line
//! `<n>`; `browser [<rev>]` lists every file of the revision's tree and opens
//! the blame view for the selected one. Without a terminal both print their
//! content once (the annotation, or one path per line).
//! Cite: <https://git-scm.com/docs/git-gui> (COMMANDS, EXAMPLES).

use super::term::{Key, Keys};
use crate::cmd::shared::{resolve_rev, tree_blob_map};
use crate::error::{CliError, CliResult};
use crate::util::open_repo;
use std::io::{Read, Write};

/// Annotation lines for `[<rev>] <file>`.
pub fn blame_lines(rev: Option<&str>, file: &str) -> CliResult<Vec<String>> {
    let mut args = Vec::new();
    if let Some(r) = rev {
        args.push(r.to_string());
    }
    args.push("--".into());
    args.push(file.into());
    let mut buf = Vec::new();
    crate::cmd::blame::blame_to(&args, &mut buf)?;
    Ok(String::from_utf8_lossy(&buf)
        .lines()
        .map(str::to_string)
        .collect())
}

/// Every file path in the tree of `rev`, sorted.
pub fn tree_paths(rev: &str) -> CliResult<Vec<String>> {
    let repo = open_repo()?;
    let oid = resolve_rev(&repo, rev)
        .map_err(|_| CliError::fatal(format!("invalid revision '{rev}'")))?;
    let mut paths: Vec<String> = tree_blob_map(&repo, &oid)?.into_keys().collect();
    paths.sort();
    Ok(paths)
}

fn fit(s: &str, w: usize) -> String {
    let mut out: String = s
        .chars()
        .map(|c| if c == '\t' { ' ' } else { c })
        .filter(|c| !c.is_control())
        .take(w)
        .collect();
    let n = out.chars().count();
    out.extend(std::iter::repeat_n(' ', w - n));
    out
}

/// A scrolling list. With `select`, Enter returns the highlighted index;
/// `q`, Esc, Ctrl-C or end of input return `None`.
pub fn list_view<R: Read>(
    title: &str,
    lines: &[String],
    start: usize,
    select: bool,
    keys: &mut Keys<R>,
    out: &mut dyn Write,
    (w, h): (usize, usize),
) -> CliResult<Option<usize>> {
    let body = h.saturating_sub(2).max(1);
    let mut cur = start.min(lines.len().saturating_sub(1));
    let mut top = cur.saturating_sub(body / 2);
    out.write_all(b"\x1b[2J")?;
    loop {
        if cur < top {
            top = cur;
        } else if cur >= top + body {
            top = cur + 1 - body;
        }
        let mut f = String::from("\x1b[H");
        f.push_str(&format!("\x1b[7m{}\x1b[0m\r\n", fit(title, w)));
        for i in 0..body {
            let text = fit(lines.get(top + i).map(String::as_str).unwrap_or(""), w);
            if top + i == cur && !lines.is_empty() {
                f.push_str(&format!("\x1b[7m{text}\x1b[0m\r\n"));
            } else {
                f.push_str(&format!("{text}\r\n"));
            }
        }
        let help = if select {
            " Up/Down move  PageUp/PageDown scroll  Enter open  q close"
        } else {
            " Up/Down move  PageUp/PageDown scroll  q close"
        };
        f.push_str(&format!("\x1b[7m{}\x1b[0m", fit(help, w)));
        out.write_all(f.as_bytes())?;
        out.flush()?;
        let last = lines.len().saturating_sub(1);
        match keys.next_key() {
            Key::Eof | Key::Ctrl('c') | Key::Esc | Key::Char('q') => return Ok(None),
            Key::Up | Key::Char('k') => cur = cur.saturating_sub(1),
            Key::Down | Key::Char('j') => cur = (cur + 1).min(last),
            Key::PageUp => cur = cur.saturating_sub(body),
            Key::PageDown | Key::Char(' ') => cur = (cur + body).min(last),
            Key::Home | Key::Char('g') => cur = 0,
            Key::End | Key::Char('G') => cur = last,
            Key::Enter if select && !lines.is_empty() => return Ok(Some(cur)),
            _ => {}
        }
    }
}
