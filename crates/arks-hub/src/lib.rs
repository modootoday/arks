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

mod http_min;
mod local_tls_smoke;
pub mod serve;
mod store;

use http_min::{http_exchange, http_exchange_bytes};
use serde_json::{Value, json};
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

pub use serve::{
    ForgeListenRebindSlot, ForgeServeOptions, default_forge_data_dir, forge_serve_scheme,
    serve_forge_api, spawn_forge_api,
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
    "smoke",
];

/// Matrix groups kept as explicit OUT refuse (marketplace / vendor depth).
/// Prefer `cmd_out_refuse` over silent ignore. Graduated local Partials live
/// elsewhere (`licenses`, `cache`, …); extension/copilot stay OUT refuse.
pub const HUB_OUT_GROUPS: &[&str] = &["extension", "copilot"];

pub fn print_help() {
    eprintln!(
        "\
arks hub — forge collaboration (Partial local API; not Claimed)

USAGE:
    arks hub [OPTIONS] <COMMAND>
    arks <COMMAND> …   (flat forge leaves under arks / gh personality)

COMMANDS:
    serve         Local forge API listen (Partial; transitional — prefer arks serve)
    auth          Authenticate (token file + optional /user probe)
    pr            Pull requests (local hub REST Partial)
    issue         Issues (local hub REST Partial)
    repo          Repositories (local hub REST Partial)
    label         Labels (local hub REST Partial)
    release       Releases list|create (local Partial)
    run           Actions runs list|create (local Partial; not CI Claimed)
    workflow      Actions workflows list (local Partial)
    milestone     Milestones list|create (local Partial)
    gist          Gists list|create (local Partial)
    project       Projects list|create (local Partial)
    discussion    Discussions list|create (local Partial)
    secret        Actions secrets list|create meta (local Partial; no value store)
    variable      Actions variables list|create (local Partial)
    ssh-key       User SSH keys list|create (local Partial)
    gpg-key       User GPG keys list|create (local Partial)
    licenses      Common SPDX license ids list (local Partial; no gh)
    cache         Actions cache list (local empty Partial; no gh)
    ruleset       Repo rulesets list (local empty Partial)
    skill         Skills list (local empty Partial)
    preview       Deploy previews list (local empty Partial)
    attestation   Attestations list (local empty Partial)
    codespace     Codespaces list (local empty Partial)
    agent-task    Agent tasks list (local empty Partial)
    browse        Print forge URL (Partial; no browser spawn)
    inbox         Alias → status forge facet (Partial)
    forge-config  Hub client path/token note (Partial; ≠ arks config)
    alias         Local shell alias snippet (Partial)
    completion    Shell completion stub (Partial)
    org           Organizations list (local Partial)
    search        Search repos|issues|prs (local Partial)
    api           Raw HTTP against ARKS_HUB_API
    smoke         HTTPS forge e2e harness Partial (+ --local-tls / --out-refuse; no gh)
    status        Aggregated status (Partial)
    co            Alias for pr checkout
    extension     OUT refuse (marketplace; not local forge)
    copilot       OUT refuse (vendor Copilot; not local forge)
    …             other leaves: see GROUPS / local Partial list helpers

ENV:
    ARKS_HUB_API         default http://127.0.0.1:3920 for local mode
    ARKS_HUB_TOKEN_FILE  token path
    ARKS_HUB_REPO        owner/name
    ARKS_HUB_DATA        serve data dir

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
        "auth" => cmd_auth(&args[1..]),
        "pr" => cmd_pr(&args[1..]),
        "issue" => cmd_issue(&args[1..]),
        "repo" => cmd_repo(&args[1..]),
        "label" => cmd_label(&args[1..]),
        "release" => cmd_release(&args[1..]),
        "run" => cmd_run(&args[1..]),
        "workflow" => cmd_workflow(&args[1..]),
        "milestone" => cmd_milestone(&args[1..]),
        "gist" => cmd_gist(&args[1..]),
        "project" => cmd_project(&args[1..]),
        "discussion" => cmd_discussion(&args[1..]),
        "secret" => cmd_secret(&args[1..]),
        "variable" => cmd_variable(&args[1..]),
        "ssh-key" => cmd_ssh_key(&args[1..]),
        "gpg-key" => cmd_gpg_key(&args[1..]),
        "licenses" => cmd_licenses(&args[1..]),
        "cache" => cmd_out_local_list(
            "cache",
            &args[1..],
            "/repos/{owner}/{repo}/actions/caches",
            true,
        ),
        "ruleset" => cmd_out_local_list(
            "ruleset",
            &args[1..],
            "/repos/{owner}/{repo}/rulesets",
            true,
        ),
        "skill" => cmd_out_local_list("skill", &args[1..], "/user/skills", false),
        "preview" => cmd_out_local_list(
            "preview",
            &args[1..],
            "/repos/{owner}/{repo}/previews",
            true,
        ),
        "attestation" => cmd_out_local_list(
            "attestation",
            &args[1..],
            "/repos/{owner}/{repo}/attestations",
            true,
        ),
        "codespace" => cmd_out_local_list("codespace", &args[1..], "/user/codespaces", false),
        "agent-task" => cmd_out_local_list("agent-task", &args[1..], "/user/agent-tasks", false),
        "browse" => cmd_browse(&args[1..]),
        "inbox" => {
            eprintln!("arks hub inbox → status (demoted alias; Partial)");
            cmd_status(&args[1..])
        }
        "forge-config" => cmd_forge_config(&args[1..]),
        "alias" => cmd_alias(&args[1..]),
        "completion" => cmd_completion(&args[1..]),
        "config" => cmd_config(&args[1..]),
        "org" => cmd_org(&args[1..]),
        "search" => cmd_search(&args[1..]),
        "api" => cmd_api(&args[1..]),
        "smoke" => cmd_https_smoke(&args[1..]),
        "status" => cmd_status(&args[1..]),
        "co" => {
            let mut forwarded = vec!["checkout".into()];
            forwarded.extend(args.iter().cloned());
            cmd_pr(&forwarded)
        }
        "extension" | "copilot" => cmd_out_explicit_refuse(first.as_str(), &args[1..]),
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
    Err("set ARKS_HUB_REPO=owner/name or --repo owner/name".to_string())
}

fn positional_num(args: &[String]) -> Option<String> {
    args.iter()
        .skip(1)
        .find(|a| a.chars().all(|c| c.is_ascii_digit()))
        .cloned()
}

fn hub_http_print(code: u16, body: &str) -> ExitCode {
    if !body.is_empty() {
        println!("{body}");
    }
    if code >= 400 {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

/// Map API prerequisite failures to peer exit classes (`gh` auth-required = 4).
fn exit_from_api_err(ctx: &str, e: &str) -> ExitCode {
    eprintln!("arks hub {ctx}: {e}");
    if e.contains("not logged in") {
        // Peer: https://cli.github.com/manual/gh_help_exit-codes — auth required → 4.
        ExitCode::from(4)
    } else {
        ExitCode::from(2)
    }
}

fn cmd_auth(args: &[String]) -> ExitCode {
    let sub = args.first().map(String::as_str).unwrap_or("");
    match sub {
        "" | "-h" | "--help" => {
            eprintln!(
                "arks hub auth — login|logout|status|token\n\
                 Local: store token; status probes GET /user on ARKS_HUB_API when http local."
            );
            ExitCode::SUCCESS
        }
        "login" => {
            let token = if let Ok(t) = env::var("ARKS_HUB_TOKEN") {
                t
            } else if args.iter().any(|a| a == "--with-token") {
                let mut buf = String::new();
                if io::stdin().read_line(&mut buf).is_err() || buf.trim().is_empty() {
                    eprintln!("arks hub auth login: expected token on stdin with --with-token");
                    return ExitCode::from(2);
                }
                buf.trim().to_string()
            } else {
                "local-dev-token".into()
            };
            let path = token_path();
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            if let Err(e) = fs::write(&path, format!("{token}\n")) {
                eprintln!("arks hub auth login: write {}: {e}", path.display());
                return ExitCode::from(1);
            }
            let base = api_base();
            if is_local_http(&base) {
                let body = format!(r#"{{"token":"{token}"}}"#);
                match http_exchange(&base, "POST", "/login", Some(&token), Some(&body)) {
                    Ok((code, resp)) => {
                        eprintln!("arks hub auth login: local /login → HTTP {code}");
                        if code >= 400 {
                            eprintln!("{resp}");
                        }
                    }
                    Err(e) => eprintln!(
                        "arks hub auth login: token stored; local /login skipped ({e}) — start forge serve?"
                    ),
                }
            }
            eprintln!("arks hub auth login: token stored at {}", path.display());
            ExitCode::SUCCESS
        }
        "logout" => {
            let path = token_path();
            let _ = fs::remove_file(&path);
            eprintln!("arks hub auth logout: cleared {}", path.display());
            ExitCode::SUCCESS
        }
        "status" => match read_token() {
            Some(tok) => {
                println!(
                    "arks hub auth status: token present (file={})",
                    token_path().display()
                );
                let base = api_base();
                println!("api={base}");
                if is_local_http(&base) {
                    match http_exchange(&base, "GET", "/user", Some(&tok), None) {
                        Ok((200, body)) => {
                            println!("user_probe=ok");
                            println!("{body}");
                        }
                        Ok((code, body)) => {
                            println!("user_probe=http_{code}");
                            println!("{body}");
                        }
                        Err(e) => {
                            println!("user_probe=error");
                            eprintln!("{e}");
                        }
                    }
                } else {
                    println!("note: non-local API probe not Claimed (Partial)");
                }
                ExitCode::SUCCESS
            }
            None => {
                println!("arks hub auth status: logged out (no token file)");
                ExitCode::SUCCESS
            }
        },
        "token" => match read_token() {
            Some(t) if args.iter().any(|a| a == "--show") => {
                println!("{t}");
                ExitCode::SUCCESS
            }
            Some(_) => {
                println!("***");
                ExitCode::SUCCESS
            }
            None => {
                eprintln!("arks hub auth token: no token stored");
                ExitCode::from(1)
            }
        },
        "refresh" | "switch" | "setup-git" => {
            eprintln!("arks hub auth {sub}: not claimed");
            ExitCode::from(2)
        }
        other => {
            eprintln!("arks hub auth: unknown subcommand `{other}`");
            ExitCode::from(2)
        }
    }
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

/// Backward-compatible alias — local http:// still preferred for Partial forge.
fn require_local_api() -> Result<(String, String), String> {
    require_api()
}

fn cmd_pr(args: &[String]) -> ExitCode {
    let sub = args.first().map(String::as_str).unwrap_or("");
    match sub {
        "" | "-h" | "--help" => {
            eprintln!(
                "arks hub pr — list|view|create|close|reopen|edit|comment|merge|checkout \
                 (local hub REST; merge=metadata only; checkout=resolve/print, never spawn git)"
            );
            ExitCode::SUCCESS
        }
        "checkout" => cmd_pr_checkout(args),
        "list" => {
            let (base, tok) = match require_local_api() {
                Ok(v) => v,
                Err(e) => return exit_from_api_err("pr list", &e),
            };
            let (owner, repo) = match repo_slug(args) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("arks hub pr list: {e}");
                    return ExitCode::from(2);
                }
            };
            let path = format!("/repos/{owner}/{repo}/pulls");
            match http_exchange(&base, "GET", &path, Some(&tok), None) {
                Ok((code, body)) => hub_http_print(code, &body),
                Err(e) => {
                    eprintln!("arks hub pr list: {e}");
                    ExitCode::from(1)
                }
            }
        }
        "view" => {
            let (base, tok) = match require_local_api() {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("arks hub pr view: {e}");
                    return ExitCode::from(2);
                }
            };
            let (owner, repo) = match repo_slug(args) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("arks hub pr view: {e}");
                    return ExitCode::from(2);
                }
            };
            let num = args
                .iter()
                .skip(1)
                .find(|a| a.chars().all(|c| c.is_ascii_digit()))
                .cloned()
                .unwrap_or_else(|| "1".into());
            let path = format!("/repos/{owner}/{repo}/pulls/{num}");
            match http_exchange(&base, "GET", &path, Some(&tok), None) {
                Ok((code, body)) => {
                    println!("{body}");
                    if code >= 400 {
                        ExitCode::from(1)
                    } else {
                        ExitCode::SUCCESS
                    }
                }
                Err(e) => {
                    eprintln!("arks hub pr view: {e}");
                    ExitCode::from(1)
                }
            }
        }
        "create" => {
            let (base, tok) = match require_local_api() {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("arks hub pr create: {e}");
                    return ExitCode::from(2);
                }
            };
            let (owner, repo) = match repo_slug(args) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("arks hub pr create: {e}");
                    return ExitCode::from(2);
                }
            };
            let title = flag_value(args, &["--title", "-t"]).unwrap_or("pull request");
            let body = flag_value(args, &["--body", "-b"]).unwrap_or("");
            let head = flag_value(args, &["--head"]).unwrap_or("head");
            let base_ref = flag_value(args, &["--base"]).unwrap_or("main");
            let payload = format!(
                r#"{{"title":{},"body":{},"head":{},"base":{}}}"#,
                serde_json::to_string(title).unwrap(),
                serde_json::to_string(body).unwrap(),
                serde_json::to_string(head).unwrap(),
                serde_json::to_string(base_ref).unwrap()
            );
            let path = format!("/repos/{owner}/{repo}/pulls");
            match http_exchange(&base, "POST", &path, Some(&tok), Some(&payload)) {
                Ok((code, body)) => hub_http_print(code, &body),
                Err(e) => {
                    eprintln!("arks hub pr create: {e}");
                    ExitCode::from(1)
                }
            }
        }
        "close" | "reopen" | "edit" => {
            let (base, tok) = match require_local_api() {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("arks hub pr {sub}: {e}");
                    return ExitCode::from(2);
                }
            };
            let (owner, repo) = match repo_slug(args) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("arks hub pr {sub}: {e}");
                    return ExitCode::from(2);
                }
            };
            let num = positional_num(args).unwrap_or_else(|| "1".into());
            let payload = match sub {
                "close" => r#"{"state":"closed"}"#.to_string(),
                "reopen" => r#"{"state":"open"}"#.to_string(),
                _ => {
                    let title = flag_value(args, &["--title", "-t"]);
                    let body = flag_value(args, &["--body", "-b"]);
                    let mut parts = Vec::new();
                    if let Some(t) = title {
                        parts.push(format!("\"title\":{}", serde_json::to_string(t).unwrap()));
                    }
                    if let Some(b) = body {
                        parts.push(format!("\"body\":{}", serde_json::to_string(b).unwrap()));
                    }
                    if parts.is_empty() {
                        parts.push(format!(
                            "\"title\":{}",
                            serde_json::to_string("edited").unwrap()
                        ));
                    }
                    format!("{{{}}}", parts.join(","))
                }
            };
            let path = format!("/repos/{owner}/{repo}/pulls/{num}");
            match http_exchange(&base, "PATCH", &path, Some(&tok), Some(&payload)) {
                Ok((code, body)) => hub_http_print(code, &body),
                Err(e) => {
                    eprintln!("arks hub pr {sub}: {e}");
                    ExitCode::from(1)
                }
            }
        }
        "comment" => {
            let (base, tok) = match require_local_api() {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("arks hub pr comment: {e}");
                    return ExitCode::from(2);
                }
            };
            let (owner, repo) = match repo_slug(args) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("arks hub pr comment: {e}");
                    return ExitCode::from(2);
                }
            };
            let num = positional_num(args).unwrap_or_else(|| "1".into());
            let body = flag_value(args, &["--body", "-b"]).unwrap_or("comment");
            let payload = format!(r#"{{"body":{}}}"#, serde_json::to_string(body).unwrap());
            let path = format!("/repos/{owner}/{repo}/issues/{num}/comments");
            match http_exchange(&base, "POST", &path, Some(&tok), Some(&payload)) {
                Ok((code, body)) => hub_http_print(code, &body),
                Err(e) => {
                    eprintln!("arks hub pr comment: {e}");
                    ExitCode::from(1)
                }
            }
        }
        "merge" => {
            let (base, tok) = match require_local_api() {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("arks hub pr merge: {e}");
                    return ExitCode::from(2);
                }
            };
            let (owner, repo) = match repo_slug(args) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("arks hub pr merge: {e}");
                    return ExitCode::from(2);
                }
            };
            let num = positional_num(args).unwrap_or_else(|| "1".into());
            let path = format!("/repos/{owner}/{repo}/pulls/{num}/merge");
            match http_exchange(&base, "PUT", &path, Some(&tok), Some("{}")) {
                Ok((code, body)) => hub_http_print(code, &body),
                Err(e) => {
                    eprintln!("arks hub pr merge: {e}");
                    ExitCode::from(1)
                }
            }
        }
        other => {
            eprintln!(
                "arks hub pr {other}: Partial stub (not Claimed); refuse exit 2 — \
                 [[sot:root/arks-in-scope-leftovers-wave]]"
            );
            ExitCode::from(2)
        }
    }
}

