//! Q6: `send-email` against a fixture SMTP server on loopback.
//!
//! The clean-room image has no git-email package, so there is no stock
//! oracle. The fixture below (test harness only, not product code) speaks
//! the RFC 5321 subset a submission client needs (EHLO/HELO, AUTH PLAIN and
//! LOGIN without TLS, MAIL FROM, RCPT TO, DATA with dot-unstuffing, RSET,
//! NOOP, QUIT), records the full session transcript and the message bytes,
//! and the tests assert the envelope, headers and body against the option
//! descriptions in <https://git-scm.com/docs/git-send-email>, RFC 5321
//! (envelope, dot-stuffing), RFC 5322 (Date, Message-ID, In-Reply-To,
//! References) and RFC 2045 / 2047 (MIME for non-ASCII content).

use base64::Engine;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::thread::{self, JoinHandle};
use std::time::Duration;

// ---------------------------------------------------------------- fixture

#[derive(Clone, Copy)]
struct ServerOpts {
    /// Answer EHLO with 502 so the client must fall back to HELO.
    reject_ehlo: bool,
    /// The `AUTH` capability line offered after EHLO, if any.
    auth: Option<&'static str>,
    /// Connections to serve before the fixture stops.
    sessions: usize,
}

impl Default for ServerOpts {
    fn default() -> Self {
        Self {
            reject_ehlo: false,
            auth: None,
            sessions: 1,
        }
    }
}

#[derive(Debug, Default)]
struct Mail {
    from: String,
    rcpts: Vec<String>,
    /// Message bytes after dot-unstuffing, CRLF line ends kept.
    data: Vec<u8>,
}

#[derive(Debug, Default)]
struct Capture {
    /// `S: …` / `C: …` lines in order (DATA content is not repeated here).
    transcript: Vec<String>,
    mails: Vec<Mail>,
    /// Decoded `(user, pass)` of each successful AUTH.
    logins: Vec<(String, String)>,
    /// Client lines that did not end in CRLF.
    bare_lf: usize,
    /// Raw DATA lines as sent (before unstuffing).
    raw_data_lines: Vec<String>,
}

struct Fixture {
    port: u16,
    handle: JoinHandle<Capture>,
}

impl Fixture {
    fn start(opts: ServerOpts) -> Self {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        let handle = thread::spawn(move || {
            let mut cap = Capture::default();
            for _ in 0..opts.sessions {
                let (s, _) = l.accept().unwrap();
                serve(s, opts, &mut cap);
            }
            cap
        });
        Self { port, handle }
    }

    fn port(&self) -> String {
        self.port.to_string()
    }

    fn finish(self) -> Capture {
        self.handle.join().expect("fixture thread")
    }
}

fn read_line(r: &mut BufReader<TcpStream>, cap: &mut Capture) -> Option<String> {
    let mut buf = Vec::new();
    if r.read_until(b'\n', &mut buf).ok()? == 0 {
        return None;
    }
    if !buf.ends_with(b"\r\n") {
        cap.bare_lf += 1;
    }
    let s = String::from_utf8_lossy(&buf);
    Some(s.trim_end_matches(['\r', '\n']).to_string())
}

