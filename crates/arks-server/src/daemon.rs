//! `git://` daemon listener (subset of git-daemon).
//!
//! Primary: <https://git-scm.com/docs/git-daemon>,
//! <https://git-scm.com/docs/gitprotocol-pack> (Git Transport initial request).
//!
//! Claimed: upload-pack service; optional receive-pack when `allow_receive` is set.
//! Requires `git-daemon-export-ok` unless `export_all`.
//! Upload-archive enable flag is Partial (protocol body not Claimed).
//! Never spawns `git` or `gh`.

use crate::error::ServerError;
use crate::stdio::{receive_pack_stream, upload_pack_stream};
use arks_repo::Repository;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

type LoopbackDaemon = (
    SocketAddr,
    Arc<AtomicBool>,
    thread::JoinHandle<Result<(), ServerError>>,
);

#[derive(Debug, Clone)]
pub struct DaemonOptions {
    /// Primary base (git `--base-path` / first positional).
    pub base_path: PathBuf,
    /// Extra allow-listed directories (additional positionals; Partial multi-dir).
    pub allowed_bases: Vec<PathBuf>,
    pub export_all: bool,
    pub allow_receive: bool,
    pub allow_upload_archive: bool,
    pub allow_upload_pack: bool,
    pub strict_paths: bool,
    /// Per-connection idle timeout after the service request (git-daemon `--timeout`).
    pub timeout_secs: u64,
    /// Timeout for reading the initial service request pkt (git-daemon `--init-timeout`).
    pub init_timeout_secs: u64,
    /// Max concurrent connection handlers (0 = unbounded).
    pub max_connections: usize,
    pub verbose: bool,
    /// Request SO_REUSEADDR-style rebind honesty (platform Partial).
    pub reuseaddr: bool,
}

impl Default for DaemonOptions {
    fn default() -> Self {
        Self {
            base_path: PathBuf::from("."),
            allowed_bases: Vec::new(),
            export_all: false,
            allow_receive: false,
            allow_upload_archive: false,
            allow_upload_pack: true,
            strict_paths: false,
            timeout_secs: 30,
            init_timeout_secs: 5,
            max_connections: 0,
            verbose: false,
            reuseaddr: true,
        }
    }
}

fn bind_listener(addr: SocketAddr, reuseaddr: bool) -> Result<TcpListener, ServerError> {
    // git-daemon --reuseaddr: prefer rebinding promptly after restart.
    // std TcpListener::bind already sets SO_REUSEADDR on many Unix targets;
    // on Windows we still bind normally (Partial — no raw setsockopt without libc).
    let _ = reuseaddr;
    let listener = TcpListener::bind(addr)?;
    Ok(listener)
}

/// Bind and serve `git://` until `stop` is set.
pub fn serve_daemon(
    addr: SocketAddr,
    opts: DaemonOptions,
    stop: Arc<AtomicBool>,
) -> Result<(), ServerError> {
    let listener = bind_listener(addr, opts.reuseaddr)?;
    listener.set_nonblocking(true)?;
    let active = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    while !stop.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((stream, peer)) => {
                if opts.max_connections > 0
                    && active.load(Ordering::SeqCst) >= opts.max_connections
                {
                    if opts.verbose {
                        eprintln!("arks daemon: max-connections refuse peer={peer}");
                    }
                    // Accept then drop (git-daemon refuses excess); do not leave backlog hung.
                    drop(stream);
                    continue;
                }
                let opts = opts.clone();
                let active = active.clone();
                active.fetch_add(1, Ordering::SeqCst);
                thread::spawn(move || {
                    let _ = handle_daemon_connection(stream, &opts);
                    active.fetch_sub(1, Ordering::SeqCst);
                });
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(20));
            }
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}

/// Bind ephemeral loopback daemon; return (addr, stop, join).
pub fn spawn_loopback_daemon(opts: DaemonOptions) -> Result<LoopbackDaemon, ServerError> {
    let listener = bind_listener(
        "127.0.0.1:0"
            .parse()
            .map_err(|e| ServerError::Msg(format!("bad loopback: {e}")))?,
        opts.reuseaddr,
    )?;
    let addr = listener.local_addr()?;
    listener.set_nonblocking(true)?;
    let stop = Arc::new(AtomicBool::new(false));
    let stop2 = stop.clone();
    let active = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let handle = thread::spawn(move || {
        while !stop2.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((stream, _)) => {
                    if opts.max_connections > 0
                        && active.load(Ordering::SeqCst) >= opts.max_connections
                    {
                        drop(stream);
                        continue;
                    }
                    let opts = opts.clone();
                    let active = active.clone();
                    active.fetch_add(1, Ordering::SeqCst);
                    thread::spawn(move || {
                        let _ = handle_daemon_connection(stream, &opts);
                        active.fetch_sub(1, Ordering::SeqCst);
                    });
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(20));
                }
                Err(e) => return Err(e.into()),
            }
        }
        Ok(())
    });
    Ok((addr, stop, handle))
}

