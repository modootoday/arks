//! Listening sockets, the accept loop and `--inetd` single-connection mode.
//!
//! Primary: <https://git-scm.com/docs/git-daemon> (`--listen`, `--port`,
//! `--max-connections`, `--timeout`, `--init-timeout`, `--inetd`). Log wording
//! follows observed stock `git daemon`.

use super::options::DaemonOptions;
use super::request::{HUNG_UP, read_service_request};
use super::session::{Outcome, handle_request};
use crate::error::ServerError;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

/// Default `git://` port (IANA `git`, git `DEFAULT_GIT_PORT`).
pub const DEFAULT_DAEMON_PORT: u16 = 9418;

/// Bound listening sockets (bind first, drop privileges, then serve).
#[derive(Debug)]
pub struct DaemonListeners {
    listeners: Vec<TcpListener>,
}

impl DaemonListeners {
    pub fn local_addrs(&self) -> Vec<SocketAddr> {
        self.listeners
            .iter()
            .filter_map(|l| l.local_addr().ok())
            .collect()
    }
}

/// Bind every address of every `--listen` host on `port`. With no host, bind
/// all interfaces as git does. Fails only when nothing could be bound.
pub fn bind_daemon(
    hosts: &[String],
    port: u16,
    opts: &DaemonOptions,
) -> Result<DaemonListeners, ServerError> {
    let log = &opts.log;
    let all = ["0.0.0.0".to_string(), "::".to_string()];
    let wildcard = hosts.is_empty();
    let hosts = if wildcard { &all[..] } else { hosts };
    let mut listeners: Vec<TcpListener> = Vec::new();
    for host in hosts {
        let addrs: Vec<SocketAddr> = match (host.as_str(), port).to_socket_addrs() {
            Ok(a) => a.collect(),
            Err(e) => {
                // std prefixes the resolver text; git prints gai_strerror() alone.
                let text = e.to_string();
                let text = text
                    .strip_prefix("failed to lookup address information: ")
                    .unwrap_or(&text);
                log.error(&format!("getaddrinfo() for {host} failed: {text}"));
                Vec::new()
            }
        };
        let mut bound_here = false;
        for addr in addrs {
            if listeners.iter().any(|l| l.local_addr().ok() == Some(addr)) {
                bound_here = true;
                continue;
            }
            match TcpListener::bind(addr) {
                Ok(l) => {
                    listeners.push(l);
                    bound_here = true;
                }
                Err(e) => log.error(&format!("Could not bind to {}: {e}", addr.ip())),
            }
        }
        if !bound_here && !wildcard {
            log.error(&format!(
                "unable to allocate any listen sockets for host {host} on port {port}"
            ));
        }
    }
    if listeners.is_empty() {
        return Err(ServerError::Msg(format!(
            "unable to allocate any listen sockets on port {port}"
        )));
    }
    Ok(DaemonListeners { listeners })
}

