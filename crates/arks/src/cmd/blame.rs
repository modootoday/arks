//! `blame` / `annotate` / `pickaxe`: per-line origin commits.
//!
//! The final file's lines start as one entry per `-L` range, owned by the
//! starting commit (or a fake "Not Committed Yet" / `--contents` commit
//! whose parent is HEAD or the given revision). Commits are processed
//! newest committer date first; each passes the parts of its entries that a
//! line diff against a parent leaves unchanged to that parent (an identical
//! blob takes everything), following whole-file renames. `-M` and `-C`
//! additionally look for moved / copied blocks in the parent's file and in
//! other files (`-C -C` / `-C -C -C` widen the search); `--ignore-rev`
//! commits hand changed lines to the nearest parent line; `--reverse`
//! walks children instead of parents; `-S` reads parents from a graft file.
//! Whatever no parent takes is blamed on the commit. Range bottoms, roots
//! (without `--root`) and commits older than `--since` are boundaries.
//! Output: default, `-c` (annotate), `-s`, `-e`, `-f`, `-n`, `-l`, `-t`,
//! `-b`, `--abbrev`, `--date`, `--score-debug`, `--color-lines`,
//! `--color-by-age`, `--porcelain`, `--line-porcelain`, `--incremental` and
//! `--show-stats`.
//! Primary: <https://git-scm.com/docs/git-blame>, <https://git-scm.com/docs/git-annotate>.
//!
//! Partial: `--show-stats` counts are an estimate of git's internal
//! counters, `--progress` prints nothing, `--encoding` does not re-encode.

use crate::cmd::diffcore::xdiff::{Algo, LineOpts, Op, diff_lines, split_lines};
use crate::cmd::log_format::{read_commit, subject_line};
use crate::cmd::opt_parse::{Item, Mode, OptDef, Takes, o, parse};
use crate::cmd::shared::{ObjKind, abbrev_hex, peel_to_kind, resolve_rev};
use crate::error::{CliError, CliResult};
use crate::util::{open_repo, work_tree};
use arks_object::{Commit, ObjectKind, Signature};
use arks_oid::Oid;
use arks_repo::Repository;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::Write;

const USAGE: &str = "usage: git blame [<options>] [<rev-opts>] [<rev>] [--] <file>\n\n    <rev-opts> are documented in git-rev-list(1)\n";

const OPTS: &[OptDef] = &[
    o("incremental", ' ', Takes::No, true),
    o("-b", 'b', Takes::No, false),
    o("root", ' ', Takes::No, true),
    o("show-stats", ' ', Takes::No, true),
    o("progress", ' ', Takes::No, true),
    o("score-debug", ' ', Takes::No, true),
    o("show-name", 'f', Takes::No, true),
    o("show-number", 'n', Takes::No, true),
    o("porcelain", 'p', Takes::No, true),
    o("line-porcelain", ' ', Takes::No, true),
    o("-c", 'c', Takes::No, false),
    o("-t", 't', Takes::No, false),
    o("-l", 'l', Takes::No, false),
    o("-s", 's', Takes::No, false),
    o("show-email", 'e', Takes::No, true),
    o("-w", 'w', Takes::No, false),
    o("ignore-rev", ' ', Takes::Req, true),
    o("ignore-revs-file", ' ', Takes::Req, true),
    o("color-lines", ' ', Takes::No, true),
    o("color-by-age", ' ', Takes::No, true),
    o("minimal", ' ', Takes::No, true),
    o("-S", 'S', Takes::Req, false),
    o("contents", ' ', Takes::Req, true),
    o("-C", 'C', Takes::Opt, false),
    o("-M", 'M', Takes::Opt, false),
    o("-L", 'L', Takes::Req, false),
    o("abbrev", ' ', Takes::Opt, true),
];

const DEFAULT_MOVE_SCORE: usize = 20;
const DEFAULT_COPY_SCORE: usize = 40;

/// One run of final lines blamed (so far) on one origin.
#[derive(Clone, Copy, Debug)]
struct Entry {
    /// 0-based first line in the final file.
    lno: usize,
    /// 0-based first line in the origin's blob.
    s_lno: usize,
    num: usize,
    origin: usize,
    /// Handed over by an `--ignore-rev` commit (never coalesced with others).
    ignored: bool,
}

/// A (commit, path) pair and its file content.
struct Origin {
    commit: Oid,
    path: String,
    blob: Option<Oid>,
    lines: Vec<Vec<u8>>,
    previous: Option<usize>,
}

#[derive(Default)]
struct Settings {
    incremental: bool,
    blank_boundary: bool,
    show_root: bool,
    show_stats: bool,
    score_debug: bool,
    show_name: Option<bool>,
    show_number: bool,
    porcelain: bool,
    line_porcelain: bool,
    compat: bool,
    raw_time: bool,
    long_names: bool,
    suppress: bool,
    email: bool,
    color_lines: bool,
    color_by_age: bool,
    abbrev: Option<usize>,
    date: String,
}

struct Engine<'a> {
    repo: &'a Repository,
    origins: Vec<Origin>,
    by_key: HashMap<(Oid, String), usize>,
    commits: HashMap<Oid, Commit>,
    fake: Option<Oid>,
    line_opts: LineOpts,
    move_score: Option<usize>,
    copy_level: usize,
    copy_score: usize,
    ignored: HashSet<Oid>,
    grafts: HashMap<Oid, Vec<Oid>>,
    uninteresting: HashSet<Oid>,
    reverse_bottom: Option<Oid>,
    children: Option<HashMap<Oid, Vec<Oid>>>,
    first_parent: bool,
    show_root: bool,
    since: Option<i64>,
    /// Commits whose lines came from more than one path.
    multi_path: HashSet<Oid>,
    boundary: HashSet<Oid>,
    patches: usize,
}

fn alnum_score(lines: &[Vec<u8>]) -> usize {
    1 + lines
        .iter()
        .map(|l| l.iter().filter(|c| c.is_ascii_alphanumeric()).count())
        .sum::<usize>()
}

fn to_lines(content: &[u8]) -> Vec<Vec<u8>> {
    split_lines(content)
        .into_iter()
        .map(|l| l.to_vec())
        .collect()
}

impl<'a> Engine<'a> {
    fn commit(&mut self, oid: Oid) -> CliResult<&Commit> {
        if !self.commits.contains_key(&oid) {
            let c = read_commit(self.repo, &oid)?;
            self.commits.insert(oid, c);
        }
        Ok(&self.commits[&oid])
    }

    fn date(&mut self, oid: Oid) -> CliResult<i64> {
        Ok(self.commit(oid)?.committer.timestamp)
    }

