//! `fetch-pack` and `send-pack`: the transport plumbing behind fetch and
//! push. Local peers go through the in-process object copy and the push
//! engine in `remote_local`; network peers through `arks_remote`.
//!
//! Primary: <https://git-scm.com/docs/git-fetch-pack>,
//! <https://git-scm.com/docs/git-send-pack>.

use crate::cmd::opt_spec::parse;
use crate::cmd::remote_local::{
    LocalPush, LocalPushOpts, PushStatus, advertised_refs, dwim, parse_refspec, push_local,
};
use crate::cmd::shared::resolve_rev;
use crate::error::{CliError, CliResult};
use crate::util::open_repo;
use std::io::Read;

const FETCH_PACK_OPTS: &[&str] = &[
    "|all",
    "|stdin",
    "q|quiet",
    "k|keep",
    "|thin",
    "|include-tag",
    "|upload-pack=",
    "|exec=",
    "|depth=",
    "|shallow-since=",
    "|shallow-exclude=",
    "|deepen-relative",
    "|refetch",
    "|no-progress",
    "|diag-url",
    "v|verbose",
    "|check-self-contained-and-connected",
    "|cloning",
    "|update-shallow",
    "|lock-pack",
    "|from-promisor",
    "|filter=",
    "|no-dependents",
    "|stateless-rpc",
    "|reject-shallow",
];

const FETCH_PACK_USAGE: &str = "usage: git fetch-pack [--all] [--stdin] [--quiet | -q] [--keep | -k] [--thin] [--include-tag] [--upload-pack=<git-upload-pack>] [--depth=<n>] [--no-progress] [--diag-url] [-v] [<host>:]<directory> [<refs>...]";

/// `--diag-url`: how the URL would be dialed (protocol, host, path).
fn diag_url(url: &str) {
    println!("Diag: url={url}");
    if let Ok(h) = arks_remote::SshRemoteUrl::parse(url) {
        println!("Diag: protocol=ssh");
        println!("Diag: userandhost={}", h.host);
        println!("Diag: port=NONE");
        println!("Diag: path={}", h.path);
    } else if let Some((scheme, rest)) = url.split_once("://") {
        let (host, path) = rest.split_once('/').unwrap_or((rest, ""));
        println!("Diag: protocol={scheme}");
        println!("Diag: hostandport={host}");
        println!("Diag: path=/{path}");
    } else {
        println!("Diag: protocol=file");
        println!("Diag: hostandport=");
        println!("Diag: path={url}");
    }
}

pub fn cmd_fetch_pack(args: &[String]) -> CliResult<()> {
    let p = parse(args, FETCH_PACK_OPTS)?;
    let Some(url) = p.args.first() else {
        return Err(CliError::Exit(129, FETCH_PACK_USAGE.into()));
    };
    if p.on("diag-url") {
        diag_url(url);
        return Ok(());
    }
    let mut wanted: Vec<String> = p.args[1..].to_vec();
    if p.on("stdin") {
        let mut s = String::new();
        std::io::stdin().read_to_string(&mut s)?;
        wanted.extend(s.split_whitespace().map(String::from));
    }
    let depth = p
        .value("depth")
        .map(|d| crate::cmd::remote::parse_depth(&d))
        .transpose()?;
    let mut repo = open_repo()?;
    if !arks_remote::is_local_remote(url) {
        let opts = arks_remote::FetchOptions {
            depth,
            ..Default::default()
        };
        let out = arks_remote::fetch_remote_with(&mut repo, url, "fetch-pack", &opts)
            .map_err(|e| CliError::Msg(e.to_string()))?;
        for (name, oid) in out.updated {
            println!("{} {}", oid.to_hex(), name);
        }
        return Ok(());
    }
    let peer = arks_remote::open_local_remote(url).map_err(|e| CliError::fatal(e.to_string()))?;
    let refs = advertised_refs(&peer)?;
    let mut hits: Vec<(String, arks_oid::Oid)> = Vec::new();
    let mut missing = false;
    if p.on("all") {
        hits = refs.clone();
    }
    for w in &wanted {
        match refs.iter().find(|(n, _)| n == w) {
            Some(r) => {
                if !hits.iter().any(|(n, _)| n == w) {
                    hits.push(r.clone());
                }
            }
            None => {
                eprintln!("error: no such remote ref {w}");
                missing = true;
            }
        }
    }
    if hits.is_empty() && !p.on("all") {
        return Err(CliError::silent(1));
    }
    if p.on("include-tag") {
        // Tags pointing at what we fetch come along (objects only).
        let tips: Vec<_> = hits.iter().map(|h| h.1).collect();
        for (n, oid) in refs.iter().filter(|(n, _)| n.starts_with("refs/tags/")) {
            if tips.contains(&crate::cmd::remote_local::peel(&peer, oid)) {
                let _ = n;
                arks_remote::copy_objects(&peer, &mut repo, &[*oid])
                    .map_err(|e| CliError::Msg(e.to_string()))?;
            }
        }
    }
    let tips: Vec<_> = hits.iter().map(|h| h.1).collect();
    if p.count("verbose") > 0 {
        for (n, o) in &hits {
            eprintln!("want {} ({n})", o.to_hex());
        }
    }
    arks_remote::copy_objects(&peer, &mut repo, &tips).map_err(|e| CliError::Msg(e.to_string()))?;
    for (n, o) in &hits {
        println!("{} {n}", o.to_hex());
    }
    if missing {
        return Err(CliError::silent(1));
    }
    Ok(())
}

