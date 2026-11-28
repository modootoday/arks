//! `arks-hub` — forge collaboration surface (Partial local forge API + client).
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
#![recursion_limit = "256"]

mod forge;
mod http_min;
mod key_formats;
mod leaves;
mod local_tls_smoke;
pub mod sealed_box;
pub mod serve;
mod store;

use http_min::http_exchange;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};

pub use forge::push_rules::{PushRefUpdate, push_verdicts};
pub use serve::{
    DEFAULT_FORGE_LISTEN, ForgeListenRebindSlot, ForgeServeOptions, ServeError,
    default_forge_data_dir, forge_serve_scheme, serve_forge_api, spawn_forge_api,
};

/// Flat / hub groups shown in stub help (gh-shaped + hub-native markers).
pub const GROUPS: &[&str] = &[
    "auth",
    "pr",
    "issue",
    "release",
    "repo",
    "label",
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
    "skill",
    "agent-task",
    "preview",
    "licenses",
    "ssh-key",
    "gpg-key",
    "ruleset",
    "rs",
    "workflow",
    "run",
    "cache",
    "secret",
    "variable",
    "extension",
    "copilot",
    "inbox",
    "forge-config",
    "serve",
    "smoke",
];

/// Matrix groups kept as explicit OUT refuse (exit 2, no network call).
///
/// `extension` / `copilot` are marketplace / vendor depth. `attestation`,
/// `codespace`, and `event` remain GitHub-proprietary OUT groups
/// ([[adr:root/20261119230000]], [[sot:root/v0-2-0-scope-expansion]] §3).
pub const HUB_OUT_GROUPS: &[&str] = &["extension", "copilot", "attestation", "codespace", "event"];

/// Refuse reason for GitHub-platform-only groups and leaves.
const PLATFORM_ONLY_WHY: &str = "GitHub-platform-only (attestations/Codespaces/Event Streams) — \
     out of scope for the arks forge ([[adr:root/20261119230000]])";

pub fn print_help() {
    eprintln!(
        "\
arks hub — forge collaboration (Partial local API; not Claimed)

USAGE:
    arks hub [OPTIONS] <COMMAND>
    arks <COMMAND> …   (flat forge leaves under arks / gh personality)

COMMANDS (gh-symmetric argv; host-tested Partial, not Claimed):
    auth          login|logout|status|switch|token|refresh (multi-account hosts file)
    pr            create|list|status|view|diff|edit|close|reopen|comment|merge|ready|
                  review|checks|lock|unlock|update-branch|revert|checkout (alias: co)
    issue         create|list|status|view|close|reopen|comment|edit|delete|lock|unlock|
                  pin|unpin|transfer|develop|artifact
    repo          create|list|view|edit|delete|fork|rename|archive|unarchive|set-default|
                  clone|sync|read-dir|read-file|gitignore|license|autolink|deploy-key
    release       create|list|view|upload|download|edit|delete|delete-asset
    label         list|create|edit|delete|clone
    ruleset, rs   list|view|check (rulesets stored by the arks forge)
    search        issues|prs|repos|code|commits
    status        Assigned issues / pull requests, review requests, mentions
    browse        Print the forge URL (never launches a browser)
    api           Authenticated request to the arks forge API
    org           list
    config        get|set|list|clear-cache (arks client settings; not git config)
    alias         set|list|delete|import
    completion    Shell completion script
    gist, project, discussion, ssh-key, gpg-key
                  arks-owned forge groups
    milestone     list|create|edit|close|delete (hub-native; arks-defined argv)
    serve         Local forge API listen (transitional — prefer arks serve)
    review        Refused: use `arks pr review` (exit 2)
    licenses, skill, preview, agent-task, inbox, forge-config, smoke
                  hub-native helpers (local Partial)
    extension, copilot and the GitHub-platform-only groups (Actions, attestations,
    Codespaces) are out of scope and refuse with exit 2.

ENV:
    ARKS_HUB_API         default http://127.0.0.1:3920 for local mode
    ARKS_HUB_TOKEN_FILE  token path
    ARKS_HUB_REPO        owner/name
    ARKS_HUB_DATA        serve data dir
    ARKS_HUB_HOSTS_FILE  accounts file (default ~/.config/arks/hub-hosts.json)

Never spawns git/gh. Design ≠ Claimed.
"
    );
}

