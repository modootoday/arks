//! `git web--browse`: open URLs or files in the configured web browser.
//!
//! Primary: <https://git-scm.com/docs/git-web--browse> (supported browser
//! names, `-b` / `-t` / `-c <conf.var>`, `web.browser`, `browser.<tool>.path`
//! and `browser.<tool>.cmd`, the konqueror note). Graphical browsers are
//! started in the background; text-mode browsers and openers run in the
//! foreground. Custom commands are evaluated by the shell with the URLs as
//! arguments.

use super::proc;
use crate::cmd::opt_parse::{OptDef, Takes::*, o, parse};
use crate::error::{CliError, CliResult};
use arks_config::ConfigSet;
use std::process::Stdio;

const OPTS: &[OptDef] = &[
    o("browser", 'b', Req, false),
    o("tool", 't', Req, false),
    o("config", 'c', Req, false),
];

const USAGE: &str = "usage: git web--browse [<options>] <url|file>...";

/// The browsers git documents, in no particular order.
const KNOWN: &[&str] = &[
    "firefox",
    "iceweasel",
    "seamonkey",
    "iceape",
    "chromium",
    "chromium-browser",
    "google-chrome",
    "chrome",
    "konqueror",
    "opera",
    "w3m",
    "elinks",
    "links",
    "lynx",
    "dillo",
    "open",
    "start",
    "cygstart",
    "xdg-open",
];

/// Browsers that keep running after the page is shown (started in the background).
fn background(b: &str) -> bool {
    matches!(
        b,
        "firefox"
            | "iceweasel"
            | "seamonkey"
            | "iceape"
            | "chromium"
            | "chromium-browser"
            | "google-chrome"
            | "chrome"
            | "konqueror"
            | "opera"
            | "dillo"
    )
}

pub fn cmd_web_browse(args: &[String]) -> CliResult<()> {
    let p = parse("web--browse", args, OPTS, Default::default())
        .map_err(|_| CliError::Exit(1, USAGE.into()))?;
    let mut urls = p.args();
    urls.extend(p.after_dashdash());
    if urls.is_empty() {
        return Err(CliError::Exit(1, USAGE.into()));
    }
    let browser = p.value("browser").or_else(|| p.value("tool"));
    let repo = crate::util::open_repo().ok();
    let cfg = ConfigSet::load(repo.as_ref().map(|r| r.common_dir()));
    browse(&cfg, browser, p.value("config").as_deref(), &urls)
}

/// Pick the browser (option, `<conf.var>`, `web.browser`, then a guess) and
/// show `urls`. Shared with `instaweb`.
pub fn browse(
    cfg: &ConfigSet,
    browser: Option<String>,
    conf_var: Option<&str>,
    urls: &[String],
) -> CliResult<()> {
    let chosen = browser
        .or_else(|| conf_var.and_then(|k| cfg.get(k)))
        .or_else(|| cfg.get("web.browser"))
        .filter(|b| !b.is_empty());
    let browser = match chosen {
        Some(b) => b,
        None => {
            guess(cfg).ok_or_else(|| CliError::Exit(1, "No known browser available.".into()))?
        }
    };
    if !KNOWN.contains(&browser.as_str()) {
        let Some(cmd) = cfg.get(&format!("browser.{browser}.cmd")) else {
            return Err(CliError::Exit(
                1,
                format!("The browser {browser} is not available as '{browser}'."),
            ));
        };
        let c = proc::shell(&format!("{cmd} \"$@\""), &cmd, urls);
        let st = proc::run(c)?;
        return match proc::code_of(&st) {
            0 => Ok(()),
            n => Err(CliError::silent(n.clamp(1, 255) as u8)),
        };
    }
    let path = cfg
        .get(&format!("browser.{browser}.path"))
        .unwrap_or_else(|| browser.clone());
    if proc::which(&path).is_none() && browser != "start" {
        return Err(CliError::Exit(
            1,
            format!("The browser {browser} is not available as '{path}'."),
        ));
    }
    let mut cmd = match browser.as_str() {
        "firefox" | "iceweasel" | "seamonkey" | "iceape" => {
            let mut c = std::process::Command::new(&path);
            c.arg("-new-tab");
            c
        }
        "konqueror" => {
            // Open a tab in a running konqueror through kfmclient beside it.
            let kfm = match path.rsplit_once('/') {
                Some((dir, "konqueror")) => format!("{dir}/kfmclient"),
                _ if path == "konqueror" => "kfmclient".to_string(),
                _ => path.clone(),
            };
            let mut c = std::process::Command::new(&kfm);
            if kfm.ends_with("kfmclient") {
                c.arg("newTab");
            }
            c
        }
        "start" => {
            let mut c = std::process::Command::new("cmd");
            c.args(["/c", "start", ""]);
            c
        }
        _ => std::process::Command::new(&path),
    };
    cmd.args(urls);
    if let Some(p) = proc::config_parameters() {
        cmd.env("GIT_CONFIG_PARAMETERS", p);
    }
    if background(&browser) {
        cmd.stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        cmd.spawn()
            .map_err(|e| CliError::Exit(1, format!("cannot start {path}: {e}")))?;
        return Ok(());
    }
    let st = proc::run(cmd)?;
    match proc::code_of(&st) {
        0 => Ok(()),
        n => Err(CliError::silent(n.clamp(1, 255) as u8)),
    }
}

/// Default browser: graphical ones under a display (konqueror first under
/// KDE, `open` on macOS), text-mode ones otherwise.
fn guess(cfg: &ConfigSet) -> Option<String> {
    let env = |k: &str| std::env::var(k).ok().filter(|v| !v.is_empty());
    let mut cands: Vec<&str> = Vec::new();
    if env("SECURITYSESSIONID").is_some()
        || env("TERM_PROGRAM").is_some() && cfg!(target_os = "macos")
    {
        cands.push("open");
    }
    if cfg!(windows) {
        cands.push("start");
    }
    if env("DISPLAY").is_some() {
        if env("KDE_FULL_SESSION").as_deref() == Some("true") {
            cands.push("konqueror");
        }
        cands.extend([
            "firefox",
            "iceweasel",
            "google-chrome",
            "chrome",
            "chromium",
            "chromium-browser",
            "konqueror",
            "opera",
            "seamonkey",
            "iceape",
            "w3m",
            "elinks",
            "links",
            "lynx",
            "dillo",
            "xdg-open",
        ]);
    } else {
        cands.extend(["w3m", "elinks", "links", "lynx"]);
    }
    cands.into_iter().find_map(|b| {
        let path = cfg
            .get(&format!("browser.{b}.path"))
            .unwrap_or_else(|| b.to_string());
        proc::which(&path).map(|_| b.to_string())
    })
}
