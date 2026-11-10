//! Wave Sv / Sv2 server porcelain (upload-pack / receive-pack / HTTP / daemon).
//!
//! Cite: `.spec/sot/cli-surface.sot.md` Wave Sv, `.spec/sot/arks-server.sot.md`,
//! `.spec/sot/arks-self-host-server.sot.md`
//! Forge-in-serve: Partial local API via `arks-hub` (design ≠ Claimed).
//! No CELIR. Lease not required. No `gix`.

use crate::error::{CliError, CliResult};
use arks_hub::{ForgeServeOptions, serve_forge_api, spawn_forge_api};
use arks_repo::Repository;
use arks_server::{
    BasicAuthConfig, DaemonOptions, HttpRequest, ServeHttpOptions, ServeSshOptions,
    handle_smart_http_auth, receive_pack_session, serve_daemon, serve_http_with_opts,
    serve_ssh_listener, update_server_info, upload_pack_session,
};
use std::io::{self, Read, Write};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

pub fn cmd_upload_pack(args: &[String]) -> CliResult<()> {
    let dir = require_dir(args)?;
    let repo = Repository::open(&dir).or_else(|_| Repository::discover(&dir))?;
    // Duplex pack-protocol (SSH command-mode / local --upload-pack).
    upload_pack_session(&repo, io::stdin(), io::stdout()).map_err(|e| CliError::Msg(e.to_string()))
}

pub fn cmd_receive_pack(args: &[String]) -> CliResult<()> {
    let dir = require_dir(args)?;
    let mut repo = Repository::open(&dir).or_else(|_| Repository::discover(&dir))?;
    receive_pack_session(&mut repo, io::stdin(), io::stdout())
        .map_err(|e| CliError::Msg(e.to_string()))
}

/// Transitional `arks serve-http` — HTTP-only (`≡ arks serve --no-ssh --no-forge`).
/// Prefer `arks serve --no-ssh --no-forge` in new call sites ([[sot:root/arks-self-host-server]]).
pub fn cmd_serve_http(args: &[String]) -> CliResult<()> {
    if args.iter().any(|a| a == "--help" || a == "-h") {
        print_serve_http_transitional_help();
        return Ok(());
    }
    run_serve_http_listener(args, "arks serve-http (transitional ≡ serve --no-ssh --no-forge)")
}

