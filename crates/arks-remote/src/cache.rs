//! In-memory credential cache: `credential-cache` client and
//! `credential-cache--daemon` (Unix domain socket).
//!
//! Cite: <https://git-scm.com/docs/git-credential-cache>,
//! <https://git-scm.com/docs/git-credential-cache--daemon> (behaviour, not code).
//! Credentials live only in the daemon's memory for `--timeout` seconds
//! (default 900). The socket defaults to `~/.git-credential-cache/socket`
//! when that directory exists, else `$XDG_CACHE_HOME/git/credential/socket`
//! (`~/.cache/...`). A `store` starts the daemon when none listens; `get`,
//! `erase` and `exit` without a daemon do nothing. The daemon exits when the
//! cache is empty (all entries expired or erased) or on `exit`.
//!
//! The request a client sends is `action=<action>`, `timeout=<seconds>` and
//! the credential attributes, then end of stream; a `get` answer is the
//! matching attributes. Client and daemon are both arks, which re-executes
//! its own binary for the daemon (never `git`).

use crate::credential::Credential;
use crate::error::RemoteError;
use std::path::{Path, PathBuf};

/// Default cache lifetime in seconds.
pub const DEFAULT_CACHE_TIMEOUT: u64 = 900;

/// The default daemon socket path.
pub fn default_cache_socket() -> PathBuf {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let legacy = home.join(".git-credential-cache");
    if legacy.is_dir() {
        return legacy.join("socket");
    }
    let cache = std::env::var_os("XDG_CACHE_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".cache"));
    cache.join("git").join("credential").join("socket")
}

/// Does the stored credential `entry` match the query `q` (fields the query sets)?
fn matches(entry: &Credential, q: &Credential, with_password: bool) -> bool {
    let eq = |a: &Option<String>, b: &Option<String>| b.is_none() || a == b;
    eq(&entry.protocol, &q.protocol)
        && eq(&entry.host, &q.host)
        && eq(&entry.path, &q.path)
        && eq(&entry.username, &q.username)
        && (!with_password || eq(&entry.password, &q.password))
}

#[cfg(unix)]
fn request_text(action: &str, timeout: u64, desc: &Credential) -> String {
    format!(
        "action={action}\ntimeout={timeout}\n{}",
        desc.format_attributes()
    )
}

fn parse_request(text: &str) -> (String, u64, Credential) {
    let mut action = String::new();
    let mut timeout = DEFAULT_CACHE_TIMEOUT;
    let mut rest = String::new();
    for line in text.lines() {
        if let Some(a) = line.strip_prefix("action=") {
            action = a.to_string();
        } else if let Some(t) = line.strip_prefix("timeout=") {
            timeout = t.parse().unwrap_or(DEFAULT_CACHE_TIMEOUT);
        } else {
            rest.push_str(line);
            rest.push('\n');
        }
    }
    (action, timeout, Credential::parse_attributes(&rest))
}

/// The cache contents and their expiry times.
#[derive(Default)]
pub struct CacheStore {
    entries: Vec<(Credential, std::time::Instant)>,
}

impl CacheStore {
    /// Drop expired entries.
    pub fn expire(&mut self, now: std::time::Instant) {
        self.entries.retain(|(_, until)| *until > now);
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Apply one request; returns the answer and whether the daemon should exit.
    pub fn handle(&mut self, text: &str, now: std::time::Instant) -> (String, bool) {
        self.expire(now);
        let (action, timeout, desc) = parse_request(text);
        match action.as_str() {
            "get" => {
                let hit = self
                    .entries
                    .iter()
                    .find(|(c, _)| matches(c, &desc, false))
                    .map(|(c, _)| {
                        let mut out = String::new();
                        if let Some(u) = &c.username {
                            out.push_str(&format!("username={u}\n"));
                        }
                        if let Some(p) = &c.password {
                            out.push_str(&format!("password={p}\n"));
                        }
                        out
                    });
                (hit.unwrap_or_default(), false)
            }
            "store" => {
                if desc.protocol.is_some() || desc.host.is_some() {
                    let key = Credential {
                        password: None,
                        ..desc.clone()
                    };
                    self.entries.retain(|(c, _)| {
                        !(c.protocol == key.protocol
                            && c.host == key.host
                            && c.path == key.path
                            && c.username == key.username)
                    });
                    let until = now + std::time::Duration::from_secs(timeout);
                    self.entries.push((desc, until));
                }
                (String::new(), false)
            }
            "erase" => {
                self.entries.retain(|(c, _)| !matches(c, &desc, true));
                (String::new(), false)
            }
            "exit" => (String::new(), true),
            _ => (String::new(), false),
        }
    }
}

#[cfg(unix)]
mod imp {
    use super::*;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::os::unix::net::{UnixListener, UnixStream};
    use std::process::{Command, Stdio};

    fn exchange(socket: &Path, request: &str) -> std::io::Result<String> {
        let mut s = UnixStream::connect(socket)?;
        s.write_all(request.as_bytes())?;
        s.shutdown(std::net::Shutdown::Write)?;
        let mut out = String::new();
        s.read_to_string(&mut out)?;
        Ok(out)
    }

    fn spawn_daemon(socket: &Path) -> Result<(), RemoteError> {
        let exe = std::env::current_exe()
            .map_err(|e| RemoteError::Msg(format!("credential-cache: {e}")))?;
        let mut child = Command::new(exe)
            .arg("credential-cache--daemon")
            .arg(socket)
            .env("ARKS_PERSONALITY", "git")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            // The daemon outlives this client: keep none of its descriptors.
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| RemoteError::Msg(format!("unable to start cache daemon: {e}")))?;
        let out = child
            .stdout
            .take()
            .ok_or_else(|| RemoteError::Msg("cache daemon stdout".into()))?;
        let mut line = String::new();
        BufReader::new(out)
            .read_line(&mut line)
            .map_err(|e| RemoteError::Msg(format!("cache daemon: {e}")))?;
        if line.trim_end() != "ok" {
            return Err(RemoteError::Msg("unable to start cache daemon".to_string()));
        }
        Ok(())
    }

    pub fn client(
        action: &str,
        desc: &Credential,
        socket: &Path,
        timeout: u64,
    ) -> Result<Option<Credential>, RemoteError> {
        let request = request_text(action, timeout, desc);
        let answer = match exchange(socket, &request) {
            Ok(a) => a,
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::NotFound | std::io::ErrorKind::ConnectionRefused
                ) =>
            {
                if action != "store" {
                    return Ok(None);
                }
                spawn_daemon(socket)?;
                exchange(socket, &request).map_err(|e| {
                    RemoteError::Msg(format!("unable to connect to cache daemon: {e}"))
                })?
            }
            Err(e) => {
                return Err(RemoteError::Msg(format!(
                    "unable to connect to cache daemon: {e}"
                )));
            }
        };
        if action == "get" && !answer.trim().is_empty() {
            return Ok(Some(Credential::parse_attributes(&answer)));
        }
        Ok(None)
    }

    pub fn daemon(socket: &Path, debug: bool) -> Result<(), RemoteError> {
        if !socket.is_absolute() {
            return Err(RemoteError::Msg(
                "socket directory must be an absolute path".into(),
            ));
        }
        if let Some(dir) = socket.parent() {
            std::fs::create_dir_all(dir)
                .map_err(|e| RemoteError::Msg(format!("unable to create socket directory: {e}")))?;
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700));
        }
        if socket.exists() && UnixStream::connect(socket).is_err() {
            let _ = std::fs::remove_file(socket);
        }
        let listener = UnixListener::bind(socket).map_err(|e| {
            RemoteError::Msg(format!("unable to bind to '{}': {e}", socket.display()))
        })?;
        listener
            .set_nonblocking(true)
            .map_err(|e| RemoteError::Msg(e.to_string()))?;
        {
            let mut out = std::io::stdout().lock();
            let _ = out.write_all(b"ok\n");
            let _ = out.flush();
        }
        let mut store = CacheStore::default();
        let mut served = false;
        loop {
            let now = std::time::Instant::now();
            store.expire(now);
            if served && store.is_empty() {
                break;
            }
            match listener.accept() {
                Ok((mut conn, _)) => {
                    let _ = conn.set_nonblocking(false);
                    let mut text = String::new();
                    if conn.read_to_string(&mut text).is_err() {
                        continue;
                    }
                    if debug {
                        eprintln!(
                            "credential-cache--daemon: request {}",
                            text.lines().next().unwrap_or("")
                        );
                    }
                    let (answer, exit) = store.handle(&text, std::time::Instant::now());
                    let _ = conn.write_all(answer.as_bytes());
                    drop(conn);
                    served = true;
                    if exit {
                        break;
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(std::time::Duration::from_millis(20));
                }
                Err(e) => return Err(RemoteError::Msg(format!("accept: {e}"))),
            }
        }
        let _ = std::fs::remove_file(socket);
        Ok(())
    }
}

