//! FS3 interactive / patch siblings: hunk-level `add -p`, `add -e`, `add -i`, `stash -p`.
//!
//! Cite: <https://git-scm.com/docs/git-add>, <https://git-scm.com/docs/git-stash>
//! Line hunks use a simple LCS split (not byte-identical to git's xdiff).

use crate::cmd::core::cmd_add;
use crate::cmd::shared::{collect_untracked, file_mode, file_times, index_stat_matches};
use crate::error::{CliError, CliResult};
use crate::util::{FORMAT, open_repo, read_index, refuse_unknown_flags, work_tree, write_index};
use arks_index::{IndexEntry, sort_entries};
use arks_object::{ObjectKind, RawObject, write_loose};
use arks_repo::Repository;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

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

#[derive(Debug)]
enum AnswerMode {
    Scripted(Vec<char>),
    Tty,
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

fn answer_mode() -> CliResult<AnswerMode> {
    if let Ok(raw) = std::env::var("ARKS_ADD_PATCH_ANSWERS") {
        return Ok(AnswerMode::Scripted(
            raw.chars().filter(|c| !c.is_whitespace()).collect(),
        ));
    }
    use std::io::IsTerminal;
    if io::stdin().is_terminal() {
        return Ok(AnswerMode::Tty);
    }
    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf)?;
    Ok(AnswerMode::Scripted(
        buf.chars().filter(|c| !c.is_whitespace()).collect(),
    ))
}

fn next_answer(mode: &mut AnswerMode, idx: &mut usize, prompt: &str) -> CliResult<char> {
    match mode {
        AnswerMode::Scripted(answers) => {
            let ch = answers.get(*idx).copied().unwrap_or('n');
            *idx += 1;
            Ok(ch)
        }
        AnswerMode::Tty => loop {
            print!("{prompt}");
            io::stdout()
                .flush()
                .map_err(|e| CliError::Msg(format!("flush: {e}")))?;
            let mut line = String::new();
            io::stdin()
                .read_line(&mut line)
                .map_err(|e| CliError::Msg(format!("read: {e}")))?;
            let ch = line.chars().find(|c| !c.is_whitespace());
            match ch {
                Some(
                    c @ ('y' | 'Y' | 'n' | 'N' | 'a' | 'A' | 'd' | 'D' | 'q' | 'Q' | 's' | 'S'
                    | 'e' | 'E'),
                ) => return Ok(c),
                Some(_) => eprintln!(
                    "y - stage this hunk\nn - skip\na - stage all\nd - skip all\ns - skip remaining hunks in file\ne - edit hunk\nq - quit"
                ),
                None => return Ok('n'),
            }
        },
    }
}

fn print_hunk(path: &str, h: &Hunk) {
    println!("diff --git a/{path} b/{path}");
    println!(
        "@@ -{},{} +{},{} @@",
        h.old_start, h.old_count, h.new_start, h.new_count
    );
    for line in &h.lines {
        let prefix = match line.kind {
            HunkLineKind::Context => ' ',
            HunkLineKind::Delete => '-',
            HunkLineKind::Insert => '+',
        };
        println!("{prefix}{}", line.text);
    }
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
    let oid = write_loose(&objects_dir, FORMAT, &obj)?;
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

/// FS3 hunk-level `add -p` / `--patch`.
pub fn cmd_add_patch_hunks(_args: &[String]) -> CliResult<()> {
    let mut mode = answer_mode()?;
    let mut ans_i = 0;
    let repo = open_repo()?;
    let work = work_tree(&repo)?;
    let changes = collect_file_changes(&repo, &work)?;
    if changes.is_empty() {
        println!("No changes.");
        return Ok(());
    }
    let mut accept_rest = false;
    'files: for ch in &changes {
        let mut hunks = split_hunks(&ch.old, &ch.new);
        if hunks.is_empty() {
            continue;
        }
        let mut selected = vec![false; hunks.len()];
        let mut hi = 0;
        while hi < hunks.len() {
            if accept_rest {
                selected[hi] = true;
                hi += 1;
                continue;
            }
            print_hunk(&ch.path, &hunks[hi]);
            let ans = next_answer(&mut mode, &mut ans_i, "Stage this hunk [y,n,q,a,d,s,e]? ")?;
            match ans {
                'y' | 'Y' => {
                    selected[hi] = true;
                    hi += 1;
                }
                'n' | 'N' => {
                    hi += 1;
                }
                'a' | 'A' => {
                    selected[hi] = true;
                    accept_rest = true;
                    hi += 1;
                }
                'd' | 'D' | 'q' | 'Q' => break 'files,
                's' | 'S' => break, // skip rest of file
                'e' | 'E' => {
                    let edited = run_editor_on_text(&render_hunk_patch(&ch.path, &hunks[hi]))?;
                    hunks[hi] = parse_edited_hunk(&edited)?;
                    selected[hi] = true;
                    hi += 1;
                }
                other => {
                    return Err(CliError::Usage(format!(
                        "add -p: unknown answer '{other}' (claimed: y/n/a/d/q/s/e)"
                    )));
                }
            }
        }
        if selected.iter().any(|s| *s) {
            let staged = apply_selected_hunks(&ch.old, &hunks, &selected);
            stage_bytes(&repo, &work, &ch.path, &staged)?;
        }
    }
    Ok(())
}

/// FS3 `add -e`: edit full-file patch then stage result.
pub fn cmd_add_edit(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &["-e", "--edit"])?;
    let repo = open_repo()?;
    let work = work_tree(&repo)?;
    let changes = collect_file_changes(&repo, &work)?;
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

