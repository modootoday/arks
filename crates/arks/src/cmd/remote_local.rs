//! `fetch` / `push` against a local repository (plain path or same-host
//! `file://`): refspec matching, fast-forward checks and ref updates on top
//! of the object copy in `arks_remote::copy_objects`.
//!
//! Primary: <https://git-scm.com/docs/git-fetch>, <https://git-scm.com/docs/git-push>,
//! <https://git-scm.com/docs/git-config#Documentation/git-config.txt-receivedenyCurrentBranch>.
//!
//! Partial: tag auto-following, `--prune`, negative refspecs and
//! `receive.denyCurrentBranch` values other than the default `refuse` are
//! not claimed.

use crate::cmd::shared::is_ancestor;
use crate::error::{CliError, CliResult};
use crate::util::open_repo;
use arks_oid::Oid;
use arks_refs::RefTarget;
use arks_repo::Repository;
use std::fs;

/// One `[+]<src>[:<dst>]` refspec.
struct Refspec {
    force: bool,
    src: String,
    dst: Option<String>,
}

fn parse_refspec(s: &str) -> Refspec {
    let (force, rest) = match s.strip_prefix('+') {
        Some(r) => (true, r),
        None => (false, s),
    };
    match rest.split_once(':') {
        Some((a, b)) => Refspec {
            force,
            src: a.to_string(),
            dst: Some(b.to_string()),
        },
        None => Refspec {
            force,
            src: rest.to_string(),
            dst: None,
        },
    }
}

/// DWIM a short source name against `refs` (exact, heads, tags, remotes).
fn dwim<'a>(name: &str, refs: &'a [(String, Oid)]) -> Option<&'a (String, Oid)> {
    let cands = [
        name.to_string(),
        format!("refs/{name}"),
        format!("refs/tags/{name}"),
        format!("refs/heads/{name}"),
        format!("refs/remotes/{name}"),
    ];
    cands.iter().find_map(|c| refs.iter().find(|(n, _)| n == c))
}

fn short(name: &str) -> &str {
    ["refs/heads/", "refs/tags/", "refs/remotes/"]
        .iter()
        .find_map(|p| name.strip_prefix(p))
        .unwrap_or(name)
}

/// Destination ref for `<dst>`: full refs pass through, else the source's namespace.
fn qualify_dst(dst: &str, src_full: &str) -> String {
    if dst.starts_with("refs/") || dst == "HEAD" {
        dst.to_string()
    } else if src_full.starts_with("refs/tags/") {
        format!("refs/tags/{dst}")
    } else {
        format!("refs/heads/{dst}")
    }
}

/// Branch `HEAD` of a non-bare repository points at, if any.
fn checked_out_branch(repo: &Repository) -> Option<String> {
    if repo.kind() == arks_repo::GitDirKind::Bare {
        return None;
    }
    match repo.head_target() {
        Ok(RefTarget::Symbolic(s)) => Some(s),
        _ => None,
    }
}