    fn blob_in(&mut self, commit: Oid, path: &str) -> CliResult<Option<Oid>> {
        let tree = self.commit(commit)?.tree;
        Ok(
            crate::cmd::shared::tree_entry_at(self.repo, &tree, path).filter(|o| {
                self.repo
                    .get_object(o)
                    .map(|x| x.kind == ObjectKind::Blob)
                    .unwrap_or(false)
            }),
        )
    }

    fn get_origin(&mut self, commit: Oid, path: &str, blob: Oid) -> CliResult<usize> {
        if let Some(i) = self.by_key.get(&(commit, path.to_string())) {
            return Ok(*i);
        }
        let content = self.repo.get_object(&blob)?.payload;
        self.origins.push(Origin {
            commit,
            path: path.to_string(),
            blob: Some(blob),
            lines: to_lines(&content),
            previous: None,
        });
        let i = self.origins.len() - 1;
        self.by_key.insert((commit, path.to_string()), i);
        Ok(i)
    }

    /// The parents blame passes to (children with `--reverse`).
    fn scapegoats(&mut self, c: Oid) -> CliResult<Vec<Oid>> {
        let mut v = if let Some(ch) = &self.children {
            ch.get(&c).cloned().unwrap_or_default()
        } else if let Some(g) = self.grafts.get(&c) {
            g.clone()
        } else {
            self.commit(c)?.parents.clone()
        };
        if self.first_parent {
            v.truncate(1);
        }
        Ok(v)
    }

    /// The origin of `path` (or what it was renamed from) in `parent`.
    fn find_origin(&mut self, parent: Oid, child: Oid, path: &str) -> CliResult<Option<usize>> {
        if let Some(b) = self.blob_in(parent, path)? {
            return Ok(Some(self.get_origin(parent, path, b)?));
        }
        if self.children.is_some() || Some(child) == self.fake {
            return Ok(None);
        }
        // Whole-file rename: the most similar file deleted by `child`.
        let ptree = self.commit(parent)?.tree;
        let ctree = self.commit(child)?.tree;
        let pf = crate::cmd::diffcore::flat_tree(self.repo, &ptree)?;
        let cf = crate::cmd::diffcore::flat_tree(self.repo, &ctree)?;
        let Some((_, dst_oid)) = cf.get(path) else {
            return Ok(None);
        };
        let dst = self.repo.get_object(dst_oid)?.payload;
        let max = crate::cmd::diffcore::options::MAX_SCORE;
        let min = max / 2;
        let mut best: Option<(u32, String, Oid)> = None;
        for (p, (mode, oid)) in &pf {
            if cf.contains_key(p) || (mode & 0o170000) == 0o160000 {
                continue;
            }
            let score = if oid == dst_oid {
                max
            } else {
                let src = self.repo.get_object(oid)?.payload;
                crate::cmd::diffcore::rename::similarity(&src, &dst, min)
            };
            if score >= min && best.as_ref().is_none_or(|b| score > b.0) {
                best = Some((score, p.clone(), *oid));
            }
        }
        match best {
            Some((_, p, oid)) => Ok(Some(self.get_origin(parent, &p, oid)?)),
            None => Ok(None),
        }
    }

    fn is_boundary(&mut self, c: Oid) -> CliResult<bool> {
        if Some(c) == self.fake {
            return Ok(false);
        }
        if self.uninteresting.contains(&c) {
            return Ok(true);
        }
        if let Some(s) = self.since {
            if self.date(c)? < s {
                return Ok(true);
            }
        }
        Ok(self.commit(c)?.parents.is_empty() && !self.show_root)
    }

    /// Diff `parent` against `target` and hand unchanged parts of `entries`
    /// to the parent; the rest is returned.
    fn pass_to_parent(
        &mut self,
        entries: Vec<Entry>,
        target: usize,
        parent: usize,
        ignored: bool,
    ) -> (Vec<Entry>, Vec<Entry>) {
        self.patches += 1;
        let (map, hunks) = {
            let a: Vec<&[u8]> = self.origins[parent]
                .lines
                .iter()
                .map(|l| l.as_slice())
                .collect();
            let b: Vec<&[u8]> = self.origins[target]
                .lines
                .iter()
                .map(|l| l.as_slice())
                .collect();
            let ops = diff_lines(&a, &b, &self.line_opts);
            let mut map = vec![None; b.len()];
            let mut hunks: Vec<(usize, usize, usize, usize)> = Vec::new();
            let (mut ai, mut bi) = (0, 0);
            let mut cur: Option<(usize, usize, usize, usize)> = None;
            for op in ops {
                match op {
                    Op::Eq => {
                        if let Some(h) = cur.take() {
                            hunks.push(h);
                        }
                        map[bi] = Some(ai);
                        ai += 1;
                        bi += 1;
                    }
                    Op::Del => {
                        let h = cur.get_or_insert((ai, 0, bi, 0));
                        h.1 += 1;
                        ai += 1;
                    }
                    Op::Ins => {
                        let h = cur.get_or_insert((ai, 0, bi, 0));
                        h.3 += 1;
                        bi += 1;
                    }
                }
            }
            if let Some(h) = cur.take() {
                hunks.push(h);
            }
            (map, hunks)
        };
        let mut passed = Vec::new();
        let mut kept = Vec::new();
        for e in entries {
            let mut k = 0;
            while k < e.num {
                let b = e.s_lno + k;
                match map.get(b).copied().flatten() {
                    Some(a0) => {
                        let mut n = 1;
                        while k + n < e.num && map.get(b + n).copied().flatten() == Some(a0 + n) {
                            n += 1;
                        }
                        passed.push(Entry {
                            lno: e.lno + k,
                            s_lno: a0,
                            num: n,
                            origin: parent,
                            ignored: e.ignored,
                        });
                        k += n;
                    }
                    None => {
                        let guess = if ignored {
                            self.guess_line(b, &hunks, target, parent)
                        } else {
                            None
                        };
                        match guess {
                            Some(a) => passed.push(Entry {
                                lno: e.lno + k,
                                s_lno: a,
                                num: 1,
                                origin: parent,
                                ignored: true,
                            }),
                            None => {
                                let mut n = 1;
                                if !ignored {
                                    while k + n < e.num
                                        && map.get(b + n).copied().flatten().is_none()
                                    {
                                        n += 1;
                                    }
                                }
                                kept.push(Entry {
                                    lno: e.lno + k,
                                    s_lno: b,
                                    num: n,
                                    origin: target,
                                    ignored: e.ignored,
                                });
                                k += n;
                                continue;
                            }
                        }
                        k += 1;
                    }
                }
            }
        }
        (passed, merge_adjacent(kept))
    }

