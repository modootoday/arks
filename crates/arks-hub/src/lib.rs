//! `arks-hub` — forge collaboration surface (Partial local forge API + client stubs).
//!
//! Wired as flat forge leaves (`arks pr|issue|auth|…`) and transitional
//! `arks hub …`. Library-only. MUST NOT spawn `gh` or `git`.
//!
//! Local forge host: `arks hub serve` or the forge module under `arks serve`
//! ([[sot:root/arks-self-host-server]]). Design ≠ Claimed.
//!
//! Cite: `.spec/sot/arks-hub-cli-surface.sot.md`,
//! `.spec/sot/arks-cli-personality-factory.sot.md`

#![forbid(unsafe_code)]

mod http_min;
pub mod serve;
mod store;

use std::process::ExitCode;

pub use serve::{
    ForgeServeOptions, default_forge_data_dir, serve_forge_api, spawn_forge_api,
};

/// Flat / hub groups shown in stub help (gh-shaped + hub-native markers).
pub const GROUPS: &[&str] = &[
    "auth",
    "pr",
    "issue",
    "release",
    "repo",
    "label",
    "run",
    "workflow",
    "search",
    "status",
    "api",
    "org",
    "config",
    "alias",
    "completion",
    "co",
    "browse",
    "gist",
    "discussion",
    "project",
    "milestone",
    "secret",
    "variable",
    "cache",
    "ruleset",
    "skill",
    "agent-task",
    "preview",
    "licenses",
    "attestation",
    "codespace",
    "ssh-key",
    "gpg-key",
    "extension",
    "copilot",
    "inbox",
    "forge-config",
    "serve",
];

pub fn print_help() {
    eprintln!(
        "\
arks hub — forge collaboration (Partial local API; not Claimed)

USAGE:
    arks hub [OPTIONS] <COMMAND>
    arks <COMMAND> …   (flat forge leaves under arks / gh personality)

COMMANDS:
    serve         Local forge API listen (Partial; transitional — prefer arks serve)
    auth pr issue repo api status …
                  Client leaves (stubs refuse until deepened; never spawn git/gh)

ENV (design / Partial):
    ARKS_HUB_API  ARKS_HUB_TOKEN_FILE  ARKS_HUB_REPO  ARKS_HUB_DATA

Never spawns git/gh. Design ≠ Claimed.
"
    );
}

/// Run hub / flat-forge argv (everything after `arks hub`, or a flat leaf + args).
pub fn run_from_args(args: &[String]) -> ExitCode {
    let Some(first) = args.first() else {
        print_help();
        return ExitCode::from(2);
    };

    match first.as_str() {
        "-h" | "--help" | "help" => {
            print_help();
            ExitCode::SUCCESS
        }
        "-V" | "--version" | "version" => {
            println!("arks hub {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        "serve" => match serve::run_serve(&args[1..]) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("arks hub serve: {e}");
                ExitCode::from(2)
            }
        },
        cmd if GROUPS.contains(&cmd) => {
            if args.get(1).map(String::as_str) == Some("--help")
                || args.get(1).map(String::as_str) == Some("-h")
                || args.get(1).map(String::as_str) == Some("help")
            {
                eprintln!(
                    "arks hub {cmd}: Partial stub help (not Claimed); refuse depth until evidenced"
                );
                return ExitCode::SUCCESS;
            }
            eprintln!(
                "arks hub: `{cmd}` recognized (Partial/Missing stub); refuse exit 2 — \
                 design ≠ Claimed ([[sot:root/arks-hub-cli-surface]])"
            );
            ExitCode::from(2)
        }
        other => {
            eprintln!("arks hub: unknown command `{other}`");
            print_help();
            ExitCode::from(2)
        }
    }
}
