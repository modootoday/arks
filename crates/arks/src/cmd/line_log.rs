//! `log -L` / `show -L`: line-range history.
//!
//! Each `-L <start>,<end>:<file>` or `-L :<funcname>:<file>` is resolved
//! against the file in the first tip. Walking the commits newest first, a
//! commit's ranges are mapped onto each parent through a line diff
//! (following whole-file renames); a commit is shown when one of its ranges
//! changed, with a patch limited to the tracked lines (the whole range as
//! context). A merge whose ranges are unchanged against one parent hands
//! them to that parent only and is not shown; otherwise it is shown without
//! a patch and every parent continues. Root commits show the ranges as
//! added. `show -L` shows the ranges of each commit as added.
//! Primary: <https://git-scm.com/docs/git-log#Documentation/git-log.txt--Lltstartgtltendgtltfilegt>,
//! <https://git-scm.com/docs/line-range-format>.

use crate::cmd::diffcore::xdiff::{LineOpts, Op, diff_lines, split_lines};
use crate::cmd::line_range::{parse_all, split_log_arg};
use crate::cmd::log_format::read_commit;
use crate::error::{CliError, CliResult};
use arks_object::{Commit, ObjectKind};
use arks_oid::Oid;
use arks_repo::Repository;
use std::collections::{BTreeMap, HashMap};

/// Tracked ranges per path (0-based, half-open, sorted, merged).
pub type Ranges = BTreeMap<String, Vec<(usize, usize)>>;

/// Split every `-L` value into (range spec, path relative to the top).
pub fn parse_specs(values: &[String], work: &std::path::Path) -> CliResult<Vec<(String, String)>> {
    let mut out = Vec::new();
    for v in values {
        let (range, file) = split_log_arg(v).ok_or_else(|| {
            CliError::fatal(format!(
                "-L argument not 'start,end:file' or ':funcname:file': {v}"
            ))
        })?;
        let path = crate::cmd::core::pathspec_rel(work, &file)?;
        out.push((range, path));
    }
    Ok(out)
}

fn blob_lines(repo: &Repository, commit: &Commit, path: &str) -> CliResult<Option<Vec<Vec<u8>>>> {
    let Some(o) = crate::cmd::shared::tree_entry_at(repo, &commit.tree, path) else {
        return Ok(None);
    };
    let obj = repo.get_object(&o)?;
    if obj.kind != ObjectKind::Blob {
        return Ok(None);
    }
    Ok(Some(
        split_lines(&obj.payload)
            .into_iter()
            .map(|l| l.to_vec())
            .collect(),
    ))
}

/// Resolve the specs against the file contents of `tip`.
pub fn initial_ranges(
    repo: &Repository,
    tip: &Commit,
    specs: &[(String, String)],
) -> CliResult<Ranges> {
    let mut by_path: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (r, p) in specs {
        by_path.entry(p.clone()).or_default().push(r.clone());
    }
    let mut out = Ranges::new();
    for (path, rs) in by_path {
        let lines = blob_lines(repo, tip, &path)?
            .ok_or_else(|| CliError::fatal(format!("There is no path {path} in the commit")))?;
        let plain: Vec<&[u8]> = lines
            .iter()
            .map(|l| l.strip_suffix(b"\n").unwrap_or(l))
            .collect();
        let spans = parse_all(&rs, &plain, &path).map_err(|e| {
            e.into_cli("usage: git log [<options>] [<revision-range>] [[--] <path>...]")
        })?;
        out.insert(
            path,
            spans
                .into_iter()
                .filter(|(b, e)| b <= e)
                .map(|(b, e)| (b - 1, e))
                .collect(),
        );
    }
    Ok(out)
}

fn merge_ranges(v: &mut Vec<(usize, usize)>) {
    v.sort_unstable();
    let mut out: Vec<(usize, usize)> = Vec::new();
    for (b, e) in v.drain(..) {
        match out.last_mut() {
            Some(l) if b <= l.1 => l.1 = l.1.max(e),
            _ => out.push((b, e)),
        }
    }
    *v = out;
}

fn header(old: Option<&str>, new: &str) -> String {
    match old {
        Some(o) => format!("diff --git a/{o} b/{new}\n--- a/{o}\n+++ b/{new}\n"),
        None => format!("diff --git a/{new} b/{new}\n--- /dev/null\n+++ b/{new}\n"),
    }
}

fn push_line(out: &mut String, mark: char, line: &[u8]) {
    out.push(mark);
    out.push_str(&String::from_utf8_lossy(
        line.strip_suffix(b"\n").unwrap_or(line),
    ));
    out.push('\n');
}

/// The ranges of `path` shown as added (root commits, `show -L`). Like
/// git, only the first range of the file is printed.
fn added_text(path: &str, lines: &[Vec<u8>], ranges: &[(usize, usize)]) -> String {
    let mut t = header(None, path);
    for (b, e) in ranges.iter().take(1) {
        t.push_str(&format!("@@ -0,0 +{},{} @@\n", b + 1, e - b));
        for l in &lines[*b..*e] {
            push_line(&mut t, '+', l);
        }
    }
    t
}

