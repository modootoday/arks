//! Diff / merge tool selection for `difftool` and `mergetool`: the built-in
//! tool names git documents (`--tool-help`), `diff.tool` / `merge.tool`
//! (and their `guitool` forms), `<mode>tool.<tool>.cmd|path|trustExitCode`,
//! and launching the selected program with `LOCAL`, `REMOTE`, `MERGED` and
//! `BASE` in its environment.
//!
//! Primary: <https://git-scm.com/docs/git-difftool>,
//! <https://git-scm.com/docs/git-mergetool>, <https://git-scm.com/docs/git-config>
//! (`diff.tool`, `merge.tool`, `difftool.*`, `mergetool.*`). Built-in command
//! lines follow each tool's own documented command-line interface.

use super::proc;
use crate::error::{CliError, CliResult};
use arks_config::ConfigSet;
use std::path::Path;

/// Which front end is asking.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Diff,
    Merge,
}

impl Mode {
    fn word(self) -> &'static str {
        match self {
            Mode::Diff => "diff",
            Mode::Merge => "merge",
        }
    }
}

/// One built-in tool.
struct Builtin {
    name: &'static str,
    /// Program looked up on PATH when `<mode>tool.<name>.path` is unset.
    exe: &'static str,
    /// Description in `--tool-help` (diff mode / merge mode).
    diff_desc: Option<&'static str>,
    merge_desc: Option<&'static str>,
    /// Shell snippets; `$T` is the program path.
    diff: &'static str,
    merge_base: &'static str,
    merge_nobase: &'static str,
}

macro_rules! tool {
    ($name:expr, $exe:expr, $dd:expr, $md:expr, $diff:expr, $mb:expr, $mn:expr) => {
        Builtin {
            name: $name,
            exe: $exe,
            diff_desc: $dd,
            merge_desc: $md,
            diff: $diff,
            merge_base: $mb,
            merge_nobase: $mn,
        }
    };
}

const VIM_D: &str = r#""$T" -R -f -d -c 'wincmd l' "$LOCAL" "$REMOTE""#;
const VIM_MB: &str = r#""$T" -f -d -c '4wincmd w | wincmd J' "$LOCAL" "$BASE" "$REMOTE" "$MERGED""#;
const VIM_MN: &str = r#""$T" -f -d -c 'wincmd l' "$LOCAL" "$MERGED" "$REMOTE""#;
const VIM1: &str = r#""$T" -f -d "$LOCAL" "$REMOTE" && cp "$LOCAL" "$MERGED""#;
const VIM2: &str = r#""$T" -f -d -c 'wincmd l' "$LOCAL" "$MERGED" "$REMOTE""#;
const VIM3: &str = r#""$T" -f "$MERGED""#;

