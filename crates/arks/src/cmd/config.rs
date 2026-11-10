//! Local repository config get/set/list/unset.

use crate::error::{CliError, CliResult};
use crate::util::{open_repo, refuse_unknown_flags};
use std::fs;
pub fn cmd_config(args: &[String]) -> CliResult<()> {
    // M16 Sy-1: `--local` / `--global` / `--system` / `--worktree` claimed (git-config).
    // Cite: https://git-scm.com/docs/git-config
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
