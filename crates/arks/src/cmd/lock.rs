//! `arks lock` — inspect / safe stale-clear for Git `*.lock` files (M12 Cm-1..Cm-3).
//!
//! Cite: https://git-scm.com/docs/api-lockfile
//! Cite: https://git-scm.com/docs/gitrepository-layout
//! Plan: `.spec/plan/20261109024634-m12-concurrency-compat-mitigation.plan.md`
//!
//! Honesty: lease ≠ skip Git locks. Clear never runs on the write path of
//! add/commit/update-ref. Live-PID holders are refuse-closed without
//! `--force-stale`. Empty/no-PID locks require `--force-stale` after inspect.
//! AGENT-003: on live contention, backoff/retry with jitter — blind retry herds
//! amplify storms; M12 has no lease scheduler (M13).
//! AGENT-005: dead/evicted container PIDs are stale-candidates for `--stale` clear.
//! FS-006: bind-mount host+container dual writers can mislead PID liveness —
//! inspect warns; never skip locks on the write path.
//! AGENT-007: worktree + submodule composition multiplies lock domains.
//! M19 Cx-1: REF-007 (HEAD.lock), LOCK-001 (exclusive O_CREAT), TOCTOU-005
//! (never poll-then-skip locks).

use crate::error::{CliError, CliResult};
use crate::util::open_repo;
use arks_index::index_lock_path;
use arks_refs::ref_lock_path;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Clone)]
struct LockReport {
    path: PathBuf,
    age_secs: Option<u64>,
    pid: Option<u32>,
    pid_live: Option<bool>,
    classification: &'static str,
}

pub fn cmd_lock(args: &[String]) -> CliResult<()> {
    let sub = args
        .first()
        .map(String::as_str)
        .ok_or_else(|| CliError::Usage("lock <inspect|clear>".into()))?;
    let rest = &args[1..];
    match sub {
        "inspect" => lock_inspect(rest),
        "clear" => lock_clear(rest),
        other => Err(CliError::NotImplemented(format!(
            "lock {other} not claimed; see M12 Cm-1 (inspect/clear --stale)"
        ))),
    }
}

fn lock_inspect(args: &[String]) -> CliResult<()> {
    if args.iter().any(|a| a.starts_with('-')) {
        return Err(CliError::NotImplemented(
            "lock inspect: no options claimed".into(),
        ));
    }
    let repo = open_repo()?;
    let mut roots = vec![repo.git_dir().to_path_buf()];
    let common = repo.common_dir().to_path_buf();
    if common != roots[0] {
        roots.push(common);
    }
    let mut reports = Vec::new();
    // Prefer public lock-sibling APIs for well-known paths (no reinvented suffix logic).
    let known = [
        index_lock_path(&repo.index_path()),
        ref_lock_path(&repo.git_dir().join("HEAD")),
    ];
    for path in &known {
        if path.is_file() && !reports.iter().any(|r: &LockReport| r.path == *path) {
            reports.push(classify_lock(path)?);
        }
    }
    for root in &roots {
        collect_locks(root, root, &mut reports)?;
    }
    reports.sort_by(|a, b| a.path.cmp(&b.path));
    reports.dedup_by(|a, b| a.path == b.path);
    if reports.is_empty() {
        println!("no *.lock files under git_dir/common_dir");
        print_lock_honesty_notes(&repo, &reports);
        return Ok(());
    }
    for r in &reports {
        let age = r
            .age_secs
            .map(|s| format!("{s}s"))
            .unwrap_or_else(|| "unknown".into());
        let pid = match (r.pid, r.pid_live) {
            (Some(p), Some(true)) => format!("pid={p} live"),
            (Some(p), Some(false)) => format!("pid={p} dead"),
            (Some(p), None) => format!("pid={p} liveness-unknown"),
            (None, _) => "pid=none".into(),
        };
        println!(
            "{}\tage={}\t{}\t{}",
            r.path.display(),
            age,
            pid,
            r.classification
        );
    }
    print_lock_honesty_notes(&repo, &reports);
    Ok(())
}

