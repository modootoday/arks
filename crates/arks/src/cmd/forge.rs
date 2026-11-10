//! Thin `gh` / forge-API wrap — Claimed-closest git-adjacent parity.
//!
//! This is **not** a forge Checks / PR / Issues / OAuth product. It maximizes
//! operator parity by forwarding to host `gh` (or documenting credential.helper
//! OAuth) while keeping Specfold honesty: full forge host UX remains Non-goal.
//!
//! Cite: `.spec/research/20261111001816-partial-completion-unblock-roadmap.research.md`
//!       `.spec/research/20261110222000-remaining-gaps-ffi-external-unblock.research.md`

use crate::error::{CliError, CliResult};
use std::env;
use std::process::{Command, Stdio};

const HONESTY: &str = "\
note: arks forge — thin wrap of host `gh` (Claimed-closest); not forge Checks/PR/Issues/OAuth product UX
note: prefer external credential.helper / Git Credential Manager for forge OAuth; embedded browser OAuth remains Refuse/Impossible
note: for raw GitHub REST, use `arks forge api …` → `gh api …` when `gh` is on PATH";

/// `arks forge [recipe|help|<gh args…>]` — forward to host `gh` when present.
pub fn cmd_forge(args: &[String]) -> CliResult<()> {
    // All `gh` flags are forwarded as-is; arks does not invent a forge flag surface.
    if args.is_empty()
        || matches!(
            args.first().map(String::as_str),
            Some("help" | "--help" | "-h")
        )
    {
        print_help();
        return Ok(());
    }

    if args.first().map(String::as_str) == Some("recipe") {
        print_recipes();
        return Ok(());
    }

    let forwarded: Vec<&str> = if args.first().map(String::as_str) == Some("--") {
        args.iter().skip(1).map(String::as_str).collect()
    } else {
        args.iter().map(String::as_str).collect()
    };

    let gh = resolve_gh().ok_or_else(|| {
        CliError::Msg(
            "forge: host `gh` not found on PATH — install GitHub CLI, or run `arks forge recipe` for docs; arks does not embed forge UX"
                .into(),
        )
    })?;

    eprintln!("{HONESTY}");
    eprintln!("note: forwarding → {} {}", gh, forwarded.join(" "));

    let status = Command::new(&gh)
        .args(&forwarded)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|e| CliError::Msg(format!("forge: failed to spawn `{gh}`: {e}")))?;

    if status.success() {
        Ok(())
    } else {
        let code = status.code().unwrap_or(1);
        Err(CliError::Msg(format!(
            "forge: host `gh` exited {code} (thin-wrap; not an arks forge-product failure class)"
        )))
    }
}

fn resolve_gh() -> Option<String> {
    if let Ok(explicit) = env::var("ARKS_GH") {
        if !explicit.is_empty() {
            return Some(explicit);
        }
    }
    which("gh")
}

fn which(bin: &str) -> Option<String> {
    let path = env::var_os("PATH")?;
    for dir in env::split_paths(&path) {
        #[cfg(windows)]
        {
            for name in [format!("{bin}.exe"), format!("{bin}.cmd"), bin.to_string()] {
                let cand = dir.join(&name);
                if cand.is_file() {
                    return Some(cand.to_string_lossy().into_owned());
                }
            }
        }
        #[cfg(not(windows))]
        {
            let cand = dir.join(bin);
            if cand.is_file() {
                return Some(cand.to_string_lossy().into_owned());
            }
        }
    }
    None
}

fn print_help() {
    println!(
        "\
arks forge — Claimed-closest thin wrap of host GitHub CLI (`gh`)

Usage:
  arks forge recipe              Print operator recipes (no network)
  arks forge <gh-args…>          Forward to `gh` on PATH (or $ARKS_GH)
  arks forge -- <gh-args…>       Same, after explicit `--`

Honesty:
  Claimed-closest = subprocess forward + credential.helper honor.
  Not Claimed: forge Checks / merge-queue / Issues product UX, embedded OAuth browser.

Examples:
  arks forge pr list
  arks forge api repos/{{owner}}/{{repo}}/actions/runs --jq '.total_count'
  arks forge auth status"
    );
    eprintln!("{HONESTY}");
}

fn print_recipes() {
    println!(
        "\
# arks forge recipes (Claimed-closest — use host tools)

## Checks / PR / Issues
  gh pr list
  gh pr checks
  gh issue list
  gh api repos/{{owner}}/{{repo}}/commits/{{sha}}/check-runs

## OAuth / credentials (external helper — already Claimed under G33)
  git config --global credential.helper manager   # Git Credential Manager
  # or: credential.helper = wincred|libsecret|osxkeychain / arks wallet helpers
  # Embedded browser OAuth inside arks remains Refuse/Impossible.

## Thin wrap
  arks forge pr list          # requires `gh` on PATH
  ARKS_GH=/path/to/gh arks forge api user

## Not claimed
  Full forge host UX parity with GitHub UI
  First-party OAuth App registration / device-flow product"
    );
    eprintln!("{HONESTY}");
}
