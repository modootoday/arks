//! Remote helper programs: `remote-ext`, `remote-fd` and
//! `remote-http` / `remote-https` / `remote-ftp` / `remote-ftps`.
//!
//! Cite: <https://git-scm.com/docs/gitremote-helpers> (the stdin command
//! protocol), <https://git-scm.com/docs/git-remote-ext>,
//! <https://git-scm.com/docs/git-remote-fd>,
//! <https://git-scm.com/docs/gitprotocol-http>, <https://git-scm.com/docs/gitprotocol-v2>.
//!
//! - `remote-ext <remote> <command>` offers `connect`: on `connect <service>`
//!   it runs `<command>` (the user's program; `%S` / `%s` expand to the
//!   service with / without `git-`, `% ` is a space, `%%` a percent, `%G` /
//!   `%V` send a git:// request header) with stdio wired to the caller.
//! - `remote-fd <remote> <infd>[,<outfd>][/<anything>]` offers `connect` over
//!   the given file descriptors.
//! - `remote-http|https|ftp|ftps <remote> [<url>]` offers the capabilities of
//!   stock git's HTTP helper: `stateless-connect` (protocol v2 proxied over
//!   HTTP), `fetch`, `get`, `option`, `push`, `check-connectivity` and
//!   `object-format`; transfers use arks's smart HTTP client. arks has no FTP
//!   client, so `ftp` / `ftps` transfers fail with an explicit error.

use crate::error::{CliError, CliResult};
use arks_remote::{HttpRequest, Transport};
use std::io::{BufRead, Read, Write};

fn read_line(input: &mut impl BufRead) -> CliResult<Option<String>> {
    let mut line = String::new();
    if input.read_line(&mut line)? == 0 {
        return Ok(None);
    }
    Ok(Some(line.trim_end_matches(['\n', '\r']).to_string()))
}

fn flush_out(text: &str) -> CliResult<()> {
    let mut out = std::io::stdout().lock();
    out.write_all(text.as_bytes())?;
    out.flush()?;
    Ok(())
}

/// Copy everything left on our stdin (including what the line reader has
/// buffered) into `sink`, closing it at end of input.
fn pump_stdin(mut input: impl BufRead + Send + 'static, mut sink: impl Write + Send + 'static) {
    std::thread::spawn(move || {
        let _ = std::io::copy(&mut input, &mut sink);
    });
}

/// The parsed `remote-ext` command: argv plus an optional git:// header.
struct ExtCommand {
    argv: Vec<String>,
    git_path: Option<String>,
    vhost: Option<String>,
}

fn parse_ext(template: &str, service: &str) -> CliResult<ExtCommand> {
    let noprefix = service.strip_prefix("git-").unwrap_or(service);
    let mut argv = Vec::new();
    let (mut git_path, mut vhost) = (None, None);
    let chars: Vec<char> = template.chars().collect();
    let mut i = 0;
    let mut cur = String::new();
    let mut in_token = false;
    // `Some(true)` = the current token is a %G path, `Some(false)` = %V host.
    let mut special: Option<bool> = None;
    let finish = |cur: &mut String,
                  special: &mut Option<bool>,
                  argv: &mut Vec<String>,
                  git_path: &mut Option<String>,
                  vhost: &mut Option<String>| {
        match special.take() {
            Some(true) => *git_path = Some(std::mem::take(cur)),
            Some(false) => *vhost = Some(std::mem::take(cur)),
            None => argv.push(std::mem::take(cur)),
        }
    };
    while i < chars.len() {
        let c = chars[i];
        if c == ' ' {
            if in_token {
                finish(&mut cur, &mut special, &mut argv, &mut git_path, &mut vhost);
                in_token = false;
            }
            i += 1;
            continue;
        }
        let token_start = !in_token;
        in_token = true;
        if c != '%' {
            cur.push(c);
            i += 1;
            continue;
        }
        let Some(&n) = chars.get(i + 1) else {
            return Err(CliError::fatal("Bad remote-ext placeholder '%'."));
        };
        match n {
            ' ' => cur.push(' '),
            '%' => cur.push('%'),
            's' if special.is_none() => cur.push_str(noprefix),
            'S' if special.is_none() => cur.push_str(service),
            'G' if token_start => special = Some(true),
            'V' if token_start => special = Some(false),
            other => {
                return Err(CliError::fatal(format!(
                    "Bad remote-ext placeholder '%{other}'."
                )));
            }
        }
        i += 2;
    }
    if in_token {
        finish(&mut cur, &mut special, &mut argv, &mut git_path, &mut vhost);
    }
    Ok(ExtCommand {
        argv,
        git_path,
        vhost,
    })
}

