//! `arks sparse-checkout` — claimed Fs-3 cone subset.
//!
//! Primary: <https://git-scm.com/docs/git-sparse-checkout>
//! Cone: `init --cone`, `set <dir>...`, `list`. Applies skip-worktree outside cone.

use crate::error::{CliError, CliResult};
use crate::util::{open_repo, read_index, refuse_unknown_flags, work_tree, write_index};
use arks_index::{CE_EXTENDED, IndexEntry};
use std::fs;
use std::path::Path;

/// Extended-flag bit: skip-worktree (`gitformat-index` / cache.h CE_SKIP_WORKTREE).
const CE_SKIP_WORKTREE: u16 = 0x4000;

pub fn cmd_sparse_checkout(args: &[String]) -> CliResult<()> {
    // M16 Sy-5: disable / reapply claimed (cone subset).
    let sub = args.first().map(String::as_str).ok_or_else(|| {
        CliError::Usage("sparse-checkout <init|set|list|disable|reapply> ...".into())
    })?;
    match sub {
        "init" => sparse_init(&args[1..]),
        "set" => sparse_set(&args[1..]),
        "list" => sparse_list(&args[1..]),
        "disable" => sparse_disable(&args[1..]),
        "reapply" => sparse_reapply(&args[1..]),
        other => Err(CliError::NotImplemented(format!(
            "sparse-checkout {other} not claimed (SPARSE-002 — non-cone / expand stampede refuse; Sy-5: init --cone / set / list / disable / reapply)"
        ))),
    }
}

fn sparse_init(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &["--cone"])?;
    if !args.iter().any(|a| a == "--cone") {
        return Err(CliError::NotImplemented(
            "sparse-checkout init requires --cone (SPARSE-002 — non-cone sparse rewrite refuse; Fs-3 cone claim only)".into(),
        ));
    }
    let repo = open_repo()?;
    set_config_bool(repo.git_dir(), "core.sparseCheckout", true)?;
    set_config_bool(repo.git_dir(), "core.sparseCheckoutCone", true)?;
    let info = repo.git_dir().join("info");
    fs::create_dir_all(&info)?;
    let path = info.join("sparse-checkout");
    if !path.is_file() {
        // Cone baseline: everything at root, no subdirs yet.
        fs::write(&path, "/*\n!/*/\n")?;
    }
    Ok(())
}

fn sparse_set(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &[])?;
    let dirs: Vec<&str> = args
        .iter()
        .filter(|a| !a.starts_with('-'))
        .map(String::as_str)
        .collect();
    if dirs.is_empty() {
        return Err(CliError::Usage("sparse-checkout set <dir>...".into()));
    }
    let mut repo = open_repo()?;
    // SPARSE-001 (M20 Cx-3): concurrent sparse-checkout cone file rewrite.
    // Cite: https://git-scm.com/docs/git-sparse-checkout + layout info/sparse-checkout
    let sparse_lock = repo.git_dir().join("info/sparse-checkout.lock");
    if sparse_lock.is_file() {
        return Err(CliError::Msg(format!(
            "Unable to create '{}' (SPARSE-001 — concurrent sparse-checkout cone edits fail-closed)",
            sparse_lock.display()
        )));
    }
    let index_lock = arks_index::index_lock_path(&repo.index_path());
    if index_lock.is_file() {
        return Err(CliError::Msg(format!(
            "{} (MISC-018 / MISC-008 — sparse-checkout set/reapply under index.lock refuse; parallel cone vs materialize)",
            index_lock.display()
        )));
    }
    // Ensure cone mode is on (stock requires init first; we accept set after init).
    set_config_bool(repo.git_dir(), "core.sparseCheckout", true)?;
    set_config_bool(repo.git_dir(), "core.sparseCheckoutCone", true)?;
    let patterns = cone_patterns(&dirs);
    let info = repo.git_dir().join("info");
    fs::create_dir_all(&info)?;
    fs::write(info.join("sparse-checkout"), patterns.join("\n") + "\n")?;

    let work = work_tree(&repo)?;
    let mut index = read_index(&repo)?;
    // skip-worktree uses CE_EXTENDED → index version >= 3 (gitformat-index).
    if index.version < 3 {
        index.version = 3;
    }
    for entry in &mut index.entries {
        if entry.stage() != 0 {
            continue;
        }
        let in_cone = path_in_cone(&entry.path, &dirs);
        set_skip_worktree(entry, !in_cone);
        let abs = work.join(&entry.path);
        if !in_cone && abs.is_file() {
            let _ = fs::remove_file(&abs);
            // Best-effort prune empty parent dirs inside the work tree.
            if let Some(parent) = abs.parent() {
                let _ = remove_empty_parents(parent, &work);
            }
        }
    }
    write_index(&repo, &index)?;
    let _ = &mut repo;
    Ok(())
}

fn sparse_list(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &[])?;
    let repo = open_repo()?;
    let path = repo.git_dir().join("info/sparse-checkout");
    if !path.is_file() {
        return Ok(());
    }
    let text = fs::read_to_string(&path)?;
    print!("{text}");
    Ok(())
}

