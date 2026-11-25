//! `git send-email`: send patches (files, directories of patches, or the
//! output of `format-patch` for revision arguments) as mail.
//!
//! Primary: <https://git-scm.com/docs/git-send-email> (composing, sending,
//! automating, administering options and `sendemail.*` configuration,
//! `sendemail.<identity>.*`, the `sendemail-validate` hook, default
//! transport: a `sendmail` found in `/usr/sbin`, `/usr/lib` or `$PATH`,
//! else SMTP to `localhost`; ports 25 / 465 (`ssl`)). Messages go through
//! `sh -c "<sendmail-cmd> -i <recipients>"` or arks' SMTP client; arks
//! never runs `git` to prepare them (`format-patch` runs in-process).

mod message;
mod smtp;

use super::proc;
use crate::cmd::opt_parse::{Item, Mode, OptDef, Takes::*, o, parse};
use crate::error::{CliError, CliResult};
use arks_config::ConfigSet;
use message::{Parsed, Suppress, auto_cc, bare, split_addrs};
use std::fs;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};

const OPTS: &[OptDef] = &[
    o("annotate", ' ', No, true),
    o("compose", ' ', No, true),
    o("from", ' ', Req, true),
    o("reply-to", ' ', Req, true),
    o("in-reply-to", ' ', Req, true),
    o("subject", ' ', Req, true),
    o("to", ' ', Req, true),
    o("cc", ' ', Req, true),
    o("bcc", ' ', Req, true),
    o("8bit-encoding", ' ', Req, true),
    o("compose-encoding", ' ', Req, true),
    o("transfer-encoding", ' ', Req, true),
    o("xmailer", ' ', No, true),
    o("envelope-sender", ' ', Req, true),
    o("sendmail-cmd", ' ', Req, true),
    o("smtp-encryption", ' ', Req, true),
    o("smtp-domain", ' ', Req, true),
    o("smtp-auth", ' ', Req, true),
    o("smtp-pass", ' ', Opt, true),
    o("smtp-server", ' ', Req, true),
    o("smtp-server-port", ' ', Req, true),
    o("smtp-server-option", ' ', Req, true),
    o("smtp-ssl", ' ', No, true),
    o("smtp-ssl-cert-path", ' ', Req, true),
    o("smtp-user", ' ', Req, true),
    o("smtp-debug", ' ', Req, true),
    o("batch-size", ' ', Req, true),
    o("relogin-delay", ' ', Req, true),
    o("identity", ' ', Req, true),
    o("to-cmd", ' ', Req, true),
    o("cc-cmd", ' ', Req, true),
    o("header-cmd", ' ', Req, true),
    o("chain-reply-to", ' ', No, true),
    o("signed-off-by-cc", ' ', No, true),
    o("signed-off-cc", ' ', No, true),
    o("cc-cover", ' ', No, true),
    o("to-cover", ' ', No, true),
    o("suppress-cc", ' ', Req, true),
    o("suppress-from", ' ', No, true),
    o("thread", ' ', No, true),
    o("confirm", ' ', Req, true),
    o("dry-run", ' ', No, true),
    o("format-patch", ' ', No, true),
    o("quiet", ' ', No, true),
    o("validate", ' ', No, true),
    o("force", ' ', No, true),
    o("dump-aliases", ' ', No, false),
];

fn die(m: impl Into<String>) -> CliError {
    CliError::Exit(1, m.into())
}

/// Option value, else `sendemail.<identity>.<key>`, else `sendemail.<key>`.
struct Conf<'a> {
    cfg: &'a ConfigSet,
    identity: Option<String>,
}

impl Conf<'_> {
    fn get(&self, key: &str) -> Option<String> {
        self.identity
            .as_ref()
            .and_then(|i| self.cfg.get(&format!("sendemail.{i}.{key}")))
            .or_else(|| self.cfg.get(&format!("sendemail.{key}")))
    }
    fn all(&self, key: &str) -> Vec<String> {
        let own = self
            .identity
            .as_ref()
            .map(|i| self.cfg.get_all(&format!("sendemail.{i}.{key}")))
            .unwrap_or_default();
        if own.is_empty() {
            self.cfg.get_all(&format!("sendemail.{key}"))
        } else {
            own
        }
    }
    fn flag(&self, key: &str) -> Option<bool> {
        self.get(key).and_then(|v| arks_config::parse_bool(&v))
    }
}

