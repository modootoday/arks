//! Merge (M15 Fs-1/Fs-2); `pull` lives in `cmd::pull`.
//!
//! Cite: <https://git-scm.com/docs/git-merge>
//! Fs-1: FF + `--no-ff` when FF-capable.
//! Fs-2: diverged three-way merge (clean or conflicted index).

use crate::cmd::shared::{commit_signature, current_branch_name, is_ancestor, resolve_rev};
use crate::error::{CliError, CliResult};
use crate::util::{open_repo, work_tree};
use arks_object::{Commit, ObjectKind, RawObject};
use arks_refs::RefTarget;
use arks_store::ObjectStore;

/// Fast-forward policy.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Ff {
    Allow,
    Never,
    Only,
}

/// Parsed `git merge` options.
#[derive(Clone)]
struct MergeOpts {
    ff: Ff,
    commit: bool,
    squash: bool,
    stat: bool,
    log: Option<usize>,
    message: Option<String>,
    edit: bool,
    cleanup: crate::cmd::trailers::Cleanup,
    strategy: Option<String>,
    into_name: Option<String>,
    quiet: bool,
    allow_unrelated: bool,
    sign_args: Vec<String>,
    autostash: bool,
    signoff: bool,
    no_verify: bool,
    verify_signatures: bool,
    favor: crate::cmd::merge_engine::Favor,
}

const STRATEGIES: &[&str] = &["octopus", "ours", "recursive", "resolve", "subtree", "ort"];

pub fn cmd_merge(args: &[String]) -> CliResult<()> {
    // Every option of `git merge -h`. Cite: https://git-scm.com/docs/git-merge
    use crate::cmd::opt_parse::{Item, Mode, parse};
    let p = parse(
        "merge",
        args,
        crate::cmd::porcelain_opts::MERGE,
        Mode::default(),
    )?;
    for op in ["abort", "quit", "continue"] {
        if p.on(op) {
            return merge_sequencer_op(op);
        }
    }
    let mut ff = Ff::Allow;
    for it in &p.items {
        if let Item::Opt { key, neg, .. } = it {
            match (*key, *neg) {
                ("ff", false) => ff = Ff::Allow,
                ("ff", true) => ff = Ff::Never,
                ("ff-only", _) => ff = Ff::Only,
                _ => {}
            }
        }
    }
    let mut stat = true;
    for it in &p.items {
        if let Item::Opt { key, neg, .. } = it {
            match (*key, *neg) {
                ("stat" | "summary", n) => stat = !n,
                ("-n", _) => stat = false,
                _ => {}
            }
        }
    }
    let messages = p.values("message");
    let mut message = (!messages.is_empty()).then(|| messages.join("\n\n"));
    if let Some(f) = p.value("file") {
        let text = if f == "-" {
            let mut s = String::new();
            std::io::Read::read_to_string(&mut std::io::stdin(), &mut s)?;
            s
        } else {
            std::fs::read_to_string(&f)
                .map_err(|e| CliError::fatal(format!("could not read file '{f}': {e}")))?
        };
        message = Some(text);
    }
    let strategies = p.values("strategy");
    for s in &strategies {
        if !STRATEGIES.contains(&s.as_str()) {
            return Err(CliError::Exit(
                1,
                format!(
                    "Could not find merge strategy '{s}'.\nAvailable strategies are: octopus ours recursive resolve subtree."
                ),
            ));
        }
    }
    // `-X ours|theirs` resolve conflicting hunks; the other ort options
    // (diff algorithms, whitespace, renames) give the same result for the
    // line merges this engine performs.
    let mut favor = crate::cmd::merge_engine::Favor::None;
    for x in p.values("strategy-option") {
        match x.as_str() {
            "ours" => favor = crate::cmd::merge_engine::Favor::Ours,
            "theirs" => favor = crate::cmd::merge_engine::Favor::Theirs,
            _ => {}
        }
    }
    let squash = p.on("squash");
    let commit = p.flag("commit") != Some(false);
    if squash && ff == Ff::Never {
        return Err(CliError::fatal(
            "options '--squash' and '--no-ff.' cannot be used together",
        ));
    }
    if squash && p.flag("commit") == Some(true) {
        return Err(CliError::fatal(
            "options '--squash' and '--commit.' cannot be used together",
        ));
    }
    let cleanup = match p.value("cleanup") {
        Some(v) => crate::cmd::trailers::parse_cleanup(&v)?,
        None => crate::cmd::trailers::Cleanup::Default,
    };
    let log = match p.last("log") {
        None => None,
        Some(None) => Some(20),
        Some(Some(n)) => Some(
            n.parse::<usize>()
                .map_err(|_| CliError::fatal(format!("invalid --log value: {n}")))?,
        ),
    };
    let o = MergeOpts {
        ff,
        commit,
        squash,
        stat,
        log,
        message,
        edit: p.on("edit"),
        cleanup,
        strategy: strategies.last().cloned(),
        into_name: p.value("into-name"),
        quiet: p.on("quiet"),
        allow_unrelated: p.on("allow-unrelated-histories"),
        sign_args: crate::cmd::commit::sign_args(&p),
        autostash: p.on("autostash"),
        signoff: p.on("signoff"),
        no_verify: p.on("no-verify"),
        verify_signatures: p.on("verify-signatures"),
        favor,
    };
    let repo = open_repo()?;
    // SEQ-004 (M20 Cx-6): rerere rr-cache lock during merge.
    let rr_lock = repo.common_dir().join("rr-cache.lock");
    if rr_lock.is_file() {
        return Err(CliError::Msg(format!(
            "{} (SEQ-004 — concurrent rerere rr-cache writes refuse; serialize conflict resolution)",
            rr_lock.display()
        )));
    }
    let positional = p.args();
    if positional.is_empty() {
        return Err(CliError::fatal("No remote for the current branch."));
    }
    if positional.len() > 1 {
        return Err(CliError::NotImplemented(
            "merge of several heads (octopus) not implemented".into(),
        ));
    }
    drop(repo);
    merge_one(&positional[0], &o)
}

