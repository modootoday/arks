//! Helpers for the plumbing family: ref transactions (`update-ref`) and
//! object well-formedness checks (`hash-object -t`).
//!
//! Cite: <https://git-scm.com/docs/git-update-ref>,
//! <https://git-scm.com/docs/git-hash-object>.

use crate::cmd::shared::resolve_rev;
use crate::error::{CliError, CliResult};
use arks_object::ObjectKind;
use arks_oid::Oid;
use arks_refs::RefTarget;
use arks_repo::Repository;
use std::io::{self, Read};

pub(crate) struct RefUpdateOpts {
    pub no_deref: bool,
    pub create_reflog: bool,
    pub message: Option<String>,
}

/// One ref transaction step. `new`: `None` delete, `Some(None)` delete via
/// zero oid, `Some(Some(oid))` set. `old`: `None` unchecked, `Some(None)`
/// must not exist, `Some(Some(oid))` must equal.
pub(crate) struct RefOp {
    pub name: String,
    pub new: Option<Option<Oid>>,
    pub old: Option<Option<Oid>>,
    pub no_deref: bool,
    /// `--stdin` `verify`: checked, never applied.
    pub verify: bool,
}

pub(crate) enum RefCheck {
    Missing(String),
    Mismatch(String),
}

impl RefOp {
    pub(crate) fn target(&self, repo: &Repository) -> String {
        let mut target = self.name.clone();
        if !self.no_deref {
            for _ in 0..5 {
                match repo.refs().read(&target) {
                    Ok(RefTarget::Symbolic(next)) => target = next,
                    _ => break,
                }
            }
        }
        target
    }

    pub(crate) fn check(&self, repo: &Repository, target: &str) -> Result<(), RefCheck> {
        let current = repo.refs().resolve(target).ok();
        match (&self.old, &current) {
            (Some(Some(want)), Some(have)) if want != have => Err(RefCheck::Mismatch(format!(
                "cannot lock ref '{target}': is at {} but expected {}",
                have.to_hex(),
                want.to_hex()
            ))),
            (Some(None), Some(_)) => Err(RefCheck::Mismatch(format!(
                "cannot lock ref '{target}': reference already exists"
            ))),
            (Some(Some(_)), None) => Err(RefCheck::Missing(format!(
                "cannot lock ref '{target}': unable to resolve reference '{target}'"
            ))),
            // Deleting a missing ref without an expected value is a no-op.
            _ => Ok(()),
        }
    }

    pub(crate) fn apply(
        &self,
        repo: &mut Repository,
        target: &str,
        o: &RefUpdateOpts,
    ) -> CliResult<()> {
        let current = repo.refs().resolve(target).ok();
        // With --no-deref a symbolic ref is replaced, so do not compare-and-swap
        // against the oid it points at.
        let symbolic = matches!(repo.refs().read(target), Ok(RefTarget::Symbolic(_)));
        let expect = if symbolic { None } else { current.as_ref() };
        match self.new {
            None | Some(None) => {
                if current.is_some() || symbolic {
                    repo.refs_mut().delete(target, expect)?;
                }
            }
            Some(Some(new)) => {
                if symbolic {
                    let path = if target.starts_with("refs/") {
                        repo.common_dir().join(target)
                    } else {
                        repo.git_dir().join(target)
                    };
                    arks_refs::write_ref_locked(&path, format!("{}\n", new.to_hex()).as_bytes())?;
                } else {
                    repo.refs_mut().update(target, &new, expect)?;
                }
                // core.logAllRefUpdates default (non-bare): heads, remotes, notes, HEAD.
                let logged = |r: &str| {
                    r == "HEAD"
                        || ["refs/heads/", "refs/remotes/", "refs/notes/"]
                            .iter()
                            .any(|p| r.starts_with(p))
                };
                let msg = o.message.as_deref().unwrap_or("");
                let log_path = if target.starts_with("refs/") {
                    repo.common_dir().join("logs").join(target)
                } else {
                    repo.git_dir().join("logs").join(target)
                };
                if logged(target) || o.create_reflog || log_path.is_file() {
                    reflog_line(repo, target, current.as_ref(), &new, msg)?;
                }
                // Updating the branch HEAD points at also logs HEAD (git
                // splits the HEAD update out of the transaction).
                let head_follows = target != "HEAD"
                    && matches!(repo.refs().read("HEAD"), Ok(RefTarget::Symbolic(ref t)) if t == target);
                if head_follows {
                    reflog_line(repo, "HEAD", current.as_ref(), &new, msg)?;
                }
            }
        }
        Ok(())
    }
}

