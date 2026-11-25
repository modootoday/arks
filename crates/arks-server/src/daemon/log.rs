//! `git daemon` logging: `--log-destination=(stderr|syslog|none)`, `--syslog`,
//! `--verbose`.
//!
//! Primary: <https://git-scm.com/docs/git-daemon> (`--log-destination`). Errors are
//! always logged; informational lines only with `--verbose`. stderr lines carry
//! a `[pid] ` prefix like stock `git daemon`. Syslog writes RFC 3164-style
//! datagrams (`<PRI>ident[pid]: message`, facility daemon) to the local
//! `/dev/log` socket; no syslog crate and no libc call are needed.

use std::fmt;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Syslog identifier used by `arks daemon`.
pub const SYSLOG_IDENT: &str = "arks-daemon";
/// Default local syslog socket.
pub const SYSLOG_SOCKET: &str = "/dev/log";

/// Facility `daemon` (3) as in RFC 5424 §6.2.1.
const FACILITY_DAEMON: u8 = 3;
const SEVERITY_ERR: u8 = 3;
const SEVERITY_INFO: u8 = 6;

/// Where log lines go.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogDestination {
    Stderr,
    Syslog,
    None,
}

impl LogDestination {
    /// Parse a `--log-destination=` value.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "stderr" => Some(Self::Stderr),
            "syslog" => Some(Self::Syslog),
            "none" => Some(Self::None),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Level {
    Info,
    Error,
}

enum Sink {
    Stderr,
    None,
    Syslog(SyslogSink),
    #[cfg(test)]
    Memory(Vec<String>),
}

/// Shared daemon logger (cheap to clone; one sink per process).
#[derive(Clone)]
pub struct DaemonLog {
    destination: LogDestination,
    verbose: bool,
    sink: Arc<Mutex<Sink>>,
}

impl Default for DaemonLog {
    fn default() -> Self {
        Self::new(LogDestination::Stderr, false)
    }
}

impl fmt::Debug for DaemonLog {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DaemonLog")
            .field("destination", &self.destination)
            .field("verbose", &self.verbose)
            .finish()
    }
}

impl DaemonLog {
    pub fn new(destination: LogDestination, verbose: bool) -> Self {
        let sink = match destination {
            LogDestination::Stderr => Sink::Stderr,
            LogDestination::None => Sink::None,
            LogDestination::Syslog => Sink::Syslog(SyslogSink::new(PathBuf::from(SYSLOG_SOCKET))),
        };
        Self {
            destination,
            verbose,
            sink: Arc::new(Mutex::new(sink)),
        }
    }

    /// Syslog logger bound to a custom socket path (tests and non-standard hosts).
    pub fn syslog_at(socket: PathBuf, verbose: bool) -> Self {
        Self {
            destination: LogDestination::Syslog,
            verbose,
            sink: Arc::new(Mutex::new(Sink::Syslog(SyslogSink::new(socket)))),
        }
    }

    #[cfg(test)]
    pub(crate) fn memory(verbose: bool) -> Self {
        Self {
            destination: LogDestination::Stderr,
            verbose,
            sink: Arc::new(Mutex::new(Sink::Memory(Vec::new()))),
        }
    }

    #[cfg(test)]
    pub(crate) fn lines(&self) -> Vec<String> {
        match &*self.sink.lock().expect("log lock") {
            Sink::Memory(v) => v.clone(),
            _ => Vec::new(),
        }
    }

    pub fn destination(&self) -> LogDestination {
        self.destination
    }

    pub fn verbose(&self) -> bool {
        self.verbose
    }

    /// Informational line (only with `--verbose`).
    pub fn info(&self, msg: &str) {
        if self.verbose {
            self.write(Level::Info, msg);
        }
    }

    /// Error line (always).
    pub fn error(&self, msg: &str) {
        self.write(Level::Error, msg);
    }

    /// Fatal error of one connection, as git's die routine reports it: `fatal: <msg>`
    /// on stderr without the `[pid]` prefix, an err datagram under syslog, nothing
    /// under `none`.
    pub fn fatal(&self, msg: &str) {
        let Ok(mut sink) = self.sink.lock() else {
            return;
        };
        match &mut *sink {
            Sink::None => {}
            Sink::Stderr => {
                let _ = std::io::stderr().write_all(format!("fatal: {msg}\n").as_bytes());
            }
            Sink::Syslog(s) => s.send(Level::Error, msg),
            #[cfg(test)]
            Sink::Memory(v) => v.push(format!("fatal: {msg}")),
        }
    }

