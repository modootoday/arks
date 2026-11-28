//! Wave Sv / Sv2 server porcelain (upload-pack / receive-pack / HTTP / daemon).
//!
//! Cite: `.spec/sot/cli-surface.sot.md` Wave Sv, `.spec/sot/arks-server.sot.md`,
//! `.spec/sot/arks-self-host-server.sot.md`
//! Forge-in-serve: Partial local API via `arks-hub` (design ≠ Claimed).
//! No CELIR. Lease not required. No `gix`.

use crate::cmd::opt_parse::{Mode, OptDef, Takes::*, o, parse};
use crate::error::{CliError, CliResult};
use arks_hub::{ForgeServeOptions, serve_forge_api, spawn_forge_api};
use arks_repo::Repository;
use arks_server::{
    BasicAuthConfig, HttpRequest, ListenRebindSlot, ProductTlsServerConfig, ServeHttpOptions,
    ServeSshOptions, default_authorized_keys_path, default_host_key_path, handle_smart_http_auth,
    load_server_config, receive_pack_session, serve_http_scheme, serve_http_with_opts,
    spawn_loopback_ssh, spawn_ssh_listener, update_server_info, upload_pack_session,
};
use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

/// Background SSH listener thread handle.
type SshListener = std::thread::JoinHandle<Result<(), arks_server::ServerError>>;
/// Background forge listener thread handle.
type ForgeListener = std::thread::JoinHandle<Result<(), String>>;

const UPLOAD_PACK_OPTS: &[OptDef] = &[
    o("strict", ' ', No, true),
    o("timeout", ' ', Req, true),
    o("stateless-rpc", ' ', No, true),
    o("advertise-refs", ' ', No, true),
    o("http-backend-info-refs", ' ', No, true),
];

const RECEIVE_PACK_OPTS: &[OptDef] = &[
    o("quiet", 'q', No, true),
    o("stateless-rpc", ' ', No, true),
    o("advertise-refs", ' ', No, true),
    o("http-backend-info-refs", ' ', No, true),
    o("reject-thin-pack-for-testing", ' ', No, true),
];

/// Open the repository a server helper names: `--strict` takes the path as
/// the git directory itself; otherwise `<dir>/.git` and `<dir>.git` are
/// tried too (git-upload-pack OPTIONS).
fn server_repo(p: &crate::cmd::opt_parse::Parsed, usage: &str) -> CliResult<Repository> {
    let operands = p.operands();
    let [dir] = operands.as_slice() else {
        return Err(CliError::Exit(129, usage.into()));
    };
    let path = PathBuf::from(dir);
    let found = if p.on("strict") {
        Repository::open(&path).ok()
    } else {
        Repository::open(&path)
            .or_else(|_| Repository::open(path.join(".git")))
            .or_else(|_| Repository::open(format!("{dir}.git")))
            .ok()
    };
    found.ok_or_else(|| CliError::fatal(format!("'{dir}' does not appear to be a git repository")))
}

/// Reader that remembers whether the client sent anything at all.
struct SawInput<R> {
    inner: R,
    any: bool,
}

impl<R: Read> Read for SawInput<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.inner.read(buf)?;
        self.any |= n > 0;
        Ok(n)
    }
}

fn hung_up() -> CliError {
    CliError::fatal("the remote end hung up unexpectedly")
}

