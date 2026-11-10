//! Remote client porcelain (fetch / push / clone / remote).

use crate::error::{CliError, CliResult};
use crate::util::{open_repo, refuse_unknown_flags};
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
    refuse_unknown_flags(args, &["--depth"])?;
    let mut depth: Option<u32> = None;
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
        ..Default::default()
    };
    let result = arks_remote::fetch_remote_with(&mut repo, &remote.url, remote_name, &opts)?;
    for (name, oid) in result.updated {
        println!("{name}: {}", oid.to_hex());
    }
    Ok(())
}

pub fn cmd_push(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &[])?;
    let remote_name = args.first().map(String::as_str).unwrap_or("origin");
    let refspec = args.get(1).map(String::as_str);
    let repo = open_repo()?;
    let cfg = repo.git_dir().join("config");
    let remotes = arks_remote::list_remotes(&cfg)?;
    let remote = remotes
        .iter()
        .find(|r| r.name == remote_name)
        .ok_or_else(|| CliError::Msg(format!("No such remote: {remote_name}")))?;
    let (refname, oid) = match refspec {
        Some(rs) if rs.contains(':') => {
            return Err(CliError::NotImplemented(
                "push refspec with ':' not claimed in M6".into(),
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
                format!("refs/heads/{name}")
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
    arks_remote::push_remote(
        &repo,
        &remote.url,
        &[arks_remote::PushUpdate {
            refname,
            new_oid: oid,
        }],
    )?;
    Ok(())
}

pub fn cmd_clone(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &["--reference", "--depth"])?;
    // MISC-009 (M20 Cx-8): clone --reference while donor under gc/maintenance.
    let mut reference: Option<PathBuf> = None;
    let mut depth: Option<u32> = None;
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
    println!("Cloned into '{}'", dest.display());
    Ok(())
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
