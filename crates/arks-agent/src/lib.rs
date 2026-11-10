//! `arks-agent` — LLM code review / CEV inspection bots (Partial; not Claimed).
//!
//! Wired into the single release binary as **`arks agent …`**. This crate is
//! library-only — never a separately shipped product binary.
//!
//! Native HTTP to LLM providers + hub APIs only — MUST NOT spawn `git`, `gh`,
//! or vendor CLIs (`ollama`, `openai`, …).
//!
//! Cite: [[sot:root/arks-agent-cli-surface]], [[sot:root/arks-agent-llm-providers]]

#![forbid(unsafe_code)]

use std::env;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

/// Top-level groups shown in help.
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

#[derive(Clone, Copy)]
struct ProviderPreset {
    id: &'static str,
    base_url: &'static str,
    notes: &'static str,
}

const PRESETS: &[ProviderPreset] = &[
    ProviderPreset {
        id: "openai-compat",
        base_url: "http://127.0.0.1:11434/v1",
        notes: "shared OpenAI-compat path (override --base-url)",
    },
    ProviderPreset {
        id: "ollama",
        base_url: "http://127.0.0.1:11434/v1",
        notes: "openai-compat preset; local Ollama default",
    },
    ProviderPreset {
        id: "openai",
        base_url: "https://api.openai.com/v1",
        notes: "openai-compat preset; needs ARKS_AGENT_API_KEY / OPENAI_API_KEY",
    },
    ProviderPreset {
        id: "claude",
        base_url: "https://api.anthropic.com",
        notes: "vendor path Partial — config only until wire Claimed",
    },
    ProviderPreset {
        id: "anthropic",
        base_url: "https://api.anthropic.com",
        notes: "alias of claude; Partial",
    },
    ProviderPreset {
        id: "grok",
        base_url: "https://api.x.ai/v1",
        notes: "vendor path Partial",
    },
    ProviderPreset {
        id: "xai",
        base_url: "https://api.x.ai/v1",
        notes: "alias of grok; Partial",
    },
    ProviderPreset {
        id: "gemini",
        base_url: "https://generativelanguage.googleapis.com",
        notes: "vendor path Partial",
    },
    ProviderPreset {
        id: "google",
        base_url: "https://generativelanguage.googleapis.com",
        notes: "alias of gemini; Partial",
    },
    ProviderPreset {
        id: "cloudflare",
        base_url: "https://api.cloudflare.com",
        notes: "Workers AI Partial — config only",
    },
];