/// Whole-file rename source of `path` in `parent` (most similar deleted file).
fn rename_source(
    repo: &Repository,
    parent: &Commit,
    child: &Commit,
    path: &str,
) -> CliResult<Option<String>> {
    let pf = crate::cmd::diffcore::flat_tree(repo, &parent.tree)?;
    let cf = crate::cmd::diffcore::flat_tree(repo, &child.tree)?;
    let Some((_, dst_oid)) = cf.get(path) else {
        return Ok(None);
    };
    let dst = repo.get_object(dst_oid)?.payload;
    let max = crate::cmd::diffcore::options::MAX_SCORE;
    let min = max / 2;
    let mut best: Option<(u32, String)> = None;
    for (p, (mode, oid)) in &pf {
        if cf.contains_key(p) || (mode & 0o170000) == 0o160000 {
            continue;
        }
        let score = if oid == dst_oid {
            max
        } else {
            let src = repo.get_object(oid)?.payload;
            crate::cmd::diffcore::rename::similarity(&src, &dst, min)
        };
        if score >= min && best.as_ref().is_none_or(|b| score > b.0) {
            best = Some((score, p.clone()));
        }
    }
    Ok(best.map(|b| b.1))
}

/// Map `ranges` of `child` onto `parent`: (parent ranges, patch text when
/// a range changed).
fn map_to_parent(
    repo: &Repository,
    child: &Commit,
    parent: &Commit,
    ranges: &Ranges,
) -> CliResult<(Ranges, Option<String>)> {
    let mut mapped = Ranges::new();
    let mut text = String::new();
    let mut changed = false;
    for (path, rs) in ranges {
        let Some(clines) = blob_lines(repo, child, path)? else {
            continue;
        };
        let ppath = match blob_lines(repo, parent, path)? {
            Some(_) => Some(path.clone()),
            None => rename_source(repo, parent, child, path)?,
        };
        let Some(ppath) = ppath else {
            changed = true;
            text.push_str(&added_text(path, &clines, rs));
            continue;
        };
        let plines = blob_lines(repo, parent, &ppath)?.unwrap_or_default();
        let a: Vec<&[u8]> = plines.iter().map(|l| l.as_slice()).collect();
        let b: Vec<&[u8]> = clines.iter().map(|l| l.as_slice()).collect();
        let ops = diff_lines(&a, &b, &LineOpts::default());
        // For each child line: Some(parent line) when unchanged; hunks
        // record the parent side of each change.
        let mut map: Vec<Option<usize>> = vec![None; b.len()];
        let mut hunk_of: Vec<Option<(usize, usize)>> = vec![None; b.len()];
        // Pure deletions: (child position, parent start, count).
        let mut deletions: Vec<(usize, usize, usize)> = Vec::new();
        let (mut ai, mut bi) = (0usize, 0usize);
        let mut k = 0;
        while k < ops.len() {
            if ops[k] == Op::Eq {
                map[bi] = Some(ai);
                ai += 1;
                bi += 1;
                k += 1;
                continue;
            }
            let (a0, b0) = (ai, bi);
            while k < ops.len() && ops[k] != Op::Eq {
                match ops[k] {
                    Op::Del => ai += 1,
                    Op::Ins => bi += 1,
                    Op::Eq => {}
                }
                k += 1;
            }
            if bi == b0 {
                deletions.push((b0, a0, ai - a0));
            }
            for h in hunk_of.iter_mut().take(bi).skip(b0) {
                *h = Some((a0, ai));
            }
        }
        let mut new_rs = Vec::new();
        let mut file_text = String::new();
        for &(s, e) in rs {
            let mut lo = usize::MAX;
            let mut hi = 0usize;
            let mut touched = false;
            for c in s..e.min(b.len()) {
                match (map[c], hunk_of[c]) {
                    (Some(p), _) => {
                        lo = lo.min(p);
                        hi = hi.max(p + 1);
                    }
                    (None, Some((x, y))) => {
                        touched = true;
                        if x < y {
                            lo = lo.min(x);
                            hi = hi.max(y);
                        }
                    }
                    (None, None) => touched = true,
                }
            }
            for &(pos, p0, n) in &deletions {
                if pos > s && pos < e {
                    touched = true;
                    lo = lo.min(p0);
                    hi = hi.max(p0 + n);
                }
            }
            let (ps, pe) = if lo == usize::MAX {
                // Everything was added: an empty parent range before it.
                let at = (s..e).find_map(|c| hunk_of[c].map(|h| h.0)).unwrap_or(0);
                (at, at)
            } else {
                (lo, hi)
            };
            if pe > ps {
                new_rs.push((ps, pe));
            }
            if !touched {
                continue;
            }
            changed = true;
            file_text.push_str(&format!(
                "@@ -{},{} +{},{} @@\n",
                ps + 1,
                pe - ps,
                s + 1,
                e - s
            ));
            let sub = diff_lines(&a[ps..pe], &b[s..e], &LineOpts::default());
            let (mut x, mut y) = (ps, s);
            for op in sub {
                match op {
                    Op::Eq => {
                        push_line(&mut file_text, ' ', b[y]);
                        x += 1;
                        y += 1;
                    }
                    Op::Del => {
                        push_line(&mut file_text, '-', a[x]);
                        x += 1;
                    }
                    Op::Ins => {
                        push_line(&mut file_text, '+', b[y]);
                        y += 1;
                    }
                }
            }
        }
        if !file_text.is_empty() {
            text.push_str(&header(Some(&ppath), path));
            text.push_str(&file_text);
        }
        merge_ranges(&mut new_rs);
        if !new_rs.is_empty() {
            mapped.entry(ppath).or_default().extend(new_rs);
        }
    }
    for v in mapped.values_mut() {
        merge_ranges(v);
    }
    Ok((mapped, changed.then_some(text)))
}

