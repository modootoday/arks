//! Exit-code policy skeleton (triple personality).
//! Cite: `.spec/sot/arks-cli-output-exit-contract.sot.md`

use super::personality::Personality;
use std::process::ExitCode;

/// Stable skeleton exit classes (not per-command Claimed tables).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitClass {
    GenericFailure = 1,
    /// Usage, unknown personality, Partial stub, enabled-but-Missing module.
    StubOrUsage = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerOutcome {
    Success,
    /// Implemented path failed (git-faithful default → 1).
    Failed,
    /// Refuse / Partial stub / usage (→ 2 under arks/gh stub policy).
    StubOrUsage,
    /// Layer honestly skipped as inapplicable.
    Skipped,
}

impl LayerOutcome {
    pub fn is_failure(self) -> bool {
        matches!(self, Self::Failed | Self::StubOrUsage)
    }
}

pub struct ExitCodePolicy;

impl ExitCodePolicy {
    pub fn map(personality: Personality, outcomes: &[LayerOutcome]) -> ExitCode {
        let ran: Vec<LayerOutcome> = outcomes
            .iter()
            .copied()
            .filter(|o| *o != LayerOutcome::Skipped)
            .collect();
        if ran.is_empty() {
            return ExitCode::from(ExitClass::StubOrUsage as u8);
        }
        match personality {
            Personality::Git => {
                // Preserve Claimed git porcelain: any failure → 1 (historical tip).
                if ran.iter().any(|o| o.is_failure()) {
                    ExitCode::from(ExitClass::GenericFailure as u8)
                } else {
                    ExitCode::SUCCESS
                }
            }
            Personality::Gh => {
                if ran.iter().any(|o| matches!(o, LayerOutcome::StubOrUsage)) {
                    ExitCode::from(ExitClass::StubOrUsage as u8)
                } else if ran.iter().any(|o| matches!(o, LayerOutcome::Failed)) {
                    ExitCode::from(ExitClass::GenericFailure as u8)
                } else {
                    ExitCode::SUCCESS
                }
            }
            Personality::Arks => {
                // Fail-closed OR of enabled layers.
                if ran.iter().any(|o| matches!(o, LayerOutcome::StubOrUsage)) {
                    ExitCode::from(ExitClass::StubOrUsage as u8)
                } else if ran.iter().any(|o| matches!(o, LayerOutcome::Failed)) {
                    ExitCode::from(ExitClass::GenericFailure as u8)
                } else {
                    ExitCode::SUCCESS
                }
            }
        }
    }

    pub fn from_class(c: ExitClass) -> ExitCode {
        ExitCode::from(c as u8)
    }
}
