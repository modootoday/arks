//! Status collection (HEAD vs index vs working tree) and the long
//! (human) `git status` rendering.
//!
//! Long format layout per git-status §OUTPUT (wt-status.c): branch line,
//! `Changes to be committed:`, `Changes not staged for commit:`,
//! `Untracked files:` sections (each with its advice lines and a trailing
//! blank line), then one summary line when nothing is staged.
//! Primary: <https://git-scm.com/docs/git-status>.
//!
//! Partial: upstream ahead/behind lines and in-progress operation notes
//! (merge, rebase, cherry-pick) are not printed.

use crate::cmd::shared::{collect_untracked, head_blob_map, index_stat_matches};
use crate::error::CliResult;
use crate::util::{read_index, work_tree};
use arks_object::{ObjectKind, RawObject};
use arks_refs::RefTarget;
use arks_repo::Repository;
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::io::Write;

/// Snapshot of the three-way status.
pub struct StatusData {
    /// `Some(name)` on a branch, `None` when detached.
    pub branch: Option<String>,
    pub head_abbrev: Option<String>,
    /// No commit on the current branch yet.
    pub initial: bool,
    /// Index vs HEAD: path -> `A` / `M` / `D`.
    pub staged: BTreeMap<String, char>,
    /// Working tree vs index: path -> `M` / `D`.
    pub unstaged: BTreeMap<String, char>,
    /// Untracked paths (directories collapsed as `dir/` unless `all`).
    pub untracked: Vec<String>,
    /// Ignored untracked paths (`--ignored`; directories as `dir/`).
    pub ignored: Vec<String>,
    /// Current directory relative to the work tree (`""` or `dir/sub/`).
    pub prefix: String,
}

impl StatusData {
    /// Path as shown by short / long status (`status.relativePaths`
    /// default): relative to the current directory.
    pub fn show(&self, path: &str) -> String {
        if self.prefix.is_empty() {
            return path.to_string();
        }
        let pre: Vec<&str> = self.prefix.trim_end_matches('/').split('/').collect();
        let comps: Vec<&str> = path.split('/').collect();
        let mut common = 0;
        while common < pre.len() && common + 1 < comps.len() && pre[common] == comps[common] {
            common += 1;
        }
        let mut out = "../".repeat(pre.len() - common);
        out.push_str(&comps[common..].join("/"));
        out
    }
}

/// Untracked-file listing mode (`-u<mode>`).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Untracked {
    No,
    Normal,
    All,
}

pub fn collect(repo: &Repository, mode: Untracked) -> CliResult<StatusData> {
    let work = work_tree(repo)?;
    let index = read_index(repo)?;
    let head_paths = head_blob_map(repo)?;
    let (branch, head_abbrev) = match repo.head_target()? {
        RefTarget::Symbolic(s) => (
            Some(s.strip_prefix("refs/heads/").unwrap_or(&s).to_string()),
            None,
        ),
        RefTarget::Direct(o) => (None, Some(crate::cmd::shared::abbrev_hex(repo, &o, 7))),
    };
    let initial = repo.head_oid().is_err();
    let mut staged = BTreeMap::new();
    let mut unstaged = BTreeMap::new();
    for e in index.entries.iter().filter(|e| e.stage() == 0) {
        match head_paths.get(&e.path) {
            None => {
                staged.insert(e.path.clone(), 'A');
            }
            Some(oid) if *oid != e.oid => {
                staged.insert(e.path.clone(), 'M');
            }
            Some(_) => {}
        }
        let abs = work.join(&e.path);
        if !abs.is_file() {
            unstaged.insert(e.path.clone(), 'D');
            continue;
        }
        // Skip re-hash when index stat cache matches (git ce_match_stat class).
        let meta = fs::metadata(&abs)?;
        if index_stat_matches(e, &meta) {
            continue;
        }
        let oid = RawObject::new(ObjectKind::Blob, fs::read(&abs)?).oid(repo.object_format())?;
        if oid != e.oid {
            unstaged.insert(e.path.clone(), 'M');
        }
    }
    let indexed: HashSet<String> = index.entries.iter().map(|e| e.path.clone()).collect();
    for p in head_paths.keys() {
        if !indexed.contains(p) {
            staged.insert(p.clone(), 'D');
        }
    }
    let mut untracked = Vec::new();
    if mode != Untracked::No {
        collect_untracked(&work, &work, &indexed, &mut untracked)?;
    }
    if mode == Untracked::Normal {
        untracked = crate::cmd::core::collapse_untracked_dirs(&untracked, &indexed);
    }
    let canon = |p: &std::path::Path| p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
    let prefix = crate::util::cwd()
        .ok()
        .and_then(|c| {
            canon(&c)
                .strip_prefix(canon(&work))
                .ok()
                .map(|r| r.to_string_lossy().replace('\\', "/"))
        })
        .filter(|r| !r.is_empty())
        .map(|r| format!("{r}/"))
        .unwrap_or_default();
    Ok(StatusData {
        prefix,
        branch,
        head_abbrev,
        initial,
        staged,
        unstaged,
        untracked,
        ignored: Vec::new(),
    })
}

