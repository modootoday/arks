//! `git cherry-pick` / `git revert`: every option of their `-h` lists.
//!
//! Each commit is applied with a three-way merge whose base is the picked
//! commit's parent (cherry-pick) or the commit itself (revert), so changes
//! merge at line level like git's ort strategy. A multi-commit run keeps
//! git's sequencer files (`.git/sequencer/{head,todo,abort-safety,opts}`)
//! so `--continue`, `--skip`, `--abort` and `--quit` resume or stop it, and
//! a stopped pick leaves `CHERRY_PICK_HEAD` / `REVERT_HEAD` and `MERGE_MSG`.
//! Cite: <https://git-scm.com/docs/git-cherry-pick>,
//! <https://git-scm.com/docs/git-revert>, <https://git-scm.com/docs/git-sequencer>
//! (behaviour, not code).

use crate::cmd::merge_engine::{Favor, MergeLabels, merge_trees_worktree};
use crate::cmd::opt_parse::{Mode, Parsed, parse};
use crate::cmd::shared::{abbrev_hex, committer_signature, resolve_rev, tree_blob_map};
use crate::cmd::trailers::Cleanup;
use crate::error::{CliError, CliResult};
use crate::util::{open_repo, read_index, work_tree};
use arks_object::{Commit, ObjectKind, RawObject};
use arks_oid::Oid;
use arks_refs::RefTarget;
use arks_repo::Repository;
use arks_store::ObjectStore;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Op {
    Pick,
    Revert,
}

impl Op {
    fn name(self) -> &'static str {
        match self {
            Op::Pick => "cherry-pick",
            Op::Revert => "revert",
        }
    }
    fn verb(self) -> &'static str {
        match self {
            Op::Pick => "pick",
            Op::Revert => "revert",
        }
    }
    fn head_file(self) -> &'static str {
        match self {
            Op::Pick => "CHERRY_PICK_HEAD",
            Op::Revert => "REVERT_HEAD",
        }
    }
}

/// Options that shape each commit of a run (persisted in `sequencer/opts`).
#[derive(Clone, Default)]
struct Opts {
    no_commit: bool,
    edit: bool,
    signoff: bool,
    record_origin: bool,
    mainline: Option<usize>,
    ff: bool,
    allow_empty: bool,
    allow_empty_message: bool,
    keep_redundant: bool,
    reference: bool,
    cleanup: Option<String>,
    strategy: Option<String>,
    strategy_options: Vec<String>,
    sign_args: Vec<String>,
}

pub fn cmd_cherry_pick(args: &[String]) -> CliResult<()> {
    run(Op::Pick, args)
}

pub fn cmd_revert(args: &[String]) -> CliResult<()> {
    run(Op::Revert, args)
}

fn usage(op: Op) -> CliError {
    let text = match op {
        Op::Pick => {
            "usage: git cherry-pick [--edit] [-n] [-m <parent-number>] [-s] [-x] [--ff]\n                       [-S[<keyid>]] <commit>...\n   or: git cherry-pick (--continue | --skip | --abort | --quit)"
        }
        Op::Revert => {
            "usage: git revert [--[no-]edit] [-n] [-m <parent-number>] [-s] [-S[<keyid>]] <commit>...\n   or: git revert (--continue | --skip | --abort | --quit)"
        }
    };
    CliError::Exit(129, text.to_string())
}

fn failed(op: Op, msg: &str) -> CliError {
    CliError::Exit(128, format!("error: {msg}\nfatal: {} failed", op.name()))
}

fn run(op: Op, args: &[String]) -> CliResult<()> {
    let table = match op {
        Op::Pick => crate::cmd::porcelain_opts::CHERRY_PICK,
        Op::Revert => crate::cmd::porcelain_opts::REVERT,
    };
    let p = parse(op.name(), args, table, Mode::default())?;
    let actions: Vec<&str> = ["quit", "continue", "abort", "skip"]
        .into_iter()
        .filter(|a| p.on(a))
        .collect();
    if actions.len() > 1 {
        return Err(CliError::fatal(format!(
            "--{} is incompatible with --{}",
            actions[1], actions[0]
        )));
    }
    if let Some(a) = actions.first() {
        if !p.args().is_empty() {
            return Err(usage(op));
        }
        return match *a {
            "quit" => quit(),
            "abort" => abort(op),
            "skip" => skip(op),
            _ => resume(op),
        };
    }
    let opts = options(op, &p)?;
    let revs: Vec<String> = p.args();
    if revs.is_empty() {
        return Err(usage(op));
    }
    let repo = open_repo()?;
    if sequencer_dir(&repo).is_dir() {
        return Err(CliError::Exit(
            128,
            format!(
                "error: {} is already in progress\nhint: try \"git {} (--continue | --abort | --quit)\"\nfatal: {} failed",
                op.name(),
                op.name(),
                op.name()
            ),
        ));
    }
    crate::cmd::replay::refuse_exclusive_sequencer_state(
        &repo,
        crate::cmd::replay::SequencerOp::CherryPick,
    )?;
    let commits = pick_list(&repo, op, &revs)?;
    if commits.is_empty() {
        return Err(failed(op, "empty commit set passed"));
    }
    let head = repo.head_oid()?;
    drop(repo);
    let todo: Vec<(Op, Oid)> = commits.into_iter().map(|c| (op, c)).collect();
    if todo.len() > 1 {
        let repo = open_repo()?;
        let dir = sequencer_dir(&repo);
        fs::create_dir_all(&dir)?;
        fs::write(dir.join("head"), format!("{}\n", head.to_hex()))?;
        fs::write(dir.join("abort-safety"), format!("{}\n", head.to_hex()))?;
        write_opts(&dir, &opts)?;
    }
    run_todo(&todo, &opts)
}

