//! Repository model behind the terminal `git gui` / `git citool` screens:
//! the "Unstaged Changes" and "Staged Changes (Will Commit)" lists, the diff
//! of one file, the commit message buffer, "Amend Last Commit", "Sign Off"
//! and the commit action itself.
//!
//! Every action goes through arks' own porcelain (`add`, `reset`, `rm
//! --cached`, `commit`), so hooks, identity and index handling are the same
//! as on the command line.
//! Cite: <https://git-scm.com/docs/git-gui> (behaviour, not code).

use crate::cmd::shared::head_blob_map;
use crate::cmd::status_report::{self, Untracked};
use crate::cmd::unified_diff::write_file_patch_ctx;
use crate::error::{CliError, CliResult};
use crate::util::{open_repo, read_index, work_tree};
use std::fs;

/// One row of a file list: the path and its one-letter state
/// (`M` modified, `A` added, `D` deleted, `?` untracked, `U` unmerged).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub path: String,
    pub state: char,
}

/// Which list a row belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Unstaged,
    Staged,
}

/// Result of the commit action.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Committed {
    /// A commit was recorded: abbreviated id and subject.
    Created { abbrev: String, subject: String },
    /// `citool --nocommit`: the user is done, nothing was recorded.
    Done,
}

/// The staging / commit state shown by the screens.
pub struct Model {
    pub branch: String,
    pub unstaged: Vec<Row>,
    pub staged: Vec<Row>,
    pub message: String,
    pub amend: bool,
    pub nocommit: bool,
    /// Message loaded from HEAD when amend was switched on (to restore on off).
    amend_loaded: Option<String>,
    /// Message the user had before switching amend on.
    saved_message: String,
}

impl Model {
    /// Open the repository at the current directory and scan it.
    pub fn open(amend: bool, nocommit: bool) -> CliResult<Self> {
        let mut m = Model {
            branch: String::new(),
            unstaged: Vec::new(),
            staged: Vec::new(),
            message: String::new(),
            amend: false,
            nocommit,
            amend_loaded: None,
            saved_message: String::new(),
        };
        m.rescan()?;
        if amend {
            m.set_amend(true)?;
        }
        Ok(m)
    }

    /// Re-read HEAD, the index and the working tree.
    pub fn rescan(&mut self) -> CliResult<()> {
        let repo = open_repo()?;
        work_tree(&repo)?;
        let d = status_report::collect(&repo, Untracked::All)?;
        self.branch = match (&d.branch, &d.head_abbrev) {
            (Some(b), _) => b.clone(),
            (None, Some(a)) => format!("(detached at {a})"),
            (None, None) => "(no branch)".into(),
        };
        let mut unstaged: Vec<Row> = d
            .unstaged
            .iter()
            .filter(|(p, _)| !d.unmerged.contains_key(*p))
            .map(|(p, c)| Row {
                path: p.clone(),
                state: *c,
            })
            .collect();
        unstaged.extend(d.unmerged.keys().map(|p| Row {
            path: p.clone(),
            state: 'U',
        }));
        unstaged.extend(d.untracked.iter().map(|p| Row {
            path: p.clone(),
            state: '?',
        }));
        unstaged.sort_by(|a, b| a.path.cmp(&b.path));
        self.unstaged = unstaged;
        self.staged = d
            .staged
            .iter()
            .filter(|(p, _)| !d.unmerged.contains_key(*p))
            .map(|(p, c)| Row {
                path: p.clone(),
                state: *c,
            })
            .collect();
        Ok(())
    }

    fn has_head() -> CliResult<bool> {
        Ok(open_repo()?.head_oid().is_ok())
    }

    /// Stage paths: present files are added, missing ones removed from the index.
    fn add_paths(paths: &[String]) -> CliResult<()> {
        let repo = open_repo()?;
        let work = work_tree(&repo)?;
        let (present, gone): (Vec<String>, Vec<String>) = paths
            .iter()
            .cloned()
            .partition(|p| fs::symlink_metadata(work.join(p)).is_ok());
        if !present.is_empty() {
            let mut args = vec!["--".to_string()];
            args.extend(present);
            crate::cmd::cmd_add(&args)?;
        }
        if !gone.is_empty() {
            let mut args = vec!["--cached".to_string(), "-q".into(), "--".into()];
            args.extend(gone);
            crate::cmd::cmd_rm(&args)?;
        }
        Ok(())
    }

    /// Stage one path (new, modified, deleted or resolved).
    pub fn stage(&mut self, path: &str) -> CliResult<()> {
        if self.side_of(path) != Some(Side::Unstaged) {
            return Err(CliError::Msg(format!("'{path}' has no unstaged changes")));
        }
        Self::add_paths(&[path.to_string()])?;
        self.rescan()
    }

    /// "Stage Changed": every modified or deleted tracked file (not untracked ones).
    pub fn stage_changed(&mut self) -> CliResult<()> {
        let paths: Vec<String> = self
            .unstaged
            .iter()
            .filter(|r| matches!(r.state, 'M' | 'D'))
            .map(|r| r.path.clone())
            .collect();
        if paths.is_empty() {
            return Err(CliError::Msg("No changes to stage.".into()));
        }
        Self::add_paths(&paths)?;
        self.rescan()
    }

    /// Unstage one path (back to HEAD, or out of the index before the first commit).
    pub fn unstage(&mut self, path: &str) -> CliResult<()> {
        if !self.staged.iter().any(|r| r.path == path) {
            return Err(CliError::Msg(format!("'{path}' has no staged changes")));
        }
        if Self::has_head()? {
            crate::cmd::cmd_reset(&["-q".into(), "--".into(), path.into()])?;
        } else {
            crate::cmd::cmd_rm(&["--cached".into(), "-q".into(), "--".into(), path.into()])?;
        }
        self.rescan()
    }

