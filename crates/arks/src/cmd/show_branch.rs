//! `show-branch`: branches and their commits side by side.
//!
//! Each named rev gets a column. Commits are walked newest first from every
//! tip, each carrying the set of tips that reach it, until only commits
//! common to every tip remain (plus `--more=<n>`). The commits seen are then
//! sorted (graph order by default, `--date-order` by commit date) and printed
//! with one mark per column (`*` for the current branch, `+` otherwise, `-`
//! for merges), named relative to the tips (`main~2`, `main^^2`). Also
//! `--list`, `--merge-base`, `--independent`, `--topics`, `--sparse`,
//! `--sha1-name`, `--no-name`, `--current`, `--all`, `--remotes`, ref globs,
//! `--color` and `--reflog[=<n>[,<base>]]`.
//! Primary: <https://git-scm.com/docs/git-show-branch>.

use crate::cmd::log_format::{read_commit, subject_line};
use crate::cmd::opt_parse::{OptDef, Takes, o, parse};
use crate::cmd::shared::{ObjKind, abbrev_hex, peel_to_kind, resolve_rev};
use crate::error::{CliError, CliResult};
use crate::util::open_repo;
use arks_object::Commit;
use arks_oid::Oid;
use arks_refs::RefTarget;
use arks_repo::Repository;
use std::collections::HashMap;
use std::io::Write;

const USAGE: &str = "usage: git show-branch [-a | --all] [-r | --remotes] [--topo-order | --date-order]\n                       [--current] [--color[=<when>] | --no-color] [--sparse]\n                       [--more=<n> | --list | --independent | --merge-base]\n                       [--no-name | --sha1-name] [--topics]\n                       [(<rev> | <glob>)...]\n   or: git show-branch (-g | --reflog)[=<n>[,<base>]] [--list] [<ref>]";

const OPTS: &[OptDef] = &[
    o("all", 'a', Takes::No, true),
    o("remotes", 'r', Takes::No, true),
    o("color", ' ', Takes::Opt, true),
    o("more", ' ', Takes::Opt, true),
    o("list", ' ', Takes::No, true),
    o("no-name", ' ', Takes::No, true),
    o("current", ' ', Takes::No, true),
    o("sha1-name", ' ', Takes::No, true),
    o("merge-base", ' ', Takes::No, true),
    o("independent", ' ', Takes::No, true),
    o("topo-order", ' ', Takes::No, false),
    o("topics", ' ', Takes::No, true),
    o("sparse", ' ', Takes::No, true),
    o("date-order", ' ', Takes::No, false),
    o("reflog", 'g', Takes::Opt, true),
];

const COLUMN_COLORS: [&str; 12] = [
    "\x1b[31m",
    "\x1b[32m",
    "\x1b[33m",
    "\x1b[34m",
    "\x1b[35m",
    "\x1b[36m",
    "\x1b[1;31m",
    "\x1b[1;32m",
    "\x1b[1;33m",
    "\x1b[1;34m",
    "\x1b[1;35m",
    "\x1b[1;36m",
];

const UNINTERESTING: u64 = 1 << 62;

fn usage_only() -> CliError {
    CliError::Exit(129, USAGE.to_string())
}

/// One commit known to the walk.
struct Node {
    commit: Commit,
    flags: u64,
    seen: bool,
}

/// Name of a commit relative to a tip.
#[derive(Clone)]
struct Name {
    head: String,
    generation: usize,
}

impl Name {
    fn render(&self) -> String {
        match self.generation {
            0 => self.head.clone(),
            1 => format!("{}^", self.head),
            n => format!("{}~{n}", self.head),
        }
    }
}

struct Walk<'a> {
    repo: &'a Repository,
    nodes: HashMap<Oid, Node>,
}

