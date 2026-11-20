//! `arks gc` / `repack` / `maintenance` — Fs-3 lease-honest consolidation.
//!
//! Primary: <https://git-scm.com/docs/git-gc>, <https://git-scm.com/docs/git-repack>
//! Lease: `maint:gc` ([[sot:root/lease-concurrency-intent]]); lease ≠ skip Git locks.
//! `gc --prune=now` prunes like stock git (argv symmetry ladder rung A2).

use crate::error::{CliError, CliResult};
use crate::util::{open_repo, refuse_unknown_flags};
use arks_lease::{DurableLeaseStore, LeaseError, maint_scope};
use arks_object::RawObject;
use arks_oid::Oid;
use arks_pack::EncodedPack;
use arks_store::ObjectStore;
use std::collections::HashSet;
use std::fs;
use std::time::Duration;

const HOLDER: &str = "arks-maint";
const TTL_SECS: u64 = 120;

/// `arks gc [--aggressive] [--auto] [--[no-]detach] [--quiet] [--prune=<date> | --no-prune]
/// [--force] [--keep-largest-pack] [--[no-]cruft] [--max-cruft-size=<n>]`.
///
/// Runs `pack-refs --all`, an all-in-one repack of everything reachable from
/// refs, reflogs and the index (loose copies removed, unreachable packed
/// objects turned loose) and `prune --expire <date>` (default
/// `gc.pruneExpire`, else 2.weeks.ago). Unreachable recent objects stay
/// loose; cruft packs are not written.
pub fn cmd_gc(args: &[String]) -> CliResult<()> {
    use crate::cmd::opt_parse::{self, OptDef, Takes::*, o};
    const SPEC: &[OptDef] = &[
        o("quiet", 'q', No, true),
        o("prune", ' ', Opt, true),
        o("cruft", ' ', No, true),
        o("max-cruft-size", ' ', Req, true),
        o("aggressive", ' ', No, true),
        o("auto", ' ', No, true),
        o("force", ' ', No, true),
        o("keep-largest-pack", ' ', No, true),
        o("detach", ' ', No, true),
    ];
    let items = opt_parse::parse("gc", args, SPEC, opt_parse::Mode::default())?;
    if !items.operands().is_empty() {
        return Err(CliError::Exit(129, "usage: git gc [<options>]".into()));
    }
    let repo = open_repo()?;
    let cfg = arks_config::ConfigSet::load(Some(repo.git_dir()));
    if items.on("auto") {
        // gc.auto threshold: run only with more loose objects than the limit
        // (git estimates from objects/17; the full count is used here).
        let limit: usize = cfg
            .get("gc.auto")
            .and_then(|v| v.parse().ok())
            .unwrap_or(6700);
        let loose = crate::cmd::plumbing_ext::loose_objects(&repo).len();
        if limit == 0 || loose <= limit {
            return Ok(());
        }
    }
    // --prune: last of --prune[=<date>] / --no-prune; bare --prune keeps the default.
    let mut expire: Option<String> = Some(
        cfg.get("gc.pruneexpire")
            .unwrap_or_else(|| "2.weeks.ago".into()),
    );
    for (_, value, neg) in items.opts().filter(|(key, _, _)| *key == "prune") {
        if neg {
            expire = None;
        } else if let Some(v) = value {
            expire = Some(v.to_string());
        }
    }
    let cutoff =
        match &expire {
            Some(v) if v == "never" => None,
            Some(v) => Some(crate::cmd::config::expiry_date(v).ok_or_else(|| {
                CliError::fatal(format!("failed to parse prune expiry value {v}"))
            })?),
            None => None,
        };
    if expire.as_deref() == Some("now") {
        // Environment-specific refusals kept from the concurrency register:
        // pruning everything at once is unsafe for these layouts.
        let common = repo.common_dir();
        if common.join("objects/info/filters").is_file() {
            return Err(CliError::Msg(
                "gc --prune=now refused (CORR-004): aggressive prune on partial/filtered clone risks dropping lazily-needed objects".into(),
            ));
        }
        if fs::read_dir(common.join("refs/replace")).is_ok_and(|mut rd| rd.next().is_some()) {
            return Err(CliError::Msg(
                "gc --prune=now refused (CORR-007): replace-refs targets must not be pruned while replace refs live".into(),
            ));
        }
        if common.join("objects/info/alternates").is_file() {
            return Err(CliError::Msg(
                "gc --prune=now refused (CORR-006 / OBJ-006): alternates borrowers race with donor prune; do not prune shared alternate ODBs under live borrowers".into(),
            ));
        }
    }
    drop(repo);
    if cfg.get("gc.packrefs").as_deref() != Some("false") {
        crate::cmd::inspect::cmd_pack_refs(&["--all".to_string(), "--prune".to_string()])?;
    }
    run_repack(&RepackOpts {
        all: true,
        loosen_unreachable: true,
        delete: true,
        quiet: true,
        keep_largest: items.on("keep-largest-pack")
            || cfg.get("gc.biggestpackthreshold").is_some_and(|v| v != "0"),
        ..RepackOpts::default()
    })?;
    if let Some(cut) = cutoff {
        let repo = open_repo()?;
        let reachable = crate::cmd::plumbing_ext::reachable_all(&repo, &[])?;
        for (oid, path) in crate::cmd::plumbing_ext::loose_objects(&repo) {
            if reachable.contains(&oid) {
                continue;
            }
            let mtime = fs::metadata(&path)
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            if mtime <= cut {
                let _ = fs::remove_file(&path);
                if let Some(dir) = path.parent() {
                    let _ = fs::remove_dir(dir);
                }
            }
        }
    }
    Ok(())
}

