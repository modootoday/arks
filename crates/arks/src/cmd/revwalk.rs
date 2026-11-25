//! Revision options and the commit walk shared by `rev-list`, `log`,
//! `show`, `whatchanged` and `shortlog`.
//!
//! Covers commit limiting (`-n`, `--skip`, dates, `--author`,
//! `--committer`, `--grep` with `--all-match` / `--invert-grep` / `-i` /
//! `-E` / `-F` / `-P`, parent counts, `--first-parent`, ref selection
//! `--all` / `--branches` / `--tags` / `--remotes` / `--glob` /
//! `--exclude` / `--not` / `--stdin`), symmetric-range sides
//! (`--left-right`, `--left-only`, `--right-only`, `--cherry-pick`,
//! `--cherry-mark`, `--cherry`), `--boundary`, history simplification
//! (`--full-history`, `--sparse`, `--simplify-by-decoration`,
//! `--ancestry-path`, `--remove-empty`), ordering (`--topo-order`,
//! `--date-order`, `--author-date-order`, `--reverse`), `--no-walk` and
//! `--walk-reflogs`.
//!
//! Primary: <https://git-scm.com/docs/git-rev-list>, <https://git-scm.com/docs/git-log>
//! (§Commit Limiting, §History Simplification, §Commit Ordering),
//! <https://git-scm.com/docs/gitrevisions>.

