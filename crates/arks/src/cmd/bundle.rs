//! `bundle create | verify | list-heads | unbundle` with git's options:
//! rev-list arguments (refs, `--all`, `--branches`, `--tags`, `--remotes`,
//! `A..B`, `^A`) become refs plus `-<oid> <subject>` prerequisites, and
//! v2 / v3 headers are written and read.
//!
//! Primary: <https://git-scm.com/docs/git-bundle>,
//! <https://git-scm.com/docs/gitformat-bundle>.

use crate::cmd::opt_spec::parse;
use crate::cmd::shared::resolve_rev;
use crate::error::{CliError, CliResult};
use crate::util::open_repo;
use arks_object::{Commit, ObjectKind, RawObject, Tree, TreeMode};
use arks_oid::Oid;
use arks_pack::{DecodedPack, EncodedPack};
use arks_repo::Repository;
use arks_store::ObjectStore;
use std::collections::HashSet;
use std::fs;

const USAGE: &str = "usage: git bundle create [-q | --quiet | --progress]\n                         [--version=<version>] <file> <git-rev-list-args>\n   or: git bundle verify [-q | --quiet] <file>\n   or: git bundle list-heads <file> [<refname>...]\n   or: git bundle unbundle [--progress] <file> [<refname>...]\n";

pub fn cmd_bundle(args: &[String]) -> CliResult<()> {
    let Some(sub) = args.first() else {
        return Err(CliError::Exit(
            129,
            format!("error: need a subcommand\n{USAGE}"),
        ));
    };
    match sub.as_str() {
        "create" => bundle_create(&args[1..]),
        "list-heads" => bundle_list_heads(&args[1..]),
        "unbundle" => bundle_unbundle(&args[1..]),
        "verify" => bundle_verify(&args[1..]),
        other => Err(CliError::Exit(
            129,
            format!("error: unknown subcommand: `{other}'\n{USAGE}"),
        )),
    }
}

/// A parsed bundle: header refs, prerequisites and the pack bytes.
struct Bundle {
    version: u8,
    prereqs: Vec<(Oid, String)>,
    refs: Vec<(Oid, String)>,
    pack: Vec<u8>,
}

fn read_bundle(path: &str) -> CliResult<Bundle> {
    let bad = || {
        CliError::Exit(
            1,
            format!("error: '{path}' does not look like a v2 or v3 bundle file"),
        )
    };
    let bytes =
        fs::read(path).map_err(|_| CliError::Exit(1, format!("error: could not open '{path}'")))?;
    let version = if bytes.starts_with(b"# v2 git bundle\n") {
        2
    } else if bytes.starts_with(b"# v3 git bundle\n") {
        3
    } else {
        return Err(bad());
    };
    let mut i = 16;
    let mut prereqs = Vec::new();
    let mut refs = Vec::new();
    loop {
        let nl = bytes[i..]
            .iter()
            .position(|&b| b == b'\n')
            .ok_or_else(bad)?;
        let line = String::from_utf8_lossy(&bytes[i..i + nl]).to_string();
        i += nl + 1;
        if line.is_empty() {
            break;
        }
        if line.starts_with('@') {
            continue;
        }
        if let Some(rest) = line.strip_prefix('-') {
            let (h, subject) = rest.split_once(' ').unwrap_or((rest, ""));
            prereqs.push((Oid::from_hex(h).map_err(|_| bad())?, subject.to_string()));
        } else {
            let (h, name) = line.split_once(' ').ok_or_else(bad)?;
            refs.push((Oid::from_hex(h).map_err(|_| bad())?, name.to_string()));
        }
    }
    Ok(Bundle {
        version,
        prereqs,
        refs,
        pack: bytes[i..].to_vec(),
    })
}

/// Commits reachable from `tips` (including them).
fn reachable(repo: &Repository, tips: &[Oid]) -> CliResult<HashSet<Oid>> {
    let mut seen = HashSet::new();
    let mut stack = tips.to_vec();
    while let Some(c) = stack.pop() {
        if !seen.insert(c) {
            continue;
        }
        let obj = repo.get_object(&c)?;
        if obj.kind == ObjectKind::Commit {
            stack.extend(Commit::decode(&obj.payload)?.parents);
        }
    }
    Ok(seen)
}

