//! Interactive hunk selection shared by `add -p`, `stash push -p`,
//! `reset -p`, `checkout -p` and `restore -p`.
//!
//! Each file's diff is shown hunk by hunk with a mode-specific prompt
//! (`Stage this hunk [y,n,q,a,d,j,J,g,/,s,e,?]? `); answers come from the
//! terminal or, one per line, from standard input. `s` splits a hunk at
//! its context runs, `e` edits it, `g` and `/` jump, `j/J/k/K` move.
//! Cite: <https://git-scm.com/docs/git-add> (§INTERACTIVE MODE, patch
//! subcommand), <https://git-scm.com/docs/git-checkout>,
//! <https://git-scm.com/docs/git-reset>, <https://git-scm.com/docs/git-stash>.

use crate::cmd::shared::file_mode;
use crate::cmd::unified_diff::{HunkStyle, write_hunks};
use crate::error::CliResult;
use crate::util::{read_index, work_tree, write_index};
use arks_index::{IndexEntry, sort_entries};
use arks_object::{ObjectKind, RawObject, Tree, TreeMode};
use arks_oid::Oid;
use arks_repo::Repository;
use arks_store::ObjectStore;
use std::collections::BTreeMap;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::Path;

/// What the selected hunks do.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum PatchMode {
    Stage,
    Stash,
    ResetHead,
    ResetNothead,
    CheckoutIndex,
    CheckoutHead,
    CheckoutNothead,
    WorktreeHead,
    WorktreeNothead,
}

impl PatchMode {
    /// Prompts for (mode change, deletion, addition, hunk).
    fn prompts(self) -> [&'static str; 4] {
        match self {
            Self::Stage => [
                "Stage mode change",
                "Stage deletion",
                "Stage addition",
                "Stage this hunk",
            ],
            Self::Stash => [
                "Stash mode change",
                "Stash deletion",
                "Stash addition",
                "Stash this hunk",
            ],
            Self::ResetHead => [
                "Unstage mode change",
                "Unstage deletion",
                "Unstage addition",
                "Unstage this hunk",
            ],
            Self::ResetNothead => [
                "Apply mode change to index",
                "Apply deletion to index",
                "Apply addition to index",
                "Apply this hunk to index",
            ],
            Self::CheckoutIndex | Self::WorktreeHead => [
                "Discard mode change from worktree",
                "Discard deletion from worktree",
                "Discard addition from worktree",
                "Discard this hunk from worktree",
            ],
            Self::CheckoutHead => [
                "Discard mode change from index and worktree",
                "Discard deletion from index and worktree",
                "Discard addition from index and worktree",
                "Discard this hunk from index and worktree",
            ],
            Self::CheckoutNothead => [
                "Apply mode change to index and worktree",
                "Apply deletion to index and worktree",
                "Apply addition to index and worktree",
                "Apply this hunk to index and worktree",
            ],
            Self::WorktreeNothead => [
                "Apply mode change to worktree",
                "Apply deletion to worktree",
                "Apply addition to worktree",
                "Apply this hunk to worktree",
            ],
        }
    }

    /// The y/n/q/a/d help lines.
    fn help(self) -> String {
        let (verb, target) = match self {
            Self::Stage => ("stage", ""),
            Self::Stash => ("stash", ""),
            Self::ResetHead => ("unstage", ""),
            Self::ResetNothead => ("apply", " to index"),
            Self::CheckoutIndex | Self::WorktreeHead => ("discard", " from worktree"),
            Self::CheckoutHead => ("discard", " from index and worktree"),
            Self::CheckoutNothead => ("apply", " to index and worktree"),
            Self::WorktreeNothead => ("apply", " to worktree"),
        };
        format!(
            "y - {verb} this hunk{target}\nn - do not {verb} this hunk{target}\nq - quit; do not {verb} this hunk or any of the remaining ones\na - {verb} this hunk and all later hunks in the file\nd - do not {verb} this hunk or any of the later hunks in the file\n"
        )
    }

    /// The displayed diff runs from the target state to the current one.
    fn reversed(self) -> bool {
        matches!(
            self,
            Self::ResetNothead | Self::CheckoutNothead | Self::WorktreeNothead
        )
    }

