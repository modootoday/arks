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
use std::collections::HashMap;
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

/// `~` / `~/` expansion for paths taken from config or the command line.
pub(crate) fn expand_home(p: &str) -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    if p == "~" {
        return home;
    }
    match p.strip_prefix("~/") {
        Some(rest) => format!("{home}/{rest}"),
        None => p.to_string(),
    }
}

/// `git credential-store` actions over `<proto>://<user>:<pass>@<host>[/<path>]` lines.
pub(crate) fn credential_store(
    op: &str,
    desc: &arks_remote::Credential,
    paths: &[std::path::PathBuf],
) -> CliResult<()> {
    let enc = |s: &str| -> String {
        s.bytes()
            .map(|b| {
                if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
                    (b as char).to_string()
                } else {
                    format!("%{b:02X}")
                }
            })
            .collect()
    };
    let dec = |s: &str| -> String {
        let b = s.as_bytes();
        let mut out = Vec::new();
        let mut i = 0;
        while i < b.len() {
            if b[i] == b'%' && i + 2 < b.len() {
                if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                    out.push(v);
                    i += 3;
                    continue;
                }
            }
            out.push(b[i]);
            i += 1;
        }
        String::from_utf8_lossy(&out).into_owned()
    };
    // (protocol, user, pass, host, path)
    let parse = |line: &str| -> Option<(String, String, String, String, String)> {
        let (proto, rest) = line.split_once("://")?;
        let (auth, hostpath) = rest.rsplit_once('@')?;
        let (user, pass) = auth.split_once(':')?;
        let (host, path) = match hostpath.split_once('/') {
            Some((h, p)) => (h.to_string(), p.to_string()),
            None => (hostpath.to_string(), String::new()),
        };
        Some((proto.to_string(), dec(user), dec(pass), host, dec(&path)))
    };
    let proto = desc.protocol.clone().unwrap_or_default();
    let host = desc.host.clone().unwrap_or_default();
    let want_path = desc.path.clone().unwrap_or_default();
    let matches = |e: &(String, String, String, String, String)| {
        e.0 == proto
            && e.3 == host
            && (e.4.is_empty() || e.4 == want_path)
            && desc.username.as_deref().is_none_or(|u| u == e.1)
    };
    match op {
        "get" => {
            for p in paths {
                let Ok(text) = std::fs::read_to_string(p) else {
                    continue;
                };
                if let Some(e) = text.lines().filter_map(parse).find(|e| matches(e)) {
                    println!("username={}", e.1);
                    println!("password={}", e.2);
                    return Ok(());
                }
            }
            Ok(())
        }
        "store" | "erase" => {
            if op == "store"
                && (desc.username.is_none() || desc.password.is_none() || proto.is_empty())
            {
                return Ok(());
            }
            // erase never creates a credential file.
            if op == "erase" && !paths.iter().any(|p| p.exists()) {
                return Ok(());
            }
            let target = paths
                .iter()
                .find(|p| p.exists())
                .unwrap_or(&paths[0])
                .clone();
            let text = std::fs::read_to_string(&target).unwrap_or_default();
            let mut lines: Vec<String> = text
                .lines()
                .filter(|l| !parse(l).is_some_and(|e| matches(&e)))
                .map(str::to_string)
                .collect();
            if op == "store" {
                let mut line = format!(
                    "{proto}://{}:{}@{host}",
                    enc(desc.username.as_deref().unwrap_or("")),
                    enc(desc.password.as_deref().unwrap_or(""))
                );
                if !want_path.is_empty() {
                    line.push('/');
                    line.push_str(&enc(&want_path).replace("%2F", "/"));
                }
                lines.insert(0, line);
            }
            let mut out = lines.join("\n");
            if !out.is_empty() {
                out.push('\n');
            }
            if let Some(parent) = target.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent)?;
                }
            }
            std::fs::write(&target, out)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o600));
            }
            Ok(())
        }
        // Unknown actions are ignored, as the credential helper protocol requires.
        _ => Ok(()),
    }
}

/// Every object reachable from refs (incl. HEAD and other worktrees' HEAD),
/// reflog entries, the index and `extra` tips.
pub(crate) fn reachable_all(
    repo: &Repository,
    extra: &[Oid],
) -> CliResult<std::collections::HashSet<Oid>> {
    let mut tips: Vec<Oid> = extra.to_vec();
    for (name, oid) in repo.refs().list_refs("")? {
        tips.push(oid);
        for (old, new, _) in crate::reflog::read_entries(repo, &name) {
            tips.push(new);
            tips.extend(old);
        }
    }
    if let Ok(h) = repo.head_oid() {
        tips.push(h);
    }
    for (old, new, _) in crate::reflog::read_entries(repo, "HEAD") {
        tips.push(new);
        tips.extend(old);
    }
    let mut seen = std::collections::HashSet::new();
    if let Ok(index) = crate::util::read_index(repo) {
        for e in &index.entries {
            if e.mode != 0o160000 {
                seen.insert(e.oid);
            }
        }
    }
    let mut stack = tips;
    while let Some(oid) = stack.pop() {
        if !seen.insert(oid) {
            continue;
        }
        let Ok(obj) = repo.get_object(&oid) else {
            continue;
        };
        match obj.kind {
            ObjectKind::Commit => {
                if let Ok(c) = arks_object::Commit::decode(&obj.payload) {
                    stack.push(c.tree);
                    stack.extend(c.parents);
                }
            }
            ObjectKind::Tree => {
                if let Ok(t) = arks_object::Tree::decode(&obj.payload, repo.oid_kind()) {
                    for e in t.entries {
                        if e.mode != arks_object::TreeMode::Gitlink {
                            stack.push(e.oid);
                        }
                    }
                }
            }
            ObjectKind::Tag => {
                if let Some(t) = crate::cmd::shared::tag_target(&obj.payload) {
                    stack.push(t);
                }
            }
            ObjectKind::Blob => {}
        }
    }
    Ok(seen)
}

/// Loose objects as (oid, path), sorted by object name.
pub(crate) fn loose_objects(repo: &Repository) -> Vec<(Oid, std::path::PathBuf)> {
    let dir = repo.store().objects_dir().to_path_buf();
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir(&dir) else {
        return out;
    };
    for ent in rd.flatten() {
        let prefix = ent.file_name().to_string_lossy().into_owned();
        if prefix.len() != 2 || !prefix.bytes().all(|b| b.is_ascii_hexdigit()) {
            continue;
        }
        let Ok(inner) = std::fs::read_dir(ent.path()) else {
            continue;
        };
        for obj in inner.flatten() {
            let hex = format!("{prefix}{}", obj.file_name().to_string_lossy());
            if let Ok(oid) = Oid::from_hex(&hex) {
                out.push((oid, obj.path()));
            }
        }
    }
    out.sort_by_key(|(o, _)| o.to_hex());
    out
}

