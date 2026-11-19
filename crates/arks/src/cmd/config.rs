//! Local repository config get/set/list/unset.

use crate::error::{CliError, CliResult};
use crate::util::{open_repo, refuse_unknown_flags};
use std::fs;
pub fn cmd_config(args: &[String]) -> CliResult<()> {
    // M16 Sy-1: `--local` / `--global` / `--system` / `--worktree` claimed (git-config).
    // Cite: https://git-scm.com/docs/git-config
    if let Some(done) = query(args)? {
        return Ok(done);
    }
    refuse_unknown_flags(
        args,
        &[
            "--get",
            "--set",
            "--list",
            "--unset",
            "--local",
            "--global",
            "--system",
            "--worktree",
        ],
    )?;
    if let Some(done) = merged_read(args)? {
        return Ok(done);
    }
    let want_global = args.iter().any(|a| a == "--global");
    let want_system = args.iter().any(|a| a == "--system");
    let want_worktree = args.iter().any(|a| a == "--worktree");
    let cfg_path = if want_system {
        system_config_path()?
    } else if want_global {
        global_config_path()?
    } else if want_worktree {
        let repo = open_repo()?;
        // WT-006 (M20 Cx-4): config.worktree.lock → refuse concurrent worktree config.
        // Cite: https://git-scm.com/docs/git-worktree + extensions.worktreeConfig
        let wt_cfg = repo.git_dir().join("config.worktree");
        let lock = {
            let mut s = wt_cfg.clone().into_os_string();
            s.push(".lock");
            std::path::PathBuf::from(s)
        };
        if lock.is_file() {
            return Err(CliError::Msg(format!(
                "could not lock config file {} (WT-006 — concurrent config.worktree edits refuse; lease ≠ skip)",
                wt_cfg.display()
            )));
        }
        wt_cfg
    } else {
        let repo = open_repo()?;
        // Also honor planted config.worktree.lock for local writes when present
        // (worktree-config race class observable without requiring --worktree).
        let lock = repo.git_dir().join("config.worktree.lock");
        if lock.is_file()
            && (args.iter().any(|a| a == "--set" || a == "--unset")
                || args.iter().filter(|a| !a.starts_with('-')).count() >= 2)
        {
            return Err(CliError::Msg(format!(
                "could not lock config file {} (WT-006 — config.worktree.lock held; refuse concurrent worktree config)",
                repo.git_dir().join("config.worktree").display()
            )));
        }
        repo.git_dir().join("config")
    };
    if want_global || want_system {
        if let Some(parent) = cfg_path.parent() {
            fs::create_dir_all(parent)?;
        }
        if !cfg_path.exists() {
            fs::write(&cfg_path, b"")?;
        }
    }
    if args.iter().any(|a| a == "--list") {
        let text = fs::read_to_string(&cfg_path).unwrap_or_default();
        print_config_list(&text);
        return Ok(());
    }
    if args.iter().any(|a| a == "--get") {
        let key = args
            .iter()
            .find(|a| !a.starts_with('-'))
            .ok_or_else(|| CliError::Usage("config --get <key>".into()))?;
        let text = fs::read_to_string(&cfg_path).unwrap_or_default();
        match config_get(&text, key) {
            Some(v) => {
                println!("{v}");
                Ok(())
            }
            None => Err(CliError::Msg(format!("key not found: {key}"))),
        }
    } else if args.iter().any(|a| a == "--unset") {
        let key = args
            .iter()
            .find(|a| !a.starts_with('-'))
            .ok_or_else(|| CliError::Usage("config --unset <key>".into()))?;
        let text = fs::read_to_string(&cfg_path).unwrap_or_default();
        let new = config_unset(&text, key)?;
        fs::write(&cfg_path, new)?;
        Ok(())
    } else if args.iter().any(|a| a == "--set") {
        let positional: Vec<&String> = args.iter().filter(|a| !a.starts_with('-')).collect();
        if positional.len() != 2 {
            return Err(CliError::Usage("config --set <key> <value>".into()));
        }
        let text = fs::read_to_string(&cfg_path).unwrap_or_default();
        let new = config_set(&text, positional[0], positional[1])?;
        fs::write(&cfg_path, new)?;
        Ok(())
    } else {
        // git-style: config <key> <value> | config <key>
        let positional: Vec<&String> = args.iter().filter(|a| !a.starts_with('-')).collect();
        match positional.len() {
            1 => {
                let text = fs::read_to_string(&cfg_path).unwrap_or_default();
                match config_get(&text, positional[0]) {
                    Some(v) => {
                        println!("{v}");
                        Ok(())
                    }
                    None => Err(CliError::Msg(format!("key not found: {}", positional[0]))),
                }
            }
            2 => {
                let text = fs::read_to_string(&cfg_path).unwrap_or_default();
                let new = config_set(&text, positional[0], positional[1])?;
                fs::write(&cfg_path, new)?;
                Ok(())
            }
            _ => Err(CliError::Usage(
                "config [--get|--set|--list|--unset] ...".into(),
            )),
        }
    }
}

