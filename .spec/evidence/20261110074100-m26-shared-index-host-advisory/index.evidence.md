---
id: evidence:root/20261110074100
title: M26 shared-index host advisory (FW-SHARED-IDX SoT unlock)
status: active
created: 2026-11-10T07:41:00Z
updated: 2026-11-10T07:41:00Z
tags: [m26, g34, shared-index, host-advisory, fw-shared-idx]
rel:
  depends_on:
    - adr:root/20261110074000
    - plan:root/20261110074000
    - sot:root/shared-index-nway-clean-room
    - sot:root/arks-lease
    - sot:root/lease-concurrency-intent
---

# M26 shared-index host advisory

## Context

SoT unlock for **FW-SHARED-IDX** under [[adr:root/20261110074000]]. Host `cargo test -p arks-lease --test g34_shared_index_contention` exercises G34-C01..C03. This receipt is **advisory** — product **Verified** requires sole-runner Docker **G34**.

## Results (host)

| Case | Outcome |
|------|---------|
| G34-C01 2-writer shared index | PASS (host) |
| G34-C02 AlreadyHeld | PASS (host) |
| G34-C03 live index.lock refuse | PASS (host) |

## Non-claims

- Not Docker **G34** Verified.
- Not M24 / G22 / G24 substitute.
- Not a rewrite of M23 L3 worktree/ref-shard.