/// `fetch <url|remote> [<refspec>...]` from a local repository.
pub(crate) fn fetch_local(
    url: &str,
    remote_name: Option<&str>,
    config_specs: &[String],
    refspecs: &[String],
    quiet: bool,
) -> CliResult<()> {
    let src = arks_remote::open_local_remote(url).map_err(|e| CliError::fatal(e.to_string()))?;
    let mut repo = open_repo()?;
    let mut remote_refs = src.refs().list_refs("refs/")?;
    if let Ok(h) = src.head_oid() {
        remote_refs.push(("HEAD".to_string(), h));
    }
    let explicit = !refspecs.is_empty();
    let specs: Vec<String> = if explicit {
        refspecs.to_vec()
    } else if !config_specs.is_empty() {
        config_specs.to_vec()
    } else {
        vec!["HEAD".to_string()]
    };
    // (source full name, oid, destination ref or None for FETCH_HEAD only, force)
    let mut plan: Vec<(String, Oid, Option<String>, bool)> = Vec::new();
    for raw in &specs {
        let spec = parse_refspec(raw);
        if let Some((sp, ss)) = spec.src.split_once('*') {
            let (dp, ds) = spec
                .dst
                .as_deref()
                .and_then(|d| d.split_once('*'))
                .ok_or_else(|| CliError::fatal(format!("invalid refspec '{raw}'")))?;
            let mut hits: Vec<&(String, Oid)> = remote_refs
                .iter()
                .filter(|(n, _)| {
                    n.starts_with(sp) && n.ends_with(ss) && n.len() >= sp.len() + ss.len()
                })
                .collect();
            hits.sort_by(|a, b| a.0.cmp(&b.0));
            for (n, o) in hits {
                let mid = &n[sp.len()..n.len() - ss.len()];
                plan.push((n.clone(), *o, Some(format!("{dp}{mid}{ds}")), spec.force));
            }
            continue;
        }
        let (full, oid) = dwim(&spec.src, &remote_refs)
            .cloned()
            .ok_or_else(|| CliError::fatal(format!("couldn't find remote ref {}", spec.src)))?;
        let dst = spec
            .dst
            .filter(|d| !d.is_empty())
            .map(|d| qualify_dst(&d, &full));
        plan.push((full, oid, dst, spec.force));
    }
    if let Some(cur) = checked_out_branch(&repo) {
        if plan.iter().any(|p| p.2.as_deref() == Some(cur.as_str())) {
            let work = crate::util::work_tree(&repo)?;
            return Err(CliError::fatal(format!(
                "refusing to fetch into branch '{cur}' checked out at '{}'",
                work.display()
            )));
        }
    }
    let tips: Vec<Oid> = plan.iter().map(|p| p.1).collect();
    arks_remote::copy_objects(&src, &mut repo, &tips).map_err(|e| CliError::Msg(e.to_string()))?;
    let mut lines = Vec::new();
    let mut fetch_head = String::new();
    let mut rejected = false;
    for (full, oid, dst, force) in &plan {
        let kind = if full.starts_with("refs/tags/") {
            "tag"
        } else {
            "branch"
        };
        let merge = if explicit || remote_name.is_none() {
            ""
        } else {
            "not-for-merge"
        };
        let label = if full == "HEAD" {
            String::new()
        } else {
            format!("{kind} '{}' of ", short(full))
        };
        fetch_head.push_str(&format!("{}\t{merge}\t{label}{url}\n", oid.to_hex()));
        let Some(dst) = dst else {
            lines.push(format!(" * {kind:<17} {:<10} -> FETCH_HEAD", short(full)));
            continue;
        };
        let old = repo.refs().resolve(dst).ok();
        match old {
            Some(o) if o == *oid => {}
            Some(o) => {
                let ff = is_ancestor(&repo, &o, oid)?;
                if dst.starts_with("refs/tags/") && !force {
                    lines.push(format!(
                        " ! [rejected]        {:<10} -> {} (would clobber existing tag)",
                        short(full),
                        short(dst)
                    ));
                    rejected = true;
                    continue;
                }
                if !ff && !force {
                    lines.push(format!(
                        " ! [rejected]        {:<10} -> {}  (non-fast-forward)",
                        short(full),
                        short(dst)
                    ));
                    rejected = true;
                    continue;
                }
                repo.refs_mut().update(dst, oid, Some(&o))?;
                lines.push(format!(
                    "   {}..{}  {:<10} -> {}",
                    &o.to_hex()[..7],
                    &oid.to_hex()[..7],
                    short(full),
                    short(dst)
                ));
            }
            None => {
                repo.refs_mut().update(dst, oid, None)?;
                let what = if kind == "tag" { "tag" } else { "branch" };
                lines.push(format!(
                    " * [new {what}]{:pad$}{:<10} -> {}",
                    "",
                    short(full),
                    short(dst),
                    pad = 18 - 6 - what.len()
                ));
            }
        }
        if dst.starts_with("refs/heads/") || dst.starts_with("refs/remotes/") {
            let _ = crate::reflog::append_reflog(
                &repo,
                dst,
                old.as_ref(),
                oid,
                &format!("fetch {url} {}: storing head", short(full)),
            );
        }
    }
    fs::write(repo.git_dir().join("FETCH_HEAD"), fetch_head)?;
    if !quiet {
        eprintln!("From {url}");
        for l in &lines {
            eprintln!("{l}");
        }
    }
    if rejected {
        return Err(CliError::silent(1));
    }
    Ok(())
}

