//! Submodules (add/init/update/status/deinit).
//!
//! Cite: https://git-scm.com/docs/git-submodule
//! Cite: https://git-scm.com/docs/gitmodules
//! Cite: https://git-scm.com/docs/gitsubmodules
//! Plan: `.spec/plan/20261109022956-worktree-submodule-residual.plan.md`

use crate::error::{CliError, CliResult};
use crate::util::{
    open_repo, read_index, refuse_unknown_flags, rel_path_from_worktree, work_tree, write_index,
};
use arks_index::{IndexEntry, sort_entries};
use arks_object::{ObjectKind, RawObject};
use arks_oid::Oid;
use arks_remote::{
    GitDaemonUrl, HttpRemoteUrl, LocalRemoteUrl, SshRemoteUrl, clone_git, clone_http, clone_local,
    clone_ssh,
};
use arks_repo::Repository;
use arks_store::ObjectStore;
use std::fs;
use std::path::Path;

pub fn cmd_submodule(args: &[String]) -> CliResult<()> {
    let sub = args
        .first()
        .map(String::as_str)
        .ok_or_else(|| CliError::Usage("submodule <add|init|update|status|deinit>".into()))?;
    let rest = &args[1..];
    match sub {
        "add" => sm_add(rest),
        "init" => sm_init(rest),
        "update" => sm_update(rest),
        "status" => sm_status(rest),
        "deinit" => sm_deinit(rest),
        "sync" => sm_sync(rest),
        other => Err(CliError::NotImplemented(format!(
            "submodule {other} not claimed"
        ))),
    }
}

fn sm_sync(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &[])?;
    let repo = open_repo()?;
    let work = work_tree(&repo)?;
    // SUB-009 (M20 Cx-6): sync config while update runs — honor .gitmodules.lock.
    let lock = work.join(".gitmodules.lock");
    if lock.is_file() {
        return Err(CliError::Msg(format!(
            "{} (SUB-009 — submodule sync under .gitmodules.lock refuse; serialize sync vs update)",
            lock.display()
        )));
    }
    // SUB-003 honesty: absorbgitdirs race class when modules/ present.
    if repo.common_dir().join("modules").is_dir() {
        eprintln!(
            "note: SUB-003 — modules/ absorbgitdirs races with clone/update; serialize absorb vs status/update"
        );
    }
    Err(CliError::Msg(
        "submodule sync: no URL rewrites claimed under contention; coordinate single syncer (SUB-009 honesty)".into(),
    ))
}