/// Resolve PR head and print checkout recipe (IW-HUB-CHECKOUT). Never spawns git/gh.
fn cmd_pr_checkout(args: &[String]) -> ExitCode {
    let (base, tok) = match require_local_api() {
        Ok(v) => v,
        Err(e) => return exit_from_api_err("pr checkout", &e),
    };
    let (owner, repo) = match repo_slug(args) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("arks hub pr checkout: {e}");
            return ExitCode::from(2);
        }
    };
    let num = positional_num(args).unwrap_or_else(|| "1".into());
    let path = format!("/repos/{owner}/{repo}/pulls/{num}");
    match http_exchange(&base, "GET", &path, Some(&tok), None) {
        Ok((code, body)) if code < 400 => {
            let v: Value = serde_json::from_str(&body).unwrap_or(json!({}));
            let head = v
                .get("head")
                .and_then(|h| {
                    h.as_object()
                        .and_then(|o| o.get("ref"))
                        .and_then(|x| x.as_str())
                        .or_else(|| h.as_str())
                })
                .unwrap_or("head");
            let sha = v
                .get("head")
                .and_then(|h| h.get("sha"))
                .and_then(|x| x.as_str())
                .unwrap_or("");
            let number = v.get("number").and_then(|x| x.as_u64()).unwrap_or(0);
            println!("pr={number}");
            println!("head_ref={head}");
            if !sha.is_empty() {
                println!("head_sha={sha}");
            }
            println!("recipe=arks fetch origin pull/{number}/head:{head} && arks checkout {head}");
            eprintln!(
                "note: pr checkout Claimed resolve/print only — never spawn git/gh \
                 ([[sot:root/arks-in-scope-leftovers-wave]])"
            );
            ExitCode::SUCCESS
        }
        Ok((code, body)) => hub_http_print(code, &body),
        Err(e) => {
            eprintln!("arks hub pr checkout: {e}");
            ExitCode::from(1)
        }
    }
}

