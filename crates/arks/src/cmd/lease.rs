//! `arks lease` — durable product lease porcelain (M13 Cd-1 / G18).
//!
//! Cite: `.spec/sot/arks-lease.sot.md`, `.spec/sot/lease-concurrency-intent.sot.md`
//! ADR: `.spec/adr/20261109032955-m13-cd1-lease-grant-resource-schema.adr.md`
//!
//! Honesty: lease ≠ skip Git locks. L3 HighConcurrency = opt-in claimed shape (M23 Claimed).
//! AGENT-003 (differentiate): AlreadyHeld teaches wait/backoff — do not stampede locks.

use crate::error::{CliError, CliResult};
use crate::util::open_repo;
use arks_lease::{DurableLeaseStore, LeaseError, LeaseRecord};
use std::time::Duration;

const DEFAULT_TTL_SECS: u64 = 60;

pub fn cmd_lease(args: &[String]) -> CliResult<()> {
    let sub = args
        .first()
        .map(String::as_str)
        .ok_or_else(|| CliError::Usage("lease <grant|release|inspect|list|expire>".into()))?;
    let rest = &args[1..];
    match sub {
        "grant" => lease_grant(rest),
        "release" => lease_release(rest),
        "inspect" => lease_inspect(rest),
        "list" => lease_list(rest),
        "expire" => lease_expire(rest),
        other => Err(CliError::NotImplemented(format!(
            "lease {other} not claimed; see M13 Cd-1 (grant/release/inspect/list/expire)"
        ))),
    }
}

fn open_store() -> CliResult<DurableLeaseStore> {
    let repo = open_repo()?;
    DurableLeaseStore::open(repo.git_dir()).map_err(lease_err)
}

fn lease_err(e: LeaseError) -> CliError {
    match e {
        LeaseError::AlreadyHeld {
            resource_id,
            holder,
        } => CliError::Msg(format!(
            "lease already held by {holder} on {resource_id}; AGENT-003: wait/backoff with jitter — do not stampede Git locks (M13 Cd-1 lease coordination; lease ≠ skip locks)"
        )),
        LeaseError::HighConcurrencyRefused => CliError::Msg(e.to_string()),
        other => CliError::Msg(other.to_string()),
    }
}

fn take_flag(args: &[String], name: &str) -> bool {
    args.iter().any(|a| a == name)
}

fn take_opt<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    let mut i = 0;
    while i < args.len() {
        if args[i] == name {
            return args.get(i + 1).map(String::as_str);
        }
        if let Some(rest) = args[i].strip_prefix(&format!("{name}=")) {
            return Some(rest);
        }
        i += 1;
    }
    None
}

fn positionals(args: &[String]) -> Vec<&str> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        if a == "--holder" || a == "--ttl-secs" {
            i += 2;
            continue;
        }
        if a.starts_with("--holder=") || a.starts_with("--ttl-secs=") {
            i += 1;
            continue;
        }
        if a.starts_with('-') {
            // Unknown flags handled by callers.
            out.push(a.as_str());
            i += 1;
            continue;
        }
        out.push(a.as_str());
        i += 1;
    }
    out
}

fn print_record(rec: &LeaseRecord) {
    println!(
        "{}\tholder={}\texpires_at_ms={}",
        rec.resource_id, rec.holder, rec.expires_at_ms
    );
}

fn lease_grant(args: &[String]) -> CliResult<()> {
    for a in args {
        if a.starts_with('-') && !a.starts_with("--holder") && !a.starts_with("--ttl-secs") {
            return Err(CliError::NotImplemented(format!("unsupported option: {a}")));
        }
    }
    let pos = positionals(args);
    let resource = pos.first().copied().ok_or_else(|| {
        CliError::Usage("lease grant <resource_id> --holder <name> [--ttl-secs N]".into())
    })?;
    let holder = take_opt(args, "--holder").ok_or_else(|| {
        CliError::Usage("lease grant <resource_id> --holder <name> [--ttl-secs N]".into())
    })?;
    let ttl_secs: u64 = take_opt(args, "--ttl-secs")
        .map(|s| {
            s.parse::<u64>()
                .map_err(|_| CliError::Usage(format!("invalid --ttl-secs: {s}")))
        })
        .transpose()?
        .unwrap_or(DEFAULT_TTL_SECS);
    let mut store = open_store()?;
    store.manager_mut().expire_due();
    let rec = store
        .manager_mut()
        .grant(resource, holder, Duration::from_secs(ttl_secs))
        .map_err(lease_err)?;
    store.save().map_err(lease_err)?;
    println!("granted");
    print_record(&rec);
    println!("note: lease ≠ skip Git locks — still acquire ref/index *.lock on Git-compat writes");
    Ok(())
}