    /// `--ignore-rev`: the parent line a changed line most likely came from.
    fn guess_line(
        &self,
        b: usize,
        hunks: &[(usize, usize, usize, usize)],
        target: usize,
        parent: usize,
    ) -> Option<usize> {
        let &(a0, alen, b0, blen) = hunks.iter().find(|h| b >= h.2 && b < h.2 + h.3)?;
        if alen == 0 {
            return None;
        }
        let pairs = |l: &[u8]| -> HashSet<(u8, u8)> {
            l.windows(2)
                .map(|w| (w[0].to_ascii_lowercase(), w[1].to_ascii_lowercase()))
                .filter(|(x, y)| !x.is_ascii_whitespace() && !y.is_ascii_whitespace())
                .collect()
        };
        let mine = pairs(&self.origins[target].lines[b]);
        let mut best: Option<(usize, usize)> = None;
        for a in a0..a0 + alen {
            let theirs = pairs(&self.origins[parent].lines[a]);
            let common = mine.intersection(&theirs).count();
            if common > 0 && best.is_none_or(|(c, _)| common > c) {
                best = Some((common, a));
            }
        }
        if let Some((_, a)) = best {
            return Some(a);
        }
        let k = b - b0;
        let _ = blen;
        (k < alen).then_some(a0 + k)
    }

    /// Best block of `lines` (an entry's text) found in `cand`'s blob:
    /// (offset in the entry, offset in cand, length, score).
    fn best_block(&self, text: &[Vec<u8>], cand: usize) -> Option<(usize, usize, usize, usize)> {
        let a: Vec<&[u8]> = self.origins[cand]
            .lines
            .iter()
            .map(|l| l.as_slice())
            .collect();
        let b: Vec<&[u8]> = text.iter().map(|l| l.as_slice()).collect();
        let ops = diff_lines(&a, &b, &self.line_opts);
        let mut map = vec![None; b.len()];
        let (mut ai, mut bi) = (0, 0);
        for op in ops {
            match op {
                Op::Eq => {
                    map[bi] = Some(ai);
                    ai += 1;
                    bi += 1;
                }
                Op::Del => ai += 1,
                Op::Ins => bi += 1,
            }
        }
        let mut best: Option<(usize, usize, usize, usize)> = None;
        let mut k = 0;
        while k < b.len() {
            let Some(a0) = map[k] else {
                k += 1;
                continue;
            };
            let mut n = 1;
            while k + n < b.len() && map[k + n] == Some(a0 + n) {
                n += 1;
            }
            let score = alnum_score(&text[k..k + n]);
            if best.is_none_or(|x| score > x.3) {
                best = Some((k, a0, n, score));
            }
            k += n;
        }
        best
    }

    /// `-M` / `-C`: move matching blocks of `entries` to the best candidate.
    fn find_blocks(
        &mut self,
        entries: Vec<Entry>,
        target: usize,
        cands: &[usize],
        threshold: usize,
    ) -> (Vec<Entry>, Vec<Entry>) {
        let mut passed = Vec::new();
        let mut work = entries;
        loop {
            let mut progress = false;
            let mut next = Vec::new();
            for e in work {
                let text: Vec<Vec<u8>> =
                    self.origins[target].lines[e.s_lno..e.s_lno + e.num].to_vec();
                let mut best: Option<(usize, usize, usize, usize, usize)> = None;
                for &c in cands {
                    if let Some((off, a0, n, score)) = self.best_block(&text, c) {
                        if best.is_none_or(|b| score > b.3) {
                            best = Some((off, a0, n, score, c));
                        }
                    }
                }
                match best {
                    Some((off, a0, n, score, c)) if score > threshold => {
                        progress = true;
                        if off > 0 {
                            next.push(Entry { num: off, ..e });
                        }
                        passed.push(Entry {
                            lno: e.lno + off,
                            s_lno: a0,
                            num: n,
                            origin: c,
                            ignored: e.ignored,
                        });
                        if off + n < e.num {
                            next.push(Entry {
                                lno: e.lno + off + n,
                                s_lno: e.s_lno + off + n,
                                num: e.num - off - n,
                                origin: target,
                                ignored: e.ignored,
                            });
                        }
                    }
                    _ => next.push(e),
                }
            }
            work = next;
            if !progress || work.is_empty() {
                break;
            }
        }
        (passed, work)
    }

    /// Process one origin: returns the entries handed to other origins and
    /// the ones it keeps (guilty).
    fn pass_blame(
        &mut self,
        target: usize,
        mut entries: Vec<Entry>,
    ) -> CliResult<(Vec<Entry>, Vec<Entry>)> {
        let c = self.origins[target].commit;
        let reverse_bottom = self.reverse_bottom == Some(c);
        if !reverse_bottom && self.is_boundary(c)? {
            self.boundary.insert(c);
            return Ok((Vec::new(), entries));
        }
        if reverse_bottom {
            self.boundary.insert(c);
        }
        let path = self.origins[target].path.clone();
        let parents = self.scapegoats(c)?;
        let mut porigins: Vec<Option<usize>> = Vec::new();
        for p in &parents {
            let po = self.find_origin(*p, c, &path)?;
            if let Some(po) = po {
                if self.origins[po].blob.is_some()
                    && self.origins[po].blob == self.origins[target].blob
                {
                    let moved: Vec<Entry> = entries
                        .into_iter()
                        .map(|e| Entry { origin: po, ..e })
                        .collect();
                    return Ok((moved, Vec::new()));
                }
                let same = porigins
                    .iter()
                    .flatten()
                    .any(|q| self.origins[*q].blob == self.origins[po].blob);
                porigins.push(if same { None } else { Some(po) });
            } else {
                porigins.push(None);
            }
        }
        let ignored = self.ignored.contains(&c);
        let mut passed = Vec::new();
        for po in porigins.iter().flatten() {
            if self.origins[target].previous.is_none() {
                self.origins[target].previous = Some(*po);
            }
            let (p, kept) = self.pass_to_parent(entries, target, *po, ignored);
            passed.extend(p);
            entries = kept;
            if entries.is_empty() {
                return Ok((passed, entries));
            }
        }
        if let Some(score) = self.move_score {
            for po in porigins.iter().flatten() {
                if entries.is_empty() {
                    break;
                }
                let (p, kept) = self.find_blocks(entries, target, &[*po], score);
                passed.extend(p);
                entries = kept;
            }
        }
        if self.copy_level > 0 && !entries.is_empty() {
            let ctree = if Some(c) == self.fake {
                None
            } else {
                Some(self.commit(c)?.tree)
            };
            for (i, p) in parents.iter().enumerate() {
                if entries.is_empty() {
                    break;
                }
                let ptree = self.commit(*p)?.tree;
                let pf = crate::cmd::diffcore::flat_tree(self.repo, &ptree)?;
                let cf = match ctree {
                    Some(t) => crate::cmd::diffcore::flat_tree(self.repo, &t)?,
                    None => BTreeMap::new(),
                };
                let harder =
                    self.copy_level >= 3 || (self.copy_level == 2 && porigins[i].is_none());
                let mut cands = Vec::new();
                for (fp, (mode, oid)) in &pf {
                    if (mode & 0o170000) == 0o160000 {
                        continue;
                    }
                    if porigins[i].is_some_and(|po| self.origins[po].path == *fp) {
                        continue;
                    }
                    let modified = cf.get(fp).is_none_or(|(_, o)| o != oid);
                    if !modified && !harder {
                        continue;
                    }
                    cands.push(self.get_origin(*p, fp, *oid)?);
                }
                let (p, kept) = self.find_blocks(entries, target, &cands, self.copy_score);
                passed.extend(p);
                entries = kept;
            }
        }
        Ok((passed, entries))
    }
}