/// Values of a repeatable option split on commas; `--no-x` clears config.
fn list(
    p: &crate::cmd::opt_parse::Parsed,
    key: &'static str,
    conf: &Conf<'_>,
    ckey: &str,
) -> Vec<String> {
    let mut out = Vec::new();
    let mut cleared = false;
    let mut cli = Vec::new();
    for it in &p.items {
        if let Item::Opt { key: k, val, neg } = it {
            if *k == key {
                if *neg {
                    cleared = true;
                    cli.clear();
                } else if let Some(v) = val {
                    cli.push(v.clone());
                }
            }
        }
    }
    if !cleared {
        for v in conf.all(ckey) {
            out.extend(split_addrs(&v));
        }
    }
    for v in cli {
        out.extend(split_addrs(&v));
    }
    out
}

fn ident_default() -> String {
    match crate::cmd::shared::commit_signature() {
        Ok(s) => format!("{} <{}>", s.name, s.email),
        Err(_) => String::new(),
    }
}

pub fn cmd_send_email(args: &[String]) -> CliResult<()> {
    let p = parse(
        "send-email",
        args,
        OPTS,
        Mode {
            keep_unknown: true,
            ..Mode::default()
        },
    )?;
    let repo = crate::util::open_repo().ok();
    let cfg = ConfigSet::load(repo.as_ref().map(|r| r.common_dir()));
    let identity = match p.last("identity") {
        Some(v) => v,
        None => cfg.get("sendemail.identity"),
    };
    let conf = Conf {
        cfg: &cfg,
        identity,
    };
    if p.on("dump-aliases") {
        return dump_aliases(&conf);
    }
    let tmp = proc::temp_dir("arks-send-email-")?;
    let r = run(&p, &conf, repo.as_ref(), &tmp);
    let _ = fs::remove_dir_all(&tmp);
    r
}

