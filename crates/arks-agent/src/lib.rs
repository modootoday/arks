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
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

/// Top-level groups shown in help.
pub const GROUPS: &[&str] = &[
    "chat",
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
    provider      list|show|test|set|smoke LLM endpoints (openai-compat first; ollama smoke Partial)
    config        show|path agent config (secrets redacted)
    doctor        Config + TCP reachability sanity check
    chat          Multi-turn openai-compat chat (`--message TEXT` repeated)
    review        Review stdin/file via openai-compat chat (Partial; --smoke [--mock])
    cev-check     Structural CEV path/config inspection (Partial; no LLM Claimed)
    run           One-shot job: review|cev-check|doctor (Partial)
    serve / bot   Loopback health + chat completions proxy (JSON + stream SSE Partial; Claimed LLM e2e Missing)

NOTES:
    Entry is `arks agent` (single arks release binary). Not a separate arks-agent product.
    Not required for git alias DoD.
    Local Ollama default: http://127.0.0.1:11434/v1
    `provider smoke [ollama|--mock]` — local chat Partial (daemon or in-process mock)
    `review --smoke --mock` — fixture + openai-compat mock server Partial
    Native HTTP / TCP only — never spawns git / gh / vendor CLIs (including ollama CLI).
    Design ≠ Claimed ([[sot:root/arks-agent-cli-surface]]).
"
    );
}

/// Ephemeral openai-compat mock (Partial): `/v1/models` + `/v1/chat/completions`.
/// Never invents Claimed LLM e2e; never spawns ollama/openai CLIs.
fn spawn_openai_compat_mock()
-> Result<(SocketAddr, Arc<AtomicBool>, thread::JoinHandle<()>), String> {
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    let addr = listener.local_addr().map_err(|e| e.to_string())?;
    let _ = listener.set_nonblocking(true);
    let stop = Arc::new(AtomicBool::new(false));
    let stop2 = stop.clone();
    let handle = thread::spawn(move || {
        while !stop2.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    let _ = stream.set_nonblocking(false);
                    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
                    let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
                    let mut buf = Vec::new();
                    let mut tmp = [0u8; 1024];
                    loop {
                        match stream.read(&mut tmp) {
                            Ok(0) => break,
                            Ok(n) => {
                                buf.extend_from_slice(&tmp[..n]);
                                if buf.windows(4).any(|w| w == b"\r\n\r\n") {
                                    // Drain declared content-length if present (Partial).
                                    let head = String::from_utf8_lossy(&buf);
                                    let cl = head.lines().find_map(|l| {
                                        l.split_once(':').and_then(|(k, v)| {
                                            if k.eq_ignore_ascii_case("content-length") {
                                                v.trim().parse::<usize>().ok()
                                            } else {
                                                None
                                            }
                                        })
                                    });
                                    if let Some(need) = cl {
                                        if let Some(i) =
                                            buf.windows(4).position(|w| w == b"\r\n\r\n")
                                        {
                                            let body_start = i + 4;
                                            if buf.len().saturating_sub(body_start) >= need {
                                                break;
                                            }
                                        }
                                    } else {
                                        break;
                                    }
                                }
                                if buf.len() > 256 * 1024 {
                                    break;
                                }
                            }
                            Err(_) => break,
                        }
                    }
                    let req = String::from_utf8_lossy(&buf);
                    let start = req.lines().next().unwrap_or("");
                    let path = start.split_whitespace().nth(1).unwrap_or("/");
                    let (status, ctype, body) = if path.contains("/models") {
                        (
                            200,
                            "application/json",
                            r#"{"object":"list","data":[{"id":"arks-mock","object":"model"}]}"#
                                .to_string(),
                        )
                    } else if path.contains("/chat/completions") {
                        (
                            200,
                            "application/json",
                            r#"{"id":"chatcmpl-arks-mock","object":"chat.completion","choices":[{"index":0,"message":{"role":"assistant","content":"arks agent openai-compat mock Partial: LGTM (design ≠ Claimed LLM e2e)"},"finish_reason":"stop"}]}"#
                                .to_string(),
                        )
                    } else {
                        (
                            404,
                            "application/json",
                            r#"{"error":"arks mock Partial unknown path"}"#.to_string(),
                        )
                    };
                    let resp = format!(
                        "HTTP/1.1 {status} OK\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nConnection: close\r\nX-Arks-Agent-Mock: partial\r\n\r\n{body}",
                        body.len()
                    );
                    let _ = stream.write_all(resp.as_bytes());
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(5));
                }
                Err(_) => break,
            }
        }
    });
    thread::sleep(Duration::from_millis(20));
    Ok((addr, stop, handle))
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
        "chat" => cmd_chat(&args[1..]),
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
    tcp_probe_retries(base_url, 3)
}

