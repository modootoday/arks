//! `pull`: fetch (every fetch option passes through), pick the merge heads
//! from `FETCH_HEAD`, then merge (`merge` options pass through to
//! `cmd_merge`) or rebase onto them.
//!
//! Primary: <https://git-scm.com/docs/git-pull>,
//! <https://git-scm.com/docs/git-fmt-merge-msg> (merge message shape).

use crate::cmd::opt_spec::{OptEvent, parse};
use crate::cmd::shared::is_ancestor;
use crate::error::{CliError, CliResult};
use crate::util::open_repo;
use arks_oid::Oid;
use arks_refs::RefTarget;

/// Every option `git pull -h` lists (pin 2.39.5 plus host 2.43 additions).
const PULL_OPTS: &[&str] = &[
    "v|verbose",
    "q|quiet",
    "|progress",
    "|recurse-submodules=?",
    "r|rebase=?",
    "n|",
    "|stat",
    "|summary",
    "|log=?",
    "|signoff=?",
    "|squash",
    "|commit",
    "|edit",
    "|cleanup=",
    "|ff",
    "|ff-only",
    "|verify",
    "|verify-signatures",
    "|autostash",
    "s|strategy=",
    "X|strategy-option=",
    "S|gpg-sign=?",
    "|allow-unrelated-histories",
    "|all",
    "a|append",
    "|upload-pack=",
    "f|force",
    "t|tags",
    "p|prune",
    "j|jobs=?",
    "|dry-run",
    "k|keep",
    "|depth=",
    "|shallow-since=",
    "|shallow-exclude=",
    "|deepen=",
    "|unshallow",
    "|update-shallow",
    "|refmap=",
    "o|server-option=",
    "4|ipv4",
    "6|ipv6",
    "|negotiation-tip=",
    "|show-forced-updates",
    "|set-upstream",
];

/// Options handed to `fetch` unchanged.
const FETCH_KEYS: &[&str] = &[
    "verbose",
    "quiet",
    "progress",
    "recurse-submodules",
    "all",
    "append",
    "upload-pack",
    "force",
    "tags",
    "prune",
    "jobs",
    "dry-run",
    "keep",
    "depth",
    "shallow-since",
    "shallow-exclude",
    "deepen",
    "unshallow",
    "update-shallow",
    "refmap",
    "server-option",
    "ipv4",
    "ipv6",
    "negotiation-tip",
    "show-forced-updates",
    "set-upstream",
];

/// Options handed to `merge` unchanged.
const MERGE_KEYS: &[&str] = &[
    "squash",
    "commit",
    "strategy",
    "strategy-option",
    "gpg-sign",
    "allow-unrelated-histories",
];

/// Re-spell one parsed occurrence as a long option.
fn respell(e: &OptEvent) -> String {
    let key = if e.key.len() == 1 {
        match e.key {
            "4" => "ipv4",
            "6" => "ipv6",
            k => k,
        }
    } else {
        e.key
    };
    match (&e.value, e.neg) {
        (_, true) => format!("--no-{key}"),
        (Some(v), false) => format!("--{key}={v}"),
        (None, false) => format!("--{key}"),
    }
}

/// One `FETCH_HEAD` line marked for merge.
struct MergeHead {
    oid: Oid,
    desc: String,
}

fn read_merge_heads(repo: &arks_repo::Repository) -> Vec<MergeHead> {
    let text = std::fs::read_to_string(repo.git_dir().join("FETCH_HEAD")).unwrap_or_default();
    text.lines()
        .filter_map(|l| {
            let mut parts = l.splitn(3, '\t');
            let oid = Oid::from_hex(parts.next()?).ok()?;
            let flag = parts.next()?;
            let desc = parts.next().unwrap_or("").to_string();
            flag.is_empty().then_some(MergeHead { oid, desc })
        })
        .collect()
}

