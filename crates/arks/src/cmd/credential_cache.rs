//! `credential-cache`, `credential-cache--daemon` and
//! `credential-helper-selector`.
//!
//! Cite: <https://git-scm.com/docs/git-credential-cache>,
//! <https://git-scm.com/docs/git-credential-cache--daemon>,
//! <https://git-scm.com/docs/gitcredentials>. The cache itself lives in
//! `arks_remote` (also used for `credential.helper = cache`).
//!
//! `credential-helper-selector` is a Git for Windows helper (pin 2.39.5 Linux
//! builds lack it) that lets the user pick the real helper and forwards the
//! request to it. arks asks on the terminal instead of a dialog, remembers an
//! "always" answer in `credential.helperselector.selected` (global config),
//! and does nothing when no choice is configured and no terminal is present.

use crate::cmd::opt_parse::{Mode, Takes, o, parse};
use crate::error::{CliError, CliResult};
use arks_remote::Credential;
use std::io::{Read, Write};
use std::path::PathBuf;

const CACHE_USAGE: &str = "usage: git credential-cache [<options>] <action>\n\n    --[no-]timeout <n>    number of seconds to cache credentials\n    --[no-]socket <path>  path of cache-daemon socket\n";
const DAEMON_USAGE: &str = "usage: git credential-cache--daemon [--debug] <socket-path>\n\n    --[no-]debug          print debugging messages to stderr\n";

fn help_on_stdout(usage: &str) -> CliError {
    print!("{usage}");
    let _ = std::io::stdout().flush();
    CliError::silent(129)
}

fn read_credential() -> CliResult<Credential> {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input)?;
    Ok(Credential::parse_attributes(&input))
}

/// `arks credential-cache [--timeout <n>] [--socket <path>] <get|store|erase|exit>`.
pub fn cmd_credential_cache(args: &[String]) -> CliResult<()> {
    if args.iter().any(|a| a == "-h") {
        return Err(help_on_stdout(CACHE_USAGE));
    }
    let defs = [
        o("timeout", ' ', Takes::Req, true),
        o("socket", ' ', Takes::Req, true),
    ];
    let p = parse("credential-cache", args, &defs, Mode::default())?;
    let positional = p.args();
    let Some(action) = positional.first() else {
        return Err(CliError::Exit(129, CACHE_USAGE.into()));
    };
    let timeout = match p.value("timeout") {
        Some(t) => t.parse::<u64>().map_err(|_| {
            CliError::Exit(
                129,
                format!("error: option `timeout' expects a numerical value\n{CACHE_USAGE}"),
            )
        })?,
        None => arks_remote::DEFAULT_CACHE_TIMEOUT,
    };
    let socket = p
        .value("socket")
        .map(|s| PathBuf::from(crate::cmd::plumbing_ext::expand_home(&s)))
        .unwrap_or_else(arks_remote::default_cache_socket);
    let desc = match action.as_str() {
        "get" | "store" | "erase" => read_credential()?,
        "exit" => Credential::default(),
        _ => return Ok(()),
    };
    let got = arks_remote::cache_client(action, &desc, &socket, timeout)
        .map_err(|e| CliError::fatal(e.to_string()))?;
    if let Some(c) = got {
        std::io::stdout().write_all(c.format_attrs().as_bytes())?;
    }
    Ok(())
}

/// `arks credential-cache--daemon [--debug] <socket-path>`.
pub fn cmd_credential_cache_daemon(args: &[String]) -> CliResult<()> {
    if args.iter().any(|a| a == "-h") {
        return Err(help_on_stdout(DAEMON_USAGE));
    }
    let defs = [o("debug", ' ', Takes::No, true)];
    let p = parse("credential-cache--daemon", args, &defs, Mode::default())?;
    let positional = p.args();
    let Some(socket) = positional.first() else {
        return Err(CliError::Exit(129, DAEMON_USAGE.into()));
    };
    arks_remote::run_cache_daemon(std::path::Path::new(socket), p.on("debug"))
        .map_err(|e| CliError::fatal(e.to_string()))
}

const SELECTED_KEY: &str = "credential.helperselector.selected";

/// Helpers offered by the selector on this platform.
#[cfg(unix)]
fn selector_choices() -> Vec<&'static str> {
    let mut v = vec!["manager"];
    if cfg!(windows) {
        v.push("wincred");
    } else if cfg!(target_os = "macos") {
        v.push("osxkeychain");
    } else {
        v.push("libsecret");
    }
    v.extend(["store", "cache"]);
    v
}

/// Ask on the terminal; `None` = no terminal or "no helper".
fn ask_on_terminal() -> CliResult<Option<(String, bool)>> {
    #[cfg(unix)]
    {
        let Ok(tty_in) = std::fs::File::open("/dev/tty") else {
            return Ok(None);
        };
        let Ok(mut tty_out) = std::fs::OpenOptions::new().write(true).open("/dev/tty") else {
            return Ok(None);
        };
        let choices = selector_choices();
        let mut menu = String::from("Select a credential helper:\n");
        for (i, c) in choices.iter().enumerate() {
            menu.push_str(&format!("  {}) {c}\n", i + 1));
        }
        menu.push_str(&format!(
            "  {}) <no helper>\nHelper [1]: ",
            choices.len() + 1
        ));
        tty_out.write_all(menu.as_bytes())?;
        use std::io::BufRead;
        let mut reader = std::io::BufReader::new(tty_in);
        let mut line = String::new();
        reader.read_line(&mut line)?;
        let pick = line.trim();
        let idx = if pick.is_empty() {
            1
        } else {
            pick.parse::<usize>().unwrap_or(0)
        };
        if idx == 0 || idx > choices.len() {
            return Ok(None);
        }
        tty_out.write_all(b"Always use this from now on? [y/N]: ")?;
        let mut always = String::new();
        reader.read_line(&mut always)?;
        let always = matches!(always.trim(), "y" | "Y" | "yes");
        Ok(Some((choices[idx - 1].to_string(), always)))
    }
    #[cfg(not(unix))]
    {
        Ok(None)
    }
}

/// `arks credential-helper-selector <get|store|erase>`.
pub fn cmd_credential_helper_selector(args: &[String]) -> CliResult<()> {
    const USAGE: &str = "usage: git credential-helper-selector <get|store|erase>";
    let Some(action) = args.first() else {
        return Err(CliError::Exit(129, USAGE.into()));
    };
    if action == "-h" || action == "--help" {
        println!("{USAGE}");
        return Err(CliError::silent(129));
    }
    if !matches!(action.as_str(), "get" | "store" | "erase") {
        return Ok(());
    }
    let desc = read_credential()?;
    let cfg = match crate::util::open_repo() {
        Ok(repo) => arks_config::ConfigSet::load(Some(repo.common_dir())),
        Err(_) => arks_config::ConfigSet::load(None),
    };
    let helper = match cfg.get(SELECTED_KEY).filter(|s| !s.is_empty()) {
        Some(h) => h,
        None => match ask_on_terminal()? {
            Some((h, always)) => {
                if always {
                    crate::cmd::cmd_config(&[
                        "--global".to_string(),
                        SELECTED_KEY.to_string(),
                        h.clone(),
                    ])?;
                }
                h
            }
            None => return Ok(()),
        },
    };
    if helper == "<no helper>" {
        return Ok(());
    }
    let got = arks_remote::credential_run_helper(&helper, action, &desc)
        .map_err(|e| CliError::fatal(e.to_string()))?;
    if let Some(c) = got {
        std::io::stdout().write_all(c.format_attrs().as_bytes())?;
    }
    Ok(())
}