/// Partial provider reachability with short backoff (not Claimed LLM e2e).
fn tcp_probe_retries(base_url: &str, attempts: u32) -> Result<(), String> {
    let (host, port) = parse_host_port(base_url)?;
    let attempts = attempts.max(1);
    let mut last = String::new();
    for i in 1..=attempts {
        match TcpStream::connect((host.as_str(), port)) {
            Ok(stream) => {
                let _ = stream.set_read_timeout(Some(Duration::from_secs(3)));
                let _ = stream.set_write_timeout(Some(Duration::from_secs(3)));
                if i > 1 {
                    eprintln!(
                        "note: provider tcp retry ok attempt={i}/{attempts} (Partial backoff; \
                         Claimed LLM e2e Missing)"
                    );
                }
                return Ok(());
            }
            Err(e) => {
                last = format!("connect {host}:{port}: {e}");
                eprintln!(
                    "note: provider tcp fail attempt={i}/{attempts}: {last} ({})",
                    classify_provider_err(&last)
                );
                if i < attempts {
                    std::thread::sleep(Duration::from_millis(40 * u64::from(i)));
                }
            }
        }
    }
    Err(format!(
        "{last} [{}] (retries={attempts}; Partial; Claimed LLM e2e Missing)",
        classify_provider_err(&last)
    ))
}

fn classify_provider_err(err: &str) -> &'static str {
    let e = err.to_ascii_lowercase();
    if e.contains("connection refused") {
        "err_class=conn_refused"
    } else if e.contains("timed out") || e.contains("timeout") {
        "err_class=timeout"
    } else if e.contains("failed to lookup")
        || e.contains("dns")
        || e.contains("name or service")
        || e.contains("no such host")
    {
        "err_class=dns"
    } else if e.contains("https") {
        "err_class=https_partial"
    } else if e.contains("connection reset") || e.contains("broken pipe") {
        "err_class=reset"
    } else {
        "err_class=other"
    }
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
    stream.set_read_timeout(Some(Duration::from_secs(5))).ok();
    stream.set_write_timeout(Some(Duration::from_secs(5))).ok();
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
                "arks agent provider — list|show <id>|test <id>|set <id>|smoke [ollama|--mock] \
                 [--base-url …] [--api-key-env VAR] [--model NAME]\n  \
                 smoke: optional local ollama or --mock openai-compat chat Partial (no CLI spawn); \
                 Claimed LLM chatbot e2e Missing unless real evidence"
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
            let key_env = env::var("ARKS_AGENT_API_KEY_ENV").ok().or_else(|| {
                load_config_map()
                    .ok()
                    .and_then(|m| m.get("api_key_env").cloned())
            });
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
                    println!("error_class={}", classify_provider_err(&e));
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
                        eprintln!(
                            "note: Partial probe — non-success HTTP ({}; design ≠ Claimed)",
                            classify_provider_err(&format!("http {code}"))
                        );
                        ExitCode::from(1)
                    } else {
                        ExitCode::SUCCESS
                    }
                }
                Err(e) => {
                    println!("models_http=skipped");
                    println!("error_class={}", classify_provider_err(&e));
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
        "smoke" => cmd_provider_ollama_smoke(args),
        other => {
            eprintln!("arks agent provider: unknown `{other}` (list|show|test|set|smoke)");
            ExitCode::from(2)
        }
    }
}

