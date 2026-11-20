//! Declarative argv parser with git `parse-options` spelling rules.
//!
//! Accepted spellings follow gitcli(7): short options bundle (`-qv`), a short
//! option's value may be stuck (`-mmsg`) or separate (`-m msg`); long options
//! take `--opt=value` or `--opt value`; an optional value must be stuck
//! (`--abbrev=8`, `-S<key>`); `--no-<opt>` negates options declared negatable;
//! a unique prefix of a long option is accepted (`--verb` for `--verbose`);
//! `--` ends option parsing. Unknown options fail with exit status 129 like
//! stock git. Cite: <https://git-scm.com/docs/gitcli>.

use crate::error::{CliError, CliResult};

/// How an option consumes a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arg {
    /// Boolean switch; a value is an error.
    None,
    /// Required value (`--opt=v`, `--opt v`, `-ov`, `-o v`).
    Required,
    /// Optional value, only when stuck (`--opt=v`, `-ov`).
    Optional,
}

/// One option declaration. `name` is the identifier handed back to callers
/// (normally the long name, or the short letter when there is no long name).
#[derive(Debug, Clone, Copy)]
pub struct Opt {
    pub name: &'static str,
    pub short: Option<char>,
    pub long: Option<&'static str>,
    pub arg: Arg,
    /// Accept `--no-<long>`.
    pub negatable: bool,
}

impl Opt {
    /// Negatable boolean long option (`--[no-]name`).
    pub const fn flag(long: &'static str) -> Self {
        Self {
            name: long,
            short: None,
            long: Some(long),
            arg: Arg::None,
            negatable: true,
        }
    }

    /// Boolean long option without `--no-` form.
    pub const fn bare(long: &'static str) -> Self {
        Self {
            name: long,
            short: None,
            long: Some(long),
            arg: Arg::None,
            negatable: false,
        }
    }

    /// Negatable long option with a required value.
    pub const fn value(long: &'static str) -> Self {
        Self {
            name: long,
            short: None,
            long: Some(long),
            arg: Arg::Required,
            negatable: true,
        }
    }

    /// Negatable long option with an optional (stuck) value.
    pub const fn opt_value(long: &'static str) -> Self {
        Self {
            name: long,
            short: None,
            long: Some(long),
            arg: Arg::Optional,
            negatable: true,
        }
    }

    /// Short-only boolean (`name` is the letter).
    pub const fn short(c: char, name: &'static str) -> Self {
        Self {
            name,
            short: Some(c),
            long: None,
            arg: Arg::None,
            negatable: false,
        }
    }

    /// Short-only option with a required value (`name` is the identifier).
    pub const fn short_value(c: char, name: &'static str) -> Self {
        Self {
            name,
            short: Some(c),
            long: None,
            arg: Arg::Required,
            negatable: false,
        }
    }

    /// Add a short alias.
    pub const fn with_short(mut self, c: char) -> Self {
        self.short = Some(c);
        self
    }

    /// Drop the `--no-` form.
    pub const fn no_neg(mut self) -> Self {
        self.negatable = false;
        self
    }

    /// Change the consumed value kind.
    pub const fn arg(mut self, arg: Arg) -> Self {
        self.arg = arg;
        self
    }
}

/// One parsed argv element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Item {
    /// An option occurrence: `name`, its value, and whether it was `--no-`.
    Opt {
        name: &'static str,
        value: Option<String>,
        neg: bool,
    },
    /// A non-option argument (including everything after `--`).
    Pos(String),
    /// The `--` separator itself (positionals after it follow as `Pos`).
    DashDash,
}

/// Parser settings.
#[derive(Debug, Clone, Copy, Default)]
pub struct Mode {
    /// Stop at the first non-option (git `PARSE_OPT_STOP_AT_NON_OPTION`).
    pub stop_at_non_option: bool,
    /// Leave unknown options as positionals instead of failing.
    pub keep_unknown: bool,
}

