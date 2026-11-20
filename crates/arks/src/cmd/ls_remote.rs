//! `ls-remote`: list a peer's advertised refs with `--heads` / `--tags` /
//! `--refs` filters, tail patterns, `--symref`, `--sort`, `--get-url` and
//! `--exit-code`.
//!
//! Primary: <https://git-scm.com/docs/git-ls-remote>,
//! <https://git-scm.com/docs/git-for-each-ref#_field_names> (sort keys).

use crate::cmd::fetch::{configured_source, default_remote};
use crate::cmd::opt_spec::parse;
use crate::error::{CliError, CliResult};
use crate::util::open_repo;
use std::cmp::Ordering;

const LS_REMOTE_OPTS: &[&str] = &[
    "q|quiet",
    "|upload-pack=",
    "|exec=",
    "t|tags",
    "h|heads",
    "b|branches",
    "|refs",
    "|get-url",
    "|sort=",
    "|exit-code",
    "|symref",
    "o|server-option=",
];

/// Shell-glob match (`*`, `?`, `[...]`) of `pat` against all of `s`.
pub(crate) fn glob_match(pat: &[u8], s: &[u8]) -> bool {
    match (pat.first(), s.first()) {
        (None, None) => true,
        (Some(b'*'), _) => glob_match(&pat[1..], s) || (!s.is_empty() && glob_match(pat, &s[1..])),
        (Some(b'?'), Some(_)) => glob_match(&pat[1..], &s[1..]),
        (Some(b'['), Some(c)) => match pat.iter().position(|&b| b == b']') {
            Some(end) if end > 1 => {
                let set = &pat[1..end];
                let (neg, set) = match set.first() {
                    Some(b'!') | Some(b'^') => (true, &set[1..]),
                    _ => (false, set),
                };
                let mut hit = false;
                let mut i = 0;
                while i < set.len() {
                    if i + 2 < set.len() && set[i + 1] == b'-' {
                        hit |= set[i] <= *c && *c <= set[i + 2];
                        i += 3;
                    } else {
                        hit |= set[i] == *c;
                        i += 1;
                    }
                }
                hit != neg && glob_match(&pat[end + 1..], &s[1..])
            }
            _ => *c == b'[' && glob_match(&pat[1..], &s[1..]),
        },
        (Some(p), Some(c)) => p == c && glob_match(&pat[1..], &s[1..]),
        _ => false,
    }
}

/// git-ls-remote patterns match the tail of a ref name at a `/` boundary.
fn tail_matches(pattern: &str, name: &str) -> bool {
    let name = name.strip_suffix("^{}").unwrap_or(name);
    let mut starts = vec![0];
    starts.extend(name.match_indices('/').map(|(i, _)| i + 1));
    starts
        .iter()
        .any(|&i| glob_match(pattern.as_bytes(), &name.as_bytes()[i..]))
}

/// `version:refname` ordering: digit runs compare numerically.
pub(crate) fn version_cmp(a: &str, b: &str) -> Ordering {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let (mut i, mut j) = (0, 0);
    while i < a.len() && j < b.len() {
        if a[i].is_ascii_digit() && b[j].is_ascii_digit() {
            let si = i;
            while i < a.len() && a[i].is_ascii_digit() {
                i += 1;
            }
            let sj = j;
            while j < b.len() && b[j].is_ascii_digit() {
                j += 1;
            }
            let x = std::str::from_utf8(&a[si..i])
                .unwrap_or("0")
                .trim_start_matches('0');
            let y = std::str::from_utf8(&b[sj..j])
                .unwrap_or("0")
                .trim_start_matches('0');
            let o = x.len().cmp(&y.len()).then_with(|| x.cmp(y));
            if o != Ordering::Equal {
                return o;
            }
        } else {
            if a[i] != b[j] {
                return a[i].cmp(&b[j]);
            }
            i += 1;
            j += 1;
        }
    }
    (a.len() - i).cmp(&(b.len() - j))
}

