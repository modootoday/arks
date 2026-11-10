//! Claimed remote URL shapes: `http(s)://`, local path / same-host `file://`,
//! SSH (`ssh://` / SCP-like `user@host:path`) for M16 Sy-4, and `git://`
//! for daemon pack-protocol clients.

use crate::error::RemoteError;
use std::path::{Path, PathBuf};

/// A remote URL in the claimed smart HTTP subset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpRemoteUrl {
    /// URL with userinfo removed (safe to log and to pass to the HTTP client).
    pub url: String,
}

impl HttpRemoteUrl {
    /// Accept `http://` and `https://`. SSH, `git://`, and other schemes are refused.
    pub fn parse(raw: &str) -> Result<Self, RemoteError> {
        let (url, _auth) = split_url(raw)?;
        Ok(Self { url })
    }
}

/// OpenSSH client URL: `ssh://[user@]host[:port]/path` or SCP-like `user@host:path`.
///
/// Cite: [git-clone(1)](https://git-scm.com/docs/git-clone) URL forms;
/// pack dialog over SSH per [gitprotocol-pack](https://git-scm.com/docs/gitprotocol-pack).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SshRemoteUrl {
    /// Original argv / config string (preserved for `remote.*.url`).
    pub raw: String,
    /// Optional SSH user (`git` in `git@host:path`).
    pub user: Option<String>,
    /// Host name or address.
    pub host: String,
    /// Optional port from `ssh://host:port/...` (SCP-like form has no port field).
    pub port: Option<u16>,
    /// Repository path passed to `git-upload-pack` / `git-receive-pack`.
    pub path: String,
}

impl SshRemoteUrl {
    /// Accept `ssh://` and SCP-like `user@host:path` (not a Windows drive).
    pub fn parse(raw: &str) -> Result<Self, RemoteError> {
        let raw_trim = raw.trim();
        if raw_trim.is_empty() {
            return Err(RemoteError::UnsupportedUrl(raw.to_string()));
        }
        if let Some(rest) = raw_trim.strip_prefix("ssh://") {
            return parse_ssh_url(raw_trim, rest);
        }
        if looks_like_scp_ssh(raw_trim) {
            return parse_scp_ssh(raw_trim);
        }
        Err(RemoteError::UnsupportedUrl(raw.to_string()))
    }

    /// `user@host` or `host` for the OpenSSH destination argument.
    pub fn destination(&self) -> String {
        match &self.user {
            Some(u) => format!("{u}@{}", self.host),
            None => self.host.clone(),
        }
    }
}

/// `git://` daemon URL: `git://host[:port]/path`.
///
/// Default port **9418** when omitted ([git-daemon](https://git-scm.com/docs/git-daemon)).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitDaemonUrl {
    /// Original argv / config string.
    pub raw: String,
    /// Host name or address (no brackets for IPv6 literals stored bare).
    pub host: String,
    /// Optional explicit port; `None` means default 9418 at connect time.
    pub port: Option<u16>,
    /// Repository path including a leading `/` when present in the URL.
    pub path: String,
}

impl GitDaemonUrl {
    /// Accept `git://host[:port]/path` (path required).
    pub fn parse(raw: &str) -> Result<Self, RemoteError> {
        let raw_trim = raw.trim();
        let Some(rest) = raw_trim.strip_prefix("git://") else {
            return Err(RemoteError::UnsupportedUrl(raw.to_string()));
        };
        let rest = rest.trim();
        if rest.is_empty() {
            return Err(RemoteError::UnsupportedUrl(raw.to_string()));
        }
        let (authority, path) = match rest.find('/') {
            Some(i) => (&rest[..i], &rest[i..]),
            None => (rest, ""),
        };
        if authority.is_empty() || path.is_empty() || path == "/" {
            return Err(RemoteError::UnsupportedUrl(raw.to_string()));
        }
        let (host, port) =
            split_host_port(authority).ok_or_else(|| RemoteError::UnsupportedUrl(raw.to_string()))?;
        if host.is_empty() {
            return Err(RemoteError::UnsupportedUrl(raw.to_string()));
        }
        Ok(Self {
            raw: raw_trim.to_string(),
            host,
            port,
            path: path.to_string(),
        })
    }
}

