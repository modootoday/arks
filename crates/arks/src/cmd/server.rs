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
    default_authorized_keys_path, default_host_key_path, handle_smart_http_auth,
    receive_pack_session, serve_daemon, serve_daemon_inetd, serve_http_with_opts,
    serve_ssh_listener, update_server_info, upload_pack_session,
};
use std::io::{self, Read, Write};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::{Command, Stdio};
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
                // Replace any seeded --listen from file store.
                forwarded.retain(|x| x != "--listen" && !listen_from_cfg.as_ref().is_some_and(|b| b == x));
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
                forge_data = Some(PathBuf::from(
                    args.get(i)
                        .ok_or_else(|| CliError::Usage("serve --forge-data needs DIR".into()))?,
                ));
            }
            "--ssh-host-key" => {
                i += 1;
                ssh_host_key = PathBuf::from(
                    args.get(i)
                        .ok_or_else(|| CliError::Usage("serve --ssh-host-key needs PATH".into()))?,
                );
            }
            "--authorized-keys" => {
                i += 1;
                ssh_authorized_keys = PathBuf::from(
                    args.get(i)
                        .ok_or_else(|| CliError::Usage("serve --authorized-keys needs PATH".into()))?,
                );
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
                forwarded.retain(|x| x != "--listen" && !listen_from_cfg.as_ref().is_some_and(|b| b == x));
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
            forwarded.iter().find_map(|a| {
                a.strip_prefix("--listen=")
                    .map(str::to_string)
            })
        })
        .or(listen_from_cfg.clone())
        .unwrap_or_else(|| "127.0.0.1:8080".into());

    // Fail-closed: SSH enabled but in-process listener Missing/Partial — do not silent-degrade.
    if enable_ssh {
        let ssh_listen: SocketAddr = ssh_listen_cfg
            .parse()
            .map_err(|e| CliError::Usage(format!("serve ssh_listen bad ADDR: {e}")))?;
        sync_serve_runtime_to_file(
            enable_http.then_some(http_bind_for_sync.as_str()),
            enable_forge.then_some(forge_listen.as_str()),
            Some(ssh_listen_cfg.as_str()),
        );
        // Directory is optional here — Missing path refuses before bind.
        let dir = forwarded
            .iter()
            .find(|a| !a.starts_with('-') && *a != "--listen" && !a.contains(':'))
            .cloned()
            // Prefer last positional (directory); skip seeded listen values.
            .or_else(|| {
                forwarded
                    .iter()
                    .rev()
                    .find(|a| !a.starts_with('-'))
                    .cloned()
            })
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        // If dir accidentally captured a HOST:PORT from --listen value, fall back.
        let dir = if dir.to_string_lossy().parse::<SocketAddr>().is_ok() {
            PathBuf::from(".")
        } else {
            dir
        };
        return serve_ssh_listener(
            ServeSshOptions {
                listen: ssh_listen,
                path: dir,
                host_key: ssh_host_key,
                authorized_keys: ssh_authorized_keys,
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

    sync_serve_runtime_to_file(
        enable_http.then_some(http_bind_for_sync.as_str()),
        enable_forge.then_some(forge_listen.as_str()),
        None,
    );

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
            eprintln!("note: Partial file store — design ≠ Claimed ([[sot:root/arks-self-host-server]])");
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
        // Partial toward sync e2e without inventing Claimed running-server hot-reload.
        "apply" | "reload" => serve_config_apply_reload(sub, /*seed_defaults=*/ false),
        other => Err(CliError::Usage(format!(
            "arks serve config: unknown `{other}` (show|set|path|validate|sync|apply|reload); see --help"
        ))),
    }
}

/// Shared Partial bind apply/reload/sync: probe listens and persist runtime_* keys.
/// Does **not** mutate a running `arks serve` process (Claimed hot-reload Missing).
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
    let mut probes: Vec<(&str, String)> = vec![
        ("bind", bind.clone()),
        ("forge_listen", forge.clone()),
    ];
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
    save_serve_config_map(&map)?;
    eprintln!(
        "arks serve config {label}: wrote runtime_* + runtime_probe_* → {} \
         (Partial bind apply/reload; clap still wins; running-server hot-reload Claimed Missing; \
         not Claimed sync e2e)",
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
    // Runtime sync keys (Partial live bind honesty; written by `serve` / `config sync`).
    "runtime_bind",
    "runtime_forge_listen",
    "runtime_ssh_listen",
    "runtime_synced_at",
    "runtime_probe_bind",
    "runtime_probe_forge_listen",
    "runtime_probe_ssh_listen",
    "runtime_apply_op",
];

fn config_bool(
    map: &std::collections::BTreeMap<String, String>,
    key: &str,
    default: bool,
) -> bool {
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
        "bind" | "forge_listen" | "ssh_listen" | "runtime_bind" | "runtime_forge_listen"
        | "runtime_ssh_listen" => {
            if value.parse::<SocketAddr>().is_ok() {
                Ok(())
            } else {
                Err("expected HOST:PORT socket address".into())
            }
        }
        "forge_data" | "basic_auth_file" | "ssh_host_key" | "ssh_authorized_keys"
        | "allow_paths" | "tls_note" | "runtime_synced_at" | "runtime_probe_bind"
        | "runtime_probe_forge_listen" | "runtime_probe_ssh_listen" | "runtime_apply_op" => {
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
    let sock: SocketAddr = addr
        .parse()
        .map_err(|e| format!("bad address: {e}"))?;
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
                bases.push(PathBuf::from(
                    args.get(i)
                        .ok_or_else(|| CliError::Usage("daemon --base-path needs PATH".into()))?,
                ));
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
                pid_file = Some(PathBuf::from(
                    args.get(i)
                        .ok_or_else(|| CliError::Usage("daemon --pid-file needs PATH".into()))?,
                ));
            }
            other if other.starts_with("--pid-file=") => {
                pid_file = Some(PathBuf::from(&other[11..]));
            }
            "--syslog" | "--informative-errors" | "--no-informative-errors"
            | "--base-path-relaxed" => {
                return Err(CliError::NotImplemented(format!(
                    "daemon option Missing (honest stub): {a} — not Claimed \
                     (syslog/informative-errors/base-path-relaxed); use foreground logging today"
                )));
            }
            other if other.starts_with("--access-hook") => {
                return Err(CliError::NotImplemented(format!(
                    "daemon --access-hook Missing (honest stub): external access-hook spawn \
                     not wired — design ≠ Claimed; refuse rather than silent-ignore ({other})"
                )));
            }
            other
                if other.starts_with("--user")
                    || other.starts_with("--group")
                    || other.starts_with("--interpolated-path")
                    || other.starts_with("--user-path")
                    || other.starts_with("--log-destination")
                    || other.starts_with("--allow-override")
                    || other.starts_with("--forbid-override")
                    || other.starts_with("--enable=")
                    || other.starts_with("--disable=") =>
            {
                return Err(CliError::NotImplemented(format!(
                    "daemon option Missing (honest stub): {other} — not Claimed \
                     (user/group/path overrides); refuse fail-closed"
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

Default: HTTP + SSH + forge enabled. Opt out with --no-http / --no-ssh / --no-forge
(preferred over redundant enable flags). Disabling all three is an error.

Honesty ([[sot:root/arks-self-host-server]]):
  - HTTP smart(+dumb) listener: wire Claimed paths (G15/G29/G36 when configured)
  - In-process SSH listener: Partial stages (bind + id + KEXINIT + curve25519 ECDH) then
    fail-closed — host-key sign / auth / channel Missing; ops SSHD + upload-pack/receive-pack valid
  - Forge-in-serve: Partial local API (separate bind; design ≠ Claimed product)
  - Practical today: `arks serve --no-ssh …` (HTTP + Partial forge) or `--no-forge` HTTP-only
  - Prefer HTTPS via ops TLS front for non-localhost HTTP; in-process TLS not Claimed
  - Hub OAuth is refused as primary git-remote auth
  - `serve config` seeds defaults + runtime_* / apply|reload probes (Partial); clap wins; ≠ config/hub
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

Not `arks config` (git-repo config). Not hub forge settings.
File: ~/.config/arks/serve.conf (or ARKS_SERVE_CONFIG).
Keys (Partial): bind, http, ssh, forge, forge_listen, ssh_listen, forge_data,
  basic_auth_file, ssh_host_key, allow_paths, tls_note,
  ssh_authorized_keys, runtime_bind, runtime_forge_listen,
  runtime_ssh_listen, runtime_synced_at, runtime_probe_*, runtime_apply_op.
Partial: file store seeds `arks serve` defaults; clap flags still win;
  `serve` / `config sync|apply|reload` write runtime_* + runtime_probe_* live-bind honesty;
  validate probes bind/forge_listen/ssh_listen. Design ≠ Claimed sync e2e /
  running-server hot-reload.
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
              <base-path> [more-base-paths…]

Default listen 127.0.0.1:9418 (IANA/Git-documented git:// port; normative).
Requires git-daemon-export-ok unless --export-all.
Multiple positionals / repeated --base-path = allow-list (Partial).
Wired Partial: timeouts, max-connections, verbose, reuseaddr (platform Partial),
  enable/disable upload-pack/receive-pack; upload-archive ACK+sideband-64k
  band-1 data + band-2 progress Partial (--format beyond tar Missing; --raw-ustar
  opt-out); --inetd Partial; --pid-file writes pid; --detach Partial re-exec
  (unix setsid / win DETACHED_PROCESS; parent may write --pid-file).
Fail-closed Missing (honest stubs): --user/--group --syslog --access-hook
  --interpolated-path --user-path overrides; Claimed archive path-filter/format parity.
  upload-archive path filters (--prefix/--exclude) refuse closed (Partial).
Never spawns git/gh. Design ≠ Claimed for full flag parity.
"
    );
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