    fn write(&self, level: Level, msg: &str) {
        let Ok(mut sink) = self.sink.lock() else {
            return;
        };
        match &mut *sink {
            Sink::None => {}
            Sink::Stderr => {
                let line = format!("[{}] {msg}\n", std::process::id());
                let _ = std::io::stderr().write_all(line.as_bytes());
            }
            Sink::Syslog(s) => s.send(level, msg),
            #[cfg(test)]
            Sink::Memory(v) => v.push(msg.to_string()),
        }
    }
}

/// Format one syslog datagram: `<PRI>arks-daemon[pid]: message`.
fn syslog_datagram(level: Level, msg: &str) -> Vec<u8> {
    let severity = match level {
        Level::Info => SEVERITY_INFO,
        Level::Error => SEVERITY_ERR,
    };
    let pri = u16::from(FACILITY_DAEMON) * 8 + u16::from(severity);
    format!("<{pri}>{SYSLOG_IDENT}[{}]: {msg}", std::process::id()).into_bytes()
}

struct SyslogSink {
    path: PathBuf,
    #[cfg(unix)]
    socket: Option<std::os::unix::net::UnixDatagram>,
}

impl SyslogSink {
    fn new(path: PathBuf) -> Self {
        Self {
            path,
            #[cfg(unix)]
            socket: None,
        }
    }

    #[cfg(unix)]
    fn send(&mut self, level: Level, msg: &str) {
        use std::os::unix::net::UnixDatagram;
        let data = syslog_datagram(level, msg);
        // Connect lazily and retry once so a restarted syslogd is picked up.
        for _ in 0..2 {
            if self.socket.is_none() {
                let Ok(sock) = UnixDatagram::unbound() else {
                    return;
                };
                if sock.connect(&self.path).is_err() {
                    return;
                }
                self.socket = Some(sock);
            }
            if let Some(sock) = &self.socket {
                if sock.send(&data).is_ok() {
                    return;
                }
            }
            self.socket = None;
        }
    }

    #[cfg(not(unix))]
    fn send(&mut self, level: Level, msg: &str) {
        // No local syslog socket on this platform: drop the line.
        let _ = (&self.path, syslog_datagram(level, msg));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn destination_values_follow_git_daemon() {
        assert_eq!(
            LogDestination::parse("stderr"),
            Some(LogDestination::Stderr)
        );
        assert_eq!(
            LogDestination::parse("syslog"),
            Some(LogDestination::Syslog)
        );
        assert_eq!(LogDestination::parse("none"), Some(LogDestination::None));
        assert_eq!(LogDestination::parse("foo"), None);
    }

    #[test]
    fn info_lines_need_verbose() {
        let quiet = DaemonLog::memory(false);
        quiet.info("hello");
        quiet.error("boom");
        assert_eq!(quiet.lines(), vec!["boom".to_string()]);
        let loud = DaemonLog::memory(true);
        loud.info("hello");
        assert_eq!(loud.lines(), vec!["hello".to_string()]);
    }

    #[test]
    fn syslog_datagram_uses_daemon_facility() {
        let info = String::from_utf8(syslog_datagram(Level::Info, "Ready to rumble")).unwrap();
        assert!(info.starts_with("<30>arks-daemon["), "{info}");
        assert!(info.ends_with("]: Ready to rumble"), "{info}");
        let err = String::from_utf8(syslog_datagram(Level::Error, "x")).unwrap();
        assert!(err.starts_with("<27>"), "{err}");
    }

    #[cfg(unix)]
    #[test]
    fn syslog_sink_writes_to_local_socket() {
        use std::os::unix::net::UnixDatagram;
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("log.sock");
        let collector = UnixDatagram::bind(&path).unwrap();
        collector
            .set_read_timeout(Some(std::time::Duration::from_secs(2)))
            .unwrap();
        let log = DaemonLog::syslog_at(path, true);
        log.info("Ready to rumble");
        log.error("'x': aliased");
        let mut buf = [0u8; 512];
        let n = collector.recv(&mut buf).unwrap();
        let first = String::from_utf8_lossy(&buf[..n]).to_string();
        assert!(first.starts_with("<30>arks-daemon["), "{first}");
        assert!(first.ends_with("Ready to rumble"), "{first}");
        let n = collector.recv(&mut buf).unwrap();
        let second = String::from_utf8_lossy(&buf[..n]).to_string();
        assert!(second.starts_with("<27>"), "{second}");
    }

    #[test]
    fn none_destination_is_silent() {
        let log = DaemonLog::new(LogDestination::None, true);
        log.info("x");
        log.error("y");
        assert_eq!(log.destination(), LogDestination::None);
    }
}