fn run(
    p: &crate::cmd::opt_parse::Parsed,
    conf: &Conf<'_>,
    repo: Option<&arks_repo::Repository>,
    tmp: &Path,
) -> CliResult<()> {
    let flag = |k: &str, ck: &str| p.flag(k).or_else(|| conf.flag(ck));
    let dry = p.on("dry-run");
    let quiet = p.on("quiet");
    let editor = proc::editor(conf.cfg);
    // Inputs: files, directories, else format-patch arguments.
    let mut files: Vec<PathBuf> = Vec::new();
    let mut rev_args: Vec<String> = Vec::new();
    let fp = p.flag("format-patch");
    for it in &p.items {
        match it {
            Item::Arg(a) | Item::AfterDashDash(a) => {
                let path = Path::new(a);
                let is_file = path.exists();
                let is_rev = repo.is_some_and(|r| crate::cmd::shared::resolve_rev(r, a).is_ok());
                if is_file && is_rev && fp.is_none() {
                    return Err(die(format!(
                        "File '{a}' exists but it could also be the range of commits\nto produce patches for.  Please disambiguate by...\n\n    * Saying \"./{a}\" if you mean a file; or\n    * Giving --format-patch option if you mean a range."
                    )));
                }
                if is_file && fp != Some(true) {
                    if path.is_dir() {
                        let mut v: Vec<PathBuf> = fs::read_dir(path)?
                            .filter_map(|e| e.ok().map(|e| e.path()))
                            .filter(|p| {
                                p.is_file()
                                    && !p
                                        .file_name()
                                        .is_some_and(|n| n.to_string_lossy().starts_with('.'))
                            })
                            .collect();
                        v.sort();
                        files.extend(v);
                    } else {
                        files.push(path.to_path_buf());
                    }
                } else {
                    rev_args.push(a.clone());
                }
            }
            Item::Unknown(u) => rev_args.push(u.clone()),
            _ => {}
        }
    }
    if !rev_args.is_empty() {
        if repo.is_none() {
            return Err(CliError::fatal("not a git repository"));
        }
        let out = tmp.join("patches");
        let mut a = vec!["-o".to_string(), out.to_string_lossy().into_owned()];
        a.extend(rev_args);
        crate::cmd::cmd_format_patch(&a)?;
        let mut v: Vec<PathBuf> = fs::read_dir(&out)
            .map(|d| d.filter_map(|e| e.ok().map(|e| e.path())).collect())
            .unwrap_or_default();
        v.sort();
        files.extend(v);
    }
    // Messages (an mbox file can hold several).
    let mut msgs: Vec<String> = Vec::new();
    for f in &files {
        let raw = fs::read(f).map_err(|e| die(format!("fatal: {}: {e}", f.display())))?;
        msgs.extend(message::split_mbox(&String::from_utf8_lossy(&raw)));
    }
    if msgs.is_empty() && !p.on("compose") {
        return Err(die("No patch files specified!"));
    }
    // Validation: the hook, then the SMTP line-length limit.
    let encoding = p
        .value("transfer-encoding")
        .or_else(|| conf.get("transferEncoding"))
        .unwrap_or_else(|| "auto".into())
        .to_ascii_lowercase();
    if flag("validate", "validate").unwrap_or(true) {
        validate(repo, &files, &encoding)?;
    }
    let tty = std::io::stdin().is_terminal();
    let from = match p.last("from").flatten().or_else(|| conf.get("from")) {
        Some(f) => f,
        None => {
            let def = ident_default();
            if tty {
                match proc::ask(&format!(
                    "Who should the emails appear to be from? [{def}] "
                )) {
                    Some(a) if !a.trim().is_empty() => a.trim().to_string(),
                    _ => def,
                }
            } else {
                def
            }
        }
    };
    let mut to = list(p, "to", conf, "to");
    let cc = list(p, "cc", conf, "cc");
    let bcc = list(p, "bcc", conf, "bcc");
    let to_cmd = p.last("to-cmd").flatten().or_else(|| conf.get("tocmd"));
    if to.is_empty() && to_cmd.is_none() && tty {
        if let Some(a) = proc::ask("To whom should the emails be sent (if anyone)? ") {
            to.extend(split_addrs(&a));
        }
    }
    let thread = flag("thread", "thread").unwrap_or(true);
    let chain = flag("chain-reply-to", "chainReplyTo").unwrap_or(false);
    let mut in_reply_to = p.last("in-reply-to").flatten();
    if in_reply_to.is_none() && thread && tty && !p.on("compose") {
        in_reply_to =
            proc::ask("Message-ID to be used as In-Reply-To for the first email (if any)? ")
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());
    }
    // Automatic Cc rules.
    let mut sup = Suppress::default();
    let mut sup_given = false;
    let sup_values: Vec<String> = match p.values("suppress-cc") {
        v if !v.is_empty() => v,
        _ => conf.all("suppresscc"),
    };
    for v in &sup_values {
        sup_given = true;
        if !sup.add(v) {
            return Err(die(format!("Unknown --suppress-cc field: '{v}'")));
        }
    }
    let suppress_from = flag("suppress-from", "suppressFrom").unwrap_or(false);
    let sob = p
        .flag("signed-off-by-cc")
        .or_else(|| p.flag("signed-off-cc"))
        .or_else(|| conf.flag("signedoffbycc"))
        .or_else(|| conf.flag("signedoffcc"))
        .unwrap_or(true);
    if sup_values.is_empty() {
        if suppress_from {
            sup.add("self");
        }
        if !sob {
            sup.add("body");
        }
    }
    let confirm = p
        .last("confirm")
        .flatten()
        .or_else(|| conf.get("confirm"))
        .unwrap_or_else(|| {
            if sup_given {
                "compose".into()
            } else {
                "auto".into()
            }
        });
    if !matches!(
        confirm.as_str(),
        "always" | "never" | "cc" | "compose" | "auto"
    ) {
        return Err(die(format!("Unknown --confirm setting: '{confirm}'")));
    }
    // --compose: an introductory message edited first.
    if p.on("compose") {
        let (intro, irt) = compose(p, &editor, &from, in_reply_to.as_deref(), tmp)?;
        if irt.is_some() {
            in_reply_to = irt;
        }
        if let Some(m) = intro {
            msgs.insert(0, m);
        }
        if msgs.is_empty() {
            return Err(die("No patch files specified!"));
        }
    }
    if flag("annotate", "annotate").unwrap_or(false) {
        for (i, m) in msgs.iter_mut().enumerate() {
            let f = tmp.join(format!("annotate-{i}"));
            fs::write(&f, &*m)?;
            proc::edit(&editor, &f)?;
            *m = String::from_utf8_lossy(&fs::read(&f)?).into_owned();
        }
    }
    let cc_cmd = p.last("cc-cmd").flatten().or_else(|| conf.get("ccCmd"));
    let header_cmd = p
        .last("header-cmd")
        .flatten()
        .or_else(|| conf.get("headerCmd"));
    let xmailer = flag("xmailer", "xmailer").unwrap_or(true);
    let reply_to = p.last("reply-to").flatten();
    let transport = Transport::resolve(p, conf, dry)?;
    let envelope = match p
        .last("envelope-sender")
        .flatten()
        .or_else(|| conf.get("envelopeSender"))
    {
        Some(e) if e == "auto" => Some(bare(&from)),
        Some(e) if !e.is_empty() => Some(bare(&e)),
        _ => None,
    };
    let batch: usize = p
        .value("batch-size")
        .or_else(|| conf.get("smtpBatchSize"))
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let relogin: u64 = p
        .value("relogin-delay")
        .or_else(|| conf.get("smtpReloginDelay"))
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let cover_cc = flag("cc-cover", "cccover").unwrap_or(false);
    let cover_to = flag("to-cover", "tocover").unwrap_or(false);
    let domain = transport.domain.clone();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let total = msgs.len() as i64;
    let mut first_id: Option<String> = None;
    let mut prev_id: Option<String> = None;
    let mut references: Vec<String> = in_reply_to.iter().cloned().collect();
    let mut send_all = confirm == "never";
    let mut session: Option<smtp::Session> = None;
    let mut sent_in_batch = 0usize;
    let mut cover_cc_list: Vec<String> = Vec::new();
    let mut cover_to_list: Vec<String> = Vec::new();
    'msgs: for (i, raw) in msgs.iter().enumerate() {
        let parsed = message::parse(raw);
        let (mut mto, mut mcc) = (to.clone(), cc.clone());
        let mut notes = Vec::new();
        for (a, note) in auto_cc(&parsed, &sup, &from, sob) {
            notes.push(note);
            mcc.push(a);
        }
        if let (Some(cmd), false) = (&cc_cmd, sup.cccmd) {
            if let Some(f) = files.get(i) {
                for a in command_lines(cmd, f)? {
                    notes.push(format!("(cc-cmd) Adding cc: {a} from: '{cmd}'"));
                    mcc.push(a);
                }
            }
        }
        if let Some(cmd) = &to_cmd {
            if let Some(f) = files.get(i) {
                for a in command_lines(cmd, f)? {
                    notes.push(format!("(to-cmd) Adding to: {a} from: '{cmd}'"));
                    mto.push(a);
                }
            }
        }
        for t in parsed.get_all("To") {
            mto.extend(split_addrs(&t));
        }
        if i == 0 {
            cover_cc_list = mcc.clone();
            cover_to_list = mto.clone();
        } else {
            if cover_cc {
                mcc.extend(cover_cc_list.iter().cloned());
            }
            if cover_to {
                mto.extend(cover_to_list.iter().cloned());
            }
        }
        dedupe(&mut mto, &[]);
        let to_bare: Vec<String> = mto.iter().map(|a| bare(a)).collect();
        dedupe(&mut mcc, &to_bare);
        if suppress_from || sup.self_ {
            let me = bare(&from);
            mcc.retain(|a| !bare(a).eq_ignore_ascii_case(&me));
        }
        // Headers.
        let subject = parsed.get("Subject").unwrap_or("").to_string();
        let date = message::rfc2822(now - total + i as i64 + 1, 0);
        let id = format!("<{}-{}-{}@{domain}>", now, i + 1, std::process::id());
        let mut h: Vec<(String, String)> = vec![("From".into(), from.clone())];
        if !mto.is_empty() {
            h.push(("To".into(), mto.join(",\n\t")));
        }
        if !mcc.is_empty() {
            h.push(("Cc".into(), mcc.join(",\n\t")));
        }
        h.push(("Subject".into(), subject.clone()));
        h.push(("Date".into(), date));
        h.push(("Message-ID".into(), id.clone()));
        if xmailer {
            h.push((
                "X-Mailer".into(),
                format!("git-send-email {}", env!("CARGO_PKG_VERSION")),
            ));
        }
        let existing_irt = parsed.get("In-Reply-To").map(str::to_string);
        if thread && existing_irt.is_none() {
            let target = if i == 0 {
                in_reply_to.clone()
            } else if chain {
                prev_id.clone()
            } else {
                first_id.clone().or_else(|| in_reply_to.clone())
            };
            if let Some(t) = target {
                h.push(("In-Reply-To".into(), t));
                h.push(("References".into(), references.join("\n\t")));
            }
        } else if let (false, None, Some(t)) = (thread, &existing_irt, &in_reply_to) {
            // `--no-thread` with `--in-reply-to`: every mail replies to it.
            h.push(("In-Reply-To".into(), t.clone()));
            h.push(("References".into(), t.clone()));
        }
        if let Some(r) = &reply_to {
            h.push(("Reply-To".into(), r.clone()));
        }
        let skip = [
            "from",
            "to",
            "cc",
            "bcc",
            "subject",
            "date",
            "message-id",
            "x-mailer",
            "references",
            "reply-to",
        ];
        for (k, v) in &parsed.headers {
            if !skip.contains(&k.to_ascii_lowercase().as_str()) {
                h.push((k.clone(), v.clone()));
            }
        }
        if let (Some(cmd), Some(f)) = (&header_cmd, files.get(i)) {
            for l in command_lines(cmd, f)? {
                if let Some((k, v)) = l.split_once(':') {
                    h.push((k.trim().to_string(), v.trim().to_string()));
                }
            }
        }
        let mut body = parsed.body.clone();
        if let Some(author) = parsed.get("From") {
            if bare(author) != bare(&from) {
                body = format!("From: {author}\n\n{body}");
            }
        }
        if !body.is_ascii() && parsed.get("Content-Type").is_none() {
            h.push(("MIME-Version".into(), "1.0".into()));
            h.push(("Content-Type".into(), "text/plain; charset=UTF-8".into()));
            h.push(("Content-Transfer-Encoding".into(), "8bit".into()));
        }
        let header_text: String = h.iter().map(|(k, v)| format!("{k}: {v}\n")).collect();
        let data = format!("{header_text}\n{body}");
        let mut rcpts: Vec<String> = mto
            .iter()
            .chain(&mcc)
            .chain(&bcc)
            .map(|a| bare(a))
            .collect();
        dedupe(&mut rcpts, &[]);
        let sender = envelope.clone().unwrap_or_else(|| bare(&from));
        // Report and confirm.
        if !quiet {
            for n in &notes {
                println!("{n}");
            }
        }
        let needs_confirm = !send_all
            && match confirm.as_str() {
                "always" => true,
                "cc" | "auto" => !notes.is_empty(),
                _ => false,
            };
        if needs_confirm {
            print!("\n{header_text}\n");
            loop {
                let a = proc::ask("Send this email? ([y]es|[n]o|[e]dit|[q]uit|[a]ll): ");
                match a
                    .as_deref()
                    .map(|s| s.trim().to_ascii_lowercase())
                    .as_deref()
                {
                    Some("y") | Some("yes") | Some("") => break,
                    Some("a") | Some("all") => {
                        send_all = true;
                        break;
                    }
                    Some("n") | Some("no") => continue 'msgs,
                    None | Some("q") | Some("quit") => return Ok(()),
                    _ => {}
                }
            }
        }
        // Send.
        let result = if dry {
            "OK".to_string()
        } else {
            match &transport.kind {
                Kind::Sendmail(cmd) => {
                    sendmail(cmd, &sender, envelope.is_some(), &rcpts, &data)?;
                    "OK".to_string()
                }
                Kind::Smtp(s) => {
                    if batch > 0 && sent_in_batch == batch {
                        if let Some(old) = session.take() {
                            old.quit();
                        }
                        std::thread::sleep(std::time::Duration::from_secs(relogin));
                        sent_in_batch = 0;
                    }
                    if session.is_none() {
                        session = Some(smtp::Session::open(s)?);
                    }
                    let code = session
                        .as_mut()
                        .map(|ss| ss.send(&sender, &rcpts, &data))
                        .unwrap_or(Ok(250))?;
                    sent_in_batch += 1;
                    code.to_string()
                }
            }
        };
        if quiet {
            println!("{}Sent {subject}", if dry { "Dry-" } else { "" });
        } else {
            println!("{}OK. Log says:", if dry { "Dry-" } else { "" });
            match &transport.kind {
                Kind::Sendmail(cmd) => {
                    println!("Sendmail: {cmd} -i {}", rcpts.join(" "))
                }
                Kind::Smtp(s) => {
                    println!("Server: {}", s.server);
                    println!("MAIL FROM:<{sender}>");
                    println!(
                        "RCPT TO:{}",
                        rcpts
                            .iter()
                            .map(|r| format!("<{r}>"))
                            .collect::<Vec<_>>()
                            .join(",")
                    );
                }
            }
            print!("{header_text}");
            println!("\nResult: {result}\n");
        }
        if first_id.is_none() {
            first_id = Some(id.clone());
        }
        if chain || i == 0 {
            references.push(id.clone());
        }
        prev_id = Some(id);
    }
    if let Some(s) = session {
        s.quit();
    }
    Ok(())
}