/// A unique `.merge_file_XXXXXX` name in `dir`.
pub(crate) fn unique_merge_file_name(dir: &std::path::Path) -> std::path::PathBuf {
    let alphabet = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
        ^ u128::from(std::process::id());
    loop {
        let mut s = String::from(".merge_file_");
        for _ in 0..6 {
            s.push(alphabet[(n % 62) as usize] as char);
            n /= 62;
        }
        let p = dir.join(&s);
        if !p.exists() {
            return p;
        }
        n = n
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
    }
}

/// `verify-pack` for one pack/idx pair: checks the pack checksum, every
/// object, and the idx; returns the verbose / stat text.
pub(crate) fn verify_one_pack(
    pack: &std::path::Path,
    idx: &std::path::Path,
    format: arks_store::ObjectFormat,
    verbose: bool,
    stat_only: bool,
) -> CliResult<String> {
    let bytes = std::fs::read(pack).map_err(|e| {
        CliError::Msg(format!(
            "packfile {} cannot be accessed: {e}",
            pack.display()
        ))
    })?;
    let idx_bytes =
        std::fs::read(idx).map_err(|e| CliError::Msg(format!("{}: {e}", idx.display())))?;
    let index = arks_pack::PackIndex::decode(&idx_bytes, format)
        .map_err(|e| CliError::Msg(format!("{}: {e}", idx.display())))?;
    let decoded = arks_pack::DecodedPack::decode(&bytes, format)
        .map_err(|e| CliError::Msg(format!("{}: {e}", pack.display())))?;
    if index.entries.len() != decoded.objects.len() || index.pack_checksum != decoded.checksum {
        return Err(CliError::Msg(format!(
            "Packfile index for {} SHA1 mismatch",
            pack.display()
        )));
    }
    if !verbose && !stat_only {
        return Ok(String::new());
    }
    let hash_len = format.digest_len();
    let body_end = bytes.len() - hash_len;
    let mut by_offset: Vec<&arks_pack::ResolvedObject> = decoded.objects.iter().collect();
    by_offset.sort_by_key(|o| o.offset);
    let offsets: Vec<u64> = by_offset.iter().map(|o| o.offset).collect();
    let oid_at: HashMap<u64, Oid> = by_offset.iter().map(|o| (o.offset, o.oid)).collect();
    // Per entry: (declared size, base offset if delta).
    let mut meta: HashMap<u64, (u64, Option<u64>)> = HashMap::new();
    for &off in &offsets {
        let mut p = off as usize;
        let mut c = bytes[p];
        p += 1;
        let ty = (c >> 4) & 7;
        let mut size = u64::from(c & 15);
        let mut shift = 4;
        while c & 0x80 != 0 {
            c = bytes[p];
            p += 1;
            size |= u64::from(c & 0x7f) << shift;
            shift += 7;
        }
        let base = match ty {
            6 => {
                let mut c = bytes[p];
                p += 1;
                let mut ofs = u64::from(c & 0x7f);
                while c & 0x80 != 0 {
                    c = bytes[p];
                    p += 1;
                    ofs = ((ofs + 1) << 7) | u64::from(c & 0x7f);
                }
                Some(off - ofs)
            }
            7 => {
                let base = Oid::from_bytes(
                    match format {
                        arks_store::ObjectFormat::Sha1 => arks_oid::OidKind::Sha1,
                        _ => arks_oid::OidKind::Sha256,
                    },
                    &bytes[p..p + hash_len],
                )
                .ok();
                base.and_then(|b| decoded.objects.iter().find(|o| o.oid == b))
                    .map(|o| o.offset)
            }
            _ => None,
        };
        meta.insert(off, (size, base));
    }
    let depth = |mut off: u64| {
        let mut d = 0usize;
        while let Some((_, Some(b))) = meta.get(&off) {
            d += 1;
            off = *b;
        }
        d
    };
    let mut out = String::new();
    let mut chains: std::collections::BTreeMap<usize, usize> = std::collections::BTreeMap::new();
    let mut non_delta = 0usize;
    // Objects are listed in idx (object name) order? git lists in pack order.
    for (i, o) in by_offset.iter().enumerate() {
        let next = offsets.get(i + 1).copied().unwrap_or(body_end as u64);
        let (size, base) = meta[&o.offset];
        let in_pack = next - o.offset;
        match base {
            None => {
                non_delta += 1;
                if !stat_only {
                    out.push_str(&format!(
                        "{} {:<6} {size} {in_pack} {}\n",
                        o.oid.to_hex(),
                        o.object.kind.as_str(),
                        o.offset
                    ));
                }
            }
            Some(b) => {
                let d = depth(o.offset);
                *chains.entry(d).or_default() += 1;
                if !stat_only {
                    out.push_str(&format!(
                        "{} {:<6} {size} {in_pack} {} {d} {}\n",
                        o.oid.to_hex(),
                        o.object.kind.as_str(),
                        o.offset,
                        oid_at.get(&b).map(Oid::to_hex).unwrap_or_default()
                    ));
                }
            }
        }
    }
    if non_delta > 0 {
        out.push_str(&format!("non delta: {non_delta} objects\n"));
    }
    for (d, n) in chains {
        out.push_str(&format!(
            "chain length = {d}: {n} object{}\n",
            if n == 1 { "" } else { "s" }
        ));
    }
    Ok(out)
}

/// `(proper name, proper email, commit name, commit email)`.
type MailmapRule = (Option<String>, String, Option<String>, Option<String>);

/// gitmailmap(5) rules from `.mailmap`, `mailmap.file` and `mailmap.blob`.
pub(crate) struct Mailmap {
    /// (old name, old email lowercase, new name, new email)
    rules: Vec<MailmapRule>,
}

