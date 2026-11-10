---
id: sot:root/arks-store
title: arks-store unit
status: active
created: 2026-11-08T09:26:51Z
updated: 2026-11-09T20:28:15Z
tags: [m2, unit, store, facade, ring-4, restart, celir-index]
verify: manual
rel:
  governs: [crates/arks-store]
  depends_on: [sot:root/storage-celir-boundary, adr:root/20261110052030, sot:root/celir-index-dual-path, sot:root/crate-architecture, sot:root/program-roadmap, plan:root/20261108090647, sot:root/arks-object, evidence:root/20261108185319]
  evidence: [evidence:root/20261108190033]
---

# arks-store unit

Version **0.5.1**. Unit SoT for Ring 4 Git-first object-store facade ([[plan:root/20261108090647]], M2 Phase S). Under [[sot:root/program-roadmap]] **v0.3.1**, M2 restart / re-verify after M1 re-prove ([[evidence:root/20261108185319]]) is carried by [[evidence:root/20261108190033]]; prior [[evidence:root/20261108093059]] remains **historical / non-carrying alone**. CELIR dual-path: [[sot:root/storage-celir-boundary]] v0.8.1 + [[adr:root/20261110052030]] + companion [[sot:root/celir-index-dual-path]]. Additive **pack-aware read + pack ingest** supports M15/M14 clone paths without exploding every received pack to loose.

## Rule

1. **Facade trait:** `ObjectStore` with associated `Error` and `put` / `get` / `exists` using [[sot:root/arks-object]] `RawObject`, [[sot:root/arks-hash]] `ObjectFormat`, and [[sot:root/arks-oid]] `Oid`. ObjectStore surface MUST stay Git-first — **MUST NOT** implement put/get/exists via CELIR ([[sot:root/storage-celir-boundary]]).
2. **M2 backends:** `MemoryObjectStore` for host-advisory unit tests. **`FilesystemGitStore`** implements the same trait for Git objects under an `objects/` root: loose via [[sot:root/arks-object]] `write_loose` / `read_loose`, plus **additive** `objects/pack/pack-*.{pack,idx}` read (lazy decode) and `ingest_pack_bytes` write ([gitformat-pack](https://git-scm.com/docs/gitformat-pack) / gitrepository-layout). Missing objects MUST surface as `StoreError::NotFound` for `get`.
3. **Optional feature `celir-index` (default OFF):** MAY path-dep `celir-index` / `celir-index-runtime` / `celir-data` and expose a compile-gated indexing companion for product-private indexes only (sidecar layout, rebuild-from-`.git`, query) per [[sot:root/celir-index-dual-path]]. MUST NOT be an ObjectStore backend. Default features MUST leave CELIR edges absent ([[adr:root/20261110052030]]).
4. **No `gix`** and no product-path `git` subprocess ([[adr:root/20261108083507]]).
5. **Out of scope (this unit):** refs/index/repo layout; MIDX/`.rev` product claims; CELIR ObjectStore; porcelain CLI for CELIR indexes; M3+ porcelain orchestration.

## Why

Routing object I/O through a small facade keeps Ring 1 types free of layout details and lets M2 exit gates (G10) target one trait while the filesystem Git backend remains stock-Git interchangeable for the claimed loose-object subset. Keeping `celir-index` default-off preserves the default Cargo graph while allowing an honest indexing-service dual-path.

## Verification

1. Host `cargo test -p arks-store` — **advisory** (facade + FS unit tests).
2. Clean-room **G10** objects-subset (`verify/gates/g10` + `run-g10.sh`) with **G1** regression green — **authoritative**; carrying receipt under v0.3.1: [[evidence:root/20261108190033]] ([[plan:root/20261108090647]], [[sot:root/interop-test-matrix]], `verify/matrices/matrix-m2.md`).
3. `spec check` clean for this SoT.
4. Default `cargo check -p arks-store` shows no required `celir-*` edge; `cargo check -p arks-store --features celir-index` succeeds when discrete-circuit packages exist; no `gix` / gitoxide.

**Prior (non-carrying alone):** [[evidence:root/20261108093059]] — historical gap-fill input under v0.3.1.
