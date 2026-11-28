//! Native `arks subtree` (E4 — `add`, `merge`, `split`, `pull`, `push`).
//!
//! Primary reference: Git `contrib/subtree/git-subtree.adoc`
//! (<https://github.com/git/git/blob/master/contrib/subtree/git-subtree.adoc>).
//! Spec: `sot:root/v0-2-0-scope-expansion` (E4).
//! Never spawns stock `git`/`gh`.

use crate::cmd::merge_engine::{MergeLabels, merge_trees_worktree};
use crate::cmd::shared::{commit_signature, committer_signature, is_ancestor, resolve_rev};
use crate::error::{CliError, CliResult};
use crate::util::{open_repo, work_tree};
use arks_object::{Commit, ObjectKind, RawObject, Tree, TreeEntry, TreeMode};
use arks_oid::Oid;
use arks_refs::RefTarget;
use arks_repo::{Repository, merge_base_oid};
use arks_store::ObjectStore;
use std::collections::{HashMap, HashSet};

const SUBTREE_USAGE: &str = "\
usage: arks subtree add   -P <prefix> [--squash] [-m <msg>] <commit>
   or: arks subtree add   -P <prefix> [--squash] [-m <msg>] <repository> <ref>
   or: arks subtree merge -P <prefix> [--squash] [-m <msg>] <commit>
   or: arks subtree split -P <prefix> [-b <branch>] [--rejoin] [--onto <onto>] [--annotate <prefix>] [<commit>]
   or: arks subtree pull  -P <prefix> [--squash] [-m <msg>] <repository> <ref>
   or: arks subtree push  -P <prefix> <repository> <refspec>
";

#[derive(Default)]
struct SubtreeOpts {
    prefix: Option<String>,
    message: Option<String>,
    branch: Option<String>,
    onto: Option<String>,
    annotate: Option<String>,
    squash: bool,
    rejoin: bool,
    quiet: bool,
    positionals: Vec<String>,
}

pub fn cmd_subtree(args: &[String]) -> CliResult<()> {
    if args.is_empty() || args.iter().any(|a| a == "-h" || a == "--help") {
        print!("{SUBTREE_USAGE}");
        return Ok(());
    }
    let sub = args[0].as_str();
    let opts = parse_opts(&args[1..])?;
    let prefix =
        normalize_prefix(opts.prefix.as_deref().ok_or_else(|| {
            CliError::Usage("subtree: you must provide the --prefix option".into())
        })?)?;
    match sub {
        "add" => cmd_subtree_add(&prefix, &opts),
        "merge" => cmd_subtree_merge(&prefix, &opts),
        "split" => cmd_subtree_split(&prefix, &opts),
        "pull" => cmd_subtree_pull(&prefix, &opts),
        "push" => cmd_subtree_push(&prefix, &opts),
        other => Err(CliError::Usage(format!(
            "subtree: unknown subcommand {other:?} (expected add|merge|split|pull|push)"
        ))),
    }
}

