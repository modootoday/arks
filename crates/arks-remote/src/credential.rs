//! Credential fill / helper / askpass / TTY / first-party OS wallets.
//!
//! Primary:
//! - <https://git-scm.com/docs/gitcredentials>
//! - <https://git-scm.com/docs/git-credential>
//!
//! Cite: `.spec/sot/credential-helper-askpass.sot.md`
//! Forge OAuth remains out of scope (AUTH residual).

use crate::error::RemoteError;
use crate::url::{PreparedUrl, basic_header, split_url};
use crate::wallet::{is_os_wallet_helper, run_wallet_helper, wallet_helper_kind};
use std::cell::RefCell;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Credential description (git-credential INPUT/OUTPUT attributes).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Credential {
    pub protocol: Option<String>,
    pub host: Option<String>,
    pub path: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
}

impl Credential {
    /// Parse blank-line-terminated attribute lines (`key=value`).
    pub fn parse(input: &str) -> Result<Self, RemoteError> {
        Ok(Self::parse_attributes(input))
    }

    /// Parse blank-line-terminated attribute lines (`key=value`).
    pub fn parse_attributes(input: &str) -> Self {
        let mut c = Credential::default();
        for line in input.lines() {
            if line.is_empty() {
                break;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            match key {
                "protocol" => c.protocol = Some(value.to_string()),
                "host" => c.host = Some(value.to_string()),
                "path" => c.path = Some(value.to_string()),
                "username" => c.username = Some(value.to_string()),
                "password" => c.password = Some(value.to_string()),
                "url" => {
                    let _ = apply_url(&mut c, value);
                }
                _ => {}
            }
        }
        c
    }

    /// Emit attributes (no trailing blank line).
    pub fn format_attrs(&self) -> String {
        let mut out = String::new();
        if let Some(v) = &self.protocol {
            out.push_str(&format!("protocol={v}\n"));
        }
        if let Some(v) = &self.host {
            out.push_str(&format!("host={v}\n"));
        }
        if let Some(v) = &self.path {
            out.push_str(&format!("path={v}\n"));
        }
        if let Some(v) = &self.username {
            out.push_str(&format!("username={v}\n"));
        }
        if let Some(v) = &self.password {
            out.push_str(&format!("password={v}\n"));
        }
        out
    }

    /// Emit attributes including terminating blank line (git-credential wire).
    pub fn format_attributes(&self) -> String {
        let mut out = self.format_attrs();
        out.push('\n');
        out
    }

    pub fn has_userpass(&self) -> bool {
        self.username.as_ref().is_some_and(|u| !u.is_empty())
            && self.password.as_ref().is_some_and(|p| !p.is_empty())
    }

    pub fn basic_authorization(&self) -> Option<String> {
        let user = self.username.as_deref()?;
        let pass = self.password.as_deref().unwrap_or("");
        if user.is_empty() {
            return None;
        }
        Some(basic_header(&format!("{user}:{pass}")))
    }
}

fn apply_url(c: &mut Credential, url: &str) -> Result<(), RemoteError> {
    let raw = url.trim();
    let (scheme, rest) = if let Some(r) = raw.strip_prefix("https://") {
        ("https", r)
    } else if let Some(r) = raw.strip_prefix("http://") {
        ("http", r)
    } else {
        return Err(RemoteError::Msg(format!(
            "credential: url= requires http(s): {url}"
        )));
    };
    c.protocol = Some(scheme.to_string());
    let (authority, path) = match rest.find('/') {
        Some(i) => (&rest[..i], Some(&rest[i + 1..])),
        None => (rest, None),
    };
    let (userinfo, hostport) = match authority.rsplit_once('@') {
        Some((user, host)) if !host.is_empty() => (Some(user), host),
        _ => (None, authority),
    };
    if !hostport.is_empty() {
        c.host = Some(hostport.to_string());
    }
    if let Some(p) = path {
        if !p.is_empty() {
            c.path = Some(p.to_string());
        }
    }
    if let Some(ui) = userinfo {
        match ui.split_once(':') {
            Some((u, p)) => {
                if c.username.is_none() {
                    c.username = Some(u.to_string());
                }
                if c.password.is_none() {
                    c.password = Some(p.to_string());
                }
            }
            None => {
                if c.username.is_none() {
                    c.username = Some(ui.to_string());
                }
            }
        }
    }
    Ok(())
}

/// Build a credential description from an HTTP(S) remote URL.
pub fn credential_from_url(raw: &str) -> Result<Credential, RemoteError> {
    let mut c = Credential::default();
    apply_url(&mut c, raw)?;
    Ok(c)
}

/// Sources consulted during fill (helpers + askpass + static username).
#[derive(Debug, Default)]
pub struct CredentialSource {
    pub helpers: Vec<String>,
    pub askpass: Option<PathBuf>,
    pub username: Option<String>,
    /// Process-local filled credential (avoids re-prompting TTY/askpass per HTTP exchange).
    filled_cache: RefCell<Option<Credential>>,
}

impl Clone for CredentialSource {
    fn clone(&self) -> Self {
        Self {
            helpers: self.helpers.clone(),
            askpass: self.askpass.clone(),
            username: self.username.clone(),
            filled_cache: RefCell::new(self.filled_cache.borrow().clone()),
        }
    }
}

impl CredentialSource {
    /// Empty source (no helpers / askpass). Used until a 401 forces discover.
    pub fn empty() -> Self {
        Self {
            helpers: Vec::new(),
            askpass: None,
            username: None,
            filled_cache: RefCell::new(None),
        }
    }