fn lease_release(args: &[String]) -> CliResult<()> {
    for a in args {
        if a.starts_with('-') && !a.starts_with("--holder") {
            return Err(CliError::NotImplemented(format!("unsupported option: {a}")));
        }
    }
    let pos = positionals(args);
    let resource = pos
        .first()
        .copied()
        .ok_or_else(|| CliError::Usage("lease release <resource_id> --holder <name>".into()))?;
    let holder = take_opt(args, "--holder")
        .ok_or_else(|| CliError::Usage("lease release <resource_id> --holder <name>".into()))?;
    let mut store = open_store()?;
    store
        .manager_mut()
        .release(resource, holder)
        .map_err(lease_err)?;
    store.save().map_err(lease_err)?;
    println!("released {resource} (holder={holder})");
    Ok(())
}

fn lease_inspect(args: &[String]) -> CliResult<()> {
    if args.iter().any(|a| a.starts_with('-')) {
        return Err(CliError::NotImplemented(
            "lease inspect: no options claimed".into(),
        ));
    }
    let mut store = open_store()?;
    store.manager_mut().expire_due();
    store.save().map_err(lease_err)?;
    if let Some(id) = args.first() {
        match store.manager().inspect(id) {
            Some(rec) => print_record(rec),
            None => println!("no lease for {id}"),
        }
    } else {
        let list = store.manager().list();
        if list.is_empty() {
            println!("no active leases");
        } else {
            for rec in list {
                print_record(rec);
            }
        }
    }
    println!(
        "note: AGENT-003 — on AlreadyHeld, wait/backoff with jitter; blind herds amplify lock storms (M13 lease coordination)"
    );
    println!(
        "note: LOCK-002 — TTL fencing: `arks lease expire` reclaims due leases before grant (lease ≠ skip Git locks)"
    );
    println!(
        "note: WT-001/SUB-001/AGENT-007 — use domain:worktree:<id> / domain:submodule:<path> for cross-domain leases; composition does not invent lock dialects"
    );
    println!(
        "note: HOOK-001/HOOK-006 — hook:critical budgets coordinate sync critical sections; no invented async hook runners"
    );
    println!(
        "note: PACK-004/MAINT-003 — maint:gc leases coordinate writers vs gc/repack; gc --prune=now remains CORR-001 refuse"
    );
    // M20 Cd-4 differentiate honesty (bucket B / G18).
    println!(
        "note: AGENT-001 — shared checkout as blackboard needs exclusive agent:blackboard lease; do not use one WT as message bus"
    );
    println!(
        "note: AGENT-010 — tooling must classify AlreadyHeld / lock fatals (wait/backoff) vs true errors; do not blind-retry all nonzeros"
    );
    println!(
        "note: AGENT-011 — parallel formatters need index:paths:formatter (or path-set) lease; lease ≠ skip index.lock"
    );
    println!(
        "note: REF-002 — packed-refs global lock storms: coordinate with refns:packed-refs lease before pack-refs writers"
    );
    println!(
        "note: REF-004 — multi-ref transactions: hold refns:txn:multi before prepare; abort-all honesty retained"
    );
    println!(
        "note: REF-008 — symref vs tip races: hold refns:HEAD (or branch tip refns) before retarget/commit"
    );
    println!(
        "note: REF-010 — concurrent identical new-ref creates: exclusive refns:refs/heads/<name> before create"
    );
    println!(
        "note: IDX-003 — prefer private GIT_INDEX_FILE + index:private:<agent> lease; do not share default index across agents"
    );
    println!(
        "note: IDX-007 — parallel same-path add: exclusive index:paths:same (or path) lease; still honor index.lock"
    );
    println!(
        "note: SEQ-005 — dual conflict resolution: exclusive seq:conflict:<path> lease; do not dual-edit conflict markers"
    );
    println!(
        "note: WT-003 — worktree prune vs live agent: exclusive domain:worktree:prune before prune; lease ≠ skip locks"
    );
    println!(
        "note: MAINT-001 — background maintenance vs agents: exclusive maint:background; foreground still honors Git locks"
    );
    // M20 Cd-5 differentiate honesty (bucket B / G18).
    println!(
        "note: AGENT-008 — sync policy-hook latency: exclusive hook:policy lease before policy pre-commit; no invented async runners"
    );
    println!(
        "note: AGENT-009 — bare-as-only-channel: partition with refns:refs/heads/agents/ plus bare coord lease; do not stampede bare tip locks"
    );
    println!(
        "note: AGENT-012 — secret-scan hook × N: exclusive hook:secret-scan lease; lease ≠ skip Git locks"
    );
    println!(
        "note: REF-005 — multi-ref reader subset: hold refns:txn:snapshot before multi-update finish; reader honesty not invented snapshot bytes"
    );
    println!(
        "note: REF-014 — remote-tracking fan-in: exclusive refns:refs/remotes/ (or packed-refs) lease before parallel fetch writers"
    );
    println!(
        "note: REF-015 — multi-push same tip: exclusive refns:receive-pack (or tip refns) before push; CAS/lock still honored"
    );
    println!(
        "note: IDX-009 — long hook holds index: compose hook:critical + index:paths:default; still honor index.lock"
    );
    println!(
        "note: IDX-012 — per-worktree index vs shared tip: index:paths:worktree lease; tip CAS/refns still separate"
    );
    println!(
        "note: WT-007 — shared hooks across worktrees: exclusive domain:worktree:hooks before shared pre-commit storm"
    );
    println!(
        "note: WT-008 — concurrent worktree add name collision: exclusive domain:worktree:add before add; lease ≠ skip admin locks"
    );
    println!(
        "note: CFG-001 — concurrent config rewrite: exclusive config:local lease; do not invent config.lock dialect; lease ≠ skip Git locks"
    );
    println!(
        "note: TOCTOU-001 — status-then-commit race: lease-before-lock (toctou:status-commit); do not invent porcelain snapshot transactions"
    );
    // M20 Cd-6 differentiate honesty (bucket B / G18).
    println!(
        "note: HOOK-004 — server update/pre-receive serialization: exclusive hook:update lease; no invented async runners"
    );
    println!(
        "note: CFG-003 — concurrent global/user config: exclusive config:global lease; do not invent config.lock dialect"
    );
    println!(
        "note: REF-019 — worktree-private vs common refs: exclusive refns:common before shared tip moves"
    );
    println!(
        "note: REF-023 — branch rename under writers: exclusive refns:rename before branch -m; tip locks still honored"
    );
    println!(
        "note: OBJ-002 — loose object write herd: exclusive obj:loose:write lease before fan-out storms"
    );
    println!(
        "note: OBJ-003 — object write before reachable ref vs prune: exclusive obj:write-before-ref; maint:gc honesty retained"
    );
    println!(
        "note: SUB-002 — concurrent submodule update same path: exclusive domain:submodule:update lease"
    );
    println!(
        "note: SUB-004 — recursive submodule status storm: exclusive domain:submodule:status lease"
    );
    println!(
        "note: SUB-007 — parallel clone into .git/modules: exclusive domain:submodule:modules lease"
    );
    println!(
        "note: TOCTOU-002 — tip read then update without CAS: lease-before-lock (toctou:tip-cas); prefer expected OID"
    );
    println!(
        "note: TOCTOU-006 — fetch then rebase without onto CAS: exclusive toctou:fetch-rebase lease"
    );
    println!(
        "note: MISC-004 — parallel clean -fdx vs writers: exclusive misc:clean lease; lease ≠ skip Git locks"
    );
    // M20 Cd-7 differentiate honesty (bucket B / G18).
    println!(
        "note: MISC-010 — shared author/committer env races: exclusive misc:identity-env lease before identity-sensitive writes"
    );
    println!(
        "note: MISC-015 — parallel worktree add from bare: exclusive domain:worktree:add-from-bare lease"
    );
    println!(
        "note: MISC-016 — huge update-ref --stdin batches: exclusive refns:txn:stdin lease; tip locks still honored"
    );
    println!("note: SUB-010 — deinit vs build in path: exclusive domain:submodule:deinit lease");
    println!(
        "note: SHAL-002 — partial-clone promisor queue pileup: exclusive shallow:promisor lease; no absolute PERF SLO"
    );
    println!("note: PACK-007 — promisor demand-fetch races: exclusive pack:promisor:fetch lease");
    println!("note: CI-003 — shared fixture bare push CAS: exclusive ci:fixture-bare lease");
    println!(
        "note: CI-004 — bot temp-branch namespace: exclusive refns:refs/heads/bot/ partition lease"
    );
    println!(
        "note: NET-001 — parallel fetch same remote: exclusive net:fetch:origin lease; packed-refs still honored"
    );
    println!(
        "note: NET-002 — parallel push same branch: exclusive net:push:branch lease; tip locks still honored"
    );
    println!(
        "note: NET-007 — credential helper stampede: exclusive net:credential lease honesty; no invented helper protocol"
    );
    println!(
        "note: LFS-002 — LFS content lock ≠ Git index.lock: exclusive lfs:path-lock honesty; do not confuse lock dialects"
    );
    // M20 Cd-8 differentiate honesty (bucket B leftovers / G18). Absolute PERF SLOs remain M14/G19.
    println!(
        "note: PERF-001 — single-writer lock ceiling herds: exclusive perf:lock:serialize lease; no absolute ms SLO"
    );
    println!(
        "note: PERF-002 — status walk storms: exclusive perf:status:walk lease; no absolute ms SLO"
    );
    println!(
        "note: PERF-003 — fsmonitor single-WD backlog: exclusive perf:fsmonitor lease; no invented daemon"
    );
    println!("note: PERF-004 — commit-graph rewrite vs walks: exclusive perf:commit-graph lease");
    println!("note: PERF-005 — loose object explosion lag: exclusive perf:loose:burst lease");
    println!(
        "note: PERF-006 — process-spawn herd: exclusive perf:process:spawn lease; no absolute ms SLO"
    );
    println!("note: PERF-007 — pack mmap thrash readers: exclusive perf:pack:mmap lease");
    println!("note: PERF-008 — parallel blame: exclusive perf:blame lease; no absolute ms SLO");
    println!("note: PERF-009 — parallel grep: exclusive perf:grep lease; no absolute ms SLO");
    println!(
        "note: PERF-010 — parallel diff --stat herd: exclusive perf:diff:stat lease; no absolute ms SLO"
    );
    println!(
        "note: CFG-002 — includeIf TOCTOU: exclusive config:includeIf lease; do not invent config.lock dialect"
    );
    println!(
        "note: AGENT-004 — cross-host NFS: agent:nfs:refuse honesty — do not green multi-host shared WT on NFS"
    );
    println!(
        "note: lease ≠ skip Git locks; L3 HighConcurrency = opt-in claimed shape (shared-index N-way OUT)"
    );
    Ok(())
}

fn lease_list(args: &[String]) -> CliResult<()> {
    if take_flag(args, "--help") || args.iter().any(|a| a.starts_with('-')) {
        if args.iter().any(|a| a.starts_with('-') && a != "--help") {
            return Err(CliError::NotImplemented(
                "lease list: no options claimed".into(),
            ));
        }
    }
    lease_inspect(&[])
}

fn lease_expire(args: &[String]) -> CliResult<()> {
    if args.iter().any(|a| a.starts_with('-')) {
        return Err(CliError::NotImplemented(
            "lease expire: no options claimed".into(),
        ));
    }
    let mut store = open_store()?;
    let expired = store.manager_mut().expire_due();
    store.save().map_err(lease_err)?;
    if expired.is_empty() {
        println!("no expired leases");
    } else {
        println!("expired {}", expired.len());
        for rec in &expired {
            print_record(rec);
        }
    }
    Ok(())
}
