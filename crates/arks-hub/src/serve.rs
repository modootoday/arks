//! Local forge API host (Partial; not Claimed).
//!
//! Used by transitional `arks hub serve` and by the forge module under
//! `arks serve` ([[sot:root/arks-self-host-server]]). Default bind:
//! `127.0.0.1:3920`. Never spawns git/gh. In-process TLS when `tls` is set (B3/B9 pattern).

use crate::http_min::{bearer_token, read_http_request, write_http_response};
use crate::store::HubStore;
use rustls::ServerConfig;
use serde_json::{Value, json};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// Stop flag + join handle of a background listener thread.
type ListenerThread = thread::JoinHandle<Result<(), String>>;

/// Shared slot for Partial live listen rebind (polled in forge accept loop).
pub type ForgeListenRebindSlot = Arc<Mutex<Option<SocketAddr>>>;

/// Options for the in-process local forge API listener (Partial).
#[derive(Clone)]
pub struct ForgeServeOptions {
    pub listen: SocketAddr,
    pub data_dir: PathBuf,
    /// Optional live listen rebind target (Partial; not Claimed zero-downtime).
    pub rebind_desired: Option<ForgeListenRebindSlot>,
    /// Optional rustls server config for in-process TLS terminate (B3/B9).
    pub tls: Option<Arc<ServerConfig>>,
    /// Root of forge repositories' Git directories (default `<data_dir>/git`).
    pub git_root: Option<PathBuf>,
    /// Clone URL base announced as `clone_url` (`<base>/<owner>/<name>.git`).
    pub git_url: Option<String>,
}

impl std::fmt::Debug for ForgeServeOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ForgeServeOptions")
            .field("listen", &self.listen)
            .field("data_dir", &self.data_dir)
            .field("rebind_desired", &self.rebind_desired.is_some())
            .field("tls", &self.tls.is_some())
            .field("git_root", &self.git_root)
            .field("git_url", &self.git_url)
            .finish()
    }
}

impl Default for ForgeServeOptions {
    fn default() -> Self {
        Self {
            listen: "127.0.0.1:3920"
                .parse()
                .expect("default forge listen parses"),
            data_dir: default_forge_data_dir(),
            rebind_desired: None,
            tls: None,
            git_root: None,
            git_url: None,
        }
    }
}

/// Scheme label for forge listen banners (`http` or `https`).
pub fn forge_serve_scheme(opts: &ForgeServeOptions) -> &'static str {
    if opts.tls.is_some() { "https" } else { "http" }
}

/// Default forge data directory (`ARKS_HUB_DATA` or `~/.config/arks/hub-data`).
pub fn default_forge_data_dir() -> PathBuf {
    HubStore::data_dir()
}

/// CLI entry for transitional `arks hub serve`.
pub fn run_serve(args: &[String]) -> Result<(), String> {
    let mut listen = "127.0.0.1:3920".to_string();
    let mut data = HubStore::data_dir();
    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        match a {
            "-h" | "--help" => {
                print_help();
                return Ok(());
            }
            "--listen" => {
                i += 1;
                listen = args
                    .get(i)
                    .ok_or_else(|| "serve --listen needs HOST:PORT".to_string())?
                    .clone();
            }
            _ if a.starts_with("--listen=") => listen = a[9..].to_string(),
            "--data" => {
                i += 1;
                data = PathBuf::from(
                    args.get(i)
                        .ok_or_else(|| "serve --data needs DIR".to_string())?,
                );
            }
            _ if a.starts_with("--data=") => data = PathBuf::from(&a[7..]),
            other if other.starts_with('-') => {
                return Err(format!("serve option not claimed: {other}"));
            }
            other => return Err(format!("serve unexpected arg: {other}")),
        }
        i += 1;
    }
    let addr: SocketAddr = listen
        .parse()
        .map_err(|e| format!("bad --listen address: {e}"))?;
    let stop = Arc::new(AtomicBool::new(false));
    serve_forge_api(
        ForgeServeOptions {
            listen: addr,
            data_dir: data,
            rebind_desired: None,
            tls: None,
            git_root: std::env::var_os("ARKS_HUB_GIT_ROOT").map(PathBuf::from),
            git_url: std::env::var("ARKS_HUB_GIT_URL").ok(),
        },
        stop,
    )
}