impl Mailmap {
    pub(crate) fn load(repo: &Repository) -> Self {
        let mut texts: Vec<String> = Vec::new();
        let cfg = arks_config::ConfigSet::load(Some(repo.git_dir()));
        if let Ok(wt) = crate::util::work_tree(repo) {
            if let Ok(t) = std::fs::read_to_string(wt.join(".mailmap")) {
                texts.push(t);
            }
        } else if let Some(t) = cfg
            .get("mailmap.blob")
            .or_else(|| Some("HEAD:.mailmap".into()))
            .and_then(|b| crate::cmd::shared::resolve_rev(repo, &b).ok())
            .and_then(|o| repo.get_object(&o).ok())
            .map(|o| String::from_utf8_lossy(&o.payload).into_owned())
        {
            texts.push(t);
        }
        if let Some(b) = cfg
            .get("mailmap.blob")
            .filter(|_| crate::util::work_tree(repo).is_ok())
        {
            if let Some(t) = crate::cmd::shared::resolve_rev(repo, &b)
                .ok()
                .and_then(|o| repo.get_object(&o).ok())
            {
                texts.push(String::from_utf8_lossy(&t.payload).into_owned());
            }
        }
        if let Some(f) = cfg.get("mailmap.file") {
            if let Ok(t) = std::fs::read_to_string(expand_home(&f)) {
                texts.push(t);
            }
        }
        let mut rules = Vec::new();
        for text in texts {
            for line in text.lines() {
                let line = line.split('#').next().unwrap_or("");
                let mut parts = Vec::new();
                let mut rest = line;
                while let Some(lt) = rest.find('<') {
                    let Some(gt) = rest[lt..].find('>') else {
                        break;
                    };
                    let name = rest[..lt].trim();
                    let email = &rest[lt + 1..lt + gt];
                    parts.push((
                        (!name.is_empty()).then(|| name.to_string()),
                        email.to_string(),
                    ));
                    rest = &rest[lt + gt + 1..];
                }
                match parts.as_slice() {
                    [(name, email)] => rules.push((None, email.to_lowercase(), name.clone(), None)),
                    [(new_name, new_email), (old_name, old_email)] => rules.push((
                        old_name.clone(),
                        old_email.to_lowercase(),
                        new_name.clone(),
                        Some(new_email.clone()),
                    )),
                    _ => {}
                }
            }
        }
        Self { rules }
    }

    /// Map one `Name <email>` contact (fatal when unparsable).
    pub(crate) fn check(&self, contact: &str) -> CliResult<String> {
        let (name, email) = match (contact.find('<'), contact.rfind('>')) {
            (Some(a), Some(b)) if b > a => (contact[..a].trim().to_string(), &contact[a + 1..b]),
            _ => {
                return Err(CliError::fatal(format!(
                    "unable to parse contact: {contact}"
                )));
            }
        };
        let le = email.to_lowercase();
        // A rule with an old name is more specific; later rules win.
        let hit = self
            .rules
            .iter()
            .rev()
            .find(|r| {
                r.1 == le
                    && r.0
                        .as_deref()
                        .is_some_and(|n| n.eq_ignore_ascii_case(&name))
            })
            .or_else(|| self.rules.iter().rev().find(|r| r.1 == le && r.0.is_none()));
        let (n, e) = match hit {
            Some((_, _, nn, ne)) => (
                nn.clone().unwrap_or_else(|| name.clone()),
                ne.clone().unwrap_or_else(|| email.to_string()),
            ),
            None => (name.clone(), email.to_string()),
        };
        Ok(if n.is_empty() {
            format!("<{e}>")
        } else {
            format!("{n} <{e}>")
        })
    }
}

/// `column` options independent of the layout.
pub(crate) struct ColumnOpts {
    pub width: Option<usize>,
    pub indent: String,
    pub nl: Option<String>,
    pub padding: i64,
}

impl Default for ColumnOpts {
    fn default() -> Self {
        Self {
            width: None,
            indent: String::new(),
            nl: None,
            padding: 1,
        }
    }
}

/// `column.ui`-style layout: enable (always/never/auto), column|row|plain, dense.
#[derive(Default, Clone, Copy)]
pub(crate) struct ColumnLayout {
    pub enabled: Option<bool>,
    pub row: bool,
    pub plain: bool,
    pub dense: bool,
}

impl ColumnLayout {
    /// Apply a comma/space separated mode list; the bad token on error.
    pub(crate) fn apply(&mut self, spec: &str) -> Result<(), String> {
        for tok in spec.split([',', ' ']).filter(|t| !t.is_empty()) {
            let (neg, t) = match tok.strip_prefix("no") {
                Some(r) if r == "dense" => (true, r),
                _ => (false, tok),
            };
            match t {
                "always" => self.enabled = Some(true),
                "never" => self.enabled = Some(false),
                "auto" => self.enabled = Some(false),
                "column" => {
                    self.row = false;
                    self.plain = false;
                    self.enabled.get_or_insert(true);
                }
                "row" => {
                    self.row = true;
                    self.plain = false;
                    self.enabled.get_or_insert(true);
                }
                "plain" => {
                    self.plain = true;
                    self.enabled.get_or_insert(true);
                }
                "dense" => self.dense = !neg,
                _ => return Err(tok.to_string()),
            }
        }
        Ok(())
    }

    /// `--raw-mode` bits (COL_ENABLED 0x10, layout mask 0x0F: column 0,
    /// row 1, plain 15; COL_DENSE 0x20).
    pub(crate) fn from_raw(r: u32) -> Self {
        let layout = r & 0x0f;
        Self {
            enabled: Some(r & 0x10 != 0 || r & 0x30 == 0x30),
            row: layout == 1,
            plain: layout == 15,
            dense: r & 0x20 != 0,
        }
    }
}

/// Render `items` like git's `print_columns` (column.c `display_table`):
/// cols = (width - indent) / (longest + padding), rows = ceil(n / cols);
/// `dense` shrinks rows while the per-column widths still fit; cells are
/// padded against the initial uniform width.
pub(crate) fn render_columns(items: &[&str], l: &ColumnLayout, o: &ColumnOpts) -> String {
    let nl = o.nl.clone().unwrap_or_else(|| "\n".into());
    let mut out = String::new();
    if items.is_empty() {
        return out;
    }
    if !l.enabled.unwrap_or(false) || l.plain {
        for it in items {
            out.push_str(&o.indent);
            out.push_str(it);
            out.push_str(&nl);
        }
        return out;
    }
    let width = o
        .width
        .or_else(|| std::env::var("COLUMNS").ok().and_then(|c| c.parse().ok()))
        .unwrap_or(80) as i64;
    let pad = o.padding;
    let n = items.len();
    let lens: Vec<i64> = items.iter().map(|s| s.chars().count() as i64).collect();
    let indent_len = o.indent.chars().count() as i64;
    let initial = lens.iter().copied().max().unwrap_or(0) + pad;
    let mut cols = if initial > 0 {
        ((width - indent_len) / initial).max(1) as usize
    } else {
        1
    };
    let mut rows = n.div_ceil(cols);
    let lin = |x: usize, y: usize, rows: usize, cols: usize| {
        if l.row { y * cols + x } else { x * rows + y }
    };
    // Index of the longest cell per column.
    let col_widths = |rows: usize, cols: usize| -> Vec<usize> {
        (0..cols)
            .map(|x| {
                let mut best = lin(x, 0, rows, cols);
                for y in 0..rows {
                    let i = lin(x, y, rows, cols);
                    if i < n && best < n && lens[best] < lens[i] {
                        best = i;
                    }
                }
                best
            })
            .collect()
    };
    let mut widths: Option<Vec<usize>> = None;
    if l.dense {
        while rows > 1 {
            let (r0, c0) = (rows, cols);
            rows -= 1;
            cols = n.div_ceil(rows);
            let w = col_widths(rows, cols);
            let total: i64 = indent_len
                + w.iter()
                    .map(|&i| lens.get(i).copied().unwrap_or(0) + pad)
                    .sum::<i64>();
            if total > width {
                rows = r0;
                cols = c0;
                break;
            }
        }
        widths = Some(col_widths(rows, cols));
    }
    'rows: for y in 0..rows {
        for x in 0..cols {
            let i = lin(x, y, rows, cols);
            if i >= n {
                break;
            }
            let mut len = lens[i];
            if let Some(w) = &widths {
                let cw = lens.get(w[x]).copied().unwrap_or(0);
                if cw < initial {
                    len += initial - cw;
                    len -= pad;
                }
            }
            let newline = if l.row {
                x == cols - 1 || i == n - 1
            } else {
                i + rows >= n
            };
            if x == 0 {
                out.push_str(&o.indent);
            }
            out.push_str(items[i]);
            if newline {
                out.push_str(&nl);
                continue 'rows;
            }
            out.push_str(&" ".repeat((initial - len).max(0) as usize));
        }
    }
    out
}

