//! Local forge API host (Partial; not Claimed).
//!
//! Used by transitional `arks hub serve` and by the forge module under
//! `arks serve` ([[sot:root/arks-self-host-server]]). Default bind:
//! `127.0.0.1:3920`. Never spawns git/gh. No TLS Claimed.

use crate::http_min::{bearer_token, read_http_request, write_http_response};
use crate::store::{HubStore, now_secs};
use serde_json::{Value, json};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Options for the in-process local forge API listener (Partial).
#[derive(Debug, Clone)]
pub struct ForgeServeOptions {
    pub listen: SocketAddr,
    pub data_dir: PathBuf,
}

impl Default for ForgeServeOptions {
    fn default() -> Self {
        Self {
            listen: "127.0.0.1:3920"
                .parse()
                .expect("default forge listen parses"),
            data_dir: HubStore::data_dir(),
        }
    }
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
    let listener = TcpListener::bind(opts.listen).map_err(|e| e.to_string())?;
    listener
        .set_nonblocking(true)
        .map_err(|e| e.to_string())?;
    eprintln!(
        "arks forge API listening on http://{}/ (data={}; Partial local API — not Claimed)",
        listener.local_addr().map_err(|e| e.to_string())?,
        opts.data_dir.display()
    );
    while !stop.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((stream, _)) => {
                let store = store.clone();
                thread::spawn(move || {
                    if let Err(e) = handle_client(stream, &store) {
                        eprintln!("arks forge API: {e}");
                    }
                });
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(10));
            }
            Err(e) => return Err(e.to_string()),
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

fn handle_client(mut stream: TcpStream, store: &Arc<Mutex<HubStore>>) -> Result<(), String> {
    let _ = stream.set_nonblocking(false);
    let req = read_http_request(&mut stream)?;
    let token = bearer_token(&req.headers);
    let (path, query) = req
        .path
        .split_once('?')
        .map(|(p, q)| (p.to_string(), q.to_string()))
        .unwrap_or((req.path.clone(), String::new()));
    let _ = query;
    let _ = now_secs;

    let mut st = store.lock().map_err(|_| "store lock poisoned".to_string())?;
    // Health is unauthenticated.
    if req.method == "GET" && (path == "/" || path == "/health") {
        let body = json!({"ok":true,"service":"arks-hub","partial":true});
        return respond(&mut stream, 200, &body);
    }

    if !st.auth_ok(token.as_deref()) && path != "/login" {
        let body =
            json!({"message":"Requires authentication","documentation_url":"arks hub auth login"});
        return respond(&mut stream, 401, &body);
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
            &mut stream,
            200,
            &json!({"token": tok, "login": st.db.user_login}),
        );
    }

    if req.method == "GET" && path == "/user" {
        return respond(
            &mut stream,
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
        return respond(&mut stream, 200, &Value::Array(repos));
    }

    if req.method == "POST" && path == "/user/repos" {
        let v: Value = serde_json::from_slice(&req.body).unwrap_or(json!({}));
        let name = v.get("name").and_then(|x| x.as_str()).unwrap_or("repo");
        let desc = v.get("description").and_then(|x| x.as_str()).unwrap_or("");
        let owner = st.db.user_login.clone();
        let repo = st.create_repo(&owner, name, desc);
        st.save()?;
        return respond(
            &mut stream,
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
                if let Some(r) = st.db.repos.iter().find(|r| r.owner == owner && r.name == repo) {
                    return respond(
                        &mut stream,
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
                return respond(&mut stream, 404, &json!({"message":"Not Found"}));
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
                    return respond(&mut stream, 200, &Value::Array(items));
                }
                if req.method == "POST" {
                    let v: Value = serde_json::from_slice(&req.body).unwrap_or(json!({}));
                    let title = v.get("title").and_then(|x| x.as_str()).unwrap_or("issue");
                    let body = v.get("body").and_then(|x| x.as_str()).unwrap_or("");
                    let issue = st.create_issue(owner, repo, title, body);
                    st.save()?;
                    return respond(
                        &mut stream,
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
            if parts.len() == 4 && parts[2] == "issues"
                && let Ok(n) = parts[3].parse::<u64>()
            {
                if let Some(i) = st
                    .db
                    .issues
                    .iter()
                    .find(|i| i.owner == owner && i.repo == repo && i.number == n)
                {
                    return respond(
                        &mut stream,
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
                return respond(&mut stream, 404, &json!({"message":"Not Found"}));
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
                    return respond(&mut stream, 200, &Value::Array(items));
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
                        &mut stream,
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
            if parts.len() == 4 && parts[2] == "pulls"
                && let Ok(n) = parts[3].parse::<u64>()
            {
                if let Some(p) = st
                    .db
                    .pulls
                    .iter()
                    .find(|p| p.owner == owner && p.repo == repo && p.number == n)
                {
                    return respond(
                        &mut stream,
                        200,
                        &json!({
                            "number": p.number,
                            "title": p.title,
                            "body": p.body,
                            "state": p.state,
                            "head": {"ref": p.head},
                            "base": {"ref": p.base},
                            "user": {"login": p.user},
                        }),
                    );
                }
                return respond(&mut stream, 404, &json!({"message":"Not Found"}));
            }
        }
    }

    respond(
        &mut stream,
        404,
        &json!({"message":"Not Found","path": path, "method": req.method}),
    )
}

fn respond(stream: &mut TcpStream, status: u16, body: &serde_json::Value) -> Result<(), String> {
    let reason = match status {
        200 => "OK",
        201 => "Created",
        401 => "Unauthorized",
        404 => "Not Found",
        _ => "Error",
    };
    let bytes = serde_json::to_vec(body).map_err(|e| e.to_string())?;
    write_http_response(stream, status, reason, "application/json", &bytes)
}