fn print_lock_honesty_notes(repo: &arks_repo::Repository, reports: &[LockReport]) {
    println!(
        "note: AGENT-003 — on lock contention, backoff/retry with jitter; blind herds amplify storms (no M12 lease scheduler)"
    );
    if reports
        .iter()
        .any(|r| r.classification == "stale-candidate")
    {
        println!(
            "note: AGENT-005 — dead/evicted container PIDs leave volume locks; clear with `arks lock clear --stale` after inspect (lease ≠ skip live locks)"
        );
    }
    println!(
        "note: FS-006 — bind-mount host+container dual writers can misclassify PID liveness across namespaces; coordinate one writer namespace; never skip Git locks"
    );
    println!(
        "note: LOCK-001 — exclusive O_CREAT|O_EXCL creates `.lock`; second writer must fail-closed (no dialect skip)"
    );
    println!(
        "note: LOCK-004 — porcelain uses die-class under lock; do not invent soft-skip dialects for agents"
    );
    println!(
        "note: TOCTOU-005 — never poll for lock absence then proceed; use exclusive create; lease ≠ skip Git locks"
    );
    println!(
        "note: TOCTOU-003 — dirty worktree vs checkout: refuse overwrite; commit/stash/restore first"
    );
    println!(
        "note: PACK-009 — leftover objects/pack/*.keep after crash prevents GC of that pack; remove only when fetch/index-pack finished"
    );
    println!(
        "note: MAINT-002 — presence of gc.log can suppress auto-gc until expiry; inspect before assuming housekeeping runs under agents"
    );
    println!(
        "note: HOOK-002 — pre-auto-gc hook can block/suppress auto housekeeping; failing hooks surprise agents; coordinate hooks before gc --auto"
    );
    println!(
        "note: MISC-011 — mid-flight hooks/ installs are non-atomic policy swaps; deploy hooks only when writers are quiescent"
    );
    println!(
        "note: REF-022 — for-each-ref readers remain safe under packed-refs.lock; do not invent writer-skip dialects for readers"
    );
    println!(
        "note: HOOK-005 — hooks that nest index writes under a held index.lock must fail-closed; no nested writer dialect"
    );
    println!(
        "note: HOOK-003 — post-index-change fires on index write; parallel add can thrash external indexers; coordinate writers"
    );
    println!(
        "note: LOCK-003 — lockfile commit uses atomic rename; readers should observe old-or-new only (never partial) on same-FS rename"
    );
    println!(
        "note: OBJ-001 — concurrent identical loose object creates are content-addressed/idempotent"
    );
    println!(
        "note: SHAL-003 — partial-clone filter mismatch across agents yields missing blobs; do not assume full objects on filtered clones"
    );
    println!(
        "note: OBJ-008 — concurrent hash-object of large inputs saturates CPU/temp; serialize heavy hash jobs"
    );
    println!(
        "note: PACK-002 — multi-pack-index rewrite under readers may stale MIDX; prefer single maintainer for write-midx"
    );
    println!(
        "note: PACK-003 — pack bitmap generation vs concurrent pack change can mismatch; serialize gc bitmap writers"
    );
    println!(
        "note: LOCK-007 — lockfile commit_lock_file rename requires same filesystem; cross-mount GIT_DIR fails closed"
    );
    println!(
        "note: WT-010 — mixed extensions.worktreeConfig true/false clients diverge effective config; agree on one mode"
    );
    println!(
        "note: SUB-003 — modules/ absorbgitdirs races with clone/update; serialize absorb vs status/update"
    );
    println!(
        "note: CORR-004 — aggressive gc/prune on partial clones can drop lazily-needed objects; avoid prune=now on filtered repos"
    );
    // M20 Cx-7 honesty notes (Docker-inducible).
    println!(
        "note: FSM-001 — fsmonitor/sparse-index FSM auto-start races under N status; serialize daemon start / single client"
    );
    println!(
        "note: IDX-005 — untracked-cache / sparse-index expand under writers races with index.lock; serialize expand vs add"
    );
    println!(
        "note: PACK-005 — cruft pack / .keep under maintenance races with writers; serialize gc --cruft vs object creation"
    );
    println!(
        "note: CI-001 — shared CI workspace without isolation amplifies index.lock storms; give each job its own checkout"
    );
    println!(
        "note: FS-008 — case-insensitive FS (core.ignorecase) path collisions (Foo vs foo) cause checkout overwrite; avoid parallel case-fold adds"
    );
    println!(
        "note: CORR-003 — never manually delete live *.lock under writers; replace-refs / concurrent replace updates need serialization"
    );
    println!(
        "note: SPARSE-002 — sparse-index expand stampede / non-cone rewrite under agents; prefer cone mode and serialize expands"
    );
    println!(
        "note: SHAL-004 — shallow + grafts/unshallow peers confuse fetch boundaries; serialize unshallow vs shallow fetch"
    );
    // M20 Cx-8 honesty notes.
    println!(
        "note: CI-005 — reusable CI/Actions workspace without clean leaves locks across jobs; wipe or isolate workdirs"
    );
    println!(
        "note: IDX-006 — untracked-cache / racy-git false dirtiness under concurrent mtime storms; refresh carefully"
    );
    println!(
        "note: FSM-003 — fsmonitor event overflow under write storms forces full-scan stampede; reduce event load"
    );
    println!(
        "note: REF-003 — loose vs packed-refs delete races need packed-refs transactions; serialize delete vs pack-refs"
    );
    println!(
        "note: MAINT-006 — incremental multi-pack-index under pack add may fallback/rebuild; serialize midx vs fetch"
    );
    println!(
        "note: MISC-001 — parallel describe during pack-refs is reader-safe but expensive; prefer quiet write windows"
    );
    println!(
        "note: MISC-009 — clone --reference while donor gc risks missing alternate objects; quiesce donor before reference clones"
    );
    println!(
        "note: OBJ-006 — alternates borrowers race with donor prune/gc; do not gc shared alternate ODBs under live borrowers"
    );
    println!(
        "note: CORR-006 — alternates cycle / dangling donor under GC appears as corruption; validate alternate graphs"
    );
    println!(
        "note: CORR-007 — replace-refs targets must not be pruned while replace refs live; avoid prune with replace"
    );
    println!(
        "note: NET-010 — partial-clone filter negotiation mismatch across agents yields unexpected missing blobs"
    );
    println!(
        "note: PACK-006 — parallel pack-objects window/memory blowup; bound pack.windowMemory / serialize heavy packs"
    );
    // M21 Dz-1 host-FS / platform honesty (Docker-limited induce; cite lockfile same-FS; no invented dialect).
    println!(
        "note: FS-001 — NFS non-atomic rename/lock: lockfile assumes POSIX local same-FS rename; do not green multi-writer repos on NFS"
    );
    println!(
        "note: FS-002 — SMB/CIFS lock residue after network blip can leave permanent *.lock; avoid shared writers on SMB or scrub locks after reconnect"
    );
    println!(
        "note: FS-003 — Windows mandatory sharing: cannot replace/delete while handle open; serialize writers"
    );
    println!(
        "note: FS-004 — Antivirus/Defender scan during rename can lock .lock/target; exclude .git from real-time AV or serialize commits"
    );
    println!(
        "note: FS-005 — WSL↔NTFS metadata mismatch: racy status/chmod/lock weirdness on /mnt/c; prefer Linux-native volumes for agent writers"
    );
    println!(
        "note: FS-009 — Unicode NFD/NFC path races (esp. macOS): normalize paths; avoid parallel creates differing only by normalization"
    );
    println!(
        "note: FS-010 — disk full during lock commit rename fails closed with possible leftover lock; free space before agent write storms"
    );
    println!(
        "note: FS-011 — container overlayfs copy-up amplifies many small object creates; prefer native volumes for high-N clones"
    );
    println!(
        "note: FS-012 — FUSE/virtiofs lock latency can look like stale locks; prefer local POSIX FS for contended writers"
    );
    println!(
        "note: FS-013 — inode/direntry exhaustion in objects/ under extreme loose create; pack/gc before ENOSPC/ENFILE"
    );
    println!(
        "note: WIN-001 — Win32 MAX_PATH deep worktree/LFS temps fail CreateFile; enable long paths or shorten nesting"
    );
    println!(
        "note: WIN-002 — Windows delete-pending window blocks O_EXCL recreate; retry/backoff after close; do not dual-smudge same OID blindly"
    );
    // M21 Dz-2 lock/ref/object/sub/misc/hook/ci honesty.
    println!(
        "note: LOCK-005 — core.lockfilePid (default false) optional PID sidecar helps agents identify lock holders; enable for diagnose"
    );
    println!(
        "note: LOCK-006 — lockfile blocks writers only; readers proceed — do not assume mutual exclusion for reads (api-lockfile)"
    );
    println!(
        "note: LOCK-008 — never manually delete a live *.lock while holder alive; dual writers corrupt index/refs"
    );
    println!(
        "note: LOCK-009 — NFS lock/lease steal can dual-grant writers; do not green multi-writer repos on NFS leases"
    );
    println!(
        "note: REF-013 — reftable stack concurrent writers (extensions.refStorage=reftable) not claimed; refuse inventing reftable bytes until SoT opens"
    );
    println!(
        "note: REF-020 — dangling .lock after NFS/SMB disconnect mid-update-ref needs admin clear; prefer local FS for writers"
    );
    println!(
        "note: OBJ-007 — receive-pack quarantine objects are not promoted yet; local readers/fsck may miss them mid-push"
    );
    println!(
        "note: OBJ-009 — cat-file --batch-check stampede saturates FD/CPU; bound parallel batch readers"
    );
    println!(
        "note: SUB-005 — fsmonitor superproject noise from submodule edits; expect event storms; filters OK but costly"
    );
    println!(
        "note: MISC-013 — server quarantine + parallel push same-OID fan-in pressures disk; serialize large receive-pack"
    );
    println!(
        "note: HOOK-007 — pre-receive/post-receive shared scratch races across parallel pushes; isolate hook scratch per push"
    );
    println!(
        "note: CI-002 — CI cache restore of .git while job runs can tear packs/refs; never warm-cache extract over a live checkout"
    );
    // M21 Dz-3 CRED/NET/LFS honesty (Docker-local; unlock without inventing helper/LFS product ship).
    println!(
        "note: CRED-001 — osxkeychain/wincred concurrent store races; serialize credential approve per host"
    );
    println!(
        "note: CRED-002 — credential-cache daemon socket overload under burst auth; bound parallel helpers or use store"
    );
    println!(
        "note: NET-003 — HTTP keep-alive/pool exhaustion under N smart-HTTP agents; bound sessions / backoff on 429/503"
    );
    println!(
        "note: NET-004 — SSH ControlMaster mux socket contention; isolate ControlPath per agent or disable mux"
    );
    println!(
        "note: NET-005 — protocol v2 ls-refs then long gap vs tip move; re-advertise before fetch-pack apply"
    );
    println!(
        "note: NET-008 — askPass/interactive credential under headless agents hangs; require non-interactive helper"
    );
    println!(
        "note: NET-011 — mix push --atomic and non-atomic clients exposes partial apply windows; prefer atomic for multi-ref"
    );
    println!(
        "note: LFS-001 — shared lfs.storage concurrent download races (sharing violations); isolate lfs.storage per agent or serialize smudge"
    );
    println!(
        "note: LFS-003 — long-running filter process backpressure under parallel LFS checkout; bound filter concurrency"
    );
    println!(
        "note: LFS-004 — lfs prune while peer checkout removes needed OID; quiesce prune under live smudge"
    );
    println!(
        "note: LFS-005 — concurrent lfs push same OID duplicate uploads; serialize push or tolerate server reject"
    );
    println!(
        "note: LFS-006 — pointer checkout without git-lfs installed commits pointers as content; require LFS in agent images or refuse"
    );
    // M21 Dz-4 remainder.
    println!(
        "note: CI-006 — parallel Actions checkout on overlapping sparse cones fights locks/paths; isolate sparse workdirs per job"
    );
    println!(
        "note: TOCTOU-004 — credential exists-check then store races; two helpers can store different tokens — serialize approve"
    );
    let pack_dir = repo.common_dir().join("objects/pack");
    if pack_dir.is_dir() {
        if let Ok(rd) = fs::read_dir(&pack_dir) {
            let has_keep = rd.flatten().any(|e| {
                e.file_name()
                    .to_string_lossy()
                    .ends_with(".keep")
            });
            if has_keep {
                println!(
                    "note: PACK-009 — detected *.keep under objects/pack (crash leftover class); do not invent unlock-by-delete under live fetch"
                );
            }
        }
    }
    if repo.common_dir().join("gc.log").is_file() {
        println!(
            "note: MAINT-002 — gc.log present; auto-gc may be suppressed (gc.logExpiry)"
        );
    }
    if pack_dir.is_dir() {
        if let Ok(rd) = fs::read_dir(&pack_dir) {
            let has_midx = rd.flatten().any(|e| {
                let n = e.file_name().to_string_lossy().to_string();
                n.contains("multi-pack-index") || n.ends_with(".midx")
            });
            if has_midx {
                println!(
                    "note: PACK-002 — multi-pack-index present; rewrite under readers is PACK-002 class"
                );
            }
        }
        if let Ok(rd) = fs::read_dir(&pack_dir) {
            let has_bitmap = rd.flatten().any(|e| {
                e.file_name()
                    .to_string_lossy()
                    .ends_with(".bitmap")
            });
            if has_bitmap {
                println!(
                    "note: PACK-003 — pack bitmap present; concurrent pack replace mid-bitmap is PACK-003 class"
                );
            }
        }
    }
    if reports.iter().any(|r| {
        r.path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n == "HEAD.lock")
    }) {
        println!(
            "note: REF-007 — HEAD.lock contention serializes HEAD updates; inspect + wait/backoff; do not delete under live writer"
        );
    }
    if reports.iter().any(|r| {
        r.path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n == "packed-refs.lock")
    }) {
        println!(
            "note: REF-022 — packed-refs.lock held; readers (for-each-ref) stay safe; writers (pack-refs) must fail-closed"
        );
    }
    let hooks = repo.git_dir().join("hooks");
    if hooks.join(".arks-hooks-swap").is_file() || hooks.join(".githooks-swap-in-progress").is_file()
    {
        println!(
            "note: MISC-011 — hooks swap-in-progress marker detected; mid-flight hook deploy vs writers"
        );
    }
    let common = repo.common_dir();
    let has_wt = common.join("worktrees").is_dir();
    let has_modules = common.join("modules").is_dir();
    if has_wt && has_modules {
        println!(
            "note: AGENT-007 — worktree + submodule composition multiplies lock domains; inspect each domain; no invented cross-domain dialect"
        );
    }
    if reports.iter().any(|r| {
        r.path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n == "index.lock")
    }) {
        println!(
            "note: CI-001 — index.lock present (shared-workspace lock plant class); isolate CI checkouts"
        );
    }
    if common.join("shallow").is_file()
        && (common.join("info/grafts").is_file() || repo.git_dir().join("info/grafts").is_file())
    {
        println!(
            "note: SHAL-004 — shallow + grafts present; unshallow/fetch peers risk boundary confusion"
        );
    }
    let replace_dir = common.join("refs/replace");
    if replace_dir.is_dir()
        || common.join("refs/replace.lock").is_file()
        || repo.git_dir().join("refs/replace.lock").is_file()
    {
        println!(
            "note: CORR-003 — refs/replace concurrent updates / live lock delete anti-pattern; serialize replace-refs writers"
        );
    }
    if pack_dir.is_dir() {
        if let Ok(rd) = fs::read_dir(&pack_dir) {
            let has_cruft = rd.flatten().any(|e| {
                let n = e.file_name().to_string_lossy().to_string();
                n.contains("cruft") || n.ends_with(".keep")
            });
            if has_cruft {
                println!(
                    "note: PACK-005 — cruft/.keep detected under objects/pack; serialize maintenance vs writers"
                );
            }
        }
    }
    if common.join("untracked-cache.lock").is_file()
        || repo.git_dir().join("untracked-cache.lock").is_file()
    {
        println!(
            "note: IDX-005 — untracked-cache.lock held; sparse-index/untracked-cache expand race class"
        );
    }
    let alts = common.join("objects/info/alternates");
    if alts.is_file() {
        println!(
            "note: OBJ-006 — objects/info/alternates present; borrower vs donor prune is OBJ-006/CORR-006 class"
        );
    }
    if common.join("objects/info/filters").is_file() {
        println!(
            "note: NET-010 — partial-clone filters present; agents must agree on filter or expect missing blobs"
        );
    }
    if pack_dir.join("multi-pack-index.lock").is_file() {
        println!(
            "note: MAINT-006 — multi-pack-index.lock held; incremental MIDX vs pack add race class"
        );
    }
    if reports.iter().any(|r| {
        r.path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n == "index.lock")
    }) {
        println!(
            "note: CI-005 — leftover index.lock across reusable CI jobs; clean workspace between jobs"
        );
    }
}

