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
    BasicAuthConfig, DaemonOptions, HttpRequest, ListenRebindSlot, ProductTlsServerConfig,
    ServeHttpOptions, ServeSshOptions, default_authorized_keys_path, default_host_key_path,
    handle_smart_http_auth, load_server_config, receive_pack_session, serve_daemon,
    serve_daemon_inetd, serve_http_scheme, serve_http_with_opts, spawn_ssh_listener,
    spawn_loopback_ssh, update_server_info, upload_pack_session,
};
use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

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
    run_serve_http_listener(
        args,
        "arks serve-http (transitional ≡ serve --no-ssh --no-forge)",
    )
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
    if args.first().map(String::as_str) == Some("ssh-smoke") {
        return cmd_serve_ssh_smoke(&args[1..]);
    }

    // Partial: file store seeds defaults; clap flags still win when present.
    let file_cfg = load_serve_config_map().unwrap_or_default();
    let mut enable_http = config_bool(&file_cfg, "http", true);
    let mut enable_ssh = config_bool(&file_cfg, "ssh", true);
    let mut enable_forge = config_bool(&file_cfg, "forge", true);
    let mut forge_listen = file_cfg
        .get("forge_listen")
        .cloned()
        .unwrap_or_else(|| "127.0.0.1:3920".into());
    let mut forge_data: Option<PathBuf> = file_cfg.get("forge_data").map(PathBuf::from);
    let mut listen_from_cfg = file_cfg.get("bind").cloned();
    let mut ssh_listen_cfg = file_cfg
        .get("ssh_listen")
        .cloned()
        .unwrap_or_else(|| "127.0.0.1:2222".into());
    let mut ssh_host_key = file_cfg
        .get("ssh_host_key")
        .map(PathBuf::from)
        .unwrap_or_else(default_host_key_path);
    let mut ssh_authorized_keys = file_cfg
        .get("ssh_authorized_keys")
        .map(PathBuf::from)
        .unwrap_or_else(default_authorized_keys_path);
    let mut forwarded = Vec::new();
    if let Some(ref bind) = listen_from_cfg {
        // Seed HTTP listen from file; explicit --listen later overrides via forwarded.
        forwarded.push("--listen".into());
        forwarded.push(bind.clone());
    }
    if let Some(auth_file) = file_cfg.get("basic_auth_file") {
        forwarded.push("--basic-auth-file".into());
        forwarded.push(auth_file.clone());
    }
    // Seed TLS paths from serve.conf when present; clap still wins.
    if let Some(cert) = file_cfg.get("tls_cert") {
        forwarded.push("--tls-cert".into());
        forwarded.push(cert.clone());
    }
    if let Some(key) = file_cfg.get("tls_key") {
        forwarded.push("--tls-key".into());
        forwarded.push(key.clone());
    }
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
            "--tls-cert" | "--tls-key" => {
                // Drop any seeded pair so CLI wins.
                if let Some(pos) = forwarded.iter().position(|x| x == a) {
                    forwarded.remove(pos);
                    if pos < forwarded.len() {
                        forwarded.remove(pos);
                    }
                }
                forwarded.push(args[i].clone());
                i += 1;
                let v = args
                    .get(i)
                    .ok_or_else(|| CliError::Usage(format!("serve {a} needs PATH")))?;
                forwarded.push(v.clone());
            }
            "--listen" => {
                // Replace any seeded --listen from file store.
                forwarded.retain(|x| {
                    x != "--listen" && !listen_from_cfg.as_ref().is_some_and(|b| b == x)
                });
                listen_from_cfg = None;
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
                forge_data =
                    Some(PathBuf::from(args.get(i).ok_or_else(|| {
                        CliError::Usage("serve --forge-data needs DIR".into())
                    })?));
            }
            "--ssh-host-key" => {
                i += 1;
                ssh_host_key =
                    PathBuf::from(args.get(i).ok_or_else(|| {
                        CliError::Usage("serve --ssh-host-key needs PATH".into())
                    })?);
            }
            "--authorized-keys" => {
                i += 1;
                ssh_authorized_keys =
                    PathBuf::from(args.get(i).ok_or_else(|| {
                        CliError::Usage("serve --authorized-keys needs PATH".into())
                    })?);
            }
            "--ssh-listen" => {
                i += 1;
                ssh_listen_cfg = args
                    .get(i)
                    .ok_or_else(|| CliError::Usage("serve --ssh-listen needs ADDR".into()))?
                    .clone();
            }
            "--basic-auth" | "--basic-auth-file" => {
                if a == "--basic-auth-file" {
                    // Drop seeded basic_auth_file pair so CLI wins.
                    if let Some(pos) = forwarded.iter().position(|x| x == "--basic-auth-file") {
                        forwarded.remove(pos);
                        if pos < forwarded.len() {
                            forwarded.remove(pos);
                        }
                    }
                }
                forwarded.push(args[i].clone());
                i += 1;
                let v = args
                    .get(i)
                    .ok_or_else(|| CliError::Usage(format!("serve {a} needs a value")))?;
                forwarded.push(v.clone());
            }
            "--dumb-only" => forwarded.push(args[i].clone()),
            other if other.starts_with("--listen=") => {
                forwarded.retain(|x| {
                    x != "--listen" && !listen_from_cfg.as_ref().is_some_and(|b| b == x)
                });
                listen_from_cfg = None;
                forwarded.push(args[i].clone());
            }
            other if other.starts_with("--forge-listen=") => {
                forge_listen = other[15..].to_string();
            }
            other if other.starts_with("--forge-data=") => {
                forge_data = Some(PathBuf::from(&other[13..]));
            }
            other if other.starts_with("--ssh-host-key=") => {
                ssh_host_key = PathBuf::from(&other[15..]);
            }
            other if other.starts_with("--authorized-keys=") => {
                ssh_authorized_keys = PathBuf::from(&other[18..]);
            }
            other if other.starts_with("--ssh-listen=") => {
                ssh_listen_cfg = other[13..].to_string();
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

    // Partial live bind sync: record intended listen targets before fail-closed / listen.
    let http_bind_for_sync = forwarded
        .windows(2)
        .find(|w| w[0] == "--listen")
        .map(|w| w[1].clone())
        .or_else(|| {
            forwarded
                .iter()
                .find_map(|a| a.strip_prefix("--listen=").map(str::to_string))
        })
        .or(listen_from_cfg.clone())
        .unwrap_or_else(|| "127.0.0.1:8080".into());

    // Directory for SSH (and optional HTTP git_dir): prefer last non-flag positional.
    let dir = forwarded
        .iter()
        .find(|a| !a.starts_with('-') && *a != "--listen" && !a.contains(':'))
        .cloned()
        .or_else(|| {
            forwarded
                .iter()
                .rev()
                .find(|a| !a.starts_with('-'))
                .cloned()
        })
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let dir = if dir.to_string_lossy().parse::<SocketAddr>().is_ok() {
        PathBuf::from(".")
    } else {
        dir
    };

    let product_tls = resolve_product_tls_from_args(&forwarded)?;
    let mut forge_opts = if enable_forge {
        Some(resolve_forge_opts(&forge_listen, forge_data)?)
    } else {
        None
    };
    if let Some(ref mut opts) = forge_opts {
        opts.tls = product_tls.clone();
    }

    sync_serve_runtime_to_file(
        enable_http.then_some(http_bind_for_sync.as_str()),
        enable_forge.then_some(forge_listen.as_str()),
        enable_ssh.then_some(ssh_listen_cfg.as_str()),
    );

    // IW-FORGE-SERVE Partial: default cohost must not silent-cut forge/HTTP when SSH is on.
    // Background modules + main blocker; bind failure fail-closed (SoT §17).
    let mut ssh_join: Option<(
        Arc<AtomicBool>,
        std::thread::JoinHandle<Result<(), arks_server::ServerError>>,
    )> = None;
    if enable_ssh {
        let ssh_listen: SocketAddr = ssh_listen_cfg
            .parse()
            .map_err(|e| CliError::Usage(format!("serve ssh_listen bad ADDR: {e}")))?;
        install_serve_config_sighup_partial("arks serve");
        let ssh_rebind = register_live_ssh_rebind_slot();
        let spawned = spawn_ssh_listener(ServeSshOptions {
            listen: ssh_listen,
            path: dir.clone(),
            host_key: ssh_host_key,
            authorized_keys: ssh_authorized_keys,
            rebind_desired: Some(ssh_rebind),
        })
        .map_err(|e| CliError::Msg(format!("arks serve: SSH bind failed: {e}")))?;
        ssh_join = Some(spawned);
    }

    let mut forge_join: Option<(
        Arc<AtomicBool>,
        std::thread::JoinHandle<Result<(), String>>,
    )> = None;
    if let Some(opts) = forge_opts.as_mut() {
        if !enable_http && !enable_ssh {
            // Forge-only: block on main.
            let forge_slot = register_live_forge_rebind_slot();
            opts.rebind_desired = Some(forge_slot);
            install_serve_config_sighup_partial("arks serve forge-only");
            let stop = Arc::new(AtomicBool::new(false));
            let result = serve_forge_api(opts.clone(), stop).map_err(|e| CliError::Msg(e));
            clear_live_forge_rebind_slot();
            return result;
        }
        let forge_slot = register_live_forge_rebind_slot();
        opts.rebind_desired = Some(forge_slot);
        let spawned = spawn_forge_api(opts.clone())
            .map_err(|e| CliError::Msg(format!("arks serve: forge API bind failed: {e}")))?;
        forge_join = Some(spawned);
    }

    let result = if enable_http {
        let label = match (enable_ssh, forge_join.is_some()) {
            (true, true) => "arks serve (HTTP + SSH + forge cohost; forge-in-serve Partial depth)",
            (true, false) => "arks serve (HTTP + SSH)",
            (false, true) => "arks serve --no-ssh (HTTP + forge)",
            (false, false) => "arks serve --no-ssh --no-forge",
        };
        if !enable_ssh {
            install_serve_config_sighup_partial(label);
        }
        run_serve_http_listener(&forwarded, label)
    } else if enable_ssh {
        // SSH-only (or SSH+forge with forge already backgrounded): block on SSH thread
        // until process stop (same lifetime as prior main-thread serve_ssh_listener).
        eprintln!(
            "arks serve: SSH primary listener (forge cohost={}); forge-in-serve Partial depth",
            forge_join.is_some()
        );
        if let Some((_stop, handle)) = ssh_join.take() {
            match handle.join() {
                Ok(Ok(())) => {}
                Ok(Err(e)) => return Err(CliError::Msg(e.to_string())),
                Err(_) => return Err(CliError::Msg("arks serve: SSH thread panicked".into())),
            }
            clear_live_ssh_rebind_slot();
            if let Some((fstop, fhandle)) = forge_join.take() {
                fstop.store(true, Ordering::SeqCst);
                let _ = fhandle.join();
                clear_live_forge_rebind_slot();
            }
            Ok(())
        } else {
            Err(CliError::Msg("arks serve: SSH join missing".into()))
        }
    } else {
        Err(CliError::Usage(
            "arks serve: HTTP disabled, forge disabled, SSH disabled — nothing to run".into(),
        ))
    };

    if let Some((stop, handle)) = ssh_join.take() {
        stop.store(true, Ordering::SeqCst);
        let _ = handle.join();
        clear_live_ssh_rebind_slot();
    }
    if let Some((stop, handle)) = forge_join.take() {
        stop.store(true, Ordering::SeqCst);
        let _ = handle.join();
        clear_live_forge_rebind_slot();
    }
    result
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
        rebind_desired: None,
        tls: None,
    })
}

