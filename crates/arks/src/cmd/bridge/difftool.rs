//! `git difftool` / `git difftool--helper`: show each changed file of a
//! `git diff` comparison in the configured diff tool (or `--extcmd`), or the
//! whole change set at once with `--dir-diff`.
//!
//! Primary: <https://git-scm.com/docs/git-difftool>. Observed peer behaviour
//! (git 2.39/2.43): the `Viewing (i/n)` prompt on stdout, temporary blobs
//! under `<tmp>/git-blob-XXXXXX/<name>`, `/dev/null` for a missing side, the
//! work-tree file itself for a work-tree side, and `fatal: external diff
//! died, stopping at <path>` (exit 128) when the tool fails under
//! `--trust-exit-code` or is unknown.

use super::proc;
use super::tools::{Files, Mode, ToolConfig, launch};
use crate::cmd::diff_cmd::{Queue, diff_queue};
use crate::cmd::diffcore::{Content, Pair, Side};
use crate::cmd::opt_parse::{Item, Mode as ParseMode, OptDef, Takes::*, o, parse};
use crate::error::{CliError, CliResult};
use arks_config::ConfigSet;
use std::fs;
use std::path::{Path, PathBuf};

const OPTS: &[OptDef] = &[
    o("gui", 'g', No, true),
    o("dir-diff", 'd', No, true),
    o("no-prompt", 'y', No, true),
    o("symlinks", ' ', No, true),
    o("tool", 't', Req, true),
    o("tool-help", ' ', No, true),
    o("trust-exit-code", ' ', No, true),
    o("extcmd", 'x', Req, true),
    o("index", ' ', No, true),
];

/// What to run for each file.
enum Runner {
    Ext(String),
    Tool(String),
}

struct Settings {
    runner: Runner,
    prompt: bool,
    trust: bool,
}

pub fn cmd_difftool(args: &[String]) -> CliResult<()> {
    let p = parse(
        "difftool",
        args,
        OPTS,
        ParseMode {
            keep_unknown: true,
            ..ParseMode::default()
        },
    )?;
    let mut diff_args: Vec<String> = Vec::new();
    let mut dashdash = false;
    for it in &p.items {
        match it {
            Item::Arg(a) | Item::Unknown(a) | Item::Number(a) => diff_args.push(a.clone()),
            Item::AfterDashDash(a) => {
                if !dashdash {
                    diff_args.push("--".into());
                    dashdash = true;
                }
                diff_args.push(a.clone());
            }
            Item::Opt {
                key: "index", neg, ..
            } => {
                if *neg {
                    diff_args.insert(0, "--no-index".into());
                }
            }
            Item::Opt { .. } => {}
        }
    }
    let repo = crate::util::open_repo().ok();
    let cfg = ConfigSet::load(repo.as_ref().map(|r| r.common_dir()));
    let tc = ToolConfig {
        cfg: &cfg,
        mode: Mode::Diff,
    };
    if p.on("tool-help") {
        tc.print_help();
        return Ok(());
    }
    // The comparison is parsed first: a bad diff option fails like `git diff`.
    let queue = match diff_queue(&diff_args) {
        Ok(q) => q,
        Err(CliError::NotImplemented(m)) if m.starts_with("unsupported option: ") => {
            let opt = m.trim_start_matches("unsupported option: ");
            return Err(CliError::Exit(
                129,
                format!(
                    "error: invalid option: {opt}\nusage: git diff [<options>] [<commit>] [--] [<path>...]"
                ),
            ));
        }
        Err(e) => return Err(e),
    };
    let gui = p.on("gui")
        || (p.flag("gui").is_none() && cfg.get_bool("difftool.guiDefault") == Some(true));
    let runner = if let Some(x) = p.value("extcmd") {
        Runner::Ext(x)
    } else {
        let tool = match p.value("tool") {
            Some(t) => t,
            None => match tc.configured(gui) {
                Some(t) => t,
                None => tc.guess()?,
            },
        };
        Runner::Tool(tool)
    };
    let prompt = match p.flag("no-prompt") {
        Some(v) => !v,
        None => cfg
            .get_bool("difftool.prompt")
            .or_else(|| cfg.get_bool("mergetool.prompt"))
            .unwrap_or(true),
    };
    let trust = p
        .flag("trust-exit-code")
        .or_else(|| cfg.get_bool("difftool.trustExitCode"))
        .unwrap_or(false);
    let s = Settings {
        runner,
        prompt,
        trust,
    };
    let symlinks = p.flag("symlinks").unwrap_or(cfg!(unix));
    match queue {
        Queue::NoIndex(files, _) => {
            if files.len() != 2 {
                return Err(CliError::Usage(
                    "usage: git diff --no-index [<options>] <path> <path>".into(),
                ));
            }
            let cwd = std::env::current_dir()?;
            let f = Files {
                local: &files[0],
                remote: &files[1],
                merged: &files[1],
                base: &files[1],
            };
            run_one(&tc, &s, &f, &files[1], 1, 1, &cwd)?;
            // Like `git diff --no-index`, differing files exit 1.
            if fs::read(&files[0]).ok() != fs::read(&files[1]).ok() {
                return Err(CliError::silent(1));
            }
            Ok(())
        }
        Queue::Repo(repo, _, pairs, with_worktree) => {
            let top = crate::util::work_tree(&repo)?;
            let pairs: Vec<Pair> = pairs.into_iter().filter(|p| !p.unmerged).collect();
            if p.on("dir-diff") {
                return dir_diff(&repo, &tc, &s, &pairs, &top, symlinks);
            }
            let c = Content::new(&repo);
            let wt = Worktree::new(&repo, &top, with_worktree)?;
            let total = pairs.len();
            for (i, pair) in pairs.iter().enumerate() {
                let mut temps = Vec::new();
                let local = side_file(&c, &wt, pair.old.as_ref(), &mut temps)?;
                let remote = side_file(&c, &wt, pair.new.as_ref(), &mut temps)?;
                let merged = pair.path().to_string();
                let f = Files {
                    local: &local,
                    remote: &remote,
                    merged: &merged,
                    base: &merged,
                };
                let r = run_one(&tc, &s, &f, &merged, i + 1, total, &top);
                for t in temps {
                    let _ = fs::remove_dir_all(t);
                }
                r?;
            }
            Ok(())
        }
    }
}

