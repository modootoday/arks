//! gh-symmetric client leaves for arks-owned forge groups
//! ([[adr:root/20261119230000]] item 7): `gist`, `discussion`, `project`,
//! `ssh-key`, `gpg-key`, `repo deploy-key`, `repo autolink`.
//!
//! Each leaf parses gh-shaped argv, talks to the forge REST API and prints
//! non-TTY gh-style output (tab-separated rows; `--json` / `--format json` where gh
//! has it). Leaves take an explicit [`Ctx`] and writer so tests can drive them
//! against an in-process forge. Never spawns git/gh.

pub(crate) mod autolink;
pub(crate) mod common;
pub(crate) mod discussion;
pub(crate) mod gist;
pub(crate) mod gotmpl;
pub(crate) mod issue;
pub(crate) mod jq;
pub(crate) mod keys;
pub(crate) mod label;
pub(crate) mod misc;
pub(crate) mod pr;
pub(crate) mod project;
pub(crate) mod release;
pub(crate) mod repo;
pub(crate) mod ruleset;
pub(crate) mod search;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_gh;

use crate::http_min::http_exchange;
use serde_json::{Map, Value};
use std::io::{self, Read, Write};
use std::process::ExitCode;

/// Client context: forge base URL, token and default repository.
#[derive(Debug, Clone)]
pub(crate) struct Ctx {
    pub base: String,
    pub token: Option<String>,
    pub default_repo: Option<String>,
}

impl Ctx {
    pub fn from_env() -> Self {
        let default_repo = crate::repo_slug(&[]).ok().map(|(o, n)| format!("{o}/{n}"));
        Self {
            base: crate::api_base(),
            token: crate::read_token(),
            default_repo,
        }
    }

    pub fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base.trim_end_matches('/'))
    }

    /// Call the forge API; `Ok(Value::Null)` for empty bodies (204).
    pub fn api(&self, method: &str, path: &str, body: Option<&Value>) -> Result<Value, Fail> {
        let tok = self
            .token
            .as_deref()
            .ok_or_else(|| Fail::Auth("not logged in — arks auth login".into()))?;
        let payload = body.map(Value::to_string);
        let (code, text) = http_exchange(&self.base, method, path, Some(tok), payload.as_deref())
            .map_err(Fail::Api)?;
        let v: Value = if text.trim().is_empty() {
            Value::Null
        } else {
            serde_json::from_str(&text).unwrap_or(Value::String(text.clone()))
        };
        if code >= 400 {
            let msg = v
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("request failed");
            return Err(Fail::Api(format!("{msg} (HTTP {code})")));
        }
        Ok(if code == 204 { Value::Null } else { v })
    }

    /// `--repo` / `-R` or the default repository, as `(owner, name)`.
    pub fn repo(&self, a: &Args<'_>) -> Result<(String, String), Fail> {
        let slug = a
            .get(&["-R", "--repo"])
            .map(str::to_string)
            .or_else(|| self.default_repo.clone())
            .ok_or_else(|| {
                Fail::Usage("set --repo owner/name, ARKS_HUB_REPO, or repo set-default".into())
            })?;
        common::parse_slug(&slug).ok_or_else(|| {
            Fail::Usage(format!(
                "expected the \"[HOST/]OWNER/REPO\" format, got {slug:?}"
            ))
        })
    }
}

/// Failure classes mapped to gh-style exit codes.
#[derive(Debug)]
pub(crate) enum Fail {
    /// Bad argv / local validation → exit 2.
    Usage(String),
    /// Forge or transport error → exit 1.
    Api(String),
    /// No token → exit 4 (gh auth-required class).
    Auth(String),
    /// `--help`, or `-h` on a leaf that does not define `-h` → group help, exit 0.
    Help,
    /// Finished with a gh-defined status and nothing more to say
    /// (`pr checks`: 1 = a check failed, 8 = checks pending).
    Exit(u8),
}

pub(crate) type Res = Result<(), Fail>;

impl From<io::Error> for Fail {
    fn from(e: io::Error) -> Self {
        Fail::Api(e.to_string())
    }
}

pub(crate) fn usage<T>(msg: impl Into<String>) -> Result<T, Fail> {
    Err(Fail::Usage(msg.into()))
}