/// `arks repack [-a] [-A] [-d] [-f] [-F] [-l] [-n] [-q] [-b] [-m] [--window=<n>]
/// [--depth=<n>] [--threads=<n>] [--keep-pack=<pack-name>] [--cruft] ...`.
///
/// Without `-a`/`-A` the loose reachable objects go into one new pack. With
/// `-a`/`-A` everything reachable (refs, reflogs, index) goes into one pack;
/// `-d` then removes the old packs (`.keep` and `--keep-pack` ones excepted)
/// and loose copies, `-A` (or `--cruft`) turns unreachable packed objects
/// loose and `-k` keeps them in the new pack. Delta and window tuning
/// options are accepted; packs are written undeltified.
pub fn cmd_repack(args: &[String]) -> CliResult<()> {
    use crate::cmd::opt_parse::{self, OptDef, Takes::*, o};
    const SPEC: &[OptDef] = &[
        o("-a", 'a', No, false),
        o("-A", 'A', No, false),
        o("cruft", ' ', No, true),
        o("cruft-expiration", ' ', Req, true),
        o("max-cruft-size", ' ', Req, true),
        o("-d", 'd', No, false),
        o("-f", 'f', No, false),
        o("-F", 'F', No, false),
        o("-n", 'n', No, false),
        o("quiet", 'q', No, true),
        o("local", 'l', No, true),
        o("write-bitmap-index", 'b', No, true),
        o("delta-islands", 'i', No, true),
        o("unpack-unreachable", ' ', Req, true),
        o("keep-unreachable", 'k', No, true),
        o("window", ' ', Req, true),
        o("window-memory", ' ', Req, true),
        o("depth", ' ', Req, true),
        o("threads", ' ', Req, true),
        o("max-pack-size", ' ', Req, true),
        o("filter", ' ', Req, true),
        o("pack-kept-objects", ' ', No, true),
        o("keep-pack", ' ', Req, true),
        o("geometric", 'g', Req, true),
        o("write-midx", 'm', No, true),
        o("expire-to", ' ', Req, true),
        o("filter-to", ' ', Req, true),
    ];
    // Positional arguments are accepted and ignored, as in stock git.
    let items = opt_parse::parse("repack", args, SPEC, opt_parse::Mode::default())?;
    let on = |n: &str| items.on(n);
    let keep_packs = items.values("keep-pack");
    let cruft = on("cruft");
    run_repack(&RepackOpts {
        all: on("-a") || on("-A") || cruft,
        loosen_unreachable: (on("-A") || cruft) && !on("keep-unreachable"),
        keep_unreachable: on("keep-unreachable"),
        delete: on("-d"),
        quiet: on("quiet"),
        pack_kept: on("pack-kept-objects"),
        keep_packs,
        keep_largest: false,
    })
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
            run_repack(&RepackOpts {
                all: true,
                delete: true,
                quiet: true,
                ..RepackOpts::default()
            })
        }
        other => Err(CliError::NotImplemented(format!(
            "maintenance {other} not claimed (run only); see cli-flag-parity-full"
        ))),
    }
}

#[derive(Default)]
struct RepackOpts {
    all: bool,
    loosen_unreachable: bool,
    keep_unreachable: bool,
    delete: bool,
    quiet: bool,
    pack_kept: bool,
    keep_packs: Vec<String>,
    keep_largest: bool,
}