/// `--abort`, `--quit` and `--continue` of an in-progress merge.
fn merge_sequencer_op(op: &str) -> CliResult<()> {
    // git-merge --abort/--continue need a merge in progress (MERGE_HEAD).
    // Cite: https://git-scm.com/docs/git-merge#Documentation/git-merge.txt---abort
    let repo = open_repo()?;
    let git_dir = repo.git_dir().to_path_buf();
    if !git_dir.join("MERGE_HEAD").is_file() {
        return match op {
            "abort" => Err(CliError::fatal(
                "There is no merge to abort (MERGE_HEAD missing).",
            )),
            "continue" => Err(CliError::fatal(
                "There is no merge in progress (MERGE_HEAD missing).",
            )),
            _ => Ok(()),
        };
    }
    let clear = || {
        for f in ["MERGE_HEAD", "MERGE_MSG", "MERGE_MODE", "AUTO_MERGE"] {
            let _ = std::fs::remove_file(git_dir.join(f));
        }
    };
    match op {
        "quit" => {
            clear();
            Ok(())
        }
        "abort" => {
            let head = repo.head_oid()?;
            let work = work_tree(&repo)?;
            crate::cmd::working_tree::hard_reset_tree(&repo, &work, &head)?;
            clear();
            Ok(())
        }
        _ => {
            drop(repo);
            crate::cmd::commit::cmd_commit(&[])
        }
    }
}

/// Stash local changes, run `f`, then re-apply them (`--autostash`).
fn with_autostash(f: impl FnOnce() -> CliResult<()>) -> CliResult<()> {
    let stashed = crate::cmd::stash::autostash_push()?;
    let result = f();
    if stashed {
        crate::cmd::stash::autostash_pop()?;
    }
    result
}