/// Read queries over the merged view (C7b): `--get-regexp <re> [<value-re>]`,
/// `--get-all`, `--show-origin`, `--show-scope`, `--name-only`,
/// `--type=bool|int` (`--bool`, `--int`), `--default`, `-l`; scope flags
/// filter the files read. Returns `None` when none of these is present.
/// Exit 1 (silent) when nothing matches, as git does.
/// Cite: https://git-scm.com/docs/git-config
fn query(args: &[String]) -> CliResult<Option<()>> {
    let triggers = [
        "--get-regexp",
        "--get-all",
        "--show-origin",
        "--show-scope",
        "--name-only",
        "--bool",
        "--int",
        "--default",
        "-l",
    ];
    if !args.iter().any(|a| {
        triggers.contains(&a.as_str()) || a.starts_with("--type=") || a.starts_with("--default=")
    }) {
        return Ok(None);
    }
    let mut op = "get";
    let (mut origin, mut scope_col, mut name_only) = (false, false, false);
    let mut ty: Option<String> = None;
    let mut default: Option<String> = None;
    let mut only: Option<arks_config::ConfigScope> = None;
    let mut pos: Vec<&String> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        i += 1;
        match a {
            "--get" => op = "get",
            "--get-all" => op = "get-all",
            "--get-regexp" => op = "get-regexp",
            "--list" | "-l" => op = "list",
            "--show-origin" => origin = true,
            "--show-scope" => scope_col = true,
            "--name-only" => name_only = true,
            "--bool" => ty = Some("bool".into()),
            "--int" => ty = Some("int".into()),
            "--local" => only = Some(arks_config::ConfigScope::Local),
            "--global" => only = Some(arks_config::ConfigScope::Global),
            "--system" => only = Some(arks_config::ConfigScope::System),
            "--default" => {
                default = args.get(i).cloned();
                i += 1;
            }
            _ if a.starts_with("--type=") => ty = Some(a["--type=".len()..].to_string()),
            _ if a.starts_with("--default=") => default = Some(a["--default=".len()..].to_string()),
            _ if a.starts_with('-') => {
                return Err(CliError::NotImplemented(format!("unsupported option: {a}")));
            }
            _ => pos.push(&args[i - 1]),
        }
    }
    let cwd = std::env::current_dir()?;
    let set = arks_config::ConfigSet::load_discovered(&cwd);
    let entries: Vec<&arks_config::ConfigEntry> = set
        .entries()
        .iter()
        .filter(|e| only.is_none_or(|s| e.scope == s))
        .collect();
    let scope_name = |s: arks_config::ConfigScope| match s {
        arks_config::ConfigScope::System => "system",
        arks_config::ConfigScope::Global => "global",
        arks_config::ConfigScope::Local => "local",
        arks_config::ConfigScope::Env | arks_config::ConfigScope::Command => "command",
    };
    let origin_name = |e: &arks_config::ConfigEntry| match &e.origin {
        Some(p) => {
            let shown = p
                .strip_prefix(&cwd)
                .map(|r| r.to_path_buf())
                .unwrap_or_else(|_| p.clone());
            format!("file:{}", shown.display())
        }
        None => "command line:".to_string(),
    };
    let typed = |v: Option<&str>| -> CliResult<String> {
        match ty.as_deref() {
            Some("bool") => match v.map_or(Some(true), arks_config::parse_bool) {
                Some(b) => Ok(b.to_string()),
                None => Err(CliError::fatal(format!(
                    "bad boolean config value '{}'",
                    v.unwrap_or("")
                ))),
            },
            Some("int") => {
                let raw = v.unwrap_or("");
                let (num, mult) = match raw.chars().last().map(|c| c.to_ascii_lowercase()) {
                    Some('k') => (&raw[..raw.len() - 1], 1024i64),
                    Some('m') => (&raw[..raw.len() - 1], 1024 * 1024),
                    Some('g') => (&raw[..raw.len() - 1], 1024 * 1024 * 1024),
                    _ => (raw, 1),
                };
                num.trim()
                    .parse::<i64>()
                    .map(|n| (n * mult).to_string())
                    .map_err(|_| CliError::fatal(format!("bad numeric config value '{raw}'")))
            }
            _ => Ok(v.unwrap_or("").to_string()),
        }
    };
    let prefix = |e: &arks_config::ConfigEntry| {
        let mut p = String::new();
        if scope_col {
            p.push_str(scope_name(e.scope));
            p.push('\t');
        }
        if origin {
            p.push_str(&origin_name(e));
            p.push('\t');
        }
        p
    };
    let mut out = Vec::new();
    match op {
        "list" => {
            for e in &entries {
                let line = match (&e.value, name_only) {
                    (_, true) | (None, _) => e.key.clone(),
                    (Some(v), false) => format!("{}={v}", e.key),
                };
                out.push(format!("{}{line}", prefix(e)));
            }
            for l in &out {
                println!("{l}");
            }
            return Ok(Some(()));
        }
        "get-regexp" => {
            let re = pos.first().ok_or_else(|| {
                CliError::Usage("config --get-regexp <regex> [<value-regex>]".into())
            })?;
            let key_re = crate::cmd::bre::Bre::with_mode(re, false, true);
            let val_re = pos
                .get(1)
                .map(|v| crate::cmd::bre::Bre::with_mode(v, false, true));
            for e in &entries {
                if !key_re.is_match(e.key.as_bytes()) {
                    continue;
                }
                if let Some(vr) = &val_re {
                    if !vr.is_match(e.value.as_deref().unwrap_or("").as_bytes()) {
                        continue;
                    }
                }
                let line = if name_only || (e.value.is_none() && ty.is_none()) {
                    e.key.clone()
                } else {
                    format!("{} {}", e.key, typed(e.value.as_deref())?)
                };
                out.push(format!("{}{line}", prefix(e)));
            }
        }
        _ => {
            let key = pos
                .first()
                .ok_or_else(|| CliError::Usage("config --get <key>".into()))?;
            let want = normalize_query_key(key);
            let hits: Vec<&&arks_config::ConfigEntry> =
                entries.iter().filter(|e| e.key == want).collect();
            let picked: Vec<&&arks_config::ConfigEntry> = if op == "get-all" {
                hits
            } else {
                hits.last().into_iter().copied().collect()
            };
            for e in picked {
                out.push(format!("{}{}", prefix(e), typed(e.value.as_deref())?));
            }
            if out.is_empty() {
                if let Some(d) = &default {
                    out.push(typed(Some(d))?);
                }
            }
        }
    }
    if out.is_empty() {
        return Err(CliError::silent(1));
    }
    for l in &out {
        println!("{l}");
    }
    Ok(Some(()))
}

