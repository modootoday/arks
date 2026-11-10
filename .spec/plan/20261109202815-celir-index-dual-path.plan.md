---
id: plan:root/20261109202815
title: CELIR index dual-path sidecar rebuild query
status: active
created: 2026-11-09T20:28:15Z
updated: 2026-11-09T20:45:00Z
tags: [celir, indexing-service, dual-path, sidecar, rebuild, query, celir-index, m24-precursor]
rel:
  depends_on:
    - sot:root/storage-celir-boundary
    - adr:root/20261110052030
    - sot:root/arks-store
    - sot:root/celir-index-dual-path
    - sot:root/lease-concurrency-intent
    - sot:root/product-verification-clean-room
    - sot:root/crate-architecture
  implements: [sot:root/celir-index-dual-path, sot:root/storage-celir-boundary]
  evidence: [evidence:root/20261109203236]
---

# CELIR index dual-path sidecar rebuild query

> **Superseded as numbered milestone:** delivery identity is now **M24** [[plan:root/20261109203430]]. This plan remains the Ci-0…Ci-exit wave ledger that fed M24.

## Goal

Ship the remaining **index-only** dual-path behind `arks-store` feature **`celir-index`** (default OFF): product-private sidecar dirs, rebuild-from-`.git`, and build → publish → open → query using real CELIR APIs. Retain VCS Refuse / FS sole object store / `arks-lease` multi-writer. Owning SoT: [[sot:root/celir-index-dual-path]]. Boundary: [[sot:root/storage-celir-boundary]] v0.8.0+. Enablement: [[adr:root/20261110052030]].

## Done rules

| Disposition | Meaning |
|-------------|---------|
| **Open** | Wave not yet shipped |
| **Closed (Claimed)** | Wave acceptance met; tip pushed; honesty intact |
| **Advisory** | Host `cargo` / unit green — **not** clean-room product verified |
| **Verified** | Docker clean-room receipt — **not required** for this index companion slice |

Slice **Claimed** iff Ci-0…Ci-2 all Closed (Claimed), `spec check` clean, feature default OFF, and residual/goal record advisory vs verified.

## Waves

| Wave | Work | Exit |
|------|------|------|
| **Ci-0** | SoT + this plan: sidecar layout, rebuild schema, query surface, Done rules; cross-link boundary / arks-store / residual | Docs accepted; `spec check` clean; commit+push |
| **Ci-1** | Implement behind `--features celir-index`: sidecar path helpers; rebuild-from-`.git` builder; Writer commit + Store open/lookup; optional runtime `build`/`Snapshot::open` wiring; feature-on tests | `cargo test -p arks-store --features celir-index` green; commit+push |
| **Ci-2** | Default-off still green; `cargo check` both modes; honesty prose; advisory evidence stamp; residual Claimed; no ObjectStore bleed | Both checks/tests green; evidence; Claimed; commit+push |
| **Ci-exit** | Recount; UpdateGoal complete only when slice Claimed | Goal STATUS=complete |

## Claimed set

| ID | Behavior |
|----|----------|
| **CI-SIDECAR** | `<git-dir>/arks/celir-index/` product-private; outside Git object/ref/pack meaning |
| **CI-REBUILD** | Rebuild rows from FS Git objects arks already owns → CELIR publish |
| **CI-QUERY** | Open + exact lookup on published sidecar generation |
| **CI-RUNTIME** | Optional JSONL path: `celir_index_runtime::build` → `Snapshot::open` → `query` |
| **CI-DEFAULT-OFF** | Default graph has zero CELIR edges; feature-off tests green |
| **CI-HONESTY** | Not ObjectStore; no CELIR-backed Git verified claim; multi-writer = arks-lease |

## Out of scope

- CELIR ObjectStore; default-on feature; Git OID export/import invention
- Docker clean-room gate for this companion (Git-compat verified remains FS clean-room elsewhere)
- Porcelain CLI surface for CELIR indexes
- Power-loss / multi-writer CELIR store claims

## Record

| Wave | Status | Tip / evidence |
|------|--------|----------------|
| Ci-0 | Closed (Claimed) | `96ed001` |
| Ci-1 | Closed (Claimed) | `9f3c618` |
| Ci-2 | Closed (Claimed) | [[evidence:root/20261109203236]] (host advisory) |
| Ci-exit | Closed (Claimed) | Slice Claimed; goal complete |