/// `fmt-merge-msg`: FETCH_HEAD lines → message (`Merge branch 'x' [of <url>] [into <dst>]`
/// plus an optional shortlog with `# By` / `# Via` credit lines).
/// Behaviour derived from observed stock git output (see family-plumbing corpus).
pub(crate) fn fmt_merge_msg(
    repo: &Repository,
    input: &str,
    message: Option<&str>,
    into: Option<&str>,
    log: Option<usize>,
) -> CliResult<String> {
    #[derive(Clone, Copy, PartialEq)]
    enum Kind {
        Branch,
        Rtb,
        Tag,
        Commit,
        Raw,
    }
    struct Entry {
        oid: Oid,
        kind: Kind,
        /// Name after the kind prefix, as written (quotes kept).
        name: String,
        /// Source after ` of `; for `Raw` the whole description.
        src: String,
    }
    let head = repo.head_oid().ok();
    let mut entries: Vec<Entry> = Vec::new();
    for (n, line) in input.lines().enumerate() {
        let bad = || CliError::fatal(format!("error in line {}: {line}", n + 1));
        let mut f = line.splitn(3, '\t');
        let (Some(hex), Some(flag), Some(desc)) = (f.next(), f.next(), f.next()) else {
            return Err(bad());
        };
        let oid = Oid::from_hex(hex).map_err(|_| bad())?;
        match flag {
            "" => {}
            "not-for-merge" => continue,
            _ => return Err(bad()),
        }
        let Some((what, src)) = desc.rsplit_once(" of ") else {
            entries.push(Entry {
                oid,
                kind: Kind::Raw,
                name: desc.to_string(),
                src: desc.to_string(),
            });
            continue;
        };
        let (kind, name) = if let Some(n) = what.strip_prefix("branch ") {
            (Kind::Branch, n)
        } else if let Some(n) = what.strip_prefix("remote-tracking branch ") {
            (Kind::Rtb, n)
        } else if let Some(n) = what.strip_prefix("tag ") {
            (Kind::Tag, n)
        } else {
            (Kind::Commit, what)
        };
        entries.push(Entry {
            oid,
            kind,
            name: name.to_string(),
            src: src.to_string(),
        });
    }
    // Drop strict ancestors of HEAD and parents reachable from another parent.
    let is_anc = |a: &Oid, b: &Oid| crate::cmd::shared::is_ancestor(repo, a, b).unwrap_or(false);
    let keep: Vec<bool> = entries
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let in_head = head
                .as_ref()
                .is_some_and(|h| *h != e.oid && is_anc(&e.oid, h));
            let in_other = entries
                .iter()
                .enumerate()
                .any(|(j, p)| j != i && p.oid != e.oid && is_anc(&e.oid, &p.oid));
            !in_head && !in_other
        })
        .collect();
    let kept: Vec<&Entry> = entries
        .iter()
        .zip(&keep)
        .filter(|(_, k)| **k)
        .map(|(e, _)| e)
        .collect();
    let mut out = String::new();
    if let Some(m) = message {
        out.push_str(m);
        if !m.ends_with('\n') {
            out.push('\n');
        }
    }
    if kept.is_empty() {
        return Ok(out);
    }
    if message.is_none() {
        // Group by source in order of first appearance; raw lines stand alone.
        let mut groups: Vec<(bool, &str)> = Vec::new();
        for e in &kept {
            let key = (e.kind == Kind::Raw, e.src.as_str());
            if !groups.contains(&key) {
                groups.push(key);
            }
        }
        let join = |one: &str, many: &str, names: &[&str]| -> String {
            match names {
                [n] => format!("{one} {n}"),
                [rest @ .., last] => format!("{many} {} and {last}", rest.join(", ")),
                [] => String::new(),
            }
        };
        let mut parts = Vec::new();
        for (raw, src) in &groups {
            if *raw {
                parts.push((*src).to_string());
                continue;
            }
            let names = |k: Kind| -> Vec<&str> {
                kept.iter()
                    .filter(|e| e.kind == k && e.src == *src)
                    .map(|e| e.name.as_str())
                    .collect()
            };
            let mut sub = Vec::new();
            for (k, one, many) in [
                (Kind::Branch, "branch", "branches"),
                (
                    Kind::Rtb,
                    "remote-tracking branch",
                    "remote-tracking branches",
                ),
                (Kind::Tag, "tag", "tags"),
                (Kind::Commit, "commit", "commits"),
            ] {
                let n = names(k);
                if !n.is_empty() {
                    sub.push(join(one, many, &n));
                }
            }
            let mut p = sub.join(", ");
            if *src != "." {
                p.push_str(&format!(" of {src}"));
            }
            parts.push(p);
        }
        out.push_str("Merge ");
        out.push_str(&parts.join("; "));
        let dest = match into {
            Some(i) => i.to_string(),
            None => match repo.head_target() {
                Ok(arks_refs::RefTarget::Symbolic(t)) => t
                    .strip_prefix("refs/heads/")
                    .map(str::to_string)
                    .unwrap_or(t),
                _ => "HEAD".to_string(),
            },
        };
        let cfg = arks_config::ConfigSet::load(Some(repo.git_dir()));
        let suppress = cfg.get_all("merge.suppressdest");
        let suppressed = if suppress.is_empty() {
            dest == "main" || dest == "master"
        } else {
            suppress.contains(&dest)
        };
        if !suppressed {
            out.push_str(&format!(" into {dest}"));
        }
        out.push('\n');
    }
    let Some(limit) = log else {
        return Ok(out);
    };
    let limit = limit.max(1);
    let cfg = arks_config::ConfigSet::load(Some(repo.git_dir()));
    let comment = cfg
        .get("core.commentchar")
        .and_then(|c| c.chars().next())
        .unwrap_or('#');
    let me_author = crate::cmd::shared::commit_signature().ok().map(|s| s.name);
    let me_committer = crate::cmd::shared::committer_signature()
        .ok()
        .map(|s| s.name);
    for e in &kept {
        let label = match e.kind {
            Kind::Raw => e.src.clone(),
            Kind::Branch | Kind::Rtb if e.src == "." => e.name.trim_matches('\'').to_string(),
            Kind::Branch | Kind::Rtb => format!("{} of {}", e.name, e.src),
            Kind::Tag if e.src == "." => format!("tag {}", e.name),
            Kind::Tag => format!("tag {} of {}", e.name, e.src),
            Kind::Commit => e.src.clone(),
        };
        // Walk commits not reachable from HEAD, newest first.
        let mut seen = std::collections::HashSet::new();
        let mut stack = vec![e.oid];
        let mut commits: Vec<(i64, arks_object::Commit)> = Vec::new();
        while let Some(c) = stack.pop() {
            if !seen.insert(c) || head.as_ref().is_some_and(|h| is_anc(&c, h)) {
                continue;
            }
            let Ok(obj) = repo.get_object(&c) else {
                continue;
            };
            let Ok(cm) = arks_object::Commit::decode(&obj.payload) else {
                continue;
            };
            stack.extend(cm.parents.iter().copied());
            commits.push((cm.committer.timestamp, cm));
        }
        commits.sort_by_key(|c| std::cmp::Reverse(c.0));
        // Credits: authors of listed commits; committers of merges and of the tip.
        let mut authors: Vec<(String, usize)> = Vec::new();
        let mut committers: Vec<(String, usize)> = Vec::new();
        let bump = |list: &mut Vec<(String, usize)>, name: &str| match list
            .iter_mut()
            .find(|(n, _)| n == name)
        {
            Some(p) => p.1 += 1,
            None => list.push((name.to_string(), 1)),
        };
        let mut subjects = Vec::new();
        for (_, cm) in &commits {
            if cm.parents.len() > 1 {
                bump(&mut committers, &cm.committer.name);
                continue;
            }
            if subjects.is_empty() {
                bump(&mut committers, &cm.committer.name);
            }
            bump(&mut authors, &cm.author.name);
            subjects.push(cm.message.lines().next().unwrap_or("").to_string());
        }
        out.push('\n');
        for (label_word, mut people, me) in [
            ("By", authors, &me_author),
            ("Via", committers, &me_committer),
        ] {
            if people.is_empty()
                || (people.len() == 1 && me.as_deref() == Some(people[0].0.as_str()))
            {
                continue;
            }
            people.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
            let text = match people.as_slice() {
                [(n, _)] => n.clone(),
                [(a, x), (b, y)] => format!("{a} ({x}) and {b} ({y})"),
                [(a, x), ..] => format!("{a} ({x}) and others"),
                [] => String::new(),
            };
            out.push_str(&format!("{comment} {label_word} {text}\n"));
        }
        let total = subjects.len();
        if total > limit {
            out.push_str(&format!("* {label}: ({total} commits)\n"));
        } else {
            out.push_str(&format!("* {label}:\n"));
        }
        for s in subjects.iter().take(limit) {
            out.push_str(&format!("  {s}\n"));
        }
        if total > limit {
            out.push_str("  ...\n");
        }
    }
    Ok(out)
}