fn cmd_issue(args: &[String]) -> ExitCode {
    let sub = args.first().map(String::as_str).unwrap_or("");
    match sub {
        "" | "-h" | "--help" => {
            eprintln!(
                "arks hub issue — list|view|create|close|reopen|edit|comment (local hub REST)"
            );
            ExitCode::SUCCESS
        }
        "list" => {
            let (base, tok) = match require_local_api() {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("arks hub issue list: {e}");
                    return ExitCode::from(2);
                }
            };
            let (owner, repo) = match repo_slug(args) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("arks hub issue list: {e}");
                    return ExitCode::from(2);
                }
            };
            let path = format!("/repos/{owner}/{repo}/issues");
            match http_exchange(&base, "GET", &path, Some(&tok), None) {
                Ok((code, body)) => {
                    println!("{body}");
                    if code >= 400 {
                        ExitCode::from(1)
                    } else {
                        ExitCode::SUCCESS
                    }
                }
                Err(e) => {
                    eprintln!("arks hub issue list: {e}");
                    ExitCode::from(1)
                }
            }
        }
        "view" => {
            let (base, tok) = match require_local_api() {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("arks hub issue view: {e}");
                    return ExitCode::from(2);
                }
            };
            let (owner, repo) = match repo_slug(args) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("arks hub issue view: {e}");
                    return ExitCode::from(2);
                }
            };
            let num = args
                .iter()
                .skip(1)
                .find(|a| a.chars().all(|c| c.is_ascii_digit()))
                .cloned()
                .unwrap_or_else(|| "1".into());
            let path = format!("/repos/{owner}/{repo}/issues/{num}");
            match http_exchange(&base, "GET", &path, Some(&tok), None) {
                Ok((code, body)) => {
                    println!("{body}");
                    if code >= 400 {
                        ExitCode::from(1)
                    } else {
                        ExitCode::SUCCESS
                    }
                }
                Err(e) => {
                    eprintln!("arks hub issue view: {e}");
                    ExitCode::from(1)
                }
            }
        }
        "create" => {
            let (base, tok) = match require_local_api() {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("arks hub issue create: {e}");
                    return ExitCode::from(2);
                }
            };
            let (owner, repo) = match repo_slug(args) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("arks hub issue create: {e}");
                    return ExitCode::from(2);
                }
            };
            let title = flag_value(args, &["--title", "-t"]).unwrap_or("issue");
            let body = flag_value(args, &["--body", "-b"]).unwrap_or("");
            let payload = format!(
                r#"{{"title":{},"body":{}}}"#,
                serde_json::to_string(title).unwrap(),
                serde_json::to_string(body).unwrap()
            );
            let path = format!("/repos/{owner}/{repo}/issues");
            match http_exchange(&base, "POST", &path, Some(&tok), Some(&payload)) {
                Ok((code, body)) => hub_http_print(code, &body),
                Err(e) => {
                    eprintln!("arks hub issue create: {e}");
                    ExitCode::from(1)
                }
            }
        }
        "close" | "reopen" | "edit" => {
            let (base, tok) = match require_local_api() {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("arks hub issue {sub}: {e}");
                    return ExitCode::from(2);
                }
            };
            let (owner, repo) = match repo_slug(args) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("arks hub issue {sub}: {e}");
                    return ExitCode::from(2);
                }
            };
            let num = positional_num(args).unwrap_or_else(|| "1".into());
            let payload = match sub {
                "close" => r#"{"state":"closed"}"#.to_string(),
                "reopen" => r#"{"state":"open"}"#.to_string(),
                _ => {
                    let title = flag_value(args, &["--title", "-t"]);
                    let body = flag_value(args, &["--body", "-b"]);
                    let mut parts = Vec::new();
                    if let Some(t) = title {
                        parts.push(format!("\"title\":{}", serde_json::to_string(t).unwrap()));
                    }
                    if let Some(b) = body {
                        parts.push(format!("\"body\":{}", serde_json::to_string(b).unwrap()));
                    }
                    if parts.is_empty() {
                        parts.push(format!(
                            "\"title\":{}",
                            serde_json::to_string("edited").unwrap()
                        ));
                    }
                    format!("{{{}}}", parts.join(","))
                }
            };
            let path = format!("/repos/{owner}/{repo}/issues/{num}");
            match http_exchange(&base, "PATCH", &path, Some(&tok), Some(&payload)) {
                Ok((code, body)) => hub_http_print(code, &body),
                Err(e) => {
                    eprintln!("arks hub issue {sub}: {e}");
                    ExitCode::from(1)
                }
            }
        }
        "comment" => {
            let (base, tok) = match require_local_api() {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("arks hub issue comment: {e}");
                    return ExitCode::from(2);
                }
            };
            let (owner, repo) = match repo_slug(args) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("arks hub issue comment: {e}");
                    return ExitCode::from(2);
                }
            };
            let num = positional_num(args).unwrap_or_else(|| "1".into());
            let body = flag_value(args, &["--body", "-b"]).unwrap_or("comment");
            let payload = format!(r#"{{"body":{}}}"#, serde_json::to_string(body).unwrap());
            let path = format!("/repos/{owner}/{repo}/issues/{num}/comments");
            match http_exchange(&base, "POST", &path, Some(&tok), Some(&payload)) {
                Ok((code, body)) => hub_http_print(code, &body),
                Err(e) => {
                    eprintln!("arks hub issue comment: {e}");
                    ExitCode::from(1)
                }
            }
        }
        other => {
            eprintln!(
                "arks hub issue {other}: Partial stub (not Claimed); refuse exit 2 — \
                 [[sot:root/arks-honesty-leftovers-wave]]"
            );
            ExitCode::from(2)
        }
    }
}

fn cmd_repo(args: &[String]) -> ExitCode {
    let sub = args.first().map(String::as_str).unwrap_or("");
    match sub {
        "" | "-h" | "--help" => {
            eprintln!(
                "arks hub repo — list|view|create (local hub REST Partial; never spawn git/gh)"
            );
            ExitCode::SUCCESS
        }
        "list" => {
            let (base, tok) = match require_local_api() {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("arks hub repo list: {e}");
                    return ExitCode::from(2);
                }
            };
            match http_exchange(&base, "GET", "/user/repos", Some(&tok), None) {
                Ok((code, body)) => {
                    println!("{body}");
                    if code >= 400 {
                        ExitCode::from(1)
                    } else {
                        ExitCode::SUCCESS
                    }
                }
                Err(e) => {
                    eprintln!("arks hub repo list: {e}");
                    ExitCode::from(1)
                }
            }
        }
        "view" => {
            let (base, tok) = match require_local_api() {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("arks hub repo view: {e}");
                    return ExitCode::from(2);
                }
            };
            let (owner, repo) = match repo_slug(args) {
                Ok(v) => v,
                Err(e) => {
                    let pos = args.iter().skip(1).find(|a| !a.starts_with('-'));
                    match pos.and_then(|s| s.split_once('/')) {
                        Some((o, n)) => (o.to_string(), n.to_string()),
                        None => {
                            eprintln!("arks hub repo view: {e}");
                            return ExitCode::from(2);
                        }
                    }
                }
            };
            let path = format!("/repos/{owner}/{repo}");
            match http_exchange(&base, "GET", &path, Some(&tok), None) {
                Ok((code, body)) => {
                    println!("{body}");
                    if code >= 400 {
                        ExitCode::from(1)
                    } else {
                        ExitCode::SUCCESS
                    }
                }
                Err(e) => {
                    eprintln!("arks hub repo view: {e}");
                    ExitCode::from(1)
                }
            }
        }
        "create" => {
            let (base, tok) = match require_local_api() {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("arks hub repo create: {e}");
                    return ExitCode::from(2);
                }
            };
            let name = args
                .iter()
                .skip(1)
                .find(|a| !a.starts_with('-'))
                .map(String::as_str)
                .unwrap_or("repo");
            let desc = flag_value(args, &["--description", "-d"]).unwrap_or("");
            let payload = format!(
                r#"{{"name":{},"description":{}}}"#,
                serde_json::to_string(name).unwrap(),
                serde_json::to_string(desc).unwrap()
            );
            match http_exchange(&base, "POST", "/user/repos", Some(&tok), Some(&payload)) {
                Ok((code, body)) => {
                    println!("{body}");
                    if code >= 400 {
                        ExitCode::from(1)
                    } else {
                        ExitCode::SUCCESS
                    }
                }
                Err(e) => {
                    eprintln!("arks hub repo create: {e}");
                    ExitCode::from(1)
                }
            }
        }
        "clone" | "sync" => {
            eprintln!("arks hub repo {sub}: Deferred — MUST NOT spawn git/gh");
            ExitCode::from(2)
        }
        other => {
            eprintln!("arks hub repo {other}: not claimed beyond list/view/create Partial");
            ExitCode::from(2)
        }
    }
}