    /// Discover helpers / askpass / username from env + optional config files.
    pub fn discover(extra_config_paths: &[PathBuf]) -> Self {
        let mut helpers = Vec::new();
        let mut username = None;
        let mut askpass_cfg: Option<PathBuf> = None;

        let mut paths: Vec<PathBuf> = Vec::new();
        if let Ok(home) = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
            paths.push(PathBuf::from(&home).join(".gitconfig"));
            paths.push(PathBuf::from(&home).join(".config/git/config"));
        }
        if let Ok(g) = std::env::var("GIT_CONFIG_GLOBAL") {
            paths.push(PathBuf::from(g));
        }
        if let Ok(extra) = std::env::var("ARKS_CREDENTIAL_CONFIG") {
            paths.push(PathBuf::from(extra));
        }
        paths.extend(extra_config_paths.iter().cloned());
        if let Ok(cwd) = std::env::current_dir() {
            if let Some(git_cfg) = find_git_config(&cwd) {
                paths.push(git_cfg);
            }
        }

        for p in &paths {
            if let Ok(text) = std::fs::read_to_string(p) {
                merge_credential_config(&text, &mut helpers, &mut username, &mut askpass_cfg);
            }
        }

        if let Ok(h) = std::env::var("ARKS_CREDENTIAL_HELPER") {
            if !h.is_empty() {
                helpers.push(h);
            }
        }

        let askpass = std::env::var_os("GIT_ASKPASS")
            .or_else(|| std::env::var_os("SSH_ASKPASS"))
            .map(PathBuf::from)
            .or(askpass_cfg);

        Self {
            helpers,
            askpass,
            username,
            filled_cache: RefCell::new(None),
        }
    }
}

fn find_git_config(start: &Path) -> Option<PathBuf> {
    let mut cur = start.to_path_buf();
    loop {
        let cfg = cur.join(".git").join("config");
        if cfg.is_file() {
            return Some(cfg);
        }
        if !cur.pop() {
            return None;
        }
    }
}

fn merge_credential_config(
    text: &str,
    helpers: &mut Vec<String>,
    username: &mut Option<String>,
    askpass: &mut Option<PathBuf>,
) {
    let mut section = String::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            section = line[1..line.len() - 1].trim().to_ascii_lowercase();
            if let Some((head, _)) = section.split_once('"') {
                section = head.trim().to_string();
            }
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim().to_ascii_lowercase();
        let value = value.trim().trim_matches('"').to_string();
        if section == "credential" {
            match key.as_str() {
                "helper" if value.is_empty() => helpers.clear(),
                "helper" => helpers.push(value),
                "username" if username.is_none() => *username = Some(value),
                _ => {}
            }
        } else if section == "core" && key == "askpass" && askpass.is_none() {
            *askpass = Some(PathBuf::from(value));
        }
    }
}

/// Enrich description via env / helpers / askpass / TTY (gitcredentials order).
pub fn fill(mut desc: Credential, source: &CredentialSource) -> Result<Credential, RemoteError> {
    if let Some(cached) = source.filled_cache.borrow().clone() {
        if cached.has_userpass() {
            merge_helper_output(&mut desc, cached);
            if desc.has_userpass() {
                return Ok(desc);
            }
        }
    }

    if let Some(u) = &source.username {
        if desc.username.is_none() {
            desc.username = Some(u.clone());
        }
    }

    if desc.username.is_none() {
        if let Ok(u) = std::env::var("ARKS_REMOTE_USERNAME") {
            if !u.is_empty() {
                desc.username = Some(u);
            }
        }
    }
    if desc.password.is_none() {
        if let Ok(p) = std::env::var("ARKS_REMOTE_PASSWORD") {
            desc.password = Some(p);
        }
    }

    if !desc.has_userpass() {
        for helper in &source.helpers {
            if desc.has_userpass() {
                break;
            }
            if let Some(got) = run_helper(helper, "get", &desc)? {
                merge_helper_output(&mut desc, got);
            }
        }
    }

    if !desc.has_userpass() {
        if let Some(ask) = &source.askpass {
            askpass_fill(&mut desc, ask)?;
        }
    }

    if !desc.has_userpass() {
        tty_fill(&mut desc)?;
    }

    if desc.has_userpass() {
        *source.filled_cache.borrow_mut() = Some(desc.clone());
    }

    Ok(desc)
}

