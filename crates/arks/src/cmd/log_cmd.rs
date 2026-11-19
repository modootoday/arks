//! `log` and `show` porcelain: option parsing, commit headers (medium /
//! oneline / user formats), decorations, and per-commit diff output.
//!
//! Record layout follows git `log-tree.c`: separator-style formats (medium,
//! `format:`) put a newline *between* commits; terminator-style formats
//! (oneline, `tformat:`) end every commit with one. When a diff follows a
//! non-oneline header, one blank line (or `---` when both a diffstat and a
//! patch are shown) precedes it. Merges show no diff in `log` (no `-m`).
//!
//! Primary: <https://git-scm.com/docs/git-log>, <https://git-scm.com/docs/git-show>,
//! <https://git-scm.com/docs/pretty-formats>.
//!
//! Partial: `show` on a merge prints the diffstat against the first parent
//! and names changed against every parent, but no combined (`--cc`) patch;
//! `--graph` only renders a linear `* ` column.

use crate::cmd::bre::Bre;
use crate::cmd::diff_render::{DiffFormat, changes_between, render};
use crate::cmd::log_format::{
    DateMode, FmtCtx, Pretty, expand_ctx, path_in_specs, resolve_range, take_pretty, walk_paths,
};
use crate::cmd::shared::{resolve_rev, tag_target, tree_blob_map};
use crate::error::{CliError, CliResult};
use crate::util::{open_repo, work_tree};
use arks_object::{Commit, ObjectKind, Signature};
use arks_oid::Oid;
use arks_refs::RefTarget;
use arks_repo::Repository;
use std::collections::HashMap;
use std::io::{self, Write};
use std::path::PathBuf;

/// Options shared by `log` and `show` after parsing.
struct LogOpts {
    pretty: Pretty,
    diff: DiffFormat,
    date: DateMode,
    decorate: bool,
    graph: bool,
    first_parent: bool,
    no_merges: bool,
    merges_only: bool,
    reverse: bool,
    all: bool,
    max_count: Option<usize>,
    authors: Vec<Bre>,
    greps: Vec<Bre>,
    revs: Vec<String>,
}