fn cmd_label(args: &[String]) -> ExitCode {
    let sub = args.first().map(String::as_str).unwrap_or("");
    match sub {
        "" | "-h" | "--help" => {
            eprintln!("arks hub label — list|create|edit|delete (local hub REST)");
            ExitCode::SUCCESS
        }
        "list" => {
            let (base, tok) = match require_local_api() {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("arks hub label list: {e}");
                    return ExitCode::from(2);
                }
            };
            let (owner, repo) = match repo_slug(args) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("arks hub label list: {e}");
                    return ExitCode::from(2);
                }
            };
            let path = format!("/repos/{owner}/{repo}/labels");
            match http_exchange(&base, "GET", &path, Some(&tok), None) {
                Ok((code, body)) => {
                    println!("{body}");
                    if code >= 400 {
                        ExitCode::from(1)
                    } else {
                        ExitCode::SUCCESS
                    }
                }
                Err(e) => {
                    eprintln!("arks hub label list: {e}");
                    ExitCode::from(1)
                }
            }
        }
        "create" => {
            let (base, tok) = match require_local_api() {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("arks hub label create: {e}");
                    return ExitCode::from(2);
                }
            };
            let (owner, repo) = match repo_slug(args) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("arks hub label create: {e}");
                    return ExitCode::from(2);
                }
            };
            let name = flag_value(args, &["--name", "-n"])
                .or_else(|| {
                    args.iter()
                        .skip(1)
                        .find(|a| !a.starts_with('-'))
                        .map(String::as_str)
                })
                .unwrap_or("label");
            let color = flag_value(args, &["--color"]).unwrap_or("ededed");
            let desc = flag_value(args, &["--description", "-d"]).unwrap_or("");
            let payload = format!(
                r#"{{"name":{},"color":{},"description":{}}}"#,
                serde_json::to_string(name).unwrap(),
                serde_json::to_string(color).unwrap(),
                serde_json::to_string(desc).unwrap()
            );
            let path = format!("/repos/{owner}/{repo}/labels");
            match http_exchange(&base, "POST", &path, Some(&tok), Some(&payload)) {
                Ok((code, body)) => hub_http_print(code, &body),
                Err(e) => {
                    eprintln!("arks hub label create: {e}");
                    ExitCode::from(1)
                }
            }
        }
        "edit" => {
            let (base, tok) = match require_local_api() {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("arks hub label edit: {e}");
                    return ExitCode::from(2);
                }
            };
            let (owner, repo) = match repo_slug(args) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("arks hub label edit: {e}");
                    return ExitCode::from(2);
                }
            };
            let name = flag_value(args, &["--name", "-n"])
                .or_else(|| {
                    args.iter()
                        .skip(1)
                        .find(|a| !a.starts_with('-') && !a.contains('/'))
                        .map(String::as_str)
                })
                .unwrap_or("label");
            let new_name = flag_value(args, &["--new-name"]);
            let color = flag_value(args, &["--color"]);
            let desc = flag_value(args, &["--description", "-d"]);
            let mut parts = Vec::new();
            if let Some(n) = new_name {
                parts.push(format!("\"new_name\":{}", serde_json::to_string(n).unwrap()));
            }
            if let Some(c) = color {
                parts.push(format!("\"color\":{}", serde_json::to_string(c).unwrap()));
            }
            if let Some(d) = desc {
                parts.push(format!(
                    "\"description\":{}",
                    serde_json::to_string(d).unwrap()
                ));
            }
            if parts.is_empty() {
                parts.push(format!(
                    "\"description\":{}",
                    serde_json::to_string("edited").unwrap()
                ));
            }
            let payload = format!("{{{}}}", parts.join(","));
            let path = format!("/repos/{owner}/{repo}/labels/{name}");
            match http_exchange(&base, "PATCH", &path, Some(&tok), Some(&payload)) {
                Ok((code, body)) => hub_http_print(code, &body),
                Err(e) => {
                    eprintln!("arks hub label edit: {e}");
                    ExitCode::from(1)
                }
            }
        }
        "delete" => {
            let (base, tok) = match require_local_api() {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("arks hub label delete: {e}");
                    return ExitCode::from(2);
                }
            };
            let (owner, repo) = match repo_slug(args) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("arks hub label delete: {e}");
                    return ExitCode::from(2);
                }
            };
            let name = flag_value(args, &["--name", "-n"])
                .or_else(|| {
                    args.iter()
                        .skip(1)
                        .find(|a| !a.starts_with('-') && !a.contains('/'))
                        .map(String::as_str)
                })
                .unwrap_or("label");
            let path = format!("/repos/{owner}/{repo}/labels/{name}");
            match http_exchange(&base, "DELETE", &path, Some(&tok), None) {
                Ok((code, body)) => hub_http_print(code, &body),
                Err(e) => {
                    eprintln!("arks hub label delete: {e}");
                    ExitCode::from(1)
                }
            }
        }
        other => {
            eprintln!(
                "arks hub label {other}: Partial stub (not Claimed); refuse exit 2 — \
                 [[sot:root/arks-honesty-leftovers-wave]]"
            );
            ExitCode::from(2)
        }
    }
}

fn cmd_org(args: &[String]) -> ExitCode {
    let sub = args.first().map(String::as_str).unwrap_or("list");
    match sub {
        "" | "list" | "-h" | "--help" => {
            if matches!(sub, "-h" | "--help") {
                eprintln!("arks hub org — list (local Partial; unique owners from /orgs)");
                return ExitCode::SUCCESS;
            }
            let (base, tok) = match require_local_api() {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("arks hub org list: {e}");
                    return ExitCode::from(2);
                }
            };
            match http_exchange(&base, "GET", "/orgs", Some(&tok), None) {
                Ok((code, body)) => {
                    println!("{body}");
                    if code >= 400 {
                        ExitCode::from(1)
                    } else {
                        ExitCode::SUCCESS
                    }
                }
                Err(e) => {
                    eprintln!("arks hub org list: {e}");
                    ExitCode::from(1)
                }
            }
        }
        other => {
            eprintln!("arks hub org {other}: not claimed beyond list Partial");
            ExitCode::from(2)
        }
    }
}

fn cmd_search(args: &[String]) -> ExitCode {
    let sub = args.first().map(String::as_str).unwrap_or("");
    match sub {
        "" | "-h" | "--help" => {
            eprintln!("arks hub search — repos|issues|prs <query> (local Partial)");
            ExitCode::SUCCESS
        }
        "repos" | "issues" | "prs" | "pulls" => {
            let kind = if sub == "prs" { "pulls" } else { sub };
            let q = args
                .iter()
                .skip(1)
                .find(|a| !a.starts_with('-'))
                .cloned()
                .unwrap_or_default();
            let (base, tok) = match require_local_api() {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("arks hub search: {e}");
                    return ExitCode::from(2);
                }
            };
            let enc: String = q
                .bytes()
                .map(|b| match b {
                    b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' => {
                        (b as char).to_string()
                    }
                    b' ' => "+".into(),
                    _ => format!("%{b:02X}"),
                })
                .collect();
            let path = format!("/search/{kind}?q={enc}");
            match http_exchange(&base, "GET", &path, Some(&tok), None) {
                Ok((code, body)) => {
                    println!("{body}");
                    if code >= 400 {
                        ExitCode::from(1)
                    } else {
                        ExitCode::SUCCESS
                    }
                }
                Err(e) => {
                    eprintln!("arks hub search: {e}");
                    ExitCode::from(1)
                }
            }
        }
        other => {
            eprintln!("arks hub search {other}: use repos|issues|prs (Partial)");
            ExitCode::from(2)
        }
    }
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
             --out-refuse: residual OUT harness Partial (extension/copilot exit 2; never gh).\n  \
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