fn parse_opts(args: &[String]) -> CliResult<SubtreeOpts> {
    let mut opts = SubtreeOpts::default();
    let mut i = 0usize;
    while i < args.len() {
        let a = args[i].as_str();
        match a {
            "-P" | "--prefix" => {
                let v = args
                    .get(i + 1)
                    .ok_or_else(|| CliError::Usage("subtree: --prefix requires a value".into()))?;
                opts.prefix = Some(v.clone());
                i += 2;
            }
            _ if a.starts_with("--prefix=") => {
                opts.prefix = Some(a["--prefix=".len()..].to_string());
                i += 1;
            }
            _ if a.starts_with("-P") && a.len() > 2 => {
                opts.prefix = Some(a[2..].to_string());
                i += 1;
            }
            "-m" | "--message" => {
                let v = args
                    .get(i + 1)
                    .ok_or_else(|| CliError::Usage("subtree: --message requires a value".into()))?;
                opts.message = Some(v.clone());
                i += 2;
            }
            _ if a.starts_with("--message=") => {
                opts.message = Some(a["--message=".len()..].to_string());
                i += 1;
            }
            "-b" | "--branch" => {
                let v = args
                    .get(i + 1)
                    .ok_or_else(|| CliError::Usage("subtree: --branch requires a value".into()))?;
                opts.branch = Some(v.clone());
                i += 2;
            }
            _ if a.starts_with("--branch=") => {
                opts.branch = Some(a["--branch=".len()..].to_string());
                i += 1;
            }
            "--onto" => {
                let v = args
                    .get(i + 1)
                    .ok_or_else(|| CliError::Usage("subtree: --onto requires a value".into()))?;
                opts.onto = Some(v.clone());
                i += 2;
            }
            _ if a.starts_with("--onto=") => {
                opts.onto = Some(a["--onto=".len()..].to_string());
                i += 1;
            }
            "--annotate" => {
                let v = args.get(i + 1).ok_or_else(|| {
                    CliError::Usage("subtree: --annotate requires a value".into())
                })?;
                opts.annotate = Some(v.clone());
                i += 2;
            }
            _ if a.starts_with("--annotate=") => {
                opts.annotate = Some(a["--annotate=".len()..].to_string());
                i += 1;
            }
            "--squash" => {
                opts.squash = true;
                i += 1;
            }
            "--rejoin" => {
                opts.rejoin = true;
                i += 1;
            }
            "-q" | "--quiet" => {
                opts.quiet = true;
                i += 1;
            }
            "--" => {
                opts.positionals.extend(args[i + 1..].iter().cloned());
                break;
            }
            _ if a.starts_with('-') => {
                return Err(CliError::Usage(format!("subtree: unknown option {a:?}")));
            }
            _ => {
                opts.positionals.push(a.to_string());
                i += 1;
            }
        }
    }
    Ok(opts)
}

fn normalize_prefix(raw: &str) -> CliResult<String> {
    let trimmed = raw.trim().trim_matches('/');
    if trimmed.is_empty() {
        return Err(CliError::Usage("subtree: prefix must not be empty".into()));
    }
    for part in trimmed.split('/') {
        if part.is_empty() || part == "." || part == ".." {
            return Err(CliError::Usage(format!(
                "subtree: invalid prefix path component in {raw:?}"
            )));
        }
    }
    Ok(trimmed.to_string())
}

fn empty_tree(repo: &mut Repository) -> CliResult<Oid> {
    let format = repo.object_format();
    let obj = RawObject::new(ObjectKind::Tree, Vec::new());
    Ok(repo.store_mut().put(format, &obj)?)
}

fn load_commit(repo: &Repository, oid: &Oid) -> CliResult<Commit> {
    let obj = repo.get_object(oid)?;
    if obj.kind != ObjectKind::Commit {
        return Err(CliError::Msg(format!(
            "subtree: {} is not a commit",
            oid.to_hex()
        )));
    }
    Ok(Commit::decode(&obj.payload)?)
}

fn tree_entries(repo: &Repository, tree: &Oid) -> CliResult<Vec<TreeEntry>> {
    let obj = repo.get_object(tree)?;
    if obj.kind != ObjectKind::Tree {
        return Ok(Vec::new());
    }
    Ok(Tree::decode(&obj.payload, repo.oid_kind())?.entries)
}

fn subtree_at(repo: &Repository, tree: &Oid, prefix: &str) -> CliResult<Option<Oid>> {
    let mut cur = *tree;
    for part in prefix.split('/').filter(|p| !p.is_empty()) {
        let Some(e) = tree_entries(repo, &cur)?
            .into_iter()
            .find(|e| e.name == part && e.mode == TreeMode::Tree)
        else {
            return Ok(None);
        };
        cur = e.oid;
    }
    Ok(Some(cur))
}

fn place_at(repo: &mut Repository, tree: &Oid, prefix: &str) -> CliResult<Oid> {
    let format = repo.object_format();
    let mut cur = *tree;
    for part in prefix.rsplit('/').filter(|p| !p.is_empty()) {
        let t = Tree::new(vec![TreeEntry {
            mode: TreeMode::Tree,
            name: part.to_string(),
            oid: cur,
        }])?;
        cur = repo
            .store_mut()
            .put(format, &RawObject::new(ObjectKind::Tree, t.encode()?))?;
    }
    Ok(cur)
}

fn write_commit(repo: &mut Repository, commit: &Commit) -> CliResult<Oid> {
    let format = repo.object_format();
    let payload = commit.encode()?;
    Ok(repo
        .store_mut()
        .put(format, &RawObject::new(ObjectKind::Commit, payload))?)
}

