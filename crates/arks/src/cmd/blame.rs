//! `blame` / `annotate`: per-line origin commits.
//!
//! Lines are passed from a commit to each parent (in parent order) when the
//! Myers line match of the parent's blob against the commit's blob pairs
//! them; whatever no parent takes is blamed on the commit. Commits are
//! processed newest committer date first. Without a revision the working
//! tree file is blamed, lines not in `HEAD` showing as `Not Committed Yet`.
//! Output shapes: default (`<hash> (<author> <date> <n>) <line>`, root
//! commits as `^<hash>`), `-s`, `-e`, `-l`, `-n`, `-L`, `--porcelain`,
//! `--line-porcelain`, and `annotate`'s tab-separated form.
//! Primary: <https://git-scm.com/docs/git-blame>, <https://git-scm.com/docs/git-annotate>.
//!
//! Partial: no rename/copy following (`-M`/`-C`), `-w`, `--reverse`,
//! `--incremental`, or `-L :<funcname>` / regex ranges.

use crate::cmd::shared::resolve_rev;
use crate::cmd::unified_diff::matched_lines;
use crate::error::{CliError, CliResult};
use crate::util::{open_repo, work_tree};
use arks_object::{Commit, Signature};
use arks_oid::Oid;
use arks_repo::Repository;
use std::collections::{BinaryHeap, HashMap};
use std::io::Write;

fn split(bytes: &[u8]) -> Vec<&[u8]> {
    if bytes.is_empty() {
        return Vec::new();
    }
    let body = bytes.strip_suffix(b"\n").unwrap_or(bytes);
    body.split(|c| *c == b'\n').collect()
}

/// Blame result for one final line: origin commit (`None` = not committed)
/// and its 1-based line number in that commit's blob.
#[derive(Clone, Copy)]
struct Origin {
    commit: Option<Oid>,
    orig: usize,
}

struct Info {
    commit: Commit,
    boundary: bool,
    previous: Option<Oid>,
}

fn blob_at(repo: &Repository, commit: &Commit, path: &str) -> CliResult<Option<Vec<u8>>> {
    Ok(
        match crate::cmd::shared::tree_entry_at(repo, &commit.tree, path) {
            Some(o) => {
                let obj = repo.get_object(&o)?;
                (obj.kind == arks_object::ObjectKind::Blob).then_some(obj.payload)
            }
            None => None,
        },
    )
}

/// Run the blame walk from `start` for `path`; `lines` are (final index,
/// index in `start`'s blob) pairs still to attribute.
fn walk(
    repo: &Repository,
    start: Oid,
    path: &str,
    lines: Vec<(usize, usize)>,
    origins: &mut [Origin],
    infos: &mut HashMap<Oid, Info>,
) -> CliResult<()> {
    let mut pending: HashMap<Oid, Vec<(usize, usize)>> = HashMap::new();
    let mut heap: BinaryHeap<(i64, u64)> = BinaryHeap::new();
    let mut by_seq: HashMap<u64, Oid> = HashMap::new();
    let mut seq = u64::MAX;
    let mut commits: HashMap<Oid, Commit> = HashMap::new();
    let load = |o: &Oid, commits: &mut HashMap<Oid, Commit>| -> CliResult<Commit> {
        if let Some(c) = commits.get(o) {
            return Ok(c.clone());
        }
        let c = crate::cmd::log_format::read_commit(repo, o)?;
        commits.insert(*o, c.clone());
        Ok(c)
    };
    let c0 = load(&start, &mut commits)?;
    pending.insert(start, lines);
    heap.push((c0.committer.timestamp, seq));
    by_seq.insert(seq, start);
    seq -= 1;
    while let Some((_, key)) = heap.pop() {
        let oid = by_seq.remove(&key).expect("seq registered");
        let Some(mut todo) = pending.remove(&oid) else {
            continue;
        };
        let c = load(&oid, &mut commits)?;
        let mine = blob_at(repo, &c, path)?.unwrap_or_default();
        let mine_lines = split(&mine);
        let mut previous = None;
        for p in &c.parents {
            if todo.is_empty() {
                break;
            }
            let pc = load(p, &mut commits)?;
            let Some(pblob) = blob_at(repo, &pc, path)? else {
                continue;
            };
            if previous.is_none() {
                previous = Some(*p);
            }
            let map = matched_lines(&split(&pblob), &mine_lines);
            let mut passed = Vec::new();
            todo.retain(|(fin, idx)| match map.get(*idx).copied().flatten() {
                Some(j) => {
                    passed.push((*fin, j));
                    false
                }
                None => true,
            });
            if !passed.is_empty() {
                let entry = pending.entry(*p).or_default();
                let was_empty = entry.is_empty();
                entry.extend(passed);
                if was_empty {
                    heap.push((pc.committer.timestamp, seq));
                    by_seq.insert(seq, *p);
                    seq -= 1;
                }
            }
        }
        for (fin, idx) in &todo {
            origins[*fin] = Origin {
                commit: Some(oid),
                orig: idx + 1,
            };
        }
        if !todo.is_empty() || !infos.contains_key(&oid) {
            let boundary = c.parents.is_empty();
            infos.entry(oid).or_insert(Info {
                commit: c,
                boundary,
                previous,
            });
        }
    }
    Ok(())
}