/// `arks serve` — single hosting process; default HTTP+SSH+forge; `config` nested.
/// Cite: [[sot:root/arks-self-host-server]] (flags win; fail-closed SSH/forge Partial).
pub fn cmd_serve(args: &[String]) -> CliResult<()> {
    if args.is_empty()
        || matches!(
            args.first().map(String::as_str),
            Some("--help" | "-h" | "help")
        )
    {
        print_serve_help();
        return Ok(());
    }
    if args.first().map(String::as_str) == Some("config") {
        return cmd_serve_config(&args[1..]);
    }

    let mut enable_http = true;
    let mut enable_ssh = true;
    let mut enable_forge = true;
    let mut forge_listen = "127.0.0.1:3920".to_string();
    let mut forge_data: Option<PathBuf> = None;
    let mut forwarded = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        match a {
            "--no-http" => enable_http = false,
            "--no-ssh" => enable_ssh = false,
            "--no-forge" => enable_forge = false,
            "--http" => enable_http = true,
            "--ssh" => enable_ssh = true,
            "--forge" => enable_forge = true,
            "--listen" => {
                forwarded.push(args[i].clone());
                i += 1;
                let v = args
                    .get(i)
                    .ok_or_else(|| CliError::Usage("serve --listen needs ADDR".into()))?;
                forwarded.push(v.clone());
            }
            "--forge-listen" => {
                i += 1;
                forge_listen = args
                    .get(i)
                    .ok_or_else(|| CliError::Usage("serve --forge-listen needs ADDR".into()))?
                    .clone();
            }
            "--forge-data" => {
                i += 1;
                forge_data = Some(PathBuf::from(
                    args.get(i)
                        .ok_or_else(|| CliError::Usage("serve --forge-data needs DIR".into()))?,
                ));
            }
            "--basic-auth" | "--basic-auth-file" => {
                forwarded.push(args[i].clone());
                i += 1;
                let v = args
                    .get(i)
                    .ok_or_else(|| CliError::Usage(format!("serve {a} needs a value")))?;
                forwarded.push(v.clone());
            }
            "--dumb-only" => forwarded.push(args[i].clone()),
            other if other.starts_with("--listen=") => forwarded.push(args[i].clone()),
            other if other.starts_with("--forge-listen=") => {
                forge_listen = other[15..].to_string();
            }
            other if other.starts_with("--forge-data=") => {
                forge_data = Some(PathBuf::from(&other[13..]));
            }
            other if other.starts_with('-') => {
                return Err(CliError::NotImplemented(format!(
                    "serve option not claimed: {other}"
                )));
            }
            _ => forwarded.push(args[i].clone()),
        }
        i += 1;
    }

    if !enable_http && !enable_ssh && !enable_forge {
        return Err(CliError::Usage(
            "arks serve: nothing to serve (`--no-http` + `--no-ssh` + `--no-forge`); omit a disable flag"
                .into(),
        ));
    }

    // Fail-closed: SSH enabled but in-process listener Missing/Partial — do not silent-degrade.
    if enable_ssh {
        let ssh_listen: SocketAddr = "127.0.0.1:2222"
            .parse()
            .expect("default ssh listen parses");
        // Directory is optional here — Missing path refuses before bind.
        let dir = forwarded
            .iter()
            .find(|a| !a.starts_with('-'))
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        return serve_ssh_listener(
            ServeSshOptions {
                listen: ssh_listen,
                path: dir,
            },
            Arc::new(AtomicBool::new(false)),
        )
        .map_err(|e| CliError::NotImplemented(e.to_string()));
    }

    let forge_opts = if enable_forge {
        Some(resolve_forge_opts(&forge_listen, forge_data)?)
    } else {
        None
    };

    match (enable_http, forge_opts) {
        (true, Some(opts)) => {
            // Cohost: forge on background thread; HTTP blocks on main (Partial).
            let (forge_stop, forge_handle) = spawn_forge_api(opts).map_err(|e| {
                CliError::Msg(format!("arks serve: forge API bind failed: {e}"))
            })?;
            let label = "arks serve --no-ssh (HTTP + Partial forge)";
            let http_result = run_serve_http_listener(&forwarded, label);
            forge_stop.store(true, Ordering::SeqCst);
            let _ = forge_handle.join();
            http_result
        }
        (true, None) => run_serve_http_listener(&forwarded, "arks serve --no-ssh --no-forge"),
        (false, Some(opts)) => {
            eprintln!(
                "arks serve --no-http --no-ssh: Partial forge API only \
                 ([[sot:root/arks-self-host-server]]); design ≠ Claimed"
            );
            let stop = Arc::new(AtomicBool::new(false));
            serve_forge_api(opts, stop).map_err(|e| CliError::Msg(e))
        }
        (false, None) => Err(CliError::Usage(
            "arks serve: HTTP disabled, forge disabled, SSH not Claimed — nothing to run".into(),
        )),
    }
}

fn resolve_forge_opts(
    forge_listen: &str,
    forge_data: Option<PathBuf>,
) -> CliResult<ForgeServeOptions> {
    let listen: SocketAddr = forge_listen
        .parse()
        .map_err(|e| CliError::Usage(format!("bad --forge-listen address: {e}")))?;
    Ok(ForgeServeOptions {
        listen,
        data_dir: forge_data.unwrap_or_else(arks_hub::default_forge_data_dir),
    })
}

/// `arks serve config` — self-host server config (Partial; not Claimed; ≠ `arks config`).
pub fn cmd_serve_config(args: &[String]) -> CliResult<()> {
    if args.is_empty()
        || matches!(
            args.first().map(String::as_str),
            Some("--help" | "-h" | "help")
        )
    {
        print_serve_config_help();
        return Ok(());
    }
    let sub = args[0].as_str();
    match sub {
        "show" | "set" | "path" | "validate" => Err(CliError::NotImplemented(format!(
            "arks serve config {sub}: design/Partial — not Claimed yet \
             ([[sot:root/arks-self-host-server]]); server config ≠ `arks config` / hub settings"
        ))),
        other => Err(CliError::Usage(format!(
            "arks serve config: unknown `{other}` (show|set|path|validate); see --help"
        ))),
    }
}