fn update_head_commit(
    repo: &mut Repository,
    old_head: Option<Oid>,
    new_oid: Oid,
    reflog_msg: &str,
) -> CliResult<()> {
    match repo.head_target()? {
        RefTarget::Symbolic(refname) => {
            repo.refs_mut()
                .update(&refname, &new_oid, old_head.as_ref())?;
            crate::reflog::append_reflog(repo, &refname, old_head.as_ref(), &new_oid, reflog_msg)?;
        }
        RefTarget::Direct(_) => {
            repo.refs_mut()
                .update("HEAD", &new_oid, old_head.as_ref())?;
        }
    }
    crate::reflog::append_reflog(repo, "HEAD", old_head.as_ref(), &new_oid, reflog_msg)?;
    Ok(())
}

fn parse_subtree_trailers(message: &str) -> (Option<String>, Option<Oid>, Option<Oid>) {
    let mut dir = None;
    let mut mainline = None;
    let mut split = None;
    for line in message.lines() {
        let trimmed = line.trim();
        if let Some(v) = trimmed.strip_prefix("git-subtree-dir:") {
            dir = Some(v.trim().trim_matches('/').to_string());
        } else if let Some(v) = trimmed.strip_prefix("git-subtree-mainline:") {
            mainline = Oid::from_hex(v.trim()).ok();
        } else if let Some(v) = trimmed.strip_prefix("git-subtree-split:") {
            split = Oid::from_hex(v.trim()).ok();
        }
    }
    (dir, mainline, split)
}

fn strip_subtree_trailers(message: &str) -> String {
    let mut lines: Vec<&str> = message
        .lines()
        .filter(|l| {
            let t = l.trim();
            !t.starts_with("git-subtree-dir:")
                && !t.starts_with("git-subtree-mainline:")
                && !t.starts_with("git-subtree-split:")
        })
        .collect();
    while lines.last().is_some_and(|l| l.trim().is_empty()) {
        lines.pop();
    }
    let mut out = lines.join("\n");
    out.push('\n');
    out
}

/// Find the most recent `git-subtree-split` commit recorded in `head`'s first-parent/BFS history
/// for `prefix`.
fn find_latest_split(repo: &Repository, head: Oid, prefix: &str) -> CliResult<Option<Oid>> {
    let mut queue = std::collections::VecDeque::new();
    let mut seen = HashSet::new();
    queue.push_back(head);
    seen.insert(head);
    while let Some(oid) = queue.pop_front() {
        let c = load_commit(repo, &oid)?;
        let (dir, _mainline, split) = parse_subtree_trailers(&c.message);
        if dir.as_deref() == Some(prefix) {
            if let Some(s) = split {
                return Ok(Some(s));
            }
        }
        for p in c.parents {
            if seen.insert(p) {
                queue.push_back(p);
            }
        }
    }
    Ok(None)
}

fn create_squash_commit(
    repo: &mut Repository,
    prefix: &str,
    target_oid: Oid,
    target_tree: Oid,
    prev_squash: Option<Oid>,
) -> CliResult<Oid> {
    let author = commit_signature()?;
    let committer = committer_signature()?;
    let short = &target_oid.to_hex()[..7.min(target_oid.to_hex().len())];
    let message = format!(
        "Squashed '{prefix}/' content from commit {short}\n\ngit-subtree-dir: {prefix}\ngit-subtree-split: {}\n",
        target_oid.to_hex()
    );
    let mut parents = Vec::new();
    if let Some(p) = prev_squash {
        parents.push(p);
    }
    let commit = Commit {
        tree: target_tree,
        parents,
        author,
        committer,
        gpgsig: None,
        message,
    };
    write_commit(repo, &commit)
}