fn peel_commit(repo: &Repository, oid: &Oid) -> Oid {
    let p = crate::cmd::remote_local::peel(repo, oid);
    if repo
        .get_object(&p)
        .is_ok_and(|o| o.kind == ObjectKind::Commit)
    {
        p
    } else {
        *oid
    }
}

fn bundle_create(args: &[String]) -> CliResult<()> {
    // Bundle options come first; everything after <file> is rev-list input.
    let file_pos = args
        .iter()
        .position(|a| !a.starts_with('-'))
        .ok_or_else(|| CliError::Exit(129, USAGE.into()))?;
    let p = parse(
        &args[..file_pos],
        &[
            "q|quiet",
            "|progress",
            "|all-progress",
            "|all-progress-implied",
            "|version=",
        ],
    )?;
    let version = match p.value("version").as_deref() {
        None => 2,
        Some("2") => 2,
        Some("3") => 3,
        Some(v) => return Err(CliError::fatal(format!("unsupported bundle version {v}"))),
    };
    let file = &args[file_pos];
    let revs = &args[file_pos + 1..];
    let repo = open_repo()?;
    // MISC-003 (M20 Cx-5): bundle create while refs move — honor HEAD.lock.
    let head_lock = repo.git_dir().join("HEAD.lock");
    if head_lock.is_file() {
        return Err(CliError::Msg(format!(
            "{} (MISC-003 — bundle create under moving refs / HEAD.lock refuse; lease ≠ skip)",
            head_lock.display()
        )));
    }
    let all_refs = repo.refs().list_refs("refs/")?;
    let mut refs: Vec<(String, Oid)> = Vec::new();
    let mut positive: Vec<Oid> = Vec::new();
    let mut negative: Vec<Oid> = Vec::new();
    let add_ref = |refs: &mut Vec<(String, Oid)>, n: &str, o: Oid| {
        if !refs.iter().any(|(x, _)| x == n) {
            refs.push((n.to_string(), o));
        }
    };
    for r in revs {
        match r.as_str() {
            "--all" => {
                if let Ok(h) = repo.head_oid() {
                    add_ref(&mut refs, "HEAD", h);
                }
                for (n, o) in &all_refs {
                    add_ref(&mut refs, n, *o);
                }
            }
            "--branches" | "--tags" | "--remotes" => {
                let pre = match r.as_str() {
                    "--branches" => "refs/heads/",
                    "--tags" => "refs/tags/",
                    _ => "refs/remotes/",
                };
                for (n, o) in all_refs.iter().filter(|(n, _)| n.starts_with(pre)) {
                    add_ref(&mut refs, n, *o);
                }
            }
            s if s.starts_with('^') => {
                negative.push(peel_commit(&repo, &resolve_rev(&repo, &s[1..])?))
            }
            s if s.contains("..") => {
                let (a, b) = s.split_once("..").expect("contains ..");
                let b = b.trim_start_matches('.');
                let a = if a.is_empty() { "HEAD" } else { a };
                let b = if b.is_empty() { "HEAD" } else { b };
                negative.push(peel_commit(&repo, &resolve_rev(&repo, a)?));
                push_rev(&repo, &all_refs, b, &mut refs, &mut positive)?;
            }
            s if s.starts_with('-') => {}
            s => push_rev(&repo, &all_refs, s, &mut refs, &mut positive)?,
        }
    }
    positive.extend(refs.iter().map(|(_, o)| peel_commit(&repo, o)));
    let excluded = reachable(&repo, &negative)?;
    let included: HashSet<Oid> = reachable(&repo, &positive)?
        .into_iter()
        .filter(|c| !excluded.contains(c))
        .collect();
    // Refs whose commit is excluded carry nothing new.
    refs.retain(|(_, o)| {
        let c = peel_commit(&repo, o);
        !excluded.contains(&c) || included.contains(&c)
    });
    if refs.is_empty()
        || (included.is_empty()
            && !refs
                .iter()
                .any(|(_, o)| repo.get_object(o).is_ok_and(|x| x.kind == ObjectKind::Tag)))
    {
        return Err(CliError::fatal("Refusing to create empty bundle."));
    }
    // Prerequisites: excluded parents of included commits, newest first.
    let mut prereqs: Vec<(i64, Oid, String)> = Vec::new();
    for c in &included {
        let commit = Commit::decode(&repo.get_object(c)?.payload)?;
        for par in commit.parents {
            if excluded.contains(&par) && !prereqs.iter().any(|(_, o, _)| *o == par) {
                let pc = Commit::decode(&repo.get_object(&par)?.payload)?;
                let subject = pc.message.lines().next().unwrap_or("").to_string();
                let t = pc.committer.timestamp.to_string().parse().unwrap_or(0);
                prereqs.push((t, par, subject));
            }
        }
    }
    prereqs.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.to_hex().cmp(&b.1.to_hex())));
    // Objects: included commits, their trees minus what prerequisites' trees hold.
    let mut have: HashSet<Oid> = HashSet::new();
    for (_, p, _) in &prereqs {
        let t = Commit::decode(&repo.get_object(p)?.payload)?.tree;
        collect_tree(&repo, &t, &mut have, &mut Vec::new())?;
    }
    let mut seen = have.clone();
    let mut objects: Vec<RawObject> = Vec::new();
    let mut commits: Vec<&Oid> = included.iter().collect();
    commits.sort_by_key(|o| o.to_hex());
    for c in commits {
        let obj = repo.get_object(c)?;
        let tree = Commit::decode(&obj.payload)?.tree;
        if seen.insert(*c) {
            objects.push(obj);
        }
        collect_tree(&repo, &tree, &mut seen, &mut objects)?;
    }
    for (_, o) in &refs {
        let mut cur = *o;
        while let Ok(obj) = repo.get_object(&cur) {
            if obj.kind != ObjectKind::Tag {
                break;
            }
            let next = crate::cmd::remote_local::peel(&repo, &cur);
            if seen.insert(cur) {
                objects.push(obj);
            }
            if next == cur {
                break;
            }
            cur = tag_inner(&repo, &cur).unwrap_or(next);
        }
    }
    let encoded = EncodedPack::from_undeltified(repo.object_format(), &objects)?;
    let mut out = Vec::new();
    out.extend_from_slice(if version == 3 {
        b"# v3 git bundle\n"
    } else {
        b"# v2 git bundle\n"
    });
    if version == 3 {
        out.extend_from_slice(b"@object-format=sha1\n");
    }
    for (_, o, s) in &prereqs {
        out.extend_from_slice(format!("-{} {s}\n", o.to_hex()).as_bytes());
    }
    for (n, o) in &refs {
        out.extend_from_slice(format!("{} {n}\n", o.to_hex()).as_bytes());
    }
    out.push(b'\n');
    out.extend_from_slice(&encoded.pack);
    fs::write(file, out)?;
    Ok(())
}