/// One commit `log -L` shows, with its patch text (`None` for merges).
pub struct Shown {
    pub index: usize,
    pub patch: Option<String>,
}

/// Filter the walked commits (newest first) down to those that change the
/// tracked ranges.
pub fn filter(
    repo: &Repository,
    list: &[(Oid, Commit)],
    tip: Oid,
    specs: &[(String, String)],
    first_parent: bool,
) -> CliResult<Vec<Shown>> {
    let tip_commit = read_commit(repo, &tip)?;
    let mut data: HashMap<Oid, Ranges> = HashMap::new();
    data.insert(tip, initial_ranges(repo, &tip_commit, specs)?);
    let mut shown = Vec::new();
    let add = |data: &mut HashMap<Oid, Ranges>, p: Oid, r: Ranges| {
        let e = data.entry(p).or_default();
        for (path, rs) in r {
            let v = e.entry(path).or_default();
            v.extend(rs);
            merge_ranges(v);
        }
    };
    for (i, (oid, commit)) in list.iter().enumerate() {
        let Some(d) = data.remove(oid) else {
            continue;
        };
        if d.values().all(|v| v.is_empty()) {
            continue;
        }
        let mut parents = commit.parents.clone();
        if first_parent {
            parents.truncate(1);
        }
        if parents.is_empty() {
            let mut t = String::new();
            for (path, rs) in &d {
                if rs.is_empty() {
                    continue;
                }
                let lines = blob_lines(repo, commit, path)?.unwrap_or_default();
                t.push_str(&added_text(path, &lines, rs));
            }
            shown.push(Shown {
                index: i,
                patch: Some(t),
            });
            continue;
        }
        let mut maps = Vec::new();
        for p in &parents {
            let pc = read_commit(repo, p)?;
            maps.push((*p, map_to_parent(repo, commit, &pc, &d)?));
        }
        if parents.len() == 1 {
            let (p, (r, t)) = maps.pop().expect("one parent");
            if t.is_some() {
                // A merge followed through its first parent shows no patch.
                let patch = if commit.parents.len() > 1 { None } else { t };
                shown.push(Shown { index: i, patch });
            }
            add(&mut data, p, r);
            continue;
        }
        if let Some(k) = maps.iter().position(|(_, (_, t))| t.is_none()) {
            let (p, (r, _)) = maps.swap_remove(k);
            add(&mut data, p, r);
            continue;
        }
        shown.push(Shown {
            index: i,
            patch: None,
        });
        for (p, (r, _)) in maps {
            add(&mut data, p, r);
        }
    }
    Ok(shown)
}

/// `show -L`: the ranges of `commit` as an addition (merges: nothing).
pub fn show_text(
    repo: &Repository,
    commit: &Commit,
    specs: &[(String, String)],
) -> CliResult<String> {
    if commit.parents.len() > 1 {
        return Ok(String::new());
    }
    let ranges = initial_ranges(repo, commit, specs)?;
    let mut t = String::new();
    for (path, rs) in &ranges {
        if rs.is_empty() {
            continue;
        }
        let lines = blob_lines(repo, commit, path)?.unwrap_or_default();
        t.push_str(&added_text(path, &lines, rs));
    }
    Ok(t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges_merge() {
        let mut v = vec![(5, 7), (0, 2), (1, 3), (7, 9)];
        merge_ranges(&mut v);
        assert_eq!(v, vec![(0, 3), (5, 9)]);
    }

    #[test]
    fn added_hunks() {
        let lines = vec![b"a\n".to_vec(), b"b\n".to_vec(), b"c\n".to_vec()];
        let t = added_text("f", &lines, &[(1, 3), (0, 1)]);
        assert!(t.ends_with("@@ -0,0 +2,2 @@\n+b\n+c\n"));
    }
}
