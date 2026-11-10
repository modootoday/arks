---
id: adr:root/20261110052030
title: Optional CELIR indexing-service product path-dep
status: active
created: 2026-11-10T05:20:30Z
updated: 2026-11-10T05:20:30Z
tags: [celir, indexing-service, cargo, enablement, dual-path, facade]
rel:
  depends_on:
    - sot:root/storage-celir-boundary
    - sot:root/crate-architecture
    - sot:root/git-compat-surface
    - sot:root/lease-concurrency-intent
    - sot:root/product-verification-clean-room
    - evidence:root/20261108192100
---

# Optional CELIR indexing-service product path-dep

## Context

Through [[sot:root/storage-celir-boundary]] **v0.6.0–v0.7.0**, arks **excluded** CELIR from the product Cargo graph entirely. That lock was correct for the **object store**: CELIR **Refuses** a frozen VCS put/get/exists Git object/ref/pack contract ([[evidence:root/20261108192100]]; reconfirmed in the discrete-circuit indexing-service pack). The indexing-service **precondition** pack at catalog pin `ef6ca5afdbf8ec95e8ca237ad895a4356ab82689` (tip hygiene retaining the pack: `3a5af1ca059da8e8b70212c5720aac1fb41098bb`) made an optional dual-path **discussable** but did **not** enable Cargo edges.

Product direction now **explicitly enables** an optional, index-only Cargo path so remaining indexing-sidecar / rebuild-from-`.git` work can proceed without inventing a CELIR object store.

## Decision

1. **Accepted — optional product Cargo path-deps for index-only CELIR.** arks **MAY** depend on discrete-circuit crates `celir-index` (`packages/celir-index/`) and `celir-index-runtime` (`crates/celir-index-runtime/`) behind a non-default Cargo feature named **`celir-index`** on `arks-store` (storage facade owner for the companion indexing surface).
2. **Default OFF.** Workspace / package default features MUST NOT enable `celir-index`. Default product builds keep **zero** CELIR edges (Git-compat FS `.git` path unchanged).
3. **Role lock — indexing service only.** Enabled CELIR edges authorize build / publish / open / query of **product-private index artifacts** (sidecar directories **outside** Git object/ref/pack meaning, and/or arks-owned **rebuild-from-`.git`** into a CELIR index root). They do **not** authorize CELIR as a Git object/ref/pack store.
4. **VCS Refuse retained.** Do **not** implement ObjectStore `put` / `get` / `exists` (or equivalent) via CELIR. Do **not** silently dual-write or replace filesystem `.git` as the sole Git-compat object store.
5. **No `arks-celir` product object-store adapter.** A thin compile-gated companion module under `arks-store` (or a future ADR-named index facade) is allowed for indexing-service linkage; a CELIR-backed ObjectStore implementor remains **forbidden**.
6. **Multi-writer** on filesystem Git remains **`arks-lease`** ([[sot:root/lease-concurrency-intent]]). CELIR index stores stay single-writer per CELIR norms.
7. **Honesty / verification.** Enabling the feature is **not** a “CELIR-backed Git verified” claim. Product “verified” for Git-compat still requires filesystem/stock-Git clean-room per [[sot:root/product-verification-clean-room]]. OID export/import via CELIR remains **absent**; sidecar + rebuild-from-`.git` are the honest interop patterns.
8. **Citation pin.** Cite discrete-circuit catalog Current `ef6ca5afdbf8ec95e8ca237ad895a4356ab82689` or any newer committed tip that retains `.agents/consumer/arks/indexing-service.md` (prefer that pack over chat).

This ADR **supersedes** the v0.6.0–v0.7.0 **Cargo-exclusion** product decision for the **scoped indexing-service edge only**. It does **not** supersede the object-store Refuse, FS-default sole store, or clean-room rules.

## Consequences

- [[sot:root/storage-celir-boundary]] advances to dual-path opt-in language (v0.8.0+).
- [[sot:root/crate-architecture]] allows the optional `celir-index` feature on `arks-store`; default graph stays CELIR-free.
- Rule `50-celir-boundary`, residual register, and program-roadmap side track record enablement + retained Refuse.
- Remaining work unblocked: product-private sidecar index dirs, rebuild-from-`.git` index builders, and lab/query wiring behind `celir-index` — still without CELIR object-store claims.
