//! Table-driven git option parsing (gitcli / parse-options shapes).
//!
//! A command declares its options once (long name, short letter, value kind,
//! negatable) and gets every spelling git accepts: `--opt`, `--opt=v`,
//! `--opt v`, `--no-opt`, unique long-option prefixes, `-x`, `-xv`, `-x v`,
//! bundled shorts (`-abc`, `-am msg`), `--end-of-options` and `--`.
//! Unknown options fail like git (`error: unknown option`, exit 129) unless the
//! caller keeps them for a downstream parser (revision / diff options).
//! Cite: <https://git-scm.com/docs/gitcli> and
//! <https://git-scm.com/docs/api-parse-options> (behaviour, not code).

use crate::error::{CliError, CliResult};

/// Whether an option takes a value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Takes {
    /// Boolean switch.
    No,
    /// Required value (`--opt v`, `--opt=v`, `-ov`, `-o v`).
    Req,
    /// Optional value, attached only (`--opt[=v]`, `-o[v]`).
    Opt,
}

/// One option definition. `long` is the long name without dashes; an option
/// with no long spelling uses `-x` as its key (so `long` starts with `-`).
#[derive(Clone, Copy, Debug)]
pub struct OptDef {
    pub long: &'static str,
    pub short: Option<char>,
    pub takes: Takes,
    /// `--no-<long>` is accepted (and, for a long name `no-x`, `--x`).
    pub neg: bool,
}

/// Shorthand constructor for option tables.
pub const fn o(long: &'static str, short: char, takes: Takes, neg: bool) -> OptDef {
    OptDef {
        long,
        short: if short == ' ' { None } else { Some(short) },
        takes,
        neg,
    }
}

/// A parsed argv element.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Item {
    /// A declared option: its table key, its value and whether it was negated.
    Opt {
        key: &'static str,
        val: Option<String>,
        neg: bool,
    },
    /// A non-option argument before `--`.
    Arg(String),
    /// Everything after `--` (the `--` itself is not kept).
    AfterDashDash(String),
    /// An undeclared option kept verbatim (only with `keep_unknown`).
    Unknown(String),
    /// `-<n>` numeric shorthand (only with `numbers`).
    Number(String),
}

/// Parser behaviour switches.
#[derive(Clone, Copy, Debug, Default)]
pub struct Mode {
    /// Keep unknown options (and the value after them, when the caller says so)
    /// instead of failing.
    pub keep_unknown: bool,
    /// Stop option parsing at the first non-option argument.
    pub stop_at_non_option: bool,
    /// Accept `-<digits>`.
    pub numbers: bool,
}

/// The result of parsing: items in argv order plus accessors with git's
/// last-one-wins / `--no-` reset semantics.
#[derive(Clone, Debug, Default)]
pub struct Parsed {
    pub items: Vec<Item>,
}

fn usage_err(cmd: &str, msg: String) -> CliError {
    CliError::Exit(
        129,
        format!("{msg}\nusage: git {cmd} [<options>]\n(see `git {cmd} -h`)"),
    )
}

fn shown(def: &OptDef) -> String {
    if def.long.starts_with('-') {
        def.long.trim_start_matches('-').to_string()
    } else {
        def.long.to_string()
    }
}