/// What the merged-in name is, for the default message.
fn merge_label(repo: &arks_repo::Repository, name: &str) -> String {
    // `<branch>~<n>` / `<branch>^`: "branch '<branch>' (early part)".
    if let Some(cut) = name.find(['~', '^']) {
        let (b, rest) = name.split_at(cut);
        let suffix_ok = rest
            .chars()
            .all(|c| c == '~' || c == '^' || c.is_ascii_digit());
        if suffix_ok && !b.is_empty() && repo.refs().resolve(&format!("refs/heads/{b}")).is_ok() {
            return format!("branch '{b}' (early part)");
        }
    }
    if name.starts_with("refs/heads/") || repo.refs().resolve(&format!("refs/heads/{name}")).is_ok()
    {
        format!("branch '{}'", name.trim_start_matches("refs/heads/"))
    } else if repo.refs().resolve(&format!("refs/remotes/{name}")).is_ok() {
        format!("remote-tracking branch '{name}'")
    } else if repo.refs().resolve(&format!("refs/tags/{name}")).is_ok() {
        format!("tag '{name}'")
    } else {
        format!("commit '{name}'")
    }
}

/// The default merge message (`fmt-merge-msg`): `Merge <label>`, plus
/// ` into <branch>` unless the branch is main/master, plus `--log` lines.
fn default_merge_message(
    repo: &arks_repo::Repository,
    name: &str,
    head: &arks_oid::Oid,
    theirs: &arks_oid::Oid,
    o: &MergeOpts,
) -> CliResult<String> {
    let mut msg = format!("Merge {}", merge_label(repo, name));
    let target = o
        .into_name
        .clone()
        .or_else(|| current_branch_name(repo).ok());
    if let Some(t) = target {
        if o.into_name.is_some() || !matches!(t.as_str(), "main" | "master") {
            msg.push_str(&format!(" into {t}"));
        }
    }
    msg.push('\n');
    if let Some(limit) = o.log {
        let ours = arks_repo::ancestor_set(repo, *head)?;
        let mut queue = vec![*theirs];
        let mut seen = std::collections::HashSet::new();
        let mut picked: Vec<(i64, String)> = Vec::new();
        while let Some(c) = queue.pop() {
            if ours.contains(&c) || !seen.insert(c) {
                continue;
            }
            let commit = crate::cmd::log_format::read_commit(repo, &c)?;
            picked.push((
                commit.committer.timestamp,
                crate::cmd::log_format::subject_line(&commit.message),
            ));
            queue.extend(commit.parents.iter().copied());
        }
        picked.sort_by_key(|x| std::cmp::Reverse(x.0));
        if !picked.is_empty() {
            let short = name.trim_start_matches("refs/heads/");
            if picked.len() > limit {
                msg.push_str(&format!("\n* {short}: ({} commits)\n", picked.len()));
            } else {
                msg.push_str(&format!("\n* {short}:\n"));
            }
            for (_, s) in picked.iter().take(limit) {
                msg.push_str(&format!("  {s}\n"));
            }
            if picked.len() > limit {
                msg.push_str("  ...\n");
            }
        }
    }
    Ok(msg)
}