/// Run one leaf against the env-derived context and stdout.
pub(crate) fn run_env(
    label: &str,
    help: &str,
    f: impl FnOnce(&Ctx, &mut dyn Write) -> Res,
) -> ExitCode {
    let ctx = Ctx::from_env();
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    finish(label, help, f(&ctx, &mut lock))
}

pub(crate) fn finish(label: &str, help: &str, r: Res) -> ExitCode {
    match r {
        Ok(()) => ExitCode::SUCCESS,
        Err(Fail::Help) => {
            eprintln!("{help}");
            ExitCode::SUCCESS
        }
        Err(Fail::Exit(code)) => ExitCode::from(code),
        Err(Fail::Usage(m)) => {
            eprintln!("arks {label}: {m}");
            crate::usage_exit()
        }
        Err(Fail::Api(m)) => {
            eprintln!("arks {label}: {m}");
            ExitCode::from(1)
        }
        Err(Fail::Auth(m)) => {
            eprintln!("arks {label}: {m}");
            ExitCode::from(4)
        }
    }
}

/// gh-shaped argv view (pflag rules): value flags, boolean flags, positionals.
///
/// Accepts `--name value`, `--name=value`, `-x value`, `-xvalue`, `-x=value`,
/// bundled boolean shorthands (`-ab`, `-abL5`), `--bool=false`, and `--` (the
/// rest is passthrough). `--help`, or `-h` when the leaf does not define `-h`,
/// is [`Fail::Help`]. Unknown flags are refused like gh. A value flag listed as
/// `"--name?"` takes its value only inline (`--name=v`); alone it reads as `""`
/// (pflag `NoOptDefVal`, e.g. `browse --commit`).
pub(crate) struct Args<'a> {
    raw: &'a [String],
    flags: Vec<(String, Option<&'a str>)>,
    pos: Vec<&'a str>,
}

impl<'a> Args<'a> {
    pub fn parse(raw: &'a [String], values: &[&str], bools: &[&str]) -> Result<Self, Fail> {
        let mut flags = Vec::new();
        let mut pos = Vec::new();
        let mut i = 0;
        while i < raw.len() {
            let t = raw[i].as_str();
            if t == "--" {
                break;
            }
            if let Some(long) = t.strip_prefix("--") {
                let (name, inline) = match long.split_once('=') {
                    Some((n, v)) => (format!("--{n}"), Some(v)),
                    None => (t.to_string(), None),
                };
                if name == "--help" {
                    return Err(Fail::Help);
                }
                let optional = values.contains(&format!("{name}?").as_str());
                if optional && inline.is_none() {
                    flags.push((name, Some("")));
                } else if values.contains(&name.as_str()) || optional {
                    let v = match inline {
                        Some(v) => v,
                        None => {
                            i += 1;
                            raw.get(i).map(String::as_str).ok_or_else(|| {
                                Fail::Usage(format!("flag needs an argument: {name}"))
                            })?
                        }
                    };
                    flags.push((name, Some(v)));
                } else if bools.contains(&name.as_str()) {
                    if let Some(v) = inline {
                        if !matches!(v, "true" | "false" | "1" | "0" | "t" | "f") {
                            return usage(format!(
                                "invalid argument {v:?} for {name:?} flag: invalid syntax"
                            ));
                        }
                    }
                    flags.push((name, inline));
                } else {
                    return usage(format!("unknown flag: {name}"));
                }
            } else if t.len() > 1 && t.starts_with('-') {
                let mut rest = &t[1..];
                while let Some(c) = rest.chars().next() {
                    let name = format!("-{c}");
                    rest = &rest[c.len_utf8()..];
                    let optional = values.contains(&format!("{name}?").as_str());
                    let is_value = values.contains(&name.as_str()) || optional;
                    let is_bool = bools.contains(&name.as_str());
                    if c == 'h' && !is_value && !is_bool {
                        return Err(Fail::Help);
                    }
                    if is_value {
                        let v = if rest.is_empty() && optional {
                            ""
                        } else if rest.is_empty() {
                            i += 1;
                            raw.get(i).map(String::as_str).ok_or_else(|| {
                                Fail::Usage(format!("flag needs an argument: '{c}' in {t}"))
                            })?
                        } else {
                            rest.strip_prefix('=').unwrap_or(rest)
                        };
                        flags.push((name, Some(v)));
                        break;
                    } else if is_bool {
                        flags.push((name, None));
                    } else {
                        return usage(format!("unknown shorthand flag: '{c}' in {t}"));
                    }
                }
            } else {
                pos.push(t);
            }
            i += 1;
        }
        Ok(Self { raw, flags, pos })
    }

