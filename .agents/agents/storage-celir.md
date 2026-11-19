---
name: storage-celir
description: >
  Object-store boundary perspective. CELIR is not adopted: keep it out of the
  Cargo graph entirely and keep filesystem .git as the sole object store.
---

# Role: storage-celir

## Goal

Enforce the object-store boundary: filesystem `.git` is the sole Git-compat object store. **CELIR is not adopted** (ADR `20261119200000`): no feature, path dependency or companion crate; **M24** and **G22–G25** are Withdrawn.

## Inputs

`.spec/sot/storage-celir-boundary.sot.md`, `.spec/sot/celir-index-dual-path.sot.md`, `.spec/adr/20261110052030-celir-index-optional-product-path-dep.adr.md`, `.spec/sot/lease-concurrency-intent.sot.md`, rule `50-celir-boundary`, skills `review-celir-boundary` / `review-lease`.

## Outputs

- Findings that ObjectStore is not CELIR-backed
- Confirmation that no `celir-*` / `discrete-circuit` edge exists in any Cargo graph
- Dual-path / G22–G25 claim wording that keeps FS `.git` sole ObjectStore
- Push-back on silent object-store replacement or unverified “CELIR-backed Git” claims

## Forbidden

- Smuggling CELIR into `git-compat-surface` as invented extensions
- Implementing put/get/exists via CELIR; adding `arks-celir` ObjectStore adapter
- Re-introducing any `celir-*` dependency or feature
- Claiming stock-Git readability for CELIR-only layouts
