//! SMTP submission for `send-email` (RFC 5321 `EHLO`/`HELO`, `MAIL FROM`,
//! `RCPT TO`, `DATA` with dot-stuffing, `QUIT`; RFC 3207 `STARTTLS`;
//! RFC 4954 `AUTH` with PLAIN, LOGIN or CRAM-MD5).

use super::super::net::{Conn, hex, hmac_md5};
use crate::error::{CliError, CliResult};
use base64::Engine;

pub struct Settings {
    pub server: String,
    pub port: u16,
    /// `ssl` (SMTPS) or `tls` (STARTTLS); anything else is plain.
    pub encryption: String,
    pub domain: String,
    pub user: Option<String>,
    pub pass: Option<String>,
    /// Allowed mechanisms (`--smtp-auth`), empty = any; `none` disables.
    pub auth: Vec<String>,
    pub verify: bool,
    pub debug: bool,
}

pub struct Session {
    c: Conn,
}

fn fail(msg: impl Into<String>) -> CliError {
    CliError::Exit(1, msg.into())
}

/// Read one (possibly multi-line) reply: code and text lines.
fn reply(c: &mut Conn) -> CliResult<(u16, Vec<String>)> {
    let mut lines = Vec::new();
    loop {
        let Some(l) = c.line()? else {
            return Err(fail(
                "Unable to initialize SMTP properly: connection closed",
            ));
        };
        let code: u16 = l.get(..3).and_then(|s| s.parse().ok()).unwrap_or(0);
        let more = l.as_bytes().get(3) == Some(&b'-');
        lines.push(l.get(4..).unwrap_or("").to_string());
        if !more {
            return Ok((code, lines));
        }
    }
}

fn expect(c: &mut Conn, ok: &[u16], what: &str) -> CliResult<Vec<String>> {
    let (code, lines) = reply(c)?;
    if !ok.contains(&code) {
        return Err(fail(format!(
            "Failed to {what}: {code} {}",
            lines.join(" ")
        )));
    }
    Ok(lines)
}

impl Session {
    pub fn open(s: &Settings) -> CliResult<Self> {
        let ssl = s.encryption == "ssl";
        let mut c = Conn::tcp(&s.server, s.port, ssl, s.verify)?;
        c.debug = s.debug;
        expect(&mut c, &[220], "initialize SMTP")?;
        let mut caps = ehlo(&mut c, &s.domain)?;
        if s.encryption == "tls" {
            c.cmd("STARTTLS", None)?;
            expect(&mut c, &[220], "start TLS")?;
            c.starttls()?;
            caps = ehlo(&mut c, &s.domain)?;
        }
        let none = s.auth.iter().any(|m| m.eq_ignore_ascii_case("none"));
        if let (Some(user), false) = (&s.user, none) {
            let pass = s.pass.clone().unwrap_or_default();
            login(&mut c, &caps, &s.auth, user, &pass)?;
        }
        Ok(Self { c })
    }

    /// Submit one message.
    pub fn send(&mut self, from: &str, rcpts: &[String], data: &str) -> CliResult<u16> {
        let c = &mut self.c;
        c.cmd(&format!("MAIL FROM:<{from}>"), None)?;
        expect(c, &[250], "send MAIL FROM")?;
        for r in rcpts {
            c.cmd(&format!("RCPT TO:<{r}>"), None)?;
            expect(c, &[250, 251], &format!("send RCPT TO <{r}>"))?;
        }
        c.cmd("DATA", None)?;
        expect(c, &[354], "send DATA")?;
        let mut out = String::new();
        for l in data.split_inclusive('\n') {
            let l = l.trim_end_matches('\n').trim_end_matches('\r');
            if l.starts_with('.') {
                out.push('.');
            }
            out.push_str(l);
            out.push_str("\r\n");
        }
        out.push_str(".\r\n");
        c.send(out.as_bytes())?;
        let (code, lines) = reply(c)?;
        if code != 250 {
            return Err(fail(format!(
                "Failed to send the message: {code} {}",
                lines.join(" ")
            )));
        }
        Ok(code)
    }

    pub fn quit(mut self) {
        let _ = self.c.cmd("QUIT", None);
        let _ = reply(&mut self.c);
        self.c.close();
    }
}

fn ehlo(c: &mut Conn, domain: &str) -> CliResult<Vec<String>> {
    c.cmd(&format!("EHLO {domain}"), None)?;
    let (code, lines) = reply(c)?;
    if code == 250 {
        return Ok(lines
            .into_iter()
            .skip(1)
            .map(|l| l.to_ascii_uppercase())
            .collect());
    }
    c.cmd(&format!("HELO {domain}"), None)?;
    expect(c, &[250], "send HELO")?;
    Ok(Vec::new())
}

fn login(
    c: &mut Conn,
    caps: &[String],
    allowed: &[String],
    user: &str,
    pass: &str,
) -> CliResult<()> {
    let offered: Vec<String> = caps
        .iter()
        .filter_map(|l| {
            l.strip_prefix("AUTH")
                .map(|r| r.trim_start_matches(['=', ' ']))
        })
        .flat_map(|r| r.split_whitespace().map(str::to_string))
        .collect();
    let ok = |m: &str| {
        offered.iter().any(|o| o == m)
            && (allowed.is_empty() || allowed.iter().any(|a| a.eq_ignore_ascii_case(m)))
    };
    let b64 = base64::engine::general_purpose::STANDARD;
    if ok("CRAM-MD5") {
        c.cmd("AUTH CRAM-MD5", None)?;
        let ch = expect(c, &[334], "authenticate")?.join("");
        let challenge = b64
            .decode(ch.trim())
            .map_err(|_| fail("bad CRAM-MD5 challenge"))?;
        let resp = format!("{user} {}", hex(&hmac_md5(pass.as_bytes(), &challenge)));
        c.cmd(&b64.encode(resp), Some("<response>"))?;
    } else if ok("PLAIN") {
        let token = b64.encode(format!("\0{user}\0{pass}"));
        c.cmd(
            &format!("AUTH PLAIN {token}"),
            Some("AUTH PLAIN <credentials>"),
        )?;
    } else if ok("LOGIN") {
        c.cmd("AUTH LOGIN", None)?;
        expect(c, &[334], "authenticate")?;
        c.cmd(&b64.encode(user), Some("<user>"))?;
        expect(c, &[334], "authenticate")?;
        c.cmd(&b64.encode(pass), Some("<password>"))?;
    } else {
        return Err(fail(
            "No supported authentication method was offered by the SMTP server",
        ));
    }
    expect(c, &[235], "authenticate").map(|_| ())
}
