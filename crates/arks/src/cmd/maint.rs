//! `arks gc` / `repack` / `maintenance` — Fs-3 lease-honest consolidation.
//!
//! Primary: <https://git-scm.com/docs/git-gc>, <https://git-scm.com/docs/git-repack>
//! Lease: `maint:gc` ([[sot:root/lease-concurrency-intent]]); lease ≠ skip Git locks.
//! CORR-001: `gc --prune=now` remains refuse-closed.

use crate::error::{CliError, CliResult};
use crate::util::{open_repo, refuse_unknown_flags};
use arks_lease::{DurableLeaseStore, LeaseError, maint_scope};
use arks_object::{Commit, ObjectKind, RawObject, Tree};
use arks_oid::Oid;
use arks_pack::EncodedPack;
use arks_repo::Repository;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::time::Duration;

const HOLDER: &str = "arks-maint";
const TTL_SECS: u64 = 120;

pub fn cmd_gc(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &["--prune=now", "--prune", "--force", "--auto"])?;
    // HOOK-002 (M20 Cx-4): pre-auto-gc honesty — agents must not assume silent auto-gc.
    // Cite: https://git-scm.com/docs/git-gc + githooks pre-auto-gc
    if args.iter().any(|a| a == "--auto") {
        eprintln!(
            "note: HOOK-002 — pre-auto-gc may block/suppress housekeeping under agents; inspect hooks before assuming auto-gc runs"
        );
    }
    if args.iter().any(|a| a == "--force") {
        // CORR-002 (M19 Cx-2): forced overlapping gc — refuse-closed (safer than dual gc).
        return Err(CliError::Msg(
            "gc --force refused (CORR-002 honesty): overlapping forced gc risks pack/prune interference; use single `arks gc` / `arks repack -ad` under maint:gc lease".into(),
        ));
    }
    if args
        .iter()
        .any(|a| a == "--prune=now" || a.starts_with("--prune="))
    {
        // CORR-004 (M20 Cx-6): partial-clone + aggressive prune misunderstanding.
        let repo = open_repo()?;
        let filters = repo.common_dir().join("objects/info/filters");
        if filters.is_file() {
            return Err(CliError::Msg(
                "gc --prune=now refused (CORR-004 / CORR-001): aggressive prune on partial/filtered clone risks dropping lazily-needed objects".into(),
            ));
        }
        // CORR-007 (M20 Cx-8): replace-refs + concurrent prune.
        let replace_dir = repo.common_dir().join("refs/replace");
        if replace_dir.is_dir() {
            if let Ok(rd) = fs::read_dir(&replace_dir) {
                if rd.flatten().next().is_some() {
                    return Err(CliError::Msg(
                        "gc --prune=now refused (CORR-007): replace-refs targets must not be pruned while replace refs live".into(),
                    ));
                }
            }
        }
        // CORR-006 / OBJ-006 (M20 Cx-8): alternates donor under GC.
        let alts = repo.common_dir().join("objects/info/alternates");
        if alts.is_file() {
            return Err(CliError::Msg(
                "gc --prune=now refused (CORR-006 / OBJ-006): alternates borrowers race with donor prune; do not prune shared alternate ODBs under live borrowers".into(),
            ));
        }
        return Err(CliError::Msg(
            "gc --prune=now refused (CORR-001 / PACK-004 honesty): prune=now under writers risks object deletion races; use `arks repack -ad` / `arks gc` without --prune=now under maint:gc lease".into(),
        ));
    }
    if args.iter().any(|a| a == "--prune") {
        return Err(CliError::NotImplemented(
            "gc --prune <date> not claimed; omit prune or see CORR-001 refuse for --prune=now"
                .into(),
        ));
    }
    run_repack(&["-a".into(), "-d".into()])
}

pub fn cmd_repack(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &["-a", "-d", "-ad", "-A"])?;
    let mut normalized = Vec::new();
    for a in args {
        if a == "-ad" || a == "-da" {
            normalized.push("-a".into());
            normalized.push("-d".into());
        } else {
            normalized.push(a.clone());
        }
    }
    if !normalized.iter().any(|a| a == "-a" || a == "-A") {
        return Err(CliError::Usage(
            "repack: Fs-3 claims -a / -ad (pack all reachable); pass -a".into(),
        ));
    }
    run_repack(&normalized)
}