/// Parse `--tls-cert` / `--tls-key` from serve forwarded args (shared by HTTP + forge cohost).
fn resolve_product_tls_from_args(args: &[String]) -> CliResult<Option<ProductTlsServerConfig>> {
    let mut tls_cert: Option<PathBuf> = None;
    let mut tls_key: Option<PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--tls-cert" => {
                i += 1;
                tls_cert =
                    Some(PathBuf::from(args.get(i).ok_or_else(|| {
                        CliError::Usage("serve --tls-cert needs PATH".into())
                    })?));
            }
            ref s if s.starts_with("--tls-cert=") => tls_cert = Some(PathBuf::from(&s[11..])),
            "--tls-key" => {
                i += 1;
                tls_key =
                    Some(PathBuf::from(args.get(i).ok_or_else(|| {
                        CliError::Usage("serve --tls-key needs PATH".into())
                    })?));
            }
            ref s if s.starts_with("--tls-key=") => tls_key = Some(PathBuf::from(&s[10..])),
            _ => {}
        }
        i += 1;
    }
    match (tls_cert.as_ref(), tls_key.as_ref()) {
        (Some(cert), Some(key)) => Ok(Some(load_server_config(cert, key).map_err(CliError::Msg)?)),
        (None, None) => Ok(None),
        (Some(_), None) => Err(CliError::Usage(
            "serve --tls-cert requires --tls-key (operator-supplied PEM pair)".into(),
        )),
        (None, Some(_)) => Err(CliError::Usage(
            "serve --tls-key requires --tls-cert (operator-supplied PEM pair)".into(),
        )),
    }
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
        "path" => {
            println!("{}", serve_config_path().display());
            Ok(())
        }
        "show" => {
            let path = serve_config_path();
            if !path.is_file() {
                eprintln!(
                    "arks serve config show: no file yet at {} (Partial defaults apply at clap)",
                    path.display()
                );
                print_serve_config_defaults();
                return Ok(());
            }
            let raw = std::fs::read_to_string(&path).map_err(|e| CliError::Msg(e.to_string()))?;
            print!("{raw}");
            if !raw.ends_with('\n') {
                println!();
            }
            eprintln!(
                "note: Partial file store — design ≠ Claimed ([[sot:root/arks-self-host-server]])"
            );
            Ok(())
        }
        "set" => {
            let key = args
                .get(1)
                .ok_or_else(|| CliError::Usage("serve config set <key> <value>".into()))?;
            let value = args
                .get(2)
                .ok_or_else(|| CliError::Usage("serve config set <key> <value>".into()))?;
            if !SERVE_CONFIG_KEYS.contains(&key.as_str()) {
                return Err(CliError::Usage(format!(
                    "serve config set: unknown key `{key}` (allowed: {})",
                    SERVE_CONFIG_KEYS.join(", ")
                )));
            }
            let mut map = load_serve_config_map()?;
            map.insert(key.clone(), value.clone());
            save_serve_config_map(&map)?;
            eprintln!(
                "arks serve config set: {key}={value} → {} (Partial; not Claimed)",
                serve_config_path().display()
            );
            Ok(())
        }
        "validate" => {
            let path = serve_config_path();
            if !path.is_file() {
                eprintln!(
                    "arks serve config validate: no file at {} — empty Partial OK",
                    path.display()
                );
                return Ok(());
            }
            let map = load_serve_config_map()?;
            let mut ok = true;
            for (k, v) in &map {
                if !SERVE_CONFIG_KEYS.contains(&k.as_str()) {
                    eprintln!("unknown key: {k}");
                    ok = false;
                    continue;
                }
                if let Err(e) = validate_serve_config_value(k, v) {
                    eprintln!("{k}: {e}");
                    ok = false;
                }
            }
            // Stronger live bind probe (Partial): briefly bind configured listen addrs.
            let mut probe_notes = std::collections::BTreeMap::new();
            let ssh_on = config_bool(&map, "ssh", true);
            let mut probe_keys: Vec<(&str, String)> = Vec::new();
            if let Some(addr) = map.get("bind") {
                probe_keys.push(("bind", addr.clone()));
            }
            if let Some(addr) = map.get("forge_listen") {
                probe_keys.push(("forge_listen", addr.clone()));
            }
            let ssh_addr = map
                .get("ssh_listen")
                .cloned()
                .unwrap_or_else(|| "127.0.0.1:2222".into());
            if ssh_on {
                probe_keys.push(("ssh_listen", ssh_addr));
            }
            for (key, addr) in probe_keys {
                match probe_bind_addr(&addr) {
                    Ok(local) => {
                        eprintln!("{key}: live_bind_probe=ok local={local}");
                        probe_notes.insert(format!("runtime_probe_{key}"), format!("ok:{local}"));
                    }
                    Err(e) => {
                        eprintln!("{key}: live_bind_probe=fail ({e})");
                        probe_notes.insert(format!("runtime_probe_{key}"), format!("fail:{e}"));
                        ok = false;
                    }
                }
            }
            // Persist probe results for e2e-ready Partial honesty (clap still wins at serve).
            if !probe_notes.is_empty() {
                let mut map2 = map.clone();
                for (k, v) in probe_notes {
                    map2.insert(k, v);
                }
                let _ = record_runtime_bind_sync(
                    &mut map2,
                    map.get("bind").map(String::as_str),
                    map.get("forge_listen").map(String::as_str),
                    map.get("ssh_listen")
                        .map(String::as_str)
                        .or(Some("127.0.0.1:2222")),
                );
                let _ = save_serve_config_map(&map2);
            }
            if ok {
                eprintln!(
                    "arks serve config validate: ok (Partial schema + live bind probe; \
                     runtime_probe_* written — not Claimed sync e2e)"
                );
                Ok(())
            } else {
                Err(CliError::Msg(
                    "arks serve config validate: failed (Partial schema / bind probe)".into(),
                ))
            }
        }
        "sync" => serve_config_apply_reload("sync", /*seed_defaults=*/ true),
        // `apply` / `reload`: re-read file store, live-probe binds, write runtime_* —
        // Partial toward sync e2e. On unix, running `arks serve` also honors SIGHUP Partial.
        "apply" | "reload" => serve_config_apply_reload(sub, /*seed_defaults=*/ false),
        // File-watch Partial: poll mtime and re-apply; does not signal a running serve.
        "watch" => serve_config_watch(&args[1..]),
        other => Err(CliError::Usage(format!(
            "arks serve config: unknown `{other}` (show|set|path|validate|sync|apply|reload|watch); see --help"
        ))),
    }
}

