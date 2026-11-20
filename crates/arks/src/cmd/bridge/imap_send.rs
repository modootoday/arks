//! `git imap-send`: upload an mbox (as made by `format-patch --stdout`)
//! from stdin into an IMAP folder with `APPEND`.
//!
//! Primary: <https://git-scm.com/docs/git-imap-send> (`-v`, `-q`,
//! `--[no-]curl`; `imap.folder`, `imap.tunnel`, `imap.host` (`imap://` /
//! `imaps://`), `imap.user`, `imap.pass`, `imap.port` (143 / 993),
//! `imap.sslverify`, `imap.preformattedHTML`, `imap.authMethod` (CRAM-MD5,
//! else `LOGIN`)) and RFC 3501 (greeting, `CAPABILITY`, `STARTTLS`,
//! `LOGIN`, `AUTHENTICATE`, `APPEND` with a literal, `LOGOUT`). arks always
//! uses its own IMAP routines; `--curl` is accepted as git ignores it when
//! built without curl support for IMAP.

use super::net::{Conn, hex, hmac_md5};
use crate::cmd::opt_parse::{OptDef, Takes::*, o, parse};
use crate::error::{CliError, CliResult};
use arks_config::ConfigSet;
use base64::Engine;
use std::io::Read;

const OPTS: &[OptDef] = &[
    o("verbose", 'v', No, true),
    o("quiet", 'q', No, true),
    o("curl", ' ', No, true),
];

const USAGE: &str = "usage: git imap-send [-v] [-q] [--[no-]curl] < <mbox>

    -v, --[no-]verbose    be more verbose
    -q, --[no-]quiet      be more quiet
    --[no-]curl           use libcurl to communicate with the IMAP server
";

struct Server {
    folder: String,
    tunnel: Option<String>,
    host: Option<String>,
    tls: bool,
    port: u16,
    user: Option<String>,
    pass: Option<String>,
    verify: bool,
    html: bool,
    auth: Option<String>,
}

fn server(cfg: &ConfigSet) -> CliResult<Server> {
    let host_cfg = cfg.get("imap.host");
    let (tls, host) = match host_cfg.as_deref() {
        Some(h) if h.starts_with("imaps://") => (true, Some(h["imaps://".len()..].to_string())),
        Some(h) if h.starts_with("imap://") => (false, Some(h["imap://".len()..].to_string())),
        Some(h) => (false, Some(h.to_string())),
        None => (false, None),
    };
    let host = host.map(|h| h.trim_end_matches('/').to_string());
    let port = match cfg.get("imap.port") {
        Some(p) => p
            .parse()
            .map_err(|_| CliError::Exit(1, format!("fatal: bad imap.port '{p}'")))?,
        None if tls => 993,
        None => 143,
    };
    Ok(Server {
        folder: cfg.get("imap.folder").unwrap_or_default(),
        tunnel: cfg.get("imap.tunnel").filter(|t| !t.is_empty()),
        host,
        tls,
        port,
        user: cfg.get("imap.user"),
        pass: cfg.get("imap.pass"),
        verify: cfg.get_bool("imap.sslverify").unwrap_or(true),
        html: cfg.get_bool("imap.preformattedHTML").unwrap_or(false),
        auth: cfg.get("imap.authMethod"),
    })
}

pub fn cmd_imap_send(args: &[String]) -> CliResult<()> {
    if args.iter().any(|a| a == "-h") {
        print!("{USAGE}");
        return Err(CliError::silent(129));
    }
    let p = parse("imap-send", args, OPTS, Default::default()).map_err(|e| match e {
        CliError::Exit(129, m) => {
            let first = m.lines().next().unwrap_or("").to_string();
            CliError::Exit(129, format!("{first}\n{}", USAGE.trim_end()))
        }
        other => other,
    })?;
    if !p.args().is_empty() {
        return Err(CliError::Exit(129, USAGE.trim_end().into()));
    }
    let verbose = p.count("verbose") > 0;
    let quiet = p.on("quiet");
    let repo = crate::util::open_repo().ok();
    let cfg = ConfigSet::load(repo.as_ref().map(|r| r.common_dir()));
    let mut srv = server(&cfg)?;
    if srv.folder.is_empty() {
        return Err(CliError::Exit(1, "error: no IMAP folder specified".into()));
    }
    if srv.tunnel.is_none() && srv.host.is_none() {
        return Err(CliError::Exit(1, "error: no imap host specified".into()));
    }
    let mut raw = Vec::new();
    std::io::stdin().read_to_end(&mut raw)?;
    let text = String::from_utf8_lossy(&raw).into_owned();
    let messages = split_mbox(&text);
    if messages.is_empty() {
        return Err(CliError::Exit(1, "error: nothing to send".into()));
    }
    let total = messages.len();
    if !quiet {
        eprintln!(
            "sending {total} message{}",
            if total == 1 { "" } else { "s" }
        );
    }
    let mut c = connect(&mut srv)?;
    c.debug = verbose;
    for (i, m) in messages.iter().enumerate() {
        let body = if srv.html { wrap_html(m) } else { m.clone() };
        append(&mut c, &srv.folder, &body)?;
        if !quiet {
            eprint!("{:4}% ({}/{}) done\r", (i + 1) * 100 / total, i + 1, total);
        }
    }
    if !quiet {
        eprintln!();
    }
    let _ = tagged(&mut c, "LOGOUT");
    c.close();
    Ok(())
}