/// Serve `git://` on `listeners` until `stop` is set.
pub fn serve_daemon(
    listeners: DaemonListeners,
    opts: DaemonOptions,
    stop: Arc<AtomicBool>,
) -> Result<(), ServerError> {
    for l in &listeners.listeners {
        l.set_nonblocking(true)?;
    }
    opts.log.info("Ready to rumble");
    let opts = Arc::new(opts);
    let active = Arc::new(AtomicUsize::new(0));
    let mut serial = 0u64;
    while !stop.load(Ordering::SeqCst) {
        let mut accepted = false;
        for l in &listeners.listeners {
            match l.accept() {
                Ok((stream, peer)) => {
                    accepted = true;
                    serial += 1;
                    dispatch(stream, peer, serial, &opts, &active);
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                Err(e) => opts.log.error(&format!("accept: {e}")),
            }
        }
        if !accepted {
            thread::sleep(Duration::from_millis(20));
        }
    }
    Ok(())
}

fn dispatch(
    stream: TcpStream,
    peer: SocketAddr,
    serial: u64,
    opts: &Arc<DaemonOptions>,
    active: &Arc<AtomicUsize>,
) {
    if opts.max_connections > 0 && active.load(Ordering::SeqCst) >= opts.max_connections {
        opts.log.error("Too many children, dropping connection");
        drop(stream);
        return;
    }
    let opts = Arc::clone(opts);
    let active = Arc::clone(active);
    active.fetch_add(1, Ordering::SeqCst);
    thread::spawn(move || {
        let outcome = handle_tcp(stream, peer, &opts);
        let suffix = match outcome {
            Ok(Outcome::Served) => "",
            _ => " (with error)",
        };
        // git names the connection by its child pid; connections here are threads.
        opts.log.info(&format!("[{serial}] Disconnected{suffix}"));
        active.fetch_sub(1, Ordering::SeqCst);
    });
}

fn timeout(secs: u64) -> Option<Duration> {
    (secs > 0).then(|| Duration::from_secs(secs))
}

fn handle_tcp(
    mut stream: TcpStream,
    peer: SocketAddr,
    opts: &DaemonOptions,
) -> Result<Outcome, ServerError> {
    // git re-executes the server per connection (after any privilege drop), and
    // that process re-checks the base path before it logs or answers anything.
    if let Some(base) = opts.base_path.as_deref() {
        if !base.is_dir() {
            let msg = format!(
                "base-path '{}' does not exist or is not a directory",
                base.display()
            );
            opts.log.fatal(&msg);
            return Err(ServerError::Msg(msg));
        }
    }
    // Accepted sockets may inherit non-blocking mode from the listener.
    stream.set_nonblocking(false)?;
    opts.log.info(&format!("Connection from {peer}"));
    stream.set_read_timeout(timeout(opts.init_timeout_secs))?;
    let req = match read_service_request(&mut stream, &opts.log) {
        Ok(req) => req,
        Err(ServerError::Msg(m)) if m == HUNG_UP => {
            opts.log.fatal(&m);
            return Err(ServerError::Msg(m));
        }
        Err(e) => return Err(e),
    };
    stream.set_read_timeout(timeout(opts.timeout_secs))?;
    stream.set_write_timeout(timeout(opts.timeout_secs))?;
    handle_request(&mut stream, &req, opts, Some(peer))
}

/// Running loopback daemon: address, stop flag, serving thread.
pub type LoopbackDaemon = (
    SocketAddr,
    Arc<AtomicBool>,
    thread::JoinHandle<Result<(), ServerError>>,
);

/// Bind an ephemeral loopback daemon; return (addr, stop, join).
pub fn spawn_loopback_daemon(opts: DaemonOptions) -> Result<LoopbackDaemon, ServerError> {
    let listeners = bind_daemon(&["127.0.0.1".to_string()], 0, &opts)?;
    let addr = listeners
        .local_addrs()
        .first()
        .copied()
        .ok_or_else(|| ServerError::Msg("loopback daemon has no address".into()))?;
    let stop = Arc::new(AtomicBool::new(false));
    let stop2 = Arc::clone(&stop);
    let handle = thread::spawn(move || serve_daemon(listeners, opts, stop2));
    Ok((addr, stop, handle))
}

/// Read and write halves joined into one stream (`--inetd` stdin/stdout).
struct Duplex<R, W> {
    input: R,
    output: W,
}

impl<R: Read, W> Read for Duplex<R, W> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.input.read(buf)
    }
}

impl<R, W: Write> Write for Duplex<R, W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.output.write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.output.flush()
    }
}