fn sm_add(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &["--", "-b", "--force", "-f", "--name", "--depth"])?;
    if args.iter().any(|a| {
        matches!(
            a.as_str(),
            "-b" | "--force" | "-f" | "--name" | "--depth" | "--reference"
        )
    }) {
        return Err(CliError::NotImplemented(
            "submodule add: -b/--force/--name/--depth not claimed".into(),
        ));
    }
    let positional: Vec<&String> = args
        .iter()
        .filter(|a| a.as_str() != "--" && !a.starts_with('-'))
        .collect();
    if positional.is_empty() {
        return Err(CliError::Usage(
            "submodule add [--] <repository> [<path>]".into(),
        ));
    }
    let url = positional[0].as_str();
    let local_ok = LocalRemoteUrl::parse(url).is_ok();
    let ssh_ok = SshRemoteUrl::parse(url).is_ok();
    let http_ok = HttpRemoteUrl::parse(url).is_ok();
    let git_ok = GitDaemonUrl::parse(url).is_ok();
    if !local_ok && !ssh_ok && !http_ok && !git_ok {
        return Err(CliError::NotImplemented(
            "submodule add: claimed URLs are local path / file:// / https:// / http:// / ssh:// / user@host:path / git://"
                .into(),
        ));
    }
    let path = if let Some(p) = positional.get(1) {
        p.to_string()
    } else if let Ok(ssh) = SshRemoteUrl::parse(url) {
        ssh.path
            .rsplit('/')
            .next()
            .unwrap_or("sub")
            .trim_end_matches(".git")
            .to_string()
    } else if let Ok(git) = GitDaemonUrl::parse(url) {
        git.path
            .rsplit('/')
            .next()
            .unwrap_or("sub")
            .trim_end_matches(".git")
            .to_string()
    } else if http_ok {
        // Infer path from final URL path segment (strip .git); cite git-submodule(1).
        url.trim_end_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or("sub")
            .trim_end_matches(".git")
            .to_string()
    } else {
        Path::new(url)
            .file_stem()
            .or_else(|| Path::new(url).file_name())
            .map(|s| s.to_string_lossy().into_owned())
            .ok_or_else(|| CliError::Usage("submodule add: cannot infer path".into()))?
            .trim_end_matches(".git")
            .to_string()
    };
    if path.contains('\\') {
        return Err(CliError::Msg(
            "submodule path must use forward slashes".into(),
        ));
    }

    let mut repo = open_repo()?;
    let work = work_tree(&repo)?;
    let rel = normalize_rel(&path);
    let abs = work.join(&rel);
    if abs.exists() {
        return Err(CliError::Msg(format!("'{rel}' already exists")));
    }
    if let Some(parent) = abs.parent() {
        fs::create_dir_all(parent)?;
    }

    // Clone into path (temporary normal clone), then absorb into .git/modules.
    // HTTPS/HTTP via arks-remote clone_http (FW-SUBMOD-HTTPS); git:// via clone_git.
    // Cite git-submodule(1).
    if ssh_ok {
        clone_ssh(url, &abs).map_err(|e| CliError::Msg(e.to_string()))?;
    } else if http_ok {
        clone_http(url, &abs).map_err(|e| CliError::Msg(e.to_string()))?;
    } else if git_ok {
        clone_git(url, &abs).map_err(|e| CliError::Msg(e.to_string()))?;
    } else {
        clone_local(url, &abs).map_err(|e| CliError::Msg(e.to_string()))?;
    }
    let sub_repo = Repository::discover(&abs)?;
    let head = sub_repo.head_oid()?;
    absorb_gitdir(&repo, &abs, &rel)?;

    // Update .gitmodules (config-safe path separators — cite git-config value escaping).
    let url_cfg = config_safe_path(url);
    let gm_path = work.join(".gitmodules");
    let mut gm = fs::read_to_string(&gm_path).unwrap_or_default();
    gm = submodule_config_set(&gm, &rel, "path", &rel)?;
    gm = submodule_config_set(&gm, &rel, "url", &url_cfg)?;
    fs::write(&gm_path, gm)?;

    // Stage gitlink + .gitmodules
    let mut index = read_index(&repo)?;
    index
        .entries
        .retain(|e| e.path != rel && e.path != ".gitmodules");
    index.entries.push(IndexEntry {
        ctime_sec: 0,
        ctime_nsec: 0,
        mtime_sec: 0,
        mtime_nsec: 0,
        dev: 0,
        ino: 0,
        mode: 0o160000,
        uid: 0,
        gid: 0,
        size: 0,
        oid: head,
        flags: (rel.len().min(0x0fff)) as u16,
        flags_extended: None,
        path: rel.clone(),
    });
    let gm_bytes = fs::read(&gm_path)?;
    let gm_obj = RawObject::new(ObjectKind::Blob, gm_bytes.clone());
    let format = repo.object_format();
    let gm_oid = repo.store_mut().put(format, &gm_obj)?;
    index.entries.push(IndexEntry {
        ctime_sec: 0,
        ctime_nsec: 0,
        mtime_sec: 0,
        mtime_nsec: 0,
        dev: 0,
        ino: 0,
        mode: 0o100644,
        uid: 0,
        gid: 0,
        size: gm_bytes.len().min(u32::MAX as usize) as u32,
        oid: gm_oid,
        flags: (".gitmodules".len().min(0x0fff)) as u16,
        flags_extended: None,
        path: ".gitmodules".into(),
    });
    sort_entries(&mut index.entries);
    write_index(&repo, &index)?;

    // Also record url in .git/config (init-equivalent for the new submodule).
    let cfg_path = repo.common_dir().join("config");
    let mut cfg = fs::read_to_string(&cfg_path).unwrap_or_default();
    cfg = submodule_config_set(&cfg, &rel, "url", &url_cfg)?;
    fs::write(&cfg_path, cfg)?;

    println!("Adding existing repo at '{rel}' to the index");
    Ok(())
}

