//! `git rebase`: every option of `git rebase -h` on one todo-list engine.
//!
//! The commits of `<upstream>..<branch>` (or a `--root` walk) become a todo
//! list in `.git/rebase-merge/git-rebase-todo` (`rebase-apply` for the apply
//! backend); HEAD is detached at the new base and each line runs in turn:
//! `pick` replays a commit with a three-way merge against its parent (or
//! fast-forwards when the parent already is HEAD), `reword` / `edit` /
//! `squash` / `fixup` / `exec` / `break` / `drop` / `update-ref` follow
//! git-rebase(1). A conflict, `edit`, `break` or failed `exec` stops with
//! the state on disk for `--continue`, `--skip`, `--abort`, `--quit`,
//! `--edit-todo` and `--show-current-patch`.
//! Cite: <https://git-scm.com/docs/git-rebase> (behaviour, not code).

use crate::cmd::merge_engine::{Favor, MergeLabels, merge_trees_worktree};
use crate::cmd::opt_parse::{Item, Mode, Parsed, parse};
use crate::cmd::shared::{abbrev_hex, committer_signature, resolve_rev};
use crate::cmd::trailers::Cleanup;
use crate::error::{CliError, CliResult};
use crate::util::{open_repo, read_index, work_tree};
use arks_object::{Commit, ObjectKind, RawObject};
use arks_oid::Oid;
use arks_refs::RefTarget;
use arks_repo::Repository;
use arks_store::ObjectStore;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

/// Options that shape every step; persisted as marker files in the state dir.
#[derive(Clone, Default)]
struct Opts {
    quiet: bool,
    verbose: bool,
    stat: bool,
    signoff: bool,
    force: bool,
    committer_date_is_author_date: bool,
    reset_author_date: bool,
    apply: bool,
    interactive: bool,
    empty: Option<String>,
    keep_empty: bool,
    allow_empty_message: bool,
    update_refs: bool,
    reapply_cherry_picks: bool,
    autostash: bool,
    favor: Favor,
    strategy: Option<String>,
    sign_args: Vec<String>,
    exec: Vec<String>,
}

const OPT_FLAGS: &[&str] = &[
    "quiet",
    "verbose",
    "stat",
    "signoff",
    "force",
    "committer-date-is-author-date",
    "reset-author-date",
    "interactive",
    "keep-empty",
    "allow-empty-message",
    "update-refs",
];

impl Opts {
    fn flags(&self) -> [bool; 11] {
        [
            self.quiet,
            self.verbose,
            self.stat,
            self.signoff,
            self.force,
            self.committer_date_is_author_date,
            self.reset_author_date,
            self.interactive,
            self.keep_empty,
            self.allow_empty_message,
            self.update_refs,
        ]
    }
}

fn state_dir(repo: &Repository) -> Option<PathBuf> {
    ["rebase-merge", "rebase-apply"]
        .iter()
        .map(|d| repo.git_dir().join(d))
        .find(|d| d.is_dir())
}

fn no_rebase() -> CliError {
    CliError::fatal("No rebase in progress?")
}

pub fn cmd_rebase(args: &[String]) -> CliResult<()> {
    let p = parse(
        "rebase",
        args,
        crate::cmd::porcelain_opts::REBASE,
        Mode::default(),
    )?;
    let actions: Vec<&str> = [
        "continue",
        "skip",
        "abort",
        "quit",
        "edit-todo",
        "show-current-patch",
    ]
    .into_iter()
    .filter(|a| p.on(a))
    .collect();
    if actions.len() > 1 {
        return Err(CliError::Exit(
            129,
            format!(
                "error: options '--{}' and '--{}' cannot be used together",
                actions[0], actions[1]
            ),
        ));
    }
    if let Some(a) = actions.first() {
        let repo = open_repo()?;
        let Some(dir) = state_dir(&repo) else {
            return Err(no_rebase());
        };
        drop(repo);
        return match *a {
            "continue" => resume(&dir, true),
            "skip" => skip(&dir),
            "abort" => abort(&dir),
            "quit" => {
                fs::remove_dir_all(&dir)?;
                Ok(())
            }
            "edit-todo" => edit_todo(&dir),
            _ => {
                let repo = open_repo()?;
                let stopped = fs::read_to_string(repo.git_dir().join("REBASE_HEAD"))
                    .map_err(|_| CliError::fatal("No rebase in progress?"))?;
                crate::cmd::cmd_show(&[stopped.trim().to_string()])
            }
        };
    }
    start(&p)
}

fn options(p: &Parsed) -> CliResult<Opts> {
    let mut favor = Favor::None;
    for x in p.values("strategy-option") {
        match x.as_str() {
            "ours" => favor = Favor::Ours,
            "theirs" => favor = Favor::Theirs,
            _ => {}
        }
    }
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
    let empty = p.value("empty");
    if let Some(e) = &empty {
        if !matches!(e.as_str(), "drop" | "keep" | "ask" | "stop") {
            return Err(CliError::fatal(format!(
                "unrecognized empty type '{e}'; valid values are \"drop\", \"keep\", and \"ask\"."
            )));
        }
    }
    let config = arks_config::ConfigSet::load(
        open_repo()
            .ok()
            .map(|r| r.common_dir().to_path_buf())
            .as_deref(),
    );
    let truthy = |k: &str| {
        config
            .get(k)
            .is_some_and(|v| matches!(v.to_ascii_lowercase().as_str(), "true" | "yes" | "on" | "1"))
    };
    // Backend choice (pin 2.39.5): the last of `--apply`, `--merge` and
    // `--interactive` wins; `--whitespace` / `-C` are apply-only options.
    let backend = p.items.iter().rev().find_map(|it| match it {
        Item::Opt {
            key: k @ ("apply" | "merge" | "interactive"),
            neg: false,
            ..
        } => Some(*k),
        _ => None,
    });
    let apply_opts = p.value("whitespace").is_some() || p.value("-C").is_some();
    let mut o = Opts {
        quiet: p.on("quiet"),
        verbose: p.on("verbose"),
        stat: p.flag("no-stat") == Some(false)
            || (p.flag("no-stat").is_none() && truthy("rebase.stat")),
        signoff: p.on("signoff"),
        force: p.on("force-rebase") || p.on("no-ff"),
        committer_date_is_author_date: p.on("committer-date-is-author-date"),
        reset_author_date: p.on("reset-author-date") || p.on("ignore-date"),
        apply: backend == Some("apply") || apply_opts,
        interactive: p.on("interactive") && backend != Some("apply"),
        empty,
        keep_empty: p.on("keep-empty"),
        allow_empty_message: p.on("allow-empty-message"),
        update_refs: p
            .flag("update-refs")
            .unwrap_or_else(|| truthy("rebase.updateRefs")),
        reapply_cherry_picks: p.on("reapply-cherry-picks"),
        autostash: p
            .flag("autostash")
            .unwrap_or_else(|| truthy("rebase.autoStash")),
        favor,
        strategy,
        sign_args: crate::cmd::commit::sign_args(p),
        exec: p.values("exec"),
    };
    if o.verbose {
        o.stat = true;
    }
    if apply_opts && (backend.is_some_and(|b| b != "apply") || !o.exec.is_empty()) {
        return Err(CliError::fatal(
            "apply options and merge options cannot be used together",
        ));
    }
    if o.apply {
        // Pin 2.39.5: options that need the merge backend fail one by one;
        // `--update-refs` is ignored by the apply backend.
        for (on, name) in [
            (!o.exec.is_empty(), "--exec"),
            (o.keep_empty, "--keep-empty"),
            (o.empty.is_some(), "--empty"),
        ] {
            if on {
                return Err(CliError::fatal(format!(
                    "{name} requires the merge backend"
                )));
            }
        }
        o.update_refs = false;
    }
    Ok(o)
}