/// Partial hot-reload toward sync e2e: poll serve-config file mtime and re-apply probes.
/// Does **not** signal a running `arks serve` (unix SIGHUP is armed inside `arks serve` itself).
fn serve_config_watch(args: &[String]) -> CliResult<()> {
    let mut duration_secs: u64 = 15;
    let mut interval_ms: u64 = 500;
    let mut i = 0usize;
    while i < args.len() {
        let a = args[i].as_str();
        match a {
            "--duration" => {
                i += 1;
                duration_secs = args
                    .get(i)
                    .ok_or_else(|| CliError::Usage("watch --duration needs SECONDS".into()))?
                    .parse()
                    .map_err(|_| {
                        CliError::Usage("watch --duration needs integer seconds".into())
                    })?;
            }
            other if let Some(v) = other.strip_prefix("--duration=") => {
                duration_secs = v.parse().map_err(|_| {
                    CliError::Usage("watch --duration needs integer seconds".into())
                })?;
            }
            "--interval-ms" => {
                i += 1;
                interval_ms = args
                    .get(i)
                    .ok_or_else(|| CliError::Usage("watch --interval-ms needs MS".into()))?
                    .parse()
                    .map_err(|_| CliError::Usage("watch --interval-ms needs integer".into()))?;
            }
            other if let Some(v) = other.strip_prefix("--interval-ms=") => {
                interval_ms = v
                    .parse()
                    .map_err(|_| CliError::Usage("watch --interval-ms needs integer".into()))?;
            }
            "-h" | "--help" | "help" => {
                eprintln!(
                    "arks serve config watch [--duration SECS] [--interval-ms MS]\n  \
                     Partial file-watch: poll config mtime and apply/reload probes.\n  \
                     Does not signal a running serve (unix: send SIGHUP to `arks serve` for Partial)."
                );
                return Ok(());
            }
            other => {
                return Err(CliError::Usage(format!(
                    "arks serve config watch: unknown `{other}`"
                )));
            }
        }
        i += 1;
    }
    let path = serve_config_path();
    let start = std::time::Instant::now();
    let mut last_mtime = std::fs::metadata(&path)
        .ok()
        .and_then(|m| m.modified().ok());
    eprintln!(
        "arks serve config watch: Partial file-watch path={} duration={duration_secs}s \
         interval={interval_ms}ms (no running-server signal; Claimed sync e2e Missing)",
        path.display()
    );
    // Initial apply so watch always leaves runtime_* stamped.
    serve_config_apply_reload("watch-initial", /*seed_defaults=*/ false)?;
    while start.elapsed() < std::time::Duration::from_secs(duration_secs.max(1)) {
        std::thread::sleep(std::time::Duration::from_millis(interval_ms.max(50)));
        let mtime = std::fs::metadata(&path)
            .ok()
            .and_then(|m| m.modified().ok());
        if mtime != last_mtime {
            eprintln!("arks serve config watch: mtime change detected → apply/reload Partial");
            last_mtime = mtime;
            let mut map = load_serve_config_map()?;
            map.insert("runtime_watch_mtime".into(), format!("{:?}", mtime));
            map.insert(
                "runtime_watch_signal".into(),
                "file-mtime Partial (unix SIGHUP Partial on running serve; Windows Missing)".into(),
            );
            save_serve_config_map(&map)?;
            serve_config_apply_reload("watch-reload", /*seed_defaults=*/ false)?;
        }
    }
    eprintln!(
        "arks serve config watch: done (Partial; toward sync e2e — not Claimed process hot-reload)"
    );
    Ok(())
}

/// Shared Partial bind apply/reload/sync: probe listens, persist runtime_* keys,
/// and request same-process live HTTP/forge rebind when listeners are registered.
/// Cross-process apply/reload cannot rebind another serve (honest no_live_listener).
fn serve_config_apply_reload(label: &str, seed_defaults: bool) -> CliResult<()> {
    let mut map = load_serve_config_map()?;
    if seed_defaults {
        if !map.contains_key("bind") {
            map.insert("bind".into(), "127.0.0.1:8080".into());
        }
        if !map.contains_key("forge_listen") {
            map.insert("forge_listen".into(), "127.0.0.1:3920".into());
        }
        if !map.contains_key("ssh_listen") {
            map.insert("ssh_listen".into(), "127.0.0.1:2222".into());
        }
        if !map.contains_key("http") {
            map.insert("http".into(), "on".into());
        }
        if !map.contains_key("ssh") {
            map.insert("ssh".into(), "on".into());
        }
        if !map.contains_key("forge") {
            map.insert("forge".into(), "on".into());
        }
    }
    let bind = map
        .get("bind")
        .cloned()
        .unwrap_or_else(|| "127.0.0.1:8080".into());
    let forge = map
        .get("forge_listen")
        .cloned()
        .unwrap_or_else(|| "127.0.0.1:3920".into());
    let ssh = map
        .get("ssh_listen")
        .cloned()
        .unwrap_or_else(|| "127.0.0.1:2222".into());
    let ssh_on = config_bool(&map, "ssh", true);
    let mut probes: Vec<(&str, String)> =
        vec![("bind", bind.clone()), ("forge_listen", forge.clone())];
    if ssh_on {
        probes.push(("ssh_listen", ssh.clone()));
    }
    let mut all_ok = true;
    for (key, addr) in &probes {
        match probe_bind_addr(addr) {
            Ok(local) => {
                map.insert(format!("runtime_probe_{key}"), format!("ok:{local}"));
                eprintln!("{label} probe {key}=ok local={local}");
            }
            Err(e) => {
                map.insert(format!("runtime_probe_{key}"), format!("fail:{e}"));
                eprintln!("{label} probe {key}=fail ({e})");
                all_ok = false;
            }
        }
    }
    record_runtime_bind_sync(
        &mut map,
        Some(&bind),
        Some(&forge),
        if ssh_on { Some(ssh.as_str()) } else { None },
    )?;
    // Stamp apply/reload intent (Partial; not Claimed running-server sync).
    map.insert(
        "runtime_apply_op".into(),
        format!("{label}:{}", if all_ok { "ok" } else { "partial_fail" }),
    );
    // Same-process live rebind Partial when durable HTTP/forge listeners registered
    // (SIGHUP path). Out-of-process apply/reload stamps honesty when no listener.
    request_live_rebind_partial(&mut map);
    save_serve_config_map(&map)?;
    eprintln!(
        "arks serve config {label}: wrote runtime_* + runtime_probe_* + runtime_rebind → {} \
         (Partial bind apply/reload + live rebind when same-process listeners armed; \
         clap still wins; Claimed zero-downtime hot-rebind Missing; not Claimed sync e2e)",
        serve_config_path().display()
    );
    if all_ok {
        Ok(())
    } else {
        Err(CliError::Msg(format!(
            "arks serve config {label}: one or more live bind probes failed (Partial; not Claimed)"
        )))
    }
}

const SERVE_CONFIG_KEYS: &[&str] = &[
    "bind",
    "http",
    "ssh",
    "forge",
    "forge_listen",
    "ssh_listen",
    "forge_data",
    "basic_auth_file",
    "ssh_host_key",
    "ssh_authorized_keys",
    "allow_paths",
    "tls_note",
    "tls_cert",
    "tls_key",
    // Runtime sync keys (Partial live bind honesty; written by `serve` / `config sync`).
    "runtime_bind",
    "runtime_forge_listen",
    "runtime_ssh_listen",
    "runtime_synced_at",
    "runtime_probe_bind",
    "runtime_probe_forge_listen",
    "runtime_probe_ssh_listen",
    "runtime_apply_op",
    "runtime_watch_mtime",
    "runtime_watch_signal",
    "runtime_rebind",
];

fn config_bool(map: &std::collections::BTreeMap<String, String>, key: &str, default: bool) -> bool {
    match map.get(key).map(String::as_str) {
        Some("on" | "true" | "1") => true,
        Some("off" | "false" | "0") => false,
        _ => default,
    }
}

fn serve_config_path() -> PathBuf {
    if let Ok(p) = std::env::var("ARKS_SERVE_CONFIG") {
        return PathBuf::from(p);
    }
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    home.join(".config").join("arks").join("serve.conf")
}

fn load_serve_config_map() -> CliResult<std::collections::BTreeMap<String, String>> {
    let path = serve_config_path();
    let mut map = std::collections::BTreeMap::new();
    if !path.is_file() {
        return Ok(map);
    }
    let raw = std::fs::read_to_string(&path).map_err(|e| CliError::Msg(e.to_string()))?;
    for (lineno, line) in raw.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            return Err(CliError::Msg(format!(
                "serve config: bad line {}: expected key=value",
                lineno + 1
            )));
        };
        map.insert(k.trim().to_string(), v.trim().to_string());
    }
    Ok(map)
}

fn save_serve_config_map(map: &std::collections::BTreeMap<String, String>) -> CliResult<()> {
    let path = serve_config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| CliError::Msg(e.to_string()))?;
    }
    let mut out = String::from("# arks serve config (Partial; ≠ arks config / hub)\n");
    for (k, v) in map {
        out.push_str(k);
        out.push('=');
        out.push_str(v);
        out.push('\n');
    }
    std::fs::write(&path, out).map_err(|e| CliError::Msg(e.to_string()))
}

