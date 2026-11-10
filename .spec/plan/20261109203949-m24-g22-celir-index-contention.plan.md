---
id: plan:root/20261109203949
title: M24 G22 CELIR index contention clean-room
status: active
created: 2026-11-09T20:39:49Z
updated: 2026-11-09T20:45:00Z
tags: [m24, g22, celir, indexing-service, clean-room, contention, dual-path]
rel:
  depends_on:
    - sot:root/celir-index-contention-clean-room
    - sot:root/celir-index-dual-path
    - plan:root/20261109203430
    - sot:root/storage-celir-boundary
    - adr:root/20261110052030
    - sot:root/lease-concurrency-intent
    - sot:root/product-verification-clean-room
    - sot:root/interop-test-matrix
    - sot:root/program-roadmap
    - sot:root/production-residual-register
  implements: [sot:root/celir-index-contention-clean-room, sot:root/celir-index-dual-path]
  evidence: [evidence:root/20261109204428]
---

# M24 — G22 CELIR index contention clean-room

## Goal

Raise M24 dual-path honesty from **host advisory** to **Docker clean-room verified** for the **index-contention** gate: N-way concurrent agents (32 / 128) exercising CELIR index build/publish/open/query and rebuild-from-`.git` under `arks-lease` + filesystem `.git`, without ObjectStore via CELIR and without tearing publication.

Owning SoT: [[sot:root/celir-index-contention-clean-room]]. Unit companion: [[sot:root/celir-index-dual-path]]. Milestone: [[plan:root/20261109203430]].

## Done rules

| Disposition | Meaning |
|-------------|---------|
| **Verified (this gate)** | Docker compose `g22` PASS + evidence receipt; N≥32 and N=128 tiers green; stock-Git readable; fail-closed; no torn Present; ObjectStore Refuse held |
| **FAIL / blocked** | Missing discrete-circuit mount/packages, OOM/sole-runner conflict, or assertion failure — **no fake PASS** |
| **Out** | G11-L3 shared-index N-way; CELIR-backed Git verified; default-on feature |

## Claimed set

| ID | Behavior |
|----|----------|
| **G22-C01** | N=32 concurrent readonly open/query after publish — identical effects |
| **G22-C02** | N=128 concurrent readonly open/query after publish — identical effects |
| **G22-C03** | N=32 concurrent rebuild without index lease — fail-closed / no torn Present; leased rebuild restores integrity |
| **G22-C04** | N=32 lease shard Git writers + concurrent index readers; `.git` stock-readable |
| **G22-C05** | ObjectStore Refuse / FS sole store honesty under feature-on graph |
| **G22-C06** | Sole-runner lock; discrete-circuit sibling mount present |

## Waves

| Wave | Work | Exit |
|------|------|------|
| **G22-0** | SoT + plan + matrix + interop row; raise dual-path verification bar | Docs; `spec check` |
| **G22-1** | Harness (`run-g22.sh`, compose `g22`, stress tests, discrete-circuit mount) | Code ready |
| **G22-exit** | Docker `g22` PASS; evidence; residual/roadmap honesty; commit+push | Verified |

## Out of scope

- Reopening L3 shared-index N-way
- Concurrent heavy G17/G19 beside this gate
- Porcelain CLI for CELIR indexes

## Record

| Wave | Status | Tip / evidence |
|------|--------|----------------|
| G22-0 | Closed | SoT + matrix + interop |
| G22-1 | Closed | harness + discrete-circuit mount |
| G22-exit | Closed (Verified) | [[evidence:root/20261109204428]]; Docker `g22` PASS N=32/128 |
