---
id: evidence:root/20261109203236
title: CELIR dual-path companion host advisory
status: active
created: 2026-11-09T20:32:36Z
updated: 2026-11-09T20:45:00Z
tags: [celir, indexing-service, dual-path, advisory, host, celir-index, m24]
rel:
  depends_on:
    - sot:root/celir-index-dual-path
    - plan:root/20261109203430
    - plan:root/20261109202815
    - sot:root/storage-celir-boundary
    - adr:root/20261110052030
    - sot:root/product-verification-clean-room
    - sot:root/program-roadmap
---

# CELIR dual-path companion — host advisory receipt (M24)

## Authority

| Field | Value |
|-------|--------|
| Milestone | **M24** ([[plan:root/20261109203430]]) |
| Slice | [[sot:root/celir-index-dual-path]] / prior waves [[plan:root/20261109202815]] |
| Scope | Optional `arks-store` feature `celir-index` (default OFF) |
| Sidecar | Beside `.git`: `<worktree>/.arks/celir-index/` |
| Verification class | **Advisory (host)** — **not** Docker clean-room product verified |
| Git-compat verified | Unchanged — still FS/stock-Git clean-room only ([[sot:root/product-verification-clean-room]]) |

## Host commands (advisory)

| Command | Result |
|---------|--------|
| `cargo check -p arks-store` | green (default; no CELIR edge) |
| `cargo check -p arks-store --features celir-index` | green |
| `cargo test -p arks-store` | green (12 unit tests) |
| `cargo test -p arks-store --features celir-index` | green (17 unit tests incl. sidecar/rebuild/runtime) |
| `spec check` | clean at Claimed tip |

## Honesty

1. Companion is **not** an ObjectStore; no put/get/exists via CELIR.
2. Enabling `celir-index` is **not** “CELIR-backed Git verified.”
3. Multi-writer on FS Git remains **`arks-lease`**; CELIR index roots stay single-writer.
4. Sidecar path `<git-dir>/arks/celir-index/` is product-private — outside Git object/ref/pack meaning.
5. Discrete-circuit APIs cited only (`Writer`/`Store`/`Snapshot`, `celir_index_runtime::build` / `Snapshot::open` / `query`).

## Tips

| Wave | Tip |
|------|-----|
| Ci-0 | `96ed001` |
| Ci-1 | `9f3c618` |
| Ci-2 / Claimed | `b889dff` |
| M24 Claimed | `3051916` |
