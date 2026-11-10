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
pub fn dispatch_overlap(
    personality: Personality,
    leaf: &str,
    args: &[String],
) -> FacadeOutcome {
    let (mode, rest) = match personality {
        Personality::Git => (PeerMode::GitOnly, args.to_vec()),
        Personality::Gh => (PeerMode::ForgeOnly, args.to_vec()),
        Personality::Arks => PeerMode::from_args(args),
    };

    let mut outcomes = Vec::new();

    match mode {
        PeerMode::GitOnly => {
            outcomes.push(run_git(leaf, &rest));
        }
        PeerMode::ForgeOnly => {
            outcomes.push(run_forge(leaf, &rest));
        }
        PeerMode::Unified => {
            // Skeleton honesty: run Git (Claimed paths). Forge unified facet is
            // honestly **Skipped** until Claimed — do not fail-closed the git exit
            // with a Partial stub (would regress status/config gates).
            // `--forge-only` / personality=gh still hit the stub path.
            println!("=== Git ===");
            let g = run_git(leaf, &rest);
            outcomes.push(g);
            println!("=== Forge ===");
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