/// Bind and serve the local forge API until `stop` is set (Partial; not Claimed).
pub fn serve_forge_api(opts: ForgeServeOptions, stop: Arc<AtomicBool>) -> Result<(), String> {
    if !opts.listen.ip().is_loopback() {
        eprintln!(
            "arks forge API: LAN/non-loopback bind is Partial (no TLS Claimed; \
             [[sot:root/arks-self-host-server]])"
        );
    }
    let mut hub = HubStore::open(&opts.data_dir)?;
    if let Some(root) = &opts.git_root {
        hub.git_root = root.clone();
    }
    hub.git_url = opts.git_url.clone();
    let mut listener = TcpListener::bind(opts.listen).map_err(|e| e.to_string())?;
    hub.public_base = format!(
        "{}://{}",
        forge_serve_scheme(&opts),
        listener.local_addr().map_err(|e| e.to_string())?
    );
    let store = Arc::new(Mutex::new(hub));
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    let mut current = opts.listen;
    let scheme = forge_serve_scheme(&opts);
    eprintln!(
        "arks forge API listening on {scheme}://{}/ (data={}; Partial local API — IW-B7-FORGE drain)",
        listener.local_addr().map_err(|e| e.to_string())?,
        opts.data_dir.display()
    );
    if opts.tls.is_some() {
        eprintln!(
            "arks forge API: in-process TLS terminate armed (operator PEMs; same pattern as arks serve HTTP B9)"
        );
    }
    // Bounded dual-bind drain (mirror HTTP B7 Claimed; research:root/20261111090827).
    let mut draining: Vec<(TcpListener, SocketAddr, Instant)> = Vec::new();
    while !stop.load(Ordering::SeqCst) {
        if let Some(slot) = &opts.rebind_desired {
            if let Ok(mut g) = slot.lock() {
                if let Some(new_addr) = g.take() {
                    if new_addr != current {
                        match TcpListener::bind(new_addr) {
                            Ok(l) => match l.set_nonblocking(true) {
                                Ok(()) => {
                                    eprintln!(
                                        "arks forge API: live rebind dual-bind {current} → {new_addr}; \
                                         prior listener draining for 5s"
                                    );
                                    let old = std::mem::replace(&mut listener, l);
                                    draining.push((
                                        old,
                                        current,
                                        Instant::now() + Duration::from_secs(5),
                                    ));
                                    current = new_addr;
                                }
                                Err(e) => {
                                    eprintln!(
                                        "arks forge API: live rebind Partial nonblocking fail \
                                         ({e}); keeping {current}"
                                    );
                                    *g = Some(new_addr);
                                }
                            },
                            Err(e) => {
                                eprintln!(
                                    "arks forge API: live rebind Partial fail {current} → \
                                     {new_addr} ({e}); keeping prior listen"
                                );
                            }
                        }
                    }
                }
            }
        }
        draining.retain(|(_, old_addr, deadline)| {
            if Instant::now() < *deadline {
                true
            } else {
                eprintln!("arks forge API: rebind drain complete; closing {old_addr}");
                false
            }
        });
        match listener.accept() {
            Ok((stream, _)) => {
                let store = store.clone();
                let tls = opts.tls.clone();
                thread::spawn(move || {
                    if let Err(e) = handle_forge_connection(stream, &store, &tls) {
                        eprintln!("arks forge API: {e}");
                    }
                });
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(10));
            }
            Err(e) => return Err(e.to_string()),
        }
        for (old_listener, _, _) in &draining {
            match old_listener.accept() {
                Ok((stream, _)) => {
                    let store = store.clone();
                    let tls = opts.tls.clone();
                    thread::spawn(move || {
                        if let Err(e) = handle_forge_connection(stream, &store, &tls) {
                            eprintln!("arks forge API: {e}");
                        }
                    });
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(e) => eprintln!("arks forge API: draining listener accept error: {e}"),
            }
        }
    }
    Ok(())
}

/// Spawn forge API on a background thread; returns stop flag + join handle.
pub fn spawn_forge_api(
    opts: ForgeServeOptions,
) -> Result<(Arc<AtomicBool>, ListenerThread), String> {
    let stop = Arc::new(AtomicBool::new(false));
    let stop2 = stop.clone();
    let handle = thread::spawn(move || serve_forge_api(opts, stop2));
    // Brief settle so bind errors surface before cohost starts.
    thread::sleep(Duration::from_millis(50));
    Ok((stop, handle))
}

fn print_help() {
    eprintln!(
        "\
arks hub serve — transitional local forge API (Partial; prefer `arks serve` forge module)

Usage:
  arks hub serve [--listen HOST:PORT] [--data DIR]

Defaults:
  --listen 127.0.0.1:3920
  --data   $ARKS_HUB_DATA or ~/.config/arks/hub-data

Point clients with:
  ARKS_HUB_API=http://127.0.0.1:3920
  ARKS_HUB_TOKEN=local-dev-token

Never spawns git/gh. No TLS Claimed. Design ≠ Claimed.
"
    );
}

fn handle_forge_connection(
    mut stream: TcpStream,
    store: &Arc<Mutex<HubStore>>,
    tls: &Option<Arc<ServerConfig>>,
) -> Result<(), String> {
    stream.set_nonblocking(false).map_err(|e| e.to_string())?;
    let _ = stream.set_read_timeout(Some(Duration::from_secs(30)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(30)));
    if let Some(tls_cfg) = tls {
        let conn = rustls::ServerConnection::new(tls_cfg.clone())
            .map_err(|e| format!("tls ServerConnection: {e}"))?;
        let mut tls_stream = rustls::StreamOwned::new(conn, stream);
        tls_stream
            .conn
            .complete_io(&mut tls_stream.sock)
            .map_err(|e| format!("tls handshake: {e}"))?;
        handle_client(&mut tls_stream, store)
    } else {
        handle_client(&mut stream, store)
    }
}

