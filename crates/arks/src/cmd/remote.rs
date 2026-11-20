//! Remote client porcelain (fetch / push / clone / remote).

use crate::error::{CliError, CliResult};
use crate::util::{open_repo, refuse_unknown_flags};
use std::fs;
use std::path::PathBuf;
pub fn cmd_remote(args: &[String]) -> CliResult<()> {
    // Cloud drop-in §O: `remote -v` / `--verbose`.
    let verbose = args.first().is_some_and(|a| a == "-v" || a == "--verbose");
    let args = if verbose { &args[1..] } else { args };
    refuse_unknown_flags(args, &[])?;
    let repo = open_repo()?;
    let cfg = repo.git_dir().join("config");
    if verbose && args.is_empty() {
        for r in arks_remote::list_remotes(&cfg)? {
            println!("{}\t{} (fetch)", r.name, r.url);
            println!("{}\t{} (push)", r.name, r.url);
        }
        return Ok(());
    }
    if args.is_empty() {
        for r in arks_remote::list_remotes(&cfg)? {
            println!("{}", r.name);
        }
        return Ok(());
    }
    match args[0].as_str() {
        "add" => {
            if args.len() != 3 {
                return Err(CliError::Usage("remote add <name> <url>".into()));
            }
            let url = &args[2];
            if arks_remote::HttpRemoteUrl::parse(url).is_err()
                && arks_remote::SshRemoteUrl::parse(url).is_err()
                && arks_remote::GitDaemonUrl::parse(url).is_err()
                && arks_remote::LocalRemoteUrl::parse(url).is_err()
            {
                return Err(CliError::Msg(format!(
                    "unsupported remote URL '{url}' (claimed: http(s), ssh://, user@host:path, git://, local path, file://)"
                )));
            }
            arks_remote::add_remote(&cfg, &args[1], url)?;
            Ok(())
        }
        "remove" | "rm" => {
            if args.len() != 2 {
                return Err(CliError::Usage("remote remove <name>".into()));
            }
            arks_remote::remove_remote(&cfg, &args[1])?;
            Ok(())
        }
        // BP-PORCELAIN ([[sot:root/arks-dod-behavioral]]): peer-comparable show/get-url/set-url.
        "show" => {
            if args.len() != 2 {
                return Err(CliError::Usage("remote show <name>".into()));
            }
            let name = &args[1];
            let Some(r) = arks_remote::get_remote(&cfg, name)? else {
                return Err(CliError::Msg(format!("No such remote: '{name}'")));
            };
            println!("* remote {}", r.name);
            println!("  Fetch URL: {}", r.url);
            println!("  Push  URL: {}", r.url);
            for spec in &r.fetch {
                println!("  Fetch = {spec}");
            }
            Ok(())
        }
        "get-url" => {
            if args.len() != 2 {
                return Err(CliError::Usage("remote get-url <name>".into()));
            }
            let name = &args[1];
            let Some(r) = arks_remote::get_remote(&cfg, name)? else {
                return Err(CliError::Msg(format!("No such remote: '{name}'")));
            };
            println!("{}", r.url);
            Ok(())
        }
        "set-url" => {
            if args.len() != 3 {
                return Err(CliError::Usage("remote set-url <name> <newurl>".into()));
            }
            arks_remote::set_remote_url(&cfg, &args[1], &args[2])?;
            Ok(())
        }
        other => Err(CliError::NotImplemented(format!(
            "remote subcommand not implemented: {other}"
        ))),
    }
}

/// Write `FETCH_HEAD` per git-fetch §OUTPUT / gitrepository-layout: one line per
/// fetched branch, `<oid>\t[not-for-merge]\tbranch '<b>' of <url>`. Explicit
/// branch arguments are all for-merge; without them only the current branch's
/// upstream is for-merge.
pub(crate) fn write_fetch_head(
    repo: &arks_repo::Repository,
    remote_name: &str,
    url: &str,
    wanted: &[String],
) -> CliResult<()> {
    let prefix = format!("refs/remotes/{remote_name}/");
    let mut lines = String::new();
    if wanted.is_empty() {
        let cfg = arks_config::ConfigSet::load(Some(repo.common_dir()));
        let upstream = match repo.head_target() {
            Ok(arks_refs::RefTarget::Symbolic(s)) => {
                let b = s.trim_start_matches("refs/heads/").to_string();
                match (
                    cfg.get(&format!("branch.{b}.remote")),
                    cfg.get(&format!("branch.{b}.merge")),
                ) {
                    (Some(r), Some(m)) if r == remote_name => {
                        Some(m.trim_start_matches("refs/heads/").to_string())
                    }
                    _ => None,
                }
            }
            _ => None,
        };
        let mut refs = repo.refs().list_refs(&prefix)?;
        refs.sort_by(|a, b| a.0.cmp(&b.0));
        // for-merge line first, as git orders it.
        refs.sort_by_key(|(n, _)| Some(n.trim_start_matches(&prefix).to_string()) != upstream);
        for (name, oid) in refs {
            let b = name.trim_start_matches(&prefix);
            if b == "HEAD" {
                continue;
            }
            let tag = if Some(b.to_string()) == upstream {
                ""
            } else {
                "not-for-merge"
            };
            lines.push_str(&format!("{}\t{tag}\tbranch '{b}' of {url}\n", oid.to_hex()));
        }
    } else {
        for w in wanted {
            if w.contains(':') {
                return Err(CliError::NotImplemented(format!(
                    "fetch <src>:<dst> refspec not claimed: {w}"
                )));
            }
            let b = w.trim_start_matches("refs/heads/");
            let oid = repo
                .refs()
                .resolve(&format!("{prefix}{b}"))
                .map_err(|_| CliError::Msg(format!("couldn't find remote ref {w}")))?;
            lines.push_str(&format!("{}\t\tbranch '{b}' of {url}\n", oid.to_hex()));
        }
    }
    fs::write(repo.git_dir().join("FETCH_HEAD"), lines)?;
    Ok(())
}

