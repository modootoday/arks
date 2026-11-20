//! Bisect start (with custom terms, --no-checkout, --first-parent) / bad /
//! good / new / old / <term> / terms / reset / run (first-parent walk).
//!
//! Cite: <https://git-scm.com/docs/git-bisect/2.39.0>
//! `run <cmd>`: exit 0 → good; 1–127 (except 125) → bad; 125 → skip (not claimed); ≥128 → abort.

use crate::cmd::replay::{read_commit, resolve_commit};
use crate::error::{CliError, CliResult};
use crate::util::{open_repo, work_tree};
use arks_oid::Oid;
use arks_refs::RefTarget;
use arks_repo::Repository;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
pub fn cmd_bisect(args: &[String]) -> CliResult<()> {
    let sub = args
        .first()
        .ok_or_else(|| CliError::Usage("bisect start|bad|good|reset|run".into()))?;
    if sub == "start" {
        return bisect_start_opts(&args[1..]);
    }
    if sub == "terms" {
        return bisect_terms(&args[1..]);
    }
    // `run` takes a shell command that may start with `-`; allow after the verb.
    if sub.as_str() != "run" && args.iter().any(|a| a.starts_with('-')) {
        return Err(CliError::NotImplemented(
            "bisect options are not claimed".into(),
        ));
    }
    // Custom terms (`--term-new` / `--term-old`) name the bad / good marks.
    let (term_bad, term_good) = {
        let repo = open_repo()?;
        read_terms(&bisect_dir(&repo))
    };
    let kind = if *sub == term_bad || sub == "new" {
        Some("bad")
    } else if *sub == term_good || sub == "old" {
        Some("good")
    } else {
        None
    };
    if let Some(k) = kind {
        return bisect_mark(k, &args[1..]);
    }
    match sub.as_str() {
        "bad" => bisect_mark("bad", &args[1..]),
        "good" => bisect_mark("good", &args[1..]),
        "reset" => bisect_reset(&args[1..]),
        "run" => bisect_run(&args[1..]),
        other => Err(CliError::NotImplemented(format!(
            "bisect {other} is not claimed (Sy-5: start/bad/good/reset/run; visualize N/A)"
        ))),
    }
}

/// `start [--term-{new,bad}=<t>] [--term-{old,good}=<t>] [--no-checkout]
/// [--first-parent] [<bad> [<good>...]]` (this engine always walks first
/// parents and keeps the work tree when `--no-checkout` is given).
fn bisect_start_opts(args: &[String]) -> CliResult<()> {
    use crate::cmd::opt_parse::{Mode, parse};
    let p = parse(
        "bisect start",
        args,
        crate::cmd::porcelain_opts::BISECT__START,
        Mode::default(),
    )?;
    if !p.after_dashdash().is_empty() {
        return Err(CliError::NotImplemented(
            "bisect start -- <pathspec> is not implemented".into(),
        ));
    }
    let new = p.value("term-new").or_else(|| p.value("term-bad"));
    let old = p.value("term-old").or_else(|| p.value("term-good"));
    for t in [&new, &old].into_iter().flatten() {
        if matches!(
            t.as_str(),
            "help"
                | "start"
                | "skip"
                | "next"
                | "reset"
                | "visualize"
                | "view"
                | "replay"
                | "log"
                | "run"
                | "terms"
        ) {
            return Err(CliError::Exit(
                1,
                format!("error: can't use the builtin command '{t}' as a term"),
            ));
        }
    }
    let pos = p.args();
    if pos.len() > 2 {
        return Err(CliError::NotImplemented(
            "bisect start with several good commits is not implemented".into(),
        ));
    }
    let terms = (new, old, p.on("no-checkout"));
    bisect_start_with(&pos, terms)
}

fn read_terms(dir: &Path) -> (String, String) {
    let text = fs::read_to_string(dir.join("terms")).unwrap_or_default();
    let mut l = text.lines();
    (
        l.next().unwrap_or("bad").to_string(),
        l.next().unwrap_or("good").to_string(),
    )
}