fn serve(s: TcpStream, opts: ServerOpts, cap: &mut Capture) {
    s.set_read_timeout(Some(Duration::from_secs(30))).unwrap();
    let mut w = s.try_clone().unwrap();
    let mut r = BufReader::new(s);
    let b64 = base64::engine::general_purpose::STANDARD;
    let say = |w: &mut TcpStream, cap: &mut Capture, m: &str| {
        cap.transcript.push(format!("S: {m}"));
        w.write_all(format!("{m}\r\n").as_bytes()).unwrap();
    };
    say(&mut w, cap, "220 fixture.example.com ESMTP q6-fixture");
    let mut cur = Mail::default();
    while let Some(line) = read_line(&mut r, cap) {
        let up = line.to_ascii_uppercase();
        let shown = if up.starts_with("AUTH PLAIN ") {
            "AUTH PLAIN <token>".to_string()
        } else {
            line.clone()
        };
        cap.transcript.push(format!("C: {shown}"));
        if up.starts_with("EHLO ") {
            if opts.reject_ehlo {
                say(&mut w, cap, "502 5.5.2 EHLO not implemented");
            } else if let Some(a) = opts.auth {
                say(&mut w, cap, "250-fixture.example.com");
                say(&mut w, cap, "250-8BITMIME");
                say(&mut w, cap, &format!("250 AUTH {a}"));
            } else {
                say(&mut w, cap, "250-fixture.example.com");
                say(&mut w, cap, "250 8BITMIME");
            }
        } else if up.starts_with("HELO ") {
            say(&mut w, cap, "250 fixture.example.com");
        } else if up.starts_with("AUTH PLAIN") {
            let token = match line.get(11..).map(str::trim).filter(|t| !t.is_empty()) {
                Some(t) => t.to_string(),
                None => {
                    say(&mut w, cap, "334 ");
                    let t = read_line(&mut r, cap).unwrap_or_default();
                    cap.transcript.push("C: <token>".into());
                    t
                }
            };
            let raw = b64.decode(token.trim()).unwrap_or_default();
            let parts: Vec<String> = raw
                .split(|b| *b == 0)
                .map(|p| String::from_utf8_lossy(p).into_owned())
                .collect();
            if parts.len() == 3 {
                cap.logins.push((parts[1].clone(), parts[2].clone()));
                say(&mut w, cap, "235 2.7.0 Authentication successful");
            } else {
                say(&mut w, cap, "535 5.7.8 bad token");
            }
        } else if up == "AUTH LOGIN" {
            say(&mut w, cap, "334 VXNlcm5hbWU6");
            let u = read_line(&mut r, cap).unwrap_or_default();
            cap.transcript.push("C: <user>".into());
            say(&mut w, cap, "334 UGFzc3dvcmQ6");
            let p = read_line(&mut r, cap).unwrap_or_default();
            cap.transcript.push("C: <password>".into());
            let dec =
                |s: &str| String::from_utf8_lossy(&b64.decode(s).unwrap_or_default()).into_owned();
            cap.logins.push((dec(&u), dec(&p)));
            say(&mut w, cap, "235 2.7.0 Authentication successful");
        } else if up.starts_with("MAIL FROM:") {
            cur = Mail {
                from: angle(&line["MAIL FROM:".len()..]),
                ..Mail::default()
            };
            say(&mut w, cap, "250 2.1.0 Ok");
        } else if up.starts_with("RCPT TO:") {
            cur.rcpts.push(angle(&line["RCPT TO:".len()..]));
            say(&mut w, cap, "250 2.1.5 Ok");
        } else if up == "DATA" {
            say(&mut w, cap, "354 End data with <CR><LF>.<CR><LF>");
            loop {
                let mut buf = Vec::new();
                if r.read_until(b'\n', &mut buf).unwrap_or(0) == 0 {
                    return;
                }
                if !buf.ends_with(b"\r\n") {
                    cap.bare_lf += 1;
                }
                if buf == b".\r\n" || buf == b".\n" {
                    break;
                }
                cap.raw_data_lines
                    .push(String::from_utf8_lossy(&buf).trim_end().to_string());
                let body = if buf.starts_with(b".") {
                    &buf[1..]
                } else {
                    &buf[..]
                };
                cur.data.extend_from_slice(body);
            }
            cap.transcript.push("C: <message>".into());
            cap.transcript.push("C: .".into());
            cap.mails.push(std::mem::take(&mut cur));
            say(&mut w, cap, "250 2.0.0 Ok: queued as Q6");
        } else if up == "RSET" || up == "NOOP" {
            cur = Mail::default();
            say(&mut w, cap, "250 2.0.0 Ok");
        } else if up == "QUIT" {
            say(&mut w, cap, "221 2.0.0 Bye");
            return;
        } else {
            say(&mut w, cap, "500 5.5.1 unknown command");
        }
    }
}

fn angle(s: &str) -> String {
    let s = s.trim();
    match (s.find('<'), s.find('>')) {
        (Some(a), Some(b)) if a < b => s[a + 1..b].to_string(),
        _ => s.to_string(),
    }
}

// ------------------------------------------------------ message decoding

struct Msg {
    headers: Vec<(String, String)>,
    /// Header block as sent (folding kept, CRLF turned into LF).
    raw_headers: String,
    body: Vec<u8>,
}

impl Msg {
    fn parse(data: &[u8]) -> Self {
        let text = data.to_vec();
        let split = text
            .windows(4)
            .position(|w| w == b"\r\n\r\n")
            .expect("header/body separator");
        let head = String::from_utf8_lossy(&text[..split]).replace("\r\n", "\n");
        let body = text[split + 4..].to_vec();
        let mut headers: Vec<(String, String)> = Vec::new();
        for l in head.lines() {
            if l.starts_with([' ', '\t']) {
                let last = headers.last_mut().expect("continuation after a header");
                last.1.push(' ');
                last.1.push_str(l.trim());
            } else {
                let (k, v) = l.split_once(':').expect("header line");
                headers.push((k.to_string(), v.trim().to_string()));
            }
        }
        Self {
            headers,
            raw_headers: head,
            body,
        }
    }

