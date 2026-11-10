//! `git://` daemon listener (subset of git-daemon).
//!
//! Primary: <https://git-scm.com/docs/git-daemon>,
//! <https://git-scm.com/docs/gitprotocol-pack> (Git Transport initial request).
//!
//! Claimed: upload-pack service; optional receive-pack when `allow_receive` is set.
//! Requires `git-daemon-export-ok` unless `export_all`.
//! Upload-archive: Partial — drains args, sends ACK, streams raw ustar of treeish
//! (sideband multiplex / format flags not Claimed).
//! `--inetd` one-shot stdio peer is Partial (no TCP bind).
//! Never spawns `git` or `gh`.

use crate::error::ServerError;
use crate::stdio::{receive_pack_stream, upload_pack_stream};
use arks_object::{Commit, ObjectKind};
use arks_oid::Oid;
use arks_protocol::{Packet, encode_packet};
use arks_repo::{Repository, flatten_tree_entries};
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

/// One-shot inetd peer on split stdin/stdout (Partial).
///
/// Drains the service request + optional upload-archive args, then fail-closed
/// with an honest Partial note. Claimed pack wire remains the TCP `serve_daemon`
/// path (duplex socket). Cite: git-daemon `--inetd`.
pub fn serve_daemon_inetd<R: Read, W: Write>(
    mut input: R,
    mut output: W,
    opts: &DaemonOptions,
) -> Result<(), ServerError> {
    let req = match read_service_request(&mut input) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("arks daemon --inetd: bad service request: {e}");
            return Err(e);
        }
    };
    if opts.verbose {
        eprintln!(
            "arks daemon --inetd: service={} path={} (Partial stdio)",
            req.service, req.path
        );
    }
    if req.service == "git-upload-archive" && opts.allow_upload_archive {
        let args = drain_upload_archive_args(&mut input)?;
        if opts.verbose {
            eprintln!("arks daemon --inetd: upload-archive args={args:?}");
        }
        let repo_path = match resolve_repo_path(opts, &req.path) {
            Ok(p) => p,
            Err(e) => return write_err_pkt(&mut output, &e.to_string()),
        };
        if !opts.export_all {
            let marker = repo_path.join("git-daemon-export-ok");
            let marker_git = repo_path.join(".git/git-daemon-export-ok");
            if !marker.is_file() && !marker_git.is_file() {
                return write_err_pkt(&mut output, "access denied");
            }
        }
        let repo = match open_repo(&repo_path) {
            Ok(r) => r,
            Err(e) => return write_err_pkt(&mut output, &format!("access denied: {e}")),
        };
        return serve_upload_archive_partial(&repo, &args, &mut output, opts.verbose);
    }
    write_err_pkt(
        &mut output,
        &format!(
            "arks daemon --inetd Partial: service={} path={}; split stdio pack not Claimed — \
             use TCP `arks daemon` for upload-pack/receive-pack",
            req.service, req.path
        ),
    )
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
            let args = drain_upload_archive_args(&mut stream)?;
            if opts.verbose {
                eprintln!(
                    "arks daemon: upload-archive Partial args={args:?} repo={}",
                    repo.git_dir().display()
                );
            }
            serve_upload_archive_partial(&repo, &args, &mut stream, opts.verbose)
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

/// Partial upload-archive body: ACK pkt-line + raw ustar of treeish (default HEAD).
/// Sideband multiplex / `--format=` / path filters are **not** Claimed.
fn serve_upload_archive_partial<W: Write>(
    repo: &Repository,
    args: &[String],
    out: &mut W,
    verbose: bool,
) -> Result<(), ServerError> {
    let treeish = args
        .iter()
        .find(|a| !a.starts_with('-') && !a.is_empty())
        .map(String::as_str)
        .unwrap_or("HEAD");
    if verbose {
        eprintln!("arks daemon: upload-archive Partial treeish={treeish} (ACK+raw ustar)");
    }
    let tree_oid = match resolve_treeish(repo, treeish) {
        Ok(o) => o,
        Err(e) => return write_err_pkt(out, &e),
    };
    let ack = encode_packet(&Packet::Data(b"ACK\n".to_vec()))
        .map_err(|e| ServerError::Protocol(e.to_string()))?;
    out.write_all(&ack)?;
    match write_ustar_tree(repo, &tree_oid, out) {
        Ok(()) => {
            out.flush()?;
            Ok(())
        }
        Err(e) => {
            // After ACK, clients expect archive bytes; best-effort note on stderr only.
            eprintln!("arks daemon: upload-archive Partial ustar err: {e}");
            Err(e)
        }
    }
}

fn resolve_treeish(repo: &Repository, treeish: &str) -> Result<Oid, String> {
    let oid = if treeish == "HEAD" {
        repo.head_oid().map_err(|e| e.to_string())?
    } else if let Ok(o) = Oid::from_hex(treeish) {
        o
    } else {
        repo.refs()
            .resolve(treeish)
            .or_else(|_| repo.refs().resolve(&format!("refs/heads/{treeish}")))
            .or_else(|_| repo.refs().resolve(&format!("refs/tags/{treeish}")))
            .map_err(|e| e.to_string())?
    };
    tree_for_oid(repo, &oid)
}