fn sparse_disable(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &[])?;
    let mut repo = open_repo()?;
    set_config_bool(repo.git_dir(), "core.sparseCheckout", false)?;
    let work = work_tree(&repo)?;
    let mut index = read_index(&repo)?;
    for entry in &mut index.entries {
        if entry.stage() != 0 {
            continue;
        }
        set_skip_worktree(entry, false);
    }
    write_index(&repo, &index)?;
    // Restore skipped files from HEAD when present.
    if let Ok(head) = repo.head_oid() {
        arks_remote::checkout_commit(&repo, &work, &head)?;
    }
    let _ = &mut repo;
    Ok(())
}

fn sparse_reapply(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &[])?;
    let repo = open_repo()?;
    let path = repo.git_dir().join("info/sparse-checkout");
    if !path.is_file() {
        return Err(CliError::Msg(
            "sparse-checkout reapply: no sparse-checkout file".into(),
        ));
    }
    let text = fs::read_to_string(&path)?;
    let dirs = cone_dirs_from_patterns(&text);
    drop(repo);
    // Reuse set path with reconstructed dirs.
    let dir_args: Vec<String> = dirs;
    sparse_set(&dir_args)
}

fn cone_dirs_from_patterns(text: &str) -> Vec<String> {
    let mut dirs = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line == "/*" || line == "!/*/" || line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(rest) = line.strip_prefix('/') {
            let d = rest.trim_matches('/');
            if !d.is_empty() {
                dirs.push(d.to_owned());
            }
        }
    }
    dirs
}

fn cone_patterns(dirs: &[&str]) -> Vec<String> {
    // Pin Git 2.39 cone file: `/*`, `!/*/`, then `/dir/` per directory
    // (recursive membership is implied by cone mode — no `/**` lines).
    // Cite: https://git-scm.com/docs/git-sparse-checkout
    let mut out = vec!["/*".into(), "!/*/".into()];
    for d in dirs {
        let d = d.trim_matches('/');
        if d.is_empty() {
            continue;
        }
        out.push(format!("/{d}/"));
    }
    out
}

fn path_in_cone(path: &str, dirs: &[&str]) -> bool {
    // Cone: tracked files at repo root always included; under dirs when prefix matches.
    if !path.contains('/') {
        return true;
    }
    for d in dirs {
        let d = d.trim_matches('/');
        if d.is_empty() {
            continue;
        }
        if path == d || path.starts_with(&format!("{d}/")) {
            return true;
        }
    }
    false
}

fn set_skip_worktree(entry: &mut IndexEntry, skip: bool) {
    let mut flags = entry.flags;
    let mut ext = entry.flags_extended.unwrap_or(0);
    if skip {
        flags |= CE_EXTENDED;
        ext |= CE_SKIP_WORKTREE;
        entry.flags = flags;
        entry.flags_extended = Some(ext);
    } else {
        ext &= !CE_SKIP_WORKTREE;
        if ext == 0 {
            entry.flags = flags & !CE_EXTENDED;
            entry.flags_extended = None;
        } else {
            entry.flags = flags | CE_EXTENDED;
            entry.flags_extended = Some(ext);
        }
    }
}

fn set_config_bool(git_dir: &Path, key: &str, value: bool) -> CliResult<()> {
    let cfg_path = git_dir.join("config");
    let text = fs::read_to_string(&cfg_path).unwrap_or_default();
    let val = if value { "true" } else { "false" };
    let new = config_set_simple(&text, key, val)?;
    fs::write(&cfg_path, new)?;
    Ok(())
}

fn config_set_simple(text: &str, key: &str, value: &str) -> CliResult<String> {
    let (section, name) = key
        .rsplit_once('.')
        .ok_or_else(|| CliError::Usage(format!("bad config key {key}")))?;
    // Reuse the same append/replace strategy as cmd/config.rs (local only).
    if let Some(existing) = config_get_simple(text, section, name) {
        if existing == value {
            return Ok(text.to_string());
        }
        let mut out = String::new();
        let mut cur = None;
        for line in text.split_inclusive('\n') {
            let t = line.trim();
            if t.starts_with('[') && t.ends_with(']') {
                cur = Some(t[1..t.len() - 1].trim().to_string());
                out.push_str(line);
                continue;
            }
            if let Some(sec) = &cur {
                if (sec.trim_matches('"') == section || *sec == section)
                    && t.split_once('=')
                        .map(|(k, _)| k.trim() == name)
                        .unwrap_or(false)
                {
                    out.push_str(&format!("\t{name} = {value}\n"));
                    continue;
                }
            }
            out.push_str(line);
        }
        return Ok(out);
    }
    let mut text = text.to_string();
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    text.push_str(&format!("[{section}]\n\t{name} = {value}\n"));
    Ok(text)
}

fn config_get_simple(text: &str, section: &str, name: &str) -> Option<String> {
    let mut cur = None;
    for line in text.lines() {
        let t = line.trim();
        if t.starts_with('[') && t.ends_with(']') {
            cur = Some(t[1..t.len() - 1].trim().trim_matches('"').to_string());
            continue;
        }
        if let Some(sec) = &cur {
            if sec.trim_matches('"') == section || *sec == section {
                if let Some((k, v)) = t.split_once('=') {
                    if k.trim() == name {
                        return Some(v.trim().to_string());
                    }
                }
            }
        }
    }
    None
}

fn remove_empty_parents(mut dir: &Path, work: &Path) -> std::io::Result<()> {
    while dir.starts_with(work) && dir != work {
        match fs::remove_dir(dir) {
            Ok(()) => {
                if let Some(parent) = dir.parent() {
                    dir = parent;
                } else {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    Ok(())
}