/// Resolve a long spelling (without the leading `--` and without `=value`)
/// to `(definition, negated)`. Exact spellings win; otherwise a unique prefix.
fn resolve_long<'a>(
    cmd: &str,
    defs: &'a [OptDef],
    name: &str,
) -> CliResult<Option<(&'a OptDef, bool)>> {
    let mut cands: Vec<(String, &'a OptDef, bool)> = Vec::new();
    for d in defs.iter().filter(|d| !d.long.starts_with('-')) {
        cands.push((d.long.to_string(), d, false));
        if d.neg {
            cands.push((format!("no-{}", d.long), d, true));
            if let Some(pos) = d.long.strip_prefix("no-") {
                cands.push((pos.to_string(), d, true));
            }
        }
    }
    if let Some((_, d, n)) = cands.iter().find(|(s, _, _)| s == name) {
        return Ok(Some((d, *n)));
    }
    let hits: Vec<&(String, &OptDef, bool)> = cands
        .iter()
        .filter(|(s, _, _)| s.starts_with(name))
        .collect();
    match hits.as_slice() {
        [] => Ok(None),
        [(_, d, n)] => Ok(Some((d, *n))),
        many => {
            // Prefixes of one option's positive and negative forms are not ambiguous.
            let first = many[0].1.long;
            if many.iter().all(|(_, d, _)| d.long == first) {
                let (_, d, n) = many[0];
                return Ok(Some((d, *n)));
            }
            Err(usage_err(
                cmd,
                format!(
                    "error: ambiguous option: {name} (could be --{} or --{})",
                    many[0].0, many[1].0
                ),
            ))
        }
    }
}

/// Parse `args` against `defs` the way git's parse-options does.
pub fn parse(cmd: &str, args: &[String], defs: &[OptDef], mode: Mode) -> CliResult<Parsed> {
    let mut items = Vec::new();
    let mut i = 0;
    let mut opts_done = false;
    while i < args.len() {
        let a = &args[i];
        i += 1;
        if opts_done {
            items.push(Item::AfterDashDash(a.clone()));
            continue;
        }
        if a == "--" {
            opts_done = true;
            continue;
        }
        if a == "--end-of-options" {
            // Everything after is a non-option argument (but `--` still splits).
            while i < args.len() {
                let b = &args[i];
                i += 1;
                if b == "--" {
                    opts_done = true;
                    break;
                }
                items.push(Item::Arg(b.clone()));
            }
            continue;
        }
        if a == "-" || !a.starts_with('-') {
            items.push(Item::Arg(a.clone()));
            if mode.stop_at_non_option {
                opts_done = true;
                // Remaining arguments are plain arguments (git keeps `--` too).
                while i < args.len() {
                    items.push(Item::Arg(args[i].clone()));
                    i += 1;
                }
            }
            continue;
        }
        if let Some(body) = a.strip_prefix("--") {
            let (name, inline) = match body.split_once('=') {
                Some((n, v)) => (n, Some(v.to_string())),
                None => (body, None),
            };
            let Some((def, neg)) = resolve_long(cmd, defs, name)? else {
                if mode.keep_unknown {
                    items.push(Item::Unknown(a.clone()));
                    continue;
                }
                return Err(usage_err(cmd, format!("error: unknown option `{body}'")));
            };
            let key = def.long;
            if neg {
                if inline.is_some() {
                    return Err(usage_err(
                        cmd,
                        format!("error: option `no-{}' takes no value", shown(def)),
                    ));
                }
                items.push(Item::Opt {
                    key,
                    val: None,
                    neg: true,
                });
                continue;
            }
            let val = match def.takes {
                Takes::No => {
                    if inline.is_some() {
                        return Err(usage_err(
                            cmd,
                            format!("error: option `{}' takes no value", shown(def)),
                        ));
                    }
                    None
                }
                Takes::Opt => inline,
                Takes::Req => match inline {
                    Some(v) => Some(v),
                    None => {
                        let Some(v) = args.get(i) else {
                            return Err(usage_err(
                                cmd,
                                format!("error: option `{}' requires a value", shown(def)),
                            ));
                        };
                        i += 1;
                        Some(v.clone())
                    }
                },
            };
            items.push(Item::Opt {
                key,
                val,
                neg: false,
            });
            continue;
        }
        // Short option(s).
        let body: Vec<char> = a[1..].chars().collect();
        if mode.numbers && body.iter().all(|c| c.is_ascii_digit()) {
            items.push(Item::Number(a[1..].to_string()));
            continue;
        }
        let mut j = 0;
        let mut pending = Vec::new();
        let mut unknown = false;
        while j < body.len() {
            let c = body[j];
            let Some(def) = defs.iter().find(|d| d.short == Some(c)) else {
                unknown = true;
                break;
            };
            match def.takes {
                Takes::No => {
                    pending.push(Item::Opt {
                        key: def.long,
                        val: None,
                        neg: false,
                    });
                    j += 1;
                }
                Takes::Opt => {
                    let rest: String = body[j + 1..].iter().collect();
                    pending.push(Item::Opt {
                        key: def.long,
                        val: (!rest.is_empty()).then_some(rest),
                        neg: false,
                    });
                    j = body.len();
                }
                Takes::Req => {
                    let rest: String = body[j + 1..].iter().collect();
                    let val = if rest.is_empty() {
                        let Some(v) = args.get(i) else {
                            return Err(usage_err(
                                cmd,
                                format!("error: switch `{c}' requires a value"),
                            ));
                        };
                        i += 1;
                        v.clone()
                    } else {
                        rest
                    };
                    pending.push(Item::Opt {
                        key: def.long,
                        val: Some(val),
                        neg: false,
                    });
                    j = body.len();
                }
            }
        }
        if unknown {
            if mode.keep_unknown {
                items.push(Item::Unknown(a.clone()));
                continue;
            }
            return Err(usage_err(
                cmd,
                format!("error: unknown switch `{}'", body[j]),
            ));
        }
        items.extend(pending);
    }
    Ok(Parsed { items })
}

impl Parsed {
    /// Last boolean state of `key` (`--no-key` → false); `None` when absent.
    pub fn flag(&self, key: &str) -> Option<bool> {
        self.items.iter().rev().find_map(|it| match it {
            Item::Opt { key: k, neg, .. } if *k == key => Some(!*neg),
            _ => None,
        })
    }

    /// `true` when the last occurrence of `key` is positive.
    pub fn on(&self, key: &str) -> bool {
        self.flag(key) == Some(true)
    }

    /// Number of positive occurrences after the last negation (`-vv` → 2).
    pub fn count(&self, key: &str) -> usize {
        let mut n = 0;
        for it in &self.items {
            if let Item::Opt { key: k, neg, .. } = it {
                if *k == key {
                    n = if *neg { 0 } else { n + 1 };
                }
            }
        }
        n
    }

    /// Last value of `key`; `--no-key` resets it. `Some(None)` = present without a value.
    pub fn last(&self, key: &str) -> Option<Option<String>> {
        let mut out = None;
        for it in &self.items {
            if let Item::Opt { key: k, val, neg } = it {
                if *k == key {
                    out = if *neg { None } else { Some(val.clone()) };
                }
            }
        }
        out
    }

    /// Last value of `key` as a plain string (`None` when absent, reset or valueless).
    pub fn value(&self, key: &str) -> Option<String> {
        self.last(key).flatten()
    }

    /// Every value of `key` in order; `--no-key` clears the list.
    pub fn values(&self, key: &str) -> Vec<String> {
        let mut out = Vec::new();
        for it in &self.items {
            if let Item::Opt { key: k, val, neg } = it {
                if *k == key {
                    if *neg {
                        out.clear();
                    } else if let Some(v) = val {
                        out.push(v.clone());
                    }
                }
            }
        }
        out
    }

    /// Non-option arguments before `--`.
    pub fn args(&self) -> Vec<String> {
        self.items
            .iter()
            .filter_map(|it| match it {
                Item::Arg(a) => Some(a.clone()),
                _ => None,
            })
            .collect()
    }

    /// Arguments after `--` (empty when there was none).
    pub fn after_dashdash(&self) -> Vec<String> {
        self.items
            .iter()
            .filter_map(|it| match it {
                Item::AfterDashDash(a) => Some(a.clone()),
                _ => None,
            })
            .collect()
    }

    /// Unknown options kept verbatim (with `keep_unknown`).
    pub fn unknown(&self) -> Vec<String> {
        self.items
            .iter()
            .filter_map(|it| match it {
                Item::Unknown(a) => Some(a.clone()),
                _ => None,
            })
            .collect()
    }
}

/// Read a pathspec file (`--pathspec-from-file`, `-` = stdin); `nul` splits on
/// NUL, otherwise on newlines (C-style quoted lines are unquoted).
pub fn read_pathspec_file(path: &str, nul: bool) -> CliResult<Vec<String>> {
    use std::io::Read;
    let mut buf = Vec::new();
    if path == "-" {
        std::io::stdin()
            .read_to_end(&mut buf)
            .map_err(|e| CliError::fatal(format!("could not read stdin: {e}")))?;
    } else {
        buf = std::fs::read(path)
            .map_err(|e| CliError::fatal(format!("could not open '{path}' for reading: {e}")))?;
    }
    let text = String::from_utf8_lossy(&buf);
    let parts: Vec<String> = if nul {
        text.split('\0')
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect()
    } else {
        text.lines()
            .map(|l| l.strip_suffix('\r').unwrap_or(l))
            .filter(|l| !l.is_empty())
            .map(unquote_c)
            .collect()
    };
    Ok(parts)
}

fn unquote_c(s: &str) -> String {
    let Some(inner) = s.strip_prefix('"').and_then(|r| r.strip_suffix('"')) else {
        return s.to_string();
    };
    let mut out = Vec::new();
    let b = inner.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'\\' && i + 1 < b.len() {
            i += 1;
            match b[i] {
                b'n' => out.push(b'\n'),
                b't' => out.push(b'\t'),
                b'"' => out.push(b'"'),
                b'\\' => out.push(b'\\'),
                d @ b'0'..=b'7' if i + 2 < b.len() => {
                    let oct = std::str::from_utf8(&b[i..i + 3]).unwrap_or("0");
                    out.push(u8::from_str_radix(oct, 8).unwrap_or(d));
                    i += 2;
                }
                other => out.push(other),
            }
        } else {
            out.push(b[i]);
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use Takes::*;

    const DEFS: &[OptDef] = &[
        o("all", 'a', No, true),
        o("message", 'm', Req, true),
        o("gpg-sign", 'S', Opt, true),
        o("no-verify", 'n', No, true),
        o("untracked-files", 'u', Opt, true),
        o("-b", 'b', Req, false),
        o("amend", ' ', No, true),
        o("author", ' ', Req, true),
    ];

    fn v(xs: &[&str]) -> Vec<String> {
        xs.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn spellings() {
        let p = parse(
            "x",
            &v(&["-am", "hi", "--no-all", "--mess=two", "-uno", "--ame", "p"]),
            DEFS,
            Mode::default(),
        )
        .unwrap();
        assert_eq!(p.flag("all"), Some(false));
        assert_eq!(p.values("message"), v(&["hi", "two"]));
        assert_eq!(p.value("untracked-files").as_deref(), Some("no"));
        assert!(p.on("amend"));
        assert_eq!(p.args(), v(&["p"]));
    }

    #[test]
    fn negations_and_short_only() {
        let p = parse(
            "x",
            &v(&["--verify", "-bfoo", "-S", "--", "-a"]),
            DEFS,
            Mode::default(),
        )
        .unwrap();
        assert_eq!(p.flag("no-verify"), Some(false));
        assert_eq!(p.value("-b").as_deref(), Some("foo"));
        assert_eq!(p.last("gpg-sign"), Some(None));
        assert_eq!(p.after_dashdash(), v(&["-a"]));
    }

    #[test]
    fn errors() {
        assert!(parse("x", &v(&["--bogus"]), DEFS, Mode::default()).is_err());
        assert!(parse("x", &v(&["-z"]), DEFS, Mode::default()).is_err());
        assert!(parse("x", &v(&["--a"]), DEFS, Mode::default()).is_err());
        assert!(parse("x", &v(&["--all=1"]), DEFS, Mode::default()).is_err());
        let p = parse(
            "x",
            &v(&["--bogus", "-z"]),
            DEFS,
            Mode {
                keep_unknown: true,
                ..Mode::default()
            },
        )
        .unwrap();
        assert_eq!(p.unknown(), v(&["--bogus", "-z"]));
    }
}
