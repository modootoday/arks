//! `git gui`, `git citool` and `git gui.tcl` as a terminal UI, plus the
//! `git gui--askpass` / `git gui--askyesno` prompts.
//!
//! git ships these as a Tcl/Tk program; arks keeps the command names and argv
//! (`gui [blame|browser|citool|version]`, `citool [--amend] [--nocommit]`)
//! and draws the windows in the terminal (owner decision: no windowing
//! toolkit). When standard input is not a terminal the staging / commit
//! window reads one action per line instead (see [`script`]), so it can be
//! scripted and tested. `ARKS_GUI_TERMINAL=force` draws the full-screen view
//! on any input (tests), `=never` always uses the line front-end.
//!
//! Cite: <https://git-scm.com/docs/git-gui>, <https://git-scm.com/docs/git-citool>
//! (behaviour, not code). Plan: `.spec/plan/20261120000000-argv-symmetry-ladder.plan.md`
//! (A3 design decisions 1).

mod model;
mod prompt;
mod screen;
mod script;
mod term;
mod viewer;

pub use prompt::{cmd_gui_askpass, cmd_gui_askyesno};

use crate::cmd::opt_parse::{Mode, OptDef, Takes, o, parse};
use crate::error::{CliError, CliResult};
use script::End;
use std::io::Write;

const GUI_USAGE: &str = "usage: git gui [<command>] [<arguments>]\n\n\
    commands:\n   blame [--line=<n>] [<rev>] <file>\n   browser [<rev>]\n   \
    citool [--amend] [--nocommit]\n   version";

const CITOOL_OPTS: &[OptDef] = &[
    o("amend", ' ', Takes::No, false),
    o("nocommit", ' ', Takes::No, false),
];
const BLAME_OPTS: &[OptDef] = &[o("line", ' ', Takes::Req, false)];

fn usage_exit(text: &str) -> CliError {
    CliError::Exit(129, text.to_string())
}

/// Full-screen views on a terminal (or when forced), the line front-end otherwise.
fn use_screen() -> bool {
    match std::env::var("ARKS_GUI_TERMINAL").as_deref() {
        Ok("force") => true,
        Ok("never") => false,
        _ => term::is_terminal(),
    }
}

/// Alternate screen + raw mode for the guard's lifetime.
struct Screen {
    _raw: Option<term::RawMode>,
}

impl Screen {
    fn enter() -> CliResult<Self> {
        let raw = term::is_terminal().then(term::RawMode::enter);
        let mut out = std::io::stdout();
        out.write_all(b"\x1b[?1049h\x1b[?25l")?;
        out.flush()?;
        Ok(Screen { _raw: raw })
    }
}

impl Drop for Screen {
    fn drop(&mut self) {
        let mut out = std::io::stdout();
        let _ = out.write_all(b"\x1b[0m\x1b[?25h\x1b[?1049l");
        let _ = out.flush();
    }
}

fn keys() -> term::Keys<std::io::Stdin> {
    term::Keys::new(std::io::stdin(), term::is_terminal())
}

/// The staging / commit window; `citool` ends after one commit.
fn run_window(citool: bool, amend: bool, nocommit: bool) -> CliResult<()> {
    let mut m = model::Model::open(amend, nocommit)?;
    let end = if use_screen() {
        let guard = Screen::enter()?;
        let mut out = std::io::stdout();
        let end = screen::run(&mut m, &mut keys(), &mut out, term::size(), citool);
        drop(guard);
        end?
    } else {
        script::run(&mut m, &mut std::io::stdin().lock(), citool)?
    };
    match end {
        // git-gui: the commit tool exits non-zero unless it made its commit.
        End::Closed if citool => Err(CliError::silent(1)),
        _ => Ok(()),
    }
}

/// `git citool [--amend] [--nocommit]`.
pub fn cmd_citool(args: &[String]) -> CliResult<()> {
    let p = parse("citool", args, CITOOL_OPTS, Mode::default())?;
    if !p.args().is_empty() || !p.after_dashdash().is_empty() {
        return Err(usage_exit("usage: git citool [--amend] [--nocommit]"));
    }
    run_window(true, p.on("amend"), p.on("nocommit"))
}

fn gui_blame(args: &[String]) -> CliResult<()> {
    let p = parse("gui blame", args, BLAME_OPTS, Mode::default())?;
    let mut pos = p.args();
    pos.extend(p.after_dashdash());
    let (rev, file) = match pos.as_slice() {
        [f] => (None, f.clone()),
        [r, f] => (Some(r.clone()), f.clone()),
        _ => {
            return Err(usage_exit(
                "usage: git gui blame [--line=<n>] [<rev>] <file>",
            ));
        }
    };
    let line = match p.value("line") {
        Some(v) => v
            .parse::<usize>()
            .map_err(|_| CliError::fatal(format!("invalid line number '{v}'")))?,
        None => 1,
    };
    let lines = viewer::blame_lines(rev.as_deref(), &file)?;
    if !use_screen() {
        let mut out = std::io::stdout().lock();
        for l in &lines {
            writeln!(out, "{l}")?;
        }
        return Ok(());
    }
    let title = format!(
        " File: {file}   Commit: {}",
        rev.as_deref().unwrap_or("Working Copy")
    );
    let _g = Screen::enter()?;
    viewer::list_view(
        &title,
        &lines,
        line.saturating_sub(1),
        false,
        &mut keys(),
        &mut std::io::stdout(),
        term::size(),
    )?;
    Ok(())
}

fn gui_browser(args: &[String]) -> CliResult<()> {
    let p = parse("gui browser", args, &[], Mode::default())?;
    let mut pos = p.args();
    pos.extend(p.after_dashdash());
    let rev = match pos.as_slice() {
        [] => "HEAD".to_string(),
        [r] => r.clone(),
        _ => return Err(usage_exit("usage: git gui browser [<rev>]")),
    };
    let paths = viewer::tree_paths(&rev)?;
    if !use_screen() {
        let mut out = std::io::stdout().lock();
        for l in &paths {
            writeln!(out, "{l}")?;
        }
        return Ok(());
    }
    let _g = Screen::enter()?;
    let mut k = keys();
    let mut out = std::io::stdout();
    let mut at = 0;
    let title = format!(" File Browser   Commit: {rev}");
    while let Some(i) = viewer::list_view(&title, &paths, at, true, &mut k, &mut out, term::size())?
    {
        at = i;
        let lines = viewer::blame_lines(Some(&rev), &paths[i])?;
        let t = format!(" File: {}   Commit: {rev}", paths[i]);
        viewer::list_view(&t, &lines, 0, false, &mut k, &mut out, term::size())?;
    }
    Ok(())
}

/// `git gui [blame|browser|citool|version] [<arguments>]`; also `git gui.tcl`.
pub fn cmd_gui(args: &[String]) -> CliResult<()> {
    let Some(first) = args.first() else {
        return run_window(false, false, false);
    };
    let rest = &args[1..];
    match first.as_str() {
        "citool" => cmd_citool(rest),
        "blame" => gui_blame(rest),
        "browser" => gui_browser(rest),
        "version" | "--version" => {
            if !rest.is_empty() {
                return Err(usage_exit("usage: git gui version"));
            }
            println!("arks gui version {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        "-h" | "--help" => Err(usage_exit(GUI_USAGE)),
        other => Err(CliError::Exit(
            129,
            format!("error: unknown gui command '{other}'\n{GUI_USAGE}"),
        )),
    }
}