struct Opts {
    suppress: bool,
    email: bool,
    long: bool,
    show_number: bool,
    porcelain: bool,
    line_porcelain: bool,
    root: bool,
    annotate: bool,
    ranges: Vec<String>,
}

fn parse_range(spec: &str, total: usize) -> CliResult<(usize, usize)> {
    let bad = || CliError::fatal(format!("invalid -L argument: {spec}"));
    let (a, b) = match spec.split_once(',') {
        Some((a, b)) => (a, b),
        None => (spec, ""),
    };
    let start: usize = if a.is_empty() {
        1
    } else {
        a.parse().map_err(|_| bad())?
    };
    let end = if b.is_empty() {
        total
    } else if let Some(n) = b.strip_prefix('+') {
        start + n.parse::<usize>().map_err(|_| bad())? - 1
    } else if let Some(n) = b.strip_prefix('-') {
        let n: usize = n.parse().map_err(|_| bad())?;
        let s = start.saturating_sub(n - 1).max(1);
        return Ok((s, start));
    } else {
        b.parse().map_err(|_| bad())?
    };
    if start == 0 || start > total {
        return Err(CliError::fatal(format!(
            "file has only {total} line{}",
            if total == 1 { "" } else { "s" }
        )));
    }
    Ok((start.min(end), end.max(start).min(total)))
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

fn run(args: &[String], annotate: bool, sink: &mut dyn Write) -> CliResult<()> {
    let args = crate::util::split_short_bundles(args, "selnpf", "L");
    let mut o = Opts {
        suppress: false,
        email: false,
        long: false,
        show_number: false,
        porcelain: false,
        line_porcelain: false,
        root: false,
        annotate,
        ranges: Vec::new(),
    };
    let mut positional: Vec<String> = Vec::new();
    let mut dashdash_path: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        let a = args[i].clone();
        i += 1;
        match a.as_str() {
            "--" => {
                dashdash_path = args.get(i).cloned();
                break;
            }
            "-s" => o.suppress = true,
            "-e" | "--show-email" => o.email = true,
            "-l" => o.long = true,
            "-n" | "--show-number" => o.show_number = true,
            "-p" | "--porcelain" => o.porcelain = true,
            "--line-porcelain" => {
                o.porcelain = true;
                o.line_porcelain = true;
            }
            "--root" => o.root = true,
            "-f" | "--show-name" | "--no-color" => {}
            "-L" => {
                o.ranges.push(
                    args.get(i)
                        .cloned()
                        .ok_or_else(|| CliError::fatal("switch `L' requires a value"))?,
                );
                i += 1;
            }
            x if x.starts_with("-L") => o.ranges.push(x[2..].to_string()),
            x if x.starts_with('-') && x.len() > 1 => {
                return Err(CliError::NotImplemented(format!("unsupported option: {x}")));
            }
            _ => positional.push(a),
        }
    }
    let (rev, path_arg) = match (dashdash_path, positional.as_slice()) {
        (Some(p), [r]) => (Some(r.clone()), p),
        (Some(p), []) => (None, p),
        (None, [p]) => (None, p.clone()),
        (None, [r, p]) => (Some(r.clone()), p.clone()),
        _ => return Err(CliError::Usage("blame [<rev>] [--] <file>".into())),
    };
    let repo = open_repo()?;
    let work = work_tree(&repo)?;
    let path = crate::cmd::core::pathspec_rel(&work, &path_arg)?;
    let (content, start, from_worktree) = match &rev {
        Some(r) => {
            let oid = resolve_rev(&repo, r)?;
            let commit = crate::cmd::shared::peel_to_kind(
                &repo,
                &oid,
                crate::cmd::shared::ObjKind::Commit,
                r,
            )?;
            let cc = crate::cmd::log_format::read_commit(&repo, &commit)?;
            let blob = blob_at(&repo, &cc, &path)?
                .ok_or_else(|| CliError::fatal(format!("no such path {path} in {r}")))?;
            (blob, Some(commit), false)
        }
        None => {
            let bytes = std::fs::read(work.join(&path))
                .map_err(|_| CliError::fatal(format!("no such path '{path}' in HEAD")))?;
            (bytes, repo.head_oid().ok(), true)
        }
    };
    let lines = split(&content);
    let mut origins = vec![
        Origin {
            commit: None,
            orig: 0,
        };
        lines.len()
    ];
    for (i, o) in origins.iter_mut().enumerate() {
        o.orig = i + 1;
    }
    let mut infos: HashMap<Oid, Info> = HashMap::new();
    if let Some(start) = start {
        let todo: Vec<(usize, usize)> = if from_worktree {
            let head = crate::cmd::log_format::read_commit(&repo, &start)?;
            match blob_at(&repo, &head, &path)? {
                Some(hb) => {
                    let hl = split(&hb);
                    matched_lines(&hl, &lines)
                        .iter()
                        .enumerate()
                        .filter_map(|(fin, m)| m.map(|j| (fin, j)))
                        .collect()
                }
                None => {
                    if !crate::util::read_index(&repo)?
                        .entries
                        .iter()
                        .any(|e| e.path == path)
                    {
                        return Err(CliError::fatal(format!("no such path '{path}' in HEAD")));
                    }
                    Vec::new()
                }
            }
        } else {
            (0..lines.len()).map(|i| (i, i)).collect()
        };
        walk(&repo, start, &path, todo, &mut origins, &mut infos)?;
    }
    let mut ranges = Vec::new();
    for r in &o.ranges {
        ranges.push(parse_range(r, lines.len())?);
    }
    if ranges.is_empty() {
        ranges.push((1, lines.len()));
    }
    let shown: Vec<usize> = (0..lines.len())
        .filter(|i| ranges.iter().any(|(s, e)| (*s..=*e).contains(&(i + 1))))
        .collect();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let uncommitted = Signature {
        name: "Not Committed Yet".into(),
        email: "not.committed.yet".into(),
        timestamp: now,
        timezone: "+0000".into(),
    };
    let author_of = |or: &Origin| -> Signature {
        match or.commit.and_then(|c| infos.get(&c)) {
            Some(info) => info.commit.author.clone(),
            None => uncommitted.clone(),
        }
    };
    let mut out = std::io::BufWriter::new(sink);
    if o.porcelain {
        let mut seen: std::collections::HashSet<Option<Oid>> = std::collections::HashSet::new();
        let mut k = 0;
        while k < shown.len() {
            let fin = shown[k];
            let or = origins[fin];
            // Group consecutive lines from the same commit with consecutive origins.
            let mut n = 1;
            while k + n < shown.len()
                && shown[k + n] == fin + n
                && origins[fin + n].commit == or.commit
                && origins[fin + n].orig == or.orig + n
            {
                n += 1;
            }
            let hex = or
                .commit
                .map(|c| c.to_hex())
                .unwrap_or_else(|| "0".repeat(40));
            for j in 0..n {
                let f = fin + j;
                let ok = origins[f];
                if j == 0 {
                    writeln!(out, "{hex} {} {} {n}", ok.orig, f + 1)?;
                } else {
                    writeln!(out, "{hex} {} {}", ok.orig, f + 1)?;
                }
                if o.line_porcelain || (j == 0 && seen.insert(or.commit)) {
                    write_porcelain_info(
                        &mut out,
                        or.commit.and_then(|c| infos.get(&c)),
                        &uncommitted,
                        &path,
                        o.root,
                    )?;
                }
                out.write_all(b"\t")?;
                out.write_all(lines[f])?;
                out.write_all(b"\n")?;
            }
            k += n;
        }
        out.flush()?;
        return Ok(());
    }
    let hash_len = if o.long { 40 } else { 8 };
    let author_w = shown
        .iter()
        .map(|f| {
            let a = author_of(&origins[*f]);
            if o.email {
                a.email.chars().count() + 2
            } else {
                a.name.chars().count()
            }
        })
        .max()
        .unwrap_or(0);
    let line_w = shown.last().map(|f| (f + 1).to_string().len()).unwrap_or(1);
    let orig_w = shown
        .iter()
        .map(|f| origins[*f].orig.to_string().len())
        .max()
        .unwrap_or(1);
    for &f in &shown {
        let or = origins[f];
        let info = or.commit.and_then(|c| infos.get(&c));
        let boundary = info.is_some_and(|i| i.boundary) && !o.root;
        let hex = or
            .commit
            .map(|c| c.to_hex())
            .unwrap_or_else(|| "0".repeat(40));
        let sig = author_of(&or);
        let date = crate::cmd::log_format::DateMode::Iso.format(&sig);
        if o.annotate {
            write!(out, "{}\t({:>10}\t{date}\t{})", &hex[..8], sig.name, f + 1)?;
            out.write_all(lines[f])?;
            out.write_all(b"\n")?;
            continue;
        }
        let shown_hash = if boundary {
            format!("^{}", &hex[..hash_len - 1])
        } else {
            hex[..hash_len].to_string()
        };
        let mut lead = shown_hash;
        if o.show_number {
            lead.push_str(&format!(" {:>orig_w$}", or.orig));
        }
        if o.suppress {
            write!(out, "{lead} {:>line_w$}) ", f + 1)?;
        } else {
            let who = if o.email {
                format!("<{}>", sig.email)
            } else {
                sig.name.clone()
            };
            write!(out, "{lead} ({who:<author_w$} {date} {:>line_w$}) ", f + 1)?;
        }
        out.write_all(lines[f])?;
        out.write_all(b"\n")?;
    }
    out.flush()?;
    Ok(())
}