fn run_serve_http_listener(args: &[String], label: &str) -> CliResult<()> {
    let mut listen = "127.0.0.1:8080".to_string();
    let mut dir: Option<PathBuf> = None;
    let mut dumb_only = false;
    let mut basic_auth_cli: Option<String> = None;
    let mut basic_auth_file: Option<PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--listen" => {
                i += 1;
                listen = args
                    .get(i)
                    .ok_or_else(|| CliError::Usage("serve --listen needs ADDR".into()))?
                    .clone();
            }
            _ if args[i].starts_with("--listen=") => {
                listen = args[i][9..].to_string();
            }
            "--dumb-only" => dumb_only = true,
            "--basic-auth" => {
                i += 1;
                basic_auth_cli = Some(
                    args.get(i)
                        .ok_or_else(|| {
                            CliError::Usage("serve --basic-auth needs user:pass".into())
                        })?
                        .clone(),
                );
            }
            "--basic-auth-file" => {
                i += 1;
                basic_auth_file = Some(PathBuf::from(
                    args.get(i)
                        .ok_or_else(|| {
                            CliError::Usage("serve --basic-auth-file needs PATH".into())
                        })?
                        .as_str(),
                ));
            }
            other if other.starts_with('-') => {
                return Err(CliError::NotImplemented(format!(
                    "serve option not claimed: {other}"
                )));
            }
            other => {
                if dir.is_some() {
                    return Err(CliError::Usage("serve takes one directory".into()));
                }
                dir = Some(PathBuf::from(other));
            }
        }
        i += 1;
    }
    let dir = dir.ok_or_else(|| CliError::Usage("serve <directory> required".into()))?;
    let git_dir = resolve_git_dir(&dir)?;
    // Refresh dumb advertisements so additive dumb GETs work beside smart HTTP.
    if let Ok(repo) = Repository::open(&git_dir) {
        let _ = update_server_info(&repo);
    }
    let addr: SocketAddr = listen
        .parse()
        .map_err(|e| CliError::Usage(format!("bad --listen address: {e}")))?;
    if !addr.ip().is_loopback() {
        eprintln!(
            "{label}: non-loopback bind (LAN/WAN) — product self-host Partial; ops owns firewall/TLS \
             ([[sot:root/arks-self-host-server]]); prefer --basic-auth + HTTPS front before exposure"
        );
    }
    let basic_auth = resolve_basic_auth(basic_auth_cli.as_deref(), basic_auth_file.as_deref())?;
    let auth_label = if basic_auth.is_some() {
        "basic-auth=on"
    } else {
        "unauthenticated"
    };
    eprintln!("{label} listening on http://{addr}/ (dumb_only={dumb_only}; {auth_label})");
    let stop = Arc::new(AtomicBool::new(false));
    serve_http_with_opts(
        git_dir,
        addr,
        stop,
        ServeHttpOptions {
            dumb_only,
            basic_auth,
        },
    )
    .map_err(|e| CliError::Msg(e.to_string()))
}

pub fn cmd_http_backend(args: &[String]) -> CliResult<()> {
    let dir = require_dir(args)?;
    let mut repo = Repository::open(&dir).or_else(|_| Repository::discover(&dir))?;
    let method = std::env::var("REQUEST_METHOD").unwrap_or_else(|_| "GET".into());
    let path = std::env::var("PATH_INFO").unwrap_or_else(|_| "/info/refs".into());
    let query = std::env::var("QUERY_STRING").unwrap_or_default();
    let git_protocol = std::env::var("HTTP_GIT_PROTOCOL")
        .ok()
        .or_else(|| std::env::var("GIT_PROTOCOL").ok());
    let authorization = std::env::var("HTTP_AUTHORIZATION")
        .ok()
        .or_else(|| std::env::var("AUTHORIZATION").ok());
    let destination = std::env::var("HTTP_DESTINATION").ok();
    let lock_token = std::env::var("HTTP_LOCK_TOKEN")
        .ok()
        .or_else(|| std::env::var("HTTP_IF").ok());
    let mut body = Vec::new();
    io::stdin().read_to_end(&mut body)?;
    let host = std::env::var("HTTP_HOST").ok();
    let req = HttpRequest {
        method,
        path,
        query,
        body,
        git_protocol,
        authorization,
        destination,
        lock_token,
        host,
    };
    let basic_auth = resolve_basic_auth(None, None)?;
    let resp = handle_smart_http_auth(&mut repo, &req, basic_auth.as_ref())
        .map_err(|e| CliError::Msg(e.to_string()))?;
    print!("Status: {}\r\n", resp.status);
    print!("Content-Type: {}\r\n", resp.content_type);
    if let Some(ref wa) = resp.www_authenticate {
        print!("WWW-Authenticate: {wa}\r\n");
    }
    for (k, v) in &resp.extra_headers {
        print!("{k}: {v}\r\n");
    }
    print!("Content-Length: {}\r\n\r\n", resp.body.len());
    io::stdout().write_all(&resp.body)?;
    Ok(())
}

