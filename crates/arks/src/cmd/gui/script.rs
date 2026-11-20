//! Line-driven front-end of the staging / commit screen, used when standard
//! input is not a terminal (scripts, tests, CI). Each input line is one
//! screen action; blank lines and `#` comments are skipped:
//!
//! | line | screen action |
//! |------|---------------|
//! | `status` | print both file lists |
//! | `stage <path>...` / `unstage <path>...` | click a file in a list |
//! | `stage-changed` | "Stage Changed" |
//! | `diff <path>` | select a file and show its diff |
//! | `message <text>` | type one line into the commit message |
//! | `clear-message` | empty the message |
//! | `amend` / `new` | "Amend Last Commit" / "New Commit" |
//! | `signoff` | "Sign Off" |
//! | `rescan` | "Rescan" |
//! | `commit` | "Commit" ("Done" with `citool --nocommit`) |
//! | `quit` | close the window |
//!
//! A failed action prints `error: ...` on stderr and the script goes on,
//! like a dialog box the user dismisses.

use super::model::{Committed, Model, Row};
use crate::error::{CliError, CliResult};
use std::io::{BufRead, Write};

/// What ended the session.
#[derive(Debug, PartialEq, Eq)]
pub enum End {
    /// One commit was made (or `--nocommit` finished) in commit-tool mode.
    Committed,
    /// The window was closed (quit or end of input).
    Closed,
}

fn print_rows(out: &mut dyn Write, title: &str, rows: &[Row]) -> CliResult<()> {
    writeln!(out, "{title}:")?;
    for r in rows {
        writeln!(out, "  {} {}", r.state, r.path)?;
    }
    Ok(())
}

/// Print the main-window summary (`status` action).
pub fn print_status(out: &mut dyn Write, m: &Model) -> CliResult<()> {
    let mode = if m.amend {
        "Amend Last Commit"
    } else {
        "New Commit"
    };
    writeln!(out, "Current Branch: {}  [{mode}]", m.branch)?;
    print_rows(out, "Unstaged Changes", &m.unstaged)?;
    print_rows(out, "Staged Changes (Will Commit)", &m.staged)?;
    Ok(())
}

fn need_paths(cmd: &str, rest: &str) -> CliResult<Vec<String>> {
    let paths: Vec<String> = rest.split_whitespace().map(str::to_string).collect();
    if paths.is_empty() {
        return Err(CliError::Msg(format!("{cmd}: no path given")));
    }
    Ok(paths)
}

fn step(m: &mut Model, line: &str, citool: bool, out: &mut dyn Write) -> CliResult<Option<End>> {
    let (cmd, rest) = line.split_once(' ').unwrap_or((line, ""));
    match cmd {
        "status" => print_status(out, m)?,
        "stage" => {
            for p in need_paths(cmd, rest)? {
                m.stage(&p)?;
            }
        }
        "unstage" => {
            for p in need_paths(cmd, rest)? {
                m.unstage(&p)?;
            }
        }
        "stage-changed" => m.stage_changed()?,
        "diff" => {
            let path = rest.trim();
            let side = m
                .side_of(path)
                .ok_or_else(|| CliError::Msg(format!("diff: no changes for '{path}'")))?;
            out.write_all(m.diff(side, path)?.as_bytes())?;
        }
        "message" => {
            m.message.push_str(rest);
            m.message.push('\n');
        }
        "clear-message" => m.message.clear(),
        "amend" => m.set_amend(true)?,
        "new" => m.set_amend(false)?,
        "signoff" => m.signoff()?,
        "rescan" => m.rescan()?,
        "commit" => match m.commit()? {
            Committed::Done => return Ok(Some(End::Committed)),
            Committed::Created { abbrev, subject } => {
                writeln!(out, "Created commit {abbrev}: {subject}")?;
                if citool {
                    return Ok(Some(End::Committed));
                }
            }
        },
        "quit" | "exit" => return Ok(Some(End::Closed)),
        other => return Err(CliError::Msg(format!("unknown action '{other}'"))),
    }
    Ok(None)
}

/// Run actions from `input` until a commit (commit-tool mode), `quit` or EOF.
pub fn run(m: &mut Model, input: &mut dyn BufRead, citool: bool) -> CliResult<End> {
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let mut line = String::new();
    loop {
        line.clear();
        if input.read_line(&mut line)? == 0 {
            return Ok(End::Closed);
        }
        let l = line.trim_end_matches(['\r', '\n']).trim_start();
        if l.is_empty() || l.starts_with('#') {
            continue;
        }
        match step(m, l, citool, &mut out) {
            Ok(Some(end)) => return Ok(end),
            Ok(None) => {}
            Err(e) => {
                out.flush()?;
                eprintln!("error: {}", e.to_string().trim_start_matches("error: "));
            }
        }
        out.flush()?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cmd::gui::model::Side;

    #[test]
    fn side_names() {
        assert_ne!(Side::Staged, Side::Unstaged);
        assert!(need_paths("stage", "  ").is_err());
        assert_eq!(need_paths("stage", "a b").unwrap(), vec!["a", "b"]);
    }
}
