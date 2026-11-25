//! `push`: option table, refspec expansion (`--all`, `--branches`, `--tags`,
//! `--mirror`, `--delete`, `--follow-tags`, `--prune`, push.default), leases,
//! the `pre-push` hook and remote-tracking / upstream bookkeeping.
//!
//! Primary: <https://git-scm.com/docs/git-push>, <https://git-scm.com/docs/githooks#_pre_push>.

use crate::cmd::fetch::{configured_source, default_remote};
use crate::cmd::opt_parse::{Mode, OptDef, Takes::*, o, parse};
use crate::cmd::remote_local::{
    LocalPush, LocalPushOpts, PushStatus, dwim, parse_refspec, push_local, short_ref,
};
use crate::cmd::shared::resolve_rev;
use crate::error::{CliError, CliResult};
use crate::util::open_repo;
use arks_oid::Oid;
use arks_refs::RefTarget;
use arks_repo::Repository;
use std::io::Write;

/// Every option `git push -h` lists (pin 2.39.5 plus host 2.43 additions).
const PUSH_OPTS: &[OptDef] = &[
    o("verbose", 'v', No, true),
    o("quiet", 'q', No, true),
    o("repo", ' ', Req, true),
    o("all", ' ', No, true),
    o("branches", ' ', No, true),
    o("mirror", ' ', No, true),
    o("delete", 'd', No, true),
    o("tags", ' ', No, true),
    o("dry-run", 'n', No, true),
    o("porcelain", ' ', No, true),
    o("force", 'f', No, true),
    o("force-with-lease", ' ', Opt, true),
    o("force-if-includes", ' ', No, true),
    o("recurse-submodules", ' ', Req, true),
    o("thin", ' ', No, true),
    o("receive-pack", ' ', Req, true),
    o("exec", ' ', Req, true),
    o("set-upstream", 'u', No, true),
    o("progress", ' ', No, true),
    o("prune", ' ', No, true),
    o("no-verify", ' ', No, true),
    o("follow-tags", ' ', No, true),
    o("signed", ' ', Opt, true),
    o("atomic", ' ', No, true),
    o("push-option", 'o', Req, true),
    o("ipv4", '4', No, true),
    o("ipv6", '6', No, true),
];

/// One `--force-with-lease` argument.
enum Lease {
    /// Bare flag: every pushed ref expects its remote-tracking value.
    All,
    /// `<ref>` (tracking value) or `<ref>:<expect>` (empty = must not exist).
    Ref(String, Option<String>),
}