/// fmt-merge-msg title for the merge heads (`Merge branch 'x' of <url>`).
fn merge_title(heads: &[MergeHead]) -> String {
    let parts: Vec<String> = heads
        .iter()
        .map(|h| {
            // `branch 'main' of URL` / `tag 'v1' of URL` / `URL`.
            let (what, src) = match h.desc.split_once(" of ") {
                Some((w, s)) => (w.to_string(), s.to_string()),
                None => (String::new(), h.desc.clone()),
            };
            if src == "." {
                what
            } else if what.is_empty() {
                src
            } else {
                format!("{what} of {src}")
            }
        })
        .collect();
    format!("Merge {}", parts.join(", "))
}

pub fn cmd_pull(args: &[String]) -> CliResult<()> {
    let p = parse(args, PULL_OPTS)?;
    let repo = open_repo()?;
    let cfg = arks_config::ConfigSet::load(Some(repo.common_dir()));
    let rebase_mode = match p.events.iter().rev().find(|e| e.key == "rebase") {
        Some(e) if e.neg => Some("false".to_string()),
        Some(e) => Some(e.value.clone().unwrap_or_else(|| "true".into())),
        None => None,
    };
    if let Some(m) = &rebase_mode {
        if !matches!(
            m.as_str(),
            "true" | "false" | "merges" | "interactive" | "i" | "m"
        ) {
            return Err(CliError::fatal(format!(
                "invalid value for '--rebase': '{m}'"
            )));
        }
    }
    let current = match repo.head_target()? {
        RefTarget::Symbolic(s) => s.strip_prefix("refs/heads/").map(String::from),
        RefTarget::Direct(_) => None,
    };
    let rebase = match rebase_mode.as_deref() {
        Some(m) => m != "false",
        None => current
            .as_deref()
            .and_then(|b| cfg.get_bool(&format!("branch.{b}.rebase")))
            .or_else(|| cfg.get_bool("pull.rebase"))
            .unwrap_or(false),
    };
    let ff_flag = p.flag("ff");
    let ff_only =
        p.on("ff-only") || (ff_flag.is_none() && cfg.get("pull.ff").as_deref() == Some("only"));
    let no_ff = ff_flag == Some(false)
        || (ff_flag.is_none()
            && !p.on("ff-only")
            && cfg.get("pull.ff").as_deref() == Some("false"));
    let reconcile_chosen = rebase_mode.is_some()
        || ff_flag.is_some()
        || p.on("ff-only")
        || cfg.get("pull.rebase").is_some()
        || cfg.get("pull.ff").is_some();

    if rebase {
        // git-pull --rebase refuses a dirty tree before fetching.
        refuse_dirty_for_rebase(&repo, p.on("autostash"))?;
    }

    // Fetch.
    let mut fetch_args: Vec<String> = p
        .events
        .iter()
        .filter(|e| FETCH_KEYS.contains(&e.key) || ["4", "6"].contains(&e.key))
        .map(respell)
        .collect();
    fetch_args.push("--update-head-ok".into());
    fetch_args.extend(p.args.iter().cloned());
    crate::cmd::fetch::cmd_fetch(&fetch_args)?;
    if p.on("dry-run") {
        return Ok(());
    }

    let repo = open_repo()?;
    let heads = read_merge_heads(&repo);
    if heads.is_empty() {
        return Err(no_merge_candidates(&p.args, &repo));
    }
    if p.on("verify-signatures") {
        for h in &heads {
            let obj = repo.get_object(&h.oid)?;
            let has_sig = String::from_utf8_lossy(&obj.payload)
                .lines()
                .take_while(|l| !l.is_empty())
                .any(|l| l.starts_with("gpgsig"));
            if !has_sig {
                return Err(CliError::fatal(format!(
                    "Commit {} does not have a GPG signature.",
                    crate::cmd::shared::abbrev_hex(&repo, &h.oid, 7)
                )));
            }
        }
    }
    let head = repo.head_oid().ok();

    if rebase {
        if heads.len() > 1 {
            return Err(CliError::fatal("Cannot rebase onto multiple branches."));
        }
        let onto = heads[0].oid;
        if let Some(h) = head {
            if is_ancestor(&repo, &onto, &h)? {
                println!(
                    "Current branch {} is up to date.",
                    current.as_deref().unwrap_or("HEAD")
                );
                return Ok(());
            }
        }
        drop(repo);
        return crate::cmd::replay::rebase_onto(&onto.to_hex());
    }

    // Divergent history needs an explicit choice (git >= 2.33 default).
    if let (Some(h), [one]) = (head, heads.as_slice()) {
        let ff = is_ancestor(&repo, &h, &one.oid)?;
        let up_to_date = is_ancestor(&repo, &one.oid, &h)?;
        if !ff && !up_to_date && !reconcile_chosen {
            return Err(CliError::Exit(
                128,
                "hint: You have divergent branches and need to specify how to reconcile them.\nhint: You can do so by running one of the following commands sometime before\nhint: your next pull:\nhint: \nhint:   git config pull.rebase false  # merge\nhint:   git config pull.rebase true   # rebase\nhint:   git config pull.ff only       # fast-forward only\nhint: \nhint: You can replace \"git config\" with \"git config --global\" to set a default\nhint: preference for all repositories. You can also pass --rebase, --no-rebase,\nhint: or --ff-only on the command line to override the configured default per\nhint: invocation.\nfatal: Need to specify how to reconcile divergent branches.".into(),
            ));
        }
        if !ff && !up_to_date && ff_only {
            return Err(CliError::fatal("Not possible to fast-forward, aborting."));
        }
    }

    // Merge message: fmt-merge-msg title, optional shortlog, optional sign-off.
    let mut msg = merge_title(&heads);
    let log_n = match p.events.iter().rev().find(|e| e.key == "log") {
        Some(e) if e.neg => None,
        Some(e) => Some(
            e.value
                .as_deref()
                .and_then(|v| v.parse().ok())
                .unwrap_or(20usize),
        ),
        None => cfg.get("merge.log").and_then(|v| {
            v.parse::<usize>()
                .ok()
                .or_else(|| arks_config::parse_bool(&v).and_then(|b| b.then_some(20)))
        }),
    };
    if let (Some(n), Some(h)) = (log_n, head) {
        msg.push_str(&shortlog_block(&repo, &heads, &h, n)?);
    }
    if p.on("signoff") {
        let who = crate::cmd::shared::committer_signature()?;
        msg.push_str(&format!("\n\nSigned-off-by: {} <{}>", who.name, who.email));
    }
    if let Some(mode) = p.value("cleanup") {
        if !matches!(
            mode.as_str(),
            "strip" | "whitespace" | "verbatim" | "scissors" | "default"
        ) {
            return Err(CliError::fatal(format!("Invalid cleanup mode {mode}")));
        }
    }
    let autostash = p
        .flag("autostash")
        .unwrap_or_else(|| cfg.get_bool("merge.autostash").unwrap_or(false));
    let stashed = autostash && worktree_dirty(&repo)?;
    if stashed {
        crate::cmd::stash::cmd_stash(&[
            "push".into(),
            "-q".into(),
            "-m".into(),
            "autostash".into(),
        ])?;
        let stash = open_repo()?.refs().resolve("refs/stash")?;
        println!(
            "Created autostash: {}",
            crate::cmd::shared::abbrev_hex(&repo, &stash, 7)
        );
    }
    let res = merge_heads(&p, &repo, &heads, head, &msg, ff_only, no_ff);
    if stashed {
        crate::cmd::stash::cmd_stash(&["pop".into(), "-q".into()])?;
        eprintln!("Applied autostash.");
    }
    res
}

