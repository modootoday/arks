//! `git mailinfo` and `git mailsplit`.
//!
//! `mailinfo <msg> <patch>` reads one mail on stdin, prints `Author:`,
//! `Email:`, `Subject:` and `Date:`, writes the message body to `<msg>` and
//! the patch (from the `---` / `diff` line on) to `<patch>`; the subject
//! loses `Re:` and `[...]` prefixes unless `-k` (all kept) or `-b` (only
//! `[PATCH ...]` dropped), and `--scissors` drops the body above a `-- >8 --`
//! line. `mailsplit -o<dir>` writes each mbox message to `<dir>/0001`, ...
//! and prints the count.
//! Cite: <https://git-scm.com/docs/git-mailinfo>,
//! <https://git-scm.com/docs/git-mailsplit> (behaviour, not code).

use crate::cmd::opt_parse::{Mode, parse};
use crate::error::{CliError, CliResult};
use std::fs;
use std::io::Read;
use std::path::PathBuf;

const MAILINFO_USAGE: &str = "usage: git mailinfo [<options>] <msg> <patch> < mail >info";

pub fn cmd_mailinfo(args: &[String]) -> CliResult<()> {
    let p = parse(
        "mailinfo",
        args,
        crate::cmd::porcelain_opts::MAILINFO,
        Mode::default(),
    )?;
    if let Some(q) = p.value("quoted-cr") {
        if !matches!(q.as_str(), "nowarn" | "warn" | "strip") {
            return Err(CliError::Exit(
                129,
                format!("error: bad action '{q}' for '--quoted-cr'"),
            ));
        }
    }
    let pos = p.args();
    let [msg_out, patch_out] = pos.as_slice() else {
        return Err(CliError::Exit(129, MAILINFO_USAGE.into()));
    };
    let keep = p.on("-k");
    let keep_brackets = p.on("-b");
    let scissors = p.flag("scissors").unwrap_or_else(|| {
        arks_config::ConfigSet::load(None)
            .get("mailinfo.scissors")
            .is_some_and(|v| matches!(v.to_ascii_lowercase().as_str(), "true" | "yes" | "on" | "1"))
    });
    let mut raw = String::new();
    std::io::stdin().read_to_string(&mut raw)?;
    if raw.trim().is_empty() {
        fs::write(msg_out, "")?;
        fs::write(patch_out, "")?;
        return Err(CliError::Exit(
            1,
            format!("error: empty patch: '{patch_out}'"),
        ));
    }
    let info = mail_info(&raw, keep, keep_brackets, scissors);
    let msg = info.msg;
    let patch = info.patch;
    fs::write(msg_out, msg)?;
    fs::write(patch_out, patch)?;
    println!("Author: {}", info.name);
    println!("Email: {}", info.email);
    println!("Subject: {}", info.subject);
    if let Some(d) = info.date {
        println!("Date: {d}");
    }
    println!();
    Ok(())
}

/// What `mailinfo` extracts from one mail.
pub(crate) struct MailInfo {
    pub name: String,
    pub email: String,
    pub subject: String,
    pub date: Option<String>,
    /// Message body (without the patch), newline-terminated when not empty.
    pub msg: String,
    /// The patch, from the `---` / `diff` / `Index:` line on.
    pub patch: String,
}

/// Split one mail into author, subject, date, message and patch
/// (`keep`: `-k`, `keep_brackets`: `-b`, `scissors`: `--scissors`).
pub(crate) fn mail_info(raw: &str, keep: bool, keep_brackets: bool, scissors: bool) -> MailInfo {
    let raw = raw.replace("\r\n", "\n");
    // Headers (folded lines joined), skipping an mbox `From ` line.
    let mut headers: Vec<(String, String)> = Vec::new();
    let mut lines = raw.split_inclusive('\n').peekable();
    if lines.peek().is_some_and(|l| l.starts_with("From ")) {
        lines.next();
    }
    let mut consumed = 0usize;
    for l in raw.split_inclusive('\n') {
        consumed += l.len();
        if consumed == l.len() && l.starts_with("From ") {
            continue;
        }
        let t = l.trim_end_matches('\n');
        if t.is_empty() {
            break;
        }
        if (t.starts_with(' ') || t.starts_with('\t')) && !headers.is_empty() {
            let last = headers.last_mut().expect("header");
            last.1.push(' ');
            last.1.push_str(t.trim());
        } else if let Some((k, v)) = t.split_once(':') {
            headers.push((k.to_ascii_lowercase(), v.trim().to_string()));
        } else {
            // Not a header: the body starts here.
            consumed -= l.len();
            break;
        }
    }
    let body = raw.get(consumed.min(raw.len())..).unwrap_or("");
    let get = |k: &str| headers.iter().find(|(h, _)| h == k).map(|(_, v)| v.clone());
    let subject = clean_subject(&get("subject").unwrap_or_default(), keep, keep_brackets);
    let (name, email) = match get("from") {
        Some(f) => split_from(&f),
        None => (String::new(), String::new()),
    };
    // Message: up to the patch separator; patch: from it on.
    let mut msg_lines: Vec<&str> = Vec::new();
    let mut patch = String::new();
    let mut in_patch = false;
    for l in body.split_inclusive('\n') {
        let t = l.trim_end_matches('\n');
        if !in_patch && (t == "---" || t.starts_with("diff -") || t.starts_with("Index: ")) {
            in_patch = true;
        }
        if in_patch {
            patch.push_str(l);
        } else {
            msg_lines.push(t);
        }
    }
    if scissors {
        if let Some(i) = msg_lines.iter().rposition(|l| is_scissors(l)) {
            msg_lines.drain(..=i);
        }
    }
    while msg_lines.first().is_some_and(|l| l.trim().is_empty()) {
        msg_lines.remove(0);
    }
    while msg_lines.last().is_some_and(|l| l.trim().is_empty()) {
        msg_lines.pop();
    }
    let mut msg = msg_lines.join("\n");
    if !msg.is_empty() {
        msg.push('\n');
    }
    MailInfo {
        name,
        email,
        subject,
        date: get("date"),
        msg,
        patch,
    }
}