fn merge_one(name: &str, o: &MergeOpts) -> CliResult<()> {
    let mut repo = open_repo()?;
    let say = |s: String| {
        if !o.quiet {
            println!("{s}");
        }
    };
    let theirs = crate::cmd::shared::peel_to_kind(
        &repo,
        &resolve_rev(&repo, name)
            .map_err(|_| CliError::fatal(format!("{name} - not something we can merge")))?,
        crate::cmd::shared::ObjKind::Commit,
        name,
    )?;
    let head = repo.head_oid()?;
    let up_to_date = head == theirs || is_ancestor(&repo, &theirs, &head)?;
    if o.verify_signatures && !up_to_date {
        let c = crate::cmd::log_format::read_commit(&repo, &theirs)?;
        if c.gpgsig.is_none() {
            return Err(CliError::fatal(format!(
                "Commit {} does not have a GPG signature.",
                crate::cmd::shared::abbrev_hex(&repo, &theirs, 7)
            )));
        }
    }
    let _ = repo.refs_mut().update("ORIG_HEAD", &head, None);
    if up_to_date {
        say(if o.squash {
            "Already up to date. (nothing to squash)".to_string()
        } else {
            "Already up to date.".to_string()
        });
        return Ok(());
    }
    if arks_repo::merge_base_oid(&repo, &head, &theirs)?.is_none() && !o.allow_unrelated {
        return Err(CliError::fatal("refusing to merge unrelated histories"));
    }
    if o.autostash {
        // Stash only once the merge is really going to happen.
        drop(repo);
        let mut plain = o.clone();
        plain.autostash = false;
        return with_autostash(|| merge_one(name, &plain));
    }
    let can_ff = is_ancestor(&repo, &head, &theirs)?;
    if can_ff && o.ff != Ff::Never && o.strategy.as_deref() != Some("ours") {
        // Fast-forward keeps non-conflicting local changes (two-way switch).
        let work = work_tree(&repo)?;
        crate::cmd::switch_tree::two_way_switch(&repo, &work, Some(&head), &theirs)
            .map_err(|e| CliError::Msg(e.to_string().replace("checkout", "merge")))?;
        say(format!(
            "Updating {}..{}",
            crate::cmd::shared::abbrev_hex(&repo, &head, 7),
            crate::cmd::shared::abbrev_hex(&repo, &theirs, 7)
        ));
        say("Fast-forward".to_string());
        if o.squash {
            say("Squash commit -- not updating HEAD".to_string());
            write_squash_msg(&repo, &head, &theirs)?;
        } else {
            match repo.head_target()? {
                RefTarget::Symbolic(branch) => {
                    repo.refs_mut().update(&branch, &theirs, Some(&head))?;
                    let _ = crate::reflog::append_head_and_branch(
                        &repo,
                        &branch,
                        Some(&head),
                        &theirs,
                        &format!("merge {name}: Fast-forward"),
                    );
                }
                RefTarget::Direct(_) => {
                    repo.refs_mut().update("HEAD", &theirs, Some(&head))?;
                }
            }
        }
        if o.stat && !o.quiet {
            print_merge_diffstat(&repo, &head, &theirs)?;
        }
        return Ok(());
    }
    if o.ff == Ff::Only {
        return Err(CliError::fatal("Not possible to fast-forward, aborting."));
    }
    refuse_dirty_tracked(&repo, &touched_paths(&repo, &head, &theirs)?)?;
    let strategy = o.strategy.clone().unwrap_or_else(|| "ort".to_string());
    let work = work_tree(&repo)?;
    let mut trivial = false;
    let tree = if strategy == "ours" {
        crate::cmd::log_format::read_commit(&repo, &head)?.tree
    } else if can_ff {
        // `--no-ff` of a fast-forwardable branch: the result is their tree.
        crate::cmd::switch_tree::two_way_switch(&repo, &work, Some(&head), &theirs)?;
        crate::cmd::log_format::read_commit(&repo, &theirs)?.tree
    } else {
        let style = arks_config::ConfigSet::load(Some(repo.common_dir()))
            .get("merge.conflictStyle")
            .unwrap_or_default();
        let labels = crate::cmd::merge_engine::MergeLabels {
            ours: "HEAD".into(),
            base: "merged common ancestors".into(),
            theirs: name.to_string(),
            diff3: matches!(style.as_str(), "diff3" | "zdiff3"),
            favor: o.favor,
        };
        let resolve = strategy == "resolve";
        if resolve {
            say("Trying really trivial in-index merge...".to_string());
        }
        let result = crate::cmd::merge_engine::merge_three_way_labeled(
            &mut repo, &work, &head, &theirs, true, &labels,
        )?;
        if resolve {
            if result.auto_merged.is_empty() && result.conflicts.is_empty() {
                say("Wonderful.".to_string());
                trivial = true;
            } else {
                say("Nope.".to_string());
                say("Trying simple merge.".to_string());
                say("Simple merge failed, trying Automatic merge.".to_string());
                eprintln!("error: Merge requires file-level merging");
            }
        }
        for p in result
            .auto_merged
            .iter()
            .filter(|p| !result.conflicts.contains(p))
        {
            say(format!("Auto-merging {p}"));
        }
        if !result.conflicts.is_empty() {
            let mut msg = default_merge_message(&repo, name, &head, &theirs, o)?;
            msg.push_str("\n# Conflicts:\n");
            for c in &result.conflicts {
                msg.push_str(&format!("#\t{c}\n"));
            }
            if o.squash {
                write_squash_msg(&repo, &head, &theirs)?;
                let conflicts_only = msg[msg.find("\n# Conflicts:").unwrap_or(0)..].to_string();
                std::fs::write(repo.git_dir().join("MERGE_MSG"), conflicts_only)?;
            } else {
                std::fs::write(
                    repo.git_dir().join("MERGE_HEAD"),
                    format!("{}\n", theirs.to_hex()),
                )?;
                std::fs::write(repo.git_dir().join("MERGE_MSG"), &msg)?;
                std::fs::write(
                    repo.git_dir().join("MERGE_MODE"),
                    if o.ff == Ff::Never { "no-ff" } else { "" },
                )?;
            }
            for c in &result.conflicts {
                if result.auto_merged.contains(c) {
                    say(format!("Auto-merging {c}"));
                    if resolve {
                        eprintln!("ERROR: content conflict in {c}");
                    } else {
                        say(format!("CONFLICT (content): Merge conflict in {c}"));
                    }
                } else {
                    say(format!(
                        "CONFLICT (modify/delete): {c} deleted in one side and modified in the other."
                    ));
                }
            }
            if resolve {
                eprintln!("fatal: merge program failed");
            }
            if o.squash {
                say("Squash commit -- not updating HEAD".to_string());
            }
            say("Automatic merge failed; fix conflicts and then commit the result.".to_string());
            return Err(CliError::silent(1));
        }
        result
            .tree
            .ok_or_else(|| CliError::Msg("merge: missing result tree".into()))?
    };
    if o.squash {
        write_squash_msg(&repo, &head, &theirs)?;
        say("Squash commit -- not updating HEAD".to_string());
        if !o.quiet {
            eprintln!("Automatic merge went well; stopped before committing as requested");
        }
        return Ok(());
    }
    let mut msg = match &o.message {
        Some(m) => {
            let mut m = m.clone();
            if !m.ends_with('\n') && o.cleanup != crate::cmd::trailers::Cleanup::Verbatim {
                m.push('\n');
            }
            m
        }
        None => default_merge_message(&repo, name, &head, &theirs, o)?,
    };
    if !o.commit {
        std::fs::write(
            repo.git_dir().join("MERGE_HEAD"),
            format!("{}\n", theirs.to_hex()),
        )?;
        std::fs::write(repo.git_dir().join("MERGE_MSG"), &msg)?;
        std::fs::write(
            repo.git_dir().join("MERGE_MODE"),
            if o.ff == Ff::Never { "no-ff" } else { "" },
        )?;
        if !o.quiet {
            eprintln!("Automatic merge went well; stopped before committing as requested");
        }
        return Ok(());
    }
    if o.signoff {
        msg = crate::cmd::core::add_signoff(&msg);
    }
    if o.edit {
        let path = repo.git_dir().join("MERGE_MSG");
        std::fs::write(&path, &msg)?;
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
                return Err(CliError::Exit(1, "error: editor failed".into()));
            }
        }
        msg = std::fs::read_to_string(&path)?;
        let _ = std::fs::remove_file(&path);
    }
    let msg = crate::cmd::trailers::cleanup(&msg, o.cleanup, o.edit);
    crate::cmd::core::run_claimed_hooks(&repo, o.no_verify, &["commit-msg"], Some(msg.as_bytes()))?;
    let author = commit_signature()?;
    let committer = crate::cmd::shared::committer_signature()?;
    let mut commit = Commit {
        tree,
        parents: vec![head, theirs],
        author,
        committer,
        gpgsig: None,
        message: msg,
    };
    let (do_sign, key) = crate::cmd::sign::want_sign_commit(&o.sign_args, &repo)?;
    if do_sign {
        if let Err(e) = crate::cmd::sign::sign_commit(&repo, &mut commit, key.as_deref()) {
            // git leaves the merge in progress when the commit cannot be signed.
            std::fs::write(
                repo.git_dir().join("MERGE_HEAD"),
                format!("{}\n", theirs.to_hex()),
            )?;
            std::fs::write(repo.git_dir().join("MERGE_MSG"), &commit.message)?;
            std::fs::write(
                repo.git_dir().join("MERGE_MODE"),
                if o.ff == Ff::Never { "no-ff" } else { "" },
            )?;
            eprintln!("{e}");
            return Err(CliError::fatal("failed to write commit object"));
        }
    }
    let cobj = RawObject::new(ObjectKind::Commit, commit.encode()?);
    let format = repo.object_format();
    let coid = repo.store_mut().put(format, &cobj)?;
    let made = if trivial {
        "In-index merge".to_string()
    } else {
        format!("Merge made by the '{strategy}' strategy.")
    };
    let reflog = format!("merge {name}: {made}");
    match repo.head_target()? {
        RefTarget::Symbolic(branch) => {
            repo.refs_mut().update(&branch, &coid, Some(&head))?;
            let _ =
                crate::reflog::append_head_and_branch(&repo, &branch, Some(&head), &coid, &reflog);
        }
        RefTarget::Direct(_) => {
            repo.refs_mut().update("HEAD", &coid, Some(&head))?;
            let _ = crate::reflog::append_reflog(&repo, "HEAD", Some(&head), &coid, &reflog);
        }
    }
    say(made);
    if o.stat && !o.quiet {
        print_merge_diffstat(&repo, &head, &coid)?;
    }
    Ok(())
}