fn handle_client<S: Read + Write>(
    stream: &mut S,
    store: &Arc<Mutex<HubStore>>,
) -> Result<(), String> {
    let req = read_http_request(&mut *stream)?;
    let token = bearer_token(&req.headers);
    let (path, query) = req
        .path
        .split_once('?')
        .map(|(p, q)| (p.to_string(), q.to_string()))
        .unwrap_or((req.path.clone(), String::new()));

    let mut st = store
        .lock()
        .map_err(|_| "store lock poisoned".to_string())?;
    // Health is unauthenticated.
    if req.method == "GET" && (path == "/" || path == "/health") {
        let body = json!({"ok":true,"service":"arks-hub","partial":true});
        return respond(stream, 200, &body);
    }

    if !st.auth_ok(token.as_deref()) && path != "/login" {
        let body =
            json!({"message":"Requires authentication","documentation_url":"arks hub auth login"});
        return respond(stream, 401, &body);
    }
    st.actor = st.login_for(token.as_deref());

    // Register token (local login bridge); `login` names the account (multi-account).
    if req.method == "POST" && path == "/login" {
        let v: Value = serde_json::from_slice(&req.body).unwrap_or(json!({}));
        let tok = v
            .get("token")
            .and_then(|x| x.as_str())
            .unwrap_or("local-dev-token")
            .to_string();
        st.ensure_token(&tok);
        if let Some(login) = v
            .get("login")
            .and_then(|x| x.as_str())
            .filter(|s| !s.is_empty())
        {
            st.db.token_logins.insert(tok.clone(), login.to_string());
        }
        st.save()?;
        let login = st.login_for(Some(&tok));
        return respond(stream, 200, &json!({"token": tok, "login": login}));
    }

    if req.method == "GET" && path == "/user" {
        let login = st.actor.clone();
        return respond(
            stream,
            200,
            &json!({"login": login, "id": 1, "type": "User", "name": login}),
        );
    }
    if req.method == "GET" {
        if let Some(login) = path.strip_prefix("/users/").filter(|l| !l.contains('/')) {
            let body = crate::forge::repos::user_json(&st, login);
            return respond(stream, 200, &body);
        }
    }

    let accept = header_value(&req.headers, "accept").unwrap_or_default();
    let content_type = header_value(&req.headers, "content-type").unwrap_or_default();
    let forge_req = crate::forge::Req {
        method: &req.method,
        path: &path,
        query: &query,
        body: &req.body,
        accept: &accept,
        content_type: &content_type,
    };
    if let Some((status, body)) = crate::forge::route(&mut st, &forge_req)? {
        // Raw payloads (asset bytes, archives, diffs) ride in marker fields.
        if let Some(b64) = body.get("__bytes_b64").and_then(Value::as_str) {
            use base64::Engine as _;
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(b64)
                .map_err(|e| e.to_string())?;
            let ctype = body
                .get("__content_type")
                .and_then(Value::as_str)
                .unwrap_or("application/octet-stream");
            return write_http_response(stream, status, "OK", ctype, &bytes);
        }
        if let Some(text) = body.get("__raw_text").and_then(Value::as_str) {
            return write_http_response(
                stream,
                status,
                "OK",
                "text/plain; charset=utf-8",
                text.as_bytes(),
            );
        }
        return respond(stream, status, &body);
    }

    respond(
        stream,
        404,
        &json!({"message":"Not Found","path": path, "method": req.method}),
    )
}

/// Minimal percent-decode for `q=` (Partial; spaces via `+` or `%20`).
pub(crate) fn urlencoding_lite_decode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                let h = |c: u8| -> Option<u8> {
                    match c {
                        b'0'..=b'9' => Some(c - b'0'),
                        b'a'..=b'f' => Some(c - b'a' + 10),
                        b'A'..=b'F' => Some(c - b'A' + 10),
                        _ => None,
                    }
                };
                if let (Some(hi), Some(lo)) = (h(bytes[i + 1]), h(bytes[i + 2])) {
                    out.push((hi << 4 | lo) as char);
                    i += 3;
                } else {
                    out.push('%');
                    i += 1;
                }
            }
            c => {
                out.push(c as char);
                i += 1;
            }
        }
    }
    out
}

fn header_value(headers: &[(String, String)], name: &str) -> Option<String> {
    headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.clone())
}

fn respond<S: Write>(stream: &mut S, status: u16, body: &serde_json::Value) -> Result<(), String> {
    let reason = match status {
        200 => "OK",
        201 => "Created",
        202 => "Accepted",
        204 => "No Content",
        401 => "Unauthorized",
        404 => "Not Found",
        405 => "Method Not Allowed",
        422 => "Unprocessable Entity",
        _ => "Error",
    };
    let bytes = serde_json::to_vec(body).map_err(|e| e.to_string())?;
    write_http_response(stream, status, reason, "application/json", &bytes)
}