/// Forward `store` to helpers (`approve`).
pub fn approve(desc: &Credential, source: &CredentialSource) -> Result<(), RemoteError> {
    for helper in &source.helpers {
        let _ = run_helper(helper, "store", desc)?;
    }
    Ok(())
}

/// Forward `erase` to helpers (`reject`).
pub fn reject(desc: &Credential, source: &CredentialSource) -> Result<(), RemoteError> {
    for helper in &source.helpers {
        let _ = run_helper(helper, "erase", desc)?;
    }
    Ok(())
}

fn merge_helper_output(desc: &mut Credential, got: Credential) {
    if let Some(u) = got.username {
        desc.username = Some(u.trim().to_string());
    }
    if let Some(p) = got.password {
        let p = p.trim();
        let p = p
            .strip_suffix(" get")
            .or_else(|| p.strip_suffix(" store"))
            .or_else(|| p.strip_suffix(" erase"))
            .unwrap_or(p)
            .trim();
        desc.password = Some(p.to_string());
    }
}

fn askpass_fill(desc: &mut Credential, program: &Path) -> Result<(), RemoteError> {
    if desc.username.is_none() {
        let u = run_askpass(program, "Username: ")?;
        if !u.is_empty() {
            desc.username = Some(u);
        }
    }
    if desc.password.is_none() {
        let user = desc.username.as_deref().unwrap_or("");
        let prompt = if user.is_empty() {
            "Password: ".to_string()
        } else {
            format!("Password for '{user}': ")
        };
        let p = run_askpass(program, &prompt)?;
        desc.password = Some(p);
    }
    Ok(())
}