/// Run hub / flat-forge argv (everything after `arks hub`, or a flat leaf + args).
/// Set when the CLI runs under personality `gh`: forge usage failures then exit
/// 1 like gh (flag and argument errors), instead of the arks usage class 2.
static GH_FAITHFUL_EXIT: AtomicBool = AtomicBool::new(false);

/// Select gh-faithful usage exits (`.spec/sot/arks-cli-output-exit-contract.sot.md`).
pub fn set_gh_faithful_exit(on: bool) {
    GH_FAITHFUL_EXIT.store(on, Ordering::Relaxed);
}

/// Exit code for a forge usage failure under the current personality.
pub(crate) fn usage_exit() -> ExitCode {
    if GH_FAITHFUL_EXIT.load(Ordering::Relaxed) {
        ExitCode::from(1)
    } else {
        ExitCode::from(2)
    }
}

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
                ExitCode::from(e.exit_code())
            }
        },
        "auth" => leaves::dispatch(
            "auth",
            leaves::misc::AUTH_HELP,
            &args[1..],
            leaves::misc::auth,
        ),
        "pr" => leaves::dispatch("pr", leaves::pr::HELP, &args[1..], leaves::pr::run),
        "issue" => leaves::dispatch("issue", leaves::issue::HELP, &args[1..], leaves::issue::run),
        "repo" => match args.get(1).map(String::as_str) {
            Some("autolink") => leaves::dispatch(
                "repo autolink",
                leaves::autolink::HELP,
                &args[2..],
                leaves::autolink::run,
            ),
            Some("deploy-key") => leaves::dispatch(
                "repo deploy-key",
                leaves::keys::DEPLOY_HELP,
                &args[2..],
                leaves::keys::run_deploy,
            ),
            _ => leaves::dispatch("repo", leaves::repo::HELP, &args[1..], leaves::repo::run),
        },
        "label" => leaves::dispatch("label", leaves::label::HELP, &args[1..], leaves::label::run),
        "release" => leaves::dispatch(
            "release",
            leaves::release::HELP,
            &args[1..],
            leaves::release::run,
        ),
        "milestone" => leaves::dispatch(
            "milestone",
            leaves::milestone::HELP,
            &args[1..],
            leaves::milestone::run,
        ),
        "review" => cmd_review_refuse(&args[1..]),
        "gist" => leaves::dispatch("gist", leaves::gist::HELP, &args[1..], leaves::gist::run),
        "project" => leaves::dispatch(
            "project",
            leaves::project::HELP,
            &args[1..],
            leaves::project::run,
        ),
        "discussion" => leaves::dispatch(
            "discussion",
            leaves::discussion::HELP,
            &args[1..],
            leaves::discussion::run,
        ),
        "ssh-key" => leaves::dispatch(
            "ssh-key",
            leaves::keys::SSH_HELP,
            &args[1..],
            leaves::keys::run_ssh,
        ),
        "gpg-key" => leaves::dispatch(
            "gpg-key",
            leaves::keys::GPG_HELP,
            &args[1..],
            leaves::keys::run_gpg,
        ),
        "ruleset" | "rs" => leaves::dispatch(
            "ruleset",
            leaves::ruleset::HELP,
            &args[1..],
            leaves::ruleset::run,
        ),
        "workflow" => leaves::dispatch(
            "workflow",
            leaves::actions::WORKFLOW_HELP,
            &args[1..],
            leaves::actions::workflow,
        ),
        "run" => leaves::dispatch(
            "run",
            leaves::actions::RUN_HELP,
            &args[1..],
            leaves::actions::run,
        ),
        "cache" => leaves::dispatch(
            "cache",
            leaves::actions::CACHE_HELP,
            &args[1..],
            leaves::actions::cache,
        ),
        "variable" => leaves::dispatch(
            "variable",
            leaves::actions::VARIABLE_HELP,
            &args[1..],
            leaves::actions::variable,
        ),
        "secret" => leaves::dispatch(
            "secret",
            leaves::actions::SECRET_HELP,
            &args[1..],
            leaves::actions::secret,
        ),
        "licenses" => cmd_licenses(&args[1..]),
        "skill" => cmd_out_local_list("skill", &args[1..], "/user/skills", false),
        "preview" => cmd_out_local_list(
            "preview",
            &args[1..],
            "/repos/{owner}/{repo}/previews",
            true,
        ),
        "agent-task" => cmd_out_local_list("agent-task", &args[1..], "/user/agent-tasks", false),
        "browse" => leaves::flat(
            "browse",
            leaves::misc::BROWSE_HELP,
            &args[1..],
            leaves::misc::browse,
        ),
        "inbox" => {
            eprintln!("arks hub inbox → status (demoted alias; Partial)");
            leaves::flat(
                "status",
                leaves::misc::STATUS_HELP,
                &args[1..],
                leaves::misc::status,
            )
        }
        "forge-config" => cmd_forge_config(&args[1..]),
        "alias" => leaves::flat(
            "alias",
            leaves::misc::ALIAS_HELP,
            &args[1..],
            leaves::misc::alias,
        ),
        "completion" => leaves::flat(
            "completion",
            "arks completion -s {bash|zsh|fish|powershell} — print a shell completion script",
            &args[1..],
            leaves::misc::completion,
        ),
        "config" => leaves::flat(
            "config",
            leaves::misc::CONFIG_HELP,
            &args[1..],
            leaves::misc::config,
        ),
        "org" => leaves::flat(
            "org",
            "arks org list [-L <n>] — organizations (owners) on the arks forge",
            &args[1..],
            leaves::misc::org,
        ),
        "search" => leaves::dispatch(
            "search",
            leaves::search::HELP,
            &args[1..],
            leaves::search::run,
        ),
        "api" => leaves::flat("api", leaves::misc::API_HELP, &args[1..], leaves::misc::api),
        "smoke" => cmd_https_smoke(&args[1..]),
        "status" => leaves::flat(
            "status",
            leaves::misc::STATUS_HELP,
            &args[1..],
            leaves::misc::status,
        ),
        "co" => {
            let mut forwarded = vec!["checkout".to_string()];
            forwarded.extend(args[1..].iter().cloned());
            leaves::dispatch("pr", leaves::pr::HELP, &forwarded, leaves::pr::run)
        }
        cmd if HUB_OUT_GROUPS.contains(&cmd) => cmd_out_explicit_refuse(cmd, &args[1..]),
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