/// Start a new rebase.
fn start(p: &Parsed) -> CliResult<()> {
    let o = options(p)?;
    let mut repo = open_repo()?;
    crate::cmd::replay::refuse_exclusive_sequencer_state(
        &repo,
        crate::cmd::replay::SequencerOp::Rebase,
    )?;
    if state_dir(&repo).is_some() {
        return Err(CliError::fatal(
            "It seems that there is already a rebase-merge directory, and\nI wonder if you are in the middle of another rebase.",
        ));
    }
    let editor = sequence_editor();
    if o.interactive && editor.is_none() {
        return Err(CliError::Usage(
            "rebase -i requires a sequence/editor: set GIT_SEQUENCE_EDITOR / ARKS_SEQUENCE_EDITOR or GIT_EDITOR / ARKS_EDITOR / EDITOR (FS1)"
                .into(),
        ));
    }
    let mut pos = p.args();
    pos.extend(p.after_dashdash());
    let root = p.on("root");
    if pos.len() > if root { 1 } else { 2 } {
        return Err(CliError::Exit(
            129,
            "usage: git rebase [-i] [options] [--exec <cmd>] [--onto <newbase> | --keep-base] [<upstream> [<branch>]]\n   or: git rebase [-i] [options] [--exec <cmd>] [--onto <newbase>] --root [<branch>]\n   or: git rebase --continue | --abort | --skip | --edit-todo".into(),
        ));
    }
    let (upstream_arg, branch_arg) = if root {
        (None, pos.first().cloned())
    } else {
        (pos.first().cloned(), pos.get(1).cloned())
    };
    let resolve_commit = |repo: &Repository, rev: &str| -> CliResult<Oid> {
        let oid = resolve_rev(repo, rev)?;
        crate::cmd::shared::peel_to_kind(repo, &oid, crate::cmd::shared::ObjKind::Commit, rev)
    };
    // Switch to <branch> first, as git does.
    if let Some(b) = &branch_arg {
        if repo.refs().resolve(&format!("refs/heads/{b}")).is_err()
            && resolve_rev(&repo, b).is_err()
        {
            return Err(CliError::fatal(format!("no such branch/commit '{b}'")));
        }
    }
    // The upstream: argument, else the branch's configured upstream.
    let upstream: Option<Oid> = match &upstream_arg {
        Some(u) => Some(
            resolve_commit(&repo, u)
                .map_err(|_| CliError::fatal(format!("invalid upstream '{u}'")))?,
        ),
        None if root => None,
        None => {
            let branch = crate::cmd::shared::current_branch_name(&repo).ok();
            let cfg = arks_config::ConfigSet::load(Some(repo.common_dir()));
            let up = branch.as_ref().and_then(|b| {
                let remote = cfg.get(&format!("branch.{b}.remote"))?;
                let merge = cfg.get(&format!("branch.{b}.merge"))?;
                let short = merge.trim_start_matches("refs/heads/").to_string();
                Some(if remote == "." {
                    format!("refs/heads/{short}")
                } else {
                    format!("refs/remotes/{remote}/{short}")
                })
            });
            match up.and_then(|u| repo.refs().resolve(&u).ok()) {
                Some(o) => Some(o),
                None => {
                    let b = branch.unwrap_or_else(|| "<branch>".into());
                    print!(
                        "{}",
                        format_args!(
                            "There is no tracking information for the current branch.\nPlease specify which branch you want to rebase against.\nSee git-rebase(1) for details.\n\n    git rebase '<branch>'\n\nIf you wish to set tracking information for this branch you can do so with:\n\n    git branch --set-upstream-to=<remote>/<branch> {b}\n\n"
                        )
                    );
                    return Err(CliError::silent(1));
                }
            }
        }
    };
    let onto_arg = p.value("onto");
    if onto_arg.is_some() && p.on("keep-base") {
        return Err(CliError::fatal(
            "options '--keep-base' and '--onto' cannot be used together",
        ));
    }
    if root && p.on("keep-base") {
        return Err(CliError::fatal(
            "options '--keep-base' and '--root' cannot be used together",
        ));
    }
    // Local changes block a rebase unless --autostash.
    let dirty = worktree_state(&repo)?;
    if let Some(msg) = &dirty {
        if !o.autostash {
            return Err(CliError::Exit(1, msg.clone()));
        }
    }
    let mut stashed = false;
    if dirty.is_some() && branch_arg.is_some() {
        drop(repo);
        stashed = crate::cmd::stash::autostash_push()?;
        repo = open_repo()?;
    }
    if let Some(b) = &branch_arg {
        drop(repo);
        let r = open_repo()?;
        let head_name = match r.head_target()? {
            RefTarget::Symbolic(s) => s,
            RefTarget::Direct(_) => String::new(),
        };
        if head_name != format!("refs/heads/{b}") {
            drop(r);
            crate::cmd::cmd_checkout(&["-q".into(), b.clone()])?;
        }
    } else {
        drop(repo);
    }
    let mut repo = open_repo()?;
    let head = repo.head_oid()?;
    let head_name = match repo.head_target()? {
        RefTarget::Symbolic(s) => s,
        RefTarget::Direct(_) => "detached HEAD".into(),
    };
    let branch_label = head_name
        .strip_prefix("refs/heads/")
        .unwrap_or(&head_name)
        .to_string();
    let onto: Oid = match (&onto_arg, p.on("keep-base"), upstream) {
        (Some(x), _, _) => {
            let (a, b) = match x.split_once("...") {
                Some((a, b)) => (Some(a), Some(b)),
                None => (None, None),
            };
            if let (Some(a), Some(b)) = (a, b) {
                let a = resolve_commit(&repo, if a.is_empty() { "HEAD" } else { a })?;
                let b = resolve_commit(&repo, if b.is_empty() { "HEAD" } else { b })?;
                arks_repo::merge_base_oid(&repo, &a, &b)?
                    .ok_or_else(|| CliError::fatal(format!("'{x}': need exactly one merge base")))?
            } else {
                resolve_commit(&repo, x).map_err(|_| {
                    CliError::fatal(format!("Does not point to a valid commit '{x}'"))
                })?
            }
        }
        (None, true, Some(u)) => arks_repo::merge_base_oid(&repo, &u, &head)?
            .ok_or_else(|| CliError::fatal("no merge base for --keep-base"))?,
        (None, _, Some(u)) => u,
        (None, _, None) => {
            // --root without --onto: replay onto an empty start.
            Oid::from_hex(&"0".repeat(head.to_hex().len())).unwrap_or(head)
        }
    };
    let root_onto = root && onto_arg.is_none();
    // Commits to replay, oldest first.
    let stop: HashSet<Oid> = match upstream {
        Some(u) => arks_repo::ancestor_set(&repo, u)?,
        None if !root_onto => arks_repo::ancestor_set(&repo, onto)?,
        None => HashSet::new(),
    };
    let mut commits = Vec::new();
    let mut has_merges = false;
    let mut queue = vec![head];
    let mut seen = HashSet::new();
    while let Some(c) = queue.pop() {
        if stop.contains(&c) || !seen.insert(c) {
            continue;
        }
        let commit = read_commit(&repo, &c)?;
        queue.extend(commit.parents.iter().copied());
        if commit.parents.len() > 1 {
            has_merges = true;
            if p.on("rebase-merges") {
                return Err(CliError::NotImplemented(
                    "rebase --rebase-merges with merge commits in the range is not implemented"
                        .into(),
                ));
            }
            continue;
        }
        commits.push((commit.committer.timestamp, c));
    }
    // Topological, oldest first: parents before children.
    let order = topo_oldest_first(&repo, commits.iter().map(|(_, c)| *c).collect())?;
    // Drop commits whose change is already upstream (patch-equivalent).
    let mut skipped = Vec::new();
    let mut todo_commits = Vec::new();
    if let (false, Some(u)) = (o.reapply_cherry_picks, upstream) {
        let ours_set = arks_repo::ancestor_set(&repo, head)?;
        let mut theirs_changes = HashSet::new();
        let mut q = vec![u];
        let mut seen = HashSet::new();
        while let Some(c) = q.pop() {
            if ours_set.contains(&c) || !seen.insert(c) {
                continue;
            }
            let commit = read_commit(&repo, &c)?;
            q.extend(commit.parents.iter().copied());
            if commit.parents.len() == 1 {
                theirs_changes.insert(change_key(&repo, &commit)?);
            }
        }
        for c in order {
            let commit = read_commit(&repo, &c)?;
            if !theirs_changes.is_empty() && theirs_changes.contains(&change_key(&repo, &commit)?) {
                skipped.push(c);
            } else {
                todo_commits.push(c);
            }
        }
    } else {
        todo_commits = order;
    }
    // Nothing to do: the branch already sits on onto.
    let mb = if root_onto {
        None
    } else {
        arks_repo::merge_base_oid(&repo, &onto, &head)?
    };
    let up_to_date = !root_onto
        && mb == Some(onto)
        && upstream.is_some_and(|u| {
            u == onto
                || p.on("keep-base")
                || arks_repo::ancestor_set(&repo, head)
                    .map(|s| s.contains(&u))
                    .unwrap_or(false)
        })
        && todo_commits.iter().all(|c| {
            read_commit(&repo, c)
                .map(|cc| cc.parents.len() == 1)
                .unwrap_or(false)
        })
        && !has_merges
        && o.exec.is_empty()
        && !o.interactive
        && skipped.is_empty();
    if up_to_date
        && !o.force
        && !o.signoff
        && !o.committer_date_is_author_date
        && !o.reset_author_date
    {
        if !o.quiet {
            println!("Current branch {branch_label} is up to date.");
        }
        return Ok(());
    }
    if up_to_date && o.force && !o.quiet {
        println!("Current branch {branch_label} is up to date, rebase forced.");
    }
    if o.stat && !root_onto {
        if let Some(m) = mb {
            if o.verbose {
                println!("Changes from {} to {}:", m.to_hex(), onto.to_hex());
            }
            print_stat(&repo, &m, &onto, true)?;
            if o.verbose {
                print_stat(&repo, &m, &onto, false)?;
            }
        }
    }
    for c in &skipped {
        eprintln!(
            "warning: skipped previously applied commit {}",
            abbrev_hex(&repo, c, 7)
        );
    }
    if !skipped.is_empty() {
        eprintln!(
            "hint: use --reapply-cherry-picks to include skipped commits\nhint: Disable this message with \"git config advice.skippedCherryPicks false\""
        );
    }
    // Build the todo list.
    let mut todo = Vec::new();
    let others: Vec<(String, Oid)> = if o.update_refs {
        repo.refs()
            .list_refs("refs/heads/")?
            .into_iter()
            .filter(|(n, _)| *n != head_name)
            .collect()
    } else {
        Vec::new()
    };
    for c in &todo_commits {
        let commit = read_commit(&repo, c)?;
        todo.push(format!(
            "pick {} {}",
            abbrev_hex(&repo, c, 7),
            crate::cmd::log_format::subject_line(&commit.message)
        ));
        for x in &o.exec {
            todo.push(format!("exec {x}"));
        }
        for (name, oid) in &others {
            if oid == c {
                todo.push(format!("update-ref {name}"));
            }
        }
    }
    let dir = repo.git_dir().join(if o.apply {
        "rebase-apply"
    } else {
        "rebase-merge"
    });
    fs::create_dir_all(&dir)?;
    fs::write(dir.join("head-name"), format!("{head_name}\n"))?;
    fs::write(dir.join("orig-head"), format!("{}\n", head.to_hex()))?;
    fs::write(dir.join("onto"), format!("{}\n", onto.to_hex()))?;
    write_opts(&dir, &o)?;
    if o.interactive {
        fs::write(dir.join("interactive"), "")?;
    }
    let mut text = todo.join("\n");
    if !text.is_empty() {
        text.push('\n');
    }
    fs::write(dir.join("git-rebase-todo"), &text)?;
    fs::write(dir.join("end"), format!("{}\n", todo.len()))?;
    if o.interactive {
        let path = dir.join("git-rebase-todo");
        let mut with_help = text.clone();
        with_help.push_str(&format!(
            "\n# Rebase {}..{} onto {} ({} command{})\n#\n# Commands:\n# p, pick <commit> = use commit\n# r, reword <commit> = use commit, but edit the commit message\n# e, edit <commit> = use commit, but stop for amending\n# s, squash <commit> = use commit, but meld into previous commit\n# f, fixup <commit> = like \"squash\" but keep only the previous commit's log message\n# x, exec <command> = run command (the rest of the line) using shell\n# b, break = stop here (continue rebase later with 'git rebase --continue')\n# d, drop <commit> = remove commit\n# u, update-ref <ref> = track a placeholder for the <ref> to be updated\n#\n# These lines can be re-ordered; they are executed from top to bottom.\n#\n# If you remove a line here THAT COMMIT WILL BE LOST.\n#\n# However, if you remove everything, the rebase will be aborted.\n#\n",
            upstream.map(|u| abbrev_hex(&repo, &u, 7)).unwrap_or_default(),
            abbrev_hex(&repo, &head, 7),
            if root_onto { "root".to_string() } else { abbrev_hex(&repo, &onto, 7) },
            todo.len(),
            if todo.len() == 1 { "" } else { "s" }
        ));
        fs::write(&path, with_help)?;
        if let Some(ed) = &editor {
            run_editor(ed, &path)?;
        }
        let edited = fs::read_to_string(&path)?;
        let kept: Vec<&str> = edited
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .collect();
        if kept.is_empty() {
            fs::remove_dir_all(&dir)?;
            return Err(CliError::Exit(1, "error: nothing to do".into()));
        }
        let mut t = kept.join("\n");
        t.push('\n');
        fs::write(&path, t)?;
        fs::write(dir.join("end"), format!("{}\n", kept.len()))?;
    }
    if stashed || (dirty.is_some() && crate::cmd::stash::autostash_push()?) {
        fs::write(dir.join("autostash"), "")?;
    }
    let _ = repo.refs_mut().update("ORIG_HEAD", &head, None);
    // Detach HEAD at the new base.
    let work = work_tree(&repo)?;
    if root_onto {
        // Start from nothing: the first pick recreates the root commit.
        fs::write(dir.join("root"), "")?;
    } else {
        crate::cmd::switch_tree::two_way_switch(&repo, &work, Some(&head), &onto)?;
        repo.refs_mut().update("HEAD", &onto, None)?;
        let _ = crate::reflog::append_reflog(
            &repo,
            "HEAD",
            Some(&head),
            &onto,
            &format!(
                "rebase (start): checkout {}",
                onto_arg
                    .clone()
                    .or(upstream_arg.clone())
                    .unwrap_or_else(|| onto.to_hex())
            ),
        );
    }
    if o.apply && !o.quiet {
        println!("First, rewinding head to replay your work on top of it...");
    }
    drop(repo);
    run(&dir)
}

