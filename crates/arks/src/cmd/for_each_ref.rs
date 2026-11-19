//! `for-each-ref`: pattern selection, `--sort`, `--count`, commit filters and
//! the `--format` atom language (claimed subset).
//!
//! Atoms: `refname[:short|:lstrip=N|:rstrip=N|:strip=N]`,
//! `objectname[:short[=N]]`, `objecttype`, `objectsize`, `deref` forms
//! (`*objectname`, `*objecttype`, ...), `subject`, `body`, `contents`,
//! `contents:subject|body`, `author*`/`committer*`/`tagger*`/`creator*`
//! (`name`, `email`, `date[:<format>]`), `creatordate`, `HEAD`, `upstream`
//! (`:short`, `:track`, `:trackshort`), `symref`, plus `%%` and `%xx` hex
//! escapes. Default format: `%(objectname) %(objecttype)\t%(refname)`.
//! Primary: <https://git-scm.com/docs/git-for-each-ref>.
//!
//! Partial: `%(if)`/`%(then)`/`%(else)`/`%(end)`, `%(align)`, `--color`,
//! `--shell`/`--perl` quoting and `version:` sorting are not claimed.

use crate::cmd::log_format::{DateMode, subject_line};
use crate::cmd::shared::{abbrev_hex, is_ancestor, peel_tags, resolve_rev, tag_target};
use crate::error::{CliError, CliResult};
use crate::util::open_repo;
use arks_object::{Commit, ObjectKind, Signature};
use arks_oid::Oid;
use arks_refs::RefTarget;
use arks_repo::Repository;
use std::io::Write;