fn options(op: Op, p: &Parsed) -> CliResult<Opts> {
    let cleanup = p.value("cleanup");
    if let Some(c) = &cleanup {
        if !matches!(
            c.as_str(),
            "strip" | "whitespace" | "verbatim" | "scissors" | "default"
        ) {
            return Err(CliError::fatal(format!("Invalid cleanup mode {c}")));
        }
    }
    let mainline = match p.value("mainline") {
        None => None,
        Some(v) => match v.parse::<usize>() {
            Ok(n) if n > 0 => Some(n),
            _ => {
                return Err(CliError::Exit(
                    129,
                    "error: option `mainline' expects a number greater than zero".into(),
                ));
            }
        },
    };
    let strategy = p.value("strategy");
    if let Some(s) = &strategy {
        if !matches!(
            s.as_str(),
            "ort" | "recursive" | "resolve" | "ours" | "subtree" | "octopus"
        ) {
            return Err(CliError::fatal(format!(
                "Could not find merge strategy '{s}'.\nAvailable strategies are: octopus ours recursive resolve subtree."
            )));
        }
    }
    let o = Opts {
        no_commit: p.on("no-commit"),
        edit: p.on("edit"),
        signoff: p.on("signoff"),
        record_origin: op == Op::Pick && p.on("-x"),
        mainline,
        ff: op == Op::Pick && p.on("ff"),
        allow_empty: op == Op::Pick && p.on("allow-empty"),
        allow_empty_message: op == Op::Pick && p.on("allow-empty-message"),
        keep_redundant: op == Op::Pick && p.on("keep-redundant-commits"),
        reference: op == Op::Revert && p.on("reference"),
        cleanup,
        strategy,
        strategy_options: p.values("strategy-option"),
        sign_args: crate::cmd::commit::sign_args(p),
    };
    if o.no_commit && o.ff {
        return Err(CliError::fatal(
            "options '--ff' and '--no-commit' cannot be used together",
        ));
    }
    if o.ff && (o.signoff || o.record_origin || o.edit || o.mainline.is_some()) {
        let other = if o.signoff {
            "--signoff"
        } else if o.record_origin {
            "-x"
        } else if o.edit {
            "--edit"
        } else {
            "--mainline"
        };
        return Err(CliError::fatal(format!(
            "options '--ff' and '{other}' cannot be used together"
        )));
    }
    Ok(o)
}

/// Commits named by the arguments: single revisions in the order given,
/// `A..B` ranges oldest first (newest first for revert), `^A B` exclusion.
fn pick_list(repo: &Repository, op: Op, revs: &[String]) -> CliResult<Vec<Oid>> {
    let resolve = |r: &str| -> CliResult<Oid> {
        let oid =
            resolve_rev(repo, r).map_err(|_| CliError::fatal(format!("bad revision '{r}'")))?;
        crate::cmd::shared::peel_to_kind(repo, &oid, crate::cmd::shared::ObjKind::Commit, r)
    };
    let ranged = revs.iter().any(|r| r.contains("..") || r.starts_with('^'));
    if !ranged {
        return revs.iter().map(|r| resolve(r)).collect();
    }
    let mut include = Vec::new();
    let mut exclude = Vec::new();
    for r in revs {
        if let Some((a, b)) = r.split_once("...") {
            let _ = (a, b);
            return Err(CliError::NotImplemented(format!(
                "{}: symmetric difference ranges are not implemented",
                op.name()
            )));
        } else if let Some((a, b)) = r.split_once("..") {
            exclude.push(resolve(if a.is_empty() { "HEAD" } else { a })?);
            include.push(resolve(if b.is_empty() { "HEAD" } else { b })?);
        } else if let Some(x) = r.strip_prefix('^') {
            exclude.push(resolve(x)?);
        } else {
            include.push(resolve(r)?);
        }
    }
    let mut hidden: HashSet<Oid> = HashSet::new();
    for x in &exclude {
        hidden.extend(arks_repo::ancestor_set(repo, *x)?);
    }
    // Reachable commits not hidden, newest first by committer date, then
    // oldest first for cherry-pick (git's `--reverse` walk).
    let mut seen = HashSet::new();
    let mut found = Vec::new();
    let mut queue = include.clone();
    while let Some(c) = queue.pop() {
        if hidden.contains(&c) || !seen.insert(c) {
            continue;
        }
        let commit = crate::cmd::log_format::read_commit(repo, &c)?;
        queue.extend(commit.parents.iter().copied());
        found.push((commit.committer.timestamp, c));
    }
    found.sort_by(|a, b| b.0.cmp(&a.0));
    let mut out: Vec<Oid> = found.into_iter().map(|(_, c)| c).collect();
    if op == Op::Pick {
        out.reverse();
    }
    Ok(out)
}