/// git's refusal for local changes, or `None` when the tree is clean.
fn worktree_state(repo: &Repository) -> CliResult<Option<String>> {
    let d = crate::cmd::status_report::collect(repo, crate::cmd::status_report::Untracked::No)?;
    let unstaged = !d.unstaged.is_empty() || !d.unmerged.is_empty();
    if unstaged && !d.staged.is_empty() {
        return Ok(Some(
            "error: cannot rebase: You have unstaged changes.\nerror: additionally, your index contains uncommitted changes.\nerror: Please commit or stash them."
                .into(),
        ));
    }
    if unstaged {
        return Ok(Some(
            "error: cannot rebase: You have unstaged changes.\nerror: Please commit or stash them."
                .into(),
        ));
    }
    if !d.staged.is_empty() {
        return Ok(Some(
            "error: cannot rebase: Your index contains uncommitted changes.\nerror: Please commit or stash them."
                .into(),
        ));
    }
    Ok(None)
}

fn topo_oldest_first(repo: &Repository, set: Vec<Oid>) -> CliResult<Vec<Oid>> {
    let members: HashSet<Oid> = set.iter().copied().collect();
    let mut out = Vec::new();
    let mut done = HashSet::new();
    // Order by committer date first so independent commits stay stable.
    let mut sorted = Vec::new();
    for c in &set {
        sorted.push((read_commit(repo, c)?.committer.timestamp, *c));
    }
    sorted.sort_by_key(|x| x.0);
    fn visit(
        repo: &Repository,
        c: Oid,
        members: &HashSet<Oid>,
        done: &mut HashSet<Oid>,
        out: &mut Vec<Oid>,
    ) -> CliResult<()> {
        if !members.contains(&c) || !done.insert(c) {
            return Ok(());
        }
        for p in read_commit(repo, &c)?.parents {
            visit(repo, p, members, done, out)?;
        }
        out.push(c);
        Ok(())
    }
    for (_, c) in sorted {
        visit(repo, c, &members, &mut done, &mut out)?;
    }
    Ok(out)
}

