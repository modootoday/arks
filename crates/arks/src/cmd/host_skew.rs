//! Host-skew cmds present on newer Git-for-Windows (2.54+/2.56+) but absent from
//! clean-room pin 2.39.5. Specfold-honest Partial stubs; never spawn git/gh.

use crate::cmd::shared::resolve_rev;
use crate::error::{CliError, CliResult};
use crate::util::{open_repo, refuse_unknown_flags};
use std::fs;

/// `arks refs` — subset: `list` → for-each-ref style names (host 2.54+).
pub fn cmd_refs(args: &[String]) -> CliResult<()> {
    if args.is_empty() || args.iter().any(|a| a == "-h" || a == "--help") {
        print!(
            "\
arks refs — refs helpers (host-skew Partial; pin 2.39.5 lacked this cmd)

Usage:
  arks refs list
  arks refs verify   # presence check only
  arks refs migrate  # not claimed
"
        );
        return Ok(());
    }
    refuse_unknown_flags(args, &["list", "verify", "migrate"])?;
    let sub = args[0].as_str();
    let repo = open_repo()?;
    match sub {
        "list" => {
            // Thin: print loose + packed ref names via show-ref plumbing path.
            crate::cmd::cmd_show_ref(&[])
        }
        "verify" => {
            let packed = repo.git_dir().join("packed-refs");
            let heads = repo.git_dir().join("refs/heads");
            println!(
                "ok\tpacked-refs={}\trefs/heads={}",
                packed.is_file(),
                heads.is_dir()
            );
            Ok(())
        }
        "migrate" => Err(CliError::NotImplemented(
            "refs migrate: ref-format migration not claimed".into(),
        )),
        other => Err(CliError::Usage(format!("refs: unknown subcommand {other}"))),
    }
}

/// Host-skew `git repo` helper body (2.54+). Not dispatched today: top-level
/// `repo` is owned by OverlapFacade (`status`/`config`/`repo`). Kept for a later
/// git-rank handoff without inventing a second leaf name.
#[allow(dead_code)]
pub fn cmd_repo(args: &[String]) -> CliResult<()> {
    if args.is_empty() || args.iter().any(|a| a == "-h" || a == "--help") {
        print!(
            "\
arks repo — repository metadata (host-skew Partial)

Usage:
  arks repo info
  arks repo structure  # not claimed
"
        );
        return Ok(());
    }
    refuse_unknown_flags(args, &["info", "structure", "--format", "-z", "--all", "--keys"])?;
    let sub = args
        .iter()
        .find(|a| !a.starts_with('-'))
        .map(String::as_str)
        .unwrap_or("info");
    let repo = open_repo()?;
    match sub {
        "info" => {
            println!("layout.bare={}", repo.work_tree().is_none());
            println!("gitdir={}", repo.git_dir().display());
            if let Ok(oid) = repo.head_oid() {
                println!("head.oid={}", oid.to_hex());
            }
            println!("objectformat=sha1");
            Ok(())
        }
        "structure" => Err(CliError::NotImplemented(
            "repo structure: not claimed (Partial; info only)".into(),
        )),
        other => Err(CliError::Usage(format!("repo: unknown subcommand {other}"))),
    }
}

/// `arks survey` — experimental repo survey counts (host 2.54+; Partial).
pub fn cmd_survey(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &["-v", "--verbose", "--progress", "-n", "--top", "--all-refs", "--branches", "--tags"])?;
    let repo = open_repo()?;
    let mut refs = 0usize;
    let heads = repo.git_dir().join("refs");
    if heads.is_dir() {
        refs += count_files(&heads)?;
    }
    let mut packs = 0usize;
    let pack_dir = repo.git_dir().join("objects/pack");
    if pack_dir.is_dir() {
        for ent in fs::read_dir(&pack_dir)? {
            if ent?.path().extension().and_then(|s| s.to_str()) == Some("pack") {
                packs += 1;
            }
        }
    }
    println!("survey.refs_loose_approx={refs}");
    println!("survey.packs={packs}");
    println!("note: survey subset — experimental host cmd; not pin 2.39.5");
    Ok(())
}

fn count_files(dir: &std::path::Path) -> CliResult<usize> {
    let mut n = 0usize;
    for ent in fs::read_dir(dir)? {
        let ent = ent?;
        let p = ent.path();
        if p.is_dir() {
            n += count_files(&p)?;
        } else {
            n += 1;
        }
    }
    Ok(n)
}