    /// Selected hunks are applied (instead of reverted) to the old side.
    fn applies_selected(self) -> bool {
        matches!(
            self,
            Self::Stage | Self::ResetNothead | Self::CheckoutNothead | Self::WorktreeNothead
        )
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Use {
    Undecided,
    Yes,
    No,
}

#[derive(Clone, Debug)]
struct Line {
    sign: u8,
    text: Vec<u8>,
    /// The line ends without a newline (`\ No newline at end of file`).
    no_eol: bool,
}

#[derive(Clone, Debug)]
struct Hunk {
    old_start: usize,
    old_count: usize,
    new_start: usize,
    new_count: usize,
    /// Text after the closing `@@` (function context), kept on splits.
    extra: String,
    lines: Vec<Line>,
    /// A mode change pseudo-hunk (no lines; shows the mode header).
    mode: Option<(u32, u32)>,
    use_: Use,
}

impl Hunk {
    fn header(&self) -> String {
        let r = |s: usize, c: usize| {
            if c == 1 {
                format!("{s}")
            } else {
                format!("{s},{c}")
            }
        };
        format!(
            "@@ -{} +{} @@{}",
            r(self.old_start, self.old_count),
            r(self.new_start, self.new_count),
            self.extra
        )
    }

    fn render(&self, out: &mut Vec<u8>) {
        if self.mode.is_none() && self.lines.is_empty() {
            return;
        }
        if let Some((a, b)) = self.mode {
            out.extend_from_slice(format!("old mode {a:o}\nnew mode {b:o}\n").as_bytes());
            return;
        }
        out.extend_from_slice(self.header().as_bytes());
        out.push(b'\n');
        for l in &self.lines {
            out.push(l.sign);
            out.extend_from_slice(&l.text);
            out.push(b'\n');
            if l.no_eol {
                out.extend_from_slice(b"\\ No newline at end of file\n");
            }
        }
    }

    /// Change groups separated by context lines can be split apart.
    fn split_count(&self) -> usize {
        let mut groups = 0;
        let mut in_change = false;
        for l in &self.lines {
            if l.sign == b' ' {
                in_change = false;
            } else if !in_change {
                groups += 1;
                in_change = true;
            }
        }
        groups
    }

    /// Split at context runs; neighbours share the context between them.
    fn split(&self) -> Vec<Hunk> {
        let n = self.lines.len();
        // Indices where change groups start and end.
        let mut groups: Vec<(usize, usize)> = Vec::new();
        let mut i = 0;
        while i < n {
            if self.lines[i].sign == b' ' {
                i += 1;
                continue;
            }
            let s = i;
            while i < n && self.lines[i].sign != b' ' {
                i += 1;
            }
            groups.push((s, i));
        }
        let mut out = Vec::new();
        for (g, &(s, e)) in groups.iter().enumerate() {
            let from = if g == 0 { 0 } else { groups[g - 1].1 };
            let to = if g + 1 == groups.len() {
                n
            } else {
                groups[g + 1].0
            };
            let lines: Vec<Line> = self.lines[from..to].to_vec();
            // Old / new line numbers of `from`.
            let (mut o, mut nn) = (self.old_start, self.new_start);
            for l in &self.lines[..from] {
                if l.sign != b'+' {
                    o += 1;
                }
                if l.sign != b'-' {
                    nn += 1;
                }
            }
            let _ = (s, e);
            let oc = lines.iter().filter(|l| l.sign != b'+').count();
            let nc = lines.iter().filter(|l| l.sign != b'-').count();
            out.push(Hunk {
                old_start: o,
                old_count: oc,
                new_start: nn,
                new_count: nc,
                extra: self.extra.clone(),
                lines,
                mode: None,
                use_: Use::Undecided,
            });
        }
        out
    }
}

/// One side of a file: mode and content (absent = no such file).
type Side = Option<(u32, Vec<u8>)>;

struct FileDiff {
    path: String,
    old: Side,
    new: Side,
    hunks: Vec<Hunk>,
    /// Binary content: one whole-file pseudo-hunk, no hunk text.
    binary: bool,
}

fn parse_hunks(text: &[u8]) -> Vec<Hunk> {
    let mut out: Vec<Hunk> = Vec::new();
    for raw in text.split_inclusive(|c| *c == b'\n') {
        let line = raw.strip_suffix(b"\n").unwrap_or(raw);
        if line.starts_with(b"@@ ") {
            let s = String::from_utf8_lossy(line).into_owned();
            let rest = &s[3..];
            let Some(end) = rest.find(" @@") else {
                continue;
            };
            let ranges = &rest[..end];
            let extra = rest[end + 3..].to_string();
            let mut parts = ranges.split(' ');
            let parse = |p: Option<&str>| -> (usize, usize) {
                let p = p.unwrap_or("").trim_start_matches(['-', '+']);
                match p.split_once(',') {
                    Some((a, b)) => (a.parse().unwrap_or(0), b.parse().unwrap_or(0)),
                    None => (p.parse().unwrap_or(0), 1),
                }
            };
            let (os, oc) = parse(parts.next());
            let (ns, nc) = parse(parts.next());
            out.push(Hunk {
                old_start: os,
                old_count: oc,
                new_start: ns,
                new_count: nc,
                extra,
                lines: Vec::new(),
                mode: None,
                use_: Use::Undecided,
            });
            continue;
        }
        let Some(h) = out.last_mut() else { continue };
        if line.starts_with(b"\\") {
            if let Some(l) = h.lines.last_mut() {
                l.no_eol = true;
            }
            continue;
        }
        if let Some((&sign, text)) = line.split_first() {
            h.lines.push(Line {
                sign,
                text: text.to_vec(),
                no_eol: false,
            });
        }
    }
    out
}

fn split_lines(b: &[u8]) -> Vec<&[u8]> {
    if b.is_empty() {
        return Vec::new();
    }
    b.split_inclusive(|c| *c == b'\n').collect()
}

/// Apply the hunks with `take(i)` to `old`; the others keep the old lines.
/// Overlapping context of split neighbours is emitted once.
fn apply(old: &[u8], hunks: &[Hunk], take: &dyn Fn(usize) -> bool) -> Vec<u8> {
    let lines = split_lines(old);
    let mut out = Vec::new();
    let mut cursor = 0usize; // old lines consumed
    for (i, h) in hunks.iter().enumerate() {
        if h.mode.is_some() {
            continue;
        }
        let start = if h.old_count == 0 {
            h.old_start
        } else {
            h.old_start.saturating_sub(1)
        };
        while cursor < start && cursor < lines.len() {
            out.extend_from_slice(lines[cursor]);
            cursor += 1;
        }
        // Lines of this hunk already emitted by a split neighbour.
        let mut skip_old = cursor.saturating_sub(start);
        let use_it = take(i);
        for l in &h.lines {
            let in_old = l.sign != b'+';
            if in_old && skip_old > 0 {
                skip_old -= 1;
                continue;
            }
            let keep = match l.sign {
                b' ' => true,
                b'-' => !use_it,
                _ => use_it,
            };
            if keep {
                out.extend_from_slice(&l.text);
                if !l.no_eol {
                    out.push(b'\n');
                }
            }
            if in_old {
                cursor += 1;
            }
        }
    }
    while cursor < lines.len() {
        out.extend_from_slice(lines[cursor]);
        cursor += 1;
    }
    out
}

/// Apply the hunks `take(i)` (reversed when `reverse`) to `base`, which
/// must hold each hunk's preimage at its position (no fuzz).
fn apply_checked(
    base: &[u8],
    hunks: &[Hunk],
    take: &dyn Fn(usize) -> bool,
    reverse: bool,
) -> Option<Vec<u8>> {
    let lines = split_lines(base);
    let mut out = Vec::new();
    let mut cursor = 0usize;
    let (pre_sign, post_sign) = if reverse { (b'+', b'-') } else { (b'-', b'+') };
    for (i, h) in hunks.iter().enumerate() {
        if h.mode.is_some() || !take(i) {
            continue;
        }
        let (start, count) = if reverse {
            (h.new_start, h.new_count)
        } else {
            (h.old_start, h.old_count)
        };
        let start = if count == 0 {
            start
        } else {
            start.saturating_sub(1)
        };
        if start < cursor {
            return None;
        }
        while cursor < start {
            out.extend_from_slice(lines.get(cursor)?);
            cursor += 1;
        }
        for l in &h.lines {
            if l.sign == b' ' || l.sign == pre_sign {
                let have = lines.get(cursor)?;
                let body = have.strip_suffix(b"\n").unwrap_or(have);
                if body != l.text.as_slice() {
                    return None;
                }
                cursor += 1;
                if l.sign == b' ' {
                    out.extend_from_slice(have);
                }
            } else if l.sign == post_sign {
                out.extend_from_slice(&l.text);
                if !l.no_eol {
                    out.push(b'\n');
                }
            }
        }
    }
    while cursor < lines.len() {
        out.extend_from_slice(lines[cursor]);
        cursor += 1;
    }
    Some(out)
}

fn tree_entries(
    repo: &Repository,
    tree: &Oid,
    prefix: &str,
    out: &mut BTreeMap<String, (u32, Oid)>,
) -> CliResult<()> {
    let obj = repo.get_object(tree)?;
    for e in Tree::decode(&obj.payload, repo.oid_kind())?.entries {
        let p = if prefix.is_empty() {
            e.name.clone()
        } else {
            format!("{prefix}/{}", e.name)
        };
        let mode = match e.mode {
            TreeMode::Tree => {
                tree_entries(repo, &e.oid, &p, out)?;
                continue;
            }
            TreeMode::Executable => 0o100755,
            TreeMode::Symlink => 0o120000,
            TreeMode::Gitlink => 0o160000,
            TreeMode::File => 0o100644,
        };
        out.insert(p, (mode, e.oid));
    }
    Ok(())
}

fn commit_tree(repo: &Repository, rev: &str) -> CliResult<BTreeMap<String, (u32, Oid)>> {
    let oid = crate::cmd::shared::resolve_rev(repo, rev)?;
    let tree =
        crate::cmd::shared::peel_to_kind(repo, &oid, crate::cmd::shared::ObjKind::Tree, rev)?;
    let mut m = BTreeMap::new();
    tree_entries(repo, &tree, "", &mut m)?;
    Ok(m)
}

fn blob(repo: &Repository, oid: &Oid) -> Vec<u8> {
    repo.get_object(oid).map(|o| o.payload).unwrap_or_default()
}

fn read_worktree(work: &Path, path: &str) -> Side {
    let abs = work.join(path);
    let meta = fs::symlink_metadata(&abs).ok()?;
    if meta.is_dir() {
        return None;
    }
    let mode = file_mode(&meta);
    let bytes = if meta.file_type().is_symlink() {
        fs::read_link(&abs)
            .ok()?
            .to_string_lossy()
            .into_owned()
            .into_bytes()
    } else {
        fs::read(&abs).ok()?
    };
    Some((mode, bytes))
}

fn in_pathspec(specs: &[String], path: &str) -> bool {
    specs.is_empty()
        || specs.iter().any(|s| {
            let s = s.trim_end_matches('/');
            s.is_empty() || s == "." || path == s || path.starts_with(&format!("{s}/"))
        })
}

/// The two sides of every changed path for `mode`.
fn collect(
    repo: &Repository,
    mode: PatchMode,
    rev: Option<&str>,
    specs: &[String],
) -> CliResult<Vec<(String, Side, Side)>> {
    let work = work_tree(repo)?;
    let index = read_index(repo)?;
    let mut idx: BTreeMap<String, (u32, Oid, bool)> = BTreeMap::new();
    for e in &index.entries {
        if e.stage() == 0 {
            let ita = e.flags_extended.is_some_and(|f| f & 0x2000 != 0);
            idx.insert(e.path.clone(), (e.mode, e.oid, ita));
        }
    }
    let idx_side = |p: &str| -> Side {
        idx.get(p)
            .filter(|(_, _, ita)| !ita)
            .map(|(m, o, _)| (*m, blob(repo, o)))
    };
    let tree = |r: &str| -> CliResult<BTreeMap<String, (u32, Oid)>> {
        match commit_tree(repo, r) {
            Ok(t) => Ok(t),
            Err(_) if r == "HEAD" => Ok(BTreeMap::new()),
            Err(e) => Err(e),
        }
    };
    let tree_side = |t: &BTreeMap<String, (u32, Oid)>, p: &str| -> Side {
        t.get(p).map(|(m, o)| (*m, blob(repo, o)))
    };
    let mut out = Vec::new();
    let mut push = |p: &str, a: Side, b: Side| {
        if a != b && in_pathspec(specs, p) {
            out.push((p.to_string(), a, b));
        }
    };
    match mode {
        PatchMode::Stage | PatchMode::CheckoutIndex => {
            for p in idx.keys() {
                push(p, idx_side(p), read_worktree(&work, p));
            }
        }
        PatchMode::ResetHead => {
            let t = tree("HEAD")?;
            let mut paths: Vec<&String> = t.keys().chain(idx.keys()).collect();
            paths.sort();
            paths.dedup();
            for p in paths {
                push(p, tree_side(&t, p), idx_side(p));
            }
        }
        PatchMode::ResetNothead => {
            let t = tree(rev.unwrap_or("HEAD"))?;
            let mut paths: Vec<&String> = t.keys().chain(idx.keys()).collect();
            paths.sort();
            paths.dedup();
            for p in paths {
                push(p, idx_side(p), tree_side(&t, p));
            }
        }
        PatchMode::Stash => {
            // Only paths HEAD knows; new files stay out of a patch stash.
            let t = tree("HEAD")?;
            for p in t.keys() {
                push(p, tree_side(&t, p), read_worktree(&work, p));
            }
        }
        PatchMode::CheckoutHead | PatchMode::WorktreeHead => {
            let t = tree(rev.unwrap_or("HEAD"))?;
            let mut paths: Vec<&String> = t.keys().chain(idx.keys()).collect();
            paths.sort();
            paths.dedup();
            for p in paths {
                push(p, tree_side(&t, p), read_worktree(&work, p));
            }
        }
        PatchMode::CheckoutNothead | PatchMode::WorktreeNothead => {
            let t = tree(rev.unwrap_or("HEAD"))?;
            let mut paths: Vec<&String> = t.keys().chain(idx.keys()).collect();
            paths.sort();
            paths.dedup();
            for p in paths {
                push(p, read_worktree(&work, p), tree_side(&t, p));
            }
        }
    }
    Ok(out)
}

fn abbrev(repo: &Repository, s: &Side) -> CliResult<String> {
    Ok(match s {
        Some((m, b)) => {
            let kind = ObjectKind::Blob;
            let _ = m;
            RawObject::new(kind, b.clone())
                .oid(repo.object_format())?
                .to_hex()[..7]
                .to_string()
        }
        None => "0000000".into(),
    })
}

fn file_header(repo: &Repository, f: &FileDiff, reversed: bool) -> CliResult<Vec<u8>> {
    let (a, b) = if reversed { ("b", "a") } else { ("a", "b") };
    let p = &f.path;
    let mut h = format!("diff --git {a}/{p} {b}/{p}\n");
    let (ao, bo) = (abbrev(repo, &f.old)?, abbrev(repo, &f.new)?);
    match (&f.old, &f.new) {
        (None, Some((m, _))) => {
            h.push_str(&format!("new file mode {m:o}\nindex {ao}..{bo}\n"));
        }
        (Some((m, _)), None) => {
            h.push_str(&format!("deleted file mode {m:o}\nindex {ao}..{bo}\n"));
        }
        (Some((m1, _)), Some((m2, _))) if m1 != m2 => {
            h.push_str(&format!("index {ao}..{bo}\n"));
        }
        (Some((m, _)), Some(_)) => h.push_str(&format!("index {ao}..{bo} {m:o}\n")),
        (None, None) => {}
    }
    let old_name = if f.old.is_some() {
        format!("{a}/{p}")
    } else {
        "/dev/null".into()
    };
    let new_name = if f.new.is_some() {
        format!("{b}/{p}")
    } else {
        "/dev/null".into()
    };
    if f.binary {
        h.push_str(&format!("Binary files {old_name} and {new_name} differ\n"));
    } else {
        h.push_str(&format!("--- {old_name}\n+++ {new_name}\n"));
    }
    Ok(h.into_bytes())
}

fn build_file(path: String, old: Side, new: Side) -> CliResult<FileDiff> {
    let mut hunks = Vec::new();
    if let (Some((m1, _)), Some((m2, _))) = (&old, &new) {
        if m1 != m2 {
            hunks.push(Hunk {
                old_start: 0,
                old_count: 0,
                new_start: 0,
                new_count: 0,
                extra: String::new(),
                lines: Vec::new(),
                mode: Some((*m1, *m2)),
                use_: Use::Undecided,
            });
        }
    }
    let a = old.as_ref().map(|s| s.1.as_slice()).unwrap_or(b"");
    let b = new.as_ref().map(|s| s.1.as_slice()).unwrap_or(b"");
    let is_bin = |x: &[u8]| x[..x.len().min(8000)].contains(&0);
    let binary = is_bin(a) || is_bin(b);
    if a != b || (old.is_none() != new.is_none()) {
        if binary {
            hunks.push(Hunk {
                old_start: 0,
                old_count: 0,
                new_start: 0,
                new_count: 0,
                extra: String::new(),
                lines: Vec::new(),
                mode: None,
                use_: Use::Undecided,
            });
        } else {
            let mut text = Vec::new();
            write_hunks(&mut text, a, b, 3, HunkStyle::default())?;
            hunks.extend(parse_hunks(&text));
        }
    }
    Ok(FileDiff {
        path,
        old,
        new,
        hunks,
        binary,
    })
}

/// Answers from the terminal or from standard input, one per line.
struct Input {
    stdin: io::Stdin,
}

impl Input {
    fn read(&mut self) -> Option<String> {
        let mut line = String::new();
        match self.stdin.lock().read_line(&mut line) {
            Ok(0) | Err(_) => None,
            Ok(_) => Some(line.trim_end_matches(['\n', '\r']).to_string()),
        }
    }
}

enum Outcome {
    Next,
    Quit,
}

fn edit_hunk(repo: &Repository, h: &Hunk, mode: PatchMode) -> CliResult<Option<Hunk>> {
    let path = repo.git_dir().join("addp-hunk-edit.diff");
    let mut text = b"# Manual hunk edit mode -- see bottom for a quick guide.\n".to_vec();
    h.render(&mut text);
    let (minus, plus) = if mode.applies_selected() || mode == PatchMode::Stash {
        ("'-'", "'+'")
    } else {
        ("'+'", "'-'")
    };
    let verb = match mode {
        PatchMode::Stage => "staging",
        PatchMode::Stash => "stashing",
        PatchMode::ResetHead => "unstaging",
        PatchMode::ResetNothead | PatchMode::CheckoutNothead | PatchMode::WorktreeNothead => {
            "applying"
        }
        _ => "discarding",
    };
    text.extend_from_slice(
        format!(
            "# ---\n# To remove {minus} lines, make them ' ' lines (context).\n# To remove {plus} lines, delete them.\n# Lines starting with # will be removed.\n# If the patch applies cleanly, the edited hunk will immediately be marked for {verb}.\n# If it does not apply cleanly, you will be given an opportunity to\n# edit again.  If all lines of the hunk are removed, then the edit is\n# aborted and the hunk is left unchanged.\n"
        )
        .as_bytes(),
    );
    fs::write(&path, &text)?;
    let cfg = arks_config::ConfigSet::load_discovered(repo.git_dir());
    let res = crate::cmd::bridge::proc::edit(&crate::cmd::bridge::proc::editor(&cfg), &path);
    let edited = fs::read(&path).unwrap_or_default();
    let _ = fs::remove_file(&path);
    res?;
    let kept: Vec<u8> = edited
        .split_inclusive(|c| *c == b'\n')
        .filter(|l| !l.starts_with(b"#"))
        .flatten()
        .copied()
        .collect();
    if kept.iter().all(|c| c.is_ascii_whitespace()) {
        return Ok(None);
    }
    let mut parsed = parse_hunks(&kept);
    let Some(mut nh) = parsed.pop() else {
        return Ok(None);
    };
    nh.old_start = h.old_start;
    nh.new_start = h.new_start;
    nh.old_count = nh.lines.iter().filter(|l| l.sign != b'+').count();
    nh.new_count = nh.lines.iter().filter(|l| l.sign != b'-').count();
    nh.extra = h.extra.clone();
    Ok(Some(nh))
}

/// Ask about each hunk of `f`; returns whether the user quit.
fn ask_file(
    repo: &Repository,
    mode: PatchMode,
    f: &mut FileDiff,
    input: &mut Input,
    out: &mut Vec<u8>,
) -> CliResult<Outcome> {
    let flush = |out: &mut Vec<u8>| -> CliResult<()> {
        let mut so = io::stdout().lock();
        so.write_all(out)?;
        so.flush()?;
        out.clear();
        Ok(())
    };
    out.extend_from_slice(&file_header(repo, f, mode.reversed())?);
    let [p_mode, p_del, p_add, p_hunk] = mode.prompts();
    let mut i = 0usize;
    loop {
        let n = f.hunks.len();
        if n == 0 {
            return Ok(Outcome::Next);
        }
        if i >= n {
            // Wrap to the first undecided hunk, or finish.
            match f.hunks.iter().position(|h| h.use_ == Use::Undecided) {
                Some(j) => i = j,
                None => return Ok(Outcome::Next),
            }
        }
        let h = f.hunks[i].clone();
        h.render(out);
        let prev_undecided = f.hunks[..i].iter().any(|h| h.use_ == Use::Undecided);
        let next_undecided = f.hunks[i + 1..].iter().any(|h| h.use_ == Use::Undecided);
        let mut opts = String::from("y,n,q,a,d");
        if prev_undecided {
            opts.push_str(",k");
        }
        if i > 0 {
            opts.push_str(",K");
        }
        if next_undecided {
            opts.push_str(",j");
        }
        if i + 1 < n {
            opts.push_str(",J");
        }
        if n > 1 {
            opts.push_str(",g,/");
        }
        let splittable = h.mode.is_none() && h.split_count() > 1;
        if splittable {
            opts.push_str(",s");
        }
        let editable = h.mode.is_none() && f.new.is_some() && !f.binary;
        if editable {
            opts.push_str(",e");
        }
        opts.push_str(",?");
        let what = if h.mode.is_some() {
            p_mode
        } else if f.new.is_none() {
            p_del
        } else if f.old.is_none() {
            p_add
        } else {
            p_hunk
        };
        out.extend_from_slice(format!("({}/{n}) {what} [{opts}]? ", i + 1).as_bytes());
        flush(out)?;
        // End of input leaves this file's remaining hunks undecided.
        let Some(ans) = input.read() else {
            return Ok(Outcome::Next);
        };
        let c = ans.chars().next();
        let lower = c.map(|c| {
            if "ynqad".contains(c.to_ascii_lowercase()) {
                c.to_ascii_lowercase()
            } else {
                c
            }
        });
        match lower {
            None => {}
            Some('y') => {
                f.hunks[i].use_ = Use::Yes;
                i = next_index(f, i);
            }
            Some('n') => {
                f.hunks[i].use_ = Use::No;
                i = next_index(f, i);
            }
            Some('a') | Some('d') => {
                let u = if lower == Some('a') {
                    Use::Yes
                } else {
                    Use::No
                };
                for h in &mut f.hunks[i..] {
                    if h.use_ == Use::Undecided {
                        h.use_ = u;
                    }
                }
                f.hunks[i].use_ = u;
                i = n;
            }
            Some('q') => {
                for h in &mut f.hunks {
                    if h.use_ == Use::Undecided {
                        h.use_ = Use::No;
                    }
                }
                return Ok(Outcome::Quit);
            }
            Some('j') => match (i + 1..n).find(|j| f.hunks[*j].use_ == Use::Undecided) {
                Some(j) => i = j,
                None => eprintln!("No next hunk"),
            },
            Some('J') => {
                if i + 1 < n {
                    i += 1;
                } else {
                    eprintln!("No next hunk");
                }
            }
            Some('k') => match (0..i).rev().find(|j| f.hunks[*j].use_ == Use::Undecided) {
                Some(j) => i = j,
                None => eprintln!("No previous hunk"),
            },
            Some('K') => {
                if i > 0 {
                    i -= 1;
                } else {
                    eprintln!("No previous hunk");
                }
            }
            Some('g') if n > 1 => {
                for (j, hk) in f.hunks.iter().enumerate().take(21) {
                    let mark = match hk.use_ {
                        Use::Yes => '+',
                        Use::No => '-',
                        Use::Undecided => ' ',
                    };
                    let summary = if hk.mode.is_some() {
                        String::new()
                    } else {
                        let hd = hk.header();
                        hd.trim_start_matches("@@ ")
                            .split(" @@")
                            .next()
                            .unwrap_or("")
                            .to_string()
                    };
                    let first = hk
                        .lines
                        .iter()
                        .find(|l| l.sign != b' ')
                        .map(|l| {
                            let mut s = vec![l.sign];
                            s.extend_from_slice(&l.text);
                            String::from_utf8_lossy(&s).into_owned()
                        })
                        .unwrap_or_default();
                    out.extend_from_slice(
                        format!("{mark}{:2}:  {summary:<19}{first}\n", j + 1).as_bytes(),
                    );
                }
                out.extend_from_slice(b"go to which hunk? ");
                flush(out)?;
                let Some(num) = input.read() else {
                    return Ok(Outcome::Quit);
                };
                match num.trim().parse::<usize>() {
                    Ok(k) if k >= 1 && k <= n => i = k - 1,
                    Ok(_) => eprintln!("Sorry, only {n} hunks available."),
                    Err(_) => eprintln!("Invalid number: '{}'", num.trim()),
                }
            }
            Some('g') => eprintln!("No other hunks to goto"),
            Some('/') if n > 1 => {
                let pat = if ans.len() > 1 {
                    ans[1..].to_string()
                } else {
                    out.extend_from_slice(b"search for regex? ");
                    flush(out)?;
                    match input.read() {
                        Some(p) => p,
                        None => return Ok(Outcome::Quit),
                    }
                };
                let hit = (1..=n).map(|d| (i + d) % n).find(|j| {
                    let mut t = Vec::new();
                    f.hunks[*j].render(&mut t);
                    String::from_utf8_lossy(&t).contains(&pat)
                });
                match hit {
                    Some(j) => i = j,
                    None => eprintln!("No hunk matches the given pattern"),
                }
            }
            Some('/') => eprintln!("No other hunks to search"),
            Some('s') if splittable => {
                let parts = h.split();
                out.extend_from_slice(format!("Split into {} hunks.\n", parts.len()).as_bytes());
                f.hunks.splice(i..=i, parts);
            }
            Some('s') => eprintln!("Sorry, cannot split this hunk"),
            Some('e') if editable => {
                flush(out)?;
                if let Some(nh) = edit_hunk(repo, &h, mode)? {
                    let mut nh = nh;
                    nh.use_ = Use::Yes;
                    f.hunks[i] = nh;
                    i = next_index(f, i);
                }
            }
            Some('e') => eprintln!("Sorry, cannot edit this hunk"),
            Some(_) => {
                // `?` and anything unknown print the help.
                let mut help = mode.help();
                for (k, line) in [
                    (
                        'k',
                        "k - leave this hunk undecided, see previous undecided hunk\n",
                    ),
                    ('K', "K - leave this hunk undecided, see previous hunk\n"),
                    (
                        'j',
                        "j - leave this hunk undecided, see next undecided hunk\n",
                    ),
                    ('J', "J - leave this hunk undecided, see next hunk\n"),
                    ('g', "g - select a hunk to go to\n"),
                    ('/', "/ - search for a hunk matching the given regex\n"),
                    ('s', "s - split the current hunk into smaller hunks\n"),
                    ('e', "e - manually edit the current hunk\n"),
                ] {
                    if opts.split(',').any(|o| o.starts_with(k)) {
                        help.push_str(line);
                    }
                }
                help.push_str("? - print help\n");
                out.extend_from_slice(help.as_bytes());
            }
        }
    }
}

fn next_index(f: &FileDiff, i: usize) -> usize {
    (i + 1..f.hunks.len())
        .find(|j| f.hunks[*j].use_ == Use::Undecided)
        .unwrap_or(f.hunks.len())
}

fn put_blob(repo: &mut Repository, bytes: &[u8]) -> CliResult<Oid> {
    let format = repo.object_format();
    Ok(repo
        .store_mut()
        .put(format, &RawObject::new(ObjectKind::Blob, bytes.to_vec()))?)
}

/// Set (or remove) `path` in the index.
fn set_index(repo: &mut Repository, work: &Path, path: &str, side: &Side) -> CliResult<()> {
    let mut index = read_index(repo)?;
    let old = index
        .entries
        .iter()
        .find(|e| e.path == path && e.stage() == 0)
        .cloned();
    index
        .entries
        .retain(|e| !(e.path == path && e.stage() == 0));
    if let Some((mode, bytes)) = side {
        let oid = put_blob(repo, bytes)?;
        let meta = fs::symlink_metadata(work.join(path)).ok();
        let times = meta
            .as_ref()
            .map(crate::cmd::shared::file_times)
            .unwrap_or((0, 0, 0, 0));
        let mut e = old.unwrap_or(IndexEntry {
            ctime_sec: 0,
            ctime_nsec: 0,
            mtime_sec: 0,
            mtime_nsec: 0,
            dev: 0,
            ino: 0,
            mode: *mode,
            uid: 0,
            gid: 0,
            size: 0,
            oid,
            flags: (path.len().min(0x0fff)) as u16,
            flags_extended: None,
            path: path.to_string(),
        });
        e.ctime_sec = times.0;
        e.ctime_nsec = times.1;
        e.mtime_sec = times.2;
        e.mtime_nsec = times.3;
        e.oid = oid;
        e.mode = *mode;
        e.size = bytes.len().min(u32::MAX as usize) as u32;
        if let Some(f) = e.flags_extended.as_mut() {
            *f &= !0x2000;
        }
        index.entries.push(e);
    }
    sort_entries(&mut index.entries);
    write_index(repo, &index)?;
    Ok(())
}

fn set_worktree(work: &Path, path: &str, side: &Side) -> CliResult<()> {
    let abs = work.join(path);
    match side {
        None => {
            let _ = fs::remove_file(&abs);
        }
        Some((mode, bytes)) => {
            if let Some(parent) = abs.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&abs, bytes)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let bits = if *mode == 0o100755 { 0o755 } else { 0o644 };
                let _ = fs::set_permissions(&abs, fs::Permissions::from_mode(bits));
            }
            #[cfg(not(unix))]
            let _ = mode;
        }
    }
    Ok(())
}

