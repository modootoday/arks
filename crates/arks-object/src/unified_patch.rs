//! Unified diff ("git diff" / format-patch body) parse and hunk apply.
//!
//! Claimed subset: `diff --git` and plain `---`/`+++` file headers,
//! `new file mode` / `deleted file mode` / `new mode`, rename and copy
//! headers, any number of `@@ -a,b +c,d @@` hunks per file, and
//! `\ No newline at end of file`. Hunks apply at their recorded line or at
//! the nearest offset where the preimage matches exactly (no fuzz). As in
//! git's apply.c, a hunk starting at old line 0/1 must match at the start
//! of the file and a hunk without trailing context must match at its end.
//! Binary patches parse as `binary` and are not applied.
//!
//! Primary: <https://git-scm.com/docs/git-apply>,
//! <https://git-scm.com/docs/diff-format>,
//! <https://www.gnu.org/software/diffutils/manual/html_node/Detailed-Unified.html>.
//! Ownership: Ring 1 presentation format — not CLI I/O.

use crate::ObjectError;

/// One line of a hunk; content keeps its `\n` unless the patch marks
/// `\ No newline at end of file`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HunkLine {
    Context(Vec<u8>),
    Del(Vec<u8>),
    Add(Vec<u8>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hunk {
    pub old_start: usize,
    pub old_count: usize,
    pub new_start: usize,
    pub new_count: usize,
    pub lines: Vec<HunkLine>,
}

/// Changes to one file. `old_path` is `None` for a creation, `new_path`
/// `None` for a deletion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilePatch {
    pub old_path: Option<String>,
    pub new_path: Option<String>,
    pub new_mode: Option<u32>,
    pub hunks: Vec<Hunk>,
    pub binary: bool,
    /// Abbreviated pre-/post-image blob ids from the `index <a>..<b>` line.
    pub index_oids: Option<(String, String)>,
}

impl FilePatch {
    /// Path shown in `--stat` / errors: the new path, else the old one.
    pub fn display_path(&self) -> &str {
        self.new_path
            .as_deref()
            .or(self.old_path.as_deref())
            .unwrap_or("")
    }

    /// `(insertions, deletions)` counted from the hunks.
    pub fn line_counts(&self) -> (usize, usize) {
        let mut ins = 0;
        let mut del = 0;
        for h in &self.hunks {
            for l in &h.lines {
                match l {
                    HunkLine::Add(_) => ins += 1,
                    HunkLine::Del(_) => del += 1,
                    HunkLine::Context(_) => {}
                }
            }
        }
        (ins, del)
    }

    /// The same change undone (`git apply -R`).
    pub fn reversed(&self) -> FilePatch {
        FilePatch {
            old_path: self.new_path.clone(),
            new_path: self.old_path.clone(),
            new_mode: self.new_mode,
            binary: self.binary,
            index_oids: self
                .index_oids
                .as_ref()
                .map(|(a, b)| (b.clone(), a.clone())),
            hunks: self
                .hunks
                .iter()
                .map(|h| Hunk {
                    old_start: h.new_start,
                    old_count: h.new_count,
                    new_start: h.old_start,
                    new_count: h.old_count,
                    lines: h
                        .lines
                        .iter()
                        .map(|l| match l {
                            HunkLine::Add(b) => HunkLine::Del(b.clone()),
                            HunkLine::Del(b) => HunkLine::Add(b.clone()),
                            HunkLine::Context(b) => HunkLine::Context(b.clone()),
                        })
                        .collect(),
                })
                .collect(),
        }
    }
}

fn bad(msg: impl Into<String>) -> ObjectError {
    ObjectError::BadPatch(msg.into())
}

/// Strip the `a/` / `b/` prefix (`-p1`) and any tab-separated timestamp.
fn header_path(raw: &[u8]) -> Option<String> {
    let s = String::from_utf8_lossy(raw);
    let s = s.split('\t').next().unwrap_or("").trim_end();
    if s == "/dev/null" {
        return None;
    }
    let s = s
        .strip_prefix('"')
        .and_then(|x| x.strip_suffix('"'))
        .unwrap_or(s);
    Some(
        s.split_once('/')
            .map(|(_, rest)| rest.to_string())
            .unwrap_or_else(|| s.to_string()),
    )
}