/// Optional local Ollama / openai-compat mock smoke Partial: TCP + `/models` + chat.
/// Never spawns the `ollama` CLI. Claimed LLM chatbot e2e remains Missing without evidence.
fn cmd_provider_ollama_smoke(args: &[String]) -> ExitCode {
    let want_mock = args.iter().any(|a| a == "--mock");
    let id = args
        .iter()
        .skip(1)
        .map(String::as_str)
        .find(|s| !s.starts_with('-'))
        .unwrap_or(if want_mock { "openai-compat" } else { "ollama" });
    if id != "ollama" && id != "openai-compat" {
        eprintln!(
            "arks agent provider smoke: optional local path is ollama|openai-compat [--mock] \
             (got `{id}`); Claimed vendor e2e Missing"
        );
        return ExitCode::from(2);
    }
    let model = flag_value(args, &["--model"])
        .map(str::to_string)
        .or_else(|| load_config_map().ok().and_then(|m| m.get("model").cloned()))
        .unwrap_or_else(|| "llama3.2".into());

    let mut mock_stop: Option<Arc<AtomicBool>> = None;
    let mut mock_handle: Option<thread::JoinHandle<()>> = None;
    let base = if want_mock {
        match spawn_openai_compat_mock() {
            Ok((addr, stop, handle)) => {
                mock_stop = Some(stop);
                mock_handle = Some(handle);
                println!("openai_compat_mock=partial");
                format!("http://{addr}/v1")
            }
            Err(e) => {
                eprintln!("arks agent provider smoke --mock: {e}");
                return ExitCode::from(1);
            }
        }
    } else {
        effective_base_url(id, flag_value(args, &["--base-url"]))
    };

    println!("provider_smoke=partial");
    println!("provider={id}");
    println!("base_url={base}");
    println!("model={model}");
    println!("spawns_ollama_cli=never");
    println!("claimed_llm_chatbot_e2e=Missing");
    if !want_mock && base.starts_with("https://") {
        println!("ollama_smoke=Missing");
        println!(
            "missing_reason=HTTPS provider smoke Partial not wired; use http://127.0.0.1:11434/v1 or --mock"
        );
        eprintln!("note: Claimed LLM chatbot e2e Missing");
        return ExitCode::from(2);
    }
    match tcp_probe(&base) {
        Ok(()) => println!("tcp=ok"),
        Err(e) => {
            println!("tcp=fail");
            println!("ollama_smoke=Missing");
            println!("missing_reason=local provider not reachable ({e})");
            println!("error_class={}", classify_provider_err(&e));
            eprintln!(
                "note: optional local provider smoke Partial — daemon absent; try --mock; \
                 Claimed LLM chatbot e2e still Missing (not invented Claimed)"
            );
            if let Some(stop) = mock_stop {
                stop.store(true, Ordering::SeqCst);
            }
            if let Some(h) = mock_handle {
                let _ = h.join();
            }
            return ExitCode::from(1);
        }
    }
    match http_get_models(&base) {
        Ok((code, body)) => {
            println!("models_http={code}");
            if !body.is_empty() {
                let preview: String = body.chars().take(160).collect();
                println!("models_preview={preview}");
            }
        }
        Err(e) => {
            println!("models_http=fail");
            println!("error_class={}", classify_provider_err(&e));
            eprintln!("{e}");
        }
    }
    let code = match http_post_chat(&base, &model, "arks agent provider smoke Partial ping") {
        Ok((code, body)) => {
            println!("chat_http={code}");
            let preview: String = body.chars().take(200).collect();
            if !preview.is_empty() {
                println!("chat_preview={preview}");
            }
            if (200..300).contains(&code) {
                if want_mock {
                    println!("mock_smoke=ok");
                } else {
                    println!("ollama_smoke=ok");
                }
                eprintln!(
                    "note: local {} smoke Partial succeeded — Claimed LLM chatbot e2e still Missing \
                     (one-shot smoke ≠ product e2e)",
                    if want_mock {
                        "openai-compat mock"
                    } else {
                        "ollama"
                    }
                );
                ExitCode::SUCCESS
            } else {
                println!("provider_smoke=partial_http_{code}");
                eprintln!("note: provider reachable but chat HTTP {code}; Claimed e2e Missing");
                ExitCode::from(1)
            }
        }
        Err(e) => {
            println!("chat_http=fail");
            if want_mock {
                println!("mock_smoke=fail");
            } else {
                println!("ollama_smoke=Missing");
            }
            println!("error={}", e);
            println!("error_class={}", classify_provider_err(&e));
            eprintln!(
                "note: optional local provider smoke Partial — chat path failed; \
                 Claimed LLM chatbot e2e Missing"
            );
            ExitCode::from(1)
        }
    };
    if let Some(stop) = mock_stop {
        stop.store(true, Ordering::SeqCst);
    }
    if let Some(h) = mock_handle {
        let _ = h.join();
    }
    code
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
        return Err("review needs stdin text or --file PATH (Partial; never spawns git/gh)".into());
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
    // Connect with Partial retry; HTTP exchange itself is single-shot (POST not blindly retried).
    let mut last_conn = String::new();
    let mut stream = None;
    for i in 1..=3u32 {
        match TcpStream::connect((host.as_str(), port)) {
            Ok(s) => {
                if i > 1 {
                    eprintln!(
                        "note: chat connect retry ok attempt={i}/3 ({})",
                        classify_provider_err(&last_conn)
                    );
                }
                stream = Some(s);
                break;
            }
            Err(e) => {
                last_conn = format!("connect {host}:{port}: {e}");
                eprintln!(
                    "note: chat connect fail attempt={i}/3: {last_conn} ({})",
                    classify_provider_err(&last_conn)
                );
                if i < 3 {
                    std::thread::sleep(Duration::from_millis(40 * u64::from(i)));
                }
            }
        }
    }
    let mut stream = stream.ok_or_else(|| {
        format!(
            "{last_conn} [{}] (Partial chat; Claimed LLM e2e Missing)",
            classify_provider_err(&last_conn)
        )
    })?;
    stream.set_read_timeout(Some(Duration::from_secs(60))).ok();
    stream.set_write_timeout(Some(Duration::from_secs(30))).ok();
    let mut headers = format!(
        "POST {path} HTTP/1.1\r\nHost: {host}:{port}\r\nContent-Type: application/json\r\nAccept: application/json\r\nContent-Length: {}\r\nConnection: close\r\n",
        body.len()
    );
    if let Some(key) = resolve_api_key() {
        headers.push_str(&format!("Authorization: Bearer {key}\r\n"));
    }
    headers.push_str("\r\n");
    stream.write_all(headers.as_bytes()).map_err(|e| {
        format!(
            "write headers: {e} [{}]",
            classify_provider_err(&e.to_string())
        )
    })?;
    stream.write_all(body.as_bytes()).map_err(|e| {
        format!(
            "write body: {e} [{}]",
            classify_provider_err(&e.to_string())
        )
    })?;
    let mut buf = Vec::new();
    stream
        .read_to_end(&mut buf)
        .map_err(|e| format!("read: {e} [{}]", classify_provider_err(&e.to_string())))?;
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
    if status == 0 || status >= 500 {
        eprintln!(
            "note: chat upstream status={status} ({}) — Partial; no blind POST retry",
            classify_provider_err(&format!("http {status}"))
        );
    }
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

fn repeated_flag_values(args: &[String], name: &str) -> Result<Vec<String>, String> {
    let mut values = Vec::new();
    let mut i = 0usize;
    while i < args.len() {
        if args[i] == name {
            i += 1;
            values.push(
                args.get(i)
                    .cloned()
                    .ok_or_else(|| format!("{name} needs a value"))?,
            );
        } else if let Some(value) = args[i].strip_prefix(&format!("{name}=")) {
            values.push(value.to_string());
        }
        i += 1;
    }
    Ok(values)
}

/// Product multi-turn path for an OpenAI-compatible provider.
///
/// Every request carries the complete prior user/assistant history. This is a
/// native HTTP path and never invokes a provider CLI.
fn cmd_chat(args: &[String]) -> ExitCode {
    if args.iter().any(|a| a == "-h" || a == "--help") {
        eprintln!(
            "arks agent chat --message TEXT [--message TEXT ...] [--base-url URL] [--model MODEL]\n\
             Sends each turn with complete conversation history over native openai-compat HTTP."
        );
        return ExitCode::SUCCESS;
    }
    let messages = match repeated_flag_values(args, "--message") {
        Ok(v) if !v.is_empty() => v,
        Ok(_) => {
            eprintln!("arks agent chat: at least one --message TEXT is required");
            return ExitCode::from(2);
        }
        Err(e) => {
            eprintln!("arks agent chat: {e}");
            return ExitCode::from(2);
        }
    };
    let map = load_config_map().unwrap_or_default();
    let id = env::var("ARKS_AGENT_PROVIDER")
        .ok()
        .or_else(|| map.get("provider").cloned())
        .unwrap_or_else(|| "openai-compat".into());
    let base = effective_base_url(&id, flag_value(args, &["--base-url"]));
    let model = flag_value(args, &["--model"])
        .map(str::to_string)
        .or_else(|| map.get("model").cloned())
        .unwrap_or_else(|| "arks-fixture".into());
    let mut history: Vec<(&str, String)> = Vec::new();
    println!("provider={id}");
    println!("base_url={base}");
    println!("model={model}");
    for (turn, user) in messages.iter().enumerate() {
        history.push(("user", truncate_chars(user, 12_000)));
        let rendered = history
            .iter()
            .map(|(role, content)| {
                format!(
                    r#"{{"role":{},"content":{}}}"#,
                    json_escape(role),
                    json_escape(content)
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        let body = format!(
            r#"{{"model":{},"messages":[{}],"stream":false}}"#,
            json_escape(&model),
            rendered
        );
        let (status, response) = match http_post_chat_raw(&base, &body) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("arks agent chat turn {}: {e}", turn + 1);
                return ExitCode::from(1);
            }
        };
        if !(200..400).contains(&status) {
            eprintln!("arks agent chat turn {}: upstream HTTP {status}", turn + 1);
            return ExitCode::from(1);
        }
        let Some(assistant) = extract_json_string_field(&response, "content") else {
            eprintln!(
                "arks agent chat turn {}: response has no assistant content",
                turn + 1
            );
            return ExitCode::from(1);
        };
        println!("turn_{}_http={status}", turn + 1);
        println!("assistant_{}={assistant}", turn + 1);
        history.push(("assistant", assistant));
    }
    println!("chat_turns_completed={}", messages.len());
    ExitCode::SUCCESS
}

fn cmd_review(args: &[String]) -> ExitCode {
    if args.iter().any(|a| a == "-h" || a == "--help") {
        eprintln!(
            "arks agent review [--file PATH] [--smoke] [--mock] — openai-compat chat Partial \
             (stdin or --file; --smoke fixture; --mock = in-process openai-compat stub)"
        );
        return ExitCode::SUCCESS;
    }
    let smoke = args.iter().any(|a| a == "--smoke");
    let want_mock = args.iter().any(|a| a == "--mock");
    if want_mock && !smoke {
        eprintln!("arks agent review: --mock requires --smoke (Partial mock harness)");
        return ExitCode::from(2);
    }
    let map = load_config_map().unwrap_or_default();
    let id = env::var("ARKS_AGENT_PROVIDER")
        .ok()
        .or_else(|| map.get("provider").cloned())
        .unwrap_or_else(|| "ollama".into());
    let model = flag_value(args, &["--model"])
        .map(str::to_string)
        .or_else(|| map.get("model").cloned())
        .unwrap_or_else(|| "llama3.2".into());

    let mut mock_stop: Option<Arc<AtomicBool>> = None;
    let mut mock_handle: Option<thread::JoinHandle<()>> = None;
    let base = if want_mock {
        match spawn_openai_compat_mock() {
            Ok((addr, stop, handle)) => {
                mock_stop = Some(stop);
                mock_handle = Some(handle);
                println!("openai_compat_mock=partial");
                println!("mock_listen=http://{addr}/v1");
                format!("http://{addr}/v1")
            }
            Err(e) => {
                eprintln!("arks agent review --smoke --mock: {e}");
                return ExitCode::from(1);
            }
        }
    } else {
        effective_base_url(&id, flag_value(args, &["--base-url"]))
    };

    if smoke {
        println!("review_smoke=partial");
        println!("spawns_ollama_cli=never");
        println!("claimed_llm_review_e2e=Missing");
    }
    let input = if smoke {
        "arks agent review --smoke fixture (Partial):\n\
         - do not invent Claimed\n\
         - never spawn git/gh/ollama CLI\n\
         fn example() { /* empty Partial */ }\n"
            .to_string()
    } else {
        match read_review_input(args) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("arks agent review: {e}");
                return ExitCode::from(2);
            }
        }
    };
    let prompt = format!(
        "You are arks agent review (Partial; not Claimed). Review the following diff/text briefly:\n\n{input}"
    );
    println!(
        "provider={}",
        if want_mock {
            "openai-compat-mock"
        } else {
            id.as_str()
        }
    );
    println!("base_url={base}");
    println!("model={model}");
    println!("input_bytes={}", input.len());
    let code = match http_post_chat(&base, &model, &prompt) {
        Ok((code, body)) => {
            println!("chat_http={code}");
            if smoke {
                let preview: String = body.chars().take(200).collect();
                if !preview.is_empty() {
                    println!("chat_preview={preview}");
                }
                if code > 0 && code < 400 {
                    println!("review_smoke=ok");
                    if want_mock {
                        println!("review_mock_smoke=ok");
                    }
                    eprintln!(
                        "note: review --smoke{} Partial succeeded — Claimed LLM review e2e still Missing \
                         (one-shot fixture ≠ product e2e)",
                        if want_mock { " --mock" } else { "" }
                    );
                    ExitCode::SUCCESS
                } else {
                    println!("review_smoke=partial_http_{code}");
                    eprintln!(
                        "note: review --smoke Partial HTTP {code}; Claimed LLM review e2e Missing"
                    );
                    ExitCode::from(1)
                }
            } else {
                println!("{body}");
                eprintln!("note: Partial openai-compat review — design ≠ Claimed");
                if code == 0 || code >= 400 {
                    ExitCode::from(1)
                } else {
                    ExitCode::SUCCESS
                }
            }
        }
        Err(e) => {
            println!("chat_http=skipped");
            eprintln!("{e}");
            if smoke && want_mock {
                println!("review_smoke=fail");
                println!("review_mock_smoke=fail");
                eprintln!(
                    "note: review --smoke --mock Partial failed against in-process stub; \
                     Claimed LLM review e2e Missing"
                );
                ExitCode::from(1)
            } else if smoke {
                println!("review_smoke=Missing");
                println!("missing_reason=provider unreachable for fixture smoke");
                eprintln!(
                    "note: review --smoke Partial — daemon absent; try --mock; \
                     Claimed LLM review e2e Missing (not invented Claimed); never spawns ollama CLI"
                );
                ExitCode::SUCCESS
            } else {
                eprintln!(
                    "arks agent review: Partial — provider unreachable; structured refuse \
                     (start ollama/openai-compat or fix base_url). Not Claimed."
                );
                ExitCode::from(2)
            }
        }
    };
    if let Some(stop) = mock_stop {
        stop.store(true, Ordering::SeqCst);
    }
    if let Some(h) = mock_handle {
        let _ = h.join();
    }
    code
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
                    findings
                        .push("possible invent-Claimed phrasing — review honesty (rule 28)".into());
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
            "arks agent {kind} [--listen HOST:PORT] [--once] — loopback health + \
             POST /v1/chat/completions openai-compat proxy Partial (JSON body pass-through + \
             stream=true SSE Partial). Claimed LLM chatbot / review e2e is Missing — \
             design ≠ Claimed."
        );
        return ExitCode::SUCCESS;
    }
    let once = args.iter().any(|a| a == "--once");
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
    let _ = listener.set_nonblocking(false);
    let map = load_config_map().unwrap_or_default();
    let provider_id = env::var("ARKS_AGENT_PROVIDER")
        .ok()
        .or_else(|| map.get("provider").cloned())
        .unwrap_or_else(|| "ollama".into());
    let base = effective_base_url(&provider_id, flag_value(args, &["--base-url"]));
    let default_model = flag_value(args, &["--model"])
        .map(str::to_string)
        .or_else(|| map.get("model").cloned())
        .unwrap_or_else(|| "llama3.2".into());
    eprintln!(
        "arks agent {kind} listening on http://{}/ (Partial health + chat proxy → {base}; \
         stream SSE Partial OK; Claimed LLM e2e Missing; {} )",
        listener
            .local_addr()
            .map(|a| a.to_string())
            .unwrap_or_else(|_| listen.clone()),
        if once {
            "--once"
        } else {
            "long-running Partial"
        }
    );
    let mut served = 0u64;
    loop {
        match listener.accept() {
            Ok((mut stream, peer)) => {
                let req = read_http_request_limited(&mut stream, 256 * 1024);
                match req {
                    Ok((method, path, req_body)) => {
                        let path_only = path.split('?').next().unwrap_or(path.as_str());
                        if method == "POST"
                            && (path_only == "/v1/chat/completions"
                                || path_only == "/chat/completions")
                        {
                            if chat_body_wants_stream(&req_body) {
                                match proxy_chat_completion_stream(
                                    &mut stream,
                                    &base,
                                    &default_model,
                                    &req_body,
                                ) {
                                    Ok(()) => {}
                                    Err(e) => {
                                        let body = format!(
                                            "{{\"error\":\"partial stream proxy failed\",\"detail\":{},\"partial\":true,\"claimed_e2e\":\"Missing\"}}",
                                            json_escape(&e)
                                        );
                                        let _ = write_http_json(&mut stream, 502, &body);
                                    }
                                }
                            } else {
                                match proxy_chat_completion(&base, &default_model, &req_body) {
                                    Ok((code, resp)) => {
                                        let st = if code == 0 { 502 } else { code };
                                        let _ = write_http_json(&mut stream, st, &resp);
                                    }
                                    Err(e) => {
                                        let body = format!(
                                            "{{\"error\":\"partial proxy failed\",\"detail\":{},\"partial\":true,\"claimed_e2e\":\"Missing\"}}",
                                            json_escape(&e)
                                        );
                                        let _ = write_http_json(&mut stream, 502, &body);
                                    }
                                }
                            }
                        } else {
                            let body = format!(
                                "{{\"ok\":true,\"service\":\"arks-agent\",\"mode\":\"{kind}\",\
                                 \"partial\":true,\"peer\":\"{peer}\",\"served\":{},\
                                 \"provider\":\"{provider_id}\",\"upstream\":{},\
                                 \"chat\":\"POST /v1/chat/completions\",\
                                 \"stream\":\"Partial SSE when stream=true\",\
                                 \"claimed_e2e\":\"Missing\"}}",
                                served + 1,
                                json_escape(&base)
                            );
                            let _ = write_http_json(&mut stream, 200, &body);
                        }
                    }
                    Err(e) => {
                        let body = format!(
                            "{{\"error\":\"bad request\",\"detail\":{},\"partial\":true}}",
                            json_escape(&e)
                        );
                        let _ = write_http_json(&mut stream, 400, &body);
                    }
                }
                served += 1;
                if once {
                    eprintln!("arks agent {kind}: served one Partial response (--once) then exit");
                    return ExitCode::SUCCESS;
                }
                if served == 1 {
                    eprintln!(
                        "arks agent {kind}: long-running Partial health+chat proxy (Ctrl-C to stop; \
                         Claimed LLM chatbot e2e Missing — design ≠ Claimed)"
                    );
                }
            }
            Err(e) => {
                eprintln!("arks agent {kind}: accept failed: {e}");
                return ExitCode::from(1);
            }
        }
    }
}

