//! `git quiltimport`: apply a quilt series onto the current branch, one
//! commit per patch, keeping the patch descriptions.
//!
//! Primary: <https://git-scm.com/docs/git-quiltimport> (`--dry-run`,
//! `--author`, `--patches` / `$QUILT_PATCHES`, `--series` / `$QUILT_SERIES`,
//! `--keep-non-patch`). Observed peer behaviour (git 2.39/2.43): each patch
//! name is printed before it is applied, a missing patch directory prints
//! `The "<dir>" directory does not exist.`, the patch name becomes the
//! subject when the description has none, and a patch that does not apply
//! stops the import with exit status 4.

use super::proc;
use crate::cmd::mail::mail_info;
use crate::cmd::opt_parse::{OptDef, Takes::*, o, parse};
use crate::error::{CliError, CliResult};
use crate::util::{open_repo, read_index};
use arks_object::{Commit, ObjectKind, RawObject, Signature};
use arks_store::ObjectStore;
use std::fs;
use std::path::{Path, PathBuf};

const OPTS: &[OptDef] = &[
    o("dry-run", 'n', No, true),
    o("author", ' ', Req, true),
    o("patches", ' ', Req, true),
    o("series", ' ', Req, true),
    o("keep-non-patch", ' ', No, true),
];

/// `Name <email>` → (name, email).
fn split_ident(s: &str) -> Option<(String, String)> {
    let open = s.find('<')?;
    let close = s[open..].find('>')? + open;
    let name = s[..open].trim().to_string();
    let email = s[open + 1..close].trim().to_string();
    if name.is_empty() || close + 1 != s.trim_end().len() {
        return None;
    }
    Some((name, email))
}

pub fn cmd_quiltimport(args: &[String]) -> CliResult<()> {
    let p = parse("quiltimport", args, OPTS, Default::default())?;
    let dry = p.on("dry-run");
    let author = match p.value("author") {
        Some(a) => Some(split_ident(&a).ok_or_else(|| {
            CliError::Exit(
                1,
                format!("fatal: author '{a}' is not of the form 'Name <email>'"),
            )
        })?),
        None => None,
    };
    let patches = p
        .value("patches")
        .or_else(|| {
            std::env::var("QUILT_PATCHES")
                .ok()
                .filter(|v| !v.is_empty())
        })
        .unwrap_or_else(|| "patches".into());
    let series = p
        .value("series")
        .or_else(|| std::env::var("QUILT_SERIES").ok().filter(|v| !v.is_empty()))
        .unwrap_or_else(|| format!("{patches}/series"));
    let mut repo = open_repo()?;
    if !Path::new(&patches).is_dir() {
        println!("The \"{patches}\" directory does not exist.");
        return Err(CliError::silent(1));
    }
    if !Path::new(&series).is_file() {
        println!("The \"{series}\" file does not exist.");
        return Err(CliError::silent(1));
    }
    let list = fs::read_to_string(&series)
        .map_err(|e| CliError::fatal(format!("could not read series file {series}: {e}")))?;
    let keep_brackets = p.on("keep-non-patch");
    let tmp = proc::temp_dir("quiltimport-")?;
    let r = (|| -> CliResult<()> {
        for line in list.lines() {
            let line = line.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            let mut words = line.split_whitespace();
            let Some(name) = words.next() else { continue };
            let level = words.find(|w| w.starts_with("-p")).map(str::to_string);
            let path = PathBuf::from(&patches).join(name);
            if !path.is_file() {
                println!("{name} doesn't exist. Skipping.");
                continue;
            }
            println!("{name}");
            let raw = fs::read(&path)
                .map_err(|e| CliError::fatal(format!("could not open {}: {e}", path.display())))?;
            let raw = String::from_utf8_lossy(&raw).into_owned();
            let info = mail_info(&raw, false, keep_brackets, false);
            let (an, ae) = if !info.email.is_empty() {
                (info.name.clone(), info.email.clone())
            } else if let Some((n, e)) = &author {
                (n.clone(), e.clone())
            } else if dry {
                eprintln!("No author found in {name}");
                continue;
            } else {
                ask_author(name, &info.msg)?
            };
            if dry {
                continue;
            }
            // `mailinfo` sees no patch in a bare diff without a mail header;
            // the whole file is the patch then.
            let patch = if info.patch.is_empty() {
                raw.clone()
            } else {
                info.patch.clone()
            };
            if patch.trim().is_empty() {
                eprintln!("Patch is empty.  Was it split wrong?");
                return Err(CliError::silent(1));
            }
            let pf = tmp.join("patch");
            fs::write(&pf, &patch)?;
            let mut aargs = vec!["--index".to_string(), "-C1".to_string()];
            if let Some(l) = &level {
                aargs.push(l.clone());
            }
            aargs.push(pf.to_string_lossy().into_owned());
            if let Err(e) = crate::cmd::cmd_apply(&aargs) {
                let m = e.to_string();
                if !m.is_empty() {
                    eprintln!("{m}");
                }
                return Err(CliError::silent(4));
            }
            let subject = if info.subject.is_empty() {
                name.to_string()
            } else {
                info.subject.clone()
            };
            let mut message = format!("{subject}\n");
            if !info.msg.is_empty() {
                message.push('\n');
                message.push_str(&info.msg);
            }
            commit(&mut repo, an, ae, info.date.as_deref(), message)?;
        }
        Ok(())
    })();
    let _ = fs::remove_dir_all(&tmp);
    r
}