fn dedupe(v: &mut Vec<String>, exclude: &[String]) {
    let mut seen: Vec<String> = exclude.iter().map(|s| s.to_ascii_lowercase()).collect();
    v.retain(|a| {
        let k = bare(a).to_ascii_lowercase();
        if seen.contains(&k) {
            false
        } else {
            seen.push(k);
            true
        }
    });
}

/// Run `cmd <file>` (to-cmd / cc-cmd / header-cmd) and return its lines.
fn command_lines(cmd: &str, file: &Path) -> CliResult<Vec<String>> {
    let mut c = proc::shell(
        &format!("{cmd} \"$@\""),
        cmd,
        &[file.to_string_lossy().into_owned()],
    );
    c.stderr(std::process::Stdio::inherit());
    let out = c
        .output()
        .map_err(|e| die(format!("fatal: unable to run '{cmd}': {e}")))?;
    if !out.status.success() {
        return Err(die(format!("fatal: command '{cmd}' failed")));
    }
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect())
}

fn validate(
    repo: Option<&arks_repo::Repository>,
    files: &[PathBuf],
    encoding: &str,
) -> CliResult<()> {
    let hook = repo.map(|r| r.common_dir().join("hooks").join("sendemail-validate"));
    for f in files {
        if let Some(h) = hook
            .as_ref()
            .filter(|h| proc::which(&h.to_string_lossy()).is_some())
        {
            let mut c = std::process::Command::new(h);
            c.arg(f);
            if !proc::run(c)?.success() {
                return Err(die(format!(
                    "fatal: {}: rejected by sendemail-validate hook",
                    f.display()
                )));
            }
        }
        if !matches!(encoding, "auto" | "base64" | "quoted-printable") {
            let text = fs::read(f)?;
            if let Some(n) = text.split(|b| *b == b'\n').position(|l| l.len() > 998) {
                return Err(die(format!(
                    "fatal: {}:{}: patch contains a line longer than 998 characters\nwarning: no patches were sent",
                    f.display(),
                    n + 1
                )));
            }
        }
    }
    Ok(())
}

