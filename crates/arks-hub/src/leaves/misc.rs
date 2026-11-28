//! gh-symmetric top-level leaves: `auth`, `status`, `browse`, `org`, `api`,
//! `config`, `alias`, `completion`.
//!
//! Accounts: `auth login|logout|switch|status|token` keep a hosts file
//! (`~/.config/arks/hub-hosts.json`, or `ARKS_HUB_HOSTS_FILE`) with one or more
//! accounts per forge host; the active account's token is also written to the
//! token file the other leaves read. arks never launches a browser and never
//! spawns git or gh.

use super::common::{enc, parse_number};
use super::{Args, Ctx, Fail, Res, arr, emit, n, s, usage, web};
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::PathBuf;

pub(crate) const AUTH_HELP: &str = "\
arks auth — accounts on arks forge hosts (gh-symmetric argv)

USAGE:
  arks auth login [--with-token | --web] [--client-id <id>] [-h <host>] [-p <protocol>] [-s <scopes>]
  arks auth logout [-h <host>] [-u <user>]
  arks auth status [-a] [-h <host>] [-t] [--json hosts]
  arks auth switch [-h <host>] [-u <user>]
  arks auth token [-h <host>] [-u <user>]
  arks auth refresh [-h <host>] [-s <scopes>] [-r <scopes>] [--reset-scopes]

Token sources: standard input (--with-token), ARKS_HUB_TOKEN, or RFC 8628
OAuth 2.0 Device Authorization Grant (--web with ARKS_OAUTH_CLIENT_ID / --client-id).
arks never spawns a browser, git, or gh.";