/// Work-tree reuse: like git's external diff, a blob that the up-to-date
/// work-tree file already holds is handed over as that file.
struct Worktree<'r> {
    repo: &'r arks_repo::Repository,
    top: PathBuf,
    index: std::collections::HashMap<String, arks_index::IndexEntry>,
    /// The comparison involves the work tree, so the index was refreshed.
    refreshed: bool,
}

impl<'r> Worktree<'r> {
    fn new(repo: &'r arks_repo::Repository, top: &Path, refreshed: bool) -> CliResult<Self> {
        let index = crate::util::read_index(repo)?
            .entries
            .into_iter()
            .filter(|e| e.stage() == 0)
            .map(|e| (e.path.clone(), e))
            .collect();
        Ok(Self {
            repo,
            top: top.to_path_buf(),
            index,
            refreshed,
        })
    }

    fn holds(&self, s: &Side) -> bool {
        let (Some(oid), Some(e)) = (s.oid, self.index.get(&s.path)) else {
            return false;
        };
        if e.oid != oid || e.mode != s.mode {
            return false;
        }
        let abs = self.top.join(&s.path);
        let Ok(meta) = fs::symlink_metadata(&abs) else {
            return false;
        };
        if !self.refreshed && !crate::cmd::diffcore::sources::stat_clean(e, &meta) {
            return false;
        }
        worktree_oid(self.repo, &abs).is_some_and(|o| o == oid)
    }
}

/// Object id of a work-tree file's content (symlinks: the link target).
fn worktree_oid(repo: &arks_repo::Repository, abs: &Path) -> Option<arks_oid::Oid> {
    let meta = fs::symlink_metadata(abs).ok()?;
    let bytes = if meta.file_type().is_symlink() {
        fs::read_link(abs)
            .ok()?
            .to_string_lossy()
            .into_owned()
            .into_bytes()
    } else if meta.is_file() {
        fs::read(abs).ok()?
    } else {
        return None;
    };
    arks_object::RawObject::new(arks_object::ObjectKind::Blob, bytes)
        .oid(repo.object_format())
        .ok()
}