/// `upload-pack [--[no-]strict] [--timeout=<n>] [--stateless-rpc]
/// [--advertise-refs] <directory>`. Cite: <https://git-scm.com/docs/git-upload-pack>.
pub fn cmd_upload_pack(args: &[String]) -> CliResult<()> {
    let p = parse("upload-pack", args, UPLOAD_PACK_OPTS, Mode::default())?;
    let repo = server_repo(
        &p,
        "usage: git upload-pack [--[no-]strict] [--timeout=<n>] [--stateless-rpc]\n                       [--advertise-refs] <directory>",
    )?;
    if let Some(t) = p.value("timeout") {
        t.parse::<u64>().map_err(|_| {
            CliError::Exit(
                129,
                "error: option `timeout' expects a numerical value".into(),
            )
        })?;
    }
    let mut out = io::stdout().lock();
    if p.on("advertise-refs") || p.on("http-backend-info-refs") {
        out.write_all(&arks_server::upload_pack_advertisement(&repo).map_err(server_err)?)?;
        out.flush()?;
        return Ok(());
    }
    if p.on("stateless-rpc") {
        // One request on stdin, one response on stdout, no advertisement.
        let mut body = Vec::new();
        io::stdin().read_to_end(&mut body)?;
        if body.is_empty() {
            return Err(hung_up());
        }
        let resp = arks_server::upload_pack_result(&repo, &body).map_err(server_err)?;
        out.write_all(&resp)?;
        out.flush()?;
        return Ok(());
    }
    let mut input = SawInput {
        inner: io::stdin(),
        any: false,
    };
    // Duplex pack-protocol (SSH command-mode / local --upload-pack).
    upload_pack_session(&repo, &mut input, &mut out).map_err(server_err)?;
    if !input.any {
        return Err(hung_up());
    }
    Ok(())
}

/// `receive-pack [-q] [--stateless-rpc] [--advertise-refs] <git-dir>`.
/// Cite: <https://git-scm.com/docs/git-receive-pack>.
pub fn cmd_receive_pack(args: &[String]) -> CliResult<()> {
    let p = parse("receive-pack", args, RECEIVE_PACK_OPTS, Mode::default())?;
    let mut repo = server_repo(&p, "usage: git receive-pack <git-dir>")?;
    let mut out = io::stdout().lock();
    if p.on("advertise-refs") || p.on("http-backend-info-refs") {
        out.write_all(&arks_server::receive_pack_advertisement(&repo).map_err(server_err)?)?;
        out.flush()?;
        return Ok(());
    }
    if p.on("stateless-rpc") {
        let mut body = Vec::new();
        io::stdin().read_to_end(&mut body)?;
        if body.is_empty() {
            return Err(hung_up());
        }
        let resp = arks_server::receive_pack(&mut repo, &body).map_err(server_err)?;
        out.write_all(&resp)?;
        out.flush()?;
        return Ok(());
    }
    let mut input = SawInput {
        inner: io::stdin(),
        any: false,
    };
    receive_pack_session(&mut repo, &mut input, &mut out).map_err(server_err)?;
    if !input.any {
        return Err(hung_up());
    }
    Ok(())
}