/// True when the index or a tracked work-tree file differs from `HEAD`.
fn worktree_dirty(repo: &arks_repo::Repository) -> CliResult<bool> {
    Ok(refuse_dirty_for_rebase(repo, false).is_err())
}

/// Fast-forward when possible, else hand the merge to `cmd_merge`.
fn merge_heads(
    p: &crate::cmd::opt_spec::Parsed,
    repo: &arks_repo::Repository,
    heads: &[MergeHead],
    head: Option<Oid>,
    msg: &str,
    ff_only: bool,
    no_ff: bool,
) -> CliResult<()> {
    let msg = msg.to_string();
    let quiet = p.on("quiet");
    let stat =
        !(quiet || p.on("n") || p.flag("stat") == Some(false) || p.flag("summary") == Some(false));
    let passthrough = p
        .events
        .iter()
        .any(|e| MERGE_KEYS.contains(&e.key) && e.key != "edit");
    if let (Some(h), [one], false, false) = (head, heads, no_ff, passthrough) {
        if h == one.oid || is_ancestor(repo, &one.oid, &h)? {
            if !quiet {
                println!("Already up to date.");
            }
            return Ok(());
        }
        if is_ancestor(repo, &h, &one.oid)? {
            let label = format!("pull {}", p.args.join(" "));
            return fast_forward(repo, &h, &one.oid, label.trim_end(), quiet, stat);
        }
    }
    let mut merge_args: Vec<String> = vec!["--no-edit".into()];
    if ff_only {
        merge_args.push("--ff-only".into());
    } else if no_ff {
        merge_args.push("--no-ff".into());
    }
    if p.on("quiet") {
        merge_args.push("-q".into());
    }
    if p.on("n") || p.flag("stat") == Some(false) || p.flag("summary") == Some(false) {
        merge_args.push("--no-stat".into());
    }
    merge_args.extend(
        p.events
            .iter()
            .filter(|e| MERGE_KEYS.contains(&e.key) && !(e.key == "edit"))
            .map(respell),
    );
    merge_args.push("-m".into());
    merge_args.push(msg);
    merge_args.extend(heads.iter().map(|h| h.oid.to_hex()));
    let _ = repo;
    crate::cmd::merge::cmd_merge(&merge_args)
}

