//! `arks daemon` — `git://` daemon with `git daemon` argv and behaviour.
//!
//! Primary: <https://git-scm.com/docs/git-daemon>. Request handling lives in
//! `arks-server` (`daemon` module); this module owns argv, startup checks and
//! process control (`--inetd`, `--detach`, `--pid-file`, `--user`/`--group`).
//! arks-only: without `--listen`, bind `127.0.0.1` (git binds every
//! interface; use `--listen=0.0.0.0` for that). Never spawns `git` or `gh`.
//! Windows (matrix SoT §Windows): `--inetd` works with `stderr` as its implied
//! log destination; syslog, `--detach` and `--user` are refused.

mod argv;
mod os;

use crate::error::{CliError, CliResult};
use argv::DaemonArgs;
use arks_server::{
    DEFAULT_DAEMON_PORT, DaemonLog, DaemonOptions, DaemonOutcome, LogDestination, bind_daemon,
    serve_daemon, serve_daemon_inetd,
};
use std::io;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

/// Default listen address when no `--listen` is given (arks-only; see module docs).
const DEFAULT_LISTEN: &str = "127.0.0.1";

/// Log destination implied by `--inetd` / `--detach`: syslog like git, except
/// on Windows, which has no syslog (matrix SoT §Windows rule 2).
const IMPLIED_BACKGROUND_LOG: LogDestination = if cfg!(windows) {
    LogDestination::Stderr
} else {
    LogDestination::Syslog
};

const HELP: &str = "\
arks daemon — serve repositories over git:// (peer of git daemon)

    --verbose                 log connections and requests
    --syslog                  short for --log-destination=syslog
    --log-destination=(stderr|syslog|none)
    --export-all              serve repositories without git-daemon-export-ok
    --timeout=<n>             seconds per pack sub-request (0: none)
    --init-timeout=<n>        seconds until the request arrives (0: none)
    --max-connections=<n>     concurrent clients (default 32, 0: unlimited)
    --strict-paths            match repository paths exactly
    --base-path=<path>        prefix for every requested path
    --base-path-relaxed       retry the unprefixed path when lookup fails
    --user-path, --user-path=<path>
                              allow ~user requests (in <path> under the home)
    --interpolated-path=<pathtemplate>
                              %H %CH %IP %P %D %% virtual hosting template
    --reuseaddr               rebind the port promptly after a restart
    --pid-file=<file>         record the process id
    --enable=<service>        enable a service site-wide
    --disable=<service>       disable a service site-wide
    --allow-override=<service>
    --forbid-override=<service>
                              allow or forbid per-repository daemon.<service>
    --[no-]informative-errors explain refusals to clients
    --access-hook=<path>      ask <path> before serving each request
    --inetd                   serve one connection on stdin/stdout
    --listen=<host_or_ipaddr> listen address (repeatable; default 127.0.0.1)
    --port=<n>                listen port (default 9418)
    --detach                  run in the background
    --user=<user>, --group=<group>
                              drop privileges after binding
    <directory>...            serve only repositories under these directories

Services: upload-pack (on), upload-archive (off), receive-pack (off).
";

pub fn cmd_daemon(args: &[String]) -> CliResult<()> {
    let a = argv::parse(args)?;
    if a.help {
        print!("{HELP}\n{}\n", argv::USAGE);
        return Ok(());
    }
    let destination = a.log_destination.unwrap_or(if a.inetd || a.detach {
        IMPLIED_BACKGROUND_LOG
    } else {
        LogDestination::Stderr
    });
    let log = DaemonLog::new(destination, a.verbose);
    // git's die routine by destination: syslog err + exit 1, `none` silent exit 128.
    let die = |msg: String| -> CliError {
        match destination {
            LogDestination::Syslog => {
                log.error(&msg);
                CliError::Exit(1, String::new())
            }
            LogDestination::None => CliError::Exit(128, String::new()),
            LogDestination::Stderr => CliError::fatal(msg),
        }
    };
    #[cfg(windows)]
    if destination == LogDestination::Syslog {
        return Err(CliError::fatal("syslog is not supported on Windows"));
    }
    #[cfg(not(any(unix, windows)))]
    if destination == LogDestination::Syslog {
        return Err(CliError::fatal("syslog is not supported on this platform"));
    }
    if a.inetd && (a.port.is_some() || !a.listen.is_empty()) {
        return Err(die(
            "--listen= and --port= are incompatible with --inetd".into()
        ));
    }
    if a.inetd && (a.detach || a.user.is_some() || a.group.is_some()) {
        return Err(die(
            "--detach, --user and --group are incompatible with --inetd".into(),
        ));
    }
    let credentials = match (a.user.as_deref(), a.group.as_deref()) {
        (None, Some(_)) => return Err(die("--group supplied without --user".into())),
        (Some(user), group) => Some(os::resolve_credentials(user, group).map_err(die)?),
        (None, None) => None,
    };
    if a.strict_paths && a.directories.is_empty() {
        return Err(die(
            "option --strict-paths requires '<directory>' arguments".into(),
        ));
    }
    if let Some(base) = a.base_path.as_deref() {
        if !base.is_dir() {
            return Err(die(format!(
                "base-path '{}' does not exist or is not a directory",
                base.display()
            )));
        }
    }
    let opts = daemon_options(&a, log.clone());
    if a.inetd {
        let peer = os::inetd_peer();
        let outcome = serve_daemon_inetd(io::stdin().lock(), io::stdout().lock(), &opts, peer)
            .map_err(|e| die(e.to_string()))?;
        return match outcome {
            DaemonOutcome::Served => Ok(()),
            DaemonOutcome::Refused => Err(CliError::Exit(255, String::new())),
        };
    }
    if a.detach {
        os::detach().map_err(die)?;
    }
    if let Some(path) = a.pid_file.as_deref() {
        std::fs::write(path, format!("{}\n", std::process::id()))
            .map_err(|e| die(format!("could not write '{}': {e}", path.display())))?;
    }
    let hosts = if a.listen.is_empty() {
        vec![DEFAULT_LISTEN.to_string()]
    } else {
        a.listen.clone()
    };
    let port = a.port.unwrap_or(DEFAULT_DAEMON_PORT);
    let listeners = bind_daemon(&hosts, port, &opts).map_err(|e| die(e.to_string()))?;
    if let Some(c) = credentials.as_ref() {
        os::drop_privileges(c).map_err(die)?;
    }
    serve_daemon(listeners, opts, Arc::new(AtomicBool::new(false))).map_err(|e| die(e.to_string()))
}

fn daemon_options(a: &DaemonArgs, log: DaemonLog) -> DaemonOptions {
    let defaults = DaemonOptions::default();
    DaemonOptions {
        base_path: a.base_path.clone(),
        base_path_relaxed: a.base_path_relaxed,
        whitelist: a.directories.clone(),
        strict_paths: a.strict_paths,
        user_path: a.user_path.clone(),
        interpolated_path: a.interpolated_path.clone(),
        export_all: a.export_all,
        services: a.services,
        informative_errors: a.informative_errors,
        access_hook: a.access_hook.clone(),
        timeout_secs: a.timeout,
        init_timeout_secs: a.init_timeout,
        max_connections: a.max_connections.unwrap_or(defaults.max_connections),
        log,
        platform: Arc::new(os::LibcPlatform),
    }
}