/// Merge entries of one origin that are adjacent in both files.
fn merge_adjacent(mut v: Vec<Entry>) -> Vec<Entry> {
    v.sort_by_key(|e| e.lno);
    let mut out: Vec<Entry> = Vec::new();
    for e in v {
        if let Some(l) = out.last_mut() {
            if l.origin == e.origin
                && l.ignored == e.ignored
                && l.lno + l.num == e.lno
                && l.s_lno + l.num == e.s_lno
            {
                l.num += e.num;
                continue;
            }
        }
        out.push(e);
    }
    out
}

pub fn cmd_blame(args: &[String]) -> CliResult<()> {
    run(args, false, &mut std::io::stdout().lock())
}

pub fn cmd_annotate(args: &[String]) -> CliResult<()> {
    run(args, true, &mut std::io::stdout().lock())
}

/// `git blame` with its output written to `out` (terminal viewers capture it).
pub fn blame_to(args: &[String], out: &mut dyn Write) -> CliResult<()> {
    run(args, false, out)
}

fn usage_err(msg: &str) -> CliError {
    CliError::Exit(129, format!("{msg}\n{USAGE}"))
}

/// Revision options blame takes from the rev-list family.
struct RevOpts {
    reverse: bool,
    first_parent: bool,
    date: Option<String>,
    since: Option<i64>,
}

fn parse_since(v: &str) -> Option<i64> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    if let Some(ts) = v.strip_prefix('@') {
        return ts.parse().ok();
    }
    let words: Vec<&str> = v.split(['.', ' ']).filter(|w| !w.is_empty()).collect();
    if let [n, unit, "ago"] = words.as_slice() {
        let n: i64 = n.parse().ok()?;
        let secs = match unit.trim_end_matches('s') {
            "second" => 1,
            "minute" => 60,
            "hour" => 3600,
            "day" => 86_400,
            "week" => 7 * 86_400,
            "month" => 30 * 86_400,
            "year" => 365 * 86_400,
            _ => return None,
        };
        return Some(now - n * secs);
    }
    Some(i64::MAX)
}

fn take_rev_opts(unknown: &[String]) -> CliResult<RevOpts> {
    let mut r = RevOpts {
        reverse: false,
        first_parent: false,
        date: None,
        since: None,
    };
    for u in unknown {
        match u.as_str() {
            "--reverse" => r.reverse = true,
            "--first-parent" => r.first_parent = true,
            "--relative-date" => r.date = Some("relative".into()),
            _ => {
                if let Some(v) = u.strip_prefix("--date=") {
                    r.date = Some(v.to_string());
                } else if u.starts_with("--encoding=") {
                } else if let Some(v) = u
                    .strip_prefix("--since=")
                    .or_else(|| u.strip_prefix("--after="))
                    .or_else(|| u.strip_prefix("--max-age="))
                {
                    r.since = parse_since(v);
                } else {
                    return Err(usage_err(&format!("error: unknown option `{u}'")));
                }
            }
        }
    }
    Ok(r)
}

/// Date column width git pads `--date` output to.
fn date_width(spec: &str) -> usize {
    let base = spec.strip_suffix("-local").unwrap_or(spec);
    if let Some(f) = spec
        .strip_prefix("format:")
        .or_else(|| spec.strip_prefix("format-local:"))
    {
        return crate::cmd::date_spec::strftime(f, 0, "+0000")
            .chars()
            .count();
    }
    match base {
        "relative" => 22,
        "human" => 16,
        "short" => 10,
        "unix" => 10,
        "raw" => 16,
        "iso" | "iso8601" | "iso-strict" | "iso8601-strict" => 25,
        "rfc" | "rfc2822" => 31,
        _ => 30,
    }
}

fn digits(n: usize) -> usize {
    n.max(1).to_string().len()
}

