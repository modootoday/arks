//! `for-each-ref`: pattern selection (`--exclude`, `--stdin`, `--ignore-case`),
//! `--sort` (numeric, date and `version:` keys), `--count`, commit filters
//! (`--merged`, `--no-merged`, `--contains`, `--no-contains`, `--points-at`),
//! `--omit-empty`, `--color`, quoting (`--shell`, `--perl`, `--python`,
//! `--tcl`) and the `--format` atom language.
//!
//! Atoms: `refname` / `upstream` / `push` / `symref` (with `:short`,
//! `:lstrip=N`, `:rstrip=N`; upstream and push also `:track[,nobracket]`,
//! `:trackshort`, `:remotename`, `:remoteref`), `objectname[:short[=N]]`,
//! `objecttype`, `objectsize[:disk]`, `deltabase`, `tree`, `parent`,
//! `numparent`, `object`, `type`, `tag`, `subject[:sanitize]`, `body`,
//! `contents[:subject|:body|:signature|:size|:lines=N]`, `trailers[:…]`,
//! `raw[:size]`, `author*` / `committer*` / `tagger*` / `creator*` (name,
//! email with `trim`/`localpart`/`mailmap`, date with any date format),
//! `HEAD`, `worktreepath`, `describe[:…]`, `ahead-behind:<commit-ish>`,
//! `signature`, `color:<color>`, the `*` (peeled) forms and the
//! `%(align)` / `%(if)` / `%(then)` / `%(else)` / `%(end)` blocks, plus `%%`
//! and `%xx` hex escapes. Default format: `%(objectname) %(objecttype)\t%(refname)`.
//! Primary: <https://git-scm.com/docs/git-for-each-ref>.
//!
//! Partial: `%(deltabase)` is always the null id (no delta lookup) and
//! `%(signature)` is empty (no signature verification here).

use crate::cmd::log_format::subject_line;
use crate::cmd::opt_parse::{OptDef, Takes, o, parse};
use crate::cmd::shared::{abbrev_hex, peel_tags, resolve_rev, tag_target};
use crate::error::{CliError, CliResult};
use crate::util::open_repo;
use arks_object::{Commit, ObjectKind, Signature};
use arks_oid::Oid;
use arks_refs::RefTarget;
use arks_repo::Repository;
use std::collections::{HashMap, HashSet};
use std::io::Write;

const USAGE: &str = "usage: git for-each-ref [<options>] [<pattern>]\n   or: git for-each-ref [--points-at <object>]\n   or: git for-each-ref [--merged [<commit>]] [--no-merged [<commit>]]\n   or: git for-each-ref [--contains [<commit>]] [--no-contains [<commit>]]";

const OPTS: &[OptDef] = &[
    o("shell", 's', Takes::No, true),
    o("perl", 'p', Takes::No, true),
    o("python", ' ', Takes::No, true),
    o("tcl", ' ', Takes::No, true),
    o("omit-empty", ' ', Takes::No, true),
    o("count", ' ', Takes::Req, true),
    o("format", ' ', Takes::Req, true),
    o("color", ' ', Takes::Opt, true),
    o("exclude", ' ', Takes::Req, true),
    o("sort", ' ', Takes::Req, true),
    o("points-at", ' ', Takes::Req, true),
    o("merged", ' ', Takes::Req, false),
    o("no-merged", ' ', Takes::Req, false),
    o("contains", ' ', Takes::Req, false),
    o("no-contains", ' ', Takes::Req, false),
    o("ignore-case", ' ', Takes::No, true),
    o("stdin", ' ', Takes::No, true),
];

fn usage(msg: &str) -> CliError {
    CliError::Exit(129, format!("error: {msg}\n{USAGE}"))
}

/// Value quoting for `--shell` / `--perl` / `--python` / `--tcl`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Quote {
    None,
    Shell,
    Perl,
    Python,
    Tcl,
}

fn quote(q: Quote, s: &str) -> String {
    match q {
        Quote::None => s.to_string(),
        Quote::Shell => format!("'{}'", s.replace('\'', "'\\''")),
        Quote::Perl => format!("'{}'", s.replace('\\', "\\\\").replace('\'', "\\'")),
        Quote::Python => {
            let mut o = String::from("'");
            for c in s.chars() {
                match c {
                    '\\' => o.push_str("\\\\"),
                    '\'' => o.push_str("\\'"),
                    '\n' => o.push_str("\\n"),
                    c => o.push(c),
                }
            }
            o.push('\'');
            o
        }
        Quote::Tcl => {
            let mut o = String::from("\"");
            for c in s.chars() {
                match c {
                    '[' | ']' | '{' | '}' | '$' | '\\' | '"' => {
                        o.push('\\');
                        o.push(c);
                    }
                    '\x0c' => o.push_str("\\f"),
                    '\r' => o.push_str("\\r"),
                    '\n' => o.push_str("\\n"),
                    '\t' => o.push_str("\\t"),
                    '\x0b' => o.push_str("\\v"),
                    c => o.push(c),
                }
            }
            o.push('"');
            o
        }
    }
}

/// Parsed view of the object a ref points at.
#[derive(Clone)]
struct RefInfo {
    name: String,
    oid: Oid,
    kind: ObjectKind,
    payload: Vec<u8>,
}

fn header_sig(payload: &[u8], key: &str) -> Option<Signature> {
    let text = String::from_utf8_lossy(payload);
    text.lines()
        .take_while(|l| !l.is_empty())
        .find_map(|l| l.strip_prefix(&format!("{key} ")))
        .and_then(|v| Signature::parse_line(v).ok())
}

fn message_of(payload: &[u8]) -> String {
    let text = String::from_utf8_lossy(payload);
    text.split_once("\n\n")
        .map(|(_, m)| m.to_string())
        .unwrap_or_default()
}

/// Split a tag message into (message, signature block).
fn split_signature(msg: &str) -> (String, String) {
    for marker in [
        "-----BEGIN PGP SIGNATURE-----",
        "-----BEGIN PGP MESSAGE-----",
        "-----BEGIN SSH SIGNATURE-----",
        "-----BEGIN SIGNED MESSAGE-----",
    ] {
        let at = if msg.starts_with(marker) {
            Some(0)
        } else {
            msg.find(&format!("\n{marker}")).map(|i| i + 1)
        };
        if let Some(i) = at {
            return (msg[..i].to_string(), msg[i..].to_string());
        }
    }
    (msg.to_string(), String::new())
}

fn body_of(msg: &str) -> String {
    let mut lines = msg.split_inclusive('\n');
    // Skip the subject paragraph and the blank lines after it.
    let mut rest = String::new();
    let mut in_subject = true;
    let mut skipping_blank = false;
    for l in lines.by_ref() {
        if in_subject {
            if l.trim().is_empty() {
                in_subject = false;
                skipping_blank = true;
            }
            continue;
        }
        if skipping_blank && l.trim().is_empty() {
            continue;
        }
        skipping_blank = false;
        rest.push_str(l);
    }
    rest
}