fn cmd_api(args: &[String]) -> ExitCode {
    if args.is_empty() || args.iter().any(|a| a == "-h" || a == "--help") {
        eprintln!("arks hub api <path> — HTTP against ARKS_HUB_API (local http Partial)");
        return ExitCode::SUCCESS;
    }
    let path = args
        .iter()
        .find(|a| !a.starts_with('-'))
        .cloned()
        .unwrap_or_else(|| "/".into());
    let path = if path.starts_with('/') {
        path
    } else {
        format!("/{path}")
    };
    let (base, tok) = match require_local_api() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("arks hub api: {e}");
            return ExitCode::from(2);
        }
    };
    match http_exchange(&base, "GET", &path, Some(&tok), None) {
        Ok((code, body)) => {
            println!("http_status={code}");
            println!("{body}");
            if code >= 400 {
                ExitCode::from(1)
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(e) => {
            eprintln!("arks hub api: {e}");
            ExitCode::from(1)
        }
    }
}

fn cmd_release(args: &[String]) -> ExitCode {
    let sub = args.first().map(String::as_str).unwrap_or("");
    match sub {
        "" | "-h" | "--help" => {
            eprintln!(
                "arks hub release — list|create|view|edit|delete|upload|download|delete-asset \
                 (local hub REST; assets = local-forge bytes)"
            );
            ExitCode::SUCCESS
        }
        "list" => {
            let (base, tok) = match require_local_api() {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("arks hub release list: {e}");
                    return ExitCode::from(2);
                }
            };
            let (owner, repo) = match repo_slug(args) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("arks hub release list: {e}");
                    return ExitCode::from(2);
                }
            };
            let path = format!("/repos/{owner}/{repo}/releases");
            match http_exchange(&base, "GET", &path, Some(&tok), None) {
                Ok((code, body)) => {
                    println!("{body}");
                    if code >= 400 {
                        ExitCode::from(1)
                    } else {
                        ExitCode::SUCCESS
                    }
                }
                Err(e) => {
                    eprintln!("arks hub release list: {e}");
                    ExitCode::from(1)
                }
            }
        }
        "create" => {
            let (base, tok) = match require_local_api() {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("arks hub release create: {e}");
                    return ExitCode::from(2);
                }
            };
            let (owner, repo) = match repo_slug(args) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("arks hub release create: {e}");
                    return ExitCode::from(2);
                }
            };
            let tag = flag_value(args, &["--tag", "-t"]).unwrap_or("v0.0.0");
            let name = flag_value(args, &["--title", "--name"]).unwrap_or(tag);
            let body = flag_value(args, &["--notes", "-n", "--body"]).unwrap_or("");
            let payload = format!(
                r#"{{"tag_name":{},"name":{},"body":{}}}"#,
                serde_json::to_string(tag).unwrap(),
                serde_json::to_string(name).unwrap(),
                serde_json::to_string(body).unwrap()
            );
            let path = format!("/repos/{owner}/{repo}/releases");
            match http_exchange(&base, "POST", &path, Some(&tok), Some(&payload)) {
                Ok((code, body)) => hub_http_print(code, &body),
                Err(e) => {
                    eprintln!("arks hub release create: {e}");
                    ExitCode::from(1)
                }
            }
        }
        "view" => {
            let (base, tok) = match require_local_api() {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("arks hub release view: {e}");
                    return ExitCode::from(2);
                }
            };
            let (owner, repo) = match repo_slug(args) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("arks hub release view: {e}");
                    return ExitCode::from(2);
                }
            };
            let tag = flag_value(args, &["--tag", "-t"])
                .or_else(|| {
                    args.iter()
                        .skip(1)
                        .find(|a| !a.starts_with('-') && !a.contains('/'))
                        .map(String::as_str)
                })
                .unwrap_or("v0.0.0");
            let path = if tag.chars().all(|c| c.is_ascii_digit()) {
                format!("/repos/{owner}/{repo}/releases/{tag}")
            } else {
                format!("/repos/{owner}/{repo}/releases/tags/{tag}")
            };
            match http_exchange(&base, "GET", &path, Some(&tok), None) {
                Ok((code, body)) => hub_http_print(code, &body),
                Err(e) => {
                    eprintln!("arks hub release view: {e}");
                    ExitCode::from(1)
                }
            }
        }
        "edit" => {
            let (base, tok) = match require_local_api() {
                Ok(v) => v,
                Err(e) => return exit_from_api_err("release edit", &e),
            };
            let (owner, repo) = match repo_slug(args) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("arks hub release edit: {e}");
                    return ExitCode::from(2);
                }
            };
            let tag = flag_value(args, &["--tag", "-t"]).unwrap_or("v0.0.0");
            let title = flag_value(args, &["--title", "--name"]);
            let notes = flag_value(args, &["--notes", "-n", "--body"]);
            let mut obj = serde_json::Map::new();
            if let Some(t) = title {
                obj.insert("name".into(), Value::String(t.into()));
            }
            if let Some(n) = notes {
                obj.insert("body".into(), Value::String(n.into()));
            }
            let payload = Value::Object(obj).to_string();
            let path = if tag.chars().all(|c| c.is_ascii_digit()) {
                format!("/repos/{owner}/{repo}/releases/{tag}")
            } else {
                // Prefer id path via view-by-tag then patch id — try tags first via GET.
                match http_exchange(
                    &base,
                    "GET",
                    &format!("/repos/{owner}/{repo}/releases/tags/{tag}"),
                    Some(&tok),
                    None,
                ) {
                    Ok((c, b)) if c < 400 => {
                        let id = serde_json::from_str::<Value>(&b)
                            .ok()
                            .and_then(|v| v.get("id").and_then(|x| x.as_u64()))
                            .unwrap_or(0);
                        format!("/repos/{owner}/{repo}/releases/{id}")
                    }
                    _ => format!("/repos/{owner}/{repo}/releases/{tag}"),
                }
            };
            match http_exchange(&base, "PATCH", &path, Some(&tok), Some(&payload)) {
                Ok((code, body)) => hub_http_print(code, &body),
                Err(e) => {
                    eprintln!("arks hub release edit: {e}");
                    ExitCode::from(1)
                }
            }
        }
        "delete" => {
            let (base, tok) = match require_local_api() {
                Ok(v) => v,
                Err(e) => return exit_from_api_err("release delete", &e),
            };
            let (owner, repo) = match repo_slug(args) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("arks hub release delete: {e}");
                    return ExitCode::from(2);
                }
            };
            let tag = flag_value(args, &["--tag", "-t"]).unwrap_or("v0.0.0");
            let path = match http_exchange(
                &base,
                "GET",
                &format!("/repos/{owner}/{repo}/releases/tags/{tag}"),
                Some(&tok),
                None,
            ) {
                Ok((c, b)) if c < 400 => {
                    let id = serde_json::from_str::<Value>(&b)
                        .ok()
                        .and_then(|v| v.get("id").and_then(|x| x.as_u64()))
                        .unwrap_or(0);
                    format!("/repos/{owner}/{repo}/releases/{id}")
                }
                _ => format!("/repos/{owner}/{repo}/releases/{tag}"),
            };
            match http_exchange(&base, "DELETE", &path, Some(&tok), None) {
                Ok((code, body)) => hub_http_print(code, &body),
                Err(e) => {
                    eprintln!("arks hub release delete: {e}");
                    ExitCode::from(1)
                }
            }
        }
        "upload" => {
            let (base, tok) = match require_local_api() {
                Ok(v) => v,
                Err(e) => return exit_from_api_err("release upload", &e),
            };
            let (owner, repo) = match repo_slug(args) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("arks hub release upload: {e}");
                    return ExitCode::from(2);
                }
            };
            let tag = flag_value(args, &["--tag", "-t"]).unwrap_or("v0.0.0");
            let file = flag_value(args, &["--file", "-f"]).unwrap_or("");
            if file.is_empty() {
                eprintln!("arks hub release upload: --file required");
                return ExitCode::from(2);
            }
            let name = flag_value(args, &["--name", "-n"])
                .map(str::to_string)
                .unwrap_or_else(|| {
                    std::path::Path::new(file)
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("asset.bin")
                        .to_string()
                });
            let bytes = match fs::read(file) {
                Ok(b) => b,
                Err(e) => {
                    eprintln!("arks hub release upload: read {file}: {e}");
                    return ExitCode::from(1);
                }
            };
            let rel_body = match http_exchange(
                &base,
                "GET",
                &format!("/repos/{owner}/{repo}/releases/tags/{tag}"),
                Some(&tok),
                None,
            ) {
                Ok((c, b)) if c < 400 => b,
                Ok((c, b)) => return hub_http_print(c, &b),
                Err(e) => {
                    eprintln!("arks hub release upload: {e}");
                    return ExitCode::from(1);
                }
            };
            let id = serde_json::from_str::<Value>(&rel_body)
                .ok()
                .and_then(|v| v.get("id").and_then(|x| x.as_u64()))
                .unwrap_or(0);
            let path = format!("/repos/{owner}/{repo}/releases/{id}/assets?name={name}");
            match http_exchange_bytes(
                &base,
                "POST",
                &path,
                Some(&tok),
                Some(&bytes),
                "application/octet-stream",
                "application/json",
            ) {
                Ok((code, body)) => {
                    let text = String::from_utf8_lossy(&body);
                    hub_http_print(code, &text)
                }
                Err(e) => {
                    eprintln!("arks hub release upload: {e}");
                    ExitCode::from(1)
                }
            }
        }
        "download" => {
            let (base, tok) = match require_local_api() {
                Ok(v) => v,
                Err(e) => return exit_from_api_err("release download", &e),
            };
            let (owner, repo) = match repo_slug(args) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("arks hub release download: {e}");
                    return ExitCode::from(2);
                }
            };
            let pos = positional_num(args);
            let asset_id = flag_value(args, &["--asset-id", "-a"])
                .or_else(|| pos.as_deref())
                .unwrap_or("1");
            let out = flag_value(args, &["--output", "-o"]).unwrap_or("asset.bin");
            let path = format!("/repos/{owner}/{repo}/releases/assets/{asset_id}");
            match http_exchange_bytes(
                &base,
                "GET",
                &path,
                Some(&tok),
                None,
                "application/octet-stream",
                "application/octet-stream",
            ) {
                Ok((code, body)) if code < 400 => {
                    if let Err(e) = fs::write(out, &body) {
                        eprintln!("arks hub release download: write {out}: {e}");
                        return ExitCode::from(1);
                    }
                    println!("wrote={out} bytes={}", body.len());
                    ExitCode::SUCCESS
                }
                Ok((code, body)) => {
                    let text = String::from_utf8_lossy(&body);
                    hub_http_print(code, &text)
                }
                Err(e) => {
                    eprintln!("arks hub release download: {e}");
                    ExitCode::from(1)
                }
            }
        }
        "delete-asset" => {
            let (base, tok) = match require_local_api() {
                Ok(v) => v,
                Err(e) => return exit_from_api_err("release delete-asset", &e),
            };
            let (owner, repo) = match repo_slug(args) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("arks hub release delete-asset: {e}");
                    return ExitCode::from(2);
                }
            };
            let pos = positional_num(args);
            let asset_id = flag_value(args, &["--asset-id", "-a"])
                .or_else(|| pos.as_deref())
                .unwrap_or("1");
            let path = format!("/repos/{owner}/{repo}/releases/assets/{asset_id}");
            match http_exchange(&base, "DELETE", &path, Some(&tok), None) {
                Ok((code, body)) => hub_http_print(code, &body),
                Err(e) => {
                    eprintln!("arks hub release delete-asset: {e}");
                    ExitCode::from(1)
                }
            }
        }
        other => {
            eprintln!(
                "arks hub release {other}: Partial stub (not Claimed); refuse exit 2 — \
                 [[sot:root/arks-in-scope-leftovers-wave]]"
            );
            ExitCode::from(2)
        }
    }
}