/// Shared `-o/--output-directory`, `-s/--suffix`, `--diagnose`/`--mode` options.
pub(crate) struct ReportOpts {
    pub dir: std::path::PathBuf,
    pub suffix: String,
    pub diagnose: Option<String>,
}

impl ReportOpts {
    pub(crate) fn parse(args: &[String], cmd: &str, bugreport: bool) -> CliResult<Self> {
        let usage = || {
            CliError::Exit(
                129,
                format!(
                    "usage: git {cmd} [(-o | --output-directory) <path>] [(-s | --suffix) <format>]"
                ),
            )
        };
        let mut o = Self {
            dir: std::path::PathBuf::from("."),
            suffix: "%Y-%m-%d-%H%M".into(),
            diagnose: None,
        };
        let mut i = 0;
        while i < args.len() {
            let a = args[i].as_str();
            i += 1;
            let mut val = |name_short: &str, name_long: &str| -> CliResult<Option<String>> {
                if a == name_short || a == name_long {
                    let v = args.get(i).cloned().ok_or_else(usage)?;
                    i += 1;
                    return Ok(Some(v));
                }
                if let Some(v) = a.strip_prefix(&format!("{name_long}=")) {
                    return Ok(Some(v.to_string()));
                }
                if name_short.len() == 2
                    && a.len() > 2
                    && a.starts_with(name_short)
                    && !a.starts_with("--")
                {
                    return Ok(Some(a[2..].to_string()));
                }
                Ok(None)
            };
            if let Some(v) = val("-o", "--output-directory")? {
                o.dir = std::path::PathBuf::from(v);
            } else if let Some(v) = val("-s", "--suffix")? {
                o.suffix = v;
            } else if a == "--no-output-directory" {
                o.dir = std::path::PathBuf::from(".");
            } else if a == "--no-suffix" {
                o.suffix.clear();
            } else if bugreport && a == "--diagnose" {
                o.diagnose = Some("stats".into());
            } else if bugreport && a == "--no-diagnose" {
                o.diagnose = None;
            } else if let Some(m) =
                a.strip_prefix(if bugreport { "--diagnose=" } else { "--mode=" })
            {
                if m != "stats" && m != "all" {
                    return Err(CliError::Exit(
                        129,
                        format!(
                            "error: invalid --{} value '{m}'",
                            if bugreport { "diagnose" } else { "mode" }
                        ),
                    ));
                }
                o.diagnose = Some(m.to_string());
            } else if !bugreport && a == "--mode" {
                let m = args.get(i).cloned().ok_or_else(usage)?;
                i += 1;
                if m != "stats" && m != "all" {
                    return Err(CliError::Exit(
                        129,
                        format!("error: invalid --mode value '{m}'"),
                    ));
                }
                o.diagnose = Some(m);
            } else if !bugreport && !a.starts_with('-') {
                // diagnose ignores positional arguments, as stock git does.
            } else {
                return Err(usage());
            }
        }
        Ok(o)
    }

