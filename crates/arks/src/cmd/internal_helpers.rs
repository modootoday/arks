//! git-internal helper commands (`*--*`) and alias rows.
//!
//! Pin 2.39.5 ships these as internal programs; arks mirrors what they do
//! from their documentation and from observed stock behaviour (oracle only):
//! - `pickaxe`: the historical name of `blame` (same options and output).
//! - `add--interactive [--patch[=<mode>]] [--] [<pathspec>...]`: the
//!   interactive add / patch front end; routed to the `add -i`, `add -p`,
//!   `reset -p`, `checkout -p` and `stash -p` bodies.
//! - `bisect--helper`: the `bisect` subcommands, spelled `--bisect-<sub>`
//!   or `<sub>`.
//! - `submodule--helper <command>`: the `submodule` subcommands.
//! - `env--helper --type=[bool|ulong] [--default=<v>] [--exit-code] <env-var>`
//!   (<https://git-scm.com/docs/git-env--helper>).
//! - `sh-i18n--envsubst [--variables] <shell-format>`
//!   (<https://git-scm.com/docs/git-sh-i18n--envsubst>).
//! - `checkout--worker [--prefix <string>]`: the parallel-checkout worker;
//!   arks checks out in-process, so the worker serves an empty request stream.
//! - `fsmonitor--daemon`: stock git's Linux builds have no file-system
//!   monitor daemon and say so; arks says the same on every platform.
//! - `update`: not a git command in any release; arks prints stock git's
//!   unknown-command message and suggestion.

use crate::cmd::opt_parse::{Mode, Takes, o, parse};
use crate::error::{CliError, CliResult};
use std::io::{Read, Write};

/// `arks pickaxe ...` = `arks blame ...`.
pub fn cmd_pickaxe(args: &[String]) -> CliResult<()> {
    crate::cmd::cmd_blame(args)
}

/// `arks update`: mirrors stock git, which has no such command.
pub fn cmd_update(_args: &[String]) -> CliResult<()> {
    Err(CliError::Exit(
        1,
        "git: 'update' is not a git command. See 'git --help'.\n\nThe most similar command is\n\tupdate-ref".into(),
    ))
}

/// `arks add--interactive [--patch[=<mode>] | -p] [--] [<pathspec>...]`.
pub fn cmd_add_interactive_helper(args: &[String]) -> CliResult<()> {
    use crate::cmd::interactive_patch as ip;
    let mut mode: Option<String> = None;
    let mut rest: Vec<String> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        match a {
            "-p" | "--patch" => mode = Some("stage".into()),
            _ if a.starts_with("--patch=") => mode = Some(a["--patch=".len()..].to_string()),
            _ => {
                rest.extend(args[i..].iter().cloned());
                break;
            }
        }
        i += 1;
    }
    match mode.as_deref() {
        None => ip::cmd_add_interactive(&rest),
        Some("stage") => ip::cmd_add_patch_hunks(&rest),
        Some("stash") => ip::cmd_stash_patch(&rest),
        Some("reset") => ip::cmd_reset_patch(&rest),
        Some("checkout" | "worktree") => ip::cmd_checkout_patch(&rest),
        Some(m) => Err(CliError::fatal(format!("unknown --patch mode: {m}"))),
    }
}

const BISECT_SUBS: &[&str] = &[
    "reset",
    "terms",
    "start",
    "next",
    "log",
    "replay",
    "skip",
    "visualize",
    "view",
    "run",
    "state",
];