fn sm_init(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &["--"])?;
    let filters: Vec<String> = args
        .iter()
        .filter(|a| a.as_str() != "--" && !a.starts_with('-'))
        .cloned()
        .collect();
    let repo = open_repo()?;
    let work = work_tree(&repo)?;
    let gm = fs::read_to_string(work.join(".gitmodules")).unwrap_or_default();
    let mods = parse_gitmodules(&gm);
    let cfg_path = repo.common_dir().join("config");
    let mut cfg = fs::read_to_string(&cfg_path).unwrap_or_default();
    for m in mods {
        if !filters.is_empty() && !filters.iter().any(|f| f == &m.name || f == &m.path) {
            continue;
        }
        if submodule_config_get(&cfg, &m.name, "url").is_some() {
            continue;
        }
        cfg = submodule_config_set(&cfg, &m.name, "url", &m.url)?;
        println!(
            "Submodule '{path}' ({url}) registered for path '{path}'",
            path = m.path,
            url = m.url
        );
    }
    fs::write(&cfg_path, cfg)?;
    Ok(())
}

fn sm_update(args: &[String]) -> CliResult<()> {
    // M16 Sy-2: `--recursive`, `--init --recursive`, `--remote` claimed.
    // Cite: https://git-scm.com/docs/git-submodule/2.39.0
    refuse_unknown_flags(
        args,
        &[
            "--",
            "--init",
            "--remote",
            "--recursive",
            "--merge",
            "--rebase",
        ],
    )?;
    if args
        .iter()
        .any(|a| matches!(a.as_str(), "--merge" | "--rebase"))
    {
        return Err(CliError::NotImplemented(
            "submodule update: --merge/--rebase not claimed".into(),
        ));
    }
    let do_init = args.iter().any(|a| a == "--init");
    let recursive = args.iter().any(|a| a == "--recursive");
    let remote = args.iter().any(|a| a == "--remote");
    let filters: Vec<String> = args
        .iter()
        .filter(|a| a.as_str() != "--" && !a.starts_with('-'))
        .cloned()
        .collect();
    if do_init {
        sm_init(&[])?;
    }
    let repo = open_repo()?;
    update_submodules_at(&repo, &filters, recursive, remote, "")
}

fn update_submodules_at(
    repo: &Repository,
    filters: &[String],
    recursive: bool,
    remote: bool,
    prefix: &str,
) -> CliResult<()> {
    let work = work_tree(repo)?;
    let index = read_index(repo)?;
    let gm = fs::read_to_string(work.join(".gitmodules")).unwrap_or_default();
    let mods = parse_gitmodules(&gm);
    let cfg = fs::read_to_string(repo.common_dir().join("config")).unwrap_or_default();

    for m in mods {
        if !filters.is_empty() && !filters.iter().any(|f| f == &m.name || f == &m.path) {
            continue;
        }
        let Some(entry) = index
            .entries
            .iter()
            .find(|e| e.stage() == 0 && e.path == m.path && e.mode == 0o160000)
        else {
            continue;
        };
        let url = submodule_config_get(&cfg, &m.name, "url")
            .or_else(|| Some(m.url.clone()))
            .ok_or_else(|| CliError::Msg(format!("no url for submodule '{}'", m.name)))?;
        let abs = work.join(&m.path);
        let target = if remote {
            resolve_submodule_remote_tip(&m, &gm, &cfg, &url)?
        } else {
            entry.oid
        };
        ensure_submodule_checkout(repo, &abs, &m.path, &url, &target)?;
        let display = if prefix.is_empty() {
            m.path.clone()
        } else {
            format!("{prefix}/{}", m.path)
        };
        println!(
            "Submodule path '{display}': checked out '{}'",
            target.to_hex()
        );
        if recursive {
            if let Ok(sub) = Repository::discover(&abs) {
                // Nested filters do not apply once we enter a submodule path.
                update_submodules_at(&sub, &[], true, remote, &display)?;
            }
        }
    }
    Ok(())
}