fn validate_serve_config_value(key: &str, value: &str) -> Result<(), String> {
    match key {
        "http" | "ssh" | "forge" => match value {
            "on" | "off" | "true" | "false" | "1" | "0" => Ok(()),
            _ => Err("expected on|off".into()),
        },
        "bind"
        | "forge_listen"
        | "ssh_listen"
        | "runtime_bind"
        | "runtime_forge_listen"
        | "runtime_ssh_listen" => {
            if value.parse::<SocketAddr>().is_ok() {
                Ok(())
            } else {
                Err("expected HOST:PORT socket address".into())
            }
        }
        "forge_data"
        | "basic_auth_file"
        | "ssh_host_key"
        | "ssh_authorized_keys"
        | "allow_paths"
        | "tls_note"
        | "tls_cert"
        | "tls_key"
        | "runtime_synced_at"
        | "runtime_probe_bind"
        | "runtime_probe_forge_listen"
        | "runtime_probe_ssh_listen"
        | "runtime_apply_op"
        | "runtime_watch_mtime"
        | "runtime_watch_signal" => {
            if value.is_empty() {
                Err("value must not be empty".into())
            } else {
                Ok(())
            }
        }
        _ => Err("unknown key".into()),
    }
}

fn probe_bind_addr(addr: &str) -> Result<String, String> {
    let sock: SocketAddr = addr.parse().map_err(|e| format!("bad address: {e}"))?;
    let listener = std::net::TcpListener::bind(sock).map_err(|e| e.to_string())?;
    let local = listener
        .local_addr()
        .map(|a| a.to_string())
        .unwrap_or_else(|_| addr.to_string());
    drop(listener);
    Ok(local)
}

fn record_runtime_bind_sync(
    map: &mut std::collections::BTreeMap<String, String>,
    http_bind: Option<&str>,
    forge_listen: Option<&str>,
    ssh_listen: Option<&str>,
) -> CliResult<()> {
    if let Some(b) = http_bind {
        map.insert("runtime_bind".into(), b.to_string());
    }
    if let Some(f) = forge_listen {
        map.insert("runtime_forge_listen".into(), f.to_string());
    }
    if let Some(s) = ssh_listen {
        map.insert("runtime_ssh_listen".into(), s.to_string());
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    map.insert("runtime_synced_at".into(), now.to_string());
    Ok(())
}

fn sync_serve_runtime_to_file(
    http_bind: Option<&str>,
    forge_listen: Option<&str>,
    ssh_listen: Option<&str>,
) {
    let mut map = match load_serve_config_map() {
        Ok(m) => m,
        Err(_) => return,
    };
    if record_runtime_bind_sync(&mut map, http_bind, forge_listen, ssh_listen).is_err() {
        return;
    }
    if save_serve_config_map(&map).is_ok() {
        eprintln!(
            "arks serve: synced runtime_* bind keys → {} (Partial live bind sync; clap still wins)",
            serve_config_path().display()
        );
    }
}

fn print_serve_config_defaults() {
    println!("# Partial clap / product defaults (not written until `set`):");
    println!("bind=127.0.0.1:8080");
    println!("http=on");
    println!("ssh=on");
    println!("forge=on");
    println!("forge_listen=127.0.0.1:3920");
    println!("ssh_listen=127.0.0.1:2222");
}

fn run_serve_http_listener(args: &[String], label: &str) -> CliResult<()> {
    let mut listen = "127.0.0.1:8080".to_string();
    let mut dir: Option<PathBuf> = None;
    let mut dumb_only = false;
    let mut basic_auth_cli: Option<String> = None;
    let mut basic_auth_file: Option<PathBuf> = None;
    let mut tls_cert: Option<PathBuf> = None;
    let mut tls_key: Option<PathBuf> = None;
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
            "--tls-cert" => {
                i += 1;
                tls_cert =
                    Some(PathBuf::from(args.get(i).ok_or_else(|| {
                        CliError::Usage("serve --tls-cert needs PATH".into())
                    })?));
            }
            _ if args[i].starts_with("--tls-cert=") => {
                tls_cert = Some(PathBuf::from(&args[i][11..]));
            }
            "--tls-key" => {
                i += 1;
                tls_key =
                    Some(PathBuf::from(args.get(i).ok_or_else(|| {
                        CliError::Usage("serve --tls-key needs PATH".into())
                    })?));
            }
            _ if args[i].starts_with("--tls-key=") => {
                tls_key = Some(PathBuf::from(&args[i][10..]));
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
    let tls = match (tls_cert.as_ref(), tls_key.as_ref()) {
        (Some(cert), Some(key)) => Some(load_server_config(cert, key).map_err(CliError::Msg)?),
        (None, None) => None,
        (Some(_), None) => {
            return Err(CliError::Usage(
                "serve --tls-cert requires --tls-key (operator-supplied PEM pair)".into(),
            ));
        }
        (None, Some(_)) => {
            return Err(CliError::Usage(
                "serve --tls-key requires --tls-cert (operator-supplied PEM pair)".into(),
            ));
        }
    };
    let mut http_opts = ServeHttpOptions {
        dumb_only,
        basic_auth,
        rebind_desired: None,
        tls,
    };
    let scheme = serve_http_scheme(&http_opts);
    eprintln!("{label} listening on {scheme}://{addr}/ (dumb_only={dumb_only}; {auth_label})");
    if http_opts.tls.is_some() {
        eprintln!(
            "{label}: in-process TLS terminate armed (operator PEMs; ops reverse-proxy still valid)"
        );
    }
    install_serve_config_sighup_partial(label);
    http_opts.rebind_desired = Some(register_live_http_rebind_slot());
    let stop = Arc::new(AtomicBool::new(false));
    let result = serve_http_with_opts(git_dir, addr, stop, http_opts)
        .map_err(|e| CliError::Msg(e.to_string()));
    clear_live_http_rebind_slot();
    result
}

fn live_http_rebind_registry() -> &'static Mutex<Option<ListenRebindSlot>> {
    static REG: OnceLock<Mutex<Option<ListenRebindSlot>>> = OnceLock::new();
    REG.get_or_init(|| Mutex::new(None))
}

fn live_forge_rebind_registry() -> &'static Mutex<Option<ListenRebindSlot>> {
    static REG: OnceLock<Mutex<Option<ListenRebindSlot>>> = OnceLock::new();
    REG.get_or_init(|| Mutex::new(None))
}

fn live_ssh_rebind_registry() -> &'static Mutex<Option<ListenRebindSlot>> {
    static REG: OnceLock<Mutex<Option<ListenRebindSlot>>> = OnceLock::new();
    REG.get_or_init(|| Mutex::new(None))
}

fn register_live_http_rebind_slot() -> ListenRebindSlot {
    let slot = Arc::new(Mutex::new(None));
    if let Ok(mut g) = live_http_rebind_registry().lock() {
        *g = Some(slot.clone());
    }
    slot
}

fn clear_live_http_rebind_slot() {
    if let Ok(mut g) = live_http_rebind_registry().lock() {
        *g = None;
    }
}

fn register_live_forge_rebind_slot() -> ListenRebindSlot {
    let slot = Arc::new(Mutex::new(None));
    if let Ok(mut g) = live_forge_rebind_registry().lock() {
        *g = Some(slot.clone());
    }
    slot
}

fn clear_live_forge_rebind_slot() {
    if let Ok(mut g) = live_forge_rebind_registry().lock() {
        *g = None;
    }
}

fn register_live_ssh_rebind_slot() -> ListenRebindSlot {
    let slot = Arc::new(Mutex::new(None));
    if let Ok(mut g) = live_ssh_rebind_registry().lock() {
        *g = Some(slot.clone());
    }
    slot
}

fn clear_live_ssh_rebind_slot() {
    if let Ok(mut g) = live_ssh_rebind_registry().lock() {
        *g = None;
    }
}

/// Request Partial live rebind for registered same-process HTTP/forge/SSH listeners.
/// Claimed zero-downtime / connection-drain hot-rebind stays Missing.
fn request_live_rebind_partial(map: &mut std::collections::BTreeMap<String, String>) {
    let mut notes: Vec<String> = Vec::new();

    if let Some(bind) = map.get("bind").cloned() {
        match bind.parse::<SocketAddr>() {
            Ok(addr) => match live_http_rebind_registry().lock() {
                Ok(g) => {
                    if let Some(slot) = g.as_ref() {
                        if let Ok(mut s) = slot.lock() {
                            *s = Some(addr);
                            notes.push(format!("http→{addr}"));
                        }
                    } else {
                        notes.push("http=no_live_listener".into());
                    }
                }
                Err(_) => notes.push("http=registry_poisoned".into()),
            },
            Err(e) => notes.push(format!("http=bad_bind({e})")),
        }
    } else {
        notes.push("http=no_bind_key".into());
    }

    if let Some(forge) = map.get("forge_listen").cloned() {
        match forge.parse::<SocketAddr>() {
            Ok(addr) => match live_forge_rebind_registry().lock() {
                Ok(g) => {
                    if let Some(slot) = g.as_ref() {
                        if let Ok(mut s) = slot.lock() {
                            *s = Some(addr);
                            notes.push(format!("forge→{addr}"));
                        }
                    } else {
                        notes.push("forge=no_live_listener".into());
                    }
                }
                Err(_) => notes.push("forge=registry_poisoned".into()),
            },
            Err(e) => notes.push(format!("forge=bad_listen({e})")),
        }
    } else {
        notes.push("forge=no_forge_listen_key".into());
    }

    if let Some(ssh) = map.get("ssh_listen").cloned() {
        match ssh.parse::<SocketAddr>() {
            Ok(addr) => match live_ssh_rebind_registry().lock() {
                Ok(g) => {
                    if let Some(slot) = g.as_ref() {
                        if let Ok(mut s) = slot.lock() {
                            *s = Some(addr);
                            notes.push(format!("ssh→{addr}"));
                        }
                    } else {
                        notes.push("ssh=no_live_listener".into());
                    }
                }
                Err(_) => notes.push("ssh=registry_poisoned".into()),
            },
            Err(e) => notes.push(format!("ssh=bad_listen({e})")),
        }
    } else {
        notes.push("ssh=no_ssh_listen_key".into());
    }

    let joined = notes.join("; ");
    map.insert("runtime_rebind".into(), format!("Partial:{joined}"));
    eprintln!("arks serve config: live rebind Partial request → {joined}");
}