/// `arks bisect--helper (--bisect-<sub> | <sub>) [<args>...]`.
pub fn cmd_bisect_helper(args: &[String]) -> CliResult<()> {
    let usage = || {
        CliError::Exit(
            129,
            "usage: git bisect--helper --bisect-reset [<commit>]\n   or: git bisect--helper --bisect-terms [--term-good | --term-old | --term-bad | --term-new]\n   or: git bisect--helper --bisect-start [--term-{new,bad}=<term> --term-{old,good}=<term>] [--no-checkout] [--first-parent] [<bad> [<good>...]] [--] [<paths>...]\n   or: git bisect--helper --bisect-next\n   or: git bisect--helper --bisect-state (bad|new) [<rev>]\n   or: git bisect--helper --bisect-state (good|old) [<rev>...]\n   or: git bisect--helper --bisect-replay <filename>\n   or: git bisect--helper --bisect-skip [(<rev>|<range>)...]\n   or: git bisect--helper --bisect-visualize\n   or: git bisect--helper --bisect-run <cmd>...".into(),
        )
    };
    let Some(first) = args.first() else {
        return Err(usage());
    };
    let sub = first.strip_prefix("--bisect-").unwrap_or(first);
    if first == "--next-all" {
        return crate::cmd::cmd_bisect(&["next".to_string()]);
    }
    if !BISECT_SUBS.contains(&sub) {
        return Err(usage());
    }
    let mut argv: Vec<String> = Vec::new();
    if sub == "state" {
        // `--bisect-state <term> [<rev>...]`: the term is the bisect verb.
        argv.extend(args[1..].iter().cloned());
        if argv.is_empty() {
            return Err(usage());
        }
    } else {
        argv.push(sub.to_string());
        argv.extend(args[1..].iter().cloned());
    }
    crate::cmd::cmd_bisect(&argv)
}

const SUBMODULE_SUBS: &[&str] = &[
    "add",
    "init",
    "deinit",
    "update",
    "status",
    "foreach",
    "summary",
    "sync",
    "absorbgitdirs",
    "set-url",
    "set-branch",
];

/// `arks submodule--helper <command> [<args>...]`.
pub fn cmd_submodule_helper(args: &[String]) -> CliResult<()> {
    const USAGE: &str = "usage: git submodule--helper <command>\n";
    let Some(sub) = args.first() else {
        return Err(CliError::Exit(
            129,
            format!("error: need a subcommand\n{USAGE}"),
        ));
    };
    if sub == "-h" || sub == "--help" {
        // parse-options prints `-h` usage on stdout.
        println!("{USAGE}");
        return Err(CliError::silent(129));
    }
    if !SUBMODULE_SUBS.contains(&sub.as_str()) {
        return Err(CliError::Exit(
            129,
            format!("error: unknown subcommand: `{sub}'\n{USAGE}"),
        ));
    }
    crate::cmd::cmd_submodule(args)
}

/// `arks fsmonitor--daemon (start|run|stop|status) [<options>]`.
pub fn cmd_fsmonitor_daemon(_args: &[String]) -> CliResult<()> {
    Err(CliError::fatal(
        "fsmonitor--daemon not supported on this platform",
    ))
}

/// `arks env--helper --type=[bool|ulong] [--default=<value>] [--exit-code] <env-var>`.
pub fn cmd_env_helper(args: &[String]) -> CliResult<()> {
    const USAGE: &str = "usage: git env--helper --type=[bool|ulong] <options> <env-var>";
    let defs = [
        o("type", ' ', Takes::Req, false),
        o("default", ' ', Takes::Req, false),
        o("exit-code", ' ', Takes::No, true),
    ];
    let p = parse("env--helper", args, &defs, Mode::default())?;
    let usage = |msg: &str| {
        let mut m = String::new();
        if !msg.is_empty() {
            m.push_str(msg);
            m.push('\n');
        }
        m.push_str(USAGE);
        CliError::Exit(129, m)
    };
    let kind = match p.value("type").as_deref() {
        Some("bool") => EnvType::Bool,
        Some("ulong") => EnvType::Ulong,
        Some(t) => {
            return Err(usage(&format!("error: unrecognized --type argument, {t}")));
        }
        None => return Err(usage("error: option `--type=[bool|ulong]' is required")),
    };
    let positional = p.args();
    let [name] = positional.as_slice() else {
        return Err(usage(""));
    };
    let exit_code = p.on("exit-code");
    let default = match p.value("default") {
        Some(d) => Some(match kind.parse(&d) {
            Some(v) => v,
            None => {
                return Err(usage(&format!(
                    "error: option `--default' expects {} value with `--type={}`, not `{d}`",
                    kind.article(),
                    kind.name()
                )));
            }
        }),
        None => None,
    };
    let value = match std::env::var(name) {
        Ok(raw) => kind.parse(&raw).ok_or_else(|| {
            CliError::fatal(format!(
                "bad {} environment value '{raw}' for '{name}'",
                kind.label()
            ))
        })?,
        Err(_) => default.unwrap_or(0),
    };
    if exit_code {
        return if value != 0 {
            Ok(())
        } else {
            Err(CliError::silent(1))
        };
    }
    match kind {
        EnvType::Bool => println!("{}", if value != 0 { "true" } else { "false" }),
        EnvType::Ulong => println!("{value}"),
    }
    Ok(())
}