/// `--compose`: edit an introductory message; returns it (when it has a
/// body) and its In-Reply-To.
type Intro = (Option<String>, Option<String>);

fn compose(
    p: &crate::cmd::opt_parse::Parsed,
    editor: &str,
    from: &str,
    irt: Option<&str>,
    tmp: &Path,
) -> CliResult<Intro> {
    let subject = p.last("subject").flatten().unwrap_or_default();
    let f = tmp.join("compose");
    let template = format!(
        "From {from} # This line is ignored.\n\
         GIT: Lines beginning in \"GIT:\" will be removed.\n\
         GIT: Consider including an overall diffstat or table of contents\n\
         GIT: for the patch you are writing.\n\
         GIT:\n\
         GIT: Clear the body content if you don't wish to send a summary.\n\
         From: {from}\n\
         Reply-To: \n\
         Subject: {subject}\n\
         In-Reply-To: {}\n\n",
        irt.unwrap_or("")
    );
    fs::write(&f, template)?;
    proc::edit(editor, &f)?;
    let text = String::from_utf8_lossy(&fs::read(&f)?).into_owned();
    let kept: String = text
        .split_inclusive('\n')
        .filter(|l| !l.starts_with("GIT:"))
        .skip_while(|l| l.starts_with("From ") && l.contains("# This line is ignored."))
        .collect();
    let parsed: Parsed = message::parse(&kept);
    let new_irt = parsed
        .get("In-Reply-To")
        .map(str::to_string)
        .filter(|s| !s.is_empty());
    let sender = parsed.get("From").unwrap_or(from).to_string();
    if parsed.body.trim().is_empty() {
        return Ok((None, new_irt));
    }
    let mut msg = String::new();
    msg.push_str(&format!("From: {sender}\n"));
    msg.push_str(&format!(
        "Subject: {}\n",
        parsed.get("Subject").unwrap_or("")
    ));
    if let Some(i) = &new_irt {
        msg.push_str(&format!("In-Reply-To: {i}\n"));
    }
    msg.push('\n');
    msg.push_str(&parsed.body);
    Ok((Some(msg), new_irt))
}

