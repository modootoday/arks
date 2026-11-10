---
id: sot:root/shared-index-nway-clean-room
title: Shared-index N-way clean-room (G34)
status: active
created: 2026-11-10T07:40:00Z
updated: 2026-11-10T07:40:00Z
tags: [m26, g34, shared-index, clean-room, lease, index-lock]
verify: manual
rel:
  depends_on:
    - adr:root/20261110074000
    - plan:root/20261110074000
    - sot:root/lease-concurrency-intent
    - sot:root/arks-lease
    - sot:root/git-compat-surface
    - sot:root/product-verification-clean-room
    - sot:root/interop-test-matrix
    - research:root/20261108071915
    - research:root/20261109023117
  evidence: [evidence:root/20261110074200]
---

# Shared-index N-way clean-room (G34)

Version **0.1.1**. Normative clean-room gate for **M26** shared-index N-way contention under product leases + Git `index.lock`. **Claimed + Verified** [[evidence:root/20261110074200]].

## Rule

### What G34 proves

| Proves | Does **not** prove |
|--------|---------------------|
| N concurrent writers (required floor **N≥2**; stretch N≥32 when harnessed) coordinate **one** `$GIT_DIR/index` via exclusive lease `index:shared` | M23 L3 worktree/ref-shard rewrite |
| Every index mutation still takes documented `index.lock` ([api-lockfile](https://git-scm.com/docs/api-lockfile)) | CELIR ObjectStore or CELIR index dual-path (≠ G22/G24) |
| Second lease holder fails closed (`AlreadyHeld`); live `index.lock` fails closed (no skip) | “Faster than Git” / perf |
| Final index is stock-readable ([gitformat-index](https://git-scm.com/docs/gitformat-index)) when stock Git peer is available in-image | Host-only Verified |

### Authority

1. Product **Verified** requires sole-runner Docker compose service `g34` / `verify/scripts/run-g34.sh` PASS + evidence.
2. Host `cargo test -p arks-lease --test g34_shared_index_contention` is **advisory** only.
3. MUST NOT cite M24 / G22 / G24 receipts as G34 substitutes.

### Matrix / entrypoint

- Matrix: `verify/matrices/matrix-g34.md`
- Script: `verify/scripts/run-g34.sh`
- Compose: `verify/docker-compose.yml` service `g34`

## Why

Shared-index N-way was Refuse-closed out of L3 ([[adr:root/20261110160000]]). M26 unlocks it only with a dedicated gate that cites Git index lock semantics and keeps lease additive.

## Verification

1. `spec check` clean for this SoT.
2. G34 PASS receipt before residual **Verified**.
3. Zero `gix`; no CELIR ObjectStore edges on the default graph.