fn sanitize(subject: &str) -> String {
    let mut out = String::new();
    let mut pending = false;
    for c in subject.chars() {
        let ok = c.is_ascii_alphanumeric() || c == '.' || c == '_';
        if !ok {
            pending = !out.is_empty();
            continue;
        }
        if pending {
            out.push('-');
            pending = false;
        }
        if c == '.' && out.ends_with('.') {
            continue;
        }
        out.push(c);
    }
    while out.ends_with('.') || out.ends_with('-') {
        out.pop();
    }
    out
}

fn strip_components(name: &str, n: i64, left: bool) -> String {
    let parts: Vec<&str> = name.split('/').collect();
    let len = parts.len() as i64;
    let k = if n < 0 { (len + n).max(0) } else { n.min(len) } as usize;
    if left {
        parts[k.min(parts.len())..].join("/")
    } else {
        parts[..parts.len().saturating_sub(k)].join("/")
    }
}

fn short_refname(name: &str) -> String {
    for p in ["refs/heads/", "refs/tags/", "refs/remotes/"] {
        if let Some(r) = name.strip_prefix(p) {
            return r.to_string();
        }
    }
    name.strip_prefix("refs/").unwrap_or(name).to_string()
}

/// `refname`-style arguments shared by refname / upstream / push / symref.
fn refname_arg(name: &str, arg: &str) -> Option<String> {
    Some(match arg {
        "" => name.to_string(),
        "short" => short_refname(name),
        _ => {
            if let Some(n) = arg.strip_prefix("lstrip=").or(arg.strip_prefix("strip=")) {
                strip_components(name, n.parse().ok()?, true)
            } else if let Some(n) = arg.strip_prefix("rstrip=") {
                strip_components(name, n.parse().ok()?, false)
            } else {
                return None;
            }
        }
    })
}

fn is_refname_arg(arg: &str) -> bool {
    refname_arg("refs/x", arg).is_some()
}

fn is_tracking_arg(arg: &str) -> bool {
    let mut parts = arg.split(',');
    let first = parts.next().unwrap_or("");
    match first {
        "track" => parts.all(|p| p == "nobracket"),
        "nobracket" => parts.all(|p| p == "track"),
        "trackshort" | "remotename" | "remoteref" => parts.next().is_none(),
        _ => false,
    }
}

fn bool_word(v: &str) -> Option<bool> {
    match v.to_ascii_lowercase().as_str() {
        "" | "true" | "yes" | "on" | "1" => Some(true),
        "false" | "no" | "off" | "0" => Some(false),
        _ => None,
    }
}

/// Expand `%n` / `%xNN` in a trailer separator.
fn literal_expand(s: &str) -> String {
    let mut out = String::new();
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 1 < b.len() && b[i + 1] == b'n' {
            out.push('\n');
            i += 2;
        } else if b[i] == b'%'
            && i + 3 < b.len() + 1
            && b.len() >= i + 4
            && b[i + 1] == b'x'
            && b[i + 2].is_ascii_hexdigit()
            && b[i + 3].is_ascii_hexdigit()
        {
            let v = u8::from_str_radix(&s[i + 2..i + 4], 16).unwrap_or(b'%');
            out.push(v as char);
            i += 4;
        } else {
            let ch = s[i..].chars().next().unwrap_or('%');
            out.push(ch);
            i += ch.len_utf8();
        }
    }
    out
}

#[derive(Default)]
struct TrailerOpts {
    only: bool,
    unfold: bool,
    keys: Vec<String>,
    valueonly: bool,
    separator: Option<String>,
    kv_separator: Option<String>,
}

fn trailer_opts(arg: &str) -> Option<TrailerOpts> {
    let mut t = TrailerOpts::default();
    if arg.is_empty() {
        return Some(t);
    }
    for part in arg.split(',') {
        let (k, v) = match part.split_once('=') {
            Some((k, v)) => (k, Some(v)),
            None => (part, None),
        };
        match k {
            "only" => t.only = bool_word(v.unwrap_or(""))?,
            "unfold" => t.unfold = bool_word(v.unwrap_or(""))?,
            "valueonly" => t.valueonly = bool_word(v.unwrap_or(""))?,
            "key" => {
                t.keys.push(v?.to_string());
                t.only = true;
            }
            "separator" => t.separator = Some(literal_expand(v?)),
            "key_value_separator" => t.kv_separator = Some(literal_expand(v?)),
            _ => return None,
        }
    }
    Some(t)
}

fn render_trailers(msg: &str, t: &TrailerOpts) -> String {
    let opts = crate::cmd::trailers::Opts {
        only_trailers: true,
        unfold: t.unfold,
        ..Default::default()
    };
    let block = crate::cmd::trailers::process(msg, &[], &opts);
    let mut items = Vec::new();
    for line in block.lines() {
        let Some((tok, val)) = line.split_once(':') else {
            continue;
        };
        let val = val.trim_start();
        if !t.keys.is_empty() && !t.keys.iter().any(|k| k.eq_ignore_ascii_case(tok.trim())) {
            continue;
        }
        items.push(if t.valueonly {
            val.to_string()
        } else {
            let sep = t.kv_separator.as_deref().unwrap_or(": ");
            format!("{tok}{sep}{val}")
        });
    }
    match &t.separator {
        Some(sep) => items.join(sep),
        None => items.iter().map(|i| format!("{i}\n")).collect(),
    }
}

struct DescribeOpts {
    tags: bool,
    abbrev: Option<usize>,
    matches: Vec<String>,
    excludes: Vec<String>,
}

fn describe_opts(arg: &str) -> Option<DescribeOpts> {
    let mut d = DescribeOpts {
        tags: false,
        abbrev: None,
        matches: Vec::new(),
        excludes: Vec::new(),
    };
    if arg.is_empty() {
        return Some(d);
    }
    for part in arg.split(',') {
        let (k, v) = match part.split_once('=') {
            Some((k, v)) => (k, Some(v)),
            None => (part, None),
        };
        match k {
            "tags" => d.tags = bool_word(v.unwrap_or(""))?,
            "abbrev" => d.abbrev = Some(v?.parse().ok()?),
            "match" => d.matches.push(v?.to_string()),
            "exclude" => d.excludes.push(v?.to_string()),
            _ => return None,
        }
    }
    Some(d)
}