/// Ask for the author of a patch without one (the description is shown).
fn ask_author(name: &str, msg: &str) -> CliResult<(String, String)> {
    eprintln!("No author found in {name}");
    println!("---");
    print!("{msg}");
    loop {
        let Some(a) = proc::ask("Author: ") else {
            return Err(CliError::silent(1));
        };
        if let Some(id) = split_ident(a.trim()) {
            return Ok(id);
        }
    }
}

/// Commit the index on top of `HEAD` with the patch's author.
fn commit(
    repo: &mut arks_repo::Repository,
    name: String,
    email: String,
    date: Option<&str>,
    message: String,
) -> CliResult<()> {
    let index = read_index(repo)?;
    let format = repo.object_format();
    let tree = crate::write_tree::write_tree_from_index(repo.store_mut(), &index.entries, format)?;
    let head = repo.head_oid().ok();
    let mut author = crate::cmd::shared::commit_signature()
        .map_err(|_| CliError::fatal("unable to determine the author"))
        .unwrap_or(Signature {
            name: String::new(),
            email: String::new(),
            timestamp: 0,
            timezone: "+0000".into(),
        });
    author.name = name;
    author.email = email;
    if let Some(d) = date.and_then(crate::cmd::patch::parse_rfc2822_date) {
        author.timestamp = d.0;
        author.timezone = d.1;
    }
    let c = Commit {
        tree,
        parents: head.into_iter().collect(),
        author,
        committer: crate::cmd::shared::committer_signature()?,
        gpgsig: None,
        message,
    };
    let oid = repo
        .store_mut()
        .put(format, &RawObject::new(ObjectKind::Commit, c.encode()?))?;
    let target = repo.head_target()?;
    let refname = match target {
        arks_refs::RefTarget::Symbolic(t) => t,
        _ => "HEAD".to_string(),
    };
    repo.refs_mut().update(&refname, &oid, head.as_ref())?;
    let msg = format!("quiltimport: {}", c.message.lines().next().unwrap_or(""));
    let _ = crate::reflog::append_reflog(repo, &refname, head.as_ref(), &oid, &msg);
    if refname != "HEAD" {
        let _ = crate::reflog::append_reflog(repo, "HEAD", head.as_ref(), &oid, &msg);
    }
    Ok(())
}