/// One resolved push update (`dst` on the remote; `new` None = delete).
pub(crate) struct LocalPush {
    pub src_label: String,
    pub dst: String,
    pub new: Option<Oid>,
    pub force: bool,
}

/// Push updates into a local repository: copy objects, refuse non-fast-forward
/// (unless forced) and the remote's checked-out branch, then update refs.
pub(crate) fn push_local(url: &str, updates: &[LocalPush], quiet: bool) -> CliResult<Vec<String>> {
    let repo = open_repo()?;
    let mut dst_repo =
        arks_remote::open_local_remote(url).map_err(|e| CliError::fatal(e.to_string()))?;
    let tips: Vec<Oid> = updates.iter().filter_map(|u| u.new).collect();
    arks_remote::copy_objects(&repo, &mut dst_repo, &tips)
        .map_err(|e| CliError::Msg(e.to_string()))?;
    let current = checked_out_branch(&dst_repo);
    let mut lines = Vec::new();
    let mut done = Vec::new();
    let mut failed = false;
    for u in updates {
        let old = dst_repo.refs().resolve(&u.dst).ok();
        let dst_short = short(&u.dst).to_string();
        if old == u.new {
            lines.push(format!(
                " = [up to date]      {} -> {dst_short}",
                u.src_label
            ));
            continue;
        }
        if let (Some(o), Some(n)) = (old, u.new) {
            let ff = repo.get_object(&o).is_ok() && is_ancestor(&repo, &o, &n)?;
            if !ff && !u.force {
                let why = if u.dst.starts_with("refs/tags/") {
                    "already exists"
                } else {
                    "non-fast-forward"
                };
                lines.push(format!(
                    " ! [rejected]        {} -> {dst_short} ({why})",
                    u.src_label
                ));
                failed = true;
                continue;
            }
        }
        if current.as_deref() == Some(u.dst.as_str()) {
            lines.push(format!(
                " ! [remote rejected] {} -> {dst_short} (branch is currently checked out)",
                u.src_label
            ));
            failed = true;
            continue;
        }
        match u.new {
            Some(n) => dst_repo.refs_mut().update(&u.dst, &n, old.as_ref())?,
            None => dst_repo.refs_mut().delete(&u.dst, old.as_ref())?,
        }
        let line = match (old, u.new) {
            (None, Some(_)) => {
                let what = if u.dst.starts_with("refs/tags/") {
                    "tag"
                } else {
                    "branch"
                };
                format!(
                    " * [new {what}]{:pad$}{} -> {dst_short}",
                    "",
                    u.src_label,
                    pad = 18 - 6 - what.len()
                )
            }
            (Some(_), None) => format!(" - [deleted]         {dst_short}"),
            (Some(o), Some(n)) => format!(
                "   {}..{}  {} -> {dst_short}",
                &o.to_hex()[..7],
                &n.to_hex()[..7],
                u.src_label
            ),
            (None, None) => continue,
        };
        lines.push(line);
        done.push(u.dst.clone());
    }
    if !quiet || failed {
        eprintln!("To {url}");
        for l in &lines {
            eprintln!("{l}");
        }
    }
    if failed {
        return Err(CliError::Exit(
            1,
            format!("error: failed to push some refs to '{url}'"),
        ));
    }
    Ok(done)
}
