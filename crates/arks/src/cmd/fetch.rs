//! `fetch`: option table, remote / group resolution and dispatch to the local
//! transport (`remote_local`) or the network client (`arks_remote`).
//!
//! Primary: <https://git-scm.com/docs/git-fetch>,
//! <https://git-scm.com/docs/fetch-options>.

use crate::cmd::opt_spec::{Parsed, parse};
use crate::cmd::remote_local::{FetchSource, LocalFetchOpts, fetch_local};
use crate::error::{CliError, CliResult};
use crate::util::open_repo;
use std::io::Read;

/// Every option `git fetch -h` lists (pin 2.39.5 plus host 2.43 additions).
pub(crate) const FETCH_OPTS: &[&str] = &[
    "v|verbose",
    "q|quiet",
    "|all",
    "|set-upstream",
    "a|append",
    "|atomic",
    "|upload-pack=",
    "f|force",
    "m|multiple",
    "t|tags",
    "n|",
    "j|jobs=",
    "|prefetch",
    "p|prune",
    "P|prune-tags",
    "|recurse-submodules=?",
    "|dry-run",
    "|porcelain",
    "|write-fetch-head",
    "k|keep",
    "u|update-head-ok",
    "|progress",
    "|depth=",
    "|shallow-since=",
    "|shallow-exclude=",
    "|deepen=",
    "|unshallow",
    "|refetch",
    "|update-shallow",
    "|refmap=",
    "o|server-option=",
    "4|ipv4",
    "6|ipv6",
    "|negotiation-tip=",
    "|negotiate-only",
    "|filter=",
    "|auto-maintenance",
    "|auto-gc",
    "|show-forced-updates",
    "|write-commit-graph",
    "|stdin",
    "|recurse-submodules-default=",
    "|submodule-prefix=",
];

/// Configured remote `name` (url, fetch refspecs, tagOpt) from the config set.
pub(crate) fn configured_source(cfg: &arks_config::ConfigSet, name: &str) -> Option<FetchSource> {
    let url = cfg
        .get_all(&format!("remote.{name}.url"))
        .into_iter()
        .next()?;
    Some(FetchSource {
        name: Some(name.to_string()),
        url,
        config_specs: cfg.get_all(&format!("remote.{name}.fetch")),
        tag_opt: cfg.get(&format!("remote.{name}.tagopt")),
    })
}

/// All configured remote names, in config order.
pub(crate) fn remote_names(cfg: &arks_config::ConfigSet) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for e in cfg.entries() {
        let Some(rest) = e.key.strip_prefix("remote.") else {
            continue;
        };
        let Some(name) = rest.strip_suffix(".url") else {
            continue;
        };
        if !out.iter().any(|n| n == name) {
            out.push(name.to_string());
        }
    }
    out
}

/// Expand a remote group (`remotes.<group>`) or return the name itself.
fn expand_group(cfg: &arks_config::ConfigSet, name: &str) -> Vec<String> {
    let group = cfg.get_all(&format!("remotes.{name}"));
    if group.is_empty() {
        vec![name.to_string()]
    } else {
        group
            .iter()
            .flat_map(|g| g.split_whitespace().map(str::to_string).collect::<Vec<_>>())
            .collect()
    }
}

pub fn cmd_fetch(args: &[String]) -> CliResult<()> {
    let p = parse(args, FETCH_OPTS)?;
    fetch_parsed(&p)
}