fn token_path() -> PathBuf {
    if let Ok(p) = env::var("ARKS_HUB_TOKEN_FILE") {
        return PathBuf::from(p);
    }
    let home = env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    home.join(".config").join("arks").join("hub-token")
}

fn api_base() -> String {
    if let Ok(v) = env::var("ARKS_HUB_API") {
        return v;
    }
    "http://127.0.0.1:3920".into()
}

fn read_token() -> Option<String> {
    let p = token_path();
    fs::read_to_string(&p)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn is_local_http(base: &str) -> bool {
    base.starts_with("http://127.")
        || base.starts_with("http://localhost")
        || base.starts_with("http://0.0.0.0")
        || base.starts_with("http://[::1]")
}

fn flag_value<'a>(args: &'a [String], names: &[&str]) -> Option<&'a str> {
    let mut i = 0;
    while i < args.len() {
        for n in names {
            if args[i] == *n {
                return args.get(i + 1).map(String::as_str);
            }
            let prefix = format!("{n}=");
            if let Some(v) = args[i].strip_prefix(&prefix) {
                return Some(v);
            }
        }
        i += 1;
    }
    None
}

fn repo_slug(args: &[String]) -> Result<(String, String), String> {
    if let Some(r) = flag_value(args, &["--repo", "-R"]) {
        let (o, n) = r
            .split_once('/')
            .ok_or_else(|| "repo must be owner/name".to_string())?;
        return Ok((o.to_string(), n.to_string()));
    }
    if let Ok(r) = env::var("ARKS_HUB_REPO") {
        let (o, n) = r
            .split_once('/')
            .ok_or_else(|| "ARKS_HUB_REPO must be owner/name".to_string())?;
        return Ok((o.to_string(), n.to_string()));
    }
    let st = load_hub_client_state();
    if let Some(r) = st.default_repo.as_deref().filter(|s| !s.is_empty()) {
        let (o, n) = r
            .split_once('/')
            .ok_or_else(|| "default_repo must be owner/name".to_string())?;
        return Ok((o.to_string(), n.to_string()));
    }
    Err("set ARKS_HUB_REPO=owner/name, --repo owner/name, or hub repo set-default".to_string())
}