/// Parsed view of the object a ref points at.
struct RefInfo {
    name: String,
    oid: Oid,
    kind: ObjectKind,
    size: usize,
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

fn body_of(msg: &str) -> String {
    // Body = everything after the subject paragraph.
    match msg.split_once("\n\n") {
        Some((_, b)) => b.to_string(),
        None => String::new(),
    }
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

fn date_mode(spec: &str) -> CliResult<DateMode> {
    DateMode::parse(if spec.is_empty() { "default" } else { spec })
}

fn upstream_of(repo: &Repository, refname: &str) -> Option<String> {
    let branch = refname.strip_prefix("refs/heads/")?;
    let cfg = arks_config::ConfigSet::load(Some(repo.common_dir()));
    let remote = cfg.get(&format!("branch.{branch}.remote"))?;
    let merge = cfg.get(&format!("branch.{branch}.merge"))?;
    let short = merge.trim_start_matches("refs/heads/");
    Some(if remote == "." {
        format!("refs/heads/{short}")
    } else {
        format!("refs/remotes/{remote}/{short}")
    })
}

fn count_only_in(repo: &Repository, tip: &Oid, base: &Oid) -> CliResult<usize> {
    let a = arks_repo::ancestor_set(repo, *tip)?;
    let b = arks_repo::ancestor_set(repo, *base)?;
    Ok(a.difference(&b).count())
}

fn load(repo: &Repository, name: &str, oid: Oid) -> CliResult<RefInfo> {
    let obj = repo.get_object(&oid)?;
    Ok(RefInfo {
        name: name.to_string(),
        oid,
        kind: obj.kind,
        size: obj.payload.len(),
        payload: obj.payload,
    })
}

/// Value of one atom for `r` (`deref` = the `*` form: peeled tag target).
fn atom(repo: &Repository, r: &RefInfo, atom: &str, head: Option<&str>) -> CliResult<String> {
    if let Some(inner) = atom.strip_prefix('*') {
        if r.kind != ObjectKind::Tag {
            return Ok(String::new());
        }
        let Some(t) = tag_target(&r.payload) else {
            return Ok(String::new());
        };
        let peeled = peel_tags(repo, &t)?;
        let inner_ref = load(repo, &r.name, peeled)?;
        return self::atom(repo, &inner_ref, inner, head);
    }
    let (key, arg) = match atom.split_once(':') {
        Some((k, a)) => (k, a),
        None => (atom, ""),
    };
    let msg = || -> String {
        match r.kind {
            ObjectKind::Commit | ObjectKind::Tag => message_of(&r.payload),
            _ => String::new(),
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
    Ok(match key {
        "refname" => match arg {
            "" => r.name.clone(),
            "short" => short_refname(&r.name),
            _ => {
                if let Some(n) = arg.strip_prefix("lstrip=").or(arg.strip_prefix("strip=")) {
                    strip_components(&r.name, n.parse().unwrap_or(0), true)
                } else if let Some(n) = arg.strip_prefix("rstrip=") {
                    strip_components(&r.name, n.parse().unwrap_or(0), false)
                } else {
                    return Err(CliError::fatal(format!(
                        "unrecognized %(refname) argument: {arg}"
                    )));
                }
            }
        },
        "objectname" => match arg {
            "" => r.oid.to_hex(),
            "short" => abbrev_hex(repo, &r.oid, 7),
            _ => match arg.strip_prefix("short=") {
                Some(n) => abbrev_hex(repo, &r.oid, n.parse().unwrap_or(7)),
                None => {
                    return Err(CliError::fatal(format!(
                        "unrecognized %(objectname) argument: {arg}"
                    )));
                }
            },
        },
        "objecttype" => r.kind.as_str().to_string(),
        "objectsize" => r.size.to_string(),
        "tree" if r.kind == ObjectKind::Commit => Commit::decode(&r.payload)?.tree.to_hex(),
        "parent" if r.kind == ObjectKind::Commit => Commit::decode(&r.payload)?
            .parents
            .iter()
            .map(Oid::to_hex)
            .collect::<Vec<_>>()
            .join(" "),
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
        "subject" => subject_line(&msg()),
        "body" => body_of(&msg()),
        "contents" => match arg {
            "" => msg(),
            "subject" => subject_line(&msg()),
            "body" => body_of(&msg()),
            _ => {
                return Err(CliError::fatal(format!(
                    "unrecognized %(contents) argument: {arg}"
                )));
            }
        },
        "HEAD" => {
            if head == Some(r.name.as_str()) {
                "*".into()
            } else {
                " ".into()
            }
        }
        "symref" => match repo.refs().read(&r.name) {
            Ok(RefTarget::Symbolic(t)) => match arg {
                "short" => short_refname(&t),
                _ => t,
            },
            _ => String::new(),
        },
        "upstream" => {
            let Some(up) = upstream_of(repo, &r.name) else {
                return Ok(String::new());
            };
            match arg {
                "" => up,
                "short" => short_refname(&up),
                "track" | "trackshort" => {
                    let Ok(u) = repo.refs().resolve(&up) else {
                        return Ok(if arg == "track" {
                            "[gone]".into()
                        } else {
                            String::new()
                        });
                    };
                    let ahead = count_only_in(repo, &r.oid, &u)?;
                    let behind = count_only_in(repo, &u, &r.oid)?;
                    if arg == "trackshort" {
                        match (ahead, behind) {
                            (0, 0) => "=".into(),
                            (_, 0) => ">".into(),
                            (0, _) => "<".into(),
                            _ => "<>".into(),
                        }
                    } else {
                        match (ahead, behind) {
                            (0, 0) => String::new(),
                            (a, 0) => format!("[ahead {a}]"),
                            (0, b) => format!("[behind {b}]"),
                            (a, b) => format!("[ahead {a}, behind {b}]"),
                        }
                    }
                }
                _ => {
                    return Err(CliError::fatal(format!(
                        "unrecognized %(upstream) argument: {arg}"
                    )));
                }
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
                    "name" => sig.name,
                    "email" => match arg {
                        "trim" => sig.email,
                        "localpart" => sig.email.split('@').next().unwrap_or("").to_string(),
                        _ => format!("<{}>", sig.email),
                    },
                    "date" => date_mode(arg)?.format(&sig),
                    _ => return Err(CliError::fatal(format!("unknown field name: {key}"))),
                });
            }
            if matches!(key, "tree" | "parent" | "type" | "object" | "tag") {
                String::new()
            } else {
                return Err(CliError::fatal(format!("unknown field name: {key}")));
            }
        }
    })
}

fn expand(repo: &Repository, fmt: &str, r: &RefInfo, head: Option<&str>) -> CliResult<String> {
    let mut out = String::new();
    let b = fmt.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] != b'%' {
            let ch = fmt[i..].chars().next().unwrap_or('%');
            out.push(ch);
            i += ch.len_utf8();
            continue;
        }
        if fmt[i..].starts_with("%%") {
            out.push('%');
            i += 2;
        } else if fmt[i..].starts_with("%(") {
            let end = fmt[i..]
                .find(')')
                .ok_or_else(|| CliError::fatal(format!("malformed format string {fmt}")))?;
            out.push_str(&atom(repo, r, &fmt[i + 2..i + end], head)?);
            i += end + 1;
        } else if i + 2 < b.len() + 1
            && fmt.len() >= i + 3
            && fmt[i + 1..i + 3].bytes().all(|c| c.is_ascii_hexdigit())
        {
            let v = u8::from_str_radix(&fmt[i + 1..i + 3], 16).unwrap_or(b'%');
            out.push(v as char);
            i += 3;
        } else {
            out.push('%');
            i += 1;
        }
    }
    Ok(out)
}

fn pattern_matches(pattern: &str, name: &str) -> bool {
    if pattern.contains(['*', '?', '[']) {
        return crate::cmd::branch_cmd::fnmatch(pattern.as_bytes(), name.as_bytes());
    }
    let p = pattern.trim_end_matches('/');
    name == p || name.starts_with(&format!("{p}/"))
}