/// Built-in tools in `--tool-help` order. Descriptions are the strings git
/// prints; the `(requires a graphical session)` suffix marks windowed tools.
fn builtins() -> Vec<Builtin> {
    vec![
        tool!(
            "araxis",
            "compare",
            Some("Use Araxis Merge (requires a graphical session)"),
            Some("Use Araxis Merge (requires a graphical session)"),
            r#""$T" -wait -2 "$LOCAL" "$REMOTE""#,
            r#""$T" -wait -merge -3 -a1 "$BASE" "$LOCAL" "$REMOTE" "$MERGED""#,
            r#""$T" -wait -2 "$LOCAL" "$REMOTE" "$MERGED""#
        ),
        tool!(
            "bc",
            "bcompare",
            Some("Use Beyond Compare (requires a graphical session)"),
            Some("Use Beyond Compare (requires a graphical session)"),
            r#""$T" "$LOCAL" "$REMOTE""#,
            r#""$T" "$LOCAL" "$REMOTE" "$BASE" -mergeoutput="$MERGED""#,
            r#""$T" "$LOCAL" "$REMOTE" -mergeoutput="$MERGED""#
        ),
        tool!(
            "bc3",
            "bcompare",
            Some("Use Beyond Compare (requires a graphical session)"),
            Some("Use Beyond Compare (requires a graphical session)"),
            r#""$T" "$LOCAL" "$REMOTE""#,
            r#""$T" "$LOCAL" "$REMOTE" "$BASE" -mergeoutput="$MERGED""#,
            r#""$T" "$LOCAL" "$REMOTE" -mergeoutput="$MERGED""#
        ),
        tool!(
            "bc4",
            "bcompare",
            Some("Use Beyond Compare (requires a graphical session)"),
            Some("Use Beyond Compare (requires a graphical session)"),
            r#""$T" "$LOCAL" "$REMOTE""#,
            r#""$T" "$LOCAL" "$REMOTE" "$BASE" -mergeoutput="$MERGED""#,
            r#""$T" "$LOCAL" "$REMOTE" -mergeoutput="$MERGED""#
        ),
        tool!(
            "codecompare",
            "CodeMerge",
            Some("Use Code Compare (requires a graphical session)"),
            Some("Use Code Compare (requires a graphical session)"),
            r#""$T" "$LOCAL" "$REMOTE""#,
            r#""$T" -MF="$LOCAL" -TF="$REMOTE" -BF="$BASE" -RF="$MERGED""#,
            r#""$T" -MF="$LOCAL" -TF="$REMOTE" -RF="$MERGED""#
        ),
        tool!(
            "deltawalker",
            "DeltaWalker",
            Some("Use DeltaWalker (requires a graphical session)"),
            Some("Use DeltaWalker (requires a graphical session)"),
            r#""$T" -nosplash "$LOCAL" "$REMOTE""#,
            r#""$T" "$LOCAL" "$REMOTE" "$BASE" -pwd="$PWD" -merged="$MERGED""#,
            r#""$T" "$LOCAL" "$REMOTE" -pwd="$PWD" -merged="$MERGED""#
        ),
        tool!(
            "diffmerge",
            "diffmerge",
            Some("Use DiffMerge (requires a graphical session)"),
            Some("Use DiffMerge (requires a graphical session)"),
            r#""$T" "$LOCAL" "$REMOTE""#,
            r#""$T" --merge --result="$MERGED" "$LOCAL" "$BASE" "$REMOTE""#,
            r#""$T" --merge --result="$MERGED" "$LOCAL" "$MERGED" "$REMOTE""#
        ),
        tool!(
            "diffuse",
            "diffuse",
            Some("Use Diffuse (requires a graphical session)"),
            Some("Use Diffuse (requires a graphical session)"),
            r#""$T" "$LOCAL" "$REMOTE""#,
            r#""$T" "$LOCAL" "$MERGED" "$REMOTE" "$BASE""#,
            r#""$T" "$LOCAL" "$MERGED" "$REMOTE""#
        ),
        tool!(
            "ecmerge",
            "ecmerge",
            Some("Use ECMerge (requires a graphical session)"),
            Some("Use ECMerge (requires a graphical session)"),
            r#""$T" --default --mode=diff2 "$LOCAL" "$REMOTE""#,
            r#""$T" "$BASE" "$LOCAL" "$REMOTE" --default --mode=merge3 --to="$MERGED""#,
            r#""$T" "$LOCAL" "$REMOTE" --default --mode=merge2 --to="$MERGED""#
        ),
        tool!(
            "emerge",
            "emacs",
            Some("Use Emacs' Emerge"),
            Some("Use Emacs' Emerge"),
            r#""$T" -f emerge-files-command "$LOCAL" "$REMOTE""#,
            r#""$T" -f emerge-files-with-ancestor-command "$LOCAL" "$REMOTE" "$BASE" "$(basename "$MERGED")""#,
            r#""$T" -f emerge-files-command "$LOCAL" "$REMOTE" "$(basename "$MERGED")""#
        ),
        tool!(
            "examdiff",
            "ExamDiff",
            Some("Use ExamDiff Pro (requires a graphical session)"),
            Some("Use ExamDiff Pro (requires a graphical session)"),
            r#""$T" "$LOCAL" "$REMOTE" -nh"#,
            r#""$T" -merge "$LOCAL" "$BASE" "$REMOTE" -o:"$MERGED" -nh"#,
            r#""$T" -merge "$LOCAL" "$REMOTE" -o:"$MERGED" -nh"#
        ),
        tool!(
            "guiffy",
            "guiffy",
            Some("Use Guiffy's Diff Tool (requires a graphical session)"),
            Some("Use Guiffy's Diff Tool (requires a graphical session)"),
            r#""$T" "$LOCAL" "$REMOTE""#,
            r#""$T" -s "$LOCAL" "$REMOTE" "$BASE" "$MERGED""#,
            r#""$T" -m "$LOCAL" "$REMOTE" "$MERGED""#
        ),
        tool!(
            "gvimdiff",
            "gvim",
            Some("Use gVim (requires a graphical session)"),
            Some(
                "Use gVim (requires a graphical session) with a custom layout (see `git help mergetool`'s `BACKEND SPECIFIC HINTS` section)"
            ),
            VIM_D,
            VIM_MB,
            VIM_MN
        ),
        tool!(
            "gvimdiff1",
            "gvim",
            None,
            Some(
                "Use gVim (requires a graphical session) with a 2 panes layout (LOCAL and REMOTE)"
            ),
            VIM_D,
            VIM1,
            VIM1
        ),
        tool!(
            "gvimdiff2",
            "gvim",
            None,
            Some(
                "Use gVim (requires a graphical session) with a 3 panes layout (LOCAL, MERGED and REMOTE)"
            ),
            VIM_D,
            VIM2,
            VIM2
        ),
        tool!(
            "gvimdiff3",
            "gvim",
            None,
            Some("Use gVim (requires a graphical session) where only the MERGED file is shown"),
            VIM_D,
            VIM3,
            VIM3
        ),
        tool!(
            "kdiff3",
            "kdiff3",
            Some("Use KDiff3 (requires a graphical session)"),
            Some("Use KDiff3 (requires a graphical session)"),
            r#""$T" --L1 "$MERGED (A)" --L2 "$MERGED (B)" "$LOCAL" "$REMOTE""#,
            r#""$T" --auto --L1 "$MERGED (Base)" --L2 "$MERGED (Local)" --L3 "$MERGED (Remote)" -o "$MERGED" "$BASE" "$LOCAL" "$REMOTE""#,
            r#""$T" --auto --L1 "$MERGED (Local)" --L2 "$MERGED (Remote)" -o "$MERGED" "$LOCAL" "$REMOTE""#
        ),
        tool!(
            "kompare",
            "kompare",
            Some("Use Kompare (requires a graphical session)"),
            None,
            r#""$T" "$LOCAL" "$REMOTE""#,
            "",
            ""
        ),
        tool!(
            "meld",
            "meld",
            Some("Use Meld (requires a graphical session)"),
            Some(
                "Use Meld (requires a graphical session) with optional `auto merge` (see `git help mergetool`'s `CONFIGURATION` section)"
            ),
            r#""$T" "$LOCAL" "$REMOTE""#,
            r#""$T" --output="$MERGED" "$LOCAL" "$BASE" "$REMOTE""#,
            r#""$T" --output="$MERGED" "$LOCAL" "$MERGED" "$REMOTE""#
        ),
        tool!(
            "nvimdiff",
            "nvim",
            Some("Use Neovim"),
            Some(
                "Use Neovim with a custom layout (see `git help mergetool`'s `BACKEND SPECIFIC HINTS` section)"
            ),
            VIM_D,
            VIM_MB,
            VIM_MN
        ),
        tool!(
            "nvimdiff1",
            "nvim",
            None,
            Some("Use Neovim with a 2 panes layout (LOCAL and REMOTE)"),
            VIM_D,
            VIM1,
            VIM1
        ),
        tool!(
            "nvimdiff2",
            "nvim",
            None,
            Some("Use Neovim with a 3 panes layout (LOCAL, MERGED and REMOTE)"),
            VIM_D,
            VIM2,
            VIM2
        ),
        tool!(
            "nvimdiff3",
            "nvim",
            None,
            Some("Use Neovim where only the MERGED file is shown"),
            VIM_D,
            VIM3,
            VIM3
        ),
        tool!(
            "opendiff",
            "opendiff",
            Some("Use FileMerge (requires a graphical session)"),
            Some("Use FileMerge (requires a graphical session)"),
            r#""$T" "$LOCAL" "$REMOTE" | cat"#,
            r#""$T" "$LOCAL" "$REMOTE" -ancestor "$BASE" -merge "$MERGED" | cat"#,
            r#""$T" "$LOCAL" "$REMOTE" -merge "$MERGED" | cat"#
        ),
        tool!(
            "p4merge",
            "p4merge",
            Some("Use HelixCore P4Merge (requires a graphical session)"),
            Some("Use HelixCore P4Merge (requires a graphical session)"),
            r#""$T" "$LOCAL" "$REMOTE""#,
            r#""$T" "$BASE" "$REMOTE" "$LOCAL" "$MERGED""#,
            r#""$T" "$LOCAL" "$REMOTE" "$LOCAL" "$MERGED""#
        ),
        tool!(
            "smerge",
            "smerge",
            Some("Use Sublime Merge (requires a graphical session)"),
            Some("Use Sublime Merge (requires a graphical session)"),
            r#""$T" mergetool "$LOCAL" "$REMOTE""#,
            r#""$T" mergetool "$BASE" "$LOCAL" "$REMOTE" -o "$MERGED""#,
            r#""$T" mergetool "$LOCAL" "$REMOTE" -o "$MERGED""#
        ),
        tool!(
            "tkdiff",
            "tkdiff",
            Some("Use TkDiff (requires a graphical session)"),
            Some("Use TkDiff (requires a graphical session)"),
            r#""$T" "$LOCAL" "$REMOTE""#,
            r#""$T" -a "$BASE" -o "$MERGED" "$LOCAL" "$REMOTE""#,
            r#""$T" -o "$MERGED" "$LOCAL" "$REMOTE""#
        ),
        tool!(
            "tortoisemerge",
            "tortoisemerge",
            None,
            Some("Use TortoiseMerge (requires a graphical session)"),
            "",
            r#""$T" -base:"$BASE" -mine:"$LOCAL" -theirs:"$REMOTE" -merged:"$MERGED""#,
            r#""$T" -mine:"$LOCAL" -theirs:"$REMOTE" -merged:"$MERGED""#
        ),
        tool!(
            "vimdiff",
            "vim",
            Some("Use Vim"),
            Some(
                "Use Vim with a custom layout (see `git help mergetool`'s `BACKEND SPECIFIC HINTS` section)"
            ),
            VIM_D,
            VIM_MB,
            VIM_MN
        ),
        tool!(
            "vimdiff1",
            "vim",
            None,
            Some("Use Vim with a 2 panes layout (LOCAL and REMOTE)"),
            VIM_D,
            VIM1,
            VIM1
        ),
        tool!(
            "vimdiff2",
            "vim",
            None,
            Some("Use Vim with a 3 panes layout (LOCAL, MERGED and REMOTE)"),
            VIM_D,
            VIM2,
            VIM2
        ),
        tool!(
            "vimdiff3",
            "vim",
            None,
            Some("Use Vim where only the MERGED file is shown"),
            VIM_D,
            VIM3,
            VIM3
        ),
        tool!(
            "winmerge",
            "WinMergeU",
            Some("Use WinMerge (requires a graphical session)"),
            Some("Use WinMerge (requires a graphical session)"),
            r#""$T" -u -e -wl "$LOCAL" "$REMOTE""#,
            r#""$T" -u -e -dl Local -dr Remote "$LOCAL" "$REMOTE" "$MERGED""#,
            r#""$T" -u -e -dl Local -dr Remote "$LOCAL" "$REMOTE" "$MERGED""#
        ),
        tool!(
            "xxdiff",
            "xxdiff",
            Some("Use xxdiff (requires a graphical session)"),
            Some("Use xxdiff (requires a graphical session)"),
            r#""$T" "$LOCAL" "$REMOTE""#,
            r#""$T" -X --show-merged-pane --merged-file "$MERGED" "$LOCAL" "$BASE" "$REMOTE""#,
            r#""$T" -X --show-merged-pane --merged-file "$MERGED" "$LOCAL" "$REMOTE""#
        ),
    ]
}