fn resolve_submodule_remote_tip(
    meta: &SubmoduleMeta,
    gitmodules: &str,
    cfg: &str,
    url: &str,
) -> CliResult<Oid> {
    // Prefer submodule.<name>.branch from .git/config, then .gitmodules.
    // Cite: git-submodule --remote (2.39.0).
    let branch = submodule_config_get(cfg, &meta.name, "branch")
        .or_else(|| submodule_config_get(gitmodules, &meta.name, "branch"));
    // Local path: resolve tip from the filesystem peer. SSH: fetch into a temp
    // probe is out of scope for --remote on Sy-4; refuse SSH with a clear message.
    if SshRemoteUrl::parse(url).is_ok() {
        return Err(CliError::NotImplemented(
            "submodule update --remote over SSH URL not claimed in Sy-4 (add/clone/fetch/push are)"
                .into(),
        ));
    }
    let local = LocalRemoteUrl::parse(url).map_err(|_| {
        CliError::Msg(format!(
            "submodule update --remote: unsupported url '{url}'"
        ))
    })?;
    let source = Repository::discover(local.path())
        .or_else(|_| Repository::open(local.path()))
        .map_err(|e| CliError::Msg(format!("submodule remote '{url}': {e}")))?;
    if let Some(b) = branch {
        let full = if b.starts_with("refs/") {
            b
        } else {
            format!("refs/heads/{b}")
        };
        return source
            .refs()
            .resolve(&full)
            .map_err(|e| CliError::Msg(format!("submodule remote branch '{full}': {e}")));
    }
    // Default: remote HEAD tip.
    source
        .head_oid()
        .map_err(|e| CliError::Msg(format!("submodule remote HEAD: {e}")))
}

fn sm_status(args: &[String]) -> CliResult<()> {
    // M16 Sy-2: `--recursive` claimed. Cite: https://git-scm.com/docs/git-submodule/2.39.0
    refuse_unknown_flags(args, &["--", "--cached", "--recursive"])?;
    if args.iter().any(|a| a == "--cached") {
        return Err(CliError::NotImplemented(
            "submodule status: --cached not claimed".into(),
        ));
    }
    let recursive = args.iter().any(|a| a == "--recursive");
    let filters: Vec<String> = args
        .iter()
        .filter(|a| a.as_str() != "--" && !a.starts_with('-'))
        .cloned()
        .collect();
    let repo = open_repo()?;
    status_submodules_at(&repo, &filters, recursive, "")
}

fn status_submodules_at(
    repo: &Repository,
    filters: &[String],
    recursive: bool,
    prefix: &str,
) -> CliResult<()> {
    let work = work_tree(repo)?;
    let index = read_index(repo)?;
    let gm = fs::read_to_string(work.join(".gitmodules")).unwrap_or_default();
    let mods = parse_gitmodules(&gm);
    // Also include index gitlinks not yet in .gitmodules (rare).
    let mut paths: Vec<(String, Oid)> = mods
        .iter()
        .filter_map(|m| {
            index
                .entries
                .iter()
                .find(|e| e.stage() == 0 && e.path == m.path && e.mode == 0o160000)
                .map(|e| (m.path.clone(), e.oid))
        })
        .collect();
    for e in index
        .entries
        .iter()
        .filter(|e| e.stage() == 0 && e.mode == 0o160000)
    {
        if !paths.iter().any(|(p, _)| p == &e.path) {
            paths.push((e.path.clone(), e.oid));
        }
    }
    paths.sort_by(|a, b| a.0.cmp(&b.0));
    for (path, recorded) in paths {
        if !filters.is_empty() && !filters.iter().any(|f| f == &path) {
            continue;
        }
        let abs = work.join(&path);
        let display = if prefix.is_empty() {
            path.clone()
        } else {
            format!("{prefix}/{path}")
        };
        let (prefix_ch, shown) = if !abs.join(".git").exists() && !abs.join(".git").is_file() {
            // Not initialized / no checkout
            ('-', recorded)
        } else if let Ok(sub) = Repository::discover(&abs) {
            match sub.head_oid() {
                Ok(head) if head == recorded => (' ', recorded),
                Ok(head) => ('+', head),
                Err(_) => ('-', recorded),
            }
        } else {
            ('-', recorded)
        };
        println!("{prefix_ch}{} {display}", shown.to_hex());
        // SUB-006 (M20 Cx-4): gitlink vs nested checkout mismatch honesty.
        // Cite: https://git-scm.com/docs/gitsubmodules (gitlink 160000)
        if prefix_ch == '+' {
            eprintln!(
                "note: SUB-006 — gitlink SHA differs from nested checkout HEAD for '{display}'; commit submodule then update superproject (or reset nested) to clear '+'"
            );
        }
        if recursive {
            if let Ok(sub) = Repository::discover(&abs) {
                status_submodules_at(&sub, &[], true, &display)?;
            }
        }
    }
    Ok(())
}

