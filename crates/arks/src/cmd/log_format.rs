//! Revision walk + pretty formats for `log` / `show`.
//!
//! Primary: <https://git-scm.com/docs/git-log> (§Commit Limiting, §PRETTY FORMATS),
//! <https://git-scm.com/docs/gitrevisions> (`A..B`, `A...B`, `^A`).
//! Cite: `.spec/sot/cloud-agent-dropin.sot.md` §O.
//!
//! Default walk order is committer-date descending (git default when no
//! `--topo-order` is given). Partial: `%d`/`%D` decorations, `%ar`/`%cr`
//! relative dates, and pathspec rename following are not implemented.

use crate::cmd::shared::resolve_rev;
use crate::error::{CliError, CliResult};
use arks_object::{Commit, ObjectKind, Signature};
use arks_oid::Oid;
use arks_repo::Repository;
use std::collections::{BinaryHeap, HashSet};

/// Parsed `--format` / `--pretty`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pretty {
    /// No format flag: command default.
    Default,
    Oneline,
    /// `--pretty=format:` — separator semantics (no newline after the last entry).
    Format(String),
    /// `--format=` / `--pretty=tformat:` — terminator semantics.
    TFormat(String),
}

/// Consume `--format=…`, `--pretty=…`, `--pretty` `<x>`, `--oneline` from `args`.
pub fn take_pretty(args: &mut Vec<String>) -> CliResult<Pretty> {
    let mut out = Pretty::Default;
    let mut i = 0;
    while i < args.len() {
        let a = args[i].clone();
        if a == "--" {
            break;
        }
        let spec = if let Some(v) = a.strip_prefix("--format=") {
            Some((v.to_string(), true))
        } else if let Some(v) = a.strip_prefix("--pretty=") {
            Some((v.to_string(), false))
        } else if a == "--format" || a == "--pretty" {
            let v = args
                .get(i + 1)
                .cloned()
                .ok_or_else(|| CliError::Usage(format!("{a} requires a value")))?;
            args.remove(i + 1);
            Some((v, a == "--format"))
        } else if a == "--oneline" {
            out = Pretty::Oneline;
            args.remove(i);
            continue;
        } else {
            None
        };
        let Some((v, is_format_flag)) = spec else {
            i += 1;
            continue;
        };
        args.remove(i);
        out = if let Some(f) = v.strip_prefix("format:") {
            Pretty::Format(f.to_string())
        } else if let Some(f) = v.strip_prefix("tformat:") {
            Pretty::TFormat(f.to_string())
        } else {
            match v.as_str() {
                "oneline" => Pretty::Oneline,
                "medium" | "short" | "full" | "fuller" | "raw" => Pretty::Default,
                _ if v.contains('%') || is_format_flag => Pretty::TFormat(v),
                other => {
                    return Err(CliError::NotImplemented(format!(
                        "unsupported pretty format: {other}"
                    )));
                }
            }
        };
    }
    Ok(out)
}

/// Commits reachable from `include` minus those reachable from `exclude`,
/// committer-date descending. `first_parent` follows only first parents.
pub fn walk(
    repo: &Repository,
    include: &[Oid],
    exclude: &[Oid],
    first_parent: bool,
) -> CliResult<Vec<(Oid, Commit)>> {
    walk_paths(repo, include, exclude, first_parent, &[])
}