/// Check one atom (without `%(` `)`) the way git does when it parses the format.
fn check_atom(raw: &str) -> CliResult<()> {
    let atom = raw.strip_prefix('*').unwrap_or(raw);
    let (key, arg, has_arg) = match atom.split_once(':') {
        Some((k, a)) => (k, a, true),
        None => (atom, "", false),
    };
    let unrec = || -> CliResult<()> {
        Err(CliError::fatal(format!(
            "unrecognized %({key}) argument: {arg}"
        )))
    };
    let no_args = || -> CliResult<()> {
        if has_arg {
            Err(CliError::fatal(format!("%({key}) does not take arguments")))
        } else {
            Ok(())
        }
    };
    match key {
        "refname" | "symref" => {
            if !is_refname_arg(arg) {
                return unrec();
            }
        }
        "upstream" | "push" => {
            if !is_refname_arg(arg) && !is_tracking_arg(arg) {
                return unrec();
            }
        }
        "objectname" | "tree" | "parent" => {
            let ok = matches!(arg, "" | "short")
                || arg
                    .strip_prefix("short=")
                    .is_some_and(|n| n.parse::<usize>().is_ok());
            if !ok {
                return unrec();
            }
        }
        "objectsize" => {
            if !matches!(arg, "" | "disk") {
                return unrec();
            }
        }
        "raw" => {
            if !matches!(arg, "" | "size") {
                return unrec();
            }
        }
        "subject" => {
            if !matches!(arg, "" | "sanitize") {
                return unrec();
            }
        }
        "contents" => {
            let ok = matches!(arg, "" | "subject" | "body" | "signature" | "size")
                || arg
                    .strip_prefix("lines=")
                    .is_some_and(|n| n.parse::<usize>().is_ok());
            if !ok {
                return unrec();
            }
        }
        "trailers" => {
            if trailer_opts(arg).is_none() {
                return Err(CliError::fatal(format!(
                    "unknown %(trailers) argument: {arg}"
                )));
            }
        }
        "describe" => {
            if describe_opts(arg).is_none() {
                return Err(CliError::fatal(format!(
                    "unrecognized %(describe) argument: {arg}"
                )));
            }
        }
        "ahead-behind" => {
            if arg.is_empty() {
                return Err(CliError::fatal(
                    "expected format: %(ahead-behind:<committish>)",
                ));
            }
        }
        "color" => {
            if arg.is_empty() {
                return Err(CliError::fatal("expected format: %(color:<color>)"));
            }
            if crate::cmd::config::parse_color(arg).is_none() {
                eprintln!("error: invalid color value: {arg}");
                return Err(CliError::fatal(format!(
                    "unrecognized color: %(color:{arg})"
                )));
            }
        }
        "rest" => return Err(CliError::fatal("this command reject atom %(rest)")),
        "body" | "objecttype" | "deltabase" | "numparent" | "HEAD" | "worktreepath" => {
            return no_args();
        }
        "object" | "type" | "tag" | "signature" | "if" | "then" | "else" | "end" | "align" => {}
        _ => {
            for role in ["author", "committer", "tagger", "creator"] {
                let Some(field) = key.strip_prefix(role) else {
                    continue;
                };
                let ok = match field {
                    "" => !has_arg,
                    "name" => matches!(arg, "" | "mailmap"),
                    "email" => {
                        arg.is_empty()
                            || arg
                                .split(',')
                                .all(|p| matches!(p, "trim" | "localpart" | "mailmap"))
                    }
                    "date" => {
                        if !crate::cmd::date_spec::valid(arg) {
                            return Err(CliError::fatal(format!("unknown date format {arg}")));
                        }
                        true
                    }
                    _ => return Err(CliError::fatal(format!("unknown field name: {key}"))),
                };
                if !ok {
                    return unrec();
                }
                return Ok(());
            }
            return Err(CliError::fatal(format!("unknown field name: {key}")));
        }
    }
    Ok(())
}

/// One format element.
#[derive(Clone, Debug)]
enum Node {
    Lit(String),
    Atom(String),
    Align {
        width: usize,
        pos: Pos,
        body: Vec<Node>,
    },
    If {
        cmp: Option<(bool, String)>,
        cond: Vec<Node>,
        then: Vec<Node>,
        els: Option<Vec<Node>>,
    },
}

#[derive(Clone, Copy, Debug)]
enum Pos {
    Left,
    Right,
    Middle,
}

fn align_spec(arg: &str) -> CliResult<(usize, Pos)> {
    let mut width = None;
    let mut pos = Pos::Left;
    for part in arg.split(',').filter(|p| !p.is_empty()) {
        let p = |v: &str| match v {
            "left" => Some(Pos::Left),
            "right" => Some(Pos::Right),
            "middle" => Some(Pos::Middle),
            _ => None,
        };
        if let Some(v) = part.strip_prefix("width=") {
            width = Some(
                v.parse()
                    .map_err(|_| CliError::fatal(format!("unrecognized width:{v} in %(align)")))?,
            );
        } else if let Some(v) = part.strip_prefix("position=") {
            pos = p(v).ok_or_else(|| CliError::fatal(format!("unrecognized position:{v}")))?;
        } else if let Some(q) = p(part) {
            pos = q;
        } else if let Ok(n) = part.parse() {
            width = Some(n);
        } else {
            return Err(CliError::fatal(format!(
                "unrecognized %(align) argument: {part}"
            )));
        }
    }
    let width =
        width.ok_or_else(|| CliError::fatal("expected format: %(align:<width>,<position>)"))?;
    Ok((width, pos))
}

