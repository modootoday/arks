//! parse-options style argv parsing for commands that declare their whole
//! option table up front (the remote / transport family first).
//!
//! Spellings follow gitcli: `-x`, bundled shorts (`-qv`, `-j4`), `--long`,
//! `--long=value`, `--long value`, `--no-long` negation (and `--long` as the
//! negation of a `no-long` option), unique-prefix abbreviation of long
//! names, and `--` ending option parsing.
//! Cite: <https://git-scm.com/docs/gitcli>.
//!
//! An option table entry is `"<short>|<long><arg>"` where `<short>` is empty
//! or one character, `<long>` may be empty, and `<arg>` is empty (boolean),
//! `=` (required value) or `=?` (optional value, attached form only).

use crate::error::{CliError, CliResult};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ArgKind {
    Flag,
    Value,
    OptValue,
}

#[derive(Clone, Debug)]
struct Opt {
    short: Option<char>,
    long: &'static str,
    kind: ArgKind,
    key: &'static str,
}

fn compile(spec: &[&'static str]) -> Vec<Opt> {
    spec.iter()
        .map(|s| {
            let (body, kind) = if let Some(b) = s.strip_suffix("=?") {
                (b, ArgKind::OptValue)
            } else if let Some(b) = s.strip_suffix('=') {
                (b, ArgKind::Value)
            } else {
                (*s, ArgKind::Flag)
            };
            let (short, long) = body.split_once('|').unwrap_or(("", body));
            let short = short.chars().next();
            let key = if long.is_empty() {
                // Short-only option: the key is the one-character spelling.
                &body[..short.map_or(0, char::len_utf8)]
            } else {
                long
            };
            Opt {
                short,
                long,
                kind,
                key,
            }
        })
        .collect()
}

/// One parsed occurrence: `key` is the long name (or the short letter for a
/// short-only option); `neg` marks `--no-<long>` (or `--<x>` for `no-<x>`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OptEvent {
    pub key: &'static str,
    pub value: Option<String>,
    pub neg: bool,
}

#[derive(Clone, Debug, Default)]
pub struct Parsed {
    pub events: Vec<OptEvent>,
    pub args: Vec<String>,
}

impl Parsed {
    /// Last boolean state of `key` (`None` when never given).
    pub fn flag(&self, key: &str) -> Option<bool> {
        self.events
            .iter()
            .rev()
            .find(|e| e.key == key)
            .map(|e| !e.neg)
    }

    /// True when `key` was given and not negated last.
    pub fn on(&self, key: &str) -> bool {
        self.flag(key) == Some(true)
    }

    /// True when `key` appears at all (either polarity).
    pub fn seen(&self, key: &str) -> bool {
        self.events.iter().any(|e| e.key == key)
    }

    /// Last value of `key`; a later negation clears it.
    pub fn value(&self, key: &str) -> Option<String> {
        match self.events.iter().rev().find(|e| e.key == key) {
            Some(e) if !e.neg => e.value.clone(),
            _ => None,
        }
    }

    /// Every value of `key` in order; a negation clears the list so far.
    pub fn values(&self, key: &str) -> Vec<String> {
        let mut out = Vec::new();
        for e in self.events.iter().filter(|e| e.key == key) {
            if e.neg {
                out.clear();
            } else if let Some(v) = &e.value {
                out.push(v.clone());
            }
        }
        out
    }

    /// `-v`-style counter: +1 per occurrence, reset by the negation.
    pub fn count(&self, key: &str) -> usize {
        let mut n = 0;
        for e in self.events.iter().filter(|e| e.key == key) {
            n = if e.neg { 0 } else { n + 1 };
        }
        n
    }
}

fn unknown_long(a: &str) -> CliError {
    CliError::Exit(129, format!("error: unknown option `{a}'"))
}

fn unknown_short(c: char) -> CliError {
    CliError::Exit(129, format!("error: unknown switch `{c}'"))
}

/// Resolve a long spelling (without the leading `--`) to (option, negated).
fn find_long<'a>(opts: &'a [Opt], name: &str) -> Result<(&'a Opt, bool), bool> {
    // Exact spellings first.
    for o in opts.iter().filter(|o| !o.long.is_empty()) {
        if o.long == name {
            return Ok((o, false));
        }
        if name.strip_prefix("no-") == Some(o.long) {
            return Ok((o, true));
        }
        if o.long.strip_prefix("no-") == Some(name) {
            return Ok((o, true));
        }
    }
    // Unique-prefix abbreviation (gitcli: long options may be abbreviated).
    let mut hit: Option<(&Opt, bool)> = None;
    let mut ambiguous = false;
    for o in opts.iter().filter(|o| !o.long.is_empty()) {
        let mut cands = vec![
            (o.long.to_string(), false),
            (format!("no-{}", o.long), true),
        ];
        if let Some(pos) = o.long.strip_prefix("no-") {
            cands.push((pos.to_string(), true));
        }
        for (spell, neg) in cands {
            if spell.starts_with(name) {
                match hit {
                    Some((h, hn)) if h.key != o.key || hn != neg => ambiguous = true,
                    _ => hit = Some((o, neg)),
                }
            }
        }
    }
    match (hit, ambiguous) {
        (Some(h), false) => Ok(h),
        _ => Err(ambiguous),
    }
}