fn write_http_json(stream: &mut impl Write, status: u16, body: &str) -> std::io::Result<()> {
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        502 => "Bad Gateway",
        _ => "OK",
    };
    let resp = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(resp.as_bytes())
}

fn chat_body_wants_stream(body: &str) -> bool {
    // Partial JSON sniff — not a Claimed parser.
    body.contains("\"stream\"")
        && (body.contains("\"stream\":true")
            || body.contains("\"stream\": true")
            || body.contains("\"stream\":1"))
}

/// Minimal HTTP/1.1 request read for agent serve (Partial; size-capped).
fn read_http_request_limited(
    stream: &mut impl Read,
    max_body: usize,
) -> Result<(String, String, String), String> {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 2048];
    loop {
        let n = stream.read(&mut tmp).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&tmp[..n]);
        if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            let (head, rest) = buf.split_at(i + 4);
            let head_str = String::from_utf8_lossy(head);
            let mut lines = head_str.split("\r\n");
            let start = lines.next().unwrap_or("");
            let mut parts = start.split_whitespace();
            let method = parts.next().unwrap_or("GET").to_string();
            let path = parts.next().unwrap_or("/").to_string();
            let mut content_length = 0usize;
            for line in lines {
                if line.is_empty() {
                    break;
                }
                if let Some((k, v)) = line.split_once(':') {
                    if k.trim().eq_ignore_ascii_case("content-length") {
                        content_length = v.trim().parse().unwrap_or(0);
                    }
                }
            }
            if content_length > max_body {
                return Err(format!("body too large ({content_length} > {max_body})"));
            }
            let mut body = rest.to_vec();
            while body.len() < content_length {
                let n = stream.read(&mut tmp).map_err(|e| e.to_string())?;
                if n == 0 {
                    break;
                }
                body.extend_from_slice(&tmp[..n]);
            }
            body.truncate(content_length);
            let body = String::from_utf8_lossy(&body).into_owned();
            return Ok((method, path, body));
        }
        if buf.len() > max_body + 8192 {
            return Err("headers too large".into());
        }
    }
    Err("incomplete HTTP request".into())
}