/// Resolve Basic auth from CLI flags and/or env (`ARKS_HTTP_BASIC` / `ARKS_HTTP_BASIC_FILE`).
fn resolve_basic_auth(
    cli_user_pass: Option<&str>,
    cli_file: Option<&std::path::Path>,
) -> CliResult<Option<BasicAuthConfig>> {
    if let Some(spec) = cli_user_pass {
        return BasicAuthConfig::parse_user_pass(spec)
            .map(Some)
            .map_err(|e| CliError::Usage(e.to_string()));
    }
    if let Some(path) = cli_file {
        return BasicAuthConfig::from_file(path)
            .map(Some)
            .map_err(|e| CliError::Usage(e.to_string()));
    }
    if let Ok(spec) = std::env::var("ARKS_HTTP_BASIC") {
        if !spec.trim().is_empty() {
            return BasicAuthConfig::parse_user_pass(&spec)
                .map(Some)
                .map_err(|e| CliError::Usage(e.to_string()));
        }
    }
    if let Ok(path) = std::env::var("ARKS_HTTP_BASIC_FILE") {
        if !path.trim().is_empty() {
            return BasicAuthConfig::from_file(path.trim())
                .map(Some)
                .map_err(|e| CliError::Usage(e.to_string()));
        }
    }
    Ok(None)
}

pub fn cmd_update_server_info(args: &[String]) -> CliResult<()> {
    let dir = require_dir_optional_cwd(args)?;
    let repo = Repository::open(&dir).or_else(|_| Repository::discover(&dir))?;
    // FS5 / MAINT-004 closest: refuse under info lock rather than racing writers.
    let info_lock = repo.git_dir().join("info").join("refs.lock");
    if info_lock.is_file() {
        return Err(CliError::Msg(format!(
            "{} (MAINT-004 — update-server-info race refuse while info/refs.lock held; lease ≠ skip)",
            info_lock.display()
        )));
    }
    update_server_info(&repo).map_err(|e| CliError::Msg(e.to_string()))
}

pub fn cmd_daemon(args: &[String]) -> CliResult<()> {
    let mut listen = "127.0.0.1:9418".to_string();
    let mut export_all = false;
    let mut allow_receive = false;
    let mut base: Option<PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--listen" => {
                i += 1;
                listen = args
                    .get(i)
                    .ok_or_else(|| CliError::Usage("daemon --listen needs ADDR".into()))?
                    .clone();
            }
            "--export-all" => export_all = true,
            "--enable=receive-pack" => allow_receive = true,
            "--help" | "-h" => {
                print_daemon_help();
                return Ok(());
            }
            other if other.starts_with('-') => {
                return Err(CliError::NotImplemented(format!(
                    "daemon option not claimed: {other}"
                )));
            }
            other => {
                if base.is_some() {
                    return Err(CliError::Usage("daemon takes one base directory".into()));
                }
                base = Some(PathBuf::from(other));
            }
        }
        i += 1;
    }
    let base = base.ok_or_else(|| CliError::Usage("daemon <base-path> required".into()))?;
    let addr: SocketAddr = listen
        .parse()
        .map_err(|e| CliError::Usage(format!("bad --listen address: {e}")))?;
    eprintln!(
        "arks daemon listening on git://{addr}/ (export_all={export_all}; receive={allow_receive})"
    );
    let stop = Arc::new(AtomicBool::new(false));
    serve_daemon(
        addr,
        DaemonOptions {
            base_path: base,
            export_all,
            allow_receive,
            strict_paths: false,
        },
        stop,
    )
    .map_err(|e| CliError::Msg(e.to_string()))
}

fn require_dir(args: &[String]) -> CliResult<PathBuf> {
    let mut dir = None;
    for a in args {
        if a == "--help" || a == "-h" {
            return Err(CliError::Usage(
                "usage: arks upload-pack|receive-pack|http-backend|update-server-info <directory>"
                    .into(),
            ));
        }
        if a.starts_with('-') {
            return Err(CliError::NotImplemented(format!("option not claimed: {a}")));
        }
        if dir.is_some() {
            return Err(CliError::Usage("exactly one directory required".into()));
        }
        dir = Some(PathBuf::from(a));
    }
    dir.ok_or_else(|| CliError::Usage("directory required".into()))
}

