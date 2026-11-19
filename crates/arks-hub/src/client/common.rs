//! Shared client plumbing for the C8 repo/issue leaves: API context, argv
//! positionals, `--json <fields>` selection and HTTP helpers. Never spawns git/gh.

use crate::http_min::http_exchange;
use serde_json::{Map, Value};
use std::process::ExitCode;

/// Authenticated API context for one `owner/repo`.
pub struct Ctx {
    pub base: String,
    pub tok: String,
    pub owner: String,
    pub repo: String,
}

impl Ctx {
    pub fn slug(&self) -> String {
        format!("{}/{}", self.owner, self.repo)
    }
}

/// Resolve API base + token + repo slug (`--repo`/`ARKS_HUB_REPO`/default repo).
pub fn ctx(args: &[String], label: &str) -> Result<Ctx, ExitCode> {
    let (base, tok) = crate::require_api().map_err(|e| crate::exit_from_api_err(label, &e))?;
    let (owner, repo) = crate::repo_slug(args).map_err(|e| usage(label, &e))?;
    Ok(Ctx {
        base,
        tok,
        owner,
        repo,
    })
}

/// Print `arks hub <label>: <msg>` and return the usage/refuse exit (2).
pub fn usage(label: &str, m: &str) -> ExitCode {
    eprintln!("arks hub {label}: {m}");
    ExitCode::from(2)
}

/// Print `arks hub <label>: <msg>` and return the failure exit (1).
pub fn fail(label: &str, m: &str) -> ExitCode {
    eprintln!("arks hub {label}: {m}");
    ExitCode::from(1)
}

/// Positional arguments after the leaf, skipping flags and their values.
///
/// `value_flags` lists flags that consume the next argument; `--flag=value`
/// forms never consume. Everything after a bare `--` is excluded.
pub fn positionals(args: &[String], value_flags: &[&str]) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        if a == "--" {
            break;
        }
        if value_flags.contains(&a) {
            i += 2;
            continue;
        }
        if a.starts_with('-') && a.len() > 1 {
            i += 1;
            continue;
        }
        out.push(args[i].clone());
        i += 1;
    }
    out
}

/// Arguments after a bare `--` (pass-through flags).
pub fn after_double_dash(args: &[String]) -> Vec<String> {
    args.iter()
        .position(|a| a == "--")
        .map(|p| args[p + 1..].to_vec())
        .unwrap_or_default()
}

pub fn has_flag(args: &[String], names: &[&str]) -> bool {
    args.iter()
        .take_while(|a| *a != "--")
        .any(|a| names.contains(&a.as_str()))
}

/// `--json <fields>` request: `Ok(None)` when absent; refuse `--jq` / `--template`.
pub fn json_fields(
    args: &[String],
    label: &str,
    known: &[&str],
) -> Result<Option<Vec<String>>, ExitCode> {
    if crate::flag_value(args, &["--jq", "-q"]).is_some()
        || crate::flag_value(args, &["--template"]).is_some()
    {
        return Err(usage(
            label,
            "--jq / --template are not implemented (Partial); use --json <fields>",
        ));
    }
    let Some(spec) = crate::flag_value(args, &["--json"]) else {
        return Ok(None);
    };
    let fields: Vec<String> = spec
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if fields.is_empty() {
        return Err(usage(
            label,
            &format!("--json requires fields; available: {}", known.join(", ")),
        ));
    }
    if let Some(bad) = fields.iter().find(|f| !known.contains(&f.as_str())) {
        return Err(usage(
            label,
            &format!(
                "unknown JSON field `{bad}`; available: {}",
                known.join(", ")
            ),
        ));
    }
    Ok(Some(fields))
}

/// Keep only `fields` from a gh-shaped object.
pub fn select(obj: &Map<String, Value>, fields: &[String]) -> Value {
    let mut out = Map::new();
    for f in fields {
        out.insert(f.clone(), obj.get(f).cloned().unwrap_or(Value::Null));
    }
    Value::Object(out)
}

pub fn print_json(v: &Value) {
    println!(
        "{}",
        serde_json::to_string_pretty(v).unwrap_or_else(|_| v.to_string())
    );
}

/// Call the API; non-2xx maps to exit 1 with the server message.
pub fn call(
    ctx_base: &str,
    tok: &str,
    method: &str,
    path: &str,
    body: Option<&Value>,
    label: &str,
) -> Result<Value, ExitCode> {
    let payload = body.map(|b| b.to_string());
    match http_exchange(ctx_base, method, path, Some(tok), payload.as_deref()) {
        Ok((code, text)) if code < 400 => {
            if text.trim().is_empty() {
                Ok(Value::Null)
            } else {
                Ok(serde_json::from_str(&text).unwrap_or(Value::Null))
            }
        }
        Ok((code, text)) => {
            let m = serde_json::from_str::<Value>(&text)
                .ok()
                .and_then(|v| v.get("message").and_then(Value::as_str).map(str::to_string))
                .unwrap_or(text);
            Err(fail(label, &format!("HTTP {code}: {m}")))
        }
        Err(e) => Err(fail(label, &e)),
    }
}

pub fn str_of<'a>(v: &'a Value, k: &str) -> &'a str {
    v.get(k).and_then(Value::as_str).unwrap_or("")
}

/// Parse `{<number> | <url>}` issue references; URLs override the repo slug.
pub fn issue_ref(arg: &str) -> Option<(Option<(String, String)>, u64)> {
    if let Ok(n) = arg.trim_start_matches('#').parse::<u64>() {
        return Some((None, n));
    }
    let tail = arg.split("://").nth(1).unwrap_or(arg);
    let segs: Vec<&str> = tail.split('/').collect();
    let pos = segs.iter().position(|s| *s == "issues")?;
    if pos < 2 {
        return None;
    }
    let n = segs.get(pos + 1)?.parse().ok()?;
    Some((Some((segs[pos - 2].into(), segs[pos - 1].into())), n))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(a: &[&str]) -> Vec<String> {
        a.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn positionals_skip_flags() {
        let a = v(&["142", "-R", "o/r", "--json=x", "2", "--", "--depth", "1"]);
        assert_eq!(positionals(&a, &["-R"]), v(&["142", "2"]));
        assert_eq!(after_double_dash(&a), v(&["--depth", "1"]));
    }

    #[test]
    fn issue_refs() {
        assert_eq!(issue_ref("12"), Some((None, 12)));
        assert_eq!(
            issue_ref("https://github.com/o/r/issues/7/artifacts/2"),
            Some((Some(("o".into(), "r".into())), 7))
        );
        assert_eq!(issue_ref("nope"), None);
    }
}