/// Forward openai-compat chat body to configured upstream (Partial proxy).
/// Prefer raw JSON pass-through; fall back to synthesized single-turn body.
fn proxy_chat_completion(
    base_url: &str,
    default_model: &str,
    req_body: &str,
) -> Result<(u16, String), String> {
    let body = ensure_chat_body(req_body, default_model, false);
    http_post_chat_raw(base_url, &body)
}

/// Streaming Partial: relay upstream SSE/`text/event-stream` bytes to client.
/// Not Claimed LLM e2e — best-effort chunk forward with size/time caps.
fn proxy_chat_completion_stream(
    client: &mut impl Write,
    base_url: &str,
    default_model: &str,
    req_body: &str,
) -> Result<(), String> {
    if base_url.starts_with("https://") {
        return Err(
            "HTTPS stream Partial — TCP/HTTP loopback only today; use http:// provider or ops TLS front"
                .into(),
        );
    }
    let body = ensure_chat_body(req_body, default_model, true);
    let (host, port) = parse_host_port(base_url)?;
    let path = chat_completions_path(base_url);
    let mut upstream = TcpStream::connect((host.as_str(), port))
        .map_err(|e| format!("connect {host}:{port}: {e}"))?;
    upstream
        .set_read_timeout(Some(Duration::from_secs(60)))
        .ok();
    upstream
        .set_write_timeout(Some(Duration::from_secs(30)))
        .ok();
    let mut headers = format!(
        "POST {path} HTTP/1.1\r\nHost: {host}:{port}\r\nContent-Type: application/json\r\nAccept: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n",
        body.len()
    );
    if let Some(key) = resolve_api_key() {
        headers.push_str(&format!("Authorization: Bearer {key}\r\n"));
    }
    headers.push_str("\r\n");
    upstream
        .write_all(headers.as_bytes())
        .map_err(|e| e.to_string())?;
    upstream
        .write_all(body.as_bytes())
        .map_err(|e| e.to_string())?;

    // Read upstream status line + headers, then relay body as SSE Partial.
    let mut head = Vec::new();
    let mut tmp = [0u8; 2048];
    loop {
        let n = upstream.read(&mut tmp).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        head.extend_from_slice(&tmp[..n]);
        if head.windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }
        if head.len() > 64 * 1024 {
            return Err("upstream headers too large".into());
        }
    }
    let split = head
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or_else(|| "incomplete upstream response".to_string())?;
    let (hdr, rest) = head.split_at(split + 4);
    let hdr_str = String::from_utf8_lossy(hdr);
    let status = hdr_str
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(0);
    if status == 0 || status >= 400 {
        let preview = String::from_utf8_lossy(rest);
        return Err(format!(
            "upstream HTTP {status}: {}",
            preview.chars().take(400).collect::<String>()
        ));
    }
    let out_head = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nConnection: close\r\nX-Arks-Partial: stream\r\nX-Arks-Claimed-E2E: Missing\r\n\r\n";
    client
        .write_all(out_head.as_bytes())
        .map_err(|e| e.to_string())?;
    if !rest.is_empty() {
        client.write_all(rest).map_err(|e| e.to_string())?;
    }
    let mut total = rest.len();
    let mut buf = [0u8; 4096];
    loop {
        match upstream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                client.write_all(&buf[..n]).map_err(|e| e.to_string())?;
                total += n;
                if total > 8 * 1024 * 1024 {
                    let _ = client.write_all(b"\ndata: [DONE]\n\n");
                    break;
                }
            }
            Err(e)
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut =>
            {
                break;
            }
            Err(e) => return Err(e.to_string()),
        }
    }
    let _ = client.flush();
    Ok(())
}

