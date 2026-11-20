//! `git merge <commit> <commit>...`: merging several heads at once.
//!
//! Cite: <https://git-scm.com/docs/git-merge> ("Merging more than two
//! branches" defaults to the octopus strategy; `-s ours` takes the current
//! tree). Heads that are already contained in `HEAD` or in another head are
//! dropped first, as stock git does; one remaining head goes back to the
//! two-head merge, none means "Already up to date.".
//! The `resolve`, `recursive`, `ort` and `subtree` strategies refuse more
//! than one head (`Merge with strategy <s> failed.`, exit 2).

use crate::cmd::merge_strategies::{OctopusRemote, commit_of, index_tree, octopus, tree_of};
use crate::cmd::shared::is_ancestor;
use crate::error::{CliError, CliResult};
use crate::util::open_repo;
use arks_object::{Commit, ObjectKind, RawObject};
use arks_oid::Oid;
use arks_refs::RefTarget;
use arks_repo::Repository;
use arks_store::ObjectStore;

/// The `git merge` options a several-head merge honours.
pub(crate) struct ManyOpts {
    pub strategy: Option<String>,
    pub quiet: bool,
    pub stat: bool,
    pub message: Option<String>,
    pub commit: bool,
    pub squash: bool,
    pub ff_only: bool,
    pub into_name: Option<String>,
    pub log: Option<usize>,
}

/// Whether a single-head merge is "really trivial" (no path changed on both
/// sides), which is all `git merge -s octopus <one-head>` can do.
pub(crate) fn trivially_mergeable(
    repo: &mut Repository,
    head: &Oid,
    theirs: &Oid,
) -> CliResult<bool> {
    let Some(base) = arks_repo::merge_base_oid(repo, head, theirs)? else {
        return Ok(false);
    };
    let (b, o, t) = (
        tree_of(repo, &base)?,
        tree_of(repo, head)?,
        tree_of(repo, theirs)?,
    );
    let probe = crate::cmd::merge_engine::merge_trees_in_memory(repo, &b, &o, &t)?;
    Ok(probe.auto_merged.is_empty() && probe.conflicts.is_empty())
}

/// What the front end should do next.
pub(crate) enum ManyOutcome {
    /// Merged (or stopped as requested); nothing left to do.
    Done,
    /// Only this head is left: run the two-head merge on it.
    Single(String),
}

fn label_line(repo: &Repository, name: &str, oid: &Oid) -> String {
    let refs = repo.refs();
    let desc = if let Some(b) = name.strip_prefix("refs/heads/") {
        format!("branch '{b}'")
    } else if refs.resolve(&format!("refs/heads/{name}")).is_ok() {
        format!("branch '{name}'")
    } else if refs.resolve(&format!("refs/remotes/{name}")).is_ok() {
        format!("remote-tracking branch '{name}'")
    } else if refs.resolve(&format!("refs/tags/{name}")).is_ok() {
        format!("tag '{name}'")
    } else {
        format!("'{name}'")
    };
    format!("{}\t\t{desc} of .\n", oid.to_hex())
}