/// Parse a `--format` string into nodes, checking every atom.
fn compile(fmt: &str) -> CliResult<Vec<Node>> {
    #[derive(PartialEq)]
    enum Part {
        Cond,
        Then,
        Else,
    }
    enum Frame {
        Top(Vec<Node>),
        Align(usize, Pos, Vec<Node>),
        If {
            cmp: Option<(bool, String)>,
            cond: Vec<Node>,
            then: Vec<Node>,
            els: Vec<Node>,
            part: Part,
            then_seen: bool,
            else_seen: bool,
        },
    }
    fn cur(stack: &mut [Frame]) -> &mut Vec<Node> {
        match stack.last_mut().expect("frame") {
            Frame::Top(v) | Frame::Align(_, _, v) => v,
            Frame::If {
                cond,
                then,
                els,
                part,
                ..
            } => match part {
                Part::Cond => cond,
                Part::Then => then,
                Part::Else => els,
            },
        }
    }
    let mut stack = vec![Frame::Top(Vec::new())];
    let mut lit = String::new();
    let b = fmt.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] != b'%' {
            let ch = fmt[i..].chars().next().unwrap_or('%');
            lit.push(ch);
            i += ch.len_utf8();
            continue;
        }
        if fmt[i..].starts_with("%%") {
            lit.push('%');
            i += 2;
            continue;
        }
        if fmt[i..].starts_with("%(") {
            let Some(end) = fmt[i..].find(')') else {
                return Err(usage(&format!("malformed format string {}", &fmt[i..])));
            };
            let atom = &fmt[i + 2..i + end];
            i += end + 1;
            if !lit.is_empty() {
                let l = std::mem::take(&mut lit);
                cur(&mut stack).push(Node::Lit(l));
            }
            check_atom(atom)?;
            let (key, arg) = match atom.split_once(':') {
                Some((k, a)) => (k, Some(a)),
                None => (atom, None),
            };
            match key {
                "align" => {
                    let (w, p) = align_spec(arg.unwrap_or(""))?;
                    stack.push(Frame::Align(w, p, Vec::new()));
                }
                "if" => {
                    let cmp = match arg {
                        None | Some("") => None,
                        Some(a) => {
                            if let Some(v) = a.strip_prefix("equals=") {
                                Some((true, v.to_string()))
                            } else if let Some(v) = a.strip_prefix("notequals=") {
                                Some((false, v.to_string()))
                            } else {
                                return Err(CliError::fatal(format!(
                                    "unrecognized %(if) argument: {a}"
                                )));
                            }
                        }
                    };
                    stack.push(Frame::If {
                        cmp,
                        cond: Vec::new(),
                        then: Vec::new(),
                        els: Vec::new(),
                        part: Part::Cond,
                        then_seen: false,
                        else_seen: false,
                    });
                }
                "then" => match stack.last_mut() {
                    Some(Frame::If {
                        part,
                        then_seen,
                        else_seen,
                        ..
                    }) => {
                        if *then_seen {
                            return Err(CliError::fatal(
                                "format: %(then) atom used more than once",
                            ));
                        }
                        if *else_seen {
                            return Err(CliError::fatal("format: %(then) atom used after %(else)"));
                        }
                        *then_seen = true;
                        *part = Part::Then;
                    }
                    _ => {
                        return Err(CliError::fatal(
                            "format: %(then) atom used without an %(if) atom",
                        ));
                    }
                },
                "else" => match stack.last_mut() {
                    Some(Frame::If {
                        part,
                        then_seen,
                        else_seen,
                        ..
                    }) => {
                        if !*then_seen {
                            return Err(CliError::fatal(
                                "format: %(else) atom used without a %(then) atom",
                            ));
                        }
                        if *else_seen {
                            return Err(CliError::fatal(
                                "format: %(else) atom used more than once",
                            ));
                        }
                        *else_seen = true;
                        *part = Part::Else;
                    }
                    _ => {
                        return Err(CliError::fatal(
                            "format: %(else) atom used without an %(if) atom",
                        ));
                    }
                },
                "end" => {
                    if stack.len() < 2 {
                        return Err(CliError::fatal(
                            "format: %(end) atom used without corresponding atom",
                        ));
                    }
                    let node = match stack.pop().expect("frame") {
                        Frame::Align(width, pos, body) => Node::Align { width, pos, body },
                        Frame::If {
                            cmp,
                            cond,
                            then,
                            els,
                            then_seen,
                            else_seen,
                            ..
                        } => {
                            if !then_seen {
                                return Err(CliError::fatal(
                                    "format: %(if) atom used without a %(then) atom",
                                ));
                            }
                            Node::If {
                                cmp,
                                cond,
                                then,
                                els: else_seen.then_some(els),
                            }
                        }
                        Frame::Top(_) => unreachable!("top frame is never popped"),
                    };
                    cur(&mut stack).push(node);
                }
                _ => cur(&mut stack).push(Node::Atom(atom.to_string())),
            }
            continue;
        }
        // `%xx` hex escape.
        if b.len() >= i + 3 && fmt[i + 1..i + 3].bytes().all(|c| c.is_ascii_hexdigit()) {
            let v = u8::from_str_radix(&fmt[i + 1..i + 3], 16).unwrap_or(b'%');
            lit.push(v as char);
            i += 3;
            continue;
        }
        lit.push('%');
        i += 1;
    }
    if !lit.is_empty() {
        cur(&mut stack).push(Node::Lit(lit));
    }
    if stack.len() > 1 {
        return Err(CliError::fatal("format: %(end) atom missing"));
    }
    match stack.pop() {
        Some(Frame::Top(v)) => Ok(v),
        _ => Err(CliError::fatal("format: %(end) atom missing")),
    }
}

/// Shared evaluation state.
struct Ctx<'a> {
    repo: &'a Repository,
    head: Option<String>,
    quote: Quote,
    color: bool,
    cfg: arks_config::ConfigSet,
    mailmap: Option<crate::cmd::mailmap::Mailmap>,
    worktrees: HashMap<String, String>,
    ancestors: std::cell::RefCell<HashMap<Oid, std::rc::Rc<HashSet<Oid>>>>,
}

impl Ctx<'_> {
    fn ancestors(&self, oid: Oid) -> CliResult<std::rc::Rc<HashSet<Oid>>> {
        if let Some(s) = self.ancestors.borrow().get(&oid) {
            return Ok(s.clone());
        }
        let set = std::rc::Rc::new(arks_repo::ancestor_set(self.repo, oid)?);
        self.ancestors.borrow_mut().insert(oid, set.clone());
        Ok(set)
    }

    fn upstream_of(&self, refname: &str) -> Option<(String, String, String)> {
        let branch = refname.strip_prefix("refs/heads/")?;
        let remote = self.cfg.get(&format!("branch.{branch}.remote"))?;
        let merge = self.cfg.get(&format!("branch.{branch}.merge"))?;
        let short = merge.trim_start_matches("refs/heads/").to_string();
        let tracking = if remote == "." {
            merge.clone()
        } else {
            format!("refs/remotes/{remote}/{short}")
        };
        Some((tracking, remote, merge))
    }

    fn push_of(&self, refname: &str) -> Option<(String, String, String)> {
        let branch = refname.strip_prefix("refs/heads/")?;
        let remote = self
            .cfg
            .get(&format!("branch.{branch}.pushRemote"))
            .or_else(|| self.cfg.get("remote.pushDefault"))
            .or_else(|| self.cfg.get(&format!("branch.{branch}.remote")))?;
        let mode = self
            .cfg
            .get("push.default")
            .unwrap_or_else(|| "simple".into());
        if mode == "upstream" {
            return self.upstream_of(refname);
        }
        if mode == "nothing" {
            return None;
        }
        let dst = format!("refs/heads/{branch}");
        let tracking = if remote == "." {
            dst.clone()
        } else {
            format!("refs/remotes/{remote}/{branch}")
        };
        Some((tracking, remote, dst))
    }

    fn sig_mapped(&self, sig: Signature) -> Signature {
        match &self.mailmap {
            Some(m) => {
                let (name, email) = m.map(&sig.name, &sig.email);
                Signature { name, email, ..sig }
            }
            None => sig,
        }
    }
}

fn load(repo: &Repository, name: &str, oid: Oid) -> CliResult<RefInfo> {
    let obj = repo.get_object(&oid)?;
    Ok(RefInfo {
        name: name.to_string(),
        oid,
        kind: obj.kind,
        payload: obj.payload,
    })
}

fn tracking(ctx: &Ctx<'_>, r: &RefInfo, up: &str, arg: &str) -> CliResult<String> {
    let nobracket = arg.split(',').any(|p| p == "nobracket");
    let short = arg.starts_with("trackshort");
    let Ok(u) = ctx.repo.refs().resolve(up) else {
        return Ok(if short {
            String::new()
        } else if nobracket {
            "gone".into()
        } else {
            "[gone]".into()
        });
    };
    let mine = ctx.ancestors(peel_tags(ctx.repo, &r.oid)?)?;
    let theirs = ctx.ancestors(peel_tags(ctx.repo, &u)?)?;
    let ahead = mine.difference(&theirs).count();
    let behind = theirs.difference(&mine).count();
    if short {
        return Ok(match (ahead, behind) {
            (0, 0) => "=".into(),
            (_, 0) => ">".into(),
            (0, _) => "<".into(),
            _ => "<>".into(),
        });
    }
    let body = match (ahead, behind) {
        (0, 0) => return Ok(String::new()),
        (a, 0) => format!("ahead {a}"),
        (0, b) => format!("behind {b}"),
        (a, b) => format!("ahead {a}, behind {b}"),
    };
    Ok(if nobracket { body } else { format!("[{body}]") })
}

