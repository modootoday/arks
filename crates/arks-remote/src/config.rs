//! `[remote "<name>"]` sections in a Git config file.
//!
//! Claimed subset: `url` and `fetch` keys written by `remote add`.
//! Primary: <https://git-scm.com/docs/git-config>

use crate::error::RemoteError;
use crate::url::{GitDaemonUrl, HttpRemoteUrl, LocalRemoteUrl, SshRemoteUrl};
use std::fs;
use std::path::Path;

/// One `[remote "<name>"]` section the client will use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteConfig {
    pub name: String,
    pub url: String,
    pub fetch: Vec<String>,
}

/// Fetch refspec `remote add` writes for `name`.
pub fn default_fetch_spec(name: &str) -> String {
    format!("+refs/heads/*:refs/remotes/{name}/*")
}

/// Read remote sections from a config file path (`.git/config`).
///
/// A missing file is an empty remote list.
pub fn list_remotes(config_file: &Path) -> Result<Vec<RemoteConfig>, RemoteError> {
    let text = match fs::read_to_string(config_file) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(RemoteError::Config(e.to_string())),
    };
    Ok(parse_remotes(&text))
}

/// Append a remote section. Accepts http(s), SSH, `git://`, and local/file:// URLs.
pub fn add_remote(config_file: &Path, name: &str, url: &str) -> Result<(), RemoteError> {
    validate_name(name)?;
    let url = url.trim();
    if HttpRemoteUrl::parse(url).is_err()
        && SshRemoteUrl::parse(url).is_err()
        && GitDaemonUrl::parse(url).is_err()
        && LocalRemoteUrl::parse(url).is_err()
    {
        return Err(RemoteError::UnsupportedUrl(url.to_string()));
    }
    let existing = list_remotes(config_file)?;
    if existing.iter().any(|r| r.name == name) {
        return Err(RemoteError::Config(format!(
            "remote '{name}' already exists"
        )));
    }
    // Preserve non-remote lines by appending a new section.
    let mut text = match fs::read_to_string(config_file) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(RemoteError::Config(e.to_string())),
    };
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    text.push_str(&format!(
        "[remote \"{name}\"]\n\turl = {url}\n\tfetch = {}\n",
        default_fetch_spec(name)
    ));
    fs::write(config_file, text).map_err(|e| RemoteError::Config(e.to_string()))?;
    Ok(())
}

/// Set `core.repositoryformatversion=1` + `extensions.partialClone=<remote>`.
///
/// Cite: [partial-clone](https://git-scm.com/docs/partial-clone),
/// [git-config](https://git-scm.com/docs/git-config) `extensions.partialClone`.
pub fn enable_partial_clone(git_dir: &Path, remote_name: &str) -> Result<(), RemoteError> {
    let config_file = git_dir.join("config");
    let mut text = match fs::read_to_string(&config_file) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(RemoteError::Config(e.to_string())),
    };
    text = upsert_config_key(&text, "core", "repositoryformatversion", "1");
    text = upsert_config_key(&text, "extensions", "partialClone", remote_name);
    if !text.ends_with('\n') {
        text.push('\n');
    }
    fs::write(&config_file, text).map_err(|e| RemoteError::Config(e.to_string()))?;
    Ok(())
}

fn upsert_config_key(text: &str, section: &str, key: &str, value: &str) -> String {
    let section_l = section.to_ascii_lowercase();
    let mut out = String::with_capacity(text.len() + 64);
    let mut in_section = false;
    let mut section_seen = false;
    let mut key_written = false;
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            if in_section && !key_written {
                out.push_str(&format!("\t{key} = {value}\n"));
                key_written = true;
            }
            let name = trimmed[1..trimmed.len() - 1].trim().to_ascii_lowercase();
            in_section = name == section_l;
            if in_section {
                section_seen = true;
            }
            out.push_str(line);
            continue;
        }
        if in_section {
            if let Some((k, _)) = trimmed.split_once('=') {
                if k.trim().eq_ignore_ascii_case(key) {
                    out.push_str(&format!("\t{key} = {value}\n"));
                    key_written = true;
                    continue;
                }
            }
        }
        out.push_str(line);
    }
    if in_section && !key_written {
        out.push_str(&format!("\t{key} = {value}\n"));
        key_written = true;
    }
    if !section_seen {
        if !out.is_empty() && !out.ends_with('\n') {
            out.push('\n');
        }
        out.push_str(&format!("[{section}]\n\t{key} = {value}\n"));
        let _ = key_written;
    } else if !key_written {
        out.push_str(&format!("[{section}]\n\t{key} = {value}\n"));
    }
    out
}

/// Delete the `[remote "<name>"]` section. Other sections stay byte-stable aside from that cut.
pub fn remove_remote(config_file: &Path, name: &str) -> Result<(), RemoteError> {
    validate_name(name)?;
    let text = fs::read_to_string(config_file).map_err(|e| RemoteError::Config(e.to_string()))?;
    let mut out = String::new();
    let mut skipping = false;
    let mut found = false;
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            if remote_header_name(trimmed).as_deref() == Some(name) {
                skipping = true;
                found = true;
                continue;
            }
            skipping = false;
        }
        if !skipping {
            out.push_str(line);
        }
    }
    if !found {
        return Err(RemoteError::Config(format!("no such remote '{name}'")));
    }
    fs::write(config_file, out).map_err(|e| RemoteError::Config(e.to_string()))
}