fn server_err(e: arks_server::ServerError) -> CliError {
    CliError::Msg(e.to_string())
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
    let mut forge_git_root: Option<PathBuf> = file_cfg.get("forge_git_root").map(PathBuf::from);
    let mut forge_git_url: Option<String> = file_cfg.get("forge_git_url").cloned();
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
            "--forge-git-root" => {
                i += 1;
                forge_git_root =
                    Some(PathBuf::from(args.get(i).ok_or_else(|| {
                        CliError::Usage("serve --forge-git-root needs DIR".into())
                    })?));
            }
            "--forge-git-url" => {
                i += 1;
                forge_git_url = Some(
                    args.get(i)
                        .ok_or_else(|| CliError::Usage("serve --forge-git-url needs URL".into()))?
                        .clone(),
                );
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
            other if other.starts_with("--forge-git-root=") => {
                forge_git_root = Some(PathBuf::from(&other[17..]));
            }
            other if other.starts_with("--forge-git-url=") => {
                forge_git_url = Some(other[16..].to_string());
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
        Some(resolve_forge_opts(
            &forge_listen,
            forge_data,
            forge_git_root,
            forge_git_url,
        )?)
    } else {
        None
    };
    if let Some(ref mut opts) = forge_opts {
        opts.tls = product_tls.clone();
        // Forge rulesets gate pushes to the HTTP and SSH listeners of this process.
        crate::cmd::serve_rulesets::install(opts);
    }

    sync_serve_runtime_to_file(
        enable_http.then_some(http_bind_for_sync.as_str()),
        enable_forge.then_some(forge_listen.as_str()),
        enable_ssh.then_some(ssh_listen_cfg.as_str()),
    );

    // IW-FORGE-SERVE Partial: default cohost must not silent-cut forge/HTTP when SSH is on.
    // Background modules + main blocker; bind failure fail-closed (SoT §17).
    let mut ssh_join: Option<(Arc<AtomicBool>, SshListener)> = None;
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

    let mut forge_join: Option<(Arc<AtomicBool>, ForgeListener)> = None;
    if let Some(opts) = forge_opts.as_mut() {
        if !enable_http && !enable_ssh {
            // Forge-only: block on main.
            let forge_slot = register_live_forge_rebind_slot();
            opts.rebind_desired = Some(forge_slot);
            install_serve_config_sighup_partial("arks serve forge-only");
            let stop = Arc::new(AtomicBool::new(false));
            let result = serve_forge_api(opts.clone(), stop).map_err(CliError::Msg);
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
            (true, true) => "arks serve (HTTP + SSH + forge)",
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
            "arks serve: SSH primary listener (forge cohost={})",
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
    forge_git_root: Option<PathBuf>,
    forge_git_url: Option<String>,
) -> CliResult<ForgeServeOptions> {
    let listen: SocketAddr = forge_listen
        .parse()
        .map_err(|e| CliError::Usage(format!("bad --forge-listen address: {e}")))?;
    Ok(ForgeServeOptions {
        listen,
        data_dir: forge_data.unwrap_or_else(arks_hub::default_forge_data_dir),
        rebind_desired: None,
        tls: None,
        git_root: forge_git_root,
        git_url: forge_git_url,
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
            s if s.starts_with("--tls-cert=") => tls_cert = Some(PathBuf::from(&s[11..])),
            "--tls-key" => {
                i += 1;
                tls_key =
                    Some(PathBuf::from(args.get(i).ok_or_else(|| {
                        CliError::Usage("serve --tls-key needs PATH".into())
                    })?));
            }
            s if s.starts_with("--tls-key=") => tls_key = Some(PathBuf::from(&s[10..])),
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
    eprintln!(
        "arks serve config watch: Partial file-watch path={} duration={duration_secs}s \
         interval={interval_ms}ms (no running-server signal; Claimed sync e2e Missing)",
        path.display()
    );
    // Initial apply so watch always leaves runtime_* stamped.
    serve_config_apply_reload("watch-initial", /*seed_defaults=*/ false)?;
    // Baseline after the initial apply so its own write is not seen as a change.
    let mut last_mtime = std::fs::metadata(&path)
        .ok()
        .and_then(|m| m.modified().ok());
    while start.elapsed() < std::time::Duration::from_secs(duration_secs.max(1)) {
        std::thread::sleep(std::time::Duration::from_millis(interval_ms.max(50)));
        let mtime = std::fs::metadata(&path)
            .ok()
            .and_then(|m| m.modified().ok());
        if mtime != last_mtime {
            eprintln!("arks serve config watch: mtime change detected → apply/reload Partial");
            let mut map = load_serve_config_map()?;
            map.insert("runtime_watch_mtime".into(), format!("{:?}", mtime));
            map.insert(
                "runtime_watch_signal".into(),
                "file-mtime Partial (unix SIGHUP Partial on running serve; Windows Missing)".into(),
            );
            save_serve_config_map(&map)?;
            serve_config_apply_reload("watch-reload", /*seed_defaults=*/ false)?;
            // The reload rewrote the file; do not treat that write as a new change.
            last_mtime = std::fs::metadata(&path)
                .ok()
                .and_then(|m| m.modified().ok());
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
    let mut disable_lfs = false;
    let mut gitweb = false;
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
            "--no-lfs" => disable_lfs = true,
            "--gitweb" => gitweb = true,
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
    let pages = if gitweb {
        Some(crate::cmd::bridge::instaweb::gitweb::page_handler(&git_dir))
    } else {
        None
    };
    let mut http_opts = ServeHttpOptions {
        dumb_only,
        basic_auth,
        rebind_desired: None,
        tls,
        pages,
        disable_lfs,
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

/// Arm the serve-config reload trigger for a running `arks serve`.
///
/// Unix: SIGHUP → `serve config` apply/reload (runtime_* probes + live HTTP/forge/SSH
/// rebind with dual-bind drain, B7). Windows has no SIGHUP (B8 N/A): an in-process
/// watcher polls the serve-config file mtime and runs the same apply/reload.
fn install_serve_config_sighup_partial(label: &str) {
    #[cfg(unix)]
    {
        use std::sync::atomic::AtomicBool as Flag;
        static INSTALLED: Flag = Flag::new(false);
        static RELOAD: Flag = Flag::new(false);
        if INSTALLED.swap(true, Ordering::SeqCst) {
            eprintln!("{label}: SIGHUP serve-config reload already armed");
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
                    "{label}: SIGHUP install failed — serve config signal reload unavailable"
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
                    eprintln!("arks serve: SIGHUP → serve config apply/reload");
                    serve_config_triggered_reload("sighup", "SIGHUP (unix)");
                }
            }
        });
        eprintln!("{label}: SIGHUP serve-config reload + live HTTP/forge/SSH rebind armed (unix)");
    }
    #[cfg(not(unix))]
    {
        use std::sync::atomic::AtomicBool as Flag;
        static INSTALLED: Flag = Flag::new(false);
        if INSTALLED.swap(true, Ordering::SeqCst) {
            eprintln!("{label}: serve-config file-mtime reload already armed");
            return;
        }
        eprintln!(
            "{label}: serve-config file-mtime reload + live HTTP/forge/SSH rebind armed \
             (Windows; SIGHUP N/A)"
        );
        // Baseline before the thread starts, so a write right after arming is seen.
        let path = serve_config_path();
        let mut last_mtime = serve_config_mtime(&path);
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(Duration::from_millis(500));
                let cur = serve_config_mtime(&path);
                if cur != last_mtime && cur.is_some() {
                    eprintln!("arks serve: serve-config file-mtime change → apply/reload");
                    serve_config_triggered_reload("win-mtime", "file-mtime (Windows; SIGHUP N/A)");
                    // The reload rewrites the config; take that write as the new
                    // baseline so the watcher does not re-trigger on it.
                    last_mtime = serve_config_mtime(&path);
                }
            }
        });
    }
}

/// Modification time of the serve-config file, if it exists.
#[cfg(not(unix))]
fn serve_config_mtime(path: &std::path::Path) -> Option<std::time::SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

/// Shared body of a signal- or mtime-triggered reload inside a running serve.
fn serve_config_triggered_reload(label: &str, signal: &str) {
    let mut map = match load_serve_config_map() {
        Ok(m) => m,
        Err(e) => {
            eprintln!("arks serve: {label} reload load err: {e}");
            return;
        }
    };
    map.insert("runtime_watch_signal".into(), signal.into());
    let _ = save_serve_config_map(&map);
    if let Err(e) = serve_config_apply_reload(label, /*seed_defaults=*/ false) {
        eprintln!("arks serve: {label} apply/reload err: {e}");
    }
}

/// `arks serve ssh-smoke` — SSH banner diagnostic against a listener, or (`--git-pack`)
/// against an ephemeral in-process listener. A green smoke is not the B1/B2 proof
/// (that is the Docker receipt); never spawns ssh/git/gh.
fn cmd_serve_ssh_smoke(args: &[String]) -> CliResult<()> {
    if args.iter().any(|a| a == "-h" || a == "--help") {
        eprintln!(
            "arks serve ssh-smoke [--listen HOST:PORT] [--git-pack]\n  \
             SSH banner check against serve ssh_listen (or --listen); exit 0 only when an \
             SSH-2.0 banner arrives.\n  \
             --git-pack: start an ephemeral in-process SSH listener, exchange \
             identification and read its KEXINIT.\n  \
             Diagnostic only: B1/B2 (OpenSSH wire, git clone ssh://) are proven by Docker \
             receipts, not by this smoke. Never spawns git/gh/ssh CLI."
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

    println!("ssh_smoke=banner");
    println!("listen={addr}");
    println!("spawns_ssh_cli=never");
    println!("ssh_git_e2e=not_checked_by_smoke (B1/B2 Docker receipt)");

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
                        Ok(())
                    } else {
                        println!("ssh_banner=unexpected");
                        Err(CliError::Msg(
                            "arks serve ssh-smoke: banner not SSH-2.0-*".into(),
                        ))
                    }
                }
                Ok(_) | Err(_) => {
                    println!("tcp_connect=ok");
                    println!("ssh_banner=none");
                    Err(CliError::Msg(format!(
                        "arks serve ssh-smoke: connected to {addr} but no SSH banner within 2s \
                         (is `arks serve` running with SSH enabled?)"
                    )))
                }
            }
        }
        Err(e) => {
            println!("tcp_connect=fail");
            println!("ssh_banner=none");
            println!("error={e}");
            Err(CliError::Msg(format!(
                "arks serve ssh-smoke: connect fail ({e})"
            )))
        }
    }
}

