---
id: adr:root/20261109032955
title: M13 Cd-1 lease grant resource schema
status: active
created: 2026-11-09T03:29:55Z
updated: 2026-11-09T04:00:00Z
tags: [m13, cd-1, cd-2, cd-3, lease, grant-schema, g18, differentiate-beyond-git, fencing, worktree, submodule, hooks, maint]
rel:
  depends_on:
    - sot:root/lease-concurrency-intent
    - sot:root/arks-lease
    - sot:root/program-roadmap
    - sot:root/interop-test-matrix
    - plan:root/20261109024635
    - adr:root/20261108232800
    - research:root/20261109023117
  evidence: [evidence:root/20261109153000, evidence:root/20261109160000, evidence:root/20261109163000]
---

# M13 Cd-1 lease grant resource schema

## Context

Milestone **M13** (Phase **Cd**) absorbs claimed **`differentiate-beyond-git`** catalog rows via **lease grant shapes** after M10 L1→L2 and M12 G17 compat mitigations ([[plan:root/20261109024635]], [[sot:root/program-roadmap]] v0.5.0). Gate **G18** requires stock-`git` pathology vs `arks` resolved compare for each claimed ID.

[[sot:root/lease-concurrency-intent]] already states scoped grants (ref namespace, path set, explicit id) and TTL fencing as **intent**, with exact schema phased. Wave **Cd-1** needs a **minimal, product-private** resource-id convention so CLI/harness oracles are stable — **without** inventing Git lock dialects, skipping `*.lock` / `index.lock` on Git-compat paths, shipping L3 N-way ([[adr:root/20261108232800]]), or pulling CELIR.

## Decision

1. **Resource id = opaque string with optional typed prefixes (Cd-1 / Cd-2).** Durable L1/L2 tables continue to store opaque `resource_id` strings. Claimed waves **recommend** (and G18 oracles use) these prefixes:
   - `refns:<git-ref-prefix>` — exclusive coordination over a ref namespace prefix (e.g. `refns:refs/heads/main`, `refns:refs/heads/agents/a/`). Does **not** replace per-ref `*.lock` bytes.
   - `index:<scope>` — exclusive coordination for index-writer herds (e.g. `index:default`). Does **not** replace `index.lock`.
   - `domain:worktree:<id>` — exclusive coordination for a linked-worktree / common-dir lock domain (Cd-2). Does **not** invent a worktree lock dialect.
   - `domain:submodule:<path>` — exclusive coordination for a submodule git-dir lock domain (Cd-2). Does **not** invent a submodule lock dialect.
   - `hook:<scope>` — exclusive budget for sync hook-critical sections (Cd-3; e.g. `hook:critical`). Does **not** invent async hook runners.
   - `maint:<scope>` — exclusive writer–gc / maintenance coordination (Cd-3; e.g. `maint:gc`). Does **not** enable concurrent prune under writers.
   - Unprefixed / other opaque ids remain valid for L1 compatibility and tests.
2. **TTL fencing stays absolute expiry (LOCK-002 differentiate).** Existing `expires_at_ms` fencing is the reclaim mechanism for dead holders; `arks lease expire` drops due leases before grant. Cd-2 proves fencing uplift vs M12 lock-file-only stale clear. No new on-disk wire format beyond current JSON lease table under `.git/arks/leases/`.
3. **CLI surface `arks lease`.** Porcelain exposes durable grant / release / inspect / list / expire against `$GIT_DIR`. Holding a lease **pre-coordinates** agents; writers MUST still use Git lock APIs on Git-compat paths (**lease ≠ skip Git locks**).
4. **AlreadyHeld = coordinated refuse (AGENT-003 differentiate).** Second holders MUST fail closed at the lease layer with wait/backoff honesty — not stampede Git locks. This is the M13 uplift beyond M12 docs-only AGENT-003 mitigation.
5. **Cross-domain composition (AGENT-007).** Distinct `domain:worktree:*` and `domain:submodule:*` leases MAY be held concurrently; the same resource id remains exclusive. Inspect MUST list both. Lease composition does **not** skip either domain’s Git locks.
6. **L3 remains refuse-closed.** No HighConcurrency pipeline; no CELIR; no invented lock-file dialect.

## Consequences

- [[plan:root/20261109024635]] Wave **Cd-1** claim-set oracles cite `refns:` / `index:` ids and G18 harness.
- `crates/arks-lease` MAY add prefix helpers; on-disk schema version unchanged for Cd-1.
- `arks` depends on `arks-lease` for the lease porcelain; dual-path honesty messages stay visible.
- Future Cd waves MAY extend prefixes (path-set, worktree domains) via ADR amendment — not silent invention.

## Supersedes / amends

- Closes the “exact grant schema” open for **Cd-1 claimed prefixes only**; does not close L3 or allocator placement.
- Does **not** amend M9/M10/M11 exits or M12 G17 mitigate receipts.