pub(crate) fn fetch_parsed(p: &Parsed) -> CliResult<()> {
    let repo = open_repo()?;
    let cfg = arks_config::ConfigSet::load(Some(repo.common_dir()));
    let shallow = repo.common_dir().join("shallow");
    // SHAL-001 (M20 Cx-4) / CORR-005 (M20 Cx-6): plant shallow.lock → fetch/deepen refuse.
    // Cite: https://git-scm.com/docs/gitrepository-layout (shallow)
    let shallow_lock = repo.common_dir().join("shallow.lock");
    if shallow_lock.is_file() {
        return Err(CliError::Msg(format!(
            "Unable to create '{}' (SHAL-001 / CORR-005 — concurrent shallow rewrite / deepen refuse; lease ≠ skip)",
            shallow_lock.display()
        )));
    }
    // SHAL-004 (M20 Cx-7): shallow + grafts while peers fetch/unshallow.
    let grafts = repo.git_dir().join("info/grafts");
    let grafts_common = repo.common_dir().join("info/grafts");
    if shallow.is_file() && (grafts.is_file() || grafts_common.is_file()) {
        return Err(CliError::Msg(
            "fetch refused (SHAL-004 — shallow + grafts interaction; serialize unshallow vs shallow fetch peers)".into(),
        ));
    }
    if p.on("unshallow") && !shallow.is_file() {
        return Err(CliError::fatal(
            "--unshallow on a complete repository does not make sense",
        ));
    }
    if p.on("negotiate-only") && p.values("negotiation-tip").is_empty() {
        return Err(CliError::fatal(
            "--negotiate-only needs one or more --negotiation-tip=*",
        ));
    }
    if p.on("porcelain") && p.on("recurse-submodules") {
        return Err(CliError::fatal(
            "options '--porcelain' and '--recurse-submodules' cannot be used together",
        ));
    }
    let depth = p
        .value("depth")
        .map(|d| crate::cmd::remote::parse_depth(&d))
        .transpose()?;
    if let Some(d) = p.value("deepen") {
        crate::cmd::remote::parse_depth(&d)?;
    }
    let filter = p
        .value("filter")
        .map(|f| crate::cmd::remote::parse_filter(&f))
        .transpose()?;
    let mut positional = p.args.clone();
    if p.on("stdin") {
        let mut s = String::new();
        std::io::stdin().read_to_string(&mut s)?;
        positional.extend(
            s.lines()
                .map(str::trim)
                .filter(|l| !l.is_empty())
                .map(String::from),
        );
    }

    // Which remotes, and which command-line refspecs.
    let (targets, refspecs): (Vec<String>, Vec<String>) = if p.on("all") {
        if !positional.is_empty() {
            return Err(CliError::fatal(
                "fetch --all does not take a repository argument",
            ));
        }
        (remote_names(&cfg), Vec::new())
    } else if p.on("multiple") {
        (
            positional
                .iter()
                .flat_map(|n| expand_group(&cfg, n))
                .collect(),
            Vec::new(),
        )
    } else {
        let first = positional.first().cloned();
        let name = match first {
            Some(n) => n,
            None => default_remote(&repo, &cfg),
        };
        let group = expand_group(&cfg, &name);
        if group.len() > 1 || (group.len() == 1 && group[0] != name) {
            if positional.len() > 1 {
                return Err(CliError::fatal("fetch --multiple does not take refspecs"));
            }
            (group, Vec::new())
        } else {
            (vec![name], positional.iter().skip(1).cloned().collect())
        }
    };

    if p.on("negotiate-only") {
        return negotiate_only(&cfg, &targets, &p.values("negotiation-tip"));
    }

    let tags = if p.on("n") {
        Some(false)
    } else {
        p.flag("tags")
    };
    let multi = p.on("multiple") || p.on("all") || targets.len() > 1;
    if multi {
        if let Some(bad) = targets
            .iter()
            .find(|t| configured_source(&cfg, t).is_none())
        {
            return Err(CliError::fatal(format!(
                "no such remote or remote group: {bad}"
            )));
        }
    }
    let mut first = true;
    let mut failed = false;
    for target in &targets {
        let multi = p.on("multiple") || p.on("all") || targets.len() > 1;
        let source = match configured_source(&cfg, target) {
            Some(s) => s,
            None if multi => {
                return Err(CliError::fatal(format!(
                    "no such remote or remote group: {target}"
                )));
            }
            None if arks_remote::is_local_remote(target)
                || arks_remote::HttpRemoteUrl::parse(target).is_ok()
                || arks_remote::SshRemoteUrl::parse(target).is_ok()
                || arks_remote::GitDaemonUrl::parse(target).is_ok() =>
            {
                FetchSource {
                    name: None,
                    url: target.clone(),
                    config_specs: Vec::new(),
                    tag_opt: None,
                }
            }
            None => {
                return Err(CliError::fatal(format!(
                    "'{target}' does not appear to be a git repository"
                )));
            }
        };
        let rname = source.name.clone().unwrap_or_default();
        let prune = p.flag("prune").unwrap_or_else(|| {
            cfg.get_bool(&format!("remote.{rname}.prune"))
                .or_else(|| cfg.get_bool("fetch.prune"))
                .unwrap_or(false)
        });
        let prune_tags = p.flag("prune-tags").unwrap_or_else(|| {
            cfg.get_bool(&format!("remote.{rname}.prunetags"))
                .or_else(|| cfg.get_bool("fetch.prunetags"))
                .unwrap_or(false)
        }) && (prune || p.on("prune-tags"));
        let opts = LocalFetchOpts {
            verbose: p.count("verbose") > 0,
            quiet: p.on("quiet"),
            porcelain: p.on("porcelain"),
            dry_run: p.on("dry-run"),
            append: p.on("append") || !first,
            write_fetch_head: p
                .flag("write-fetch-head")
                .unwrap_or_else(|| cfg.get_bool("fetch.writefetchhead").unwrap_or(true)),
            force: p.on("force"),
            tags,
            prune,
            prune_tags,
            update_head_ok: p.on("update-head-ok"),
            atomic: p.on("atomic"),
            refmap: p.seen("refmap").then(|| p.values("refmap")),
            prefetch: p.on("prefetch"),
            set_upstream: p.on("set-upstream"),
        };
        // `fetch --multiple`, groups and `--all` over several remotes announce
        // each remote on stdout.
        if (p.on("multiple") || targets.len() > 1) && !opts.quiet && !opts.porcelain {
            println!("Fetching {target}");
        }
        first = false;
        let res = if arks_remote::is_local_remote(&source.url) {
            if p.on("unshallow") {
                unshallow_local(&source.url)?;
            }
            fetch_local(&source, &refspecs, &opts)
        } else {
            fetch_network(&source, &refspecs, depth, filter.clone(), &opts)
        };
        match res {
            Ok(()) => {}
            Err(CliError::Exit(1, m)) if m.is_empty() => failed = true,
            Err(e) => return Err(e),
        }
    }
    if failed {
        return Err(CliError::silent(1));
    }
    Ok(())
}