#[derive(Clone, Copy)]
enum EnvType {
    Bool,
    Ulong,
}

impl EnvType {
    fn name(self) -> &'static str {
        match self {
            Self::Bool => "bool",
            Self::Ulong => "ulong",
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::Bool => "boolean",
            Self::Ulong => "numeric",
        }
    }
    fn article(self) -> &'static str {
        match self {
            Self::Bool => "a boolean",
            Self::Ulong => "an unsigned long",
        }
    }
    /// Git config value rules: booleans (`true/yes/on/1`, `false/no/off/0/""`
    /// or an integer) and unsigned integers with an optional k/m/g unit.
    fn parse(self, raw: &str) -> Option<u64> {
        match self {
            Self::Bool => match raw.to_ascii_lowercase().as_str() {
                "true" | "yes" | "on" => Some(1),
                "false" | "no" | "off" | "" => Some(0),
                n => n.parse::<i64>().ok().map(|v| u64::from(v != 0)),
            },
            Self::Ulong => {
                let lower = raw.to_ascii_lowercase();
                let (digits, mul) = match lower.chars().last() {
                    Some('k') => (&lower[..lower.len() - 1], 1024u64),
                    Some('m') => (&lower[..lower.len() - 1], 1024 * 1024),
                    Some('g') => (&lower[..lower.len() - 1], 1024 * 1024 * 1024),
                    _ => (lower.as_str(), 1),
                };
                digits.parse::<u64>().ok()?.checked_mul(mul)
            }
        }
    }
}

/// `arks sh-i18n--envsubst [--variables] <shell-format>`.
pub fn cmd_sh_i18n_envsubst(args: &[String]) -> CliResult<()> {
    match args {
        [] => {
            eprintln!("error: we won't substitute all variables on stdin for you");
            Ok(())
        }
        [format] => {
            let names = shell_variables(format);
            let mut input = String::new();
            std::io::stdin().read_to_string(&mut input)?;
            let out = substitute(&input, &names);
            std::io::stdout().write_all(out.as_bytes())?;
            Ok(())
        }
        [flag, format] => {
            if flag != "--variables" {
                eprintln!("error: first argument must be --variables when two are given");
                return Ok(());
            }
            for n in shell_variables(format) {
                println!("{n}");
            }
            Ok(())
        }
        _ => {
            eprintln!("error: too many arguments");
            Ok(())
        }
    }
}

/// A `$NAME` / `${NAME}` reference at byte `i` of `s`: `(name, end)`.
fn variable_at(s: &[u8], i: usize) -> Option<(String, usize)> {
    if s.get(i) != Some(&b'$') {
        return None;
    }
    let braced = s.get(i + 1) == Some(&b'{');
    let start = i + 1 + usize::from(braced);
    let first = *s.get(start)?;
    if !(first.is_ascii_alphabetic() || first == b'_') {
        return None;
    }
    let mut end = start + 1;
    while end < s.len() && (s[end].is_ascii_alphanumeric() || s[end] == b'_') {
        end += 1;
    }
    let name = String::from_utf8_lossy(&s[start..end]).to_string();
    if braced {
        if s.get(end) != Some(&b'}') {
            return None;
        }
        end += 1;
    }
    Some((name, end))
}

/// Variable names referenced in `format`, in order (duplicates kept).
fn shell_variables(format: &str) -> Vec<String> {
    let s = format.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < s.len() {
        match variable_at(s, i) {
            Some((name, end)) => {
                out.push(name);
                i = end;
            }
            None => i += 1,
        }
    }
    out
}