fn handle_daemon_connection(
    mut stream: TcpStream,
    opts: &DaemonOptions,
) -> Result<(), ServerError> {
    // Accepted sockets inherit nonblocking from the listener on some platforms
    // (notably Windows); force blocking for pack-protocol I/O.
    stream.set_nonblocking(false)?;
    let init = Duration::from_secs(opts.init_timeout_secs.max(1));
    let idle = Duration::from_secs(opts.timeout_secs.max(1));
    stream.set_read_timeout(Some(init))?;
    stream.set_write_timeout(Some(init))?;
    let req = match read_service_request(&mut stream) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("arks daemon: bad service request: {e}");
            return Err(e);
        }
    };
    // After the initial request, use --timeout for the pack session.
    stream.set_read_timeout(Some(idle))?;
    stream.set_write_timeout(Some(idle))?;
    if opts.verbose {
        eprintln!(
            "arks daemon: service={} path={} base={}",
            req.service,
            req.path,
            opts.base_path.display()
        );
    }
    let repo_path = match resolve_repo_path(opts, &req.path) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("arks daemon: resolve failed: {e}");
            let _ = write_err_pkt(&mut stream, &e.to_string());
            return Ok(());
        }
    };
    eprintln!("arks daemon: repo={}", repo_path.display());
    if !opts.export_all {
        let marker = repo_path.join("git-daemon-export-ok");
        let marker_git = repo_path.join(".git/git-daemon-export-ok");
        if !marker.is_file() && !marker_git.is_file() {
            let _ = write_err_pkt(&mut stream, "access denied");
            return Ok(());
        }
    }
    let mut repo = match open_repo(&repo_path) {
        Ok(r) => r,
        Err(e) => {
            let _ = write_err_pkt(&mut stream, &format!("access denied: {e}"));
            return Ok(());
        }
    };
    let result = match req.service.as_str() {
        "git-upload-pack" if opts.allow_upload_pack => match upload_pack_stream(&repo, &mut stream)
        {
            Ok(()) => {
                if opts.verbose {
                    eprintln!("arks daemon: upload-pack session ok");
                }
                Ok(())
            }
            Err(e) => {
                eprintln!("arks daemon: upload-pack session err: {e}");
                Err(e)
            }
        },
        "git-upload-pack" => {
            write_err_pkt(&mut stream, "upload-pack not enabled")?;
            Ok(())
        }
        "git-receive-pack" if opts.allow_receive => receive_pack_stream(&mut repo, &mut stream),
        "git-receive-pack" => {
            write_err_pkt(&mut stream, "receive-pack not enabled")?;
            Ok(())
        }
        "git-upload-archive" if opts.allow_upload_archive => {
            // Partial: flag accepted; protocol body not Claimed.
            write_err_pkt(
                &mut stream,
                "upload-archive enabled but protocol subset not claimed",
            )?;
            Ok(())
        }
        "git-upload-archive" => {
            write_err_pkt(&mut stream, "upload-archive not enabled")?;
            Ok(())
        }
        other => {
            write_err_pkt(&mut stream, &format!("unknown service: {other}"))?;
            Ok(())
        }
    };
    if let Err(e) = &result {
        let _ = write_err_pkt(&mut stream, &e.to_string());
    }
    result
}

fn write_err_pkt(stream: &mut TcpStream, msg: &str) -> Result<(), ServerError> {
    // gitprotocol-pack: error-line = PKT-LINE("ERR" SP text)
    use arks_protocol::{Packet, encode_packet};
    let line = format!("ERR {msg}\n");
    let pkt = encode_packet(&Packet::Data(line.into_bytes()))
        .map_err(|e| ServerError::Protocol(e.to_string()))?;
    stream.write_all(&pkt)?;
    stream.flush()?;
    Ok(())
}

struct ServiceRequest {
    service: String,
    path: String,
    #[allow(dead_code)]
    host: Option<String>,
    #[allow(dead_code)]
    extra: Vec<String>,
}