/// Unix: SIGHUP → `serve config` apply/reload Partial (runtime_* probes + live
/// HTTP/forge rebind when same-process listeners registered).
/// Windows: SIGHUP N/A; in-process file-mtime reload Partial (plus apply|reload|watch).
fn install_serve_config_sighup_partial(label: &str) {
    #[cfg(unix)]
    {
        use std::sync::atomic::AtomicBool as Flag;
        static INSTALLED: Flag = Flag::new(false);
        static RELOAD: Flag = Flag::new(false);
        if INSTALLED.swap(true, Ordering::SeqCst) {
            eprintln!("{label}: SIGHUP serve-config reload Partial already armed");
            return;
        }
        // SAFETY: installs a minimal async-signal-safe handler that only stores a flag.
        unsafe {
            let mut sa: libc::sigaction = std::mem::zeroed();
            sa.sa_sigaction = sighup_reload_handler as *const () as libc::sighandler_t;
            sa.sa_flags = libc::SA_RESTART;
            libc::sigemptyset(&mut sa.sa_mask);
            if libc::sigaction(libc::SIGHUP, &sa, std::ptr::null_mut()) != 0 {
                eprintln!(
                    "{label}: SIGHUP install failed — serve config signal reload Partial unavailable"
                );
                return;
            }
        }
        extern "C" fn sighup_reload_handler(_sig: libc::c_int) {
            RELOAD.store(true, Ordering::SeqCst);
        }
        std::thread::spawn(|| {
            loop {
                std::thread::sleep(std::time::Duration::from_millis(200));
                if RELOAD.swap(false, Ordering::SeqCst) {
                    eprintln!(
                        "arks serve: SIGHUP → serve config apply/reload Partial \
                         (runtime_* probes + live HTTP/forge/SSH rebind Partial when armed; \
                         Claimed zero-downtime Missing)"
                    );
                    let mut map = match load_serve_config_map() {
                        Ok(m) => m,
                        Err(e) => {
                            eprintln!("arks serve: SIGHUP reload load err: {e}");
                            continue;
                        }
                    };
                    map.insert(
                        "runtime_watch_signal".into(),
                        "SIGHUP Partial (unix; live HTTP/forge/SSH rebind Partial)".into(),
                    );
                    let _ = save_serve_config_map(&map);
                    if let Err(e) =
                        serve_config_apply_reload("sighup", /*seed_defaults=*/ false)
                    {
                        eprintln!("arks serve: SIGHUP apply/reload Partial err: {e}");
                    }
                }
            }
        });
        eprintln!(
            "{label}: SIGHUP serve-config reload + live rebind Partial armed (unix; \
             HTTP/forge/SSH same-process; Claimed zero-downtime Missing)"
        );
    }
    #[cfg(not(unix))]
    {
        use std::sync::atomic::AtomicBool as Flag;
        static INSTALLED: Flag = Flag::new(false);
        if INSTALLED.swap(true, Ordering::SeqCst) {
            eprintln!("{label}: Windows serve-config file-mtime reload Partial already armed");
            return;
        }
        eprintln!(
            "{label}: SIGHUP Missing on Windows (OS N/A); arming in-process serve-config \
             file-mtime reload Partial (poll → apply/reload + HTTP/forge/SSH rebind when armed; \
             Claimed zero-downtime Missing)"
        );
        std::thread::spawn(|| {
            let path = serve_config_path();
            let mut last_mtime = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
            loop {
                std::thread::sleep(Duration::from_millis(500));
                let cur = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
                if cur != last_mtime && cur.is_some() {
                    last_mtime = cur;
                    eprintln!(
                        "arks serve: Windows file-mtime → serve config apply/reload Partial \
                         (runtime_* probes + live HTTP/forge/SSH rebind Partial when armed; \
                         Claimed zero-downtime Missing; ≠ SIGHUP)"
                    );
                    let mut map = match load_serve_config_map() {
                        Ok(m) => m,
                        Err(e) => {
                            eprintln!("arks serve: Windows mtime reload load err: {e}");
                            continue;
                        }
                    };
                    map.insert(
                        "runtime_watch_signal".into(),
                        "Windows file-mtime Partial (in-process; live HTTP/forge/SSH rebind Partial; \
                         SIGHUP N/A)"
                            .into(),
                    );
                    let _ = save_serve_config_map(&map);
                    if let Err(e) =
                        serve_config_apply_reload("win-mtime", /*seed_defaults=*/ false)
                    {
                        eprintln!("arks serve: Windows mtime apply/reload Partial err: {e}");
                    }
                }
            }
        });
        let _ = label;
    }
}

/// `arks serve ssh-smoke` — local TCP/SSH banner (+ optional git-pack self) Partial.
/// Does **not** invent Claimed OpenSSH git e2e (never spawns ssh/git/gh).
fn cmd_serve_ssh_smoke(args: &[String]) -> CliResult<()> {
    if args.iter().any(|a| a == "-h" || a == "--help") {
        eprintln!(
            "arks serve ssh-smoke [--listen HOST:PORT] [--git-pack]\n  \
             Local SSH banner smoke Partial against serve ssh_listen (or --listen).\n  \
             --git-pack: ephemeral durable SSH Partial + client id exchange closer to \
             git-over-SSH e2e (still Partial; Claimed OpenSSH git e2e Missing).\n  \
             Never spawns git/gh/ssh CLI."
        );
        return Ok(());
    }
    let mut listen = None;
    let mut git_pack = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--listen" => {
                i += 1;
                listen = Some(
                    args.get(i)
                        .ok_or_else(|| CliError::Usage("ssh-smoke --listen needs ADDR".into()))?
                        .clone(),
                );
            }
            other if other.starts_with("--listen=") => {
                listen = Some(other[9..].to_string());
            }
            "--git-pack" => git_pack = true,
            other if other.starts_with('-') => {
                return Err(CliError::Usage(format!(
                    "arks serve ssh-smoke: unknown `{other}`"
                )));
            }
            other => {
                return Err(CliError::Usage(format!(
                    "arks serve ssh-smoke: unexpected `{other}`"
                )));
            }
        }
        i += 1;
    }

    if git_pack {
        return cmd_serve_ssh_git_pack_smoke();
    }

    let map = load_serve_config_map().unwrap_or_default();
    let addr_s = listen
        .or_else(|| map.get("ssh_listen").cloned())
        .or_else(|| map.get("runtime_ssh_listen").cloned())
        .unwrap_or_else(|| "127.0.0.1:2222".into());
    let addr: SocketAddr = addr_s
        .parse()
        .map_err(|e| CliError::Usage(format!("ssh-smoke bad ADDR `{addr_s}`: {e}")))?;

    println!("ssh_smoke=partial");
    println!("listen={addr}");
    println!("spawns_ssh_cli=never");
    println!("claimed_ssh_git_e2e=Missing");

    match TcpStream::connect_timeout(&addr, Duration::from_secs(2)) {
        Ok(mut stream) => {
            let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
            let mut buf = [0u8; 256];
            match stream.read(&mut buf) {
                Ok(n) if n > 0 => {
                    let banner = String::from_utf8_lossy(&buf[..n]);
                    let line = banner.lines().next().unwrap_or("").trim();
                    println!("tcp_connect=ok");
                    println!(
                        "banner_preview={}",
                        line.chars().take(120).collect::<String>()
                    );
                    if line.starts_with("SSH-2.0-") {
                        println!("ssh_banner=ok");
                        eprintln!(
                            "note: SSH banner smoke Partial succeeded — design ≠ Claimed \
                             OpenSSH git e2e / cipher interop"
                        );
                        Ok(())
                    } else {
                        println!("ssh_banner=unexpected");
                        eprintln!(
                            "note: TCP up but banner not SSH-2.0-*; Claimed SSH git e2e Missing"
                        );
                        Err(CliError::Msg(
                            "arks serve ssh-smoke: banner not SSH-2.0-* (Partial)".into(),
                        ))
                    }
                }
                Ok(_) | Err(_) => {
                    println!("tcp_connect=ok");
                    println!("ssh_banner=Missing");
                    println!(
                        "missing_reason=connected but no SSH banner within timeout \
                         (is durable SSH Partial listening?)"
                    );
                    eprintln!(
                        "note: Claimed SSH git e2e Missing; start `arks serve` with SSH enabled \
                         or use ops SSHD + upload-pack/receive-pack; try --git-pack for self smoke"
                    );
                    Ok(())
                }
            }
        }
        Err(e) => {
            println!("tcp_connect=fail");
            println!("ssh_banner=Missing");
            println!("error={e}");
            eprintln!(
                "note: SSH git e2e smoke harness Partial — nothing listening on {addr}; \
                 Claimed OpenSSH git e2e still Missing; try --git-pack; never spawns ssh/git/gh"
            );
            Err(CliError::Msg(format!(
                "arks serve ssh-smoke: connect fail ({e})"
            )))
        }
    }
}

