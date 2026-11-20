//! OS lookups the daemon needs: `~user` home directories (`--user-path`) and
//! client host names (`%CH` / `%IP` for `--interpolated-path` and
//! `--access-hook`).
//!
//! This crate forbids `unsafe`, so the default [`StdPlatform`] uses only the
//! standard library (`/etc/passwd`, `ToSocketAddrs`); the CLI may inject a
//! platform backed by `getpwnam(3)` / `getaddrinfo(3)` for NSS and canonical
//! names.

use std::net::ToSocketAddrs;
use std::path::PathBuf;

/// Result of resolving the host name a client sent with `host=`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HostInfo {
    /// `%CH`: canonical host name.
    pub canonical: String,
    /// `%IP`: first address the name resolves to.
    pub ip: String,
}

/// OS services used while resolving a request.
pub trait DaemonPlatform: Send + Sync {
    /// Home directory of `user` (`~user` expansion).
    fn home_dir(&self, user: &str) -> Option<PathBuf>;
    /// Resolve a client-supplied host name; `None` when it does not resolve.
    fn lookup_host(&self, host: &str) -> Option<HostInfo>;
}

/// Standard-library implementation.
#[derive(Debug, Clone, Copy, Default)]
pub struct StdPlatform;

impl DaemonPlatform for StdPlatform {
    fn home_dir(&self, user: &str) -> Option<PathBuf> {
        let text = std::fs::read_to_string("/etc/passwd").ok()?;
        passwd_home(&text, user)
    }

    fn lookup_host(&self, host: &str) -> Option<HostInfo> {
        if host.is_empty() {
            return None;
        }
        let addr = (host, 0u16).to_socket_addrs().ok()?.next()?;
        Some(HostInfo {
            canonical: host.to_string(),
            ip: addr.ip().to_string(),
        })
    }
}

/// Home directory of `user` in `passwd(5)` text.
pub(super) fn passwd_home(text: &str, user: &str) -> Option<PathBuf> {
    text.lines().find_map(|line| {
        let fields: Vec<&str> = line.split(':').collect();
        (fields.len() >= 7 && fields[0] == user).then(|| PathBuf::from(fields[5]))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passwd_home_reads_sixth_field() {
        let text = "root:x:0:0:root:/root:/bin/sh\nalice:x:1000:1000::/home/alice:/bin/sh\n";
        assert_eq!(
            passwd_home(text, "alice"),
            Some(PathBuf::from("/home/alice"))
        );
        assert_eq!(passwd_home(text, "bob"), None);
    }

    #[test]
    fn std_lookup_resolves_numeric_hosts() {
        let info = StdPlatform.lookup_host("127.0.0.1").unwrap();
        assert_eq!(info.ip, "127.0.0.1");
        assert!(StdPlatform.lookup_host("").is_none());
    }
}