pub fn cmd_maintenance(args: &[String]) -> CliResult<()> {
    let sub = args
        .first()
        .map(String::as_str)
        .ok_or_else(|| CliError::Usage("maintenance run".into()))?;
    match sub {
        "run" => {
            refuse_unknown_flags(&args[1..], &[])?;
            let repo = open_repo()?;
            // MAINT-005 (M20 Cx-7): commit-graph / concurrent maintenance task refuse.
            // Cite: https://git-scm.com/docs/git-maintenance + commit-graph
            let cg_lock = repo.common_dir().join("objects/info/commit-graph.lock");
            let cg_lock_alt = repo.git_dir().join("commit-graph.lock");
            let maint_lock = repo.common_dir().join("maintenance.lock");
            if cg_lock.is_file() || cg_lock_alt.is_file() || maint_lock.is_file() {
                let p = if cg_lock.is_file() {
                    cg_lock
                } else if cg_lock_alt.is_file() {
                    cg_lock_alt
                } else {
                    maint_lock
                };
                return Err(CliError::Msg(format!(
                    "{} (MAINT-005 — concurrent maintenance / commit-graph verify-vs-rewrite refuse; lease ≠ skip)",
                    p.display()
                )));
            }
            // PACK-005 (M20 Cx-7): cruft/.keep under maintenance honesty refuse when cruft lock held.
            let pack_dir = repo.common_dir().join("objects/pack");
            if pack_dir.is_dir() {
                if let Ok(rd) = fs::read_dir(&pack_dir) {
                    let cruft_lock = rd.flatten().any(|e| {
                        let n = e.file_name().to_string_lossy().to_string();
                        n.contains("cruft") && n.ends_with(".lock")
                    });
                    if cruft_lock {
                        return Err(CliError::Msg(
                            "PACK-005 — cruft pack lock under objects/pack; refuse overlapping maintenance vs cruft writers".into(),
                        ));
                    }
                }
            }
            // MAINT-006 (M20 Cx-8): incremental MIDX under pack add.
            let midx_lock = pack_dir.join("multi-pack-index.lock");
            if midx_lock.is_file() {
                return Err(CliError::Msg(format!(
                    "{} (MAINT-006 — incremental multi-pack-index under pack add refuse; serialize midx vs fetch)",
                    midx_lock.display()
                )));
            }
            drop(repo);
            run_repack(&["-a".into(), "-d".into()])
        }
        other => Err(CliError::NotImplemented(format!(
            "maintenance {other} not claimed (run only); see cli-flag-parity-full"
        ))),
    }
}

fn run_repack(args: &[String]) -> CliResult<()> {
    let delete_redundant = args.iter().any(|a| a == "-d");
    let mut repo = open_repo()?;
    // PACK-001 (M20 Cx-5): concurrent pack-objects / index-pack while repack.
    let pack_dir = repo.common_dir().join("objects/pack");
    if pack_dir.is_dir() {
        if let Ok(rd) = fs::read_dir(&pack_dir) {
            let colliding = rd.flatten().any(|e| {
                let n = e.file_name().to_string_lossy().to_string();
                n.ends_with(".lock")
                    && (n.contains("tmp_pack")
                        || n.ends_with(".pack.lock")
                        || n.contains("pack-") && n.ends_with(".lock"))
            });
            if colliding {
                return Err(CliError::Msg(
                    "PACK-001 — concurrent pack writer lock under objects/pack; refuse overlapping repack/index-pack (lease ≠ skip)".into(),
                ));
            }
        }
    }
    let mut lease = DurableLeaseStore::open(repo.git_dir()).map_err(lease_err)?;
    lease.manager_mut().expire_due();
    let resource = maint_scope("gc");
    lease
        .manager_mut()
        .grant(&resource, HOLDER, Duration::from_secs(TTL_SECS))
        .map_err(lease_err)?;
    lease.save().map_err(lease_err)?;

    let result = (|| -> CliResult<()> {
        let refs = repo.refs().list_refs("")?;
        let mut seen = HashSet::new();
        let mut objects = Vec::new();
        for (name, oid) in &refs {
            // Include notes refs in reachable set.
            let _ = name;
            walk_object(&repo, oid, &mut seen, &mut objects)?;
        }
        // Also walk HEAD when symbolic resolves outside listed tip (usually covered).
        if let Ok(head) = repo.head_oid() {
            walk_object(&repo, &head, &mut seen, &mut objects)?;
        }
        if objects.is_empty() {
            println!("Nothing new to pack.");
            return Ok(());
        }
        let format = repo.object_format();
        let encoded = EncodedPack::from_undeltified(format, &objects)?;
        repo.store_mut()
            .ingest_pack_bytes(&encoded.pack, format)
            .map_err(|e| CliError::Msg(e.to_string()))?;

        if delete_redundant {
            delete_loose_for(&repo, &seen)?;
            delete_old_packs(repo.store().objects_dir(), &encoded.pack)?;
        }
        println!(
            "repacked {} object(s) under maint:gc (lease ≠ skip Git locks)",
            objects.len()
        );
        Ok(())
    })();

    // Always release our lease (best-effort).
    let _ = lease.manager_mut().release(&resource, HOLDER);
    let _ = lease.save();
    result
}