/// The side that results from taking the hunks `take(i)` on `old`.
fn result_side(f: &FileDiff, take: &dyn Fn(usize) -> bool) -> Side {
    let mode_taken = f
        .hunks
        .iter()
        .enumerate()
        .any(|(i, h)| h.mode.is_some() && take(i));
    let content_hunks: Vec<usize> = (0..f.hunks.len())
        .filter(|i| f.hunks[*i].mode.is_none())
        .collect();
    let all_taken = !content_hunks.is_empty() && content_hunks.iter().all(|i| take(*i));
    let none_taken = content_hunks.iter().all(|i| !take(*i));
    match (&f.old, &f.new) {
        (_, None) if all_taken => None,
        (None, Some(_)) if none_taken => None,
        _ => {
            let old_bytes = f.old.as_ref().map(|s| s.1.clone()).unwrap_or_default();
            let bytes = if f.binary {
                if all_taken {
                    f.new.as_ref().map(|s| s.1.clone()).unwrap_or_default()
                } else {
                    old_bytes
                }
            } else {
                apply(&old_bytes, &f.hunks, take)
            };
            let mode = match (&f.old, &f.new) {
                (Some((m1, _)), Some((m2, _))) => {
                    if mode_taken {
                        *m2
                    } else {
                        *m1
                    }
                }
                (Some((m, _)), None) | (None, Some((m, _))) => *m,
                (None, None) => 0o100644,
            };
            Some((mode, bytes))
        }
    }
}