fn ensure_chat_body(req_body: &str, default_model: &str, stream: bool) -> String {
    let trimmed = req_body.trim();
    if trimmed.starts_with('{') && trimmed.contains("\"messages\"") {
        // Pass-through Partial: ensure model + stream flags when absent.
        let mut body = trimmed.to_string();
        if extract_json_string_field(&body, "model").is_none() {
            // Inject model after opening brace.
            body.insert_str(1, &format!("\"model\":{},", json_escape(default_model)));
        }
        if stream && !chat_body_wants_stream(&body) {
            if body.ends_with('}') {
                body.pop();
                if !body.trim_end().ends_with(',') && body.len() > 1 {
                    body.push(',');
                }
                body.push_str("\"stream\":true}");
            }
        }
        if !stream && chat_body_wants_stream(&body) {
            body = body.replace("\"stream\":true", "\"stream\":false");
            body = body.replace("\"stream\": true", "\"stream\":false");
        }
        return body;
    }
    let model =
        extract_json_string_field(req_body, "model").unwrap_or_else(|| default_model.into());
    let prompt = extract_chat_user_content(req_body)
        .unwrap_or_else(|| req_body.chars().take(12_000).collect());
    format!(
        r#"{{"model":{},"messages":[{{"role":"user","content":{}}}],"stream":{}}}"#,
        json_escape(&model),
        json_escape(&truncate_chars(&prompt, 12_000)),
        if stream { "true" } else { "false" }
    )
}