enum Kind {
    Sendmail(String),
    Smtp(smtp::Settings),
}

struct Transport {
    kind: Kind,
    domain: String,
}

impl Transport {
    fn resolve(p: &crate::cmd::opt_parse::Parsed, conf: &Conf<'_>, dry: bool) -> CliResult<Self> {
        let domain = p
            .last("smtp-domain")
            .flatten()
            .or_else(|| conf.get("smtpDomain"))
            .unwrap_or_else(hostname);
        let cmd = p
            .last("sendmail-cmd")
            .flatten()
            .or_else(|| conf.get("sendmailcmd"));
        let server = p
            .last("smtp-server")
            .flatten()
            .or_else(|| conf.get("smtpServer"));
        if let Some(c) = cmd.filter(|_| server.is_none()) {
            return Ok(Self {
                kind: Kind::Sendmail(c),
                domain,
            });
        }
        let server = match server {
            Some(s) if s.starts_with('/') => {
                return Ok(Self {
                    kind: Kind::Sendmail(proc::sq(&s)),
                    domain,
                });
            }
            Some(s) => s,
            None => {
                let found = ["/usr/sbin/sendmail", "/usr/lib/sendmail"]
                    .iter()
                    .map(|s| s.to_string())
                    .find(|s| Path::new(s).exists())
                    .or_else(|| proc::which("sendmail").map(|p| p.to_string_lossy().into_owned()));
                match found {
                    Some(sm) => {
                        return Ok(Self {
                            kind: Kind::Sendmail(proc::sq(&sm)),
                            domain,
                        });
                    }
                    None => "localhost".into(),
                }
            }
        };
        let mut enc = p
            .last("smtp-encryption")
            .flatten()
            .or_else(|| conf.get("smtpEncryption"))
            .unwrap_or_default()
            .to_ascii_lowercase();
        if p.on("smtp-ssl") || conf.flag("smtpssl") == Some(true) {
            enc = "ssl".into();
        }
        let port_s = p
            .last("smtp-server-port")
            .flatten()
            .or_else(|| conf.get("smtpServerPort"));
        let port: u16 = match port_s.as_deref() {
            Some("smtp") => 25,
            Some("submission") => 587,
            Some("smtps") => 465,
            Some(v) => v
                .parse()
                .map_err(|_| die(format!("fatal: bad port '{v}'")))?,
            None if enc == "ssl" => 465,
            None => 25,
        };
        let user = p
            .last("smtp-user")
            .flatten()
            .or_else(|| conf.get("smtpUser"));
        let mut pass = match p.last("smtp-pass") {
            Some(v) => Some(v.unwrap_or_default()),
            None => conf.get("smtpPass"),
        };
        let auth: Vec<String> = match p.last("smtp-auth") {
            Some(None) => vec!["none".into()],
            Some(Some(v)) => v.split_whitespace().map(str::to_string).collect(),
            None => conf
                .get("smtpAuth")
                .map(|v| v.split_whitespace().map(str::to_string).collect())
                .unwrap_or_default(),
        };
        let auth = if p.flag("smtp-auth") == Some(false) {
            vec!["none".to_string()]
        } else {
            auth
        };
        if let (Some(u), None, false) = (&user, &pass, dry) {
            if !auth.iter().any(|a| a == "none") {
                let cred = arks_remote::Credential {
                    protocol: Some("smtp".into()),
                    host: Some(format!("{server}:{port}")),
                    username: Some(u.clone()),
                    ..Default::default()
                };
                let src = arks_remote::CredentialSource::discover(&[]);
                pass = arks_remote::credential_fill(cred, &src)
                    .map_err(|e| die(format!("fatal: {e}")))?
                    .password;
            }
        }
        let verify = p
            .last("smtp-ssl-cert-path")
            .flatten()
            .or_else(|| conf.get("smtpsslcertpath"))
            .is_none_or(|v| !v.is_empty());
        let debug = p
            .last("smtp-debug")
            .flatten()
            .or_else(|| conf.get("smtpDebug"))
            .is_some_and(|v| v == "1");
        Ok(Self {
            kind: Kind::Smtp(smtp::Settings {
                server,
                port,
                encryption: enc,
                domain: domain.clone(),
                user,
                pass,
                auth,
                verify,
                debug,
            }),
            domain,
        })
    }
}