fn parse_opts(args: &[String], cmd: &str) -> CliResult<(LogOpts, Vec<String>)> {
    let (head, paths): (Vec<String>, Vec<String>) = match args.iter().position(|a| a == "--") {
        Some(i) => (args[..i].to_vec(), args[i + 1..].to_vec()),
        None => (args.to_vec(), Vec::new()),
    };
    let mut head = head;
    let pretty = take_pretty(&mut head)?;
    let mut o = LogOpts {
        pretty,
        diff: DiffFormat::default(),
        date: DateMode::Default,
        decorate: false,
        graph: false,
        first_parent: false,
        no_merges: false,
        merges_only: false,
        reverse: false,
        all: false,
        max_count: None,
        authors: Vec::new(),
        greps: Vec::new(),
        revs: Vec::new(),
    };
    let mut icase = false;
    let mut author_pats = Vec::new();
    let mut grep_pats = Vec::new();
    let count = |v: &str| -> CliResult<usize> {
        v.parse()
            .map_err(|_| CliError::fatal(format!("'{v}': not an integer")))
    };
    let mut i = 0;
    while i < head.len() {
        let a = head[i].clone();
        i += 1;
        if o.diff.take_flag(&a)? {
            continue;
        }
        let mut value = |name: &str| -> CliResult<Option<String>> {
            if let Some(v) = a.strip_prefix(&format!("{name}=")) {
                return Ok(Some(v.to_string()));
            }
            if a == name {
                let v = head
                    .get(i)
                    .cloned()
                    .ok_or_else(|| CliError::fatal(format!("option '{name}' requires a value")))?;
                i += 1;
                return Ok(Some(v));
            }
            Ok(None)
        };
        if let Some(v) = value("--max-count")? {
            o.max_count = Some(count(&v)?);
        } else if let Some(v) = value("--date")? {
            o.date = DateMode::parse(&v)?;
        } else if let Some(v) = value("--author")? {
            author_pats.push(v);
        } else if let Some(v) = value("--grep")? {
            grep_pats.push(v);
        } else if a == "-n" {
            let v = head
                .get(i)
                .cloned()
                .ok_or_else(|| CliError::fatal("switch `n' requires a value"))?;
            i += 1;
            o.max_count = Some(count(&v)?);
        } else if let Some(rest) = a.strip_prefix("-n").filter(|r| !r.is_empty()) {
            o.max_count = Some(count(rest)?);
        } else if a.len() > 1
            && a.starts_with('-')
            && !a.starts_with("--")
            && a[1..].bytes().all(|b| b.is_ascii_digit())
        {
            o.max_count = Some(count(&a[1..])?);
        } else {
            match a.as_str() {
                "--graph" => o.graph = true,
                "--first-parent" => o.first_parent = true,
                "--no-merges" => o.no_merges = true,
                "--merges" => o.merges_only = true,
                "--reverse" => o.reverse = true,
                "--all" => o.all = true,
                "--decorate" | "--decorate=short" => o.decorate = true,
                "--no-decorate" | "--no-color" | "--color=never" => o.decorate = false,
                "-i" | "--regexp-ignore-case" => icase = true,
                x if x.starts_with('-') && x.len() > 1 => {
                    return Err(CliError::NotImplemented(format!(
                        "{cmd}: unsupported option: {x}"
                    )));
                }
                _ => o.revs.push(a.clone()),
            }
        }
    }
    o.authors = author_pats.iter().map(|p| Bre::new(p, icase)).collect();
    o.greps = grep_pats.iter().map(|p| Bre::new(p, icase)).collect();
    Ok((o, paths))
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

/// Ref decorations per commit, in git `format_decorations` order: `HEAD`
/// (as `HEAD -> <branch>` when attached) first, then refs in reverse
/// refname order; tags as `tag: <name>`, annotated tags on their target.
pub(crate) fn decorations(repo: &Repository) -> CliResult<HashMap<Oid, Vec<String>>> {
    let mut map: HashMap<Oid, Vec<String>> = HashMap::new();
    let head_branch = match repo.head_target() {
        Ok(RefTarget::Symbolic(s)) => Some(s),
        _ => None,
    };
    let mut refs = repo.refs().list_refs("refs/")?;
    refs.sort_by(|a, b| a.0.cmp(&b.0));
    for (name, oid) in refs.iter().rev() {
        if Some(name) == head_branch.as_ref() {
            continue;
        }
        let label = if let Some(t) = name.strip_prefix("refs/tags/") {
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
    if let Ok(head) = repo.head_oid() {
        let label = match &head_branch {
            Some(b) => format!(
                "HEAD -> {}",
                b.strip_prefix("refs/heads/").unwrap_or(b.as_str())
            ),
            None => "HEAD".to_string(),
        };
        map.entry(head).or_default().insert(0, label);
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

fn person_line(label: &str, sig: &Signature) -> String {
    format!("{label}{} <{}>\n", sig.name, sig.email)
}

/// Writes commit records with git's separator / terminator rules.
struct Printer<'a> {
    repo: &'a Repository,
    opts: &'a LogOpts,
    ctx: FmtCtx<'a>,
    shown_one: bool,
}

impl Printer<'_> {
    fn header(&self, oid: &Oid, c: &Commit) -> String {
        let deco = self.ctx.decoration(oid);
        let deco = if deco.is_empty() || !self.opts.decorate {
            String::new()
        } else {
            format!(" ({deco})")
        };
        match &self.opts.pretty {
            Pretty::Default => {
                let mut s = format!("commit {}{deco}\n", oid.to_hex());
                if c.parents.len() > 1 {
                    let ps: Vec<String> = c
                        .parents
                        .iter()
                        .map(|p| p.to_hex()[..7].to_string())
                        .collect();
                    s.push_str(&format!("Merge: {}\n", ps.join(" ")));
                }
                s.push_str(&person_line("Author: ", &c.author));
                s.push_str(&format!("Date:   {}\n", self.opts.date.format(&c.author)));
                s.push('\n');
                s.push_str(&indented_message(&c.message));
                s
            }
            Pretty::Oneline => format!("{}{deco} {}\n", &oid.to_hex()[..7], c.subject()),
            Pretty::TFormat(f) => format!("{}\n", expand_ctx(f, oid, c, &self.ctx)),
            Pretty::Format(f) => expand_ctx(f, oid, c, &self.ctx),
        }
    }

    fn uses_terminator(&self) -> bool {
        matches!(self.opts.pretty, Pretty::Oneline | Pretty::TFormat(_))
    }

    fn format_is_empty(&self) -> bool {
        matches!(&self.opts.pretty, Pretty::Format(f) | Pretty::TFormat(f) if f.is_empty())
    }

    /// One commit record; `changes` = diff to show after the header.
    fn commit(
        &mut self,
        out: &mut impl Write,
        oid: &Oid,
        c: &Commit,
        changes: &[crate::cmd::diff_render::Change],
    ) -> CliResult<()> {
        if self.shown_one && !self.uses_terminator() {
            writeln!(out)?;
        }
        self.shown_one = true;
        let mut text = self.header(oid, c);
        if self.opts.diff.any() && !changes.is_empty() {
            if !matches!(self.opts.pretty, Pretty::Oneline) && !self.format_is_empty() {
                if self.opts.diff.stat && self.opts.diff.patch {
                    text.push_str("---");
                }
                text.push('\n');
            }
            let mut buf = Vec::new();
            render(&mut buf, self.repo, &self.opts.diff, changes)?;
            text.push_str(&String::from_utf8_lossy(&buf));
        }
        if self.opts.graph {
            for (i, line) in text.split_inclusive('\n').enumerate() {
                out.write_all(if i == 0 { b"* " } else { b"  " })?;
                out.write_all(line.as_bytes())?;
            }
        } else {
            out.write_all(text.as_bytes())?;
        }
        Ok(())
    }
}

/// Diff of a commit against its first parent (root: against the empty tree).
fn commit_changes(
    repo: &Repository,
    oid: &Oid,
    c: &Commit,
    specs: &[String],
) -> CliResult<Vec<crate::cmd::diff_render::Change>> {
    let parent = match c.parents.first() {
        Some(p) => tree_blob_map(repo, p)?,
        None => HashMap::new(),
    };
    let mine = tree_blob_map(repo, oid)?;
    let keep = |m: HashMap<String, Oid>| -> HashMap<String, Oid> {
        m.into_iter()
            .filter(|(k, _)| path_in_specs(k, specs))
            .collect()
    };
    Ok(changes_between(&keep(parent), &keep(mine), None))
}

pub fn cmd_log(args: &[String]) -> CliResult<()> {
    // M16 Sy-1 + cloud drop-in §O. Cite: https://git-scm.com/docs/git-log
    let (opts, paths) = parse_opts(args, "log")?;
    let repo = open_repo()?;
    let specs = pathspecs(&repo, &paths);
    let mut revs = opts.revs.clone();
    if opts.all {
        let mut refs = repo.refs().list_refs("refs/")?;
        refs.sort_by(|a, b| a.0.cmp(&b.0));
        revs.extend(refs.into_iter().map(|(n, _)| n));
        if repo.head_oid().is_ok() {
            revs.push("HEAD".into());
        }
    }
    let (mut inc, exc) = resolve_range(&repo, &revs)?;
    if inc.is_empty() {
        return Ok(());
    }
    // Non-commit tips (e.g. a tag of a tree) are skipped; tags peel to commits.
    let mut tips = Vec::new();
    for o in inc.drain(..) {
        let p = crate::cmd::shared::peel_tags(&repo, &o)?;
        if repo.get_object(&p)?.kind == ObjectKind::Commit && !tips.contains(&p) {
            tips.push(p);
        }
    }
    let mut excl = Vec::new();
    for o in exc {
        excl.push(crate::cmd::shared::peel_tags(&repo, &o)?);
    }
    let mut selected = Vec::new();
    for (oid, c) in walk_paths(&repo, &tips, &excl, opts.first_parent, &specs)? {
        if (opts.no_merges && c.parents.len() > 1) || (opts.merges_only && c.parents.len() < 2) {
            continue;
        }
        if !opts.authors.is_empty() {
            let who = format!("{} <{}>", c.author.name, c.author.email);
            if !opts.authors.iter().any(|r| r.is_match(who.as_bytes())) {
                continue;
            }
        }
        if !opts.greps.is_empty()
            && !opts
                .greps
                .iter()
                .any(|r| c.message.lines().any(|l| r.is_match(l.as_bytes())))
        {
            continue;
        }
        selected.push((oid, c));
        if !opts.reverse && opts.max_count.is_some_and(|m| selected.len() >= m) {
            break;
        }
    }
    if let Some(m) = opts.max_count {
        selected.truncate(m);
    }
    if opts.reverse {
        selected.reverse();
    }
    let decos = if opts.decorate || pretty_uses_decoration(&opts.pretty) {
        Some(decorations(&repo)?)
    } else {
        None
    };
    let mut printer = Printer {
        repo: &repo,
        opts: &opts,
        ctx: FmtCtx {
            date: opts.date,
            decorations: decos.as_ref(),
        },
        shown_one: false,
    };
    let mut out = io::BufWriter::new(io::stdout().lock());
    for (oid, c) in &selected {
        let changes = if opts.diff.any() && c.parents.len() < 2 {
            commit_changes(&repo, oid, c, &specs)?
        } else {
            Vec::new()
        };
        printer.commit(&mut out, oid, c, &changes)?;
    }
    out.flush()?;
    Ok(())
}

fn pretty_uses_decoration(p: &Pretty) -> bool {
    matches!(p, Pretty::Format(f) | Pretty::TFormat(f) if f.contains("%d") || f.contains("%D"))
}

pub fn cmd_show(args: &[String]) -> CliResult<()> {
    // Cloud drop-in §O. Cite: https://git-scm.com/docs/git-show
    let (mut opts, paths) = parse_opts(args, "show")?;
    let repo = open_repo()?;
    let specs = pathspecs(&repo, &paths);
    let d = &opts.diff;
    if !(d.patch || d.stat || d.numstat || d.shortstat || d.name_only || d.name_status) {
        opts.diff.patch = true;
    }
    let revs = if opts.revs.is_empty() {
        vec!["HEAD".to_string()]
    } else {
        opts.revs.clone()
    };
    let decos = if opts.decorate || pretty_uses_decoration(&opts.pretty) {
        Some(decorations(&repo)?)
    } else {
        None
    };
    let mut out = io::BufWriter::new(io::stdout().lock());
    let mut printer = Printer {
        repo: &repo,
        opts: &opts,
        ctx: FmtCtx {
            date: opts.date,
            decorations: decos.as_ref(),
        },
        shown_one: false,
    };
    for rev in &revs {
        let mut oid = resolve_rev(&repo, rev)?;
        loop {
            let obj = repo.get_object(&oid)?;
            match obj.kind {
                ObjectKind::Commit => {
                    let c = Commit::decode(&obj.payload)?;
                    let changes = show_changes(&repo, &oid, &c, &opts.diff, &specs)?;
                    printer.commit(&mut out, &oid, &c, &changes)?;
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

/// `show` diff: first-parent for ordinary commits; for merges, the diffstat
/// against the first parent and names that differ from every parent (no
/// combined patch -- Partial).
fn show_changes(
    repo: &Repository,
    oid: &Oid,
    c: &Commit,
    fmt: &DiffFormat,
    specs: &[String],
) -> CliResult<Vec<crate::cmd::diff_render::Change>> {
    if !fmt.any() {
        return Ok(Vec::new());
    }
    if c.parents.len() < 2 {
        return commit_changes(repo, oid, c, specs);
    }
    if fmt.stat || fmt.numstat || fmt.shortstat {
        return commit_changes(repo, oid, c, specs);
    }
    if fmt.name_only || fmt.name_status {
        let mine = tree_blob_map(repo, oid)?;
        let mut all = commit_changes(repo, oid, c, specs)?;
        for p in &c.parents[1..] {
            let pm = tree_blob_map(repo, p)?;
            all.retain(|ch| pm.get(&ch.path) != mine.get(&ch.path));
        }
        return Ok(all);
    }
    Ok(Vec::new())
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
        write!(out, "{}", person_line("Tagger: ", t))?;
        writeln!(out, "Date:   {}", date.format(t))?;
    }
    writeln!(out)?;
    // Signature blocks are part of the message body as stored.
    write!(out, "{body}")?;
    if !body.is_empty() && !body.ends_with('\n') {
        writeln!(out)?;
    }
    Ok(())
}