impl Walk<'_> {
    fn node(&mut self, oid: Oid) -> CliResult<&mut Node> {
        if !self.nodes.contains_key(&oid) {
            let commit = read_commit(self.repo, &oid)?;
            self.nodes.insert(
                oid,
                Node {
                    commit,
                    flags: 0,
                    seen: false,
                },
            );
        }
        Ok(self.nodes.get_mut(&oid).expect("inserted"))
    }

    fn date(&mut self, oid: Oid) -> CliResult<i64> {
        Ok(self.node(oid)?.commit.committer.timestamp)
    }

    /// Insert after every queued commit that is not older (git's insert-by-date).
    fn insert_by_date(&mut self, queue: &mut Vec<Oid>, oid: Oid) -> CliResult<()> {
        let d = self.date(oid)?;
        let mut at = queue.len();
        for (i, q) in queue.iter().enumerate() {
            if self.nodes[q].commit.committer.timestamp < d {
                at = i;
                break;
            }
        }
        queue.insert(at, oid);
        Ok(())
    }

    /// Walk from `revs` until only commits common to every tip remain (plus `extra`).
    fn join(&mut self, revs: &[Oid], mut extra: i64) -> CliResult<Vec<Oid>> {
        let n = revs.len();
        let all_revs: u64 = (1u64 << n) - 1;
        let mut queue: Vec<Oid> = Vec::new();
        for (i, r) in revs.iter().enumerate() {
            self.node(*r)?.flags |= 1 << i;
            self.insert_by_date(&mut queue, *r)?;
        }
        let mut seen = Vec::new();
        while !queue.is_empty() {
            let still_interesting = queue
                .iter()
                .any(|q| self.nodes[q].flags & UNINTERESTING == 0);
            let oid = queue.remove(0);
            let mut flags = self.nodes[&oid].flags & (all_revs | UNINTERESTING);
            if !still_interesting && extra <= 0 {
                break;
            }
            if !still_interesting {
                extra -= 1;
            }
            let node = self.nodes.get_mut(&oid).expect("queued");
            if !node.seen {
                node.seen = true;
                seen.push(oid);
            }
            if flags & all_revs == all_revs {
                flags |= UNINTERESTING;
            }
            let parents = node.commit.parents.clone();
            for p in parents {
                let pn = self.node(p)?;
                if pn.flags & flags == flags {
                    continue;
                }
                pn.flags |= flags;
                self.insert_by_date(&mut queue, p)?;
            }
        }
        // Propagate UNINTERESTING through the commits seen.
        loop {
            let mut changed = false;
            for c in &seen {
                if self.nodes[c].flags & UNINTERESTING == 0 {
                    continue;
                }
                for p in self.nodes[c].commit.parents.clone() {
                    if let Some(pn) = self.nodes.get_mut(&p) {
                        if pn.seen && pn.flags & UNINTERESTING == 0 {
                            pn.flags |= UNINTERESTING;
                            changed = true;
                        }
                    }
                }
            }
            if !changed {
                break;
            }
        }
        Ok(seen)
    }

    /// Topological sort of `list`: graph order (LIFO from the tips in walk
    /// order) or commit-date order.
    fn sort(&self, list: &[Oid], by_date: bool) -> Vec<Oid> {
        let inlist: HashMap<Oid, usize> = list.iter().enumerate().map(|(i, o)| (*o, i)).collect();
        let mut indeg: HashMap<Oid, usize> = list.iter().map(|o| (*o, 0)).collect();
        for c in list {
            for p in &self.nodes[c].commit.parents {
                if let Some(d) = indeg.get_mut(p) {
                    *d += 1;
                }
            }
        }
        let mut out = Vec::with_capacity(list.len());
        if by_date {
            // (date, -insertion counter) max-heap.
            let mut heap = std::collections::BinaryHeap::new();
            let mut counter: i64 = 0;
            for c in list {
                if indeg[c] == 0 {
                    heap.push((
                        self.nodes[c].commit.committer.timestamp,
                        -counter,
                        inlist[c],
                    ));
                    counter += 1;
                }
            }
            while let Some((_, _, ci)) = heap.pop() {
                let c = list[ci];
                out.push(c);
                for p in &self.nodes[&c].commit.parents {
                    if !inlist.contains_key(p) {
                        continue;
                    }
                    let d = indeg.get_mut(p).expect("in list");
                    *d -= 1;
                    if *d == 0 {
                        heap.push((
                            self.nodes[p].commit.committer.timestamp,
                            -counter,
                            inlist[p],
                        ));
                        counter += 1;
                    }
                }
            }
        } else {
            let mut stack: Vec<Oid> = list.iter().filter(|c| indeg[*c] == 0).copied().collect();
            stack.reverse();
            while let Some(c) = stack.pop() {
                out.push(c);
                for p in &self.nodes[&c].commit.parents {
                    if !inlist.contains_key(p) {
                        continue;
                    }
                    let d = indeg.get_mut(p).expect("in list");
                    *d -= 1;
                    if *d == 0 {
                        stack.push(*p);
                    }
                }
            }
        }
        out
    }
}

