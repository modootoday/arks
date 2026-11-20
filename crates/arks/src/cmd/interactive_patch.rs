//! FS3 interactive / patch siblings: hunk-level `add -p`, `add -e`, `add -i`, `stash -p`.
//!
//! Cite: <https://git-scm.com/docs/git-add>, <https://git-scm.com/docs/git-stash>
//! Line hunks use a simple LCS split (not byte-identical to git's xdiff).

use crate::cmd::add_patch::{self, PatchMode};
use crate::cmd::core::cmd_add;
use crate::cmd::shared::{collect_untracked, file_mode, file_times, index_stat_matches};
use crate::error::{CliError, CliResult};
use crate::util::{open_repo, read_index, refuse_unknown_flags, work_tree, write_index};
use arks_index::{IndexEntry, sort_entries};
use arks_object::{ObjectKind, RawObject, write_loose};
use arks_repo::Repository;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{self, Write};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct FileChange {
    pub path: String,
    pub old: Vec<u8>,
    pub new: Vec<u8>,
    /// True when path is untracked (old empty / not in index).
    pub untracked: bool,
}

#[derive(Debug, Clone)]
pub struct Hunk {
    pub old_start: usize, // 1-based
    pub old_count: usize,
    pub new_start: usize,
    pub new_count: usize,
    pub lines: Vec<HunkLine>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HunkLineKind {
    Context,
    Delete,
    Insert,
}

#[derive(Debug, Clone)]
pub struct HunkLine {
    pub kind: HunkLineKind,
    pub text: String,
}

/// Collect dirty tracked + untracked file changes (WT vs index for tracked).
pub fn collect_file_changes(repo: &Repository, work: &Path) -> CliResult<Vec<FileChange>> {
    let index = read_index(repo)?;
    let mut out = Vec::new();
    let stage0: HashMap<&str, &IndexEntry> = index
        .entries
        .iter()
        .filter(|e| e.stage() == 0)
        .map(|e| (e.path.as_str(), e))
        .collect();
    for (path, entry) in &stage0 {
        let abs = work.join(path);
        if !abs.is_file() {
            let old = repo.get_object(&entry.oid)?.payload;
            out.push(FileChange {
                path: (*path).to_string(),
                old,
                new: Vec::new(),
                untracked: false,
            });
            continue;
        }
        let meta = fs::metadata(&abs)?;
        if index_stat_matches(entry, &meta) {
            continue;
        }
        let new = fs::read(&abs)?;
        let old = repo.get_object(&entry.oid)?.payload;
        if old == new {
            continue;
        }
        out.push(FileChange {
            path: (*path).to_string(),
            old,
            new,
            untracked: false,
        });
    }
    let indexed: HashSet<_> = stage0.keys().map(|s| (*s).to_string()).collect();
    let mut untracked = Vec::new();
    collect_untracked(work, work, &indexed, &mut untracked)?;
    for path in untracked {
        let abs = work.join(&path);
        if !abs.is_file() {
            continue;
        }
        let new = fs::read(&abs)?;
        out.push(FileChange {
            path,
            old: Vec::new(),
            new,
            untracked: true,
        });
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(out)
}

/// Split old/new into unified-style hunks with 3 lines of context.
pub fn split_hunks(old: &[u8], new: &[u8]) -> Vec<Hunk> {
    let old_lines: Vec<String> = lines_owned(old);
    let new_lines: Vec<String> = lines_owned(new);
    if old_lines.is_empty() && new_lines.is_empty() {
        return Vec::new();
    }
    let edits = lcs_edits(&old_lines, &new_lines);
    // Group into raw change runs then expand with context.
    let mut raw: Vec<(usize, usize, Vec<HunkLine>)> = Vec::new();
    // Track positions in old/new while walking edits.
    let mut oi = 0usize;
    let mut ni = 0usize;
    let mut cur: Vec<HunkLine> = Vec::new();
    let mut cur_old_start = 0usize;
    let mut cur_new_start = 0usize;
    let mut in_hunk = false;
    let flush = |cur: &mut Vec<HunkLine>,
                 in_hunk: &mut bool,
                 raw: &mut Vec<(usize, usize, Vec<HunkLine>)>,
                 old_start: usize,
                 new_start: usize| {
        if *in_hunk && !cur.is_empty() {
            raw.push((old_start, new_start, std::mem::take(cur)));
        }
        *in_hunk = false;
    };
    for ed in &edits {
        match ed {
            Edit::Equal => {
                if in_hunk {
                    // keep a little context inside; flush after 6 consecutive equals later
                    cur.push(HunkLine {
                        kind: HunkLineKind::Context,
                        text: old_lines[oi].clone(),
                    });
                    // If we have >6 trailing context, split.
                    let trailing = cur
                        .iter()
                        .rev()
                        .take_while(|l| l.kind == HunkLineKind::Context)
                        .count();
                    if trailing > 6 {
                        // drop extras beyond 3 and flush
                        while cur
                            .iter()
                            .rev()
                            .take_while(|l| l.kind == HunkLineKind::Context)
                            .count()
                            > 3
                        {
                            cur.pop();
                        }
                        flush(
                            &mut cur,
                            &mut in_hunk,
                            &mut raw,
                            cur_old_start,
                            cur_new_start,
                        );
                    }
                }
                oi += 1;
                ni += 1;
            }
            Edit::Delete => {
                if !in_hunk {
                    in_hunk = true;
                    let start = oi.saturating_sub(3);
                    for line in old_lines.iter().take(oi).skip(start) {
                        cur.push(HunkLine {
                            kind: HunkLineKind::Context,
                            text: line.clone(),
                        });
                    }
                    cur_old_start = start;
                    cur_new_start = ni.saturating_sub(oi - start);
                }
                cur.push(HunkLine {
                    kind: HunkLineKind::Delete,
                    text: old_lines[oi].clone(),
                });
                oi += 1;
            }
            Edit::Insert => {
                if !in_hunk {
                    in_hunk = true;
                    let start = oi.saturating_sub(3);
                    for line in old_lines.iter().take(oi).skip(start) {
                        cur.push(HunkLine {
                            kind: HunkLineKind::Context,
                            text: line.clone(),
                        });
                    }
                    cur_old_start = start;
                    cur_new_start = ni.saturating_sub(oi - start);
                }
                cur.push(HunkLine {
                    kind: HunkLineKind::Insert,
                    text: new_lines[ni].clone(),
                });
                ni += 1;
            }
        }
    }
    flush(
        &mut cur,
        &mut in_hunk,
        &mut raw,
        cur_old_start,
        cur_new_start,
    );
    // If no edits, whole-file as one hunk when content differs (binary-ish).
    if raw.is_empty() && old != new {
        let mut lines = Vec::new();
        for l in &old_lines {
            lines.push(HunkLine {
                kind: HunkLineKind::Delete,
                text: l.clone(),
            });
        }
        for l in &new_lines {
            lines.push(HunkLine {
                kind: HunkLineKind::Insert,
                text: l.clone(),
            });
        }
        raw.push((0, 0, lines));
    }
    raw.into_iter()
        .map(|(os, ns, lines)| {
            let old_count = lines
                .iter()
                .filter(|l| l.kind != HunkLineKind::Insert)
                .count();
            let new_count = lines
                .iter()
                .filter(|l| l.kind != HunkLineKind::Delete)
                .count();
            Hunk {
                old_start: os + 1,
                old_count,
                new_start: ns + 1,
                new_count,
                lines,
            }
        })
        .collect()
}

fn lines_owned(bytes: &[u8]) -> Vec<String> {
    if bytes.is_empty() {
        return Vec::new();
    }
    String::from_utf8_lossy(bytes)
        .lines()
        .map(|s| s.to_string())
        .collect()
}

#[derive(Clone, Copy)]
enum Edit {
    Equal,
    Delete,
    Insert,
}

fn lcs_edits(a: &[String], b: &[String]) -> Vec<Edit> {
    let n = a.len();
    let m = b.len();
    // DP table — fine for typical file sizes in interactive patch.
    let mut dp = vec![vec![0u32; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            dp[i][j] = if a[i] == b[j] {
                dp[i + 1][j + 1] + 1
            } else {
                dp[i + 1][j].max(dp[i][j + 1])
            };
        }
    }
    let mut out = Vec::new();
    let mut i = 0;
    let mut j = 0;
    while i < n && j < m {
        if a[i] == b[j] {
            out.push(Edit::Equal);
            i += 1;
            j += 1;
        } else if dp[i + 1][j] >= dp[i][j + 1] {
            out.push(Edit::Delete);
            i += 1;
        } else {
            out.push(Edit::Insert);
            j += 1;
        }
    }
    while i < n {
        out.push(Edit::Delete);
        i += 1;
    }
    while j < m {
        out.push(Edit::Insert);
        j += 1;
    }
    out
}

/// Apply selected hunks onto `old` to produce staged bytes.
pub fn apply_selected_hunks(old: &[u8], hunks: &[Hunk], selected: &[bool]) -> Vec<u8> {
    debug_assert_eq!(hunks.len(), selected.len());
    let old_lines = lines_owned(old);
    let mut out_lines: Vec<String> = Vec::new();
    let mut cursor = 0usize; // 0-based into old_lines
    for (h, take) in hunks.iter().zip(selected.iter()) {
        let start = h.old_start.saturating_sub(1);
        while cursor < start && cursor < old_lines.len() {
            out_lines.push(old_lines[cursor].clone());
            cursor += 1;
        }
        if *take {
            for line in &h.lines {
                match line.kind {
                    HunkLineKind::Context | HunkLineKind::Insert => {
                        out_lines.push(line.text.clone())
                    }
                    HunkLineKind::Delete => {}
                }
            }
        } else {
            for line in &h.lines {
                match line.kind {
                    HunkLineKind::Context | HunkLineKind::Delete => {
                        out_lines.push(line.text.clone())
                    }
                    HunkLineKind::Insert => {}
                }
            }
        }
        cursor = start + h.old_count;
    }
    while cursor < old_lines.len() {
        out_lines.push(old_lines[cursor].clone());
        cursor += 1;
    }
    let mut bytes = out_lines.join("\n").into_bytes();
    if !old.is_empty() && old.ends_with(b"\n") && !bytes.is_empty() && !bytes.ends_with(b"\n") {
        bytes.push(b'\n');
    }
    if old.is_empty() && !hunks.is_empty() && selected.iter().any(|s| *s) && !bytes.ends_with(b"\n")
    {
        // untracked new file — git usually keeps final newline if present in WT
        if let Some(h) = hunks.last() {
            if h.lines
                .iter()
                .any(|l| l.kind == HunkLineKind::Insert && l.text.is_empty())
            {
                // no-op
            }
        }
    }
    bytes
}

fn render_hunk_patch(path: &str, h: &Hunk) -> String {
    let mut s = format!("diff --git a/{path} b/{path}\n");
    s.push_str(&format!(
        "@@ -{},{} +{},{} @@\n",
        h.old_start, h.old_count, h.new_start, h.new_count
    ));
    for line in &h.lines {
        let prefix = match line.kind {
            HunkLineKind::Context => ' ',
            HunkLineKind::Delete => '-',
            HunkLineKind::Insert => '+',
        };
        s.push(prefix);
        s.push_str(&line.text);
        s.push('\n');
    }
    s
}

fn parse_edited_hunk(text: &str) -> CliResult<Hunk> {
    let mut lines = Vec::new();
    let mut old_start = 1;
    let mut new_start = 1;
    for raw in text.lines() {
        if raw.starts_with("@@ ") {
            // @@ -a,b +c,d @@
            let rest = raw.trim_start_matches('@').trim();
            let parts: Vec<&str> = rest.split_whitespace().collect();
            if parts.len() >= 2 {
                if let Some(o) = parts[0].strip_prefix('-') {
                    old_start = o.split(',').next().unwrap_or("1").parse().unwrap_or(1);
                }
                if let Some(n) = parts[1].strip_prefix('+') {
                    new_start = n.split(',').next().unwrap_or("1").parse().unwrap_or(1);
                }
            }
            continue;
        }
        if raw.starts_with("diff ") || raw.starts_with("--- ") || raw.starts_with("+++ ") {
            continue;
        }
        if raw.is_empty() {
            continue;
        }
        let (kind, text) = match raw.as_bytes()[0] {
            b'+' => (HunkLineKind::Insert, &raw[1..]),
            b'-' => (HunkLineKind::Delete, &raw[1..]),
            b' ' => (HunkLineKind::Context, &raw[1..]),
            _ => (HunkLineKind::Context, raw),
        };
        lines.push(HunkLine {
            kind,
            text: text.to_string(),
        });
    }
    let old_count = lines
        .iter()
        .filter(|l| l.kind != HunkLineKind::Insert)
        .count();
    let new_count = lines
        .iter()
        .filter(|l| l.kind != HunkLineKind::Delete)
        .count();
    Ok(Hunk {
        old_start,
        old_count,
        new_start,
        new_count,
        lines,
    })
}

fn run_editor_on_text(initial: &str) -> CliResult<String> {
    let editor = std::env::var("ARKS_EDITOR")
        .or_else(|_| std::env::var("GIT_EDITOR"))
        .or_else(|_| std::env::var("EDITOR"))
        .map_err(|_| {
            CliError::Usage("patch edit requires GIT_EDITOR / ARKS_EDITOR / EDITOR (FS3)".into())
        })?;
    let dir = std::env::temp_dir();
    let path = dir.join(format!("arks-patch-edit-{}.patch", std::process::id()));
    fs::write(&path, initial)?;
    let mut argv: Vec<String> = editor.split_whitespace().map(|s| s.to_string()).collect();
    if argv.is_empty() {
        return Err(CliError::Usage("empty editor".into()));
    }
    argv.push(path.display().to_string());
    let status = std::process::Command::new(&argv[0])
        .args(&argv[1..])
        .status()
        .map_err(|e| CliError::Msg(format!("spawn editor: {e}")))?;
    if !status.success() {
        let _ = fs::remove_file(&path);
        return Err(CliError::Msg(format!(
            "editor exited {}",
            status.code().unwrap_or(-1)
        )));
    }
    let text = fs::read_to_string(&path)?;
    let _ = fs::remove_file(&path);
    Ok(text)
}

fn stage_bytes(repo: &Repository, work: &Path, path: &str, bytes: &[u8]) -> CliResult<()> {
    let objects_dir = repo.store().objects_dir().to_path_buf();
    // Write temp content? For staging we hash `bytes` directly, not WT.
    let obj = RawObject::new(ObjectKind::Blob, bytes.to_vec());
    let oid = write_loose(&objects_dir, repo.object_format(), &obj)?;
    let abs = work.join(path);
    // Prefer WT meta if file exists; else synthetic.
    let (meta_mode, times, size) = if abs.is_file() {
        let meta = fs::metadata(&abs)?;
        (
            file_mode(&meta),
            file_times(&meta),
            meta.len().min(u32::MAX as u64) as u32,
        )
    } else {
        (
            0o100644,
            (0, 0, 0, 0),
            bytes.len().min(u32::MAX as usize) as u32,
        )
    };
    let entry = IndexEntry {
        ctime_sec: times.0,
        ctime_nsec: times.1,
        mtime_sec: times.2,
        mtime_nsec: times.3,
        dev: 0,
        ino: 0,
        mode: meta_mode,
        uid: 0,
        gid: 0,
        size,
        oid,
        flags: (path.len().min(0x0fff)) as u16,
        flags_extended: None,
        path: path.to_owned(),
    };
    let mut index = read_index(repo)?;
    index
        .entries
        .retain(|e| !(e.path == path && e.stage() == 0));
    index.entries.push(entry);
    sort_entries(&mut index.entries);
    write_index(repo, &index)?;
    Ok(())
}

/// `add -p` / `--patch`: see [`crate::cmd::add_patch`].
pub fn cmd_add_patch_hunks(args: &[String]) -> CliResult<()> {
    let specs: Vec<String> = args
        .iter()
        .filter(|a| a.as_str() != "-p" && a.as_str() != "--")
        .cloned()
        .collect();
    let specs = rel_specs(&specs)?;
    add_patch::run(PatchMode::Stage, None, &specs)?;
    Ok(())
}

/// Pathspecs relative to the work tree root.
fn rel_specs(specs: &[String]) -> CliResult<Vec<String>> {
    if specs.is_empty() {
        return Ok(Vec::new());
    }
    let repo = open_repo()?;
    let work = work_tree(&repo)?;
    let cwd = std::env::current_dir()?;
    let mut out = Vec::new();
    for s in specs {
        let abs = cwd.join(s);
        match crate::util::rel_path_from_worktree(&work, &abs) {
            Ok(r) => out.push(r),
            Err(_) => out.push(s.clone()),
        }
    }
    Ok(out)
}

/// FS3 `add -e`: edit full-file patch then stage result.
pub fn cmd_add_edit(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &["-e", "--edit"])?;
    let repo = open_repo()?;
    let work = work_tree(&repo)?;
    // `add -e` edits the diff of tracked files only (untracked files are not part of it).
    let changes: Vec<_> = collect_file_changes(&repo, &work)?
        .into_iter()
        .filter(|c| !c.untracked)
        .collect();
    if changes.is_empty() {
        println!("No changes.");
        return Ok(());
    }
    for ch in &changes {
        let hunks = split_hunks(&ch.old, &ch.new);
        let mut patch = format!("diff --git a/{} b/{}\n", ch.path, ch.path);
        patch.push_str(&format!("--- a/{}\n+++ b/{}\n", ch.path, ch.path));
        for h in &hunks {
            patch.push_str(&render_hunk_patch(&ch.path, h));
        }
        let edited = run_editor_on_text(&patch)?;
        // Re-parse all hunks from edited text and apply all as selected.
        let mut parsed = Vec::new();
        let mut buf = String::new();
        let mut in_hunk = false;
        for line in edited.lines() {
            if line.starts_with("@@ ") {
                if in_hunk {
                    parsed.push(parse_edited_hunk(&buf)?);
                    buf.clear();
                }
                in_hunk = true;
                buf.push_str(line);
                buf.push('\n');
            } else if in_hunk {
                if line.starts_with("diff --git ") {
                    parsed.push(parse_edited_hunk(&buf)?);
                    buf.clear();
                    in_hunk = false;
                } else {
                    buf.push_str(line);
                    buf.push('\n');
                }
            }
        }
        if in_hunk && !buf.is_empty() {
            parsed.push(parse_edited_hunk(&buf)?);
        }
        if parsed.is_empty() {
            continue;
        }
        let sel = vec![true; parsed.len()];
        let staged = apply_selected_hunks(&ch.old, &parsed, &sel);
        stage_bytes(&repo, &work, &ch.path, &staged)?;
    }
    Ok(())
}

/// `+<added>/-<deleted>` line counts (`binary` for binary content).
fn numstat_cell(old: &[u8], new: &[u8]) -> String {
    use crate::cmd::diffcore::xdiff;
    if xdiff::is_binary(old) || xdiff::is_binary(new) {
        return "binary".into();
    }
    let a = xdiff::split_lines(old);
    let b = xdiff::split_lines(new);
    let (ins, del) = xdiff::count(&xdiff::diff_lines(&a, &b, &xdiff::LineOpts::default()));
    format!("+{ins}/-{del}")
}

/// `add -i` status rows: `(path, staged, unstaged)` for every tracked path
/// that differs between HEAD and the index or the index and the work tree.
fn status_rows(specs: &[String]) -> CliResult<Vec<(String, String, String)>> {
    let repo = open_repo()?;
    let work = work_tree(&repo)?;
    let index = read_index(&repo)?;
    let mut head: Vec<IndexEntry> = Vec::new();
    if let Ok(h) = repo.head_oid() {
        let tree = crate::cmd::log_format::read_commit(&repo, &h)?.tree;
        crate::cmd::plumbing::flatten_tree_to_index(&repo, &tree, "", &mut head)?;
    }
    let head: HashMap<String, arks_oid::Oid> = head.into_iter().map(|e| (e.path, e.oid)).collect();
    let stage0: HashMap<&str, &IndexEntry> = index
        .entries
        .iter()
        .filter(|e| e.stage() == 0)
        .map(|e| (e.path.as_str(), e))
        .collect();
    let blob = |oid: Option<&arks_oid::Oid>| -> CliResult<Vec<u8>> {
        Ok(match oid {
            Some(o) => repo.get_object(o)?.payload,
            None => Vec::new(),
        })
    };
    let mut paths: Vec<&str> = head
        .keys()
        .map(String::as_str)
        .chain(stage0.keys().copied())
        .collect();
    paths.sort_unstable();
    paths.dedup();
    let mut rows = Vec::new();
    for path in paths {
        if !specs.is_empty() && !crate::cmd::log_format::path_in_specs(path, specs) {
            continue;
        }
        let h = head.get(path);
        let i = stage0.get(path).map(|e| &e.oid);
        let staged = if h == i {
            "unchanged".to_string()
        } else {
            numstat_cell(&blob(h)?, &blob(i)?)
        };
        let unstaged = match stage0.get(path) {
            None => "nothing".to_string(),
            Some(e) => {
                let abs = work.join(path);
                match fs::metadata(&abs) {
                    Ok(m) if index_stat_matches(e, &m) => "nothing".to_string(),
                    Ok(m) if m.is_file() => {
                        let now = fs::read(&abs)?;
                        let old = blob(Some(&e.oid))?;
                        if now == old {
                            "nothing".to_string()
                        } else {
                            numstat_cell(&old, &now)
                        }
                    }
                    _ => numstat_cell(&blob(Some(&e.oid))?, &[]),
                }
            }
        };
        if staged != "unchanged" || unstaged != "nothing" {
            rows.push((path.to_string(), staged, unstaged));
        }
    }
    Ok(rows)
}

fn print_status_rows(rows: &[(String, String, String)]) {
    if rows.is_empty() {
        return;
    }
    println!("     {:>12} {:>12} path", "staged", "unstaged");
    for (n, (path, staged, unstaged)) in rows.iter().enumerate() {
        println!("{:>3}: {staged:>12} {unstaged:>12} {path}", n + 1);
    }
}

fn print_main_menu() {
    println!("*** Commands ***");
    println!("  1: [s]tatus\t  2: [u]pdate\t  3: [r]evert\t  4: [a]dd untracked");
    println!("  5: [p]atch\t  6: [d]iff\t  7: [q]uit\t  8: [h]elp");
}

/// Stage every tracked modification (`update` with all paths selected).
fn update_all_tracked() -> CliResult<()> {
    let repo = open_repo()?;
    let work = work_tree(&repo)?;
    let changes = collect_file_changes(&repo, &work)?;
    let paths: Vec<String> = changes
        .into_iter()
        .filter(|c| !c.untracked)
        .map(|c| c.path)
        .collect();
    drop(repo);
    if paths.is_empty() {
        println!("No tracked changes to update.");
    } else {
        cmd_add(&paths)?;
        println!("updated {} path(s)", paths.len());
    }
    Ok(())
}

/// `add -i`: the status table and the command menu of git-add(1)
/// INTERACTIVE MODE (layout as observed from pin 2.39.5). `status` and
/// `quit` (or end of input: `Bye.`) follow the pin; `update` stages every
/// tracked change and `patch` runs `add -p` (Partial: no path selection
/// prompts). `ARKS_ADD_INTERACTIVE_ANSWERS` scripts the answers.
pub fn cmd_add_interactive(args: &[String]) -> CliResult<()> {
    let flags: Vec<String> = args
        .iter()
        .take_while(|a| a.starts_with('-') && a.as_str() != "--")
        .cloned()
        .collect();
    refuse_unknown_flags(&flags, &["-i", "--interactive"])?;
    let specs = {
        let repo = open_repo()?;
        let work = work_tree(&repo)?;
        args.iter()
            .skip(flags.len())
            .filter(|a| a.as_str() != "--")
            .map(|p| crate::cmd::core::pathspec_rel(&work, p))
            .collect::<CliResult<Vec<_>>>()?
    };
    let mut scripted: Option<std::vec::IntoIter<String>> =
        std::env::var("ARKS_ADD_INTERACTIVE_ANSWERS").ok().map(|s| {
            s.split(|c: char| c.is_whitespace() || c == ',' || c == ';')
                .filter(|p| !p.is_empty())
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
                .into_iter()
        });
    print_status_rows(&status_rows(&specs)?);
    println!();
    print_main_menu();
    loop {
        print!("What now> ");
        io::stdout().flush().ok();
        let answer = match scripted.as_mut() {
            Some(it) => it.next().inspect(|c| println!("{c}")),
            None => {
                let mut line = String::new();
                match io::stdin().read_line(&mut line)? {
                    0 => None,
                    _ => Some(line.trim().to_string()),
                }
            }
        };
        let Some(choice) = answer else {
            println!();
            println!("Bye.");
            break;
        };
        match choice.as_str() {
            "1" | "s" | "status" => {
                print_status_rows(&status_rows(&specs)?);
                println!();
            }
            "2" | "u" | "update" => update_all_tracked()?,
            "5" | "p" | "patch" => cmd_add_patch_hunks(&[])?,
            "7" | "q" | "quit" => {
                println!("Bye.");
                break;
            }
            "" => {}
            other => eprintln!("Huh ({other})?"),
        }
        print_main_menu();
    }
    Ok(())
}

/// Whether `rev` names the commit HEAD points at.
fn is_head(repo: &arks_repo::Repository, rev: &str) -> bool {
    let a = crate::cmd::shared::resolve_rev(repo, rev).ok();
    let b = repo.head_oid().ok();
    rev == "HEAD" || (a.is_some() && a == b)
}

/// `reset -p [<tree-ish>] [--] [<pathspec>...]`.
pub fn cmd_reset_patch(args: &[String]) -> CliResult<()> {
    let repo = open_repo()?;
    let rest: Vec<String> = args
        .iter()
        .filter(|a| a.as_str() != "-p")
        .cloned()
        .collect();
    let (rev, specs) = add_patch::rev_and_paths(&repo, &rest);
    let specs = rel_specs(&specs)?;
    let mode = match &rev {
        Some(r) if !is_head(&repo, r) => PatchMode::ResetNothead,
        _ => PatchMode::ResetHead,
    };
    drop(repo);
    add_patch::run(mode, rev.as_deref(), &specs)?;
    Ok(())
}

/// `checkout -p [<tree-ish>] [--] [<pathspec>...]`.
pub fn cmd_checkout_patch(args: &[String]) -> CliResult<()> {
    let repo = open_repo()?;
    let rest: Vec<String> = args
        .iter()
        .filter(|a| a.as_str() != "-p")
        .cloned()
        .collect();
    let (rev, specs) = add_patch::rev_and_paths(&repo, &rest);
    let specs = rel_specs(&specs)?;
    let mode = match &rev {
        None => PatchMode::CheckoutIndex,
        Some(r) if is_head(&repo, r) => PatchMode::CheckoutHead,
        Some(_) => PatchMode::CheckoutNothead,
    };
    drop(repo);
    add_patch::run(mode, rev.as_deref(), &specs)?;
    Ok(())
}

/// `restore -p [--source=<tree>] [--staged] [--worktree] [--] <pathspec>...`.
pub fn cmd_restore_patch(
    source: Option<&str>,
    staged: bool,
    worktree: bool,
    specs: &[String],
) -> CliResult<()> {
    let repo = open_repo()?;
    let worktree = worktree || !staged;
    let head = source.is_none_or(|s| is_head(&repo, s));
    let mode = match (staged, worktree) {
        (true, true) if head => PatchMode::CheckoutHead,
        (true, true) => PatchMode::CheckoutNothead,
        (true, false) if head => PatchMode::ResetHead,
        (true, false) => PatchMode::ResetNothead,
        _ => match source {
            None => PatchMode::CheckoutIndex,
            Some(_) if head => PatchMode::WorktreeHead,
            Some(_) => PatchMode::WorktreeNothead,
        },
    };
    let rev = source
        .map(str::to_string)
        .or_else(|| staged.then(|| "HEAD".to_string()));
    let specs = rel_specs(specs)?;
    drop(repo);
    add_patch::run(mode, rev.as_deref(), &specs)?;
    Ok(())
}

/// `stash push -p [-m <message>] [-q] [--] [<pathspec>...]`: stash the
/// selected hunks (index kept), then roll them back from the work tree.
pub fn cmd_stash_patch(args: &[String]) -> CliResult<()> {
    let mut message: Option<String> = None;
    let mut quiet = false;
    let mut specs = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-p" | "--patch" | "push" | "--" => {}
            "-q" | "--quiet" => quiet = true,
            "-m" | "--message" => {
                i += 1;
                message = args.get(i).cloned();
            }
            a if a.starts_with("--message=") => message = Some(a["--message=".len()..].to_string()),
            a => specs.push(a.to_string()),
        }
        i += 1;
    }
    let specs = rel_specs(&specs)?;
    let Some(applied) = add_patch::run_files(PatchMode::Stash, None, &specs)? else {
        if !quiet {
            println!("No local changes to save");
        }
        return Ok(());
    };
    if applied.is_empty() {
        return Err(CliError::Exit(1, "No changes selected".into()));
    }
    let selected: Vec<(String, add_patch::Side)> = applied
        .iter()
        .map(|(p, st, _)| (p.clone(), st.clone()))
        .collect();
    crate::cmd::stash::push_patch_stash(&selected, message.as_deref(), quiet)?;
    let repo = open_repo()?;
    let work = work_tree(&repo)?;
    for (p, _, remain) in &applied {
        let abs = work.join(p);
        match remain {
            Some((_, bytes)) => fs::write(&abs, bytes)?,
            None => {
                let _ = fs::remove_file(&abs);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hunk_split_and_apply_partial() {
        let old = b"a\nb\nc\nd\n";
        let new = b"a\nB\nc\nD\n";
        let hunks = split_hunks(old, new);
        assert!(!hunks.is_empty());
        let mut sel = vec![false; hunks.len()];
        sel[0] = true;
        let staged = apply_selected_hunks(old, &hunks, &sel);
        let s = String::from_utf8_lossy(&staged);
        assert!(s.contains('B') || s.contains('D') || s.contains('a'));
    }
}