/// Fast-forward the current branch (index and work tree carried like a
/// two-way checkout), then git-merge's `Updating` / `Fast-forward` report.
fn fast_forward(
    repo: &arks_repo::Repository,
    head: &Oid,
    theirs: &Oid,
    label: &str,
    quiet: bool,
    stat: bool,
) -> CliResult<()> {
    let mut repo_mut = open_repo()?;
    let work = crate::util::work_tree(repo)?;
    crate::cmd::switch_tree::two_way_switch(repo, &work, Some(head), theirs)
        .map_err(|e| CliError::Msg(e.to_string().replace("checkout", "merge")))?;
    match repo.head_target()? {
        RefTarget::Symbolic(branch) => {
            repo_mut.refs_mut().update(&branch, theirs, Some(head))?;
            let _ = crate::reflog::append_head_and_branch(
                repo,
                &branch,
                Some(head),
                theirs,
                &format!("{label}: Fast-forward"),
            );
        }
        RefTarget::Direct(_) => repo_mut.refs_mut().update("HEAD", theirs, Some(head))?,
    }
    if quiet {
        return Ok(());
    }
    println!(
        "Updating {}..{}",
        crate::cmd::shared::abbrev_hex(repo, head, 7),
        crate::cmd::shared::abbrev_hex(repo, theirs, 7)
    );
    println!("Fast-forward");
    if stat {
        print_diffstat(repo, head, theirs)?;
    }
    Ok(())
}

