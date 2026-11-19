//! Merged config view in stock precedence order (§FILES, §ENVIRONMENT).

use crate::parse::parse_config_text;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// Where an entry came from (low → high precedence).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConfigScope {
    System,
    Global,
    Local,
    Env,
    Command,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigEntry {
    pub key: String,
    pub value: Option<String>,
    pub scope: ConfigScope,
    /// File the entry was read from (`None` for environment / command line).
    pub origin: Option<PathBuf>,
}

/// Ordered entries; later entries override earlier ones for single-valued reads.
#[derive(Debug, Clone, Default)]
pub struct ConfigSet {
    entries: Vec<ConfigEntry>,
}

const MAX_INCLUDE_DEPTH: usize = 10;

/// Process-wide `git -c <k>=<v>` overrides (highest precedence).
static CLI_OVERRIDES: std::sync::Mutex<Vec<(String, Option<String>)>> =
    std::sync::Mutex::new(Vec::new());

/// Record a `-c name[=value]` global option; applies to every later load.
pub fn push_cli_override(spec: &str) {
    let (k, v) = match spec.split_once('=') {
        Some((k, v)) => (k.to_string(), Some(v.to_string())),
        None => (spec.to_string(), None),
    };
    if let Ok(mut g) = CLI_OVERRIDES.lock() {
        g.push((k, v));
    }
}

impl ConfigSet {
    pub fn new() -> Self {
        Self::default()
    }

    /// Load system, global, optional repository `config`, then env and
    /// `GIT_CONFIG_PARAMETERS`. Unreadable or malformed files are skipped
    /// (callers that must surface parse errors use [`ConfigSet::add_file`]).
    pub fn load(git_dir: Option<&Path>) -> Self {
        let mut set = Self::new();
        if env::var_os("GIT_CONFIG_NOSYSTEM").is_none_or(|v| !is_truthy(&v.to_string_lossy())) {
            if let Some(p) = system_config_path() {
                let _ = set.add_file(&p, ConfigScope::System);
            }
        }
        for p in global_config_paths() {
            let _ = set.add_file(&p, ConfigScope::Global);
        }
        if let Some(dir) = git_dir {
            let _ = set.add_file(&dir.join("config"), ConfigScope::Local);
        }
        set.add_env_count();
        if let Ok(params) = env::var("GIT_CONFIG_PARAMETERS") {
            for (k, v) in parse_config_parameters(&params) {
                set.push(k, v, ConfigScope::Command);
            }
        }
        if let Ok(g) = CLI_OVERRIDES.lock() {
            for (k, v) in g.iter() {
                set.push(k.clone(), v.clone(), ConfigScope::Command);
            }
        }
        set
    }

    /// Load with the repository found by walking up from `start` (or `GIT_DIR`).
    pub fn load_discovered(start: &Path) -> Self {
        Self::load(discover_git_dir(start).as_deref())
    }

    pub fn push(&mut self, key: String, value: Option<String>, scope: ConfigScope) {
        self.entries.push(ConfigEntry {
            key: normalize_key(&key),
            value,
            scope,
            origin: None,
        });
    }

    /// Parse `path` and append its entries (following `include.path`).
    pub fn add_file(&mut self, path: &Path, scope: ConfigScope) -> Result<(), String> {
        self.add_file_depth(path, scope, 0)
    }