    /// Last value of any of `names`.
    pub fn get(&self, names: &[&str]) -> Option<&'a str> {
        self.flags
            .iter()
            .rev()
            .find(|(n, v)| v.is_some() && names.contains(&n.as_str()))
            .and_then(|(_, v)| *v)
    }

    /// Every value of `names`, in argv order (repeatable flags).
    pub fn get_all(&self, names: &[&str]) -> Vec<&'a str> {
        self.flags
            .iter()
            .filter(|(n, _)| names.contains(&n.as_str()))
            .filter_map(|(_, v)| *v)
            .collect()
    }

    /// Repeatable, comma-separated list flag (pflag `strings`).
    pub fn list(&self, names: &[&str]) -> Vec<String> {
        self.get_all(names)
            .iter()
            .flat_map(|v| v.split(','))
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect()
    }

    /// Boolean flag: `None` when absent, else its explicit or implied value.
    pub fn bool_opt(&self, names: &[&str]) -> Option<bool> {
        self.flags
            .iter()
            .rev()
            .find(|(n, _)| names.contains(&n.as_str()))
            .map(|(_, v)| !matches!(v.unwrap_or("true"), "false" | "0" | "f"))
    }

    pub fn has(&self, names: &[&str]) -> bool {
        self.bool_opt(names).unwrap_or(false)
    }

    /// Non-flag arguments before `--`.
    pub fn positionals(&self) -> Vec<&'a str> {
        self.pos.clone()
    }

    /// Arguments after `--` (gh hands these to its git; arks reports them unused).
    pub fn passthrough(&self) -> &'a [String] {
        match self.raw.iter().position(|t| t == "--") {
            Some(i) => &self.raw[i + 1..],
            None => &[],
        }
    }

    pub fn limit(&self, default: usize) -> Result<usize, Fail> {
        match self.get(&["-L", "--limit"]) {
            None => Ok(default),
            Some(s) => match s.parse::<usize>() {
                Ok(n) if n > 0 => Ok(n),
                _ => usage(format!("invalid limit: {s}")),
            },
        }
    }
}

/// gh export flags (`--json`, `-q/--jq`, `-t/--template`) as value flags.
pub(crate) const EXPORT: &[&str] = &["--json", "-q", "--jq", "-t", "--template"];

/// gh `--json fields`, optionally post-processed by `--jq` or `--template`.
pub(crate) struct Export {
    fields: Vec<String>,
    jq: Option<String>,
    template: Option<String>,
}

impl Export {
    /// `Ok(None)` when `--json` is absent (then `--jq` / `--template` are refused).
    pub fn parse(a: &Args<'_>, available: &[&str]) -> Result<Option<Self>, Fail> {
        let jq = a.get(&["-q", "--jq"]).map(str::to_string);
        let template = a.get(&["-t", "--template"]).map(str::to_string);
        if a.get(&["--json"]).is_none() {
            if jq.is_some() {
                return usage("cannot use `--jq` without specifying `--json`");
            }
            if template.is_some() {
                return usage("cannot use `--template` without specifying `--json`");
            }
            return Ok(None);
        }
        if jq.is_some() && template.is_some() {
            return usage("only one of `--jq` or `--template` may be used");
        }
        let fields = a.list(&["--json"]);
        if fields.is_empty() {
            return usage(format!(
                "Specify one or more comma-separated fields for `--json`:\n  {}",
                available.join("\n  ")
            ));
        }
        if let Some(bad) = fields.iter().find(|f| !available.contains(&f.as_str())) {
            return usage(format!(
                "Unknown JSON field: {bad:?}\nAvailable fields:\n  {}",
                available.join("\n  ")
            ));
        }
        Ok(Some(Self {
            fields,
            jq,
            template,
        }))
    }

    fn pick(&self, r: &Map<String, Value>) -> Value {
        Value::Object(
            self.fields
                .iter()
                .map(|f| (f.clone(), r.get(f).cloned().unwrap_or(Value::Null)))
                .collect(),
        )
    }

    pub fn rows(&self, out: &mut dyn Write, rows: &[Map<String, Value>]) -> Res {
        let v = Value::Array(rows.iter().map(|r| self.pick(r)).collect());
        emit(out, &v, self.jq.as_deref(), self.template.as_deref())
    }