fn require_dir_optional_cwd(args: &[String]) -> CliResult<PathBuf> {
    if args.is_empty() {
        return Ok(std::env::current_dir()?);
    }
    require_dir(args)
}

fn resolve_git_dir(dir: &std::path::Path) -> CliResult<PathBuf> {
    if let Ok(repo) = Repository::open(dir) {
        return Ok(repo.git_dir().to_path_buf());
    }
    let repo = Repository::discover(dir)?;
    Ok(repo.git_dir().to_path_buf())
}

fn print_serve_help() {
    print!(
        "\
arks serve — self-host umbrella (HTTP + SSH + forge API; Wave Sv / M11+M25)

Usage:
  arks serve [--no-http] [--no-ssh] [--no-forge] [--listen ADDR|--listen=ADDR]
             [--forge-listen ADDR] [--forge-data DIR]
             [--dumb-only] [--basic-auth user:pass] [--basic-auth-file PATH] <directory>
  arks serve config show|set|path|validate …

Default: HTTP + SSH + forge enabled. Opt out with --no-http / --no-ssh / --no-forge
(preferred over redundant enable flags). Disabling all three is an error.

Honesty ([[sot:root/arks-self-host-server]]):
  - HTTP smart(+dumb) listener: wire Claimed paths (G15/G29/G36 when configured)
  - In-process SSH listener: Missing/Partial — fail-closed via serve_ssh_listener
  - Forge-in-serve: Partial local API (separate bind; design ≠ Claimed product)
  - Practical today: `arks serve --no-ssh …` (HTTP + Partial forge) or `--no-forge` HTTP-only
  - Ops SSHD + upload-pack/receive-pack remains valid SSH remotes today
  - Prefer HTTPS via ops TLS front for non-localhost HTTP; in-process TLS not Claimed
  - Hub OAuth is refused as primary git-remote auth
  - `serve config` is server config (Partial) — not `arks config`, not hub

Ports ([[sot:root/arks-self-host-server]] / [[research:root/20261111055609]]):
  Normative targets (clone URLs / dedicated host): SSH 22, HTTP 80, HTTPS 443, git:// 9418
  Unprivileged loopback demo binds (current process defaults): HTTP 127.0.0.1:8080;
    SSH stub 127.0.0.1:2222 — privilege / host-sshd workarounds, NOT more-standard Git ports
  Forge API (arks-local convention): 127.0.0.1:3920 (--forge-listen / --forge-data)
LAN/WAN: ops owns firewall + TLS — not Claimed WAN-safe.
"
    );
}

fn print_serve_http_transitional_help() {
    print!(
        "\
arks serve-http — transitional HTTP-only alias (≡ `arks serve --no-ssh --no-forge`)

Prefer:  arks serve --no-ssh --no-forge [flags…] <directory>
See:     arks serve --help
"
    );
}

fn print_serve_config_help() {
    print!(
        "\
arks serve config — self-host server configuration (design / Partial; not Claimed)

Usage:
  arks serve config show
  arks serve config set <key> <value>
  arks serve config path
  arks serve config validate

Not `arks config` (git-repo config). Not hub forge settings.
Keys (design): bind, transports, Basic auth refs, TLS notes, SSH key paths, allow-paths.
"
    );
}

fn print_daemon_help() {
    print!(
        "\
arks daemon — git:// TCP daemon (git-daemon peer; Wave Sv2 / M25)

Usage:
  arks daemon [--listen HOST[:PORT]|--listen=…] [--export-all]
              [--enable=receive-pack] <base-path>

Default listen 127.0.0.1:9418 (IANA/Git-documented git:// port; normative).
Requires git-daemon-export-ok unless --export-all.
Claimed today: bind + export-all + optional receive-pack via arks-server daemon.
Not claimed yet (fail-closed if passed): --port --base-path --strict-paths
  --timeout --init-timeout --max-connections --verbose --reuseaddr
  --enable=upload-archive --disable=* --detach --inetd --pid-file
  --user/--group --syslog --access-hook multi-base positionals.
Never spawns git/gh. Design ≠ Claimed for full flag parity.
"
    );
}