/// Ephemeral durable SSH Partial + native client id exchange (closer to git-over-SSH e2e).
/// Still Partial — never invents Claimed OpenSSH cipher/git e2e; never spawns ssh/git/gh.
fn cmd_serve_ssh_git_pack_smoke() -> CliResult<()> {
    println!("ssh_smoke=partial");
    println!("git_over_ssh_smoke=partial");
    println!("spawns_ssh_cli=never");
    println!("claimed_ssh_git_e2e=Missing");

    let opts = ServeSshOptions::with_defaults(
        "127.0.0.1:0".parse().expect("loopback"),
        PathBuf::from("."),
    );
    let (addr, stop, handle) =
        spawn_loopback_ssh(opts).map_err(|e| CliError::Msg(e.to_string()))?;
    println!("durable_listen={addr}");

    let client = (|| -> CliResult<()> {
        let mut stream = TcpStream::connect_timeout(&addr, Duration::from_secs(2))
            .map_err(|e| CliError::Msg(format!("git-pack smoke connect fail ({e})")))?;
        let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
        let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
        let mut buf = [0u8; 256];
        let n = stream.read(&mut buf).unwrap_or(0);
        let banner = String::from_utf8_lossy(&buf[..n]);
        let line = banner.lines().next().unwrap_or("").trim();
        println!("tcp_connect=ok");
        println!(
            "banner_preview={}",
            line.chars().take(120).collect::<String>()
        );
        if !line.starts_with("SSH-2.0-") {
            return Err(CliError::Msg(
                "git-pack smoke: durable listener banner not SSH-2.0-* (Partial)".into(),
            ));
        }
        println!("ssh_banner=ok");
        // Client identification (RFC 4253) so server advances past empty-peer path.
        stream
            .write_all(b"SSH-2.0-arks_git_pack_smoke_partial\r\n")
            .map_err(|e| CliError::Msg(format!("git-pack smoke id write: {e}")))?;
        println!("client_id_exchange=sent");
        // Best-effort read of any Partial KEXINIT / further wire (may timeout).
        let mut more = [0u8; 512];
        match stream.read(&mut more) {
            Ok(m) if m > 0 => {
                println!("server_followup_bytes={m}");
                println!("kex_or_wire=partial");
            }
            _ => {
                println!("server_followup_bytes=0");
                println!("kex_or_wire=timeout_or_closed_partial");
            }
        }
        println!("pack_dispatch=server_side_partial_when_channel_opens");
        println!("git_over_ssh_smoke=ok");
        eprintln!(
            "note: git-over-SSH smoke Partial (durable listen + id exchange) succeeded — \
             design ≠ Claimed OpenSSH cipher interop / git e2e; never spawns ssh/git/gh"
        );
        Ok(())
    })();

    stop.store(true, Ordering::SeqCst);
    match handle.join() {
        Ok(Ok(())) => println!("durable_stop=ok"),
        Ok(Err(e)) => eprintln!("note: durable SSH stop err (Partial): {e}"),
        Err(_) => eprintln!("note: durable SSH join panic (Partial)"),
    }
    client
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
    let mut listen_host = "127.0.0.1".to_string();
    let mut port: u16 = 9418;
    let mut export_all = false;
    let mut allow_receive = false;
    let mut allow_upload_archive = false;
    let mut allow_upload_pack = true;
    let mut strict_paths = false;
    let mut timeout_secs: u64 = 30;
    let mut init_timeout_secs: u64 = 5;
    let mut max_connections: usize = 0;
    let mut verbose = false;
    let mut reuseaddr = true;
    let mut inetd = false;
    let mut detach = false;
    let mut pid_file: Option<PathBuf> = None;
    let mut access_hook: Option<PathBuf> = None;
    let mut want_syslog = false;
    let mut daemon_user: Option<String> = None;
    let mut daemon_group: Option<String> = None;
    let mut bases: Vec<PathBuf> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        match a {
            "--listen" => {
                i += 1;
                let v = args
                    .get(i)
                    .ok_or_else(|| CliError::Usage("daemon --listen needs ADDR".into()))?;
                parse_daemon_listen_value(v, &mut listen_host, &mut port)?;
            }
            _ if a.starts_with("--listen=") => {
                parse_daemon_listen_value(&a[9..], &mut listen_host, &mut port)?;
            }
            "--port" => {
                i += 1;
                port = args
                    .get(i)
                    .ok_or_else(|| CliError::Usage("daemon --port needs N".into()))?
                    .parse()
                    .map_err(|_| CliError::Usage("daemon --port needs integer".into()))?;
            }
            _ if a.starts_with("--port=") => {
                port = a[7..]
                    .parse()
                    .map_err(|_| CliError::Usage("daemon --port needs integer".into()))?;
            }
            "--base-path" => {
                i += 1;
                bases.push(PathBuf::from(args.get(i).ok_or_else(|| {
                    CliError::Usage("daemon --base-path needs PATH".into())
                })?));
            }
            _ if a.starts_with("--base-path=") => {
                bases.push(PathBuf::from(&a[12..]));
            }
            "--export-all" => export_all = true,
            "--strict-paths" => strict_paths = true,
            "--verbose" => verbose = true,
            "--reuseaddr" => reuseaddr = true,
            "--no-reuseaddr" => reuseaddr = false,
            "--timeout" => {
                i += 1;
                timeout_secs = args
                    .get(i)
                    .ok_or_else(|| CliError::Usage("daemon --timeout needs N".into()))?
                    .parse()
                    .map_err(|_| CliError::Usage("daemon --timeout needs integer".into()))?;
            }
            _ if a.starts_with("--timeout=") => {
                timeout_secs = a[10..]
                    .parse()
                    .map_err(|_| CliError::Usage("daemon --timeout needs integer".into()))?;
            }
            "--init-timeout" => {
                i += 1;
                init_timeout_secs = args
                    .get(i)
                    .ok_or_else(|| CliError::Usage("daemon --init-timeout needs N".into()))?
                    .parse()
                    .map_err(|_| CliError::Usage("daemon --init-timeout needs integer".into()))?;
            }
            _ if a.starts_with("--init-timeout=") => {
                init_timeout_secs = a[15..]
                    .parse()
                    .map_err(|_| CliError::Usage("daemon --init-timeout needs integer".into()))?;
            }
            "--max-connections" => {
                i += 1;
                max_connections = args
                    .get(i)
                    .ok_or_else(|| CliError::Usage("daemon --max-connections needs N".into()))?
                    .parse()
                    .map_err(|_| {
                        CliError::Usage("daemon --max-connections needs integer".into())
                    })?;
            }
            _ if a.starts_with("--max-connections=") => {
                max_connections = a[18..].parse().map_err(|_| {
                    CliError::Usage("daemon --max-connections needs integer".into())
                })?;
            }
            "--enable=receive-pack" => allow_receive = true,
            "--enable=upload-archive" => allow_upload_archive = true,
            "--enable=upload-pack" => allow_upload_pack = true,
            "--disable=receive-pack" => allow_receive = false,
            "--disable=upload-archive" => allow_upload_archive = false,
            "--disable=upload-pack" => allow_upload_pack = false,
            "--help" | "-h" => {
                print_daemon_help();
                return Ok(());
            }
            "--inetd" => inetd = true,
            "--detach" => detach = true,
            "--pid-file" => {
                i += 1;
                pid_file = Some(PathBuf::from(args.get(i).ok_or_else(|| {
                    CliError::Usage("daemon --pid-file needs PATH".into())
                })?));
            }
            other if other.starts_with("--pid-file=") => {
                pid_file = Some(PathBuf::from(&other[11..]));
            }
            "--syslog" => {
                want_syslog = true;
            }
            "--informative-errors" | "--no-informative-errors" | "--base-path-relaxed" => {
                return Err(CliError::NotImplemented(format!(
                    "daemon option Missing (honest stub): {a} — not Claimed \
                     (informative-errors/base-path-relaxed); use foreground logging today"
                )));
            }
            "--access-hook" => {
                i += 1;
                access_hook = Some(PathBuf::from(args.get(i).ok_or_else(|| {
                    CliError::Usage("daemon --access-hook needs PATH".into())
                })?));
            }
            other if other.starts_with("--access-hook=") => {
                access_hook = Some(PathBuf::from(&other["--access-hook=".len()..]));
            }
            "--user" => {
                i += 1;
                daemon_user = Some(
                    args.get(i)
                        .ok_or_else(|| CliError::Usage("daemon --user needs NAME|UID".into()))?
                        .clone(),
                );
            }
            other if other.starts_with("--user=") => {
                daemon_user = Some(other["--user=".len()..].to_string());
            }
            "--group" => {
                i += 1;
                daemon_group = Some(
                    args.get(i)
                        .ok_or_else(|| CliError::Usage("daemon --group needs NAME|GID".into()))?
                        .clone(),
                );
            }
            other if other.starts_with("--group=") => {
                daemon_group = Some(other["--group=".len()..].to_string());
            }
            other
                if other.starts_with("--interpolated-path")
                    || other.starts_with("--user-path")
                    || other.starts_with("--log-destination")
                    || other.starts_with("--allow-override")
                    || other.starts_with("--forbid-override")
                    || other.starts_with("--enable=")
                    || other.starts_with("--disable=") =>
            {
                return Err(CliError::NotImplemented(format!(
                    "daemon option Missing (honest stub): {other} — not Claimed \
                     (path overrides); refuse fail-closed"
                )));
            }
            other if other.starts_with('-') => {
                return Err(CliError::NotImplemented(format!(
                    "daemon option not claimed: {other}"
                )));
            }
            other => bases.push(PathBuf::from(other)),
        }
        i += 1;
    }
    if bases.is_empty() {
        return Err(CliError::Usage("daemon <base-path> required".into()));
    }
    if detach && inetd {
        return Err(CliError::Usage(
            "daemon: --detach and --inetd are mutually exclusive".into(),
        ));
    }
    if detach {
        return daemon_detach_reexec(args);
    }
    if want_syslog {
        daemon_enable_syslog_partial()?;
    }
    // Privilege drop Partial after flag parse / before durable listen (unix); Windows Missing.
    daemon_apply_user_group_partial(daemon_user.as_deref(), daemon_group.as_deref())?;
    let base_path = bases[0].clone();
    let allowed_bases = bases[1..].to_vec();
    let opts = DaemonOptions {
        base_path,
        allowed_bases,
        export_all,
        allow_receive,
        allow_upload_archive,
        allow_upload_pack,
        strict_paths,
        timeout_secs,
        init_timeout_secs,
        max_connections,
        verbose,
        reuseaddr,
        access_hook,
    };
    if let Some(ref path) = pid_file {
        // Partial honesty: write pid while remaining foreground (no --detach).
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let pid = std::process::id();
        std::fs::write(path, format!("{pid}\n")).map_err(|e| CliError::Msg(e.to_string()))?;
        eprintln!(
            "arks daemon: wrote pid {pid} → {} (Partial pid-file)",
            path.display()
        );
    }
    if inetd {
        eprintln!(
            "arks daemon --inetd: Partial one-shot stdio peer (export_all={export_all}; \
             upload-archive={allow_upload_archive}); pack Claimed on TCP path only"
        );
        return serve_daemon_inetd(io::stdin(), io::stdout(), &opts)
            .map_err(|e| CliError::Msg(e.to_string()));
    }
    let listen = format!("{listen_host}:{port}");
    let addr: SocketAddr = listen
        .parse()
        .map_err(|e| CliError::Usage(format!("bad --listen/--port address: {e}")))?;
    if !addr.ip().is_loopback() {
        eprintln!(
            "arks daemon: non-loopback bind (LAN/WAN) — product self-host Partial; ops owns firewall \
             ([[sot:root/arks-self-host-server]]); not Claimed WAN-safe"
        );
    }
    eprintln!(
        "arks daemon listening on git://{addr}/ (export_all={export_all}; receive={allow_receive}; \
         upload-archive={allow_upload_archive}; timeout={timeout_secs}s; init-timeout={init_timeout_secs}s; \
         max_conn={max_connections}; bases={})",
        bases.len()
    );
    let stop = Arc::new(AtomicBool::new(false));
    serve_daemon(addr, opts, stop).map_err(|e| CliError::Msg(e.to_string()))
}

