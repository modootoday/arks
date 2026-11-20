//! Diff output selection + rendering shared by `diff`, `log` and `show`.
//!
//! Output order follows git `diff_flush`: name-only / name-status first, then
//! `--numstat`, `--stat`, `--shortstat`, then (after a blank separator line when
//! anything was printed before it) the patch.
//!
//! Primary: <https://git-scm.com/docs/git-diff> (§OPTIONS: `-p`, `-U<n>`,
//! `--stat`, `--numstat`, `--shortstat`, `--name-only`, `--name-status`,
//! `--exit-code`, `--quiet`), <https://git-scm.com/docs/diff-format>.
//!
//! Partial: rename/copy detection is not done; file modes print as `100644`.

use crate::cmd::unified_diff::{
    StatChange, StatRow, shortstat_line, stat_change, write_file_patch_ctx, write_stat,
};
use crate::error::{CliError, CliResult};
use arks_oid::Oid;
use arks_repo::Repository;
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Selected diff output formats (git `output_format` bits) plus `-U<n>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffFormat {
    pub patch: bool,
    pub stat: bool,
    pub numstat: bool,
    pub shortstat: bool,
    pub name_only: bool,
    pub name_status: bool,
    /// `-s` / `--no-patch`: suppress all diff output.
    pub no_output: bool,
    pub context: usize,
}

impl Default for DiffFormat {
    fn default() -> Self {
        Self {
            patch: false,
            stat: false,
            numstat: false,
            shortstat: false,
            name_only: false,
            name_status: false,
            no_output: false,
            context: 3,
        }
    }
}

impl DiffFormat {
    /// Any output format selected (after `-s` is applied).
    pub fn any(&self) -> bool {
        !self.no_output
            && (self.patch
                || self.stat
                || self.numstat
                || self.shortstat
                || self.name_only
                || self.name_status)
    }

    /// Consume one diff-output option. Returns `Ok(true)` when recognised.
    #[allow(dead_code)]
    pub fn take_flag(&mut self, a: &str) -> CliResult<bool> {
        match a {
            "-p" | "-u" | "--patch" => {
                self.patch = true;
                self.no_output = false;
            }
            "-s" | "--no-patch" => self.no_output = true,
            "--stat" => self.stat = true,
            "--numstat" => self.numstat = true,
            "--shortstat" => self.shortstat = true,
            "--name-only" => self.name_only = true,
            "--name-status" => self.name_status = true,
            "--patch-with-stat" => {
                self.patch = true;
                self.stat = true;
            }
            _ => {
                let n = a
                    .strip_prefix("--unified=")
                    .or_else(|| a.strip_prefix("-U").filter(|r| !r.is_empty()));
                let Some(n) = n else {
                    return Ok(false);
                };
                self.context = n
                    .parse()
                    .map_err(|_| CliError::fatal(format!("invalid argument to {a}")))?;
                self.patch = true;
            }
        }
        Ok(true)
    }
}

/// One changed path. `work` = read the new side from this working-tree file.
pub struct Change {
    pub path: String,
    pub old: Option<Oid>,
    pub new: Option<Oid>,
    pub work: Option<PathBuf>,
}

/// git `--diff-filter`: upper-case letters select, lower-case exclude.
/// Cite: https://git-scm.com/docs/git-diff#Documentation/git-diff.txt---diff-filterACDMRTUXB82308203
#[allow(dead_code)]
pub fn status_passes(st: char, filter: &str) -> bool {
    let includes: Vec<char> = filter.chars().filter(|c| c.is_ascii_uppercase()).collect();
    let excluded = filter.contains(st.to_ascii_lowercase());
    (includes.is_empty() || includes.contains(&st)) && !excluded
}

impl Change {
    /// `A` added, `D` deleted, `M` modified.
    pub fn status(&self) -> char {
        match (self.old.is_some(), self.new.is_some()) {
            (false, _) => 'A',
            (_, false) => 'D',
            _ => 'M',
        }
    }
}

/// Paths whose OIDs differ between two path -> OID maps, sorted bytewise.
/// With `work`, right-side contents are read from that working tree.
pub fn changes_between(
    a: &HashMap<String, Oid>,
    b: &HashMap<String, Oid>,
    work: Option<&Path>,
) -> Vec<Change> {
    let mut keys: Vec<&String> = a.keys().chain(b.keys()).collect();
    keys.sort();
    keys.dedup();
    keys.into_iter()
        .filter(|k| a.get(*k) != b.get(*k))
        .map(|k| Change {
            path: k.clone(),
            old: a.get(k).copied(),
            new: b.get(k).copied(),
            work: match (b.get(k), work) {
                (Some(_), Some(w)) => Some(w.join(k)),
                _ => None,
            },
        })
        .collect()
}

fn old_bytes(repo: &Repository, c: &Change) -> CliResult<Option<Vec<u8>>> {
    Ok(match &c.old {
        Some(o) => Some(repo.get_object(o)?.payload),
        None => None,
    })
}

fn new_bytes(repo: &Repository, c: &Change) -> CliResult<Option<Vec<u8>>> {
    Ok(match (&c.new, &c.work) {
        (Some(_), Some(p)) => Some(std::fs::read(p)?),
        (Some(o), None) => Some(repo.get_object(o)?.payload),
        (None, _) => None,
    })
}

/// Render `changes` in the selected formats (git `diff_flush` order).
pub fn render(
    out: &mut impl Write,
    repo: &Repository,
    fmt: &DiffFormat,
    changes: &[Change],
) -> CliResult<()> {
    if !fmt.any() || changes.is_empty() {
        return Ok(());
    }
    let mut separator = false;
    if fmt.name_only || fmt.name_status {
        for c in changes {
            if fmt.name_status {
                writeln!(out, "{}\t{}", c.status(), c.path)?;
            } else {
                writeln!(out, "{}", c.path)?;
            }
        }
        separator = true;
    }
    if fmt.stat || fmt.numstat || fmt.shortstat {
        let mut rows = Vec::with_capacity(changes.len());
        for c in changes {
            let (a, b) = (old_bytes(repo, c)?, new_bytes(repo, c)?);
            rows.push(StatRow {
                path: c.path.clone(),
                change: stat_change(a.as_deref(), b.as_deref()),
            });
        }
        if fmt.numstat {
            for r in &rows {
                match r.change {
                    StatChange::Lines(i, d) => writeln!(out, "{i}\t{d}\t{}", r.path)?,
                    StatChange::Binary(..) => writeln!(out, "-\t-\t{}", r.path)?,
                }
            }
        }
        if fmt.stat {
            write_stat(out, &rows)?;
        }
        if fmt.shortstat && !fmt.stat {
            if let Some(line) = shortstat_line(&rows) {
                writeln!(out, "{line}")?;
            }
        }
        separator = true;
    }
    if fmt.patch {
        if separator {
            writeln!(out)?;
        }
        for c in changes {
            let (a, b) = (old_bytes(repo, c)?, new_bytes(repo, c)?);
            write_file_patch_ctx(
                out,
                repo.object_format(),
                &c.path,
                a.as_deref(),
                b.as_deref(),
                fmt.context,
            )?;
        }
    }
    Ok(())
}