/// The file handed to the tool for one side: `/dev/null` when absent, the
/// work-tree path for a work-tree side, else a temporary copy of the blob.
fn side_file(
    c: &Content<'_>,
    wt: &Worktree<'_>,
    side: Option<&Side>,
    temps: &mut Vec<PathBuf>,
) -> CliResult<String> {
    let Some(s) = side else {
        return Ok("/dev/null".into());
    };
    if s.work.is_some() || s.oid.is_none() || wt.holds(s) {
        return Ok(s.path.clone());
    }
    let dir = proc::temp_dir("git-blob-")?;
    let name = s.path.rsplit('/').next().unwrap_or("blob");
    let file = dir.join(name);
    fs::write(&file, c.bytes(s)?.as_slice())?;
    temps.push(dir);
    Ok(file.to_string_lossy().into_owned())
}

/// Prompt (unless disabled) and launch the tool for one file, the way
/// `difftool--helper` does for each external-diff call.
fn run_one(
    tc: &ToolConfig<'_>,
    s: &Settings,
    f: &Files<'_>,
    path: &str,
    n: usize,
    total: usize,
    cwd: &Path,
) -> CliResult<()> {
    let shown = match &s.runner {
        Runner::Ext(x) => x.clone(),
        Runner::Tool(t) => t.clone(),
    };
    if s.prompt {
        let q = format!("\nViewing ({n}/{total}): '{path}'\nLaunch '{shown}' [Y/n]? ");
        match proc::ask(&q) {
            None => return Ok(()),
            Some(a) if a.trim().eq_ignore_ascii_case("n") => return Ok(()),
            Some(_) => {}
        }
    }
    let script = match &s.runner {
        Runner::Ext(x) => format!("{x} \"$LOCAL\" \"$REMOTE\""),
        Runner::Tool(t) => match tc.command(t, false) {
            Ok(c) => c,
            Err(CliError::Exit(_, m)) => {
                eprintln!("{m}");
                return Err(died(path));
            }
            Err(e) => return Err(e),
        },
    };
    let code = launch(&script, f, cwd)?;
    if code != 0 && s.trust {
        return Err(died(path));
    }
    Ok(())
}

fn died(path: &str) -> CliError {
    CliError::fatal(format!("external diff died, stopping at {path}"))
}

/// `--dir-diff`: copy both sides into `left/` and `right/` of a temporary
/// directory (work-tree files become symlinks with `--symlinks`) and run the
/// tool once on the two directories. Edits to copied work-tree files are
/// written back afterwards.
fn dir_diff(
    repo: &arks_repo::Repository,
    tc: &ToolConfig<'_>,
    s: &Settings,
    pairs: &[Pair],
    top: &Path,
    symlinks: bool,
) -> CliResult<()> {
    if pairs.is_empty() {
        return Ok(());
    }
    let c = Content::new(repo);
    let tmp = proc::temp_dir("git-difftool.")?;
    let left = tmp.join("left");
    let right = tmp.join("right");
    fs::create_dir_all(&left)?;
    fs::create_dir_all(&right)?;
    let mut copied_back: Vec<(PathBuf, PathBuf)> = Vec::new();
    for p in pairs {
        if let Some(o) = &p.old {
            write_side(&c, o, &left.join(&o.path))?;
        }
        if let Some(n) = &p.new {
            let dst = right.join(&n.path);
            let from_worktree = n.work.is_some()
                || n.oid.is_none()
                || worktree_oid(repo, &top.join(&n.path)).is_some_and(|o| Some(o) == n.oid);
            if from_worktree {
                let src = top.join(&n.path);
                if let Some(parent) = dst.parent() {
                    fs::create_dir_all(parent)?;
                }
                if symlinks && link(&src, &dst).is_ok() {
                    continue;
                }
                fs::copy(&src, &dst)?;
                copied_back.push((dst, src));
            } else {
                write_side(&c, n, &dst)?;
            }
        }
    }
    let l = format!("{}/", left.display());
    let r = format!("{}/", right.display());
    let none = String::new();
    let f = Files {
        local: &l,
        remote: &r,
        merged: &none,
        base: &none,
    };
    let code = match &s.runner {
        // In directory mode the extcmd is run as a program (no shell).
        Runner::Ext(x) => {
            let mut c = std::process::Command::new(x);
            c.arg(&l).arg(&r).current_dir(top);
            if let Some(p) = proc::config_parameters() {
                c.env("GIT_CONFIG_PARAMETERS", p);
            }
            let _ = std::io::Write::flush(&mut std::io::stdout());
            match c.status() {
                Ok(st) => proc::code_of(&st),
                Err(e) => {
                    let _ = fs::remove_dir_all(&tmp);
                    let why = match e.kind() {
                        std::io::ErrorKind::NotFound => "No such file or directory".to_string(),
                        _ => e.to_string(),
                    };
                    eprintln!("error: cannot run {x}: {why}\nwarning: failed: -1");
                    return Err(CliError::silent(1));
                }
            }
        }
        Runner::Tool(t) => launch(&tc.command(t, false)?, &f, top)?,
    };
    for (copy, orig) in copied_back {
        if let (Ok(a), Ok(b)) = (fs::read(&copy), fs::read(&orig)) {
            if a != b {
                fs::write(&orig, a)?;
            }
        }
    }
    let _ = fs::remove_dir_all(&tmp);
    if code != 0 {
        // Directory mode always reports a failing tool.
        eprintln!("warning: failed: {code}");
        return Err(CliError::silent(1));
    }
    Ok(())
}

