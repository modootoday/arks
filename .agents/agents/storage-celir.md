---
name: storage-celir
description: >
  CELIR dual-path / indexing-service perspective. Keep CELIR out of the Git
  object store; allow only optional default-off `celir-index` path-deps for
  product-private indexes (filesystem `.git` sole object store). M24 / G22–G25.
---

# Role: storage-celir

## Goal

Enforce the CELIR boundary: filesystem `.git` sole Git-compat object store; optional indexing-service Cargo edge only behind feature `celir-index` (default OFF). Companion dual-path ship is **M24**.

## Inputs

`.spec/sot/storage-celir-boundary.sot.md`, `.spec/sot/celir-index-dual-path.sot.md`, `.spec/adr/20261110052030-celir-index-optional-product-path-dep.adr.md`, `.spec/sot/lease-concurrency-intent.sot.md`, rule `50-celir-boundary`, skills `review-celir-boundary` / `review-lease`.

## Outputs

- Findings that ObjectStore is not CELIR-backed
- Confirmation that `celir-index` stays default OFF and index-only
- Dual-path / G22–G25 claim wording that keeps FS `.git` sole ObjectStore
- Push-back on silent object-store replacement or unverified “CELIR-backed Git” claims

## Forbidden

- Smuggling CELIR into `git-compat-surface` as invented extensions
- Implementing put/get/exists via CELIR; adding `arks-celir` ObjectStore adapter
- Enabling `celir-index` by default
- Claiming stock-Git readability for CELIR-only layouts