fn require_api() -> Result<(String, String), String> {
    let base = api_base();
    if !(base.starts_with("http://") || base.starts_with("https://")) {
        return Err(format!(
            "hub API base must be http:// or https:// (got {base})"
        ));
    }
    if base.starts_with("https://") {
        eprintln!(
            "note: HTTPS remote hub client Partial via ureq TLS — design ≠ Claimed forge remote e2e; never spawns gh"
        );
    }
    let tok = read_token().ok_or_else(|| "not logged in — arks hub auth login".to_string())?;
    Ok((base, tok))
}

/// Remote HTTPS forge e2e harness Partial: smoke against `https://` ARKS_HUB_API
/// (or `--url`), else document Missing and optionally probe local `http://`.
/// Never spawns `gh`. Design ≠ Claimed forge remote e2e.
fn cmd_https_smoke(args: &[String]) -> ExitCode {
    if args.iter().any(|a| a == "-h" || a == "--help") {
        eprintln!(
            "arks hub smoke [--url URL] [--path /user] [--out-refuse] [--local-tls]\n  \
             HTTPS forge e2e harness Partial (ureq TLS). Prefer https:// ARKS_HUB_API.\n  \
             Without a reachable https endpoint: documents Missing (not Claimed).\n  \
             --local-tls: ephemeral self-signed HTTPS stub + insecure ureq client path Partial.\n  \
             --out-refuse: residual OUT harness Partial (OUT groups exit 2; never gh).\n  \
             Optional local http:// fallback probe — never spawns gh. Claimed forge e2e Missing."
        );
        return ExitCode::SUCCESS;
    }
    if args.iter().any(|a| a == "--out-refuse") {
        return cmd_hub_out_refuse_smoke();
    }
    if args.iter().any(|a| a == "--local-tls") {
        return cmd_hub_local_tls_smoke(args);
    }
    let path = flag_value(args, &["--path"]).unwrap_or("/user");
    let path = if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    };
    let url_override = flag_value(args, &["--url"]).map(str::to_string);
    let base = url_override.unwrap_or_else(api_base);
    let tok = read_token().unwrap_or_else(|| "local-dev-token".into());

    println!("hub_smoke=partial");
    println!("base_url={base}");
    println!("path={path}");
    println!("spawns_gh=never");

    if base.starts_with("https://") {
        match http_exchange(&base, "GET", &path, Some(&tok), None) {
            Ok((code, body)) => {
                println!("https_smoke=ok");
                println!("http_status={code}");
                if !body.is_empty() {
                    let preview: String = body.chars().take(240).collect();
                    println!("body_preview={preview}");
                }
                eprintln!(
                    "note: HTTPS forge smoke Partial succeeded — design ≠ Claimed forge remote e2e"
                );
                if code >= 400 {
                    ExitCode::from(1)
                } else {
                    ExitCode::SUCCESS
                }
            }
            Err(e) => {
                println!("https_smoke=fail");
                println!("error={e}");
                eprintln!(
                    "note: HTTPS forge e2e harness Partial — transport/TLS fail; \
                     Claimed forge remote e2e still Missing; never spawns gh"
                );
                ExitCode::from(1)
            }
        }
    } else if is_local_http(&base) {
        println!("https_smoke=Missing");
        println!(
            "missing_reason=ARKS_HUB_API is http local ({base}); set https:// URL for HTTPS harness Partial"
        );
        match http_exchange(&base, "GET", &path, Some(&tok), None) {
            Ok((code, _)) => {
                println!("local_http_fallback=ok");
                println!("http_status={code}");
                eprintln!(
                    "note: documented HTTPS Missing; local http probe Partial only — \
                     Claimed forge remote e2e Missing"
                );
                ExitCode::SUCCESS
            }
            Err(e) => {
                println!("local_http_fallback=fail");
                println!("error={e}");
                eprintln!(
                    "note: HTTPS Missing (no local https forge); local http also unreachable — \
                     start `arks serve --no-ssh` / hub serve, or point ARKS_HUB_API at https://"
                );
                ExitCode::from(1)
            }
        }
    } else if base.starts_with("http://") {
        println!("https_smoke=Missing");
        println!(
            "missing_reason=non-https ARKS_HUB_API ({base}); use https:// for harness Partial"
        );
        eprintln!("note: Claimed forge remote e2e Missing; never spawns gh");
        ExitCode::from(2)
    } else {
        eprintln!("arks hub smoke: expected http:// or https:// base (got {base})");
        ExitCode::from(2)
    }
}