fn chat_completions_path(base_url: &str) -> String {
    let root = base_url.trim_end_matches('/');
    if root.ends_with("/v1") {
        return "/v1/chat/completions".to_string();
    }
    if let Some(rest) = base_url
        .strip_prefix("http://")
        .and_then(|r| r.split_once('/').map(|(_, p)| p))
    {
        let rest = rest.trim_end_matches('/');
        if rest.is_empty() {
            return "/v1/chat/completions".into();
        } else if rest.ends_with("/v1") {
            return format!("/{rest}/chat/completions");
        } else {
            return format!("/{rest}/chat/completions");
        }
    }
    "/v1/chat/completions".into()
}

fn http_post_chat_raw(base_url: &str, body: &str) -> Result<(u16, String), String> {
    if base_url.starts_with("https://") {
        return Err(
            "HTTPS chat Partial — TCP/HTTP loopback only today; use http:// provider or ops TLS front"
                .into(),
        );
    }
    let (host, port) = parse_host_port(base_url)?;
    let path = chat_completions_path(base_url);
    let mut stream = TcpStream::connect((host.as_str(), port))
        .map_err(|e| format!("connect {host}:{port}: {e}"))?;
    stream.set_read_timeout(Some(Duration::from_secs(60))).ok();
    stream.set_write_timeout(Some(Duration::from_secs(30))).ok();
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
    stream
        .write_all(body.as_bytes())
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
    let resp_body = text
        .split("\r\n\r\n")
        .nth(1)
        .unwrap_or("")
        .chars()
        .take(32_768)
        .collect::<String>();
    Ok((status, resp_body))
}

fn extract_json_string_field(raw: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let i = raw.find(&needle)?;
    let after = &raw[i + needle.len()..];
    let after = after.trim_start().trim_start_matches(':').trim_start();
    if !after.starts_with('"') {
        return None;
    }
    let mut out = String::new();
    let mut chars = after.chars().skip(1);
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                if let Some(n) = chars.next() {
                    out.push(n);
                }
            }
            '"' => break,
            c => out.push(c),
        }
    }
    if out.is_empty() { None } else { Some(out) }
}

/// Best-effort: last `"content":"…"` under messages (Partial; not full JSON parse Claimed).
fn extract_chat_user_content(raw: &str) -> Option<String> {
    let mut last = None;
    let mut search = raw;
    while let Some(i) = search.find("\"content\"") {
        let after = &search[i + "\"content\"".len()..];
        let after = after.trim_start().trim_start_matches(':').trim_start();
        if after.starts_with('"') {
            if let Some(v) = extract_json_string_field(&format!("\"content\":{after}"), "content") {
                last = Some(v);
            }
        }
        search = &search[i + 1..];
    }
    last
}