/// Ephemeral in-process SSH listener + native client identification exchange and
/// KEXINIT read. Diagnostic only (B1/B2 proof is the Docker receipt); never spawns
/// ssh/git/gh.
fn cmd_serve_ssh_git_pack_smoke() -> CliResult<()> {
    println!("ssh_smoke=git-pack");
    println!("spawns_ssh_cli=never");
    println!("ssh_git_e2e=not_checked_by_smoke (B1/B2 Docker receipt)");

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
                "git-pack smoke: listener banner not SSH-2.0-*".into(),
            ));
        }
        println!("ssh_banner=ok");
        // Client identification (RFC 4253) so server advances past empty-peer path.
        stream
            .write_all(b"SSH-2.0-arks_ssh_smoke\r\n")
            .map_err(|e| CliError::Msg(format!("git-pack smoke id write: {e}")))?;
        println!("client_id_exchange=sent");
        // RFC 4253 §7.1: the server's first binary packet is SSH_MSG_KEXINIT (20):
        // uint32 packet_length, byte padding_length, then the message number.
        let mut wire: Vec<u8> = match banner.find("\r\n") {
            Some(end) => buf[end + 2..n].to_vec(),
            None => Vec::new(),
        };
        let mut more = [0u8; 512];
        while wire.len() < 6 {
            match stream.read(&mut more) {
                Ok(m) if m > 0 => wire.extend_from_slice(&more[..m]),
                _ => break,
            }
        }
        println!("server_followup_bytes={}", wire.len());
        if wire.len() < 6 || wire[5] != 20 {
            println!("server_kexinit=none");
            return Err(CliError::Msg(
                "git-pack smoke: no SSH_MSG_KEXINIT from the listener within 2s".into(),
            ));
        }
        println!("server_kexinit=ok");
        println!("git_over_ssh_smoke=ok");
        Ok(())
    })();

    stop.store(true, Ordering::SeqCst);
    match handle.join() {
        Ok(Ok(())) => println!("durable_stop=ok"),
        Ok(Err(e)) => eprintln!("note: ephemeral SSH listener stop err: {e}"),
        Err(_) => eprintln!("note: ephemeral SSH listener join panic"),
    }
    client
}