fn describe(ctx: &Ctx<'_>, commit: Oid, d: &DescribeOpts) -> CliResult<String> {
    let repo = ctx.repo;
    let mut cands: Vec<(String, Oid, bool)> = Vec::new();
    let mut refs = repo.refs().list_refs("refs/tags/")?;
    refs.sort_by(|a, b| a.0.cmp(&b.0));
    for (full, oid) in refs {
        let short = full.trim_start_matches("refs/tags/").to_string();
        if !d.matches.is_empty()
            && !d
                .matches
                .iter()
                .any(|p| crate::cmd::branch_cmd::fnmatch(p.as_bytes(), short.as_bytes()))
        {
            continue;
        }
        if d.excludes
            .iter()
            .any(|p| crate::cmd::branch_cmd::fnmatch(p.as_bytes(), short.as_bytes()))
        {
            continue;
        }
        let annotated = repo
            .get_object(&oid)
            .map(|o| o.kind == ObjectKind::Tag)
            .unwrap_or(false);
        if !annotated && !d.tags {
            continue;
        }
        let Ok(c) = crate::cmd::shared::peel_to_kind(
            repo,
            &oid,
            crate::cmd::shared::ObjKind::Commit,
            &full,
        ) else {
            continue;
        };
        cands.push((short, c, annotated));
    }
    let abbrev = d.abbrev.unwrap_or(7);
    let mut exact: Vec<&(String, Oid, bool)> = cands.iter().filter(|c| c.1 == commit).collect();
    exact.sort_by_key(|c| !c.2);
    if let Some(c) = exact.first() {
        return Ok(c.0.clone());
    }
    let reach = ctx.ancestors(commit)?;
    let mut best: Option<(usize, &(String, Oid, bool))> = None;
    for c in &cands {
        if !reach.contains(&c.1) {
            continue;
        }
        let theirs = ctx.ancestors(c.1)?;
        let depth = reach.iter().filter(|o| !theirs.contains(o)).count();
        if best.is_none_or(|(dd, b)| depth < dd || (depth == dd && c.2 && !b.2)) {
            best = Some((depth, c));
        }
    }
    Ok(match best {
        Some((_, c)) if abbrev == 0 => c.0.clone(),
        Some((depth, c)) => format!(
            "{}-{depth}-g{}",
            c.0,
            abbrev_hex(repo, &commit, abbrev.max(4))
        ),
        None => String::new(),
    })
}

/// Value of one atom for `r`.
fn atom(ctx: &Ctx<'_>, r: &RefInfo, atom: &str) -> CliResult<String> {
    let repo = ctx.repo;
    if let Some(inner) = atom.strip_prefix('*') {
        if r.kind != ObjectKind::Tag {
            return Ok(String::new());
        }
        if inner == "refname" {
            return Ok(format!("{}^{{}}", r.name));
        }
        let Some(t) = tag_target(&r.payload) else {
            return Ok(String::new());
        };
        let peeled = peel_tags(repo, &t)?;
        let inner_ref = load(repo, &r.name, peeled)?;
        return self::atom(ctx, &inner_ref, inner);
    }
    let (key, arg) = match atom.split_once(':') {
        Some((k, a)) => (k, a),
        None => (atom, ""),
    };
    let full_msg = || -> String {
        match r.kind {
            ObjectKind::Commit | ObjectKind::Tag => message_of(&r.payload),
            _ => String::new(),
        }
    };
    let msg = || -> String {
        let m = full_msg();
        if r.kind == ObjectKind::Tag {
            split_signature(&m).0
        } else {
            m
        }
    };
    let person = |role: &str| -> Option<Signature> {
        match (role, r.kind) {
            ("creator", ObjectKind::Tag) | ("tagger", ObjectKind::Tag) => {
                header_sig(&r.payload, "tagger")
            }
            ("creator", ObjectKind::Commit) | ("committer", ObjectKind::Commit) => {
                header_sig(&r.payload, "committer")
            }
            ("author", ObjectKind::Commit) => header_sig(&r.payload, "author"),
            _ => None,
        }
    };
    let short_of = |oid: &Oid| -> String {
        match arg {
            "" => oid.to_hex(),
            "short" => abbrev_hex(repo, oid, 7),
            _ => {
                let n = arg
                    .strip_prefix("short=")
                    .and_then(|n| n.parse().ok())
                    .unwrap_or(7);
                abbrev_hex(repo, oid, n)
            }
        }
    };
    Ok(match key {
        "refname" => refname_arg(&r.name, arg).unwrap_or_default(),
        "objectname" => short_of(&r.oid),
        "objecttype" => r.kind.as_str().to_string(),
        "objectsize" => match arg {
            "disk" => crate::cmd::cat_file::disk_size(repo, &r.oid).to_string(),
            _ => r.payload.len().to_string(),
        },
        "deltabase" => "0".repeat(r.oid.to_hex().len()),
        "tree" if r.kind == ObjectKind::Commit => short_of(&Commit::decode(&r.payload)?.tree),
        "parent" if r.kind == ObjectKind::Commit => Commit::decode(&r.payload)?
            .parents
            .iter()
            .map(&short_of)
            .collect::<Vec<_>>()
            .join(" "),
        "numparent" if r.kind == ObjectKind::Commit => {
            Commit::decode(&r.payload)?.parents.len().to_string()
        }
        "type" if r.kind == ObjectKind::Tag => String::from_utf8_lossy(&r.payload)
            .lines()
            .find_map(|l| l.strip_prefix("type ").map(str::to_string))
            .unwrap_or_default(),
        "object" if r.kind == ObjectKind::Tag => tag_target(&r.payload)
            .map(|o| o.to_hex())
            .unwrap_or_default(),
        "tag" if r.kind == ObjectKind::Tag => String::from_utf8_lossy(&r.payload)
            .lines()
            .find_map(|l| l.strip_prefix("tag ").map(str::to_string))
            .unwrap_or_default(),
        "tree" | "parent" | "numparent" | "type" | "object" | "tag" => String::new(),
        "raw" => match arg {
            "size" => r.payload.len().to_string(),
            _ => String::from_utf8_lossy(&r.payload).into_owned(),
        },
        "subject" => {
            let s = subject_line(&msg());
            if arg == "sanitize" { sanitize(&s) } else { s }
        }
        "body" => body_of(&msg()),
        "contents" => match arg {
            "" => msg(),
            "subject" => subject_line(&msg()),
            "body" => body_of(&msg()),
            "size" => msg().len().to_string(),
            "signature" => {
                if r.kind == ObjectKind::Tag {
                    split_signature(&full_msg()).1
                } else {
                    String::new()
                }
            }
            _ => {
                let n: usize = arg
                    .strip_prefix("lines=")
                    .and_then(|n| n.parse().ok())
                    .unwrap_or(0);
                let m = msg();
                let body = m.trim_end_matches('\n');
                let mut out = String::new();
                for (k, l) in body.split('\n').take(n).enumerate() {
                    if k > 0 {
                        out.push_str("\n    ");
                    }
                    out.push_str(l);
                }
                out
            }
        },
        "trailers" => {
            let t = trailer_opts(arg).unwrap_or_default();
            render_trailers(&msg(), &t)
        }
        "HEAD" => {
            if ctx.head.as_deref() == Some(r.name.as_str()) {
                "*".into()
            } else {
                " ".into()
            }
        }
        "symref" => match repo.refs().read(&r.name) {
            Ok(RefTarget::Symbolic(t)) => refname_arg(&t, arg).unwrap_or_default(),
            _ => String::new(),
        },
        "upstream" | "push" => {
            let found = if key == "upstream" {
                ctx.upstream_of(&r.name)
            } else {
                ctx.push_of(&r.name)
            };
            let Some((tracking_ref, remote, remote_ref)) = found else {
                return Ok(String::new());
            };
            if arg.starts_with("track") || arg.starts_with("nobracket") {
                return tracking(ctx, r, &tracking_ref, arg);
            }
            match arg {
                "remotename" => remote,
                "remoteref" => remote_ref,
                _ => refname_arg(&tracking_ref, arg).unwrap_or_default(),
            }
        }
        "worktreepath" => ctx.worktrees.get(&r.name).cloned().unwrap_or_default(),
        "describe" => {
            let d = describe_opts(arg).unwrap_or(DescribeOpts {
                tags: false,
                abbrev: None,
                matches: Vec::new(),
                excludes: Vec::new(),
            });
            match r.kind {
                ObjectKind::Commit => describe(ctx, r.oid, &d)?,
                _ => String::new(),
            }
        }
        "ahead-behind" => {
            let base = resolve_rev(repo, arg)
                .map_err(|_| CliError::fatal(format!("failed to find '{arg}'")))?;
            let base = peel_tags(repo, &base)?;
            let tip = peel_tags(repo, &r.oid)?;
            let a = ctx.ancestors(tip)?;
            let b = ctx.ancestors(base)?;
            format!("{} {}", a.difference(&b).count(), b.difference(&a).count())
        }
        "signature" => String::new(),
        "color" => {
            if ctx.color {
                crate::cmd::config::parse_color(arg).unwrap_or_default()
            } else {
                String::new()
            }
        }
        _ => {
            for role in ["author", "committer", "tagger", "creator"] {
                let Some(field) = key.strip_prefix(role) else {
                    continue;
                };
                let Some(sig) = person(role) else {
                    return Ok(String::new());
                };
                return Ok(match field {
                    "" => format!(
                        "{} <{}> {} {}",
                        sig.name, sig.email, sig.timestamp, sig.timezone
                    ),
                    "name" => {
                        if arg == "mailmap" {
                            ctx.sig_mapped(sig).name
                        } else {
                            sig.name
                        }
                    }
                    "email" => {
                        let parts: Vec<&str> = arg.split(',').collect();
                        let sig = if parts.contains(&"mailmap") {
                            ctx.sig_mapped(sig)
                        } else {
                            sig
                        };
                        if parts.contains(&"localpart") {
                            sig.email.split('@').next().unwrap_or("").to_string()
                        } else if parts.contains(&"trim") {
                            sig.email
                        } else {
                            format!("<{}>", sig.email)
                        }
                    }
                    "date" => crate::cmd::date_spec::format(arg, &sig)?,
                    _ => return Err(CliError::fatal(format!("unknown field name: {key}"))),
                });
            }
            return Err(CliError::fatal(format!("unknown field name: {key}")));
        }
    })
}