    /// `<dir>/<prefix>-<strftime(suffix)>.<ext>`.
    pub(crate) fn target(&self, prefix: &str, ext: &str) -> CliResult<std::path::PathBuf> {
        std::fs::create_dir_all(&self.dir).map_err(|e| {
            CliError::fatal(format!(
                "could not create leading directories for '{}': {e}",
                self.dir.display()
            ))
        })?;
        let stamp = strftime_now(&self.suffix);
        let name = if stamp.is_empty() {
            format!("{prefix}.{ext}")
        } else {
            format!("{prefix}-{stamp}.{ext}")
        };
        let p = self.dir.join(name);
        if p.exists() {
            return Err(CliError::fatal(format!(
                "file '{}' already exists",
                p.display()
            )));
        }
        Ok(p)
    }
}

/// Minimal strftime (`%Y %m %d %H %M %S %%`) in UTC.
fn strftime_now(fmt: &str) -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    // civil_from_days (Howard Hinnant).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    let mut out = String::new();
    let mut it = fmt.chars();
    while let Some(c) = it.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        match it.next() {
            Some('Y') => out.push_str(&format!("{y:04}")),
            Some('m') => out.push_str(&format!("{m:02}")),
            Some('d') => out.push_str(&format!("{d:02}")),
            Some('H') => out.push_str(&format!("{:02}", rem / 3600)),
            Some('M') => out.push_str(&format!("{:02}", rem % 3600 / 60)),
            Some('S') => out.push_str(&format!("{:02}", rem % 60)),
            Some('%') => out.push('%'),
            Some(o) => {
                out.push('%');
                out.push(o);
            }
            None => out.push('%'),
        }
    }
    out
}