/// `http-backend`: CGI program. Stock git takes no arguments and finds the
/// repository from `GIT_PROJECT_ROOT` + `PATH_INFO` (or `PATH_TRANSLATED`);
/// arks also accepts the repository directory as one argument.
/// Cite: <https://git-scm.com/docs/git-http-backend>.
pub fn cmd_http_backend(args: &[String]) -> CliResult<()> {
    let path_info = std::env::var("PATH_INFO").unwrap_or_else(|_| "/info/refs".into());
    let (dir, path) = if args.is_empty() {
        cgi_repo_and_path(&path_info)?
    } else {
        (require_dir(args)?, path_info)
    };
    let mut repo = Repository::open(&dir).or_else(|_| Repository::discover(&dir))?;
    let method = std::env::var("REQUEST_METHOD").unwrap_or_else(|_| "GET".into());
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

/// Split a CGI request into the repository directory and the service path
/// (`/info/refs`, `/git-upload-pack`, `/HEAD`, `/objects/...`).
fn cgi_repo_and_path(path_info: &str) -> CliResult<(PathBuf, String)> {
    const SERVICES: &[&str] = &[
        "/info/refs",
        "/git-upload-pack",
        "/git-receive-pack",
        "/HEAD",
        "/objects/",
    ];
    let cut = SERVICES
        .iter()
        .filter_map(|s| path_info.rfind(s))
        .min()
        .ok_or_else(|| CliError::fatal(format!("Request not supported: '{path_info}'")))?;
    let (repo_part, service) = path_info.split_at(cut);
    let dir = match std::env::var("GIT_PROJECT_ROOT") {
        Ok(root) => PathBuf::from(root).join(repo_part.trim_start_matches('/')),
        Err(_) => {
            let translated = std::env::var("PATH_TRANSLATED").map_err(|_| {
                CliError::fatal("No GIT_PROJECT_ROOT or PATH_TRANSLATED from server")
            })?;
            let base = translated
                .strip_suffix(service)
                .ok_or_else(|| CliError::fatal("PATH_TRANSLATED does not end in PATH_INFO"))?;
            PathBuf::from(base)
        }
    };
    let exported = std::env::var_os("GIT_HTTP_EXPORT_ALL").is_some()
        || dir.join("git-daemon-export-ok").is_file()
        || dir.join(".git").join("git-daemon-export-ok").is_file();
    if !exported {
        return Err(CliError::fatal(format!(
            "Repository not exported: '{}'",
            dir.display()
        )));
    }
    Ok((dir, service.to_string()))
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

/// `update-server-info [-f | --force]` in the current repository (arks also
/// takes an optional repository path). Cite: <https://git-scm.com/docs/git-update-server-info>.
pub fn cmd_update_server_info(args: &[String]) -> CliResult<()> {
    let p = parse(
        "update-server-info",
        args,
        &[o("force", 'f', No, true)],
        Mode::default(),
    )?;
    let operands = p.operands();
    let dir = match operands.as_slice() {
        [] => std::env::current_dir()?,
        [d] => PathBuf::from(d),
        _ => {
            return Err(CliError::Exit(
                129,
                "usage: git update-server-info [-f | --force]".into(),
            ));
        }
    };
    let repo = Repository::open(&dir).or_else(|_| Repository::discover(&dir))?;
    // FS5 / MAINT-004 closest: refuse under info lock rather than racing writers.
    let info_lock = repo.git_dir().join("info").join("refs.lock");
    if info_lock.is_file() {
        return Err(CliError::Msg(format!(
            "{} (MAINT-004 — update-server-info race refuse while info/refs.lock held; lease ≠ skip)",
            info_lock.display()
        )));
    }
    // The files are always rewritten in full, so --force changes nothing.
    update_server_info(&repo).map_err(|e| CliError::Msg(e.to_string()))
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

Status ([[sot:root/arks-self-host-server]] §17; receipts under .spec/evidence/):
  - HTTP smart(+dumb) listener: wire Claimed (G15/G29/G36 when configured)
  - In-process SSH listener: durable accept + live rebind; OpenSSH ChaCha wire and
    stock `git clone ssh://` Claimed (B1/B2); bind failure fail-closed; ops SSHD +
    upload-pack/receive-pack also valid
  - Forge API in serve: PR/issue hosting e2e Claimed (IW-FORGE-SERVE; separate bind)
  - In-process TLS (--tls-cert/--tls-key): Claimed (B9); an ops TLS front is also valid
  - Hub OAuth is refused as primary git-remote auth
  - `serve config` seeds + runtime_* / apply|reload|watch; live HTTP/forge/SSH rebind with
    dual-bind drain Claimed (B7); reload trigger: SIGHUP on unix, serve-config file-mtime
    on Windows (no SIGHUP there); clap wins; ≠ config/hub
  - `serve ssh-smoke` / `--git-pack`: SSH banner / self-listener diagnostic (≠ B1/B2 proof)
  - Optional: --ssh-host-key PATH --authorized-keys PATH

Ports ([[sot:root/arks-self-host-server]] / [[research:root/20261111055609]]):
  Normative targets (clone URLs / dedicated host): SSH 22, HTTP 80, HTTPS 443, git:// 9418
  Unprivileged loopback demo binds (current process defaults): HTTP 127.0.0.1:8080;
    SSH 127.0.0.1:2222 — privilege / host-sshd workarounds, NOT more-standard Git ports
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

  --gitweb   also serve the repository browser pages `git instaweb` shows
             (gitweb actions at `/`); git HTTP paths are unchanged
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