use crate::cmd::bre::Bre;
use crate::cmd::diffcore::options::Args;
use crate::cmd::log_format::{ChildFirst, path_in_specs, read_commit};
use crate::cmd::shared::{peel_tags, resolve_rev};
use crate::error::{CliError, CliResult};
use arks_object::{Commit, ObjectKind};
use arks_oid::Oid;
use arks_repo::Repository;
use std::collections::{BinaryHeap, HashMap, HashSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum RegexMode {
    #[default]
    Basic,
    Extended,
    Fixed,
    Perl,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Order {
    Topo,
    Date,
    AuthorDate,
}

/// A ref-selection request pending `--exclude` patterns.
#[derive(Clone, Debug)]
enum RefSel {
    All,
    Prefix(&'static str, Option<String>),
    Glob(String),
}

#[derive(Clone, Debug, Default)]
pub struct RevOpts {
    /// Revision arguments in order; a leading `^` excludes.
    pub revs: Vec<String>,
    negate: bool,
    pending_excludes: Vec<String>,
    pub max_count: Option<usize>,
    pub skip: usize,
    pub since: Option<i64>,
    pub until: Option<i64>,
    pub since_as_filter: bool,
    pub authors: Vec<String>,
    pub committers: Vec<String>,
    pub greps: Vec<String>,
    pub grep_reflog: Vec<String>,
    pub all_match: bool,
    pub invert_grep: bool,
    pub icase: bool,
    pub regex: RegexMode,
    pub min_parents: Option<usize>,
    pub max_parents: Option<usize>,
    pub first_parent: bool,
    pub exclude_first_parent_only: bool,
    pub left_right: bool,
    pub left_only: bool,
    pub right_only: bool,
    pub cherry_pick: bool,
    pub cherry_mark: bool,
    pub boundary: bool,
    pub order: Option<Order>,
    pub reverse: bool,
    pub full_history: bool,
    pub sparse: bool,
    pub simplify_merges: bool,
    pub simplify_by_decoration: bool,
    pub show_pulls: bool,
    /// Parent rewriting is on (`--parents`, `--children`, `--graph`,
    /// `--simplify-merges`): with `--full-history` merges always stay.
    pub rewrite: bool,
    pub ancestry_path: Option<Vec<String>>,
    pub remove_empty: bool,
    pub no_walk: Option<bool>,
    pub walk_reflogs: bool,
    pub ignore_missing: bool,
    pub stdin: bool,
    pub merge: bool,
    pub reflog: bool,
    pub bisect: bool,
    /// Paths after `--` (worktree-relative).
    pub specs: Vec<String>,
}

fn int(v: &str) -> CliResult<usize> {
    v.parse()
        .map_err(|_| CliError::fatal(format!("'{v}': not an integer")))
}

fn date(v: &str) -> CliResult<i64> {
    if let Ok(n) = v.parse::<i64>() {
        return Ok(n);
    }
    crate::cmd::shared::parse_git_date(v)
        .map(|(t, _)| t)
        .or_else(|| approxidate(v, now()))
        .ok_or_else(|| CliError::fatal(format!("unsupported date: {v}")))
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Days since 1970-01-01 of a proleptic Gregorian date.
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// The human date forms the git documentation shows for `--since` /
/// `--until` beyond strict ISO 8601: a date alone (`2023-11-15`,
/// `2023.11.15`, `2023/11/15`, `11/15/2023`, `Nov 15 2023`), which keeps
/// the current time of day, and relative forms (`3 days ago`,
/// `3.days.ago`, `yesterday`, `now`). Times are UTC.
fn approxidate(v: &str, now: i64) -> Option<i64> {
    let v = v.trim().to_ascii_lowercase();
    let words: Vec<&str> = v.split(['.', ' ', ',']).filter(|w| !w.is_empty()).collect();
    let unit = |w: &str| -> Option<i64> {
        let w = w.strip_suffix('s').unwrap_or(w);
        Some(match w {
            "second" | "sec" => 1,
            "minute" | "min" => 60,
            "hour" => 3600,
            "day" => 86_400,
            "week" => 7 * 86_400,
            "month" => 30 * 86_400,
            "year" => 365 * 86_400,
            _ => return None,
        })
    };
    match words.as_slice() {
        ["now"] | ["today"] => return Some(now),
        ["yesterday"] => return Some(now - 86_400),
        [n, u, "ago"] => return Some(now - n.parse::<i64>().ok()? * unit(u)?),
        [u, "ago"] => return Some(now - unit(u)?),
        _ => {}
    }
    const MONTHS: [&str; 12] = [
        "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
    ];
    let month = |w: &str| {
        MONTHS
            .iter()
            .position(|m| w.starts_with(m))
            .map(|i| i as i64 + 1)
    };
    let num = |w: &str| w.parse::<i64>().ok();
    let (y, m, d) = match words.as_slice() {
        [one] => {
            let parts: Vec<i64> = one.split(['-', '/']).map(num).collect::<Option<Vec<_>>>()?;
            match parts.as_slice() {
                [y, m, d] if *y > 31 => (*y, *m, *d),
                [m, d, y] if *y > 31 => (*y, *m, *d),
                _ => return None,
            }
        }
        [y, m, d] if num(y).is_some_and(|y| y > 31) => (num(y)?, num(m)?, num(d)?),
        [mo, d, y] if month(mo).is_some() => (num(y)?, month(mo)?, num(d)?),
        [d, mo, y] if month(mo).is_some() => (num(y)?, month(mo)?, num(d)?),
        _ => return None,
    };
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    Some(days_from_civil(y, m, d) * 86_400 + now.rem_euclid(86_400))
}

impl RevOpts {
    /// Try to consume the revision option at `a.cur()`.
    pub fn take(&mut self, a: &mut Args<'_>) -> CliResult<bool> {
        let cur = a.cur();
        match cur {
            "--all" => self.add_refs(RefSel::All),
            "--branches" => self.add_refs(RefSel::Prefix("refs/heads/", None)),
            "--tags" => self.add_refs(RefSel::Prefix("refs/tags/", None)),
            "--remotes" => self.add_refs(RefSel::Prefix("refs/remotes/", None)),
            "--not" => self.negate = !self.negate,
            "--first-parent" => self.first_parent = true,
            "--exclude-first-parent-only" => self.exclude_first_parent_only = true,
            "--merges" => self.min_parents = Some(2),
            "--no-merges" => self.max_parents = Some(1),
            "--no-min-parents" => self.min_parents = None,
            "--no-max-parents" => self.max_parents = None,
            "--left-right" => self.left_right = true,
            "--left-only" => self.left_only = true,
            "--right-only" => self.right_only = true,
            "--cherry-pick" => self.cherry_pick = true,
            "--cherry-mark" => self.cherry_mark = true,
            "--cherry" => {
                self.right_only = true;
                self.cherry_mark = true;
                self.no_walk = None;
            }
            "--boundary" => self.boundary = true,
            "--topo-order" => self.order = Some(Order::Topo),
            "--date-order" => self.order = Some(Order::Date),
            "--author-date-order" => self.order = Some(Order::AuthorDate),
            "--reverse" => self.reverse = true,
            "--full-history" => self.full_history = true,
            "--dense" => self.sparse = false,
            "--sparse" => self.sparse = true,
            "--simplify-merges" => {
                self.simplify_merges = true;
                self.full_history = true;
                self.rewrite = true;
            }
            // Decorated commits are the "changed" ones; the rest is
            // simplified like --simplify-merges in topological order.
            "--simplify-by-decoration" => {
                self.simplify_by_decoration = true;
                self.simplify_merges = true;
                self.full_history = true;
                self.rewrite = true;
                self.order.get_or_insert(Order::Topo);
            }
            "--show-pulls" => self.show_pulls = true,
            "--ancestry-path" => {
                self.ancestry_path.get_or_insert_with(Vec::new);
            }
            "--remove-empty" => self.remove_empty = true,
            "--no-walk" => self.no_walk = Some(true),
            "--do-walk" => self.no_walk = None,
            "-g" | "--walk-reflogs" => self.walk_reflogs = true,
            "--ignore-missing" => self.ignore_missing = true,
            "--stdin" => self.stdin = true,
            "--merge" => self.merge = true,
            "--reflog" => self.reflog = true,
            "--bisect" => self.bisect = true,
            "--all-match" => self.all_match = true,
            "--no-all-match" => self.all_match = false,
            "--invert-grep" => self.invert_grep = true,
            "--no-invert-grep" => self.invert_grep = false,
            "-i" | "--regexp-ignore-case" => self.icase = true,
            "--basic-regexp" => self.regex = RegexMode::Basic,
            "-E" | "--extended-regexp" => self.regex = RegexMode::Extended,
            "-F" | "--fixed-strings" => self.regex = RegexMode::Fixed,
            "-P" | "--perl-regexp" => self.regex = RegexMode::Perl,
            "--since-as-filter" => {
                return Err(CliError::fatal(
                    "option '--since-as-filter' requires a value",
                ));
            }
            // Bitmaps, progress and alternates do not change the result.
            "--use-bitmap-index" | "--alternate-refs" | "--single-worktree" => {}
            _ => return self.take_valued(a),
        }
        Ok(true)
    }

    fn take_valued(&mut self, a: &mut Args<'_>) -> CliResult<bool> {
        let cur = a.cur();
        if let Some(v) = a.long_req("--max-count")?.or(a.short_req('n')?) {
            self.max_count = Some(int(&v)?);
        } else if cur.len() > 1
            && cur.starts_with('-')
            && !cur.starts_with("--")
            && cur[1..].bytes().all(|b| b.is_ascii_digit())
        {
            self.max_count = Some(int(&cur[1..])?);
        } else if let Some(v) = a.long_req("--skip")? {
            self.skip = int(&v)?;
        } else if let Some(v) = a
            .long_req("--since")?
            .or(a.long_req("--after")?)
            .or(a.long_req("--max-age")?)
        {
            self.since = Some(date(&v)?);
        } else if let Some(v) = a.long_req("--since-as-filter")? {
            self.since = Some(date(&v)?);
            self.since_as_filter = true;
        } else if let Some(v) = a
            .long_req("--until")?
            .or(a.long_req("--before")?)
            .or(a.long_req("--min-age")?)
        {
            self.until = Some(date(&v)?);
        } else if let Some(v) = a.long_req("--author")? {
            self.authors.push(v);
        } else if let Some(v) = a.long_req("--committer")? {
            self.committers.push(v);
        } else if let Some(v) = a.long_req("--grep-reflog")? {
            self.grep_reflog.push(v);
        } else if let Some(v) = a.long_req("--grep")? {
            self.greps.push(v);
        } else if let Some(v) = a.long_req("--min-parents")? {
            self.min_parents = Some(int(&v)?);
        } else if let Some(v) = a.long_req("--max-parents")? {
            let n: i64 = v
                .parse()
                .map_err(|_| CliError::fatal(format!("'{v}': not an integer")))?;
            self.max_parents = if n < 0 { None } else { Some(n as usize) };
        } else if let Some(v) = a.long_opt("--branches") {
            self.add_refs(RefSel::Prefix("refs/heads/", v));
        } else if let Some(v) = a.long_opt("--tags") {
            self.add_refs(RefSel::Prefix("refs/tags/", v));
        } else if let Some(v) = a.long_opt("--remotes") {
            self.add_refs(RefSel::Prefix("refs/remotes/", v));
        } else if let Some(v) = a.long_req("--glob")? {
            self.add_refs(RefSel::Glob(v));
        } else if let Some(v) = a.long_req("--exclude")? {
            self.pending_excludes.push(v);
        } else if let Some(v) = a.long_req("--exclude-hidden")? {
            if !matches!(v.as_str(), "fetch" | "receive" | "uploadpack") {
                return Err(CliError::fatal(format!(
                    "unsupported section for hidden refs: {v}"
                )));
            }
        } else if let Some(v) = a.long_opt("--ancestry-path") {
            let l = self.ancestry_path.get_or_insert_with(Vec::new);
            if let Some(v) = v {
                l.push(v);
            }
        } else if let Some(v) = a.long_opt("--no-walk") {
            self.no_walk = Some(v.as_deref() != Some("unsorted"));
        } else {
            return Ok(false);
        }
        Ok(true)
    }

    /// A positional revision argument (honours `--not`).
    pub fn push_rev(&mut self, r: &str) {
        if self.negate {
            if let Some(x) = r.strip_prefix('^') {
                self.revs.push(x.to_string());
            } else if r.contains("..") {
                self.revs.push(r.to_string());
            } else {
                self.revs.push(format!("^{r}"));
            }
        } else {
            self.revs.push(r.to_string());
        }
    }

    fn add_refs(&mut self, sel: RefSel) {
        let ex = std::mem::take(&mut self.pending_excludes);
        let neg = if self.negate { "^" } else { "" };
        // Encoded as a pseudo revision resolved later.
        let code = match sel {
            RefSel::All => "all".to_string(),
            RefSel::Prefix(p, pat) => format!("prefix\u{1}{p}\u{1}{}", pat.unwrap_or_default()),
            RefSel::Glob(g) => format!("glob\u{1}{g}"),
        };
        self.revs
            .push(format!("{neg}\u{0}{code}\u{2}{}", ex.join("\u{3}")));
    }

    /// Any argument requested a pseudo ref set (`--all` and friends).
    pub fn has_ref_selection(&self) -> bool {
        self.revs
            .iter()
            .any(|r| r.trim_start_matches('^').starts_with('\u{0}'))
    }
}

fn glob_match(pat: &str, s: &str) -> bool {
    crate::cmd::branch_cmd::fnmatch(pat.as_bytes(), s.as_bytes())
}

/// Ref names selected by a pseudo revision.
pub(crate) fn select_refs(repo: &Repository, code: &str) -> CliResult<Vec<String>> {
    let (sel, ex) = code.split_once('\u{2}').unwrap_or((code, ""));
    let excludes: Vec<&str> = ex.split('\u{3}').filter(|s| !s.is_empty()).collect();
    let mut refs: Vec<String> = repo
        .refs()
        .list_refs("refs/")?
        .into_iter()
        .map(|(n, _)| n)
        .collect();
    refs.sort();
    let mut out: Vec<String> = match sel.split('\u{1}').collect::<Vec<_>>().as_slice() {
        ["all"] => {
            let mut v = refs.clone();
            if repo.head_oid().is_ok() {
                v.push("HEAD".into());
            }
            v
        }
        ["prefix", p, pat] => refs
            .iter()
            .filter(|r| r.starts_with(p))
            .filter(|r| {
                if pat.is_empty() {
                    return true;
                }
                let rest = &r[p.len()..];
                let pat = if pat.contains(['*', '?', '[']) {
                    pat.to_string()
                } else {
                    format!("{pat}/*")
                };
                glob_match(&pat, rest) || rest == pat.trim_end_matches("/*")
            })
            .cloned()
            .collect(),
        ["glob", g] => {
            let g = if g.starts_with("refs/") {
                g.to_string()
            } else {
                format!("refs/{g}")
            };
            let g = if g.contains(['*', '?', '[']) {
                g
            } else {
                format!("{g}/*")
            };
            refs.iter().filter(|r| glob_match(&g, r)).cloned().collect()
        }
        _ => Vec::new(),
    };
    out.retain(|r| {
        !excludes.iter().any(|e| {
            let full = if e.starts_with("refs/") {
                e.to_string()
            } else {
                format!("refs/*/{e}")
            };
            glob_match(e, r)
                || glob_match(&full, r)
                || glob_match(e, r.rsplit('/').next().unwrap_or(r))
        })
    });
    Ok(out)
}

/// One commit of the walk result.
#[derive(Clone)]
pub struct Walked {
    pub oid: Oid,
    pub commit: Commit,
    /// `<` / `>` side for symmetric ranges (with `--left-right`).
    pub side: Option<char>,
    /// `--boundary` commit.
    pub boundary: bool,
    /// `--cherry-mark`: `=` equivalent on the other side, `+` not.
    pub cherry: Option<char>,
    /// Parents as rewritten for display (`--parents`, graphs).
    pub parents: Vec<Oid>,
    /// `-g`: reflog selector and message.
    pub reflog: Option<(String, String)>,
}

/// Resolved tips.
pub struct Tips {
    pub include: Vec<Oid>,
    pub exclude: Vec<Oid>,
    /// Commits reachable from the left side of a symmetric range.
    pub left: Option<HashSet<Oid>>,
    pub symmetric: bool,
    /// Every tip in argument order (for `--no-walk`).
    pub given: Vec<Oid>,
    /// The include tips' ref names (for `--source`).
    pub names: HashMap<Oid, String>,
}

fn peel_commit(repo: &Repository, oid: &Oid) -> CliResult<Option<Oid>> {
    let p = peel_tags(repo, oid)?;
    Ok((repo.get_object(&p)?.kind == ObjectKind::Commit).then_some(p))
}

/// Resolve revision arguments (and `--stdin` lines) into tips. With no
/// positive revision, `default` (normally `HEAD`) is used.
pub fn resolve_tips(repo: &Repository, o: &RevOpts, default: Option<&str>) -> CliResult<Tips> {
    let mut revs = o.revs.clone();
    if o.stdin {
        use std::io::BufRead;
        let mut not = false;
        for line in std::io::stdin().lock().lines() {
            let line = line?;
            let l = line.trim();
            if l.is_empty() {
                continue;
            }
            if l == "--not" {
                not = !not;
                continue;
            }
            if l == "--" {
                break;
            }
            if not {
                revs.push(format!("^{l}"));
            } else {
                revs.push(l.to_string());
            }
        }
    }
    let mut t = Tips {
        include: Vec::new(),
        exclude: Vec::new(),
        left: None,
        symmetric: false,
        given: Vec::new(),
        names: HashMap::new(),
    };
    let side =
        |s: &str| -> CliResult<Oid> { resolve_rev(repo, if s.is_empty() { "HEAD" } else { s }) };
    let mut left_tips = Vec::new();
    let mut any_positive = false;
    for r in &revs {
        let (neg, body) = match r.strip_prefix('^') {
            Some(b) => (true, b),
            None => (false, r.as_str()),
        };
        if let Some(code) = body.strip_prefix('\u{0}') {
            for name in select_refs(repo, code)? {
                let oid = repo
                    .refs()
                    .resolve(&name)
                    .ok()
                    .or_else(|| (name == "HEAD").then(|| repo.head_oid().ok()).flatten());
                let Some(oid) = oid else { continue };
                let Some(c) = peel_commit(repo, &oid)? else {
                    continue;
                };
                if neg {
                    t.exclude.push(c);
                } else {
                    any_positive = true;
                    t.include.push(c);
                    t.given.push(c);
                    t.names.entry(c).or_insert(name);
                }
            }
            continue;
        }
        let res: CliResult<()> = (|| {
            if let Some((l, rr)) = body.split_once("...") {
                let (a, b) = (side(l)?, side(rr)?);
                let (a, b) = (
                    peel_commit(repo, &a)?.unwrap_or(a),
                    peel_commit(repo, &b)?.unwrap_or(b),
                );
                any_positive = true;
                t.include.push(a);
                t.include.push(b);
                t.given.push(a);
                t.given.push(b);
                t.names.entry(a).or_insert_with(|| {
                    if l.is_empty() {
                        "HEAD".into()
                    } else {
                        l.to_string()
                    }
                });
                t.names.entry(b).or_insert_with(|| {
                    if rr.is_empty() {
                        "HEAD".into()
                    } else {
                        rr.to_string()
                    }
                });
                for base in merge_bases(repo, &a, &b)? {
                    t.exclude.push(base);
                }
                left_tips.push(a);
                t.symmetric = true;
            } else if let Some((l, rr)) = body.split_once("..") {
                let (a, b) = (side(l)?, side(rr)?);
                let a = peel_commit(repo, &a)?.unwrap_or(a);
                let b = peel_commit(repo, &b)?.unwrap_or(b);
                any_positive = true;
                t.exclude.push(a);
                t.include.push(b);
                t.given.push(b);
                t.names.entry(b).or_insert_with(|| {
                    if rr.is_empty() {
                        "HEAD".into()
                    } else {
                        rr.to_string()
                    }
                });
            } else if let Some(base) = body.strip_suffix("^@") {
                let c = peel_commit(repo, &side(base)?)?
                    .ok_or_else(|| CliError::fatal(format!("{body}: not a commit")))?;
                for p in read_commit(repo, &c)?.parents {
                    if neg {
                        t.exclude.push(p);
                    } else {
                        any_positive = true;
                        t.include.push(p);
                        t.given.push(p);
                    }
                }
            } else if let Some(base) = body.strip_suffix("^!") {
                let c = peel_commit(repo, &side(base)?)?
                    .ok_or_else(|| CliError::fatal(format!("{body}: not a commit")))?;
                any_positive = true;
                t.include.push(c);
                t.given.push(c);
                t.exclude.extend(read_commit(repo, &c)?.parents);
            } else {
                let oid = side(body)?;
                let Some(c) = peel_commit(repo, &oid)? else {
                    if neg {
                        return Ok(());
                    }
                    t.given.push(oid);
                    return Ok(());
                };
                if neg {
                    t.exclude.push(c);
                } else {
                    any_positive = true;
                    t.include.push(c);
                    t.given.push(c);
                    t.names.entry(c).or_insert_with(|| body.to_string());
                }
            }
            Ok(())
        })();
        if let Err(e) = res {
            if o.ignore_missing {
                continue;
            }
            return Err(e);
        }
    }
    if o.merge {
        let head = resolve_rev(repo, "HEAD")?;
        let mh = resolve_rev(repo, "MERGE_HEAD")
            .map_err(|_| CliError::fatal("--merge without MERGE_HEAD?"))?;
        t.include.push(head);
        t.include.push(mh);
        for b in merge_bases(repo, &head, &mh)? {
            t.exclude.push(b);
        }
        any_positive = true;
    }
    if !any_positive && !o.has_ref_selection() {
        if let Some(d) = default {
            if let Ok(h) = resolve_rev(repo, d) {
                if let Some(c) = peel_commit(repo, &h)? {
                    t.include.push(c);
                    t.given.push(c);
                    t.names.entry(c).or_insert_with(|| d.to_string());
                }
            }
        }
    }
    if t.symmetric {
        let mut l = HashSet::new();
        for x in left_tips {
            l.extend(arks_repo::ancestor_set(repo, x)?);
        }
        t.left = Some(l);
    }
    Ok(t)
}

/// Every best common ancestor of `a` and `b`.
pub fn merge_bases(repo: &Repository, a: &Oid, b: &Oid) -> CliResult<Vec<Oid>> {
    let aa = arks_repo::ancestor_set(repo, *a)?;
    let bb = arks_repo::ancestor_set(repo, *b)?;
    let common: Vec<Oid> = aa.intersection(&bb).copied().collect();
    let common_set: HashSet<Oid> = common.iter().copied().collect();
    // Drop common ancestors reachable from another common ancestor.
    let mut out = Vec::new();
    for c in &common {
        let mut redundant = false;
        for d in &common {
            if d != c && arks_repo::ancestor_set(repo, *d)?.contains(c) {
                redundant = true;
                break;
            }
        }
        if !redundant {
            out.push(*c);
        }
    }
    let _ = common_set;
    out.sort_by_key(|o| {
        std::cmp::Reverse(
            read_commit(repo, o)
                .map(|c| c.committer.timestamp)
                .unwrap_or(0),
        )
    });
    Ok(out)
}

fn compile(p: &str, o: &RevOpts) -> Bre {
    match o.regex {
        RegexMode::Fixed => {
            let mut esc = String::new();
            for ch in p.chars() {
                if "\\.[]*^$+?|(){}".contains(ch) {
                    esc.push('\\');
                }
                esc.push(ch);
            }
            Bre::with_mode(&esc, o.icase, false)
        }
        RegexMode::Extended | RegexMode::Perl => Bre::with_mode(p, o.icase, true),
        RegexMode::Basic => Bre::with_mode(p, o.icase, false),
    }
}

/// Header / message filters (`--author`, `--committer`, `--grep`).
pub fn message_filter(o: &RevOpts) -> impl Fn(&Commit) -> bool + '_ {
    let authors: Vec<Bre> = o.authors.iter().map(|p| compile(p, o)).collect();
    let committers: Vec<Bre> = o.committers.iter().map(|p| compile(p, o)).collect();
    let greps: Vec<Bre> = o.greps.iter().map(|p| compile(p, o)).collect();
    move |c: &Commit| {
        let who = |s: &arks_object::Signature| format!("{} <{}>", s.name, s.email);
        if !authors.is_empty() {
            let w = who(&c.author);
            if !authors.iter().any(|r| r.is_match(w.as_bytes())) {
                return false;
            }
        }
        if !committers.is_empty() {
            let w = who(&c.committer);
            if !committers.iter().any(|r| r.is_match(w.as_bytes())) {
                return false;
            }
        }
        if greps.is_empty() {
            return true;
        }
        let hit = |r: &Bre| c.message.lines().any(|l| r.is_match(l.as_bytes()));
        let ok = if o.all_match {
            greps.iter().all(hit)
        } else {
            greps.iter().any(hit)
        };
        ok != o.invert_grep
    }
}

/// One walked commit: id, parsed commit, rewritten parents, and whether
/// pathspec simplification kept it (TREESAME commits are dropped).
type RawEntry = (Oid, Commit, Vec<Oid>, bool);

/// Committer-date ordered walk with pathspec simplification. Returns the
/// walked commits with their (possibly simplified) parents.
fn raw_walk(
    repo: &Repository,
    o: &RevOpts,
    include: &[Oid],
    hidden: &HashSet<Oid>,
    stop_at: &HashSet<Oid>,
    decorated: Option<&HashSet<Oid>>,
) -> CliResult<Vec<RawEntry>> {
    let specs = &o.specs;
    let mut tree_cache: HashMap<Oid, HashMap<String, Oid>> = HashMap::new();
    let mut filtered = |tree: Oid| -> CliResult<HashMap<String, Oid>> {
        if let Some(m) = tree_cache.get(&tree) {
            return Ok(m.clone());
        }
        let mut all = HashMap::new();
        arks_repo::flatten_tree(repo, &tree, "", &mut all)?;
        let m: HashMap<String, Oid> = all
            .into_iter()
            .filter(|(k, _)| path_in_specs(k, specs))
            .collect();
        tree_cache.insert(tree, m.clone());
        Ok(m)
    };
    let mut heap: BinaryHeap<(i64, u64)> = BinaryHeap::new();
    let mut by_seq: HashMap<u64, Oid> = HashMap::new();
    let mut seen: HashSet<Oid> = HashSet::new();
    let mut seq = u64::MAX;
    for x in include {
        if seen.insert(*x) {
            let c = read_commit(repo, x)?;
            heap.push((c.committer.timestamp, seq));
            by_seq.insert(seq, *x);
            seq -= 1;
        }
    }
    let mut out = Vec::new();
    while let Some((_, key)) = heap.pop() {
        let oid = by_seq.remove(&key).unwrap_or_else(|| unreachable!());
        if stop_at.contains(&oid) {
            continue;
        }
        let c = read_commit(repo, &oid)?;
        let mut parents: Vec<Oid> = if o.first_parent {
            c.parents.iter().take(1).copied().collect()
        } else {
            c.parents.clone()
        };
        let mut show = true;
        if let (true, Some(d)) = (specs.is_empty(), decorated) {
            // Without paths, an undecorated commit is TREESAME to all
            // parents and a decorated one differs from all of them.
            let merge_kept = parents.len() > 1 && o.rewrite;
            show = d.contains(&oid) || merge_kept || o.sparse;
        } else if !specs.is_empty() {
            let mine = filtered(c.tree)?;
            if parents.is_empty() {
                show = !mine.is_empty();
            } else {
                let mut same = None;
                let mut all_same = true;
                for p in &parents {
                    if filtered(read_commit(repo, p)?.tree)? == mine {
                        same.get_or_insert(*p);
                    } else {
                        all_same = false;
                    }
                }
                let first_same = same.is_some() && same == parents.first().copied();
                if o.full_history {
                    // A merge is hidden only when TREESAME to every parent;
                    // with parent rewriting merges always stay.
                    show = !all_same || o.sparse || (parents.len() > 1 && o.rewrite);
                } else if let Some(p) = same {
                    // `--show-pulls`: a merge that differs from its first
                    // parent but brought the path in from another one.
                    show = o.sparse || (o.show_pulls && parents.len() > 1 && !first_same);
                    parents = vec![p];
                } else if o.remove_empty && mine.is_empty() {
                    parents.clear();
                }
            }
        }
        for p in &parents {
            if seen.insert(*p) {
                let pc = read_commit(repo, p)?;
                heap.push((pc.committer.timestamp, seq));
                by_seq.insert(seq, *p);
                seq -= 1;
            }
        }
        let is_hidden = hidden.contains(&oid);
        out.push((oid, c, parents, show && !is_hidden));
        let _ = is_hidden;
    }
    Ok(out)
}

/// Parent rewriting for pathspec-limited walks: each parent that the
/// simplification hid is replaced by its nearest shown ancestors (an
/// excluded parent stays as it is), duplicates dropped.
fn rewrite_parents(walked: &mut [RawEntry], hidden: &HashSet<Oid>) {
    let info: HashMap<Oid, (Vec<Oid>, bool)> = walked
        .iter()
        .map(|(oid, _, ps, show)| (*oid, (ps.clone(), *show)))
        .collect();
    let mut memo: HashMap<Oid, Vec<Oid>> = HashMap::new();
    // Resolve one commit to the shown commits that stand in for it.
    let resolve = |start: Oid, memo: &mut HashMap<Oid, Vec<Oid>>| -> Vec<Oid> {
        let mut stack = vec![(start, false)];
        while let Some((x, expanded)) = stack.pop() {
            if memo.contains_key(&x) {
                continue;
            }
            let Some((ps, show)) = info.get(&x) else {
                memo.insert(x, vec![x]);
                continue;
            };
            if *show || hidden.contains(&x) {
                memo.insert(x, vec![x]);
                continue;
            }
            if !expanded {
                stack.push((x, true));
                for p in ps {
                    if !memo.contains_key(p) {
                        stack.push((*p, false));
                    }
                }
                continue;
            }
            let mut v: Vec<Oid> = Vec::new();
            for p in ps {
                for r in memo.get(p).cloned().unwrap_or_default() {
                    if !v.contains(&r) {
                        v.push(r);
                    }
                }
            }
            memo.insert(x, v);
        }
        memo.get(&start).cloned().unwrap_or_default()
    };
    for (_, _, ps, show) in walked.iter_mut() {
        if !*show {
            continue;
        }
        let mut v: Vec<Oid> = Vec::new();
        for p in ps.iter() {
            for r in resolve(*p, &mut memo) {
                if !v.contains(&r) {
                    v.push(r);
                }
            }
        }
        *ps = v;
    }
}

/// `--simplify-merges`: oldest first, drop rewritten parents that are
/// ancestors of another parent; a merge left with one parent it is
/// TREESAME to disappears and its children see that parent instead.
fn simplify_merges(
    repo: &Repository,
    specs: &[String],
    decorated: Option<&HashSet<Oid>>,
    walked: &mut [RawEntry],
) -> CliResult<()> {
    let filtered = |tree: &Oid| -> CliResult<HashMap<String, Oid>> {
        let mut all = HashMap::new();
        arks_repo::flatten_tree(repo, tree, "", &mut all)?;
        Ok(all
            .into_iter()
            .filter(|(k, _)| path_in_specs(k, specs))
            .collect())
    };
    let mut replaced: HashMap<Oid, Vec<Oid>> = HashMap::new();
    for i in (0..walked.len()).rev() {
        let (oid, c, ps, show) = &mut walked[i];
        if !*show {
            continue;
        }
        let mut v: Vec<Oid> = Vec::new();
        for p in ps.iter() {
            for r in replaced.get(p).cloned().unwrap_or_else(|| vec![*p]) {
                if !v.contains(&r) {
                    v.push(r);
                }
            }
        }
        if v.len() > 1 {
            let mut keep = Vec::new();
            for p in &v {
                let mut redundant = false;
                for q in &v {
                    if q != p && arks_repo::ancestor_set(repo, *q)?.contains(p) {
                        redundant = true;
                        break;
                    }
                }
                if !redundant {
                    keep.push(*p);
                }
            }
            v = keep;
        }
        if c.parents.len() > 1 && v.len() <= 1 {
            // Compare with the one parent left (none left: the first).
            let base = v.first().copied().unwrap_or(c.parents[0]);
            let same = match decorated {
                Some(d) => !d.contains(oid),
                None => filtered(&c.tree)? == filtered(&read_commit(repo, &base)?.tree)?,
            };
            if same {
                *show = false;
                replaced.insert(*oid, v.clone());
            }
        }
        *ps = v;
    }
    Ok(())
}

/// Patch id of a commit's first-parent diff (for `--cherry-*`).
pub fn patch_id(repo: &Repository, oid: &Oid, c: &Commit) -> CliResult<Option<String>> {
    if c.parents.len() > 1 {
        return Ok(None);
    }
    let parent_tree = match c.parents.first() {
        Some(p) => Some(read_commit(repo, p)?.tree),
        None => None,
    };
    let _ = oid;
    let content = crate::cmd::diffcore::Content::new(repo);
    let pairs = crate::cmd::diffcore::tree_pairs(
        repo,
        parent_tree.as_ref(),
        Some(&c.tree),
        true,
        false,
        &[],
    )?;
    let mut o = crate::cmd::diffcore::options::DiffOptions {
        patch: true,
        ..Default::default()
    };
    o.finish()?;
    let mut buf = Vec::new();
    crate::cmd::diffcore::emit::emit(&mut buf, &content, &o, Default::default(), &pairs)?;
    Ok(
        crate::cmd::patch_id::patch_ids(&buf, crate::cmd::patch_id::Mode::Stable)
            .into_iter()
            .next()
            .map(|(id, _)| id),
    )
}

/// Run the walk: tips, limiting, simplification, ordering and `--boundary`.
pub fn walk(repo: &Repository, o: &RevOpts, tips: &Tips) -> CliResult<Vec<Walked>> {
    if let Some(sorted) = o.no_walk {
        let mut v: Vec<Walked> = Vec::new();
        let mut seen = HashSet::new();
        for g in &tips.given {
            if !seen.insert(*g) || tips.exclude.contains(g) {
                continue;
            }
            let Ok(c) = read_commit(repo, g) else {
                continue;
            };
            v.push(Walked {
                oid: *g,
                parents: c.parents.clone(),
                commit: c,
                side: None,
                boundary: false,
                cherry: None,
                reflog: None,
            });
        }
        if sorted {
            v.sort_by_key(|w| std::cmp::Reverse(w.commit.committer.timestamp));
        }
        let v = v
            .into_iter()
            .skip(o.skip)
            .take(o.max_count.unwrap_or(usize::MAX))
            .collect();
        return Ok(v);
    }
    let mut hidden: HashSet<Oid> = HashSet::new();
    let walk_from = |x: &Oid| -> CliResult<HashSet<Oid>> {
        if o.exclude_first_parent_only {
            let mut s = HashSet::new();
            let mut cur = Some(*x);
            while let Some(c) = cur {
                if !s.insert(c) {
                    break;
                }
                cur = read_commit(repo, &c)?.parents.first().copied();
            }
            Ok(s)
        } else {
            Ok(arks_repo::ancestor_set(repo, *x)?)
        }
    };
    for x in &tips.exclude {
        hidden.extend(walk_from(x)?);
    }
    let decorated: Option<HashSet<Oid>> = if o.simplify_by_decoration {
        let mut s = HashSet::new();
        for (_, oid) in repo.refs().list_refs("refs/")? {
            s.insert(peel_tags(repo, &oid)?);
        }
        if let Ok(h) = repo.head_oid() {
            s.insert(h);
        }
        Some(s)
    } else {
        None
    };
    let stop = HashSet::new();
    let mut walked = raw_walk(repo, o, &tips.include, &hidden, &stop, decorated.as_ref())?;
    if !o.specs.is_empty() || decorated.is_some() {
        rewrite_parents(&mut walked, &hidden);
        if o.simplify_merges {
            let dec = decorated.as_ref().filter(|_| o.specs.is_empty());
            simplify_merges(repo, &o.specs, dec, &mut walked)?;
        }
    }
    let keep = message_filter(o);
    // Ancestry path: only commits that are descendants of the bottoms
    // (or of the given commits) and ancestors of the tips.
    let ancestry: Option<HashSet<Oid>> = match &o.ancestry_path {
        Some(list) => {
            let mut bottoms: Vec<Oid> = tips.exclude.clone();
            for r in list {
                bottoms.push(resolve_rev(repo, r)?);
            }
            let mut set = HashSet::new();
            // Commits whose ancestry contains a bottom.
            for (oid, _, _, _) in &walked {
                let anc = arks_repo::ancestor_set(repo, *oid)?;
                if bottoms.iter().any(|b| anc.contains(b) && b != oid) {
                    set.insert(*oid);
                }
            }
            Some(set)
        }
        None => None,
    };
    let mut list: Vec<Walked> = Vec::new();
    for (oid, c, parents, show) in &walked {
        if !show {
            continue;
        }
        if let Some(t) = o.since {
            if c.committer.timestamp < t {
                continue;
            }
        }
        if o.until.is_some_and(|t| c.committer.timestamp > t) {
            continue;
        }
        let np = c.parents.len();
        if o.min_parents.is_some_and(|m| np < m) || o.max_parents.is_some_and(|m| np > m) {
            continue;
        }
        if !keep(c) {
            continue;
        }
        if ancestry.as_ref().is_some_and(|s| !s.contains(oid)) {
            continue;
        }
        let side = tips
            .left
            .as_ref()
            .map(|l| if l.contains(oid) { '<' } else { '>' });
        list.push(Walked {
            oid: *oid,
            commit: c.clone(),
            side,
            boundary: false,
            cherry: None,
            parents: parents.clone(),
            reflog: None,
        });
    }
    // Symmetric-difference sides.
    if tips.symmetric && (o.cherry_pick || o.cherry_mark) {
        let mut ids: HashMap<Oid, String> = HashMap::new();
        for w in &list {
            if let Some(id) = patch_id(repo, &w.oid, &w.commit)? {
                ids.insert(w.oid, id);
            }
        }
        let left: HashSet<&String> = list
            .iter()
            .filter(|w| w.side == Some('<'))
            .filter_map(|w| ids.get(&w.oid))
            .collect();
        let right: HashSet<&String> = list
            .iter()
            .filter(|w| w.side == Some('>'))
            .filter_map(|w| ids.get(&w.oid))
            .collect();
        let equiv: HashSet<Oid> = list
            .iter()
            .filter(|w| {
                ids.get(&w.oid).is_some_and(|id| {
                    if w.side == Some('<') {
                        right.contains(id)
                    } else {
                        left.contains(id)
                    }
                })
            })
            .map(|w| w.oid)
            .collect();
        if o.cherry_mark {
            for w in &mut list {
                w.cherry = Some(if equiv.contains(&w.oid) { '=' } else { '+' });
            }
        } else {
            list.retain(|w| !equiv.contains(&w.oid));
        }
    }
    if o.left_only {
        list.retain(|w| w.side != Some('>'));
    }
    if o.right_only {
        list.retain(|w| w.side != Some('<'));
    }
    // Ordering.
    let order = o.order;
    if let Some(ord) = order {
        let shown: HashSet<Oid> = list.iter().map(|w| w.oid).collect();
        let by: HashMap<Oid, Walked> = list.iter().map(|w| (w.oid, w.clone())).collect();
        let ids: Vec<Oid> = list.iter().map(|w| w.oid).collect();
        let sorted = crate::cmd::log_format::child_first_order(
            &ids,
            |x| {
                // The sort sees every parent a commit has, even when
                // --first-parent walked only the first one.
                let ps = if o.first_parent {
                    &by[x].commit.parents
                } else {
                    &by[x].parents
                };
                ps.iter().copied().filter(|p| shown.contains(p)).collect()
            },
            |x| match ord {
                Order::AuthorDate => by[x].commit.author.timestamp,
                _ => by[x].commit.committer.timestamp,
            },
            if ord == Order::Topo {
                ChildFirst::Topo
            } else {
                ChildFirst::Date
            },
        );
        let mut by = by;
        list = sorted.into_iter().filter_map(|x| by.remove(&x)).collect();
    }
    let mut list: Vec<Walked> = list
        .into_iter()
        .skip(o.skip)
        .take(o.max_count.unwrap_or(usize::MAX))
        .collect();
    if o.boundary {
        let shown: HashSet<Oid> = list.iter().map(|w| w.oid).collect();
        let mut extra: Vec<Oid> = Vec::new();
        for w in &list {
            for p in &w.parents {
                if !shown.contains(p) && !extra.contains(p) && hidden.contains(p) {
                    extra.push(*p);
                }
            }
        }
        for b in extra {
            let c = read_commit(repo, &b)?;
            list.push(Walked {
                oid: b,
                parents: c.parents.clone(),
                commit: c,
                side: None,
                boundary: true,
                cherry: None,
                reflog: None,
            });
        }
    }
    if o.reverse {
        list.reverse();
    }
    Ok(list)
}

/// `-g` / `--walk-reflogs`: reflog entries of the given refs, newest first.
pub fn walk_reflogs(repo: &Repository, o: &RevOpts) -> CliResult<Vec<Walked>> {
    let names: Vec<String> = if o.revs.is_empty() {
        vec!["HEAD".into()]
    } else {
        o.revs.clone()
    };
    let mut out = Vec::new();
    for n in names {
        let full = if n == "HEAD" || n.starts_with("refs/") {
            n.clone()
        } else if repo.refs().resolve(&format!("refs/heads/{n}")).is_ok() {
            format!("refs/heads/{n}")
        } else {
            n.clone()
        };
        let entries = crate::reflog::read_entries(repo, &full);
        let short = n.clone();
        let keep = message_filter(o);
        for (k, (_, new, msg)) in entries.iter().rev().enumerate() {
            let Ok(c) = read_commit(repo, new) else {
                continue;
            };
            if !keep(&c) {
                continue;
            }
            out.push(Walked {
                oid: *new,
                parents: c.parents.clone(),
                commit: c,
                side: None,
                boundary: false,
                cherry: None,
                reflog: Some((format!("{short}@{{{k}}}"), msg.clone())),
            });
        }
    }
    Ok(out
        .into_iter()
        .skip(o.skip)
        .take(o.max_count.unwrap_or(usize::MAX))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn approxidate_forms() {
        // 2026-11-20 00:58:04 UTC
        let now = 1_795_222_684;
        let tod = now % 86_400;
        let day = days_from_civil(2023, 11, 15) * 86_400;
        assert_eq!(day, 1_700_006_400);
        for v in [
            "2023-11-15",
            "2023.11.15",
            "2023/11/15",
            "11/15/2023",
            "Nov 15 2023",
        ] {
            assert_eq!(approxidate(v, now), Some(day + tod), "{v}");
        }
        assert_eq!(approxidate("3 days ago", now), Some(now - 3 * 86_400));
        assert_eq!(approxidate("3.days.ago", now), Some(now - 3 * 86_400));
        assert_eq!(approxidate("yesterday", now), Some(now - 86_400));
        assert_eq!(approxidate("bogus", now), None);
    }
}