    pub fn one(&self, out: &mut dyn Write, row: &Map<String, Value>) -> Res {
        emit(
            out,
            &self.pick(row),
            self.jq.as_deref(),
            self.template.as_deref(),
        )
    }
}

/// Write `v` as pretty JSON, or through `--jq` / `--template` when given.
pub(crate) fn emit(
    out: &mut dyn Write,
    v: &Value,
    jq: Option<&str>,
    template: Option<&str>,
) -> Res {
    if let Some(expr) = jq {
        for line in jq::run(expr, v).map_err(Fail::Api)? {
            writeln!(out, "{line}")?;
        }
        return Ok(());
    }
    if let Some(t) = template {
        write!(out, "{}", gotmpl::run(t, v).map_err(Fail::Api)?)?;
        return Ok(());
    }
    print_json(out, v)
}

/// gh `--web`: arks never launches a browser; it prints the URL it would open.
pub(crate) fn web(out: &mut dyn Write, url: &str) -> Res {
    eprintln!("Opening {url} in your browser.");
    eprintln!("note: arks does not launch a browser; open the URL yourself");
    writeln!(out, "{url}")?;
    Ok(())
}

/// Read a file, or stdin for `None` / `-`.
pub(crate) fn read_input(path: Option<&str>) -> Result<String, Fail> {
    match path {
        None | Some("-") => {
            let mut s = String::new();
            io::stdin().read_to_string(&mut s)?;
            Ok(s)
        }
        Some(p) => {
            std::fs::read_to_string(p).map_err(|e| Fail::Usage(format!("failed to read {p}: {e}")))
        }
    }
}

/// gh `--json` (+ `--jq` / `--template`) over gh-named rows (camelCase keys).
/// Returns `true` when the export flags handled the output.
pub(crate) fn json_out(
    out: &mut dyn Write,
    a: &Args<'_>,
    rows: &[Map<String, Value>],
    available: &[&str],
    single: bool,
) -> Result<bool, Fail> {
    let Some(x) = Export::parse(a, available)? else {
        return Ok(false);
    };
    match (single, rows.first()) {
        (true, Some(r)) => x.one(out, r)?,
        (true, None) => emit(out, &Value::Null, None, None)?,
        (false, _) => x.rows(out, rows)?,
    }
    Ok(true)
}

/// `print_json` through the export filters (`--format json` leaves).
pub(crate) fn json_filtered(out: &mut dyn Write, a: &Args<'_>, v: &Value) -> Res {
    emit(out, v, a.get(&["-q", "--jq"]), a.get(&["-t", "--template"]))
}

pub(crate) fn print_json(out: &mut dyn Write, v: &Value) -> Res {
    writeln!(
        out,
        "{}",
        serde_json::to_string_pretty(v).unwrap_or_default()
    )?;
    Ok(())
}

/// One tab-separated row (gh non-TTY table form).
pub(crate) fn row(out: &mut dyn Write, cols: &[&str]) -> Res {
    let cleaned: Vec<String> = cols.iter().map(|c| c.replace(['\t', '\n'], " ")).collect();
    writeln!(out, "{}", cleaned.join("\t"))?;
    Ok(())
}

pub(crate) fn s<'v>(v: &'v Value, key: &str) -> &'v str {
    v.get(key).and_then(Value::as_str).unwrap_or("")
}

pub(crate) fn n(v: &Value, key: &str) -> u64 {
    v.get(key).and_then(Value::as_u64).unwrap_or(0)
}

pub(crate) fn arr(v: &Value) -> &[Value] {
    v.as_array().map(Vec::as_slice).unwrap_or(&[])
}

/// Group entry: help on bare / `help` / `--help`, else run `f` against env + stdout
/// (a leaf's own parser turns `--help` or an undeclared `-h` into the same help).
pub(crate) fn dispatch(
    label: &str,
    help: &str,
    args: &[String],
    f: fn(&Ctx, &[String], &mut dyn Write) -> Res,
) -> ExitCode {
    let first = args.first().map(String::as_str).unwrap_or("");
    let wants_help = args
        .iter()
        .take_while(|a| a.as_str() != "--")
        .any(|a| a == "--help");
    if first.is_empty() || first == "help" || first == "-h" || wants_help {
        eprintln!("{help}");
        return ExitCode::SUCCESS;
    }
    // A group run with flags but no command: gh accepts the group's inherited
    // `-R/--repo` (and shows the group help); anything else is an unknown flag.
    if first.starts_with('-') {
        let name = first.split_once('=').map_or(first, |(n, _)| n);
        if matches!(name, "-R" | "--repo") {
            eprintln!("{help}");
            return ExitCode::SUCCESS;
        }
        eprintln!("arks {label}: unknown flag: {name}");
        return crate::usage_exit();
    }
    let sub = format!("{label} {first}");
    run_env(&sub, help, |ctx, out| f(ctx, args, out))
}

