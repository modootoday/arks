---
id: evidence:root/20261110074200
title: M26 G34 shared-index contention Verified
status: active
created: 2026-11-10T07:42:00Z
updated: 2026-11-10T00:00:20Z
tags: [m26, g34, shared-index, clean-room, verified, fw-shared-idx]
rel:
  depends_on: [adr:root/20261110074000, plan:root/20261110074000, sot:root/shared-index-nway-clean-room, sot:root/arks-lease, sot:root/interop-test-matrix, sot:root/product-verification-clean-room, evidence:root/20261110074100]
files:
  - {path: receipt.json, sha256: 529a1d7a6ac2599f9b3eb33264eb3a8337d34abadedce31213de24a7e13af8e8, bytes: 320, store: repo}
---

# M26 G34 shared-index contention (FW-SHARED-IDX)

## Context

Product Verified for shared-index N-way under [[adr:root/20261110074000]]. Gate **G34** (≠ G22/G24; G33 reserved elsewhere). Primary: [api-lockfile](https://git-scm.com/docs/api-lockfile), [gitformat-index](https://git-scm.com/docs/gitformat-index).

## Environment

| Field | Value |
|-------|-------|
| Compose | `verify/docker-compose.yml` service `g34` |
| Image | `arks-verify-m1:0.1.0` |
| Image Id | `sha256:b235193c9ae45991f057aec1551feb769bf796ca917af1745110917cf53dfada` |
| Stock Git | git version 2.39.5 |
| rustc | 1.99.1 |
| Pre-commit tip | `eea8ebaa92b287e777657700bdefbdae4fec2bd1` |
| Receipt | `verify/gates/g34/out/receipt.json` |

## Results

| Gate | Command | Outcome |
|------|---------|---------|
| **G34** | `docker compose … run --rm g34` | **PASS** — G34-C01..C03 (2-writer shared index; AlreadyHeld; live `index.lock` refuse) |

## Non-claims

- Does not rewrite M23 L3 worktree/ref-shard.
- Does not substitute M24 / G22 / G24 CELIR receipts.
- Does not claim CELIR ObjectStore; zero gix.