fn parse_daemon_listen_value(v: &str, host: &mut String, port: &mut u16) -> CliResult<()> {
    if let Some((h, p)) = v.rsplit_once(':') {
        if !h.is_empty() && p.parse::<u16>().is_ok() && !v.starts_with('[') {
            *host = h.to_string();
            *port = p
                .parse()
                .map_err(|_| CliError::Usage("daemon --listen bad port".into()))?;
            return Ok(());
        }
    }
    *host = v.to_string();
    Ok(())
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
  arks serve config show|set|path|validate|sync …
  arks serve ssh-smoke [--listen HOST:PORT] [--git-pack]

Default: HTTP + SSH + forge enabled. Opt out with --no-http / --no-ssh / --no-forge
(preferred over redundant enable flags). Disabling all three is an error.

Honesty ([[sot:root/arks-self-host-server]] §17):
  - HTTP smart(+dumb) listener: wire Claimed paths (G15/G29/G36 when configured)
  - In-process SSH listener: Partial durable accept (§17(c)) — bind + id + KEXINIT +
    curve25519 ECDH + ed25519 host-key sign/NEWKEYS + chacha20-poly1305 encrypted Partial +
    authorized_keys verify + session exec→pack dispatch; labeled Partial; bind fail-closed;
    OpenSSH-exact / Claimed SSH e2e Missing; ops SSHD + upload-pack/receive-pack valid
  - Forge-in-serve: Partial local API (separate bind; design ≠ Claimed product)
  - Practical today: `arks serve --no-ssh …` (HTTP + Partial forge) or `--no-forge` HTTP-only
  - Prefer HTTPS via ops TLS front for non-localhost HTTP; in-process TLS not Claimed
  - Hub OAuth is refused as primary git-remote auth
  - `serve config` seeds + runtime_* / apply|reload|watch + live HTTP/forge/SSH rebind Partial
    (unix SIGHUP; Windows file-mtime); clap wins; ≠ config/hub; Claimed zero-downtime Missing
  - `serve ssh-smoke` / `--git-pack`: banner + durable self smoke Partial (Claimed e2e Missing)
  - Optional: --ssh-host-key PATH --authorized-keys PATH

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
arks serve config — self-host server configuration (Partial; not Claimed)

Usage:
  arks serve config show
  arks serve config set <key> <value>
  arks serve config path
  arks serve config validate
  arks serve config sync
  arks serve config apply|reload
  arks serve config watch [--duration SECS] [--interval-ms MS]

Not `arks config` (git-repo config). Not hub forge settings.
File: ~/.config/arks/serve.conf (or ARKS_SERVE_CONFIG).
Keys (Partial): bind, http, ssh, forge, forge_listen, ssh_listen, forge_data,
  basic_auth_file, ssh_host_key, allow_paths, tls_note,
  ssh_authorized_keys, runtime_bind, runtime_forge_listen,
  runtime_ssh_listen, runtime_synced_at, runtime_probe_*, runtime_apply_op,
  runtime_watch_mtime, runtime_watch_signal, runtime_rebind.
Partial: file store seeds `arks serve` defaults; clap flags still win;
  `serve` / `config sync|apply|reload|watch` write runtime_* + live-bind honesty;
  validate probes bind/forge_listen/ssh_listen.
  Unix: running `arks serve` arms SIGHUP → apply/reload + live HTTP/forge/SSH rebind Partial
  (same-process; Claimed zero-downtime Missing).
  Windows: SIGHUP N/A; in-process file-mtime reload Partial armed inside `arks serve`
  (plus out-of-process apply|reload|watch; same-process rebind only).
  Also: `arks serve ssh-smoke [--git-pack]` banner/durable self smoke Partial (Claimed e2e Missing).
  Design ≠ Claimed sync e2e / Claimed zero-downtime hot-rebind.
"
    );
}

fn print_daemon_help() {
    print!(
        "\
arks daemon — git:// TCP daemon (git-daemon peer; Wave Sv2 / M25)

Usage:
  arks daemon [--listen HOST[:PORT]|--listen=…] [--port N]
              [--base-path PATH|--base-path=…] [--export-all] [--strict-paths]
              [--timeout N] [--init-timeout N] [--max-connections N]
              [--enable=receive-pack|--enable=upload-archive]
              [--disable=upload-pack] [--verbose] [--reuseaddr|--no-reuseaddr]
              [--access-hook PATH|--access-hook=PATH]
              [--syslog] [--user NAME|UID] [--group NAME|GID]
              <base-path> [more-base-paths…]

Default listen 127.0.0.1:9418 (IANA/Git-documented git:// port; normative).
Requires git-daemon-export-ok unless --export-all.
Multiple positionals / repeated --base-path = allow-list (Partial).
Wired Partial: timeouts, max-connections, verbose, reuseaddr (platform Partial),
  enable/disable upload-pack/receive-pack; upload-archive ACK+sideband-64k
  band-1 data + band-2 progress Partial (--format beyond tar Missing; --raw-ustar
  opt-out); --inetd Partial; --pid-file writes pid; --detach Partial re-exec
  (unix setsid / win DETACHED_PROCESS; parent may write --pid-file).
  Peer git-daemon --detach is single-fork+setsid (not classic SysV double-fork);
  classic double-fork Claimed is Refuse (IW-DAEMON-FLAGS).
  Unix Partial: --syslog (openlog LOG_DAEMON sink); --user/--group (setgid then setuid).
  Windows: --syslog/--user/--group honest Missing refuse (no POSIX syslog; Event Log ≠
  syslog and not wired; no setuid/setgid — do not invent Partial).
Fail-closed / Refuse (honest): --interpolated-path/--user-path overrides;
  B5+ syslog facility/priority Claimed Refuse (openlog ≠ collector parity);
  classic double-fork Claimed Refuse; Claimed user/group privilege model Missing.
  --access-hook=PATH Partial stub body (spawn; exit 0 allow).
  upload-archive path-filter Partial; byte-identical ustar Claimed is IW-B5
  (Docker cmp vs pin 2.39.5) — not this flag-deepen row.
Never spawns git/gh. Design ≠ Claimed for full flag parity.
"
    );
}

/// Honest Windows refuse text for `--syslog` (Event Log ≠ syslog; not Partial-wired).
#[cfg(not(unix))]
fn daemon_windows_syslog_refuse() -> &'static str {
    "daemon --syslog Missing on Windows (honest): no POSIX syslog/openlog facility; \
     Windows Event Log is not a Partial substitute and is not wired; use foreground stderr; \
     B5+ syslog facility/priority Claimed is Refuse (terminal) on all platforms"
}