/// Merge every head in `names` into `HEAD`.
pub(crate) fn merge_heads(names: &[String], o: &ManyOpts) -> CliResult<ManyOutcome> {
    let mut repo = open_repo()?;
    let head = repo.head_oid()?;
    let mut heads: Vec<(String, Oid)> = Vec::new();
    for n in names {
        let oid = commit_of(&repo, n)
            .map_err(|_| CliError::fatal(format!("{n} - not something we can merge")))?;
        heads.push((n.clone(), oid));
    }
    // Drop heads already contained in HEAD or in another head.
    let mut kept: Vec<(String, Oid)> = Vec::new();
    for (i, (n, oid)) in heads.iter().enumerate() {
        if *oid == head || is_ancestor(&repo, oid, &head)? {
            continue;
        }
        let mut covered = false;
        for (j, (_, other)) in heads.iter().enumerate() {
            if i != j && other != oid && is_ancestor(&repo, oid, other)? {
                covered = true;
                break;
            }
        }
        if !covered && !kept.iter().any(|(_, k)| k == oid) {
            kept.push((n.clone(), *oid));
        }
    }
    match kept.len() {
        0 => {
            if !o.quiet {
                println!("Already up to date.");
            }
            return Ok(ManyOutcome::Done);
        }
        1 => return Ok(ManyOutcome::Single(kept[0].0.clone())),
        _ => {}
    }
    let strategy = o.strategy.clone().unwrap_or_else(|| "octopus".to_string());
    if o.ff_only {
        return Err(CliError::Exit(
            128,
            "hint: Diverging branches can't be fast-forwarded, you need to either:\nhint: \nhint: \tgit merge --no-ff\nhint: \nhint: or:\nhint: \nhint: \tgit rebase\nhint: \nhint: Disable this message with \"git config advice.diverging false\"\nfatal: Not possible to fast-forward, aborting.".into(),
        ));
    }
    let _ = repo.refs_mut().update("ORIG_HEAD", &head, None);
    let (tree, head_ff) = match strategy.as_str() {
        "octopus" => {
            // git merge hands the strategy full object names.
            let remotes: Vec<OctopusRemote> = kept
                .iter()
                .map(|(n, oid)| OctopusRemote {
                    arg: oid.to_hex(),
                    name: n.clone(),
                    oid: *oid,
                })
                .collect();
            let outcome = octopus(&mut repo, &head, &remotes)?;
            match outcome.code {
                0 => {}
                1 => {
                    write_merge_state(&repo, &kept, &message_for(&repo, &kept, o)?, true)?;
                    println!("Automatic merge failed; fix conflicts and then commit the result.");
                    return Err(CliError::silent(1));
                }
                _ => {
                    return Err(CliError::Exit(
                        2,
                        "Merge with strategy octopus failed.".into(),
                    ));
                }
            }
            let tree = index_tree(&mut repo)?
                .ok_or_else(|| CliError::Msg("merge: unmerged index after octopus".into()))?;
            (tree, outcome.head_fast_forwarded)
        }
        "ours" => (tree_of(&repo, &head)?, false),
        "resolve" => {
            return Err(CliError::Exit(
                2,
                "Merge with strategy resolve failed.".into(),
            ));
        }
        other => {
            return Err(CliError::Exit(
                2,
                format!(
                    "error: Not handling anything other than two heads merge.\nMerge with strategy {other} failed."
                ),
            ));
        }
    };
    let msg = message_for(&repo, &kept, o)?;
    if o.squash {
        write_squash(&repo, &head, &kept)?;
        eprintln!("Automatic merge went well; stopped before committing as requested");
        println!("Squash commit -- not updating HEAD");
        return Ok(ManyOutcome::Done);
    }
    if !o.commit {
        write_merge_state(&repo, &kept, &msg, false)?;
        eprintln!("Automatic merge went well; stopped before committing as requested");
        return Ok(ManyOutcome::Done);
    }
    let mut parents = Vec::new();
    if !head_ff {
        parents.push(head);
    }
    parents.extend(kept.iter().map(|(_, oid)| *oid));
    let commit = Commit {
        tree,
        parents,
        author: crate::cmd::shared::commit_signature()?,
        committer: crate::cmd::shared::committer_signature()?,
        gpgsig: None,
        message: msg,
    };
    let format = repo.object_format();
    let coid = repo.store_mut().put(
        format,
        &RawObject::new(ObjectKind::Commit, commit.encode()?),
    )?;
    let made = format!("Merge made by the '{strategy}' strategy.");
    let reflog = format!("merge {}: {made}", names.join(" "));
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
    if !o.quiet {
        println!("{made}");
        if o.stat {
            crate::cmd::merge::print_merge_diffstat(&repo, &head, &coid)?;
        }
    }
    Ok(ManyOutcome::Done)
}

fn message_for(repo: &Repository, kept: &[(String, Oid)], o: &ManyOpts) -> CliResult<String> {
    if let Some(m) = &o.message {
        let mut m = m.clone();
        if !m.ends_with('\n') {
            m.push('\n');
        }
        return Ok(m);
    }
    let input: String = kept
        .iter()
        .map(|(n, oid)| label_line(repo, n, oid))
        .collect();
    crate::cmd::plumbing_ext::fmt_merge_msg(repo, &input, None, o.into_name.as_deref(), o.log)
}

/// `MERGE_HEAD` (one line per head), `MERGE_MSG` and `MERGE_MODE`.
fn write_merge_state(
    repo: &Repository,
    kept: &[(String, Oid)],
    msg: &str,
    conflicts: bool,
) -> CliResult<()> {
    let git_dir = repo.git_dir();
    let heads: String = kept
        .iter()
        .map(|(_, o)| format!("{}\n", o.to_hex()))
        .collect();
    std::fs::write(git_dir.join("MERGE_HEAD"), heads)?;
    let mut text = msg.to_string();
    if conflicts {
        let index = crate::util::read_index(repo)?;
        let mut paths: Vec<&str> = index
            .entries
            .iter()
            .filter(|e| e.stage() > 0)
            .map(|e| e.path.as_str())
            .collect();
        paths.dedup();
        text.push_str("\n# Conflicts:\n");
        for p in paths {
            text.push_str(&format!("#\t{p}\n"));
        }
    }
    std::fs::write(git_dir.join("MERGE_MSG"), text)?;
    std::fs::write(git_dir.join("MERGE_MODE"), "")?;
    Ok(())
}

/// `SQUASH_MSG` listing the commits the squash brings in, newest first.
fn write_squash(repo: &Repository, head: &Oid, kept: &[(String, Oid)]) -> CliResult<()> {
    let ours = arks_repo::ancestor_set(repo, *head)?;
    let mut queue: Vec<Oid> = kept.iter().map(|(_, o)| *o).collect();
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
