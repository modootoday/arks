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
}

impl std::fmt::Debug for ForgeServeOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ForgeServeOptions")
            .field("listen", &self.listen)
            .field("data_dir", &self.data_dir)
            .field("rebind_desired", &self.rebind_desired.is_some())
            .field("tls", &self.tls.is_some())
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
    let store = Arc::new(Mutex::new(HubStore::open(&opts.data_dir)?));
    let mut listener = TcpListener::bind(opts.listen).map_err(|e| e.to_string())?;
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
) -> Result<(Arc<AtomicBool>, thread::JoinHandle<Result<(), String>>), String> {
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
    let query_q = query
        .split('&')
        .filter_map(|pair| {
            let (k, v) = pair.split_once('=')?;
            if k == "q" {
                Some(urlencoding_lite_decode(v))
            } else {
                None
            }
        })
        .next()
        .unwrap_or_default();

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

    // Register token (local login bridge).
    if req.method == "POST" && path == "/login" {
        let v: Value = serde_json::from_slice(&req.body).unwrap_or(json!({}));
        let tok = v
            .get("token")
            .and_then(|x| x.as_str())
            .unwrap_or("local-dev-token");
        st.ensure_token(tok);
        st.save()?;
        return respond(
            stream,
            200,
            &json!({"token": tok, "login": st.db.user_login}),
        );
    }

    if req.method == "GET" && path == "/user" {
        return respond(
            stream,
            200,
            &json!({"login": st.db.user_login, "id": 1, "type": "User"}),
        );
    }

    if req.method == "GET" && path == "/user/repos" {
        let repos: Vec<Value> = st
            .db
            .repos
            .iter()
            .map(|r| {
                json!({
                    "id": r.id,
                    "name": r.name,
                    "full_name": format!("{}/{}", r.owner, r.name),
                    "private": r.private,
                    "description": r.description,
                    "owner": {"login": r.owner},
                })
            })
            .collect();
        return respond(stream, 200, &Value::Array(repos));
    }

    if req.method == "POST" && path == "/user/repos" {
        let v: Value = serde_json::from_slice(&req.body).unwrap_or(json!({}));
        let name = v.get("name").and_then(|x| x.as_str()).unwrap_or("repo");
        let desc = v.get("description").and_then(|x| x.as_str()).unwrap_or("");
        let owner = st.db.user_login.clone();
        let repo = st.create_repo(&owner, name, desc);
        st.save()?;
        return respond(
            stream,
            201,
            &json!({
                "id": repo.id,
                "name": repo.name,
                "full_name": format!("{}/{}", repo.owner, repo.name),
                "private": repo.private,
                "description": repo.description,
                "owner": {"login": repo.owner},
            }),
        );
    }

    // /repos/{owner}/{repo}/…
    if let Some(rest) = path.strip_prefix("/repos/") {
        let parts: Vec<&str> = rest.split('/').collect();
        if parts.len() >= 2 {
            let owner = parts[0];
            let repo = parts[1];
            if parts.len() == 2 && req.method == "GET" {
                if let Some(r) = st
                    .db
                    .repos
                    .iter()
                    .find(|r| r.owner == owner && r.name == repo)
                {
                    return respond(
                        stream,
                        200,
                        &json!({
                            "id": r.id,
                            "name": r.name,
                            "full_name": format!("{}/{}", r.owner, r.name),
                            "private": r.private,
                            "description": r.description,
                            "owner": {"login": r.owner},
                        }),
                    );
                }
                return respond(stream, 404, &json!({"message":"Not Found"}));
            }
            if parts.len() == 3 && parts[2] == "issues" {
                if req.method == "GET" {
                    let items: Vec<Value> = st
                        .db
                        .issues
                        .iter()
                        .filter(|i| i.owner == owner && i.repo == repo)
                        .map(|i| {
                            json!({
                                "number": i.number,
                                "title": i.title,
                                "body": i.body,
                                "state": i.state,
                                "user": {"login": i.user},
                            })
                        })
                        .collect();
                    return respond(stream, 200, &Value::Array(items));
                }
                if req.method == "POST" {
                    let v: Value = serde_json::from_slice(&req.body).unwrap_or(json!({}));
                    let title = v.get("title").and_then(|x| x.as_str()).unwrap_or("issue");
                    let body = v.get("body").and_then(|x| x.as_str()).unwrap_or("");
                    let issue = st.create_issue(owner, repo, title, body);
                    st.save()?;
                    return respond(
                        stream,
                        201,
                        &json!({
                            "number": issue.number,
                            "title": issue.title,
                            "body": issue.body,
                            "state": issue.state,
                            "user": {"login": issue.user},
                        }),
                    );
                }
            }
            if parts.len() == 4
                && parts[2] == "issues"
                && let Ok(n) = parts[3].parse::<u64>()
            {
                if req.method == "PATCH" {
                    let v: Value = serde_json::from_slice(&req.body).unwrap_or(json!({}));
                    let title = v.get("title").and_then(|x| x.as_str());
                    let body = v.get("body").and_then(|x| x.as_str());
                    let state = v.get("state").and_then(|x| x.as_str());
                    if let Some(i) = st.patch_issue(owner, repo, n, title, body, state) {
                        st.save()?;
                        return respond(
                            stream,
                            200,
                            &json!({
                                "number": i.number,
                                "title": i.title,
                                "body": i.body,
                                "state": i.state,
                                "user": {"login": i.user},
                            }),
                        );
                    }
                    return respond(stream, 404, &json!({"message":"Not Found"}));
                }
                if let Some(i) = st
                    .db
                    .issues
                    .iter()
                    .find(|i| i.owner == owner && i.repo == repo && i.number == n)
                {
                    return respond(
                        stream,
                        200,
                        &json!({
                            "number": i.number,
                            "title": i.title,
                            "body": i.body,
                            "state": i.state,
                            "user": {"login": i.user},
                        }),
                    );
                }
                return respond(stream, 404, &json!({"message":"Not Found"}));
            }
            // Issue/PR comments (GitHub shares /issues/{n}/comments for both).
            if parts.len() == 5
                && parts[2] == "issues"
                && parts[4] == "comments"
                && let Ok(n) = parts[3].parse::<u64>()
            {
                if req.method == "POST" {
                    let v: Value = serde_json::from_slice(&req.body).unwrap_or(json!({}));
                    let body = v.get("body").and_then(|x| x.as_str()).unwrap_or("");
                    let c = st.create_comment(owner, repo, n, body);
                    st.save()?;
                    return respond(
                        stream,
                        201,
                        &json!({
                            "id": c.id,
                            "body": c.body,
                            "user": {"login": c.user},
                            "issue_url": format!("/repos/{owner}/{repo}/issues/{n}"),
                        }),
                    );
                }
                if req.method == "GET" {
                    let items: Vec<Value> = st
                        .db
                        .comments
                        .iter()
                        .filter(|c| c.owner == owner && c.repo == repo && c.number == n)
                        .map(|c| {
                            json!({
                                "id": c.id,
                                "body": c.body,
                                "user": {"login": c.user},
                            })
                        })
                        .collect();
                    return respond(stream, 200, &Value::Array(items));
                }
            }
            if parts.len() == 3 && parts[2] == "pulls" {
                if req.method == "GET" {
                    let items: Vec<Value> = st
                        .db
                        .pulls
                        .iter()
                        .filter(|p| p.owner == owner && p.repo == repo)
                        .map(|p| {
                            json!({
                                "number": p.number,
                                "title": p.title,
                                "body": p.body,
                                "state": p.state,
                                "head": {"ref": p.head},
                                "base": {"ref": p.base},
                                "user": {"login": p.user},
                            })
                        })
                        .collect();
                    return respond(stream, 200, &Value::Array(items));
                }
                if req.method == "POST" {
                    let v: Value = serde_json::from_slice(&req.body).unwrap_or(json!({}));
                    let title = v.get("title").and_then(|x| x.as_str()).unwrap_or("pr");
                    let body = v.get("body").and_then(|x| x.as_str()).unwrap_or("");
                    let head = v.get("head").and_then(|x| x.as_str()).unwrap_or("head");
                    let base = v.get("base").and_then(|x| x.as_str()).unwrap_or("main");
                    let pull = st.create_pull(owner, repo, title, body, head, base);
                    st.save()?;
                    return respond(
                        stream,
                        201,
                        &json!({
                            "number": pull.number,
                            "title": pull.title,
                            "body": pull.body,
                            "state": pull.state,
                            "head": {"ref": pull.head},
                            "base": {"ref": pull.base},
                            "user": {"login": pull.user},
                        }),
                    );
                }
            }
            if parts.len() == 4
                && parts[2] == "pulls"
                && let Ok(n) = parts[3].parse::<u64>()
            {
                if req.method == "PATCH" {
                    let v: Value = serde_json::from_slice(&req.body).unwrap_or(json!({}));
                    let title = v.get("title").and_then(|x| x.as_str());
                    let body = v.get("body").and_then(|x| x.as_str());
                    let state = v.get("state").and_then(|x| x.as_str());
                    if let Some(p) = st.patch_pull(owner, repo, n, title, body, state) {
                        st.save()?;
                        return respond(
                            stream,
                            200,
                            &json!({
                                "number": p.number,
                                "title": p.title,
                                "body": p.body,
                                "state": p.state,
                                "merged": p.merged,
                                "head": {"ref": p.head},
                                "base": {"ref": p.base},
                                "user": {"login": p.user},
                            }),
                        );
                    }
                    return respond(stream, 404, &json!({"message":"Not Found"}));
                }
                if let Some(p) = st
                    .db
                    .pulls
                    .iter()
                    .find(|p| p.owner == owner && p.repo == repo && p.number == n)
                {
                    return respond(
                        stream,
                        200,
                        &json!({
                            "number": p.number,
                            "title": p.title,
                            "body": p.body,
                            "state": p.state,
                            "merged": p.merged,
                            "head": {"ref": p.head},
                            "base": {"ref": p.base},
                            "user": {"login": p.user},
                        }),
                    );
                }
                return respond(stream, 404, &json!({"message":"Not Found"}));
            }
            // Metadata merge only (≠ git merge; never spawn git).
            if parts.len() == 5
                && parts[2] == "pulls"
                && parts[4] == "merge"
                && let Ok(n) = parts[3].parse::<u64>()
                && req.method == "PUT"
            {
                if let Some(p) = st.merge_pull(owner, repo, n) {
                    st.save()?;
                    return respond(
                        stream,
                        200,
                        &json!({
                            "sha": format!("local-merge-{}", p.number),
                            "merged": true,
                            "message": "Pull Request successfully merged (local-forge metadata)",
                            "number": p.number,
                            "state": p.state,
                        }),
                    );
                }
                return respond(stream, 404, &json!({"message":"Not Found"}));
            }
            if parts.len() == 3 && parts[2] == "labels" {
                if req.method == "GET" {
                    let items: Vec<Value> = st
                        .db
                        .labels
                        .iter()
                        .filter(|l| l.owner == owner && l.repo == repo)
                        .map(|l| {
                            json!({
                                "id": l.id,
                                "name": l.name,
                                "color": l.color,
                                "description": l.description,
                            })
                        })
                        .collect();
                    return respond(stream, 200, &Value::Array(items));
                }
                if req.method == "POST" {
                    let v: Value = serde_json::from_slice(&req.body).unwrap_or(json!({}));
                    let name = v.get("name").and_then(|x| x.as_str()).unwrap_or("label");
                    let color = v.get("color").and_then(|x| x.as_str()).unwrap_or("ededed");
                    let desc = v.get("description").and_then(|x| x.as_str()).unwrap_or("");
                    let label = st.create_label(owner, repo, name, color, desc);
                    st.save()?;
                    return respond(
                        stream,
                        201,
                        &json!({
                            "id": label.id,
                            "name": label.name,
                            "color": label.color,
                            "description": label.description,
                        }),
                    );
                }
            }
            if parts.len() == 4 && parts[2] == "labels" {
                let name = parts[3];
                if req.method == "PATCH" {
                    let v: Value = serde_json::from_slice(&req.body).unwrap_or(json!({}));
                    let new_name = v.get("new_name").and_then(|x| x.as_str());
                    let color = v.get("color").and_then(|x| x.as_str());
                    let desc = v.get("description").and_then(|x| x.as_str());
                    if let Some(label) = st.patch_label(owner, repo, name, new_name, color, desc)
                    {
                        st.save()?;
                        return respond(
                            stream,
                            200,
                            &json!({
                                "id": label.id,
                                "name": label.name,
                                "color": label.color,
                                "description": label.description,
                            }),
                        );
                    }
                    return respond(stream, 404, &json!({"message":"Not Found"}));
                }
                if req.method == "DELETE" {
                    if st.delete_label(owner, repo, name) {
                        st.save()?;
                        return respond(stream, 204, &json!({}));
                    }
                    return respond(stream, 404, &json!({"message":"Not Found"}));
                }
            }
            // Releases (local forge; assets Claimed under IW-HUB-ASSETS).
            if parts.len() == 3 && parts[2] == "releases" {
                if req.method == "GET" {
                    let items: Vec<Value> = st
                        .db
                        .releases
                        .iter()
                        .filter(|r| r.owner == owner && r.repo == repo)
                        .map(|r| release_json(&st, r))
                        .collect();
                    return respond(stream, 200, &Value::Array(items));
                }
                if req.method == "POST" {
                    let v: Value = serde_json::from_slice(&req.body).unwrap_or(json!({}));
                    let tag = v
                        .get("tag_name")
                        .and_then(|x| x.as_str())
                        .unwrap_or("v0.0.0");
                    let name = v.get("name").and_then(|x| x.as_str()).unwrap_or("");
                    let body = v.get("body").and_then(|x| x.as_str()).unwrap_or("");
                    let rel = st.create_release(owner, repo, tag, name, body);
                    st.save()?;
                    return respond(stream, 201, &release_json(&st, &rel));
                }
            }
            // release view / edit / delete by id
            if parts.len() == 4 && parts[2] == "releases" {
                let key = parts[3];
                if req.method == "GET" {
                    if let Some(r) = st.find_release(owner, repo, key) {
                        return respond(stream, 200, &release_json(&st, r));
                    }
                    return respond(stream, 404, &json!({"message":"Not Found"}));
                }
                if req.method == "PATCH" {
                    let v: Value = serde_json::from_slice(&req.body).unwrap_or(json!({}));
                    let name = v.get("name").and_then(|x| x.as_str());
                    let body = v.get("body").and_then(|x| x.as_str());
                    let tag = v.get("tag_name").and_then(|x| x.as_str());
                    if let Some(r) = st.update_release(owner, repo, key, name, body, tag) {
                        st.save()?;
                        return respond(stream, 200, &release_json(&st, &r));
                    }
                    return respond(stream, 404, &json!({"message":"Not Found"}));
                }
                if req.method == "DELETE" {
                    if st.delete_release(owner, repo, key) {
                        st.save()?;
                        return respond(stream, 204, &json!({}));
                    }
                    return respond(stream, 404, &json!({"message":"Not Found"}));
                }
            }
            if parts.len() == 5
                && parts[2] == "releases"
                && parts[3] == "tags"
                && req.method == "GET"
            {
                if let Some(r) = st.find_release(owner, repo, parts[4]) {
                    return respond(stream, 200, &release_json(&st, r));
                }
                return respond(stream, 404, &json!({"message":"Not Found"}));
            }
            // POST /repos/{o}/{r}/releases/{id}/assets?name=
            if parts.len() == 5
                && parts[2] == "releases"
                && parts[4] == "assets"
                && req.method == "POST"
            {
                let release_id: u64 = parts[3].parse().unwrap_or(0);
                let name = query
                    .split('&')
                    .find_map(|pair| {
                        let (k, v) = pair.split_once('=')?;
                        (k == "name").then(|| v.to_string())
                    })
                    .unwrap_or_else(|| "asset.bin".into());
                let ctype = header_value(&req.headers, "content-type")
                    .unwrap_or_else(|| "application/octet-stream".into());
                if let Some(a) =
                    st.upload_release_asset(owner, repo, release_id, &name, &ctype, &req.body)
                {
                    st.save()?;
                    return respond(
                        stream,
                        201,
                        &json!({
                            "id": a.id,
                            "name": a.name,
                            "size": a.size,
                            "content_type": a.content_type,
                            "state": "uploaded",
                        }),
                    );
                }
                return respond(stream, 404, &json!({"message":"Not Found"}));
            }
            // GET/DELETE /repos/{o}/{r}/releases/assets/{id}
            if parts.len() == 5
                && parts[2] == "releases"
                && parts[3] == "assets"
            {
                let asset_id: u64 = parts[4].parse().unwrap_or(0);
                if req.method == "GET" {
                    if let Some(a) = st.find_asset(owner, repo, asset_id) {
                        let accept = header_value(&req.headers, "accept").unwrap_or_default();
                        if accept.contains("application/octet-stream") {
                            if let Some(bytes) = st.read_asset_bytes(a.id) {
                                return write_http_response(
                                    stream,
                                    200,
                                    "OK",
                                    &a.content_type,
                                    &bytes,
                                );
                            }
                        }
                        return respond(
                            stream,
                            200,
                            &json!({
                                "id": a.id,
                                "name": a.name,
                                "size": a.size,
                                "content_type": a.content_type,
                                "state": "uploaded",
                            }),
                        );
                    }
                    return respond(stream, 404, &json!({"message":"Not Found"}));
                }
                if req.method == "DELETE" {
                    if st.delete_asset(owner, repo, asset_id) {
                        st.save()?;
                        return respond(stream, 204, &json!({}));
                    }
                    return respond(stream, 404, &json!({"message":"Not Found"}));
                }
            }
            // Actions workflows / runs (local-forge metadata — not CI compute Claimed).
            if parts.len() == 4 && parts[2] == "actions" && parts[3] == "workflows" {
                if req.method == "GET" {
                    st.ensure_default_workflow(owner, repo);
                    st.save()?;
                    let items: Vec<Value> = st
                        .db
                        .workflows
                        .iter()
                        .filter(|w| w.owner == owner && w.repo == repo)
                        .map(|w| {
                            json!({
                                "id": w.id,
                                "name": w.name,
                                "path": w.path,
                                "state": w.state,
                            })
                        })
                        .collect();
                    return respond(
                        stream,
                        200,
                        &json!({"total_count": items.len(), "workflows": items}),
                    );
                }
            }
            // GET/PUT workflow by id|path; POST dispatches
            if parts.len() == 5 && parts[2] == "actions" && parts[3] == "workflows" {
                let key = parts[4];
                if req.method == "GET" {
                    st.ensure_default_workflow(owner, repo);
                    st.save()?;
                    if let Some(w) = st.find_workflow(owner, repo, key) {
                        return respond(
                            stream,
                            200,
                            &json!({
                                "id": w.id,
                                "name": w.name,
                                "path": w.path,
                                "state": w.state,
                            }),
                        );
                    }
                    return respond(stream, 404, &json!({"message":"Not Found"}));
                }
                if req.method == "PUT" && (key == "enable" || key.ends_with("/enable")) {
                    // unreachable shape; handled below
                }
            }
            if parts.len() == 6
                && parts[2] == "actions"
                && parts[3] == "workflows"
                && parts[5] == "enable"
                && req.method == "PUT"
            {
                st.ensure_default_workflow(owner, repo);
                if let Some(_w) = st.set_workflow_state(owner, repo, parts[4], "active") {
                    st.save()?;
                    return respond(stream, 204, &json!({}));
                }
                return respond(stream, 404, &json!({"message":"Not Found"}));
            }
            if parts.len() == 6
                && parts[2] == "actions"
                && parts[3] == "workflows"
                && parts[5] == "disable"
                && req.method == "PUT"
            {
                st.ensure_default_workflow(owner, repo);
                if let Some(_w) = st.set_workflow_state(owner, repo, parts[4], "disabled_manually") {
                    st.save()?;
                    return respond(stream, 204, &json!({}));
                }
                return respond(stream, 404, &json!({"message":"Not Found"}));
            }
            if parts.len() == 6
                && parts[2] == "actions"
                && parts[3] == "workflows"
                && parts[5] == "dispatches"
                && req.method == "POST"
            {
                st.ensure_default_workflow(owner, repo);
                let v: Value = serde_json::from_slice(&req.body).unwrap_or(json!({}));
                let rref = v.get("ref").and_then(|x| x.as_str()).unwrap_or("main");
                if let Some(run) = st.dispatch_workflow(owner, repo, parts[4], rref) {
                    st.save()?;
                    return respond(
                        stream,
                        204,
                        &json!({
                            "id": run.id,
                            "status": run.status,
                            "head_branch": run.head_branch,
                        }),
                    );
                }
                return respond(stream, 422, &json!({"message":"workflow not dispatchable"}));
            }
            if parts.len() == 4 && parts[2] == "actions" && parts[3] == "runs" {
                if req.method == "GET" {
                    let items: Vec<Value> = st
                        .db
                        .runs
                        .iter()
                        .filter(|r| r.owner == owner && r.repo == repo)
                        .map(|r| run_json(r))
                        .collect();
                    return respond(
                        stream,
                        200,
                        &json!({"total_count": items.len(), "workflow_runs": items}),
                    );
                }
                if req.method == "POST" {
                    let v: Value = serde_json::from_slice(&req.body).unwrap_or(json!({}));
                    let name = v.get("name").and_then(|x| x.as_str()).unwrap_or("manual");
                    let branch = v
                        .get("head_branch")
                        .and_then(|x| x.as_str())
                        .unwrap_or("main");
                    let run = st.create_run(owner, repo, name, branch);
                    st.save()?;
                    return respond(stream, 201, &run_json(&run));
                }
            }
            if parts.len() == 5 && parts[2] == "actions" && parts[3] == "runs" {
                let run_id: u64 = parts[4].parse().unwrap_or(0);
                if req.method == "GET" {
                    if let Some(r) = st.find_run(owner, repo, run_id) {
                        return respond(stream, 200, &run_json(r));
                    }
                    return respond(stream, 404, &json!({"message":"Not Found"}));
                }
                if req.method == "DELETE" {
                    if st.delete_run(owner, repo, run_id) {
                        st.save()?;
                        return respond(stream, 204, &json!({}));
                    }
                    return respond(stream, 404, &json!({"message":"Not Found"}));
                }
            }
            if parts.len() == 6 && parts[2] == "actions" && parts[3] == "runs" {
                let run_id: u64 = parts[4].parse().unwrap_or(0);
                let action = parts[5];
                if action == "cancel" && req.method == "POST" {
                    if let Some(r) = st.cancel_run(owner, repo, run_id) {
                        st.save()?;
                        return respond(stream, 202, &run_json(&r));
                    }
                    return respond(stream, 404, &json!({"message":"Not Found"}));
                }
                if action == "rerun" && req.method == "POST" {
                    if let Some(r) = st.rerun_run(owner, repo, run_id) {
                        st.save()?;
                        return respond(stream, 201, &run_json(&r));
                    }
                    return respond(stream, 404, &json!({"message":"Not Found"}));
                }
                if action == "artifacts" && req.method == "GET" {
                    let items: Vec<Value> = st
                        .db
                        .artifacts
                        .iter()
                        .filter(|a| a.owner == owner && a.repo == repo && a.run_id == run_id)
                        .map(|a| {
                            json!({
                                "id": a.id,
                                "name": a.name,
                                "size_in_bytes": a.size,
                            })
                        })
                        .collect();
                    return respond(
                        stream,
                        200,
                        &json!({"total_count": items.len(), "artifacts": items}),
                    );
                }
            }
            // GET /repos/{o}/{r}/actions/artifacts/{id}/zip
            if parts.len() == 6
                && parts[2] == "actions"
                && parts[3] == "artifacts"
                && parts[5] == "zip"
                && req.method == "GET"
            {
                let aid: u64 = parts[4].parse().unwrap_or(0);
                if let Some(bytes) = st.read_run_artifact_bytes(aid) {
                    return write_http_response(stream, 200, "OK", "application/zip", &bytes);
                }
                return respond(stream, 404, &json!({"message":"Not Found"}));
            }
            // Milestones / projects / secrets / variables / discussions (Partial local forge).
            if parts.len() == 3 && parts[2] == "milestones" {
                if req.method == "GET" {
                    let items: Vec<Value> = st
                        .db
                        .milestones
                        .iter()
                        .filter(|m| m.owner == owner && m.repo == repo)
                        .map(|m| {
                            json!({
                                "number": m.number,
                                "title": m.title,
                                "description": m.description,
                                "state": m.state,
                            })
                        })
                        .collect();
                    return respond(stream, 200, &Value::Array(items));
                }
                if req.method == "POST" {
                    let v: Value = serde_json::from_slice(&req.body).unwrap_or(json!({}));
                    let title = v
                        .get("title")
                        .and_then(|x| x.as_str())
                        .unwrap_or("milestone");
                    let desc = v.get("description").and_then(|x| x.as_str()).unwrap_or("");
                    let m = st.create_milestone(owner, repo, title, desc);
                    st.save()?;
                    return respond(
                        stream,
                        201,
                        &json!({
                            "number": m.number,
                            "title": m.title,
                            "description": m.description,
                            "state": m.state,
                        }),
                    );
                }
            }
            if parts.len() == 3 && parts[2] == "projects" {
                if req.method == "GET" {
                    let items: Vec<Value> = st
                        .db
                        .projects
                        .iter()
                        .filter(|p| p.owner == owner && p.repo == repo)
                        .map(|p| {
                            json!({
                                "id": p.id,
                                "title": p.title,
                                "body": p.body,
                                "state": p.state,
                            })
                        })
                        .collect();
                    return respond(stream, 200, &Value::Array(items));
                }
                if req.method == "POST" {
                    let v: Value = serde_json::from_slice(&req.body).unwrap_or(json!({}));
                    let title = v.get("title").and_then(|x| x.as_str()).unwrap_or("project");
                    let body = v.get("body").and_then(|x| x.as_str()).unwrap_or("");
                    let p = st.create_project(owner, repo, title, body);
                    st.save()?;
                    return respond(
                        stream,
                        201,
                        &json!({
                            "id": p.id,
                            "title": p.title,
                            "body": p.body,
                            "state": p.state,
                        }),
                    );
                }
            }
            if parts.len() == 4 && parts[2] == "actions" && parts[3] == "secrets" {
                if req.method == "GET" {
                    let items: Vec<Value> = st
                        .db
                        .secrets
                        .iter()
                        .filter(|s| s.owner == owner && s.repo == repo)
                        .map(|s| json!({"id": s.id, "name": s.name}))
                        .collect();
                    return respond(
                        stream,
                        200,
                        &json!({"total_count": items.len(), "secrets": items}),
                    );
                }
                if req.method == "POST" {
                    let v: Value = serde_json::from_slice(&req.body).unwrap_or(json!({}));
                    let name = v.get("name").and_then(|x| x.as_str()).unwrap_or("SECRET");
                    // Values are never stored — metadata only (Partial).
                    let s = st.create_secret_meta(owner, repo, name);
                    st.save()?;
                    return respond(stream, 201, &json!({"id": s.id, "name": s.name}));
                }
            }
            if parts.len() == 4 && parts[2] == "actions" && parts[3] == "variables" {
                if req.method == "GET" {
                    let items: Vec<Value> = st
                        .db
                        .variables
                        .iter()
                        .filter(|x| x.owner == owner && x.repo == repo)
                        .map(|x| {
                            json!({
                                "id": x.id,
                                "name": x.name,
                                "value": x.value,
                            })
                        })
                        .collect();
                    return respond(
                        stream,
                        200,
                        &json!({"total_count": items.len(), "variables": items}),
                    );
                }
                if req.method == "POST" {
                    let v: Value = serde_json::from_slice(&req.body).unwrap_or(json!({}));
                    let name = v.get("name").and_then(|x| x.as_str()).unwrap_or("VAR");
                    let value = v.get("value").and_then(|x| x.as_str()).unwrap_or("");
                    let var = st.create_variable(owner, repo, name, value);
                    st.save()?;
                    return respond(
                        stream,
                        201,
                        &json!({
                            "id": var.id,
                            "name": var.name,
                            "value": var.value,
                        }),
                    );
                }
            }
            if parts.len() == 3 && parts[2] == "discussions" {
                if req.method == "GET" {
                    let items: Vec<Value> = st
                        .db
                        .discussions
                        .iter()
                        .filter(|d| d.owner == owner && d.repo == repo)
                        .map(|d| {
                            json!({
                                "number": d.number,
                                "title": d.title,
                                "body": d.body,
                                "category": d.category,
                            })
                        })
                        .collect();
                    return respond(stream, 200, &Value::Array(items));
                }
                if req.method == "POST" {
                    let v: Value = serde_json::from_slice(&req.body).unwrap_or(json!({}));
                    let title = v
                        .get("title")
                        .and_then(|x| x.as_str())
                        .unwrap_or("discussion");
                    let body = v.get("body").and_then(|x| x.as_str()).unwrap_or("");
                    let cat = v
                        .get("category")
                        .and_then(|x| x.as_str())
                        .unwrap_or("General");
                    let d = st.create_discussion(owner, repo, title, body, cat);
                    st.save()?;
                    return respond(
                        stream,
                        201,
                        &json!({
                            "number": d.number,
                            "title": d.title,
                            "body": d.body,
                            "category": d.category,
                        }),
                    );
                }
            }
        }
    }

    // Gists / user keys (Partial local forge).
    if path == "/gists" {
        if req.method == "GET" {
            let items: Vec<Value> = st
                .db
                .gists
                .iter()
                .map(|g| {
                    json!({
                        "id": g.id,
                        "description": g.description,
                        "public": g.public,
                        "files": g.files,
                    })
                })
                .collect();
            return respond(stream, 200, &Value::Array(items));
        }
        if req.method == "POST" {
            let v: Value = serde_json::from_slice(&req.body).unwrap_or(json!({}));
            let desc = v.get("description").and_then(|x| x.as_str()).unwrap_or("");
            let public = v.get("public").and_then(|x| x.as_bool()).unwrap_or(true);
            let files = v
                .get("files")
                .and_then(|x| x.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|f| f.as_str().map(str::to_string))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let g = st.create_gist(desc, public, files);
            st.save()?;
            return respond(
                stream,
                201,
                &json!({
                    "id": g.id,
                    "description": g.description,
                    "public": g.public,
                    "files": g.files,
                }),
            );
        }
    }
    if path == "/user/keys" {
        if req.method == "GET" {
            let items: Vec<Value> = st
                .db
                .ssh_keys
                .iter()
                .map(|k| json!({"id": k.id, "title": k.title, "key": k.key}))
                .collect();
            return respond(stream, 200, &Value::Array(items));
        }
        if req.method == "POST" {
            let v: Value = serde_json::from_slice(&req.body).unwrap_or(json!({}));
            let title = v.get("title").and_then(|x| x.as_str()).unwrap_or("key");
            let key = v.get("key").and_then(|x| x.as_str()).unwrap_or("");
            let k = st.create_ssh_key(title, key);
            st.save()?;
            return respond(
                stream,
                201,
                &json!({"id": k.id, "title": k.title, "key": k.key}),
            );
        }
    }
    if path == "/user/gpg_keys" {
        if req.method == "GET" {
            let items: Vec<Value> = st
                .db
                .gpg_keys
                .iter()
                .map(|k| json!({"id": k.id, "name": k.name, "key_id": k.key_id}))
                .collect();
            return respond(stream, 200, &Value::Array(items));
        }
        if req.method == "POST" {
            let v: Value = serde_json::from_slice(&req.body).unwrap_or(json!({}));
            let name = v.get("name").and_then(|x| x.as_str()).unwrap_or("gpg");
            let key_id = v
                .get("key_id")
                .and_then(|x| x.as_str())
                .unwrap_or("00000000");
            let k = st.create_gpg_key(name, key_id);
            st.save()?;
            return respond(
                stream,
                201,
                &json!({"id": k.id, "name": k.name, "key_id": k.key_id}),
            );
        }
    }

    // GET /orgs — unique owners from local repos (Partial).
    if req.method == "GET" && path == "/orgs" {
        let mut owners: Vec<String> = st.db.repos.iter().map(|r| r.owner.clone()).collect();
        owners.sort();
        owners.dedup();
        let items: Vec<Value> = owners
            .into_iter()
            .map(|login| json!({"login": login, "type": "Organization"}))
            .collect();
        return respond(stream, 200, &Value::Array(items));
    }

    // GET /search/{repos|issues|pulls}?q=… — local substring filter (Partial).
    if req.method == "GET" && path.starts_with("/search/") {
        let kind = path.trim_start_matches("/search/");
        let q = query_q.to_ascii_lowercase();
        let items = match kind {
            "repos" => {
                let arr: Vec<Value> = st
                    .db
                    .repos
                    .iter()
                    .filter(|r| {
                        q.is_empty()
                            || r.name.to_ascii_lowercase().contains(&q)
                            || r.description.to_ascii_lowercase().contains(&q)
                            || r.owner.to_ascii_lowercase().contains(&q)
                    })
                    .map(|r| {
                        json!({
                            "id": r.id,
                            "name": r.name,
                            "full_name": format!("{}/{}", r.owner, r.name),
                            "description": r.description,
                        })
                    })
                    .collect();
                json!({"total_count": arr.len(), "items": arr})
            }
            "issues" => {
                let arr: Vec<Value> = st
                    .db
                    .issues
                    .iter()
                    .filter(|i| {
                        q.is_empty()
                            || i.title.to_ascii_lowercase().contains(&q)
                            || i.body.to_ascii_lowercase().contains(&q)
                    })
                    .map(|i| {
                        json!({
                            "number": i.number,
                            "title": i.title,
                            "state": i.state,
                            "repository_url": format!("/repos/{}/{}", i.owner, i.repo),
                        })
                    })
                    .collect();
                json!({"total_count": arr.len(), "items": arr})
            }
            "pulls" | "prs" => {
                let arr: Vec<Value> = st
                    .db
                    .pulls
                    .iter()
                    .filter(|p| {
                        q.is_empty()
                            || p.title.to_ascii_lowercase().contains(&q)
                            || p.body.to_ascii_lowercase().contains(&q)
                    })
                    .map(|p| {
                        json!({
                            "number": p.number,
                            "title": p.title,
                            "state": p.state,
                            "repository_url": format!("/repos/{}/{}", p.owner, p.repo),
                        })
                    })
                    .collect();
                json!({"total_count": arr.len(), "items": arr})
            }
            other => {
                return respond(
                    stream,
                    404,
                    &json!({"message": format!("search kind `{other}` not claimed (Partial)")}),
                );
            }
        };
        return respond(stream, 200, &items);
    }

    respond(
        stream,
        404,
        &json!({"message":"Not Found","path": path, "method": req.method}),
    )
}