/// Messages of an mbox: split at `From ` lines, which are dropped; line
/// ends become CRLF.
pub(crate) fn split_mbox(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut cur: Option<String> = None;
    for line in text.split_inclusive('\n') {
        if line.starts_with("From ") {
            if let Some(m) = cur.take() {
                out.push(m);
            }
            cur = Some(String::new());
            continue;
        }
        let Some(m) = cur.as_mut() else {
            // Text before any `From ` line is a message of its own.
            cur = Some(String::new());
            let m = cur.as_mut().unwrap_or_else(|| unreachable!());
            m.push_str(&crlf(line));
            continue;
        };
        m.push_str(&crlf(line));
    }
    if let Some(m) = cur {
        out.push(m);
    }
    out.retain(|m| !m.trim().is_empty());
    out
}

fn crlf(line: &str) -> String {
    match line.strip_suffix('\n') {
        Some(l) => format!("{}\r\n", l.strip_suffix('\r').unwrap_or(l)),
        None => line.to_string(),
    }
}

/// `imap.preformattedHTML`: the body as escaped text inside `<pre>`.
fn wrap_html(msg: &str) -> String {
    let (head, body) = msg.split_once("\r\n\r\n").unwrap_or((msg, ""));
    let esc = body
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    format!("{head}\r\nContent-Type: text/html;\r\n\r\n<pre>\r\n{esc}</pre>\r\n")
}

static TAG: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);

fn next_tag() -> String {
    format!(
        "a{:03}",
        TAG.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    )
}

/// Send a tagged command and wait for its completion; untagged lines are
/// returned with the result. Continuation requests are an error here.
fn tagged(c: &mut Conn, cmd: &str) -> CliResult<Vec<String>> {
    tagged_shown(c, cmd, None)
}

fn tagged_shown(c: &mut Conn, cmd: &str, shown: Option<&str>) -> CliResult<Vec<String>> {
    let tag = next_tag();
    let shown = shown.map(|s| format!("{tag} {s}"));
    c.cmd(&format!("{tag} {cmd}"), shown.as_deref())?;
    wait(c, &tag)
}

fn wait(c: &mut Conn, tag: &str) -> CliResult<Vec<String>> {
    let mut untagged = Vec::new();
    loop {
        let Some(l) = c.line()? else {
            return Err(CliError::Exit(
                1,
                "IMAP error: unexpected EOF from server".into(),
            ));
        };
        if let Some(rest) = l.strip_prefix(&format!("{tag} ")) {
            if rest.starts_with("OK") {
                return Ok(untagged);
            }
            return Err(CliError::Exit(1, format!("IMAP command failed: {rest}")));
        }
        untagged.push(l);
    }
}