fn unknown(cmd: &str, what: &str) -> CliError {
    CliError::Exit(
        129,
        format!("error: unknown option `{what}'\nusage: git {cmd} [<options>]"),
    )
}

fn usage_err(cmd: &str, msg: String) -> CliError {
    CliError::Exit(129, format!("error: {msg}\nusage: git {cmd} [<options>]"))
}

/// Parse `args` against `spec`. `cmd` names the command in error messages.
pub fn parse(cmd: &str, args: &[String], spec: &[Opt], mode: Mode) -> CliResult<Vec<Item>> {
    let mut out = Vec::with_capacity(args.len());
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        i += 1;
        if a == "--" {
            out.push(Item::DashDash);
            out.extend(args[i..].iter().cloned().map(Item::Pos));
            break;
        }
        if let Some(body) = a.strip_prefix("--") {
            let (name, stuck) = match body.split_once('=') {
                Some((n, v)) => (n, Some(v.to_string())),
                None => (body, None),
            };
            let Some((opt, neg)) = find_long(cmd, spec, name)? else {
                if mode.keep_unknown {
                    out.push(Item::Pos(a.clone()));
                    continue;
                }
                return Err(unknown(cmd, name));
            };
            let long = opt.long.unwrap_or(opt.name);
            let value = if neg {
                if stuck.is_some() {
                    return Err(usage_err(cmd, format!("option `no-{long}' takes no value")));
                }
                None
            } else {
                match opt.arg {
                    Arg::None => {
                        if stuck.is_some() {
                            return Err(usage_err(cmd, format!("option `{long}' takes no value")));
                        }
                        None
                    }
                    Arg::Optional => stuck,
                    Arg::Required => match stuck {
                        Some(v) => Some(v),
                        None => {
                            let v = args.get(i).cloned().ok_or_else(|| {
                                usage_err(cmd, format!("option `{long}' requires a value"))
                            })?;
                            i += 1;
                            Some(v)
                        }
                    },
                }
            };
            out.push(Item::Opt {
                name: opt.name,
                value,
                neg,
            });
            continue;
        }
        if a.len() > 1 && a.starts_with('-') {
            let chars: Vec<char> = a[1..].chars().collect();
            let mut j = 0;
            let mut parsed = Vec::new();
            let mut failed = false;
            while j < chars.len() {
                let c = chars[j];
                let Some(opt) = spec.iter().find(|o| o.short == Some(c)) else {
                    failed = true;
                    break;
                };
                j += 1;
                match opt.arg {
                    Arg::None => parsed.push(Item::Opt {
                        name: opt.name,
                        value: None,
                        neg: false,
                    }),
                    Arg::Optional => {
                        let rest: String = chars[j..].iter().collect();
                        parsed.push(Item::Opt {
                            name: opt.name,
                            value: (!rest.is_empty()).then_some(rest),
                            neg: false,
                        });
                        j = chars.len();
                    }
                    Arg::Required => {
                        let rest: String = chars[j..].iter().collect();
                        let v = if rest.is_empty() {
                            let v = args.get(i).cloned().ok_or_else(|| {
                                usage_err(cmd, format!("switch `{c}' requires a value"))
                            })?;
                            i += 1;
                            v
                        } else {
                            rest
                        };
                        parsed.push(Item::Opt {
                            name: opt.name,
                            value: Some(v),
                            neg: false,
                        });
                        j = chars.len();
                    }
                }
            }
            if failed {
                if mode.keep_unknown {
                    out.push(Item::Pos(a.clone()));
                    continue;
                }
                let c = chars[j];
                return Err(CliError::Exit(
                    129,
                    format!("error: unknown switch `{c}'\nusage: git {cmd} [<options>]"),
                ));
            }
            out.extend(parsed);
            continue;
        }
        if mode.stop_at_non_option {
            out.extend(args[i - 1..].iter().cloned().map(Item::Pos));
            break;
        }
        out.push(Item::Pos(a.clone()));
    }
    Ok(out)
}