/// Minimal well-formedness check for `hash-object -t <type>` (fsck refusal).
pub(crate) fn validate_object(
    kind: ObjectKind,
    payload: &[u8],
    format: arks_store::ObjectFormat,
) -> CliResult<()> {
    let bad = |m: &str| {
        CliError::Exit(
            128,
            format!("error: object fails fsck: {m}\nfatal: refusing to create malformed object"),
        )
    };
    match kind {
        ObjectKind::Blob => Ok(()),
        ObjectKind::Commit => {
            if !payload.starts_with(b"tree ") {
                return Err(bad("missingTree: invalid format - expected 'tree' line"));
            }
            arks_object::Commit::decode(payload)
                .map(|_| ())
                .map_err(|_| bad("badCommit: invalid commit"))
        }
        ObjectKind::Tree => {
            let kind = match format {
                arks_store::ObjectFormat::Sha1 => arks_oid::OidKind::Sha1,
                _ => arks_oid::OidKind::Sha256,
            };
            arks_object::Tree::decode(payload, kind)
                .map(|_| ())
                .map_err(|_| {
                    CliError::Exit(
                        128,
                        "error: too-short tree object\nerror: object fails fsck: badTree: cannot be parsed as a tree\nfatal: refusing to create malformed object".into(),
                    )
                })
        }
        ObjectKind::Tag => {
            if !payload.starts_with(b"object ") {
                return Err(bad(
                    "missingObject: invalid format - expected 'object' line",
                ));
            }
            Ok(())
        }
    }
}

/// `update-ref --stdin`: parse every command, verify all, then apply.
pub(crate) fn update_ref_stdin(
    repo: &mut Repository,
    nul: bool,
    o: &RefUpdateOpts,
) -> CliResult<()> {
    let mut input = Vec::new();
    io::stdin().read_to_end(&mut input)?;
    let text = String::from_utf8_lossy(&input).into_owned();
    let mut ops: Vec<RefOp> = Vec::new();
    let mut no_deref_next = o.no_deref;
    let mut out = String::new();
    // Tokenize: newline-terminated commands, or NUL-separated fields with -z.
    let mut fields: std::collections::VecDeque<String> = if nul {
        text.split('\0').map(str::to_string).collect()
    } else {
        text.lines().map(str::to_string).collect()
    };
    let parse_val = |repo: &Repository, cmd: &str, v: &str| -> CliResult<Option<Oid>> {
        if v.is_empty() || (v.len() >= 40 && v.bytes().all(|b| b == b'0')) {
            return Ok(None);
        }
        resolve_rev(repo, v)
            .map(Some)
            .map_err(|_| CliError::fatal(format!("{cmd}: invalid <newvalue>: {v}")))
    };
    let mut started = false;
    while let Some(line) = fields.pop_front() {
        if line.is_empty() {
            continue;
        }
        let (cmd, rest) = line.split_once(' ').unwrap_or((line.as_str(), ""));
        let mut args: Vec<String> = if nul {
            let mut v = vec![rest.to_string()];
            let want = match cmd {
                "update" => 2,
                "create" => 1,
                "delete" | "verify" => 1,
                _ => 0,
            };
            for _ in 0..want {
                v.push(fields.pop_front().unwrap_or_default());
            }
            v
        } else {
            rest.split(' ').map(str::to_string).collect()
        };
        args.retain(|a| !(a.is_empty() && !nul));
        let name = args.first().cloned().unwrap_or_default();
        match cmd {
            "start" => {
                started = true;
                out.push_str("start: ok\n");
                continue;
            }
            "prepare" => {
                out.push_str("prepare: ok\n");
                continue;
            }
            "commit" => {
                apply_all(repo, &ops, o)?;
                ops.clear();
                out.push_str("commit: ok\n");
                continue;
            }
            "abort" => {
                ops.clear();
                out.push_str("abort: ok\n");
                continue;
            }
            "option" => {
                if name == "no-deref" {
                    no_deref_next = true;
                    continue;
                }
                return Err(CliError::fatal(format!("option unknown: {name}")));
            }
            "update" | "create" | "delete" | "verify" => {}
            other => {
                return Err(CliError::fatal(format!("unknown command: {other}")));
            }
        }
        if name.is_empty() {
            return Err(CliError::fatal(format!("{cmd}: missing <ref>")));
        }
        let op = match cmd {
            "update" => {
                let newv = args.get(1).cloned().unwrap_or_default();
                if newv.is_empty() {
                    return Err(CliError::fatal(format!(
                        "update {name}: missing <newvalue>"
                    )));
                }
                RefOp {
                    new: Some(parse_val(repo, &format!("update {name}"), &newv)?),
                    old: match args.get(2).filter(|s| !s.is_empty()) {
                        Some(v) => Some(parse_val(repo, &format!("update {name}"), v)?),
                        None => None,
                    },
                    name,
                    no_deref: no_deref_next,
                    verify: false,
                }
            }
            "create" => {
                let newv = args.get(1).cloned().unwrap_or_default();
                let new = parse_val(repo, &format!("create {name}"), &newv)?;
                if new.is_none() {
                    return Err(CliError::fatal(format!("create {name}: zero <newvalue>")));
                }
                RefOp {
                    name,
                    new: Some(new),
                    old: Some(None),
                    no_deref: no_deref_next,
                    verify: false,
                }
            }
            "delete" => RefOp {
                old: match args.get(1).filter(|s| !s.is_empty()) {
                    Some(v) => Some(parse_val(repo, &format!("delete {name}"), v)?),
                    None => None,
                },
                name,
                new: None,
                no_deref: no_deref_next,
                verify: false,
            },
            _ => {
                // verify: `old` absent means the ref must not exist.
                let old = match args.get(1).filter(|s| !s.is_empty()) {
                    Some(v) => parse_val(repo, &format!("verify {name}"), v)?,
                    None => None,
                };
                RefOp {
                    name,
                    new: Some(None),
                    old: Some(old),
                    no_deref: no_deref_next,
                    verify: true,
                }
            }
        };
        no_deref_next = o.no_deref;
        ops.push(op);
    }
    if !started {
        apply_all(repo, &ops, o)?;
    } else if !ops.is_empty() {
        // An unterminated transaction is aborted.
    }
    print!("{out}");
    Ok(())
}