/// Minimal percent-decode for `q=` (Partial; spaces via `+` or `%20`).
fn urlencoding_lite_decode(s: &str) -> String {
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

fn run_json(r: &crate::store::WorkflowRun) -> Value {
    json!({
        "id": r.id,
        "name": r.name,
        "status": r.status,
        "conclusion": r.conclusion,
        "workflow_id": r.workflow_id,
        "head_branch": r.head_branch,
    })
}

fn release_json(st: &HubStore, r: &crate::store::Release) -> Value {
    let upload_url = format!(
        "/repos/{}/{}/releases/{}/assets{{?name,label}}",
        r.owner, r.repo, r.id
    );
    let assets: Vec<Value> = st
        .db
        .assets
        .iter()
        .filter(|a| a.release_id == r.id)
        .map(|a| {
            json!({
                "id": a.id,
                "name": a.name,
                "size": a.size,
                "content_type": a.content_type,
                "state": "uploaded",
            })
        })
        .collect();
    json!({
        "id": r.id,
        "tag_name": r.tag_name,
        "name": r.name,
        "body": r.body,
        "draft": r.draft,
        "prerelease": r.prerelease,
        "upload_url": upload_url,
        "assets": assets,
    })
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
        422 => "Unprocessable Entity",
        _ => "Error",
    };
    let bytes = serde_json::to_vec(body).map_err(|e| e.to_string())?;
    write_http_response(stream, status, reason, "application/json", &bytes)
}
