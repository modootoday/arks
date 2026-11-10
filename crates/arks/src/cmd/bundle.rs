//! Bundle create / list-heads / verify / unbundle — wires `arks_pack` bundle codec.

use crate::cmd::replay::one_rev;
use crate::error::{CliError, CliResult};
use crate::util::{FORMAT, OID_KIND, open_repo, refuse_unknown_flags};
use arks_object::{Commit, ObjectKind, RawObject, Tree};
use arks_oid::Oid;
use arks_pack::{DecodedPack, EncodedPack, encode_bundle, parse_bundle};
use arks_repo::Repository;
use arks_store::ObjectStore;
use std::collections::HashSet;
use std::fs;
pub fn cmd_bundle(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &["--all"])?;
    let sub = args
        .first()
        .ok_or_else(|| CliError::Usage("bundle create|list-heads|unbundle".into()))?;
    match sub.as_str() {
        "create" => bundle_create(&args[1..]),
        "list-heads" => bundle_list_heads(&args[1..]),
        "unbundle" => bundle_unbundle(&args[1..]),
        "verify" => bundle_verify(&args[1..]),
        other => Err(CliError::NotImplemented(format!(
            "bundle {other} is not claimed"
        ))),
    }
}

fn bundle_create(args: &[String]) -> CliResult<()> {
    let mut all = false;
    let mut positional = Vec::new();
    for arg in args {
        if arg == "--all" {
            all = true;
        } else if !arg.starts_with('-') {
            positional.push(arg.as_str());
        }
    }
    let file = positional
        .first()
        .ok_or_else(|| CliError::Usage("bundle create <file> (--all | <ref>...)".into()))?;
    let listed = &positional[1..];
    if all && !listed.is_empty() {
        return Err(CliError::Usage(
            "bundle create: pass --all or listed refs, not both".into(),
        ));
    }
    if !all && listed.is_empty() {
        return Err(CliError::Usage(
            "bundle create <file> (--all | <ref>...)".into(),
        ));
    }
    let repo = open_repo()?;
    // MISC-003 (M20 Cx-5): bundle create while refs move — honor HEAD.lock.
    let head_lock = repo.git_dir().join("HEAD.lock");
    if head_lock.is_file() {
        return Err(CliError::Msg(format!(
            "{} (MISC-003 — bundle create under moving refs / HEAD.lock refuse; lease ≠ skip)",
            head_lock.display()
        )));
    }
    let mut refs = if all {
        repo.refs().list_refs("")?
    } else {
        let mut out = Vec::new();
        for name in listed {
            out.push(resolve_bundle_ref(&repo, name)?);
        }
        out
    };
    if refs.is_empty() {
        return Err(CliError::Msg("bundle create: no refs".into()));
    }
    refs.sort_by(|a, b| a.0.cmp(&b.0));
    refs.dedup_by(|a, b| a.0 == b.0);
    let mut seen = HashSet::new();
    let mut objects = Vec::new();
    for (_, oid) in &refs {
        walk_object(&repo, oid, &mut seen, &mut objects)?;
    }
    let encoded = EncodedPack::from_undeltified(FORMAT, &objects)?;
    let bytes = encode_bundle(&refs, &encoded.pack);
    fs::write(file, bytes)?;
    println!("created bundle {file}");
    Ok(())
}

fn resolve_bundle_ref(repo: &Repository, name: &str) -> CliResult<(String, Oid)> {
    let candidates = [
        name.to_owned(),
        format!("refs/heads/{name}"),
        format!("refs/tags/{name}"),
        format!("refs/remotes/{name}"),
    ];
    for candidate in candidates {
        if let Ok(oid) = repo.refs().resolve(&candidate) {
            return Ok((candidate, oid));
        }
    }
    Err(CliError::Msg(format!("unknown ref: {name}")))
}

fn walk_object(
    repo: &Repository,
    oid: &Oid,
    seen: &mut HashSet<Oid>,
    out: &mut Vec<RawObject>,
) -> CliResult<()> {
    if !seen.insert(*oid) {
        return Ok(());
    }
    let obj = repo.get_object(oid)?;
    match obj.kind {
        ObjectKind::Commit => {
            let commit = Commit::decode(&obj.payload)?;
            let tree = commit.tree;
            let parents = commit.parents.clone();
            out.push(obj);
            walk_object(repo, &tree, seen, out)?;
            for parent in parents {
                walk_object(repo, &parent, seen, out)?;
            }
        }
        ObjectKind::Tree => {
            let tree = Tree::decode(&obj.payload, OID_KIND)?;
            let children: Vec<Oid> = tree.entries.into_iter().map(|entry| entry.oid).collect();
            out.push(obj);
            for child in children {
                walk_object(repo, &child, seen, out)?;
            }
        }
        ObjectKind::Blob => out.push(obj),
        ObjectKind::Tag => {
            let target = tag_target(&obj.payload);
            out.push(obj);
            if let Some(target) = target {
                walk_object(repo, &target, seen, out)?;
            }
        }
    }
    Ok(())
}

fn tag_target(payload: &[u8]) -> Option<Oid> {
    let text = std::str::from_utf8(payload).ok()?;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("object ") {
            return Oid::from_hex(rest.trim()).ok();
        }
        if line.is_empty() {
            break;
        }
    }
    None
}

fn bundle_list_heads(args: &[String]) -> CliResult<()> {
    let file = one_rev(args, "bundle list-heads <file>")?;
    let bytes = fs::read(file)?;
    let bundle = parse_bundle(&bytes)?;
    for (oid, name) in bundle.refs {
        println!("{} {name}", oid.to_hex());
    }
    Ok(())
}

fn bundle_verify(args: &[String]) -> CliResult<()> {
    let file = one_rev(args, "bundle verify <file>")?;
    let bytes = fs::read(file)?;
    let bundle = parse_bundle(&bytes)?;
    if bundle.refs.is_empty() {
        return Err(CliError::Msg("bundle verify: no refs".into()));
    }
    let decoded = DecodedPack::decode(&bundle.pack, FORMAT)?;
    let mut by_oid: HashSet<Oid> = HashSet::new();
    for object in &decoded.objects {
        by_oid.insert(object.oid);
    }
    for (oid, name) in &bundle.refs {
        if !by_oid.contains(oid) {
            return Err(CliError::Msg(format!(
                "bundle verify: missing object for {name}"
            )));
        }
    }
    println!(
        "The bundle contains {} ref(s) and {} object(s)",
        bundle.refs.len(),
        decoded.objects.len()
    );
    Ok(())
}

fn bundle_unbundle(args: &[String]) -> CliResult<()> {
    let file = one_rev(args, "bundle unbundle <file>")?;
    let bytes = fs::read(file)?;
    let bundle = parse_bundle(&bytes)?;
    let decoded = DecodedPack::decode(&bundle.pack, FORMAT)?;
    let mut repo = open_repo()?;
    for object in &decoded.objects {
        let stored = repo.store_mut().put(FORMAT, &object.object)?;
        if stored != object.oid {
            return Err(CliError::Msg(format!(
                "pack object id mismatch for {}",
                object.oid.to_hex()
            )));
        }
    }
    for (oid, name) in &bundle.refs {
        if name.starts_with("refs/") {
            repo.refs_mut().update(name, oid, None)?;
        }
        println!("{} {name}", oid.to_hex());
    }
    Ok(())
}