fn apply_all(repo: &mut Repository, ops: &[RefOp], o: &RefUpdateOpts) -> CliResult<()> {
    let mut targets = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for op in ops {
        let name = op.name.clone();
        if !seen.insert(name.clone()) {
            return Err(CliError::fatal(format!(
                "multiple updates for ref '{name}' not allowed"
            )));
        }
        let probe = RefOp {
            name: name.clone(),
            new: op.new,
            old: op.old,
            no_deref: op.no_deref,
            verify: op.verify,
        };
        let target = probe.target(repo);
        let verify = op.verify;
        let check = if verify {
            let current = repo.refs().resolve(&target).ok();
            match (&op.old, &current) {
                (Some(Some(w)), Some(h)) if w != h => Err(format!(
                    "cannot lock ref '{target}': is at {} but expected {}",
                    h.to_hex(),
                    w.to_hex()
                )),
                (Some(Some(_)), None) => Err(format!(
                    "cannot lock ref '{target}': unable to resolve reference '{target}'"
                )),
                (Some(None), Some(_)) => Err(format!(
                    "cannot lock ref '{target}': reference already exists"
                )),
                _ => Ok(()),
            }
        } else {
            probe.check(repo, &target).map_err(|e| match e {
                RefCheck::Missing(m) | RefCheck::Mismatch(m) => m,
            })
        };
        if let Err(m) = check {
            return Err(CliError::fatal(m));
        }
        targets.push((probe, target, verify));
    }
    for (op, target, verify) in targets {
        if !verify {
            op.apply(repo, &target, o)?;
        }
    }
    Ok(())
}

/// Append a reflog line; an empty message leaves out the tab (git format).
fn reflog_line(
    repo: &Repository,
    refname: &str,
    old: Option<&Oid>,
    new: &Oid,
    msg: &str,
) -> CliResult<()> {
    if !msg.is_empty() {
        return crate::reflog::append_reflog(repo, refname, old, new, msg);
    }
    crate::reflog::refuse_if_reflog_locked(repo, refname)?;
    let sig = crate::cmd::shared::committer_signature()?;
    let zero = "0".repeat(new.to_hex().len());
    let line = format!(
        "{} {} {} <{}> {} {}\n",
        old.map(Oid::to_hex).unwrap_or(zero),
        new.to_hex(),
        sig.name,
        sig.email,
        sig.timestamp,
        sig.timezone
    );
    let path = if refname.starts_with("refs/") {
        repo.common_dir().join("logs").join(refname)
    } else {
        repo.git_dir().join("logs").join(refname)
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    use std::io::Write;
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?
        .write_all(line.as_bytes())?;
    Ok(())
}