/// Print `arks agent` help to stderr.
pub fn print_help() {
    eprintln!(
        "\
arks agent — LLM review / CEV bots (Partial; not Claimed)

USAGE:
    arks agent [OPTIONS] <COMMAND>

OPTIONS:
    -h, --help       Print help
    -V, --version    Print version

COMMANDS (Partial honesty):
    provider      list|show|test|set LLM endpoints (openai-compat first)
    config        show|path agent config (secrets redacted)
    doctor        Config + TCP reachability sanity check
    review        Review stdin/file via openai-compat chat (Partial; HTTP loopback)
    cev-check     Structural CEV path/config inspection (Partial; no LLM Claimed)
    run           One-shot job: review|cev-check|doctor (Partial)
    serve / bot   Loopback health listener (Partial; not a chatbot)

NOTES:
    Entry is `arks agent` (single arks release binary). Not a separate arks-agent product.
    Not required for git alias DoD. Not a general chatbot.
    Local Ollama default: http://127.0.0.1:11434/v1
    Native HTTP / TCP only — never spawns git / gh / vendor CLIs.
    Design ≠ Claimed ([[sot:root/arks-agent-cli-surface]]).
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
        "provider" => cmd_provider(&args[1..]),
        "config" => cmd_config(&args[1..]),
        "doctor" => cmd_doctor(&args[1..]),
        "review" => cmd_review(&args[1..]),
        "cev-check" => cmd_cev_check(&args[1..]),
        "run" => cmd_run(&args[1..]),
        "serve" | "bot" => cmd_serve_bot(first, &args[1..]),
        other if GROUPS.contains(&other) => {
            eprintln!(
                "arks agent: `{other}` recognized (Partial stub); refuse exit 2 — design ≠ Claimed"
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

fn config_path() -> PathBuf {
    if let Ok(p) = env::var("ARKS_AGENT_CONFIG") {
        return PathBuf::from(p);
    }
    let home = env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    home.join(".config").join("arks").join("agent.conf")
}

fn load_config_map() -> Result<std::collections::BTreeMap<String, String>, String> {
    let path = config_path();
    let mut map = std::collections::BTreeMap::new();
    if !path.is_file() {
        return Ok(map);
    }
    let raw = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        map.insert(k.trim().to_string(), v.trim().to_string());
    }
    Ok(map)
}

fn save_config_map(map: &std::collections::BTreeMap<String, String>) -> Result<(), String> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut out = String::from("# arks agent config (Partial; never commit secrets)\n");
    for (k, v) in map {
        out.push_str(k);
        out.push('=');
        out.push_str(v);
        out.push('\n');
    }
    fs::write(&path, out).map_err(|e| e.to_string())
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

fn resolve_provider(id: &str) -> Option<ProviderPreset> {
    PRESETS.iter().copied().find(|p| p.id == id)
}

fn effective_base_url(id: &str, override_url: Option<&str>) -> String {
    if let Some(u) = override_url {
        return u.to_string();
    }
    if let Ok(u) = env::var("ARKS_AGENT_BASE_URL") {
        if !u.is_empty() {
            return u;
        }
    }
    if let Ok(map) = load_config_map() {
        if let Some(u) = map.get("base_url") {
            if !u.is_empty() {
                return u.clone();
            }
        }
        if let Some(cfg_id) = map.get("provider") {
            if cfg_id == id {
                if let Some(u) = map.get("base_url") {
                    return u.clone();
                }
            }
        }
    }
    resolve_provider(id)
        .map(|p| p.base_url.to_string())
        .unwrap_or_else(|| "http://127.0.0.1:11434/v1".into())
}

fn parse_host_port(base_url: &str) -> Result<(String, u16), String> {
    let rest = base_url
        .strip_prefix("http://")
        .or_else(|| base_url.strip_prefix("https://"))
        .ok_or_else(|| format!("unsupported URL scheme (need http/https): {base_url}"))?;
    let hostport = rest.split('/').next().unwrap_or(rest);
    if let Some((h, p)) = hostport.rsplit_once(':') {
        let port: u16 = p
            .parse()
            .map_err(|_| format!("bad port in base URL: {base_url}"))?;
        Ok((h.to_string(), port))
    } else if base_url.starts_with("https://") {
        Ok((hostport.to_string(), 443))
    } else {
        Ok((hostport.to_string(), 80))
    }
}

fn tcp_probe(base_url: &str) -> Result<(), String> {
    let (host, port) = parse_host_port(base_url)?;
    let stream = TcpStream::connect((host.as_str(), port))
        .map_err(|e| format!("connect {host}:{port}: {e}"))?;
    let _ = stream.set_read_timeout(Some(Duration::from_secs(3)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(3)));
    Ok(())
}

/// Best-effort HTTP GET for openai-compat `/models` (Partial; no TLS Claimed beyond std).
fn http_get_models(base_url: &str) -> Result<(u16, String), String> {
    if base_url.starts_with("https://") {
        return Err(
            "HTTPS provider probe Partial — TCP-only today; use http:// loopback or ops TLS front"
                .into(),
        );
    }
    let (host, port) = parse_host_port(base_url)?;
    let root = base_url.trim_end_matches('/');
    let path = if root.ends_with("/v1") {
        "/v1/models"
    } else {
        "/models"
    };
    // Prefer path relative to host: use full path from URL.
    let url_path = {
        let rest = base_url
            .strip_prefix("http://")
            .unwrap_or(base_url)
            .split_once('/')
            .map(|(_, p)| format!("/{p}"))
            .unwrap_or_else(|| "/".into());
        let rest = rest.trim_end_matches('/');
        if rest.ends_with("/v1") || rest.is_empty() {
            format!("{}/models", if rest.is_empty() { "" } else { rest })
        } else {
            format!("{rest}/models")
        }
    };
    let _ = path;
    let mut stream = TcpStream::connect((host.as_str(), port))
        .map_err(|e| format!("connect {host}:{port}: {e}"))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .ok();
    stream
        .set_write_timeout(Some(Duration::from_secs(5)))
        .ok();
    let req = format!(
        "GET {url_path} HTTP/1.1\r\nHost: {host}:{port}\r\nAccept: application/json\r\nConnection: close\r\n\r\n"
    );
    stream
        .write_all(req.as_bytes())
        .map_err(|e| e.to_string())?;
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).map_err(|e| e.to_string())?;
    let text = String::from_utf8_lossy(&buf);
    let status = text
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let body = text
        .split("\r\n\r\n")
        .nth(1)
        .unwrap_or("")
        .chars()
        .take(512)
        .collect::<String>();
    Ok((status, body))
}

fn cmd_provider(args: &[String]) -> ExitCode {
    let sub = args.first().map(String::as_str).unwrap_or("");
    match sub {
        "" | "-h" | "--help" | "help" => {
            eprintln!(
                "arks agent provider — list|show <id>|test <id>|set <id> [--base-url …] [--api-key-env VAR]"
            );
            ExitCode::SUCCESS
        }
        "list" => {
            for p in PRESETS {
                println!("{}\t{}\t{}", p.id, p.base_url, p.notes);
            }
            eprintln!("note: Partial inventory — design ≠ Claimed");
            ExitCode::SUCCESS
        }
        "show" => {
            let id = args.get(1).map(String::as_str).unwrap_or("");
            if id.is_empty() {
                eprintln!("arks agent provider show: needs <id>");
                return ExitCode::from(2);
            }
            let Some(p) = resolve_provider(id) else {
                eprintln!("arks agent provider show: unknown id `{id}`");
                return ExitCode::from(2);
            };
            let base = effective_base_url(id, None);
            println!("id={}", p.id);
            println!("base_url={base}");
            println!("notes={}", p.notes);
            let key_env = env::var("ARKS_AGENT_API_KEY_ENV")
                .ok()
                .or_else(|| load_config_map().ok().and_then(|m| m.get("api_key_env").cloned()));
            if let Some(v) = key_env {
                println!("api_key_env={v}");
            }
            println!("api_key={}", redact_key_present());
            ExitCode::SUCCESS
        }
        "test" => {
            let id = args.get(1).map(String::as_str).unwrap_or("");
            if id.is_empty() {
                eprintln!("arks agent provider test: needs <id>");
                return ExitCode::from(2);
            }
            if resolve_provider(id).is_none() {
                eprintln!("arks agent provider test: unknown id `{id}`");
                return ExitCode::from(2);
            }
            let base = effective_base_url(id, flag_value(args, &["--base-url"]));
            println!("provider={id}");
            println!("base_url={base}");
            match tcp_probe(&base) {
                Ok(()) => println!("tcp=ok"),
                Err(e) => {
                    println!("tcp=fail");
                    eprintln!("{e}");
                    return ExitCode::from(1);
                }
            }
            match http_get_models(&base) {
                Ok((code, body)) => {
                    println!("models_http={code}");
                    if !body.is_empty() {
                        println!("{body}");
                    }
                    if code == 0 || code >= 400 {
                        eprintln!("note: Partial probe — non-success HTTP (design ≠ Claimed)");
                        ExitCode::from(1)
                    } else {
                        ExitCode::SUCCESS
                    }
                }
                Err(e) => {
                    println!("models_http=skipped");
                    eprintln!("{e}");
                    // TCP ok is enough for local Partial doctor/test honesty.
                    ExitCode::SUCCESS
                }
            }
        }
        "set" => {
            let id = args.get(1).map(String::as_str).unwrap_or("");
            if id.is_empty() || resolve_provider(id).is_none() {
                eprintln!("arks agent provider set: needs known <id>");
                return ExitCode::from(2);
            }
            let mut map = match load_config_map() {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("arks agent provider set: {e}");
                    return ExitCode::from(1);
                }
            };
            map.insert("provider".into(), id.into());
            if let Some(u) = flag_value(args, &["--base-url"]) {
                map.insert("base_url".into(), u.into());
            } else if let Some(p) = resolve_provider(id) {
                map.insert("base_url".into(), p.base_url.into());
            }
            if let Some(v) = flag_value(args, &["--api-key-env"]) {
                map.insert("api_key_env".into(), v.into());
            }
            if let Some(m) = flag_value(args, &["--model"]) {
                map.insert("model".into(), m.into());
            }
            if let Err(e) = save_config_map(&map) {
                eprintln!("arks agent provider set: {e}");
                return ExitCode::from(1);
            }
            eprintln!(
                "arks agent provider set: provider={id} → {} (Partial; not Claimed)",
                config_path().display()
            );
            ExitCode::SUCCESS
        }
        other => {
            eprintln!("arks agent provider: unknown `{other}` (list|show|test|set)");
            ExitCode::from(2)
        }
    }
}

fn redact_key_present() -> &'static str {
    if env::var_os("ARKS_AGENT_API_KEY").is_some() || env::var_os("OPENAI_API_KEY").is_some() {
        "***"
    } else if let Ok(map) = load_config_map() {
        if let Some(var) = map.get("api_key_env") {
            if env::var_os(var).is_some() {
                return "***";
            }
        }
        "none"
    } else {
        "none"
    }
}