pub fn cmd_push(args: &[String]) -> CliResult<()> {
    let p = parse("push", args, PUSH_OPTS, Mode::default())?;
    let operands = p.operands();
    let repo = open_repo()?;
    let cfg = arks_config::ConfigSet::load(Some(repo.common_dir()));
    if p.on("mirror") && (p.on("all") || p.on("branches") || p.on("tags") || p.on("delete")) {
        return Err(CliError::fatal(
            "options '--mirror' and '--all/--branches/--tags/--delete' cannot be used together",
        ));
    }
    if (p.on("all") || p.on("branches")) && p.on("tags") {
        return Err(CliError::fatal(
            "options '--all/--branches' and '--tags' cannot be used together",
        ));
    }
    if p.on("delete") && operands.len() < 2 {
        return Err(CliError::fatal(
            "--delete doesn't make sense without any refs",
        ));
    }
    if let Some(sm) = p.value("recurse-submodules") {
        if !matches!(sm.as_str(), "check" | "on-demand" | "only" | "no") {
            return Err(CliError::fatal(format!(
                "bad recurse-submodules argument: {sm}"
            )));
        }
    }
    let signed = p.value("signed").unwrap_or_else(|| {
        if p.on("signed") {
            "yes".into()
        } else {
            "no".into()
        }
    });
    let signed_required = match signed.as_str() {
        "yes" | "true" | "on" | "1" => true,
        "no" | "false" | "off" | "0" | "if-asked" => false,
        other => {
            return Err(CliError::fatal(format!("bad signed argument: {other}")));
        }
    };

    let mut positional = operands.clone();
    let repo_arg = if !positional.is_empty() {
        Some(positional.remove(0))
    } else {
        p.value("repo")
    };
    let current = match repo.head_target() {
        Ok(RefTarget::Symbolic(s)) => s.strip_prefix("refs/heads/").map(String::from),
        _ => None,
    };
    let remote_name = repo_arg.unwrap_or_else(|| {
        current
            .as_deref()
            .and_then(|b| cfg.get(&format!("branch.{b}.pushremote")))
            .or_else(|| cfg.get("remote.pushdefault"))
            .unwrap_or_else(|| default_remote(&repo, &cfg))
    });
    let configured = configured_source(&cfg, &remote_name);
    let url = match &configured {
        Some(s) => cfg
            .get_all(&format!("remote.{remote_name}.pushurl"))
            .into_iter()
            .next()
            .unwrap_or_else(|| s.url.clone()),
        None if arks_remote::is_local_remote(&remote_name)
            || arks_remote::HttpRemoteUrl::parse(&remote_name).is_ok()
            || arks_remote::SshRemoteUrl::parse(&remote_name).is_ok()
            || arks_remote::GitDaemonUrl::parse(&remote_name).is_ok() =>
        {
            remote_name.clone()
        }
        None => {
            return Err(CliError::fatal(format!(
                "'{remote_name}' does not appear to be a git repository\nfatal: Could not read from remote repository.\n\nPlease make sure you have the correct access rights\nand the repository exists."
            )));
        }
    };
    let local = arks_remote::is_local_remote(&url);
    let mirror = p.on("mirror")
        || (configured.is_some()
            && cfg
                .get_bool(&format!("remote.{remote_name}.mirror"))
                .unwrap_or(false));
    let force_all = p.on("force");
    if p.count("verbose") > 0 && !p.on("porcelain") {
        eprintln!("Pushing to {url}");
    }

    // Remote refs (for DWIM, prune, mirror and statuses): the local repository's
    // refs, or the receive-pack advertisement of a network remote.
    let remote_refs: Vec<(String, Oid)> = if local {
        let r = arks_remote::open_local_remote(&url).map_err(|e| CliError::fatal(e.to_string()))?;
        r.refs().list_refs("refs/")?
    } else {
        arks_remote::ls_remote_for_push(&url)?
            .into_iter()
            .filter(|r| r.name.starts_with("refs/"))
            .map(|r| (r.name, r.oid))
            .collect()
    };
    let local_refs = repo.refs().list_refs("refs/")?;

    // Expand refspecs into updates.
    let mut updates: Vec<LocalPush> = Vec::new();
    let add = |u: LocalPush, list: &mut Vec<LocalPush>| {
        if !list.iter().any(|x| x.dst == u.dst) {
            list.push(u);
        }
    };
    let mut specs: Vec<String> = positional.clone();
    if p.on("delete") {
        specs = specs.iter().map(|s| format!(":{s}")).collect();
    }
    if mirror {
        for (n, oid) in local_refs.iter().filter(|(n, _)| n != "refs/stash") {
            add(lp(n, n, Some(*oid), true), &mut updates);
        }
        for (n, _) in &remote_refs {
            if !local_refs.iter().any(|(l, _)| l == n && l != "refs/stash") {
                add(lp("", n, None, true), &mut updates);
            }
        }
    }
    if p.on("all") || p.on("branches") {
        for (n, oid) in local_refs
            .iter()
            .filter(|(n, _)| n.starts_with("refs/heads/"))
        {
            add(lp(n, n, Some(*oid), force_all), &mut updates);
        }
    }
    if p.on("tags") {
        for (n, oid) in local_refs
            .iter()
            .filter(|(n, _)| n.starts_with("refs/tags/"))
        {
            add(lp(n, n, Some(*oid), force_all), &mut updates);
        }
    }
    let mut prune_globs = Vec::new();
    if specs.is_empty() && !mirror && !p.on("all") && !p.on("branches") && !p.on("tags") {
        let config_push = if configured.is_some() {
            cfg.get_all(&format!("remote.{remote_name}.push"))
        } else {
            Vec::new()
        };
        if !config_push.is_empty() {
            specs = config_push;
        } else {
            specs.push(default_push_spec(
                &repo,
                &cfg,
                &remote_name,
                configured.is_some(),
            )?);
        }
    }
    for raw in &specs {
        if raw == ":" || raw == "+:" {
            // Matching refspec: branches that exist on both sides.
            for (n, oid) in local_refs
                .iter()
                .filter(|(n, _)| n.starts_with("refs/heads/"))
            {
                if remote_refs.iter().any(|(r, _)| r == n) {
                    add(
                        lp(n, n, Some(*oid), raw.starts_with('+') || force_all),
                        &mut updates,
                    );
                }
            }
            continue;
        }
        let spec = parse_refspec(raw);
        let force = spec.force || force_all;
        if spec.is_glob() {
            prune_globs.push(spec.clone());
            for (n, oid) in &local_refs {
                if let Some(d) = spec.map_src(n) {
                    add(lp(n, &d, Some(*oid), force), &mut updates);
                }
            }
            continue;
        }
        if spec.src.is_empty() {
            let d = spec.dst.clone().unwrap_or_default();
            let full = qualify_remote(&d, None, &remote_refs)
                .ok_or_else(|| CliError::Exit(1, format!("error: unable to delete '{d}': remote ref does not exist\nerror: failed to push some refs to '{url}'")))?;
            add(lp("", &full, None, force), &mut updates);
            continue;
        }
        let (src_full, oid) = resolve_src(&repo, &local_refs, &spec.src)?;
        let dst = match spec.dst.as_deref() {
            Some(d) if !d.is_empty() => qualify_remote(d, Some(&src_full), &remote_refs)
                .ok_or_else(|| CliError::Exit(1, format!("error: The destination you provided is not a full refname (i.e.,\nstarting with \"refs/\"). We tried to guess what you meant by:\n\n- Looking for a ref that matches '{d}' on the remote side.\n- Checking if the <src> being pushed ('{}')\n  is a ref in \"refs/{{heads,tags}}/\". If so we add a corresponding\n  refs/{{heads,tags}}/ prefix on the remote side.\n\nNeither worked, so we gave up. You must fully qualify the ref.\nerror: failed to push some refs to '{url}'", spec.src)))?,
            _ if src_full.starts_with("refs/") => src_full.clone(),
            _ => {
                return Err(CliError::Exit(1, format!("error: src refspec {} does not match any\nerror: failed to push some refs to '{url}'", spec.src)));
            }
        };
        let label = if spec.src == "HEAD" || !src_full.starts_with("refs/") {
            spec.src.clone()
        } else {
            src_full.clone()
        };
        add(lp(&label, &dst, Some(oid), force), &mut updates);
    }
    if p.on("follow-tags") || cfg.get_bool("push.followtags").unwrap_or(false) {
        follow_tags(&repo, &local_refs, &remote_refs, &mut updates)?;
    }
    if p.on("prune") {
        for spec in &prune_globs {
            for (n, _) in &remote_refs {
                if let Some(src) = spec.map_dst(n) {
                    if !local_refs.iter().any(|(l, _)| *l == src) {
                        add(lp("", n, None, true), &mut updates);
                    }
                }
            }
        }
    }
    if updates.is_empty() {
        if !p.on("porcelain") {
            eprintln!("Everything up-to-date");
        }
        return Ok(());
    }

    // Leases.
    let leases: Vec<Lease> = p
        .opts()
        .filter(|(key, _, _)| *key == "force-with-lease")
        .fold(Vec::new(), |mut acc, (_, value, neg)| {
            if neg {
                acc.clear();
            } else {
                match value {
                    None => acc.push(Lease::All),
                    Some(v) => match v.split_once(':') {
                        Some((r, x)) => acc.push(Lease::Ref(r.to_string(), Some(x.to_string()))),
                        None => acc.push(Lease::Ref(v.to_string(), None)),
                    },
                }
            }
            acc
        });
    for u in updates.iter_mut() {
        let tracking = configured.as_ref().and_then(|s| {
            s.config_specs
                .iter()
                .find_map(|sp| parse_refspec(sp).map_src(&u.dst))
                .and_then(|t| repo.refs().resolve(&t).ok())
        });
        for l in &leases {
            match l {
                Lease::All => u.expect = Some(tracking),
                Lease::Ref(r, x) => {
                    let full = if r.starts_with("refs/") {
                        r.clone()
                    } else {
                        format!("refs/heads/{r}")
                    };
                    if full != u.dst && short_ref(&u.dst) != r {
                        continue;
                    }
                    u.expect = Some(match x.as_deref() {
                        None => tracking,
                        Some("") => None,
                        Some(rev) => Some(resolve_rev(&repo, rev)?),
                    });
                }
            }
        }
    }

    if local && !p.values("push-option").is_empty() {
        return Err(CliError::fatal(
            "the receiving end does not support push options\nfatal: the remote end hung up unexpectedly",
        ));
    }
    if local && signed_required {
        return Err(CliError::fatal(
            "the receiving end does not support --signed push\nfatal: the remote end hung up unexpectedly",
        ));
    }
    if local && signed == "if-asked" {
        eprintln!(
            "warning: not sending a push certificate since the receiving end does not support --signed push"
        );
    }
    if !p.on("no-verify") && !p.on("dry-run") {
        run_pre_push(&repo, &remote_name, &url, &updates, &remote_refs)?;
    }
    let opts = LocalPushOpts {
        quiet: p.on("quiet"),
        verbose: p.count("verbose") > 0,
        porcelain: p.on("porcelain"),
        dry_run: p.on("dry-run"),
        atomic: p.on("atomic")
            || cfg
                .get_bool(&format!("remote.{remote_name}.atomic"))
                .unwrap_or(false)
            || cfg.get_bool("push.atomic").unwrap_or(false),
    };
    let statuses = if local {
        push_local(&url, &updates, &remote_refs, &opts)?
    } else {
        push_network(
            &repo,
            &url,
            &updates,
            &p.values("push-option"),
            opts.dry_run,
        )?
    };
    let ok = |s: &PushStatus| {
        matches!(
            s,
            PushStatus::New
                | PushStatus::FastForward(_)
                | PushStatus::Forced(_)
                | PushStatus::Deleted
                | PushStatus::UpToDate
        )
    };
    let mut repo_mut = open_repo()?;
    let mut tail = Vec::new();
    if !opts.dry_run {
        // git-push: update refs/remotes/<remote>/<branch> after a successful push.
        if let Some(src) = &configured {
            for (u, _) in updates.iter().zip(&statuses).filter(|(_, s)| ok(s)) {
                let Some(t) = src
                    .config_specs
                    .iter()
                    .find_map(|sp| parse_refspec(sp).map_src(&u.dst))
                else {
                    continue;
                };
                match u.new {
                    Some(n) => repo_mut.refs_mut().update(&t, &n, None)?,
                    None => crate::cmd::remote_local::delete_ref_any(&mut repo_mut, &t)?,
                }
            }
        }
        if p.on("set-upstream") {
            for (u, _) in updates.iter().zip(&statuses).filter(|(_, s)| ok(s)) {
                let (Some(b), Some(_)) = (u.dst.strip_prefix("refs/heads/"), u.new) else {
                    continue;
                };
                let local_branch = u
                    .src_label
                    .strip_prefix("refs/heads/")
                    .map(String::from)
                    .or_else(|| (u.src_label == "HEAD").then(|| current.clone()).flatten());
                let Some(lb) = local_branch else { continue };
                crate::cmd::config::cmd_config(&[
                    format!("branch.{lb}.remote"),
                    remote_name.clone(),
                ])?;
                crate::cmd::config::cmd_config(&[format!("branch.{lb}.merge"), u.dst.clone()])?;
                tail.push(format!(
                    "branch '{lb}' set up to track '{remote_name}/{b}'."
                ));
            }
        }
    }
    let mut out = std::io::stdout().lock();
    for l in &tail {
        writeln!(out, "{l}")?;
    }
    if opts.porcelain {
        writeln!(out, "Done")?;
    }
    if statuses.iter().any(|s| !ok(s)) {
        return Err(CliError::silent(1));
    }
    Ok(())
}