/// Run interactive selection for `mode`; returns the per-file results that
/// were applied (path, result side) — used by `stash -p`.
pub(crate) fn run(
    mode: PatchMode,
    rev: Option<&str>,
    specs: &[String],
) -> CliResult<Vec<(String, Side, Side)>> {
    Ok(run_files(mode, rev, specs)?.unwrap_or_default())
}

/// [`run`], with `None` when there was nothing to ask about.
pub(crate) fn run_files(
    mode: PatchMode,
    rev: Option<&str>,
    specs: &[String],
) -> CliResult<Option<Vec<(String, Side, Side)>>> {
    let mut repo = crate::util::open_repo()?;
    let work = work_tree(&repo)?;
    let changed = collect(&repo, mode, rev, specs)?;
    let mut files = Vec::new();
    for (p, a, b) in changed {
        let f = build_file(p, a, b)?;
        if !f.hunks.is_empty() {
            files.push(f);
        }
    }
    if files.is_empty() {
        if mode != PatchMode::Stash {
            eprintln!("No changes.");
        }
        return Ok(None);
    }
    let mut input = Input { stdin: io::stdin() };
    let mut out = Vec::new();
    let mut applied = Vec::new();
    let mut quit = false;
    for f in &mut files {
        if quit {
            break;
        }
        let outcome = ask_file(&repo, mode, f, &mut input, &mut out)?;
        if matches!(outcome, Outcome::Quit) {
            quit = true;
        }
        if f.hunks.iter().any(|h| h.use_ == Use::Yes) {
            let yes = |i: usize| f.hunks[i].use_ == Use::Yes;
            let not_yes = |i: usize| f.hunks[i].use_ != Use::Yes;
            let take: &dyn Fn(usize) -> bool = if mode.applies_selected() {
                &yes
            } else {
                &not_yes
            };
            let result = result_side(f, take);
            match mode {
                PatchMode::Stage | PatchMode::ResetHead | PatchMode::ResetNothead => {
                    set_index(&mut repo, &work, &f.path, &result)?;
                }
                PatchMode::CheckoutIndex | PatchMode::WorktreeHead | PatchMode::WorktreeNothead => {
                    set_worktree(&work, &f.path, &result)?;
                }
                PatchMode::CheckoutHead | PatchMode::CheckoutNothead => {
                    // The same hunks must also apply to the index entry.
                    let index_side = read_index(&repo)?
                        .entries
                        .iter()
                        .find(|e| e.path == f.path && e.stage() == 0)
                        .map(|e| (e.mode, blob(&repo, &e.oid)));
                    let reverse = mode == PatchMode::CheckoutHead;
                    let index_result = match (&index_side, &result) {
                        (_, None) => Some(None),
                        (Some((m, b)), Some((rm, _))) => apply_checked(b, &f.hunks, &yes, reverse)
                            .map(|bytes| Some((if *m == *rm { *m } else { *rm }, bytes))),
                        (None, Some(_)) if f.old.is_none() != reverse => Some(result.clone()),
                        (None, Some(_)) => None,
                    };
                    match index_result {
                        Some(ir) => {
                            set_worktree(&work, &f.path, &result)?;
                            set_index(&mut repo, &work, &f.path, &ir)?;
                        }
                        None => {
                            eprintln!(
                                "error: patch failed: {}:{}\nerror: {}: patch does not apply\nThe selected hunks do not apply to the index!",
                                f.path,
                                f.hunks
                                    .iter()
                                    .find(|h| h.mode.is_none())
                                    .map_or(1, |h| h.old_start),
                                f.path
                            );
                            out.extend_from_slice(b"Apply them to the worktree anyway? ");
                            {
                                let mut so = io::stdout().lock();
                                so.write_all(&out)?;
                                so.flush()?;
                            }
                            out.clear();
                            let yes_answer =
                                input.read().is_some_and(|a| a.starts_with(['y', 'Y']));
                            if yes_answer {
                                set_worktree(&work, &f.path, &result)?;
                            } else {
                                eprintln!("Nothing was applied.\n");
                            }
                        }
                    }
                }
                PatchMode::Stash => {
                    let stashed = result_side(f, &yes);
                    applied.push((f.path.clone(), stashed, result));
                }
            }
        }
        out.push(b'\n');
        let mut so = io::stdout().lock();
        so.write_all(&out)?;
        so.flush()?;
        out.clear();
    }
    Ok(Some(applied))
}