/// `--inetd`: serve exactly one connection on `input`/`output`. `peer` is the
/// remote address when the CLI could read it from the inherited socket.
pub fn serve_daemon_inetd<R: Read, W: Write>(
    input: R,
    output: W,
    opts: &DaemonOptions,
    peer: Option<SocketAddr>,
) -> Result<Outcome, ServerError> {
    let mut io = Duplex { input, output };
    if let Some(p) = peer {
        opts.log.info(&format!("Connection from {p}"));
    }
    let req = read_service_request(&mut io, &opts.log)?;
    handle_request(&mut io, &req, opts, peer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daemon::log::DaemonLog;

    #[test]
    fn host_port_listen_value_is_not_a_host() {
        // Stock git daemon: `--listen=127.0.0.1:19507` fails getaddrinfo.
        let opts = DaemonOptions {
            log: DaemonLog::memory(false),
            ..DaemonOptions::default()
        };
        let err = bind_daemon(&["127.0.0.1:19507".into()], 0, &opts).unwrap_err();
        assert!(
            err.to_string()
                .contains("unable to allocate any listen sockets on port 0")
        );
        let lines = opts.log.lines();
        assert!(
            lines[0].starts_with("getaddrinfo() for 127.0.0.1:19507 failed"),
            "{lines:?}"
        );
        assert_eq!(
            lines[1],
            "unable to allocate any listen sockets for host 127.0.0.1:19507 on port 0"
        );
    }

    #[test]
    fn repeated_listen_and_host_names_bind() {
        let opts = DaemonOptions {
            log: DaemonLog::memory(false),
            ..DaemonOptions::default()
        };
        let l = bind_daemon(&["127.0.0.1".into(), "localhost".into()], 0, &opts).unwrap();
        assert!(!l.local_addrs().is_empty());
        assert!(l.local_addrs().iter().all(|a| a.ip().is_loopback()));
    }

    #[test]
    fn inetd_refusal_reports_refused() {
        let opts = DaemonOptions {
            log: DaemonLog::memory(false),
            ..DaemonOptions::default()
        };
        let req = b"0020git-upload-pack /nope/x.git\0".to_vec();
        let mut out = Vec::new();
        let r = serve_daemon_inetd(&req[..], &mut out, &opts, None).unwrap();
        assert_eq!(r, Outcome::Refused);
        assert!(String::from_utf8_lossy(&out).contains("ERR access denied"));
    }

    #[test]
    fn short_request_is_the_hung_up_fatal() {
        let opts = DaemonOptions {
            log: DaemonLog::memory(false),
            ..DaemonOptions::default()
        };
        for input in [&b""[..], &b"0030git-upl"[..]] {
            let err = serve_daemon_inetd(input, Vec::new(), &opts, None).unwrap_err();
            assert_eq!(err.to_string(), HUNG_UP);
        }
    }

    fn connect_and_close(opts: DaemonOptions) -> Vec<String> {
        let log = opts.log.clone();
        let (addr, stop, handle) = spawn_loopback_daemon(opts).unwrap();
        drop(TcpStream::connect(addr).unwrap());
        for _ in 0..100 {
            if log.lines().iter().any(|l| l.contains("Disconnected")) {
                break;
            }
            thread::sleep(Duration::from_millis(20));
        }
        stop.store(true, Ordering::SeqCst);
        handle.join().unwrap().unwrap();
        log.lines()
    }

    #[test]
    fn closed_connection_logs_fatal_and_numbered_disconnect() {
        let lines = connect_and_close(DaemonOptions {
            log: DaemonLog::memory(true),
            ..DaemonOptions::default()
        });
        assert!(
            lines.iter().any(|l| l.starts_with("Connection from ")),
            "{lines:?}"
        );
        assert!(lines.contains(&format!("fatal: {HUNG_UP}")), "{lines:?}");
        assert!(
            lines.contains(&"[1] Disconnected (with error)".to_string()),
            "{lines:?}"
        );
    }

    #[test]
    fn connection_rechecks_base_path() {
        let gone = tempfile::tempdir().unwrap().path().join("gone");
        let lines = connect_and_close(DaemonOptions {
            log: DaemonLog::memory(true),
            base_path: Some(gone.clone()),
            ..DaemonOptions::default()
        });
        let fatal = format!(
            "fatal: base-path '{}' does not exist or is not a directory",
            gone.display()
        );
        assert!(lines.contains(&fatal), "{lines:?}");
        assert!(
            !lines.iter().any(|l| l.starts_with("Connection from ")),
            "{lines:?}"
        );
    }
}