/// Remote of the current branch (`branch.<b>.remote`), else `origin`.
pub(crate) fn default_remote(repo: &arks_repo::Repository, cfg: &arks_config::ConfigSet) -> String {
    if let Ok(arks_refs::RefTarget::Symbolic(s)) = repo.head_target() {
        if let Some(b) = s.strip_prefix("refs/heads/") {
            if let Some(r) = cfg.get(&format!("branch.{b}.remote")) {
                return r;
            }
        }
    }
    "origin".into()
}

/// `--unshallow` from a local peer: copy the history behind every shallow
/// boundary, then drop `.git/shallow`.
fn unshallow_local(url: &str) -> CliResult<()> {
    let src = arks_remote::open_local_remote(url).map_err(|e| CliError::fatal(e.to_string()))?;
    let mut repo = open_repo()?;
    let shallow = repo.common_dir().join("shallow");
    let text = std::fs::read_to_string(&shallow).unwrap_or_default();
    let mut tips = Vec::new();
    for l in text.lines() {
        let Ok(oid) = arks_oid::Oid::from_hex(l.trim()) else {
            continue;
        };
        if let Ok(obj) = src.get_object(&oid) {
            if let Ok(c) = arks_object::Commit::decode(&obj.payload) {
                tips.extend(c.parents);
            }
        }
    }
    arks_remote::copy_objects(&src, &mut repo, &tips).map_err(|e| CliError::Msg(e.to_string()))?;
    let _ = std::fs::remove_file(shallow);
    Ok(())
}