/// `arks remote-ext <remote> <command>`.
pub fn cmd_remote_ext(args: &[String]) -> CliResult<()> {
    let [_remote, template] = args else {
        return Err(CliError::Exit(
            129,
            "usage: git remote-ext <remote> <url>".into(),
        ));
    };
    let mut input = std::io::BufReader::new(std::io::stdin());
    loop {
        let Some(line) = read_line(&mut input)? else {
            return Ok(());
        };
        if line == "capabilities" {
            flush_out("*connect\n\n")?;
            continue;
        }
        let Some(service) = line.strip_prefix("connect ") else {
            eprint!("Bad command");
            return Err(CliError::silent(1));
        };
        flush_out("\n")?;
        let cmd = parse_ext(template, service)?;
        let Some(prog) = cmd.argv.first() else {
            return Err(CliError::fatal("remote-ext command is empty"));
        };
        let mut child = match std::process::Command::new(prog)
            .args(&cmd.argv[1..])
            .env("GIT_EXT_SERVICE", service)
            .env(
                "GIT_EXT_SERVICE_NOPREFIX",
                service.strip_prefix("git-").unwrap_or(service),
            )
            .stdin(std::process::Stdio::piped())
            .spawn()
        {
            Ok(c) => c,
            Err(_) => {
                return Err(CliError::Exit(
                    128,
                    format!(
                        "error: cannot run {prog}: No such file or directory\nfatal: Can't run specified command"
                    ),
                ));
            }
        };
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| CliError::Msg("remote-ext: child stdin".into()))?;
        if let Some(path) = &cmd.git_path {
            let mut req = format!("{service} {path}\0");
            if let Some(h) = &cmd.vhost {
                req.push_str(&format!("host={h}\0"));
            }
            let pkt = format!("{:04x}{req}", req.len() + 4);
            let _ = stdin.write_all(pkt.as_bytes());
        }
        pump_stdin(input, stdin);
        let status = child.wait()?;
        return match status.code() {
            Some(0) => Ok(()),
            Some(c) => Err(CliError::silent(u8::try_from(c).unwrap_or(1))),
            None => Err(CliError::silent(1)),
        };
    }
}

/// Parse `<infd>[,<outfd>][/<anything>]`.
fn parse_fd_url(url: &str) -> Option<(i32, i32)> {
    let main = url.split('/').next().unwrap_or("");
    let (a, b) = match main.split_once(',') {
        Some((a, b)) => (a, Some(b)),
        None => (main, None),
    };
    let inf: i32 = a.parse().ok()?;
    let outf: i32 = match b {
        Some(b) => b.parse().ok()?,
        None => inf,
    };
    Some((inf, outf))
}

/// `arks remote-fd <remote> <infd>[,<outfd>][/<anything>]`.
pub fn cmd_remote_fd(args: &[String]) -> CliResult<()> {
    let [_remote, url] = args else {
        return Err(CliError::Exit(
            129,
            "usage: git remote-fd <remote> <url>".into(),
        ));
    };
    let Some((inf, outf)) = parse_fd_url(url) else {
        return Err(CliError::fatal("Bad URL syntax"));
    };
    let mut input = std::io::BufReader::new(std::io::stdin());
    loop {
        let Some(line) = read_line(&mut input)? else {
            return Ok(());
        };
        if line == "capabilities" {
            flush_out("*connect\n\n")?;
            continue;
        }
        if line.starts_with("connect ") {
            flush_out("\n")?;
            let (mut from, to) = fd_files(inf, outf)?;
            pump_stdin(input, to);
            let mut out = std::io::stdout().lock();
            std::io::copy(&mut from, &mut out)?;
            out.flush()?;
            return Ok(());
        }
        return Err(CliError::fatal(format!("Bad command: {line}")));
    }
}

#[cfg(unix)]
fn fd_files(inf: i32, outf: i32) -> CliResult<(std::fs::File, std::fs::File)> {
    use std::os::unix::io::FromRawFd;
    let dup = |fd: i32| -> CliResult<std::fs::File> {
        // SAFETY: `dup` returns a fresh descriptor we own (or -1); the
        // original descriptor stays with the caller that passed it to us.
        let n = unsafe { libc::dup(fd) };
        if n < 0 {
            return Err(CliError::fatal(format!("bad file descriptor {fd}")));
        }
        // SAFETY: `n` is a valid descriptor owned by nothing else.
        Ok(unsafe { std::fs::File::from_raw_fd(n) })
    };
    Ok((dup(inf)?, dup(outf)?))
}