#[cfg(not(unix))]
mod imp {
    use super::*;

    const NO_SOCKETS: &str =
        "credential-cache unavailable; no unix-socket support on this platform";

    pub fn client(
        _action: &str,
        _desc: &Credential,
        _socket: &Path,
        _timeout: u64,
    ) -> Result<Option<Credential>, RemoteError> {
        Err(RemoteError::Msg(NO_SOCKETS.into()))
    }

    pub fn daemon(_socket: &Path, _debug: bool) -> Result<(), RemoteError> {
        Err(RemoteError::Msg(NO_SOCKETS.into()))
    }
}

/// Send one helper action (`get` / `store` / `erase` / `exit`) to the cache.
pub fn cache_client(
    action: &str,
    desc: &Credential,
    socket: &Path,
    timeout: u64,
) -> Result<Option<Credential>, RemoteError> {
    imp::client(action, desc, socket, timeout)
}

/// Serve the cache on `socket` until it is empty or told to exit.
pub fn run_cache_daemon(socket: &Path, debug: bool) -> Result<(), RemoteError> {
    imp::daemon(socket, debug)
}

/// `credential.helper = cache [--timeout <n>] [--socket <path>]` options.
pub(crate) fn helper_options(helper: &str) -> (PathBuf, u64) {
    let mut socket = default_cache_socket();
    let mut timeout = DEFAULT_CACHE_TIMEOUT;
    let words: Vec<&str> = helper.split_whitespace().skip(1).collect();
    let mut i = 0;
    while i < words.len() {
        let w = words[i];
        let (name, val) = match w.split_once('=') {
            Some((n, v)) => (n, Some(v.to_string())),
            None => (w, words.get(i + 1).map(|s| s.to_string())),
        };
        let consumed = !w.contains('=');
        match name {
            "--timeout" => {
                if let Some(v) = val.and_then(|v| v.parse().ok()) {
                    timeout = v;
                }
                i += usize::from(consumed);
            }
            "--socket" => {
                if let Some(v) = val {
                    socket = PathBuf::from(v);
                }
                i += usize::from(consumed);
            }
            _ => {}
        }
        i += 1;
    }
    (socket, timeout)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cred(text: &str) -> Credential {
        Credential::parse_attributes(text)
    }

    #[test]
    fn store_get_erase_expire() {
        let mut s = CacheStore::default();
        let t0 = std::time::Instant::now();
        let store = "action=store\ntimeout=60\nprotocol=https\nhost=example.com\nusername=u\npassword=p\n\n";
        assert_eq!(s.handle(store, t0), (String::new(), false));
        let get = "action=get\ntimeout=60\nprotocol=https\nhost=example.com\n\n";
        assert_eq!(s.handle(get, t0).0, "username=u\npassword=p\n");
        let other = "action=get\ntimeout=60\nprotocol=https\nhost=other.example\n\n";
        assert_eq!(s.handle(other, t0).0, "");
        let later = t0 + std::time::Duration::from_secs(61);
        assert_eq!(s.handle(get, later).0, "");
        assert!(s.is_empty());
        s.handle(store, t0);
        s.handle("action=erase\nprotocol=https\nhost=example.com\n\n", t0);
        assert!(s.is_empty());
        assert!(s.handle("action=exit\n\n", t0).1);
        assert!(matches(&cred("host=a\n"), &cred("\n"), true));
    }

    #[test]
    fn helper_option_spellings() {
        let (sock, t) = helper_options("cache --timeout=30 --socket /tmp/s");
        assert_eq!((sock, t), (PathBuf::from("/tmp/s"), 30));
        let (_, t) = helper_options("cache --timeout 5");
        assert_eq!(t, 5);
    }
}