/// Give names to the commits in `list` relative to the tips.
fn name_commits(
    walk: &Walk<'_>,
    list: &[Oid],
    revs: &[Oid],
    ref_names: &[String],
) -> HashMap<Oid, Name> {
    let mut names: HashMap<Oid, Name> = HashMap::new();
    for c in list {
        if names.contains_key(c) {
            continue;
        }
        if let Some(i) = revs.iter().position(|r| r == c) {
            names.insert(
                *c,
                Name {
                    head: ref_names[i].clone(),
                    generation: 0,
                },
            );
        }
    }
    let first_parent_chain = |names: &mut HashMap<Oid, Name>, start: Oid| -> usize {
        let mut named = 0;
        let mut c = start;
        while let Some(n) = names.get(&c).cloned() {
            let Some(node) = walk.nodes.get(&c) else {
                break;
            };
            let Some(p) = node.commit.parents.first().copied() else {
                break;
            };
            if names.contains_key(&p) {
                break;
            }
            names.insert(
                p,
                Name {
                    head: n.head,
                    generation: n.generation + 1,
                },
            );
            named += 1;
            c = p;
        }
        named
    };
    loop {
        let mut i = 0;
        for c in list {
            i += first_parent_chain(&mut names, *c);
        }
        if i == 0 {
            break;
        }
    }
    loop {
        let mut i = 0;
        for c in list {
            let Some(n) = names.get(c).cloned() else {
                continue;
            };
            let Some(node) = walk.nodes.get(c) else {
                continue;
            };
            for (k, p) in node.commit.parents.iter().enumerate() {
                if names.contains_key(p) {
                    continue;
                }
                let mut new = n.render();
                if k == 0 {
                    new.push('^');
                } else {
                    new.push_str(&format!("^{}", k + 1));
                }
                names.insert(
                    *p,
                    Name {
                        head: new,
                        generation: 0,
                    },
                );
                i += 1;
                first_parent_chain(&mut names, *p);
            }
        }
        if i == 0 {
            break;
        }
    }
    names
}

fn want_color(spec: Option<Option<String>>, cfg: &arks_config::ConfigSet) -> bool {
    let v = match spec {
        Some(None) => "always".to_string(),
        Some(Some(v)) => v,
        None => cfg
            .get("color.showbranch")
            .or_else(|| cfg.get("color.ui"))
            .unwrap_or_else(|| "auto".into()),
    };
    match v.to_ascii_lowercase().as_str() {
        "always" | "true" | "yes" | "on" => true,
        "auto" => std::io::IsTerminal::is_terminal(&std::io::stdout()),
        _ => false,
    }
}

/// Is `name` (as typed) the branch HEAD points at?
fn rev_is_head(head: Option<&str>, name: &str) -> bool {
    let Some(head) = head else {
        return false;
    };
    if name == head {
        return true;
    }
    let Some(short) = head.strip_prefix("refs/heads/") else {
        return false;
    };
    name == short || name.strip_prefix("heads/") == Some(short)
}

/// Display name of a ref found by `--all` / `--remotes` / a glob.
fn display_name(refname: &str, all_heads: bool) -> String {
    if let Some(s) = refname.strip_prefix("refs/heads/") {
        return s.to_string();
    }
    if let Some(s) = refname.strip_prefix("refs/remotes/") {
        if !all_heads {
            return s.to_string();
        }
    }
    refname.strip_prefix("refs/").unwrap_or(refname).to_string()
}

fn glob_matches(pattern: &str, refname: &str) -> bool {
    let target = if pattern.starts_with("refs/") {
        refname.to_string()
    } else {
        let slashes = pattern.matches('/').count();
        let parts: Vec<&str> = refname.split('/').collect();
        if parts.len() <= slashes + 1 {
            return false;
        }
        parts[parts.len() - slashes - 1..].join("/")
    };
    crate::cmd::branch_cmd::fnmatch(pattern.as_bytes(), target.as_bytes())
}

/// A reflog entry: (new oid, timestamp, message).
fn reflog_entries(repo: &Repository, refname: &str) -> Vec<(Oid, i64, String)> {
    let path = if refname == "HEAD" {
        repo.git_dir().join("logs").join("HEAD")
    } else {
        repo.common_dir().join("logs").join(refname)
    };
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for l in text.lines() {
        let (head, msg) = l.split_once('\t').unwrap_or((l, ""));
        let mut it = head.splitn(3, ' ');
        let _old = it.next();
        let Some(Ok(new)) = it.next().map(Oid::from_hex) else {
            continue;
        };
        let ident = it.next().unwrap_or("");
        let ts = ident
            .rsplit(' ')
            .nth(1)
            .and_then(|t| t.parse().ok())
            .unwrap_or(0);
        out.push((new, ts, msg.to_string()));
    }
    out.reverse();
    out
}