fn cmd_subtree_add(prefix: &str, opts: &SubtreeOpts) -> CliResult<()> {
    let rev_spec = match opts.positionals.len() {
        1 => opts.positionals[0].clone(),
        2 => {
            crate::cmd::fetch::cmd_fetch(&[
                opts.positionals[0].clone(),
                opts.positionals[1].clone(),
            ])?;
            "FETCH_HEAD".to_string()
        }
        _ => {
            return Err(CliError::Usage(
                "subtree add: expected <commit> or <repository> <ref>".into(),
            ));
        }
    };

    let mut repo = open_repo()?;
    let work = work_tree(&repo)?;
    let head_oid = resolve_rev(&repo, "HEAD")?;
    let head_commit = load_commit(&repo, &head_oid)?;
    if subtree_at(&repo, &head_commit.tree, prefix)?.is_some() {
        return Err(CliError::Msg(format!(
            "subtree add: prefix '{prefix}' already exists"
        )));
    }
    let target_oid = resolve_rev(&repo, &rev_spec)?;
    let target_commit = load_commit(&repo, &target_oid)?;

    let empty = empty_tree(&mut repo)?;
    let shifted_theirs = place_at(&mut repo, &target_commit.tree, prefix)?;
    let res = merge_trees_worktree(
        &mut repo,
        &work,
        &empty,
        &head_commit.tree,
        &shifted_theirs,
        true,
        &MergeLabels::default(),
    )?;
    if !res.conflicts.is_empty() {
        return Err(CliError::Msg(format!(
            "subtree add: merge conflict in {}",
            res.conflicts.join(", ")
        )));
    }
    let merged_tree = res
        .tree
        .ok_or_else(|| CliError::Msg("subtree add: merge produced no tree".into()))?;

    let second_parent = if opts.squash {
        create_squash_commit(&mut repo, prefix, target_oid, target_commit.tree, None)?
    } else {
        target_oid
    };

    let default_msg = format!("Add '{prefix}/' from commit '{}'", target_oid.to_hex());
    let body = opts.message.as_deref().unwrap_or(&default_msg);
    let message = format!(
        "{body}\n\ngit-subtree-dir: {prefix}\ngit-subtree-mainline: {}\ngit-subtree-split: {}\n",
        head_oid.to_hex(),
        target_oid.to_hex()
    );
    let commit = Commit {
        tree: merged_tree,
        parents: vec![head_oid, second_parent],
        author: commit_signature()?,
        committer: committer_signature()?,
        gpgsig: None,
        message,
    };
    let new_oid = write_commit(&mut repo, &commit)?;
    update_head_commit(
        &mut repo,
        Some(head_oid),
        new_oid,
        &format!("subtree add: {prefix}"),
    )?;
    if !opts.quiet {
        println!("Added dir '{prefix}'");
    }
    Ok(())
}

fn cmd_subtree_merge(prefix: &str, opts: &SubtreeOpts) -> CliResult<()> {
    if opts.positionals.len() != 1 {
        return Err(CliError::Usage("subtree merge: expected <commit>".into()));
    }
    let mut repo = open_repo()?;
    let work = work_tree(&repo)?;
    let head_oid = resolve_rev(&repo, "HEAD")?;
    let head_commit = load_commit(&repo, &head_oid)?;
    let target_oid = resolve_rev(&repo, &opts.positionals[0])?;
    let target_commit = load_commit(&repo, &target_oid)?;

    let prev_split = find_latest_split(&repo, head_oid, prefix)?;
    let base_tree = if let Some(prev) = prev_split {
        let base_sub_oid = if repo.get_object(&prev).is_ok() {
            merge_base_oid(&repo, &prev, &target_oid)?.unwrap_or(prev)
        } else {
            prev
        };
        if let Ok(base_c) = load_commit(&repo, &base_sub_oid) {
            place_at(&mut repo, &base_c.tree, prefix)?
        } else {
            empty_tree(&mut repo)?
        }
    } else {
        empty_tree(&mut repo)?
    };
    let shifted_theirs = place_at(&mut repo, &target_commit.tree, prefix)?;

    let res = merge_trees_worktree(
        &mut repo,
        &work,
        &base_tree,
        &head_commit.tree,
        &shifted_theirs,
        true,
        &MergeLabels::default(),
    )?;
    if !res.conflicts.is_empty() {
        return Err(CliError::Msg(format!(
            "subtree merge: merge conflict in {}",
            res.conflicts.join(", ")
        )));
    }
    let merged_tree = res
        .tree
        .ok_or_else(|| CliError::Msg("subtree merge: merge produced no tree".into()))?;

    let second_parent = if opts.squash {
        create_squash_commit(
            &mut repo,
            prefix,
            target_oid,
            target_commit.tree,
            prev_split,
        )?
    } else {
        target_oid
    };

    let default_msg = format!("Merge commit '{}' as '{prefix}'", target_oid.to_hex());
    let body = opts.message.as_deref().unwrap_or(&default_msg);
    let message = format!(
        "{body}\n\ngit-subtree-dir: {prefix}\ngit-subtree-mainline: {}\ngit-subtree-split: {}\n",
        head_oid.to_hex(),
        target_oid.to_hex()
    );
    let commit = Commit {
        tree: merged_tree,
        parents: vec![head_oid, second_parent],
        author: commit_signature()?,
        committer: committer_signature()?,
        gpgsig: None,
        message,
    };
    let new_oid = write_commit(&mut repo, &commit)?;
    update_head_commit(
        &mut repo,
        Some(head_oid),
        new_oid,
        &format!("subtree merge: {prefix}"),
    )?;
    if !opts.quiet {
        println!("Merged commit '{}' into '{prefix}'", target_oid.to_hex());
    }
    Ok(())
}

