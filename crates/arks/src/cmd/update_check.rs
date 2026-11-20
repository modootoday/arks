//! `git update-git-for-windows [-y|--yes] [--gui] [--quiet] [--testing]`.
//!
//! Git for Windows uses this command to look for a newer installer. arks
//! keeps the name and options on every OS but checks for a newer **arks**
//! release and only reports it: nothing is downloaded or installed, and Git
//! for Windows is never touched (owner decision A3-2).
//!
//! The release feed is `ARKS_UPDATE_URL`, else config `arks.updateUrl`, else
//! the GitHub releases API of the arks repository (`/releases/latest`, or
//! `/releases` with `--testing` so pre-releases count). `http(s)://` and
//! `file://` feeds are read; the body is a GitHub release object or list, or
//! plain text whose first line is the version and second line a URL.
//! `--quiet` prints nothing when arks is up to date; `--yes` and `--gui` are
//! accepted for argv compatibility (there is no installer prompt or window).
//!
//! Plan: `.spec/plan/20261120000000-argv-symmetry-ladder.plan.md` (A3 design decisions 2).

use crate::cmd::opt_parse::{Mode, OptDef, Takes, o, parse};
use crate::error::{CliError, CliResult};
use std::cmp::Ordering;

const OPTS: &[OptDef] = &[
    o("yes", 'y', Takes::No, false),
    o("gui", ' ', Takes::No, false),
    o("quiet", ' ', Takes::No, false),
    o("testing", ' ', Takes::No, false),
];

const DEFAULT_FEED: &str = "https://api.github.com/repos/modootoday/arks/releases";

/// The newest release named by a feed.
#[derive(Debug, PartialEq, Eq)]
pub struct Release {
    pub version: String,
    pub url: Option<String>,
}

fn feed_url(testing: bool) -> String {
    if let Ok(u) = std::env::var("ARKS_UPDATE_URL") {
        if !u.is_empty() {
            return u;
        }
    }
    let cwd = std::env::current_dir().unwrap_or_default();
    if let Some(u) = arks_config::ConfigSet::load_discovered(&cwd).get("arks.updateUrl") {
        if !u.is_empty() {
            return u;
        }
    }
    if testing {
        DEFAULT_FEED.to_string()
    } else {
        format!("{DEFAULT_FEED}/latest")
    }
}

fn fetch(url: &str) -> CliResult<Vec<u8>> {
    if let Some(path) = url.strip_prefix("file://") {
        return std::fs::read(path)
            .map_err(|e| CliError::Exit(1, format!("error: could not read {url}: {e}")));
    }
    if url.starts_with("http://") || url.starts_with("https://") {
        return arks_remote::http_get(url)
            .map(|r| r.body)
            .map_err(|e| CliError::Exit(1, format!("error: could not fetch {url}: {e}")));
    }
    Err(CliError::Exit(
        1,
        format!("error: unsupported update feed URL '{url}' (http, https or file)"),
    ))
}

fn from_json(v: &serde_json::Value, testing: bool) -> Option<Release> {
    let pick = |r: &serde_json::Value| -> Option<Release> {
        if r.get("draft").and_then(|d| d.as_bool()) == Some(true) {
            return None;
        }
        if !testing && r.get("prerelease").and_then(|d| d.as_bool()) == Some(true) {
            return None;
        }
        Some(Release {
            version: r.get("tag_name")?.as_str()?.to_string(),
            url: r
                .get("html_url")
                .and_then(|u| u.as_str())
                .map(str::to_string),
        })
    };
    match v {
        serde_json::Value::Array(list) => list.iter().find_map(pick),
        _ => pick(v),
    }
}

/// Parse a feed body (GitHub release JSON or plain text).
pub fn parse_feed(body: &[u8], testing: bool) -> Option<Release> {
    let text = String::from_utf8_lossy(body);
    let t = text.trim_start();
    if t.starts_with('{') || t.starts_with('[') {
        let v: serde_json::Value = serde_json::from_str(t).ok()?;
        return from_json(&v, testing);
    }
    let mut lines = t.lines().map(str::trim).filter(|l| !l.is_empty());
    let version = lines.next()?.to_string();
    Some(Release {
        version,
        url: lines.next().map(str::to_string),
    })
}