pub fn cmd_ls_remote(args: &[String]) -> CliResult<()> {
    let p = parse(args, LS_REMOTE_OPTS)?;
    let heads = p.on("heads") || p.on("branches");
    let tags = p.on("tags");
    let repo = open_repo().ok();
    let cfg = arks_config::ConfigSet::load(repo.as_ref().map(|r| r.common_dir()));
    let explicit = p.args.first().cloned();
    let remote = match (&explicit, &repo) {
        (Some(r), _) => r.clone(),
        (None, Some(r)) => default_remote(r, &cfg),
        (None, None) => {
            return Err(CliError::fatal("No remote configured to list refs from."));
        }
    };
    let url = match configured_source(&cfg, &remote) {
        Some(s) => s.url,
        None if explicit.is_none() => {
            return Err(CliError::fatal("No remote configured to list refs from."));
        }
        None => remote.clone(),
    };
    if p.on("get-url") {
        println!("{url}");
        return Ok(());
    }
    if explicit.is_none() && !p.on("quiet") {
        eprintln!("From {url}");
    }
    if !arks_remote::is_local_remote(&url)
        && arks_remote::HttpRemoteUrl::parse(&url).is_err()
        && arks_remote::SshRemoteUrl::parse(&url).is_err()
        && arks_remote::GitDaemonUrl::parse(&url).is_err()
    {
        return Err(CliError::fatal(format!(
            "'{url}' does not appear to be a git repository\nfatal: Could not read from remote repository.\n\nPlease make sure you have the correct access rights\nand the repository exists."
        )));
    }
    let patterns: Vec<&String> = p.args.iter().skip(1).collect();
    let refs = arks_remote::ls_remote(&url).map_err(|e| CliError::fatal(e.to_string()))?;
    let mut rows: Vec<(String, String)> = refs
        .into_iter()
        .filter(|r| {
            let n = r.name.as_str();
            if p.on("refs") && (!n.starts_with("refs/") || n.ends_with("^{}")) {
                return false;
            }
            if (heads || tags)
                && !((heads && n.starts_with("refs/heads/"))
                    || (tags && n.starts_with("refs/tags/")))
            {
                return false;
            }
            patterns.is_empty() || patterns.iter().any(|pat| tail_matches(pat, n))
        })
        .map(|r| (r.oid.to_hex(), r.name))
        .collect();
    for key in p.values("sort").iter().rev() {
        let (rev, k) = match key.strip_prefix('-') {
            Some(k) => (true, k),
            None => (false, key.as_str()),
        };
        let cmp = |a: &(String, String), b: &(String, String)| -> Ordering {
            match k {
                "refname" => a.1.cmp(&b.1),
                "version:refname" | "v:refname" => version_cmp(&a.1, &b.1),
                "objectname" => a.0.cmp(&b.0),
                _ => Ordering::Equal,
            }
        };
        if !matches!(
            k,
            "refname" | "version:refname" | "v:refname" | "objectname"
        ) {
            return Err(CliError::fatal(format!("unknown field name: {k}")));
        }
        if rev {
            rows.sort_by(|a, b| cmp(b, a));
        } else {
            rows.sort_by(cmp);
        }
    }
    let symref_head = if p.on("symref") && arks_remote::is_local_remote(&url) {
        arks_remote::open_local_remote(&url)
            .ok()
            .and_then(|r| match r.head_target() {
                Ok(arks_refs::RefTarget::Symbolic(s)) => Some(s),
                _ => None,
            })
    } else {
        None
    };
    let mut out = String::new();
    for (oid, name) in &rows {
        if name == "HEAD" {
            if let Some(t) = &symref_head {
                out.push_str(&format!("ref: {t}\tHEAD\n"));
            }
        }
        out.push_str(&format!("{oid}\t{name}\n"));
    }
    print!("{out}");
    if rows.is_empty() && p.on("exit-code") {
        return Err(CliError::silent(2));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn globs_and_versions() {
        assert!(glob_match(b"v*", b"v1"));
        assert!(glob_match(b"v[0-9]", b"v2"));
        assert!(!glob_match(b"v?", b"v10"));
        assert!(tail_matches("main", "refs/heads/main"));
        assert!(!tail_matches("ain", "refs/heads/main"));
        assert_eq!(version_cmp("v2", "v10"), Ordering::Less);
        assert_eq!(
            version_cmp("refs/tags/v1", "refs/tags/v1^{}"),
            Ordering::Less
        );
    }
}
