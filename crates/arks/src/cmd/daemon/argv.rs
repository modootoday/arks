//! `arks daemon` argv with `git daemon` spelling.
//!
//! Primary: <https://git-scm.com/docs/git-daemon> (SYNOPSIS, OPTIONS). Values use
//! `--opt=<value>` only; option parsing stops at the first non-option or `--`,
//! and the remaining words are the directory whitelist. Error shapes follow
//! observed stock `git daemon` (usage → 129, fatal → 128).

use crate::error::CliError;
use arks_server::{DaemonService, LogDestination, ServiceTable};
use std::path::PathBuf;

/// Usage text in the shape of `git daemon -h`.
pub(super) const USAGE: &str = "\
usage: arks daemon [--verbose] [--syslog] [--export-all]
           [--timeout=<n>] [--init-timeout=<n>] [--max-connections=<n>]
           [--strict-paths] [--base-path=<path>] [--base-path-relaxed]
           [--user-path | --user-path=<path>]
           [--interpolated-path=<path>]
           [--reuseaddr] [--pid-file=<file>]
           [--(enable|disable|allow-override|forbid-override)=<service>]
           [--access-hook=<path>]
           [--inetd | [--listen=<host_or_ipaddr>] [--port=<n>]
                      [--detach] [--user=<user> [--group=<group>]]
           [--log-destination=(stderr|syslog|none)]
           [<directory>...]";

/// Parsed `arks daemon` command line.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(super) struct DaemonArgs {
    pub help: bool,
    pub verbose: bool,
    pub log_destination: Option<LogDestination>,
    pub export_all: bool,
    pub timeout: u64,
    pub init_timeout: u64,
    pub max_connections: Option<usize>,
    pub strict_paths: bool,
    pub base_path: Option<PathBuf>,
    pub base_path_relaxed: bool,
    pub user_path: Option<String>,
    pub interpolated_path: Option<String>,
    pub reuseaddr: bool,
    pub pid_file: Option<PathBuf>,
    pub services: ServiceTable,
    pub informative_errors: bool,
    pub access_hook: Option<PathBuf>,
    pub inetd: bool,
    pub listen: Vec<String>,
    pub port: Option<u16>,
    pub detach: bool,
    pub user: Option<String>,
    pub group: Option<String>,
    pub directories: Vec<PathBuf>,
}

fn usage() -> CliError {
    CliError::Exit(129, USAGE.to_string())
}

/// C `atoi`-style number: optional sign, leading digits, anything else ends it.
fn lenient_int(v: &str) -> i64 {
    let v = v.trim_start();
    let (neg, digits) = match v.as_bytes().first() {
        Some(b'-') => (true, &v[1..]),
        Some(b'+') => (false, &v[1..]),
        _ => (false, v),
    };
    let n = digits
        .bytes()
        .take_while(u8::is_ascii_digit)
        .fold(0i64, |acc, d| {
            acc.saturating_mul(10).saturating_add(i64::from(d - b'0'))
        });
    if neg { -n } else { n }
}

fn service(name: &str) -> Result<DaemonService, CliError> {
    DaemonService::from_name(name).ok_or_else(|| CliError::fatal(format!("No such service {name}")))
}