fn sequencer_dir(repo: &Repository) -> PathBuf {
    repo.git_dir().join("sequencer")
}

fn write_opts(dir: &std::path::Path, o: &Opts) -> CliResult<()> {
    let mut s = String::from("[options]\n");
    let mut flag = |k: &str, v: bool| {
        if v {
            s.push_str(&format!("\t{k} = true\n"));
        }
    };
    flag("no-commit", o.no_commit);
    flag("edit", o.edit);
    flag("signoff", o.signoff);
    flag("record-origin", o.record_origin);
    flag("allow-ff", o.ff);
    flag("allow-empty", o.allow_empty);
    flag("allow-empty-message", o.allow_empty_message);
    flag("keep-redundant-commits", o.keep_redundant);
    flag("reference", o.reference);
    if let Some(m) = o.mainline {
        s.push_str(&format!("\tmainline = {m}\n"));
    }
    if let Some(c) = &o.cleanup {
        s.push_str(&format!("\tdefault-msg-cleanup = {c}\n"));
    }
    if let Some(st) = &o.strategy {
        s.push_str(&format!("\tstrategy = {st}\n"));
    }
    for x in &o.strategy_options {
        s.push_str(&format!("\tstrategy-option = {x}\n"));
    }
    for a in &o.sign_args {
        s.push_str(&format!("\tgpg-sign = {a}\n"));
    }
    fs::write(dir.join("opts"), s)?;
    Ok(())
}

fn read_opts(dir: &std::path::Path) -> Opts {
    let mut o = Opts::default();
    let Ok(text) = fs::read_to_string(dir.join("opts")) else {
        return o;
    };
    for line in text.lines() {
        let Some((k, v)) = line.trim().split_once(" = ") else {
            continue;
        };
        let on = v == "true";
        match k {
            "no-commit" => o.no_commit = on,
            "edit" => o.edit = on,
            "signoff" => o.signoff = on,
            "record-origin" => o.record_origin = on,
            "allow-ff" => o.ff = on,
            "allow-empty" => o.allow_empty = on,
            "allow-empty-message" => o.allow_empty_message = on,
            "keep-redundant-commits" => o.keep_redundant = on,
            "reference" => o.reference = on,
            "mainline" => o.mainline = v.parse().ok(),
            "default-msg-cleanup" => o.cleanup = Some(v.to_string()),
            "strategy" => o.strategy = Some(v.to_string()),
            "strategy-option" => o.strategy_options.push(v.to_string()),
            "gpg-sign" => o.sign_args.push(v.to_string()),
            _ => {}
        }
    }
    o
}

/// `todo` lines: `pick <abbrev> <subject>` / `revert <abbrev> <subject>`.
fn write_todo(repo: &Repository, todo: &[(Op, Oid)]) -> CliResult<()> {
    let dir = sequencer_dir(repo);
    if !dir.is_dir() {
        return Ok(());
    }
    let mut s = String::new();
    for (op, c) in todo {
        let commit = crate::cmd::log_format::read_commit(repo, c)?;
        s.push_str(&format!(
            "{} {} {}\n",
            op.verb(),
            abbrev_hex(repo, c, 7),
            crate::cmd::log_format::subject_line(&commit.message)
        ));
    }
    fs::write(dir.join("todo"), s)?;
    Ok(())
}

fn read_todo(repo: &Repository) -> CliResult<Vec<(Op, Oid)>> {
    let Ok(text) = fs::read_to_string(sequencer_dir(repo).join("todo")) else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for line in text.lines() {
        let mut w = line.split_whitespace();
        let op = match w.next() {
            Some("pick" | "p") => Op::Pick,
            Some("revert") => Op::Revert,
            _ => continue,
        };
        if let Some(r) = w.next() {
            out.push((op, resolve_rev(repo, r)?));
        }
    }
    Ok(out)
}

fn clear_state(repo: &Repository) {
    let _ = fs::remove_dir_all(sequencer_dir(repo));
    for f in ["CHERRY_PICK_HEAD", "REVERT_HEAD", "MERGE_MSG", "AUTO_MERGE"] {
        let _ = fs::remove_file(repo.git_dir().join(f));
    }
}