/// MISC-009 (M20 Cx-8): `clone --reference` while the donor is under gc/maintenance.
pub(crate) fn refuse_busy_reference(ref_path: &std::path::Path) -> CliResult<()> {
    let donor_git = if ref_path.join(".git").is_dir() {
        ref_path.join(".git")
    } else {
        ref_path.to_path_buf()
    };
    let donor_lock = donor_git.join("gc.lock");
    let donor_maint = donor_git.join("maintenance.lock");
    let donor_pack_lock = donor_git.join("objects/pack").join("pack.lock");
    if donor_lock.is_file() || donor_maint.is_file() || donor_pack_lock.is_file() {
        return Err(CliError::Msg(
            "clone --reference refused (MISC-009 — donor under gc/maintenance; missing alternate objects risk; quiesce donor first)".into(),
        ));
    }
    Ok(())
}

/// FS5 + NET-006: seed a fresh clone's objects from `--bundle-uri` (local file
/// or http(s) download with race/truncation honesty).
pub(crate) fn seed_bundle_uri(uri: &str, dest: &std::path::Path) -> CliResult<()> {
    let local_bundle = resolve_bundle_uri(uri)?;
    let cwd = std::env::current_dir().map_err(|e| CliError::Msg(e.to_string()))?;
    std::env::set_current_dir(dest).map_err(|e| CliError::Msg(e.to_string()))?;
    let unbundle_args = vec!["unbundle".into(), local_bundle.display().to_string()];
    let res = crate::cmd::bundle::cmd_bundle(&unbundle_args);
    let _ = std::env::set_current_dir(&cwd);
    res?;
    eprintln!("note: FS5/NET-006 — seeded objects from --bundle-uri {uri}");
    Ok(())
}

/// Resolve `--bundle-uri` to a local file (download http(s) atomically; NET-006).
fn resolve_bundle_uri(uri: &str) -> CliResult<PathBuf> {
    if uri.starts_with("http://") || uri.starts_with("https://") {
        return download_http_bundle(uri);
    }
    let path = PathBuf::from(uri);
    if !path.is_file() {
        return Err(CliError::Msg(format!(
            "clone --bundle-uri: not a local file or http(s) URL ('{}')",
            path.display()
        )));
    }
    Ok(path)
}

fn download_http_bundle(url: &str) -> CliResult<PathBuf> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    url.hash(&mut hasher);
    let key = format!("{:016x}", hasher.finish());
    let cache = std::env::temp_dir().join("arks-bundle-uri");
    fs::create_dir_all(&cache).map_err(|e| CliError::Msg(e.to_string()))?;
    let lock = cache.join(format!("{key}.lock"));
    let part = cache.join(format!("{key}.part"));
    let final_path = cache.join(format!("{key}.bundle"));
    // NET-006: concurrent clones sharing this URI serialize via exclusive lock file.
    if lock.is_file() {
        return Err(CliError::Msg(format!(
            "{} (NET-006 — concurrent HTTP bundle-uri download refuse; serialize clones sharing this URI; lease ≠ skip)",
            lock.display()
        )));
    }
    fs::write(&lock, b"arks-bundle-uri\n").map_err(|e| CliError::Msg(e.to_string()))?;
    let cleanup = |part: &PathBuf, lock: &PathBuf| {
        let _ = fs::remove_file(part);
        let _ = fs::remove_file(lock);
    };
    let response = match arks_remote::http_get(url) {
        Ok(r) => r,
        Err(e) => {
            cleanup(&part, &lock);
            return Err(CliError::Msg(format!("bundle-uri GET {url}: {e}")));
        }
    };
    if response.status != 200 {
        cleanup(&part, &lock);
        return Err(CliError::Msg(format!(
            "bundle-uri GET {url}: HTTP {}",
            response.status
        )));
    }
    if response.body.is_empty() {
        cleanup(&part, &lock);
        return Err(CliError::Msg(
            "NET-006 — empty HTTP bundle-uri body; refuse unbundle".into(),
        ));
    }
    if let Some((_, cl)) = response
        .headers
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case("content-length"))
    {
        if let Ok(expected) = cl.parse::<usize>() {
            if response.body.len() != expected {
                cleanup(&part, &lock);
                return Err(CliError::Msg(format!(
                    "NET-006 — truncated HTTP bundle-uri body (got {} bytes, Content-Length {expected}); refuse unbundle",
                    response.body.len()
                )));
            }
        }
    }
    if let Err(e) = fs::write(&part, &response.body) {
        cleanup(&part, &lock);
        return Err(CliError::Msg(e.to_string()));
    }
    if let Err(e) = fs::rename(&part, &final_path) {
        cleanup(&part, &lock);
        return Err(CliError::Msg(e.to_string()));
    }
    let _ = fs::remove_file(&lock);
    Ok(final_path)
}

