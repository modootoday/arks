//! `git instaweb`: start, stop or restart a local web server for the
//! repository and open it in a browser.
//!
//! Primary: <https://git-scm.com/docs/git-instaweb> (`--local`, `--httpd`,
//! `--module-path`, `--port` (default 1234), `--browser`, `--start`,
//! `--stop`, `--restart`, the `instaweb.*` variables and the `web.browser`
//! fallback). arks serves the repository with its own HTTP server
//! (`arks serve-http`: the repository over git HTTP, no gitweb pages) instead
//! of gitweb under an external httpd;
//! the `--httpd` names git supports are accepted and recorded, and the
//! server's process id is kept in `$GIT_DIR/pid` as git does.

use crate::cmd::opt_parse::{OptDef, Takes::*, o, parse};
use crate::error::{CliError, CliResult};
use crate::util::open_repo;
use arks_config::ConfigSet;
use std::fs;
use std::path::Path;
use std::process::Stdio;

const OPTS: &[OptDef] = &[
    o("local", 'l', No, true),
    o("port", 'p', Req, true),
    o("httpd", 'd', Req, true),
    o("browser", 'b', Req, true),
    o("module-path", 'm', Req, true),
    o("stop", ' ', No, true),
    o("start", ' ', No, true),
    o("restart", ' ', No, true),
];

const USAGE: &str = "usage: git instaweb [options] (--start | --stop | --restart)

    -l, --local           only bind on 127.0.0.1
    -p, --port ...        the port to bind to
    -d, --httpd ...       the command to launch
    -b, --browser ...     the browser to launch
    -m, --module-path ...
                          the module path (only needed for apache2)

Action
    --stop                stop the web server
    --start               start the web server
    --restart             restart the web server
";

const HTTPDS: &[&str] = &[
    "lighttpd", "apache2", "mongoose", "plackup", "python", "webrick",
];

pub fn cmd_instaweb(args: &[String]) -> CliResult<()> {
    if args.iter().any(|a| a == "-h") {
        print!("{USAGE}");
        return Err(CliError::silent(129));
    }
    let p = parse("instaweb", args, OPTS, Default::default()).map_err(|e| match e {
        CliError::Exit(129, m) => {
            let first = m.lines().next().unwrap_or("").to_string();
            CliError::Exit(129, format!("{first}\n{}", USAGE.trim_end()))
        }
        other => other,
    })?;
    // `start`, `stop` and `restart` also work as words.
    let mut action = None;
    for a in p.args() {
        match a.as_str() {
            "start" | "stop" | "restart" => action = Some(a),
            _ => return Err(CliError::Exit(129, USAGE.trim_end().to_string())),
        }
    }
    for k in ["start", "stop", "restart"] {
        if p.on(k) {
            action = Some(k.to_string());
        }
    }
    let repo = open_repo()?;
    let cfg = ConfigSet::load(Some(repo.common_dir()));
    let git_dir = fs::canonicalize(repo.git_dir()).unwrap_or_else(|_| repo.git_dir().to_path_buf());
    let pid_file = git_dir.join("pid");
    match action.as_deref() {
        Some("stop") => {
            stop(&pid_file);
            return Ok(());
        }
        Some("restart") => stop(&pid_file),
        _ => {}
    }
    let local = p
        .flag("local")
        .or_else(|| cfg.get_bool("instaweb.local"))
        .unwrap_or(false);
    let port = p
        .value("port")
        .or_else(|| cfg.get("instaweb.port"))
        .unwrap_or_else(|| "1234".into());
    let port: u16 = port
        .parse()
        .map_err(|_| CliError::Exit(1, format!("invalid port: {port}")))?;
    let httpd = p
        .value("httpd")
        .or_else(|| cfg.get("instaweb.httpd"))
        .unwrap_or_else(|| "lighttpd".into());
    let httpd_name = httpd.split_whitespace().next().unwrap_or("");
    let base = httpd_name.rsplit('/').next().unwrap_or(httpd_name);
    if !HTTPDS.iter().any(|h| base.starts_with(h)) {
        return Err(CliError::Exit(
            1,
            format!("Unknown httpd specified: {httpd_name}"),
        ));
    }
    let module_path = p
        .value("module-path")
        .or_else(|| cfg.get("instaweb.modulePath"));
    stop(&pid_file);
    let bind = if local { "127.0.0.1" } else { "0.0.0.0" };
    start(
        &git_dir,
        &pid_file,
        bind,
        port,
        &httpd,
        module_path.as_deref(),
    )?;
    if action.is_some() {
        return Ok(());
    }
    let url = format!("http://127.0.0.1:{port}");
    let browser = p.value("browser");
    let conf_var = cfg
        .get("instaweb.browser")
        .map(|_| "instaweb.browser")
        .unwrap_or("web.browser");
    if super::web_browse::browse(&cfg, browser, Some(conf_var), std::slice::from_ref(&url)).is_err()
    {
        println!("{url}");
    }
    Ok(())
}

/// Record the server settings under `$GIT_DIR/gitweb` and start the
/// detached server, keeping its pid in `$GIT_DIR/pid`.
fn start(
    git_dir: &Path,
    pid_file: &Path,
    bind: &str,
    port: u16,
    httpd: &str,
    module_path: Option<&str>,
) -> CliResult<()> {
    let conf_dir = git_dir.join("gitweb");
    fs::create_dir_all(&conf_dir)?;
    let mut conf = format!("httpd = {httpd}\nlisten = {bind}:{port}\nserver = arks serve-http\n");
    if let Some(m) = module_path {
        conf.push_str(&format!("module-path = {m}\n"));
    }
    fs::write(conf_dir.join("instaweb.conf"), conf)?;
    let exe =
        std::env::current_exe().map_err(|e| CliError::fatal(format!("cannot locate arks: {e}")))?;
    let log = fs::File::create(conf_dir.join("instaweb.log"))?;
    let mut c = std::process::Command::new(exe);
    c.args(["serve-http", "--listen"])
        .arg(format!("{bind}:{port}"))
        .arg(git_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(log)
        .env("ARKS_PERSONALITY", "arks");
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        c.process_group(0);
    }
    let child = c
        .spawn()
        .map_err(|e| CliError::fatal(format!("cannot start the web server: {e}")))?;
    fs::write(pid_file, format!("{}\n", child.id()))?;
    // Give the listener a moment so the browser does not race it.
    let addr = format!("127.0.0.1:{port}");
    for _ in 0..50 {
        if std::net::TcpStream::connect(&addr).is_ok() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    Ok(())
}

/// Stop the server named by the pid file (nothing when there is none).
fn stop(pid_file: &Path) {
    let Ok(text) = fs::read_to_string(pid_file) else {
        return;
    };
    if let Ok(pid) = text.trim().parse::<i32>() {
        kill(pid);
    }
    let _ = fs::remove_file(pid_file);
}

#[cfg(unix)]
fn kill(pid: i32) {
    if pid > 0 {
        // SAFETY: plain kill(2) on a pid we recorded; no memory is shared.
        unsafe {
            libc::kill(pid, libc::SIGTERM);
        }
    }
}

#[cfg(not(unix))]
fn kill(pid: i32) {
    let _ = std::process::Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/F"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}