fn run_repack(o: &RepackOpts) -> CliResult<()> {
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
        let reachable = crate::cmd::plumbing_ext::reachable_all(&repo, &[])?;
        let loose: Vec<(Oid, std::path::PathBuf)> = crate::cmd::plumbing_ext::loose_objects(&repo);
        // Existing packs: (base path, kept?, object names).
        let mut packs: Vec<(std::path::PathBuf, bool, Vec<Oid>)> = Vec::new();
        if let Ok(rd) = fs::read_dir(&pack_dir) {
            let mut idx: Vec<std::path::PathBuf> = rd
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|x| x == "idx"))
                .collect();
            idx.sort();
            for p in idx {
                let Ok(bytes) = fs::read(&p) else { continue };
                let Ok(index) = arks_pack::PackIndex::decode(&bytes, repo.object_format()) else {
                    continue;
                };
                let base = p.with_extension("");
                let name = base
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let kept = !o.pack_kept
                    && (base.with_extension("keep").is_file()
                        || o.keep_packs.iter().any(|k| {
                            k.trim_end_matches(".pack") == name || k == &format!("{name}.pack")
                        }));
                packs.push((base, kept, index.entries.iter().map(|e| e.oid).collect()));
            }
        }
        if o.keep_largest {
            let size = |b: &std::path::PathBuf| {
                fs::metadata(b.with_extension("pack"))
                    .map(|m| m.len())
                    .unwrap_or(0)
            };
            if let Some(i) = (0..packs.len()).max_by_key(|i| size(&packs[*i].0)) {
                packs[i].1 = true;
            }
        }
        let in_kept: HashSet<Oid> = packs
            .iter()
            .filter(|p| p.1)
            .flat_map(|p| p.2.iter().copied())
            .collect();
        let in_any_pack: HashSet<Oid> = packs.iter().flat_map(|p| p.2.iter().copied()).collect();
        // Objects for the new pack.
        let mut want: Vec<Oid> = Vec::new();
        let mut chosen: HashSet<Oid> = HashSet::new();
        if o.all {
            let mut sorted: Vec<Oid> = reachable
                .iter()
                .filter(|x| !in_kept.contains(*x))
                .copied()
                .collect();
            if o.keep_unreachable {
                sorted.extend(
                    packs
                        .iter()
                        .filter(|p| !p.1)
                        .flat_map(|p| p.2.iter().copied())
                        .chain(loose.iter().map(|(x, _)| *x))
                        .filter(|x| !reachable.contains(x)),
                );
            }
            sorted.sort_by_key(|x| x.to_hex());
            for x in sorted {
                if chosen.insert(x) {
                    want.push(x);
                }
            }
        } else {
            for (x, _) in &loose {
                if reachable.contains(x) && !in_any_pack.contains(x) && chosen.insert(*x) {
                    want.push(*x);
                }
            }
        }
        let mut objects: Vec<RawObject> = Vec::new();
        for x in &want {
            if let Ok(obj) = repo.get_object(x) {
                objects.push(obj);
            }
        }
        if objects.is_empty() {
            if !o.quiet {
                println!("Nothing new to pack.");
            }
            return Ok(());
        }
        // Unreachable objects of packs about to be deleted, turned loose (-A).
        let mut loosen: Vec<RawObject> = Vec::new();
        if o.all && o.delete && o.loosen_unreachable {
            for p in packs.iter().filter(|p| !p.1) {
                for x in &p.2 {
                    if !reachable.contains(x) {
                        if let Ok(obj) = repo.get_object(x) {
                            loosen.push(obj);
                        }
                    }
                }
            }
        }
        let format = repo.object_format();
        let encoded = EncodedPack::from_undeltified(format, &objects)?;
        repo.store_mut()
            .ingest_pack_bytes(&encoded.pack, format)
            .map_err(|e| CliError::Msg(e.to_string()))?;
        if o.delete {
            if o.all {
                for (base, kept, _) in &packs {
                    if *kept {
                        continue;
                    }
                    if fs::read(base.with_extension("pack")).is_ok_and(|b| b == encoded.pack) {
                        continue;
                    }
                    for ext in ["pack", "idx", "rev", "bitmap", "mtimes", "promisor"] {
                        let _ = fs::remove_file(base.with_extension(ext));
                    }
                }
                for obj in loosen {
                    let _ = repo.store_mut().put(format, &obj);
                }
            }
            // prune-packed: loose copies of packed objects.
            let packed_now: HashSet<Oid> = chosen.union(&in_kept).copied().collect();
            let still_packed: HashSet<Oid> = if o.all {
                packed_now
            } else {
                packed_now.union(&in_any_pack).copied().collect()
            };
            for (x, path) in &loose {
                if still_packed.contains(x) {
                    let _ = fs::remove_file(path);
                    if let Some(dir) = path.parent() {
                        let _ = fs::remove_dir(dir);
                    }
                }
            }
        }
        Ok(())
    })();

    // Always release our lease (best-effort).
    let _ = lease.manager_mut().release(&resource, HOLDER);
    let _ = lease.save();
    result
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