/// [`walk`] limited to `specs` with git's default history simplification:
/// a commit TREESAME (on `specs`) to a parent is not shown, and a merge
/// TREESAME to some parent follows only that parent.
/// Cite: <https://git-scm.com/docs/git-log#_history_simplification>.
pub fn walk_paths(
    repo: &Repository,
    include: &[Oid],
    exclude: &[Oid],
    first_parent: bool,
    specs: &[String],
) -> CliResult<Vec<(Oid, Commit)>> {
    let mut tree_cache: std::collections::HashMap<Oid, std::collections::HashMap<String, Oid>> =
        std::collections::HashMap::new();
    let mut filtered = |tree: Oid| -> CliResult<std::collections::HashMap<String, Oid>> {
        if let Some(m) = tree_cache.get(&tree) {
            return Ok(m.clone());
        }
        let m: std::collections::HashMap<String, Oid> = tree_blob_map_of_tree(repo, &tree)?
            .into_iter()
            .filter(|(k, _)| path_in_specs(k, specs))
            .collect();
        tree_cache.insert(tree, m.clone());
        Ok(m)
    };
    let mut hidden: HashSet<Oid> = HashSet::new();
    for x in exclude {
        hidden.extend(arks_repo::ancestor_set(repo, *x)?);
    }
    // Oid is not Ord: heap keys are (date, unique seq); seq → Oid side table.
    let mut heap: BinaryHeap<(i64, u64)> = BinaryHeap::new();
    let mut by_seq: std::collections::HashMap<u64, Oid> = std::collections::HashMap::new();
    let mut seen: HashSet<Oid> = HashSet::new();
    let mut seq = u64::MAX;
    for o in include {
        if seen.insert(*o) {
            let c = read_commit(repo, o)?;
            heap.push((c.committer.timestamp, seq));
            by_seq.insert(seq, *o);
            seq -= 1;
        }
    }
    let mut out = Vec::new();
    while let Some((_, key)) = heap.pop() {
        let oid = by_seq.remove(&key).expect("seq registered on push");
        let c = read_commit(repo, &oid)?;
        let mut parents: Vec<Oid> = if first_parent {
            c.parents.iter().take(1).copied().collect()
        } else {
            c.parents.clone()
        };
        let mut show = true;
        if !specs.is_empty() {
            let mine = filtered(c.tree)?;
            if parents.is_empty() {
                show = !mine.is_empty();
            } else {
                let mut same = None;
                for p in &parents {
                    if filtered(read_commit(repo, p)?.tree)? == mine {
                        same = Some(*p);
                        break;
                    }
                }
                if let Some(p) = same {
                    show = false;
                    parents = vec![p];
                }
            }
        }
        for p in parents {
            if seen.insert(p) {
                let pc = read_commit(repo, &p)?;
                heap.push((pc.committer.timestamp, seq));
                by_seq.insert(seq, p);
                seq -= 1;
            }
        }
        if show && !hidden.contains(&oid) {
            out.push((oid, c));
        }
    }
    Ok(out)
}

pub fn read_commit(repo: &Repository, oid: &Oid) -> CliResult<Commit> {
    let obj = repo.get_object(oid)?;
    if obj.kind != ObjectKind::Commit {
        return Err(CliError::Msg(format!("{} is not a commit", oid.to_hex())));
    }
    Ok(Commit::decode(&obj.payload)?)
}

/// Resolve rev arguments into (include, exclude) per gitrevisions.
pub fn resolve_range(repo: &Repository, revs: &[String]) -> CliResult<(Vec<Oid>, Vec<Oid>)> {
    let mut inc = Vec::new();
    let mut exc = Vec::new();
    let side =
        |s: &str| -> CliResult<Oid> { resolve_rev(repo, if s.is_empty() { "HEAD" } else { s }) };
    for r in revs {
        if let Some((l, rr)) = r.split_once("...") {
            let a = side(l)?;
            let b = side(rr)?;
            inc.push(a);
            inc.push(b);
            if let Some(base) = arks_repo::merge_base_oid(repo, &a, &b)? {
                exc.push(base);
            }
        } else if let Some((l, rr)) = r.split_once("..") {
            exc.push(side(l)?);
            inc.push(side(rr)?);
        } else if let Some(x) = r.strip_prefix('^') {
            exc.push(side(x)?);
        } else {
            inc.push(side(r)?);
        }
    }
    if inc.is_empty() {
        match repo.head_oid() {
            Ok(o) => inc.push(o),
            Err(_) => return Ok((Vec::new(), Vec::new())),
        }
    }
    Ok((inc, exc))
}

/// Pathspec prefix match (`""` matches everything).
pub fn path_in_specs(path: &str, specs: &[String]) -> bool {
    specs.is_empty()
        || specs.iter().any(|s| {
            s.is_empty()
                || path == s
                || (path.len() > s.len()
                    && path.starts_with(s.as_str())
                    && path.as_bytes()[s.len()] == b'/')
        })
}

fn tree_blob_map_of_tree(
    repo: &Repository,
    tree: &Oid,
) -> CliResult<std::collections::HashMap<String, Oid>> {
    let mut out = std::collections::HashMap::new();
    arks_repo::flatten_tree(repo, tree, "", &mut out)?;
    Ok(out)
}

/// `--date=<format>` selection (git-log §Commit Formatting, `--date`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DateMode {
    #[default]
    Default,
    Iso,
    IsoStrict,
    Short,
    Raw,
    Unix,
    Rfc2822,
}

impl DateMode {
    /// Parse a `--date=` value (claimed subset; `relative`/`local`/`human` refuse).
    pub fn parse(v: &str) -> CliResult<Self> {
        Ok(match v {
            "default" => Self::Default,
            "iso" | "iso8601" => Self::Iso,
            "iso-strict" | "iso8601-strict" => Self::IsoStrict,
            "short" => Self::Short,
            "raw" => Self::Raw,
            "unix" => Self::Unix,
            "rfc" | "rfc2822" => Self::Rfc2822,
            other => {
                return Err(CliError::NotImplemented(format!(
                    "unsupported --date format: {other}"
                )));
            }
        })
    }