/// Local self-signed HTTPS + remote-client path smoke Partial (not Claimed TLS / forge e2e).
fn cmd_hub_local_tls_smoke(args: &[String]) -> ExitCode {
    let path = flag_value(args, &["--path"]).unwrap_or("/user");
    println!("hub_smoke=partial");
    println!("local_tls_smoke=partial");
    println!("spawns_gh=never");
    println!("claimed_forge_remote_e2e=Missing");
    println!("claimed_in_process_tls=Missing");
    match local_tls_smoke::run_local_https_smoke(path) {
        Ok((addr, status, body)) => {
            println!("listen=https://{addr}");
            println!("http_status={status}");
            let preview: String = body.chars().take(240).collect();
            if !preview.is_empty() {
                println!("body_preview={preview}");
            }
            if (200..300).contains(&status) {
                println!("local_tls_smoke=ok");
                eprintln!(
                    "note: local HTTPS self-signed smoke Partial (ureq insecure client path) ok — \
                     design ≠ Claimed forge remote e2e / in-process TLS product; never spawns gh"
                );
                ExitCode::SUCCESS
            } else {
                println!("local_tls_smoke=partial_http_{status}");
                eprintln!(
                    "note: local TLS stub reachable but HTTP {status}; Claimed forge e2e Missing"
                );
                ExitCode::from(1)
            }
        }
        Err(e) => {
            println!("local_tls_smoke=fail");
            println!("error={e}");
            eprintln!(
                "note: local HTTPS smoke Partial failed; Claimed forge remote e2e Missing; \
                 never spawns gh"
            );
            ExitCode::from(1)
        }
    }
}

/// Residual OUT refuse smoke Partial: prove every `HUB_OUT_GROUPS` leaf stays OUT exit 2.
/// Never spawns `gh`. Does **not** invent Claimed forge remote e2e.
fn cmd_hub_out_refuse_smoke() -> ExitCode {
    println!("hub_smoke=partial");
    println!("out_refuse_smoke=partial");
    println!("spawns_gh=never");
    println!("claimed_forge_remote_e2e=Missing");
    let mut failed = false;
    for name in HUB_OUT_GROUPS {
        // Invoke the same OUT refuse path operators hit (`arks hub extension|run|…`).
        let code = cmd_out_explicit_refuse(name, &[]);
        let ok = code == ExitCode::from(2);
        println!("out_group={name}");
        println!("out_expect=2");
        if ok {
            println!("out_status=refuse_ok");
        } else {
            println!("out_status=refuse_unexpected");
            failed = true;
        }
    }
    if failed {
        eprintln!(
            "note: residual OUT refuse smoke Partial failed expectation — \
             Claimed forge remote e2e still Missing; never spawns gh"
        );
        ExitCode::from(1)
    } else {
        eprintln!(
            "note: residual OUT refuse smoke Partial ok (all OUT groups refuse); \
             Claimed forge remote HTTPS e2e still Missing; never spawns gh"
        );
        ExitCode::SUCCESS
    }
}

/// Explicit OUT refuse for marketplace/vendor leaves (`extension` / `copilot`).
/// Not graduated to local forge list — product boundary is refuse, not empty invent.
fn cmd_out_explicit_refuse(name: &str, args: &[String]) -> ExitCode {
    if args.first().map(String::as_str) == Some("--help")
        || args.first().map(String::as_str) == Some("-h")
        || args.first().map(String::as_str) == Some("help")
    {
        eprintln!(
            "arks hub {name}: OUT matrix leaf — explicit refuse (not local-forge Partial list). \
             Marketplace/vendor depth and GitHub-platform-only services are out of \
             self-host forge scope ([[sot:root/arks-hub-cli-surface]], \
             [[adr:root/20261119230000]]). Design ≠ Claimed."
        );
        return ExitCode::SUCCESS;
    }
    let why = match name {
        "extension" => {
            "GitHub Marketplace / IDE extensions are OUT — no local forge inventory invent"
        }
        "copilot" => "GitHub Copilot / vendor assistant APIs are OUT — not a local forge leaf",
        _ => PLATFORM_ONLY_WHY,
    };
    eprintln!(
        "arks hub: `{name}` OUT explicit refuse exit 2 — {why} \
         ([[sot:root/arks-hub-cli-surface]]); never spawns gh; design ≠ Claimed"
    );
    ExitCode::from(2)
}