/// `arks last-modified` — path tip commit oid subset (host 2.54+).
pub fn cmd_last_modified(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &["-r", "--recursive", "-t", "--show-trees", "--max-depth", "-z"])?;
    let path = args
        .iter()
        .find(|a| !a.starts_with('-'))
        .ok_or_else(|| CliError::Usage("last-modified <path>".into()))?;
    let repo = open_repo()?;
    let head = resolve_rev(&repo, "HEAD")?;
    // Subset: print HEAD tip only (full path history walk not claimed).
    println!("{}\t{}", head.to_hex(), path);
    eprintln!("note: last-modified subset — prints HEAD for path; history walk not claimed");
    Ok(())
}

/// `arks history` — host 2.54+ reword/split (help + refuse).
pub fn cmd_history(args: &[String]) -> CliResult<()> {
    if args.is_empty() || args.iter().any(|a| a == "-h" || a == "--help") {
        print!(
            "\
arks history — rewrite helpers (host-skew; not claimed beyond help)

Usage:
  arks history reword|split …

Full history rewrite not claimed (use rebase/filter carefully outside arks).
"
        );
        return Ok(());
    }
    Err(CliError::NotImplemented(format!(
        "history {}: not claimed (Partial help only)",
        args[0]
    )))
}

/// `arks replay` — experimental (host 2.54+); help + refuse.
pub fn cmd_replay_exp(args: &[String]) -> CliResult<()> {
    if args.is_empty() || args.iter().any(|a| a == "-h" || a == "--help") {
        print!(
            "\
arks replay — experimental revision replay (host-skew; not claimed)

Use arks cherry-pick / rebase for claimed workflows.
"
        );
        return Ok(());
    }
    Err(CliError::NotImplemented(
        "replay: experimental host cmd not claimed (Partial help only)".into(),
    ))
}

/// `arks diff-pairs` — host 2.54+ plumbing (help + refuse).
pub fn cmd_diff_pairs(args: &[String]) -> CliResult<()> {
    let _ = args;
    Err(CliError::NotImplemented(
        "diff-pairs: host 2.54+ plumbing not claimed; use arks diff-tree / diff".into(),
    ))
}

/// `arks format-rev` — host 2.56 experimental (help + refuse).
pub fn cmd_format_rev(args: &[String]) -> CliResult<()> {
    if args.iter().any(|a| a == "-h" || a == "--help") || args.is_empty() {
        println!(
            "arks format-rev - experimental host 2.56 cmd; not claimed (use arks log --format)"
        );
        return Ok(());
    }
    Err(CliError::NotImplemented(
        "format-rev: experimental host 2.56 not claimed".into(),
    ))
}

/// `arks url-parse` — host 2.56 plumbing Partial: print scheme/host/path best-effort.
pub fn cmd_url_parse(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &["-c", "--component"])?;
    let mut component: Option<String> = None;
    let mut url: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        if args[i] == "-c" || args[i] == "--component" {
            i += 1;
            component = args.get(i).cloned();
        } else if let Some(rest) = args[i].strip_prefix("--component=") {
            component = Some(rest.to_string());
        } else if !args[i].starts_with('-') {
            url = Some(args[i].clone());
        }
        i += 1;
    }
    let url = url.ok_or_else(|| CliError::Usage("url-parse [-c component] <url>".into()))?;
    let (scheme, rest) = if let Some(i) = url.find("://") {
        (&url[..i], &url[i + 3..])
    } else if let Some((userhost, path)) = url.split_once(':') {
        // SCP-like
        let host = userhost.rsplit_once('@').map(|(_, h)| h).unwrap_or(userhost);
        match component.as_deref() {
            None | Some("host") => println!("{host}"),
            Some("path") => println!("{path}"),
            Some("scheme") => println!("ssh"),
            Some(other) => {
                return Err(CliError::NotImplemented(format!(
                    "url-parse component '{other}' not claimed"
                )));
            }
        }
        return Ok(());
    } else {
        return Err(CliError::Msg("url-parse: unrecognized URL".into()));
    };
    let (host, path) = rest
        .split_once('/')
        .map(|(h, p)| (h, format!("/{p}")))
        .unwrap_or((rest, String::new()));
    match component.as_deref() {
        None => {
            println!("scheme={scheme}");
            println!("host={host}");
            println!("path={path}");
        }
        Some("scheme") => println!("{scheme}"),
        Some("host") => println!("{host}"),
        Some("path") => println!("{path}"),
        Some(other) => {
            return Err(CliError::NotImplemented(format!(
                "url-parse component '{other}' not claimed"
            )));
        }
    }
    Ok(())
}