fn write_side(c: &Content<'_>, s: &Side, dst: &Path) -> CliResult<()> {
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(dst, c.bytes(s)?.as_slice())?;
    Ok(())
}

#[cfg(unix)]
fn link(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(src, dst)
}

#[cfg(not(unix))]
fn link(_src: &Path, _dst: &Path) -> std::io::Result<()> {
    Err(std::io::Error::other("symlinks unavailable"))
}

/// `git difftool--helper <path> <old-file> <old-hex> <old-mode> <new-file>
/// <new-hex> <new-mode>`: the external-diff entry point. Settings come from
/// the environment difftool exports (`GIT_DIFF_TOOL`, `GIT_DIFFTOOL_EXTCMD`,
/// `GIT_DIFFTOOL_NO_PROMPT`, `GIT_DIFFTOOL_TRUST_EXIT_CODE`,
/// `GIT_DIFF_PATH_COUNTER` / `GIT_DIFF_PATH_TOTAL`).
pub fn cmd_difftool_helper(args: &[String]) -> CliResult<()> {
    if args.len() < 7 {
        // A single argument is an unmerged path: nothing to show.
        return Ok(());
    }
    let repo = crate::util::open_repo().ok();
    let cfg = ConfigSet::load(repo.as_ref().map(|r| r.common_dir()));
    let tc = ToolConfig {
        cfg: &cfg,
        mode: Mode::Diff,
    };
    let env = |k: &str| std::env::var(k).ok().filter(|v| !v.is_empty());
    let runner = if let Some(x) = env("GIT_DIFFTOOL_EXTCMD") {
        Runner::Ext(x)
    } else {
        let gui = env("GIT_MERGETOOL_GUI").as_deref() == Some("true");
        Runner::Tool(match env("GIT_DIFF_TOOL") {
            Some(t) => t,
            None => match tc.configured(gui) {
                Some(t) => t,
                None => tc.guess()?,
            },
        })
    };
    let prompt = if env("GIT_DIFFTOOL_NO_PROMPT").as_deref() == Some("true") {
        false
    } else if env("GIT_DIFFTOOL_PROMPT").as_deref() == Some("true") {
        true
    } else {
        cfg.get_bool("difftool.prompt")
            .or_else(|| cfg.get_bool("mergetool.prompt"))
            .unwrap_or(true)
    };
    let trust = env("GIT_DIFFTOOL_TRUST_EXIT_CODE").as_deref() == Some("true");
    let s = Settings {
        runner,
        prompt,
        trust,
    };
    let n: usize = env("GIT_DIFF_PATH_COUNTER")
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);
    let total: usize = env("GIT_DIFF_PATH_TOTAL")
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);
    let merged = &args[0];
    let f = Files {
        local: &args[1],
        remote: &args[4],
        merged,
        base: merged,
    };
    let cwd = std::env::current_dir()?;
    run_one(&tc, &s, &f, merged, n, total, &cwd).map_err(|e| match e {
        CliError::Exit(128, _) => CliError::silent(1),
        other => other,
    })
}