/// The object an annotated tag points at (one level).
fn tag_inner(repo: &Repository, oid: &Oid) -> Option<Oid> {
    let obj = repo.get_object(oid).ok()?;
    String::from_utf8_lossy(&obj.payload)
        .lines()
        .find_map(|l| l.strip_prefix("object "))
        .and_then(|h| Oid::from_hex(h.trim()).ok())
}

fn push_rev(
    repo: &Repository,
    all_refs: &[(String, Oid)],
    rev: &str,
    refs: &mut Vec<(String, Oid)>,
    positive: &mut Vec<Oid>,
) -> CliResult<()> {
    if let Some((n, o)) = crate::cmd::remote_local::dwim(rev, all_refs) {
        if !refs.iter().any(|(x, _)| x == n) {
            refs.push((n.clone(), *o));
        }
        return Ok(());
    }
    if rev == "HEAD" {
        let h = repo.head_oid()?;
        refs.push(("HEAD".into(), h));
        return Ok(());
    }
    positive.push(peel_commit(repo, &resolve_rev(repo, rev)?));
    Ok(())
}

fn collect_tree(
    repo: &Repository,
    tree: &Oid,
    seen: &mut HashSet<Oid>,
    out: &mut Vec<RawObject>,
) -> CliResult<()> {
    if !seen.insert(*tree) {
        return Ok(());
    }
    let obj = repo.get_object(tree)?;
    let t = Tree::decode(&obj.payload, repo.oid_kind())?;
    out.push(obj);
    for e in &t.entries {
        match e.mode {
            TreeMode::Tree => collect_tree(repo, &e.oid, seen, out)?,
            TreeMode::Gitlink => {}
            _ => {
                if seen.insert(e.oid) {
                    out.push(repo.get_object(&e.oid)?);
                }
            }
        }
    }
    Ok(())
}