/// `.git/SQUASH_MSG` after `--squash`: the squashed commits, newest first.
fn write_squash_msg(
    repo: &arks_repo::Repository,
    head: &arks_oid::Oid,
    theirs: &arks_oid::Oid,
) -> CliResult<()> {
    let ours = arks_repo::ancestor_set(repo, *head)?;
    let mut queue = vec![*theirs];
    let mut seen = std::collections::HashSet::new();
    let mut picked = Vec::new();
    while let Some(c) = queue.pop() {
        if ours.contains(&c) || !seen.insert(c) {
            continue;
        }
        let commit = crate::cmd::log_format::read_commit(repo, &c)?;
        queue.extend(commit.parents.iter().copied());
        picked.push((c, commit));
    }
    picked.sort_by_key(|x| std::cmp::Reverse(x.1.committer.timestamp));
    let mut text = String::from("Squashed commit of the following:\n");
    for (oid, c) in picked {
        text.push_str(&format!(
            "\ncommit {}\nAuthor: {} <{}>\nDate:   {}\n\n",
            oid.to_hex(),
            c.author.name,
            c.author.email,
            crate::cmd::log_format::default_date(&c.author)
        ));
        for l in c.message.lines() {
            if l.is_empty() {
                text.push('\n');
            } else {
                text.push_str(&format!("    {l}\n"));
            }
        }
    }
    std::fs::write(repo.git_dir().join("SQUASH_MSG"), text)?;
    Ok(())
}

