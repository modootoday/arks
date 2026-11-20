//! Full-screen terminal front-end of the staging / commit window: the two
//! file lists on the left, the selected file's diff on the right, the commit
//! message box and a key-help line at the bottom.
//!
//! Keys: Up/Down (`k`/`j`) select, Tab switches list, Enter/Space stage or
//! unstage the selected file, `S` "Stage Changed", `e` edits the message (Esc
//! ends editing), `a` toggles "Amend Last Commit", `o` "Sign Off", `c`
//! commits ("Done" with `--nocommit`), `r` rescans, PageUp/PageDown scroll
//! the diff, `q` or Ctrl-C closes the window.

use super::model::{Committed, Model, Row, Side};
use super::script::End;
use super::term::{Key, Keys};
use crate::error::CliResult;
use std::io::{Read, Write};

struct Ui {
    focus: Side,
    sel: [usize; 2],
    editing: bool,
    scroll: usize,
    status: String,
    diff_key: Option<(Side, String)>,
    diff: Vec<String>,
}

fn idx(side: Side) -> usize {
    match side {
        Side::Unstaged => 0,
        Side::Staged => 1,
    }
}

fn rows(m: &Model, side: Side) -> &[Row] {
    match side {
        Side::Unstaged => &m.unstaged,
        Side::Staged => &m.staged,
    }
}

fn other(side: Side) -> Side {
    match side {
        Side::Unstaged => Side::Staged,
        Side::Staged => Side::Unstaged,
    }
}

/// Printable text cut to `w` columns (tabs expanded, controls dropped).
fn fit(s: &str, w: usize) -> String {
    let mut out = String::new();
    let mut n = 0;
    for c in s.chars() {
        if c == '\t' {
            for _ in 0..(4 - n % 4) {
                if n < w {
                    out.push(' ');
                    n += 1;
                }
            }
            continue;
        }
        if c.is_control() {
            continue;
        }
        if n >= w {
            break;
        }
        out.push(c);
        n += 1;
    }
    out.extend(std::iter::repeat_n(' ', w - n));
    out
}

impl Ui {
    fn selected<'a>(&self, m: &'a Model) -> Option<&'a Row> {
        rows(m, self.focus).get(self.sel[idx(self.focus)])
    }

    fn clamp(&mut self, m: &Model) {
        for side in [Side::Unstaged, Side::Staged] {
            let n = rows(m, side).len();
            let s = &mut self.sel[idx(side)];
            if *s >= n {
                *s = n.saturating_sub(1);
            }
        }
        if rows(m, self.focus).is_empty() && !rows(m, other(self.focus)).is_empty() {
            self.focus = other(self.focus);
        }
    }

    fn refresh_diff(&mut self, m: &Model) {
        let key = self.selected(m).map(|r| (self.focus, r.path.clone()));
        if key == self.diff_key {
            return;
        }
        self.scroll = 0;
        self.diff = match &key {
            Some((side, path)) => match m.diff(*side, path) {
                Ok(t) => t.lines().map(str::to_string).collect(),
                Err(e) => vec![format!("error: {e}")],
            },
            None => vec!["No changes.".into()],
        };
        self.diff_key = key;
    }

    fn render(&mut self, m: &Model, out: &mut dyn Write, (w, h): (usize, usize)) -> CliResult<()> {
        self.refresh_diff(m);
        let mut f = String::from("\x1b[H");
        let mode = if m.amend {
            "Amend Last Commit"
        } else {
            "New Commit"
        };
        let title = format!(" arks gui   Current Branch: {}   [{mode}]", m.branch);
        f.push_str(&format!("\x1b[7m{}\x1b[0m\r\n", fit(&title, w)));
        let body = h.saturating_sub(8).max(2);
        let lw = (w * 2 / 5).clamp(20, w.saturating_sub(20).max(20));
        let rw = w.saturating_sub(lw + 1);
        let mut left: Vec<String> = Vec::new();
        let half = body / 2;
        for (side, title, cap) in [
            (Side::Unstaged, "Unstaged Changes", half),
            (Side::Staged, "Staged Changes (Will Commit)", body - half),
        ] {
            left.push(format!("\x1b[1m{}\x1b[0m", fit(title, lw)));
            let list = rows(m, side);
            let room = cap.saturating_sub(1);
            let sel = self.sel[idx(side)];
            let start = if sel >= room { sel + 1 - room } else { 0 };
            for i in 0..room {
                match list.get(start + i) {
                    Some(r) => {
                        let text = fit(&format!(" {} {}", r.state, r.path), lw);
                        if side == self.focus && start + i == sel && !self.editing {
                            left.push(format!("\x1b[7m{text}\x1b[0m"));
                        } else {
                            left.push(text);
                        }
                    }
                    None => left.push(fit("", lw)),
                }
            }
        }
        for row in 0..body {
            let l = left.get(row).cloned().unwrap_or_else(|| fit("", lw));
            let d = self.diff.get(self.scroll + row).map(String::as_str);
            let color = match d {
                Some(s) if s.starts_with('+') && !s.starts_with("+++") => "\x1b[32m",
                Some(s) if s.starts_with('-') && !s.starts_with("---") => "\x1b[31m",
                Some(s) if s.starts_with("@@") => "\x1b[36m",
                Some(s) if s.starts_with("diff ") => "\x1b[1m",
                _ => "",
            };
            let text = fit(d.unwrap_or(""), rw);
            f.push_str(&format!("{l}|{color}{text}\x1b[0m\r\n"));
        }
        let hint = if self.editing {
            "Commit Message: (editing; Esc ends editing)"
        } else {
            "Commit Message:"
        };
        f.push_str(&format!("\x1b[1m{}\x1b[0m\r\n", fit(hint, w)));
        let lines: Vec<&str> = m.message.split('\n').collect();
        let skip = if self.editing {
            lines.len().saturating_sub(4)
        } else {
            0
        };
        for i in 0..4 {
            let mut text = lines.get(skip + i).copied().unwrap_or("").to_string();
            if self.editing && skip + i + 1 == lines.len() {
                text.push('_');
            }
            f.push_str(&format!("{}\r\n", fit(&format!("  {text}"), w)));
        }
        let commit_word = if m.nocommit { "done" } else { "commit" };
        let help = format!(
            " Up/Down select  Tab list  Enter stage/unstage  S stage changed  e message  a amend  o sign off  c {commit_word}  r rescan  q quit"
        );
        f.push_str(&format!("\x1b[7m{}\x1b[0m\r\n", fit(&help, w)));
        f.push_str(&fit(&self.status, w));
        out.write_all(f.as_bytes())?;
        out.flush()?;
        Ok(())
    }

    fn act(&mut self, m: &mut Model, r: CliResult<()>) {
        match r {
            Ok(()) => self.status.clear(),
            Err(e) => {
                self.status = format!("error: {}", e.to_string().trim_start_matches("error: "))
            }
        }
        self.diff_key = None;
        self.clamp(m);
    }

    fn move_sel(&mut self, m: &Model, down: bool) {
        let n = rows(m, self.focus).len();
        let s = &mut self.sel[idx(self.focus)];
        if down {
            if *s + 1 < n {
                *s += 1;
            } else if !rows(m, other(self.focus)).is_empty() && self.focus == Side::Unstaged {
                self.focus = Side::Staged;
                self.sel[1] = 0;
            }
        } else if *s > 0 {
            *s -= 1;
        } else if self.focus == Side::Staged && !m.unstaged.is_empty() {
            self.focus = Side::Unstaged;
            self.sel[0] = m.unstaged.len() - 1;
        }
    }
}