fn run(args: &[String], annotate: bool, sink: &mut dyn Write) -> CliResult<()> {
    // `--date x` / `--encoding x` take a separate value in the rev-list family.
    let mut argv: Vec<String> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        if (a == "--date" || a == "--encoding" || a == "--since" || a == "--after")
            && i + 1 < args.len()
        {
            argv.push(format!("{a}={}", args[i + 1]));
            i += 2;
            continue;
        }
        argv.push(a.clone());
        i += 1;
    }
    let p = parse(
        "blame",
        &argv,
        OPTS,
        Mode {
            keep_unknown: true,
            ..Mode::default()
        },
    )
    .map_err(|e| match e {
        CliError::Exit(129, m) => usage_err(m.split("\nusage:").next().unwrap_or("")),
        other => other,
    })?;
    let rev = take_rev_opts(&p.unknown())?;
    let mut st = Settings {
        compat: annotate,
        date: "iso".into(),
        ..Settings::default()
    };
    let mut move_score: Option<usize> = None;
    let mut copy_level = 0usize;
    let mut copy_score = DEFAULT_COPY_SCORE;
    let mut ranges: Vec<String> = Vec::new();
    let mut ignore_revs: Vec<String> = Vec::new();
    let mut ignore_files: Vec<String> = Vec::new();
    let mut contents: Option<String> = None;
    let mut graft_file: Option<String> = None;
    let mut minimal = false;
    let mut ws = false;
    for it in &p.items {
        let Item::Opt { key, val, neg } = it else {
            continue;
        };
        let on = !neg;
        match *key {
            "incremental" => st.incremental = on,
            "-b" => st.blank_boundary = true,
            "root" => st.show_root = on,
            "show-stats" => st.show_stats = on,
            "progress" => {}
            "score-debug" => st.score_debug = on,
            "show-name" => st.show_name = Some(on),
            "show-number" => st.show_number = on,
            "porcelain" => st.porcelain = on,
            "line-porcelain" => {
                st.line_porcelain = on;
                st.porcelain = on;
            }
            "-c" => st.compat = true,
            "-t" => st.raw_time = true,
            "-l" => st.long_names = true,
            "-s" => st.suppress = true,
            "show-email" => st.email = on,
            "-w" => ws = true,
            "ignore-rev" => {
                if *neg {
                    ignore_revs.clear();
                } else if let Some(v) = val {
                    ignore_revs.push(v.clone());
                }
            }
            "ignore-revs-file" => {
                if *neg {
                    ignore_files.clear();
                } else if let Some(v) = val {
                    ignore_files.push(v.clone());
                }
            }
            "color-lines" => st.color_lines = on,
            "color-by-age" => st.color_by_age = on,
            "minimal" => minimal = on,
            "-S" => graft_file = val.clone(),
            "contents" => contents = if *neg { None } else { val.clone() },
            "-C" => {
                copy_level += 1;
                if move_score.is_none() {
                    move_score = Some(DEFAULT_MOVE_SCORE);
                }
                if let Some(v) = val {
                    copy_score = v.parse().map_err(|_| {
                        usage_err(&format!("error: switch `C' expects a numerical value"))
                    })?;
                }
            }
            "-M" => {
                move_score = Some(match val {
                    Some(v) => v
                        .parse()
                        .map_err(|_| usage_err("error: switch `M' expects a numerical value"))?,
                    None => DEFAULT_MOVE_SCORE,
                });
            }
            "-L" => {
                if let Some(v) = val {
                    ranges.push(v.clone());
                }
            }
            "abbrev" => {
                st.abbrev = if *neg {
                    Some(usize::MAX)
                } else {
                    match val {
                        None => Some(7),
                        Some(v) => Some(v.parse().map_err(|_| {
                            usage_err("error: option `abbrev' expects a numerical value")
                        })?),
                    }
                };
            }
            _ => {}
        }
    }
    if let Some(d) = &rev.date {
        st.date = d.clone();
    }
    if st.raw_time {
        st.date = "raw".into();
    }
    if !crate::cmd::date_spec::valid(&st.date) {
        return Err(CliError::fatal(format!("unknown date format {}", st.date)));
    }

    // Positionals: [<rev>...] [--] <file> (also the old `<file> <rev>`).
    let before: Vec<String> = p.args();
    let after: Vec<String> = p.after_dashdash();
    let repo = open_repo()?;
    let work = work_tree(&repo)?;
    let (revs, path_arg): (Vec<String>, String) = if !after.is_empty() {
        if after.len() != 1 {
            return Err(usage_err(""));
        }
        (before, after[0].clone())
    } else {
        match before.as_slice() {
            [] => return Err(usage_err("")),
            [f] => (Vec::new(), f.clone()),
            [a, b]
                if work.join(a).exists()
                    && resolve_rev(&repo, b).is_ok()
                    && resolve_rev(&repo, a).is_err() =>
            {
                (vec![b.clone()], a.clone())
            }
            many => (
                many[..many.len() - 1].to_vec(),
                many[many.len() - 1].clone(),
            ),
        }
    };
    let path = crate::cmd::core::pathspec_rel(&work, &path_arg)?;

    // Revisions: one positive (start), any number of negatives.
    let mut positive: Vec<(String, Oid)> = Vec::new();
    let mut negative: Vec<Oid> = Vec::new();
    let resolve_commit = |r: &str| -> CliResult<Oid> {
        let oid =
            resolve_rev(&repo, r).map_err(|_| CliError::fatal(format!("bad revision '{r}'")))?;
        peel_to_kind(&repo, &oid, ObjKind::Commit, r)
            .map_err(|_| CliError::fatal(format!("bad revision '{r}'")))
    };
    for r in &revs {
        if let Some((a, b)) = r.split_once("..") {
            if b.starts_with('.') {
                return Err(CliError::fatal(format!("bad revision '{r}'")));
            }
            let a = if a.is_empty() { "HEAD" } else { a };
            let b = if b.is_empty() { "HEAD" } else { b };
            negative.push(resolve_commit(a)?);
            positive.push((b.to_string(), resolve_commit(b)?));
        } else if let Some(n) = r.strip_prefix('^') {
            negative.push(resolve_commit(n)?);
        } else {
            positive.push((r.clone(), resolve_commit(r)?));
        }
    }
    if positive.len() > 1 {
        return Err(CliError::fatal("cannot handle more than one rev."));
    }

    let mut eng = Engine {
        repo: &repo,
        origins: Vec::new(),
        by_key: HashMap::new(),
        commits: HashMap::new(),
        fake: None,
        line_opts: LineOpts {
            algo: if minimal { Algo::Minimal } else { Algo::Myers },
            ignore_all_space: ws,
            ..LineOpts::default()
        },
        move_score,
        copy_level,
        copy_score,
        ignored: HashSet::new(),
        grafts: HashMap::new(),
        uninteresting: HashSet::new(),
        reverse_bottom: None,
        children: None,
        first_parent: rev.first_parent,
        show_root: st.show_root,
        since: rev.since,
        multi_path: HashSet::new(),
        boundary: HashSet::new(),
        patches: 0,
    };
    let cfg = arks_config::ConfigSet::load(Some(repo.common_dir()));
    if ignore_files.is_empty()
        && !p.items.iter().any(|it| {
            matches!(
                it,
                Item::Opt {
                    key: "ignore-revs-file",
                    neg: true,
                    ..
                }
            )
        })
    {
        ignore_files = cfg.get_all("blame.ignoreRevsFile");
    }
    for f in &ignore_files {
        let text = std::fs::read_to_string(work.join(f))
            .or_else(|_| std::fs::read_to_string(f))
            .map_err(|_| CliError::fatal(format!("could not open object name list: {f}")))?;
        for l in text.lines() {
            let l = l.split('#').next().unwrap_or("").trim();
            if l.is_empty() {
                continue;
            }
            let o = resolve_rev(&repo, l)
                .map_err(|_| CliError::fatal(format!("invalid object name: {l}")))?;
            eng.ignored.insert(o);
        }
    }
    for r in &ignore_revs {
        let o = resolve_rev(&repo, r)
            .ok()
            .and_then(|o| peel_to_kind(&repo, &o, ObjKind::Commit, r).ok())
            .ok_or_else(|| CliError::fatal(format!("cannot find revision {r} to ignore")))?;
        eng.ignored.insert(o);
    }
    if let Some(g) = &graft_file {
        let text = std::fs::read_to_string(g).map_err(|e| {
            CliError::fatal(format!(
                "reading graft file '{g}' failed: {}",
                match e.kind() {
                    std::io::ErrorKind::NotFound => "No such file or directory".to_string(),
                    std::io::ErrorKind::PermissionDenied => "Permission denied".to_string(),
                    _ => e.to_string(),
                }
            ))
        })?;
        for l in text.lines() {
            let mut it = l.split_whitespace();
            let Some(c) = it.next().and_then(|h| Oid::from_hex(h).ok()) else {
                continue;
            };
            let ps: Vec<Oid> = it.filter_map(|h| Oid::from_hex(h).ok()).collect();
            eng.grafts.insert(c, ps);
        }
    }
    for n in &negative {
        eng.uninteresting
            .extend(arks_repo::ancestor_set(&repo, *n)?);
    }

    // The final file and its owner.
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let mut fake_label: Option<(String, String, String)> = None;
    let final_origin: usize;
    if rev.reverse {
        if contents.is_some() {
            return Err(CliError::fatal(
                "--contents and --reverse do not blend well.",
            ));
        }
        let (bottom, top) = match (negative.first(), positive.first()) {
            (Some(b), Some((_, t))) => (*b, *t),
            (Some(b), None) => (*b, repo.head_oid()?),
            (None, Some((_, t))) => (*t, repo.head_oid()?),
            (None, None) => {
                return Err(CliError::fatal(
                    "--reverse and --first-parent together require specified latest commit",
                ));
            }
        };
        // Children within bottom..top.
        let mut children: HashMap<Oid, Vec<Oid>> = HashMap::new();
        let excluded = arks_repo::ancestor_set(&repo, bottom)?;
        let mut list: Vec<(i64, Oid)> = Vec::new();
        for c in arks_repo::ancestor_set(&repo, top)? {
            if excluded.contains(&c) {
                continue;
            }
            list.push((eng.date(c)?, c));
        }
        list.sort_by(|a, b| b.0.cmp(&a.0));
        for (_, c) in &list {
            let mut ps = eng.commit(*c)?.parents.clone();
            if rev.first_parent {
                ps.truncate(1);
            }
            for pp in ps {
                children.entry(pp).or_default().push(*c);
            }
        }
        eng.children = Some(children);
        eng.uninteresting.clear();
        eng.reverse_bottom = Some(bottom);
        eng.first_parent = false;
        let blob = eng
            .blob_in(bottom, &path)?
            .ok_or_else(|| CliError::fatal(format!("no such path {path} in {}", revs.join(" "))))?;
        final_origin = eng.get_origin(bottom, &path, blob)?;
    } else if positive.is_empty() || contents.is_some() {
        // Working tree (or --contents) on top of HEAD / the given revision.
        let parent = match positive.first() {
            Some((_, o)) => Some(*o),
            None => repo.head_oid().ok(),
        };
        let content = match &contents {
            Some(f) if f == "-" => {
                let mut b = Vec::new();
                std::io::Read::read_to_end(&mut std::io::stdin(), &mut b)?;
                b
            }
            Some(f) => std::fs::read(f)
                .map_err(|_| CliError::fatal(format!("cannot open or read '{f}'")))?,
            None => std::fs::read(work.join(&path))
                .map_err(|_| CliError::fatal(format!("no such path '{path}' in HEAD")))?,
        };
        if contents.is_none() {
            let in_head = match parent {
                Some(h) => eng.blob_in(h, &path)?.is_some(),
                None => false,
            };
            if !in_head
                && !crate::util::read_index(&repo)?
                    .entries
                    .iter()
                    .any(|e| e.path == path)
            {
                return Err(CliError::fatal(format!("no such path '{path}' in HEAD")));
            }
        }
        let hexlen = parent.map(|o| o.to_hex().len()).unwrap_or(40);
        let fake =
            Oid::from_hex(&"0".repeat(hexlen)).map_err(|_| CliError::fatal("bad object format"))?;
        let (who, mail, src) = match &contents {
            Some(f) => ("External file (--contents)", "external.file", f.clone()),
            None => ("Not Committed Yet", "not.committed.yet", path.clone()),
        };
        let sig = Signature {
            name: who.into(),
            email: mail.into(),
            timestamp: now,
            timezone: "+0000".into(),
        };
        let tree = parent
            .and_then(|h| read_commit(&repo, &h).ok())
            .map(|c| c.tree)
            .unwrap_or(fake);
        eng.commits.insert(
            fake,
            Commit {
                tree,
                parents: parent.into_iter().collect(),
                author: sig.clone(),
                committer: sig,
                gpgsig: None,
                message: format!("Version of {path} from {src}\n"),
            },
        );
        eng.fake = Some(fake);
        fake_label = Some((who.into(), mail.into(), src));
        eng.origins.push(Origin {
            commit: fake,
            path: path.clone(),
            blob: None,
            lines: to_lines(&content),
            previous: None,
        });
        final_origin = eng.origins.len() - 1;
        eng.by_key.insert((fake, path.clone()), final_origin);
    } else {
        let (name, oid) = positive[0].clone();
        let blob = eng
            .blob_in(oid, &path)?
            .ok_or_else(|| CliError::fatal(format!("no such path {path} in {name}")))?;
        final_origin = eng.get_origin(oid, &path, blob)?;
    }
    let _ = fake_label;

    let final_lines: Vec<Vec<u8>> = eng.origins[final_origin].lines.clone();
    let total = final_lines.len();
    let plain: Vec<&[u8]> = final_lines
        .iter()
        .map(|l| l.strip_suffix(b"\n").unwrap_or(l))
        .collect();
    let spans = if ranges.is_empty() {
        vec![(1, total)]
    } else {
        crate::cmd::line_range::parse_all(&ranges, &plain, &path)
            .map_err(|e| e.into_cli(USAGE.trim_end()))?
    };
    let mut start_entries = Vec::new();
    for (b, e) in spans {
        if total == 0 || b > e {
            continue;
        }
        start_entries.push(Entry {
            lno: b - 1,
            s_lno: b - 1,
            num: e - b + 1,
            origin: final_origin,
            ignored: false,
        });
    }

    let mailmap = crate::cmd::mailmap::Mailmap::load(Some(&work), &cfg);
    let mut out = std::io::BufWriter::new(sink);

    // The walk: newest commit first; ties in insertion order.
    let mut pending: BTreeMap<usize, Vec<Entry>> = BTreeMap::new();
    let mut queue: Vec<(i64, u64, Oid)> = Vec::new();
    let mut seq: u64 = 0;
    let mut queued: HashSet<Oid> = HashSet::new();
    let mut guilty: Vec<Entry> = Vec::new();
    let mut emitted_info: HashSet<Oid> = HashSet::new();
    let fc = eng.origins[final_origin].commit;
    if !start_entries.is_empty() {
        pending.insert(final_origin, start_entries);
        let d = eng.date(fc)?;
        queue.push((d, seq, fc));
        queued.insert(fc);
        seq += 1;
    }
    while !queue.is_empty() {
        let best = (0..queue.len())
            .max_by(|&a, &b| {
                queue[a]
                    .0
                    .cmp(&queue[b].0)
                    .then(queue[b].1.cmp(&queue[a].1))
            })
            .expect("non-empty");
        let (_, _, c) = queue.remove(best);
        queued.remove(&c);
        let mine: Vec<usize> = pending
            .keys()
            .copied()
            .filter(|o| eng.origins[*o].commit == c)
            .collect();
        for o in mine {
            let Some(entries) = pending.remove(&o) else {
                continue;
            };
            let (passed, kept) = eng.pass_blame(o, entries)?;
            for e in passed {
                let pc = eng.origins[e.origin].commit;
                pending.entry(e.origin).or_default().push(e);
                if !queued.contains(&pc) {
                    let d = eng.date(pc)?;
                    queue.push((d, seq, pc));
                    seq += 1;
                    queued.insert(pc);
                }
            }
            let mut kept = kept;
            kept.sort_by_key(|e| e.lno);
            if st.incremental {
                for e in &kept {
                    let oc = eng.origins[e.origin].commit;
                    let hex = oc.to_hex();
                    writeln!(out, "{hex} {} {} {}", e.s_lno + 1, e.lno + 1, e.num)?;
                    if emitted_info.insert(oc) {
                        write_info(&mut out, &eng, oc, &mailmap)?;
                    }
                    write_filename_info(&mut out, &eng, e.origin)?;
                }
            }
            guilty.extend(kept);
        }
    }
    if st.incremental {
        out.flush()?;
        return Ok(());
    }

    // Coalesce and output.
    guilty.sort_by_key(|e| e.lno);
    let mut ents: Vec<Entry> = Vec::new();
    for e in guilty {
        if let Some(l) = ents.last_mut() {
            if l.origin == e.origin
                && l.ignored == e.ignored
                && l.lno + l.num == e.lno
                && l.s_lno + l.num == e.s_lno
            {
                l.num += e.num;
                continue;
            }
        }
        ents.push(e);
    }
    let mut paths_per_commit: HashMap<Oid, HashSet<String>> = HashMap::new();
    for e in &ents {
        let o = &eng.origins[e.origin];
        paths_per_commit
            .entry(o.commit)
            .or_default()
            .insert(o.path.clone());
    }
    for (c, ps) in &paths_per_commit {
        if ps.len() > 1 {
            eng.multi_path.insert(*c);
        }
    }

    if st.porcelain {
        let mut seen: HashSet<Oid> = HashSet::new();
        for e in &ents {
            let oc = eng.origins[e.origin].commit;
            let hex = oc.to_hex();
            for k in 0..e.num {
                if k == 0 {
                    writeln!(out, "{hex} {} {} {}", e.s_lno + 1, e.lno + 1, e.num)?;
                } else {
                    writeln!(out, "{hex} {} {}", e.s_lno + k + 1, e.lno + k + 1)?;
                }
                let first = seen.insert(oc);
                if st.line_porcelain || (k == 0 && first) {
                    write_info(&mut out, &eng, oc, &mailmap)?;
                    write_filename_info(&mut out, &eng, e.origin)?;
                } else if k == 0 && eng.multi_path.contains(&oc) {
                    write_filename_info(&mut out, &eng, e.origin)?;
                }
                out.write_all(b"\t")?;
                out.write_all(plain[e.lno + k])?;
                out.write_all(b"\n")?;
            }
        }
        if st.show_stats {
            write_stats(&mut out, &eng)?;
        }
        out.flush()?;
        return Ok(());
    }

    // Column widths.
    let hexlen = eng.origins[final_origin].commit.to_hex().len();
    let mut abbrev = match st.abbrev {
        Some(usize::MAX) => hexlen,
        Some(n) => n.max(4),
        None => 7,
    };
    if !st.long_names && abbrev < hexlen {
        let mut need = abbrev;
        for e in &ents {
            let oc = eng.origins[e.origin].commit;
            if Some(oc) == eng.fake {
                continue;
            }
            need = need.max(abbrev_hex(&repo, &oc, abbrev).len());
        }
        abbrev = need;
    }
    let length = if st.long_names {
        hexlen
    } else {
        (abbrev + 1).min(hexlen)
    };
    let show_name = st
        .show_name
        .unwrap_or_else(|| ents.iter().any(|e| eng.origins[e.origin].path != path));
    let author_of = |eng: &mut Engine<'_>, oc: Oid| -> CliResult<Signature> {
        let a = eng.commit(oc)?.author.clone();
        if Some(oc) == eng.fake {
            return Ok(a);
        }
        let (name, email) = mailmap.map(&a.name, &a.email);
        Ok(Signature { name, email, ..a })
    };
    let mut longest_author = 0;
    let mut longest_file = 0;
    let mut max_orig = 0;
    let mut max_final = 0;
    let mut max_score = 0;
    for e in &ents {
        let oc = eng.origins[e.origin].commit;
        let a = author_of(&mut eng, oc)?;
        let w = if st.email {
            a.email.chars().count() + 2
        } else {
            a.name.chars().count()
        };
        longest_author = longest_author.max(w);
        longest_file = longest_file.max(eng.origins[e.origin].path.chars().count());
        max_orig = max_orig.max(e.s_lno + e.num);
        max_final = max_final.max(e.lno + e.num);
        let text: Vec<Vec<u8>> = final_lines[e.lno..e.lno + e.num].to_vec();
        max_score = max_score.max(alnum_score(&text));
    }
    // refcnt for --score-debug: entries plus live origins that point back.
    let mut alive: HashSet<usize> = ents.iter().map(|e| e.origin).collect();
    let mut stack: Vec<usize> = alive.iter().copied().collect();
    while let Some(x) = stack.pop() {
        if let Some(pv) = eng.origins[x].previous {
            if alive.insert(pv) {
                stack.push(pv);
            }
        }
    }
    let mut refcnts: HashMap<usize, usize> = HashMap::new();
    for e in &ents {
        *refcnts.entry(e.origin).or_default() += 1;
    }
    for x in &alive {
        if let Some(pv) = eng.origins[*x].previous {
            *refcnts.entry(pv).or_default() += 1;
        }
    }
    let dw = date_width(&st.date);
    let age_colors = if st.color_by_age {
        Some(heat_map(&cfg, now))
    } else {
        None
    };
    for e in &ents {
        let oc = eng.origins[e.origin].commit;
        let hex = oc.to_hex();
        let boundary = eng.boundary.contains(&oc);
        let a = author_of(&mut eng, oc)?;
        let time = {
            let mut t = if st.raw_time {
                format!("{} {}", a.timestamp, a.timezone)
            } else {
                crate::cmd::date_spec::format(&st.date, &a)?
            };
            if !st.raw_time {
                let w = t.chars().count();
                if w < dw {
                    t.push_str(&" ".repeat(dw - w));
                }
            }
            t
        };
        let score = alnum_score(&final_lines[e.lno..e.lno + e.num]);
        let rc = refcnts.get(&e.origin).copied().unwrap_or(0);
        for k in 0..e.num {
            let lno = e.lno + k;
            let s_lno = e.s_lno + k;
            let mut meta = String::new();
            let mut len = length;
            if boundary {
                if st.blank_boundary {
                    meta.push_str(&" ".repeat(len));
                    len = 0;
                } else if !st.compat {
                    meta.push('^');
                    len -= 1;
                }
            }
            if len > 0 {
                meta.push_str(&hex[..len.min(hex.len())]);
            }
            let who = if st.email {
                format!("<{}>", a.email)
            } else {
                a.name.clone()
            };
            if st.compat {
                out.write_all(meta.as_bytes())?;
                write!(out, "\t({who:>10}\t{time:>10}\t{})", lno + 1)?;
                out.write_all(plain[lno])?;
                out.write_all(b"\n")?;
                continue;
            }
            if st.score_debug {
                meta.push_str(&format!(" {score:>w$} {rc:02}", w = digits(max_score)));
            }
            if show_name {
                meta.push_str(&format!(
                    " {:<w$}",
                    eng.origins[e.origin].path,
                    w = longest_file
                ));
            }
            if st.show_number {
                meta.push_str(&format!(" {:>w$}", s_lno + 1, w = digits(max_orig)));
            }
            if !st.suppress {
                let pad = longest_author.saturating_sub(who.chars().count());
                meta.push_str(&format!(" ({who}{} {time:>10}", " ".repeat(pad)));
            }
            meta.push_str(&format!(" {:>w$}) ", lno + 1, w = digits(max_final)));
            let color = match &age_colors {
                Some(h) => Some(heat_color(h, a.timestamp)),
                None if st.color_lines && k > 0 => Some(
                    cfg.get("color.blame.repeatedLines")
                        .and_then(|c| crate::cmd::config::parse_color(&c))
                        .unwrap_or_else(|| "\x1b[36m".into()),
                ),
                None => None,
            };
            match color {
                Some(c) => {
                    out.write_all(c.as_bytes())?;
                    out.write_all(meta.as_bytes())?;
                    out.write_all(b"\x1b[m")?;
                }
                None => out.write_all(meta.as_bytes())?,
            }
            out.write_all(plain[lno])?;
            out.write_all(b"\n")?;
        }
    }
    if st.show_stats {
        write_stats(&mut out, &eng)?;
    }
    out.flush()?;
    Ok(())
}