/// Parse `args` against `spec`, allowing options and arguments to mix.
pub fn parse(args: &[String], spec: &[&'static str]) -> CliResult<Parsed> {
    parse_with(args, spec, false)
}

/// Parse `args`; with `stop_at_arg`, the first non-option ends option parsing.
pub fn parse_with(args: &[String], spec: &[&'static str], stop_at_arg: bool) -> CliResult<Parsed> {
    let opts = compile(spec);
    let mut out = Parsed::default();
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        i += 1;
        if a == "--" {
            out.args.extend(args[i..].iter().cloned());
            break;
        }
        if let Some(body) = a.strip_prefix("--") {
            let (name, inline) = match body.split_once('=') {
                Some((n, v)) => (n, Some(v.to_string())),
                None => (body, None),
            };
            let (o, neg) = find_long(&opts, name).map_err(|amb| {
                if amb {
                    CliError::Exit(129, format!("error: ambiguous option: {name}"))
                } else {
                    unknown_long(name)
                }
            })?;
            let value = match (o.kind, neg, inline) {
                (_, true, Some(_)) | (ArgKind::Flag, false, Some(_)) => {
                    return Err(CliError::Exit(
                        129,
                        format!("error: option `{name}' takes no value"),
                    ));
                }
                (ArgKind::Value, false, None) => {
                    let v = args.get(i).cloned().ok_or_else(|| {
                        CliError::Exit(129, format!("error: option `{name}' requires a value"))
                    })?;
                    i += 1;
                    Some(v)
                }
                (_, _, v) => v,
            };
            out.events.push(OptEvent {
                key: o.key,
                value,
                neg,
            });
            continue;
        }
        if a.len() > 1 && a.starts_with('-') {
            let chars: Vec<char> = a[1..].chars().collect();
            let mut j = 0;
            while j < chars.len() {
                let c = chars[j];
                let o = opts
                    .iter()
                    .find(|o| o.short == Some(c))
                    .ok_or_else(|| unknown_short(c))?;
                j += 1;
                let rest: String = chars[j..].iter().collect();
                let value = match o.kind {
                    ArgKind::Flag => None,
                    ArgKind::OptValue => {
                        j = chars.len();
                        (!rest.is_empty()).then_some(rest)
                    }
                    ArgKind::Value => {
                        j = chars.len();
                        if rest.is_empty() {
                            let v = args.get(i).cloned().ok_or_else(|| {
                                CliError::Exit(129, format!("error: switch `{c}' requires a value"))
                            })?;
                            i += 1;
                            Some(v)
                        } else {
                            Some(rest)
                        }
                    }
                };
                // A short spelling of a `no-<x>` option is its positive form.
                out.events.push(OptEvent {
                    key: o.key,
                    value,
                    neg: false,
                });
            }
            continue;
        }
        out.args.push(a.clone());
        if stop_at_arg {
            out.args.extend(args[i..].iter().cloned());
            break;
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(xs: &[&str]) -> Vec<String> {
        xs.iter().map(|s| s.to_string()).collect()
    }

    const SPEC: &[&str] = &[
        "v|verbose",
        "q|quiet",
        "j|jobs=",
        "|recurse-submodules=?",
        "n|no-checkout",
        "4|ipv4",
        "o|server-option=",
        "|tags",
    ];

    #[test]
    fn spellings() {
        let p = parse(
            &v(&[
                "-qv",
                "-j4",
                "--jobs",
                "2",
                "--no-tags",
                "--checkout",
                "-4",
                "pos",
                "-o",
                "x",
                "--server-option=y",
                "--recurse-submodules",
                "--",
                "-v",
            ]),
            SPEC,
        )
        .unwrap();
        assert_eq!(p.count("verbose"), 1);
        assert!(p.on("quiet"));
        assert_eq!(p.value("jobs").as_deref(), Some("2"));
        assert_eq!(p.flag("tags"), Some(false));
        assert_eq!(p.flag("no-checkout"), Some(false));
        assert!(p.on("ipv4"));
        assert_eq!(p.values("server-option"), v(&["x", "y"]));
        assert!(p.on("recurse-submodules"));
        assert_eq!(p.args, v(&["pos", "-v"]));
    }

    #[test]
    fn abbreviations_and_errors() {
        let p = parse(&v(&["--verb", "--no-verb", "--recurse=on-demand"]), SPEC).unwrap();
        assert_eq!(p.count("verbose"), 0);
        assert_eq!(p.value("recurse-submodules").as_deref(), Some("on-demand"));
        assert!(parse(&v(&["--bogus"]), SPEC).is_err());
        assert!(parse(&v(&["-x"]), SPEC).is_err());
        assert!(parse(&v(&["--jobs"]), SPEC).is_err());
        assert!(parse(&v(&["--tags=1"]), SPEC).is_err());
    }
}