/// One account on one host.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
struct Account {
    token: String,
    #[serde(default)]
    git_protocol: String,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
struct Host {
    #[serde(default)]
    active: String,
    #[serde(default)]
    accounts: BTreeMap<String, Account>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
struct Hosts {
    #[serde(default)]
    hosts: BTreeMap<String, Host>,
}

fn hosts_path() -> PathBuf {
    if let Ok(p) = std::env::var("ARKS_HUB_HOSTS_FILE") {
        return PathBuf::from(p);
    }
    crate::token_path()
        .parent()
        .map(|p| p.join("hub-hosts.json"))
        .unwrap_or_else(|| PathBuf::from("hub-hosts.json"))
}

fn load_hosts() -> Hosts {
    std::fs::read_to_string(hosts_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_hosts(h: &Hosts) -> Res {
    let p = hosts_path();
    if let Some(d) = p.parent() {
        std::fs::create_dir_all(d)?;
    }
    std::fs::write(&p, serde_json::to_vec_pretty(h).unwrap_or_default())?;
    Ok(())
}

/// Write (or clear) the active token file the other leaves read.
fn write_active(token: Option<&str>) -> Res {
    let p = crate::token_path();
    match token {
        Some(t) => {
            if let Some(d) = p.parent() {
                std::fs::create_dir_all(d)?;
            }
            std::fs::write(&p, format!("{t}\n"))?;
        }
        None => {
            let _ = std::fs::remove_file(&p);
        }
    }
    Ok(())
}

fn host_of(base: &str) -> String {
    base.split_once("://")
        .map_or(base, |(_, r)| r)
        .trim_end_matches('/')
        .split('/')
        .next()
        .unwrap_or("")
        .to_string()
}

fn mask(t: &str) -> String {
    let keep: String = t.chars().take(4).collect();
    format!(
        "{keep}{}",
        "*".repeat(t.chars().count().saturating_sub(4).min(36))
    )
}

pub(crate) fn auth(ctx: &Ctx, args: &[String], out: &mut dyn Write) -> Res {
    let sub = args.first().map(String::as_str).unwrap_or("");
    let raw = args.get(1..).unwrap_or(&[]);
    let cur_host = host_of(&ctx.base);
    match sub {
        "login" => {
            let a = Args::parse(
                raw,
                &[
                    "-p",
                    "--git-protocol",
                    "-h",
                    "--hostname",
                    "-s",
                    "--scopes",
                    "--client-id",
                ],
                &[
                    "-c",
                    "--clipboard",
                    "--insecure-storage",
                    "--skip-ssh-key",
                    "-w",
                    "--web",
                    "--with-token",
                ],
            )?;
            if a.has(&["-w", "--web"]) && a.has(&["--with-token"]) {
                return usage("cannot combine --web and --with-token");
            }
            if let Some(p) = a.get(&["-p", "--git-protocol"]) {
                if !matches!(p, "ssh" | "https") {
                    return usage(format!(
                        "invalid argument {p:?} for \"-p, --git-protocol\" flag: valid values are {{ssh|https}}"
                    ));
                }
            }
            let host = a
                .get(&["-h", "--hostname"])
                .map(str::to_string)
                .unwrap_or(cur_host.clone());
            let token = if a.has(&["-w", "--web"]) {
                let client_id = a
                    .get(&["--client-id"])
                    .map(str::to_string)
                    .or_else(|| {
                        std::env::var("ARKS_OAUTH_CLIENT_ID")
                            .ok()
                            .filter(|v| !v.trim().is_empty())
                    })
                    .ok_or_else(|| {
                        Fail::Usage(
                            "OAuth device login (--web) requires --client-id <id> or ARKS_OAUTH_CLIENT_ID (RFC 8628)".into(),
                        )
                    })?;
                let scope = a
                    .get(&["-s", "--scopes"])
                    .unwrap_or("repo read:org")
                    .to_string();
                let cfg = crate::device_flow::DeviceAuthConfig::for_host(&host, client_id, scope);
                let mut err_stream = std::io::stderr();
                let issued = crate::device_flow::run_device_flow(&cfg, &mut err_stream)
                    .map_err(Fail::Api)?;
                issued.access_token
            } else if a.has(&["--with-token"]) {
                if a.get(&["-s", "--scopes"]).is_some() {
                    eprintln!(
                        "note: arks forge tokens carry no OAuth scopes; --scopes changes nothing"
                    );
                }
                let mut buf = String::new();
                std::io::stdin().read_to_string(&mut buf)?;
                let t = buf.trim().to_string();
                if t.is_empty() {
                    return usage("expected a token on standard input with --with-token");
                }
                t
            } else if let Ok(t) = std::env::var("ARKS_HUB_TOKEN") {
                t
            } else {
                "local-dev-token".into()
            };
            let mut login = String::new();
            if host == cur_host {
                let mut body = json!({"token": token});
                if let Ok(l) = std::env::var("ARKS_HUB_LOGIN") {
                    body["login"] = json!(l);
                }
                match crate::http_min::http_exchange(
                    &ctx.base,
                    "POST",
                    "/login",
                    Some(&token),
                    Some(&body.to_string()),
                ) {
                    Ok((code, resp)) if code < 400 => {
                        let v: Value = serde_json::from_str(&resp).unwrap_or(Value::Null);
                        login = s(&v, "login").to_string();
                    }
                    Ok((code, resp)) => {
                        return Err(Fail::Api(format!(
                            "token rejected by {host} (HTTP {code}): {resp}"
                        )));
                    }
                    Err(e) => eprintln!(
                        "note: token stored; {host} did not answer ({e}) — start `arks serve`?"
                    ),
                }
            }
            if login.is_empty() {
                login = std::env::var("ARKS_HUB_LOGIN").unwrap_or_else(|_| "local".into());
            }
            let mut h = load_hosts();
            let entry = h.hosts.entry(host.clone()).or_default();
            entry.accounts.insert(
                login.clone(),
                Account {
                    token: token.clone(),
                    git_protocol: a.get(&["-p", "--git-protocol"]).unwrap_or("https").into(),
                },
            );
            entry.active = login.clone();
            save_hosts(&h)?;
            if host == cur_host {
                write_active(Some(&token))?;
            }
            if a.has(&["--insecure-storage"]) {
                eprintln!(
                    "note: arks always stores tokens in plain text at {}",
                    crate::token_path().display()
                );
            }
            eprintln!("✓ Logged in to {host} account {login}");
            eprintln!(
                "arks hub auth login: token stored at {}",
                crate::token_path().display()
            );
            let _ = out;
            Ok(())
        }
        "logout" => {
            let a = Args::parse(raw, &["-h", "--hostname", "-u", "--user"], &[])?;
            let host = a
                .get(&["-h", "--hostname"])
                .map(str::to_string)
                .unwrap_or(cur_host.clone());
            let mut h = load_hosts();
            let Some(entry) = h.hosts.get_mut(&host) else {
                // Legacy single-token state.
                write_active(None)?;
                eprintln!("✓ Logged out of {host}");
                return Ok(());
            };
            let user = a
                .get(&["-u", "--user"])
                .map(str::to_string)
                .unwrap_or(entry.active.clone());
            if entry.accounts.remove(&user).is_none() {
                return Err(Fail::Api(format!("not logged in to {host} account {user}")));
            }
            let next = entry
                .accounts
                .iter()
                .next()
                .map(|(l, a)| (l.clone(), a.token.clone()));
            if entry.active == user {
                entry.active = next.as_ref().map(|n| n.0.clone()).unwrap_or_default();
                if host == cur_host {
                    write_active(next.as_ref().map(|n| n.1.as_str()))?;
                }
            }
            if entry.accounts.is_empty() {
                h.hosts.remove(&host);
            }
            save_hosts(&h)?;
            eprintln!("✓ Logged out of {host} account {user}");
            if let Some((l, _)) = next {
                eprintln!("✓ Switched active account for {host} to {l}");
            }
            let _ = out;
            Ok(())
        }
        "switch" => {
            let a = Args::parse(raw, &["-h", "--hostname", "-u", "--user"], &[])?;
            let host = a
                .get(&["-h", "--hostname"])
                .map(str::to_string)
                .unwrap_or(cur_host.clone());
            let mut h = load_hosts();
            let entry = h
                .hosts
                .get_mut(&host)
                .ok_or_else(|| Fail::Api(format!("not logged in to {host}")))?;
            let target = match a.get(&["-u", "--user"]) {
                Some(u) => u.to_string(),
                None => {
                    let others: Vec<&String> = entry
                        .accounts
                        .keys()
                        .filter(|k| **k != entry.active)
                        .collect();
                    match others.as_slice() {
                        [one] => (*one).clone(),
                        [] => {
                            return Err(Fail::Api(format!("there is only one account on {host}")));
                        }
                        _ => {
                            return usage(
                                "more than one other account: pass --user when not running interactively",
                            );
                        }
                    }
                }
            };
            let acct =
                entry.accounts.get(&target).cloned().ok_or_else(|| {
                    Fail::Api(format!("not logged in to {host} account {target}"))
                })?;
            entry.active = target.clone();
            save_hosts(&h)?;
            if host == cur_host {
                write_active(Some(&acct.token))?;
            }
            eprintln!("✓ Switched active account for {host} to {target}");
            let _ = out;
            Ok(())
        }
        "token" => {
            let a = Args::parse(raw, &["-h", "--hostname", "-u", "--user"], &["--show"])?;
            let host = a
                .get(&["-h", "--hostname"])
                .map(str::to_string)
                .unwrap_or(cur_host.clone());
            let h = load_hosts();
            let tok = match (h.hosts.get(&host), a.get(&["-u", "--user"])) {
                (Some(e), Some(u)) => e.accounts.get(u).map(|a| a.token.clone()),
                (Some(e), None) => e.accounts.get(&e.active).map(|a| a.token.clone()),
                (None, None) if host == cur_host => ctx.token.clone(),
                _ => None,
            };
            match tok {
                Some(t) => {
                    writeln!(out, "{t}")?;
                    Ok(())
                }
                None => Err(Fail::Api(format!("no oauth token found for {host}"))),
            }
        }
        "status" => {
            let a = Args::parse(
                raw,
                &["-h", "--hostname", "--json", "-q", "--jq", "--template"],
                &["-a", "--active", "-t", "--show-token"],
            )?;
            let mut h = load_hosts();
            if h.hosts.is_empty() {
                if let Some(t) = &ctx.token {
                    h.hosts.insert(
                        cur_host.clone(),
                        Host {
                            active: String::new(),
                            accounts: BTreeMap::from([(
                                String::new(),
                                Account {
                                    token: t.clone(),
                                    git_protocol: "https".into(),
                                },
                            )]),
                        },
                    );
                }
            }
            let only = a.get(&["-h", "--hostname"]);
            let mut json_hosts = Map::new();
            let mut text = String::new();
            for (host, e) in &h.hosts {
                if only.is_some_and(|o| o != host) {
                    continue;
                }
                let mut list = Vec::new();
                for (login, acct) in &e.accounts {
                    let active = *login == e.active || e.active.is_empty();
                    if a.has(&["-a", "--active"]) && !active {
                        continue;
                    }
                    let (ok, who) = if *host == cur_host {
                        match crate::http_min::http_exchange(
                            &ctx.base,
                            "GET",
                            "/user",
                            Some(&acct.token),
                            None,
                        ) {
                            Ok((200, body)) => (
                                true,
                                s(
                                    &serde_json::from_str::<Value>(&body).unwrap_or(Value::Null),
                                    "login",
                                )
                                .to_string(),
                            ),
                            _ => (false, login.clone()),
                        }
                    } else {
                        (true, login.clone())
                    };
                    let who = if who.is_empty() { login.clone() } else { who };
                    let shown = if a.has(&["-t", "--show-token"]) {
                        acct.token.clone()
                    } else {
                        mask(&acct.token)
                    };
                    text.push_str(&format!(
                        "{host}\n  {} Logged in to {host} account {who} ({})\n  - Active account: {active}\n  - Git operations protocol: {}\n  - Token: {shown}\n",
                        if ok { "✓" } else { "X" },
                        crate::token_path().display(),
                        if acct.git_protocol.is_empty() { "https" } else { &acct.git_protocol },
                    ));
                    let mut j = json!({"state": if ok { "success" } else { "error" }, "active": active, "host": host, "login": who, "tokenSource": crate::token_path().display().to_string(), "scopes": "", "gitProtocol": acct.git_protocol});
                    if a.has(&["-t", "--show-token"]) {
                        j["token"] = json!(acct.token);
                    }
                    list.push(j);
                }
                json_hosts.insert(host.clone(), Value::Array(list));
            }
            if let Some(f) = a.get(&["--json"]) {
                if f != "hosts" {
                    return usage(format!(
                        "Unknown JSON field: {f:?}\nAvailable fields:\n  hosts"
                    ));
                }
                return emit(
                    out,
                    &json!({"hosts": json_hosts}),
                    a.get(&["-q", "--jq"]),
                    a.get(&["--template"]),
                );
            }
            if a.get(&["-q", "--jq", "--template"]).is_some() {
                return usage("cannot use `--jq` or `--template` without specifying `--json`");
            }
            if text.is_empty() {
                eprintln!(
                    "You are not logged into any arks forge hosts. To log in, run: arks auth login"
                );
                return Err(Fail::Exit(1));
            }
            write!(out, "{text}")?;
            Ok(())
        }
        "refresh" => {
            let a = Args::parse(
                raw,
                &[
                    "-h",
                    "--hostname",
                    "-r",
                    "--remove-scopes",
                    "-s",
                    "--scopes",
                ],
                &["-c", "--clipboard", "--insecure-storage", "--reset-scopes"],
            )?;
            let host = a
                .get(&["-h", "--hostname"])
                .map(str::to_string)
                .unwrap_or(cur_host.clone());
            if host != cur_host {
                return Err(Fail::Api(format!("not logged in to {host}")));
            }
            let login = ctx.me()?;
            if a.get(&["-s", "--scopes", "-r", "--remove-scopes"])
                .is_some()
                || a.has(&["--reset-scopes"])
            {
                eprintln!(
                    "note: arks forge tokens carry no OAuth scopes; nothing to add or remove"
                );
            }
            eprintln!(
                "✓ Authentication for {host} account {login} is valid (arks tokens are not refreshed)"
            );
            let _ = out;
            Ok(())
        }
        "setup-git" => usage(
            "auth setup-git is out of scope: arks is its own credential surface ([[adr:root/20261119230000]])",
        ),
        "" | "help" => Err(Fail::Help),
        other => usage(format!("unknown command {other:?} for \"arks auth\"")),
    }
}

pub(crate) const STATUS_HELP: &str = "\
arks status — your work across the arks forge: assigned issues and pull requests,
review requests and mentions (gh-symmetric argv; host-tested Partial)

USAGE:
  arks status [-o <org>] [-e <owner/repo>,...]";

pub(crate) fn status(ctx: &Ctx, args: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(args, &["-e", "--exclude", "-o", "--org"], &[])?;
    let me = ctx.me()?;
    let exclude = a.list(&["-e", "--exclude"]);
    let mut scope = String::new();
    if let Some(org) = a.get(&["-o", "--org"]) {
        scope = format!(" user:{org}");
    }
    let find = |q: &str| -> Result<Vec<Value>, Fail> {
        let res = ctx.get(&format!(
            "/search/issues?q={}&per_page=100",
            enc(&format!("{q}{scope}"))
        ))?;
        Ok(arr(&res["items"])
            .iter()
            .filter(|x| {
                !exclude
                    .iter()
                    .any(|e| x["repository"]["full_name"].as_str() == Some(e))
            })
            .cloned()
            .collect())
    };
    let sections = [
        (
            "Assigned Issues",
            find(&format!("is:open type:issue assignee:{me}"))?,
        ),
        (
            "Assigned Pull Requests",
            find(&format!("is:open type:pr assignee:{me}"))?,
        ),
        (
            "Review Requests",
            find(&format!("is:open type:pr review-requested:{me}"))?,
        ),
        ("Mentions", find(&format!("is:open mentions:{me}"))?),
    ];
    for (title, items) in &sections {
        writeln!(out, "{title}")?;
        if items.is_empty() {
            writeln!(out, "Nothing here ^_^")?;
        }
        for x in items {
            writeln!(
                out,
                "{}#{}  {}",
                s(&x["repository"], "full_name"),
                n(x, "number"),
                s(x, "title")
            )?;
        }
        writeln!(out)?;
    }
    writeln!(out, "Repository Activity")?;
    let mut activity = 0;
    for repo in arr(&ctx.get("/user/repos")?).iter().filter(|r| {
        a.get(&["-o", "--org"])
            .is_none_or(|o| r["owner"]["login"].as_str() == Some(o))
    }) {
        let full = s(repo, "full_name");
        if exclude.iter().any(|e| e == full) {
            continue;
        }
        let issues = ctx
            .get(&format!("/repos/{full}/issues?state=open&per_page=5"))
            .unwrap_or(json!([]));
        for i in arr(&issues).iter().filter(|i| s(&i["user"], "login") != me) {
            writeln!(
                out,
                "{full}#{}  {} (new issue)",
                n(i, "number"),
                s(i, "title")
            )?;
            activity += 1;
        }
    }
    if activity == 0 {
        writeln!(out, "Nothing here ^_^")?;
    }
    Ok(())
}

pub(crate) const BROWSE_HELP: &str = "\
arks browse — print the arks forge URL for a repository, file, issue, pull request,
commit or settings page (gh-symmetric argv). arks never opens a browser: the URL is
printed (gh's --no-browser behaviour) and noted on standard error.

USAGE:
  arks browse [<number> | <path> | <commit-sha>] [flags]";

pub(crate) fn browse(ctx: &Ctx, args: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(
        args,
        &["-b", "--branch", "-c?", "--commit?", "-R", "--repo"],
        &[
            "-a",
            "--actions",
            "--blame",
            "-n",
            "--no-browser",
            "-p",
            "--projects",
            "-r",
            "--releases",
            "-s",
            "--settings",
            "-w",
            "--wiki",
        ],
    )?;
    let (o, r) = ctx.repo(&a)?;
    let picks = [
        ("--actions", &["-a", "--actions"][..]),
        ("--projects", &["-p", "--projects"]),
        ("--releases", &["-r", "--releases"]),
        ("--settings", &["-s", "--settings"]),
        ("--wiki", &["-w", "--wiki"]),
    ];
    let chosen: Vec<&str> = picks
        .iter()
        .filter(|(_, f)| a.has(f))
        .map(|(n, _)| *n)
        .collect();
    if chosen.len() > 1
        || (!chosen.is_empty()
            && (a.get(&["-b", "--branch"]).is_some() || a.get(&["-c", "--commit"]).is_some()))
    {
        return usage(
            "arguments not supported when using `--projects`, `--wiki`, `--settings`, `--actions` or `--releases` together or with `--branch` / `--commit`",
        );
    }
    let arg = a.positionals().first().copied();
    let mut path = format!("/{o}/{r}");
    match chosen.first().copied() {
        Some("--actions") => {
            return usage(
                "Actions are GitHub-platform-only; the arks forge has no actions page ([[adr:root/20261119230000]])",
            );
        }
        Some("--projects") => path.push_str("/projects"),
        Some("--releases") => path.push_str("/releases"),
        Some("--settings") => path.push_str("/settings"),
        Some("--wiki") => path.push_str("/wiki"),
        _ => {
            let commit = a
                .get(&["-c", "--commit"])
                .map(|c| if c.is_empty() { "last" } else { c });
            let commit = match commit {
                Some("last") => {
                    let list = ctx.get(&format!("/repos/{o}/{r}/commits?per_page=1"))?;
                    Some(
                        arr(&list)
                            .first()
                            .map(|c| s(c, "sha").to_string())
                            .ok_or_else(|| Fail::Api("no commits found".into()))?,
                    )
                }
                other => other.map(str::to_string),
            };
            match arg {
                Some(x) if parse_number(x).is_some() && !x.contains('/') && !x.contains('.') => {
                    let num = parse_number(x).unwrap_or(0);
                    let is_pr = ctx.get(&format!("/repos/{o}/{r}/pulls/{num}")).is_ok();
                    path.push_str(&format!("/{}/{num}", if is_pr { "pull" } else { "issues" }));
                }
                Some(x)
                    if x.len() >= 7
                        && x.len() <= 40
                        && x.chars().all(|c| c.is_ascii_hexdigit())
                        && commit.is_none()
                        && a.get(&["-b", "--branch"]).is_none() =>
                {
                    path.push_str(&format!("/commit/{x}"));
                }
                Some(x) => {
                    let (file, line) = match x.rsplit_once(':') {
                        Some((f, l))
                            if l.split('-').all(|p| p.chars().all(|c| c.is_ascii_digit())) =>
                        {
                            (f, Some(l))
                        }
                        _ => (x, None),
                    };
                    let rev = match (&commit, a.get(&["-b", "--branch"])) {
                        (Some(c), _) => c.clone(),
                        (None, Some(b)) => b.to_string(),
                        (None, None) => {
                            s(&ctx.get(&format!("/repos/{o}/{r}"))?, "default_branch").to_string()
                        }
                    };
                    let view = if a.has(&["--blame"]) { "blame" } else { "blob" };
                    path.push_str(&format!("/{view}/{rev}/{}", file.trim_start_matches('/')));
                    if let Some(l) = line {
                        let (s1, s2) = l.split_once('-').unwrap_or((l, ""));
                        path.push_str(&format!("#L{s1}"));
                        if !s2.is_empty() {
                            path.push_str(&format!("-L{s2}"));
                        }
                    }
                }
                None => {
                    if a.has(&["--blame"]) {
                        return usage("`--blame` requires a file path argument");
                    }
                    if let Some(c) = commit {
                        path.push_str(&format!("/tree/{c}"));
                    } else if let Some(b) = a.get(&["-b", "--branch"]) {
                        path.push_str(&format!("/tree/{b}"));
                    }
                }
            }
        }
    }
    let url = ctx.web_url(&path);
    if a.has(&["-n", "--no-browser"]) {
        writeln!(out, "{url}")?;
        return Ok(());
    }
    web(out, &url)
}

pub(crate) fn org(ctx: &Ctx, args: &[String], out: &mut dyn Write) -> Res {
    let sub = args.first().map(String::as_str).unwrap_or("list");
    match sub {
        "list" | "ls" => {
            let a = Args::parse(args.get(1..).unwrap_or(&[]), &["-L", "--limit"], &[])?;
            let list = ctx.get("/orgs")?;
            let mut items = arr(&list).to_vec();
            items.truncate(a.limit(30)?);
            if items.is_empty() {
                eprintln!(
                    "There are no organizations associated with @{}",
                    ctx.me().unwrap_or_default()
                );
            }
            for o in &items {
                writeln!(out, "{}", s(o, "login"))?;
            }
            Ok(())
        }
        "" | "help" => Err(Fail::Help),
        other => usage(format!("unknown command {other:?} for \"arks org\"")),
    }
}

pub(crate) const API_HELP: &str = "\
arks api — authenticated HTTP request to the arks forge API (gh-symmetric argv)

USAGE:
  arks api <endpoint> [-X <method>] [-f k=v] [-F k=v|@file] [-H k:v] [--input <file>]
           [--paginate [--slurp]] [-i] [--silent] [--verbose] [-q <jq>] [-t <template>]

The endpoint is a path (`repos/{owner}/{repo}/issues`; `{owner}`, `{repo}` and
`{branch}` are filled from the current repository) on the arks forge.";

fn typed(v: &str) -> Result<Value, Fail> {
    Ok(match v {
        "true" => json!(true),
        "false" => json!(false),
        "null" => Value::Null,
        _ if v.parse::<i64>().is_ok() => json!(v.parse::<i64>().unwrap_or(0)),
        _ if v.starts_with('@') => {
            let text = super::read_input(Some(&v[1..]))?;
            json!(text.trim_end_matches('\n'))
        }
        _ => json!(v),
    })
}

/// Insert `key` (with gh's `a[b]` / `a[]` nesting) into `obj`.
fn insert_field(obj: &mut Map<String, Value>, key: &str, val: Value) {
    let mut parts: Vec<String> = Vec::new();
    let (head, rest) = key.split_once('[').map_or((key, ""), |(h, r)| (h, r));
    parts.push(head.to_string());
    for p in rest.split('[').filter(|p| !p.is_empty()) {
        parts.push(p.trim_end_matches(']').to_string());
    }
    fn put(target: &mut Value, parts: &[String], val: Value) {
        let Some((first, more)) = parts.split_first() else {
            *target = val;
            return;
        };
        if first.is_empty() {
            if !target.is_array() {
                *target = json!([]);
            }
            if more.is_empty() {
                if let Some(a) = target.as_array_mut() {
                    a.push(val);
                }
            } else {
                let mut child = json!({});
                put(&mut child, more, val);
                if let Some(a) = target.as_array_mut() {
                    a.push(child);
                }
            }
            return;
        }
        if !target.is_object() {
            *target = json!({});
        }
        let slot = target
            .as_object_mut()
            .map(|m| m.entry(first.clone()).or_insert(Value::Null));
        if let Some(slot) = slot {
            put(slot, more, val);
        }
    }
    let mut root = Value::Object(std::mem::take(obj));
    put(&mut root, &parts, val);
    if let Value::Object(m) = root {
        *obj = m;
    }
}

pub(crate) fn api(ctx: &Ctx, args: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(
        args,
        &[
            "--cache",
            "-F",
            "--field",
            "-H",
            "--header",
            "--hostname",
            "--input",
            "-q",
            "--jq",
            "-X",
            "--method",
            "-p",
            "--preview",
            "-f",
            "--raw-field",
            "-t",
            "--template",
        ],
        &[
            "-i",
            "--include",
            "--paginate",
            "--silent",
            "--slurp",
            "--verbose",
        ],
    )?;
    let Some(endpoint) = a.positionals().first().copied() else {
        return Err(Fail::Help);
    };
    if a.has(&["--slurp"]) && !a.has(&["--paginate"]) {
        return usage("`--paginate` required when passing `--slurp`");
    }
    if a.has(&["--slurp"])
        && (a.get(&["-q", "--jq"]).is_some() || a.get(&["-t", "--template"]).is_some())
    {
        return usage("the `--slurp` option is not supported with `--jq` or `--template`");
    }
    if let Some(h) = a.get(&["--hostname"]) {
        if host_of(&ctx.base) != h {
            return Err(Fail::Api(format!(
                "this arks client talks to {} (ARKS_HUB_API); --hostname {h} is not configured",
                host_of(&ctx.base)
            )));
        }
    }
    if let Some(c) = a.get(&["--cache"]) {
        if !c.ends_with(['s', 'm', 'h']) || c[..c.len() - 1].parse::<u64>().is_err() {
            return usage(format!(
                "invalid argument {c:?} for \"--cache\" flag: time: invalid duration"
            ));
        }
        eprintln!("note: arks does not cache API responses; --cache {c} is accepted and ignored");
    }
    if !a.get_all(&["-p", "--preview"]).is_empty() {
        eprintln!("note: the arks forge has no API previews; --preview is ignored");
    }
    let mut path = endpoint.to_string();
    if path.contains('{') {
        if let Ok((o, r)) = ctx.repo(&a) {
            path = path.replace("{owner}", &o).replace("{repo}", &r);
        }
        if let Some(b) = super::common::Local::discover().and_then(|l| l.branch()) {
            path = path.replace("{branch}", &b);
        }
    }
    if let Some(rest) = path.split_once("://").map(|(_, r)| r.to_string()) {
        path = rest
            .split_once('/')
            .map(|(_, p)| format!("/{p}"))
            .unwrap_or_else(|| "/".into());
    }
    if !path.starts_with('/') {
        path = format!("/{path}");
    }
    let mut fields = Map::new();
    for f in a.get_all(&["-f", "--raw-field"]) {
        let (k, v) = f.split_once('=').ok_or_else(|| {
            Fail::Usage(format!(
                "field {f:?} requires a value separated by an '=' sign"
            ))
        })?;
        insert_field(&mut fields, k, json!(v));
    }
    for f in a.get_all(&["-F", "--field"]) {
        let (k, v) = f.split_once('=').ok_or_else(|| {
            Fail::Usage(format!(
                "field {f:?} requires a value separated by an '=' sign"
            ))
        })?;
        insert_field(&mut fields, k, typed(v)?);
    }
    let has_fields = !fields.is_empty();
    let method = a
        .get(&["-X", "--method"])
        .map(str::to_ascii_uppercase)
        .unwrap_or_else(|| {
            if has_fields || a.get(&["--input"]).is_some() {
                "POST".into()
            } else {
                "GET".into()
            }
        });
    let mut body: Option<Vec<u8>> = None;
    if let Some(inp) = a.get(&["--input"]) {
        let raw = super::read_input(Some(inp))?;
        body = Some(raw.into_bytes());
        if has_fields {
            let q: Vec<String> = fields
                .iter()
                .map(|(k, v)| {
                    format!(
                        "{}={}",
                        enc(k),
                        enc(&match v {
                            Value::String(s) => s.clone(),
                            o => o.to_string(),
                        })
                    )
                })
                .collect();
            path.push(if path.contains('?') { '&' } else { '?' });
            path.push_str(&q.join("&"));
        }
    } else if has_fields {
        if method == "GET" {
            let q: Vec<String> = fields
                .iter()
                .map(|(k, v)| {
                    format!(
                        "{}={}",
                        enc(k),
                        enc(&match v {
                            Value::String(s) => s.clone(),
                            o => o.to_string(),
                        })
                    )
                })
                .collect();
            path.push(if path.contains('?') { '&' } else { '?' });
            path.push_str(&q.join("&"));
        } else {
            body = Some(Value::Object(fields).to_string().into_bytes());
        }
    }
    let mut accept = "application/json".to_string();
    for h in a.get_all(&["-H", "--header"]) {
        let (k, v) = h.split_once(':').ok_or_else(|| {
            Fail::Usage(format!("header {h:?} requires a value separated by ':'"))
        })?;
        if k.trim().eq_ignore_ascii_case("accept") {
            accept = v.trim().to_string();
        } else {
            eprintln!(
                "note: header {} is not forwarded by the arks client (only Accept)",
                k.trim()
            );
        }
    }
    let tok = ctx
        .token
        .as_deref()
        .ok_or_else(|| Fail::Auth("not logged in — arks auth login".into()))?;
    if a.has(&["--verbose"]) {
        eprintln!(
            "* Request to {}{path}\n> {method} {path} HTTP/1.1\n> Accept: {accept}\n> Authorization: token ****",
            ctx.base.trim_end_matches('/')
        );
    }
    let (code, bytes) = crate::http_min::http_exchange_bytes(
        &ctx.base,
        &method,
        &path,
        Some(tok),
        body.as_deref(),
        "application/json",
        &accept,
    )
    .map_err(Fail::Api)?;
    if a.has(&["-i", "--include"]) || a.has(&["--verbose"]) {
        let reason = match code {
            200 => "OK",
            201 => "Created",
            202 => "Accepted",
            204 => "No Content",
            404 => "Not Found",
            422 => "Unprocessable Entity",
            _ => "",
        };
        writeln!(out, "HTTP/1.1 {code} {reason}")?;
        writeln!(out, "Content-Type: application/json\n")?;
    }
    let text = String::from_utf8_lossy(&bytes).into_owned();
    if !a.has(&["--silent"]) && !text.is_empty() {
        let jq = a.get(&["-q", "--jq"]);
        let tmpl = a.get(&["-t", "--template"]);
        match serde_json::from_str::<Value>(&text) {
            Ok(v) if jq.is_some() || tmpl.is_some() => emit(out, &v, jq, tmpl)?,
            Ok(v) if a.has(&["--slurp"]) => emit(out, &json!([v]), None, None)?,
            Ok(v) => {
                writeln!(out, "{}", serde_json::to_string(&v).unwrap_or_default())?;
            }
            Err(_) => write!(out, "{text}")?,
        }
    }
    if code >= 400 {
        let msg = serde_json::from_str::<Value>(&text)
            .ok()
            .and_then(|v| v.get("message").and_then(Value::as_str).map(str::to_string))
            .unwrap_or_default();
        eprintln!("arks: {msg} (HTTP {code})");
        return Err(Fail::Exit(1));
    }
    Ok(())
}

pub(crate) const CONFIG_HELP: &str = "\
arks config — arks forge client settings (gh-symmetric argv; ≠ git config)

USAGE:
  arks config get <key> [-h <host>]
  arks config set <key> <value> [-h <host>]
  arks config list [-h <host>]
  arks config clear-cache

Keys: git_protocol, editor, prompt, prefer_editor_prompt, pager, http_unix_socket,
browser, color_labels, accessible_colors, accessible_prompter, spinner (and any other).";

const CONFIG_DEFAULTS: &[(&str, &str)] = &[
    ("git_protocol", "https"),
    ("editor", ""),
    ("prompt", "enabled"),
    ("prefer_editor_prompt", "disabled"),
    ("pager", ""),
    ("http_unix_socket", ""),
    ("browser", ""),
    ("color_labels", "disabled"),
    ("accessible_colors", "disabled"),
    ("accessible_prompter", "disabled"),
    ("spinner", "enabled"),
];

fn config_key(host: Option<&str>, key: &str) -> String {
    match host {
        Some(h) => format!("hosts.{h}.{key}"),
        None => key.to_string(),
    }
}

pub(crate) fn config(_ctx: &Ctx, args: &[String], out: &mut dyn Write) -> Res {
    let sub = args.first().map(String::as_str).unwrap_or("");
    let raw = args.get(1..).unwrap_or(&[]);
    let mut st = crate::load_hub_client_state();
    match sub {
        "get" => {
            let a = Args::parse(raw, &["-h", "--host"], &[])?;
            let key = a
                .positionals()
                .first()
                .copied()
                .ok_or_else(|| Fail::Usage("usage: arks config get <key>".into()))?;
            let host = a.get(&["-h", "--host"]);
            let val = st
                .config
                .get(&config_key(host, key))
                .or_else(|| st.config.get(key))
                .cloned()
                .or_else(|| match key {
                    "api" => Some(crate::api_base()),
                    "token_file" => Some(crate::token_path().display().to_string()),
                    "client_state" => Some(crate::hub_client_state_path().display().to_string()),
                    _ => CONFIG_DEFAULTS
                        .iter()
                        .find(|(k, _)| *k == key)
                        .map(|(_, v)| v.to_string()),
                });
            match val {
                Some(v) => {
                    writeln!(out, "{v}")?;
                    Ok(())
                }
                None => Err(Fail::Exit(1)),
            }
        }
        "set" => {
            let a = Args::parse(raw, &["-h", "--host"], &[])?;
            let pos = a.positionals();
            let (Some(key), Some(value)) = (pos.first(), pos.get(1)) else {
                return usage("usage: arks config set <key> <value>");
            };
            let allowed: &[(&str, &[&str])] = &[
                ("git_protocol", &["https", "ssh"]),
                ("prompt", &["enabled", "disabled"]),
                ("prefer_editor_prompt", &["enabled", "disabled"]),
                ("color_labels", &["enabled", "disabled"]),
                ("accessible_colors", &["enabled", "disabled"]),
                ("accessible_prompter", &["enabled", "disabled"]),
                ("spinner", &["enabled", "disabled"]),
            ];
            if let Some((_, vals)) = allowed.iter().find(|(k, _)| k == key) {
                if !vals.contains(value) {
                    return usage(format!(
                        "failed to set {key:?} to {value:?}: valid values are {}",
                        vals.join(", ")
                    ));
                }
            }
            if matches!(*key, "api" | "token_file" | "client_state") {
                return usage(format!(
                    "`{key}` comes from the environment (ARKS_HUB_API / ARKS_HUB_TOKEN_FILE / ARKS_HUB_CLIENT_STATE)"
                ));
            }
            if !CONFIG_DEFAULTS.iter().any(|(k, _)| k == key) {
                eprintln!("! warning: '{key}' is not a known configuration key");
            }
            st.config
                .insert(config_key(a.get(&["-h", "--host"]), key), value.to_string());
            crate::save_hub_client_state(&st).map_err(Fail::Api)?;
            Ok(())
        }
        "list" | "ls" | "" => {
            let a = Args::parse(raw, &["-h", "--host"], &[])?;
            let host = a.get(&["-h", "--host"]);
            for (k, d) in CONFIG_DEFAULTS {
                let v = st
                    .config
                    .get(&config_key(host, k))
                    .or_else(|| st.config.get(*k))
                    .map(String::as_str)
                    .unwrap_or(d);
                writeln!(out, "{k}={v}")?;
            }
            for (k, v) in &st.config {
                if !k.starts_with("hosts.") && !CONFIG_DEFAULTS.iter().any(|(d, _)| d == k) {
                    writeln!(out, "{k}={v}")?;
                }
            }
            Ok(())
        }
        "clear-cache" => {
            Args::parse(raw, &[], &[])?;
            let cache = crate::token_path().parent().map(|p| p.join("hub-cache"));
            if let Some(c) = cache {
                let _ = std::fs::remove_dir_all(c);
            }
            eprintln!("✓ Cleared the cache");
            let _ = out;
            Ok(())
        }
        "help" => Err(Fail::Help),
        other => usage(format!("unknown command {other:?} for \"arks config\"")),
    }
}

pub(crate) const ALIAS_HELP: &str = "\
arks alias — shortcuts for arks forge commands (gh-symmetric argv)

USAGE:
  arks alias set <alias> <expansion> [--clobber] [-s]
  arks alias list
  arks alias delete {<alias> | --all}
  arks alias import [<filename> | -] [--clobber]

Expansions are stored in the arks client state; a `!`-prefixed (or --shell)
expansion is recorded as a shell alias. arks does not edit shell profiles.";

pub(crate) fn alias(_ctx: &Ctx, args: &[String], out: &mut dyn Write) -> Res {
    let sub = args.first().map(String::as_str).unwrap_or("");
    let raw = args.get(1..).unwrap_or(&[]);
    let mut st = crate::load_hub_client_state();
    let reserved = [
        "alias",
        "api",
        "auth",
        "browse",
        "completion",
        "config",
        "gist",
        "issue",
        "label",
        "org",
        "pr",
        "project",
        "release",
        "repo",
        "ruleset",
        "search",
        "ssh-key",
        "gpg-key",
        "status",
        "discussion",
    ];
    match sub {
        "set" => {
            let a = Args::parse(raw, &[], &["--clobber", "-s", "--shell"])?;
            let pos = a.positionals();
            let (Some(name), Some(exp)) = (pos.first(), pos.get(1)) else {
                return usage("usage: arks alias set <alias> <expansion>");
            };
            if reserved.contains(name) {
                return Err(Fail::Api(format!(
                    "could not create alias {name}: already a command"
                )));
            }
            if st.aliases.contains_key(*name) && !a.has(&["--clobber"]) {
                return Err(Fail::Api(format!(
                    "could not create alias {name}: name already taken (use --clobber to overwrite)"
                )));
            }
            let exp = if a.has(&["-s", "--shell"]) && !exp.starts_with('!') {
                format!("!{exp}")
            } else {
                exp.to_string()
            };
            st.aliases.insert(name.to_string(), exp.clone());
            crate::save_hub_client_state(&st).map_err(Fail::Api)?;
            eprintln!("✓ Added alias {name}");
            writeln!(out, "{name}={exp}")?;
            Ok(())
        }
        "list" | "ls" | "" => {
            Args::parse(raw, &[], &[])?;
            if st.aliases.is_empty() {
                eprintln!("no aliases configured");
                return Ok(());
            }
            for (k, v) in &st.aliases {
                writeln!(out, "{k}: {v}")?;
            }
            Ok(())
        }
        "delete" => {
            let a = Args::parse(raw, &[], &["--all"])?;
            if a.has(&["--all"]) {
                st.aliases.clear();
            } else {
                let name = a
                    .positionals()
                    .first()
                    .copied()
                    .ok_or_else(|| Fail::Usage("specify an alias to delete or --all".into()))?;
                if st.aliases.remove(name).is_none() {
                    return Err(Fail::Api(format!("no such alias {name}")));
                }
                eprintln!("✓ Deleted alias {name}");
                writeln!(out, "deleted={name}")?;
            }
            crate::save_hub_client_state(&st).map_err(Fail::Api)?;
            Ok(())
        }
        "import" => {
            let a = Args::parse(raw, &[], &["--clobber"])?;
            let text = super::read_input(a.positionals().first().copied())?;
            let mut n = 0usize;
            for line in text.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                // gh imports YAML `name: expansion`; `name=expansion` is the older arks form.
                let pair = line.split_once(": ").or_else(|| line.split_once('='));
                let Some((k, v)) = pair else {
                    continue;
                };
                let (k, v) = (k.trim(), v.trim().trim_matches(['\'', '"']));
                if st.aliases.contains_key(k) && !a.has(&["--clobber"]) {
                    eprintln!("! Could not import alias {k}: name already taken");
                    continue;
                }
                st.aliases.insert(k.into(), v.into());
                n += 1;
            }
            crate::save_hub_client_state(&st).map_err(Fail::Api)?;
            writeln!(out, "imported={n}")?;
            Ok(())
        }
        "help" => Err(Fail::Help),
        other => usage(format!("unknown command {other:?} for \"arks alias\"")),
    }
}

pub(crate) fn completion(_ctx: &Ctx, args: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(args, &["-s", "--shell"], &[])?;
    let shell = a
        .get(&["-s", "--shell"])
        .or(a.positionals().first().copied())
        .ok_or_else(|| Fail::Usage("error: the value for `--shell` is required".into()))?;
    let words = crate::GROUPS.join(" ");
    match shell {
        "bash" => {
            writeln!(
                out,
                "# arks forge completion (emit only; no shell profile is changed)"
            )?;
            writeln!(out, "complete -W \"{words}\" arks")?;
        }
        "zsh" => {
            writeln!(out, "#compdef arks")?;
            writeln!(out, "_arks() {{ _arguments '1: :({words})' }}")?;
            writeln!(out, "compdef _arks arks")?;
        }
        "fish" => writeln!(out, "complete -c arks -f -a \"{words}\"")?,
        "powershell" => {
            writeln!(
                out,
                "Register-ArgumentCompleter -Native -CommandName arks -ScriptBlock {{"
            )?;
            writeln!(out, "  param($wordToComplete)")?;
            writeln!(
                out,
                "  '{}' -split ' ' | Where-Object {{ $_ -like \"$wordToComplete*\" }}",
                words
            )?;
            writeln!(out, "}}")?;
        }
        other => {
            return usage(format!(
                "unsupported shell type {other:?}: valid values are {{bash|zsh|fish|powershell}}"
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::insert_field;
    use serde_json::{Map, json};

    #[test]
    fn api_field_nesting() {
        let mut m = Map::new();
        insert_field(&mut m, "name", json!("x"));
        insert_field(&mut m, "rules[][type]", json!("deletion"));
        insert_field(&mut m, "conditions[ref_name][include][]", json!("~ALL"));
        assert_eq!(
            serde_json::Value::Object(m),
            json!({"name": "x", "rules": [{"type": "deletion"}], "conditions": {"ref_name": {"include": ["~ALL"]}}})
        );
    }
}
