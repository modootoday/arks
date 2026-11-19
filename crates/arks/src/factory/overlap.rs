//! OverlapFacade — git rank-0 → gh rank-1 parse; unified render; fail-closed exit.
//! Cite: `.spec/sot/arks-cli-personality-factory.sot.md`
//!
//! Forge peer routes through `arks-hub` Partial stubs (ExitCodePolicy). Design ≠ Claimed.

use super::exit_code::LayerOutcome;
use super::git_dispatch::dispatch_git_leaf;
use super::personality::Personality;
use crate::error::CliError;
use std::process::ExitCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerMode {
    /// Unified Git + Forge (arks default).
    Unified,
    GitOnly,
    ForgeOnly,
}

impl PeerMode {
    /// Design flag names (Partial until clap Claimed).
    pub fn from_args(args: &[String]) -> (Self, Vec<String>) {
        let mut mode = Self::Unified;
        let mut rest = Vec::with_capacity(args.len());
        for a in args {
            match a.as_str() {
                "--git-only" => mode = Self::GitOnly,
                "--forge-only" => mode = Self::ForgeOnly,
                // `--porcelain` on overlap → git script form only (design).
                "--porcelain" if mode == Self::Unified => {
                    mode = Self::GitOnly;
                    rest.push(a.clone());
                }
                _ => rest.push(a.clone()),
            }
        }
        (mode, rest)
    }
}

pub struct FacadeOutcome {
    pub exit: ExitCode,
}

/// Dispatch overlap leaf under the given personality.
pub fn dispatch_overlap(personality: Personality, leaf: &str, args: &[String]) -> FacadeOutcome {
    let (mut mode, rest) = match personality {
        Personality::Git => (PeerMode::GitOnly, args.to_vec()),
        Personality::Gh => (PeerMode::ForgeOnly, args.to_vec()),
        Personality::Arks => PeerMode::from_args(args),
    };
    if mode == PeerMode::Unified {
        mode = rank_parse(leaf, &rest);
    }

    let mut outcomes = Vec::new();

    match mode {
        PeerMode::GitOnly => {
            outcomes.push(run_git(leaf, &rest));
        }
        PeerMode::ForgeOnly => {
            outcomes.push(run_forge(leaf, &rest));
        }
        PeerMode::Unified => {
            // status|config|repo under arks: Git Claimed path + honest Forge skip.
            // Do not OR a Partial forge stub into the git exit (would regress gates).
            println!("=== Git ({leaf}) ===");
            let g = run_git(leaf, &rest);
            outcomes.push(g);
            println!("=== Forge ({leaf}) ===");
            eprintln!(
                "note: arks {leaf}: forge unified section Partial — skipped (not Claimed); \
                 use --forge-only or ARKS_PERSONALITY=gh for hub facet \
                 ([[sot:root/arks-cli-output-exit-contract]])"
            );
            outcomes.push(LayerOutcome::Skipped);
        }
    }

    let exit = super::exit_code::ExitCodePolicy::map(personality, &outcomes);
    FacadeOutcome { exit }
}

/// Rank 0 (git grammar) then rank 1 (gh grammar) for one invocation
/// ([[sot:root/arks-cli-personality-factory]] rule 14). Only `repo` has disjoint
/// subcommand sets today: git owns `info` / `structure`; every other `repo`
/// subcommand is forge-client grammar. `status` / `config` keep the unified view.
fn rank_parse(leaf: &str, args: &[String]) -> PeerMode {
    if leaf != "repo" {
        return PeerMode::Unified;
    }
    match args
        .iter()
        .find(|a| !a.starts_with('-'))
        .map(String::as_str)
    {
        None => PeerMode::Unified,
        Some("info" | "structure") => PeerMode::GitOnly,
        Some(_) => PeerMode::ForgeOnly,
    }
}

fn run_git(leaf: &str, args: &[String]) -> LayerOutcome {
    match dispatch_git_leaf(leaf, args) {
        Ok(()) => LayerOutcome::Success,
        Err(CliError::Usage(m)) | Err(CliError::NotImplemented(m)) => {
            eprintln!("error: {m}");
            // Preserve Claimed git failure class → Failed (maps to 1 under git/arks OR).
            LayerOutcome::Failed
        }
        Err(CliError::Msg(m)) => {
            eprintln!("error: {m}");
            LayerOutcome::Failed
        }
        Err(CliError::Exit(_, m)) => {
            if !m.is_empty() {
                eprintln!("{m}");
            }
            LayerOutcome::Failed
        }
    }
}

fn run_forge(leaf: &str, args: &[String]) -> LayerOutcome {
    let mut hub_argv = Vec::with_capacity(args.len() + 1);
    hub_argv.push(leaf.to_string());
    hub_argv.extend(args.iter().cloned());
    let code = arks_hub::run_from_args(&hub_argv);
    if code == ExitCode::SUCCESS {
        LayerOutcome::Success
    } else if code == ExitCode::from(2) {
        LayerOutcome::StubOrUsage
    } else {
        LayerOutcome::Failed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peer_mode_flags() {
        let args = vec!["--git-only".into(), "-sb".into()];
        assert_eq!(PeerMode::from_args(&args).0, PeerMode::GitOnly);
        let args = vec!["--forge-only".into(), "list".into()];
        assert_eq!(PeerMode::from_args(&args).0, PeerMode::ForgeOnly);
        let args = vec!["--porcelain".into()];
        let (m, rest) = PeerMode::from_args(&args);
        assert_eq!(m, PeerMode::GitOnly);
        assert_eq!(rest, vec!["--porcelain".to_string()]);
    }

    #[test]
    fn repo_rank_parse() {
        let v = |s: &[&str]| s.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        assert_eq!(rank_parse("repo", &v(&["info"])), PeerMode::GitOnly);
        assert_eq!(
            rank_parse("repo", &v(&["autolink", "list"])),
            PeerMode::ForgeOnly
        );
        assert_eq!(
            rank_parse("repo", &v(&["deploy-key", "list"])),
            PeerMode::ForgeOnly
        );
        assert_eq!(rank_parse("repo", &v(&["list"])), PeerMode::ForgeOnly);
        assert_eq!(rank_parse("repo", &v(&[])), PeerMode::Unified);
        assert_eq!(rank_parse("status", &v(&["-s"])), PeerMode::Unified);
    }
}