fn pad(s: String, width: usize, pos: Pos) -> String {
    let len = s.chars().count();
    if len >= width {
        return s;
    }
    let gap = width - len;
    match pos {
        Pos::Left => format!("{s}{}", " ".repeat(gap)),
        Pos::Right => format!("{}{s}", " ".repeat(gap)),
        Pos::Middle => format!("{}{s}{}", " ".repeat(gap / 2), " ".repeat(gap - gap / 2)),
    }
}

/// Render `nodes`; values at the top level are quoted, nested blocks are
/// quoted as a whole when they close at the top level.
fn render(ctx: &Ctx<'_>, nodes: &[Node], r: &RefInfo, top: bool) -> CliResult<String> {
    let mut out = String::new();
    let q = |s: String| if top { quote(ctx.quote, &s) } else { s };
    for n in nodes {
        match n {
            Node::Lit(l) => out.push_str(l),
            Node::Atom(a) => out.push_str(&q(atom(ctx, r, a)?)),
            Node::Align { width, pos, body } => {
                let inner = render(ctx, body, r, false)?;
                out.push_str(&q(pad(inner, *width, *pos)));
            }
            Node::If {
                cmp,
                cond,
                then,
                els,
            } => {
                let c = render(ctx, cond, r, false)?;
                let yes = match cmp {
                    None => !c.trim().is_empty(),
                    Some((true, v)) => c == *v,
                    Some((false, v)) => c != *v,
                };
                let chosen = if yes {
                    render(ctx, then, r, false)?
                } else {
                    match els {
                        Some(e) => render(ctx, e, r, false)?,
                        None => String::new(),
                    }
                };
                out.push_str(&q(chosen));
            }
        }
    }
    Ok(out)
}

fn last_color(nodes: &[Node]) -> Option<String> {
    let mut found = None;
    for n in nodes {
        let inner = match n {
            Node::Atom(a) => a.strip_prefix("color:").map(str::to_string),
            Node::Align { body, .. } => last_color(body),
            Node::If {
                cond, then, els, ..
            } => {
                let mut f = last_color(cond);
                if let Some(c) = last_color(then) {
                    f = Some(c);
                }
                if let Some(c) = els.as_deref().and_then(last_color) {
                    f = Some(c);
                }
                f
            }
            Node::Lit(_) => None,
        };
        if inner.is_some() {
            found = inner;
        }
    }
    found
}

fn literal_prefix(pattern: &str) -> &str {
    match pattern.find(['*', '?', '[', '\\']) {
        Some(i) => &pattern[..i],
        None => pattern,
    }
}

fn glob_match(pattern: &str, name: &str, icase: bool) -> bool {
    if icase {
        crate::cmd::branch_cmd::fnmatch(
            pattern.to_lowercase().as_bytes(),
            name.to_lowercase().as_bytes(),
        )
    } else {
        crate::cmd::branch_cmd::fnmatch(pattern.as_bytes(), name.as_bytes())
    }
}