fn write_porcelain_info(
    out: &mut impl Write,
    info: Option<&Info>,
    uncommitted: &Signature,
    path: &str,
    root: bool,
) -> CliResult<()> {
    let (author, committer, summary) = match info {
        Some(i) => (
            i.commit.author.clone(),
            i.commit.committer.clone(),
            crate::cmd::log_format::subject_line(&i.commit.message),
        ),
        None => (
            uncommitted.clone(),
            uncommitted.clone(),
            "Version of working tree file".to_string(),
        ),
    };
    writeln!(out, "author {}", author.name)?;
    writeln!(out, "author-mail <{}>", author.email)?;
    writeln!(out, "author-time {}", author.timestamp)?;
    writeln!(out, "author-tz {}", author.timezone)?;
    writeln!(out, "committer {}", committer.name)?;
    writeln!(out, "committer-mail <{}>", committer.email)?;
    writeln!(out, "committer-time {}", committer.timestamp)?;
    writeln!(out, "committer-tz {}", committer.timezone)?;
    writeln!(out, "summary {summary}")?;
    if let Some(i) = info {
        if let Some(p) = i.previous {
            writeln!(out, "previous {} {path}", p.to_hex())?;
        } else if i.boundary && !root {
            writeln!(out, "boundary")?;
        }
    }
    writeln!(out, "filename {path}")?;
    Ok(())
}