/// Compare dotted versions (`v` prefix and `+build` ignored; a release sorts
/// after its `-pre` versions).
pub fn cmp_versions(a: &str, b: &str) -> Ordering {
    fn split(v: &str) -> (Vec<u64>, Option<String>) {
        let v = v.trim().trim_start_matches(['v', 'V']);
        let v = v.split('+').next().unwrap_or("");
        let (core, pre) = match v.split_once('-') {
            Some((c, p)) => (c, Some(p.to_string())),
            None => (v, None),
        };
        let nums = core
            .split('.')
            .map(|p| {
                p.chars()
                    .take_while(char::is_ascii_digit)
                    .collect::<String>()
                    .parse()
                    .unwrap_or(0)
            })
            .collect();
        (nums, pre)
    }
    let (an, ap) = split(a);
    let (bn, bp) = split(b);
    let n = an.len().max(bn.len());
    for i in 0..n {
        let (x, y) = (
            an.get(i).copied().unwrap_or(0),
            bn.get(i).copied().unwrap_or(0),
        );
        match x.cmp(&y) {
            Ordering::Equal => {}
            o => return o,
        }
    }
    match (ap, bp) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (Some(x), Some(y)) => x.cmp(&y),
    }
}

pub fn cmd_update_git_for_windows(args: &[String]) -> CliResult<()> {
    let p = parse("update-git-for-windows", args, OPTS, Mode::default())?;
    if !p.args().is_empty() || !p.after_dashdash().is_empty() {
        return Err(CliError::Exit(
            129,
            "usage: git update-git-for-windows [-y|--yes] [--gui] [--quiet] [--testing]".into(),
        ));
    }
    let testing = p.on("testing");
    let quiet = p.on("quiet");
    let url = feed_url(testing);
    let body = fetch(&url)?;
    let rel = parse_feed(&body, testing)
        .ok_or_else(|| CliError::Exit(1, format!("error: no release found in {url}")))?;
    let current = env!("CARGO_PKG_VERSION");
    if cmp_versions(&rel.version, current) == Ordering::Greater {
        println!(
            "A newer arks release is available: {} (installed: {current})",
            rel.version
        );
        if let Some(u) = &rel.url {
            println!("{u}");
        }
        println!("arks does not install updates (and never installs Git for Windows).");
    } else if !quiet {
        println!("arks {current} is up to date.");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_compare() {
        assert_eq!(cmp_versions("v1.2.0", "1.1.9"), Ordering::Greater);
        assert_eq!(cmp_versions("1.2", "1.2.0"), Ordering::Equal);
        assert_eq!(cmp_versions("1.2.0-rc1", "1.2.0"), Ordering::Less);
        assert_eq!(cmp_versions("0.0.0", "v0.0.1"), Ordering::Less);
        assert_eq!(cmp_versions("2.0.0+build.5", "2.0.0"), Ordering::Equal);
    }

    #[test]
    fn feeds_parse() {
        let obj =
            br#"{"tag_name":"v1.0.0","html_url":"https://example.invalid/r/1","prerelease":false}"#;
        assert_eq!(
            parse_feed(obj, false),
            Some(Release {
                version: "v1.0.0".into(),
                url: Some("https://example.invalid/r/1".into())
            })
        );
        let list = br#"[{"tag_name":"v2.0.0-rc1","prerelease":true},{"tag_name":"v1.5.0","draft":true},{"tag_name":"v1.4.0"}]"#;
        assert_eq!(parse_feed(list, false).unwrap().version, "v1.4.0");
        assert_eq!(parse_feed(list, true).unwrap().version, "v2.0.0-rc1");
        let text = b"\n1.9.0\nhttps://example.invalid/dl\n";
        assert_eq!(
            parse_feed(text, false).unwrap().url.as_deref(),
            Some("https://example.invalid/dl")
        );
        assert_eq!(parse_feed(b"[]", false), None);
        assert_eq!(parse_feed(b"{bad", false), None);
    }
}