pub(super) fn parse(args: &[String]) -> Result<DaemonArgs, CliError> {
    let mut a = DaemonArgs {
        services: ServiceTable::default(),
        ..DaemonArgs::default()
    };
    let mut i = 0;
    while i < args.len() {
        let arg = args[i].as_str();
        if arg == "--" {
            a.directories = args[i + 1..].iter().map(PathBuf::from).collect();
            return Ok(a);
        }
        if !arg.starts_with('-') {
            a.directories = args[i..].iter().map(PathBuf::from).collect();
            return Ok(a);
        }
        let (key, value) = match arg.split_once('=') {
            Some((k, v)) => (k, Some(v)),
            None => (arg, None),
        };
        match (key, value) {
            ("--help", None) => a.help = true,
            ("--verbose", None) => a.verbose = true,
            ("--syslog", None) => a.log_destination = Some(LogDestination::Syslog),
            ("--log-destination", Some(v)) => {
                a.log_destination =
                    Some(LogDestination::parse(v).ok_or_else(|| {
                        CliError::fatal(format!("unknown log destination '{v}'"))
                    })?);
            }
            ("--export-all", None) => a.export_all = true,
            ("--timeout", Some(v)) => a.timeout = lenient_int(v).max(0) as u64,
            ("--init-timeout", Some(v)) => a.init_timeout = lenient_int(v).max(0) as u64,
            ("--max-connections", Some(v)) => {
                a.max_connections = Some(lenient_int(v).max(0) as usize);
            }
            ("--strict-paths", None) => a.strict_paths = true,
            ("--base-path", Some(v)) => a.base_path = Some(PathBuf::from(v)),
            ("--base-path-relaxed", None) => a.base_path_relaxed = true,
            ("--user-path", None) => a.user_path = Some(String::new()),
            ("--user-path", Some(v)) => a.user_path = Some(v.to_string()),
            ("--interpolated-path", Some(v)) => a.interpolated_path = Some(v.to_string()),
            ("--reuseaddr", None) => a.reuseaddr = true,
            ("--pid-file", Some(v)) => a.pid_file = Some(PathBuf::from(v)),
            ("--enable", Some(v)) => a.services.get_mut(service(v)?).enabled = true,
            ("--disable", Some(v)) => a.services.get_mut(service(v)?).enabled = false,
            ("--allow-override", Some(v)) => a.services.get_mut(service(v)?).overridable = true,
            ("--forbid-override", Some(v)) => {
                a.services.get_mut(service(v)?).overridable = false;
            }
            ("--informative-errors", None) => a.informative_errors = true,
            ("--no-informative-errors", None) => a.informative_errors = false,
            ("--access-hook", Some(v)) => a.access_hook = Some(PathBuf::from(v)),
            ("--inetd", None) => a.inetd = true,
            ("--listen", Some(v)) => a.listen.push(v.to_string()),
            ("--port", Some(v)) => a.port = Some(v.parse().map_err(|_| usage())?),
            ("--detach", None) => a.detach = true,
            ("--user", Some(v)) => a.user = Some(v.to_string()),
            ("--group", Some(v)) => a.group = Some(v.to_string()),
            _ => return Err(usage()),
        }
        i += 1;
    }
    Ok(a)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(args: &[&str]) -> Result<DaemonArgs, CliError> {
        parse(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    }

    fn code(r: Result<DaemonArgs, CliError>) -> (u8, String) {
        match r {
            Err(CliError::Exit(c, m)) => (c, m),
            other => panic!("expected exit error, got {other:?}"),
        }
    }

    #[test]
    fn git_spelling_parses() {
        let a = p(&[
            "--verbose",
            "--listen=127.0.0.1",
            "--listen=::1",
            "--port=9419",
            "--base-path=/srv",
            "--base-path-relaxed",
            "--user-path=public_git",
            "--interpolated-path=/v/%H%D",
            "--enable=receive-pack",
            "--forbid-override=upload-pack",
            "--informative-errors",
            "--access-hook=/bin/hook",
            "--log-destination=none",
            "--max-connections=-1",
            "--timeout=abc",
            "/srv/a",
            "--not-an-option",
        ])
        .unwrap();
        assert_eq!(a.listen, vec!["127.0.0.1", "::1"]);
        assert_eq!(a.port, Some(9419));
        assert_eq!(a.user_path.as_deref(), Some("public_git"));
        assert!(a.services.receive_pack.enabled);
        assert!(!a.services.upload_pack.overridable);
        assert_eq!(a.log_destination, Some(LogDestination::None));
        assert_eq!(a.max_connections, Some(0));
        assert_eq!(a.timeout, 0);
        // Everything after the first directory is a directory, as in git.
        assert_eq!(
            a.directories,
            vec![PathBuf::from("/srv/a"), PathBuf::from("--not-an-option")]
        );
        let b = p(&["--user-path", "--", "--x"]).unwrap();
        assert_eq!(b.user_path.as_deref(), Some(""));
        assert_eq!(b.directories, vec![PathBuf::from("--x")]);
    }

    #[test]
    fn separate_value_forms_are_usage_errors() {
        for bad in [
            &["--port", "9418"][..],
            &["--listen", "127.0.0.1"],
            &["--base-path", "/srv"],
            &["--no-reuseaddr"],
            &["--bogus"],
            &["--port=abc"],
            &["--verbose=1"],
            &["--access-hook"],
            &["-h"],
        ] {
            let (c, m) = code(p(bad));
            assert_eq!(c, 129, "{bad:?}");
            assert!(m.starts_with("usage: arks daemon"), "{m}");
        }
    }

    #[test]
    fn unknown_service_and_log_destination_are_fatal() {
        assert_eq!(
            code(p(&["--enable=bogus"])),
            (128, "fatal: No such service bogus".to_string())
        );
        assert_eq!(
            code(p(&["--allow-override=bogus"])),
            (128, "fatal: No such service bogus".to_string())
        );
        assert_eq!(
            code(p(&["--log-destination=foo"])),
            (128, "fatal: unknown log destination 'foo'".to_string())
        );
    }

    #[test]
    fn lenient_numbers_follow_atoi() {
        assert_eq!(lenient_int("12abc"), 12);
        assert_eq!(lenient_int("-3"), -3);
        assert_eq!(lenient_int("x"), 0);
    }
}