/// `section.sub.name` with section and name lower-cased (subsection kept).
fn normalize_query_key(key: &str) -> String {
    let first = key.find('.');
    let last = key.rfind('.');
    match (first, last) {
        (Some(f), Some(l)) if f != l => format!(
            "{}{}{}",
            key[..f].to_ascii_lowercase(),
            &key[f..l],
            key[l..].to_ascii_lowercase()
        ),
        _ => key.to_ascii_lowercase(),
    }
}

/// Unscoped reads see every file in git precedence (git-config §FILES):
/// system → global → repo → `GIT_CONFIG_COUNT` → `-c`
/// ([[sot:root/cloud-agent-dropin]] §C). Returns `None` for writes / scoped ops.
fn merged_read(args: &[String]) -> CliResult<Option<()>> {
    let scoped = args.iter().any(|a| {
        matches!(
            a.as_str(),
            "--local" | "--global" | "--system" | "--worktree"
        )
    });
    let writes = args.iter().any(|a| a == "--set" || a == "--unset");
    let positional: Vec<&String> = args.iter().filter(|a| !a.starts_with('-')).collect();
    let list = args.iter().any(|a| a == "--list");
    let get = args.iter().any(|a| a == "--get") || (!list && positional.len() == 1);
    if scoped || writes || !(list || get) {
        return Ok(None);
    }
    let cwd = std::env::current_dir()?;
    let set = arks_config::ConfigSet::load_discovered(&cwd);
    if list {
        for e in set.entries() {
            match &e.value {
                Some(v) => println!("{}={v}", e.key),
                None => println!("{}", e.key),
            }
        }
        return Ok(Some(()));
    }
    let key = positional
        .first()
        .ok_or_else(|| CliError::Usage("config --get <key>".into()))?;
    match set.get(key) {
        Some(v) => {
            println!("{v}");
            Ok(Some(()))
        }
        None => Err(CliError::Msg(format!("key not found: {key}"))),
    }
}