fn lp(src: &str, dst: &str, new: Option<Oid>, force: bool) -> LocalPush {
    LocalPush {
        src_label: src.to_string(),
        dst: dst.to_string(),
        new,
        force,
        expect: None,
    }
}

/// `push.default` (default `simple`) with no refspec: the current branch.
fn default_push_spec(
    repo: &Repository,
    cfg: &arks_config::ConfigSet,
    remote: &str,
    named: bool,
) -> CliResult<String> {
    let branch = match repo.head_target()? {
        RefTarget::Symbolic(s) => s
            .strip_prefix("refs/heads/")
            .map(String::from)
            .ok_or_else(|| CliError::fatal("You are not currently on a branch."))?,
        RefTarget::Direct(_) => {
            return Err(CliError::fatal(
                "You are not currently on a branch.\nTo push the history leading to the current (detached HEAD)\nstate now, use\n\n    git push <remote> HEAD:<name-of-remote-branch>\n",
            ));
        }
    };
    let mode = cfg.get("push.default").unwrap_or_else(|| "simple".into());
    let up_remote = cfg.get(&format!("branch.{branch}.remote"));
    let up_merge = cfg.get(&format!("branch.{branch}.merge"));
    // A branch without `branch.<b>.remote` belongs to `origin` (non-triangular).
    let same_remote = up_remote.as_deref().unwrap_or("origin") == remote;
    match mode.as_str() {
        "nothing" => Err(CliError::fatal(
            "You didn't specify any refspecs to push, and push.default is \"nothing\".",
        )),
        "upstream" | "tracking" | "simple" if named && (same_remote || mode != "simple") => {
            match up_merge {
                Some(m) if same_remote => {
                    if mode == "simple" && m != format!("refs/heads/{branch}") {
                        return Err(CliError::fatal(format!(
                            "The upstream branch of your current branch does not match\nthe name of your current branch.  To push to the upstream branch\non the remote, use\n\n    git push {remote} HEAD:{}\n\nTo push to the branch of the same name on the remote, use\n\n    git push {remote} HEAD\n",
                            m.trim_start_matches("refs/heads/")
                        )));
                    }
                    Ok(format!("refs/heads/{branch}:{m}"))
                }
                _ => Err(CliError::fatal(format!(
                    "The current branch {branch} has no upstream branch.\nTo push the current branch and set the remote as upstream, use\n\n    git push --set-upstream {remote} {branch}\n\nTo have this happen automatically for branches without a tracking\nupstream, see 'push.autoSetupRemote' in 'git help config'.\n"
                ))),
            }
        }
        "matching" => Ok(":".into()),
        _ => Ok(format!("refs/heads/{branch}:refs/heads/{branch}")),
    }
}