fn parse_ssh_url(raw: &str, rest: &str) -> Result<SshRemoteUrl, RemoteError> {
    // ssh://[user@]host[:port]/path  — path may be absolute (/repo.git) or ~/...
    let rest = rest.trim();
    if rest.is_empty() {
        return Err(RemoteError::UnsupportedUrl(raw.to_string()));
    }
    let (authority, path) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, ""),
    };
    if authority.is_empty() || path.is_empty() || path == "/" {
        return Err(RemoteError::UnsupportedUrl(raw.to_string()));
    }
    let (user, hostport) = match authority.rsplit_once('@') {
        Some((user, host)) if !user.is_empty() && !host.is_empty() => {
            (Some(user.to_string()), host)
        }
        _ => (None, authority),
    };
    let (host, port) =
        split_host_port(hostport).ok_or_else(|| RemoteError::UnsupportedUrl(raw.to_string()))?;
    Ok(SshRemoteUrl {
        raw: raw.to_string(),
        user,
        host,
        port,
        path: path.to_string(),
    })
}

fn parse_scp_ssh(raw: &str) -> Result<SshRemoteUrl, RemoteError> {
    let (userhost, path) = raw
        .split_once(':')
        .ok_or_else(|| RemoteError::UnsupportedUrl(raw.to_string()))?;
    if path.is_empty() || path.starts_with("//") {
        return Err(RemoteError::UnsupportedUrl(raw.to_string()));
    }
    let (user, host) = match userhost.rsplit_once('@') {
        Some((user, host)) if !user.is_empty() && !host.is_empty() => {
            (Some(user.to_string()), host)
        }
        _ => (None, userhost),
    };
    if host.is_empty() || host.contains('/') {
        return Err(RemoteError::UnsupportedUrl(raw.to_string()));
    }
    Ok(SshRemoteUrl {
        raw: raw.to_string(),
        user,
        host: host.to_string(),
        port: None,
        path: path.to_string(),
    })
}

fn split_host_port(hostport: &str) -> Option<(String, Option<u16>)> {
    if hostport.is_empty() {
        return None;
    }
    // IPv6 in brackets: [2001:db8::1]:22
    if let Some(rest) = hostport.strip_prefix('[') {
        let (host, after) = rest.split_once(']')?;
        if host.is_empty() {
            return None;
        }
        if after.is_empty() {
            return Some((host.to_string(), None));
        }
        let port = after.strip_prefix(':')?.parse().ok()?;
        return Some((host.to_string(), Some(port)));
    }
    if let Some((host, port_s)) = hostport.rsplit_once(':') {
        // Distinguish host:port from bare IPv6 (has multiple ':').
        if !host.contains(':') && !port_s.is_empty() {
            let port = port_s.parse().ok()?;
            if host.is_empty() {
                return None;
            }
            return Some((host.to_string(), Some(port)));
        }
    }
    Some((hostport.to_string(), None))
}

/// Same-host local clone source: filesystem path or `file://` with empty/`localhost` host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalRemoteUrl {
    /// Original argv string preserved for `remote.origin.url`.
    pub raw: String,
    /// Resolved filesystem path to the source repository (bare or worktree).
    pub path: PathBuf,
}