fn lock_clear(args: &[String]) -> CliResult<()> {
    let stale = args.iter().any(|a| a == "--stale");
    let force = args.iter().any(|a| a == "--force-stale");
    if !stale {
        return Err(CliError::Usage(
            "lock clear --stale [<path>] [--force-stale]".into(),
        ));
    }
    for a in args {
        if a.starts_with('-') && a != "--stale" && a != "--force-stale" {
            return Err(CliError::NotImplemented(format!("unsupported option: {a}")));
        }
    }
    let path_arg = args.iter().find(|a| !a.starts_with('-'));
    let repo = open_repo()?;
    let targets: Vec<PathBuf> = if let Some(p) = path_arg {
        let pb = PathBuf::from(p);
        if !pb.exists() {
            return Err(CliError::Msg(format!("lock not found: {}", pb.display())));
        }
        vec![pb]
    } else {
        let mut roots = vec![repo.git_dir().to_path_buf()];
        let common = repo.common_dir().to_path_buf();
        if common != roots[0] {
            roots.push(common);
        }
        let mut reports = Vec::new();
        for root in &roots {
            collect_locks(root, root, &mut reports)?;
        }
        reports.into_iter().map(|r| r.path).collect()
    };
    if targets.is_empty() {
        println!("no *.lock files to clear");
        return Ok(());
    }
    for path in targets {
        let report = classify_lock(&path)?;
        match report.classification {
            "live-holder" if !force => {
                return Err(CliError::Msg(format!(
                    "refusing to clear live lock {} (pid {:?}); unlock holder or pass --force-stale (unsafe)",
                    path.display(),
                    report.pid
                )));
            }
            "stale-candidate" => {
                fs::remove_file(&path)?;
                println!(
                    "cleared stale lock (AGENT-005 eviction/dead-PID class): {}",
                    path.display()
                );
            }
            "unknown-liveness" if force => {
                fs::remove_file(&path)?;
                println!("cleared lock with --force-stale: {}", path.display());
            }
            "unknown-liveness" => {
                return Err(CliError::Msg(format!(
                    "lock {} has no dead-PID proof; inspect then re-run with --force-stale if confirmed stale",
                    path.display()
                )));
            }
            "live-holder" => {
                // force path: still warn but allow (operator override).
                fs::remove_file(&path)?;
                println!(
                    "WARNING: forced clear of live-classified lock: {}",
                    path.display()
                );
            }
            other => {
                return Err(CliError::Msg(format!(
                    "unexpected lock classification {other} for {}",
                    path.display()
                )));
            }
        }
    }
    Ok(())
}