fn dwim_ref(repo: &Repository, name: &str) -> Option<String> {
    if name == "HEAD" {
        return Some("HEAD".into());
    }
    let cands = [
        name.to_string(),
        format!("refs/{name}"),
        format!("refs/tags/{name}"),
        format!("refs/heads/{name}"),
        format!("refs/remotes/{name}"),
        format!("refs/remotes/{name}/HEAD"),
    ];
    cands
        .into_iter()
        .find(|c| c.starts_with("refs/") && repo.refs().resolve(c).is_ok())
}

fn bad_ref(name: &str) -> CliError {
    CliError::fatal(format!("bad sha1 reference {name}"))
}

pub fn cmd_show_branch(args: &[String]) -> CliResult<()> {
    let p = parse("show-branch", args, OPTS, Default::default()).map_err(|e| match e {
        CliError::Exit(129, m) => CliError::Exit(
            129,
            m.split("\nusage:").next().unwrap_or("").to_string() + "\n" + USAGE,
        ),
        other => other,
    })?;
    let mut all_heads = p.on("all");
    let mut all_remotes = p.on("remotes");
    if all_heads {
        all_remotes = true;
    }
    // --list is --more=-1; the last of --more / --list wins.
    let mut extra: i64 = 0;
    for (k, v, neg) in p.opts() {
        match (k, neg) {
            ("more", false) => {
                extra = match v {
                    None => 1,
                    Some(v) => v.parse().map_err(|_| {
                        CliError::Exit(
                            129,
                            format!("error: option `more' expects a numerical value\n{USAGE}"),
                        )
                    })?,
                }
            }
            ("more", true) | ("list", true) => extra = 0,
            ("list", false) => extra = -1,
            _ => {}
        }
    }
    let no_name = p.on("no-name");
    let sha1_name = p.on("sha1-name");
    let merge_base = p.on("merge-base");
    let independent = p.on("independent");
    let topics = p.on("topics");
    let sparse = p.on("sparse");
    let date_order = p
        .opts()
        .rfind(|(k, _, _)| *k == "topo-order" || *k == "date-order")
        .is_some_and(|(k, _, _)| k == "date-order");
    let reflog = p.last("reflog").map(|v| v.unwrap_or_default());
    let mut operands = p.operands();
    let repo = open_repo()?;
    let cfg = arks_config::ConfigSet::load(Some(repo.common_dir()));
    let color = want_color(p.last("color"), &cfg);

    if reflog.is_some() && (0 < extra || all_heads || all_remotes || independent || merge_base) {
        return Err(CliError::fatal(
            "options '--reflog' and '--all/--remotes/--independent/--merge-base' cannot be used together",
        ));
    }
    if extra != 0 && (independent || merge_base) {
        return Err(usage_only());
    }
    // `--topics <one>` and no arguments at all show every branch.
    if reflog.is_none()
        && !all_heads
        && !all_remotes
        && (operands.is_empty() || (topics && operands.len() == 1))
    {
        if operands.is_empty() {
            operands = cfg.get_all("showbranch.default");
        }
        if operands.is_empty() || topics {
            all_heads = true;
        }
    }
    let head_ref = match repo.head_target() {
        Ok(RefTarget::Symbolic(s)) => Some(s),
        _ => None,
    };

    let mut ref_names: Vec<String> = Vec::new();
    let mut revs: Vec<Oid> = Vec::new();
    let mut reflog_msgs: Vec<String> = Vec::new();

    if let Some(spec) = reflog {
        if operands.len() > 1 {
            return Err(CliError::fatal("--reflog option needs one branch name"));
        }
        let (mut count, mut base) = (4usize, 0usize);
        if !spec.is_empty() {
            let bad = || CliError::Exit(129, format!("error: unrecognized reflog param '{spec}'"));
            let (n, b) = match spec.split_once(',') {
                Some((n, b)) => (n, Some(b)),
                None => (spec.as_str(), None),
            };
            count = n.parse().map_err(|_| bad())?;
            if let Some(b) = b {
                base = b.parse().map_err(|_| bad())?;
            }
            if count == 0 {
                count = 4;
            }
        }
        let (refname, shown) = match operands.first() {
            Some(n) => (
                dwim_ref(&repo, n).ok_or_else(|| CliError::fatal(format!("no such ref {n}")))?,
                n.clone(),
            ),
            None => match &head_ref {
                Some(h) => (h.clone(), h.clone()),
                None => ("HEAD".to_string(), "HEAD".to_string()),
            },
        };
        let entries = reflog_entries(&repo, &refname);
        if entries.is_empty() {
            return Err(CliError::fatal(format!("no reflog for '{shown}'")));
        }
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        for k in base..base + count {
            let Some((oid, _, _)) = entries.get(k) else {
                break;
            };
            let src = &entries[k.saturating_sub(1)];
            let msg = src.2.trim_end_matches('\n');
            ref_names.push(format!("{shown}@{{{k}}}"));
            revs.push(*oid);
            reflog_msgs.push(format!(
                "({}) {msg}",
                crate::cmd::date_spec::relative(src.1, now)
            ));
        }
    } else {
        let push = |names: &mut Vec<String>, revs: &mut Vec<Oid>, name: String, oid: Oid| {
            if names.contains(&name) {
                return;
            }
            names.push(name);
            revs.push(oid);
        };
        for a in &operands {
            if a.contains(['*', '?', '[']) {
                let mut matched = false;
                let mut refs = repo.refs().list_refs("refs/")?;
                refs.sort_by(|x, y| x.0.cmp(&y.0));
                for (r, oid) in refs {
                    if !glob_matches(a, &r) {
                        continue;
                    }
                    let Ok(c) = peel_to_kind(&repo, &oid, ObjKind::Commit, &r) else {
                        continue;
                    };
                    matched = true;
                    let name = if let Some(s) = r.strip_prefix("refs/heads/") {
                        s.to_string()
                    } else {
                        r.strip_prefix("refs/").unwrap_or(&r).to_string()
                    };
                    push(&mut ref_names, &mut revs, name, c);
                }
                if !matched {
                    eprintln!("error: no matching refs with {a}");
                }
                continue;
            }
            let oid = match resolve_rev(&repo, a) {
                Ok(o) => o,
                Err(_) => dwim_ref(&repo, a)
                    .and_then(|r| repo.refs().resolve(&r).ok())
                    .ok_or_else(|| bad_ref(a))?,
            };
            let c = peel_to_kind(&repo, &oid, ObjKind::Commit, a).map_err(|_| bad_ref(a))?;
            push(&mut ref_names, &mut revs, a.clone(), c);
        }
        let mut add_refs = |prefix: &str| -> CliResult<()> {
            let mut refs = repo.refs().list_refs(prefix)?;
            refs.sort_by(|x, y| x.0.cmp(&y.0));
            for (r, oid) in refs {
                if r.ends_with("/HEAD") && prefix == "refs/remotes/" {
                    continue;
                }
                let Ok(c) = peel_to_kind(&repo, &oid, ObjKind::Commit, &r) else {
                    continue;
                };
                push(&mut ref_names, &mut revs, display_name(&r, all_heads), c);
            }
            Ok(())
        };
        if all_heads {
            add_refs("refs/heads/")?;
        }
        if all_remotes {
            add_refs("refs/remotes/")?;
        }
        if p.on("current") {
            if let Some(h) = &head_ref {
                if !ref_names.iter().any(|n| rev_is_head(Some(h), n)) {
                    let short = h.strip_prefix("refs/heads/").unwrap_or(h).to_string();
                    if let Ok(oid) = repo.refs().resolve(h) {
                        ref_names.push(short);
                        revs.push(oid);
                    }
                }
            }
        }
    }

    let mut out = std::io::BufWriter::new(std::io::stdout().lock());
    if revs.is_empty() {
        eprintln!("No revs to be shown.");
        return Ok(());
    }
    let num_rev = revs.len();
    let head_at: Option<usize> = if reflog_msgs.is_empty() {
        ref_names
            .iter()
            .position(|n| rev_is_head(head_ref.as_deref(), n))
    } else {
        None
    };
    let mut walk = Walk {
        repo: &repo,
        nodes: HashMap::new(),
    };
    let subject_of = |walk: &mut Walk<'_>, oid: Oid| -> CliResult<String> {
        Ok(subject_line(&walk.node(oid)?.commit.message))
    };
    let header_text = |walk: &mut Walk<'_>, i: usize| -> CliResult<String> {
        Ok(match reflog_msgs.get(i) {
            Some(m) => m.clone(),
            None => subject_of(walk, revs[i])?,
        })
    };

    if extra < 0 {
        // --list: one line per rev.
        for (i, name) in ref_names.iter().enumerate() {
            let mark = if head_at == Some(i) { '*' } else { ' ' };
            let text = header_text(&mut walk, i)?;
            writeln!(out, "{mark} [{name}] {text}")?;
        }
        out.flush()?;
        return Ok(());
    }

    let seen = walk.join(&revs, extra)?;
    let all_revs: u64 = (1u64 << num_rev) - 1;

    if merge_base {
        let sorted = walk.sort(&seen, date_order);
        let mut found = false;
        for c in sorted {
            let f = walk.nodes[&c].flags;
            if f & UNINTERESTING == 0 && f & all_revs == all_revs {
                writeln!(out, "{}", c.to_hex())?;
                found = true;
                walk.nodes.get_mut(&c).expect("seen").flags |= UNINTERESTING;
            }
        }
        out.flush()?;
        return if found {
            Ok(())
        } else {
            Err(CliError::silent(1))
        };
    }
    if independent {
        for (i, r) in revs.iter().enumerate() {
            if walk.nodes[r].flags & all_revs == 1 << i {
                writeln!(out, "{}", r.to_hex())?;
            }
        }
        out.flush()?;
        return Ok(());
    }

    let paint = |i: usize, c: char| -> String {
        if color {
            format!("{}{c}\x1b[m", COLUMN_COLORS[i % COLUMN_COLORS.len()])
        } else {
            c.to_string()
        }
    };
    if num_rev > 1 {
        for (i, name) in ref_names.iter().enumerate() {
            let mark = if head_at == Some(i) { '*' } else { '!' };
            let text = header_text(&mut walk, i)?;
            writeln!(out, "{}{} [{name}] {text}", " ".repeat(i), paint(i, mark))?;
        }
        writeln!(out, "{}", "-".repeat(num_rev))?;
    }

    let sorted = walk.sort(&seen, date_order);
    let names = if sha1_name || no_name {
        HashMap::new()
    } else {
        name_commits(&walk, &sorted, &revs, &ref_names)
    };
    let mut shown_merge_point = false;
    for c in &sorted {
        let flags = walk.nodes[c].flags;
        let is_merge_point = flags & all_revs == all_revs;
        shown_merge_point |= is_merge_point;
        if num_rev > 1 {
            let is_merge = walk.nodes[c].commit.parents.len() > 1;
            if topics && !is_merge_point && flags & 1 != 0 {
                continue;
            }
            if !sparse && is_merge && !revs.contains(c) && (flags & all_revs).count_ones() == 1 {
                continue;
            }
            let mut line = String::new();
            for i in 0..num_rev {
                if flags & (1 << i) == 0 {
                    line.push(' ');
                    continue;
                }
                let mark = if is_merge {
                    '-'
                } else if head_at == Some(i) {
                    '*'
                } else {
                    '+'
                };
                line.push_str(&paint(i, mark));
            }
            write!(out, "{line} ")?;
        }
        if !no_name {
            let label = match names.get(c) {
                Some(n) => n.render(),
                None => abbrev_hex(&repo, c, 7),
            };
            write!(out, "[{label}] ")?;
        }
        writeln!(out, "{}", subject_of(&mut walk, *c)?)?;
        if shown_merge_point {
            extra -= 1;
            if extra < 0 {
                break;
            }
        }
    }
    out.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn head_detection() {
        let h = Some("refs/heads/main");
        assert!(rev_is_head(h, "main"));
        assert!(rev_is_head(h, "heads/main"));
        assert!(rev_is_head(h, "refs/heads/main"));
        assert!(!rev_is_head(h, "HEAD"));
        assert!(!rev_is_head(None, "main"));
    }

    #[test]
    fn globs_match_trailing_components() {
        assert!(glob_matches("v*", "refs/tags/v1"));
        assert!(glob_matches("heads/*", "refs/heads/main"));
        assert!(glob_matches("refs/tags/*", "refs/tags/t0"));
        assert!(!glob_matches("heads/*", "refs/tags/v1"));
        assert_eq!(display_name("refs/remotes/o/m", true), "remotes/o/m");
        assert_eq!(display_name("refs/remotes/o/m", false), "o/m");
    }

    #[test]
    fn names_render() {
        let n = Name {
            head: "main".into(),
            generation: 2,
        };
        assert_eq!(n.render(), "main~2");
    }
}