/// What a commit changes (path, before, after), to spot patch-equivalent picks.
/// (path, before, after) for each path a commit changes.
type ChangeKey = Vec<(String, Option<Oid>, Option<Oid>)>;

fn change_key(repo: &Repository, c: &Commit) -> CliResult<ChangeKey> {
    let mine = tree_map(repo, &c.tree)?;
    let parent = match c.parents.first() {
        Some(p) => tree_map(repo, &read_commit(repo, p)?.tree)?,
        None => HashMap::new(),
    };
    let mut keys: Vec<&String> = mine.keys().chain(parent.keys()).collect();
    keys.sort();
    keys.dedup();
    Ok(keys
        .into_iter()
        .filter(|k| mine.get(*k) != parent.get(*k))
        .map(|k| (k.clone(), parent.get(k).copied(), mine.get(k).copied()))
        .collect())
}

fn print_stat(repo: &Repository, from: &Oid, to: &Oid, summary: bool) -> CliResult<()> {
    use std::io::Write;
    std::io::stdout().flush()?;
    crate::cmd::cmd_diff(&["--stat".to_string(), from.to_hex(), to.to_hex()])?;
    if summary {
        // `--summary`: creation and deletion lines with their modes.
        let modes = |c: &Oid| -> CliResult<HashMap<String, (Oid, String)>> {
            let tree = read_commit(repo, c)?.tree;
            let mut out = HashMap::new();
            let mut stack = vec![(String::new(), tree)];
            while let Some((prefix, t)) = stack.pop() {
                let obj = repo.get_object(&t)?;
                for e in arks_object::Tree::decode(&obj.payload, repo.oid_kind())?.entries {
                    let name = format!("{prefix}{}", e.name);
                    if e.mode.is_tree() {
                        stack.push((format!("{name}/"), e.oid));
                    } else {
                        out.insert(name, (e.oid, e.mode.as_str().to_string()));
                    }
                }
            }
            Ok(out)
        };
        let a = modes(from)?;
        let b = modes(to)?;
        let mut keys: Vec<&String> = a.keys().chain(b.keys()).collect();
        keys.sort();
        keys.dedup();
        for k in keys {
            match (a.get(k), b.get(k)) {
                (None, Some((_, m))) => println!(" create mode {m} {k}"),
                (Some((_, m)), None) => println!(" delete mode {m} {k}"),
                (Some((_, x)), Some((_, y))) if x != y => {
                    println!(" mode change {x} => {y} {k}")
                }
                _ => {}
            }
        }
    }
    Ok(())
}

fn write_opts(dir: &Path, o: &Opts) -> CliResult<()> {
    for (name, on) in OPT_FLAGS.iter().zip(o.flags()) {
        if on {
            fs::write(dir.join(format!("opt-{name}")), "")?;
        }
    }
    if o.apply {
        fs::write(dir.join("apply"), "")?;
    }
    if o.reapply_cherry_picks {
        fs::write(dir.join("opt-reapply-cherry-picks"), "")?;
    }
    if let Some(e) = &o.empty {
        fs::write(dir.join("opt-empty"), e)?;
    }
    if let Some(s) = &o.strategy {
        fs::write(dir.join("strategy"), s)?;
    }
    let favor = match o.favor {
        Favor::Ours => "ours",
        Favor::Theirs => "theirs",
        _ => "",
    };
    if !favor.is_empty() {
        fs::write(dir.join("strategy_opts"), favor)?;
    }
    if !o.sign_args.is_empty() {
        fs::write(dir.join("gpg_sign_opt"), o.sign_args.join("\n"))?;
    }
    Ok(())
}