fn parse_range(s: &str) -> Option<(usize, usize)> {
    match s.split_once(',') {
        Some((a, b)) => Some((a.parse().ok()?, b.parse().ok()?)),
        None => Some((s.parse().ok()?, 1)),
    }
}

/// `@@ -a[,b] +c[,d] @@ ...`
fn parse_hunk_header(line: &[u8]) -> Option<(usize, usize, usize, usize)> {
    let s = std::str::from_utf8(line).ok()?;
    let rest = s.strip_prefix("@@ -")?;
    let (ranges, _) = rest.split_once(" @@")?;
    let (old, new) = ranges.split_once(" +")?;
    let (a, b) = parse_range(old)?;
    let (c, d) = parse_range(new)?;
    Some((a, b, c, d))
}

/// Parse every file patch in `text` (mail headers and commit messages
/// around the diff are skipped).
pub fn parse_unified_diff(text: &[u8]) -> Result<Vec<FilePatch>, ObjectError> {
    let lines: Vec<&[u8]> = text.split_inclusive(|b| *b == b'\n').collect();
    let strip = |l: &[u8]| -> Vec<u8> { l.strip_suffix(b"\n").unwrap_or(l).to_vec() };
    let mut files = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let line = strip(lines[i]);
        let git_header = line.starts_with(b"diff --git ");
        let plain_header =
            line.starts_with(b"--- ") && lines.get(i + 1).is_some_and(|n| n.starts_with(b"+++ "));
        if !git_header && !plain_header {
            i += 1;
            continue;
        }
        let mut fp = FilePatch {
            old_path: None,
            new_path: None,
            new_mode: None,
            hunks: Vec::new(),
            binary: false,
            index_oids: None,
        };
        let (mut creating, mut deleting) = (false, false);
        if git_header {
            let rest = String::from_utf8_lossy(&line["diff --git ".len()..]).to_string();
            if let Some((a, b)) = rest.split_once(" b/") {
                let a = a.strip_prefix("a/").unwrap_or(a);
                fp.old_path = Some(a.to_string());
                fp.new_path = Some(b.to_string());
            }
            i += 1;
            // Extended header lines up to the first hunk / next file.
            while i < lines.len() {
                let l = strip(lines[i]);
                let ls = String::from_utf8_lossy(&l).to_string();
                if l.starts_with(b"diff --git ") || l.starts_with(b"@@ ") {
                    break;
                }
                if let Some(m) = ls.strip_prefix("new file mode ") {
                    creating = true;
                    fp.new_mode = u32::from_str_radix(m.trim(), 8).ok();
                } else if ls.starts_with("deleted file mode ") {
                    deleting = true;
                } else if let Some(m) = ls.strip_prefix("new mode ") {
                    fp.new_mode = u32::from_str_radix(m.trim(), 8).ok();
                } else if let Some(p) = ls
                    .strip_prefix("rename from ")
                    .or_else(|| ls.strip_prefix("copy from "))
                {
                    fp.old_path = Some(p.to_string());
                } else if let Some(p) = ls
                    .strip_prefix("rename to ")
                    .or_else(|| ls.strip_prefix("copy to "))
                {
                    fp.new_path = Some(p.to_string());
                } else if ls.starts_with("Binary files ") || ls == "GIT binary patch" {
                    fp.binary = true;
                } else if let Some(p) = ls.strip_prefix("--- ") {
                    match header_path(p.as_bytes()) {
                        None => creating = true,
                        Some(x) => fp.old_path = Some(x),
                    }
                } else if let Some(p) = ls.strip_prefix("+++ ") {
                    match header_path(p.as_bytes()) {
                        None => deleting = true,
                        Some(x) => fp.new_path = Some(x),
                    }
                } else if let Some(rest) = ls.strip_prefix("index ") {
                    let ids = rest.split_whitespace().next().unwrap_or("");
                    if let Some((a, b)) = ids.split_once("..") {
                        fp.index_oids = Some((a.to_string(), b.to_string()));
                    }
                } else if !ls.starts_with("old mode ")
                    && !ls.starts_with("similarity index ")
                    && !ls.starts_with("dissimilarity index ")
                {
                    break;
                }
                i += 1;
            }
        } else {
            fp.old_path = header_path(&line[4..]);
            creating = fp.old_path.is_none();
            let next = strip(lines[i + 1]);
            fp.new_path = header_path(&next[4..]);
            deleting = fp.new_path.is_none();
            i += 2;
        }
        // Hunks.
        while i < lines.len() && lines[i].starts_with(b"@@ ") {
            let (a, b, c, d) = parse_hunk_header(&strip(lines[i]))
                .ok_or_else(|| bad(format!("corrupt hunk header at line {}", i + 1)))?;
            i += 1;
            let mut h = Hunk {
                old_start: a,
                old_count: b,
                new_start: c,
                new_count: d,
                lines: Vec::new(),
            };
            let (mut old_left, mut new_left) = (b, d);
            while i < lines.len() && (old_left > 0 || new_left > 0) {
                let raw = lines[i];
                let body = raw.get(1..).unwrap_or(b"").to_vec();
                // An empty line inside a hunk is an empty context line.
                let (tag, body) = if raw == b"\n" {
                    (b' ', b"\n".to_vec())
                } else {
                    (raw[0], body)
                };
                match tag {
                    b' ' => {
                        h.lines.push(HunkLine::Context(body));
                        old_left = old_left.saturating_sub(1);
                        new_left = new_left.saturating_sub(1);
                    }
                    b'-' => {
                        h.lines.push(HunkLine::Del(body));
                        old_left = old_left.saturating_sub(1);
                    }
                    b'+' => {
                        h.lines.push(HunkLine::Add(body));
                        new_left = new_left.saturating_sub(1);
                    }
                    b'\\' => {}
                    _ => return Err(bad(format!("corrupt patch at line {}", i + 1))),
                }
                i += 1;
                // `\ No newline at end of file` applies to the line just read.
                if i < lines.len() && lines[i].starts_with(b"\\ ") {
                    if let Some(last) = h.lines.last_mut() {
                        let (HunkLine::Context(v) | HunkLine::Del(v) | HunkLine::Add(v)) = last;
                        if v.last() == Some(&b'\n') {
                            v.pop();
                        }
                    }
                    i += 1;
                }
            }
            fp.hunks.push(h);
        }
        if creating {
            fp.old_path = None;
        }
        if deleting {
            fp.new_path = None;
        }
        if fp.old_path.is_none() && fp.new_path.is_none() {
            return Err(bad("patch with no file name"));
        }
        files.push(fp);
    }
    if files.is_empty() {
        return Err(bad("No valid patches in input"));
    }
    Ok(files)
}