fn run_askpass(program: &Path, prompt: &str) -> Result<String, RemoteError> {
    let output = Command::new(program)
        .arg(prompt)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| RemoteError::Msg(format!("askpass spawn {}: {e}", program.display())))?;
    if !output.status.success() {
        return Err(RemoteError::Msg(format!(
            "askpass failed ({}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    let line = String::from_utf8_lossy(&output.stdout);
    Ok(line.lines().next().unwrap_or("").trim().to_string())
}

fn terminal_prompt_allowed() -> bool {
    if matches!(std::env::var("GIT_TERMINAL_PROMPT").as_deref(), Ok("0")) {
        return false;
    }
    // Windows CI often exposes CONIN$ with no human at the keyboard; opening it
    // and read_line blocks the HTTP exchange forever (Linux/macOS skip missing tty).
    if std::env::var_os("CI").is_some() || std::env::var_os("GITHUB_ACTIONS").is_some() {
        return false;
    }
    true
}

fn open_tty_reader() -> Option<Box<dyn BufRead>> {
    #[cfg(unix)]
    {
        std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/tty")
            .ok()
            .map(|f| Box::new(std::io::BufReader::new(f)) as Box<dyn BufRead>)
    }
    #[cfg(windows)]
    {
        std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open("CONIN$")
            .ok()
            .map(|f| Box::new(std::io::BufReader::new(f)) as Box<dyn BufRead>)
    }
    #[cfg(not(any(unix, windows)))]
    {
        None
    }
}

fn tty_write_prompt(prompt: &str) -> Result<(), RemoteError> {
    #[cfg(unix)]
    {
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .open("/dev/tty")
            .map_err(|e| RemoteError::Msg(format!("open /dev/tty for prompt: {e}")))?;
        f.write_all(prompt.as_bytes())
            .map_err(|e| RemoteError::Msg(format!("tty prompt write: {e}")))?;
        f.flush()
            .map_err(|e| RemoteError::Msg(format!("tty prompt flush: {e}")))?;
        Ok(())
    }
    #[cfg(windows)]
    {
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .open("CONOUT$")
            .or_else(|_| std::fs::OpenOptions::new().write(true).open("CONIN$"))
            .map_err(|e| RemoteError::Msg(format!("open console for prompt: {e}")))?;
        f.write_all(prompt.as_bytes())
            .map_err(|e| RemoteError::Msg(format!("tty prompt write: {e}")))?;
        f.flush()
            .map_err(|e| RemoteError::Msg(format!("tty prompt flush: {e}")))?;
        Ok(())
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = prompt;
        Err(RemoteError::Msg("no TTY support on this platform".into()))
    }
}

fn tty_fill(desc: &mut Credential) -> Result<(), RemoteError> {
    if !terminal_prompt_allowed() {
        return Ok(());
    }
    // No controlling TTY → skip silently (fail closed later if still incomplete).
    let Some(mut reader) = open_tty_reader() else {
        return Ok(());
    };
    if desc.username.is_none() {
        if tty_write_prompt("Username: ").is_err() {
            return Ok(());
        }
        let mut line = String::new();
        if reader.read_line(&mut line).is_err() {
            return Ok(());
        }
        let u = line.trim_end_matches(['\r', '\n']).to_string();
        if !u.is_empty() {
            desc.username = Some(u);
        }
    }
    if desc.password.is_none() {
        let user = desc.username.as_deref().unwrap_or("");
        let prompt = if user.is_empty() {
            "Password: ".to_string()
        } else {
            format!("Password for '{user}': ")
        };
        if tty_write_prompt(&prompt).is_err() {
            return Ok(());
        }
        let mut line = String::new();
        if reader.read_line(&mut line).is_err() {
            return Ok(());
        }
        desc.password = Some(line.trim_end_matches(['\r', '\n']).to_string());
    }
    Ok(())
}

fn run_helper(
    helper: &str,
    operation: &str,
    desc: &Credential,
) -> Result<Option<Credential>, RemoteError> {
    if let Some(kind) = wallet_helper_kind(helper) {
        if is_os_wallet_helper(helper) {
            return run_wallet_helper(kind, operation, desc);
        }
    }
    let cmdline = helper_command(helper);
    let mut cmd = if cfg!(windows) {
        let mut c = Command::new("cmd");
        c.arg("/C").arg(format!("{cmdline} {operation}"));
        c
    } else {
        let mut c = Command::new("sh");
        c.arg("-c").arg(format!("{cmdline} {operation}"));
        c
    };
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| RemoteError::Msg(format!("credential helper spawn: {e}")))?;
    {
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| RemoteError::Msg("credential helper stdin".into()))?;
        // Helper may exit before consuming stdin (macOS/race); BrokenPipe is not fatal.
        match stdin.write_all(desc.format_attributes().as_bytes()) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => {}
            Err(e) => {
                return Err(RemoteError::Msg(format!(
                    "credential helper stdin write: {e}"
                )));
            }
        }
    }
    let output = child
        .wait_with_output()
        .map_err(|e| RemoteError::Msg(format!("credential helper wait: {e}")))?;
    if operation == "get" {
        if !output.status.success() {
            return Ok(None);
        }
        let text = String::from_utf8_lossy(&output.stdout);
        if text.trim().is_empty() {
            return Ok(None);
        }
        return Ok(Some(Credential::parse_attributes(&text)));
    }
    Ok(None)
}

fn helper_command(helper: &str) -> String {
    let h = helper.trim();
    if let Some(rest) = h.strip_prefix('!') {
        return rest.to_string();
    }
    let is_abs = h.starts_with('/')
        || (h.len() >= 3
            && h.as_bytes()[1] == b':'
            && (h.as_bytes()[2] == b'\\' || h.as_bytes()[2] == b'/'));
    if is_abs {
        return h.to_string();
    }
    format!("git credential-{h}")
}

/// Resolve Authorization for an HTTP(S) URL using Claimed fill order.
pub fn resolve_authorization(
    raw_url: &str,
    source: &CredentialSource,
) -> Result<Option<String>, RemoteError> {
    let (_stripped, from_url) = split_url(raw_url)?;
    if let Some(userinfo) = from_url {
        return Ok(Some(basic_header(&userinfo)));
    }
    let mut desc = credential_from_url(raw_url)?;
    desc = fill(desc, source)?;
    Ok(desc.basic_authorization())
}