/// Apply `todo` front to back; stop (keeping the rest in the sequencer) at
/// the first commit that needs the user.
fn run_todo(todo: &[(Op, Oid)], opts: &Opts) -> CliResult<()> {
    for i in 0..todo.len() {
        let repo = open_repo()?;
        write_todo(&repo, &todo[i..])?;
        drop(repo);
        let (op, oid) = todo[i];
        pick_one(op, &oid, opts)?;
        let repo = open_repo()?;
        if sequencer_dir(&repo).is_dir() {
            if let Ok(h) = repo.head_oid() {
                fs::write(
                    sequencer_dir(&repo).join("abort-safety"),
                    format!("{}\n", h.to_hex()),
                )?;
            }
        }
    }
    let repo = open_repo()?;
    let _ = fs::remove_dir_all(sequencer_dir(&repo));
    Ok(())
}

/// Commit subject plus abbreviated id, as conflict labels and errors show it.
fn describe(repo: &Repository, oid: &Oid, c: &Commit) -> String {
    format!(
        "{} ({})",
        abbrev_hex(repo, oid, 7),
        crate::cmd::log_format::subject_line(&c.message)
    )
}

fn is_trailer_para(para: &str) -> bool {
    !para.is_empty()
        && para.lines().all(|l| {
            l.starts_with("(cherry picked from commit ")
                || l.split_once(": ").is_some_and(|(k, _)| {
                    !k.is_empty() && k.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '-')
                })
        })
}

/// Append a trailer-like line: directly after an existing trailer block,
/// otherwise after a blank line.
fn append_trailer(msg: &str, line: &str) -> String {
    let body = msg.trim_end_matches('\n');
    if body.lines().last() == Some(line) {
        return format!("{body}\n");
    }
    let last = body.rsplit("\n\n").next().unwrap_or("");
    if body.contains("\n\n") && is_trailer_para(last) {
        format!("{body}\n{line}\n")
    } else {
        format!("{body}\n\n{line}\n")
    }
}

fn message_for(repo: &Repository, op: Op, oid: &Oid, c: &Commit, o: &Opts) -> CliResult<String> {
    let mut msg = match op {
        Op::Pick => {
            let mut m = c.message.clone();
            if !m.ends_with('\n') {
                m.push('\n');
            }
            if o.record_origin {
                m = append_trailer(&m, &format!("(cherry picked from commit {})", oid.to_hex()));
            }
            m
        }
        Op::Revert => {
            let subject = crate::cmd::log_format::subject_line(&c.message);
            let mut m = if o.reference {
                let date = short_date(&c.author);
                format!(
                    "# *** SAY WHY WE ARE REVERTING ON THE TITLE LINE ***\n\nThis reverts commit {} ({subject}, {date})",
                    abbrev_hex(repo, oid, 7)
                )
            } else {
                let title = match subject
                    .strip_prefix("Revert \"")
                    .and_then(|r| r.strip_suffix('"'))
                {
                    Some(inner) => format!("Reapply \"{inner}\""),
                    None => format!("Revert \"{subject}\""),
                };
                format!("{title}\n\nThis reverts commit {}", oid.to_hex())
            };
            if c.parents.len() > 1 {
                let parent = c.parents[o.mainline.unwrap_or(1) - 1];
                m.push_str(&format!(
                    ", reversing\nchanges made to {}",
                    if o.reference {
                        abbrev_hex(repo, &parent, 7)
                    } else {
                        parent.to_hex()
                    }
                ));
            }
            m.push_str(".\n");
            m
        }
    };
    if o.signoff {
        let me = committer_signature()?;
        msg = append_trailer(&msg, &format!("Signed-off-by: {} <{}>", me.name, me.email));
    }
    Ok(msg)
}

/// The `-n` / conflict hint lines git prints after "could not apply".
fn conflict_hint(op: Op, no_commit: bool) -> String {
    if no_commit {
        "hint: after resolving the conflicts, mark the corrected paths\nhint: with 'git add <paths>' or 'git rm <paths>'".to_string()
    } else {
        let n = op.name();
        format!(
            "hint: After resolving the conflicts, mark them with\nhint: \"git add/rm <pathspec>\", then run\nhint: \"git {n} --continue\".\nhint: You can instead skip this commit with \"git {n} --skip\".\nhint: To abort and get back to the state before \"git {n}\",\nhint: run \"git {n} --abort\"."
        )
    }
}