/// Apply `fp`'s hunks to `old` (the preimage file content). On a mismatch
/// returns the 1-based `old_start` of the first hunk that does not apply.
pub fn apply_hunks(old: &[u8], fp: &FilePatch) -> Result<Vec<u8>, usize> {
    let src: Vec<&[u8]> = old.split_inclusive(|b| *b == b'\n').collect();
    let mut out: Vec<u8> = Vec::with_capacity(old.len());
    let mut pos = 0usize; // next unconsumed source line
    for h in &fp.hunks {
        let pre: Vec<&[u8]> = h
            .lines
            .iter()
            .filter_map(|l| match l {
                HunkLine::Context(b) | HunkLine::Del(b) => Some(b.as_slice()),
                HunkLine::Add(_) => None,
            })
            .collect();
        let want = if h.old_count == 0 {
            h.old_start
        } else {
            h.old_start.saturating_sub(1)
        };
        let trailing = h
            .lines
            .iter()
            .rev()
            .take_while(|l| matches!(l, HunkLine::Context(_)))
            .count();
        let match_beginning = h.old_start <= 1;
        let match_end = trailing == 0;
        let fits = |at: usize| {
            at + pre.len() <= src.len()
                && src[at..at + pre.len()] == pre[..]
                && (!match_beginning || at == 0)
                && (!match_end || at + pre.len() == src.len())
        };
        let max = src.len().saturating_sub(pre.len());
        let mut found = None;
        for off in 0..=src.len() {
            let cands = [want.checked_add(off), want.checked_sub(off)];
            for at in cands.into_iter().flatten() {
                if at >= pos && at <= max && fits(at) {
                    found = Some(at);
                    break;
                }
            }
            if found.is_some() {
                break;
            }
        }
        let at = found.ok_or(h.old_start)?;
        for l in &src[pos..at] {
            out.extend_from_slice(l);
        }
        for l in &h.lines {
            match l {
                HunkLine::Context(b) | HunkLine::Add(b) => out.extend_from_slice(b),
                HunkLine::Del(_) => {}
            }
        }
        pos = at + pre.len();
    }
    for l in &src[pos..] {
        out.extend_from_slice(l);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PATCH: &str = "diff --git a/f.txt b/f.txt\nindex 1..2 100644\n--- a/f.txt\n+++ b/f.txt\n@@ -1,3 +1,3 @@\n a\n-b\n+B\n c\n@@ -5,2 +5,3 @@\n e\n f\n+g\n";

    #[test]
    fn parse_and_apply_multi_hunk() {
        let fps = parse_unified_diff(PATCH.as_bytes()).unwrap();
        assert_eq!(fps.len(), 1);
        assert_eq!(fps[0].hunks.len(), 2);
        assert_eq!(fps[0].line_counts(), (2, 1));
        let out = apply_hunks(b"a\nb\nc\nd\ne\nf\n", &fps[0]).unwrap();
        assert_eq!(out, b"a\nB\nc\nd\ne\nf\ng\n");
        let back = apply_hunks(&out, &fps[0].reversed()).unwrap();
        assert_eq!(back, b"a\nb\nc\nd\ne\nf\n");
    }

    #[test]
    fn apply_with_offset_and_mismatch() {
        let fps = parse_unified_diff(PATCH.as_bytes()).unwrap();
        // The first hunk starts at line 1, so it must match at the beginning.
        assert_eq!(apply_hunks(b"x\na\nb\nc\nd\ne\nf\n", &fps[0]), Err(1));
        assert_eq!(apply_hunks(b"q\nr\n", &fps[0]), Err(1));
        // No trailing context: the second hunk must match at the end.
        assert_eq!(apply_hunks(b"a\nb\nc\nd\ne\nf\nz\n", &fps[0]), Err(5));
        let mid = "--- a/f\n+++ b/f\n@@ -3,3 +3,3 @@\n c\n-d\n+D\n e\n";
        let fp = &parse_unified_diff(mid.as_bytes()).unwrap()[0];
        let out = apply_hunks(b"x\na\nb\nc\nd\ne\nf\n", fp).unwrap();
        assert_eq!(out, b"x\na\nb\nc\nD\ne\nf\n");
    }

    #[test]
    fn creation_deletion_and_no_newline() {
        let p = "diff --git a/n b/n\nnew file mode 100644\n--- /dev/null\n+++ b/n\n@@ -0,0 +1 @@\n+x\n\\ No newline at end of file\ndiff --git a/d b/d\ndeleted file mode 100644\n--- a/d\n+++ /dev/null\n@@ -1 +0,0 @@\n-y\n";
        let fps = parse_unified_diff(p.as_bytes()).unwrap();
        assert_eq!(fps[0].old_path, None);
        assert_eq!(fps[0].new_mode, Some(0o100644));
        assert_eq!(apply_hunks(b"", &fps[0]).unwrap(), b"x");
        assert_eq!(fps[1].new_path, None);
        assert_eq!(apply_hunks(b"y\n", &fps[1]).unwrap(), b"");
    }
}
