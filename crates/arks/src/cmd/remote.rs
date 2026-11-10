//! Remote client porcelain (fetch / push / clone / remote).

use crate::error::{CliError, CliResult};
use crate::util::{open_repo, refuse_unknown_flags};
use std::fs;
use std::path::PathBuf;
pub fn cmd_remote(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &[])?;
    let repo = open_repo()?;
    let cfg = repo.git_dir().join("config");
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
            {
                return Err(CliError::Msg(format!(
                    "unsupported remote URL '{url}' (claimed: http(s), ssh://, user@host:path, git://)"
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
        other => Err(CliError::NotImplemented(format!(
            "remote subcommand not implemented: {other}"
        ))),
    }
}

pub fn cmd_fetch(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &["--depth", "--filter"])?;
    let mut depth: Option<u32> = None;
    let mut filter: Option<String> = None;
    let mut positional = Vec::new();
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--depth" {
            i += 1;
            let n = args
                .get(i)
                .ok_or_else(|| CliError::Usage("fetch --depth <n>".into()))?;
            depth = Some(parse_depth(n)?);
        } else if let Some(rest) = args[i].strip_prefix("--depth=") {
            depth = Some(parse_depth(rest)?);
        } else if args[i] == "--filter" {
            i += 1;
            let spec = args
                .get(i)
                .ok_or_else(|| CliError::Usage("fetch --filter <spec>".into()))?;
            filter = Some(parse_filter(spec)?);
        } else if let Some(rest) = args[i].strip_prefix("--filter=") {
            filter = Some(parse_filter(rest)?);
        } else {
            positional.push(args[i].clone());
        }
        i += 1;
    }
    let remote_name = positional.first().map(String::as_str).unwrap_or("origin");
    let mut repo = open_repo()?;
    // SHAL-001 (M20 Cx-4) / CORR-005 (M20 Cx-6): plant shallow.lock → fetch/deepen refuse.
    // Cite: https://git-scm.com/docs/gitrepository-layout (shallow)
    let shallow_lock = repo.common_dir().join("shallow.lock");
    if shallow_lock.is_file() {
        return Err(CliError::Msg(format!(
            "Unable to create '{}' (SHAL-001 / CORR-005 — concurrent shallow rewrite / deepen refuse; lease ≠ skip)",
            shallow_lock.display()
        )));
    }
    // SHAL-004 (M20 Cx-7): shallow + grafts while peers fetch/unshallow.
    let shallow = repo.common_dir().join("shallow");
    let grafts = repo.git_dir().join("info/grafts");
    let grafts_common = repo.common_dir().join("info/grafts");
    if shallow.is_file() && (grafts.is_file() || grafts_common.is_file()) {
        return Err(CliError::Msg(
            "fetch refused (SHAL-004 — shallow + grafts interaction; serialize unshallow vs shallow fetch peers)".into(),
        ));
    }
    let cfg = repo.git_dir().join("config");
    let remotes = arks_remote::list_remotes(&cfg)?;
    let remote = remotes
        .iter()
        .find(|r| r.name == remote_name)
        .ok_or_else(|| CliError::Msg(format!("No such remote: {remote_name}")))?;
    let opts = arks_remote::FetchOptions {
        depth,
        filter,
        ..Default::default()
    };
    let result = arks_remote::fetch_remote_with(&mut repo, &remote.url, remote_name, &opts)?;
    for (name, oid) in result.updated {
        println!("{name}: {}", oid.to_hex());
    }
    Ok(())
}