fn pattern_matches(pattern: &str, name: &str, icase: bool) -> bool {
    if pattern.contains(['*', '?', '[']) {
        // Refs are iterated under the literal prefix (case-sensitively).
        return name.starts_with(literal_prefix(pattern)) && glob_match(pattern, name, icase);
    }
    let p = pattern.trim_end_matches('/');
    name == p || name.starts_with(&format!("{p}/"))
}

fn exclude_matches(pattern: &str, name: &str, icase: bool) -> bool {
    if pattern.contains(['*', '?', '[']) {
        return glob_match(pattern, name, icase);
    }
    let (p, n) = if icase {
        (pattern.to_lowercase(), name.to_lowercase())
    } else {
        (pattern.to_string(), name.to_string())
    };
    let p = p.trim_end_matches('/');
    n == p || n.starts_with(&format!("{p}/"))
}

/// Sort key value for one ref.
#[derive(PartialEq, PartialOrd)]
enum KeyVal {
    Num(i128),
    Str(String),
}

fn version_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let (ab, bb) = (a.as_bytes(), b.as_bytes());
    let (mut i, mut j) = (0, 0);
    while i < ab.len() && j < bb.len() {
        if ab[i].is_ascii_digit() && bb[j].is_ascii_digit() {
            let si = i;
            while i < ab.len() && ab[i].is_ascii_digit() {
                i += 1;
            }
            let sj = j;
            while j < bb.len() && bb[j].is_ascii_digit() {
                j += 1;
            }
            let na = a[si..i].trim_start_matches('0');
            let nb = b[sj..j].trim_start_matches('0');
            let ord = na.len().cmp(&nb.len()).then_with(|| na.cmp(nb));
            if ord != std::cmp::Ordering::Equal {
                return ord;
            }
        } else {
            if ab[i] != bb[j] {
                return ab[i].cmp(&bb[j]);
            }
            i += 1;
            j += 1;
        }
    }
    (ab.len() - i).cmp(&(bb.len() - j))
}

struct SortKey {
    atom: String,
    desc: bool,
    version: bool,
}

fn sort_value(ctx: &Ctx<'_>, r: &RefInfo, key: &SortKey, icase: bool) -> CliResult<KeyVal> {
    let bare = key.atom.trim_start_matches('*');
    let base = bare.split(':').next().unwrap_or("");
    if base.ends_with("date") {
        let v = atom(
            ctx,
            r,
            &format!("{}:unix", key.atom.split(':').next().unwrap_or("")),
        )?;
        return Ok(KeyVal::Num(v.parse().unwrap_or(0)));
    }
    if matches!(base, "objectsize" | "numparent") || bare == "contents:size" || bare == "raw:size" {
        let v = atom(ctx, r, &key.atom)?;
        return Ok(KeyVal::Num(v.parse().unwrap_or(0)));
    }
    let v = atom(ctx, r, &key.atom)?;
    Ok(KeyVal::Str(if icase { v.to_lowercase() } else { v }))
}

fn worktree_map(repo: &Repository) -> HashMap<String, String> {
    let mut map = HashMap::new();
    if let (Ok(RefTarget::Symbolic(s)), Ok(w)) = (repo.head_target(), crate::util::work_tree(repo))
    {
        map.insert(s, w.to_string_lossy().into_owned());
    }
    let Ok(rd) = std::fs::read_dir(repo.common_dir().join("worktrees")) else {
        return map;
    };
    for e in rd.flatten() {
        let dir = e.path();
        let Ok(head) = std::fs::read_to_string(dir.join("HEAD")) else {
            continue;
        };
        let Some(target) = head.trim().strip_prefix("ref: ") else {
            continue;
        };
        let Ok(gitdir) = std::fs::read_to_string(dir.join("gitdir")) else {
            continue;
        };
        let p = std::path::Path::new(gitdir.trim());
        let wt = p.parent().unwrap_or(p).to_string_lossy().into_owned();
        map.entry(target.to_string()).or_insert(wt);
    }
    map
}

fn want_color(spec: Option<Option<String>>, cfg: &arks_config::ConfigSet) -> CliResult<bool> {
    let v = match spec {
        Some(None) => "always".to_string(),
        Some(Some(v)) => v,
        None => cfg.get("color.ui").unwrap_or_else(|| "auto".into()),
    };
    Ok(match v.to_ascii_lowercase().as_str() {
        "always" | "true" | "yes" | "on" => true,
        "never" | "false" | "no" | "off" => false,
        "auto" => std::io::IsTerminal::is_terminal(&std::io::stdout()),
        other => {
            return Err(usage(&format!(
                "option `color' expects \"always\", \"auto\", or \"never\", got '{other}'"
            )));
        }
    })
}

