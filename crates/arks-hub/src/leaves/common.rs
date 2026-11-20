//! Helpers shared by the gh-symmetric client leaves: API calls with raw bodies,
//! `@me`, `[HOST/]OWNER/REPO` and URL arguments, the local repository the command
//! runs in (read in-process with the arks Git crates; never spawns git), body and
//! editor input, and gh's non-TTY text conventions.

use super::{Args, Ctx, Fail, read_input, usage};
use crate::forge::git::GitRepo;
use crate::http_min::http_exchange_bytes;
use serde_json::Value;
use std::path::PathBuf;

/// `owner/name` from `owner/name`, `host/owner/name` or a forge URL.
pub(crate) fn parse_slug(s: &str) -> Option<(String, String)> {
    let s = s.trim().trim_end_matches('/').trim_end_matches(".git");
    let s = s
        .split_once("://")
        .map(|(_, r)| r.split_once('/').map_or("", |(_, p)| p))
        .unwrap_or(s);
    let parts: Vec<&str> = s.split('/').filter(|p| !p.is_empty()).collect();
    let (o, n) = match parts.as_slice() {
        [o, n] => (*o, *n),
        [_, o, n] if !s.contains("://") => (*o, *n),
        [o, n, ..] => (*o, *n),
        _ => return None,
    };
    Some((o.to_string(), n.to_string()))
}

/// Number from `123`, `#123`, or a URL ending in `/<kind>/123[...]`.
pub(crate) fn parse_number(s: &str) -> Option<u64> {
    let s = s.trim().trim_start_matches('#');
    if let Ok(n) = s.parse() {
        return Some(n);
    }
    let parts: Vec<&str> = s.split(['/', '#', '?']).collect();
    parts
        .windows(2)
        .find(|w| matches!(w[0], "pull" | "pulls" | "issues" | "discussions"))
        .and_then(|w| w[1].parse().ok())
}

/// Repository named in a URL argument (`…/owner/name/pull/1`).
pub(crate) fn url_repo(s: &str) -> Option<(String, String)> {
    if !s.contains("://") {
        return None;
    }
    let path = s.split_once("://")?.1.split_once('/')?.1;
    let parts: Vec<&str> = path.split('/').collect();
    match parts.as_slice() {
        [o, n, kind, ..] if matches!(*kind, "pull" | "issues" | "releases" | "discussions") => {
            Some((o.to_string(), n.to_string()))
        }
        _ => None,
    }
}

impl Ctx {
    pub fn get(&self, path: &str) -> Result<Value, Fail> {
        self.api("GET", path, None)
    }

    /// Raw exchange (asset bytes, archives, diffs); errors on HTTP ≥ 400.
    pub fn raw(
        &self,
        method: &str,
        path: &str,
        body: Option<&[u8]>,
        content_type: &str,
        accept: &str,
    ) -> Result<Vec<u8>, Fail> {
        let tok = self
            .token
            .as_deref()
            .ok_or_else(|| Fail::Auth("not logged in — arks auth login".into()))?;
        let (code, bytes) = http_exchange_bytes(
            &self.base,
            method,
            path,
            Some(tok),
            body,
            content_type,
            accept,
        )
        .map_err(Fail::Api)?;
        if code >= 400 {
            let v: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
            let msg = v
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("request failed");
            return Err(Fail::Api(format!("{msg} (HTTP {code})")));
        }
        Ok(bytes)
    }

    /// Login of the authenticated account.
    pub fn me(&self) -> Result<String, Fail> {
        let u = self.get("/user")?;
        Ok(u.get("login")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string())
    }

    /// Replace `@me` with the authenticated login.
    pub fn who(&self, s: &str) -> Result<String, Fail> {
        if s == "@me" {
            self.me()
        } else {
            Ok(s.to_string())
        }
    }

    pub fn whos(&self, list: &[String]) -> Result<Vec<String>, Fail> {
        list.iter().map(|s| self.who(s)).collect()
    }

    /// Web URL for a forge path (`/owner/name/pull/1`).
    pub fn web_url(&self, path: &str) -> String {
        self.url(path)
    }
}

/// The local Git repository the command runs in, read in-process.
pub(crate) struct Local {
    pub repo: GitRepo,
    pub git_dir: PathBuf,
}

impl Local {
    pub fn discover() -> Option<Self> {
        let d = arks_repo::discover(std::env::current_dir().ok()?).ok()?;
        let git_dir = d.git_dir.clone();
        Some(Self {
            repo: GitRepo::open(&git_dir).ok()?,
            git_dir,
        })
    }

    /// Current branch (`None` when detached or unborn without a name).
    pub fn branch(&self) -> Option<String> {
        let head = std::fs::read_to_string(self.git_dir.join("HEAD")).ok()?;
        head.trim()
            .strip_prefix("ref: refs/heads/")
            .map(str::to_string)
    }
}

/// `--body` / `--body-file` (`-` = stdin) → `Some(text)`.
pub(crate) fn body_of(a: &Args<'_>, body: &[&str], file: &[&str]) -> Result<Option<String>, Fail> {
    if let Some(b) = a.get(body) {
        return Ok(Some(b.to_string()));
    }
    match a.get(file) {
        Some(f) => read_input(Some(f)).map(Some),
        None => Ok(None),
    }
}