/// Replace references to `names` in `input` with their environment values.
fn substitute(input: &str, names: &[String]) -> String {
    let s = input.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(s.len());
    let mut i = 0;
    while i < s.len() {
        if let Some((name, end)) = variable_at(s, i) {
            if names.contains(&name) {
                out.extend_from_slice(std::env::var(&name).unwrap_or_default().as_bytes());
                i = end;
                continue;
            }
        }
        out.push(s[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Read one pkt-line from `r`: `Ok(None)` for a flush packet.
pub(crate) fn read_pkt(r: &mut impl Read) -> CliResult<Option<Vec<u8>>> {
    let hung_up = || CliError::fatal("the remote end hung up unexpectedly");
    let mut len = [0u8; 4];
    r.read_exact(&mut len).map_err(|_| hung_up())?;
    let n = std::str::from_utf8(&len)
        .ok()
        .and_then(|h| usize::from_str_radix(h, 16).ok())
        .ok_or_else(|| {
            CliError::fatal(format!(
                "protocol error: bad line length character: {}",
                String::from_utf8_lossy(&len)
            ))
        })?;
    if n == 0 {
        return Ok(None);
    }
    if n < 4 {
        return Err(CliError::fatal(format!(
            "protocol error: bad line length {n}"
        )));
    }
    let mut buf = vec![0u8; n - 4];
    r.read_exact(&mut buf).map_err(|_| hung_up())?;
    Ok(Some(buf))
}

/// `arks checkout--worker [--prefix <string>]`.
pub fn cmd_checkout_worker(args: &[String]) -> CliResult<()> {
    const USAGE: &str = "usage: git checkout--worker [<options>]\n\n    --[no-]prefix <string>\n                          when creating files, prepend <string>\n";
    let defs = [o("prefix", ' ', Takes::Req, true)];
    let p = parse("checkout--worker", args, &defs, Mode::default())?;
    if !p.args().is_empty() || !p.after_dashdash().is_empty() {
        return Err(CliError::Exit(129, USAGE.into()));
    }
    let mut input = std::io::stdin().lock();
    match read_pkt(&mut input)? {
        None => Ok(()),
        Some(_) => Err(CliError::fatal(
            "checkout--worker: arks checks files out in-process and serves no parallel-checkout requests",
        )),
    }
}

/// `arks upload-archive--writer <repository>`: read `argument <arg>`
/// pkt-lines up to a flush from stdin and write the archive to stdout.
pub fn cmd_upload_archive_writer(args: &[String]) -> CliResult<()> {
    let usage = || CliError::Exit(129, "usage: git upload-archive <repository>".into());
    let [dir] = args else {
        return Err(usage());
    };
    if dir == "-h" {
        return Err(usage());
    }
    let repo = arks_repo::Repository::open(dir)
        .or_else(|_| arks_repo::Repository::open(std::path::Path::new(dir).join(".git")))
        .map_err(|_| CliError::fatal(format!("'{dir}' does not appear to be a git repository")))?;
    let mut input = std::io::stdin().lock();
    let mut argv: Vec<String> = Vec::new();
    while let Some(line) = read_pkt(&mut input)? {
        let text = String::from_utf8_lossy(&line);
        let text = text.strip_suffix('\n').unwrap_or(&text);
        let Some(a) = text.strip_prefix("argument ") else {
            return Err(CliError::fatal("'argument' token or flush expected"));
        };
        argv.push(a.to_string());
    }
    for a in &argv {
        for forbidden in ["--remote", "--output", "--exec"] {
            if a == forbidden || a.starts_with(&format!("{forbidden}=")) {
                return Err(CliError::fatal(format!("Unexpected option {forbidden}")));
            }
        }
    }
    let bytes = crate::cmd::archive::archive_bytes_for_args(&repo, &argv)?;
    std::io::stdout().write_all(&bytes)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envsubst_variables_and_substitution() {
        assert_eq!(
            shell_variables("x $FOO ${BAR} $FOO $1 ${X_Y}z ${OPEN"),
            vec!["FOO", "BAR", "FOO", "X_Y"]
        );
        let names = vec!["A3_ENVSUBST_UNSET".to_string()];
        assert_eq!(
            substitute("a $A3_ENVSUBST_UNSET b ${OTHER} $$", &names),
            "a  b ${OTHER} $$"
        );
    }

    #[test]
    fn env_types_parse_like_config() {
        assert_eq!(EnvType::Bool.parse("Yes"), Some(1));
        assert_eq!(EnvType::Bool.parse(""), Some(0));
        assert_eq!(EnvType::Bool.parse("2"), Some(1));
        assert_eq!(EnvType::Bool.parse("maybe"), None);
        assert_eq!(EnvType::Ulong.parse("2k"), Some(2048));
        assert_eq!(EnvType::Ulong.parse("-1"), None);
    }
}