/// `--negotiate-only`: print the negotiation tips' commits the peer has.
fn negotiate_only(
    cfg: &arks_config::ConfigSet,
    targets: &[String],
    tips: &[String],
) -> CliResult<()> {
    let repo = open_repo()?;
    let url = targets
        .first()
        .map(|t| {
            configured_source(cfg, t)
                .map(|s| s.url)
                .unwrap_or_else(|| t.clone())
        })
        .unwrap_or_default();
    if !arks_remote::is_local_remote(&url) {
        return Err(CliError::NotImplemented(
            "fetch --negotiate-only is claimed for local-path remotes only".into(),
        ));
    }
    let src = arks_remote::open_local_remote(&url).map_err(|e| CliError::fatal(e.to_string()))?;
    // Every commit behind the tips that the peer also has, newest first.
    let mut seen = std::collections::HashSet::new();
    let mut queue: Vec<(i64, arks_oid::Oid)> = Vec::new();
    let date = |o: &arks_oid::Oid| -> i64 {
        repo.get_object(o)
            .ok()
            .and_then(|obj| arks_object::Commit::decode(&obj.payload).ok())
            .and_then(|c| c.committer.timestamp.to_string().parse().ok())
            .unwrap_or(0)
    };
    for t in tips {
        let oid = crate::cmd::shared::resolve_rev(&repo, t)?;
        if seen.insert(oid) {
            queue.push((date(&oid), oid));
        }
    }
    let mut common = Vec::new();
    while !queue.is_empty() {
        queue.sort_by_key(|(d, _)| *d);
        let (_, c) = queue.pop().expect("non-empty");
        if src.get_object(&c).is_ok() {
            common.push(c);
        }
        if let Ok(obj) = repo.get_object(&c) {
            if let Ok(commit) = arks_object::Commit::decode(&obj.payload) {
                for par in commit.parents {
                    if seen.insert(par) {
                        queue.push((date(&par), par));
                    }
                }
            }
        }
    }
    for c in oidset_order(&common) {
        println!("{}", c.to_hex());
    }
    Ok(())
}

/// Order in which stock git lists an oid set (an open-addressing hash table
/// keyed by the first four oid bytes, quadratic probing, power-of-two growth
/// at a 0.77 load factor), simulated for insertion order `items`.
fn oidset_order(items: &[arks_oid::Oid]) -> Vec<arks_oid::Oid> {
    fn place(table: &mut [Option<arks_oid::Oid>], o: arks_oid::Oid) {
        let b = o.as_bytes();
        let h = u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize;
        let mask = table.len() - 1;
        let (mut i, mut step) = (h & mask, 0);
        while table[i].is_some() {
            step += 1;
            i = (i + step) & mask;
        }
        table[i] = Some(o);
    }
    let mut table: Vec<Option<arks_oid::Oid>> = Vec::new();
    let mut upper = 0usize;
    for (n, o) in items.iter().enumerate() {
        if n >= upper {
            let size = (table.len() + 1).next_power_of_two().max(4);
            let old: Vec<arks_oid::Oid> = table.iter().flatten().copied().collect();
            table = vec![None; size];
            upper = (size as f64 * 0.77 + 0.5) as usize;
            for x in old {
                place(&mut table, x);
            }
        }
        place(&mut table, *o);
    }
    table.into_iter().flatten().collect()
}

/// Network fetch through `arks_remote` (http(s), ssh, git://): remote-tracking
/// refs from the claimed client, then `FETCH_HEAD`.
fn fetch_network(
    source: &FetchSource,
    refspecs: &[String],
    depth: Option<u32>,
    filter: Option<String>,
    o: &LocalFetchOpts,
) -> CliResult<()> {
    let name = source.name.clone().ok_or_else(|| {
        CliError::NotImplemented(
            "fetch from a bare network URL without a configured remote is not claimed".into(),
        )
    })?;
    let mut repo = open_repo()?;
    if o.dry_run {
        return Err(CliError::NotImplemented(
            "fetch --dry-run is claimed for local-path remotes only".into(),
        ));
    }
    let opts = arks_remote::FetchOptions {
        depth,
        filter,
        ..Default::default()
    };
    let result = arks_remote::fetch_remote_with(&mut repo, &source.url, &name, &opts)?;
    for (n, oid) in result.updated {
        if o.porcelain {
            println!("* {} {} {n}", "0".repeat(oid.to_hex().len()), oid.to_hex());
        } else if !o.quiet {
            println!("{n}: {}", oid.to_hex());
        }
    }
    if o.write_fetch_head {
        crate::cmd::remote::write_fetch_head(&repo, &name, &source.url, refspecs)?;
    }
    Ok(())
}
