//! Bisect start / bad / good / reset / run (first-parent).
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
    // `run` takes a shell command that may start with `-`; allow after the verb.
    if sub.as_str() != "run" && args.iter().any(|a| a.starts_with('-')) {
        return Err(CliError::NotImplemented(
            "bisect options are not claimed".into(),
        ));
    }
    match sub.as_str() {
        "start" => bisect_start(&args[1..]),
        "bad" => bisect_mark("bad", &args[1..]),
        "good" => bisect_mark("good", &args[1..]),
        "reset" => bisect_reset(&args[1..]),
        "run" => bisect_run(&args[1..]),
        other => Err(CliError::NotImplemented(format!(
            "bisect {other} is not claimed (Sy-5: start/bad/good/reset/run; visualize N/A)"
        ))),
    }
}

fn bisect_dir(repo: &Repository) -> PathBuf {
    repo.git_dir().join("arks-bisect")
}

fn bisect_start(args: &[String]) -> CliResult<()> {
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
    match repo.head_target()? {
        RefTarget::Symbolic(name) => fs::write(dir.join("start"), format!("{name}\n"))?,
        RefTarget::Direct(oid) => fs::write(dir.join("start"), format!("{}\n", oid.to_hex()))?,
    }
    if let Some(bad) = args.first() {
        let oid = resolve_commit(&repo, bad)?;
        fs::write(dir.join("bad"), format!("{}\n", oid.to_hex()))?;
    }
    if let Some(good) = args.get(1) {
        let oid = resolve_commit(&repo, good)?;
        fs::write(dir.join("good"), format!("{}\n", oid.to_hex()))?;
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
    println!("bisect reset");
    Ok(())
}

fn bisect_step(repo: &mut Repository) -> CliResult<()> {
    let dir = bisect_dir(repo);
    let bad = read_oid_file(&dir.join("bad"))?;
    let good = read_oid_file(&dir.join("good"))?;
    let (bad, good) = match (bad, good) {
        (Some(bad), Some(good)) => (bad, good),
        _ => {
            println!("bisect started; mark good and bad to search");
            return Ok(());
        }
    };
    let list = first_parent_range(repo, good, bad)?;
    if list.is_empty() {
        return Err(CliError::Msg("bisect range is empty".into()));
    }
    if list.len() == 1 {
        detach_at(repo, &list[0])?;
        println!("{} is the first bad commit", list[0].to_hex());
        return Ok(());
    }
    let mid = (list.len() - 1) / 2;
    detach_at(repo, &list[mid])?;
    let left = list.len() - 1;
    println!("Bisecting: {left} revision(s) left to test after this");
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
        return Err(CliError::Msg("good and bad are the same commit".into()));
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