/// `terms [--term-good | --term-bad]`.
fn bisect_terms(args: &[String]) -> CliResult<()> {
    use crate::cmd::opt_parse::{Mode, parse};
    let p = parse(
        "bisect terms",
        args,
        crate::cmd::porcelain_opts::BISECT__TERMS,
        Mode::default(),
    )?;
    let repo = open_repo()?;
    let dir = bisect_dir(&repo);
    if !dir.is_dir() {
        return Err(CliError::Exit(1, "error: no terms defined".into()));
    }
    let (bad, good) = read_terms(&dir);
    if p.on("term-good") || p.on("term-old") {
        println!("{good}");
    } else if p.on("term-bad") || p.on("term-new") {
        println!("{bad}");
    } else {
        println!("Your current terms are {good} for the old state\nand {bad} for the new state.");
    }
    Ok(())
}

fn bisect_dir(repo: &Repository) -> PathBuf {
    repo.git_dir().join("arks-bisect")
}

fn bisect_start_with(
    args: &[String],
    terms: (Option<String>, Option<String>, bool),
) -> CliResult<()> {
    if args.len() > 2 {
        return Err(CliError::Usage("bisect start [<bad> [<good>]]".into()));
    }
    let mut repo = open_repo()?;
    let dir = bisect_dir(&repo);
    // BISECT-001: exclusive bisect session — refuse concurrent start (stock Git
    // BISECT_* state / arks-bisect admin dir). Catalog M19 Cx-2.
    if dir.exists()
        || repo.git_dir().join("BISECT_LOG").is_file()
        || repo.git_dir().join("BISECT_START").is_file()
    {
        return Err(CliError::Msg(
            "bisect already in progress; refuse concurrent bisect start (BISECT-001)".into(),
        ));
    }
    fs::create_dir_all(&dir)?;
    fs::write(
        dir.join("terms"),
        format!(
            "{}\n{}\n",
            terms.0.unwrap_or_else(|| "bad".into()),
            terms.1.unwrap_or_else(|| "good".into())
        ),
    )?;
    if terms.2 {
        fs::write(dir.join("no-checkout"), "")?;
    }
    match repo.head_target()? {
        RefTarget::Symbolic(name) => fs::write(dir.join("start"), format!("{name}\n"))?,
        RefTarget::Direct(oid) => fs::write(dir.join("start"), format!("{}\n", oid.to_hex()))?,
    }
    if let Some(bad) = args.first() {
        let oid = resolve_commit(&repo, bad)?;
        fs::write(dir.join("bad"), format!("{}\n", oid.to_hex()))?;
        record_ref(&mut repo, "bad", &oid)?;
    }
    if let Some(good) = args.get(1) {
        let oid = resolve_commit(&repo, good)?;
        fs::write(dir.join("good"), format!("{}\n", oid.to_hex()))?;
        record_ref(&mut repo, "good", &oid)?;
    }
    bisect_step(&mut repo)
}

fn bisect_mark(kind: &str, args: &[String]) -> CliResult<()> {
    if args.len() > 1 {
        return Err(CliError::Usage(format!("bisect {kind} [<rev>]")));
    }
    let mut repo = open_repo()?;
    let dir = bisect_dir(&repo);
    if !dir.is_dir() {
        return Err(CliError::Msg("bisect is not started".into()));
    }
    // BISECT-002: dirty worktree vs bisect checkout — refuse advancing while dirty.
    if crate::cmd::tag::worktree_dirty(&repo)? {
        return Err(CliError::Msg(
            "bisect: worktree is dirty; refuse concurrent agent edits mid-bisect (BISECT-002)"
                .into(),
        ));
    }
    let oid = match args.first() {
        Some(rev) => resolve_commit(&repo, rev)?,
        None => {
            let oid = repo.head_oid()?;
            let _ = read_commit(&repo, &oid)?;
            oid
        }
    };
    fs::write(dir.join(kind), format!("{}\n", oid.to_hex()))?;
    record_ref(&mut repo, kind, &oid)?;
    bisect_step(&mut repo)
}