    fn get(&self, k: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(k))
            .map(|(_, v)| v.as_str())
    }

    fn count(&self, k: &str) -> usize {
        self.headers
            .iter()
            .filter(|(n, _)| n.eq_ignore_ascii_case(k))
            .count()
    }

    fn names(&self) -> Vec<&str> {
        self.headers.iter().map(|(n, _)| n.as_str()).collect()
    }

    fn body_text(&self) -> String {
        String::from_utf8_lossy(&self.body).replace("\r\n", "\n")
    }
}

/// RFC 5322 §3.6.4 msg-id shape: `<left@right>`, no whitespace.
fn assert_msg_id(id: &str, domain: &str) {
    assert!(id.starts_with('<') && id.ends_with('>'), "{id}");
    let inner = &id[1..id.len() - 1];
    assert!(!inner.contains(char::is_whitespace), "{id}");
    assert!(!inner.contains(['<', '>']), "{id}");
    let (left, right) = inner.rsplit_once('@').expect("@ in Message-ID");
    assert!(!left.is_empty() && !left.contains('@'), "{id}");
    assert_eq!(right, domain, "{id}");
}

/// RFC 5322 §3.3 date-time as `Www, D Mon YYYY HH:MM:SS +zzzz`; returns
/// seconds since the epoch for ordering checks.
fn parse_date(v: &str) -> i64 {
    let p: Vec<&str> = v.split_whitespace().collect();
    assert_eq!(p.len(), 6, "{v}");
    assert!(
        ["Mon,", "Tue,", "Wed,", "Thu,", "Fri,", "Sat,", "Sun,"].contains(&p[0]),
        "{v}"
    );
    let d: i64 = p[1].parse().expect("day");
    let months = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let m = months.iter().position(|x| *x == p[2]).expect("month") as i64 + 1;
    let y: i64 = p[3].parse().expect("year");
    let t: Vec<i64> = p[4].split(':').map(|x| x.parse().expect("time")).collect();
    assert_eq!(t.len(), 3, "{v}");
    let z = p[5];
    assert!(
        z.len() == 5 && (z.starts_with('+') || z.starts_with('-')),
        "{v}"
    );
    let zm: i64 = z[1..3].parse::<i64>().unwrap() * 60 + z[3..].parse::<i64>().unwrap();
    let zs = if z.starts_with('-') { -zm } else { zm };
    // Days from civil (proleptic Gregorian).
    let (y2, m2) = if m <= 2 { (y - 1, m + 9) } else { (y, m - 3) };
    let era = y2.div_euclid(400);
    let yoe = y2 - era * 400;
    let doy = (153 * m2 + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    days * 86_400 + t[0] * 3600 + t[1] * 60 + t[2] - zs * 60
}

// ------------------------------------------------------------- arks runs

const DOMAIN: &str = "client.example.com";

fn arks(dir: &Path) -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_arks"));
    c.current_dir(dir)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("ARKS_PERSONALITY", "git")
        .env("GIT_AUTHOR_NAME", "A U Thor")
        .env("GIT_AUTHOR_EMAIL", "author@example.com")
        .env("GIT_COMMITTER_NAME", "C O Mitter")
        .env("GIT_COMMITTER_EMAIL", "committer@example.com")
        .env("GIT_AUTHOR_DATE", "1700000000 +0000")
        .env("GIT_COMMITTER_DATE", "1700000000 +0000")
        // An editor that fails: any unexpected --annotate / --compose run
        // would make the command fail.
        .env("GIT_EDITOR", "false")
        .env_remove("GIT_ASKPASS")
        .env_remove("SSH_ASKPASS")
        .stdin(Stdio::null());
    c
}

fn run(dir: &Path, args: &[&str]) -> Output {
    arks(dir).args(args).output().expect("run arks")
}