fn read_opts(dir: &Path) -> Opts {
    let has = |n: &str| dir.join(n).exists();
    Opts {
        quiet: has("opt-quiet"),
        verbose: has("opt-verbose"),
        stat: has("opt-stat"),
        signoff: has("opt-signoff"),
        force: has("opt-force"),
        committer_date_is_author_date: has("opt-committer-date-is-author-date"),
        reset_author_date: has("opt-reset-author-date"),
        apply: has("apply"),
        interactive: has("opt-interactive"),
        empty: fs::read_to_string(dir.join("opt-empty")).ok(),
        keep_empty: has("opt-keep-empty"),
        allow_empty_message: has("opt-allow-empty-message"),
        update_refs: has("opt-update-refs"),
        reapply_cherry_picks: has("opt-reapply-cherry-picks"),
        autostash: has("autostash"),
        favor: match fs::read_to_string(dir.join("strategy_opts"))
            .ok()
            .as_deref()
        {
            Some("ours") => Favor::Ours,
            Some("theirs") => Favor::Theirs,
            _ => Favor::None,
        },
        strategy: fs::read_to_string(dir.join("strategy")).ok(),
        sign_args: fs::read_to_string(dir.join("gpg_sign_opt"))
            .map(|s| s.lines().map(String::from).collect())
            .unwrap_or_default(),
        exec: Vec::new(),
    }
}

fn sequence_editor() -> Option<String> {
    [
        "ARKS_SEQUENCE_EDITOR",
        "GIT_SEQUENCE_EDITOR",
        "ARKS_EDITOR",
        "GIT_EDITOR",
        "EDITOR",
    ]
    .iter()
    .find_map(|v| std::env::var(v).ok().filter(|s| !s.is_empty()))
}

fn run_editor(editor: &str, path: &Path) -> CliResult<()> {
    let ok = std::process::Command::new("sh")
        .arg("-c")
        .arg(format!("{editor} \"$@\""))
        .arg(editor)
        .arg(path)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if ok {
        Ok(())
    } else {
        Err(CliError::Exit(
            1,
            "error: there was a problem with the editor".into(),
        ))
    }
}

fn read_commit(repo: &Repository, oid: &Oid) -> CliResult<Commit> {
    crate::cmd::log_format::read_commit(repo, oid)
}

