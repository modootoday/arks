//! `log`, `show` and `whatchanged`: option parsing (revision options from
//! `revwalk`, diff options from `diffcore`, log formatting options),
//! commit headers (medium / short / full / fuller / raw / oneline / user
//! formats), decorations, merge diffs (`-m`, `-c`, `--cc`,
//! `--diff-merges`) and per-commit diff output.
//!
//! Record layout follows git: separator-style formats (medium, `format:`)
//! put a newline *between* commits; terminator-style formats (oneline,
//! `tformat:`) end every commit with one. When a diff follows a
//! non-oneline header, one blank line (or `---` when both a diffstat and a
//! patch are shown) precedes it.
//!
//! Primary: <https://git-scm.com/docs/git-log>, <https://git-scm.com/docs/git-show>,
//! <https://git-scm.com/docs/git-whatchanged>, <https://git-scm.com/docs/pretty-formats>.
//!
//! `-L` line-range history lives in `line_log`.
//!
//! Partial: `--follow` rename following is a plain pathspec limit; `log --graph` draws the lane graph
//! over a topo-ordered walk; `show --graph` prints each commit as a lone `*`.

use crate::cmd::diffcore::combined;
use crate::cmd::diffcore::emit::{EmitCtx, emit};
use crate::cmd::diffcore::options::{Args, DiffOptions};
use crate::cmd::diffcore::{self, Content, Pair};
use crate::cmd::log_format::{DateMode, FmtCtx, Pretty, expand_ctx, read_commit, take_pretty};
use crate::cmd::log_graph::Graph;
use crate::cmd::mailmap::Mailmap;
use crate::cmd::revwalk::{self, RevOpts, Walked};
use crate::cmd::shared::{abbrev_hex, resolve_rev, tag_target};
use crate::error::{CliError, CliResult};
use crate::util::{open_repo, work_tree};
use arks_object::{Commit, ObjectKind, Signature};
use arks_oid::Oid;
use arks_refs::RefTarget;
use arks_repo::Repository;
use std::collections::{HashMap, HashSet};
use std::io::{self, Write};
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DiffMerges {
    Off,
    FirstParent,
    Separate,
    Combined,
    Dense,
    /// `--remerge-diff`: diff against an in-memory re-merge of the parents.
    Remerge,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Decorate {
    No,
    Short,
    Full,
}

/// Options shared by `log`, `show` and `whatchanged` after parsing.
struct LogOpts {
    pretty: Pretty,
    diff: DiffOptions,
    rev: RevOpts,
    date: DateMode,
    decorate: Option<Decorate>,
    decorate_refs: Vec<String>,
    decorate_refs_exclude: Vec<String>,
    graph: bool,
    abbrev_commit: Option<bool>,
    abbrev: usize,
    source: bool,
    mailmap: bool,
    quiet: bool,
    merges: Option<DiffMerges>,
    combined_all_paths: bool,
    parents: bool,
    children: bool,
    log_size: bool,
    /// The user asked for a diff format (`-p`, `--stat`, ...).
    diff_given: bool,
    /// `-L <range>:<file>` values (line-range history).
    line_ranges: Vec<String>,
}

fn parse_opts(args: &[String], cmd: &str) -> CliResult<(LogOpts, Vec<String>)> {
    let (head, paths): (Vec<String>, Vec<String>) = match args.iter().position(|a| a == "--") {
        Some(i) => (args[..i].to_vec(), args[i + 1..].to_vec()),
        None => (args.to_vec(), Vec::new()),
    };
    let mut head = head;
    // `--oneline` implies `--abbrev-commit`; the later of it and
    // `--no-abbrev-commit` wins.
    let last = |f: &str| head.iter().rposition(|a| a == f);
    let oneline_abbrevs = match (last("--oneline"), last("--no-abbrev-commit")) {
        (Some(a), Some(b)) => a > b,
        _ => false,
    };
    let pretty = take_pretty(&mut head)?;
    let remerge_patch = head.iter().any(|a| a == "--remerge-diff");
    let repo = open_repo().ok();
    let mut diff = DiffOptions::default();
    crate::cmd::diff_cmd::porcelain_defaults(repo.as_ref(), &mut diff);
    let mut o = LogOpts {
        pretty,
        diff,
        rev: RevOpts::default(),
        date: DateMode::Default,
        decorate: None,
        decorate_refs: Vec::new(),
        decorate_refs_exclude: Vec::new(),
        graph: false,
        abbrev_commit: None,
        abbrev: 7,
        source: false,
        mailmap: false,
        quiet: false,
        merges: None,
        combined_all_paths: false,
        parents: false,
        children: false,
        log_size: false,
        diff_given: false,
        line_ranges: Vec::new(),
    };
    if let Some(r) = &repo {
        let cfg = arks_config::ConfigSet::load(Some(r.common_dir()));
        if cfg.get_bool("log.mailmap") == Some(true) {
            o.mailmap = true;
        }
        if let Some(d) = cfg.get("log.date") {
            o.date = DateMode::parse(&d).unwrap_or_default();
        }
        if cfg.get_bool("log.abbrevcommit") == Some(true) {
            o.abbrev_commit = Some(true);
        }
        match cfg.get("log.decorate").as_deref() {
            Some("full") => o.decorate = Some(Decorate::Full),
            Some("short") | Some("true") | Some("yes") | Some("1") => {
                o.decorate = Some(Decorate::Short)
            }
            _ => {}
        }
    }
    let mut a = Args::new(&head);
    while a.i < head.len() {
        let cur = a.cur().to_string();
        let before = (o.diff.any_format(), o.diff.no_output);
        if o.diff.take(&mut a)? {
            if (o.diff.any_format(), o.diff.no_output) != before {
                o.diff_given = true;
            }
            a.i += 1;
            continue;
        }
        if o.rev.take(&mut a)? {
            a.i += 1;
            continue;
        }
        if take_log_opt(&mut o, &mut a)? {
            a.i += 1;
            continue;
        }
        if cur.starts_with('-') && cur.len() > 1 {
            return Err(CliError::NotImplemented(format!(
                "{cmd}: unsupported option: {cur}"
            )));
        }
        // Pushed in order so `--not` negates only what follows it.
        o.rev.push_rev(&cur);
        a.i += 1;
    }
    o.diff.finish();
    if oneline_abbrevs {
        o.abbrev_commit = Some(true);
    }
    // `--abbrev` is a diff option too; it also sets the commit-id length.
    if o.diff.abbrev_given {
        o.abbrev = match o.diff.abbrev {
            Some(0) => 40,
            Some(n) => n.clamp(4, 40),
            None => 7,
        };
    }
    // `--remerge-diff` (not `--diff-merges=remerge`) without a format implies `-p`.
    if remerge_patch && o.merges == Some(DiffMerges::Remerge) && !o.diff_given {
        o.diff.patch = true;
        o.diff_given = true;
    }
    // In log, `-c` / `--cc` without a format imply `-p`.
    if cmd != "show"
        && matches!(o.merges, Some(DiffMerges::Combined | DiffMerges::Dense))
        && !o.diff_given
    {
        o.diff.patch = true;
        o.diff_given = true;
    }
    Ok((o, paths))
}

fn take_log_opt(o: &mut LogOpts, a: &mut Args<'_>) -> CliResult<bool> {
    let cur = a.cur();
    match cur {
        "--graph" => o.graph = true,
        "--no-graph" => o.graph = false,
        "-q" | "--quiet" => o.quiet = true,
        "--no-quiet" => o.quiet = false,
        "--source" => o.source = true,
        "--no-source" => o.source = false,
        "--use-mailmap" | "--mailmap" => o.mailmap = true,
        "--no-use-mailmap" | "--no-mailmap" => o.mailmap = false,
        "--decorate" => o.decorate = Some(Decorate::Short),
        "--no-decorate" => o.decorate = Some(Decorate::No),
        "--clear-decorations" => {
            o.decorate_refs.clear();
            o.decorate_refs_exclude.clear();
        }
        "--no-decorate-refs" => o.decorate_refs.clear(),
        "--no-decorate-refs-exclude" => o.decorate_refs_exclude.clear(),
        "--abbrev-commit" => o.abbrev_commit = Some(true),
        "--no-abbrev-commit" => o.abbrev_commit = Some(false),
        "--no-abbrev" => o.abbrev = 40,
        "--parents" => o.parents = true,
        "--children" => o.children = true,
        "--log-size" => o.log_size = true,
        "--left-right" => o.rev.left_right = true,
        "-m" => o.merges = Some(DiffMerges::Separate),
        // `-c` / `--cc` reset an earlier `--combined-all-paths`.
        "-c" => {
            o.merges = Some(DiffMerges::Combined);
            o.combined_all_paths = false;
        }
        "--cc" => {
            o.merges = Some(DiffMerges::Dense);
            o.combined_all_paths = false;
        }
        "--dd" => {
            o.merges = Some(DiffMerges::FirstParent);
            o.diff.patch = true;
            o.diff_given = true;
        }
        "--no-diff-merges" => o.merges = Some(DiffMerges::Off),
        "--combined-all-paths" => o.combined_all_paths = true,
        "--relative-date" => o.date = DateMode::parse("relative")?,
        // Notes refs and signatures: with none present the output is the same.
        "--notes"
        | "--no-notes"
        | "--show-notes"
        | "--standard-notes"
        | "--no-standard-notes"
        | "--show-signature"
        | "--no-show-signature"
        | "--expand-tabs"
        | "--no-expand-tabs"
        | "--show-linear-break" => {}
        "--remerge-diff" => o.merges = Some(DiffMerges::Remerge),
        _ => {
            if let Some(v) = a.long_opt("--decorate") {
                o.decorate = Some(match v.as_deref() {
                    None | Some("short") | Some("auto") => Decorate::Short,
                    Some("full") => Decorate::Full,
                    Some("no") => Decorate::No,
                    Some(x) => {
                        return Err(CliError::fatal(format!("invalid --decorate option: {x}")));
                    }
                });
            } else if let Some(v) = a.long_req("--decorate-refs")? {
                o.decorate_refs.push(v);
            } else if let Some(v) = a.long_req("--decorate-refs-exclude")? {
                o.decorate_refs_exclude.push(v);
            } else if let Some(v) = a.long_req("--date")? {
                o.date = DateMode::parse(&v)?;
            } else if let Some(v) = a.long_opt("--abbrev") {
                o.abbrev = match v {
                    Some(n) => {
                        let n: usize = n
                            .parse()
                            .map_err(|_| CliError::fatal("--abbrev expects a numerical value"))?;
                        if n == 0 { 40 } else { n.clamp(4, 40) }
                    }
                    None => 7,
                };
                o.diff.abbrev = Some(o.abbrev);
            } else if let Some(v) = a.long_req("--diff-merges")? {
                o.merges = Some(match v.as_str() {
                    "off" | "none" => DiffMerges::Off,
                    "on" | "m" | "separate" => DiffMerges::Separate,
                    "first-parent" | "1" => DiffMerges::FirstParent,
                    "combined" | "c" => DiffMerges::Combined,
                    "dense-combined" | "cc" => DiffMerges::Dense,
                    "remerge" | "r" => DiffMerges::Remerge,
                    x => {
                        return Err(CliError::fatal(format!(
                            "invalid value for '--diff-merges': '{x}'"
                        )));
                    }
                });
            } else if a.long_req("--encoding")?.is_some()
                || a.long_opt("--expand-tabs").is_some()
                || a.long_opt("--notes").is_some()
                || a.long_opt("--show-notes").is_some()
            {
                // UTF-8 commits, no notes: nothing changes.
            } else if let Some(v) = a.short_req('L')? {
                o.line_ranges.push(v);
            } else {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

/// Worktree-relative pathspecs from cwd-relative arguments.
fn pathspecs(repo: &Repository, paths: &[String]) -> Vec<String> {
    let work = work_tree(repo).ok();
    paths
        .iter()
        .map(|p| {
            let abs = std::env::current_dir()
                .map(|c| c.join(p))
                .unwrap_or_else(|_| PathBuf::from(p));
            let rel = work
                .as_ref()
                .and_then(|w| {
                    abs.strip_prefix(w)
                        .ok()
                        .map(|r| r.to_string_lossy().replace('\\', "/"))
                })
                .unwrap_or_else(|| p.clone());
            let rel = rel.trim_end_matches('/').trim_start_matches("./");
            if rel == "." {
                String::new()
            } else {
                rel.to_string()
            }
        })
        .collect()
}

fn refname_matches(pat: &str, name: &str) -> bool {
    let full = if pat.starts_with("refs/") {
        pat.to_string()
    } else {
        format!("refs/{pat}")
    };
    let globbed = if full.contains(['*', '?', '[']) {
        full
    } else {
        format!("{}/*", full.trim_end_matches('/'))
    };
    crate::cmd::branch_cmd::fnmatch(globbed.as_bytes(), name.as_bytes())
        || name == pat
        || name == format!("refs/{pat}")
}

/// Ref decorations per commit, in git order: `HEAD` (as `HEAD -> <branch>`
/// when attached) first, then refs in reverse refname order; tags as
/// `tag: <name>`, annotated tags on their target.
pub(crate) fn decoration_labels(repo: &Repository) -> CliResult<HashMap<Oid, Vec<String>>> {
    decorations_filtered(repo, false, &[], &[])
}

fn decorations_filtered(
    repo: &Repository,
    full: bool,
    include: &[String],
    exclude: &[String],
) -> CliResult<HashMap<Oid, Vec<String>>> {
    let mut map: HashMap<Oid, Vec<String>> = HashMap::new();
    let head_branch = match repo.head_target() {
        Ok(RefTarget::Symbolic(s)) => Some(s),
        _ => None,
    };
    let keep = |name: &str| -> bool {
        (include.is_empty() || include.iter().any(|p| refname_matches(p, name)))
            && !exclude.iter().any(|p| refname_matches(p, name))
    };
    let mut refs = repo.refs().list_refs("refs/")?;
    refs.sort_by(|a, b| a.0.cmp(&b.0));
    for (name, oid) in refs.iter().rev() {
        if Some(name) == head_branch.as_ref() && keep("HEAD") {
            continue;
        }
        if !keep(name) {
            continue;
        }
        let label = if full {
            if name.starts_with("refs/tags/") {
                format!("tag: {name}")
            } else {
                name.clone()
            }
        } else if let Some(t) = name.strip_prefix("refs/tags/") {
            format!("tag: {t}")
        } else if let Some(h) = name.strip_prefix("refs/heads/") {
            h.to_string()
        } else if let Some(r) = name.strip_prefix("refs/remotes/") {
            r.to_string()
        } else {
            name.clone()
        };
        let mut target = *oid;
        if let Ok(obj) = repo.get_object(oid) {
            if obj.kind == ObjectKind::Tag {
                if let Some(t) = tag_target(&obj.payload) {
                    target = crate::cmd::shared::peel_tags(repo, &t).unwrap_or(t);
                }
            }
        }
        map.entry(target).or_default().push(label);
    }
    if keep("HEAD") {
        if let Ok(head) = repo.head_oid() {
            let label = match &head_branch {
                Some(b) if keep(b) => format!(
                    "HEAD -> {}",
                    if full {
                        b.as_str()
                    } else {
                        b.strip_prefix("refs/heads/").unwrap_or(b.as_str())
                    }
                ),
                _ => "HEAD".to_string(),
            };
            map.entry(head).or_default().insert(0, label);
        }
    }
    Ok(map)
}

/// Commit message as printed by the medium format: leading blank lines
/// dropped, trailing whitespace trimmed, every line indented by four spaces.
fn indented_message(msg: &str) -> String {
    let mut out = String::new();
    let mut started = false;
    for line in msg.trim_end().lines() {
        if !started && line.trim().is_empty() {
            continue;
        }
        started = true;
        out.push_str("    ");
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// Per-run formatting state for commit headers.
struct HeaderFmt<'a> {
    repo: &'a Repository,
    pretty: Pretty,
    date: DateMode,
    decorate: bool,
    abbrev: Option<usize>,
    parents: bool,
    children: Option<&'a HashMap<Oid, Vec<Oid>>>,
    source: Option<&'a HashMap<Oid, String>>,
    mailmap: Option<&'a Mailmap>,
    log_size: bool,
    ctx: FmtCtx<'a>,
}

impl HeaderFmt<'_> {
    fn hex(&self, oid: &Oid) -> String {
        match self.abbrev {
            Some(n) if n < 40 => abbrev_hex(self.repo, oid, n),
            _ => oid.to_hex(),
        }
    }

    fn person(&self, label: &str, sig: &Signature) -> String {
        let (n, e) = match self.mailmap {
            Some(m) => m.map(&sig.name, &sig.email),
            None => (sig.name.clone(), sig.email.clone()),
        };
        format!("{label}{n} <{e}>\n")
    }

    /// `commit` line (or oneline prefix) pieces: marks, id, parents /
    /// children, `(from <parent>)`, decorations and source.
    fn id_line(
        &self,
        oid: &Oid,
        c: &Commit,
        mark: &str,
        from: Option<&Oid>,
        oneline: bool,
    ) -> String {
        let mut s = format!("{mark}{}", self.hex(oid));
        if self.parents {
            for p in &c.parents {
                s.push(' ');
                s.push_str(&self.hex(p));
            }
        }
        if let Some(ch) = self.children {
            for x in ch.get(oid).into_iter().flatten() {
                s.push(' ');
                s.push_str(&self.hex(x));
            }
        }
        if let Some(p) = from {
            s.push_str(&format!(" (from {})", self.hex(p)));
        }
        if self.decorate {
            let d = self.ctx.decoration(oid);
            if !d.is_empty() {
                s.push_str(&format!(" ({d})"));
            }
        }
        if let Some(src) = self.source.and_then(|m| m.get(oid)) {
            s.push('\t');
            s.push_str(src);
        }
        let _ = oneline;
        s
    }

    /// `commit <id>` line, then `log size <n>` (bytes of the rest of the
    /// record) when `--log-size` is on, then the rest.
    fn with_commit_line(
        &self,
        oid: &Oid,
        c: &Commit,
        mark: &str,
        from: Option<&Oid>,
        rest: String,
    ) -> String {
        let mut s = format!("commit {}\n", self.id_line(oid, c, mark, from, false));
        if self.log_size {
            s.push_str(&format!("log size {}\n", rest.len()));
        }
        s.push_str(&rest);
        s
    }

    fn header(&self, oid: &Oid, c: &Commit, mark: &str, from: Option<&Oid>) -> String {
        let commit_line = || {
            let mut s = String::new();
            if c.parents.len() > 1 && !matches!(self.pretty, Pretty::Raw) {
                let ps: Vec<String> = c
                    .parents
                    .iter()
                    .map(|p| abbrev_hex(self.repo, p, self.abbrev.unwrap_or(7).min(40)))
                    .collect();
                s.push_str(&format!("Merge: {}\n", ps.join(" ")));
            }
            s
        };
        match &self.pretty {
            Pretty::Default => {
                let mut s = commit_line();
                s.push_str(&self.person("Author: ", &c.author));
                s.push_str(&format!("Date:   {}\n", self.date.format(&c.author)));
                s.push('\n');
                s.push_str(&indented_message(&c.message));
                self.with_commit_line(oid, c, mark, from, s)
            }
            Pretty::Short => {
                let mut s = commit_line();
                s.push_str(&self.person("Author: ", &c.author));
                s.push('\n');
                s.push_str(&indented_message(&crate::cmd::log_format::subject_line(
                    &c.message,
                )));
                self.with_commit_line(oid, c, mark, from, s)
            }
            Pretty::Full => {
                let mut s = commit_line();
                s.push_str(&self.person("Author: ", &c.author));
                s.push_str(&self.person("Commit: ", &c.committer));
                s.push('\n');
                s.push_str(&indented_message(&c.message));
                self.with_commit_line(oid, c, mark, from, s)
            }
            Pretty::Fuller => {
                let mut s = commit_line();
                s.push_str(&self.person("Author:     ", &c.author));
                s.push_str(&format!("AuthorDate: {}\n", self.date.format(&c.author)));
                s.push_str(&self.person("Commit:     ", &c.committer));
                s.push_str(&format!("CommitDate: {}\n", self.date.format(&c.committer)));
                s.push('\n');
                s.push_str(&indented_message(&c.message));
                self.with_commit_line(oid, c, mark, from, s)
            }
            Pretty::Raw => {
                let mut s = format!("commit {}\n", self.id_line(oid, c, mark, from, false));
                s.push_str(&format!("tree {}\n", c.tree.to_hex()));
                for p in &c.parents {
                    s.push_str(&format!("parent {}\n", p.to_hex()));
                }
                let sig = |l: &str, g: &Signature| {
                    format!(
                        "{l} {} <{}> {} {}\n",
                        g.name, g.email, g.timestamp, g.timezone
                    )
                };
                s.push_str(&sig("author", &c.author));
                s.push_str(&sig("committer", &c.committer));
                s.push('\n');
                s.push_str(&indented_message(&c.message));
                s
            }
            Pretty::Oneline => format!(
                "{} {}\n",
                self.id_line(oid, c, mark, from, true),
                crate::cmd::log_format::subject_line(&c.message)
            ),
            Pretty::TFormat(f) => format!("{}{}\n", mark_for_format(mark), self.expand(f, oid, c)),
            Pretty::Format(f) => format!("{}{}", mark_for_format(mark), self.expand(f, oid, c)),
        }
    }

    fn expand(&self, f: &str, oid: &Oid, c: &Commit) -> String {
        match self.mailmap {
            Some(m)
                if f.contains("%aN")
                    || f.contains("%aE")
                    || f.contains("%cN")
                    || f.contains("%cE") =>
            {
                let mut c2 = c.clone();
                let (an, ae) = m.map(&c.author.name, &c.author.email);
                let (cn, ce) = m.map(&c.committer.name, &c.committer.email);
                let f = f
                    .replace("%aN", &an)
                    .replace("%aE", &ae)
                    .replace("%cN", &cn)
                    .replace("%cE", &ce);
                c2.message = c.message.clone();
                expand_ctx(&f, oid, &c2, &self.ctx)
            }
            _ => expand_ctx(f, oid, c, &self.ctx),
        }
    }
}

/// User formats show `--left-right` / `--boundary` marks before the text.
fn mark_for_format(mark: &str) -> &str {
    mark
}

/// One commit header as `diff-tree --pretty=<spec>` prints it.
pub(crate) fn format_commit_header(
    repo: &Repository,
    oid: &Oid,
    c: &Commit,
    spec: &str,
) -> CliResult<String> {
    let mut v = vec![format!("--pretty={spec}")];
    let pretty = take_pretty(&mut v)?;
    let h = HeaderFmt {
        repo,
        pretty,
        date: DateMode::Default,
        decorate: false,
        abbrev: None,
        parents: false,
        children: None,
        source: None,
        mailmap: None,
        log_size: false,
        ctx: FmtCtx::default(),
    };
    Ok(h.header(oid, c, "", None))
}

/// Writes commit records with git's separator / terminator rules.
struct Printer<'a> {
    content: &'a Content<'a>,
    opts: &'a LogOpts,
    fmt: HeaderFmt<'a>,
    shown_one: bool,
    graph: Option<Graph<Oid>>,
    missing_newline: bool,
}

impl Printer<'_> {
    fn uses_terminator(&self) -> bool {
        matches!(self.opts.pretty, Pretty::Oneline | Pretty::TFormat(_))
    }

    fn format_is_empty(&self) -> bool {
        matches!(&self.opts.pretty, Pretty::Format(f) | Pretty::TFormat(f) if f.is_empty())
    }

    /// Render the diff part of a record (after the header).
    fn diff_text(&self, diff: &RecordDiff, force_sep: bool) -> CliResult<String> {
        if let RecordDiff::Text(t) = diff {
            return Ok(t.clone());
        }
        let d = &self.opts.diff;
        let empty = match diff {
            RecordDiff::None => true,
            RecordDiff::Pairs(p) => p.is_empty(),
            RecordDiff::Combined(e, _, _) => e.is_empty(),
            RecordDiff::Text(t) => t.is_empty(),
            RecordDiff::Remerge(p, _) => p.is_empty(),
        };
        if !d.any_format() || d.no_output || (empty && !force_sep) {
            return Ok(String::new());
        }
        let mut text = String::new();
        let is_combined = matches!(diff, RecordDiff::Combined(..));
        // Merges' combined output keeps the blank line even after --oneline.
        if (is_combined || !matches!(self.opts.pretty, Pretty::Oneline)) && !self.format_is_empty()
        {
            text.push_str(&d.line_prefix);
            if !is_combined
                && (d.stat || d.numstat || d.shortstat || d.dirstat.is_some() || d.summary)
                && d.patch
                && !d.name_only
                && !d.name_status
            {
                text.push_str("---");
            }
            text.push('\n');
        }
        let mut buf = Vec::new();
        match diff {
            RecordDiff::None | RecordDiff::Text(_) => {}
            RecordDiff::Pairs(p) => emit(&mut buf, self.content, d, EmitCtx::default(), p)?,
            RecordDiff::Remerge(p, msgs) => {
                let mut b = Vec::new();
                emit(&mut b, self.content, d, EmitCtx::default(), p)?;
                let t = crate::cmd::remerge::annotate(&String::from_utf8_lossy(&b), msgs);
                buf.extend_from_slice(t.as_bytes());
            }
            RecordDiff::Combined(e, mode, fp) => combined::emit(
                &mut buf,
                self.content,
                d,
                *mode,
                self.opts.combined_all_paths,
                EmitCtx::default(),
                fp,
                e,
            )?,
        }
        text.push_str(&String::from_utf8_lossy(&buf));
        Ok(text)
    }

    /// One commit record.
    #[allow(clippy::too_many_arguments)]
    fn commit(
        &mut self,
        out: &mut impl Write,
        oid: &Oid,
        c: &Commit,
        mark: &str,
        from: Option<&Oid>,
        diff: &RecordDiff,
        force_sep: bool,
    ) -> CliResult<()> {
        let header = self.fmt.header(oid, c, mark, from);
        let diff_text = self.diff_text(diff, force_sep)?;
        if self.graph.is_some() {
            return self.commit_with_graph(out, &header, &diff_text);
        }
        let lp = self.opts.diff.line_prefix.clone();
        if self.shown_one && !self.uses_terminator() {
            write!(out, "{lp}")?;
            writeln!(out)?;
        }
        self.shown_one = true;
        let text = header + &diff_text;
        if self.opts.graph {
            // `show --graph`: every record is a lone commit.
            for (i, line) in text.split_inclusive('\n').enumerate() {
                out.write_all(if i == 0 { b"* " } else { b"  " })?;
                out.write_all(line.as_bytes())?;
            }
        } else if lp.is_empty() {
            out.write_all(text.as_bytes())?;
        } else {
            // Diff lines carry the prefix already; prefix header lines.
            let hlen = text.len() - diff_text.len();
            for line in text[..hlen].split_inclusive('\n') {
                out.write_all(lp.as_bytes())?;
                out.write_all(line.as_bytes())?;
            }
            out.write_all(diff_text.as_bytes())?;
        }
        Ok(())
    }

    /// [`Printer::commit`] for `log --graph`.
    fn commit_with_graph(
        &mut self,
        out: &mut impl Write,
        header: &str,
        diff_text: &str,
    ) -> CliResult<()> {
        let terminator = self.uses_terminator();
        let format_is_empty = self.format_is_empty();
        let Some(g) = self.graph.as_mut() else {
            return Ok(());
        };
        if self.shown_one && !terminator {
            if !self.missing_newline {
                out.write_all(g.separator_row().as_bytes())?;
            }
            writeln!(out)?;
        }
        self.shown_one = true;
        let msg = if terminator {
            header.strip_suffix('\n').unwrap_or(header)
        } else {
            header
        };
        self.missing_newline = !msg.ends_with('\n');
        while !g.commit_row_next() && !g.finished() {
            writeln!(out, "{}", g.next_row())?;
        }
        out.write_all(g.next_row().as_bytes())?;
        for (i, line) in msg.split_inclusive('\n').enumerate() {
            if i > 0 {
                out.write_all(g.next_row().as_bytes())?;
            }
            out.write_all(line.as_bytes())?;
        }
        if !g.finished() {
            if !msg.is_empty() && !msg.ends_with('\n') {
                writeln!(out)?;
            }
            let mut first = true;
            while !g.finished() {
                if !first {
                    writeln!(out)?;
                }
                first = false;
                out.write_all(g.next_row().as_bytes())?;
            }
            if msg.ends_with('\n') {
                writeln!(out)?;
            }
        }
        if terminator && !format_is_empty {
            if !self.missing_newline {
                out.write_all(g.next_row().as_bytes())?;
            }
            writeln!(out)?;
        }
        for line in diff_text.split_inclusive('\n') {
            out.write_all(g.next_row().as_bytes())?;
            out.write_all(line.as_bytes())?;
        }
        Ok(())
    }
}

/// The diff shown with one record.
enum RecordDiff {
    None,
    Pairs(Vec<Pair>),
    /// Combined entries, mode, and the first-parent pairs that stat
    /// formats describe.
    Combined(Vec<combined::Entry>, combined::Mode, Vec<Pair>),
    /// Pre-rendered text after the header (`-L` patches).
    Text(String),
    /// `--remerge-diff` pairs and their conflict messages.
    Remerge(Vec<Pair>, Vec<(String, String)>),
}

/// Diff of `c` against `parent` (root: the empty tree) through diffcore.
fn diff_against(
    content: &Content<'_>,
    o: &DiffOptions,
    parent: Option<&Oid>,
    c: &Commit,
    specs: &[String],
) -> CliResult<Vec<Pair>> {
    let repo = content.repo;
    let pt = match parent {
        Some(p) => Some(read_commit(repo, p)?.tree),
        None => None,
    };
    let mut pairs = diffcore::tree_pairs(repo, pt.as_ref(), Some(&c.tree), true, false, specs)?;
    let unchanged = if o.find_copies_harder {
        match &pt {
            Some(t) => crate::cmd::diffcore::sources::tree_sides(repo, t)?
                .into_values()
                .filter(|s| diffcore::spec_match(&s.path, specs))
                .collect(),
            None => Vec::new(),
        }
    } else {
        Vec::new()
    };
    diffcore::transform_with_sources(content, o, &mut pairs, &unchanged)?;
    Ok(pairs)
}

fn pretty_uses_decoration(p: &Pretty) -> bool {
    matches!(p, Pretty::Format(f) | Pretty::TFormat(f) if f.contains("%d") || f.contains("%D"))
}

fn mark_of(w: &Walked, o: &LogOpts) -> String {
    if w.boundary {
        return "- ".into();
    }
    if let Some(c) = w.cherry {
        return format!("{c} ");
    }
    if o.rev.left_right {
        if let Some(s) = w.side {
            return format!("{s} ");
        }
    }
    String::new()
}

/// Shared driver for `log` and `whatchanged`.
/// `-L`: keep the commits that change the tracked ranges (newest first,
/// before `-n` / `--skip`) and return their patches (`\n` + text).
fn line_log_select(
    repo: &Repository,
    opts: &LogOpts,
    paths: &[String],
    list: &mut Vec<Walked>,
) -> CliResult<HashMap<Oid, String>> {
    if !paths.is_empty() {
        return Err(CliError::fatal(
            "-L<range>:<file> cannot be used with pathspec",
        ));
    }
    let d = &opts.diff;
    if d.raw
        || d.name_only
        || d.name_status
        || d.numstat
        || d.stat
        || d.shortstat
        || d.dirstat.is_some()
        || d.summary
        || d.check
    {
        return Err(CliError::fatal(
            "-L does not yet support diff formats besides -p and -s",
        ));
    }
    let work = work_tree(repo)?;
    let specs = crate::cmd::line_log::parse_specs(&opts.line_ranges, &work)?;
    let tips = revwalk::resolve_tips(repo, &opts.rev, Some("HEAD"))?;
    let Some(tip) = tips.include.first().copied() else {
        list.clear();
        return Ok(HashMap::new());
    };
    let pairs: Vec<(Oid, Commit)> = list.iter().map(|w| (w.oid, w.commit.clone())).collect();
    let shown = crate::cmd::line_log::filter(repo, &pairs, tip, &specs, opts.rev.first_parent)?;
    let mut patches = HashMap::new();
    let mut keep = Vec::new();
    let mut all: Vec<Option<Walked>> = list.drain(..).map(Some).collect();
    for s in shown {
        if let Some(w) = all[s.index].take() {
            patches.insert(w.oid, format!("\n{}", s.patch.unwrap_or_default()));
            keep.push(w);
        }
    }
    *list = keep;
    Ok(patches)
}

fn run_log(args: &[String], cmd: &str) -> CliResult<()> {
    let (mut opts, paths) = parse_opts(args, cmd)?;
    if cmd == "whatchanged" && !opts.diff.any_format() {
        opts.diff.raw = true;
        opts.diff_given = true;
    }
    if opts.graph && opts.rev.reverse {
        return Err(CliError::fatal(
            "options '--reverse' and '--graph' cannot be used together",
        ));
    }
    let repo = open_repo()?;
    opts.rev.specs = pathspecs(&repo, &paths);
    if opts.parents || opts.children || opts.graph {
        opts.rev.rewrite = true;
    }
    if opts.graph && opts.rev.order.is_none() {
        opts.rev.order = Some(revwalk::Order::Topo);
    }
    if opts.diff.follow && opts.rev.specs.len() != 1 {
        return Err(CliError::fatal("--follow requires exactly one pathspec"));
    }
    let mut list = if opts.rev.walk_reflogs {
        revwalk::walk_reflogs(&repo, &opts.rev)?
    } else {
        let tips = revwalk::resolve_tips(&repo, &opts.rev, Some("HEAD"))?;
        if tips.include.is_empty() && tips.given.is_empty() {
            return Ok(());
        }
        // Diff-based limiting and the graph see past `-n` / `--skip`:
        // walk unlimited, limit (and reverse) afterwards.
        let mut ro = opts.rev.clone();
        ro.max_count = None;
        ro.skip = 0;
        ro.reverse = false;
        let mut l = revwalk::walk(&repo, &ro, &tips)?;
        if opts.source {
            let src = sources(&repo, &tips, &l)?;
            SOURCE.with(|s| *s.borrow_mut() = src);
        }
        if opts.rev.left_right {
            // sides already set by the walk
        }
        l.retain(|_| true);
        l
    };
    let line_log = if opts.line_ranges.is_empty() {
        None
    } else {
        Some(line_log_select(&repo, &opts, &paths, &mut list)?)
    };
    let content = Content::new(&repo);
    let merges = opts.merges.unwrap_or(if opts.rev.first_parent {
        DiffMerges::FirstParent
    } else {
        DiffMerges::Off
    });
    // `-q` in log does not suppress diffs (only `show -q` does).
    let wants_diff = opts.diff.any_format();
    let selects = opts.diff.pickaxe.is_some()
        || opts.diff.find_object.is_some()
        || opts.diff.diff_filter.is_some();
    let diff_filter_opts = opts.diff.clone();
    // Pickaxe / --diff-filter limit commits by their (first-parent) diff.
    if selects {
        let mut keep = Vec::new();
        for w in list.drain(..) {
            if w.commit.parents.len() > 1 && merges != DiffMerges::FirstParent {
                continue;
            }
            let pairs = diff_against(
                &content,
                &diff_filter_opts,
                w.commit.parents.first(),
                &w.commit,
                &opts.rev.specs,
            )?;
            if !pairs.is_empty() {
                keep.push(w);
            }
        }
        list = keep;
    }
    // whatchanged hides commits without a diff (merges included) before
    // `-n` / `--skip` count them.
    if cmd == "whatchanged" && !opts.rev.walk_reflogs {
        let mut keep = Vec::new();
        for w in list.drain(..) {
            if w.commit.parents.len() > 1 {
                if merges != DiffMerges::Off {
                    keep.push(w);
                }
                continue;
            }
            let pairs = diff_against(
                &content,
                &opts.diff,
                w.commit.parents.first(),
                &w.commit,
                &opts.rev.specs,
            )?;
            if !pairs.is_empty() {
                keep.push(w);
            }
        }
        list = keep;
    }
    let decorate = match opts.decorate {
        Some(Decorate::No) => false,
        Some(_) => true,
        None => false,
    };
    let full = opts.decorate == Some(Decorate::Full);
    let decos = if decorate || pretty_uses_decoration(&opts.pretty) {
        Some(decorations_filtered(
            &repo,
            full,
            &opts.decorate_refs,
            &opts.decorate_refs_exclude,
        )?)
    } else {
        None
    };
    let children: Option<HashMap<Oid, Vec<Oid>>> = if opts.children {
        let mut m: HashMap<Oid, Vec<Oid>> = HashMap::new();
        for w in &list {
            for p in &w.parents {
                m.entry(*p).or_default().push(w.oid);
            }
        }
        // Each child is prepended as the walk reaches it.
        for v in m.values_mut() {
            v.reverse();
        }
        Some(m)
    } else {
        None
    };
    let mailmap = if opts.mailmap {
        let cfg = arks_config::ConfigSet::load(Some(repo.common_dir()));
        Some(Mailmap::load(work_tree(&repo).ok().as_deref(), &cfg))
    } else {
        None
    };
    let source_map = SOURCE.with(|s| s.borrow().clone());
    let abbrev = match (opts.abbrev_commit, &opts.pretty) {
        (Some(false), _) => None,
        (Some(true), _) | (None, Pretty::Oneline) => Some(opts.abbrev),
        _ => None,
    };
    let graph_parents: HashMap<Oid, Vec<Oid>> = if opts.graph {
        let shown: HashSet<Oid> = list.iter().map(|w| w.oid).collect();
        list.iter()
            .map(|w| {
                let lim = if opts.rev.first_parent { 1 } else { usize::MAX };
                let mut kept: Vec<Oid> = Vec::new();
                for p in w.parents.iter().take(lim) {
                    if shown.contains(p) && !kept.contains(p) {
                        kept.push(*p);
                    }
                }
                (w.oid, kept)
            })
            .collect()
    } else {
        HashMap::new()
    };
    let mut graph_parents = graph_parents;
    if !opts.rev.walk_reflogs {
        list = list
            .into_iter()
            .skip(opts.rev.skip)
            .take(opts.rev.max_count.unwrap_or(usize::MAX))
            .collect();
        if opts.rev.reverse {
            list.reverse();
        }
    }
    let mut printer = Printer {
        content: &content,
        opts: &opts,
        fmt: HeaderFmt {
            repo: &repo,
            pretty: opts.pretty.clone(),
            date: opts.date,
            decorate,
            abbrev,
            parents: opts.parents,
            children: children.as_ref(),
            source: opts.source.then_some(&source_map),
            mailmap: mailmap.as_ref(),
            log_size: opts.log_size,
            ctx: FmtCtx {
                date: opts.date,
                decorations: decos.as_ref(),
            },
        },
        shown_one: false,
        graph: opts.graph.then(Graph::new),
        missing_newline: false,
    };
    let mut out = io::BufWriter::new(io::stdout().lock());
    // `--parents` / `--children` / `--graph` with a pathspec show (and
    // diff against) the rewritten parents.
    let rewrite = (!opts.rev.specs.is_empty() || opts.rev.simplify_by_decoration)
        && (opts.parents || opts.children || opts.graph);
    for w in &list {
        let rewritten;
        let w = if rewrite && w.parents != w.commit.parents {
            let mut x = w.clone();
            x.commit.parents = x.parents.clone();
            rewritten = x;
            &rewritten
        } else {
            w
        };
        let mark = mark_of(w, &opts);
        if let Some(g) = printer.graph.as_mut() {
            g.next_commit(w.oid, graph_parents.remove(&w.oid).unwrap_or_default());
        }
        if let Some((sel, msg)) = &w.reflog {
            // `-g`: reflog selector lines follow the commit line.
            let _ = (sel, msg);
        }
        let is_merge = w.commit.parents.len() > 1;
        if let Some(patches) = &line_log {
            let diff = match patches.get(&w.oid) {
                Some(t) if !opts.diff.no_output => RecordDiff::Text(t.clone()),
                _ => RecordDiff::None,
            };
            printer.commit(&mut out, &w.oid, &w.commit, &mark, None, &diff, false)?;
            continue;
        }
        if !wants_diff {
            printer.commit(
                &mut out,
                &w.oid,
                &w.commit,
                &mark,
                None,
                &RecordDiff::None,
                false,
            )?;
            continue;
        }
        if is_merge {
            match merges {
                DiffMerges::Off => {
                    if cmd == "whatchanged" {
                        continue;
                    }
                    printer.commit(
                        &mut out,
                        &w.oid,
                        &w.commit,
                        &mark,
                        None,
                        &RecordDiff::None,
                        false,
                    )?;
                }
                DiffMerges::FirstParent => {
                    let p = diff_against(
                        &content,
                        &opts.diff,
                        w.commit.parents.first(),
                        &w.commit,
                        &opts.rev.specs,
                    )?;
                    printer.commit(
                        &mut out,
                        &w.oid,
                        &w.commit,
                        &mark,
                        None,
                        &RecordDiff::Pairs(p),
                        false,
                    )?;
                }
                DiffMerges::Separate => {
                    for parent in &w.commit.parents {
                        let p = diff_against(
                            &content,
                            &opts.diff,
                            Some(parent),
                            &w.commit,
                            &opts.rev.specs,
                        )?;
                        printer.commit(
                            &mut out,
                            &w.oid,
                            &w.commit,
                            &mark,
                            Some(parent),
                            &RecordDiff::Pairs(p),
                            false,
                        )?;
                    }
                }
                DiffMerges::Remerge => {
                    let (p, msgs) = crate::cmd::remerge::remerge_pairs(
                        &content,
                        &opts.diff,
                        &w.commit,
                        &opts.rev.specs,
                    )?;
                    printer.commit(
                        &mut out,
                        &w.oid,
                        &w.commit,
                        &mark,
                        None,
                        &RecordDiff::Remerge(p, msgs),
                        false,
                    )?;
                }
                DiffMerges::Combined | DiffMerges::Dense => {
                    let mode = if merges == DiffMerges::Dense {
                        combined::Mode::Dense
                    } else {
                        combined::Mode::Combined
                    };
                    let e = combined::entries(
                        &content,
                        &opts.diff,
                        &w.commit.parents,
                        &w.oid,
                        &opts.rev.specs,
                    )?;
                    let fp = if combined::wants_stat(&opts.diff) {
                        diff_against(
                            &content,
                            &opts.diff,
                            w.commit.parents.first(),
                            &w.commit,
                            &opts.rev.specs,
                        )?
                    } else {
                        Vec::new()
                    };
                    printer.commit(
                        &mut out,
                        &w.oid,
                        &w.commit,
                        &mark,
                        None,
                        &RecordDiff::Combined(e, mode, fp),
                        false,
                    )?;
                }
            }
            continue;
        }
        let pairs = diff_against(
            &content,
            &opts.diff,
            w.commit.parents.first(),
            &w.commit,
            &opts.rev.specs,
        )?;
        if cmd == "whatchanged" && pairs.is_empty() {
            continue;
        }
        printer.commit(
            &mut out,
            &w.oid,
            &w.commit,
            &mark,
            None,
            &RecordDiff::Pairs(pairs),
            false,
        )?;
    }
    out.flush()?;
    Ok(())
}

thread_local! {
    static SOURCE: std::cell::RefCell<HashMap<Oid, String>> = std::cell::RefCell::new(HashMap::new());
}

/// `--source`: the tip name each shown commit was first reached from
/// (tips walked newest first, as the walk visits them).
fn sources(
    repo: &Repository,
    tips: &revwalk::Tips,
    list: &[Walked],
) -> CliResult<HashMap<Oid, String>> {
    let mut m: HashMap<Oid, String> = HashMap::new();
    let wanted: HashSet<Oid> = list.iter().map(|w| w.oid).collect();
    // Date-ordered propagation from the tips.
    let mut heap: std::collections::BinaryHeap<(i64, u64)> = std::collections::BinaryHeap::new();
    let mut by: HashMap<u64, Oid> = HashMap::new();
    let mut seq = u64::MAX;
    let mut seen = HashSet::new();
    for t in &tips.include {
        if seen.insert(*t) {
            let name = tips.names.get(t).cloned().unwrap_or_else(|| t.to_hex());
            m.entry(*t).or_insert(name);
            heap.push((read_commit(repo, t)?.committer.timestamp, seq));
            by.insert(seq, *t);
            seq -= 1;
        }
    }
    while let Some((_, k)) = heap.pop() {
        let oid = by[&k];
        let name = m.get(&oid).cloned().unwrap_or_default();
        for p in read_commit(repo, &oid)?.parents {
            m.entry(p).or_insert_with(|| name.clone());
            if seen.insert(p) && (wanted.contains(&p) || m.len() < 100_000) {
                heap.push((read_commit(repo, &p)?.committer.timestamp, seq));
                by.insert(seq, p);
                seq -= 1;
            }
        }
    }
    m.retain(|k, _| wanted.contains(k));
    Ok(m)
}

pub fn cmd_log(args: &[String]) -> CliResult<()> {
    // Cite: https://git-scm.com/docs/git-log
    run_log(args, "log")
}

pub fn cmd_whatchanged(args: &[String]) -> CliResult<()> {
    // Cite: https://git-scm.com/docs/git-whatchanged
    run_log(args, "whatchanged")
}

pub fn cmd_show(args: &[String]) -> CliResult<()> {
    // Cite: https://git-scm.com/docs/git-show
    let (mut opts, paths) = parse_opts(args, "show")?;
    let repo = open_repo()?;
    let specs = pathspecs(&repo, &paths);
    if !opts.diff.any_format() && !opts.diff.no_output {
        opts.diff.patch = true;
    }
    if opts.quiet {
        opts.diff.no_output = true;
    }
    let revs = if opts.rev.revs.is_empty() {
        vec!["HEAD".to_string()]
    } else {
        opts.rev.revs.clone()
    };
    let decorate = matches!(opts.decorate, Some(Decorate::Short) | Some(Decorate::Full));
    let decos = if decorate || pretty_uses_decoration(&opts.pretty) {
        Some(decorations_filtered(
            &repo,
            opts.decorate == Some(Decorate::Full),
            &opts.decorate_refs,
            &opts.decorate_refs_exclude,
        )?)
    } else {
        None
    };
    let mailmap = if opts.mailmap {
        let cfg = arks_config::ConfigSet::load(Some(repo.common_dir()));
        Some(Mailmap::load(work_tree(&repo).ok().as_deref(), &cfg))
    } else {
        None
    };
    let content = Content::new(&repo);
    let empty_src = HashMap::new();
    let merges = opts.merges.unwrap_or(DiffMerges::Dense);
    let abbrev = match (opts.abbrev_commit, &opts.pretty) {
        (Some(false), _) => None,
        (Some(true), _) | (None, Pretty::Oneline) => Some(opts.abbrev),
        _ => None,
    };
    let mut out = io::BufWriter::new(io::stdout().lock());
    let mut printer = Printer {
        content: &content,
        opts: &opts,
        fmt: HeaderFmt {
            repo: &repo,
            pretty: opts.pretty.clone(),
            date: opts.date,
            decorate,
            abbrev,
            parents: opts.parents,
            children: None,
            source: opts.source.then_some(&empty_src),
            mailmap: mailmap.as_ref(),
            log_size: opts.log_size,
            ctx: FmtCtx {
                date: opts.date,
                decorations: decos.as_ref(),
            },
        },
        shown_one: false,
        graph: None,
        missing_newline: false,
    };
    for rev in &revs {
        if rev.contains("..") {
            // A range walks like `log` (`show A..B`).
            let mut ro = opts.rev.clone();
            ro.revs = vec![rev.clone()];
            ro.no_walk = None;
            ro.specs = specs.clone();
            let tips = revwalk::resolve_tips(&repo, &ro, None)?;
            for w in revwalk::walk(&repo, &ro, &tips)? {
                show_commit(
                    &mut printer,
                    &mut out,
                    &content,
                    &opts,
                    merges,
                    &w.oid,
                    &w.commit,
                    &specs,
                )?;
            }
            continue;
        }
        let mut oid = resolve_rev(&repo, rev)?;
        loop {
            let obj = repo.get_object(&oid)?;
            match obj.kind {
                ObjectKind::Commit => {
                    let c = Commit::decode(&obj.payload)?;
                    show_commit(
                        &mut printer,
                        &mut out,
                        &content,
                        &opts,
                        merges,
                        &oid,
                        &c,
                        &specs,
                    )?;
                    break;
                }
                ObjectKind::Tag => {
                    if printer.shown_one {
                        writeln!(out)?;
                    }
                    write_tag(&mut out, &obj.payload, opts.date)?;
                    printer.shown_one = true;
                    oid = tag_target(&obj.payload)
                        .ok_or_else(|| CliError::Msg(format!("{rev}: bad tag object")))?;
                }
                ObjectKind::Blob => {
                    out.write_all(&obj.payload)?;
                    break;
                }
                ObjectKind::Tree => {
                    if printer.shown_one {
                        writeln!(out)?;
                    }
                    writeln!(out, "tree {rev}\n")?;
                    let t = arks_object::Tree::decode(&obj.payload, repo.oid_kind())?;
                    for e in &t.entries {
                        let slash = if e.mode.is_tree() { "/" } else { "" };
                        writeln!(out, "{}{slash}", e.name)?;
                    }
                    printer.shown_one = true;
                    break;
                }
            }
        }
    }
    out.flush()?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn show_commit(
    printer: &mut Printer<'_>,
    out: &mut impl Write,
    content: &Content<'_>,
    opts: &LogOpts,
    merges: DiffMerges,
    oid: &Oid,
    c: &Commit,
    specs: &[String],
) -> CliResult<()> {
    if !opts.line_ranges.is_empty() {
        let work = work_tree(content.repo)?;
        let ls = crate::cmd::line_log::parse_specs(&opts.line_ranges, &work)?;
        let t = crate::cmd::line_log::show_text(content.repo, c, &ls)?;
        let diff = if opts.diff.no_output {
            RecordDiff::None
        } else {
            RecordDiff::Text(format!("\n{t}"))
        };
        return printer.commit(out, oid, c, "", None, &diff, false);
    }
    if c.parents.len() < 2 {
        let p = diff_against(content, &opts.diff, c.parents.first(), c, specs)?;
        return printer.commit(out, oid, c, "", None, &RecordDiff::Pairs(p), false);
    }
    let d = &opts.diff;
    match merges {
        DiffMerges::Off => printer.commit(out, oid, c, "", None, &RecordDiff::None, false),
        DiffMerges::FirstParent => {
            let p = diff_against(content, d, c.parents.first(), c, specs)?;
            printer.commit(out, oid, c, "", None, &RecordDiff::Pairs(p), false)
        }
        DiffMerges::Separate => {
            for parent in &c.parents {
                let p = diff_against(content, d, Some(parent), c, specs)?;
                printer.commit(out, oid, c, "", Some(parent), &RecordDiff::Pairs(p), false)?;
            }
            Ok(())
        }
        DiffMerges::Remerge => {
            let (p, msgs) = crate::cmd::remerge::remerge_pairs(content, d, c, specs)?;
            printer.commit(out, oid, c, "", None, &RecordDiff::Remerge(p, msgs), false)
        }
        DiffMerges::Combined | DiffMerges::Dense => {
            // Stat formats of a merge show the first-parent diff.
            let fp = if combined::wants_stat(d) {
                diff_against(content, d, c.parents.first(), c, specs)?
            } else {
                Vec::new()
            };
            let mode = if merges == DiffMerges::Dense {
                combined::Mode::Dense
            } else {
                combined::Mode::Combined
            };
            let e = combined::entries(content, d, &c.parents, oid, specs)?;
            printer.commit(
                out,
                oid,
                c,
                "",
                None,
                &RecordDiff::Combined(e, mode, fp),
                true,
            )
        }
    }
}

/// Annotated tag header as `git show` prints it: `tag <name>`, `Tagger:`,
/// `Date:`, blank line, then the tag message.
fn write_tag(out: &mut impl Write, payload: &[u8], date: DateMode) -> CliResult<()> {
    let text = String::from_utf8_lossy(payload);
    let (head, body) = text.split_once("\n\n").unwrap_or((&text, ""));
    let mut name = "";
    let mut tagger = None;
    for line in head.lines() {
        if let Some(v) = line.strip_prefix("tag ") {
            name = v;
        } else if let Some(v) = line.strip_prefix("tagger ") {
            tagger = Signature::parse_line(v).ok();
        }
    }
    writeln!(out, "tag {name}")?;
    if let Some(t) = &tagger {
        writeln!(out, "Tagger: {} <{}>", t.name, t.email)?;
        writeln!(out, "Date:   {}", date.format(t))?;
    }
    writeln!(out)?;
    write!(out, "{body}")?;
    if !body.is_empty() && !body.ends_with('\n') {
        writeln!(out)?;
    }
    Ok(())
}