/// Run the window until a commit (commit-tool mode) or close.
pub fn run<R: Read>(
    m: &mut Model,
    keys: &mut Keys<R>,
    out: &mut dyn Write,
    size: (usize, usize),
    citool: bool,
) -> CliResult<End> {
    let mut ui = Ui {
        focus: Side::Unstaged,
        sel: [0, 0],
        editing: false,
        scroll: 0,
        status: String::new(),
        diff_key: None,
        diff: Vec::new(),
    };
    ui.clamp(m);
    out.write_all(b"\x1b[2J")?;
    loop {
        ui.render(m, out, size)?;
        let key = keys.next_key();
        if matches!(key, Key::Eof | Key::Ctrl('c')) {
            return Ok(End::Closed);
        }
        if ui.editing {
            match key {
                Key::Esc => ui.editing = false,
                Key::Enter => m.message.push('\n'),
                Key::Backspace => {
                    m.message.pop();
                }
                Key::Tab => m.message.push('\t'),
                Key::Char(c) => m.message.push(c),
                _ => {}
            }
            continue;
        }
        let body = size.1.saturating_sub(8).max(2);
        match key {
            Key::Char('q') => return Ok(End::Closed),
            Key::Up | Key::Char('k') => ui.move_sel(m, false),
            Key::Down | Key::Char('j') => ui.move_sel(m, true),
            Key::Tab | Key::Left | Key::Right => {
                if !rows(m, other(ui.focus)).is_empty() {
                    ui.focus = other(ui.focus);
                }
            }
            Key::Enter | Key::Char(' ') | Key::Char('s') | Key::Char('u') => {
                if let Some(r) = ui.selected(m).cloned() {
                    let res = m.toggle(ui.focus, &r.path);
                    ui.act(m, res);
                }
            }
            Key::Char('S') => {
                let res = m.stage_changed();
                ui.act(m, res);
            }
            Key::Char('e') | Key::Char('i') | Key::Char('m') => ui.editing = true,
            Key::Char('a') => {
                let res = m.set_amend(!m.amend);
                ui.act(m, res);
            }
            Key::Char('o') => {
                let res = m.signoff();
                ui.act(m, res);
            }
            Key::Char('r') | Key::Ctrl('r') => {
                let res = m.rescan();
                ui.act(m, res);
            }
            Key::Char('c') => {
                out.write_all(b"\x1b[2J")?;
                match m.commit() {
                    Ok(Committed::Done) => return Ok(End::Committed),
                    Ok(Committed::Created { abbrev, subject }) => {
                        if citool {
                            return Ok(End::Committed);
                        }
                        ui.act(m, Ok(()));
                        ui.status = format!("Created commit {abbrev}: {subject}");
                    }
                    Err(e) => ui.act(m, Err(e)),
                }
            }
            Key::PageDown | Key::Char('J') => {
                ui.scroll = (ui.scroll + body / 2).min(ui.diff.len().saturating_sub(1));
            }
            Key::PageUp | Key::Char('K') => ui.scroll = ui.scroll.saturating_sub(body / 2),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_pads_and_cuts() {
        assert_eq!(fit("ab", 4), "ab  ");
        assert_eq!(fit("abcdef", 3), "abc");
        assert_eq!(fit("\tx", 6), "    x ");
        assert_eq!(fit("a\x07b", 3), "ab ");
    }
}