fn pick_one(op: Op, oid: &Oid, o: &Opts) -> CliResult<()> {
    let mut repo = open_repo()?;
    let c = crate::cmd::log_format::read_commit(&repo, oid)?;
    let head = repo.head_oid()?;
    let head_c = crate::cmd::log_format::read_commit(&repo, &head)?;
    // Parent the change is taken against (`-m` picks it for merges).
    let parent = if c.parents.len() > 1 {
        let Some(m) = o.mainline else {
            return Err(failed(
                op,
                &format!(
                    "commit {} is a merge but no -m option was given.",
                    oid.to_hex()
                ),
            ));
        };
        Some(*c.parents.get(m - 1).ok_or_else(|| {
            failed(
                op,
                &format!("commit {} does not have parent {m}", oid.to_hex()),
            )
        })?)
    } else {
        if let Some(m) = o.mainline {
            if m > 1 || c.parents.is_empty() {
                return Err(failed(
                    op,
                    &format!(
                        "mainline was specified but commit {} is not a merge.",
                        oid.to_hex()
                    ),
                ));
            }
        }
        c.parents.first().copied()
    };
    // The index must match HEAD unless the result is not committed.
    let index = read_index(&repo)?;
    let head_map = tree_blob_map(&repo, &head)?;
    let idx_dirty = index
        .entries
        .iter()
        .any(|e| e.stage() != 0 || head_map.get(&e.path) != Some(&e.oid))
        || head_map.len() != index.entries.iter().filter(|e| e.stage() == 0).count();
    if idx_dirty && !o.no_commit {
        return Err(CliError::Exit(
            128,
            format!(
                "error: your local changes would be overwritten by {}.\nhint: commit your changes or stash them to proceed.\nfatal: {} failed",
                op.name(),
                op.name()
            ),
        ));
    }
    // `--ff`: the commit's parent is HEAD, so HEAD simply moves to it.
    if op == Op::Pick && o.ff && parent == Some(head) {
        let work = work_tree(&repo)?;
        crate::cmd::switch_tree::two_way_switch(&repo, &work, Some(&head), oid)?;
        update_head(&repo, &head, oid, &format!("cherry-pick: fast-forward"))?;
        return Ok(());
    }
    let parent_tree = match parent {
        Some(p) => crate::cmd::log_format::read_commit(&repo, &p)?.tree,
        None => put(&mut repo, ObjectKind::Tree, Vec::new())?,
    };
    let (base_tree, theirs_tree) = match op {
        Op::Pick => (parent_tree, c.tree),
        Op::Revert => (c.tree, parent_tree),
    };
    let work = work_tree(&repo)?;
    // Local changes to paths the pick touches would be overwritten.
    let base_map = tree_map_of(&repo, &base_tree)?;
    let theirs_map = tree_map_of(&repo, &theirs_tree)?;
    let touched: Vec<&String> = base_map
        .keys()
        .chain(theirs_map.keys())
        .filter(|k| base_map.get(*k) != theirs_map.get(*k))
        .collect();
    let mut dirty = Vec::new();
    for p in &touched {
        let entry = index
            .entries
            .iter()
            .find(|e| &e.path == *p && e.stage() == 0);
        let abs = work.join(p);
        let on_disk = fs::read(&abs).ok().map(|b| {
            RawObject::new(ObjectKind::Blob, b)
                .oid(repo.object_format())
                .ok()
        });
        let wt = on_disk.flatten();
        if wt != entry.map(|e| e.oid) && !(wt.is_none() && entry.is_none()) {
            dirty.push((*p).clone());
        } else if o.no_commit && entry.map(|e| e.oid) != head_map.get(*p).copied() {
            dirty.push((*p).clone());
        }
    }
    if !dirty.is_empty() {
        dirty.sort();
        let list: String = dirty.iter().map(|p| format!("\t{p}\n")).collect();
        return Err(CliError::Exit(
            128,
            format!(
                "error: Your local changes to the following files would be overwritten by merge:\n{list}Please commit your changes or stash them before you merge.\nAborting\nfatal: {} failed",
                op.name()
            ),
        ));
    }
    let desc = describe(&repo, oid, &c);
    let style = arks_config::ConfigSet::load(Some(repo.common_dir()))
        .get("merge.conflictStyle")
        .unwrap_or_default();
    let mut favor = Favor::None;
    for x in &o.strategy_options {
        match x.as_str() {
            "ours" => favor = Favor::Ours,
            "theirs" => favor = Favor::Theirs,
            _ => {}
        }
    }
    let labels = MergeLabels {
        ours: "HEAD".into(),
        base: match op {
            Op::Pick => format!("parent of {desc}"),
            Op::Revert => desc.clone(),
        },
        theirs: match op {
            Op::Pick => desc.clone(),
            Op::Revert => format!("parent of {desc}"),
        },
        diff3: matches!(style.as_str(), "diff3" | "zdiff3"),
        favor,
    };
    let result = if o.strategy.as_deref() == Some("ours") {
        // `-s ours`: keep HEAD's tree.
        crate::cmd::merge_engine::MergeResult {
            tree: Some(head_c.tree),
            conflicts: Vec::new(),
            auto_merged: Vec::new(),
        }
    } else {
        merge_trees_worktree(
            &mut repo,
            &work,
            &base_tree,
            &head_c.tree,
            &theirs_tree,
            true,
            &labels,
        )?
    };
    if o.no_commit {
        // The merge rebuilt the index from trees; keep staged changes to
        // paths the pick did not touch (`-n` allows a dirty index).
        let touched: HashSet<&String> = touched.iter().copied().collect();
        let mut now = read_index(&repo)?;
        for e in index.entries.iter().filter(|e| e.stage() == 0) {
            if !touched.contains(&e.path) && head_map.get(&e.path) != Some(&e.oid) {
                now.entries.retain(|x| x.path != e.path);
                now.entries.push(e.clone());
            }
        }
        let kept: HashSet<&String> = index.entries.iter().map(|e| &e.path).collect();
        now.entries.retain(|x| {
            touched.contains(&x.path) || kept.contains(&x.path) || !head_map.contains_key(&x.path)
        });
        arks_index::sort_entries(&mut now.entries);
        crate::util::write_index(&repo, &now)?;
    }
    let mut msg = message_for(&repo, op, oid, &c, o)?;
    for p in result
        .auto_merged
        .iter()
        .filter(|p| !result.conflicts.contains(p))
    {
        println!("Auto-merging {p}");
    }
    let git_dir = repo.git_dir().to_path_buf();
    let write_head_file = |with: bool| -> CliResult<()> {
        if with {
            fs::write(git_dir.join(op.head_file()), format!("{}\n", oid.to_hex()))?;
        }
        Ok(())
    };
    // Revert keeps REVERT_HEAD even with --no-commit; cherry-pick does not.
    let keep_head_file = !o.no_commit || op == Op::Revert;
    if !result.conflicts.is_empty() {
        for p in &result.conflicts {
            let kind = match (
                base_map.contains_key(p),
                head_map.contains_key(p),
                theirs_map.contains_key(p),
            ) {
                (true, true, true) => "content",
                (false, true, true) => "add/add",
                _ => "modify/delete",
            };
            if kind == "modify/delete" {
                let theirs_label = if parent.is_none() && op == Op::Revert {
                    "(empty tree)".to_string()
                } else {
                    labels.theirs.clone()
                };
                let (gone, kept) = if head_map.contains_key(p) {
                    (theirs_label, "HEAD".to_string())
                } else {
                    ("HEAD".to_string(), theirs_label)
                };
                println!(
                    "CONFLICT (modify/delete): {p} deleted in {gone} and modified in {kept}.  Version {kept} of {p} left in tree."
                );
            } else {
                println!("Auto-merging {p}");
                println!("CONFLICT ({kind}): Merge conflict in {p}");
            }
        }
        msg.push_str("\n# Conflicts:\n");
        for p in &result.conflicts {
            msg.push_str(&format!("#\t{p}\n"));
        }
        fs::write(git_dir.join("MERGE_MSG"), &msg)?;
        write_head_file(keep_head_file)?;
        return Err(CliError::Exit(
            1,
            format!(
                "error: could not {} {}... {}\n{}",
                if op == Op::Pick { "apply" } else { "revert" },
                abbrev_hex(&repo, oid, 7),
                crate::cmd::log_format::subject_line(&c.message),
                conflict_hint(op, o.no_commit)
            ),
        ));
    }
    let tree = result
        .tree
        .ok_or_else(|| CliError::Msg("merge: missing result tree".into()))?;
    if o.no_commit {
        fs::write(git_dir.join("MERGE_MSG"), &msg)?;
        write_head_file(keep_head_file)?;
        return Ok(());
    }
    let originally_empty = parent_tree == c.tree;
    if tree == head_c.tree && !(o.keep_redundant || (originally_empty && o.allow_empty)) {
        fs::write(git_dir.join("MERGE_MSG"), &msg)?;
        write_head_file(true)?;
        drop(repo);
        let repo = open_repo()?;
        crate::cmd::status_report::print_long_status(&repo)?;
        let n = op.name();
        return Err(CliError::Exit(
            1,
            format!(
                "The previous {n} is now empty, possibly due to conflict resolution.\nIf you wish to commit it anyway, use:\n\n    git commit --allow-empty\n"
            ),
        ));
    }
    commit_result(&mut repo, op, oid, &c, tree, head, msg, o)
}