    pub fn format(self, sig: &Signature) -> String {
        match self {
            Self::Default => default_date(sig),
            Self::Iso => iso_like(sig),
            Self::IsoStrict => iso_strict(sig),
            Self::Short => short_date(sig),
            Self::Raw => format!("{} {}", sig.timestamp, sig.timezone),
            Self::Unix => sig.timestamp.to_string(),
            Self::Rfc2822 => rfc2822(sig),
        }
    }
}

/// Per-walk formatting context: date mode and ref decorations (`%d`/`%D`).
#[derive(Default)]
pub struct FmtCtx<'a> {
    pub date: DateMode,
    pub decorations: Option<&'a std::collections::HashMap<Oid, Vec<String>>>,
}

impl FmtCtx<'_> {
    /// Decoration names for `oid` joined with `, ` (empty when none).
    pub fn decoration(&self, oid: &Oid) -> String {
        self.decorations
            .and_then(|d| d.get(oid))
            .map(|v| v.join(", "))
            .unwrap_or_default()
    }
}

/// Expand pretty-format placeholders for one commit.
#[cfg(test)]
pub fn expand(fmt: &str, oid: &Oid, c: &Commit) -> String {
    expand_ctx(fmt, oid, c, &FmtCtx::default())
}

/// [`expand`] honouring `--date=` and decorations.
pub fn expand_ctx(fmt: &str, oid: &Oid, c: &Commit, ctx: &FmtCtx<'_>) -> String {
    let hex = oid.to_hex();
    let mut out = String::new();
    let mut chars = fmt.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '%' {
            out.push(ch);
            continue;
        }
        let Some(n) = chars.next() else {
            out.push('%');
            break;
        };
        match n {
            '%' => out.push('%'),
            'n' => out.push('\n'),
            'H' => out.push_str(&hex),
            'h' => out.push_str(&hex[..7]),
            'T' => out.push_str(&c.tree.to_hex()),
            't' => out.push_str(&c.tree.to_hex()[..7]),
            'P' => out.push_str(&join(c.parents.iter().map(|p| p.to_hex()))),
            'p' => out.push_str(&join(c.parents.iter().map(|p| p.to_hex()[..7].to_string()))),
            's' => out.push_str(&subject_line(&c.message)),
            'b' => out.push_str(&body(&c.message)),
            'B' => out.push_str(&c.message),
            'd' | 'D' => {
                let d = ctx.decoration(oid);
                if n == 'D' {
                    out.push_str(&d);
                } else if !d.is_empty() {
                    out.push_str(&format!(" ({d})"));
                }
            }
            'a' | 'c' => {
                let sig = if n == 'a' { &c.author } else { &c.committer };
                let Some(k) = chars.next() else {
                    out.push('%');
                    out.push(n);
                    break;
                };
                match person(sig, k, ctx.date) {
                    Some(v) => out.push_str(&v),
                    None => {
                        out.push('%');
                        out.push(n);
                        out.push(k);
                    }
                }
            }
            other => {
                out.push('%');
                out.push(other);
            }
        }
    }
    out
}

fn join(it: impl Iterator<Item = String>) -> String {
    it.collect::<Vec<_>>().join(" ")
}

/// Subject = first paragraph joined with spaces (git `%s`).
pub fn subject_line(msg: &str) -> String {
    msg.split("\n\n")
        .next()
        .unwrap_or("")
        .lines()
        .map(str::trim)
        .collect::<Vec<_>>()
        .join(" ")
}

fn body(msg: &str) -> String {
    match msg.split_once("\n\n") {
        Some((_, b)) => b.to_string(),
        None => String::new(),
    }
}

fn person(sig: &Signature, k: char, mode: DateMode) -> Option<String> {
    Some(match k {
        'n' | 'N' => sig.name.clone(),
        'e' | 'E' => sig.email.clone(),
        'l' => sig.email.split('@').next().unwrap_or("").to_string(),
        't' => sig.timestamp.to_string(),
        'd' => mode.format(sig),
        'D' => rfc2822(sig),
        'I' => iso_strict(sig),
        'i' => iso_like(sig),
        's' => short_date(sig),
        _ => return None,
    })
}

/// Minutes east of UTC from `+HHMM` / `-HHMM`.
fn tz_minutes(tz: &str) -> i64 {
    let sign = if tz.starts_with('-') { -1 } else { 1 };
    let d = tz.trim_start_matches(['+', '-']);
    if d.len() != 4 {
        return 0;
    }
    let h: i64 = d[..2].parse().unwrap_or(0);
    let m: i64 = d[2..].parse().unwrap_or(0);
    sign * (h * 60 + m)
}