fn cmd_run(args: &[String]) -> ExitCode {
    let sub = args.first().map(String::as_str).unwrap_or("");
    match sub {
        "" | "-h" | "--help" => {
            eprintln!(
                "arks hub run — list|create|view|watch|cancel|rerun|download|delete \
                 (local-forge Actions metadata; not CI compute)"
            );
            ExitCode::SUCCESS
        }
        "list" => {
            let (base, tok) = match require_local_api() {
                Ok(v) => v,
                Err(e) => return exit_from_api_err("run list", &e),
            };
            let (owner, repo) = match repo_slug(args) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("arks hub run list: {e}");
                    return ExitCode::from(2);
                }
            };
            let path = format!("/repos/{owner}/{repo}/actions/runs");
            match http_exchange(&base, "GET", &path, Some(&tok), None) {
                Ok((code, body)) => hub_http_print(code, &body),
                Err(e) => {
                    eprintln!("arks hub run list: {e}");
                    ExitCode::from(1)
                }
            }
        }
        "create" => {
            let (base, tok) = match require_local_api() {
                Ok(v) => v,
                Err(e) => return exit_from_api_err("run create", &e),
            };
            let (owner, repo) = match repo_slug(args) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("arks hub run create: {e}");
                    return ExitCode::from(2);
                }
            };
            let name = flag_value(args, &["--name", "-n"]).unwrap_or("manual");
            let branch = flag_value(args, &["--branch", "-b"]).unwrap_or("main");
            let payload = format!(
                r#"{{"name":{},"head_branch":{}}}"#,
                serde_json::to_string(name).unwrap(),
                serde_json::to_string(branch).unwrap()
            );
            let path = format!("/repos/{owner}/{repo}/actions/runs");
            match http_exchange(&base, "POST", &path, Some(&tok), Some(&payload)) {
                Ok((code, body)) => hub_http_print(code, &body),
                Err(e) => {
                    eprintln!("arks hub run create: {e}");
                    ExitCode::from(1)
                }
            }
        }
        "view" => {
            let (base, tok) = match require_local_api() {
                Ok(v) => v,
                Err(e) => return exit_from_api_err("run view", &e),
            };
            let (owner, repo) = match repo_slug(args) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("arks hub run view: {e}");
                    return ExitCode::from(2);
                }
            };
            let id = positional_num(args).unwrap_or_else(|| "1".into());
            let path = format!("/repos/{owner}/{repo}/actions/runs/{id}");
            match http_exchange(&base, "GET", &path, Some(&tok), None) {
                Ok((code, body)) => hub_http_print(code, &body),
                Err(e) => {
                    eprintln!("arks hub run view: {e}");
                    ExitCode::from(1)
                }
            }
        }
        "cancel" => {
            let (base, tok) = match require_local_api() {
                Ok(v) => v,
                Err(e) => return exit_from_api_err("run cancel", &e),
            };
            let (owner, repo) = match repo_slug(args) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("arks hub run cancel: {e}");
                    return ExitCode::from(2);
                }
            };
            let id = positional_num(args).unwrap_or_else(|| "1".into());
            let path = format!("/repos/{owner}/{repo}/actions/runs/{id}/cancel");
            match http_exchange(&base, "POST", &path, Some(&tok), None) {
                Ok((code, body)) => hub_http_print(code, &body),
                Err(e) => {
                    eprintln!("arks hub run cancel: {e}");
                    ExitCode::from(1)
                }
            }
        }
        "rerun" => {
            let (base, tok) = match require_local_api() {
                Ok(v) => v,
                Err(e) => return exit_from_api_err("run rerun", &e),
            };
            let (owner, repo) = match repo_slug(args) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("arks hub run rerun: {e}");
                    return ExitCode::from(2);
                }
            };
            let id = positional_num(args).unwrap_or_else(|| "1".into());
            let path = format!("/repos/{owner}/{repo}/actions/runs/{id}/rerun");
            match http_exchange(&base, "POST", &path, Some(&tok), None) {
                Ok((code, body)) => hub_http_print(code, &body),
                Err(e) => {
                    eprintln!("arks hub run rerun: {e}");
                    ExitCode::from(1)
                }
            }
        }
        "delete" => {
            let (base, tok) = match require_local_api() {
                Ok(v) => v,
                Err(e) => return exit_from_api_err("run delete", &e),
            };
            let (owner, repo) = match repo_slug(args) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("arks hub run delete: {e}");
                    return ExitCode::from(2);
                }
            };
            let id = positional_num(args).unwrap_or_else(|| "1".into());
            let path = format!("/repos/{owner}/{repo}/actions/runs/{id}");
            match http_exchange(&base, "DELETE", &path, Some(&tok), None) {
                Ok((code, body)) => hub_http_print(code, &body),
                Err(e) => {
                    eprintln!("arks hub run delete: {e}");
                    ExitCode::from(1)
                }
            }
        }
        "watch" => {
            let (base, tok) = match require_local_api() {
                Ok(v) => v,
                Err(e) => return exit_from_api_err("run watch", &e),
            };
            let (owner, repo) = match repo_slug(args) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("arks hub run watch: {e}");
                    return ExitCode::from(2);
                }
            };
            let id = positional_num(args).unwrap_or_else(|| "1".into());
            let path = format!("/repos/{owner}/{repo}/actions/runs/{id}");
            for _ in 0..40 {
                match http_exchange(&base, "GET", &path, Some(&tok), None) {
                    Ok((code, body)) if code < 400 => {
                        let v: Value = serde_json::from_str(&body).unwrap_or(json!({}));
                        let status = v
                            .get("status")
                            .and_then(|x| x.as_str())
                            .unwrap_or("unknown");
                        let conclusion = v
                            .get("conclusion")
                            .and_then(|x| x.as_str())
                            .unwrap_or("");
                        println!("status={status} conclusion={conclusion}");
                        if status == "completed" || status == "failed" {
                            return ExitCode::SUCCESS;
                        }
                    }
                    Ok((code, body)) => return hub_http_print(code, &body),
                    Err(e) => {
                        eprintln!("arks hub run watch: {e}");
                        return ExitCode::from(1);
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            eprintln!("arks hub run watch: timeout waiting for terminal status");
            ExitCode::from(1)
        }
        "download" => {
            let (base, tok) = match require_local_api() {
                Ok(v) => v,
                Err(e) => return exit_from_api_err("run download", &e),
            };
            let (owner, repo) = match repo_slug(args) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("arks hub run download: {e}");
                    return ExitCode::from(2);
                }
            };
            let id = positional_num(args).unwrap_or_else(|| "1".into());
            let out = flag_value(args, &["--output", "-o"]).unwrap_or("run-artifact.bin");
            let list_path = format!("/repos/{owner}/{repo}/actions/runs/{id}/artifacts");
            let art_id = match http_exchange(&base, "GET", &list_path, Some(&tok), None) {
                Ok((c, b)) if c < 400 => serde_json::from_str::<Value>(&b)
                    .ok()
                    .and_then(|v| {
                        v.get("artifacts")
                            .and_then(|a| a.as_array())
                            .and_then(|arr| arr.first())
                            .and_then(|x| x.get("id"))
                            .and_then(|x| x.as_u64())
                    })
                    .unwrap_or(0),
                Ok((c, b)) => return hub_http_print(c, &b),
                Err(e) => {
                    eprintln!("arks hub run download: {e}");
                    return ExitCode::from(1);
                }
            };
            if art_id == 0 {
                eprintln!("arks hub run download: no local-forge artifact for run {id}");
                return ExitCode::from(1);
            }
            let path = format!("/repos/{owner}/{repo}/actions/artifacts/{art_id}/zip");
            match http_exchange_bytes(
                &base,
                "GET",
                &path,
                Some(&tok),
                None,
                "application/octet-stream",
                "application/zip",
            ) {
                Ok((code, body)) if code < 400 => {
                    if let Err(e) = fs::write(out, &body) {
                        eprintln!("arks hub run download: write {out}: {e}");
                        return ExitCode::from(1);
                    }
                    println!("wrote={out} bytes={}", body.len());
                    ExitCode::SUCCESS
                }
                Ok((code, body)) => {
                    let text = String::from_utf8_lossy(&body);
                    hub_http_print(code, &text)
                }
                Err(e) => {
                    eprintln!("arks hub run download: {e}");
                    ExitCode::from(1)
                }
            }
        }
        other => {
            eprintln!(
                "arks hub run {other}: Partial stub — [[sot:root/arks-in-scope-leftovers-wave]]"
            );
            ExitCode::from(2)
        }
    }
}

fn cmd_workflow(args: &[String]) -> ExitCode {
    let sub = args.first().map(String::as_str).unwrap_or("list");
    match sub {
        "-h" | "--help" => {
            eprintln!(
                "arks hub workflow — list|view|enable|disable|run \
                 (local-forge metadata; not CI compute)"
            );
            ExitCode::SUCCESS
        }
        "" | "list" => {
            let (base, tok) = match require_local_api() {
                Ok(v) => v,
                Err(e) => return exit_from_api_err("workflow list", &e),
            };
            let (owner, repo) = match repo_slug(args) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("arks hub workflow list: {e}");
                    return ExitCode::from(2);
                }
            };
            let path = format!("/repos/{owner}/{repo}/actions/workflows");
            match http_exchange(&base, "GET", &path, Some(&tok), None) {
                Ok((code, body)) => hub_http_print(code, &body),
                Err(e) => {
                    eprintln!("arks hub workflow list: {e}");
                    ExitCode::from(1)
                }
            }
        }
        "view" => {
            let (base, tok) = match require_local_api() {
                Ok(v) => v,
                Err(e) => return exit_from_api_err("workflow view", &e),
            };
            let (owner, repo) = match repo_slug(args) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("arks hub workflow view: {e}");
                    return ExitCode::from(2);
                }
            };
            let key = args
                .iter()
                .skip(1)
                .find(|a| !a.starts_with('-') && !a.contains('/'))
                .map(String::as_str)
                .unwrap_or("1");
            let path = format!("/repos/{owner}/{repo}/actions/workflows/{key}");
            match http_exchange(&base, "GET", &path, Some(&tok), None) {
                Ok((code, body)) => hub_http_print(code, &body),
                Err(e) => {
                    eprintln!("arks hub workflow view: {e}");
                    ExitCode::from(1)
                }
            }
        }
        "enable" | "disable" => {
            let (base, tok) = match require_local_api() {
                Ok(v) => v,
                Err(e) => return exit_from_api_err(&format!("workflow {sub}"), &e),
            };
            let (owner, repo) = match repo_slug(args) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("arks hub workflow {sub}: {e}");
                    return ExitCode::from(2);
                }
            };
            let key = args
                .iter()
                .skip(1)
                .find(|a| !a.starts_with('-') && !a.contains('/'))
                .map(String::as_str)
                .unwrap_or("1");
            let path = format!("/repos/{owner}/{repo}/actions/workflows/{key}/{sub}");
            match http_exchange(&base, "PUT", &path, Some(&tok), None) {
                Ok((code, body)) => hub_http_print(code, &body),
                Err(e) => {
                    eprintln!("arks hub workflow {sub}: {e}");
                    ExitCode::from(1)
                }
            }
        }
        "run" => {
            let (base, tok) = match require_local_api() {
                Ok(v) => v,
                Err(e) => return exit_from_api_err("workflow run", &e),
            };
            let (owner, repo) = match repo_slug(args) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("arks hub workflow run: {e}");
                    return ExitCode::from(2);
                }
            };
            let key = args
                .iter()
                .skip(1)
                .find(|a| !a.starts_with('-') && !a.contains('/'))
                .map(String::as_str)
                .unwrap_or("1");
            let rref = flag_value(args, &["--ref", "-r"]).unwrap_or("main");
            let payload = format!(r#"{{"ref":{}}}"#, serde_json::to_string(rref).unwrap());
            let path = format!("/repos/{owner}/{repo}/actions/workflows/{key}/dispatches");
            match http_exchange(&base, "POST", &path, Some(&tok), Some(&payload)) {
                Ok((code, body)) => {
                    if code == 204 || code < 400 {
                        println!("{{\"ok\":true,\"ref\":{}}}", serde_json::to_string(rref).unwrap());
                        ExitCode::SUCCESS
                    } else {
                        hub_http_print(code, &body)
                    }
                }
                Err(e) => {
                    eprintln!("arks hub workflow run: {e}");
                    ExitCode::from(1)
                }
            }
        }
        other => {
            eprintln!(
                "arks hub workflow {other}: Partial stub — [[sot:root/arks-in-scope-leftovers-wave]]"
            );
            ExitCode::from(2)
        }
    }
}