/// `arks hub review`: refused duplicate of `pr review`
/// ([[sot:root/arks-hub-gh-command-matrix]] rule 12c). No forge call.
fn cmd_review_refuse(args: &[String]) -> ExitCode {
    let why = "a top-level review group would duplicate `arks pr review` (gh-identical argv); \
               refused, out of scope ([[sot:root/arks-hub-gh-command-matrix]] rule 12c, \
               [[adr:root/20261119230000]])";
    if matches!(
        args.first().map(String::as_str),
        Some("-h" | "--help" | "help")
    ) {
        eprintln!(
            "arks review: {why}\nUse: arks pr review [<number>] --approve|--comment|--request-changes -b <body>"
        );
        return ExitCode::SUCCESS;
    }
    eprintln!("arks review: refused — {why}; use `arks pr review`");
    usage_exit()
}

/// Cheap OUT→local graduation: list against local forge path or empty JSON array.
/// `needs_repo` selects whether `--repo` / ARKS_HUB_REPO is required for path fill.
fn cmd_out_local_list(name: &str, args: &[String], path_tmpl: &str, needs_repo: bool) -> ExitCode {
    let sub = args.first().map(String::as_str).unwrap_or("list");
    if matches!(sub, "-h" | "--help" | "help") {
        eprintln!(
            "arks hub {name} [list] — local forge Partial list (OUT graduation; no gh spawn; \
             design ≠ Claimed)"
        );
        return ExitCode::SUCCESS;
    }
    if !matches!(sub, "" | "list") {
        eprintln!(
            "arks hub {name} {sub}: Partial — only `list` wired; depth beyond inventory refused"
        );
        return ExitCode::from(2);
    }
    let path = if needs_repo {
        let (owner, repo) = match repo_slug(args) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("arks hub {name}: {e}");
                return ExitCode::from(2);
            }
        };
        path_tmpl
            .replace("{owner}", &owner)
            .replace("{repo}", &repo)
    } else {
        path_tmpl.to_string()
    };
    let (base, tok) = match require_api() {
        Ok(v) => v,
        Err(e) => {
            // Empty local inventory when forge is down — still Partial discoverability.
            eprintln!("arks hub {name} list: {e}");
            println!("[]");
            eprintln!(
                "note: {name} Partial empty list (API unavailable) — design ≠ Claimed; never spawns gh"
            );
            return ExitCode::SUCCESS;
        }
    };
    match http_exchange(&base, "GET", &path, Some(&tok), None) {
        Ok((code, body)) => {
            if code >= 400 {
                // Forge may not implement the leaf yet — empty array + honesty.
                println!("[]");
                eprintln!(
                    "note: {name} Partial empty list (HTTP {code} from {path}) — design ≠ Claimed leaf API"
                );
                ExitCode::SUCCESS
            } else {
                println!("{body}");
                eprintln!("note: {name} Partial local list — design ≠ Claimed");
                ExitCode::SUCCESS
            }
        }
        Err(e) => {
            println!("[]");
            eprintln!("note: {name} Partial empty list ({e}) — design ≠ Claimed; never spawns gh");
            ExitCode::SUCCESS
        }
    }
}

/// Cheap OUT graduation: static SPDX-ish inventory (Partial; not Claimed license API).
fn cmd_licenses(args: &[String]) -> ExitCode {
    let sub = args.first().map(String::as_str).unwrap_or("list");
    match sub {
        "" | "list" | "-h" | "--help" | "help" => {
            if matches!(sub, "-h" | "--help" | "help") {
                eprintln!(
                    "arks hub licenses [list] — common SPDX license ids (Partial local; no gh spawn)"
                );
                return ExitCode::SUCCESS;
            }
            // Small static set — Partial discoverability only (not full SPDX Claimed).
            const IDS: &[&str] = &[
                "MIT",
                "Apache-2.0",
                "BSD-3-Clause",
                "BSD-2-Clause",
                "GPL-3.0-only",
                "GPL-3.0-or-later",
                "LGPL-3.0-only",
                "MPL-2.0",
                "Unlicense",
                "CC0-1.0",
            ];
            for id in IDS {
                println!("{id}");
            }
            eprintln!(
                "note: licenses Partial static inventory — design ≠ Claimed SPDX/forge license API"
            );
            ExitCode::SUCCESS
        }
        other => {
            eprintln!(
                "arks hub licenses {other}: Partial — only `list` wired; depth beyond inventory refused"
            );
            ExitCode::from(2)
        }
    }
}