pub fn cmd_push(args: &[String]) -> CliResult<()> {
    // `-u` / `--set-upstream`: write branch.<name>.remote + .merge after a successful
    // branch push (git-push). Tag short names DWIM to refs/tags/ when no head exists.
    refuse_unknown_flags(
        args,
        &[
            "--force-with-lease",
            "--push-option",
            "-o",
            "-u",
            "--set-upstream",
        ],
    )?;
    let mut force_with_lease = false;
    let mut set_upstream = false;
    let mut lease_expect: Option<(String, String)> = None;
    let mut push_options: Vec<String> = Vec::new();
    let mut positional: Vec<String> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--force-with-lease" {
            force_with_lease = true;
        } else if let Some(rest) = args[i].strip_prefix("--force-with-lease=") {
            force_with_lease = true;
            let (r, e) = rest
                .split_once(':')
                .ok_or_else(|| CliError::Usage("push --force-with-lease=<ref>:<expect>".into()))?;
            lease_expect = Some((r.to_string(), e.to_string()));
        } else if args[i] == "-u" || args[i] == "--set-upstream" {
            set_upstream = true;
        } else if args[i] == "--push-option" || args[i] == "-o" {
            i += 1;
            let v = args
                .get(i)
                .ok_or_else(|| CliError::Usage("push --push-option <string>".into()))?;
            push_options.push(v.clone());
        } else if let Some(rest) = args[i].strip_prefix("--push-option=") {
            push_options.push(rest.to_string());
        } else if args[i].starts_with('-') {
            return Err(CliError::Usage(format!("unknown push flag {}", args[i])));
        } else {
            positional.push(args[i].clone());
        }
        i += 1;
    }
    let remote_name = positional.first().map(String::as_str).unwrap_or("origin");
    let refspec = positional.get(1).map(String::as_str);
    let repo = open_repo()?;
    let cfg = repo.git_dir().join("config");
    let remotes = arks_remote::list_remotes(&cfg)?;
    let remote = remotes
        .iter()
        .find(|r| r.name == remote_name)
        .ok_or_else(|| CliError::Msg(format!("No such remote: {remote_name}")))?;
    let (refname, oid) = match refspec {
        // Empty-source delete: `arks push <remote> :<ref>` (S3 / git push shape).
        Some(rs) if rs.starts_with(':') => {
            let name = rs[1..].trim();
            if name.is_empty() {
                return Err(CliError::Usage("push :<ref>".into()));
            }
            let full = if name.starts_with("refs/") {
                name.to_string()
            } else {
                // Deletes prefer heads; tags use full refs/tags/ or existing tag DWIM.
                let heads = format!("refs/heads/{name}");
                let tags = format!("refs/tags/{name}");
                if repo.refs().resolve(&tags).is_ok() && repo.refs().resolve(&heads).is_err() {
                    tags
                } else {
                    heads
                }
            };
            let zero = arks_oid::Oid::from_hex("0000000000000000000000000000000000000000")
                .map_err(|e| CliError::Msg(e.to_string()))?;
            (full, zero)
        }
        Some(rs) if rs.contains(':') => {
            return Err(CliError::NotImplemented(
                "push src:dst refspec not claimed".into(),
            ));
        }
        Some(name) => {
            let full = if name == "HEAD" {
                match repo.head_target()? {
                    arks_refs::RefTarget::Symbolic(s) => s,
                    arks_refs::RefTarget::Direct(_) => {
                        return Err(CliError::Msg(
                            "detached HEAD push requires explicit ref".into(),
                        ));
                    }
                }
            } else if name.starts_with("refs/") {
                name.to_string()
            } else {
                resolve_push_dwim(&repo, name)?
            };
            let oid = repo.refs().resolve(&full)?;
            (full, oid)
        }
        None => {
            let oid = repo.head_oid()?;
            let full = match repo.head_target()? {
                arks_refs::RefTarget::Symbolic(s) => s,
                arks_refs::RefTarget::Direct(_) => {
                    return Err(CliError::Msg(
                        "detached HEAD push requires explicit ref".into(),
                    ));
                }
            };
            (full, oid)
        }
    };
    let expected_old = if let Some((r, expect_hex)) = lease_expect {
        let full = if r.starts_with("refs/") {
            r
        } else {
            format!("refs/heads/{r}")
        };
        if full != refname {
            return Err(CliError::Usage(
                "push --force-with-lease ref must match the pushed ref".into(),
            ));
        }
        Some(arks_oid::Oid::from_hex(&expect_hex).map_err(|e| CliError::Msg(e.to_string()))?)
    } else if force_with_lease {
        // Default: expect remote-tracking tip for the same branch.
        let tracking = format!(
            "refs/remotes/{}/{}",
            remote_name,
            refname
                .strip_prefix("refs/heads/")
                .unwrap_or(refname.as_str())
        );
        Some(repo.refs().resolve(&tracking).map_err(|_| {
            CliError::Msg(format!(
                "force-with-lease: missing tracking ref {tracking}; pass --force-with-lease=<ref>:<expect>"
            ))
        })?)
    } else {
        None
    };
    arks_remote::push_remote_with(
        &repo,
        &remote.url,
        &[arks_remote::PushUpdate {
            refname: refname.clone(),
            new_oid: oid,
            expected_old,
        }],
        &arks_remote::PushOptions { push_options },
    )?;
    if set_upstream {
        if let Some(branch) = refname.strip_prefix("refs/heads/") {
            // Match git: branch.<name>.remote + branch.<name>.merge
            crate::cmd::config::cmd_config(&[
                format!("branch.{branch}.remote"),
                remote_name.to_string(),
            ])?;
            crate::cmd::config::cmd_config(&[format!("branch.{branch}.merge"), refname.clone()])?;
        }
    }
    Ok(())
}

