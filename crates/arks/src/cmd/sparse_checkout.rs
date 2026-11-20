//! `arks sparse-checkout`: init, set, add, list, disable, reapply and
//! check-rules in cone and non-cone mode (skip-worktree outside the rules).
//!
//! Primary: <https://git-scm.com/docs/git-sparse-checkout>

use crate::error::{CliError, CliResult};
use crate::util::{open_repo, read_index, work_tree, write_index};
use arks_index::{CE_EXTENDED, IndexEntry};
use std::fs;
use std::path::Path;

/// Extended-flag bit: skip-worktree (`gitformat-index` / cache.h CE_SKIP_WORKTREE).
const CE_SKIP_WORKTREE: u16 = 0x4000;

const USAGE: &str = "usage: git sparse-checkout (init | list | set | add | reapply | disable | check-rules) [<options>]";

pub fn cmd_sparse_checkout(args: &[String]) -> CliResult<()> {
    // Every subcommand and option of `git sparse-checkout -h`.
    // Cite: https://git-scm.com/docs/git-sparse-checkout
    use crate::cmd::opt_parse::{Mode, parse};
    use crate::cmd::porcelain_opts as t;
    let Some(sub) = args.first().map(String::as_str) else {
        return Err(CliError::Exit(
            129,
            format!("error: need a subcommand\n{USAGE}\n"),
        ));
    };
    let rest = &args[1..];
    let p = |defs| {
        parse(
            &format!("sparse-checkout {sub}"),
            rest,
            defs,
            Mode::default(),
        )
    };
    match sub {
        "init" => {
            let p = p(t::SPARSE_CHECKOUT__INIT)?;
            sparse_init(p.flag("cone") != Some(false), p.flag("sparse-index"))
        }
        "set" | "add" => {
            let defs = if sub == "set" {
                t::SPARSE_CHECKOUT__SET
            } else {
                t::SPARSE_CHECKOUT__ADD
            };
            let p = p(defs)?;
            let mut pats = p.args();
            pats.extend(p.after_dashdash());
            if p.on("stdin") {
                let mut s = String::new();
                std::io::Read::read_to_string(&mut std::io::stdin(), &mut s)?;
                pats.extend(
                    s.lines()
                        .map(|l| l.trim().to_string())
                        .filter(|l| !l.is_empty()),
                );
            }
            let repo = open_repo()?;
            let cfg = arks_config::ConfigSet::load(Some(repo.common_dir()));
            let truthy = |k: &str| {
                cfg.get(k)
                    .map(|v| matches!(v.to_ascii_lowercase().as_str(), "true" | "yes" | "on" | "1"))
            };
            let existing_cone = truthy("core.sparseCheckoutCone").unwrap_or(true);
            if sub == "add" {
                if truthy("core.sparseCheckout") != Some(true) {
                    return Err(CliError::fatal("no sparse-checkout to add to"));
                }
                let old = fs::read_to_string(repo.git_dir().join("info/sparse-checkout"))
                    .unwrap_or_default();
                let mut all: Vec<String> = if existing_cone {
                    cone_dirs_from_patterns(&old)
                } else {
                    old.lines().map(String::from).collect()
                };
                all.extend(pats);
                drop(repo);
                return sparse_set(&all, existing_cone, None);
            }
            let cone = p.flag("cone").unwrap_or(existing_cone);
            drop(repo);
            sparse_set(&pats, cone, p.flag("sparse-index"))
        }
        "list" => {
            let _ = p(&[])?;
            sparse_list()
        }
        "disable" => {
            let _ = p(&[])?;
            sparse_disable()
        }
        "reapply" => {
            let p = p(t::SPARSE_CHECKOUT__REAPPLY)?;
            sparse_reapply(p.flag("cone"), p.flag("sparse-index"))
        }
        "check-rules" => {
            let p = p(t::SPARSE_CHECKOUT__CHECK_RULES)?;
            check_rules(p.on("-z"), p.flag("cone"), p.value("rules-file"))
        }
        other => Err(CliError::Exit(
            129,
            format!("error: unknown subcommand: `{other}'\n{USAGE}\n"),
        )),
    }
}

fn sparse_init(cone: bool, sparse_index: Option<bool>) -> CliResult<()> {
    let repo = open_repo()?;
    set_config_bool(repo.git_dir(), "core.sparseCheckout", true)?;
    set_config_bool(repo.git_dir(), "core.sparseCheckoutCone", cone)?;
    if let Some(si) = sparse_index {
        set_config_bool(repo.git_dir(), "index.sparse", si)?;
    }
    let info = repo.git_dir().join("info");
    fs::create_dir_all(&info)?;
    let path = info.join("sparse-checkout");
    if !path.is_file() {
        // Baseline for both modes: everything at the root, no subdirectories.
        fs::write(&path, "/*\n!/*/\n")?;
    }
    Ok(())
}