/// Write a stored (uncompressed) zip with repository diagnostics: a
/// `diagnostics.log` summary, plus the `.git` metadata files for `all`.
pub(crate) fn write_diagnostics(zip: &std::path::Path, mode: &str) -> CliResult<()> {
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    let mut log = format!(
        "arks version {}\ncpu: {}\nno commit associated with this build\nsizeof-long: {}\nsizeof-size_t: {}\nshell-path: /bin/sh\n",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::ARCH,
        std::mem::size_of::<std::ffi::c_long>(),
        std::mem::size_of::<usize>()
    );
    if let Ok(repo) = crate::util::open_repo() {
        let root = crate::util::work_tree(&repo)
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|_| repo.git_dir().to_path_buf());
        log.push_str(&format!("Repository root: {}\n", root.display()));
        let objects = repo.store().objects_dir().to_path_buf();
        // packs-local.txt: one `%-70s %16d` line per file in objects/pack.
        let mut packs = String::from("Contents of .git/objects:\n");
        if let Ok(rd) = std::fs::read_dir(objects.join("pack")) {
            for e in rd.flatten() {
                let n = e.file_name().to_string_lossy().into_owned();
                let size = e.metadata().map(|m| m.len()).unwrap_or(0);
                packs.push_str(&format!("{n:<70} {size:>16}\n"));
            }
        }
        files.push(("packs-local.txt".into(), packs.into_bytes()));
        // objects-local.txt: loose object count per fan-out directory.
        let mut stats = format!("Object directory stats for {}:\n", objects.display());
        let mut total = 0usize;
        if let Ok(rd) = std::fs::read_dir(&objects) {
            for e in rd.flatten() {
                let n = e.file_name().to_string_lossy().into_owned();
                if n.len() != 2 || !n.chars().all(|c| c.is_ascii_hexdigit()) {
                    continue;
                }
                let count = std::fs::read_dir(e.path()).map(|d| d.count()).unwrap_or(0);
                total += count;
                stats.push_str(&format!("{n} : {count:>7} files\n"));
            }
        }
        stats.push_str(&format!("Total: {total} loose objects"));
        files.push(("objects-local.txt".into(), stats.into_bytes()));
        if mode == "all" {
            // A copy of the git directory without objects/ and refs/.
            let git_dir = repo.git_dir().to_path_buf();
            let mut stack = vec![git_dir.clone()];
            while let Some(dir) = stack.pop() {
                let Ok(rd) = std::fs::read_dir(&dir) else {
                    continue;
                };
                for e in rd.flatten() {
                    let path = e.path();
                    let rel = path.strip_prefix(&git_dir).unwrap_or(&path).to_path_buf();
                    if rel == std::path::Path::new("objects") || rel == std::path::Path::new("refs")
                    {
                        continue;
                    }
                    if path.is_dir() {
                        stack.push(path);
                    } else if let Ok(b) = std::fs::read(&path) {
                        files.push((format!(".git/{}", rel.display()), b));
                    }
                }
            }
        }
    }
    // Like git, the log is echoed on stdout as it is collected.
    print!("Collecting diagnostic info\n\n{log}");
    files.insert(0, ("diagnostics.log".into(), log.into_bytes()));
    std::fs::write(zip, stored_zip(&files))?;
    Ok(())
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for &b in data {
        crc ^= u32::from(b);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xedb8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

fn stored_zip(files: &[(String, Vec<u8>)]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut central = Vec::new();
    for (name, data) in files {
        let off = out.len() as u32;
        let crc = crc32(data);
        let n = name.as_bytes();
        let size = data.len() as u32;
        out.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
        out.extend_from_slice(&[20, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        out.extend_from_slice(&crc.to_le_bytes());
        out.extend_from_slice(&size.to_le_bytes());
        out.extend_from_slice(&size.to_le_bytes());
        out.extend_from_slice(&(n.len() as u16).to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(n);
        out.extend_from_slice(data);
        central.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
        central.extend_from_slice(&[20, 0, 20, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        central.extend_from_slice(&crc.to_le_bytes());
        central.extend_from_slice(&size.to_le_bytes());
        central.extend_from_slice(&size.to_le_bytes());
        central.extend_from_slice(&(n.len() as u16).to_le_bytes());
        central.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        central.extend_from_slice(&off.to_le_bytes());
        central.extend_from_slice(n);
    }
    let cd_off = out.len() as u32;
    let cd_len = central.len() as u32;
    out.extend_from_slice(&central);
    out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
    out.extend_from_slice(&[0, 0, 0, 0]);
    out.extend_from_slice(&(files.len() as u16).to_le_bytes());
    out.extend_from_slice(&(files.len() as u16).to_le_bytes());
    out.extend_from_slice(&cd_len.to_le_bytes());
    out.extend_from_slice(&cd_off.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out
}

/// Temporary directory removed on drop.
pub(crate) struct ScratchDir(pub std::path::PathBuf);

impl ScratchDir {
    pub(crate) fn path(&self) -> &std::path::Path {
        &self.0
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Three-way line merge settings (`git merge-file` / xdiff merge).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum MergeStyle {
    /// Conflicts refined zealously, nearby conflicts joined.
    Merge,
    /// Base section shown, no refinement.
    Diff3,
    /// Base section shown, common prefix/suffix moved out of the conflict.
    Zdiff3,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum MergeFavor {
    None,
    Ours,
    Theirs,
    Union,
}

/// Split into lines that keep their `\n` (the last may lack it).
fn split_lines(t: &[u8]) -> Vec<&[u8]> {
    let mut out = Vec::new();
    let mut start = 0;
    for (i, b) in t.iter().enumerate() {
        if *b == b'\n' {
            out.push(&t[start..=i]);
            start = i + 1;
        }
    }
    if start < t.len() {
        out.push(&t[start..]);
    }
    out
}

/// Changed regions `(a0, a1, b0, b1)` between `a` and `b` (LCS based, with
/// common prefix/suffix trimmed first).
fn diff_hunks(a: &[&[u8]], b: &[&[u8]]) -> Vec<(usize, usize, usize, usize)> {
    let mut pre = 0;
    while pre < a.len() && pre < b.len() && a[pre] == b[pre] {
        pre += 1;
    }
    let mut suf = 0;
    while suf < a.len() - pre && suf < b.len() - pre && a[a.len() - 1 - suf] == b[b.len() - 1 - suf]
    {
        suf += 1;
    }
    let (am, bm) = (&a[pre..a.len() - suf], &b[pre..b.len() - suf]);
    let (n, m) = (am.len(), bm.len());
    if n == 0 && m == 0 {
        return Vec::new();
    }
    // Matched pairs via LCS table (falls back to one hunk when too large).
    let mut pairs: Vec<(usize, usize)> = Vec::new();
    if n > 0 && m > 0 && n.saturating_mul(m) <= 16_000_000 {
        let mut t = vec![0u32; (n + 1) * (m + 1)];
        for i in (0..n).rev() {
            for j in (0..m).rev() {
                t[i * (m + 1) + j] = if am[i] == bm[j] {
                    t[(i + 1) * (m + 1) + j + 1] + 1
                } else {
                    t[(i + 1) * (m + 1) + j].max(t[i * (m + 1) + j + 1])
                };
            }
        }
        let (mut i, mut j) = (0, 0);
        while i < n && j < m {
            if am[i] == bm[j] {
                pairs.push((i, j));
                i += 1;
                j += 1;
            } else if t[(i + 1) * (m + 1) + j] >= t[i * (m + 1) + j + 1] {
                i += 1;
            } else {
                j += 1;
            }
        }
    }
    let mut out: Vec<(usize, usize, usize, usize)> = Vec::new();
    let (mut i, mut j) = (0, 0);
    for (pi, pj) in pairs.into_iter().chain(std::iter::once((n, m))) {
        if pi > i || pj > j {
            out.push((pre + i, pre + pi, pre + j, pre + pj));
        }
        i = pi + 1;
        j = pj + 1;
    }
    // Like xdiff, slide pure insertions/deletions as far down as they go.
    let mut k = 0;
    while k < out.len() {
        let (mut a0, mut a1, mut b0, mut b1) = out[k];
        loop {
            let next_start = out.get(k + 1).map(|h| (h.0, h.2));
            let can = if a0 == a1 && b1 < b.len() && a0 < a.len() {
                b[b0] == b[b1] && next_start.is_none_or(|(na, _)| na > a0)
            } else if b0 == b1 && a1 < a.len() && b0 < b.len() {
                a[a0] == a[a1] && next_start.is_none_or(|(_, nb)| nb > b0)
            } else {
                false
            };
            if !can {
                break;
            }
            a0 += 1;
            a1 += 1;
            b0 += 1;
            b1 += 1;
        }
        out[k] = (a0, a1, b0, b1);
        // Join with the next hunk when they now touch.
        if let Some(nh) = out.get(k + 1).copied() {
            if nh.0 == a1 && nh.2 == b1 {
                out[k] = (a0, nh.1, b0, nh.3);
                out.remove(k + 1);
                continue;
            }
        }
        k += 1;
    }
    out
}

/// Three-way merge of `ours` and `theirs` against `base`. Returns the merged
/// bytes and the number of conflicts. Behaviour derived from observed
/// `git merge-file` output (see the family-plumbing corpus).
pub(crate) fn merge_three(
    base: &[u8],
    ours: &[u8],
    theirs: &[u8],
    labels: [&str; 3],
    style: MergeStyle,
    favor: MergeFavor,
    marker: usize,
) -> (Vec<u8>, usize) {
    let (bl, ol, tl) = (split_lines(base), split_lines(ours), split_lines(theirs));
    let h1 = diff_hunks(&bl, &ol);
    let h2 = diff_hunks(&bl, &tl);
    // Regions: Plain(ours range) or Conflict(base, ours, theirs ranges).
    enum Region {
        Plain(Vec<u8>),
        Conflict {
            b: (usize, usize),
            o: (usize, usize),
            t: (usize, usize),
        },
    }
    let mut regions: Vec<Region> = Vec::new();
    let (mut i1, mut i2) = (0usize, 0usize);
    let mut pos = 0usize; // base position emitted so far
    // Map a base index to the side index given the hunks before it.
    let map = |h: &[(usize, usize, usize, usize)], upto: usize, x: usize| -> usize {
        let mut off: isize = 0;
        for k in &h[..upto] {
            off += (k.3 - k.2) as isize - (k.1 - k.0) as isize;
        }
        (x as isize + off) as usize
    };
    let push_base = |regions: &mut Vec<Region>, from: usize, to: usize| {
        if to > from {
            let mut v = Vec::new();
            for l in &bl[from..to] {
                v.extend_from_slice(l);
            }
            regions.push(Region::Plain(v));
        }
    };
    while i1 < h1.len() || i2 < h2.len() {
        let next1 = h1.get(i1).map(|h| h.0).unwrap_or(usize::MAX);
        let next2 = h2.get(i2).map(|h| h.0).unwrap_or(usize::MAX);
        let start = next1.min(next2);
        push_base(&mut regions, pos, start);
        // Grow the region while hunks of either side overlap or touch it.
        let (s1, s2) = (i1, i2);
        let mut end = start;
        loop {
            if let Some(h) = h1.get(i1).filter(|h| h.0 <= end) {
                end = end.max(h.1);
                i1 += 1;
            } else if let Some(h) = h2.get(i2).filter(|h| h.0 <= end) {
                end = end.max(h.1);
                i2 += 1;
            } else {
                break;
            }
        }
        let o0 = map(&h1, s1, start);
        let o1 = map(&h1, i1, end);
        let t0 = map(&h2, s2, start);
        let t1 = map(&h2, i2, end);
        if i1 == s1 || i2 == s2 || ol[o0..o1] == tl[t0..t1] {
            // Only one side changed, or both changed identically.
            let src: &[&[u8]] = if i1 > s1 { &ol[o0..o1] } else { &tl[t0..t1] };
            let mut v = Vec::new();
            for l in src {
                v.extend_from_slice(l);
            }
            regions.push(Region::Plain(v));
        } else {
            regions.push(Region::Conflict {
                b: (start, end),
                o: (o0, o1),
                t: (t0, t1),
            });
        }
        pos = end;
    }
    push_base(&mut regions, pos, bl.len());

    // Refinement.
    let mut refined: Vec<Region> = Vec::new();
    for r in regions {
        let Region::Conflict { b, o, t } = r else {
            refined.push(r);
            continue;
        };
        match style {
            MergeStyle::Diff3 => refined.push(Region::Conflict { b, o, t }),
            MergeStyle::Zdiff3 => {
                let (mut o, mut t) = (o, t);
                let mut head = Vec::new();
                while o.0 < o.1 && t.0 < t.1 && ol[o.0] == tl[t.0] {
                    head.extend_from_slice(ol[o.0]);
                    o.0 += 1;
                    t.0 += 1;
                }
                let mut tail: Vec<&[u8]> = Vec::new();
                while o.1 > o.0 && t.1 > t.0 && ol[o.1 - 1] == tl[t.1 - 1] {
                    tail.push(ol[o.1 - 1]);
                    o.1 -= 1;
                    t.1 -= 1;
                }
                if !head.is_empty() {
                    refined.push(Region::Plain(head));
                }
                refined.push(Region::Conflict { b, o, t });
                if !tail.is_empty() {
                    let mut v = Vec::new();
                    for l in tail.iter().rev() {
                        v.extend_from_slice(l);
                    }
                    refined.push(Region::Plain(v));
                }
            }
            MergeStyle::Merge => {
                let hs = diff_hunks(&ol[o.0..o.1], &tl[t.0..t.1]);
                let (mut po, mut pt) = (0usize, 0usize);
                for (a0, a1, c0, c1) in hs {
                    if a0 > po {
                        let mut v = Vec::new();
                        for l in &ol[o.0 + po..o.0 + a0] {
                            v.extend_from_slice(l);
                        }
                        refined.push(Region::Plain(v));
                    }
                    refined.push(Region::Conflict {
                        b,
                        o: (o.0 + a0, o.0 + a1),
                        t: (t.0 + c0, t.0 + c1),
                    });
                    po = a1;
                    pt = c1;
                }
                let _ = pt;
                if o.0 + po < o.1 {
                    let mut v = Vec::new();
                    for l in &ol[o.0 + po..o.1] {
                        v.extend_from_slice(l);
                    }
                    refined.push(Region::Plain(v));
                }
            }
        }
    }
    // Join conflicts separated by fewer than four lines containing alnum.
    let mut joined: Vec<Region> = Vec::new();
    if style == MergeStyle::Merge {
        let mut k = 0;
        while k < refined.len() {
            if let (
                Some(Region::Conflict { .. }),
                Some(Region::Plain(gap)),
                Some(Region::Conflict { .. }),
            ) = (joined.last(), refined.get(k), refined.get(k + 1))
            {
                let alnum = split_lines(gap)
                    .iter()
                    .filter(|l| l.iter().any(|c| c.is_ascii_alphanumeric()))
                    .count();
                if alnum < 4 {
                    let gap_lines = split_lines(gap).len();
                    if let (
                        Some(Region::Conflict { b, o, t }),
                        Some(Region::Conflict {
                            b: b2,
                            o: o2,
                            t: t2,
                        }),
                    ) = (joined.pop(), refined.get(k + 1))
                    {
                        let _ = gap_lines;
                        joined.push(Region::Conflict {
                            b: (b.0.min(b2.0), b.1.max(b2.1)),
                            o: (o.0, o2.1),
                            t: (t.0, t2.1),
                        });
                        k += 2;
                        continue;
                    }
                }
            }
            joined.push(std::mem::replace(
                &mut refined[k],
                Region::Plain(Vec::new()),
            ));
            k += 1;
        }
    } else {
        joined = refined;
    }

    let mut out = Vec::new();
    let mut conflicts = 0;
    let section = |out: &mut Vec<u8>, lines: &[&[u8]]| {
        for l in lines {
            out.extend_from_slice(l);
        }
        if out.last().is_some_and(|c| *c != b'\n') && !lines.is_empty() {
            out.push(b'\n');
        }
    };
    let n = joined.len();
    for (idx, r) in joined.into_iter().enumerate() {
        match r {
            Region::Plain(v) => out.extend_from_slice(&v),
            Region::Conflict { b, o, t } => {
                let (os, ts) = (&ol[o.0..o.1], &tl[t.0..t.1]);
                match favor {
                    MergeFavor::Ours => os.iter().for_each(|l| out.extend_from_slice(l)),
                    MergeFavor::Theirs => ts.iter().for_each(|l| out.extend_from_slice(l)),
                    MergeFavor::Union => {
                        os.iter().for_each(|l| out.extend_from_slice(l));
                        if !ts.is_empty() && out.last().is_some_and(|c| *c != b'\n') {
                            out.push(b'\n');
                        }
                        ts.iter().for_each(|l| out.extend_from_slice(l));
                        if idx + 1 < n && out.last().is_some_and(|c| *c != b'\n') {
                            out.push(b'\n');
                        }
                    }
                    MergeFavor::None => {
                        conflicts += 1;
                        let m = |c: char| c.to_string().repeat(marker);
                        let label = |s: &str| {
                            if s.is_empty() {
                                String::new()
                            } else {
                                format!(" {s}")
                            }
                        };
                        out.extend_from_slice(
                            format!("{}{}\n", m('<'), label(labels[0])).as_bytes(),
                        );
                        section(&mut out, os);
                        if style != MergeStyle::Merge {
                            out.extend_from_slice(
                                format!("{}{}\n", m('|'), label(labels[1])).as_bytes(),
                            );
                            section(&mut out, &bl[b.0..b.1]);
                        }
                        out.extend_from_slice(format!("{}\n", m('=')).as_bytes());
                        section(&mut out, ts);
                        out.extend_from_slice(
                            format!("{}{}\n", m('>'), label(labels[2])).as_bytes(),
                        );
                    }
                }
            }
        }
    }
    (out, conflicts)
}