pub(crate) fn parse_depth(raw: &str) -> CliResult<u32> {
    let n: u32 = raw
        .parse()
        .map_err(|_| CliError::Usage(format!("invalid --depth '{raw}'")))?;
    if n == 0 {
        return Err(CliError::Usage("--depth must be a positive integer".into()));
    }
    Ok(n)
}

/// Claimed partial-clone filter-spec (`blob:none` / `tree:0`; R3 / S1 / O2).
pub(crate) fn parse_filter(raw: &str) -> CliResult<String> {
    arks_remote::validate_claimed_filter(raw)
        .map_err(|e| CliError::NotImplemented(e.to_string()))?;
    Ok(raw.to_string())
}

/// Infer a directory name from a clone URL (HTTP, SSH, or path).
pub(crate) fn clone_dest_name(url: &str) -> String {
    let trimmed = url.trim().trim_end_matches('/');
    if let Ok(ssh) = arks_remote::SshRemoteUrl::parse(trimmed) {
        let leaf = ssh
            .path
            .rsplit('/')
            .next()
            .unwrap_or(ssh.path.as_str())
            .trim_end_matches(".git");
        if !leaf.is_empty() {
            return leaf.to_string();
        }
    }
    if let Ok(git) = arks_remote::GitDaemonUrl::parse(trimmed) {
        let leaf = git
            .path
            .rsplit('/')
            .next()
            .unwrap_or(git.path.as_str())
            .trim_end_matches(".git");
        if !leaf.is_empty() {
            return leaf.to_string();
        }
    }
    let base = trimmed
        .rsplit(['/', '\\', ':'])
        .next()
        .unwrap_or("repo")
        .trim_end_matches(".git");
    if base.is_empty() {
        "repo".into()
    } else {
        base.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clone_refuses_malformed_sparse_oid_filter() {
        let err = crate::cmd::clone::cmd_clone(&[
            "--filter=sparse:oid=abc".into(),
            "https://example.invalid/repo.git".into(),
        ])
        .expect_err("sparse:oid needs hex oid");
        let s = err.to_string();
        assert!(
            s.contains("sparse:oid") && (s.contains("40") || s.contains("hex")),
            "unexpected refuse text: {s}"
        );
    }

    #[test]
    fn clone_refuses_unclaimed_filter_spec() {
        let err = crate::cmd::clone::cmd_clone(&[
            "--filter=blob:limit=*".into(),
            "https://example.invalid/repo.git".into(),
        ])
        .expect_err("blob:limit=* must refuse");
        let s = err.to_string();
        assert!(s.contains("not claimed"), "unexpected refuse text: {s}");
    }

    #[test]
    fn parse_filter_accepts_blob_none_tree_0_and_blob_limit() {
        assert_eq!(parse_filter("blob:none").unwrap(), "blob:none");
        assert_eq!(parse_filter("tree:0").unwrap(), "tree:0");
        assert_eq!(parse_filter("blob:limit=1k").unwrap(), "blob:limit=1k");
        assert_eq!(parse_filter("blob:limit=1024").unwrap(), "blob:limit=1024");
    }

    #[test]
    fn parse_filter_accepts_object_type_combine_and_sparse_oid() {
        assert_eq!(
            parse_filter("object:type=blob").unwrap(),
            "object:type=blob"
        );
        assert_eq!(
            parse_filter("combine:blob:none+tree:0").unwrap(),
            "combine:blob:none+tree:0"
        );
        assert_eq!(
            parse_filter("combine:object:type=blob+blob:limit=1k").unwrap(),
            "combine:object:type=blob+blob:limit=1k"
        );
        let sparse = format!("sparse:oid={}", "a".repeat(40));
        assert_eq!(parse_filter(&sparse).unwrap(), sparse);
        assert_eq!(
            parse_filter(&format!("combine:blob:none+{sparse}")).unwrap(),
            format!("combine:blob:none+{sparse}")
        );
        assert!(parse_filter("combine:blob:none+sparse:oid=x").is_err());
        assert!(parse_filter("object:type=widget").is_err());
    }
}