fn cmd_status(_args: &[String]) -> ExitCode {
    let base = api_base();
    println!("api={base}");
    println!(
        "auth={}",
        if read_token().is_some() {
            "token"
        } else {
            "none"
        }
    );
    if let (Ok((base, tok)), Ok((owner, repo))) = (require_local_api(), repo_slug(&[])) {
        let issues = http_exchange(
            &base,
            "GET",
            &format!("/repos/{owner}/{repo}/issues"),
            Some(&tok),
            None,
        );
        let prs = http_exchange(
            &base,
            "GET",
            &format!("/repos/{owner}/{repo}/pulls"),
            Some(&tok),
            None,
        );
        match (issues, prs) {
            (Ok((ci, bi)), Ok((cp, bp))) => {
                println!("issues_http={ci}");
                println!("pulls_http={cp}");
                let _ = writeln!(io::stderr(), "issues={bi}");
                let _ = writeln!(io::stderr(), "pulls={bp}");
            }
            _ => println!("note: status aggregation Partial — forge serve may be down"),
        }
    } else {
        println!("note: set ARKS_HUB_REPO and start forge serve for aggregation");
    }
    ExitCode::SUCCESS
}

fn local_list_create(
    noun: &str,
    args: &[String],
    list_path: &str,
    create_path: &str,
    create_payload: Option<String>,
) -> ExitCode {
    let sub = args.first().map(String::as_str).unwrap_or("");
    match sub {
        "" | "-h" | "--help" => {
            eprintln!("arks hub {noun} — list|create (local hub REST Partial)");
            ExitCode::SUCCESS
        }
        "list" => {
            let (base, tok) = match require_local_api() {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("arks hub {noun} list: {e}");
                    return ExitCode::from(2);
                }
            };
            match http_exchange(&base, "GET", list_path, Some(&tok), None) {
                Ok((code, body)) => {
                    println!("{body}");
                    if code >= 400 {
                        ExitCode::from(1)
                    } else {
                        ExitCode::SUCCESS
                    }
                }
                Err(e) => {
                    eprintln!("arks hub {noun} list: {e}");
                    ExitCode::from(1)
                }
            }
        }
        "create" => {
            let Some(payload) = create_payload else {
                eprintln!("arks hub {noun} create: missing payload builder");
                return ExitCode::from(2);
            };
            let (base, tok) = match require_local_api() {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("arks hub {noun} create: {e}");
                    return ExitCode::from(2);
                }
            };
            match http_exchange(&base, "POST", create_path, Some(&tok), Some(&payload)) {
                Ok((code, body)) => {
                    println!("{body}");
                    if code >= 400 {
                        ExitCode::from(1)
                    } else {
                        ExitCode::SUCCESS
                    }
                }
                Err(e) => {
                    eprintln!("arks hub {noun} create: {e}");
                    ExitCode::from(1)
                }
            }
        }
        other => {
            eprintln!("arks hub {noun} {other}: not claimed beyond list/create Partial");
            ExitCode::from(2)
        }
    }
}

fn cmd_milestone(args: &[String]) -> ExitCode {
    let (owner, repo) = match repo_slug(args) {
        Ok(v) => v,
        Err(e) => {
            if matches!(
                args.first().map(String::as_str),
                Some("" | "-h" | "--help") | None
            ) {
                eprintln!("arks hub milestone — list|create (local Partial); needs --repo");
                return ExitCode::SUCCESS;
            }
            eprintln!("arks hub milestone: {e}");
            return ExitCode::from(2);
        }
    };
    let path = format!("/repos/{owner}/{repo}/milestones");
    let payload = if args.first().map(String::as_str) == Some("create") {
        let title = flag_value(args, &["--title", "-t"]).unwrap_or("milestone");
        let desc = flag_value(args, &["--description", "-d"]).unwrap_or("");
        Some(format!(
            r#"{{"title":{},"description":{}}}"#,
            serde_json::to_string(title).unwrap(),
            serde_json::to_string(desc).unwrap()
        ))
    } else {
        None
    };
    local_list_create("milestone", args, &path, &path, payload)
}

fn cmd_gist(args: &[String]) -> ExitCode {
    let payload = if args.first().map(String::as_str) == Some("create") {
        let desc = flag_value(args, &["--desc", "--description"]).unwrap_or("");
        let file = flag_value(args, &["--filename", "-f"]).unwrap_or("gist.txt");
        Some(format!(
            r#"{{"description":{},"public":true,"files":[{}]}}"#,
            serde_json::to_string(desc).unwrap(),
            serde_json::to_string(file).unwrap()
        ))
    } else {
        None
    };
    local_list_create("gist", args, "/gists", "/gists", payload)
}

fn cmd_project(args: &[String]) -> ExitCode {
    let (owner, repo) = match repo_slug(args) {
        Ok(v) => v,
        Err(e) => {
            if matches!(
                args.first().map(String::as_str),
                Some("" | "-h" | "--help") | None
            ) {
                eprintln!("arks hub project — list|create (local Partial); needs --repo");
                return ExitCode::SUCCESS;
            }
            eprintln!("arks hub project: {e}");
            return ExitCode::from(2);
        }
    };
    let path = format!("/repos/{owner}/{repo}/projects");
    let payload = if args.first().map(String::as_str) == Some("create") {
        let title = flag_value(args, &["--title", "-t"]).unwrap_or("project");
        let body = flag_value(args, &["--body", "-b"]).unwrap_or("");
        Some(format!(
            r#"{{"title":{},"body":{}}}"#,
            serde_json::to_string(title).unwrap(),
            serde_json::to_string(body).unwrap()
        ))
    } else {
        None
    };
    local_list_create("project", args, &path, &path, payload)
}

fn cmd_discussion(args: &[String]) -> ExitCode {
    let (owner, repo) = match repo_slug(args) {
        Ok(v) => v,
        Err(e) => {
            if matches!(
                args.first().map(String::as_str),
                Some("" | "-h" | "--help") | None
            ) {
                eprintln!("arks hub discussion — list|create (local Partial); needs --repo");
                return ExitCode::SUCCESS;
            }
            eprintln!("arks hub discussion: {e}");
            return ExitCode::from(2);
        }
    };
    let path = format!("/repos/{owner}/{repo}/discussions");
    let payload = if args.first().map(String::as_str) == Some("create") {
        let title = flag_value(args, &["--title", "-t"]).unwrap_or("discussion");
        let body = flag_value(args, &["--body", "-b"]).unwrap_or("");
        let cat = flag_value(args, &["--category", "-c"]).unwrap_or("General");
        Some(format!(
            r#"{{"title":{},"body":{},"category":{}}}"#,
            serde_json::to_string(title).unwrap(),
            serde_json::to_string(body).unwrap(),
            serde_json::to_string(cat).unwrap()
        ))
    } else {
        None
    };
    local_list_create("discussion", args, &path, &path, payload)
}

fn cmd_secret(args: &[String]) -> ExitCode {
    let (owner, repo) = match repo_slug(args) {
        Ok(v) => v,
        Err(e) => {
            if matches!(
                args.first().map(String::as_str),
                Some("" | "-h" | "--help") | None
            ) {
                eprintln!(
                    "arks hub secret — list|create (local meta Partial; values never stored)"
                );
                return ExitCode::SUCCESS;
            }
            eprintln!("arks hub secret: {e}");
            return ExitCode::from(2);
        }
    };
    let path = format!("/repos/{owner}/{repo}/actions/secrets");
    let payload = if args.first().map(String::as_str) == Some("create") {
        let name = flag_value(args, &["--name", "-n"]).unwrap_or("SECRET");
        Some(format!(
            r#"{{"name":{}}}"#,
            serde_json::to_string(name).unwrap()
        ))
    } else {
        None
    };
    local_list_create("secret", args, &path, &path, payload)
}

fn cmd_variable(args: &[String]) -> ExitCode {
    let (owner, repo) = match repo_slug(args) {
        Ok(v) => v,
        Err(e) => {
            if matches!(
                args.first().map(String::as_str),
                Some("" | "-h" | "--help") | None
            ) {
                eprintln!("arks hub variable — list|create (local Partial); needs --repo");
                return ExitCode::SUCCESS;
            }
            eprintln!("arks hub variable: {e}");
            return ExitCode::from(2);
        }
    };
    let path = format!("/repos/{owner}/{repo}/actions/variables");
    let payload = if args.first().map(String::as_str) == Some("create") {
        let name = flag_value(args, &["--name", "-n"]).unwrap_or("VAR");
        let value = flag_value(args, &["--value", "-v"]).unwrap_or("");
        Some(format!(
            r#"{{"name":{},"value":{}}}"#,
            serde_json::to_string(name).unwrap(),
            serde_json::to_string(value).unwrap()
        ))
    } else {
        None
    };
    local_list_create("variable", args, &path, &path, payload)
}

fn cmd_ssh_key(args: &[String]) -> ExitCode {
    let payload = if args.first().map(String::as_str) == Some("create") {
        let title = flag_value(args, &["--title", "-t"]).unwrap_or("key");
        let key = flag_value(args, &["--key", "-k"]).unwrap_or("ssh-ed25519 AAAA Partial");
        Some(format!(
            r#"{{"title":{},"key":{}}}"#,
            serde_json::to_string(title).unwrap(),
            serde_json::to_string(key).unwrap()
        ))
    } else {
        None
    };
    local_list_create("ssh-key", args, "/user/keys", "/user/keys", payload)
}