fn ok(dir: &Path, args: &[&str]) -> String {
    let out = run(dir, args);
    assert!(
        out.status.success(),
        "{args:?} failed: {}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// A repository with three commits; the third carries trailers.
fn repo(root: &Path) -> PathBuf {
    let d = root.join("repo");
    std::fs::create_dir_all(&d).unwrap();
    ok(&d, &["init", "-q", "-b", "main"]);
    ok(&d, &["config", "commit.gpgsign", "false"]);
    std::fs::write(d.join("a.txt"), "one\n").unwrap();
    ok(&d, &["add", "a.txt"]);
    ok(&d, &["commit", "-q", "-m", "first change"]);
    std::fs::write(d.join("a.txt"), "one\ntwo\n").unwrap();
    ok(&d, &["commit", "-q", "-am", "second change"]);
    std::fs::write(d.join("a.txt"), "one\ntwo\nthree\n").unwrap();
    ok(
        &d,
        &[
            "commit",
            "-q",
            "-am",
            "third change\n\nSigned-off-by: S O B <sob@example.com>\nAcked-by: A C K <ack@example.com>\nCc: B C <bodycc@example.com>",
        ],
    );
    d
}

/// Common arguments: fixture server, fixed EHLO domain, no prompts.
fn base(port: &str) -> Vec<&str> {
    vec![
        "send-email",
        "--smtp-server=127.0.0.1",
        "--smtp-server-port",
        port,
        "--smtp-domain=client.example.com",
        "--from=Sender <sender@example.com>",
        "--confirm=never",
    ]
}

fn send(d: &Path, opts: ServerOpts, extra: &[&str]) -> (String, Capture) {
    let fx = Fixture::start(opts);
    let port = fx.port();
    let mut a = base(&port);
    a.extend_from_slice(extra);
    let out = ok(d, &a);
    (out, fx.finish())
}

fn msgs(cap: &Capture) -> Vec<Msg> {
    cap.mails.iter().map(|m| Msg::parse(&m.data)).collect()
}

// ----------------------------------------------------------------- tests

#[test]
fn envelope_session_and_headers_of_one_patch() {
    let t = tempfile::tempdir().unwrap();
    let d = repo(t.path());
    let (out, cap) = send(
        &d,
        ServerOpts::default(),
        &[
            "--to=One <one@example.com>",
            "--to=two@example.com,three@example.com",
            "--cc=cc@example.com",
            "--bcc=hidden@example.com",
            "--suppress-cc=all",
            // --subject only seeds --compose; patch subjects are kept.
            "--subject=ignored without compose",
            "-1",
        ],
    );
    // Session order (RFC 5321 §3.3): greeting, EHLO with the configured
    // FQDN, one transaction, QUIT.
    let tr: Vec<&str> = cap.transcript.iter().map(String::as_str).collect();
    assert_eq!(tr[0], "S: 220 fixture.example.com ESMTP q6-fixture");
    assert_eq!(tr[1], "C: EHLO client.example.com");
    let client: Vec<&str> = tr
        .iter()
        .filter(|l| l.starts_with("C: "))
        .copied()
        .collect();
    assert_eq!(
        client,
        vec![
            "C: EHLO client.example.com",
            "C: MAIL FROM:<sender@example.com>",
            "C: RCPT TO:<one@example.com>",
            "C: RCPT TO:<two@example.com>",
            "C: RCPT TO:<three@example.com>",
            "C: RCPT TO:<cc@example.com>",
            "C: RCPT TO:<hidden@example.com>",
            "C: DATA",
            "C: <message>",
            "C: .",
            "C: QUIT",
        ],
        "{tr:#?}"
    );
    assert_eq!(cap.bare_lf, 0, "every client line ends in CRLF");
    assert_eq!(cap.mails.len(), 1);
    let mail = &cap.mails[0];
    assert_eq!(mail.from, "sender@example.com");
    let m = Msg::parse(&mail.data);
    assert_eq!(m.get("From"), Some("Sender <sender@example.com>"));
    let to = m.get("To").unwrap();
    for a in [
        "One <one@example.com>",
        "two@example.com",
        "three@example.com",
    ] {
        assert!(to.contains(a), "{to}");
    }
    assert_eq!(m.get("Cc"), Some("cc@example.com"));
    assert_eq!(m.count("Bcc"), 0, "Bcc stays in the envelope only");
    assert!(!m.raw_headers.contains("hidden@example.com"));
    assert_eq!(m.get("Subject"), Some("[PATCH] third change"));
    parse_date(m.get("Date").expect("Date present"));
    assert_eq!(m.count("Date"), 1);
    assert_msg_id(m.get("Message-ID").unwrap(), DOMAIN);
    assert_eq!(m.count("Message-ID"), 1);
    assert!(
        m.get("X-Mailer")
            .is_some_and(|v| v.starts_with("git-send-email "))
    );
    // A single patch without --in-reply-to starts a thread.
    assert_eq!(m.count("In-Reply-To"), 0);
    assert_eq!(m.count("References"), 0);
    // The author differs from the sender: the body keeps the author first.
    let body = m.body_text();
    assert!(
        body.starts_with("From: A U Thor <author@example.com>\n\n"),
        "{body}"
    );
    assert!(
        body.contains("Signed-off-by: S O B <sob@example.com>\n"),
        "{body}"
    );
    assert!(body.contains("\n---\n"), "{body}");
    assert!(body.contains("+three\n"), "{body}");
    // Report on stdout.
    assert!(out.contains("OK. Log says:"), "{out}");
    assert!(out.contains("Server: 127.0.0.1"), "{out}");
    assert!(out.contains("MAIL FROM:<sender@example.com>"), "{out}");
    assert!(out.contains("Result: 250"), "{out}");
}

#[test]
fn helo_fallback_when_ehlo_is_refused() {
    let t = tempfile::tempdir().unwrap();
    let d = repo(t.path());
    let (_, cap) = send(
        &d,
        ServerOpts {
            reject_ehlo: true,
            ..ServerOpts::default()
        },
        &["--to=list@example.com", "--suppress-cc=all", "-1"],
    );
    let tr = cap.transcript.join("\n");
    assert!(tr.contains("C: EHLO client.example.com\nS: 502"), "{tr}");
    assert!(tr.contains("C: HELO client.example.com\nS: 250"), "{tr}");
    assert_eq!(cap.mails.len(), 1);
}

#[test]
fn auth_plain_and_login_without_tls() {
    for (mech, expect) in [
        ("PLAIN", "C: AUTH PLAIN <token>"),
        ("LOGIN", "C: AUTH LOGIN"),
    ] {
        let t = tempfile::tempdir().unwrap();
        let d = repo(t.path());
        let (_, cap) = send(
            &d,
            ServerOpts {
                auth: Some(mech),
                ..ServerOpts::default()
            },
            &[
                "--to=list@example.com",
                "--suppress-cc=all",
                "--smtp-user=fixture-user",
                "--smtp-pass=fixture-pass",
                "-1",
            ],
        );
        let tr = cap.transcript.join("\n");
        assert!(tr.contains(expect), "{mech}: {tr}");
        assert_eq!(
            cap.logins,
            vec![("fixture-user".to_string(), "fixture-pass".to_string())],
            "{mech}"
        );
        // AUTH comes after EHLO and before the transaction.
        let auth_at = tr.find("C: AUTH").unwrap();
        assert!(tr.find("C: EHLO").unwrap() < auth_at);
        assert!(auth_at < tr.find("C: MAIL FROM").unwrap());
        assert_eq!(cap.mails.len(), 1);
    }
}

fn three(cap: &Capture) -> (Vec<Msg>, Vec<String>) {
    let m = msgs(cap);
    assert_eq!(m.len(), 3);
    let ids: Vec<String> = m
        .iter()
        .map(|x| x.get("Message-ID").unwrap().to_string())
        .collect();
    for id in &ids {
        assert_msg_id(id, DOMAIN);
    }
    assert!(
        ids[0] != ids[1] && ids[1] != ids[2] && ids[0] != ids[2],
        "{ids:?}"
    );
    // Dates strictly increase so a mail reader sorts the series in order.
    let dates: Vec<i64> = m
        .iter()
        .map(|x| parse_date(x.get("Date").unwrap()))
        .collect();
    assert!(dates[0] < dates[1] && dates[1] < dates[2], "{dates:?}");
    for (i, x) in m.iter().enumerate() {
        assert_eq!(
            x.get("Subject"),
            Some(
                [
                    "[PATCH 1/3] first change",
                    "[PATCH 2/3] second change",
                    "[PATCH 3/3] third change"
                ][i]
            )
        );
    }
    (m, ids)
}

fn refs(m: &Msg) -> Vec<String> {
    m.get("References")
        .unwrap_or("")
        .split_whitespace()
        .map(str::to_string)
        .collect()
}

#[test]
fn shallow_threading_with_in_reply_to() {
    let t = tempfile::tempdir().unwrap();
    let d = repo(t.path());
    let (_, cap) = send(
        &d,
        ServerOpts::default(),
        &[
            "--to=list@example.com",
            "--suppress-cc=all",
            "--thread",
            "--no-chain-reply-to",
            "--in-reply-to=<root@example.com>",
            "-3",
        ],
    );
    let (m, ids) = three(&cap);
    let root = "<root@example.com>".to_string();
    assert_eq!(m[0].get("In-Reply-To"), Some(root.as_str()));
    assert_eq!(refs(&m[0]), vec![root.clone()]);
    for x in &m[1..] {
        assert_eq!(x.get("In-Reply-To"), Some(ids[0].as_str()));
        assert_eq!(refs(x), vec![root.clone(), ids[0].clone()]);
    }
    assert_eq!(cap.transcript.iter().filter(|l| *l == "C: QUIT").count(), 1);
    assert_eq!(
        cap.transcript
            .iter()
            .filter(|l| l.starts_with("C: EHLO"))
            .count(),
        1
    );
}

#[test]
fn shallow_threading_is_the_default_without_in_reply_to() {
    let t = tempfile::tempdir().unwrap();
    let d = repo(t.path());
    let (_, cap) = send(
        &d,
        ServerOpts::default(),
        &["--to=list@example.com", "--suppress-cc=all", "-3"],
    );
    let (m, ids) = three(&cap);
    assert_eq!(m[0].count("In-Reply-To"), 0);
    assert_eq!(m[0].count("References"), 0);
    for x in &m[1..] {
        assert_eq!(x.get("In-Reply-To"), Some(ids[0].as_str()));
        assert_eq!(refs(x), vec![ids[0].clone()]);
    }
}

#[test]
fn deep_threading_with_chain_reply_to() {
    let t = tempfile::tempdir().unwrap();
    let d = repo(t.path());
    let (_, cap) = send(
        &d,
        ServerOpts::default(),
        &[
            "--to=list@example.com",
            "--suppress-cc=all",
            "--chain-reply-to",
            "--in-reply-to=<root@example.com>",
            "-3",
        ],
    );
    let (m, ids) = three(&cap);
    let root = "<root@example.com>".to_string();
    assert_eq!(m[0].get("In-Reply-To"), Some(root.as_str()));
    assert_eq!(m[1].get("In-Reply-To"), Some(ids[0].as_str()));
    assert_eq!(m[2].get("In-Reply-To"), Some(ids[1].as_str()));
    assert_eq!(refs(&m[1]), vec![root.clone(), ids[0].clone()]);
    assert_eq!(refs(&m[2]), vec![root, ids[0].clone(), ids[1].clone()]);
}

#[test]
fn no_thread_adds_no_threading_headers() {
    let t = tempfile::tempdir().unwrap();
    let d = repo(t.path());
    let (_, cap) = send(
        &d,
        ServerOpts::default(),
        &[
            "--to=list@example.com",
            "--suppress-cc=all",
            "--no-thread",
            "-3",
        ],
    );
    let (m, _) = three(&cap);
    for x in &m {
        assert_eq!(x.count("In-Reply-To"), 0);
        assert_eq!(x.count("References"), 0);
    }
}

#[test]
fn no_thread_with_in_reply_to_replies_every_mail_to_it() {
    // "Make the first mail (or all the mails with --no-thread) appear as a
    // reply to the given Message-ID".
    let t = tempfile::tempdir().unwrap();
    let d = repo(t.path());
    let (_, cap) = send(
        &d,
        ServerOpts::default(),
        &[
            "--to=list@example.com",
            "--suppress-cc=all",
            "--no-thread",
            "--in-reply-to=<root@example.com>",
            "-3",
        ],
    );
    let (m, _) = three(&cap);
    for x in &m {
        assert_eq!(x.get("In-Reply-To"), Some("<root@example.com>"));
        assert_eq!(refs(x), vec!["<root@example.com>".to_string()]);
    }
}

#[test]
fn dry_run_opens_no_connection() {
    let t = tempfile::tempdir().unwrap();
    let d = repo(t.path());
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    l.set_nonblocking(true).unwrap();
    let port = l.local_addr().unwrap().port().to_string();
    let mut a = base(&port);
    a.extend_from_slice(&[
        "--dry-run",
        "--to=list@example.com",
        "--suppress-cc=all",
        "--in-reply-to=<root@example.com>",
        "-3",
    ]);
    let out = ok(&d, &a);
    assert!(
        matches!(l.accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock),
        "dry run connected to the server"
    );
    assert_eq!(out.matches("Dry-OK. Log says:").count(), 3, "{out}");
    assert!(out.contains("Subject: [PATCH 2/3] second change"), "{out}");
    assert!(out.contains("In-Reply-To: <root@example.com>"), "{out}");
    assert!(out.contains("RCPT TO:<list@example.com>"), "{out}");
    assert_eq!(out.matches("Message-ID: <").count(), 3, "{out}");
}

fn rcpts_with(d: &Path, suppress: &[&str]) -> Vec<String> {
    let mut extra = vec!["--to=list@example.com"];
    extra.extend_from_slice(suppress);
    extra.push("-1");
    let (_, cap) = send(d, ServerOpts::default(), &extra);
    assert_eq!(cap.mails.len(), 1);
    cap.mails[0].rcpts.clone()
}

#[test]
fn suppress_cc_categories_shape_the_envelope() {
    let t = tempfile::tempdir().unwrap();
    let d = repo(t.path());
    let all = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    // Default: author (mbox From), Signed-off-by, other -by and body Cc.
    assert_eq!(
        rcpts_with(&d, &[]),
        all(&[
            "list@example.com",
            "author@example.com",
            "sob@example.com",
            "ack@example.com",
            "bodycc@example.com"
        ])
    );
    assert_eq!(
        rcpts_with(&d, &["--suppress-cc=author"]),
        all(&[
            "list@example.com",
            "sob@example.com",
            "ack@example.com",
            "bodycc@example.com"
        ])
    );
    assert_eq!(
        rcpts_with(&d, &["--suppress-cc=sob"]),
        all(&[
            "list@example.com",
            "author@example.com",
            "ack@example.com",
            "bodycc@example.com"
        ])
    );
    assert_eq!(
        rcpts_with(&d, &["--suppress-cc=misc-by"]),
        all(&[
            "list@example.com",
            "author@example.com",
            "sob@example.com",
            "bodycc@example.com"
        ])
    );
    assert_eq!(
        rcpts_with(&d, &["--suppress-cc=bodycc"]),
        all(&[
            "list@example.com",
            "author@example.com",
            "sob@example.com",
            "ack@example.com"
        ])
    );
    assert_eq!(
        rcpts_with(&d, &["--suppress-cc=body"]),
        all(&["list@example.com", "author@example.com"])
    );
    assert_eq!(
        rcpts_with(&d, &["--suppress-cc=author", "--suppress-cc=body"]),
        all(&["list@example.com"])
    );
    assert_eq!(
        rcpts_with(&d, &["--suppress-cc=all"]),
        all(&["list@example.com"])
    );
}

#[test]
fn annotate_off_runs_no_editor() {
    // GIT_EDITOR is `false`: an editor run would fail the command.
    let t = tempfile::tempdir().unwrap();
    let d = repo(t.path());
    ok(&d, &["config", "sendemail.annotate", "true"]);
    let (_, cap) = send(
        &d,
        ServerOpts::default(),
        &[
            "--no-annotate",
            "--to=list@example.com",
            "--suppress-cc=all",
            "-2",
        ],
    );
    assert_eq!(cap.mails.len(), 2);
}

/// A hand-written patch mail without MIME headers.
fn patch_file(dir: &Path, name: &str, subject: &str, body: &str) -> PathBuf {
    let f = dir.join(name);
    std::fs::write(
        &f,
        format!(
            "From 0000000000000000000000000000000000000000 Mon Sep 17 00:00:00 2001\n\
             From: Sender <sender@example.com>\n\
             Date: Tue, 14 Nov 2023 22:13:20 +0000\n\
             Subject: {subject}\n\n{body}"
        ),
    )
    .unwrap();
    f
}

#[test]
fn non_ascii_content_is_declared_as_mime() {
    let t = tempfile::tempdir().unwrap();
    let body = "Caf\u{e9} na\u{ef}ve r\u{e9}sum\u{e9}.\n---\n a.txt | 1 +\n";
    let f = patch_file(t.path(), "0001.patch", "[PATCH] caf\u{e9} fix", body);
    let fs = f.to_string_lossy().into_owned();
    for (enc, cs) in [
        (None, "UTF-8"),
        (Some("--8bit-encoding=ISO-8859-1"), "ISO-8859-1"),
    ] {
        let mut extra = vec!["--to=list@example.com", "--suppress-cc=all"];
        extra.extend(enc);
        extra.push(&fs);
        let (_, cap) = send(t.path(), ServerOpts::default(), &extra);
        let m = &msgs(&cap)[0];
        assert_eq!(m.get("MIME-Version"), Some("1.0"));
        assert_eq!(
            m.get("Content-Type"),
            Some(format!("text/plain; charset={cs}").as_str())
        );
        assert_eq!(m.get("Content-Transfer-Encoding"), Some("8bit"));
        // RFC 2047: the raw header block is ASCII, the subject an encoded word.
        assert!(m.raw_headers.is_ascii(), "{}", m.raw_headers);
        assert_eq!(
            m.get("Subject"),
            Some(format!("=?{cs}?q?=5BPATCH=5D_caf=C3=A9_fix?=").as_str())
        );
        // 8bit: the body bytes go through unchanged.
        assert_eq!(m.body_text(), body);
    }
}

#[test]
fn format_patch_non_ascii_output_gets_one_mime_block() {
    let t = tempfile::tempdir().unwrap();
    let d = repo(t.path());
    std::fs::write(d.join("a.txt"), "caf\u{e9}\n").unwrap();
    ok(&d, &["commit", "-q", "-am", "caf\u{e9} na\u{ef}ve"]);
    let (_, cap) = send(
        &d,
        ServerOpts::default(),
        &["--to=list@example.com", "--suppress-cc=all", "-1"],
    );
    let m = &msgs(&cap)[0];
    assert_eq!(m.count("MIME-Version"), 1, "{:?}", m.names());
    assert_eq!(m.count("Content-Type"), 1, "{:?}", m.names());
    assert_eq!(m.count("Content-Transfer-Encoding"), 1, "{:?}", m.names());
    assert!(m.raw_headers.is_ascii(), "{}", m.raw_headers);
    assert!(m.body_text().contains("+caf\u{e9}\n"));
}

fn qp_decode(s: &str) -> Vec<u8> {
    let s = s.replace("=\n", "");
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'=' {
            out.push(u8::from_str_radix(&s[i + 1..i + 3], 16).unwrap());
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    out
}

#[test]
fn transfer_encodings() {
    let t = tempfile::tempdir().unwrap();
    let body = format!("Caf\u{e9} = coffee \n---\n{}\n", "x".repeat(120));
    let f = patch_file(t.path(), "0001.patch", "[PATCH] enc", &body);
    let fs = f.to_string_lossy().into_owned();
    // quoted-printable: every encoded line fits 76 and decodes back.
    let (_, cap) = send(
        t.path(),
        ServerOpts::default(),
        &[
            "--to=l@example.com",
            "--suppress-cc=all",
            "--transfer-encoding=quoted-printable",
            &fs,
        ],
    );
    let m = &msgs(&cap)[0];
    assert_eq!(m.get("Content-Transfer-Encoding"), Some("quoted-printable"));
    assert_eq!(m.get("Content-Type"), Some("text/plain; charset=UTF-8"));
    let wire = m.body_text();
    assert!(
        wire.is_ascii() && wire.lines().all(|l| l.len() <= 76),
        "{wire}"
    );
    assert_eq!(String::from_utf8(qp_decode(&wire)).unwrap(), body);
    // base64.
    let (_, cap) = send(
        t.path(),
        ServerOpts::default(),
        &[
            "--to=l@example.com",
            "--suppress-cc=all",
            "--transfer-encoding=base64",
            &fs,
        ],
    );
    let m = &msgs(&cap)[0];
    assert_eq!(m.get("Content-Transfer-Encoding"), Some("base64"));
    let joined: String = m.body_text().split_whitespace().collect();
    let dec = base64::engine::general_purpose::STANDARD
        .decode(joined)
        .unwrap();
    assert_eq!(String::from_utf8(dec).unwrap(), body);
    // auto: 8bit when every line fits the SMTP limit ...
    let (_, cap) = send(
        t.path(),
        ServerOpts::default(),
        &["--to=l@example.com", "--suppress-cc=all", &fs],
    );
    assert_eq!(msgs(&cap)[0].get("Content-Transfer-Encoding"), Some("8bit"));
    // ... and quoted-printable otherwise.
    let long = patch_file(
        t.path(),
        "long.patch",
        "[PATCH] long",
        &format!("{}\n", "y".repeat(1200)),
    );
    let ls = long.to_string_lossy().into_owned();
    let (_, cap) = send(
        t.path(),
        ServerOpts::default(),
        &["--to=l@example.com", "--suppress-cc=all", &ls],
    );
    let m = &msgs(&cap)[0];
    assert_eq!(m.get("Content-Transfer-Encoding"), Some("quoted-printable"));
    assert!(cap.raw_data_lines.iter().all(|l| l.len() <= 998));
    // 7bit refuses a non-ASCII message before anything is sent.
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    l.set_nonblocking(true).unwrap();
    let port = l.local_addr().unwrap().port().to_string();
    let mut a = base(&port);
    a.extend_from_slice(&[
        "--to=l@example.com",
        "--suppress-cc=all",
        "--transfer-encoding=7bit",
        &fs,
    ]);
    let out = run(t.path(), &a);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("7bit"));
    assert!(matches!(l.accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock));
}

#[test]
fn data_lines_are_dot_stuffed() {
    let t = tempfile::tempdir().unwrap();
    let body = "Leading dots.\n.hidden\n..two\n.\nend\n";
    let f = patch_file(t.path(), "0001.patch", "[PATCH] dots", body);
    let fs = f.to_string_lossy().into_owned();
    let (_, cap) = send(
        t.path(),
        ServerOpts::default(),
        &["--to=l@example.com", "--suppress-cc=all", &fs],
    );
    for l in ["..hidden", "...two", ".."] {
        assert!(
            cap.raw_data_lines.iter().any(|x| x == l),
            "{l}: {:?}",
            cap.raw_data_lines
        );
    }
    assert_eq!(msgs(&cap)[0].body_text(), body);
}

#[test]
fn batch_size_reconnects() {
    let t = tempfile::tempdir().unwrap();
    let d = repo(t.path());
    let (_, cap) = send(
        &d,
        ServerOpts {
            sessions: 2,
            ..ServerOpts::default()
        },
        &[
            "--to=list@example.com",
            "--suppress-cc=all",
            "--batch-size=2",
            "--relogin-delay=0",
            "-3",
        ],
    );
    assert_eq!(cap.mails.len(), 3);
    assert_eq!(
        cap.transcript
            .iter()
            .filter(|l| l.starts_with("C: EHLO"))
            .count(),
        2
    );
    assert_eq!(cap.transcript.iter().filter(|l| *l == "C: QUIT").count(), 2);
}