/// Resolve a short push name: prefer `refs/heads/`, else `refs/tags/` (git DWIM).
fn resolve_push_dwim(repo: &arks_repo::Repository, name: &str) -> CliResult<String> {
    let heads = format!("refs/heads/{name}");
    if repo.refs().resolve(&heads).is_ok() {
        return Ok(heads);
    }
    let tags = format!("refs/tags/{name}");
    if repo.refs().resolve(&tags).is_ok() {
        return Ok(tags);
    }
    // Default error path keeps heads form (historical message shape).
    Err(CliError::Msg(format!("ref not found: {heads}")))
}

pub fn cmd_clone(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(
        args,
        &["--reference", "--depth", "--filter", "--bundle-uri"],
    )?;
    // MISC-009 (M20 Cx-8): clone --reference while donor under gc/maintenance.
    let mut reference: Option<PathBuf> = None;
    let mut depth: Option<u32> = None;
    let mut filter: Option<String> = None;
    // FS5: local-file `--bundle-uri` seeds objects via unbundle after clone.
    let mut bundle_uri: Option<PathBuf> = None;
    let mut positional = Vec::new();
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--reference" {
            i += 1;
            let p = args
                .get(i)
                .ok_or_else(|| CliError::Usage("clone --reference <repo>".into()))?;
            reference = Some(PathBuf::from(p));
        } else if let Some(rest) = args[i].strip_prefix("--reference=") {
            reference = Some(PathBuf::from(rest));
        } else if args[i] == "--depth" {
            i += 1;
            let n = args
                .get(i)
                .ok_or_else(|| CliError::Usage("clone --depth <n>".into()))?;
            depth = Some(parse_depth(n)?);
        } else if let Some(rest) = args[i].strip_prefix("--depth=") {
            depth = Some(parse_depth(rest)?);
        } else if args[i] == "--filter" {
            i += 1;
            let spec = args
                .get(i)
                .ok_or_else(|| CliError::Usage("clone --filter <spec>".into()))?;
            filter = Some(parse_filter(spec)?);
        } else if let Some(rest) = args[i].strip_prefix("--filter=") {
            filter = Some(parse_filter(rest)?);
        } else if args[i] == "--bundle-uri" {
            i += 1;
            let p = args
                .get(i)
                .ok_or_else(|| CliError::Usage("clone --bundle-uri <path>".into()))?;
            bundle_uri = Some(PathBuf::from(p));
        } else if let Some(rest) = args[i].strip_prefix("--bundle-uri=") {
            bundle_uri = Some(PathBuf::from(rest));
        } else {
            positional.push(args[i].clone());
        }
        i += 1;
    }
    if let Some(ref_path) = &reference {
        let donor_git = if ref_path.join(".git").is_dir() {
            ref_path.join(".git")
        } else {
            ref_path.clone()
        };
        let donor_lock = donor_git.join("gc.lock");
        let donor_maint = donor_git.join("maintenance.lock");
        let donor_pack_lock = donor_git.join("objects/pack").join("pack.lock");
        if donor_lock.is_file() || donor_maint.is_file() || donor_pack_lock.is_file() {
            return Err(CliError::Msg(
                "clone --reference refused (MISC-009 — donor under gc/maintenance; missing alternate objects risk; quiesce donor first)".into(),
            ));
        }
        // Honesty even when unlocked: still warn.
        eprintln!(
            "note: MISC-009 — clone --reference borrows donor objects; do not gc donor under live reference clones"
        );
    }
    if positional.is_empty() || positional.len() > 2 {
        return Err(CliError::Usage("clone <url> [<directory>]".into()));
    }
    let url = &positional[0];
    let dest = if let Some(d) = positional.get(1) {
        PathBuf::from(d)
    } else {
        PathBuf::from(clone_dest_name(url))
    };
    // NET-009: early refuse when dest already non-empty (before transport).
    if dest.exists() {
        if !dest.is_dir() {
            return Err(CliError::Msg(format!(
                "destination path '{}' exists and is not a directory (NET-009)",
                dest.display()
            )));
        }
        let mut entries = std::fs::read_dir(&dest).map_err(|e| CliError::Msg(e.to_string()))?;
        if entries.next().is_some() {
            return Err(CliError::Msg(format!(
                "destination path '{}' exists and is not an empty directory (NET-009 — concurrent clone dest collision refuse)",
                dest.display()
            )));
        }
    }
    let opts = arks_remote::FetchOptions {
        depth,
        filter,
        ..Default::default()
    };
    if arks_remote::HttpRemoteUrl::parse(url).is_ok() {
        arks_remote::clone_http_with(url, &dest, &opts)?;
    } else if arks_remote::SshRemoteUrl::parse(url).is_ok() {
        arks_remote::clone_ssh_with(url, &dest, &opts)?;
    } else if arks_remote::GitDaemonUrl::parse(url).is_ok() {
        arks_remote::clone_git_with(url, &dest, &opts)?;
    } else if arks_remote::LocalRemoteUrl::parse(url).is_ok() {
        arks_remote::clone_local_with(url, &dest, &opts)?;
    } else {
        return Err(CliError::Msg(format!(
            "unsupported clone URL '{url}' (claimed: http(s), ssh://, user@host:path, git://, local path, same-host file://)"
        )));
    }
    if let Some(bundle) = &bundle_uri {
        // FS5 + NET-006: local file or http(s) download with race/truncation honesty.
        let uri = bundle.display().to_string();
        let local_bundle = resolve_bundle_uri(&uri)?;
        let cwd = std::env::current_dir().map_err(|e| CliError::Msg(e.to_string()))?;
        std::env::set_current_dir(&dest).map_err(|e| CliError::Msg(e.to_string()))?;
        let unbundle_args = vec!["unbundle".into(), local_bundle.display().to_string()];
        let res = crate::cmd::bundle::cmd_bundle(&unbundle_args);
        let _ = std::env::set_current_dir(&cwd);
        res?;
        eprintln!("note: FS5/NET-006 — seeded objects from --bundle-uri {uri}");
    }
    println!("Cloned into '{}'", dest.display());
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

fn parse_depth(raw: &str) -> CliResult<u32> {
    let n: u32 = raw
        .parse()
        .map_err(|_| CliError::Usage(format!("invalid --depth '{raw}'")))?;
    if n == 0 {
        return Err(CliError::Usage("--depth must be a positive integer".into()));
    }
    Ok(n)
}

/// Claimed partial-clone filter-spec (`blob:none` / `tree:0`; R3 / S1 / O2).
fn parse_filter(raw: &str) -> CliResult<String> {
    arks_remote::validate_claimed_filter(raw)
        .map_err(|e| CliError::NotImplemented(e.to_string()))?;
    Ok(raw.to_string())
}

/// Infer a directory name from a clone URL (HTTP, SSH, or path).
fn clone_dest_name(url: &str) -> String {
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
        let err = cmd_clone(&[
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
        let err = cmd_clone(&[
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