impl LocalRemoteUrl {
    /// Accept absolute/relative paths and same-host `file://` URLs.
    ///
    /// Refuses `http(s)://`, SSH, `git://`, and `file://` with a non-local host.
    pub fn parse(raw: &str) -> Result<Self, RemoteError> {
        let raw_trim = raw.trim();
        if raw_trim.is_empty() {
            return Err(RemoteError::UnsupportedUrl(raw.to_string()));
        }
        // Same-host file:// first.
        if let Some(rest) = raw_trim.strip_prefix("file://") {
            let path = parse_file_url_path(rest)
                .ok_or_else(|| RemoteError::UnsupportedUrl(raw.to_string()))?;
            if path.as_os_str().is_empty() {
                return Err(RemoteError::UnsupportedUrl(raw.to_string()));
            }
            return Ok(Self {
                raw: raw_trim.to_string(),
                path,
            });
        }
        // Explicit non-local schemes.
        if looks_like_http(raw_trim)
            || raw_trim.starts_with("git://")
            || raw_trim.starts_with("ssh://")
            || raw_trim.contains("://")
            || looks_like_scp_ssh(raw_trim)
        {
            return Err(RemoteError::UnsupportedUrl(raw.to_string()));
        }
        // Plain filesystem path (Unix absolute/relative, Windows drive, UNC).
        let path = PathBuf::from(raw_trim);
        if path.as_os_str().is_empty() {
            return Err(RemoteError::UnsupportedUrl(raw.to_string()));
        }
        Ok(Self {
            raw: raw_trim.to_string(),
            path,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

fn looks_like_http(raw: &str) -> bool {
    raw.starts_with("http://") || raw.starts_with("https://")
}

/// SCP-like `user@host:path` (not a Windows drive letter).
fn looks_like_scp_ssh(raw: &str) -> bool {
    let Some((_, after_at)) = raw.split_once('@') else {
        return false;
    };
    // host:path — require a colon after @ that is not only a drive-style path.
    after_at.contains(':') && !after_at.starts_with('/')
}

fn parse_file_url_path(rest: &str) -> Option<PathBuf> {
    // file:///abs  | file:///C:/abs | file://localhost/abs | file://localhost/C:/...
    let rest = rest.trim();
    if rest.is_empty() {
        return None;
    }
    if let Some(stripped) = rest.strip_prefix('/') {
        // file:///C:/repo → C:/repo on Windows; file:///tmp/repo → /tmp/repo
        let bytes = rest.as_bytes();
        if bytes.len() >= 3 && bytes[2] == b':' {
            return Some(PathBuf::from(stripped));
        }
        return Some(PathBuf::from(rest));
    }
    let (host, path) = match rest.split_once('/') {
        Some((h, p)) => (h, p),
        None => return None,
    };
    if !(host.is_empty() || host.eq_ignore_ascii_case("localhost")) {
        return None;
    }
    if path.is_empty() {
        return None;
    }
    // Windows drive form: file://localhost/C:/repo
    if path.len() >= 2 && path.as_bytes()[1] == b':' {
        Some(PathBuf::from(path))
    } else {
        Some(PathBuf::from(format!("/{path}")))
    }
}

pub(crate) struct PreparedUrl {
    pub url: String,
    pub authorization: Option<String>,
}

/// Strip userinfo and resolve Authorization via credential fill.
///
/// Order (non-interactive): URL userinfo → `ARKS_REMOTE_*` env →
/// `credential.helper` → `GIT_ASKPASS` / `core.askPass` / `SSH_ASKPASS`.
/// Cite: [[sot:root/credential-helper-askpass]], gitcredentials / git-credential.
pub(crate) fn prepare_url_with_source(
    raw: &str,
    source: &crate::credential::CredentialSource,
) -> Result<PreparedUrl, RemoteError> {
    crate::credential::prepare_authorized_url_with(raw, source)
}

pub(crate) fn split_url(raw: &str) -> Result<(String, Option<String>), RemoteError> {
    let raw = raw.trim();
    if raw.is_empty() || raw.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(RemoteError::UnsupportedUrl(raw.to_string()));
    }
    let rest = if let Some(rest) = raw.strip_prefix("https://") {
        rest
    } else if let Some(rest) = raw.strip_prefix("http://") {
        rest
    } else {
        return Err(RemoteError::UnsupportedUrl(raw.to_string()));
    };
    let scheme = if raw.starts_with("https://") {
        "https"
    } else {
        "http"
    };
    let (authority, path) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, ""),
    };
    if authority.is_empty() {
        return Err(RemoteError::UnsupportedUrl(raw.to_string()));
    }
    let (userinfo, hostport) = match authority.rsplit_once('@') {
        Some((user, host)) if !host.is_empty() => (Some(user.to_string()), host),
        _ => (None, authority),
    };
    if hostport.is_empty() || hostport.starts_with(':') {
        return Err(RemoteError::UnsupportedUrl(raw.to_string()));
    }
    let url = if path.is_empty() {
        format!("{scheme}://{hostport}")
    } else {
        format!("{scheme}://{hostport}{path}")
    };
    Ok((url, userinfo))
}

pub(crate) fn basic_header(userinfo: &str) -> String {
    format!("Basic {}", base64(userinfo.as_bytes()))
}

fn base64(data: &[u8]) -> String {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    let mut i = 0;
    while i + 3 <= data.len() {
        let n = (u32::from(data[i]) << 16) | (u32::from(data[i + 1]) << 8) | u32::from(data[i + 2]);
        out.push(T[((n >> 18) & 63) as usize] as char);
        out.push(T[((n >> 12) & 63) as usize] as char);
        out.push(T[((n >> 6) & 63) as usize] as char);
        out.push(T[(n & 63) as usize] as char);
        i += 3;
    }
    let rem = data.len() - i;
    if rem == 1 {
        let n = u32::from(data[i]) << 16;
        out.push(T[((n >> 18) & 63) as usize] as char);
        out.push(T[((n >> 12) & 63) as usize] as char);
        out.push('=');
        out.push('=');
    } else if rem == 2 {
        let n = (u32::from(data[i]) << 16) | (u32::from(data[i + 1]) << 8);
        out.push(T[((n >> 18) & 63) as usize] as char);
        out.push(T[((n >> 12) & 63) as usize] as char);
        out.push(T[((n >> 6) & 63) as usize] as char);
        out.push('=');
    }
    out
}

pub(crate) fn join_url(base: &str, rel: &str) -> String {
    format!(
        "{}/{}",
        base.trim_end_matches('/'),
        rel.trim_start_matches('/')
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http_and_https_ok_ssh_refused() {
        assert!(HttpRemoteUrl::parse("http://example.com/r.git").is_ok());
        assert!(HttpRemoteUrl::parse("https://github.com/org/repo.git").is_ok());
        assert!(matches!(
            HttpRemoteUrl::parse("git@example.com:a/b.git"),
            Err(RemoteError::UnsupportedUrl(_))
        ));
        assert!(matches!(
            HttpRemoteUrl::parse("ssh://git@example.com/a/b.git"),
            Err(RemoteError::UnsupportedUrl(_))
        ));
        assert!(matches!(
            HttpRemoteUrl::parse("git://example.com/a.git"),
            Err(RemoteError::UnsupportedUrl(_))
        ));
    }

    #[test]
    fn ssh_url_and_scp_ok() {
        let u = SshRemoteUrl::parse("ssh://git@example.com/a/b.git").unwrap();
        assert_eq!(u.user.as_deref(), Some("git"));
        assert_eq!(u.host, "example.com");
        assert_eq!(u.port, None);
        assert_eq!(u.path, "/a/b.git");
        assert_eq!(u.destination(), "git@example.com");

        let p = SshRemoteUrl::parse("ssh://example.com:2222/~/repo.git").unwrap();
        assert_eq!(p.port, Some(2222));
        assert_eq!(p.path, "/~/repo.git");

        let s = SshRemoteUrl::parse("git@example.com:org/repo.git").unwrap();
        assert_eq!(s.user.as_deref(), Some("git"));
        assert_eq!(s.host, "example.com");
        assert_eq!(s.path, "org/repo.git");
        assert!(SshRemoteUrl::parse("http://example.com/r.git").is_err());
        assert!(SshRemoteUrl::parse("git://example.com/a.git").is_err());
        assert!(SshRemoteUrl::parse(r"C:\Users\example\repo").is_err());
    }

    #[test]
    fn git_daemon_url_ok() {
        let u = GitDaemonUrl::parse("git://example.com/a/b.git").unwrap();
        assert_eq!(u.host, "example.com");
        assert_eq!(u.port, None);
        assert_eq!(u.path, "/a/b.git");
        let p = GitDaemonUrl::parse("git://127.0.0.1:9419/repo.git").unwrap();
        assert_eq!(p.host, "127.0.0.1");
        assert_eq!(p.port, Some(9419));
        assert_eq!(p.path, "/repo.git");
        assert!(GitDaemonUrl::parse("http://example.com/a.git").is_err());
        assert!(GitDaemonUrl::parse("git://example.com").is_err());
        assert!(GitDaemonUrl::parse("git://example.com/").is_err());
    }

    #[test]
    fn userinfo_stripped_into_basic() {
        let prepared = prepare_url_with_source(
            "http://user:secret@127.0.0.1:9/r.git",
            &crate::credential::CredentialSource::default(),
        )
        .unwrap();
        assert_eq!(prepared.url, "http://127.0.0.1:9/r.git");
        assert_eq!(
            prepared.authorization.as_deref(),
            Some("Basic dXNlcjpzZWNyZXQ=")
        );
    }

    #[test]
    fn local_path_and_file_url_ok() {
        let p = LocalRemoteUrl::parse("/tmp/repo.git").unwrap();
        assert_eq!(p.path, PathBuf::from("/tmp/repo.git"));
        let f = LocalRemoteUrl::parse("file:///tmp/repo.git").unwrap();
        assert_eq!(f.path, PathBuf::from("/tmp/repo.git"));
        let l = LocalRemoteUrl::parse("file://localhost/tmp/repo.git").unwrap();
        assert_eq!(l.path, PathBuf::from("/tmp/repo.git"));
        let drive = LocalRemoteUrl::parse(r"C:\Users\example\repo").unwrap();
        assert_eq!(drive.path, PathBuf::from(r"C:\Users\example\repo"));
        assert!(LocalRemoteUrl::parse("http://example.com/r.git").is_err());
        assert!(LocalRemoteUrl::parse("file://otherhost/tmp/r.git").is_err());
        assert!(LocalRemoteUrl::parse("git@example.com:a/b.git").is_err());
    }
}