const SEND_PACK_OPTS: &[&str] = &[
    "v|verbose",
    "q|quiet",
    "|receive-pack=",
    "|exec=",
    "|remote=",
    "|all",
    "n|dry-run",
    "|mirror",
    "f|force",
    "|signed=?",
    "|push-option=",
    "|progress",
    "|thin",
    "|atomic",
    "|stateless-rpc",
    "|stdin",
    "|helper-status",
    "|force-with-lease=?",
    "|force-if-includes",
];

pub fn cmd_send_pack(args: &[String]) -> CliResult<()> {
    let p = parse(args, SEND_PACK_OPTS)?;
    let Some(url) = p.args.first().cloned() else {
        return Err(CliError::Exit(
            129,
            "usage: git send-pack [--mirror] [--dry-run] [--force]\n              [--receive-pack=<git-receive-pack>]\n              [--verbose] [--thin] [--atomic]\n              [--[no-]signed | --signed=(true|false|if-asked)]\n              [<host>:]<directory> (--all | <ref>...)".into(),
        ));
    };
    let mut specs: Vec<String> = p.args[1..].to_vec();
    if p.on("stdin") {
        let mut s = String::new();
        std::io::stdin().read_to_string(&mut s)?;
        specs.extend(
            s.lines()
                .map(str::trim)
                .filter(|l| !l.is_empty())
                .map(String::from),
        );
    }
    let repo = open_repo()?;
    if !arks_remote::is_local_remote(&url) {
        let mut list = Vec::new();
        for raw in &specs {
            let spec = parse_refspec(raw);
            let oid = resolve_rev(&repo, &spec.src)?;
            let dst = spec.dst.clone().unwrap_or_else(|| spec.src.clone());
            let full = if dst.starts_with("refs/") {
                dst
            } else {
                format!("refs/heads/{dst}")
            };
            list.push(arks_remote::PushUpdate {
                refname: full,
                new_oid: oid,
                expected_old: None,
            });
        }
        arks_remote::push_remote_with(
            &repo,
            &url,
            &list,
            &arks_remote::PushOptions {
                push_options: p.values("push-option"),
            },
        )
        .map_err(|e| CliError::Msg(e.to_string()))?;
        for u in &list {
            println!("ok {}", u.refname);
        }
        return Ok(());
    }
    if !p.values("push-option").is_empty() {
        return Err(CliError::fatal(
            "the receiving end does not support push options\nfatal: the remote end hung up unexpectedly",
        ));
    }
    if matches!(p.value("signed").as_deref(), Some("yes" | "true"))
        || (p.on("signed") && p.value("signed").is_none())
    {
        return Err(CliError::fatal(
            "the receiving end does not support --signed push\nfatal: the remote end hung up unexpectedly",
        ));
    }
    let peer = arks_remote::open_local_remote(&url).map_err(|e| CliError::fatal(e.to_string()))?;
    let remote_refs = peer.refs().list_refs("refs/")?;
    let local_refs = repo.refs().list_refs("refs/")?;
    let force = p.on("force");
    let mut updates: Vec<LocalPush> = Vec::new();
    let push = |list: &mut Vec<LocalPush>, src: &str, dst: &str, new, force| {
        if !list.iter().any(|u: &LocalPush| u.dst == dst) {
            list.push(LocalPush {
                src_label: src.to_string(),
                dst: dst.to_string(),
                new,
                force,
                expect: None,
            });
        }
    };
    if p.on("mirror") {
        for (n, o) in local_refs.iter().filter(|(n, _)| n != "refs/stash") {
            push(&mut updates, n, n, Some(*o), true);
        }
        for (n, _) in &remote_refs {
            if !local_refs.iter().any(|(l, _)| l == n && l != "refs/stash") {
                push(&mut updates, "", n, None, true);
            }
        }
    } else if p.on("all") {
        for (n, o) in local_refs
            .iter()
            .filter(|(n, _)| n.starts_with("refs/heads/"))
        {
            push(&mut updates, n, n, Some(*o), force);
        }
    } else if specs.is_empty() {
        // Matching refs: heads present on both sides.
        for (n, o) in local_refs
            .iter()
            .filter(|(n, _)| n.starts_with("refs/heads/"))
        {
            if remote_refs.iter().any(|(r, _)| r == n) {
                push(&mut updates, n, n, Some(*o), force);
            }
        }
    }
    for raw in &specs {
        let spec = parse_refspec(raw);
        let f = spec.force || force;
        if spec.src.is_empty() {
            let d = spec.dst.unwrap_or_default();
            let full = if d.starts_with("refs/") {
                d
            } else {
                format!("refs/heads/{d}")
            };
            push(&mut updates, "", &full, None, f);
            continue;
        }
        let (src_full, oid) = match dwim(&spec.src, &local_refs) {
            Some((n, o)) => (n.clone(), *o),
            None => (spec.src.clone(), resolve_rev(&repo, &spec.src)?),
        };
        let dst = match spec.dst.as_deref() {
            Some(d) if d.starts_with("refs/") => d.to_string(),
            Some(d) => format!("refs/heads/{d}"),
            None => src_full.clone(),
        };
        let label = crate::cmd::remote_local::short_ref(&src_full).to_string();
        push(&mut updates, &label, &dst, Some(oid), f);
    }
    // --force-with-lease=<ref>:<expect> (a bare lease needs tracking refs).
    for e in p
        .events
        .iter()
        .filter(|e| e.key == "force-with-lease" && !e.neg)
    {
        if let Some((r, x)) = e.value.as_deref().and_then(|v| v.split_once(':')) {
            let full = if r.starts_with("refs/") {
                r.to_string()
            } else {
                format!("refs/heads/{r}")
            };
            let expect = if x.is_empty() {
                None
            } else {
                Some(resolve_rev(&repo, x)?)
            };
            for u in updates.iter_mut().filter(|u| u.dst == full) {
                u.expect = Some(expect);
            }
        }
    }
    if updates.is_empty() {
        eprintln!("Everything up-to-date");
        return Ok(());
    }
    let opts = LocalPushOpts {
        quiet: false,
        verbose: p.count("verbose") > 0,
        porcelain: false,
        dry_run: p.on("dry-run"),
        atomic: p.on("atomic"),
    };
    drop(repo);
    let statuses = push_local(&url, &updates, &remote_refs, &opts)?;
    if p.on("helper-status") {
        // One line per advertised ref (unmatched ones "no match"), new refs after.
        let mut lines = Vec::new();
        for (n, _) in &remote_refs {
            match updates.iter().position(|u| &u.dst == n) {
                Some(i) => lines.push(helper_line(n, &statuses[i])),
                None => lines.push(format!("error {n} no match")),
            }
        }
        for (u, s) in updates.iter().zip(&statuses) {
            if !remote_refs.iter().any(|(n, _)| *n == u.dst) {
                lines.push(helper_line(&u.dst, s));
            }
        }
        for l in lines {
            println!("{l}");
        }
    }
    if statuses
        .iter()
        .any(|s| matches!(s, PushStatus::Rejected(_) | PushStatus::RemoteRejected(_)))
    {
        return Err(CliError::silent(255));
    }
    Ok(())
}

fn helper_line(name: &str, s: &PushStatus) -> String {
    match s {
        PushStatus::Rejected("non-fast-forward") => format!("error {name} non-fast forward"),
        PushStatus::Rejected(why) | PushStatus::RemoteRejected(why) => {
            format!("error {name} {why}")
        }
        PushStatus::UpToDate => format!("ok {name} up to date"),
        _ => format!("ok {name}"),
    }
}