fn cmd_config(args: &[String]) -> ExitCode {
    let sub = args.first().map(String::as_str).unwrap_or("show");
    match sub {
        "-h" | "--help" | "help" => {
            eprintln!("arks agent config — show|path");
            ExitCode::SUCCESS
        }
        "path" => {
            println!("{}", config_path().display());
            ExitCode::SUCCESS
        }
        "show" | "" => {
            let path = config_path();
            println!("path={}", path.display());
            match load_config_map() {
                Ok(map) if map.is_empty() => {
                    println!("# empty Partial config — defaults apply");
                    println!(
                        "provider={}",
                        env::var("ARKS_AGENT_PROVIDER").unwrap_or_else(|_| "ollama".into())
                    );
                    println!(
                        "base_url={}",
                        env::var("ARKS_AGENT_BASE_URL")
                            .unwrap_or_else(|_| "http://127.0.0.1:11434/v1".into())
                    );
                    println!("api_key={}", redact_key_present());
                }
                Ok(map) => {
                    for (k, v) in map {
                        if k.contains("key") && k != "api_key_env" {
                            println!("{k}=***");
                        } else {
                            println!("{k}={v}");
                        }
                    }
                    println!("api_key={}", redact_key_present());
                }
                Err(e) => {
                    eprintln!("arks agent config show: {e}");
                    return ExitCode::from(1);
                }
            }
            eprintln!("note: secrets redacted; Partial ≠ Claimed");
            ExitCode::SUCCESS
        }
        other => {
            eprintln!("arks agent config: unknown `{other}` (show|path)");
            ExitCode::from(2)
        }
    }
}