pub(crate) fn fetch_specs_for(config_file: &Path, remote_name: &str) -> Vec<String> {
    list_remotes(config_file)
        .ok()
        .and_then(|list| list.into_iter().find(|r| r.name == remote_name))
        .map(|r| {
            if r.fetch.is_empty() {
                vec![default_fetch_spec(remote_name)]
            } else {
                r.fetch
            }
        })
        .unwrap_or_else(|| vec![default_fetch_spec(remote_name)])
}

/// Map `refs/heads/main` through `+refs/heads/*:refs/remotes/origin/*`.
pub(crate) fn map_refspec(spec: &str, src: &str) -> Option<String> {
    let spec = spec.strip_prefix('+').unwrap_or(spec);
    let (from, to) = spec.split_once(':')?;
    let from_prefix = from.strip_suffix('*')?;
    let to_prefix = to.strip_suffix('*')?;
    let rest = src.strip_prefix(from_prefix)?;
    if rest.contains('*') {
        return None;
    }
    Some(format!("{to_prefix}{rest}"))
}

fn parse_remotes(text: &str) -> Vec<RemoteConfig> {
    let mut out = Vec::new();
    let mut current: Option<RemoteConfig> = None;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
            continue;
        }
        if trimmed.starts_with('[') {
            if let Some(cfg) = current.take() {
                if !cfg.url.is_empty() {
                    out.push(cfg);
                }
            }
            current = remote_header_name(trimmed).map(|name| RemoteConfig {
                name,
                url: String::new(),
                fetch: Vec::new(),
            });
            continue;
        }
        let Some(cfg) = current.as_mut() else {
            continue;
        };
        let Some((key, value)) = split_key(trimmed) else {
            continue;
        };
        if key == "url" && cfg.url.is_empty() {
            cfg.url = value;
        } else if key == "fetch" {
            cfg.fetch.push(value);
        }
    }
    if let Some(cfg) = current {
        if !cfg.url.is_empty() {
            out.push(cfg);
        }
    }
    out
}

fn remote_header_name(header: &str) -> Option<String> {
    let header = header.trim().trim_end_matches(']').trim();
    let rest = header.strip_prefix("[remote")?.trim();
    let name = rest.strip_prefix('"')?.strip_suffix('"')?;
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

fn split_key(line: &str) -> Option<(String, String)> {
    let (key, value) = line.split_once('=')?;
    let key = key.trim();
    if key.is_empty() {
        return None;
    }
    Some((key.to_string(), value.trim().to_string()))
}

fn validate_name(name: &str) -> Result<(), RemoteError> {
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-')
    {
        return Err(RemoteError::Config(format!("invalid remote name '{name}'")));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn add_list_remove_preserves_core() {
        let dir = TempDir::new().unwrap();
        let cfg = dir.path().join("config");
        fs::write(
            &cfg,
            "[core]\n\trepositoryformatversion = 0\n\tbare = false\n",
        )
        .unwrap();
        add_remote(&cfg, "origin", "http://example.com/r.git").unwrap();
        let list = list_remotes(&cfg).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "origin");
        assert_eq!(list[0].url, "http://example.com/r.git");
        assert_eq!(
            list[0].fetch,
            vec!["+refs/heads/*:refs/remotes/origin/*".to_string()]
        );
        let text = fs::read_to_string(&cfg).unwrap();
        assert!(text.contains("[core]"));
        assert!(text.contains("repositoryformatversion"));
        remove_remote(&cfg, "origin").unwrap();
        assert!(list_remotes(&cfg).unwrap().is_empty());
        let text = fs::read_to_string(&cfg).unwrap();
        assert!(text.contains("[core]"));
        assert!(!text.contains("example.com"));
    }

    #[test]
    fn ssh_url_accepted() {
        let dir = TempDir::new().unwrap();
        let cfg = dir.path().join("config");
        fs::write(&cfg, "[core]\n").unwrap();
        add_remote(&cfg, "origin", "git@example.com:a/b.git").unwrap();
        let remotes = list_remotes(&cfg).unwrap();
        assert_eq!(remotes[0].url, "git@example.com:a/b.git");
        add_remote(&cfg, "daemon", "git://example.com/a.git").unwrap();
        let remotes = list_remotes(&cfg).unwrap();
        assert!(remotes.iter().any(|r| r.url == "git://example.com/a.git"));
        let err = add_remote(&cfg, "bad", "ftp://example.com/a.git").unwrap_err();
        assert!(matches!(err, RemoteError::UnsupportedUrl(_)));
    }

    #[test]
    fn refspec_maps_heads() {
        let spec = "+refs/heads/*:refs/remotes/origin/*";
        assert_eq!(
            map_refspec(spec, "refs/heads/main").as_deref(),
            Some("refs/remotes/origin/main")
        );
        assert!(map_refspec(spec, "refs/tags/v1").is_none());
    }
}