fn delete_loose_for(repo: &Repository, oids: &HashSet<Oid>) -> CliResult<()> {
    let objects = repo.store().objects_dir();
    for oid in oids {
        let hex = oid.to_hex();
        if hex.len() < 3 {
            continue;
        }
        let path = objects.join(&hex[..2]).join(&hex[2..]);
        if path.is_file() {
            let _ = fs::remove_file(&path);
        }
    }
    Ok(())
}

fn delete_old_packs(objects_dir: &Path, new_pack: &[u8]) -> CliResult<()> {
    let pack_dir = objects_dir.join("pack");
    if !pack_dir.is_dir() {
        return Ok(());
    }
    // Identify new pack by content equality of written file names via checksum suffix
    // already chosen by ingest; keep any pack that still matches new bytes, drop others
    // unless a `.keep` sibling exists.
    let mut keep_names = HashSet::new();
    for ent in fs::read_dir(&pack_dir)? {
        let ent = ent?;
        let name = ent.file_name();
        let name = name.to_string_lossy();
        if name.ends_with(".keep") {
            let base = name.trim_end_matches(".keep");
            keep_names.insert(format!("{base}.pack"));
            keep_names.insert(format!("{base}.idx"));
        }
    }
    for ent in fs::read_dir(&pack_dir)? {
        let ent = ent?;
        let path = ent.path();
        let name = ent.file_name();
        let name = name.to_string_lossy().to_string();
        if !(name.ends_with(".pack") || name.ends_with(".idx")) {
            continue;
        }
        if keep_names.contains(&name) {
            continue;
        }
        if name.ends_with(".pack") {
            if let Ok(bytes) = fs::read(&path) {
                if bytes == new_pack {
                    continue;
                }
            }
        } else if name.ends_with(".idx") {
            // Drop idx when corresponding pack is dropped; if pack kept, keep idx.
            let pack_name = name.replacen(".idx", ".pack", 1);
            let pack_path = pack_dir.join(&pack_name);
            if pack_path.is_file() {
                if let Ok(bytes) = fs::read(&pack_path) {
                    if bytes == new_pack {
                        continue;
                    }
                }
                if keep_names.contains(&pack_name) {
                    continue;
                }
            }
        }
        let _ = fs::remove_file(&path);
    }
    Ok(())
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
            let tree = Tree::decode(&obj.payload, repo.oid_kind())?;
            let children: Vec<Oid> = tree.entries.into_iter().map(|e| e.oid).collect();
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

fn lease_err(e: LeaseError) -> CliError {
    match e {
        LeaseError::AlreadyHeld {
            resource_id,
            holder,
        } => CliError::Msg(format!(
            "maint:gc lease already held by {holder} on {resource_id}; PACK-004/MAINT-003: wait/backoff — do not race gc/repack under writers (lease ≠ skip Git locks)"
        )),
        other => CliError::Msg(other.to_string()),
    }
}