fn cmd_doctor(_args: &[String]) -> ExitCode {
    let map = load_config_map().unwrap_or_default();
    let id = env::var("ARKS_AGENT_PROVIDER")
        .ok()
        .or_else(|| map.get("provider").cloned())
        .unwrap_or_else(|| "ollama".into());
    let base = effective_base_url(&id, None);
    println!("config={}", config_path().display());
    println!("provider={id}");
    println!("base_url={base}");
    println!("api_key={}", redact_key_present());
    match tcp_probe(&base) {
        Ok(()) => {
            println!("tcp=ok");
            println!("doctor=ok (Partial reachability; LLM review not Claimed)");
            ExitCode::SUCCESS
        }
        Err(e) => {
            println!("tcp=fail");
            eprintln!("{e}");
            eprintln!("doctor=fail (Partial — start local provider or fix base_url)");
            ExitCode::from(1)
        }
    }
}

fn read_review_input(args: &[String]) -> Result<String, String> {
    if let Some(path) = flag_value(args, &["--file", "-f"]) {
        return fs::read_to_string(path).map_err(|e| e.to_string());
    }
    let mut buf = String::new();
    std::io::stdin()
        .read_to_string(&mut buf)
        .map_err(|e| e.to_string())?;
    if buf.trim().is_empty() {
        return Err(
            "review needs stdin text or --file PATH (Partial; never spawns git/gh)".into(),
        );
    }
    Ok(buf)
}