/// Ignored, untracked paths below the work tree for `status --ignored`:
/// an excluded directory without tracked files is listed once as `dir/`.
pub fn collect_ignored(repo: &Repository) -> CliResult<Vec<String>> {
    let work = work_tree(repo)?;
    let indexed: HashSet<String> = read_index(repo)?
        .entries
        .into_iter()
        .map(|e| e.path)
        .collect();
    let mut ig = crate::cmd::ignore::Ignore::new(&work);
    let mut out = Vec::new();
    ignored_rec(&work, &work, &indexed, &mut ig, &mut out)?;
    out.sort();
    Ok(out)
}

fn ignored_rec(
    work: &std::path::Path,
    dir: &std::path::Path,
    indexed: &HashSet<String>,
    ig: &mut crate::cmd::ignore::Ignore,
    out: &mut Vec<String>,
) -> CliResult<()> {
    for ent in fs::read_dir(dir)? {
        let ent = ent?;
        if ent.file_name() == ".git" {
            continue;
        }
        let path = ent.path();
        let rel = path
            .strip_prefix(work)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        let is_dir = path.is_dir();
        if indexed.contains(&rel) {
            continue;
        }
        let has_tracked = is_dir && indexed.iter().any(|p| p.starts_with(&format!("{rel}/")));
        if ig.is_ignored(&rel, is_dir) && !has_tracked {
            out.push(if is_dir { format!("{rel}/") } else { rel });
        } else if is_dir {
            ignored_rec(work, &path, indexed, ig, out)?;
        }
    }
    Ok(())
}

fn label(code: char) -> &'static str {
    match code {
        'A' => "new file:",
        'D' => "deleted:",
        _ => "modified:",
    }
}

/// Long-format report (git-status default output).
pub fn write_long(out: &mut impl Write, d: &StatusData, mode: Untracked) -> CliResult<()> {
    match (&d.branch, &d.head_abbrev) {
        (Some(b), _) => writeln!(out, "On branch {b}")?,
        (None, Some(h)) => writeln!(out, "HEAD detached at {h}")?,
        (None, None) => writeln!(out, "Not currently on any branch.")?,
    }
    if d.initial {
        writeln!(out, "\nNo commits yet\n")?;
    }
    if !d.staged.is_empty() {
        writeln!(out, "Changes to be committed:")?;
        if d.initial {
            writeln!(out, "  (use \"git rm --cached <file>...\" to unstage)")?;
        } else {
            writeln!(out, "  (use \"git restore --staged <file>...\" to unstage)")?;
        }
        for (p, c) in &d.staged {
            writeln!(out, "\t{:<12}{}", label(*c), d.show(p))?;
        }
        writeln!(out)?;
    }
    if !d.unstaged.is_empty() {
        writeln!(out, "Changes not staged for commit:")?;
        if d.unstaged.values().any(|c| *c == 'D') {
            writeln!(
                out,
                "  (use \"git add/rm <file>...\" to update what will be committed)"
            )?;
        } else {
            writeln!(
                out,
                "  (use \"git add <file>...\" to update what will be committed)"
            )?;
        }
        writeln!(
            out,
            "  (use \"git restore <file>...\" to discard changes in working directory)"
        )?;
        for (p, c) in &d.unstaged {
            writeln!(out, "\t{:<12}{}", label(*c), d.show(p))?;
        }
        writeln!(out)?;
    }
    if !d.untracked.is_empty() {
        writeln!(out, "Untracked files:")?;
        writeln!(
            out,
            "  (use \"git add <file>...\" to include in what will be committed)"
        )?;
        for p in &d.untracked {
            writeln!(out, "\t{}", d.show(p))?;
        }
        writeln!(out)?;
    }
    if !d.ignored.is_empty() {
        writeln!(out, "Ignored files:")?;
        writeln!(
            out,
            "  (use \"git add -f <file>...\" to include in what will be committed)"
        )?;
        for p in &d.ignored {
            writeln!(out, "\t{}", d.show(p))?;
        }
        writeln!(out)?;
    }
    if !d.staged.is_empty() && mode == Untracked::No {
        writeln!(
            out,
            "Untracked files not listed (use -u option to show untracked files)"
        )?;
    }
    if d.staged.is_empty() {
        if !d.unstaged.is_empty() {
            writeln!(
                out,
                "no changes added to commit (use \"git add\" and/or \"git commit -a\")"
            )?;
        } else if !d.untracked.is_empty() {
            writeln!(
                out,
                "nothing added to commit but untracked files present (use \"git add\" to track)"
            )?;
        } else if d.initial {
            writeln!(
                out,
                "nothing to commit (create/copy files and use \"git add\" to track)"
            )?;
        } else if mode == Untracked::No {
            writeln!(out, "nothing to commit (use -u to show untracked files)")?;
        } else {
            writeln!(out, "nothing to commit, working tree clean")?;
        }
    }
    Ok(())
}

/// Collect + print the long status (used after `stash apply`/`pop`).
pub fn print_long_status(repo: &Repository) -> CliResult<()> {
    let d = collect(repo, Untracked::Normal)?;
    let mut out = std::io::BufWriter::new(std::io::stdout().lock());
    write_long(&mut out, &d, Untracked::Normal)?;
    out.flush()?;
    Ok(())
}