fn quoted(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

fn connect(srv: &mut Server) -> CliResult<Conn> {
    let mut c = match &srv.tunnel {
        Some(t) => Conn::tunnel(t)?,
        None => {
            let host = srv.host.clone().unwrap_or_default();
            Conn::tcp(&host, srv.port, srv.tls, srv.verify)?
        }
    };
    let greet = c
        .line()?
        .ok_or_else(|| CliError::Exit(1, "IMAP error: no greeting from server".into()))?;
    if greet.starts_with("* BYE") || !greet.starts_with("* ") {
        return Err(CliError::Exit(
            1,
            format!("IMAP error: bad greeting: {greet}"),
        ));
    }
    if greet.starts_with("* PREAUTH") {
        return Ok(c);
    }
    let caps = capabilities(&mut c)?;
    if srv.tunnel.is_none() && !c.is_tls() && caps.iter().any(|x| x == "STARTTLS") {
        tagged(&mut c, "STARTTLS")?;
        c.starttls()?;
    }
    let host = srv.host.clone().unwrap_or_default();
    let scheme = if srv.tls { "imaps" } else { "imap" };
    let mut cred = arks_remote::Credential {
        protocol: Some(scheme.to_string()),
        host: Some(host.clone()),
        username: srv.user.clone(),
        password: srv.pass.clone(),
        ..Default::default()
    };
    if cred.username.is_none() || cred.password.is_none() {
        let source = arks_remote::CredentialSource::discover(&[]);
        cred = arks_remote::credential_fill(cred, &source)
            .map_err(|e| CliError::Exit(1, format!("fatal: {e}")))?;
    }
    let user = cred.username.clone().unwrap_or_default();
    let pass = cred.password.clone().unwrap_or_default();
    srv.user = Some(user.clone());
    let b64 = base64::engine::general_purpose::STANDARD;
    match srv.auth.as_deref().map(str::to_ascii_uppercase).as_deref() {
        Some("CRAM-MD5") => {
            if !caps.iter().any(|x| x == "AUTH=CRAM-MD5") {
                return Err(CliError::Exit(
                    1,
                    "IMAP error: You specified CRAM-MD5 as authentication method, but the server does not support it".into(),
                ));
            }
            let tag = next_tag();
            c.cmd(&format!("{tag} AUTHENTICATE CRAM-MD5"), None)?;
            let ch = c
                .line()?
                .ok_or_else(|| CliError::Exit(1, "IMAP error: no challenge".into()))?;
            let ch = ch.trim_start_matches('+').trim();
            let challenge = b64
                .decode(ch)
                .map_err(|_| CliError::Exit(1, "IMAP error: bad CRAM-MD5 challenge".into()))?;
            let resp = format!("{user} {}", hex(&hmac_md5(pass.as_bytes(), &challenge)));
            c.cmd(&b64.encode(resp), Some("<CRAM-MD5 response>"))?;
            wait(&mut c, &tag)?;
        }
        Some(m) if !m.is_empty() && m != "LOGIN" => {
            return Err(CliError::Exit(
                1,
                format!("IMAP error: authentication method {m} is not supported"),
            ));
        }
        _ => {
            if caps.iter().any(|x| x == "LOGINDISABLED") {
                return Err(CliError::Exit(
                    1,
                    "IMAP error: LOGIN disabled by the server".into(),
                ));
            }
            if !c.is_tls() && srv.tunnel.is_none() {
                eprintln!("*** IMAP Warning *** Password is being sent in the clear");
            }
            let line = format!("LOGIN {} {}", quoted(&user), quoted(&pass));
            tagged_shown(&mut c, &line, Some("LOGIN <user> <pass>"))
                .map_err(|_| CliError::Exit(1, "IMAP error: LOGIN failed".into()))?;
        }
    }
    Ok(c)
}

fn capabilities(c: &mut Conn) -> CliResult<Vec<String>> {
    let lines = tagged(c, "CAPABILITY")?;
    Ok(lines
        .iter()
        .filter_map(|l| l.strip_prefix("* CAPABILITY "))
        .flat_map(|l| l.split_whitespace().map(|w| w.to_ascii_uppercase()))
        .collect())
}

fn append(c: &mut Conn, folder: &str, msg: &str) -> CliResult<()> {
    let tag = next_tag();
    c.cmd(
        &format!("{tag} APPEND {} {{{}}}", quoted(folder), msg.len()),
        None,
    )?;
    let Some(l) = c.line()? else {
        return Err(CliError::Exit(
            1,
            "IMAP error: unexpected EOF from server".into(),
        ));
    };
    if !l.starts_with('+') {
        return Err(CliError::Exit(
            1,
            format!("IMAP command 'APPEND' failed: {l}"),
        ));
    }
    c.send(msg.as_bytes())?;
    c.send(b"\r\n")?;
    wait(c, &tag).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mbox_split() {
        let m = "From 1 Mon Sep 17 00:00:00 2001\nSubject: a\n\nx\nFrom 2 Mon\nSubject: b\n\ny\n";
        let v = split_mbox(m);
        assert_eq!(
            v,
            vec!["Subject: a\r\n\r\nx\r\n", "Subject: b\r\n\r\ny\r\n"]
        );
    }
}