/// Best-effort openai-compat chat completions POST (HTTP loopback Partial).
fn http_post_chat(base_url: &str, model: &str, prompt: &str) -> Result<(u16, String), String> {
    if base_url.starts_with("https://") {
        return Err(
            "HTTPS chat Partial — TCP/HTTP loopback only today; use http:// provider or ops TLS front"
                .into(),
        );
    }
    let (host, port) = parse_host_port(base_url)?;
    let root = base_url.trim_end_matches('/');
    let path = if root.ends_with("/v1") {
        "/v1/chat/completions".to_string()
    } else if let Some(rest) = base_url
        .strip_prefix("http://")
        .and_then(|r| r.split_once('/').map(|(_, p)| p))
    {
        let rest = rest.trim_end_matches('/');
        if rest.is_empty() {
            "/v1/chat/completions".into()
        } else if rest.ends_with("/v1") {
            format!("/{rest}/chat/completions")
        } else {
            format!("/{rest}/chat/completions")
        }
    } else {
        "/v1/chat/completions".into()
    };
    let body = format!(
        r#"{{"model":{},"messages":[{{"role":"user","content":{}}}],"stream":false}}"#,
        json_escape(model),
        json_escape(&truncate_chars(prompt, 12_000))
    );
    let mut stream = TcpStream::connect((host.as_str(), port))
        .map_err(|e| format!("connect {host}:{port}: {e}"))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(60)))
        .ok();
    stream
        .set_write_timeout(Some(Duration::from_secs(30)))
        .ok();
    let mut headers = format!(
        "POST {path} HTTP/1.1\r\nHost: {host}:{port}\r\nContent-Type: application/json\r\nAccept: application/json\r\nContent-Length: {}\r\nConnection: close\r\n",
        body.len()
    );
    if let Some(key) = resolve_api_key() {
        headers.push_str(&format!("Authorization: Bearer {key}\r\n"));
    }
    headers.push_str("\r\n");
    stream
        .write_all(headers.as_bytes())
        .map_err(|e| e.to_string())?;
    stream.write_all(body.as_bytes()).map_err(|e| e.to_string())?;
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).map_err(|e| e.to_string())?;
    let text = String::from_utf8_lossy(&buf);
    let status = text
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let resp_body = text
        .split("\r\n\r\n")
        .nth(1)
        .unwrap_or("")
        .chars()
        .take(8_192)
        .collect::<String>();
    Ok((status, resp_body))
}

fn json_escape(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn truncate_chars(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        s.chars().take(max).collect::<String>() + "\n…[truncated Partial]"
    }
}

fn resolve_api_key() -> Option<String> {
    if let Ok(k) = env::var("ARKS_AGENT_API_KEY") {
        if !k.is_empty() {
            return Some(k);
        }
    }
    if let Ok(k) = env::var("OPENAI_API_KEY") {
        if !k.is_empty() {
            return Some(k);
        }
    }
    if let Ok(map) = load_config_map() {
        if let Some(var) = map.get("api_key_env") {
            if let Ok(k) = env::var(var) {
                if !k.is_empty() {
                    return Some(k);
                }
            }
        }
    }
    None
}