fn sm_deinit(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &["--", "-f", "--force", "--all"])?;
    if args.iter().any(|a| a == "--all") {
        return Err(CliError::NotImplemented(
            "submodule deinit --all not claimed".into(),
        ));
    }
    let force = args.iter().any(|a| a == "-f" || a == "--force");
    let paths: Vec<String> = args
        .iter()
        .filter(|a| a.as_str() != "--" && !a.starts_with('-'))
        .cloned()
        .collect();
    if paths.is_empty() {
        return Err(CliError::Usage(
            "submodule deinit [-f] [--] <path>...".into(),
        ));
    }
    let repo = open_repo()?;
    let work = work_tree(&repo)?;
    let gm = fs::read_to_string(work.join(".gitmodules")).unwrap_or_default();
    let mods = parse_gitmodules(&gm);
    let cfg_path = repo.common_dir().join("config");
    let mut cfg = fs::read_to_string(&cfg_path).unwrap_or_default();

    for path in paths {
        let rel = normalize_rel(&path);
        let name = mods
            .iter()
            .find(|m| m.path == rel || m.name == rel)
            .map(|m| m.name.clone())
            .unwrap_or_else(|| rel.clone());
        let abs = work.join(&rel);
        if abs.exists() {
            if !force && has_local_modifications(&abs)? {
                return Err(CliError::Msg(format!(
                    "Submodule work tree '{rel}' contains local modifications; use -f to discard"
                )));
            }
            // Remove contents but keep directory? Git empties the work tree.
            if abs.is_dir() {
                for entry in fs::read_dir(&abs)? {
                    let entry = entry?;
                    let p = entry.path();
                    if entry.file_type()?.is_dir() {
                        fs::remove_dir_all(&p)?;
                    } else {
                        fs::remove_file(&p)?;
                    }
                }
            }
        }
        cfg = submodule_config_unset_section(&cfg, &name);
        println!("Cleared directory '{rel}'");
    }
    fs::write(&cfg_path, cfg)?;
    Ok(())
}

#[derive(Debug, Clone)]
struct SubmoduleMeta {
    name: String,
    path: String,
    url: String,
}

fn parse_gitmodules(text: &str) -> Vec<SubmoduleMeta> {
    let mut out = Vec::new();
    let mut name: Option<String> = None;
    let mut path = String::new();
    let mut url = String::new();
    let flush = |out: &mut Vec<SubmoduleMeta>,
                 name: &mut Option<String>,
                 path: &mut String,
                 url: &mut String| {
        if let Some(n) = name.take() {
            let p = if path.is_empty() {
                n.clone()
            } else {
                path.clone()
            };
            out.push(SubmoduleMeta {
                name: n,
                path: p,
                url: url.clone(),
            });
            path.clear();
            url.clear();
        }
    };
    for line in text.lines() {
        let t = line.trim();
        if t.starts_with('[') && t.ends_with(']') {
            flush(&mut out, &mut name, &mut path, &mut url);
            if let Some(n) = parse_submodule_section(t) {
                name = Some(n);
            }
            continue;
        }
        if name.is_none() {
            continue;
        }
        if let Some((k, v)) = t.split_once('=') {
            match k.trim() {
                "path" => path = v.trim().to_string(),
                "url" => url = v.trim().to_string(),
                _ => {}
            }
        }
    }
    flush(&mut out, &mut name, &mut path, &mut url);
    out
}