/// Leaf without subcommands (`status`, `browse`, `api`, …): help on `help` /
/// `--help`, else run `f` with every argument.
pub(crate) fn flat(
    label: &str,
    help: &str,
    args: &[String],
    f: fn(&Ctx, &[String], &mut dyn Write) -> Res,
) -> ExitCode {
    let wants_help = args
        .iter()
        .take_while(|a| a.as_str() != "--")
        .any(|a| a == "--help")
        || args.first().map(String::as_str) == Some("help");
    if wants_help {
        eprintln!("{help}");
        return ExitCode::SUCCESS;
    }
    run_env(label, help, |ctx, out| f(ctx, args, out))
}

#[cfg(test)]
mod args_tests {
    use super::{Args, EXPORT, Export, Fail};
    use serde_json::{Map, json};

    fn v(s: &str) -> Vec<String> {
        s.split_whitespace().map(String::from).collect()
    }

    #[test]
    fn pflag_spellings() {
        let raw = v("-dL5 --title=x -l a,b --label c pos --draft=false -- --rest");
        let a = Args::parse(
            &raw,
            &["-L", "--title", "-l", "--label"],
            &["-d", "--draft"],
        )
        .unwrap();
        assert_eq!(a.get(&["-L"]), Some("5"));
        assert_eq!(a.get(&["--title"]), Some("x"));
        assert_eq!(a.list(&["-l", "--label"]), ["a", "b", "c"]);
        assert!(!a.has(&["-d", "--draft"]), "last --draft=false wins");
        assert_eq!(a.bool_opt(&["--draft"]), Some(false));
        assert_eq!(a.positionals(), ["pos"]);
        assert_eq!(a.passthrough(), ["--rest"]);
    }

    #[test]
    fn refusals_and_help() {
        let unknown = |s: &str| match Args::parse(&v(s), &["-L"], &[]) {
            Err(Fail::Usage(m)) => m,
            _ => String::new(),
        };
        assert!(unknown("--nope").contains("unknown flag"));
        assert!(unknown("-z").contains("unknown shorthand flag"));
        assert!(unknown("-L").contains("needs an argument"));
        assert!(matches!(
            Args::parse(&v("--help"), &[], &[]),
            Err(Fail::Help)
        ));
        assert!(matches!(Args::parse(&v("-h"), &[], &[]), Err(Fail::Help)));
        // A leaf that defines -h (gh --hostname) keeps it.
        let raw = v("-h example.org");
        let a = Args::parse(&raw, &["-h", "--hostname"], &[]).unwrap();
        assert_eq!(a.get(&["-h", "--hostname"]), Some("example.org"));
    }

    #[test]
    fn export_json_jq_template() {
        let mut row = Map::new();
        row.insert("number".into(), json!(7));
        row.insert("title".into(), json!("t"));
        let raw = v("--json number --jq .[0].number");
        let a = Args::parse(&raw, EXPORT, &[]).unwrap();
        let x = Export::parse(&a, &["number", "title"]).unwrap().unwrap();
        let mut out = Vec::new();
        x.rows(&mut out, std::slice::from_ref(&row)).unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), "7\n");
        let raw = v("--json title -t {{.title}}");
        let a = Args::parse(&raw, EXPORT, &[]).unwrap();
        let mut out = Vec::new();
        Export::parse(&a, &["title"])
            .unwrap()
            .unwrap()
            .one(&mut out, &row)
            .unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), "t");
        for bad in ["--jq .x", "--json nope", "--json title --jq . -t x"] {
            let raw = v(bad);
            let a = Args::parse(&raw, EXPORT, &[]).unwrap();
            assert!(Export::parse(&a, &["title"]).is_err(), "{bad}");
        }
    }
}