/// Refuse when the index differs from HEAD or a tracked file differs from the index.
/// Refuse a merge over local changes: any staged change (git-merge: "abort
/// if there are any changes registered in the index relative to HEAD"), or
/// a work-tree change on a path the merge touches (`touched` = paths that
/// differ between HEAD and the merged-in commit).
/// Cite: https://git-scm.com/docs/git-merge#_pre_merge_checks
fn refuse_dirty_tracked(
    repo: &arks_repo::Repository,
    touched: &std::collections::HashSet<String>,
) -> CliResult<()> {
    let head = crate::cmd::shared::head_blob_map(repo)?;
    let index = crate::util::read_index(repo)?;
    let work = work_tree(repo)?;
    let mut dirty = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for e in index.entries.iter().filter(|e| e.stage() == 0) {
        seen.insert(e.path.clone());
        let abs = work.join(&e.path);
        let wt_same = match std::fs::metadata(&abs) {
            Ok(m) if crate::cmd::shared::index_stat_matches(e, &m) => true,
            Ok(_) => {
                std::fs::read(&abs).ok().and_then(|b| {
                    RawObject::new(ObjectKind::Blob, b)
                        .oid(repo.object_format())
                        .ok()
                }) == Some(e.oid)
            }
            Err(_) => false,
        };
        if head.get(&e.path) != Some(&e.oid) || (!wt_same && touched.contains(&e.path)) {
            dirty.push(e.path.clone());
        }
    }
    dirty.extend(head.keys().filter(|k| !seen.contains(*k)).cloned());
    if dirty.is_empty() {
        return Ok(());
    }
    dirty.sort();
    let list: String = dirty.iter().map(|p| format!("\t{p}\n")).collect();
    Err(CliError::Exit(
        2,
        format!(
            "error: Your local changes to the following files would be overwritten by merge:\n{list}Please commit your changes or stash them before you merge.\nAborting\nMerge with strategy ort failed."
        ),
    ))
}

