---
id: plan:root/20261109203430
title: M24 CELIR index dual-path migration
status: active
created: 2026-11-09T20:34:30Z
updated: 2026-11-09T20:50:00Z
tags: [m24, celir, indexing-service, dual-path, sidecar, rebuild, query, celir-index, migration, g22]
rel:
  depends_on:
    - sot:root/program-roadmap
    - sot:root/celir-index-dual-path
    - sot:root/celir-index-contention-clean-room
    - sot:root/storage-celir-boundary
    - adr:root/20261110052030
    - sot:root/arks-store
    - sot:root/lease-concurrency-intent
    - sot:root/product-verification-clean-room
    - plan:root/20261109202815
    - plan:root/20261109203949
  implements: [sot:root/celir-index-dual-path, sot:root/program-roadmap]
  evidence: [evidence:root/20261109203236, evidence:root/20261109204428]
---

# M24 — CELIR index dual-path / migration

## Goal

Numbered milestone **M24**: optional CELIR **indexing-service** dual-path / migration behind `arks-store` feature **`celir-index`** (default OFF). Ship product-private **sidecar beside `.git`**, **rebuild-from-`.git`**, and **build → publish → open → query** using real discrete-circuit APIs. Retain ObjectStore **Refuse**, FS `.git` sole object store, and **`arks-lease`** multi-writer on the filesystem store.

Owning unit SoT: [[sot:root/celir-index-dual-path]]. Boundary: [[sot:root/storage-celir-boundary]]. Enablement: [[adr:root/20261110052030]]. Prior companion waves: [[plan:root/20261109202815]] (folded into this milestone).

## Done rules

| Disposition | Meaning |
|-------------|---------|
| **Closed (Claimed)** | Sidecar beside `.git` + rebuild-from-`.git` + build/publish/open/query behind `celir-index`; ObjectStore Refuse; FS default; arks-lease multi-writer; tests both modes; `spec check`; advisory evidence; tip pushed |
| **Advisory** | Host `cargo` / unit green — **not** Docker clean-room contention verified |
| **Verified (G22)** | Docker clean-room **G22** index-contention PASS ([[plan:root/20261109203949]], [[sot:root/celir-index-contention-clean-room]]) — still **not** “CELIR-backed Git verified” |

## Claimed set

| ID | Behavior |
|----|----------|
| **M24-SIDECAR** | Non-bare: `<worktree>/.arks/celir-index/` beside `.git`; bare: `<bare>/.arks/celir-index/`; outside object/ref/pack meaning |
| **M24-REBUILD** | arks enumerates FS Git objects → CELIR publish |
| **M24-QUERY** | Open + exact lookup on published sidecar |
| **M24-RUNTIME** | Optional JSONL: `celir_index_runtime::build` → `Snapshot::open` → `query` |
| **M24-DEFAULT-OFF** | Default graph zero CELIR edges |
| **M24-HONESTY** | Not ObjectStore; no invented OID export/import; multi-writer = arks-lease |

## Waves

| Wave | Work | Exit |
|------|------|------|
| **M24-0** | Canonicalize as M24 in roadmap / residual / this plan; align sidecar **beside** `.git` | Docs + layout lock; `spec check` |
| **M24-1** | Impl + tests (feature on/off); cargo check both modes | Host green |
| **M24-exit** | Evidence honesty; residual Claimed; push | Slice Claimed |

## Out of scope

- CELIR ObjectStore; default-on feature; invented Git OID export/import
- Porcelain CLI for CELIR indexes
- Shared-index N-way (G11-L3 OUT); CELIR-backed Git verified

## Record

| Wave | Status | Tip / evidence |
|------|--------|----------------|
| M24-0 | Closed (Claimed) | canonicalize + beside-`.git` layout |
| M24-1 | Closed (Claimed) | prior `9f3c618` + beside-`.git` align |
| M24-exit | Closed (Claimed / host advisory) | [[evidence:root/20261109203236]]; tip `3051916` |
| M24-G22 | Closed (Verified) | [[evidence:root/20261109204428]]; [[plan:root/20261109203949]] |
