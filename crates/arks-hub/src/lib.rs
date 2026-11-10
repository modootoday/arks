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
pub mod serve;
mod store;

use http_min::http_exchange;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
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
    auth          Authenticate (token file + optional /user probe)
    pr            Pull requests (local hub REST Partial)
    issue         Issues (local hub REST Partial)
    repo          Repositories (local hub REST Partial)
    label         Labels (local hub REST Partial)
    release       Releases list|create (local Partial)
    run           Actions runs list|create (local Partial; not CI Claimed)
    workflow      Actions workflows list (local Partial)
    org           Organizations list (local Partial)
    search        Search repos|issues|prs (local Partial)
    api           Raw HTTP against ARKS_HUB_API
    status        Aggregated status (Partial)
    co            Alias for pr checkout
    …             Other matrix groups: discoverability refuse until evidenced

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
        "org" => cmd_org(&args[1..]),
        "search" => cmd_search(&args[1..]),
        "api" => cmd_api(&args[1..]),
        "status" => cmd_status(&args[1..]),
        "co" => {
            eprintln!("arks hub co → pr checkout (alias; checkout worktree not claimed)");
            cmd_pr(&["checkout".into()])
        }
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

fn require_local_api() -> Result<(String, String), String> {
    let base = api_base();
    if !base.starts_with("http://") {
        return Err(format!(
            "HTTPS remote execute not Claimed yet; use local forge API (got {base})"
        ));
    }
    let tok = read_token().ok_or_else(|| "not logged in — arks hub auth login".to_string())?;
    Ok((base, tok))
}

fn cmd_pr(args: &[String]) -> ExitCode {
    let sub = args.first().map(String::as_str).unwrap_or("");
    match sub {
        "" | "-h" | "--help" => {
            eprintln!("arks hub pr — list|view|create (local hub REST Partial)");
            ExitCode::SUCCESS
        }
        "list" => {
            let (base, tok) = match require_local_api() {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("arks hub pr list: {e}");
                    return ExitCode::from(2);
                }
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
                Ok((code, body)) => {
                    println!("{body}");
                    if code >= 400 {
                        ExitCode::from(1)
                    } else {
                        ExitCode::SUCCESS
                    }
                }
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
                Ok((code, body)) => {
                    println!("{body}");
                    if code >= 400 {
                        ExitCode::from(1)
                    } else {
                        ExitCode::SUCCESS
                    }
                }
                Err(e) => {
                    eprintln!("arks hub pr create: {e}");
                    ExitCode::from(1)
                }
            }
        }
        other => {
            eprintln!("arks hub pr {other}: not claimed beyond list/view/create Partial");
            ExitCode::from(2)
        }
    }
}

fn cmd_issue(args: &[String]) -> ExitCode {
    let sub = args.first().map(String::as_str).unwrap_or("");
    match sub {
        "" | "-h" | "--help" => {
            eprintln!("arks hub issue — list|view|create (local hub REST Partial)");
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
                Ok((code, body)) => {
                    println!("{body}");
                    if code >= 400 {
                        ExitCode::from(1)
                    } else {
                        ExitCode::SUCCESS
                    }
                }
                Err(e) => {
                    eprintln!("arks hub issue create: {e}");
                    ExitCode::from(1)
                }
            }
        }
        other => {
            eprintln!("arks hub issue {other}: not claimed beyond list/view/create Partial");
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
            eprintln!("arks hub label — list|create (local hub REST Partial)");
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
                Ok((code, body)) => {
                    println!("{body}");
                    if code >= 400 {
                        ExitCode::from(1)
                    } else {
                        ExitCode::SUCCESS
                    }
                }
                Err(e) => {
                    eprintln!("arks hub label create: {e}");
                    ExitCode::from(1)
                }
            }
        }
        other => {
            eprintln!("arks hub label {other}: not claimed beyond list/create Partial");
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
            eprintln!("arks hub release — list|create (local hub REST Partial)");
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
                Ok((code, body)) => {
                    println!("{body}");
                    if code >= 400 {
                        ExitCode::from(1)
                    } else {
                        ExitCode::SUCCESS
                    }
                }
                Err(e) => {
                    eprintln!("arks hub release create: {e}");
                    ExitCode::from(1)
                }
            }
        }
        other => {
            eprintln!("arks hub release {other}: not claimed beyond list/create Partial");
            ExitCode::from(2)
        }
    }
}

fn cmd_run(args: &[String]) -> ExitCode {
    let sub = args.first().map(String::as_str).unwrap_or("");
    match sub {
        "" | "-h" | "--help" => {
            eprintln!(
                "arks hub run — list|create (local actions runs Partial; not CI Claimed)"
            );
            ExitCode::SUCCESS
        }
        "list" => {
            let (base, tok) = match require_local_api() {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("arks hub run list: {e}");
                    return ExitCode::from(2);
                }
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
                Ok((code, body)) => {
                    println!("{body}");
                    if code >= 400 {
                        ExitCode::from(1)
                    } else {
                        ExitCode::SUCCESS
                    }
                }
                Err(e) => {
                    eprintln!("arks hub run list: {e}");
                    ExitCode::from(1)
                }
            }
        }
        "create" => {
            let (base, tok) = match require_local_api() {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("arks hub run create: {e}");
                    return ExitCode::from(2);
                }
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
                Ok((code, body)) => {
                    println!("{body}");
                    if code >= 400 {
                        ExitCode::from(1)
                    } else {
                        ExitCode::SUCCESS
                    }
                }
                Err(e) => {
                    eprintln!("arks hub run create: {e}");
                    ExitCode::from(1)
                }
            }
        }
        other => {
            eprintln!("arks hub run {other}: not claimed beyond list/create Partial");
            ExitCode::from(2)
        }
    }
}

fn cmd_workflow(args: &[String]) -> ExitCode {
    let sub = args.first().map(String::as_str).unwrap_or("list");
    match sub {
        "" | "list" | "-h" | "--help" => {
            if matches!(sub, "-h" | "--help") {
                eprintln!("arks hub workflow — list (local Partial; not CI Claimed)");
                return ExitCode::SUCCESS;
            }
            let (base, tok) = match require_local_api() {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("arks hub workflow list: {e}");
                    return ExitCode::from(2);
                }
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
                Ok((code, body)) => {
                    println!("{body}");
                    if code >= 400 {
                        ExitCode::from(1)
                    } else {
                        ExitCode::SUCCESS
                    }
                }
                Err(e) => {
                    eprintln!("arks hub workflow list: {e}");
                    ExitCode::from(1)
                }
            }
        }
        other => {
            eprintln!("arks hub workflow {other}: not claimed beyond list Partial");
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