fn hostname() -> String {
    fs::read_to_string("/etc/hostname")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| std::env::var("HOSTNAME").ok().filter(|s| !s.is_empty()))
        .map(|h| {
            if h.contains('.') {
                h
            } else {
                format!("{h}.localdomain")
            }
        })
        .unwrap_or_else(|| "localhost.localdomain".into())
}

/// Hand one message to a sendmail-like program (`-i`, recipients as args,
/// `-f <sender>` for an explicit envelope sender).
fn sendmail(
    cmd: &str,
    sender: &str,
    explicit_from: bool,
    rcpts: &[String],
    data: &str,
) -> CliResult<()> {
    let mut args: Vec<String> = Vec::new();
    if explicit_from {
        args.push("-f".into());
        args.push(sender.to_string());
    }
    args.extend(rcpts.iter().cloned());
    let mut c = proc::shell(&format!("{cmd} -i \"$@\""), cmd, &args);
    c.stdin(std::process::Stdio::piped());
    let _ = std::io::Write::flush(&mut std::io::stdout());
    let mut child = c
        .spawn()
        .map_err(|e| die(format!("fatal: unable to run {cmd}: {e}")))?;
    if let Some(mut i) = child.stdin.take() {
        std::io::Write::write_all(&mut i, data.as_bytes())?;
    }
    let st = child.wait()?;
    if !st.success() {
        return Err(die(format!(
            "fatal: {cmd} exited with status {}",
            proc::code_of(&st)
        )));
    }
    Ok(())
}