fn tree_for_oid(repo: &Repository, oid: &Oid) -> Result<Oid, String> {
    let obj = repo.get_object(oid).map_err(|e| e.to_string())?;
    match obj.kind {
        ObjectKind::Commit => {
            let c = Commit::decode(&obj.payload).map_err(|e| e.to_string())?;
            Ok(c.tree)
        }
        ObjectKind::Tree => Ok(*oid),
        ObjectKind::Tag => {
            let text = std::str::from_utf8(&obj.payload).map_err(|_| "tag not utf-8")?;
            let mut target = None;
            let mut typ = None;
            for line in text.lines() {
                if line.is_empty() {
                    break;
                }
                if let Some(rest) = line.strip_prefix("object ") {
                    target = Some(Oid::from_hex(rest.trim()).map_err(|e| e.to_string())?);
                } else if let Some(rest) = line.strip_prefix("type ") {
                    typ = Some(rest.trim().to_owned());
                }
            }
            let target = target.ok_or_else(|| "tag missing object".to_string())?;
            if typ.as_deref() == Some("tree") {
                Ok(target)
            } else {
                tree_for_oid(repo, &target)
            }
        }
        ObjectKind::Blob => Err("cannot archive a blob".into()),
    }
}

fn write_ustar_tree<W: Write>(
    repo: &Repository,
    tree_oid: &Oid,
    out: &mut W,
) -> Result<(), ServerError> {
    let mut entries: Vec<(String, Oid)> = Vec::new();
    flatten_tree_entries(repo, tree_oid, "", &mut entries)
        .map_err(|e| ServerError::Msg(e.to_string()))?;
    entries.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
    for (path, oid) in entries {
        if path.len() > 100 {
            continue; // Partial: long paths skipped (not Claimed pax).
        }
        let obj = repo
            .get_object(&oid)
            .map_err(|e| ServerError::Msg(e.to_string()))?;
        if obj.kind != ObjectKind::Blob {
            continue;
        }
        write_ustar_entry(out, &path, &obj.payload, 0o644)?;
    }
    out.write_all(&[0u8; 1024])?;
    Ok(())
}

fn write_ustar_entry<W: Write>(
    out: &mut W,
    path: &str,
    data: &[u8],
    mode: u32,
) -> Result<(), ServerError> {
    let mut header = [0u8; 512];
    header[..path.len()].copy_from_slice(path.as_bytes());
    write_octal(&mut header[100..108], mode as u64);
    write_octal(&mut header[108..116], 0);
    write_octal(&mut header[116..124], 0);
    write_octal(&mut header[124..136], data.len() as u64);
    write_octal(&mut header[136..148], 0);
    header[148..156].fill(b' ');
    header[156] = b'0';
    header[257..262].copy_from_slice(b"ustar");
    header[262] = 0;
    header[263..265].copy_from_slice(b"00");
    let sum: u32 = header.iter().map(|&b| u32::from(b)).sum();
    write_octal(&mut header[148..156], u64::from(sum));
    out.write_all(&header)?;
    out.write_all(data)?;
    let pad = (512 - (data.len() % 512)) % 512;
    if pad > 0 {
        out.write_all(&vec![0u8; pad])?;
    }
    Ok(())
}

fn write_octal(dst: &mut [u8], value: u64) {
    dst.fill(b'0');
    let end = dst.len().saturating_sub(1);
    if end == 0 {
        return;
    }
    let mut v = value;
    let mut i = end;
    if v == 0 {
        dst[end - 1] = b'0';
        dst[end] = 0;
        return;
    }
    while v > 0 && i > 0 {
        i -= 1;
        dst[i] = b'0' + (v % 8) as u8;
        v /= 8;
    }
    dst[end] = 0;
}

fn drain_upload_archive_args<R: Read>(input: &mut R) -> Result<Vec<String>, ServerError> {
    let mut args = Vec::new();
    // Cap reads so a stuck peer cannot hang forever (Partial).
    for _ in 0..64 {
        let mut lenbuf = [0u8; 4];
        match input.read_exact(&mut lenbuf) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(e.into()),
        }
        let pkt_len = parse_pkt_len(&lenbuf)?;
        // 0000 flush / 0001 delim / 0002 response-end — end of argument list.
        if pkt_len < 4 {
            break;
        }
        let mut payload = vec![0u8; pkt_len - 4];
        if !payload.is_empty() {
            input.read_exact(&mut payload)?;
        }
        let line = String::from_utf8_lossy(&payload);
        let line = line.trim_end_matches(['\n', '\r']);
        if let Some(arg) = line.strip_prefix("argument ") {
            args.push(arg.to_string());
        } else if !line.is_empty() {
            args.push(line.to_string());
        }
    }
    Ok(args)
}

fn write_err_pkt<W: Write>(stream: &mut W, msg: &str) -> Result<(), ServerError> {
    // gitprotocol-pack: error-line = PKT-LINE("ERR" SP text)
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

fn read_service_request<R: Read>(stream: &mut R) -> Result<ServiceRequest, ServerError> {
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