fn bisect_reset(args: &[String]) -> CliResult<()> {
    if !args.is_empty() {
        return Err(CliError::NotImplemented(
            "bisect reset <commit> is not claimed".into(),
        ));
    }
    let mut repo = open_repo()?;
    let dir = bisect_dir(&repo);
    if !dir.is_dir() {
        println!("We are not bisecting.");
        return Ok(());
    }
    let start = fs::read_to_string(dir.join("start"))?;
    let start = start.trim();
    // git reports where HEAD was and switches back (stderr).
    let before = repo.head_target()?;
    match &before {
        RefTarget::Symbolic(b) if b == start => {
            eprintln!("Already on '{}'", b.trim_start_matches("refs/heads/"));
        }
        _ => {
            if let Ok(h) = repo.head_oid() {
                let subj = read_commit(&repo, &h)
                    .map(|c| crate::cmd::log_format::subject_line(&c.message))
                    .unwrap_or_default();
                eprintln!(
                    "Previous HEAD position was {} {subj}",
                    crate::cmd::shared::abbrev_hex(&repo, &h, 7)
                );
            }
            if let Some(b) = start.strip_prefix("refs/heads/") {
                eprintln!("Switched to branch '{b}'");
            }
        }
    }
    for (name, oid) in repo.refs().list_refs("refs/bisect/")? {
        let _ = repo.refs_mut().delete(&name, Some(&oid));
    }
    if start.starts_with("refs/") {
        repo.refs_mut().set_symbolic("HEAD", start)?;
        let oid = repo.refs().resolve(start)?;
        let work = work_tree(&repo)?;
        arks_remote::checkout_commit(&repo, &work, &oid)?;
    } else {
        let oid = Oid::from_hex(start)?;
        repo.refs_mut().update("HEAD", &oid, None)?;
        let work = work_tree(&repo)?;
        arks_remote::checkout_commit(&repo, &work, &oid)?;
    }
    fs::remove_dir_all(&dir)?;
    Ok(())
}

/// Roughly how many more tests a range of `all` candidates needs.
fn estimate_steps(all: usize) -> usize {
    if all < 3 {
        return 0;
    }
    let n = usize::BITS as usize - 1 - all.leading_zeros() as usize;
    let e = 1usize << n;
    let x = all - e;
    if e < 3 * x { n } else { n - 1 }
}

fn bisect_step(repo: &mut Repository) -> CliResult<()> {
    let dir = bisect_dir(repo);
    let bad = read_oid_file(&dir.join("bad"))?;
    let good = read_oid_file(&dir.join("good"))?;
    let (term_bad, _) = read_terms(&dir);
    let (bad, good) = match (bad, good) {
        (Some(bad), Some(good)) => (bad, good),
        (None, None) => {
            println!("status: waiting for both good and bad commits");
            return Ok(());
        }
        (Some(_), None) => {
            println!("status: waiting for good commit(s), bad commit known");
            return Ok(());
        }
        (None, Some(_)) => {
            println!("status: waiting for bad commit, 1 good commit known");
            return Ok(());
        }
    };
    let list = first_parent_range(repo, good, bad)?;
    if list.is_empty() {
        return Err(CliError::Msg("bisect range is empty".into()));
    }
    let all = list.len();
    if all == 1 {
        println!("{} is the first {term_bad} commit", list[0].to_hex());
        use std::io::Write;
        std::io::stdout().flush()?;
        return crate::cmd::cmd_show(&["--stat".to_string(), list[0].to_hex()]);
    }
    // The commit to test: `all / 2` commits of the range below it (observed
    // on stock git for linear ranges; three candidates test the middle one).
    let weight = if all == 3 { 2 } else { all / 2 };
    let pick = list[weight - 1];
    let left = all - weight - 1;
    let steps = estimate_steps(all);
    println!(
        "Bisecting: {left} revision{} left to test after this (roughly {steps} step{})",
        if left == 1 { "" } else { "s" },
        if steps == 1 { "" } else { "s" }
    );
    detach_at(repo, &pick)?;
    let subj = read_commit(repo, &pick)
        .map(|c| crate::cmd::log_format::subject_line(&c.message))
        .unwrap_or_default();
    println!("[{}] {subj}", pick.to_hex());
    Ok(())
}