/// `color.blame.highlightRecent`: (color, newer-than threshold) steps.
fn heat_map(cfg: &arks_config::ConfigSet, now: i64) -> Vec<(String, i64)> {
    let spec = cfg
        .get("color.blame.highlightRecent")
        .unwrap_or_else(|| "blue,12 month ago,white,1 month ago,red".into());
    let mut steps: Vec<(String, i64)> = Vec::new();
    let mut color = String::new();
    for part in spec.split(',') {
        let part = part.trim();
        if let Some(c) = crate::cmd::config::parse_color(part).filter(|c| !c.is_empty()) {
            color = c;
            continue;
        }
        let words: Vec<&str> = part.split_whitespace().collect();
        if let [n, unit, ..] = words.as_slice() {
            let n: i64 = n.parse().unwrap_or(0);
            let secs = match unit.trim_end_matches('s') {
                "day" => 86_400,
                "week" => 7 * 86_400,
                "month" => 2_629_746,
                "year" => 31_556_952,
                _ => 0,
            };
            steps.push((color.clone(), now - n * secs));
        }
    }
    steps.push((color, i64::MAX));
    steps
}

fn heat_color(steps: &[(String, i64)], ts: i64) -> String {
    steps
        .iter()
        .find(|(_, limit)| ts < *limit)
        .map(|(c, _)| c.clone())
        .unwrap_or_default()
}

