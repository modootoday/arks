//! Wave Sv / Sv2 server porcelain (upload-pack / receive-pack / HTTP / daemon).
//!
//! Cite: `.spec/sot/cli-surface.sot.md` Wave Sv, `.spec/sot/arks-server.sot.md`
//! No forge. No CELIR. Lease not required. No `gix`.

use crate::error::{CliError, CliResult};
use arks_repo::Repository;
use arks_server::{
    BasicAuthConfig, DaemonOptions, HttpRequest, ServeHttpOptions, handle_smart_http_auth,
    receive_pack_session, serve_daemon, serve_http_with_opts, update_server_info,
    upload_pack_session,
};
use std::io::{self, Read, Write};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

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

pub fn cmd_serve_http(args: &[String]) -> CliResult<()> {
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
                    .ok_or_else(|| CliError::Usage("serve-http --listen needs ADDR".into()))?
                    .clone();
            }
            "--dumb-only" => dumb_only = true,
            "--basic-auth" => {
                i += 1;
                basic_auth_cli = Some(
                    args.get(i)
                        .ok_or_else(|| CliError::Usage("serve-http --basic-auth needs user:pass".into()))?
                        .clone(),
                );
            }
            "--basic-auth-file" => {
                i += 1;
                basic_auth_file = Some(PathBuf::from(
                    args.get(i)
                        .ok_or_else(|| {
                            CliError::Usage("serve-http --basic-auth-file needs PATH".into())
                        })?
                        .as_str(),
                ));
            }
            "--help" | "-h" => {
                print_serve_help();
                return Ok(());
            }
            other if other.starts_with('-') => {
                return Err(CliError::NotImplemented(format!(
                    "serve-http option not claimed: {other}"
                )));
            }
            other => {
                if dir.is_some() {
                    return Err(CliError::Usage("serve-http takes one directory".into()));
                }
                dir = Some(PathBuf::from(other));
            }
        }
        i += 1;
    }
    let dir = dir.ok_or_else(|| CliError::Usage("serve-http <directory> required".into()))?;
    let git_dir = resolve_git_dir(&dir)?;
    // Refresh dumb advertisements so additive dumb GETs work beside smart HTTP.
    if let Ok(repo) = Repository::open(&git_dir) {
        let _ = update_server_info(&repo);
    }
    let addr: SocketAddr = listen
        .parse()
        .map_err(|e| CliError::Usage(format!("bad --listen address: {e}")))?;
    let basic_auth = resolve_basic_auth(basic_auth_cli.as_deref(), basic_auth_file.as_deref())?;
    let auth_label = if basic_auth.is_some() {
        "basic-auth=on"
    } else {
        "unauthenticated"
    };
    eprintln!("arks serve-http listening on http://{addr}/ (dumb_only={dumb_only}; {auth_label})");
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
    let mut body = Vec::new();
    io::stdin().read_to_end(&mut body)?;
    let req = HttpRequest {
        method,
        path,
        query,
        body,
        git_protocol,
        authorization,
    };
    let basic_auth = resolve_basic_auth(None, None)?;
    let resp = handle_smart_http_auth(&mut repo, &req, basic_auth.as_ref())
        .map_err(|e| CliError::Msg(e.to_string()))?;
    print!("Status: {}\r\n", resp.status);
    print!("Content-Type: {}\r\n", resp.content_type);
    if let Some(ref wa) = resp.www_authenticate {
        print!("WWW-Authenticate: {wa}\r\n");
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
arks serve-http — smart (+ dumb) HTTP Git server (Wave Sv / M11+M25)

Usage:
  arks serve-http [--listen ADDR] [--dumb-only] [--basic-auth user:pass]
                  [--basic-auth-file PATH] <directory>

Unauthenticated loopback is the default (G15 clean-room). Optional in-process
HTTP Basic via `--basic-auth` / `--basic-auth-file` or env `ARKS_HTTP_BASIC` /
`ARKS_HTTP_BASIC_FILE` (G36). Forge OAuth is refused. Ops reverse-proxy / TLS
MAY still front the listener. Dumb GETs are additive beside smart HTTP.
`--dumb-only` forces dumb info/refs replies (G29 / smart-client fallback).
"
    );
}

fn print_daemon_help() {
    print!(
        "\
arks daemon — git:// TCP daemon (Wave Sv2 / M25)

Usage:
  arks daemon [--listen ADDR] [--export-all] [--enable=receive-pack] <base-path>

Requires git-daemon-export-ok in each repo unless --export-all.
"
    );
}
