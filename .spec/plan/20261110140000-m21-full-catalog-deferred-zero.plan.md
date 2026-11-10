---
id: plan:root/20261110140000
title: M21 full catalog deferred zero
status: archived
created: 2026-11-10T14:00:00Z
updated: 2026-11-10T14:40:00Z
tags: [milestone, m21, phase-dz, concurrency, deferred-zero, g17, g18, catalog-closure, done]
rel:
  depends_on:
    - sot:root/program-roadmap
    - sot:root/multi-agent-git-concurrency-scenarios
    - sot:root/production-residual-register
    - sot:root/lease-concurrency-intent
    - sot:root/interop-test-matrix
    - sot:root/product-verification-clean-room
    - sot:root/cli-surface
    - research:root/20261109023117
    - research:root/20261110021546
    - plan:root/20261110130000
    - adr:root/20261108232800
  implements: [sot:root/program-roadmap, sot:root/multi-agent-git-concurrency-scenarios]
---

# M21 — Full catalog Deferred=0 (Phase Dz)

## Goal (binding Done bar)

Unattended full Git-contention catalog closure for arks.

**SUCCESS** = every catalog ID has exactly one **terminal** disposition **AND** Deferred count is **0**:

1. **Claimed+Docker clean-room closed** (G17/G18), OR
2. **Refuse-closed** with ADR+SoT (**L3-class only**), OR
3. **N/A-out-of-scope** with primary-doc cite.

**Closed-deferred is not a terminal success disposition** for this program. Prior “keep deferred for humans / policy later / Docker-induce-thin / bucket B exhausted” stops are **work items**, not exits.

Forbidden premature stops: bucket B exhausted; Docker-induce-thin; policy/G8 later; keep-active-for-humans.

CELIR excluded. No force-push. No fake greens. Honesty form: `Deferred=0 Claimed=X Refuse=Y N/A=Z`.

## Baseline (post M20 Cd-8)

| Disposition | Count |
|-------------|------:|
| Claimed+clean-room-closed (G17∪G18 unique) | 174 |
| N/A-out-of-scope | 9 |
| Refuse-closed (catalog rows) | 0 (+ L3 product valve) |
| **Closed-deferred (work queue)** | **38** |
| Total | 221 |

## Work queue — migrate Closed-deferred → terminal

All **38** IDs below are the M21 work queue (formerly Closed-deferred). Process in waves of **≤12**. Each wave: unlock SoT/ADR/harness as needed → implement → Docker clean-room → evidence → commit → push → next wave. Do **not** UpdateGoal complete until Deferred=0.

### Wave Dz-1 — host-FS / platform harness (G17) — 12

`FS-001`, `FS-002`, `FS-003`, `FS-004`, `FS-005`, `FS-009`, `FS-010`, `FS-011`, `FS-012`, `FS-013`, `WIN-001`, `WIN-002`

Unlock: host-FS/platform honesty harness in Docker (cite lockfile same-FS / platform docs; no invented NFS dialect). Prefer Claim+G17 honesty over leaving Deferred.

### Wave Dz-2 — lock / ref / object / submodule / maint / CI local (G17) — 12

`LOCK-005`, `LOCK-006`, `LOCK-008`, `LOCK-009`, `REF-013`, `REF-020`, `OBJ-007`, `OBJ-009`, `SUB-005`, `MISC-013`, `HOOK-007`, `CI-002`

Unlock: reftable honesty / quarantine / lockfilePid / NFS-lock notes as needed. REF-013 may Claim as reftable-unclaimed honesty (crate: reftable later) without inventing reftable bytes.

### Wave Dz-3 — CRED / NET / LFS unlock (G17 or G18) — 12

`CRED-001`, `CRED-002`, `NET-003`, `NET-004`, `NET-005`, `NET-008`, `NET-011`, `LFS-001`, `LFS-003`, `LFS-004`, `LFS-005`, `LFS-006`

Unlock: G8/policy residual for live-forge **or** Docker-local smart-HTTP / lease honesty equivalent (same pattern as Cd-7 NET/LFS-002). LFS remains CLI exclusion for porcelain ship; catalog rows still require terminal disposition (Claim honesty or N/A cite [[sot:root/cli-surface]]). Prefer Claim honesty consistent with LFS-002 unless dedicated LFS SoT opens.

### Wave Dz-4 — remainder (G17) — 2

`CI-006`, `TOCTOU-004`

## Terminal disposition rules (normative)

| Disposition | Allowed when |
|-------------|--------------|
| **Claimed** | Explicit wave claim + Docker G17/G18 dual PASS receipt |
| **Refuse-closed** | **L3-class only** — ADR + lease SoT valve; not a dump for hard harness work |
| **N/A-out-of-scope** | Opportunity truly out-of-scope **and** primary-doc cite recorded |
| **Closed-deferred** | **Forbidden as M21 exit**; may appear only as in-progress queue during waves |

## In scope

1. Catalog / roadmap / residual-register Done-bar rewrite; this plan; research honesty sync.
2. Unlock G8/host-FS/CRED/NET/LFS harness seams in-repo.
3. Engine/CLI honesty notes + G17/G18 tests for each queued ID.
4. Docker clean-room evidence per wave; `spec check`; commit+push each wave.

## Out of scope

- CELIR product deps.
- Shipping L3 HighConcurrency (stays refuse-closed unless superseding ADR).
- Force-push; fake greens; claiming absolute PERF ms as issue-resolution Done (M14/G19-separated).
- Declaring Deferred=0 before every queued ID has a terminal disposition.

## Exit gates

| Gate | Requirement |
|------|-------------|
| Catalog | Deferred **0**; Claimed+N/A+Refuse = 221; bare Open 0 |
| Clean-room | Every newly Claimed ID has Docker G17/G18 PASS |
| Specfold | `spec check` clean |
| L3 | Refuse-closed valve unchanged unless ADR supersedes |

## Status

**Closed** — Waves Dz-1…Dz-4 Docker G17 dual PASS. Disposition honesty: **Deferred=0 Claimed=212 Refuse=0 N/A=9** (L3 product valve refuse-closed separately). Evidence: [[evidence:root/20261110141000]], [[evidence:root/20261110142000]], [[evidence:root/20261110143000]], [[evidence:root/20261110144000]].

## Cross-links

[[sot:root/multi-agent-git-concurrency-scenarios]], [[sot:root/program-roadmap]], [[sot:root/production-residual-register]], [[research:root/20261110021546]], [[adr:root/20261108232800]]