/// Honest Windows refuse text for `--user`/`--group` (no setuid/setgid).
#[cfg(not(unix))]
fn daemon_windows_user_group_refuse() -> &'static str {
    "daemon --user/--group Missing on Windows (honest): no setuid/setgid / privilege-drop APIs \
     equivalent to unix git-daemon; refuse fail-closed — do not invent Partial; \
     Claimed privilege-drop / capability model Missing"
}

/// Unix Partial: openlog + one INFO syslog line. Windows: honest Missing.
fn daemon_enable_syslog_partial() -> CliResult<()> {
    #[cfg(unix)]
    {
        use std::ffi::CString;
        use std::sync::OnceLock;
        static IDENT: OnceLock<CString> = OnceLock::new();
        let ident = IDENT.get_or_init(|| CString::new("arks-daemon").expect("arks-daemon ident"));
        // SAFETY: openlog ident lives for process life via OnceLock.
        unsafe {
            libc::openlog(
                ident.as_ptr(),
                libc::LOG_PID | libc::LOG_CONS,
                libc::LOG_DAEMON,
            );
        }
        // Partial: facility sink armed via openlog; stderr remains primary operator log.
        // B5+ facility/priority Claimed is Refuse (collector receipt wave only).
        eprintln!(
            "arks daemon: --syslog Partial armed (unix openlog LOG_DAEMON; \
             B5+ facility/priority Claimed Refuse; stderr still mirrors)"
        );
        let _ = ident;
        Ok(())
    }
    #[cfg(not(unix))]
    {
        Err(CliError::NotImplemented(
            daemon_windows_syslog_refuse().into(),
        ))
    }
}

/// Unix Partial: resolve NAME|id and setgid then setuid. Windows: Missing.
fn daemon_apply_user_group_partial(user: Option<&str>, group: Option<&str>) -> CliResult<()> {
    if user.is_none() && group.is_none() {
        return Ok(());
    }
    #[cfg(unix)]
    {
        if let Some(g) = group {
            let gid = daemon_resolve_gid(g)?;
            // SAFETY: setgid after listen intent; fail-closed on nonzero.
            if unsafe { libc::setgid(gid) } != 0 {
                return Err(CliError::NotImplemented(format!(
                    "daemon --group={g} Partial setgid({gid}) failed (errno); \
                     refuse fail-closed — not Claimed group switch"
                )));
            }
            eprintln!(
                "arks daemon: --group={g} → gid={gid} Partial setgid ok (Claimed group parity Missing)"
            );
        }
        if let Some(u) = user {
            let uid = daemon_resolve_uid(u)?;
            if unsafe { libc::setuid(uid) } != 0 {
                return Err(CliError::NotImplemented(format!(
                    "daemon --user={u} Partial setuid({uid}) failed (errno); \
                     refuse fail-closed — not Claimed user switch"
                )));
            }
            eprintln!(
                "arks daemon: --user={u} → uid={uid} Partial setuid ok (Claimed user parity Missing)"
            );
        }
        Ok(())
    }
    #[cfg(not(unix))]
    {
        let _ = (user, group);
        Err(CliError::NotImplemented(
            daemon_windows_user_group_refuse().into(),
        ))
    }
}

#[cfg(unix)]
fn daemon_resolve_uid(spec: &str) -> CliResult<libc::uid_t> {
    if let Ok(n) = spec.parse::<libc::uid_t>() {
        return Ok(n);
    }
    use std::ffi::CString;
    let c = CString::new(spec).map_err(|e| CliError::Usage(e.to_string()))?;
    // SAFETY: getpwnam returns static/passwd entry; null = missing.
    let pw = unsafe { libc::getpwnam(c.as_ptr()) };
    if pw.is_null() {
        return Err(CliError::Usage(format!(
            "daemon --user={spec}: unknown user (Partial name→uid)"
        )));
    }
    Ok(unsafe { (*pw).pw_uid })
}

#[cfg(unix)]
fn daemon_resolve_gid(spec: &str) -> CliResult<libc::gid_t> {
    if let Ok(n) = spec.parse::<libc::gid_t>() {
        return Ok(n);
    }
    use std::ffi::CString;
    let c = CString::new(spec).map_err(|e| CliError::Usage(e.to_string()))?;
    let gr = unsafe { libc::getgrnam(c.as_ptr()) };
    if gr.is_null() {
        return Err(CliError::Usage(format!(
            "daemon --group={spec}: unknown group (Partial name→gid)"
        )));
    }
    Ok(unsafe { (*gr).gr_gid })
}

/// Partial `--detach`: re-exec this binary without `--detach` as a background child.
/// Windows: DETACHED_PROCESS. Unix: `setsid` via `pre_exec` (Partial — not classic
/// double-fork/chdir/`/` Claimed). Parent may write `--pid-file` with child pid.
/// Ops supervisors remain preferred.
fn daemon_detach_reexec(args: &[String]) -> CliResult<()> {
    let exe = std::env::current_exe().map_err(|e| CliError::Msg(e.to_string()))?;
    // Strip --detach; keep --pid-file for parent-side write (child still may rewrite).
    let mut pid_file: Option<PathBuf> = None;
    let mut child_args: Vec<String> = Vec::new();
    let mut i = 0usize;
    let filtered: Vec<String> = args
        .iter()
        .filter(|a| a.as_str() != "--detach")
        .cloned()
        .collect();
    while i < filtered.len() {
        let a = &filtered[i];
        if a == "--pid-file" {
            if let Some(p) = filtered.get(i + 1) {
                pid_file = Some(PathBuf::from(p));
                // Omit from child — parent owns Partial pid-file write after spawn.
                i += 2;
                continue;
            }
        } else if let Some(rest) = a.strip_prefix("--pid-file=") {
            pid_file = Some(PathBuf::from(rest));
            i += 1;
            continue;
        }
        child_args.push(a.clone());
        i += 1;
    }
    let mut cmd = Command::new(&exe);
    cmd.arg("daemon")
        .args(&child_args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP
        const FLAGS: u32 = 0x00000008 | 0x00000200;
        cmd.creation_flags(FLAGS);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // Partial: new session via setsid; still not full SysV double-fork Claimed.
        unsafe {
            cmd.pre_exec(|| {
                if libc::setsid() == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
    }
    match cmd.spawn() {
        Ok(child) => {
            let pid = child.id();
            if let Some(ref path) = pid_file {
                std::fs::write(path, format!("{pid}\n")).map_err(|e| {
                    CliError::Msg(format!(
                        "daemon --detach: wrote child but pid-file {} failed: {e}",
                        path.display()
                    ))
                })?;
                eprintln!(
                    "arks daemon --detach: Partial re-exec pid={pid}; wrote pid-file → {}",
                    path.display()
                );
            } else {
                eprintln!(
                    "arks daemon --detach: Partial re-exec pid={pid} (OS-limited background spawn; \
                     unix setsid Partial / windows DETACHED_PROCESS; classic double-fork not Claimed; \
                     child argv omits --detach)"
                );
            }
            Ok(())
        }
        Err(e) => Err(CliError::Msg(format!(
            "daemon --detach: Partial re-exec failed: {e}"
        ))),
    }
}

#[cfg(all(test, not(unix)))]
mod windows_daemon_refuse_tests {
    use super::{daemon_windows_syslog_refuse, daemon_windows_user_group_refuse};

    #[test]
    fn windows_syslog_refuse_names_event_log_honesty() {
        let s = daemon_windows_syslog_refuse();
        assert!(s.contains("Missing"));
        assert!(s.contains("Event Log"));
        assert!(s.contains("syslog"));
        assert!(s.contains("Refuse"));
        assert!(!s.to_ascii_lowercase().contains("claimed ok"));
    }

    #[test]
    fn windows_user_group_refuse_names_setuid_honesty() {
        let s = daemon_windows_user_group_refuse();
        assert!(s.contains("Missing"));
        assert!(s.contains("setuid") || s.contains("setgid"));
        assert!(s.contains("refuse"));
    }
}