/// Resolve a long name (exact, `no-` negation, or unique prefix).
fn find_long<'a>(cmd: &str, spec: &'a [Opt], name: &str) -> CliResult<Option<(&'a Opt, bool)>> {
    if let Some(o) = spec.iter().find(|o| o.long == Some(name)) {
        return Ok(Some((o, false)));
    }
    if let Some(base) = name.strip_prefix("no-") {
        if let Some(o) = spec.iter().find(|o| o.long == Some(base) && o.negatable) {
            return Ok(Some((o, true)));
        }
    }
    // Unique-prefix abbreviation over positive and negated spellings.
    let mut hits: Vec<(&Opt, bool, String)> = Vec::new();
    for o in spec {
        let Some(l) = o.long else { continue };
        if l.starts_with(name) {
            hits.push((o, false, l.to_string()));
        }
        if o.negatable {
            let n = format!("no-{l}");
            if n.starts_with(name) && name.len() > 3 {
                hits.push((o, true, n));
            }
        }
    }
    match hits.len() {
        0 => Ok(None),
        1 => Ok(Some((hits[0].0, hits[0].1))),
        _ => Err(usage_err(
            cmd,
            format!(
                "ambiguous option: {name} (could be --{} or --{})",
                hits[0].2, hits[1].2
            ),
        )),
    }
}

/// Positional arguments in order (excluding the `--` marker).
pub fn positionals(items: &[Item]) -> Vec<String> {
    items
        .iter()
        .filter_map(|i| match i {
            Item::Pos(p) => Some(p.clone()),
            _ => None,
        })
        .collect()
}

/// Last-wins boolean state for `name` (`--x` → true, `--no-x` → false).
pub fn flag(items: &[Item], name: &str) -> Option<bool> {
    items.iter().rev().find_map(|i| match i {
        Item::Opt { name: n, neg, .. } if *n == name => Some(!neg),
        _ => None,
    })
}

/// True when the last occurrence of `name` is positive.
pub fn on(items: &[Item], name: &str) -> bool {
    flag(items, name).unwrap_or(false)
}

/// Last value of `name`; a later `--no-name` resets it to `None`.
pub fn value(items: &[Item], name: &str) -> Option<String> {
    items.iter().rev().find_map(|i| match i {
        Item::Opt {
            name: n,
            value,
            neg,
            ..
        } if *n == name => Some(if *neg { None } else { value.clone() }),
        _ => None,
    })?
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(xs: &[&str]) -> Vec<String> {
        xs.iter().map(|s| s.to_string()).collect()
    }

    const SPEC: &[Opt] = &[
        Opt::flag("verbose").with_short('v'),
        Opt::flag("quiet").with_short('q'),
        Opt::value("message").with_short('m'),
        Opt::opt_value("abbrev"),
        Opt::flag("verify"),
        Opt::short('z', "z"),
    ];

    #[test]
    fn bundles_and_values() {
        let it = parse(
            "t",
            &v(&["-qvmhello", "x", "--abbrev=7"]),
            SPEC,
            Mode::default(),
        )
        .unwrap();
        assert!(on(&it, "quiet") && on(&it, "verbose"));
        assert_eq!(value(&it, "message").as_deref(), Some("hello"));
        assert_eq!(value(&it, "abbrev").as_deref(), Some("7"));
        assert_eq!(positionals(&it), v(&["x"]));
    }

    #[test]
    fn negation_prefix_and_dashdash() {
        let it = parse(
            "t",
            &v(&["--verb", "--no-verbose", "--message", "m", "--", "-q"]),
            SPEC,
            Mode::default(),
        )
        .unwrap();
        assert_eq!(flag(&it, "verbose"), Some(false));
        assert_eq!(positionals(&it), v(&["-q"]));
        assert!(parse("t", &v(&["--ver"]), SPEC, Mode::default()).is_err());
        assert!(parse("t", &v(&["--bogus"]), SPEC, Mode::default()).is_err());
        assert!(parse("t", &v(&["--quiet=1"]), SPEC, Mode::default()).is_err());
    }
}