struct Civil {
    y: i64,
    mo: u32,
    d: u32,
    h: u32,
    mi: u32,
    s: u32,
    wd: u32,
}

/// Unix seconds → civil time (Howard Hinnant's days-from-civil inverse).
fn civil(ts: i64, tz: &str) -> Civil {
    let t = ts + tz_minutes(tz) * 60;
    let days = t.div_euclid(86_400);
    let secs = t.rem_euclid(86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + if mo <= 2 { 1 } else { 0 };
    Civil {
        y,
        mo,
        d,
        h: (secs / 3600) as u32,
        mi: ((secs % 3600) / 60) as u32,
        s: (secs % 60) as u32,
        wd: ((days + 4).rem_euclid(7)) as u32,
    }
}

const WD: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const MO: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// git `--date=default`: `Thu Nov 19 18:00:00 2026 +0000`.
pub fn default_date(sig: &Signature) -> String {
    let c = civil(sig.timestamp, &sig.timezone);
    format!(
        "{} {} {} {:02}:{:02}:{:02} {} {}",
        WD[c.wd as usize],
        MO[(c.mo - 1) as usize],
        c.d,
        c.h,
        c.mi,
        c.s,
        c.y,
        sig.timezone
    )
}

fn iso_strict(sig: &Signature) -> String {
    let c = civil(sig.timestamp, &sig.timezone);
    let tz = &sig.timezone;
    let tz = if tz.len() == 5 {
        format!("{}:{}", &tz[..3], &tz[3..])
    } else {
        tz.clone()
    };
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}{tz}",
        c.y, c.mo, c.d, c.h, c.mi, c.s
    )
}

fn iso_like(sig: &Signature) -> String {
    let c = civil(sig.timestamp, &sig.timezone);
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02} {}",
        c.y, c.mo, c.d, c.h, c.mi, c.s, sig.timezone
    )
}

/// git `--date=rfc2822`: `Wed, 15 Nov 2023 05:13:20 +0000`.
fn rfc2822(sig: &Signature) -> String {
    let c = civil(sig.timestamp, &sig.timezone);
    format!(
        "{}, {} {} {} {:02}:{:02}:{:02} {}",
        WD[c.wd as usize],
        c.d,
        MO[(c.mo - 1) as usize],
        c.y,
        c.h,
        c.mi,
        c.s,
        sig.timezone
    )
}

fn short_date(sig: &Signature) -> String {
    let c = civil(sig.timestamp, &sig.timezone);
    format!("{:04}-{:02}-{:02}", c.y, c.mo, c.d)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sig(ts: i64, tz: &str) -> Signature {
        Signature {
            name: "A U Thor".into(),
            email: "author@example.com".into(),
            timestamp: ts,
            timezone: tz.into(),
        }
    }

    #[test]
    fn dates_match_git_shapes() {
        // 1112911993 = Thu Apr 7 15:13:13 2005 -0700 (git test-lib default).
        let s = sig(1_112_911_993, "-0700");
        assert_eq!(default_date(&s), "Thu Apr 7 15:13:13 2005 -0700");
        assert_eq!(iso_strict(&s), "2005-04-07T15:13:13-07:00");
        assert_eq!(short_date(&sig(0, "+0000")), "1970-01-01");
    }

    #[test]
    fn placeholders() {
        let c = Commit {
            tree: Oid::from_hex("4b825dc642cb6eb9a060e54bf8d69288fbee4904").unwrap(),
            parents: vec![],
            author: sig(0, "+0000"),
            committer: sig(0, "+0000"),
            gpgsig: None,
            message: "subj line\nwrapped\n\nbody text\n".into(),
        };
        let oid = Oid::from_hex("0123456789abcdef0123456789abcdef01234567").unwrap();
        assert_eq!(
            expand("%h %s|%an <%ae>|%%|%x", &oid, &c),
            "0123456 subj line wrapped|A U Thor <author@example.com>|%|%x"
        );
        assert_eq!(expand("%b", &oid, &c), "body text\n");
    }

    #[test]
    fn take_pretty_forms() {
        let mut a: Vec<String> = vec!["--format=%H".into(), "x".into()];
        assert_eq!(take_pretty(&mut a).unwrap(), Pretty::TFormat("%H".into()));
        assert_eq!(a, vec!["x".to_string()]);
        let mut b: Vec<String> = vec!["--pretty=format:%s".into()];
        assert_eq!(take_pretty(&mut b).unwrap(), Pretty::Format("%s".into()));
        let mut c: Vec<String> = vec!["--pretty=oneline".into()];
        assert_eq!(take_pretty(&mut c).unwrap(), Pretty::Oneline);
    }
}