/// Paths whose blobs differ between the trees of `a` and `b`.
fn touched_paths(
    repo: &arks_repo::Repository,
    a: &arks_oid::Oid,
    b: &arks_oid::Oid,
) -> CliResult<std::collections::HashSet<String>> {
    let x = crate::cmd::shared::tree_blob_map(repo, a)?;
    let y = crate::cmd::shared::tree_blob_map(repo, b)?;
    Ok(x.keys()
        .chain(y.keys())
        .filter(|k| x.get(*k) != y.get(*k))
        .cloned()
        .collect())
}

/// git merge's closing diffstat (`--stat` block plus `create mode` /
/// `delete mode` / `mode change` summary lines) from `from` to `to`.
/// Cite: https://git-scm.com/docs/git-merge (merge.stat, default true).
fn print_merge_diffstat(
    repo: &arks_repo::Repository,
    from: &arks_oid::Oid,
    to: &arks_oid::Oid,
) -> CliResult<()> {
    use crate::cmd::switch_tree::commit_files;
    use crate::cmd::unified_diff::{StatRow, stat_change, write_stat};
    let a = commit_files(repo, from)?;
    let b = commit_files(repo, to)?;
    let mut keys: Vec<&String> = a.keys().chain(b.keys()).collect();
    keys.sort();
    keys.dedup();
    let mut rows = Vec::new();
    let mut modes = Vec::new();
    for k in keys {
        let (x, y) = (a.get(k), b.get(k));
        if x == y {
            continue;
        }
        let xp = match x {
            Some(f) => Some(repo.get_object(&f.oid)?.payload),
            None => None,
        };
        let yp = match y {
            Some(f) => Some(repo.get_object(&f.oid)?.payload),
            None => None,
        };
        rows.push(StatRow {
            path: k.clone(),
            change: stat_change(xp.as_deref(), yp.as_deref()),
        });
        match (x, y) {
            (None, Some(f)) => modes.push(format!(" create mode {:o} {k}", f.mode)),
            (Some(f), None) => modes.push(format!(" delete mode {:o} {k}", f.mode)),
            (Some(p), Some(q)) if p.mode != q.mode => {
                modes.push(format!(" mode change {:o} => {:o} {k}", p.mode, q.mode))
            }
            _ => {}
        }
    }
    let mut out = std::io::stdout().lock();
    write_stat(&mut out, &rows)?;
    for m in modes {
        std::io::Write::write_all(&mut out, format!("{m}\n").as_bytes())?;
    }
    Ok(())
}