fn sparse_set(pats: &[String], cone: bool, sparse_index: Option<bool>) -> CliResult<()> {
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
    set_config_bool(repo.git_dir(), "core.sparseCheckout", true)?;
    set_config_bool(repo.git_dir(), "core.sparseCheckoutCone", cone)?;
    if let Some(si) = sparse_index {
        set_config_bool(repo.git_dir(), "index.sparse", si)?;
    }
    let dirs: Vec<&str> = pats.iter().map(|d| d.trim_matches('/')).collect();
    let lines: Vec<String> = if cone {
        cone_patterns(&dirs)
    } else if pats.is_empty() {
        vec!["/*".into(), "!/*/".into()]
    } else {
        pats.to_vec()
    };
    let info = repo.git_dir().join("info");
    fs::create_dir_all(&info)?;
    fs::write(info.join("sparse-checkout"), lines.join("\n") + "\n")?;
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
        let included = if cone {
            path_in_cone(&entry.path, &dirs)
        } else {
            matches_patterns(&entry.path, &lines)
        };
        let abs = work.join(&entry.path);
        if !included {
            // Files with local changes stay (git warns and leaves them).
            let dirty = match fs::read(&abs) {
                Ok(b) => {
                    arks_object::RawObject::new(arks_object::ObjectKind::Blob, b)
                        .oid(repo.object_format())
                        .ok()
                        != Some(entry.oid)
                }
                Err(_) => false,
            };
            if dirty {
                continue;
            }
            set_skip_worktree(entry, true);
            if abs.is_file() {
                let _ = fs::remove_file(&abs);
                if let Some(parent) = abs.parent() {
                    let _ = remove_empty_parents(parent, &work);
                }
            }
        } else {
            let was_skipped = entry
                .flags_extended
                .is_some_and(|e| e & CE_SKIP_WORKTREE != 0);
            set_skip_worktree(entry, false);
            if was_skipped && !abs.exists() {
                if let Some(parent) = abs.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(&abs, repo.get_object(&entry.oid)?.payload)?;
            }
        }
    }
    write_index(&repo, &index)?;
    let _ = &mut repo;
    Ok(())
}

/// Non-cone patterns (gitignore syntax, last match wins, `!` negates).
fn matches_patterns(path: &str, lines: &[String]) -> bool {
    let mut included = false;
    for raw in lines {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (neg, pat) = match line.strip_prefix('!') {
            Some(p) => (true, p),
            None => (false, line),
        };
        let dir_only = pat.ends_with('/');
        let pat = pat.trim_end_matches('/');
        let anchored = pat.starts_with('/') || pat.contains('/');
        let pat = pat.trim_start_matches('/');
        let comps: Vec<&str> = path.split('/').collect();
        let hit = if anchored {
            // Match the whole path or one of its leading directories.
            (1..=comps.len()).any(|n| {
                let prefix = comps[..n].join("/");
                let is_dir = n < comps.len();
                (!dir_only || is_dir) && glob_path(pat, &prefix)
            })
        } else {
            comps.iter().enumerate().any(|(i, c)| {
                let is_dir = i + 1 < comps.len();
                (!dir_only || is_dir) && glob_path(pat, c)
            })
        };
        if hit {
            included = !neg;
        }
    }
    included
}

/// fnmatch with FNM_PATHNAME: `*` and `?` do not cross `/`.
fn glob_path(pat: &str, s: &str) -> bool {
    fn rec(p: &[char], t: &[char]) -> bool {
        match p.first() {
            None => t.is_empty(),
            Some('*') => (0..=t.len())
                .take_while(|i| *i == 0 || t[i - 1] != '/')
                .any(|i| rec(&p[1..], &t[i..])),
            Some('?') => !t.is_empty() && t[0] != '/' && rec(&p[1..], &t[1..]),
            Some(c) => t.first() == Some(c) && rec(&p[1..], &t[1..]),
        }
    }
    let p: Vec<char> = pat.chars().collect();
    let t: Vec<char> = s.chars().collect();
    rec(&p, &t)
}

fn is_sparse(repo: &arks_repo::Repository) -> bool {
    arks_config::ConfigSet::load(Some(repo.common_dir()))
        .get("core.sparseCheckout")
        .is_some_and(|v| matches!(v.to_ascii_lowercase().as_str(), "true" | "yes" | "on" | "1"))
}

