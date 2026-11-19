//! gh-symmetric client leaves for arks-owned forge groups
//! ([[adr:root/20261119230000]] item 7): `gist`, `discussion`, `project`,
//! `ssh-key`, `gpg-key`, `repo deploy-key`, `repo autolink`.
//!
//! Each leaf parses gh-shaped argv, talks to the forge REST API and prints
//! non-TTY gh-style output (tab-separated rows; `--json` / `--format json` where gh
//! has it). Leaves take an explicit [`Ctx`] and writer so tests can drive them
//! against an in-process forge. Never spawns git/gh.

pub(crate) mod autolink;
pub(crate) mod discussion;
pub(crate) mod gist;
pub(crate) mod keys;
pub(crate) mod project;

#[cfg(test)]
mod tests;

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
        let (o, n) = slug
            .split_once('/')
            .filter(|(o, n)| !o.is_empty() && !n.is_empty() && !n.contains('/'))
            .ok_or_else(|| Fail::Usage(format!("repo must be owner/name (got {slug})")))?;
        Ok((o.to_string(), n.to_string()))
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
pub(crate) fn run_env(label: &str, f: impl FnOnce(&Ctx, &mut dyn Write) -> Res) -> ExitCode {
    let ctx = Ctx::from_env();
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    finish(label, f(&ctx, &mut lock))
}

pub(crate) fn finish(label: &str, r: Res) -> ExitCode {
    match r {
        Ok(()) => ExitCode::SUCCESS,
        Err(Fail::Usage(m)) => {
            eprintln!("arks {label}: {m}");
            ExitCode::from(2)
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

/// gh-shaped argv view: value flags, boolean flags, positionals.
pub(crate) struct Args<'a> {
    raw: &'a [String],
    values: Vec<&'a str>,
    bools: Vec<&'a str>,
}

impl<'a> Args<'a> {
    /// Parse `raw` (args after the leaf name); refuses unknown flags like gh.
    pub fn parse(raw: &'a [String], values: &[&'a str], bools: &[&'a str]) -> Result<Self, Fail> {
        let a = Self {
            raw,
            values: values.to_vec(),
            bools: bools.to_vec(),
        };
        let mut i = 0;
        while i < raw.len() {
            let t = raw[i].as_str();
            if t == "--" {
                break;
            }
            if t.starts_with('-') && t != "-" {
                let name = t.split_once('=').map_or(t, |(n, _)| n);
                if values.contains(&name) {
                    if !t.contains('=') {
                        if i + 1 >= raw.len() {
                            return usage(format!("flag needs an argument: {name}"));
                        }
                        i += 1;
                    }
                } else if !bools.contains(&name) {
                    return usage(format!("unknown flag: {name}"));
                }
            }
            i += 1;
        }
        Ok(a)
    }

    /// Last value of any of `names`.
    pub fn get(&self, names: &[&str]) -> Option<&'a str> {
        let mut found = None;
        let mut i = 0;
        while i < self.raw.len() {
            let t = self.raw[i].as_str();
            if t == "--" {
                break;
            }
            for n in names {
                if t == *n && self.values.contains(n) {
                    found = self.raw.get(i + 1).map(String::as_str);
                } else if let Some(v) = t.strip_prefix(n).and_then(|r| r.strip_prefix('=')) {
                    found = Some(v);
                }
            }
            if self.values.contains(&t) {
                i += 1;
            }
            i += 1;
        }
        found
    }

    pub fn has(&self, names: &[&str]) -> bool {
        self.raw
            .iter()
            .take_while(|t| t.as_str() != "--")
            .any(|t| names.contains(&t.as_str()) && self.bools.contains(&t.as_str()))
    }

    /// Non-flag arguments before `--`.
    pub fn positionals(&self) -> Vec<&'a str> {
        let mut out = Vec::new();
        let mut i = 0;
        while i < self.raw.len() {
            let t = self.raw[i].as_str();
            if t == "--" {
                break;
            }
            if t.starts_with('-') && t != "-" {
                if self.values.contains(&t) {
                    i += 1;
                }
            } else {
                out.push(t);
            }
            i += 1;
        }
        out
    }

    /// Arguments after `--` (gh passes these to git; arks reports them unused).
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

/// Print `--json f1,f2` selection over gh-named rows (camelCase keys).
pub(crate) fn print_json_fields(
    out: &mut dyn Write,
    rows: &[Map<String, Value>],
    fields: &str,
    available: &[&str],
    single: bool,
) -> Res {
    let wanted: Vec<&str> = fields
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    if wanted.is_empty() {
        return usage(format!(
            "specify fields for --json: {}",
            available.join(", ")
        ));
    }
    if let Some(bad) = wanted.iter().find(|f| !available.contains(f)) {
        return usage(format!(
            "unknown JSON field: \"{bad}\" (available: {})",
            available.join(", ")
        ));
    }
    let pick = |r: &Map<String, Value>| -> Value {
        Value::Object(
            wanted
                .iter()
                .map(|f| (f.to_string(), r.get(*f).cloned().unwrap_or(Value::Null)))
                .collect(),
        )
    };
    let v = if single {
        rows.first().map(pick).unwrap_or(Value::Null)
    } else {
        Value::Array(rows.iter().map(pick).collect())
    };
    writeln!(
        out,
        "{}",
        serde_json::to_string_pretty(&v).unwrap_or_default()
    )?;
    Ok(())
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

/// Group entry: help on bare / `-h` / `--help`, else run `f` against env + stdout.
pub(crate) fn dispatch(
    label: &str,
    help: &str,
    args: &[String],
    f: fn(&Ctx, &[String], &mut dyn Write) -> Res,
) -> ExitCode {
    let first = args.first().map(String::as_str).unwrap_or("");
    if first.is_empty() || first == "help" || args.iter().any(|a| a == "-h" || a == "--help") {
        eprintln!("{help}");
        return ExitCode::SUCCESS;
    }
    let sub = format!("{label} {first}");
    run_env(&sub, |ctx, out| f(ctx, args, out))
}