#[cfg(not(unix))]
fn fd_files(_inf: i32, _outf: i32) -> CliResult<(std::fs::File, std::fs::File)> {
    Err(CliError::fatal(
        "remote-fd: file descriptor transport needs a Unix platform",
    ))
}

const HTTP_CAPABILITIES: &str =
    "stateless-connect\nfetch\nget\noption\npush\ncheck-connectivity\nobject-format\n\n";

/// Options a helper session collected (`option <name> <value>`).
#[derive(Default)]
struct HelperOptions {
    depth: Option<u32>,
    filter: Option<String>,
    object_format: bool,
    dry_run: bool,
}

impl HelperOptions {
    fn set(&mut self, name: &str, value: &str) -> &'static str {
        match name {
            "verbosity" | "progress" | "followtags" | "check-connectivity" | "cloning"
            | "update-shallow" | "pushcert" | "push-option" | "cas" | "deepen-since"
            | "deepen-not" | "deepen-relative" | "from-promisor" | "refetch" | "no-dependents"
            | "atomic" | "family" => "ok",
            "depth" => match value.parse() {
                Ok(d) => {
                    self.depth = Some(d);
                    "ok"
                }
                Err(_) => "error invalid depth",
            },
            "filter" => {
                self.filter = Some(value.to_string());
                "ok"
            }
            "dry-run" => {
                self.dry_run = value == "true";
                "ok"
            }
            "object-format" => {
                self.object_format = value == "true";
                "ok"
            }
            _ => "unsupported",
        }
    }
}

/// `arks remote-http|remote-https|remote-ftp|remote-ftps <remote> [<url>]`.
pub fn cmd_remote_curl(args: &[String]) -> CliResult<()> {
    let Some(remote) = args.first() else {
        return Err(CliError::Exit(
            1,
            "error: remote-curl: usage: git remote-curl <remote> [<url>]".into(),
        ));
    };
    let url = match args.get(1) {
        Some(u) => u.clone(),
        None => configured_url(remote).unwrap_or_else(|| remote.clone()),
    };
    let ftp = url.starts_with("ftp://") || url.starts_with("ftps://");
    let unable = |why: String| CliError::fatal(format!("unable to access '{url}': {why}"));
    let no_ftp = || unable("protocol ftp is not available in arks (no FTP client)".into());
    let mut opts = HelperOptions::default();
    let mut input = std::io::BufReader::new(std::io::stdin());
    loop {
        // A blank line ends the session; end of input without one is an
        // error exit (as stock git's helper does), with nothing printed.
        let Some(line) = read_line(&mut input)? else {
            return Err(CliError::silent(1));
        };
        if line.is_empty() {
            return Ok(());
        }
        if line == "capabilities" {
            flush_out(HTTP_CAPABILITIES)?;
        } else if let Some(rest) = line.strip_prefix("option ") {
            let (name, value) = rest.split_once(' ').unwrap_or((rest, ""));
            flush_out(&format!("{}\n", opts.set(name, value)))?;
        } else if line == "list" || line.starts_with("list ") {
            if ftp {
                return Err(no_ftp());
            }
            let (refs, head) =
                arks_remote::ls_remote_with_head(&url).map_err(|e| unable(e.to_string()))?;
            let mut out = String::new();
            if opts.object_format {
                out.push_str(":object-format sha1\n");
            }
            for r in &refs {
                if r.name == "HEAD" {
                    if let Some(t) = &head {
                        out.push_str(&format!("@{t} HEAD\n"));
                        continue;
                    }
                }
                out.push_str(&format!("{} {}\n", r.oid.to_hex(), r.name));
            }
            out.push('\n');
            flush_out(&out)?;
        } else if line.starts_with("fetch ") {
            let mut wants = vec![line];
            while let Some(l) = read_line(&mut input)? {
                if l.is_empty() {
                    break;
                }
                wants.push(l);
            }
            if ftp {
                return Err(no_ftp());
            }
            helper_fetch(&url, &wants, &opts).map_err(|e| unable(e.to_string()))?;
            flush_out("\n")?;
        } else if line.starts_with("push ") {
            let mut specs = vec![line];
            while let Some(l) = read_line(&mut input)? {
                if l.is_empty() {
                    break;
                }
                specs.push(l);
            }
            if ftp {
                return Err(no_ftp());
            }
            flush_out(&helper_push(&url, &specs, &opts)?)?;
        } else if let Some(rest) = line.strip_prefix("get ") {
            let (uri, path) = rest.split_once(' ').ok_or_else(|| {
                CliError::fatal(format!("http transport does not support {line}"))
            })?;
            if ftp {
                return Err(no_ftp());
            }
            let resp = arks_remote::http_get(uri).map_err(|e| unable(e.to_string()))?;
            std::fs::write(path, resp.body)?;
            flush_out("\n")?;
        } else if let Some(service) = line.strip_prefix("stateless-connect ") {
            if ftp {
                return Err(no_ftp());
            }
            if stateless_connect(&url, service, &mut input)? {
                return Ok(());
            }
        } else {
            return Err(CliError::Exit(
                1,
                format!("error: remote-curl: unknown command '{line}' from git"),
            ));
        }
    }
}

