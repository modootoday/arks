//! `arks-agent` — LLM code review / CEV inspection bots (skeleton; not Claimed).
//!
//! Wired into the single release binary as **`arks agent …`**. This crate is
//! library-only — never a separately shipped product binary.
//!
//! Native HTTP to LLM providers + hub APIs only — MUST NOT spawn `git`, `gh`,
//! or vendor CLIs (`ollama`, `openai`, …).

#![forbid(unsafe_code)]

use std::process::ExitCode;

/// Top-level groups shown in stub help.
pub const GROUPS: &[&str] = &[
    "review",
    "cev-check",
    "provider",
    "config",
    "run",
    "serve",
    "bot",
    "doctor",
];

/// Print `arks agent` stub help to stderr.
pub fn print_help() {
    eprintln!(
        "\
arks agent — LLM review / CEV bots (skeleton; not Claimed)

USAGE:
    arks agent [OPTIONS] <COMMAND>

OPTIONS:
    -h, --help       Print help
    -V, --version    Print version

COMMANDS (intent — not implemented yet):
    review        Automatic code review (local diff and/or hub PR)
    cev-check     Consistency / Evidence / Verify inspection vs Specfold
    provider      List / show / test / set LLM endpoints (OpenAI-compat, …)
    config        Agent configuration
    run           One-shot bot job from a profile
    serve         Long-running bot listener (later)
    bot           Bot aliases (later; may fold into serve/run)
    doctor        Config / provider reachability sanity check

NOTES:
    Entry is `arks agent` (single arks release binary). Not a separate arks-agent product.
    Not required for git alias DoD. Not a general chatbot.
    Providers: openai-compat (OpenAI, Ollama, LocalAI, vLLM, LM Studio, …),
    Cloudflare, Claude, Grok, Gemini — see SoT arks-agent-llm-providers.
    Local Ollama default base URL: http://127.0.0.1:11434/v1
    Native HTTP only — never spawns git / gh / vendor CLIs.
"
    );
}

/// Run agent subcommand args (everything after `arks agent`).
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
            println!("arks agent {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        cmd if GROUPS.contains(&cmd) => {
            eprintln!(
                "arks agent: `{cmd}` is not implemented yet (design stub; LLM providers forthcoming)."
            );
            ExitCode::from(2)
        }
        other => {
            eprintln!("arks agent: unknown command `{other}`");
            print_help();
            ExitCode::from(2)
        }
    }
}