fn parse_submodule_section(header: &str) -> Option<String> {
    // [submodule "name"]
    let inner = header.trim().trim_start_matches('[').trim_end_matches(']');
    let rest = inner.strip_prefix("submodule")?.trim();
    let name = rest.trim().trim_matches('"');
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

fn submodule_config_get(text: &str, name: &str, key: &str) -> Option<String> {
    let mut cur = None;
    for line in text.lines() {
        let t = line.trim();
        if t.starts_with('[') && t.ends_with(']') {
            cur = parse_submodule_section(t);
            continue;
        }
        if cur.as_deref() == Some(name) {
            if let Some((k, v)) = t.split_once('=') {
                if k.trim() == key {
                    return Some(v.trim().to_string());
                }
            }
        }
    }
    None
}

fn submodule_config_set(text: &str, name: &str, key: &str, value: &str) -> CliResult<String> {
    if submodule_config_get(text, name, key).is_some() {
        let mut out = String::new();
        let mut cur = None;
        for line in text.split_inclusive('\n') {
            let t = line.trim();
            if t.starts_with('[') && t.ends_with(']') {
                cur = parse_submodule_section(t);
                out.push_str(line);
                continue;
            }
            if cur.as_deref() == Some(name)
                && t.split_once('=')
                    .map(|(k, _)| k.trim() == key)
                    .unwrap_or(false)
            {
                out.push_str(&format!("\t{key} = {value}\n"));
                continue;
            }
            out.push_str(line);
        }
        return Ok(out);
    }
    let mut text = text.to_string();
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    let header = format!("[submodule \"{name}\"]");
    if text.contains(&header) {
        // Insert key after header line.
        let mut out = String::new();
        for line in text.split_inclusive('\n') {
            out.push_str(line);
            if line.trim() == header {
                out.push_str(&format!("\t{key} = {value}\n"));
            }
        }
        Ok(out)
    } else {
        text.push_str(&format!("{header}\n\t{key} = {value}\n"));
        Ok(text)
    }
}

fn submodule_config_unset_section(text: &str, name: &str) -> String {
    let mut out = String::new();
    let mut skip = false;
    for line in text.split_inclusive('\n') {
        let t = line.trim();
        if t.starts_with('[') && t.ends_with(']') {
            skip = parse_submodule_section(t).as_deref() == Some(name);
            if skip {
                continue;
            }
        }
        if skip {
            continue;
        }
        out.push_str(line);
    }
    out
}

fn absorb_gitdir(super_repo: &Repository, worktree_path: &Path, rel: &str) -> CliResult<()> {
    let modules = super_repo.common_dir().join("modules").join(rel);
    if let Some(parent) = modules.parent() {
        fs::create_dir_all(parent)?;
    }
    let nested_git = worktree_path.join(".git");
    if nested_git.is_dir() {
        if modules.exists() {
            fs::remove_dir_all(&modules)?;
        }
        fs::rename(&nested_git, &modules)?;
    } else if nested_git.is_file() {
        // Already a gitfile — leave as-is if pointing into modules.
        return Ok(());
    } else {
        return Err(CliError::Msg(format!(
            "submodule at '{}' has no .git",
            worktree_path.display()
        )));
    }
    let modules_abs = strip_verbatim(fs::canonicalize(&modules).unwrap_or(modules.clone()));
    fs::write(
        worktree_path.join(".git"),
        format!("gitdir: {}\n", modules_abs.display()),
    )?;
    // Git-compat: record core.worktree on the absorbed modules dir (submodule lock domain).
    // Use forward slashes so stock git-config parsers accept the value on Windows.
    let wt_abs =
        strip_verbatim(fs::canonicalize(worktree_path).unwrap_or(worktree_path.to_path_buf()));
    let wt_cfg = config_safe_path(&wt_abs.to_string_lossy());
    let cfg_path = modules.join("config");
    let mut cfg = fs::read_to_string(&cfg_path).unwrap_or_default();
    if !cfg.contains("worktree") {
        if !cfg.contains("[core]") {
            cfg.push_str("[core]\n");
        }
        // Insert after [core] if needed — simple append of worktree line under core.
        if let Some(idx) = cfg.find("[core]") {
            let insert_at = idx + "[core]".len();
            cfg.insert_str(insert_at, &format!("\n\tworktree = {wt_cfg}\n"));
        } else {
            cfg.push_str(&format!("[core]\n\tworktree = {wt_cfg}\n"));
        }
        fs::write(&cfg_path, cfg)?;
    }
    Ok(())
}

fn config_safe_path(raw: &str) -> String {
    raw.replace('\\', "/")
}

fn strip_verbatim(path: std::path::PathBuf) -> std::path::PathBuf {
    let s = path.to_string_lossy();
    if let Some(rest) = s.strip_prefix(r"\\?\") {
        std::path::PathBuf::from(rest)
    } else {
        path
    }
}

fn ensure_submodule_checkout(
    super_repo: &Repository,
    abs: &Path,
    rel: &str,
    url: &str,
    commit: &Oid,
) -> CliResult<()> {
    let modules = super_repo.common_dir().join("modules").join(rel);
    if !modules.join("HEAD").is_file() {
        // Need to clone into modules then link.
        if abs.exists() {
            // clear empty dir from checkout of gitlink
            if abs.is_dir() {
                let _ = fs::remove_dir_all(abs);
            }
        }
        if let Some(parent) = abs.parent() {
            fs::create_dir_all(parent)?;
        }
        if SshRemoteUrl::parse(url).is_ok() {
            clone_ssh(url, abs).map_err(|e| CliError::Msg(e.to_string()))?;
        } else if HttpRemoteUrl::parse(url).is_ok() {
            clone_http(url, abs).map_err(|e| CliError::Msg(e.to_string()))?;
        } else if GitDaemonUrl::parse(url).is_ok() {
            clone_git(url, abs).map_err(|e| CliError::Msg(e.to_string()))?;
        } else {
            clone_local(url, abs).map_err(|e| CliError::Msg(e.to_string()))?;
        }
        absorb_gitdir(super_repo, abs, rel)?;
    } else if !abs.exists() || (!abs.join(".git").exists() && !abs.join(".git").is_file()) {
        fs::create_dir_all(abs)?;
        let modules_abs = fs::canonicalize(&modules).unwrap_or(modules.clone());
        fs::write(
            abs.join(".git"),
            format!("gitdir: {}\n", modules_abs.display()),
        )?;
    }
    let discovered = Repository::discover(abs)?;
    let git_dir = discovered.git_dir().to_path_buf();
    // Ensure object exists in submodule store; for local URLs, merge objects from source
    // (needed for `update --remote` tips newer than the initial clone).
    {
        let probe = Repository::open(&git_dir)?;
        if probe.get_object(commit).is_err() {
            if let Ok(local) = LocalRemoteUrl::parse(url) {
                if let Ok(source) =
                    Repository::discover(local.path()).or_else(|_| Repository::open(local.path()))
                {
                    merge_objects_dir(source.git_dir().join("objects"), git_dir.join("objects"))?;
                }
            }
        }
    }
    let sub = Repository::open(&git_dir)?;
    if sub.get_object(commit).is_err() {
        return Err(CliError::Msg(format!(
            "submodule '{rel}' missing commit {}",
            commit.to_hex()
        )));
    }
    // Detached HEAD at recorded/remote tip.
    fs::write(git_dir.join("HEAD"), format!("{}\n", commit.to_hex()))?;
    let sub = Repository::discover(abs)?;
    arks_remote::checkout_commit(&sub, abs, commit).map_err(|e| CliError::Msg(e.to_string()))?;
    Ok(())
}

/// Merge-copy loose + pack object files from `src` into `dst` (local fetch subset).
fn merge_objects_dir(src: std::path::PathBuf, dst: std::path::PathBuf) -> CliResult<()> {
    if !src.is_dir() {
        return Ok(());
    }
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(&src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let to = dst.join(entry.file_name());
        if ty.is_dir() {
            merge_objects_dir(entry.path(), to)?;
        } else if ty.is_file() {
            if to.exists() {
                continue;
            }
            if let Some(parent) = to.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(entry.path(), &to)?;
        }
    }
    Ok(())
}

fn has_local_modifications(path: &Path) -> CliResult<bool> {
    if !path.exists() {
        return Ok(false);
    }
    // If we cannot open as repo, treat non-empty as dirty unless force.
    let Ok(repo) = Repository::discover(path) else {
        return Ok(fs::read_dir(path)?.next().is_some());
    };
    let Ok(work) = work_tree(&repo) else {
        return Ok(false);
    };
    let index = read_index(&repo)?;
    for e in index.entries.iter().filter(|e| e.stage() == 0) {
        let p = work.join(&e.path);
        if !p.is_file() {
            return Ok(true);
        }
        let bytes = fs::read(&p)?;
        let digest = arks_hash::hash_object(arks_hash::ObjectFormat::Sha1, "blob", &bytes)
            .map_err(|err| CliError::Msg(err.to_string()))?;
        let oid = Oid::from_bytes(repo.oid_kind(), digest.as_bytes())
            .map_err(|err| CliError::Msg(err.to_string()))?;
        if oid != e.oid {
            return Ok(true);
        }
    }
    Ok(false)
}

fn normalize_rel(path: &str) -> String {
    path.replace('\\', "/").trim_matches('/').to_string()
}

#[allow(dead_code)]
fn _rel_helper(work: &Path, path: &Path) -> CliResult<String> {
    rel_path_from_worktree(work, path)
}
