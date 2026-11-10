//! `git://` daemon listener (subset of git-daemon).
//!
//! Primary: <https://git-scm.com/docs/git-daemon>,
//! <https://git-scm.com/docs/gitprotocol-pack> (Git Transport initial request).
//!
//! Claimed: upload-pack service; optional receive-pack when `allow_receive` is set.
//! Requires `git-daemon-export-ok` unless `export_all`.

use crate::error::ServerError;
use crate::stdio::{receive_pack_stream, upload_pack_stream};
// upload_pack_session kept for CLI stdio; daemon uses single-stream helpers.
use arks_repo::Repository;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

#[derive(Debug, Clone)]
pub struct DaemonOptions {
    pub base_path: PathBuf,
    pub export_all: bool,
    pub allow_receive: bool,
    pub strict_paths: bool,
}

/// Bind and serve `git://` until `stop` is set.
pub fn serve_daemon(
    addr: SocketAddr,
    opts: DaemonOptions,
    stop: Arc<AtomicBool>,
) -> Result<(), ServerError> {
    let listener = TcpListener::bind(addr)?;
    listener.set_nonblocking(true)?;
    while !stop.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((stream, _)) => {
                let opts = opts.clone();
                thread::spawn(move || {
                    let _ = handle_daemon_connection(stream, &opts);
                });
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(std::time::Duration::from_millis(20));
            }
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}

/// Bind ephemeral loopback daemon; return (addr, stop, join).
pub fn spawn_loopback_daemon(
    opts: DaemonOptions,
) -> Result<(SocketAddr, Arc<AtomicBool>, thread::JoinHandle<Result<(), ServerError>>), ServerError>
{
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let addr = listener.local_addr()?;
    listener.set_nonblocking(true)?;
    let stop = Arc::new(AtomicBool::new(false));
    let stop2 = stop.clone();
    let handle = thread::spawn(move || {
        while !stop2.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((stream, _)) => {
                    let opts = opts.clone();
                    thread::spawn(move || {
                        let _ = handle_daemon_connection(stream, &opts);
                    });
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(std::time::Duration::from_millis(20));
                }
                Err(e) => return Err(e.into()),
            }
        }
        Ok(())
    });
    Ok((addr, stop, handle))
}

fn handle_daemon_connection(mut stream: TcpStream, opts: &DaemonOptions) -> Result<(), ServerError> {
    // Accepted sockets inherit nonblocking from the listener on some platforms
    // (notably Windows); force blocking for pack-protocol I/O.
    stream.set_nonblocking(false)?;
    stream.set_read_timeout(Some(std::time::Duration::from_secs(30)))?;
    stream.set_write_timeout(Some(std::time::Duration::from_secs(30)))?;
    let req = match read_service_request(&mut stream) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("arks daemon: bad service request: {e}");
            return Err(e);
        }
    };
    eprintln!(
        "arks daemon: service={} path={} base={}",
        req.service,
        req.path,
        opts.base_path.display()
    );
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
            // git-daemon --no-informative-errors: access denied
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
    // Pack protocol is half-duplex on one TCP stream (advertise → read → respond).
    let result = match req.service.as_str() {
        "git-upload-pack" => match upload_pack_stream(&repo, &mut stream) {
            Ok(()) => {
                eprintln!("arks daemon: upload-pack session ok");
                Ok(())
            }
            Err(e) => {
                eprintln!("arks daemon: upload-pack session err: {e}");
                Err(e)
            }
        },
        "git-receive-pack" if opts.allow_receive => receive_pack_stream(&mut repo, &mut stream),
        "git-receive-pack" => {
            write_err_pkt(&mut stream, "receive-pack not enabled")?;
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
    // First pkt-line: "git-upload-pack /path\0host=…\0[\0extra\0…]"
    let mut lenbuf = [0u8; 4];
    stream.read_exact(&mut lenbuf)?;
    let pkt_len = parse_pkt_len(&lenbuf)?;
    if pkt_len < 4 || pkt_len > 65_520 {
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
    // Split on NUL.
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

fn resolve_repo_path(opts: &DaemonOptions, req_path: &str) -> Result<PathBuf, ServerError> {
    let rel = req_path.trim_start_matches('/');
    let candidate = if rel.is_empty() {
        opts.base_path.clone()
    } else {
        opts.base_path.join(rel)
    };
    // Prefer existing path; avoid canonicalize prefix mismatches (`\\?\` on Windows).
    let resolved = if candidate.exists() {
        candidate
    } else if candidate.with_extension("git").exists() {
        candidate.with_extension("git")
    } else {
        candidate
    };
    let base_norm = strip_verbatim(opts.base_path.as_path());
    let resolved_norm = strip_verbatim(&resolved);
    if opts.strict_paths {
        if resolved_norm != base_norm && !resolved_norm.starts_with(&base_norm) {
            return Err(ServerError::Msg(format!(
                "path not allowed: {}",
                resolved.display()
            )));
        }
    } else if !resolved_norm.starts_with(&base_norm) && resolved_norm != base_norm {
        return Err(ServerError::Msg(format!(
            "path not allowed: {}",
            resolved.display()
        )));
    }
    if !resolved.exists() {
        return Err(ServerError::Msg(format!(
            "no repository at {}",
            resolved.display()
        )));
    }
    Ok(resolved)
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
