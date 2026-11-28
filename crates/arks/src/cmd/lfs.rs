//! Native `arks lfs` CLI (E6 — zero `git-lfs` subprocesses).
//!
//! Primary references:
//! - <https://github.com/git-lfs/git-lfs/blob/main/docs/spec.md>
//! - <https://github.com/git-lfs/git-lfs/blob/main/docs/api/batch.md>
//!
//! Spec: `sot:root/v0-2-0-scope-expansion` §2.7 (E6).

use crate::cmd::shared::{head_blob_map, resolve_rev, tree_blob_map};
use crate::error::{CliError, CliResult};
use crate::util::{open_repo, work_tree};
use arks_remote::lfs::{
    LfsPointer, LfsStore, clean, download_objects, endpoint_from_remote_url, filter_process,
    smudge, upload_objects,
};
use std::collections::BTreeMap;
use std::fs;
use std::io;

const LFS_USAGE: &str = "\
usage: arks lfs <subcommand> [<args>]

SUBCOMMANDS:
  version                          Print native arks LFS version
  env                              Show LFS endpoint and local store paths
  install [--local]                Configure [filter \"lfs\"] in repo config
  track [<pattern>...]             Track file patterns in .gitattributes
  untrack <pattern>...             Remove file patterns from .gitattributes
  pointer --file=<path> [--check]  Build or validate a canonical LFS v1 pointer
  clean [<path>]                   Filter stdin to a canonical LFS v1 pointer
  smudge [<path>]                  Filter LFS pointer on stdin to object bytes
  filter-process                   Long-running Git filter protocol v2
  ls-files [-l|--long] [-n] [rev]  List LFS pointer files in HEAD or <rev>
  status                           Show LFS objects in working tree / HEAD
  fetch [<remote>]                 Download missing LFS objects via Batch API
  pull [<remote>]                  Fetch LFS objects and smudge worktree files
  push [<remote>] [<ref>]          Upload LFS objects via Batch API
";

pub fn cmd_lfs(args: &[String]) -> CliResult<()> {
    if args.is_empty() || args.iter().any(|a| a == "-h" || a == "--help") {
        print!("{LFS_USAGE}");
        return Ok(());
    }
    let sub = args[0].as_str();
    let rest = &args[1..];
    match sub {
        "version" => {
            println!(
                "arks-lfs/{} (native Rust; Git LFS v1 spec)",
                env!("CARGO_PKG_VERSION")
            );
            Ok(())
        }
        "env" => cmd_lfs_env(),
        "install" => cmd_lfs_install(rest),
        "track" => cmd_lfs_track(rest),
        "untrack" => cmd_lfs_untrack(rest),
        "pointer" => cmd_lfs_pointer(rest),
        "clean" => cmd_lfs_clean(),
        "smudge" => cmd_lfs_smudge(),
        "filter-process" => cmd_lfs_filter_process(),
        "ls-files" => cmd_lfs_ls_files(rest),
        "status" => cmd_lfs_status(),
        "fetch" => cmd_lfs_fetch(rest),
        "pull" => cmd_lfs_pull(rest),
        "push" => cmd_lfs_push(rest),
        other => Err(CliError::Usage(format!(
            "lfs: unknown subcommand {other:?} (run `arks lfs --help`)"
        ))),
    }
}

fn resolve_lfs_endpoint(
    repo: &arks_repo::Repository,
    remote_arg: Option<&str>,
) -> CliResult<String> {
    if let Ok(explicit) = std::env::var("ARKS_LFS_ENDPOINT") {
        if !explicit.trim().is_empty() {
            return Ok(explicit);
        }
    }
    let cfg = arks_config::ConfigSet::load(Some(repo.git_dir()));
    if let Some(u) = cfg.get("lfs.url") {
        if !u.trim().is_empty() {
            return Ok(u);
        }
    }
    let remote_name = remote_arg.unwrap_or("origin");
    if remote_name.starts_with("http://") || remote_name.starts_with("https://") {
        return Ok(endpoint_from_remote_url(remote_name));
    }
    let key = format!("remote.{remote_name}.url");
    let url = cfg.get(&key).ok_or_else(|| {
        CliError::Msg(format!(
            "lfs: remote '{remote_name}' has no URL configured (set remote.{remote_name}.url or lfs.url)"
        ))
    })?;
    Ok(endpoint_from_remote_url(&url))
}

fn resolve_lfs_auth() -> Option<String> {
    if let Ok(tok) = std::env::var("ARKS_LFS_TOKEN") {
        if !tok.trim().is_empty() {
            return Some(format!("Bearer {}", tok.trim()));
        }
    }
    if let Ok(basic) = std::env::var("ARKS_LFS_BASIC_AUTH") {
        if !basic.trim().is_empty() {
            use base64::Engine as _;
            let encoded = base64::engine::general_purpose::STANDARD.encode(basic.as_bytes());
            return Some(format!("Basic {encoded}"));
        }
    }
    None
}

fn cmd_lfs_env() -> CliResult<()> {
    let repo = open_repo()?;
    let store = LfsStore::open(repo.common_dir());
    let endpoint = resolve_lfs_endpoint(&repo, None).unwrap_or_else(|_| "(not configured)".into());
    println!("arks-lfs/{} (native Rust)", env!("CARGO_PKG_VERSION"));
    println!("Endpoint={endpoint}");
    println!(
        "LocalWorkingDir={}",
        work_tree(&repo)
            .map(|p| p.display().to_string())
            .unwrap_or_default()
    );
    println!("LocalGitDir={}", repo.git_dir().display());
    println!("LocalMediaDir={}", store.objects_dir().display());
    println!("LocalTmpDir={}", store.tmp_dir().display());
    Ok(())
}

fn cmd_lfs_install(args: &[String]) -> CliResult<()> {
    let _local = args.iter().any(|a| a == "--local");
    let repo = open_repo()?;
    let cfg_path = repo.common_dir().join("config");
    let mut text = fs::read_to_string(&cfg_path).unwrap_or_default();
    if !text.contains("[filter \"lfs\"]") {
        if !text.is_empty() && !text.ends_with('\n') {
            text.push('\n');
        }
        text.push_str(
            "[filter \"lfs\"]\n\
\tclean = arks lfs clean -- %f\n\
\tsmudge = arks lfs smudge -- %f\n\
\tprocess = arks lfs filter-process\n\
\trequired = true\n",
        );
        fs::write(&cfg_path, text)?;
    }
    println!(
        "Updated Git LFS filter configuration in {}",
        cfg_path.display()
    );
    Ok(())
}

fn cmd_lfs_track(args: &[String]) -> CliResult<()> {
    let repo = open_repo()?;
    let work = work_tree(&repo)?;
    let attr_path = work.join(".gitattributes");
    let existing = fs::read_to_string(&attr_path).unwrap_or_default();
    if args.is_empty() {
        println!("Listing tracked patterns");
        for line in existing.lines() {
            if line.contains("filter=lfs") {
                if let Some(pat) = line.split_whitespace().next() {
                    println!("    {pat} (.gitattributes)");
                }
            }
        }
        return Ok(());
    }
    let mut out = existing;
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    for pat in args {
        if pat.starts_with('-') {
            continue;
        }
        let rule = format!("{pat} filter=lfs diff=lfs merge=lfs -text");
        if !out.lines().any(|l| l.trim() == rule) {
            out.push_str(&rule);
            out.push('\n');
            println!("Tracking \"{pat}\"");
        } else {
            println!("\"{pat}\" already supported");
        }
    }
    fs::write(&attr_path, out)?;
    Ok(())
}

fn cmd_lfs_untrack(args: &[String]) -> CliResult<()> {
    if args.is_empty() {
        return Err(CliError::Usage("lfs untrack: expected <pattern>...".into()));
    }
    let repo = open_repo()?;
    let work = work_tree(&repo)?;
    let attr_path = work.join(".gitattributes");
    let existing = fs::read_to_string(&attr_path).unwrap_or_default();
    let kept: Vec<&str> = existing
        .lines()
        .filter(|l| {
            let pat = l.split_whitespace().next().unwrap_or("");
            !(l.contains("filter=lfs") && args.iter().any(|a| a == pat))
        })
        .collect();
    let mut out = kept.join("\n");
    if !out.is_empty() {
        out.push('\n');
    }
    fs::write(&attr_path, out)?;
    for pat in args {
        println!("Untracking \"{pat}\"");
    }
    Ok(())
}

fn cmd_lfs_pointer(args: &[String]) -> CliResult<()> {
    let mut file: Option<String> = None;
    let mut check = false;
    let mut i = 0usize;
    while i < args.len() {
        let a = args[i].as_str();
        if a == "--check" {
            check = true;
            i += 1;
        } else if a == "--file" {
            file = args.get(i + 1).cloned();
            i += 2;
        } else if let Some(v) = a.strip_prefix("--file=") {
            file = Some(v.to_string());
            i += 1;
        } else {
            return Err(CliError::Usage(format!(
                "lfs pointer: unknown option {a:?}"
            )));
        }
    }
    let bytes = if let Some(p) = file {
        fs::read(&p)?
    } else {
        let mut buf = Vec::new();
        io::Read::read_to_end(&mut io::stdin(), &mut buf)?;
        buf
    };
    if check {
        if LfsPointer::parse(&bytes).is_some() {
            return Ok(());
        }
        return Err(CliError::Msg(
            "lfs pointer --check: input is not a valid Git LFS v1 pointer".into(),
        ));
    }
    let ptr = LfsPointer::from_bytes(&bytes);
    print!("{}", ptr.encode());
    Ok(())
}

fn cmd_lfs_clean() -> CliResult<()> {
    let repo = open_repo()?;
    let store = LfsStore::open(repo.common_dir());
    let stdin = io::stdin();
    let stdout = io::stdout();
    clean(&store, stdin.lock(), stdout.lock()).map_err(|e| CliError::Msg(e.to_string()))?;
    Ok(())
}

fn cmd_lfs_smudge() -> CliResult<()> {
    let repo = open_repo()?;
    let store = LfsStore::open(repo.common_dir());
    let skip = std::env::var("GIT_LFS_SKIP_SMUDGE")
        .is_ok_and(|v| v == "1" || v.eq_ignore_ascii_case("true"));
    let stdin = io::stdin();
    let stdout = io::stdout();
    smudge(&store, stdin.lock(), stdout.lock(), skip).map_err(|e| CliError::Msg(e.to_string()))?;
    Ok(())
}

fn cmd_lfs_filter_process() -> CliResult<()> {
    let repo = open_repo()?;
    let store = LfsStore::open(repo.common_dir());
    let skip = std::env::var("GIT_LFS_SKIP_SMUDGE")
        .is_ok_and(|v| v == "1" || v.eq_ignore_ascii_case("true"));
    let stdin = io::stdin();
    let stdout = io::stdout();
    filter_process(&store, stdin.lock(), stdout.lock(), skip)
        .map_err(|e| CliError::Msg(e.to_string()))?;
    Ok(())
}

fn scan_repo_pointers(
    repo: &arks_repo::Repository,
    rev: Option<&str>,
) -> CliResult<BTreeMap<String, LfsPointer>> {
    let blobs = match rev {
        Some(r) => {
            let oid = resolve_rev(repo, r)?;
            tree_blob_map(repo, &oid)?
        }
        None => head_blob_map(repo)?,
    };
    let mut out = BTreeMap::new();
    for (path, oid) in blobs {
        if let Ok(obj) = repo.get_object(&oid) {
            if let Some(ptr) = LfsPointer::parse(&obj.payload) {
                out.insert(path, ptr);
            }
        }
    }
    Ok(out)
}

fn cmd_lfs_ls_files(args: &[String]) -> CliResult<()> {
    let mut long = false;
    let mut name_only = false;
    let mut rev: Option<String> = None;
    for a in args {
        match a.as_str() {
            "-l" | "--long" => long = true,
            "-n" | "--name-only" => name_only = true,
            s if s.starts_with('-') => {
                return Err(CliError::Usage(format!(
                    "lfs ls-files: unknown option {s:?}"
                )));
            }
            s => rev = Some(s.to_string()),
        }
    }
    let repo = open_repo()?;
    let store = LfsStore::open(repo.common_dir());
    let pointers = scan_repo_pointers(&repo, rev.as_deref())?;
    for (path, ptr) in pointers {
        if name_only {
            println!("{path}");
        } else {
            let marker = if store.exists(&ptr.oid) { '*' } else { '-' };
            let oid_disp = if long {
                ptr.oid.as_str()
            } else {
                &ptr.oid[..10]
            };
            println!("{oid_disp} {marker} {path}");
        }
    }
    Ok(())
}

fn cmd_lfs_status() -> CliResult<()> {
    let repo = open_repo()?;
    let store = LfsStore::open(repo.common_dir());
    let pointers = scan_repo_pointers(&repo, None)?;
    let present = pointers.values().filter(|p| store.exists(&p.oid)).count();
    println!(
        "Git LFS objects in HEAD: {} total ({} present locally, {} missing)",
        pointers.len(),
        present,
        pointers.len().saturating_sub(present)
    );
    Ok(())
}

fn cmd_lfs_fetch(args: &[String]) -> CliResult<()> {
    let remote = args
        .iter()
        .find(|a| !a.starts_with('-'))
        .map(String::as_str);
    let repo = open_repo()?;
    let endpoint = resolve_lfs_endpoint(&repo, remote)?;
    let store = LfsStore::open(repo.common_dir());
    let pointers: Vec<LfsPointer> = scan_repo_pointers(&repo, None)?.into_values().collect();
    let policy = arks_net::HttpsPolicy::from_environment();
    let auth = resolve_lfs_auth();
    let downloaded = download_objects(&store, &endpoint, &pointers, auth.as_deref(), &policy)
        .map_err(|e| CliError::Msg(e.to_string()))?;
    println!("Fetched {downloaded} LFS object(s) from {endpoint}");
    Ok(())
}

fn cmd_lfs_pull(args: &[String]) -> CliResult<()> {
    cmd_lfs_fetch(args)?;
    let repo = open_repo()?;
    let work = work_tree(&repo)?;
    let store = LfsStore::open(repo.common_dir());
    let pointers = scan_repo_pointers(&repo, None)?;
    let mut smudged_count = 0usize;
    for (rel, ptr) in pointers {
        let abs = work.join(&rel);
        if let Ok(existing) = fs::read(&abs) {
            if LfsPointer::parse(&existing).is_some() && store.exists(&ptr.oid) {
                let data = store
                    .read_verified(&ptr.oid, Some(ptr.size))
                    .map_err(|e| CliError::Msg(e.to_string()))?;
                fs::write(&abs, data)?;
                smudged_count += 1;
            }
        }
    }
    println!("Smudged {smudged_count} worktree file(s)");
    Ok(())
}

fn cmd_lfs_push(args: &[String]) -> CliResult<()> {
    let positionals: Vec<&str> = args
        .iter()
        .filter(|a| !a.starts_with('-'))
        .map(String::as_str)
        .collect();
    let remote = positionals.first().copied();
    let rev = positionals.get(1).copied();
    let repo = open_repo()?;
    let endpoint = resolve_lfs_endpoint(&repo, remote)?;
    let store = LfsStore::open(repo.common_dir());
    let pointers: Vec<LfsPointer> = scan_repo_pointers(&repo, rev)?.into_values().collect();
    let policy = arks_net::HttpsPolicy::from_environment();
    let auth = resolve_lfs_auth();
    let uploaded = upload_objects(&store, &endpoint, &pointers, auth.as_deref(), &policy)
        .map_err(|e| CliError::Msg(e.to_string()))?;
    println!("Uploaded {uploaded} LFS object(s) to {endpoint}");
    Ok(())
}
