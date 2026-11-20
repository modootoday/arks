//! The text `range-diff` compares for one commit: its metadata, commit
//! message, notes and file diffs, laid out as stock git prints them inside a
//! range-diff (paths without `a/` / `b/` prefixes, no index lines, hunk
//! headers without line numbers).
//!
//! Primary: <https://git-scm.com/docs/git-range-diff> (§DESCRIPTION,
//! §ALGORITHM, `--[no-]notes`). The section layout follows observed stock
//! output:
//!
//! ```text
//!  ## Metadata ##
//! Author: <name> <<email>>
//!
//!  ## Commit message ##
//!     <message line, indented by four>
//!
//!  ## Notes ##                     (one block per notes ref shown)
//!     <note line>
//!
//!  ## <path> ##                    (or `(new)`, `(deleted)`, `<a> => <b>`,
//! @@ <path>: <function name>        `(mode change <a> => <b>)`)
//!  context / -removed / +added
//! ```

use crate::cmd::diffcore::options::DiffOptions;
use crate::cmd::diffcore::xdiff::{self, HunkOpts, Op};
use crate::cmd::diffcore::{self, Content, MODE_GITLINK, Pair, Side};
use crate::error::CliResult;
use arks_object::Commit;
use arks_oid::Oid;

/// One commit's patch text plus the pieces the matching step needs.
pub struct PatchText {
    pub text: String,
    /// Byte offset where the file diffs start (after metadata, message and
    /// notes): two commits whose diffs are equal match exactly.
    pub diff_offset: usize,
    /// Line count of the file diffs, separator lines not counted: the base
    /// of the cost of leaving the commit unpaired.
    pub diff_size: usize,
}

impl PatchText {
    pub fn diff(&self) -> &str {
        &self.text[self.diff_offset..]
    }
}

/// Notes shown for a commit: (heading, note text) per notes ref.
pub type Notes = Vec<(String, String)>;

/// Tabs expanded to eight-column stops (git log's default for `medium`).
fn expand_tabs(line: &str) -> String {
    if !line.contains('\t') {
        return line.to_string();
    }
    let mut out = String::new();
    let mut col = 0usize;
    for ch in line.chars() {
        if ch == '\t' {
            let n = 8 - col % 8;
            out.extend(std::iter::repeat_n(' ', n));
            col += n;
        } else {
            out.push(ch);
            col += 1;
        }
    }
    out
}

fn push_indented(out: &mut String, text: &str) {
    for line in text.trim_end_matches('\n').split('\n') {
        if line.is_empty() {
            out.push('\n');
        } else {
            out.push_str("    ");
            out.push_str(&expand_tabs(line));
            out.push('\n');
        }
    }
}

/// Build the patch text of `commit` (`oid`) against its first parent.
pub fn build(
    content: &Content<'_>,
    inner: &DiffOptions,
    oid: &Oid,
    commit: &Commit,
    notes: &Notes,
    specs: &[String],
) -> CliResult<PatchText> {
    let _ = oid;
    let mut text = String::new();
    text.push_str(" ## Metadata ##\n");
    text.push_str(&format!(
        "Author: {} <{}>\n",
        commit.author.name, commit.author.email
    ));
    text.push_str("\n ## Commit message ##\n");
    push_indented(&mut text, &commit.message);
    text.push('\n');
    for (heading, note) in notes {
        text.push_str(&format!("\n ## {heading} ##\n"));
        push_indented(&mut text, note);
        text.push('\n');
    }
    let diff_offset = text.len();
    let repo = content.repo;
    let parent_tree = match commit.parents.first() {
        Some(p) => Some(crate::cmd::log_format::read_commit(repo, p)?.tree),
        None => None,
    };
    let mut pairs = diffcore::tree_pairs(
        repo,
        parent_tree.as_ref(),
        Some(&commit.tree),
        true,
        false,
        specs,
    )?;
    diffcore::transform(content, inner, &mut pairs)?;
    let mut sections: Vec<String> = Vec::new();
    for p in &pairs {
        if p.status() == 'T' {
            sections.push(file_section(
                content,
                inner,
                &Pair::new(p.old.clone(), None),
            )?);
            sections.push(file_section(
                content,
                inner,
                &Pair::new(None, p.new.clone()),
            )?);
        } else {
            sections.push(file_section(content, inner, p)?);
        }
    }
    let diff = sections.join("\n");
    let diff_size = diff.lines().filter(|l| !l.is_empty()).count();
    text.push_str(&diff);
    Ok(PatchText {
        text,
        diff_offset,
        diff_size,
    })
}