/// Topological post-order walk (parents before children) of all commits reachable from `tip`.
fn topo_ancestors(repo: &Repository, tip: Oid) -> CliResult<Vec<Oid>> {
    let mut visited = HashSet::new();
    let mut order = Vec::new();
    let mut stack: Vec<(Oid, bool)> = vec![(tip, false)];
    while let Some((oid, expanded)) = stack.pop() {
        if expanded {
            order.push(oid);
            continue;
        }
        if !visited.insert(oid) {
            continue;
        }
        stack.push((oid, true));
        let c = load_commit(repo, &oid)?;
        for p in c.parents.into_iter().rev() {
            if !visited.contains(&p) {
                stack.push((p, false));
            }
        }
    }
    Ok(order)
}

fn run_split(
    repo: &mut Repository,
    prefix: &str,
    start_rev: &str,
    onto: Option<&str>,
    annotate: Option<&str>,
) -> CliResult<Oid> {
    let start_oid = resolve_rev(repo, start_rev)?;
    let onto_oid = match onto {
        Some(r) => Some(resolve_rev(repo, r)?),
        None => None,
    };
    let order = topo_ancestors(repo, start_oid)?;
    let mut mapped: HashMap<Oid, Option<Oid>> = HashMap::new();
    let mut split_trees: HashMap<Oid, Oid> = HashMap::new();

    for oid in order {
        let c = load_commit(repo, &oid)?;
        let Some(sub_tree) = subtree_at(repo, &c.tree, prefix)? else {
            mapped.insert(oid, None);
            continue;
        };

        // If this commit already records an exact split for this prefix whose tree matches,
        // reuse the recorded split OID directly.
        let (tr_dir, _tr_main, tr_split) = parse_subtree_trailers(&c.message);
        if tr_dir.as_deref() == Some(prefix) {
            if let Some(rec_split) = tr_split {
                if let Ok(rec_c) = load_commit(repo, &rec_split) {
                    if rec_c.tree == sub_tree {
                        split_trees.insert(rec_split, rec_c.tree);
                        mapped.insert(oid, Some(rec_split));
                        continue;
                    }
                }
            }
        }

        let mut new_parents: Vec<Oid> = Vec::new();
        for p in &c.parents {
            if let Some(Some(mp)) = mapped.get(p) {
                if !new_parents.contains(mp) {
                    new_parents.push(*mp);
                }
            }
        }
        if new_parents.is_empty() {
            if let Some(o) = onto_oid {
                new_parents.push(o);
            }
        }

        // Skip commits that did not change the projected subtree when there is a single parent.
        if new_parents.len() == 1 {
            let p = new_parents[0];
            let p_tree = if let Some(t) = split_trees.get(&p) {
                *t
            } else {
                load_commit(repo, &p)?.tree
            };
            if p_tree == sub_tree {
                mapped.insert(oid, Some(p));
                continue;
            }
        }

        let cleaned = strip_subtree_trailers(&c.message);
        let message = match annotate {
            Some(ann) => format!("{ann}{cleaned}"),
            None => cleaned,
        };
        let split_commit = Commit {
            tree: sub_tree,
            parents: new_parents,
            author: c.author,
            committer: c.committer,
            gpgsig: None,
            message,
        };
        let new_oid = write_commit(repo, &split_commit)?;
        split_trees.insert(new_oid, sub_tree);
        mapped.insert(oid, Some(new_oid));
    }

    mapped.get(&start_oid).copied().flatten().ok_or_else(|| {
        CliError::Msg(format!(
            "subtree split: no revisions found for prefix '{prefix}'"
        ))
    })
}

