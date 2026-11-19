//! `arks repo clone` and `arks repo sync` (local mode).
//!
//! Peers: <https://cli.github.com/manual/gh_repo_clone>,
//! <https://cli.github.com/manual/gh_repo_sync>.
//! Clone URL comes from `GET /repos/{o}/{r}` (`clone_url`, fork `parent`)
//! (<https://docs.github.com/en/rest/repos/repos#get-a-repository>); the clone
//! itself runs in-process through `arks-remote` — never `git`.
//!
//! Partial:
//! - `clone -- <gitflags>`: only `--depth N` passes through; other flags refuse.
//! - `sync <destination-repository>` (remote mode) and `--source` refuse: the
//!   fork model is not implemented (`repo fork` is Refuse/N/A).

use super::common::{after_double_dash, call, fail, has_flag, positionals, str_of, usage};
use super::local_git;
use arks_repo::Repository;
use serde_json::Value;
use std::path::PathBuf;
use std::process::ExitCode;

const CLONE_VALUE_FLAGS: &[&str] = &["-u", "--upstream-remote-name"];

fn looks_like_url(s: &str) -> bool {
    s.contains("://")
        || s.starts_with('/')
        || s.starts_with('.')
        || (s.contains('@') && s.contains(':'))
}

fn default_dir(src: &str) -> String {
    let last = src
        .trim_end_matches('/')
        .rsplit(['/', ':'])
        .next()
        .unwrap_or(src);
    last.trim_end_matches(".git").to_string()
}

fn parse_depth(extra: &[String]) -> Result<Option<u32>, String> {
    let mut depth = None;
    let mut i = 0;
    while i < extra.len() {
        let a = extra[i].as_str();
        let v = if a == "--depth" {
            i += 1;
            extra.get(i).map(String::as_str)
        } else {
            a.strip_prefix("--depth=")
        };
        match v {
            Some(n) if a.starts_with("--depth") => {
                depth = Some(
                    n.parse::<u32>()
                        .map_err(|_| format!("--depth expects a number (got {n})"))?,
                );
            }
            _ => {
                return Err(format!(
                    "git clone flag `{a}` is not supported (only --depth)"
                ));
            }
        }
        i += 1;
    }
    Ok(depth)
}

pub fn clone(args: &[String]) -> ExitCode {
    let label = "repo clone";
    let pos = positionals(args, CLONE_VALUE_FLAGS);
    let Some(target) = pos.first() else {
        return usage(label, "<repository> required");
    };
    let depth = match parse_depth(&after_double_dash(args)) {
        Ok(d) => d,
        Err(e) => return usage(label, &e),
    };
    let mut parent: Option<Value> = None;
    let url = if looks_like_url(target) {
        target.clone()
    } else {
        let (base, tok) = match crate::require_api() {
            Ok(v) => v,
            Err(e) => return crate::exit_from_api_err(label, &e),
        };
        let slug = if target.contains('/') {
            target.clone()
        } else {
            match call(&base, &tok, "GET", "/user", None, label) {
                Ok(u) => format!("{}/{target}", str_of(&u, "login")),
                Err(c) => return c,
            }
        };
        let repo = match call(&base, &tok, "GET", &format!("/repos/{slug}"), None, label) {
            Ok(v) => v,
            Err(c) => return c,
        };
        let u = str_of(&repo, "clone_url");
        if u.is_empty() {
            return fail(
                label,
                &format!("{slug} has no clone_url (no hosted git storage)"),
            );
        }
        if repo.get("fork").and_then(Value::as_bool) == Some(true) {
            parent = repo.get("parent").cloned();
        }
        u.to_string()
    };
    let dest = PathBuf::from(pos.get(1).cloned().unwrap_or_else(|| default_dir(target)));
    if let Err(e) = local_git::check_clone_dest(&dest) {
        return usage(label, &e);
    }
    eprintln!("Cloning into '{}'...", dest.display());
    if let Err(e) = local_git::clone_url(&url, &dest, depth) {
        return fail(label, &e);
    }
    if let Some(p) = parent.filter(|_| !has_flag(args, &["--no-upstream"])) {
        let purl = str_of(&p, "clone_url");
        if !purl.is_empty() {
            let mut name = crate::flag_value(args, &["--upstream-remote-name", "-u"])
                .unwrap_or("upstream")
                .to_string();
            if name == "@owner" {
                name = p
                    .get("owner")
                    .and_then(|o| o.get("login"))
                    .and_then(Value::as_str)
                    .unwrap_or("upstream")
                    .to_string();
            }
            let cfg = dest.join(".git").join("config");
            if let Err(e) = arks_remote::add_remote(&cfg, &name, purl) {
                return fail(label, &format!("add remote {name}: {e}"));
            }
        }
    }
    ExitCode::SUCCESS
}