fn sparse_list() -> CliResult<()> {
    let repo = open_repo()?;
    if !is_sparse(&repo) {
        return Err(CliError::fatal("this worktree is not sparse"));
    }
    let path = repo.git_dir().join("info/sparse-checkout");
    if !path.is_file() {
        return Ok(());
    }
    let text = fs::read_to_string(&path)?;
    let cone = arks_config::ConfigSet::load(Some(repo.common_dir()))
        .get("core.sparseCheckoutCone")
        .is_none_or(|v| matches!(v.to_ascii_lowercase().as_str(), "true" | "yes" | "on" | "1"));
    if cone {
        // Cone mode lists the directories (only the deepest of each chain).
        let dirs = cone_dirs_from_patterns(&text);
        for d in &dirs {
            if !dirs
                .iter()
                .any(|o| o != d && o.starts_with(&format!("{d}/")))
            {
                println!("{d}");
            }
        }
    } else {
        print!("{text}");
    }
    Ok(())
}

fn sparse_disable() -> CliResult<()> {
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

fn sparse_reapply(cone: Option<bool>, sparse_index: Option<bool>) -> CliResult<()> {
    let repo = open_repo()?;
    if !is_sparse(&repo) {
        return Err(CliError::fatal(
            "must be in a sparse-checkout to reapply sparsity patterns",
        ));
    }
    let path = repo.git_dir().join("info/sparse-checkout");
    let text = fs::read_to_string(&path).unwrap_or_default();
    let was_cone = arks_config::ConfigSet::load(Some(repo.common_dir()))
        .get("core.sparseCheckoutCone")
        .is_none_or(|v| matches!(v.to_ascii_lowercase().as_str(), "true" | "yes" | "on" | "1"));
    let cone = cone.unwrap_or(was_cone);
    let pats: Vec<String> = if cone {
        cone_dirs_from_patterns(&text)
    } else {
        text.lines().map(String::from).collect()
    };
    drop(repo);
    sparse_set(&pats, cone, sparse_index)
}

/// `check-rules`: print the paths from stdin that the rules include.
fn check_rules(nul: bool, cone: Option<bool>, rules_file: Option<String>) -> CliResult<()> {
    let repo = open_repo()?;
    let (text, default_cone) = match &rules_file {
        Some(f) => (
            fs::read_to_string(f)
                .map_err(|_| CliError::fatal("unable to load existing sparse-checkout patterns"))?,
            true,
        ),
        None => {
            let t = fs::read_to_string(repo.git_dir().join("info/sparse-checkout"))
                .map_err(|_| CliError::fatal("unable to load existing sparse-checkout patterns"))?;
            let c = arks_config::ConfigSet::load(Some(repo.common_dir()))
                .get("core.sparseCheckoutCone")
                .is_none_or(|v| {
                    matches!(v.to_ascii_lowercase().as_str(), "true" | "yes" | "on" | "1")
                });
            (t, c)
        }
    };
    let cone = cone.unwrap_or(default_cone);
    let lines: Vec<String> = if cone && rules_file.is_some() {
        // A rules file in cone mode lists directories.
        let dirs: Vec<&str> = text.lines().map(|l| l.trim().trim_matches('/')).collect();
        cone_patterns(&dirs)
    } else {
        text.lines().map(String::from).collect()
    };
    let dirs = cone_dirs_from_patterns(&lines.join("\n"));
    let dirs: Vec<&str> = dirs.iter().map(String::as_str).collect();
    let mut input = String::new();
    std::io::Read::read_to_string(&mut std::io::stdin(), &mut input)?;
    let sep = if nul { '\0' } else { '\n' };
    for path in input.split(sep).filter(|p| !p.is_empty()) {
        let hit = if cone {
            path_in_cone(path, &dirs)
        } else {
            matches_patterns(path, &lines)
        };
        if hit {
            print!("{path}{sep}");
        }
    }
    Ok(())
}

fn cone_dirs_from_patterns(text: &str) -> Vec<String> {
    let mut dirs = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line == "/*" || line.starts_with('!') || line.is_empty() || line.starts_with('#') {
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
    // Each directory's parents appear as `/<parent>/` plus `!/<parent>/*/`.
    let mut out: Vec<String> = vec!["/*".into(), "!/*/".into()];
    let mut all: Vec<String> = Vec::new();
    for d in dirs {
        let d = d.trim_matches('/');
        if d.is_empty() {
            continue;
        }
        let parts: Vec<&str> = d.split('/').collect();
        for n in 1..=parts.len() {
            let p = parts[..n].join("/");
            if !all.contains(&p) {
                all.push(p);
            }
        }
    }
    all.sort();
    for p in &all {
        out.push(format!("/{p}/"));
        if all.iter().any(|o| o.starts_with(&format!("{p}/"))) {
            out.push(format!("!/{p}/*/"));
        }
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