fn cmd_review(args: &[String]) -> ExitCode {
    if args.iter().any(|a| a == "-h" || a == "--help") {
        eprintln!(
            "arks agent review [--file PATH] — openai-compat chat Partial (stdin or --file)"
        );
        return ExitCode::SUCCESS;
    }
    let map = load_config_map().unwrap_or_default();
    let id = env::var("ARKS_AGENT_PROVIDER")
        .ok()
        .or_else(|| map.get("provider").cloned())
        .unwrap_or_else(|| "ollama".into());
    let base = effective_base_url(&id, flag_value(args, &["--base-url"]));
    let model = flag_value(args, &["--model"])
        .map(str::to_string)
        .or_else(|| map.get("model").cloned())
        .unwrap_or_else(|| "llama3.2".into());
    let input = match read_review_input(args) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("arks agent review: {e}");
            return ExitCode::from(2);
        }
    };
    let prompt = format!(
        "You are arks agent review (Partial; not Claimed). Review the following diff/text briefly:\n\n{input}"
    );
    println!("provider={id}");
    println!("base_url={base}");
    println!("model={model}");
    println!("input_bytes={}", input.len());
    match http_post_chat(&base, &model, &prompt) {
        Ok((code, body)) => {
            println!("chat_http={code}");
            println!("{body}");
            eprintln!("note: Partial openai-compat review — design ≠ Claimed");
            if code == 0 || code >= 400 {
                ExitCode::from(1)
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(e) => {
            println!("chat_http=skipped");
            eprintln!("{e}");
            eprintln!(
                "arks agent review: Partial — provider unreachable; structured refuse \
                 (start ollama/openai-compat or fix base_url). Not Claimed."
            );
            ExitCode::from(2)
        }
    }
}

fn cmd_cev_check(args: &[String]) -> ExitCode {
    if args.iter().any(|a| a == "-h" || a == "--help") {
        eprintln!(
            "arks agent cev-check [PATH] — structural CEV/config inspection (Partial; no LLM Claimed)"
        );
        return ExitCode::SUCCESS;
    }
    let target = args
        .iter()
        .find(|a| !a.starts_with('-'))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    println!("path={}", target.display());
    let mut findings = Vec::new();
    if !target.exists() {
        findings.push(format!("missing path: {}", target.display()));
    } else if target.is_file() {
        match fs::read_to_string(&target) {
            Ok(raw) => {
                println!("bytes={}", raw.len());
                if raw.contains('\u{ac00}') {
                    findings.push(
                        "Hangul detected in inspected file (English-only product text rule)".into(),
                    );
                }
                if raw.to_ascii_lowercase().contains("claimed")
                    && raw.to_ascii_lowercase().contains("pass")
                    && !raw.contains("not Claimed")
                {
                    findings.push(
                        "possible invent-Claimed phrasing — review honesty (rule 28)".into(),
                    );
                }
                if raw.len() > 512 * 1024 {
                    findings.push("file >512KiB — CEV depth Partial".into());
                }
            }
            Err(e) => findings.push(format!("read error: {e}")),
        }
    } else {
        // Directory: look for common CEV / evidence markers (structural only).
        let markers = [
            ".spec",
            ".spec.local",
            "verify",
            "AGENTS.md",
            "docs/completion.md",
        ];
        for m in markers {
            let p = target.join(m);
            println!(
                "marker={} {}",
                m,
                if p.exists() { "present" } else { "absent" }
            );
        }
        let conf = config_path();
        println!(
            "agent_config={} {}",
            conf.display(),
            if conf.is_file() { "present" } else { "absent" }
        );
    }
    if findings.is_empty() {
        println!("cev_check=ok (Partial structural; LLM CEV not Claimed)");
        ExitCode::SUCCESS
    } else {
        for f in &findings {
            eprintln!("finding: {f}");
        }
        println!("cev_check=findings (Partial; not Claimed)");
        ExitCode::from(1)
    }
}

fn cmd_run(args: &[String]) -> ExitCode {
    let sub = args.first().map(String::as_str).unwrap_or("");
    match sub {
        "" | "-h" | "--help" | "help" => {
            eprintln!("arks agent run — review|cev-check|doctor [args…] (Partial one-shot)");
            ExitCode::SUCCESS
        }
        "review" => cmd_review(&args[1..]),
        "cev-check" => cmd_cev_check(&args[1..]),
        "doctor" => cmd_doctor(&args[1..]),
        other => {
            eprintln!(
                "arks agent run {other}: Partial — known jobs: review|cev-check|doctor; \
                 custom bot scripts not Claimed"
            );
            ExitCode::from(2)
        }
    }
}

fn cmd_serve_bot(kind: &str, args: &[String]) -> ExitCode {
    if args.iter().any(|a| a == "-h" || a == "--help") {
        eprintln!(
            "arks agent {kind} [--listen HOST:PORT] — loopback health Partial (not a chatbot)"
        );
        return ExitCode::SUCCESS;
    }
    let listen = flag_value(args, &["--listen"])
        .unwrap_or("127.0.0.1:3930")
        .to_string();
    let addr: std::net::SocketAddr = match listen.parse() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("arks agent {kind}: bad --listen: {e}");
            return ExitCode::from(2);
        }
    };
    if !addr.ip().is_loopback() {
        eprintln!(
            "arks agent {kind}: non-loopback bind Partial — ops owns firewall; not Claimed WAN-safe"
        );
    }
    let listener = match std::net::TcpListener::bind(addr) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("arks agent {kind}: bind failed: {e}");
            return ExitCode::from(1);
        }
    };
    eprintln!(
        "arks agent {kind} listening on http://{}/ (Partial health only — LLM serve not Claimed)",
        listener
            .local_addr()
            .map(|a| a.to_string())
            .unwrap_or_else(|_| listen.clone())
    );
    // Single-accept Partial demo: respond once then exit (long-running bot Missing).
    match listener.accept() {
        Ok((mut stream, peer)) => {
            let mut buf = [0u8; 1024];
            let _ = stream.read(&mut buf);
            let body = format!(
                "{{\"ok\":true,\"service\":\"arks-agent\",\"mode\":\"{kind}\",\"partial\":true,\"peer\":\"{peer}\"}}"
            );
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(resp.as_bytes());
            eprintln!(
                "arks agent {kind}: served one Partial health response then exit \
                 (long-running bot Missing — design ≠ Claimed)"
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("arks agent {kind}: accept failed: {e}");
            ExitCode::from(1)
        }
    }
}