pub fn cmd_for_each_ref(args: &[String]) -> CliResult<()> {
    let mut args = args.to_vec();
    // `--merged`, `--no-merged`, `--contains`, `--no-contains` default to HEAD
    // when they are the last argument.
    if args.last().is_some_and(|a| {
        matches!(
            a.as_str(),
            "--merged" | "--no-merged" | "--contains" | "--no-contains"
        )
    }) {
        args.push("HEAD".into());
    }
    let p = parse("for-each-ref", &args, OPTS, Default::default()).map_err(|e| match e {
        CliError::Exit(129, m) => CliError::Exit(
            129,
            m.split("\nusage:").next().unwrap_or("").to_string() + "\n" + USAGE,
        ),
        other => other,
    })?;
    let mut quote_style = Quote::None;
    let mut styles = 0;
    for (k, q) in [
        ("shell", Quote::Shell),
        ("perl", Quote::Perl),
        ("python", Quote::Python),
        ("tcl", Quote::Tcl),
    ] {
        if p.on(k) {
            styles += 1;
            quote_style = q;
        }
    }
    if styles > 1 {
        return Err(usage("more than one quoting style?"));
    }
    let count = match p.value("count") {
        Some(v) => {
            let n: i64 = v
                .parse()
                .map_err(|_| usage("option `count' expects a numerical value"))?;
            if n < 0 {
                return Err(usage(&format!("invalid --count argument: `{v}'")));
            }
            n as usize
        }
        None => 0,
    };
    let format = p
        .value("format")
        .unwrap_or_else(|| "%(objectname) %(objecttype)\t%(refname)".to_string());
    let icase = p.on("ignore-case");
    let omit_empty = p.on("omit-empty");
    let mut patterns = p.operands();
    if p.on("stdin") {
        if !patterns.is_empty() {
            return Err(CliError::fatal("unknown arguments supplied with --stdin"));
        }
        let mut s = String::new();
        std::io::Read::read_to_string(&mut std::io::stdin(), &mut s)?;
        patterns = s
            .lines()
            .filter(|l| !l.is_empty())
            .map(str::to_string)
            .collect();
    }
    let excludes = p.values("exclude");
    let nodes = compile(&format)?;
    let mut sorts: Vec<SortKey> = Vec::new();
    for s in p.values("sort") {
        let (desc, k) = match s.strip_prefix('-') {
            Some(k) => (true, k),
            None => (false, s.as_str()),
        };
        let (version, k) = match k.strip_prefix("version:").or(k.strip_prefix("v:")) {
            Some(k) => (true, k),
            None => (false, k),
        };
        check_atom(k)?;
        sorts.push(SortKey {
            atom: k.to_string(),
            desc,
            version,
        });
    }
    let repo = open_repo()?;
    let mut points_at = Vec::new();
    for v in p.values("points-at") {
        let o =
            resolve_rev(&repo, &v).map_err(|_| usage(&format!("malformed object name '{v}'")))?;
        points_at.push(o);
    }
    let mut commit_filters: Vec<(&str, Vec<Oid>)> = Vec::new();
    for k in ["merged", "no-merged", "contains", "no-contains"] {
        let mut v = Vec::new();
        for name in p.values(k) {
            let o = resolve_rev(&repo, &name)
                .map_err(|_| CliError::fatal(format!("malformed object name {name}")))?;
            let c = crate::cmd::shared::peel_to_kind(
                &repo,
                &o,
                crate::cmd::shared::ObjKind::Commit,
                &name,
            )
            .map_err(|_| CliError::fatal(format!("object '{name}' is not a commit")))?;
            v.push(c);
        }
        if !v.is_empty() {
            commit_filters.push((k, v));
        }
    }
    let cfg = arks_config::ConfigSet::load(Some(repo.common_dir()));
    let color = want_color(p.last("color"), &cfg)?;
    let uses_mailmap =
        format.contains(":mailmap") || sorts.iter().any(|s| s.atom.contains("mailmap"));
    let mailmap = uses_mailmap.then(|| {
        let work = crate::util::work_tree(&repo).ok();
        crate::cmd::mailmap::Mailmap::load(work.as_deref(), &cfg)
    });
    let head = match repo.head_target() {
        Ok(RefTarget::Symbolic(s)) => Some(s),
        _ => None,
    };
    let worktrees = if format.contains("worktreepath") {
        worktree_map(&repo)
    } else {
        HashMap::new()
    };
    let ctx = Ctx {
        repo: &repo,
        head,
        quote: quote_style,
        color,
        cfg,
        mailmap,
        worktrees,
        ancestors: Default::default(),
    };
    // REF-022 (M20 Cx-4): readers remain safe under packed-refs.lock (no writer dialect).
    let mut refs = repo.refs().list_refs("refs/")?;
    refs.retain(|(n, _)| {
        (patterns.is_empty() || patterns.iter().any(|p| pattern_matches(p, n, icase)))
            && !excludes.iter().any(|p| exclude_matches(p, n, icase))
    });
    let mut infos = Vec::new();
    for (name, oid) in refs {
        let peeled = peel_tags(&repo, &oid)?;
        if !points_at.is_empty() {
            let one_level = match repo.get_object(&oid) {
                Ok(o) if o.kind == ObjectKind::Tag => tag_target(&o.payload),
                _ => None,
            };
            if !points_at
                .iter()
                .any(|f| *f == oid || Some(*f) == one_level || *f == peeled)
            {
                continue;
            }
        }
        if !commit_filters.is_empty() {
            let commit = crate::cmd::shared::peel_to_kind(
                &repo,
                &peeled,
                crate::cmd::shared::ObjKind::Commit,
                &name,
            )
            .ok();
            let Some(c) = commit else {
                continue;
            };
            let mut keep = true;
            for (k, fs) in &commit_filters {
                let mut any = false;
                for f in fs {
                    any |= match *k {
                        "merged" | "no-merged" => ctx.ancestors(*f)?.contains(&c),
                        _ => ctx.ancestors(c)?.contains(f),
                    };
                }
                keep &= match *k {
                    "merged" | "contains" => any,
                    _ => !any,
                };
            }
            if !keep {
                continue;
            }
        }
        infos.push(load(&repo, &name, oid)?);
    }
    // Default and tie-break order: refname. The last --sort key is primary.
    let mut keyed: Vec<(Vec<KeyVal>, RefInfo)> = Vec::new();
    for r in infos {
        let mut ks = Vec::new();
        for k in &sorts {
            ks.push(sort_value(&ctx, &r, k, icase)?);
        }
        keyed.push((ks, r));
    }
    keyed.sort_by(|a, b| {
        for (idx, k) in sorts.iter().enumerate().rev() {
            let ord = match (&a.0[idx], &b.0[idx]) {
                (KeyVal::Str(x), KeyVal::Str(y)) if k.version => version_cmp(x, y),
                (x, y) => x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal),
            };
            let ord = if k.desc { ord.reverse() } else { ord };
            if ord != std::cmp::Ordering::Equal {
                return ord;
            }
        }
        if icase {
            a.1.name
                .to_lowercase()
                .cmp(&b.1.name.to_lowercase())
                .then_with(|| a.1.name.cmp(&b.1.name))
        } else {
            a.1.name.cmp(&b.1.name)
        }
    });
    if count > 0 {
        keyed.truncate(count);
    }
    // A format whose last color atom is not `reset` ends each line with a reset.
    let reset_at_eol = color && last_color(&nodes).is_some_and(|c| c != "reset");
    let mut out = std::io::BufWriter::new(std::io::stdout().lock());
    for (_, r) in &keyed {
        let mut line = render(&ctx, &nodes, r, true)?;
        if reset_at_eol {
            line.push_str("\x1b[m");
        }
        if omit_empty && line.is_empty() {
            continue;
        }
        writeln!(out, "{line}")?;
    }
    out.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quoting_styles() {
        assert_eq!(quote(Quote::Shell, "a'b"), "'a'\\''b'");
        assert_eq!(quote(Quote::Perl, "a'b\\"), "'a\\'b\\\\'");
        assert_eq!(quote(Quote::Python, "a\nb"), "'a\\nb'");
        assert_eq!(quote(Quote::Tcl, "$x[y]\t"), "\"\\$x\\[y\\]\\t\"");
    }

    #[test]
    fn sanitized_subjects() {
        assert_eq!(sanitize("main: tail readme"), "main-tail-readme");
        assert_eq!(sanitize("a'b\"c\\d$e[f]{g}\th"), "a-b-c-d-e-f-g-h");
        assert_eq!(sanitize("..x..y.."), ".x.y");
        assert_eq!(sanitize("a.-b"), "a.-b");
    }

    #[test]
    fn format_blocks() {
        assert!(compile("%(if)x").is_err());
        assert!(compile("%(end)").is_err());
        assert!(compile("%(then)").is_err());
        assert!(compile("%(if)%(then)%(then)%(end)").is_err());
        assert!(compile("%(align)x%(end)").is_err());
        assert!(compile("%(bogus)").is_err());
        assert!(compile("%(align:10,right)%(refname)%(end)").is_ok());
        assert_eq!(pad("ab".into(), 5, Pos::Middle), " ab  ");
    }

    #[test]
    fn version_order() {
        assert_eq!(version_cmp("v1.10", "v1.9"), std::cmp::Ordering::Greater);
        assert_eq!(version_cmp("t0", "v1"), std::cmp::Ordering::Less);
    }
}