fn configured_url(remote: &str) -> Option<String> {
    let repo = crate::util::open_repo().ok()?;
    arks_config::ConfigSet::load(Some(repo.common_dir())).get(&format!("remote.{remote}.url"))
}

fn helper_fetch(url: &str, lines: &[String], opts: &HelperOptions) -> CliResult<()> {
    let mut wants = Vec::new();
    for l in lines {
        let hex = l
            .strip_prefix("fetch ")
            .and_then(|r| r.split(' ').next())
            .unwrap_or("");
        let oid = arks_oid::Oid::from_hex(hex)
            .map_err(|_| CliError::fatal(format!("protocol error: expected sha/ref, got '{l}'")))?;
        wants.push(oid);
    }
    let mut repo = crate::util::open_repo()?;
    let have_all = wants
        .iter()
        .all(|w| arks_store::ObjectStore::exists(repo.store(), w).unwrap_or(false));
    if have_all {
        return Ok(());
    }
    let fo = arks_remote::FetchOptions {
        depth: opts.depth,
        filter: opts.filter.clone(),
        want_oids: Some(wants),
        mark_promisor: opts.filter.is_some(),
        ..Default::default()
    };
    arks_remote::fetch_remote_with(&mut repo, url, "origin", &fo)
        .map_err(|e| CliError::Msg(e.to_string()))?;
    Ok(())
}

fn helper_push(url: &str, lines: &[String], opts: &HelperOptions) -> CliResult<String> {
    let repo = crate::util::open_repo()?;
    let mut updates = Vec::new();
    let mut dsts = Vec::new();
    for l in lines {
        let spec = l.strip_prefix("push ").unwrap_or(l);
        let spec = spec.strip_prefix('+').unwrap_or(spec);
        let (src, dst) = spec.split_once(':').unwrap_or((spec, spec));
        let new_oid = if src.is_empty() {
            arks_oid::Oid::from_hex(&"0".repeat(40)).map_err(|e| CliError::Msg(e.to_string()))?
        } else {
            crate::cmd::shared::resolve_rev(&repo, src)?
        };
        dsts.push(dst.to_string());
        updates.push(arks_remote::PushUpdate {
            refname: dst.to_string(),
            new_oid,
            expected_old: None,
        });
    }
    let mut out = String::new();
    if opts.dry_run {
        for d in &dsts {
            out.push_str(&format!("ok {d}\n"));
        }
    } else {
        match arks_remote::push_remote_with(&repo, url, &updates, &Default::default()) {
            Ok(()) => {
                for d in &dsts {
                    out.push_str(&format!("ok {d}\n"));
                }
            }
            Err(e) => {
                let why = e.to_string().replace('\n', " ");
                for d in &dsts {
                    out.push_str(&format!("error {d} {why}\n"));
                }
            }
        }
    }
    out.push('\n');
    Ok(out)
}

/// Read pkt-lines from `input` up to and including a flush packet; `None` at
/// end of input.
fn read_request(input: &mut impl Read) -> CliResult<Option<Vec<u8>>> {
    let mut req = Vec::new();
    loop {
        let mut len = [0u8; 4];
        match input.read_exact(&mut len) {
            Ok(()) => {}
            Err(_) if req.is_empty() => return Ok(None),
            Err(_) => return Err(CliError::fatal("the remote end hung up unexpectedly")),
        }
        req.extend_from_slice(&len);
        let n = std::str::from_utf8(&len)
            .ok()
            .and_then(|h| usize::from_str_radix(h, 16).ok())
            .ok_or_else(|| CliError::fatal("protocol error: bad line length"))?;
        match n {
            0 => return Ok(Some(req)),
            1 | 2 => continue,
            3 => return Err(CliError::fatal("protocol error: bad line length 3")),
            _ => {
                let mut buf = vec![0u8; n - 4];
                input
                    .read_exact(&mut buf)
                    .map_err(|_| CliError::fatal("the remote end hung up unexpectedly"))?;
                req.extend_from_slice(&buf);
            }
        }
    }
}