/// Run the configured editor (`GH_EDITOR`, `VISUAL`, `EDITOR`) on `initial`.
/// arks starts the user's editor only; it never spawns git or gh.
pub(crate) fn edit_text(initial: &str) -> Result<String, Fail> {
    let editor = ["ARKS_EDITOR", "GH_EDITOR", "VISUAL", "EDITOR"]
        .iter()
        .find_map(|k| std::env::var(k).ok().filter(|v| !v.trim().is_empty()))
        .ok_or_else(|| {
            Fail::Usage("--editor needs an editor: set GH_EDITOR, VISUAL or EDITOR".into())
        })?;
    let path = std::env::temp_dir().join(format!(
        "arks-edit-{}-{}.md",
        std::process::id(),
        crate::store::now_secs()
    ));
    std::fs::write(&path, initial)?;
    let mut parts = editor.split_whitespace();
    let prog = parts.next().unwrap_or("vi");
    let status = std::process::Command::new(prog)
        .args(parts)
        .arg(&path)
        .status()
        .map_err(|e| Fail::Usage(format!("could not start editor {prog:?}: {e}")))?;
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let _ = std::fs::remove_file(&path);
    if !status.success() {
        return usage(format!("editor {prog:?} exited with {status}"));
    }
    Ok(text)
}

/// Split editor text into title (first line) and body (the rest).
pub(crate) fn title_body(text: &str) -> (String, String) {
    let mut lines = text.lines();
    let title = lines.next().unwrap_or("").trim().to_string();
    let body = lines.collect::<Vec<_>>().join("\n").trim().to_string();
    (title, body)
}

/// gh upper-case state words (`OPEN`, `CLOSED`, `MERGED`, `DRAFT`).
pub(crate) fn upper(s: &str) -> String {
    s.to_ascii_uppercase()
}

/// `names` of a JSON array of `{name|login}` objects, joined by `", "`.
pub(crate) fn names(v: &Value, key: &str) -> String {
    v.get(key)
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|x| {
                    x.get("name")
                        .or_else(|| x.get("login"))
                        .and_then(Value::as_str)
                })
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default()
}

/// Write the failed input of a `create` to a file gh's `--recover` can read back.
pub(crate) fn save_recover(kind: &str, input: &Value) -> Option<PathBuf> {
    let path = std::env::temp_dir().join(format!(
        "arks-{kind}-create-{}.json",
        crate::store::now_secs()
    ));
    std::fs::write(&path, serde_json::to_vec_pretty(input).ok()?).ok()?;
    Some(path)
}

pub(crate) fn load_recover(path: &str) -> Result<Value, Fail> {
    let raw = std::fs::read_to_string(path)
        .map_err(|e| Fail::Usage(format!("failed to read {path}: {e}")))?;
    serde_json::from_str(&raw).map_err(|e| Fail::Usage(format!("invalid recover file {path}: {e}")))
}

/// Seconds since the epoch for an RFC 3339 UTC timestamp (`2026-01-02T03:04:05Z`).
pub(crate) fn iso_secs(s: &str) -> Option<u64> {
    let b = s.as_bytes();
    if b.len() < 19 {
        return None;
    }
    let num = |a: usize, z: usize| s.get(a..z)?.parse::<i64>().ok();
    let (y, m, d) = (num(0, 4)?, num(5, 7)?, num(8, 10)?);
    let (hh, mm, ss) = (num(11, 13)?, num(14, 16)?, num(17, 19)?);
    let y2 = if m <= 2 { y - 1 } else { y };
    let era = y2.div_euclid(400);
    let yoe = y2 - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    u64::try_from(days * 86_400 + hh * 3_600 + mm * 60 + ss).ok()
}

/// Percent-encode a query value.
pub(crate) fn enc(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
            out.push(char::from(b));
        } else if b == b' ' {
            out.push('+');
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// Percent-encode one path segment.
pub(crate) fn seg(s: &str) -> String {
    enc(s).replace('+', "%20")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs_and_numbers() {
        assert_eq!(parse_slug("o/r"), Some(("o".into(), "r".into())));
        assert_eq!(
            parse_slug("host.example/o/r"),
            Some(("o".into(), "r".into()))
        );
        assert_eq!(
            parse_slug("http://127.0.0.1:9/o/r.git"),
            Some(("o".into(), "r".into()))
        );
        assert_eq!(parse_slug("nope"), None);
        assert_eq!(parse_number("#12"), Some(12));
        assert_eq!(parse_number("http://h/o/r/pull/7/files"), Some(7));
        assert_eq!(
            url_repo("http://h/o/r/issues/3"),
            Some(("o".into(), "r".into()))
        );
        assert_eq!(enc("a b/c"), "a+b%2Fc");
        assert_eq!(iso_secs("2023-11-14T22:13:20Z"), Some(1_700_000_000));
        assert_eq!(
            iso_secs(&crate::forge::iso8601(951_782_400)),
            Some(951_782_400)
        );
    }
}