fn filtered<'a>(b: &'a Bundle, names: &[String]) -> Vec<&'a (Oid, String)> {
    b.refs
        .iter()
        .filter(|(_, n)| names.is_empty() || names.iter().any(|x| x == n))
        .collect()
}

fn bundle_list_heads(args: &[String]) -> CliResult<()> {
    let p = parse(args, &[])?;
    let Some(file) = p.args.first() else {
        return Err(CliError::Exit(129, USAGE.into()));
    };
    let b = read_bundle(file)?;
    for (oid, name) in filtered(&b, &p.args[1..]) {
        println!("{} {name}", oid.to_hex());
    }
    Ok(())
}

/// Missing prerequisites in the current repository.
fn check_prereqs(b: &Bundle) -> CliResult<()> {
    let repo = open_repo()?;
    let missing: Vec<&(Oid, String)> = b
        .prereqs
        .iter()
        .filter(|(o, _)| repo.get_object(o).is_err())
        .collect();
    if missing.is_empty() {
        return Ok(());
    }
    let mut msg = String::from("error: Repository lacks these prerequisite commits:");
    for (o, s) in missing {
        msg.push_str(&format!("\nerror: {} {s}", o.to_hex()));
    }
    Err(CliError::Exit(1, msg))
}

fn bundle_verify(args: &[String]) -> CliResult<()> {
    let p = parse(args, &["q|quiet"])?;
    let Some(file) = p.args.first() else {
        return Err(CliError::Exit(129, USAGE.into()));
    };
    let b = read_bundle(file)?;
    check_prereqs(&b)?;
    if !p.on("quiet") {
        let n = b.refs.len();
        let mut out = String::new();
        if n == 1 {
            out.push_str("The bundle contains this ref:\n");
        } else {
            out.push_str(&format!("The bundle contains these {n} refs:\n"));
        }
        for (o, name) in &b.refs {
            out.push_str(&format!("{} {name}\n", o.to_hex()));
        }
        match b.prereqs.len() {
            0 => out.push_str("The bundle records a complete history.\n"),
            1 => out.push_str("The bundle requires this ref:\n"),
            k => out.push_str(&format!("The bundle requires these {k} refs:\n")),
        }
        for (o, _) in &b.prereqs {
            out.push_str(&format!("{} \n", o.to_hex()));
        }
        out.push_str("The bundle uses this hash algorithm: sha1\n");
        print!("{out}");
    }
    let _ = b.version;
    eprintln!("{file} is okay");
    Ok(())
}

fn bundle_unbundle(args: &[String]) -> CliResult<()> {
    let p = parse(args, &["|progress"])?;
    let Some(file) = p.args.first() else {
        return Err(CliError::Exit(129, USAGE.into()));
    };
    let b = read_bundle(file)?;
    check_prereqs(&b)?;
    let mut repo = open_repo()?;
    let decoded = DecodedPack::decode(&b.pack, repo.object_format())?;
    for object in &decoded.objects {
        let format = repo.object_format();
        let stored = repo.store_mut().put(format, &object.object)?;
        if stored != object.oid {
            return Err(CliError::Msg(format!(
                "pack object id mismatch for {}",
                object.oid.to_hex()
            )));
        }
    }
    // git-bundle unbundle stores objects only; refs are the caller's job.
    for (oid, name) in filtered(&b, &p.args[1..]) {
        println!("{} {name}", oid.to_hex());
    }
    Ok(())
}