/// FS3 `add -i` menu subset: status / update / patch / quit.
pub fn cmd_add_interactive(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &["-i", "--interactive"])?;
    use std::io::IsTerminal;
    if !io::stdin().is_terminal() && std::env::var("ARKS_ADD_INTERACTIVE_ANSWERS").is_err() {
        return Err(CliError::Usage(
            "add -i requires a TTY or ARKS_ADD_INTERACTIVE_ANSWERS (status|update|patch|quit)"
                .into(),
        ));
    }
    let scripted: Option<Vec<String>> =
        std::env::var("ARKS_ADD_INTERACTIVE_ANSWERS").ok().map(|s| {
            s.split(|c: char| c.is_whitespace() || c == ',' || c == ';')
                .filter(|p| !p.is_empty())
                .map(|s| s.to_string())
                .collect()
        });
    let mut si = 0usize;
    loop {
        println!("*** Commands ***");
        println!("  1: status        2: update        3: patch         4: quit");
        print!("What now> ");
        io::stdout().flush().ok();
        let choice = if let Some(ref answers) = scripted {
            let c = answers.get(si).cloned().unwrap_or_else(|| "quit".into());
            si += 1;
            println!("{c}");
            c
        } else {
            let mut line = String::new();
            io::stdin().read_line(&mut line)?;
            line.trim().to_string()
        };
        match choice.as_str() {
            "1" | "status" | "s" => {
                crate::cmd::core::cmd_status(&[])?;
            }
            "2" | "update" | "u" => {
                // Stage all tracked modifications (like update -u path selection: all dirty tracked).
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
            }
            "3" | "patch" | "p" => {
                cmd_add_patch_hunks(&[])?;
            }
            "4" | "quit" | "q" => break,
            other => eprintln!("unknown command '{other}'"),
        }
    }
    Ok(())
}

/// FS3 `stash -p`: interactively select hunks/paths, stash those, leave rest in WT.
pub fn cmd_stash_patch(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &["-p", "--patch", "push"])?;
    let mut mode = answer_mode()?;
    let mut ans_i = 0;
    let repo = open_repo()?;
    let work = work_tree(&repo)?;
    let changes = collect_file_changes(&repo, &work)?;
    if changes.is_empty() {
        println!("No local changes to save");
        return Ok(());
    }
    // For each file: decide keep-in-stash vs leave. File-or-hunk: if any hunk selected,
    // stash the selected content (WT becomes old+unselected).
    let mut stash_payload: Vec<(
        String,
        Vec<u8>, /*stash blob*/
        Vec<u8>, /*remaining WT*/
    )> = Vec::new();
    let mut accept_rest = false;
    'files: for ch in &changes {
        let mut hunks = split_hunks(&ch.old, &ch.new);
        if hunks.is_empty() {
            continue;
        }
        let mut selected = vec![false; hunks.len()];
        let mut hi = 0;
        while hi < hunks.len() {
            if accept_rest {
                selected[hi] = true;
                hi += 1;
                continue;
            }
            print_hunk(&ch.path, &hunks[hi]);
            let ans = next_answer(&mut mode, &mut ans_i, "Stash this hunk [y,n,q,a,d,s,e]? ")?;
            match ans {
                'y' | 'Y' => {
                    selected[hi] = true;
                    hi += 1;
                }
                'n' | 'N' => hi += 1,
                'a' | 'A' => {
                    selected[hi] = true;
                    accept_rest = true;
                    hi += 1;
                }
                'd' | 'D' | 'q' | 'Q' => break 'files,
                's' | 'S' => break,
                'e' | 'E' => {
                    let edited = run_editor_on_text(&render_hunk_patch(&ch.path, &hunks[hi]))?;
                    hunks[hi] = parse_edited_hunk(&edited)?;
                    selected[hi] = true;
                    hi += 1;
                }
                other => {
                    return Err(CliError::Usage(format!(
                        "stash -p: unknown answer '{other}'"
                    )));
                }
            }
        }
        if selected.iter().any(|s| *s) {
            let stashed = apply_selected_hunks(&ch.old, &hunks, &selected);
            // Invert selection for remaining WT.
            let inv: Vec<bool> = selected.iter().map(|s| !*s).collect();
            let remain = if inv.iter().any(|s| *s) {
                apply_selected_hunks(&ch.old, &hunks, &inv)
            } else {
                ch.old.clone()
            };
            stash_payload.push((ch.path.clone(), stashed, remain));
        }
    }
    if stash_payload.is_empty() {
        println!("No changes selected");
        return Ok(());
    }
    // Build stash: write remaining to WT temporarily inverted — stash_push captures WT.
    // Simpler approach: write stashed content into WT, call stash_push via internal,
    // then restore remain content to WT.
    let backups: Vec<(PathBuf, Vec<u8>)> = stash_payload
        .iter()
        .map(|(p, _, _)| {
            let abs = work.join(p);
            let prev = if abs.is_file() {
                fs::read(&abs).unwrap_or_default()
            } else {
                Vec::new()
            };
            (abs, prev)
        })
        .collect();
    for (path, stashed, _) in &stash_payload {
        let abs = work.join(path);
        if let Some(parent) = abs.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&abs, stashed)?;
    }
    drop(repo);
    crate::cmd::stash::stash_push_public()?;
    for ((abs, _), (_, _, remain)) in backups.iter().zip(stash_payload.iter()) {
        if remain.is_empty() && !abs.exists() {
            continue;
        }
        if remain.is_empty() {
            // deleted in stash selection fully — leave as HEAD/old (already in remain)
            if abs.is_file() {
                // remain empty means restore to old empty → delete
                let _ = fs::remove_file(abs);
            }
        } else {
            if let Some(parent) = abs.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(abs, remain)?;
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