/// Strip a smart-HTTP `# service=<svc>` preamble (pkt-line plus flush).
fn strip_service_preamble(body: &[u8]) -> &[u8] {
    if body.len() > 8 && &body[4..6] == b"# " {
        if let Some(n) = std::str::from_utf8(&body[..4])
            .ok()
            .and_then(|h| usize::from_str_radix(h, 16).ok())
        {
            let rest = &body[n.min(body.len())..];
            return rest.strip_prefix(b"0000").unwrap_or(rest);
        }
    }
    body
}

/// `stateless-connect <service>`: proxy protocol v2 requests over HTTP.
/// Returns `false` when the server is not v2 (`fallback`: keep serving commands).
fn stateless_connect(url: &str, service: &str, input: &mut impl BufRead) -> CliResult<bool> {
    let unable = |why: String| CliError::fatal(format!("unable to access '{url}': {why}"));
    let base = url.trim_end_matches('/');
    let mut transport = arks_remote::UreqTransport::new();
    let info = transport
        .exchange(&HttpRequest {
            method: "GET".into(),
            url: format!("{base}/info/refs?service={service}"),
            headers: vec![("Git-Protocol".into(), "version=2".into())],
            body: Vec::new(),
        })
        .map_err(|e| unable(e.to_string()))?;
    if info.status >= 400 {
        return Err(unable(format!("HTTP {}", info.status)));
    }
    let ad = strip_service_preamble(&info.body);
    if !ad.starts_with(b"000eversion 2\n") {
        // Not a v2 server: the caller falls back to `fetch` / `push`.
        flush_out("fallback\n")?;
        return Ok(false);
    }
    {
        let mut out = std::io::stdout().lock();
        out.write_all(b"\n")?;
        out.write_all(ad)?;
        out.flush()?;
    }
    while let Some(req) = read_request(input)? {
        let resp = transport
            .exchange(&HttpRequest {
                method: "POST".into(),
                url: format!("{base}/{service}"),
                headers: vec![
                    (
                        "Content-Type".into(),
                        format!("application/x-{service}-request"),
                    ),
                    ("Accept".into(), format!("application/x-{service}-result")),
                    ("Git-Protocol".into(), "version=2".into()),
                ],
                body: req,
            })
            .map_err(|e| unable(e.to_string()))?;
        let mut out = std::io::stdout().lock();
        out.write_all(&resp.body)?;
        out.write_all(b"0002")?;
        out.flush()?;
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ext_placeholders() {
        let c = parse_ext("echo %S %s % x %% a%sb", "git-upload-pack").unwrap();
        assert_eq!(
            c.argv,
            vec![
                "echo",
                "git-upload-pack",
                "upload-pack",
                " x",
                "%",
                "aupload-packb"
            ]
        );
        let c = parse_ext("nc host 9418 %G/repo.git %Vexample.org", "git-upload-pack").unwrap();
        assert_eq!(c.argv, vec!["nc", "host", "9418"]);
        assert_eq!(c.git_path.as_deref(), Some("/repo.git"));
        assert_eq!(c.vhost.as_deref(), Some("example.org"));
        assert!(parse_ext("echo %q", "git-upload-pack").is_err());
        assert!(parse_ext("echo %G%Vx", "git-upload-pack").is_err());
    }

    #[test]
    fn fd_urls() {
        assert_eq!(parse_fd_url("0,1"), Some((0, 1)));
        assert_eq!(parse_fd_url("7"), Some((7, 7)));
        assert_eq!(parse_fd_url("7/foo"), Some((7, 7)));
        assert_eq!(parse_fd_url("x"), None);
        assert_eq!(parse_fd_url("0,x"), None);
    }

    #[test]
    fn service_preamble() {
        let body = b"001e# service=git-upload-pack\n0000000eversion 2\n0000";
        assert_eq!(strip_service_preamble(body), b"000eversion 2\n0000");
        assert_eq!(
            strip_service_preamble(b"000eversion 2\n"),
            b"000eversion 2\n"
        );
    }
}