fn config_get(text: &str, key: &str) -> Option<String> {
    let (section_norm, name) = split_key(key)?;
    let mut cur_norm: Option<String> = None;
    for line in text.lines() {
        let t = line.trim();
        if t.starts_with('[') && t.ends_with(']') {
            cur_norm = Some(normalize_section_inner(&t[1..t.len() - 1]));
            continue;
        }
        if let Some(sec) = &cur_norm {
            if *sec == section_norm {
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

fn config_set(text: &str, key: &str, value: &str) -> CliResult<String> {
    let (section_norm, name) =
        split_key(key).ok_or_else(|| CliError::Usage(format!("config: bad key {key}")))?;
    let header = section_header(&section_norm);
    if config_get(text, key).is_some() {
        let mut out = String::new();
        let mut cur_norm: Option<String> = None;
        for line in text.split_inclusive('\n') {
            let t = line.trim();
            if t.starts_with('[') && t.ends_with(']') {
                cur_norm = Some(normalize_section_inner(&t[1..t.len() - 1]));
                out.push_str(line);
                continue;
            }
            if let Some(sec) = &cur_norm {
                if *sec == section_norm
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
    if text.contains(&header) {
        // append under section: rewrite simply by appending new section override
        text.push_str(&format!("{header}\n\t{name} = {value}\n"));
    } else {
        text.push_str(&format!("{header}\n\t{name} = {value}\n"));
    }
    Ok(text)
}

fn config_unset(text: &str, key: &str) -> CliResult<String> {
    let (section_norm, name) =
        split_key(key).ok_or_else(|| CliError::Usage(format!("config: bad key {key}")))?;
    let mut out = String::new();
    let mut cur_norm: Option<String> = None;
    let mut found = false;
    for line in text.split_inclusive('\n') {
        let t = line.trim();
        if t.starts_with('[') && t.ends_with(']') {
            cur_norm = Some(normalize_section_inner(&t[1..t.len() - 1]));
            out.push_str(line);
            continue;
        }
        if let Some(sec) = &cur_norm {
            if *sec == section_norm
                && t.split_once('=')
                    .map(|(k, _)| k.trim() == name)
                    .unwrap_or(false)
            {
                found = true;
                continue;
            }
        }
        out.push_str(line);
    }
    if !found {
        return Err(CliError::Msg(format!("key not found: {key}")));
    }
    Ok(out)
}

fn print_config_list(text: &str) {
    let mut cur_norm: Option<String> = None;
    for line in text.lines() {
        let t = line.trim();
        if t.starts_with('[') && t.ends_with(']') {
            cur_norm = Some(normalize_section_inner(&t[1..t.len() - 1]));
            continue;
        }
        if let Some((k, v)) = t.split_once('=') {
            if let Some(sec) = &cur_norm {
                println!("{sec}.{}={}", k.trim(), v.trim());
            }
        }
    }
}

/// Split `section.name` or `section.subsection.name` → (normalized section, name).
/// Normalized section is `section` or `section.subsection` (matches git-config list form).
fn split_key(key: &str) -> Option<(String, String)> {
    let (section, name) = key.rsplit_once('.')?;
    Some((section.to_string(), name.to_string()))
}

/// Normalize `[user]` / `[gpg "ssh"]` / `[gpg.ssh]` inners to `user` / `gpg.ssh`.
fn normalize_section_inner(inner: &str) -> String {
    let inner = inner.trim();
    if let Some((sec, rest)) = inner.split_once(' ') {
        let sub = rest.trim().trim_matches('"');
        format!("{sec}.{sub}")
    } else {
        inner.trim_matches('"').to_string()
    }
}

/// Emit Git-compatible section header (`[gpg "ssh"]` for subsections).
fn section_header(section_norm: &str) -> String {
    if let Some((sec, sub)) = section_norm.split_once('.') {
        format!("[{sec} \"{sub}\"]")
    } else {
        format!("[{section_norm}]")
    }
}

fn global_config_path() -> CliResult<std::path::PathBuf> {
    // Prefer XDG then HOME/.gitconfig — cite git-config home file.
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        return Ok(std::path::PathBuf::from(xdg).join("git/config"));
    }
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map_err(|_| CliError::Msg("config --global: HOME/USERPROFILE unset".into()))?;
    Ok(std::path::PathBuf::from(home).join(".gitconfig"))
}

fn system_config_path() -> CliResult<std::path::PathBuf> {
    if let Ok(p) = std::env::var("GIT_CONFIG_SYSTEM") {
        return Ok(std::path::PathBuf::from(p));
    }
    // Portable fallback for clean-room / Windows host without /etc.
    if cfg!(windows) {
        let home = std::env::var("PROGRAMDATA").unwrap_or_else(|_| "C:\\ProgramData".into());
        Ok(std::path::PathBuf::from(home).join("Git/config"))
    } else {
        Ok(std::path::PathBuf::from("/etc/gitconfig"))
    }
}