    /// Which list holds `path` (unstaged wins when it is in both).
    pub fn side_of(&self, path: &str) -> Option<Side> {
        if self.unstaged.iter().any(|r| r.path == path) {
            Some(Side::Unstaged)
        } else if self.staged.iter().any(|r| r.path == path) {
            Some(Side::Staged)
        } else {
            None
        }
    }

    /// Toggle: stage an unstaged path, unstage a staged one.
    pub fn toggle(&mut self, side: Side, path: &str) -> CliResult<()> {
        match side {
            Side::Unstaged => self.stage(path),
            Side::Staged => self.unstage(path),
        }
    }

    /// Patch text of `path` on `side` (index vs working tree, or HEAD vs index).
    pub fn diff(&self, side: Side, path: &str) -> CliResult<String> {
        let repo = open_repo()?;
        let work = work_tree(&repo)?;
        let index = read_index(&repo)?;
        let index_blob = index
            .entries
            .iter()
            .find(|e| e.path == path && e.stage() == 0)
            .map(|e| repo.get_object(&e.oid).map(|o| o.payload))
            .transpose()?;
        let (old, new) = match side {
            Side::Unstaged => {
                if self
                    .unstaged
                    .iter()
                    .any(|r| r.path == path && r.state == 'U')
                {
                    return Ok(format!(
                        "Unmerged path {path}: resolve it, then stage it.\n"
                    ));
                }
                let abs = work.join(path);
                let new = if abs.is_file() {
                    Some(fs::read(&abs)?)
                } else {
                    None
                };
                (index_blob, new)
            }
            Side::Staged => {
                let head = head_blob_map(&repo)?;
                let old = head
                    .get(path)
                    .map(|o| repo.get_object(o).map(|o| o.payload))
                    .transpose()?;
                (old, index_blob)
            }
        };
        let mut out = Vec::new();
        write_file_patch_ctx(
            &mut out,
            repo.object_format(),
            path,
            old.as_deref(),
            new.as_deref(),
            3,
        )?;
        Ok(String::from_utf8_lossy(&out).into_owned())
    }

    /// Switch "Amend Last Commit" on or off. On loads HEAD's message.
    pub fn set_amend(&mut self, on: bool) -> CliResult<()> {
        if on == self.amend {
            return Ok(());
        }
        if on {
            let repo = open_repo()?;
            let head = repo
                .head_oid()
                .map_err(|_| CliError::Msg("No commit to amend.".into()))?;
            let c = crate::cmd::log_format::read_commit(&repo, &head)?;
            self.saved_message = std::mem::take(&mut self.message);
            self.message = c.message.clone();
            self.amend_loaded = Some(c.message);
        } else {
            let loaded = self.amend_loaded.take();
            if loaded.as_deref() == Some(self.message.as_str()) {
                self.message = std::mem::take(&mut self.saved_message);
            }
        }
        self.amend = on;
        Ok(())
    }

    /// "Sign Off": append a `Signed-off-by:` line for the committer.
    pub fn signoff(&mut self) -> CliResult<()> {
        let who = crate::cmd::shared::committer_signature()?;
        let line = format!("Signed-off-by: {} <{}>", who.name, who.email);
        if self.message.lines().any(|l| l == line) {
            return Ok(());
        }
        let body = self.message.trim_end().to_string();
        let last_is_trailer = body
            .lines()
            .last()
            .is_some_and(|l| l.starts_with("Signed-off-by: "));
        self.message = if body.is_empty() {
            format!("\n{line}\n")
        } else if last_is_trailer {
            format!("{body}\n{line}\n")
        } else {
            format!("{body}\n\n{line}\n")
        };
        Ok(())
    }

    /// The commit button ("Commit", or "Done" with `--nocommit`).
    pub fn commit(&mut self) -> CliResult<Committed> {
        if self.unstaged.iter().any(|r| r.state == 'U') {
            return Err(CliError::Msg(
                "Unmerged files cannot be committed. Resolve and stage them first.".into(),
            ));
        }
        if self.nocommit {
            return Ok(Committed::Done);
        }
        if !self.amend && self.staged.is_empty() {
            return Err(CliError::Msg(
                "No changes to commit. Stage at least one file before committing.".into(),
            ));
        }
        let text = self.message.trim();
        if text.is_empty() {
            return Err(CliError::Msg(
                "Please supply a commit message before committing.".into(),
            ));
        }
        let repo = open_repo()?;
        let msg_file = repo.git_dir().join("GITGUI_EDITMSG");
        fs::write(&msg_file, format!("{text}\n"))?;
        let mut args = vec![
            "-q".to_string(),
            "--cleanup=whitespace".to_string(),
            "-F".to_string(),
            msg_file.to_string_lossy().into_owned(),
        ];
        if self.amend {
            args.push("--amend".into());
        }
        let res = crate::cmd::cmd_commit(&args);
        let _ = fs::remove_file(&msg_file);
        res?;
        let repo = open_repo()?;
        let head = repo.head_oid()?;
        let abbrev = crate::cmd::shared::abbrev_hex(&repo, &head, 7);
        let subject = text.lines().next().unwrap_or("").to_string();
        self.message.clear();
        self.saved_message.clear();
        self.amend_loaded = None;
        self.amend = false;
        self.rescan()?;
        Ok(Committed::Created { abbrev, subject })
    }
}