fn is_scissors(line: &str) -> bool {
    let t = line.trim();
    (t.contains(">8") || t.contains("8<") || t.contains(">%") || t.contains("%<"))
        && t.chars().filter(|c| *c == '-').count() >= 2
        && t.chars()
            .all(|c| matches!(c, '-' | ' ' | '>' | '<' | '8' | '%'))
}

fn split_from(from: &str) -> (String, String) {
    if let Some(start) = from.find('<') {
        if let Some(end) = from[start..].find('>') {
            let email = from[start + 1..start + end].trim().to_string();
            let name = from[..start].trim().trim_matches('"').to_string();
            let name = if name.is_empty() { email.clone() } else { name };
            return (name, email);
        }
    }
    (from.trim().to_string(), from.trim().to_string())
}

/// Drop `Re:` / `[...]` prefixes (`-b`: only brackets naming PATCH; `-k`: none).
fn clean_subject(s: &str, keep: bool, keep_brackets: bool) -> String {
    if keep {
        return s.trim().to_string();
    }
    let mut t = s.trim();
    loop {
        let before = t;
        if t.len() >= 3 && t[..3].eq_ignore_ascii_case("re:") {
            t = t[3..].trim_start();
        }
        if let Some(rest) = t.strip_prefix('[') {
            if let Some((tag, after)) = rest.split_once(']') {
                if !keep_brackets || tag.contains("PATCH") {
                    t = after.trim_start();
                } else if keep_brackets {
                    // Keep this bracket; strip PATCH brackets after it.
                    let kept = format!("[{tag}] ");
                    let tail = clean_subject(after, false, true);
                    return format!("{kept}{tail}").trim().to_string();
                }
            }
        }
        if t == before {
            break;
        }
    }
    t.trim().to_string()
}

const MAILSPLIT_USAGE: &str = "usage: git mailsplit [-d<prec>] [-f<n>] [-b] [--keep-cr] -o<directory> [(<mbox>|<Maildir>)...]";

pub fn cmd_mailsplit(args: &[String]) -> CliResult<()> {
    // git mailsplit reads its options by hand (attached values only).
    let mut dir: Option<PathBuf> = None;
    let mut prec = 4usize;
    let mut skip = 0usize;
    let mut allow_bare = false;
    let mut keep_cr = false;
    let mut mboxrd = false;
    let mut files = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        i += 1;
        if a == "--" {
            files.extend(args[i..].iter().cloned());
            break;
        } else if a == "-b" {
            allow_bare = true;
        } else if a == "--keep-cr" {
            keep_cr = true;
        } else if a == "--mboxrd" {
            mboxrd = true;
        } else if let Some(v) = a.strip_prefix("-d") {
            prec = match v.parse::<usize>() {
                Ok(n) if (3..10).contains(&n) => n,
                _ => return Err(CliError::Exit(129, MAILSPLIT_USAGE.into())),
            };
        } else if let Some(v) = a.strip_prefix("-f") {
            skip = v
                .parse()
                .map_err(|_| CliError::Exit(129, MAILSPLIT_USAGE.into()))?;
        } else if let Some(v) = a.strip_prefix("-o").filter(|v| !v.is_empty()) {
            dir = Some(PathBuf::from(v));
        } else if a.starts_with('-') && a.len() > 1 {
            return Err(CliError::fatal(format!("unknown option: {a}")));
        } else {
            files.push(a.clone());
        }
    }
    let Some(dir) = dir else {
        if files.is_empty() {
            return Err(CliError::Exit(129, MAILSPLIT_USAGE.into()));
        }
        // Without -o git splits nothing.
        println!("0");
        return Ok(());
    };
    let mut text = String::new();
    if files.is_empty() {
        std::io::stdin().read_to_string(&mut text)?;
    }
    for f in &files {
        let bytes = fs::read(f)
            .map_err(|e| CliError::Exit(1, format!("error: cannot stat {f}: {}", plain_io(&e))))?;
        text.push_str(&String::from_utf8_lossy(&bytes));
    }
    if !keep_cr {
        text = text.replace("\r\n", "\n");
    }
    let mut messages: Vec<String> = Vec::new();
    for line in text.split_inclusive('\n') {
        if line.starts_with("From ") || (allow_bare && messages.is_empty()) {
            messages.push(String::new());
        }
        let Some(cur) = messages.last_mut() else {
            continue;
        };
        if mboxrd {
            if let Some(rest) = line.strip_prefix('>') {
                if rest.trim_start_matches('>').starts_with("From ") {
                    cur.push_str(rest);
                    continue;
                }
            }
        }
        cur.push_str(line);
    }
    for (k, m) in messages.iter().enumerate() {
        let name = format!("{:0prec$}", skip + k + 1);
        let path = dir.join(&name);
        fs::write(&path, m).map_err(|e| {
            CliError::fatal(format!(
                "unable to create '{}': {}",
                path.display(),
                plain_io(&e)
            ))
        })?;
    }
    println!("{}", messages.len());
    Ok(())
}

/// An io error as strerror text (without Rust's `(os error N)` tail).
fn plain_io(e: &std::io::Error) -> String {
    let s = e.to_string();
    match s.find(" (os error") {
        Some(i) => s[..i].to_string(),
        None => s,
    }
}