/// git's refs for marked commits: `refs/bisect/bad`, `refs/bisect/good-<oid>`.
fn record_ref(repo: &mut Repository, kind: &str, oid: &Oid) -> CliResult<()> {
    let name = if kind == "bad" {
        "refs/bisect/bad".to_string()
    } else {
        format!("refs/bisect/good-{}", oid.to_hex())
    };
    let old = repo.refs().resolve(&name).ok();
    repo.refs_mut().update(&name, oid, old.as_ref())?;
    Ok(())
}

/// M16 Sy-5: `bisect run <cmd> [args...]` — drive good/bad from command exit status.
fn bisect_run(args: &[String]) -> CliResult<()> {
    if args.is_empty() {
        return Err(CliError::Usage("bisect run <cmd> [<args>...]".into()));
    }
    let repo = open_repo()?;
    let dir = bisect_dir(&repo);
    if !dir.is_dir() {
        return Err(CliError::Msg("bisect is not started".into()));
    }
    drop(repo);
    loop {
        {
            let repo = open_repo()?;
            let dir = bisect_dir(&repo);
            let bad = read_oid_file(&dir.join("bad"))?;
            let good = read_oid_file(&dir.join("good"))?;
            let (bad, good) = match (bad, good) {
                (Some(b), Some(g)) => (b, g),
                _ => {
                    return Err(CliError::Msg(
                        "bisect run: need both good and bad marked".into(),
                    ));
                }
            };
            let list = first_parent_range(&repo, good, bad)?;
            if list.len() <= 1 {
                if let Some(first) = list.first() {
                    println!("{} is the first bad commit", first.to_hex());
                }
                return Ok(());
            }
        }
        let status = Command::new(&args[0])
            .args(&args[1..])
            .status()
            .map_err(|e| CliError::Msg(format!("bisect run: failed to spawn {}: {e}", args[0])))?;
        let code = status.code().unwrap_or(128);
        if code == 0 {
            bisect_mark("good", &[])?;
        } else if code == 125 {
            return Err(CliError::NotImplemented(
                "bisect run skip (exit 125) is not claimed".into(),
            ));
        } else if (1..128).contains(&code) {
            bisect_mark("bad", &[])?;
        } else {
            return Err(CliError::Msg(format!(
                "bisect run: command aborted with status {code}"
            )));
        }
        // Re-check termination after mark.
        let repo = open_repo()?;
        let dir = bisect_dir(&repo);
        let bad = read_oid_file(&dir.join("bad"))?.unwrap();
        let good = read_oid_file(&dir.join("good"))?.unwrap();
        let list = first_parent_range(&repo, good, bad)?;
        if list.len() <= 1 {
            if let Some(first) = list.first() {
                println!("{} is the first bad commit", first.to_hex());
            }
            return Ok(());
        }
    }
}

fn first_parent_range(repo: &Repository, good: Oid, bad: Oid) -> CliResult<Vec<Oid>> {
    if good == bad {
        let (b, g) = read_terms(&bisect_dir(repo));
        return Err(CliError::Exit(
            1,
            format!("{} was both {g} and {b}", bad.to_hex()),
        ));
    }
    let mut newest = Vec::new();
    let mut oid = bad;
    loop {
        if oid == good {
            break;
        }
        newest.push(oid);
        let commit = read_commit(repo, &oid)?;
        match commit.parents.first() {
            Some(parent) => oid = *parent,
            None => {
                return Err(CliError::Msg(
                    "good is not a first-parent ancestor of bad".into(),
                ));
            }
        }
    }
    newest.reverse();
    Ok(newest)
}

fn detach_at(repo: &mut Repository, oid: &Oid) -> CliResult<()> {
    if bisect_dir(repo).join("no-checkout").exists() {
        // --no-checkout: record the commit to test without touching HEAD.
        repo.refs_mut().update("BISECT_HEAD", oid, None)?;
        return Ok(());
    }
    repo.refs_mut().update("HEAD", oid, None)?;
    let work = work_tree(repo)?;
    arks_remote::checkout_commit(repo, &work, oid)?;
    Ok(())
}

fn read_oid_file(path: &Path) -> CliResult<Option<Oid>> {
    if !path.is_file() {
        return Ok(None);
    }
    let text = fs::read_to_string(path)?;
    let text = text.trim();
    if text.is_empty() {
        return Ok(None);
    }
    Ok(Some(Oid::from_hex(text)?))
}