fn read_service_request(stream: &mut TcpStream) -> Result<ServiceRequest, ServerError> {
    let mut lenbuf = [0u8; 4];
    stream.read_exact(&mut lenbuf)?;
    let pkt_len = parse_pkt_len(&lenbuf)?;
    if !(4..=65_520).contains(&pkt_len) {
        return Err(ServerError::Protocol("bad service request length".into()));
    }
    let mut payload = vec![0u8; pkt_len - 4];
    if !payload.is_empty() {
        stream.read_exact(&mut payload)?;
    }
    parse_service_payload(&payload)
}

fn parse_pkt_len(lenbuf: &[u8; 4]) -> Result<usize, ServerError> {
    let mut n = 0usize;
    for &b in lenbuf {
        n = (n << 4)
            + match b {
                b'0'..=b'9' => (b - b'0') as usize,
                b'a'..=b'f' => (b - b'a' + 10) as usize,
                _ => return Err(ServerError::Protocol("bad pkt-len hex".into())),
            };
    }
    Ok(n)
}

fn parse_service_payload(payload: &[u8]) -> Result<ServiceRequest, ServerError> {
    let parts: Vec<&[u8]> = payload.split(|b| *b == 0).collect();
    let head = std::str::from_utf8(parts.first().copied().unwrap_or(b""))
        .map_err(|_| ServerError::Protocol("service request not utf-8".into()))?;
    let mut it = head.splitn(2, ' ');
    let service = it
        .next()
        .ok_or_else(|| ServerError::Protocol("missing service".into()))?
        .to_string();
    let path = it
        .next()
        .ok_or_else(|| ServerError::Protocol("missing path".into()))?
        .to_string();
    let mut host = None;
    let mut extra = Vec::new();
    for part in parts.iter().skip(1) {
        if part.is_empty() {
            continue;
        }
        let s = std::str::from_utf8(part)
            .map_err(|_| ServerError::Protocol("host/extra not utf-8".into()))?;
        if let Some(h) = s.strip_prefix("host=") {
            host = Some(h.to_string());
        } else {
            extra.push(s.to_string());
        }
    }
    Ok(ServiceRequest {
        service,
        path,
        host,
        extra,
    })
}

fn all_bases(opts: &DaemonOptions) -> Vec<PathBuf> {
    let mut v = vec![opts.base_path.clone()];
    v.extend(opts.allowed_bases.iter().cloned());
    v
}

fn resolve_repo_path(opts: &DaemonOptions, req_path: &str) -> Result<PathBuf, ServerError> {
    let rel = req_path.trim_start_matches('/');
    let mut last_err = ServerError::Msg(format!("no repository for path {req_path}"));
    for base in all_bases(opts) {
        let candidate = if rel.is_empty() {
            base.clone()
        } else {
            base.join(rel)
        };
        // Without --strict-paths, allow foo ↔ foo.git alias (git-daemon default).
        let resolved = if candidate.exists() {
            candidate.clone()
        } else if !opts.strict_paths && candidate.with_extension("git").exists() {
            candidate.with_extension("git")
        } else if !opts.strict_paths {
            let stripped = candidate.as_os_str().to_string_lossy();
            if let Some(s) = stripped.strip_suffix(".git") {
                let alt = PathBuf::from(s);
                if alt.exists() {
                    alt
                } else {
                    candidate.clone()
                }
            } else {
                candidate.clone()
            }
        } else {
            candidate.clone()
        };
        let base_norm = normalize_path(&base);
        let resolved_norm = normalize_path(&resolved);
        if !resolved_norm.starts_with(&base_norm) && resolved_norm != base_norm {
            last_err = ServerError::Msg(format!("path not allowed: {}", resolved.display()));
            continue;
        }
        if resolved.exists() {
            return Ok(resolved);
        }
        last_err = ServerError::Msg(format!("no repository at {}", resolved.display()));
    }
    Err(last_err)
}

fn normalize_path(path: &Path) -> PathBuf {
    let canon = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    strip_verbatim(&canon)
}

fn strip_verbatim(path: &Path) -> PathBuf {
    let s = path.to_string_lossy();
    if let Some(rest) = s.strip_prefix(r"\\?\") {
        PathBuf::from(rest)
    } else {
        path.to_path_buf()
    }
}

fn open_repo(path: &Path) -> Result<Repository, ServerError> {
    if let Ok(r) = Repository::open(path) {
        return Ok(r);
    }
    let git = path.join(".git");
    if git.is_dir() {
        return Repository::open(&git).map_err(|e| ServerError::Msg(e.to_string()));
    }
    Repository::open(path).map_err(|e| ServerError::Msg(e.to_string()))
}