/// Resolve a push source: a local ref (DWIM) or any revision.
fn resolve_src(
    repo: &Repository,
    local_refs: &[(String, Oid)],
    src: &str,
) -> CliResult<(String, Oid)> {
    if src == "HEAD" || src == "@" {
        let oid = repo.head_oid()?;
        return Ok(match repo.head_target()? {
            RefTarget::Symbolic(s) => (s, oid),
            RefTarget::Direct(_) => ("HEAD".into(), oid),
        });
    }
    if let Some((n, o)) = dwim(src, local_refs) {
        return Ok((n.clone(), *o));
    }
    match resolve_rev(repo, src) {
        Ok(oid) => Ok((src.to_string(), oid)),
        Err(_) => Err(CliError::Exit(
            1,
            format!("error: src refspec {src} does not match any"),
        )),
    }
}

/// Qualify a push destination: full names pass, else match the remote's
/// refs, else follow the source's namespace (git-push `<dst>` DWIM).
fn qualify_remote(dst: &str, src_full: Option<&str>, remote: &[(String, Oid)]) -> Option<String> {
    if dst.starts_with("refs/") {
        return Some(dst.to_string());
    }
    for p in ["refs/heads/", "refs/tags/"] {
        let c = format!("{p}{dst}");
        if remote.iter().any(|(n, _)| *n == c) {
            return Some(c);
        }
    }
    match src_full {
        Some(s) if s.starts_with("refs/tags/") => Some(format!("refs/tags/{dst}")),
        Some(s) if s.starts_with("refs/heads/") || s == "HEAD" => Some(format!("refs/heads/{dst}")),
        Some(_) => None,
        None => None,
    }
}