fn builtin(mode: Mode, name: &str) -> Option<Builtin> {
    builtins().into_iter().find(|b| {
        b.name == name
            && match mode {
                Mode::Diff => b.diff_desc.is_some(),
                Mode::Merge => b.merge_desc.is_some(),
            }
    })
}

/// Config lookups shared by both front ends; difftool falls back to the
/// mergetool variables (git-difftool CONFIGURATION).
pub struct ToolConfig<'a> {
    pub cfg: &'a ConfigSet,
    pub mode: Mode,
}

impl ToolConfig<'_> {
    fn tool_var(&self, tool: &str, var: &str) -> Option<String> {
        let own = self
            .cfg
            .get(&format!("{}tool.{tool}.{var}", self.mode.word()));
        match self.mode {
            Mode::Diff => own.or_else(|| self.cfg.get(&format!("mergetool.{tool}.{var}"))),
            Mode::Merge => own,
        }
    }

    /// `<mode>tool.<tool>.cmd`.
    pub fn cmd(&self, tool: &str) -> Option<String> {
        self.tool_var(tool, "cmd")
    }

    /// `<mode>tool.<tool>.path` or the built-in program name.
    pub fn path(&self, tool: &str) -> String {
        self.tool_var(tool, "path")
            .or_else(|| builtin(self.mode, tool).map(|b| b.exe.to_string()))
            .unwrap_or_else(|| tool.to_string())
    }

    /// `<mode>tool.<tool>.trustExitCode` (difftool: `difftool.trustExitCode`
    /// is handled by the front end).
    pub fn trust_exit_code(&self, tool: &str) -> bool {
        self.tool_var(tool, "trustExitCode")
            .and_then(|v| arks_config::parse_bool(&v))
            .unwrap_or(false)
    }

    /// Known tool: built in for this mode, or with a configured command.
    pub fn valid(&self, tool: &str) -> bool {
        builtin(self.mode, tool).is_some() || self.cmd(tool).is_some()
    }

    /// The configured tool (`-g` reads the `guitool` variables first).
    pub fn configured(&self, gui: bool) -> Option<String> {
        let order: &[&str] = match (self.mode, gui) {
            (Mode::Diff, true) => &["diff.guitool", "merge.guitool", "diff.tool", "merge.tool"],
            (Mode::Diff, false) => &["diff.tool", "merge.tool"],
            (Mode::Merge, true) => &["merge.guitool", "merge.tool"],
            (Mode::Merge, false) => &["merge.tool"],
        };
        order
            .iter()
            .find_map(|k| self.cfg.get(k))
            .filter(|v| !v.is_empty())
    }

    /// Pick a default when nothing is configured: explain on stderr (as git
    /// does), then take the first candidate whose program is installed.
    pub fn guess(&self) -> CliResult<String> {
        let mut cands: Vec<&str> = Vec::new();
        let display = std::env::var_os("DISPLAY").is_some_and(|d| !d.is_empty());
        if display {
            let gui: &[&str] = match self.mode {
                Mode::Diff => &[
                    "opendiff",
                    "kdiff3",
                    "tkdiff",
                    "xxdiff",
                    "meld",
                    "kompare",
                    "gvimdiff",
                    "diffuse",
                    "diffmerge",
                    "ecmerge",
                    "p4merge",
                    "araxis",
                    "bc",
                    "codecompare",
                    "smerge",
                ],
                Mode::Merge => &[
                    "opendiff",
                    "kdiff3",
                    "tkdiff",
                    "xxdiff",
                    "meld",
                    "tortoisemerge",
                    "gvimdiff",
                    "diffuse",
                    "diffmerge",
                    "ecmerge",
                    "p4merge",
                    "araxis",
                    "bc",
                    "codecompare",
                    "smerge",
                ],
            };
            cands.extend_from_slice(gui);
        }
        let tail: &[&str] = match self.mode {
            Mode::Diff => &["kompare", "emerge", "vimdiff", "nvimdiff"],
            Mode::Merge => &["tortoisemerge", "emerge", "vimdiff", "nvimdiff"],
        };
        for t in tail {
            if !cands.contains(t) {
                cands.push(t);
            }
        }
        let editor = std::env::var("VISUAL")
            .or_else(|_| std::env::var("EDITOR"))
            .unwrap_or_default();
        let prefer: &[&str] = if editor.contains("nvim") {
            &["nvimdiff", "vimdiff"]
        } else if editor.contains("vim") {
            &["vimdiff", "nvimdiff"]
        } else if editor.contains("emacs") {
            &["emerge"]
        } else {
            &[]
        };
        for p in prefer.iter().rev() {
            if let Some(i) = cands.iter().position(|c| c == p) {
                let t = cands.remove(i);
                cands.insert(0, t);
            }
        }
        let w = self.mode.word();
        eprintln!(
            "\nThis message is displayed because '{w}.tool' is not configured.\n\
             See 'git {w}tool --tool-help' or 'git help config' for more details.\n\
             'git {w}tool' will now attempt to use one of the following tools:\n{}",
            cands.join(" ")
        );
        for c in cands {
            if proc::which(&self.path(c)).is_some() {
                return Ok(c.to_string());
            }
        }
        Err(CliError::Exit(
            1,
            "No known merge tool is available.".to_string(),
        ))
    }

    /// The shell snippet that runs `tool` (`base`: a merge base exists).
    pub fn command(&self, tool: &str, base: bool) -> CliResult<String> {
        if let Some(cmd) = self.cmd(tool) {
            return Ok(cmd);
        }
        let Some(b) = builtin(self.mode, tool) else {
            return Err(CliError::Exit(1, format!("Unknown merge tool {tool}")));
        };
        let snippet = match (self.mode, base) {
            (Mode::Diff, _) => b.diff,
            (Mode::Merge, true) => b.merge_base,
            (Mode::Merge, false) => b.merge_nobase,
        };
        Ok(format!("T={}; {snippet}", proc::sq(&self.path(tool))))
    }

    /// `--tool-help`.
    pub fn print_help(&self) {
        let w = self.mode.word();
        let mut avail = Vec::new();
        let mut unavail = Vec::new();
        for b in builtins() {
            let desc = match self.mode {
                Mode::Diff => b.diff_desc,
                Mode::Merge => b.merge_desc,
            };
            let Some(desc) = desc else { continue };
            let line = format!("\t\t{:<17}{desc}", b.name);
            if proc::which(&self.path(b.name)).is_some() {
                avail.push(line);
            } else {
                unavail.push(line);
            }
        }
        let mut user: Vec<String> = Vec::new();
        let sections: &[&str] = match self.mode {
            Mode::Diff => &["difftool", "mergetool"],
            Mode::Merge => &["mergetool"],
        };
        for sec in sections {
            for (name, _) in self.cfg.subsections(sec, "cmd") {
                if !user.contains(&name) {
                    user.push(name);
                }
            }
        }
        user.sort();
        let mut out = format!("'git {w}tool --tool=<tool>' may be set to one of the following:\n");
        for l in &avail {
            out.push_str(l);
            out.push('\n');
        }
        if !user.is_empty() {
            out.push_str("\n\tuser-defined:\n");
            for u in &user {
                let cmd = self.cmd(u).unwrap_or_default();
                out.push_str(&format!("\t\t{u}.cmd {cmd}\n"));
            }
        }
        if !unavail.is_empty() {
            out.push_str("\nThe following tools are valid, but not currently available:\n");
            for l in &unavail {
                out.push_str(l);
                out.push('\n');
            }
        }
        out.push_str(
            "\nSome of the tools listed above only work in a windowed\n\
             environment. If run in a terminal-only session, they will fail.\n",
        );
        print!("{out}");
    }
}

/// Environment for one tool run.
pub struct Files<'a> {
    pub local: &'a str,
    pub remote: &'a str,
    pub merged: &'a str,
    pub base: &'a str,
}

/// Run `script` through `sh -c` in `cwd` with the tool variables exported.
pub fn launch(script: &str, files: &Files<'_>, cwd: &Path) -> CliResult<i32> {
    let mut c = proc::shell(script, "sh", &[]);
    c.current_dir(cwd)
        .env("LOCAL", files.local)
        .env("REMOTE", files.remote)
        .env("MERGED", files.merged)
        .env("BASE", files.base);
    let st = proc::run(c)?;
    Ok(proc::code_of(&st))
}