fn cmd_gpg_key(args: &[String]) -> ExitCode {
    let payload = if args.first().map(String::as_str) == Some("create") {
        let name = flag_value(args, &["--name", "-n"]).unwrap_or("gpg");
        let key_id = flag_value(args, &["--key-id"]).unwrap_or("00000000");
        Some(format!(
            r#"{{"name":{},"key_id":{}}}"#,
            serde_json::to_string(name).unwrap(),
            serde_json::to_string(key_id).unwrap()
        ))
    } else {
        None
    };
    local_list_create("gpg-key", args, "/user/gpg_keys", "/user/gpg_keys", payload)
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

/// Residual OUT refuse smoke Partial: prove `extension`/`copilot` stay OUT exit 2.
/// Never spawns `gh`. Does **not** invent Claimed forge remote e2e.
fn cmd_hub_out_refuse_smoke() -> ExitCode {
    println!("hub_smoke=partial");
    println!("out_refuse_smoke=partial");
    println!("spawns_gh=never");
    println!("claimed_forge_remote_e2e=Missing");
    let mut failed = false;
    for name in HUB_OUT_GROUPS {
        // Invoke the same OUT refuse path operators hit (`arks hub extension|copilot`).
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
            "note: residual OUT refuse smoke Partial ok (extension/copilot stay OUT); \
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
             Marketplace/vendor depth is out of self-host forge scope \
             ([[sot:root/arks-hub-cli-surface]]). Design ≠ Claimed."
        );
        return ExitCode::SUCCESS;
    }
    let why = match name {
        "extension" => {
            "GitHub Marketplace / IDE extensions are OUT — no local forge inventory invent"
        }
        "copilot" => "GitHub Copilot / vendor assistant APIs are OUT — not a local forge leaf",
        _ => "OUT/later matrix group — refuse depth",
    };
    eprintln!(
        "arks hub: `{name}` OUT explicit refuse exit 2 — {why} \
         ([[sot:root/arks-hub-cli-surface]]); never spawns gh; design ≠ Claimed"
    );
    ExitCode::from(2)
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

fn cmd_browse(args: &[String]) -> ExitCode {
    if args.iter().any(|a| a == "-h" || a == "--help") {
        eprintln!("arks hub browse [PATH] — print forge URL (Partial; never spawns a browser)");
        return ExitCode::SUCCESS;
    }
    let base = api_base();
    let path = args
        .iter()
        .find(|a| !a.starts_with('-'))
        .cloned()
        .unwrap_or_else(|| {
            repo_slug(args)
                .map(|(o, n)| format!("/{o}/{n}"))
                .unwrap_or_else(|_| "/".into())
        });
    let path = if path.starts_with('/') {
        path
    } else {
        format!("/{path}")
    };
    println!("{base}{path}");
    eprintln!("note: browse Partial — URL printed only; no browser spawn Claimed");
    ExitCode::SUCCESS
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

fn cmd_config(args: &[String]) -> ExitCode {
    let sub = args.first().map(String::as_str).unwrap_or("list");
    match sub {
        "" | "list" | "-h" | "--help" => {
            if matches!(sub, "-h" | "--help") {
                eprintln!(
                    "arks hub config — get|set|list|clear-cache (client state file; ≠ git config)"
                );
                return ExitCode::SUCCESS;
            }
            println!("api={}", api_base());
            println!("token_file={}", token_path().display());
            println!("client_state={}", hub_client_state_path().display());
            let st = load_hub_client_state();
            for (k, v) in &st.config {
                println!("{k}={v}");
            }
            ExitCode::SUCCESS
        }
        "get" => {
            let key = args.get(1).map(String::as_str).unwrap_or("api");
            match key {
                "api" => println!("{}", api_base()),
                "token_file" => println!("{}", token_path().display()),
                "client_state" => println!("{}", hub_client_state_path().display()),
                other => {
                    let st = load_hub_client_state();
                    if let Some(v) = st.config.get(other) {
                        println!("{v}");
                    } else {
                        eprintln!("arks hub config get: unknown key `{other}`");
                        return ExitCode::from(2);
                    }
                }
            }
            ExitCode::SUCCESS
        }
        "set" => {
            let key = match args.get(1).map(String::as_str) {
                Some(k) if !k.starts_with('-') => k,
                _ => {
                    eprintln!("arks hub config set: KEY VALUE required");
                    return ExitCode::from(2);
                }
            };
            let value = args.get(2).map(String::as_str).unwrap_or("");
            if matches!(key, "api" | "token_file" | "client_state") {
                eprintln!(
                    "arks hub config set: `{key}` is env/path derived — set ARKS_HUB_API / \
                     ARKS_HUB_TOKEN_FILE / ARKS_HUB_CLIENT_STATE instead"
                );
                return ExitCode::from(2);
            }
            let mut st = load_hub_client_state();
            st.config.insert(key.into(), value.into());
            if let Err(e) = save_hub_client_state(&st) {
                eprintln!("arks hub config set: {e}");
                return ExitCode::from(1);
            }
            println!("{key}={value}");
            ExitCode::SUCCESS
        }
        "clear-cache" => {
            let mut st = load_hub_client_state();
            st.config.clear();
            if let Err(e) = save_hub_client_state(&st) {
                eprintln!("arks hub config clear-cache: {e}");
                return ExitCode::from(1);
            }
            println!("cleared=client_config");
            ExitCode::SUCCESS
        }
        other => {
            eprintln!(
                "arks hub config {other}: Partial stub — [[sot:root/arks-in-scope-leftovers-wave]]"
            );
            ExitCode::from(2)
        }
    }
}

fn cmd_alias(args: &[String]) -> ExitCode {
    let sub = args.first().map(String::as_str).unwrap_or("");
    if matches!(sub, "-h" | "--help") || args.iter().any(|a| a == "-h" || a == "--help") {
        eprintln!("arks hub alias — list|set|delete|import (client state; no shell profile mutation)");
        return ExitCode::SUCCESS;
    }
    match sub {
        "" | "list" => {
            let st = load_hub_client_state();
            if st.aliases.is_empty() {
                println!("# no aliases; defaults:");
                println!("gh=arks");
                println!("pr=arks pr");
                println!("issue=arks issue");
            } else {
                for (k, v) in &st.aliases {
                    println!("{k}={v}");
                }
            }
            ExitCode::SUCCESS
        }
        "set" => {
            let name = match args.get(1).map(String::as_str) {
                Some(n) if !n.starts_with('-') => n,
                _ => {
                    eprintln!("arks hub alias set: NAME EXPANSION required");
                    return ExitCode::from(2);
                }
            };
            let expansion = args.get(2).map(String::as_str).unwrap_or("");
            let mut st = load_hub_client_state();
            st.aliases.insert(name.into(), expansion.into());
            if let Err(e) = save_hub_client_state(&st) {
                eprintln!("arks hub alias set: {e}");
                return ExitCode::from(1);
            }
            println!("{name}={expansion}");
            ExitCode::SUCCESS
        }
        "delete" => {
            let name = match args.get(1).map(String::as_str) {
                Some(n) if !n.starts_with('-') => n,
                _ => {
                    eprintln!("arks hub alias delete: NAME required");
                    return ExitCode::from(2);
                }
            };
            let mut st = load_hub_client_state();
            if st.aliases.remove(name).is_none() {
                eprintln!("arks hub alias delete: `{name}` not found");
                return ExitCode::from(1);
            }
            if let Err(e) = save_hub_client_state(&st) {
                eprintln!("arks hub alias delete: {e}");
                return ExitCode::from(1);
            }
            println!("deleted={name}");
            ExitCode::SUCCESS
        }
        "import" => {
            let file = match args.get(1).map(String::as_str) {
                Some(f) if !f.starts_with('-') => f,
                _ => {
                    eprintln!("arks hub alias import: FILE required (lines NAME=EXPANSION)");
                    return ExitCode::from(2);
                }
            };
            let raw = match fs::read_to_string(file) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("arks hub alias import: {e}");
                    return ExitCode::from(1);
                }
            };
            let mut st = load_hub_client_state();
            let mut n = 0usize;
            for line in raw.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if let Some((k, v)) = line.split_once('=') {
                    st.aliases.insert(k.trim().into(), v.trim().into());
                    n += 1;
                }
            }
            if let Err(e) = save_hub_client_state(&st) {
                eprintln!("arks hub alias import: {e}");
                return ExitCode::from(1);
            }
            println!("imported={n}");
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!(
                "arks hub alias: unknown leaf — [[sot:root/arks-in-scope-leftovers-wave]]"
            );
            ExitCode::from(2)
        }
    }
}

fn cmd_completion(args: &[String]) -> ExitCode {
    if args.iter().any(|a| a == "-h" || a == "--help") {
        eprintln!("arks hub completion [bash|zsh|fish|powershell] — emit script to stdout");
        return ExitCode::SUCCESS;
    }
    let shell = args
        .iter()
        .find(|a| !a.starts_with('-'))
        .map(String::as_str)
        .unwrap_or("bash");
    match shell {
        "bash" => {
            println!("# arks hub completion (IW-HUB-UX); emit only — no installer mutation");
            println!("complete -W \"{}\" arks", GROUPS.join(" "));
        }
        "zsh" => {
            println!("#compdef arks");
            println!("_arks() {{ _arguments '1: :({})' }}", GROUPS.join(" "));
        }
        "fish" => {
            println!("complete -c arks -a \"{}\"", GROUPS.join(" "));
        }
        "powershell" | "pwsh" => {
            println!("# PowerShell Register-ArgumentCompleter for arks hub leaves");
            println!("# leaves: {}", GROUPS.join(" "));
        }
        other => {
            eprintln!(
                "arks hub completion: shell `{other}` not claimed (bash|zsh|fish|powershell)"
            );
            return ExitCode::from(2);
        }
    }
    ExitCode::SUCCESS
}