pub fn cmd_for_each_ref(args: &[String]) -> CliResult<()> {
    let mut format = "%(objectname) %(objecttype)\t%(refname)".to_string();
    let mut sorts: Vec<String> = Vec::new();
    let mut count: Option<usize> = None;
    let mut patterns: Vec<String> = Vec::new();
    let mut filters: Vec<(&'static str, String)> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        i += 1;
        let mut value = |name: &str| -> CliResult<Option<String>> {
            if let Some(v) = a.strip_prefix(&format!("{name}=")) {
                return Ok(Some(v.to_string()));
            }
            if a == name {
                let v = args.get(i).cloned().ok_or_else(|| {
                    CliError::fatal(format!("option `{}' requires a value", &name[2..]))
                })?;
                i += 1;
                return Ok(Some(v));
            }
            Ok(None)
        };
        if let Some(v) = value("--format")? {
            format = v;
        } else if let Some(v) = value("--sort")? {
            sorts.push(v);
        } else if let Some(v) = value("--count")? {
            count = Some(
                v.parse()
                    .map_err(|_| CliError::fatal(format!("invalid --count: {v}")))?,
            );
        } else if let Some(v) = value("--points-at")? {
            filters.push(("points-at", v));
        } else if a == "--merged" || a == "--no-merged" || a == "--contains" || a == "--no-contains"
        {
            let v = args
                .get(i)
                .filter(|v| !v.starts_with('-'))
                .cloned()
                .unwrap_or_else(|| "HEAD".into());
            if args.get(i).is_some_and(|x| !x.starts_with('-')) {
                i += 1;
            }
            let key: &'static str = match a {
                "--merged" => "merged",
                "--no-merged" => "no-merged",
                "--contains" => "contains",
                _ => "no-contains",
            };
            filters.push((key, v));
        } else if a == "--no-color" || a == "--color=never" {
        } else if a.starts_with('-') {
            return Err(CliError::NotImplemented(format!("unsupported option: {a}")));
        } else {
            patterns.push(a.to_string());
        }
    }
    let repo = open_repo()?;
    // REF-022 (M20 Cx-4): readers remain safe under packed-refs.lock (no writer dialect).
    let mut refs = repo.refs().list_refs("refs/")?;
    refs.retain(|(n, _)| patterns.is_empty() || patterns.iter().any(|p| pattern_matches(p, n)));
    let mut resolved = Vec::new();
    for (k, v) in &filters {
        let o = resolve_rev(&repo, v)
            .map_err(|_| CliError::fatal(format!("malformed object name {v}")))?;
        resolved.push((*k, o));
    }
    let head = match repo.head_target() {
        Ok(RefTarget::Symbolic(s)) => Some(s),
        _ => None,
    };
    let mut infos = Vec::new();
    for (name, oid) in refs {
        let info = load(&repo, &name, oid)?;
        let peeled = peel_tags(&repo, &oid)?;
        let mut keep = true;
        for (k, f) in &resolved {
            let commit = crate::cmd::shared::peel_to_kind(
                &repo,
                &peeled,
                crate::cmd::shared::ObjKind::Commit,
                &name,
            )
            .ok();
            keep &= match *k {
                "points-at" => oid == *f || peeled == *f,
                "merged" => commit.is_some_and(|c| is_ancestor(&repo, &c, f).unwrap_or(false)),
                "no-merged" => commit.is_some_and(|c| !is_ancestor(&repo, &c, f).unwrap_or(true)),
                "contains" => commit.is_some_and(|c| is_ancestor(&repo, f, &c).unwrap_or(false)),
                _ => commit.is_some_and(|c| !is_ancestor(&repo, f, &c).unwrap_or(true)),
            };
        }
        if keep {
            infos.push(info);
        }
    }
    // Default sort: refname. Later --sort keys are primary (git applies them last).
    infos.sort_by(|a, b| a.name.cmp(&b.name));
    for key in &sorts {
        let (desc, k) = match key.strip_prefix('-') {
            Some(k) => (true, k),
            None => (false, key.as_str()),
        };
        let mut keyed: Vec<(String, RefInfo)> = Vec::new();
        for r in infos.drain(..) {
            let v = if k.ends_with("date") {
                let raw = atom(&repo, &r, &format!("{k}:unix"), head.as_deref())?;
                format!("{:>20}", raw)
            } else {
                atom(&repo, &r, k, head.as_deref())?
            };
            keyed.push((v, r));
        }
        keyed.sort_by(|a, b| if desc { b.0.cmp(&a.0) } else { a.0.cmp(&b.0) });
        infos = keyed.into_iter().map(|(_, r)| r).collect();
    }
    if let Some(n) = count {
        infos.truncate(n);
    }
    let mut out = std::io::BufWriter::new(std::io::stdout().lock());
    for r in &infos {
        writeln!(out, "{}", expand(&repo, &format, r, head.as_deref())?)?;
    }
    out.flush()?;
    Ok(())
}