fn tree_map(repo: &Repository, tree: &Oid) -> CliResult<HashMap<String, Oid>> {
    let mut out = HashMap::new();
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

fn put(repo: &mut Repository, kind: ObjectKind, payload: Vec<u8>) -> CliResult<Oid> {
    let format = repo.object_format();
    Ok(repo
        .store_mut()
        .put(format, &RawObject::new(kind, payload))?)
}

fn read_lines(path: &Path) -> Vec<String> {
    fs::read_to_string(path)
        .map(|t| {
            t.lines()
                .map(str::trim)
                .filter(|l| !l.is_empty() && !l.starts_with('#'))
                .map(String::from)
                .collect()
        })
        .unwrap_or_default()
}

fn append_done(dir: &Path, line: &str) -> CliResult<()> {
    let mut done = fs::read_to_string(dir.join("done")).unwrap_or_default();
    done.push_str(line);
    done.push('\n');
    fs::write(dir.join("done"), done)?;
    Ok(())
}

/// Current HEAD while rebasing (`None` before the first `--root` pick).
fn current_head(repo: &Repository, dir: &Path) -> Option<Oid> {
    if dir.join("root").exists() {
        None
    } else {
        repo.head_oid().ok()
    }
}

fn set_head(
    repo: &mut Repository,
    dir: &Path,
    old: Option<Oid>,
    new: &Oid,
    msg: &str,
) -> CliResult<()> {
    repo.refs_mut().update("HEAD", new, None)?;
    let _ = fs::remove_file(dir.join("root"));
    let _ = crate::reflog::append_reflog(repo, "HEAD", old.as_ref(), new, msg);
    Ok(())
}

/// What stopped the todo run.
enum Stop {
    /// Exit with this error (conflict, failed exec).
    Fail(CliError),
    /// Stopped on purpose (edit, break): exit 0.
    Pause,
}

/// Run the remaining todo lines, then finish.
fn run(dir: &Path) -> CliResult<()> {
    let o = read_opts(dir);
    loop {
        let todo = read_lines(&dir.join("git-rebase-todo"));
        let Some(line) = todo.first().cloned() else {
            break;
        };
        let rest = todo[1..].join("\n");
        fs::write(
            dir.join("git-rebase-todo"),
            if rest.is_empty() {
                String::new()
            } else {
                format!("{rest}\n")
            },
        )?;
        append_done(dir, &line)?;
        let n = read_lines(&dir.join("done")).len();
        fs::write(dir.join("msgnum"), format!("{n}\n"))?;
        let total = fs::read_to_string(dir.join("end"))
            .ok()
            .and_then(|t| t.trim().parse::<usize>().ok())
            .unwrap_or(n);
        if !o.quiet && !o.apply {
            eprint!("Rebasing ({n}/{total})\r");
        }
        match step(dir, &line, &o, &todo[1..]) {
            Ok(()) => {}
            Err(Stop::Pause) => return Ok(()),
            Err(Stop::Fail(e)) => return Err(e),
        }
    }
    finish(dir, &o)
}

fn step(dir: &Path, line: &str, o: &Opts, rest: &[String]) -> Result<(), Stop> {
    let mut words = line.splitn(2, char::is_whitespace);
    let verb = words.next().unwrap_or("");
    let arg = words.next().unwrap_or("").trim();
    let fail = Stop::Fail;
    match verb {
        "pick" | "p" | "reword" | "r" | "edit" | "e" | "squash" | "s" | "fixup" | "f" => {
            let rev = arg.split_whitespace().next().unwrap_or("");
            let rev = if rev == "-C" || rev == "-c" {
                arg.split_whitespace().nth(1).unwrap_or("")
            } else {
                rev
            };
            let repo = open_repo().map_err(fail)?;
            let oid = resolve_rev(&repo, rev)
                .map_err(|_| fail(CliError::Exit(1, format!("error: invalid line: {line}"))))?;
            drop(repo);
            let kind = match verb {
                "reword" | "r" => Kind::Reword,
                "edit" | "e" => Kind::Edit,
                "squash" | "s" => Kind::Squash,
                "fixup" | "f" => Kind::Fixup,
                _ => Kind::Pick,
            };
            pick(dir, &oid, kind, o, rest)
        }
        "exec" | "x" => {
            eprintln!("Executing: {arg}");
            let ok = std::process::Command::new("sh")
                .arg("-c")
                .arg(arg)
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
            if ok {
                Ok(())
            } else {
                Err(fail(CliError::Exit(
                    1,
                    format!(
                        "warning: execution failed: {arg}\nYou can fix the problem, and then run\n\n  git rebase --continue\n"
                    ),
                )))
            }
        }
        "break" | "b" => {
            if let Ok(repo) = open_repo() {
                if let Ok(h) = repo.head_oid() {
                    let subject = read_commit(&repo, &h)
                        .map(|c| crate::cmd::log_format::subject_line(&c.message))
                        .unwrap_or_default();
                    eprintln!("Stopped at {} ({subject})", abbrev_hex(&repo, &h, 7));
                }
            }
            Err(Stop::Pause)
        }
        "drop" | "d" | "noop" => Ok(()),
        "update-ref" | "u" => {
            let mut refs = fs::read_to_string(dir.join("update-refs")).unwrap_or_default();
            let repo = open_repo().map_err(fail)?;
            let head = repo.head_oid().map_err(|e| fail(e.into()))?;
            refs.push_str(&format!("{arg} {}\n", head.to_hex()));
            fs::write(dir.join("update-refs"), refs).map_err(|e| fail(e.into()))?;
            Ok(())
        }
        other => Err(fail(CliError::NotImplemented(format!(
            "rebase todo command '{other}' is not implemented"
        )))),
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Pick,
    Reword,
    Edit,
    Squash,
    Fixup,
}

fn add_signoff_line(msg: &str) -> CliResult<String> {
    let me = committer_signature()?;
    let line = format!("Signed-off-by: {} <{}>", me.name, me.email);
    let body = msg.trim_end_matches('\n');
    if body.lines().last() == Some(line.as_str()) {
        return Ok(format!("{body}\n"));
    }
    Ok(crate::cmd::core::add_signoff(msg))
}

fn pick(dir: &Path, oid: &Oid, kind: Kind, o: &Opts, rest: &[String]) -> Result<(), Stop> {
    let fail = Stop::Fail;
    let mut repo = open_repo().map_err(fail)?;
    let c = read_commit(&repo, oid).map_err(fail)?;
    let head = current_head(&repo, dir);
    let squashing = matches!(kind, Kind::Squash | Kind::Fixup);
    // Fast-forward: the commit already sits on HEAD.
    let parent = c.parents.first().copied();
    let rewrite = o.force
        || o.signoff
        || o.committer_date_is_author_date
        || o.reset_author_date
        || !o.sign_args.is_empty();
    if !rewrite && !squashing && parent == head && kind != Kind::Reword {
        set_head(&mut repo, dir, head, oid, "rebase: fast-forward").map_err(fail)?;
        let work = work_tree(&repo).map_err(fail)?;
        if let Some(h) = head {
            crate::cmd::switch_tree::two_way_switch(&repo, &work, Some(&h), oid).map_err(fail)?;
        } else {
            crate::cmd::working_tree::hard_reset_tree(&repo, &work, oid).map_err(fail)?;
        }
        if o.apply && !o.quiet {
            println!(
                "Applying: {}",
                crate::cmd::log_format::subject_line(&c.message)
            );
        }
        if kind == Kind::Edit {
            return Err(edit_stop(&repo, oid, &c));
        }
        return Ok(());
    }
    if o.apply && !o.quiet {
        println!(
            "Applying: {}",
            crate::cmd::log_format::subject_line(&c.message)
        );
    }
    let empty_tree = put(&mut repo, ObjectKind::Tree, Vec::new()).map_err(fail)?;
    let parent_tree = match parent {
        Some(p) => read_commit(&repo, &p).map_err(fail)?.tree,
        None => empty_tree,
    };
    let head_tree = match head {
        Some(h) => read_commit(&repo, &h).map_err(fail)?.tree,
        None => empty_tree,
    };
    let work = work_tree(&repo).map_err(fail)?;
    let desc = format!(
        "{} ({})",
        abbrev_hex(&repo, oid, 7),
        crate::cmd::log_format::subject_line(&c.message)
    );
    let labels = MergeLabels {
        ours: "HEAD".into(),
        base: format!("parent of {desc}"),
        theirs: desc.clone(),
        diff3: false,
        favor: o.favor,
    };
    let result = if o.strategy.as_deref() == Some("ours") {
        crate::cmd::merge_engine::MergeResult {
            tree: Some(head_tree),
            conflicts: Vec::new(),
            auto_merged: Vec::new(),
        }
    } else {
        merge_trees_worktree(
            &mut repo,
            &work,
            &parent_tree,
            &head_tree,
            &c.tree,
            true,
            &labels,
        )
        .map_err(fail)?
    };
    let git_dir = repo.git_dir().to_path_buf();
    if !result.conflicts.is_empty() {
        if o.apply {
            println!("Using index info to reconstruct a base tree...");
            let base_map = tree_map(&repo, &parent_tree).map_err(fail)?;
            let new_map = tree_map(&repo, &c.tree).map_err(fail)?;
            let mut paths: Vec<&String> = base_map.keys().chain(new_map.keys()).collect();
            paths.sort();
            paths.dedup();
            for p in paths {
                let st = match (base_map.get(p), new_map.get(p)) {
                    (None, Some(_)) => 'A',
                    (Some(_), None) => 'D',
                    (a, b) if a != b => 'M',
                    _ => continue,
                };
                println!("{st}\t{p}");
            }
            println!("Falling back to patching base and 3-way merge...");
        }
        for p in result
            .auto_merged
            .iter()
            .filter(|p| !result.conflicts.contains(p))
        {
            println!("Auto-merging {p}");
        }
        let base_map = tree_map(&repo, &parent_tree).map_err(fail)?;
        let head_map = tree_map(&repo, &head_tree).map_err(fail)?;
        for p in &result.conflicts {
            let kind = match (base_map.contains_key(p), head_map.contains_key(p)) {
                (true, true) => "content",
                (false, true) => "add/add",
                _ => "modify/delete",
            };
            if kind == "modify/delete" {
                println!(
                    "CONFLICT (modify/delete): {p} deleted in HEAD and modified in {desc}.  Version {desc} of {p} left in tree."
                );
            } else {
                println!("Auto-merging {p}");
                println!("CONFLICT ({kind}): Merge conflict in {p}");
            }
        }
        let mut msg = c.message.clone();
        if !msg.ends_with('\n') {
            msg.push('\n');
        }
        msg.push_str("\n# Conflicts:\n");
        for p in &result.conflicts {
            msg.push_str(&format!("#\t{p}\n"));
        }
        let _ = fs::write(git_dir.join("MERGE_MSG"), &msg);
        let _ = fs::write(git_dir.join("REBASE_HEAD"), format!("{}\n", oid.to_hex()));
        let _ = fs::write(dir.join("stopped-sha"), format!("{}\n", oid.to_hex()));
        let _ = fs::write(dir.join("message"), &c.message);
        let _ = fs::write(dir.join("kind"), kind_name(kind));
        let short = abbrev_hex(&repo, oid, 7);
        let subject = crate::cmd::log_format::subject_line(&c.message);
        let text = if o.apply {
            let n = read_lines(&dir.join("done")).len();
            let _ = fs::remove_file(git_dir.join("MERGE_MSG"));
            println!(
                "Patch failed at {n:04} {subject}\nResolve all conflicts manually, mark them as resolved with\n\"git add/rm <conflicted_files>\", then run \"git rebase --continue\".\nYou can instead skip this commit: run \"git rebase --skip\".\nTo abort and get back to the state before \"git rebase\", run \"git rebase --abort\"."
            );
            "error: Failed to merge in the changes.\nhint: Use 'git am --show-current-patch=diff' to see the failed patch".to_string()
        } else {
            format!(
                "error: could not apply {short}... {subject}\nhint: Resolve all conflicts manually, mark them as resolved with\nhint: \"git add/rm <conflicted_files>\", then run \"git rebase --continue\".\nhint: You can instead skip this commit: run \"git rebase --skip\".\nhint: To abort and get back to the state before \"git rebase\", run \"git rebase --abort\".\nCould not apply {short}... {subject}"
            )
        };
        return Err(fail(CliError::Exit(1, text)));
    }
    let tree = result
        .tree
        .ok_or_else(|| fail(CliError::Msg("merge: missing result tree".into())))?;
    let _ = rest;
    commit_step(&mut repo, dir, oid, &c, tree, head, kind, o)
}

fn kind_name(k: Kind) -> &'static str {
    match k {
        Kind::Pick => "pick",
        Kind::Reword => "reword",
        Kind::Edit => "edit",
        Kind::Squash => "squash",
        Kind::Fixup => "fixup",
    }
}

fn edit_stop(repo: &Repository, oid: &Oid, c: &Commit) -> Stop {
    let _ = fs::write(
        repo.git_dir().join("REBASE_HEAD"),
        format!("{}\n", oid.to_hex()),
    );
    eprintln!(
        "Stopped at {}...  {}\nYou can amend the commit now, with\n\n  git commit --amend \n\nOnce you are satisfied with your changes, run\n\n  git rebase --continue",
        abbrev_hex(repo, oid, 7),
        crate::cmd::log_format::subject_line(&c.message)
    );
    Stop::Pause
}

#[allow(clippy::too_many_arguments)]
fn commit_step(
    repo: &mut Repository,
    dir: &Path,
    oid: &Oid,
    c: &Commit,
    tree: Oid,
    head: Option<Oid>,
    kind: Kind,
    o: &Opts,
) -> Result<(), Stop> {
    let fail = Stop::Fail;
    let squashing = matches!(kind, Kind::Squash | Kind::Fixup);
    let head_commit = match head {
        Some(h) => Some(read_commit(repo, &h).map_err(fail)?),
        None => None,
    };
    // Becoming empty: drop (default), keep, or stop.
    let head_tree = head_commit.as_ref().map(|h| h.tree);
    let parent_tree = match c.parents.first() {
        Some(p) => Some(read_commit(repo, p).map_err(fail)?.tree),
        None => None,
    };
    let originally_empty = parent_tree == Some(c.tree);
    if !squashing && Some(tree) == head_tree && !originally_empty {
        match o.empty.as_deref() {
            Some("keep") => {}
            Some("ask" | "stop") if !o.apply => {
                let _ = fs::write(
                    repo.git_dir().join("REBASE_HEAD"),
                    format!("{}\n", oid.to_hex()),
                );
                return Err(fail(CliError::Exit(
                    1,
                    "The previous cherry-pick is now empty, possibly due to conflict resolution.\nIf you wish to commit it anyway, use:\n\n    git commit --allow-empty\n\nOtherwise, please use 'git rebase --skip'".into(),
                )));
            }
            _ => return Ok(()),
        }
    }
    let mut message = c.message.clone();
    if !message.ends_with('\n') {
        message.push('\n');
    }
    let (parents, base_msg) = if squashing {
        let h = head_commit
            .clone()
            .ok_or_else(|| fail(CliError::fatal("cannot 'squash' without a previous commit")))?;
        (h.parents.clone(), Some(h.message.clone()))
    } else {
        (head.into_iter().collect(), None)
    };
    if let Some(prev) = base_msg {
        message = match kind {
            Kind::Fixup => prev,
            _ => {
                let mut m = prev.trim_end_matches('\n').to_string();
                m.push_str("\n\n");
                m.push_str(message.trim_end_matches('\n'));
                m.push('\n');
                m
            }
        };
    }
    if o.signoff {
        message = add_signoff_line(&message).map_err(fail)?;
    }
    if matches!(kind, Kind::Reword) || (kind == Kind::Squash && o.interactive) {
        if let Some(ed) = sequence_editor_plain() {
            let path = repo.git_dir().join("COMMIT_EDITMSG");
            fs::write(&path, &message).map_err(|e| fail(e.into()))?;
            run_editor(&ed, &path).map_err(fail)?;
            let edited = fs::read_to_string(&path).map_err(|e| fail(e.into()))?;
            message = crate::cmd::trailers::cleanup(&edited, Cleanup::Strip, true);
        }
    }
    // A squash or fixup keeps the authorship of the commit it melds into.
    let mut author = match (&head_commit, squashing) {
        (Some(h), true) => h.author.clone(),
        _ => c.author.clone(),
    };
    let mut committer = committer_signature().map_err(fail)?;
    if o.reset_author_date {
        // git takes the current time here, not GIT_AUTHOR_DATE.
        author.timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(committer.timestamp);
        author.timezone = committer.timezone.clone();
    }
    if o.committer_date_is_author_date {
        committer.timestamp = author.timestamp;
        committer.timezone = author.timezone.clone();
    }
    let mut commit = Commit {
        tree,
        parents,
        author,
        committer,
        gpgsig: None,
        message,
    };
    let (do_sign, key) = crate::cmd::sign::want_sign_commit(&o.sign_args, repo).map_err(fail)?;
    if do_sign {
        if let Err(e) = crate::cmd::sign::sign_commit(repo, &mut commit, key.as_deref()) {
            let g = repo.git_dir().to_path_buf();
            let _ = fs::write(g.join("MERGE_MSG"), &commit.message);
            let _ = fs::write(g.join("CHERRY_PICK_HEAD"), format!("{}\n", oid.to_hex()));
            let _ = fs::write(g.join("REBASE_HEAD"), format!("{}\n", oid.to_hex()));
            let _ = fs::write(dir.join("stopped-sha"), format!("{}\n", oid.to_hex()));
            return Err(fail(CliError::Exit(
                1,
                format!(
                    "{e}\nerror: failed to write commit object\nerror: could not commit staged changes."
                ),
            )));
        }
    }
    let payload = commit.encode().map_err(|e| fail(e.into()))?;
    let new = put(repo, ObjectKind::Commit, payload).map_err(fail)?;
    let subject = crate::cmd::log_format::subject_line(&commit.message);
    let verb = match kind {
        Kind::Pick | Kind::Edit => "pick",
        Kind::Reword => "reword",
        Kind::Squash => "squash",
        Kind::Fixup => "fixup",
    };
    set_head(
        repo,
        dir,
        head,
        &new,
        &format!("rebase ({verb}): {subject}"),
    )
    .map_err(fail)?;
    let _ = fs::remove_file(repo.git_dir().join("MERGE_MSG"));
    if dir.join("resumed").exists() {
        // Committing a resolved stop goes through `git commit`, which reports it.
        let _ = fs::remove_file(dir.join("resumed"));
        if !o.quiet {
            crate::cmd::core::print_commit_summary_dated(repo, &new, Some(false)).map_err(fail)?;
        }
    }
    if matches!(kind, Kind::Squash | Kind::Reword) && !o.quiet {
        // The melded message goes through `git commit --amend`, which reports it.
        crate::cmd::core::print_commit_summary_dated(repo, &new, Some(true)).map_err(fail)?;
    }
    if kind == Kind::Edit {
        return Err(edit_stop(repo, oid, c));
    }
    Ok(())
}

fn sequence_editor_plain() -> Option<String> {
    ["ARKS_EDITOR", "GIT_EDITOR", "EDITOR"]
        .iter()
        .find_map(|v| std::env::var(v).ok().filter(|s| !s.is_empty()))
}

/// Move the rebased branch to HEAD, re-attach HEAD and clean up.
fn finish(dir: &Path, o: &Opts) -> CliResult<()> {
    let mut repo = open_repo()?;
    let head_name = fs::read_to_string(dir.join("head-name"))?
        .trim()
        .to_string();
    let onto = fs::read_to_string(dir.join("onto"))?.trim().to_string();
    let orig = resolve_rev(&repo, fs::read_to_string(dir.join("orig-head"))?.trim())?;
    let head = match current_head(&repo, dir) {
        Some(h) => h,
        None => orig,
    };
    if head_name.starts_with("refs/") {
        let old = repo.refs().resolve(&head_name).ok();
        repo.refs_mut().update(&head_name, &head, old.as_ref())?;
        let _ = crate::reflog::append_reflog(
            &repo,
            &head_name,
            old.as_ref(),
            &head,
            &format!("rebase (finish): {head_name} onto {onto}"),
        );
        repo.refs_mut().set_symbolic("HEAD", &head_name)?;
        let _ = crate::reflog::append_reflog(
            &repo,
            "HEAD",
            Some(&head),
            &head,
            &format!("rebase (finish): returning to {head_name}"),
        );
    }
    if let Ok(text) = fs::read_to_string(dir.join("update-refs")) {
        for line in text.lines() {
            if let Some((name, hex)) = line.split_once(' ') {
                if let Ok(new) = Oid::from_hex(hex) {
                    let old = repo.refs().resolve(name).ok();
                    repo.refs_mut().update(name, &new, old.as_ref())?;
                }
            }
        }
    }
    let autostash = dir.join("autostash").exists();
    fs::remove_dir_all(dir)?;
    let _ = fs::write(
        repo.git_dir().join("ORIG_HEAD"),
        format!("{}\n", orig.to_hex()),
    );
    drop(repo);
    if autostash {
        crate::cmd::stash::autostash_pop()?;
    }
    if !o.quiet {
        if head_name.starts_with("refs/") {
            eprintln!("Successfully rebased and updated {head_name}.");
        } else {
            eprintln!("Successfully rebased and updated detached HEAD.");
        }
    }
    Ok(())
}

/// `--continue`: commit the resolved stop (if any), then run the rest.
fn resume(dir: &Path, _commit_needed: bool) -> CliResult<()> {
    let o = read_opts(dir);
    let mut repo = open_repo()?;
    let index = read_index(&repo)?;
    if index.entries.iter().any(|e| e.stage() != 0) {
        return Err(CliError::Exit(
            1,
            "error: Committing is not possible because you have unmerged files.\nhint: Fix them up in the work tree, and then use 'git add/rm <file>'\nhint: as appropriate to mark resolution and make a commit.\nfatal: Exiting because of an unresolved conflict.".into(),
        ));
    }
    if let Ok(hex) = fs::read_to_string(dir.join("stopped-sha")) {
        let oid = resolve_rev(&repo, hex.trim())?;
        let c = read_commit(&repo, &oid)?;
        let format = repo.object_format();
        let tree =
            crate::write_tree::write_tree_from_index(repo.store_mut(), &index.entries, format)?;
        let head = current_head(&repo, dir);
        let kind = match fs::read_to_string(dir.join("kind")).ok().as_deref() {
            Some("squash") => Kind::Squash,
            Some("fixup") => Kind::Fixup,
            Some("reword") => Kind::Reword,
            _ => Kind::Pick,
        };
        let _ = fs::remove_file(dir.join("stopped-sha"));
        fs::write(dir.join("resumed"), "")?;
        match commit_step(&mut repo, dir, &oid, &c, tree, head, kind, &o) {
            Ok(()) | Err(Stop::Pause) => {}
            Err(Stop::Fail(e)) => return Err(e),
        }
    }
    drop(repo);
    run(dir)
}

fn skip(dir: &Path) -> CliResult<()> {
    let repo = open_repo()?;
    let work = work_tree(&repo)?;
    if let Some(h) = current_head(&repo, dir) {
        crate::cmd::working_tree::hard_reset_tree(&repo, &work, &h)?;
    }
    let _ = fs::remove_file(repo.git_dir().join("MERGE_MSG"));
    let _ = fs::remove_file(dir.join("stopped-sha"));
    drop(repo);
    run(dir)
}

fn abort(dir: &Path) -> CliResult<()> {
    let mut repo = open_repo()?;
    let head_name = fs::read_to_string(dir.join("head-name"))?
        .trim()
        .to_string();
    let orig = resolve_rev(&repo, fs::read_to_string(dir.join("orig-head"))?.trim())?;
    let work = work_tree(&repo)?;
    crate::cmd::working_tree::hard_reset_tree(&repo, &work, &orig)?;
    let head = repo.head_oid().ok();
    if head_name.starts_with("refs/") {
        repo.refs_mut().set_symbolic("HEAD", &head_name)?;
        let _ = crate::reflog::append_reflog(
            &repo,
            "HEAD",
            head.as_ref(),
            &orig,
            &format!("rebase (abort): returning to {head_name}"),
        );
    } else {
        repo.refs_mut().update("HEAD", &orig, None)?;
    }
    let autostash = dir.join("autostash").exists();
    fs::remove_dir_all(dir)?;
    for f in ["MERGE_MSG", "REBASE_HEAD"] {
        let _ = fs::remove_file(repo.git_dir().join(f));
    }
    drop(repo);
    if autostash {
        crate::cmd::stash::autostash_pop()?;
    }
    Ok(())
}

fn edit_todo(dir: &Path) -> CliResult<()> {
    if !dir.join("interactive").exists() && !dir.join("opt-interactive").exists() {
        return Err(CliError::fatal(
            "The --edit-todo action can only be used during interactive rebase.",
        ));
    }
    if let Some(ed) = sequence_editor() {
        run_editor(&ed, &dir.join("git-rebase-todo"))?;
    }
    Ok(())
}