/// Split `-p` arguments into (revision, pathspecs): the first argument
/// before `--` is a revision when it names a commit.
pub(crate) fn rev_and_paths(repo: &Repository, args: &[String]) -> (Option<String>, Vec<String>) {
    let (before, after): (Vec<String>, Vec<String>) = match args.iter().position(|a| a == "--") {
        Some(i) => (args[..i].to_vec(), args[i + 1..].to_vec()),
        None => (args.to_vec(), Vec::new()),
    };
    let mut before = before;
    let mut rev = None;
    if let Some(first) = before.first() {
        let is_commit = crate::cmd::shared::resolve_rev(repo, first)
            .ok()
            .and_then(|o| {
                crate::cmd::shared::peel_to_kind(repo, &o, crate::cmd::shared::ObjKind::Tree, first)
                    .ok()
            })
            .is_some();
        if is_commit {
            rev = Some(before.remove(0));
        }
    }
    before.extend(after);
    (rev, before)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_and_partial_apply() {
        let old = b"fn one\na\nb\nc\nd\ne\nf\ng\n";
        let new = b"fn one\nA\nb\nc\nd\nE\nf\ng\n";
        let f = build_file(
            "s".into(),
            Some((0o100644, old.to_vec())),
            Some((0o100644, new.to_vec())),
        )
        .unwrap();
        assert_eq!(f.hunks.len(), 1);
        let parts = f.hunks[0].split();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].header(), "@@ -1,5 +1,5 @@");
        assert_eq!(parts[1].header(), "@@ -3,6 +3,6 @@");
        let got = apply(old, &parts, &|i| i == 0);
        assert_eq!(got, b"fn one\nA\nb\nc\nd\ne\nf\ng\n");
        let got = apply(old, &parts, &|i| i == 1);
        assert_eq!(got, b"fn one\na\nb\nc\nd\nE\nf\ng\n");
        let got = apply(old, &parts, &|_| true);
        assert_eq!(got, new);
    }
}