fn write_stats(out: &mut impl Write, eng: &Engine<'_>) -> CliResult<()> {
    writeln!(out, "num read blob: {}", 1 + 2 * eng.patches)?;
    writeln!(out, "num get patch: {}", eng.patches)?;
    writeln!(out, "num commits: {}", eng.patches)?;
    Ok(())
}

fn write_info(
    out: &mut impl Write,
    eng: &Engine<'_>,
    oc: Oid,
    mailmap: &crate::cmd::mailmap::Mailmap,
) -> CliResult<()> {
    let c = &eng.commits[&oc];
    let fake = Some(oc) == eng.fake;
    let map = |s: &Signature| -> (String, String) {
        if fake {
            (s.name.clone(), s.email.clone())
        } else {
            mailmap.map(&s.name, &s.email)
        }
    };
    let (an, ae) = map(&c.author);
    let (cn, ce) = map(&c.committer);
    writeln!(out, "author {an}")?;
    writeln!(out, "author-mail <{ae}>")?;
    writeln!(out, "author-time {}", c.author.timestamp)?;
    writeln!(out, "author-tz {}", c.author.timezone)?;
    writeln!(out, "committer {cn}")?;
    writeln!(out, "committer-mail <{ce}>")?;
    writeln!(out, "committer-time {}", c.committer.timestamp)?;
    writeln!(out, "committer-tz {}", c.committer.timezone)?;
    writeln!(out, "summary {}", subject_line(&c.message))?;
    if eng.boundary.contains(&oc) {
        writeln!(out, "boundary")?;
    }
    Ok(())
}

fn write_filename_info(out: &mut impl Write, eng: &Engine<'_>, origin: usize) -> CliResult<()> {
    let o = &eng.origins[origin];
    if let Some(pv) = o.previous {
        let p = &eng.origins[pv];
        writeln!(out, "previous {} {}", p.commit.to_hex(), p.path)?;
    }
    writeln!(out, "filename {}", o.path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn date_widths() {
        assert_eq!(date_width("iso"), 25);
        assert_eq!(date_width("relative"), 22);
        assert_eq!(date_width("format:%Y"), 4);
        assert_eq!(date_width("local"), 30);
    }

    #[test]
    fn merge_runs() {
        let e = |lno, s_lno, num, origin| Entry {
            lno,
            s_lno,
            num,
            origin,
            ignored: false,
        };
        let m = merge_adjacent(vec![e(2, 2, 1, 0), e(0, 0, 2, 0), e(3, 9, 1, 0)]);
        assert_eq!(m.len(), 2);
        assert_eq!(m[0].num, 3);
    }

    #[test]
    fn scores() {
        assert_eq!(alnum_score(&[b"alpha\n".to_vec(), b"beta\n".to_vec()]), 10);
    }
}