#[allow(clippy::too_many_arguments)]
fn commit_result(
    repo: &mut Repository,
    op: Op,
    oid: &Oid,
    c: &Commit,
    tree: Oid,
    head: Oid,
    mut msg: String,
    o: &Opts,
) -> CliResult<()> {
    let git_dir = repo.git_dir().to_path_buf();
    if o.edit {
        msg = edit_message(&git_dir, &msg)?;
    }
    let mode = match o.cleanup.as_deref() {
        Some(m) => crate::cmd::trailers::parse_cleanup(m)?,
        None if o.edit => Cleanup::Default,
        None => Cleanup::Whitespace,
    };
    let msg = crate::cmd::trailers::cleanup(&msg, mode, o.edit);
    if msg.trim().is_empty() && !o.allow_empty_message {
        fs::write(git_dir.join("MERGE_MSG"), &msg)?;
        return Err(CliError::Exit(
            1,
            "Aborting commit due to empty commit message.".into(),
        ));
    }
    let author = match op {
        Op::Pick => c.author.clone(),
        Op::Revert => crate::cmd::shared::commit_signature()?,
    };
    let mut commit = Commit {
        tree,
        parents: vec![head],
        author,
        committer: committer_signature()?,
        gpgsig: None,
        message: msg,
    };
    let (do_sign, key) = crate::cmd::sign::want_sign_commit(&o.sign_args, repo)?;
    if do_sign {
        if let Err(e) = crate::cmd::sign::sign_commit(repo, &mut commit, key.as_deref()) {
            fs::write(git_dir.join("MERGE_MSG"), &commit.message)?;
            if op == Op::Pick {
                fs::write(git_dir.join(op.head_file()), format!("{}\n", oid.to_hex()))?;
            }
            return Err(CliError::Exit(
                128,
                format!(
                    "{e}\nerror: failed to write commit object\nfatal: {} failed",
                    op.name()
                ),
            ));
        }
    }
    let subject = crate::cmd::log_format::subject_line(&commit.message);
    let coid = put(repo, ObjectKind::Commit, commit.encode()?)?;
    update_head(repo, &head, &coid, &format!("{}: {subject}", op.name()))?;
    for f in [op.head_file(), "MERGE_MSG", "AUTO_MERGE"] {
        let _ = fs::remove_file(git_dir.join(f));
    }
    if !crate::util::quiet() {
        // An edited message goes through `git commit`, which shows the date
        // only when it differs; otherwise the sequencer always shows it.
        crate::cmd::core::print_commit_summary_dated(repo, &coid, !o.edit)?;
    }
    Ok(())
}

