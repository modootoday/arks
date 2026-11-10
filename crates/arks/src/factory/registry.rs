//! CommandLayer Registry skeleton — personality → exclusive / overlap dispatch.
//! Cite: `.spec/sot/arks-cli-personality-factory.sot.md`
//!
//! Flat forge leaves forward to `arks-hub` Partial stubs (exit 2). Agent remains
//! an in-tree refuse stub until the agent crate lands. Design ≠ Claimed.

use super::exit_code::{ExitClass, ExitCodePolicy, LayerOutcome};
use super::git_dispatch::{
    dispatch_git_leaf, is_exclusive_forge_leaf, is_overlap_leaf, is_serve_leaf,
};
use super::overlap::dispatch_overlap;
use super::personality::{Personality, resolve_personality};
use std::env;
use std::process::ExitCode;

const HELP: &str = "\
arks — Wave L+H+Pa+Pb+Pc + Sv (+ CLI personality skeleton)

Usage:
  arks [--version] [--help] <command> [<args>]

Personality (Partial skeleton — [[sot:root/arks-cli-personality-factory]]):
  ARKS_PERSONALITY=git|gh|arks   (first wins)
  else argv0/shim basename git|gh|arks
  else default arks (git-first merged surface)

Overlap leaves (status|config|repo) under arks: unified Git + Forge-skip stub;
  --git-only / --forge-only / --porcelain select a single peer.

Flat forge leaves (Partial/stub): pr issue auth release label run workflow
  search api org … — forward to arks-hub lib (refuse Missing).
  Transitional: arks hub … → same hub lib.

Serve: arks serve [--no-http|--no-ssh|--no-forge] …  (forge/SSH Missing → fail-closed)
  Transitional: arks serve-http ≡ serve --no-ssh --no-forge

Git porcelain/plumbing: historical inventory (Claimed rows unchanged).
Agent: arks agent … (Partial stub)
";

/// Top-level argv dispatch after personality resolution.
pub fn dispatch_argv() -> ExitCode {
    let argv0 = env::args().next();
    let personality = match resolve_personality(argv0.as_deref()) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCodePolicy::from_class(ExitClass::StubOrUsage);
        }
    };

    let mut args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty()
        || args.first().map(String::as_str) == Some("--help")
        || args.first().map(String::as_str) == Some("-h")
    {
        print_help(personality);
        return ExitCode::SUCCESS;
    }
    if args.first().map(String::as_str) == Some("--version") {
        println!("arks version {}", env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }

    let cmd = args.remove(0);
    if cmd.starts_with('-') {
        eprintln!("error: unsupported global option: {cmd}");
        return ExitCodePolicy::from_class(ExitClass::StubOrUsage);
    }

    // Transitional HubAliasLayer → arks-hub.
    if cmd == "hub" {
        return arks_hub::run_from_args(&args);
    }
    // AgentLayer (stub until agent crate commit).
    if cmd == "agent" {
        return stub_agent(&args);
    }

    if cmd == "help" {
        print_help(personality);
        return ExitCode::SUCCESS;
    }
    if cmd == "version" {
        println!("arks version {}", env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }

    // OverlapFacade.
    if is_overlap_leaf(&cmd) {
        return dispatch_overlap(personality, &cmd, &args).exit;
    }

    // Exclusive forge leaves (flat) → arks-hub Partial stubs.
    if is_exclusive_forge_leaf(&cmd) {
        return dispatch_forge(personality, &cmd, &args);
    }

    // Serve tip leaf.
    if is_serve_leaf(&cmd) {
        return run_serve_leaf(&cmd, &args);
    }

    // Personality=gh should not pretend to own exclusive git porcelain.
    if personality == Personality::Gh {
        eprintln!(
            "error: `{cmd}` is not a gh-personality leaf; use ARKS_PERSONALITY=git|arks \
             or a forge leaf ([[sot:root/arks-cli-personality-factory]])"
        );
        return ExitCodePolicy::from_class(ExitClass::StubOrUsage);
    }

    // GitPorcelainLayer (+ daemon and remaining inventory).
    run_git_preserving(&cmd, &args)
}

fn stub_agent(args: &[String]) -> ExitCode {
    if args.first().map(String::as_str) == Some("--help")
        || args.first().map(String::as_str) == Some("-h")
        || args.first().map(String::as_str) == Some("help")
    {
        eprintln!("arks agent: Partial skeleton stub (not Claimed); agent crate lands later");
        return ExitCode::SUCCESS;
    }
    eprintln!(
        "error: arks agent: Partial stub — refuse (exit 2); design ≠ Claimed \
         ([[sot:root/arks-cli-personality-factory]])"
    );
    ExitCodePolicy::from_class(ExitClass::StubOrUsage)
}

fn dispatch_forge(personality: Personality, leaf: &str, args: &[String]) -> ExitCode {
    if personality == Personality::Git {
        eprintln!(
            "error: `{leaf}` is a forge leaf; not available under git personality \
             (set ARKS_PERSONALITY=arks|gh or invoke flat under arks)"
        );
        return ExitCodePolicy::from_class(ExitClass::StubOrUsage);
    }
    let mut hub_argv = Vec::with_capacity(args.len() + 1);
    hub_argv.push(leaf.to_string());
    hub_argv.extend(args.iter().cloned());
    arks_hub::run_from_args(&hub_argv)
}

fn run_git_preserving(cmd: &str, args: &[String]) -> ExitCode {
    match dispatch_git_leaf(cmd, args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            // Historical tip: all git-path CliError → exit 1 (Claimed gates).
            eprintln!("error: {e}");
            let _ = LayerOutcome::Failed;
            ExitCode::from(ExitClass::GenericFailure as u8)
        }
    }
}

fn run_serve_leaf(cmd: &str, args: &[String]) -> ExitCode {
    use crate::error::CliError;
    match dispatch_git_leaf(cmd, args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(CliError::Usage(m)) | Err(CliError::NotImplemented(m)) => {
            eprintln!("error: {m}");
            ExitCodePolicy::from_class(ExitClass::StubOrUsage)
        }
        Err(CliError::Msg(m)) => {
            eprintln!("error: {m}");
            ExitCode::from(ExitClass::GenericFailure as u8)
        }
    }
}

fn print_help(personality: Personality) {
    print!("{HELP}");
    eprintln!(
        "\n(active personality: {} — Partial skeleton; design ≠ Claimed)",
        personality.as_str()
    );
}