    fn add_file_depth(
        &mut self,
        path: &Path,
        scope: ConfigScope,
        depth: usize,
    ) -> Result<(), String> {
        let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let entries = parse_config_text(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        for e in entries {
            if e.key == "include.path" && depth < MAX_INCLUDE_DEPTH {
                if let Some(v) = e.value.as_deref() {
                    let inc = expand_path(v);
                    let inc = if inc.is_relative() {
                        path.parent().unwrap_or(Path::new(".")).join(inc)
                    } else {
                        inc
                    };
                    let _ = self.add_file_depth(&inc, scope, depth + 1);
                }
                continue;
            }
            self.entries.push(ConfigEntry {
                key: e.key,
                value: e.value,
                scope,
                origin: Some(path.to_path_buf()),
            });
        }
        Ok(())
    }

    fn add_env_count(&mut self) {
        let Some(count) = env::var("GIT_CONFIG_COUNT")
            .ok()
            .and_then(|c| c.trim().parse::<usize>().ok())
        else {
            return;
        };
        for i in 0..count {
            let Ok(k) = env::var(format!("GIT_CONFIG_KEY_{i}")) else {
                continue;
            };
            let v = env::var(format!("GIT_CONFIG_VALUE_{i}")).ok();
            self.push(k, v, ConfigScope::Env);
        }
    }

    pub fn entries(&self) -> &[ConfigEntry] {
        &self.entries
    }

    /// Last value for `key` (bare `name` reads as `"true"`).
    pub fn get(&self, key: &str) -> Option<String> {
        let key = normalize_key(key);
        self.entries
            .iter()
            .rev()
            .find(|e| e.key == key)
            .map(|e| e.value.clone().unwrap_or_else(|| "true".into()))
    }

    /// All values for a multi-valued `key`, in precedence order.
    pub fn get_all(&self, key: &str) -> Vec<String> {
        let key = normalize_key(key);
        self.entries
            .iter()
            .filter(|e| e.key == key)
            .map(|e| e.value.clone().unwrap_or_else(|| "true".into()))
            .collect()
    }

    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.get(key).and_then(|v| parse_bool(&v))
    }

    /// `~` / `~/` expanded path value.
    pub fn get_path(&self, key: &str) -> Option<PathBuf> {
        self.get(key)
            .filter(|v| !v.is_empty())
            .map(|v| expand_path(&v))
    }

    /// All `(subsection_key_suffix, value)` pairs whose key starts with `prefix.`
    /// and ends with `.suffix` — e.g. `url.*.insteadof`.
    pub fn subsections(&self, section: &str, name: &str) -> Vec<(String, String)> {
        let section = section.to_ascii_lowercase();
        let name = name.to_ascii_lowercase();
        let mut out = Vec::new();
        for e in &self.entries {
            let Some(rest) = e.key.strip_prefix(&format!("{section}.")) else {
                continue;
            };
            let Some(sub) = rest.strip_suffix(&format!(".{name}")) else {
                continue;
            };
            out.push((
                sub.to_string(),
                e.value.clone().unwrap_or_else(|| "true".into()),
            ));
        }
        out
    }
}

/// git-config boolean (§Values: boolean).
pub fn parse_bool(v: &str) -> Option<bool> {
    match v.trim().to_ascii_lowercase().as_str() {
        "true" | "yes" | "on" | "1" => Some(true),
        "false" | "no" | "off" | "0" | "" => Some(false),
        _ => None,
    }
}

fn is_truthy(v: &str) -> bool {
    parse_bool(v).unwrap_or(true)
}

/// Normalize `Section.Sub.Name` → `section.Sub.name` (subsection case kept).
fn normalize_key(key: &str) -> String {
    let Some(first) = key.find('.') else {
        return key.to_ascii_lowercase();
    };
    let last = key.rfind('.').unwrap_or(first);
    if first == last {
        return key.to_ascii_lowercase();
    }
    format!(
        "{}.{}.{}",
        key[..first].to_ascii_lowercase(),
        &key[first + 1..last],
        key[last + 1..].to_ascii_lowercase()
    )
}

/// Parse `GIT_CONFIG_PARAMETERS` (`'k'='v'` or `'k=v'` items, shell single-quoted).
pub fn parse_config_parameters(s: &str) -> Vec<(String, Option<String>)> {
    let mut out = Vec::new();
    let mut words: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut in_q = false;
    let mut have = false;
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\'' => {
                in_q = !in_q;
                have = true;
            }
            '\\' if !in_q => {
                if let Some(n) = chars.next() {
                    cur.push(n);
                    have = true;
                }
            }
            c if c.is_whitespace() && !in_q => {
                if have {
                    words.push(std::mem::take(&mut cur));
                    have = false;
                }
            }
            c => {
                cur.push(c);
                have = true;
            }
        }
    }
    if have {
        words.push(cur);
    }
    // Stock git emits `'key'='value'`; unquoting above joins those into `key=value`.
    for w in words {
        match w.split_once('=') {
            Some((k, v)) => out.push((k.to_string(), Some(v.to_string()))),
            None => out.push((w, None)),
        }
    }
    out
}