fn cmd_subtree_split(prefix: &str, opts: &SubtreeOpts) -> CliResult<()> {
    if opts.positionals.len() > 1 {
        return Err(CliError::Usage(
            "subtree split: expected at most one <commit>".into(),
        ));
    }
    let start_rev = opts
        .positionals
        .first()
        .map(String::as_str)
        .unwrap_or("HEAD");
    let mut repo = open_repo()?;
    let split_oid = run_split(
        &mut repo,
        prefix,
        start_rev,
        opts.onto.as_deref(),
        opts.annotate.as_deref(),
    )?;

    if let Some(branch) = &opts.branch {
        let refname = if branch.starts_with("refs/") {
            branch.clone()
        } else {
            format!("refs/heads/{branch}")
        };
        let old_branch = repo.refs().resolve(&refname).ok();
        if let Some(existing) = old_branch {
            if existing != split_oid && !is_ancestor(&repo, &existing, &split_oid)? {
                return Err(CliError::Msg(format!(
                    "subtree split: branch '{branch}' is not an ancestor of created commit {}",
                    split_oid.to_hex()
                )));
            }
        }
        repo.refs_mut()
            .update(&refname, &split_oid, old_branch.as_ref())?;
    }

    if opts.rejoin {
        let head_oid = resolve_rev(&repo, "HEAD")?;
        let head_commit = load_commit(&repo, &head_oid)?;
        let default_msg = format!("Split '{prefix}/' into commit '{}'", split_oid.to_hex());
        let body = opts.message.as_deref().unwrap_or(&default_msg);
        let message = format!(
            "{body}\n\ngit-subtree-dir: {prefix}\ngit-subtree-mainline: {}\ngit-subtree-split: {}\n",
            head_oid.to_hex(),
            split_oid.to_hex()
        );
        let rejoin_commit = Commit {
            tree: head_commit.tree,
            parents: vec![head_oid, split_oid],
            author: commit_signature()?,
            committer: committer_signature()?,
            gpgsig: None,
            message,
        };
        let rejoin_oid = write_commit(&mut repo, &rejoin_commit)?;
        update_head_commit(
            &mut repo,
            Some(head_oid),
            rejoin_oid,
            &format!("subtree split --rejoin: {prefix}"),
        )?;
    }

    println!("{}", split_oid.to_hex());
    Ok(())
}

fn cmd_subtree_pull(prefix: &str, opts: &SubtreeOpts) -> CliResult<()> {
    if opts.positionals.len() != 2 {
        return Err(CliError::Usage(
            "subtree pull: expected <repository> <ref>".into(),
        ));
    }
    crate::cmd::fetch::cmd_fetch(&[opts.positionals[0].clone(), opts.positionals[1].clone()])?;
    let merge_opts = SubtreeOpts {
        prefix: opts.prefix.clone(),
        message: opts.message.clone(),
        squash: opts.squash,
        quiet: opts.quiet,
        positionals: vec!["FETCH_HEAD".to_string()],
        ..SubtreeOpts::default()
    };
    cmd_subtree_merge(prefix, &merge_opts)
}

fn cmd_subtree_push(prefix: &str, opts: &SubtreeOpts) -> CliResult<()> {
    if opts.positionals.len() != 2 {
        return Err(CliError::Usage(
            "subtree push: expected <repository> <refspec>".into(),
        ));
    }
    let mut repo = open_repo()?;
    let split_oid = run_split(
        &mut repo,
        prefix,
        "HEAD",
        opts.onto.as_deref(),
        opts.annotate.as_deref(),
    )?;
    drop(repo);
    let target_ref = if opts.positionals[1].starts_with("refs/") {
        opts.positionals[1].clone()
    } else {
        format!("refs/heads/{}", opts.positionals[1])
    };
    let refspec = format!("{}:{target_ref}", split_oid.to_hex());
    crate::cmd::push::cmd_push(&[opts.positionals[0].clone(), refspec])
}