/// `--stat` block plus `create mode` / `delete mode` / `mode change` lines.
fn print_diffstat(repo: &arks_repo::Repository, from: &Oid, to: &Oid) -> CliResult<()> {
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
        let xp = x
            .map(|f| repo.get_object(&f.oid))
            .transpose()?
            .map(|o| o.payload);
        let yp = y
            .map(|f| repo.get_object(&f.oid))
            .transpose()?
            .map(|o| o.payload);
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

/// `--log`: `* <what> of <url>:` then up to `n` merged subjects.
fn shortlog_block(
    repo: &arks_repo::Repository,
    heads: &[MergeHead],
    head: &Oid,
    n: usize,
) -> CliResult<String> {
    let mut out = String::new();
    let base = crate::cmd::shared::is_ancestor_set(repo, *head)?;
    for h in heads {
        let mut subjects = Vec::new();
        let mut stack = vec![h.oid];
        let mut seen = std::collections::HashSet::new();
        let mut commits = Vec::new();
        while let Some(c) = stack.pop() {
            if base.contains(&c) || !seen.insert(c) {
                continue;
            }
            let obj = repo.get_object(&c)?;
            let commit = arks_object::Commit::decode(&obj.payload)?;
            commits.push((commit.committer.timestamp, commit.message.clone()));
            stack.extend(commit.parents.iter().copied());
        }
        commits.sort_by_key(|c| std::cmp::Reverse(c.0));
        for (_, m) in commits.iter().take(n) {
            subjects.push(m.lines().next().unwrap_or("").to_string());
        }
        if subjects.is_empty() {
            continue;
        }
        let label = h.desc.replace("branch ", "");
        out.push_str(&format!("\n\n* {label}:"));
        if commits.len() > n {
            out.push_str("\n  ...");
        }
        for s in subjects {
            out.push_str(&format!("\n  {s}"));
        }
    }
    Ok(out)
}

fn refuse_dirty_for_rebase(repo: &arks_repo::Repository, autostash: bool) -> CliResult<()> {
    if autostash {
        return Ok(());
    }
    let head = crate::cmd::shared::head_blob_map(repo).unwrap_or_default();
    let index = crate::util::read_index(repo)?;
    let work = crate::util::work_tree(repo)?;
    let mut staged = index
        .entries
        .iter()
        .filter(|e| e.stage() == 0)
        .any(|e| head.get(&e.path) != Some(&e.oid));
    staged |= head.len() != index.entries.iter().filter(|e| e.stage() == 0).count();
    let unstaged = index.entries.iter().filter(|e| e.stage() == 0).any(|e| {
        let abs = work.join(&e.path);
        match std::fs::read(&abs) {
            Ok(b) => {
                arks_object::RawObject::new(arks_object::ObjectKind::Blob, b)
                    .oid(repo.object_format())
                    .ok()
                    != Some(e.oid)
            }
            Err(_) => true,
        }
    });
    if !staged && !unstaged {
        return Ok(());
    }
    let mut msg = String::new();
    if unstaged {
        msg.push_str("error: cannot pull with rebase: You have unstaged changes.\n");
        if staged {
            msg.push_str("error: additionally, your index contains uncommitted changes.\n");
        }
    } else {
        msg.push_str("error: cannot pull with rebase: Your index contains uncommitted changes.\n");
    }
    msg.push_str("error: Please commit or stash them.");
    Err(CliError::Exit(128, msg))
}

fn no_merge_candidates(args: &[String], repo: &arks_repo::Repository) -> CliError {
    let current = crate::cmd::shared::current_branch_name(repo).ok();
    let b = current.as_deref().unwrap_or("<branch>");
    match args.first() {
        Some(remote) if args.len() == 1 => CliError::Exit(
            1,
            format!(
                "You asked to pull from the remote '{remote}', but did not specify\na branch. Because this is not the default configured remote\nfor your current branch, you must specify a branch on the command line."
            ),
        ),
        Some(_) => CliError::Exit(
            1,
            "There are no candidates for merging among the refs that you just fetched.\nGenerally this means that you provided a wildcard refspec which had no\nmatches on the remote end.".into(),
        ),
        None => CliError::Exit(
            1,
            format!(
                "There is no tracking information for the current branch.\nPlease specify which branch you want to merge with.\nSee git-pull(1) for details.\n\n    git pull <remote> <branch>\n\nIf you wish to set tracking information for this branch you can do so with:\n\n    git branch --set-upstream-to=origin/<branch> {b}\n"
            ),
        ),
    }
}