fn dump_aliases(conf: &Conf<'_>) -> CliResult<()> {
    let mut names: Vec<String> = Vec::new();
    let kind = conf.get("aliasfiletype").unwrap_or_default();
    for f in conf.all("aliasesfile") {
        let path = if let Some(rest) = f.strip_prefix("~/") {
            std::env::var("HOME")
                .map(|h| format!("{h}/{rest}"))
                .unwrap_or(f.clone())
        } else {
            f.clone()
        };
        let text = fs::read_to_string(&path).unwrap_or_default();
        for l in text.lines() {
            let l = l.trim();
            if l.is_empty() || l.starts_with('#') {
                continue;
            }
            let name = match kind.as_str() {
                "sendmail" => l.split_once(':').map(|(n, _)| n.trim().to_string()),
                "pine" | "elm" => l.split_whitespace().next().map(str::to_string),
                "gnus" => l
                    .strip_prefix("(define-mail-alias \"")
                    .and_then(|r| r.split('"').next())
                    .map(str::to_string),
                _ => {
                    let mut w = l.split_whitespace();
                    match w.next() {
                        Some("alias") => w.next().map(str::to_string),
                        _ => None,
                    }
                }
            };
            if let Some(n) = name {
                if !names.contains(&n) {
                    names.push(n);
                }
            }
        }
    }
    names.sort();
    for n in names {
        println!("{n}");
    }
    Ok(())
}