fn edit_message(git_dir: &std::path::Path, msg: &str) -> CliResult<String> {
    let path = git_dir.join("COMMIT_EDITMSG");
    fs::write(&path, msg)?;
    if let Some(editor) = ["ARKS_EDITOR", "GIT_EDITOR", "EDITOR"]
        .iter()
        .find_map(|v| std::env::var(v).ok().filter(|s| !s.is_empty()))
    {
        let ok = std::process::Command::new("sh")
            .arg("-c")
            .arg(format!("{editor} \"$@\""))
            .arg(&editor)
            .arg(&path)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if !ok {
            return Err(CliError::Exit(
                1,
                "error: there was a problem with the editor".into(),
            ));
        }
    }
    Ok(fs::read_to_string(&path)?)
}

fn put(repo: &mut Repository, kind: ObjectKind, payload: Vec<u8>) -> CliResult<Oid> {
    let format = repo.object_format();
    Ok(repo
        .store_mut()
        .put(format, &RawObject::new(kind, payload))?)
}

fn tree_map_of(repo: &Repository, tree: &Oid) -> CliResult<std::collections::HashMap<String, Oid>> {
    let mut out = std::collections::HashMap::new();
    let mut stack = vec![(String::new(), *tree)];
    while let Some((prefix, t)) = stack.pop() {
        let obj = repo.get_object(&t)?;
        for e in arks_object::Tree::decode(&obj.payload, repo.oid_kind())?.entries {
            let name = format!("{prefix}{}", e.name);
            if e.mode.is_tree() {
                stack.push((format!("{name}/"), e.oid));
            } else {
                out.insert(name, e.oid);
            }
        }
    }
    Ok(out)
}

fn update_head(repo: &Repository, old: &Oid, new: &Oid, reflog: &str) -> CliResult<()> {
    let mut r = open_repo()?;
    match repo.head_target()? {
        RefTarget::Symbolic(branch) => {
            r.refs_mut().update(&branch, new, Some(old))?;
            let _ = crate::reflog::append_head_and_branch(repo, &branch, Some(old), new, reflog);
        }
        RefTarget::Direct(_) => {
            r.refs_mut().update("HEAD", new, Some(old))?;
            let _ = crate::reflog::append_reflog(repo, "HEAD", Some(old), new, reflog);
        }
    }
    Ok(())
}

/// Which operation a stopped pick belongs to, from its pseudo-ref.
fn stopped_op(repo: &Repository) -> Option<Op> {
    if repo.git_dir().join("CHERRY_PICK_HEAD").is_file() {
        Some(Op::Pick)
    } else if repo.git_dir().join("REVERT_HEAD").is_file() {
        Some(Op::Revert)
    } else {
        None
    }
}