/// Prepare cleaned URL + Authorization for an explicit credential source.
pub fn prepare_authorized_url_with(
    raw: &str,
    source: &CredentialSource,
) -> Result<PreparedUrl, RemoteError> {
    let (url, _) = split_url(raw)?;
    let authorization = resolve_authorization(raw, source)?;
    Ok(PreparedUrl { url, authorization })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn parse_and_format_roundtrip() {
        let raw = "protocol=https\nhost=example.com\nusername=bob\npassword=secr3t\n\n";
        let c = Credential::parse(raw).unwrap();
        assert_eq!(c.username.as_deref(), Some("bob"));
        assert!(c.format_attrs().contains("password=secr3t"));
        assert!(c.has_userpass());
        assert_eq!(
            c.basic_authorization().as_deref(),
            Some("Basic Ym9iOnNlY3IzdA==")
        );
    }

    #[test]
    fn url_attribute_expands() {
        let c = Credential::parse("url=https://alice@example.com/foo.git\n\n").unwrap();
        assert_eq!(c.protocol.as_deref(), Some("https"));
        assert_eq!(c.host.as_deref(), Some("example.com"));
        assert_eq!(c.path.as_deref(), Some("foo.git"));
        assert_eq!(c.username.as_deref(), Some("alice"));
    }

    #[test]
    fn fill_preserves_existing_userpass() {
        let source = CredentialSource::default();
        let desc = Credential {
            protocol: Some("http".into()),
            host: Some("127.0.0.1".into()),
            username: Some("preset".into()),
            password: Some("preset-pass".into()),
            ..Default::default()
        };
        let filled = fill(desc, &source).unwrap();
        assert_eq!(filled.username.as_deref(), Some("preset"));
        assert_eq!(filled.password.as_deref(), Some("preset-pass"));
    }

    #[test]
    fn fill_from_askpass_program() {
        let dir = TempDir::new().unwrap();
        #[cfg(windows)]
        let script = {
            let p = dir.path().join("askpass.cmd");
            fs::write(
                &p,
                "@echo off\r\necho %*| findstr /I Username >nul && (echo askuser& exit /b 0)\r\necho askpass\r\n",
            )
            .unwrap();
            p
        };
        #[cfg(not(windows))]
        let script = {
            let p = dir.path().join("askpass.sh");
            fs::write(
                &p,
                "#!/bin/sh\ncase \"$1\" in\n  *sername*) echo askuser ;;\n  *) echo askpass ;;\nesac\n",
            )
            .unwrap();
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&p).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&p, perms).unwrap();
            p
        };
        let source = CredentialSource {
            askpass: Some(script),
            ..Default::default()
        };
        let desc = Credential {
            protocol: Some("http".into()),
            host: Some("127.0.0.1".into()),
            ..Default::default()
        };
        let filled = fill(desc, &source).unwrap();
        assert_eq!(filled.username.as_deref(), Some("askuser"));
        assert_eq!(filled.password.as_deref(), Some("askpass"));
    }

    #[test]
    fn helper_shell_snippet_get() {
        let dir = TempDir::new().unwrap();
        #[cfg(windows)]
        let helper = {
            let p = dir.path().join("helper.cmd");
            fs::write(
                &p,
                "@echo off\r\nif not \"%1\"==\"get\" exit /b 0\r\necho username=huser\r\necho password=hpass\r\necho.\r\n",
            )
            .unwrap();
            format!("!{}", p.display())
        };
        #[cfg(not(windows))]
        let helper = {
            let p = dir.path().join("helper.sh");
            fs::write(
                &p,
                "#!/bin/sh\nif [ \"$1\" = get ]; then\n  printf 'username=huser\\npassword=hpass\\n\\n'\nfi\n",
            )
            .unwrap();
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&p).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&p, perms).unwrap();
            format!("!{}", p.display())
        };
        let source = CredentialSource {
            helpers: vec![helper],
            ..Default::default()
        };
        let desc = Credential {
            protocol: Some("http".into()),
            host: Some("example.com".into()),
            ..Default::default()
        };
        let filled = fill(desc, &source).unwrap();
        assert_eq!(filled.username.as_deref(), Some("huser"));
        assert_eq!(filled.password.as_deref(), Some("hpass"));
    }

    #[test]
    fn helper_command_transforms() {
        assert_eq!(helper_command("store"), "git credential-store");
        assert_eq!(helper_command("!f() { echo x; }; f"), "f() { echo x; }; f");
        assert_eq!(helper_command("/usr/bin/myhelper"), "/usr/bin/myhelper");
    }

    #[test]
    fn prepare_authorized_url_uses_userinfo() {
        let prepared = prepare_authorized_url_with(
            "http://user:secret@127.0.0.1:9/r.git",
            &CredentialSource::default(),
        )
        .unwrap();
        assert_eq!(prepared.url, "http://127.0.0.1:9/r.git");
        assert_eq!(
            prepared.authorization.as_deref(),
            Some("Basic dXNlcjpzZWNyZXQ=")
        );
    }
}