/// `--follow-tags`: annotated tags pointing into pushed history the remote lacks.
fn follow_tags(
    repo: &Repository,
    local_refs: &[(String, Oid)],
    remote_refs: &[(String, Oid)],
    updates: &mut Vec<LocalPush>,
) -> CliResult<()> {
    let mut reach = std::collections::HashSet::new();
    for u in updates.iter() {
        if let Some(n) = u.new {
            reach.extend(crate::cmd::shared::is_ancestor_set(repo, n).unwrap_or_default());
        }
    }
    for (n, oid) in local_refs
        .iter()
        .filter(|(n, _)| n.starts_with("refs/tags/"))
    {
        if remote_refs.iter().any(|(r, _)| r == n) || updates.iter().any(|u| u.dst == *n) {
            continue;
        }
        let Ok(obj) = repo.get_object(oid) else {
            continue;
        };
        if obj.kind != arks_object::ObjectKind::Tag {
            continue;
        }
        let target = crate::cmd::remote_local::peel(repo, oid);
        if reach.contains(&target) {
            updates.push(lp(n, n, Some(*oid), false));
        }
    }
    Ok(())
}

/// githooks `pre-push`: args `<remote> <url>`, one stdin line per update.
fn run_pre_push(
    repo: &Repository,
    name: &str,
    url: &str,
    updates: &[LocalPush],
    remote_refs: &[(String, Oid)],
) -> CliResult<()> {
    let hook = repo.git_dir().join("hooks/pre-push");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        match std::fs::metadata(&hook) {
            Ok(m) if m.is_file() && m.permissions().mode() & 0o111 != 0 => {}
            _ => return Ok(()),
        }
    }
    #[cfg(not(unix))]
    if !hook.is_file() {
        return Ok(());
    }
    let zero = "0".repeat(40);
    let mut stdin = String::new();
    for u in updates {
        let remote = remote_refs
            .iter()
            .find(|(n, _)| *n == u.dst)
            .map(|(_, o)| o.to_hex())
            .unwrap_or_else(|| zero.clone());
        let (lref, loid) = match u.new {
            Some(n) => (u.src_label.clone(), n.to_hex()),
            None => ("(delete)".into(), zero.clone()),
        };
        stdin.push_str(&format!("{lref} {loid} {} {remote}\n", u.dst));
    }
    let mut child = std::process::Command::new(&hook)
        .arg(name)
        .arg(url)
        .env("GIT_DIR", repo.git_dir())
        .stdin(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| CliError::Msg(format!("pre-push: {e}")))?;
    if let Some(mut s) = child.stdin.take() {
        let _ = s.write_all(stdin.as_bytes());
    }
    let st = child.wait()?;
    if !st.success() {
        return Err(CliError::Exit(
            1,
            format!("error: failed to push some refs to '{url}'"),
        ));
    }
    Ok(())
}

/// Network push through `arks_remote` (claimed client).
fn push_network(
    repo: &Repository,
    url: &str,
    updates: &[LocalPush],
    push_options: &[String],
    dry_run: bool,
) -> CliResult<Vec<PushStatus>> {
    let statuses: Vec<PushStatus> = updates
        .iter()
        .map(|u| {
            if u.new.is_some() {
                PushStatus::New
            } else {
                PushStatus::Deleted
            }
        })
        .collect();
    if dry_run {
        eprintln!("To {url}");
        for u in updates {
            eprintln!(" * {} -> {} (dry run)", u.src_label, u.dst);
        }
        return Ok(statuses);
    }
    let zero = Oid::from_hex(&"0".repeat(40)).map_err(|e| CliError::Msg(e.to_string()))?;
    let list: Vec<arks_remote::PushUpdate> = updates
        .iter()
        .map(|u| arks_remote::PushUpdate {
            refname: u.dst.clone(),
            new_oid: u.new.unwrap_or(zero),
            expected_old: u.expect.flatten(),
        })
        .collect();
    arks_remote::push_remote_with(
        repo,
        url,
        &list,
        &arks_remote::PushOptions {
            push_options: push_options.to_vec(),
        },
    )?;
    Ok(statuses)
}