fn quit() -> CliResult<()> {
    let repo = open_repo()?;
    let _ = fs::remove_dir_all(sequencer_dir(&repo));
    for f in ["CHERRY_PICK_HEAD", "REVERT_HEAD", "MERGE_MSG"] {
        let _ = fs::remove_file(repo.git_dir().join(f));
    }
    Ok(())
}

fn abort(op: Op) -> CliResult<()> {
    let repo = open_repo()?;
    let seq = sequencer_dir(&repo);
    let target = if let Ok(h) = fs::read_to_string(seq.join("head")) {
        Some(resolve_rev(&repo, h.trim())?)
    } else if stopped_op(&repo).is_some() {
        Some(repo.head_oid()?)
    } else {
        None
    };
    let Some(target) = target else {
        return Err(failed(op, "no cherry-pick or revert in progress"));
    };
    let head = repo.head_oid()?;
    let work = work_tree(&repo)?;
    crate::cmd::working_tree::hard_reset_tree(&repo, &work, &target)?;
    if head != target {
        update_head(&repo, &head, &target, "cherry-pick: abort")?;
    }
    clear_state(&repo);
    Ok(())
}

fn skip(op: Op) -> CliResult<()> {
    let repo = open_repo()?;
    if stopped_op(&repo) != Some(op) {
        return Err(failed(op, &format!("no {} in progress", op.name())));
    }
    let head = repo.head_oid()?;
    let work = work_tree(&repo)?;
    crate::cmd::working_tree::hard_reset_tree(&repo, &work, &head)?;
    for f in ["CHERRY_PICK_HEAD", "REVERT_HEAD", "MERGE_MSG", "AUTO_MERGE"] {
        let _ = fs::remove_file(repo.git_dir().join(f));
    }
    let todo = read_todo(&repo)?;
    if todo.len() <= 1 {
        let _ = fs::remove_dir_all(sequencer_dir(&repo));
        return Ok(());
    }
    let opts = read_opts(&sequencer_dir(&repo));
    drop(repo);
    run_todo(&todo[1..], &opts)
}

/// `--continue`: commit the resolved pick, then run the rest of the todo.
fn resume(op: Op) -> CliResult<()> {
    let mut repo = open_repo()?;
    let seq = sequencer_dir(&repo);
    let todo = read_todo(&repo)?;
    let stopped = stopped_op(&repo);
    if stopped.is_none() && !seq.is_dir() {
        return Err(failed(op, "no cherry-pick or revert in progress"));
    }
    let opts = read_opts(&seq);
    let mut rest: &[(Op, Oid)] = &todo;
    if let Some(sop) = stopped {
        let index = read_index(&repo)?;
        if index.entries.iter().any(|e| e.stage() != 0) {
            return Err(CliError::Exit(
                128,
                format!(
                    "error: Committing is not possible because you have unmerged files.\nhint: Fix them up in the work tree, and then use 'git add/rm <file>'\nhint: as appropriate to mark resolution and make a commit.\nfatal: {} failed",
                    op.name()
                ),
            ));
        }
        let picked = fs::read_to_string(repo.git_dir().join(sop.head_file()))?;
        let picked = resolve_rev(&repo, picked.trim())?;
        let c = crate::cmd::log_format::read_commit(&repo, &picked)?;
        let head = repo.head_oid()?;
        let msg = fs::read_to_string(repo.git_dir().join("MERGE_MSG"))
            .unwrap_or_else(|_| c.message.clone());
        let msg = crate::cmd::trailers::cleanup(&msg, Cleanup::Strip, true);
        let format = repo.object_format();
        let tree =
            crate::write_tree::write_tree_from_index(repo.store_mut(), &index.entries, format)?;
        let mut o = opts.clone();
        o.cleanup = Some("verbatim".into());
        o.edit = false;
        commit_result(&mut repo, sop, &picked, &c, tree, head, msg, &o)?;
        if rest.first().map(|(_, x)| *x) == Some(picked) {
            rest = &rest[1..];
        }
    }
    if rest.is_empty() {
        let repo = open_repo()?;
        let _ = fs::remove_dir_all(sequencer_dir(&repo));
        return Ok(());
    }
    let rest = rest.to_vec();
    drop(repo);
    run_todo(&rest, &opts)
}

/// `--date=short` (`YYYY-MM-DD` in the signature's own zone).
fn short_date(sig: &arks_object::Signature) -> String {
    let tz = sig.timezone.as_str();
    let sign = if tz.starts_with('-') { -1 } else { 1 };
    let digits = tz.trim_start_matches(['+', '-']);
    let off = if digits.len() == 4 {
        let h: i64 = digits[..2].parse().unwrap_or(0);
        let m: i64 = digits[2..].parse().unwrap_or(0);
        sign * (h * 3600 + m * 60)
    } else {
        0
    };
    let days = (sig.timestamp + off).div_euclid(86_400);
    // Civil date from days since 1970-01-01 (proleptic Gregorian).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}