fn side_bytes(c: &Content<'_>, s: Option<&Side>) -> CliResult<Vec<u8>> {
    Ok(match s {
        None => Vec::new(),
        Some(s) if s.mode == MODE_GITLINK => match s.oid {
            Some(o) => format!("Subproject commit {}\n", o.to_hex()).into_bytes(),
            None => Vec::new(),
        },
        Some(s) => c.bytes(s)?.to_vec(),
    })
}

fn default_funcname(l: &[u8]) -> Option<&[u8]> {
    diffcore::emit::is_funcname(l).then_some(l)
}

/// One file's part of the patch text.
fn file_section(c: &Content<'_>, o: &DiffOptions, p: &Pair) -> CliResult<String> {
    let old_path = p.old_path().to_string();
    let new_path = p.path().to_string();
    let mut head = match (&p.old, &p.new) {
        (None, _) => format!("{new_path} (new)"),
        (_, None) => format!("{old_path} (deleted)"),
        _ if p.is_rename_or_copy() => format!("{old_path} => {new_path}"),
        _ => new_path.clone(),
    };
    if let (Some(a), Some(b)) = (&p.old, &p.new) {
        if a.mode != b.mode {
            head.push_str(&format!(" (mode change {:06o} => {:06o})", a.mode, b.mode));
        }
    }
    let mut out = format!(" ## {head} ##\n");
    let a = side_bytes(c, p.old.as_ref())?;
    let b = side_bytes(c, p.new.as_ref())?;
    if p.old.is_some() && p.new.is_some() && a == b {
        return Ok(out);
    }
    if !o.text && (xdiff::is_binary(&a) || xdiff::is_binary(&b)) {
        let an = if p.old.is_some() {
            &old_path
        } else {
            "/dev/null"
        };
        let bn = if p.new.is_some() {
            &new_path
        } else {
            "/dev/null"
        };
        // Lines that are not part of a hunk carry a leading space.
        out.push_str(&format!(" Binary files {an} and {bn} differ\n"));
        return Ok(out);
    }
    let al = xdiff::split_lines(&a);
    let bl = xdiff::split_lines(&b);
    let ops = xdiff::diff_lines(&al, &bl, &o.line);
    let ho = HunkOpts {
        context: o.context(),
        inter: o.inter_hunk,
        function_context: false,
    };
    let none = |_: usize, _: usize| false;
    let hunks = xdiff::hunks(&ops, &al, ho, &none, &diffcore::emit::is_funcname);
    let pos = xdiff::positions(&ops);
    let mut bytes: Vec<u8> = Vec::new();
    for h in hunks {
        bytes.extend_from_slice(b"@@");
        if let Some(f) = diffcore::emit::funcname_by(&al, h.a0, &default_funcname) {
            bytes.extend_from_slice(format!(" {new_path}: ").as_bytes());
            bytes.extend_from_slice(f);
        }
        bytes.push(b'\n');
        for (i, op) in ops.iter().enumerate().take(h.end).skip(h.start) {
            let (ai, bi) = pos[i];
            let (mark, line) = match op {
                Op::Eq => (b' ', bl[bi]),
                Op::Del => (b'-', al[ai]),
                Op::Ins => (b'+', bl[bi]),
            };
            bytes.push(mark);
            bytes.extend_from_slice(line);
            if !line.ends_with(b"\n") {
                bytes.extend_from_slice(b"\n \\ No newline at end of file\n");
            }
        }
    }
    out.push_str(&String::from_utf8_lossy(&bytes));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_lines_are_indented_and_tabs_expanded() {
        let mut s = String::new();
        push_indented(&mut s, "subject\n\nbody\twith tab\n");
        assert_eq!(s, "    subject\n\n    body    with tab\n");
    }
}