fn home_dir() -> Option<PathBuf> {
    env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .filter(|h| !h.is_empty())
        .map(PathBuf::from)
}

fn expand_path(v: &str) -> PathBuf {
    if v == "~" {
        return home_dir().unwrap_or_else(|| PathBuf::from(v));
    }
    if let Some(rest) = v.strip_prefix("~/") {
        if let Some(h) = home_dir() {
            return h.join(rest);
        }
    }
    PathBuf::from(v)
}

fn system_config_path() -> Option<PathBuf> {
    if let Some(p) = env::var_os("GIT_CONFIG_SYSTEM") {
        return Some(PathBuf::from(p));
    }
    if cfg!(unix) {
        Some(PathBuf::from("/etc/gitconfig"))
    } else {
        env::var_os("PROGRAMDATA").map(|p| PathBuf::from(p).join("Git").join("config"))
    }
}

/// Global files in read order: XDG then `~/.gitconfig`; `GIT_CONFIG_GLOBAL` replaces both.
fn global_config_paths() -> Vec<PathBuf> {
    if let Some(p) = env::var_os("GIT_CONFIG_GLOBAL") {
        return vec![PathBuf::from(p)];
    }
    let mut out = Vec::new();
    let xdg = env::var_os("XDG_CONFIG_HOME")
        .filter(|x| !x.is_empty())
        .map(PathBuf::from)
        .or_else(|| home_dir().map(|h| h.join(".config")));
    if let Some(x) = xdg {
        out.push(x.join("git").join("config"));
    }
    if let Some(h) = home_dir() {
        out.push(h.join(".gitconfig"));
    }
    out
}

/// `GIT_DIR` or the nearest `.git` (dir or gitfile) walking up from `start`.
pub fn discover_git_dir(start: &Path) -> Option<PathBuf> {
    if let Some(d) = env::var_os("GIT_DIR") {
        return Some(PathBuf::from(d));
    }
    let mut cur = Some(start);
    while let Some(dir) = cur {
        let dotgit = dir.join(".git");
        if dotgit.is_dir() {
            return Some(dotgit);
        }
        if dotgit.is_file() {
            if let Ok(text) = fs::read_to_string(&dotgit) {
                if let Some(p) = text.trim().strip_prefix("gitdir:") {
                    let p = PathBuf::from(p.trim());
                    return Some(if p.is_relative() { dir.join(p) } else { p });
                }
            }
        }
        cur = dir.parent();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn precedence_last_wins_and_case() {
        let mut s = ConfigSet::new();
        s.push("User.Name".into(), Some("a".into()), ConfigScope::Global);
        s.push("user.name".into(), Some("b".into()), ConfigScope::Local);
        s.push(
            "url.HTTPS://x/.insteadOf".into(),
            Some("git@x:".into()),
            ConfigScope::Env,
        );
        assert_eq!(s.get("user.NAME").as_deref(), Some("b"));
        assert_eq!(
            s.subsections("url", "insteadof"),
            vec![("HTTPS://x/".to_string(), "git@x:".to_string())]
        );
    }

    #[test]
    fn file_include_and_bool() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("inc"), "[commit]\n\tgpgsign\n").unwrap();
        fs::write(
            dir.path().join("main"),
            "[include]\n\tpath = inc\n[user]\n\tname = N\n",
        )
        .unwrap();
        let mut s = ConfigSet::new();
        s.add_file(&dir.path().join("main"), ConfigScope::Global)
            .unwrap();
        assert_eq!(s.get_bool("commit.gpgsign"), Some(true));
        assert_eq!(s.get("user.name").as_deref(), Some("N"));
    }

    #[test]
    fn config_parameters_forms() {
        let got = parse_config_parameters("'user.name'='A B' 'core.x=1' 'flag'");
        assert_eq!(
            got,
            vec![
                ("user.name".into(), Some("A B".into())),
                ("core.x".into(), Some("1".into())),
                ("flag".into(), None),
            ]
        );
    }
}