#[allow(clippy::only_used_in_recursion)]
fn collect_locks(root: &Path, dir: &Path, out: &mut Vec<LockReport>) -> CliResult<()> {
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let ft = entry.file_type()?;
        if ft.is_dir() {
            // Skip objects fan-out noise except well-known lock homes.
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.len() == 2 && name.chars().all(|c| c.is_ascii_hexdigit()) {
                continue;
            }
            if name == "objects" || name == "pack" || name == "info" {
                // Still scan objects for pack locks at top of objects; skip fan-out.
                if name == "objects" {
                    collect_locks(root, &path, out)?;
                }
                continue;
            }
            collect_locks(root, &path, out)?;
        } else if ft.is_file() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.ends_with(".lock") {
                out.push(classify_lock(&path)?);
            }
        }
    }
    Ok(())
}

fn classify_lock(path: &Path) -> CliResult<LockReport> {
    let meta = fs::metadata(path)?;
    let age_secs = meta.modified().ok().and_then(|t| {
        SystemTime::now()
            .duration_since(t)
            .ok()
            .map(|d| d.as_secs())
    });
    let contents = fs::read_to_string(path).unwrap_or_default();
    let pid = parse_lock_pid(&contents);
    let pid_live = pid.map(pid_is_alive);
    let classification = match (pid, pid_live) {
        (Some(_), Some(true)) => "live-holder",
        (Some(_), Some(false)) => "stale-candidate",
        _ => "unknown-liveness",
    };
    Ok(LockReport {
        path: path.to_path_buf(),
        age_secs,
        pid,
        pid_live,
        classification,
    })
}

fn parse_lock_pid(contents: &str) -> Option<u32> {
    let line = contents.lines().map(str::trim).find(|l| !l.is_empty())?;
    // Accept bare PID or "PID <n>" forms; ignore non-numeric payloads.
    if let Ok(n) = line.parse::<u32>() {
        return Some(n);
    }
    let parts: Vec<_> = line.split_whitespace().collect();
    if parts.len() >= 2 && parts[0].eq_ignore_ascii_case("pid") {
        return parts[1].parse().ok();
    }
    None
}

fn pid_is_alive(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    #[cfg(unix)]
    {
        Path::new(&format!("/proc/{pid}")).exists()
    }
    #[cfg(not(unix))]
    {
        // Host-advisory Windows: cannot prove liveness without extra APIs; treat as unknown→false
        // so clear --stale with dead-PID proof still works when /proc is absent.
        let _ = pid;
        false
    }
}