fn cmd_forge_config(args: &[String]) -> ExitCode {
    if args.iter().any(|a| a == "-h" || a == "--help") {
        eprintln!(
            "arks hub forge-config — show hub client paths (Partial; ≠ arks config / serve config)"
        );
        return ExitCode::SUCCESS;
    }
    println!("api={}", api_base());
    println!("token_file={}", token_path().display());
    println!("data_dir={}", store::HubStore::data_dir().display());
    eprintln!("note: forge-config Partial client facet — design ≠ Claimed");
    ExitCode::SUCCESS
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
struct HubClientState {
    #[serde(default)]
    config: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    aliases: std::collections::BTreeMap<String, String>,
    /// Default `owner/name` for repo_slug when env/--repo unset (IW-HUB-STUBS).
    #[serde(default)]
    default_repo: Option<String>,
}

fn hub_client_state_path() -> PathBuf {
    if let Ok(p) = env::var("ARKS_HUB_CLIENT_STATE") {
        return PathBuf::from(p);
    }
    let home = env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    home.join(".config").join("arks").join("hub-client.json")
}

fn load_hub_client_state() -> HubClientState {
    let p = hub_client_state_path();
    fs::read_to_string(&p)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_hub_client_state(st: &HubClientState) -> Result<(), String> {
    let p = hub_client_state_path();
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let raw = serde_json::to_string_pretty(st).map_err(|e| e.to_string())?;
    fs::write(&p, raw).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn argv(s: &str) -> Vec<String> {
        s.split_whitespace().map(String::from).collect()
    }

    #[test]
    fn platform_only_groups_refuse_exit_2() {
        for g in ["attestation", "codespace", "event"] {
            assert!(
                HUB_OUT_GROUPS.contains(&g),
                "{g} missing from HUB_OUT_GROUPS"
            );
            assert!(!GROUPS.contains(&g), "{g} still listed as a hub group");
            assert_eq!(
                run_from_args(&argv(&format!("{g} list --repo local/x"))),
                ExitCode::from(2),
                "{g} list"
            );
        }
        // E2: Actions groups are promoted into GROUPS and respond to --help with exit 0.
        for g in ["run", "workflow", "cache", "secret", "variable"] {
            assert!(!HUB_OUT_GROUPS.contains(&g), "{g} still in HUB_OUT_GROUPS");
            assert!(GROUPS.contains(&g), "{g} missing from GROUPS");
            assert_eq!(
                run_from_args(&argv(&format!("{g} --help"))),
                ExitCode::SUCCESS,
                "{g} --help"
            );
        }
    }

    /// Q18 rule 12c: `review` refuses without a forge call.
    #[test]
    fn review_group_refuses_exit_2() {
        for leaf in ["review", "review list", "review 1 --approve"] {
            assert_eq!(run_from_args(&argv(leaf)), ExitCode::from(2), "{leaf}");
        }
        assert_eq!(run_from_args(&argv("review --help")), ExitCode::SUCCESS);
        assert!(!GROUPS.contains(&"review"), "review is not a hub group");
    }

    /// Q18 rule 12b: transitional serve exits 2 on bad argv before binding.
    #[test]
    fn hub_serve_bad_argv_exit_2() {
        for leaf in [
            "serve --listen",
            "serve --listen nonsense",
            "serve --listen 127.0.0.1",
            "serve --tls-cert x",
            "serve extra",
            "serve --data",
        ] {
            assert_eq!(run_from_args(&argv(leaf)), ExitCode::from(2), "{leaf}");
        }
        assert_eq!(run_from_args(&argv("serve --help")), ExitCode::SUCCESS);
    }

    #[test]
    fn platform_only_leaves_refuse_exit_2() {
        for leaf in ["release verify v1", "release verify-asset v1 a.bin"] {
            assert_eq!(run_from_args(&argv(leaf)), ExitCode::from(2), "{leaf}");
        }
    }

    /// Formerly `later` in-scope rows are wired leaves now (not OUT, not refused).
    #[test]
    fn formerly_later_rows_are_in_scope_groups() {
        for g in ["ruleset", "rs", "browse"] {
            assert!(!HUB_OUT_GROUPS.contains(&g), "{g} still OUT");
            assert!(GROUPS.contains(&g), "{g} missing from GROUPS");
        }
        // Help works without a forge.
        for leaf in [
            "ruleset --help",
            "pr checks --help",
            "repo gitignore --help",
        ] {
            assert_eq!(run_from_args(&argv(leaf)), ExitCode::SUCCESS, "{leaf}");
        }
    }
}