pub fn sync(args: &[String]) -> ExitCode {
    let label = "repo sync";
    let pos = positionals(args, &["-b", "--branch", "-s", "--source"]);
    if !pos.is_empty() {
        return usage(
            label,
            "remote destination sync is not implemented (fork model absent; `repo fork` is Refuse/N/A); \
             run without <destination-repository> inside a local clone",
        );
    }
    if crate::flag_value(args, &["--source", "-s"]).is_some() {
        return usage(
            label,
            "--source is not implemented for local sync (Partial); the remote `upstream` \
             (else `origin`) is the source",
        );
    }
    let force = has_flag(args, &["--force"]);
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut repo = match Repository::discover(&cwd) {
        Ok(r) => r,
        Err(e) => return fail(label, &format!("not a git repository: {e}")),
    };
    let cfg = repo.git_dir().join("config");
    let remote = if arks_remote::get_remote(&cfg, "upstream")
        .ok()
        .flatten()
        .is_some()
    {
        "upstream"
    } else {
        "origin"
    };
    let outcome = match local_git::fetch(&mut repo, remote) {
        Ok(o) => o,
        Err(e) => return fail(label, &e),
    };
    let branch = match crate::flag_value(args, &["--branch", "-b"]) {
        Some(b) => b.to_string(),
        None => match outcome.head_branch.as_deref() {
            Some(b) => b.trim_start_matches("refs/heads/").to_string(),
            None => return fail(label, "could not determine the source default branch"),
        },
    };
    let tracking = format!("refs/remotes/{remote}/{branch}");
    let new = match repo.refs().resolve(&tracking) {
        Ok(o) => o,
        Err(_) => {
            return fail(
                label,
                &format!("branch `{branch}` not found on remote `{remote}`"),
            );
        }
    };
    let local = repo.refs().resolve(&format!("refs/heads/{branch}")).ok();
    if let Some(old) = &local
        && !force
    {
        match local_git::is_ancestor(&repo, old, &new) {
            Ok(true) => {}
            Ok(false) => {
                return fail(
                    label,
                    "can't sync because there are diverging changes; use `--force` to overwrite the destination branch",
                );
            }
            Err(e) => return fail(label, &e),
        }
    }
    if local_git::current_branch(&repo).as_deref() == Some(branch.as_str()) {
        match local_git::is_dirty(&repo) {
            Ok(false) => {}
            Ok(true) => {
                return fail(
                    label,
                    "can't sync because there are local changes; please stash them before trying again",
                );
            }
            Err(e) => return fail(label, &e),
        }
    }
    if local.as_ref() != Some(&new)
        && let Err(e) = local_git::move_branch(&mut repo, &branch, &new)
    {
        return fail(label, &e);
    }
    println!("Synced the \"{branch}\" branch from \"{remote}\" to local repository");
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clone_target_shapes() {
        assert!(looks_like_url("https://h/o/r.git"));
        assert!(looks_like_url("git@github.com:o/r.git"));
        assert!(looks_like_url("/tmp/x.git"));
        assert!(!looks_like_url("o/r"));
        assert_eq!(default_dir("o/r"), "r");
        assert_eq!(default_dir("https://h/o/r.git"), "r");
        assert_eq!(default_dir("git@h:o/r.git"), "r");
    }

    #[test]
    fn depth_passthrough_only() {
        let v = |a: &[&str]| a.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(parse_depth(&v(&["--depth", "1"])).unwrap(), Some(1));
        assert_eq!(parse_depth(&v(&["--depth=3"])).unwrap(), Some(3));
        assert!(parse_depth(&v(&["--bare"])).is_err());
        assert_eq!(parse_depth(&[]).unwrap(), None);
    }
}
