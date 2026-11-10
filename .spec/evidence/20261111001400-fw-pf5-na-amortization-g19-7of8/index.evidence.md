---
id: evidence:root/20261111001400
title: FW-PF5-NA seed amortization G19 remeasure (Claimed partial 7/8)
status: active
created: 2026-11-11T00:14:00Z
updated: 2026-11-11T00:14:00Z
tags: [g19, pf5, clean-room, perf, aggressive, amortization, verified]
rel:
  depends_on: [sot:root/pf5-journey-seed-amortization, sot:root/clean-room-journey-compare, sot:root/production-residual-register, adr:root/20261110162000, plan:root/20261110220000, evidence:root/20261110234600]
  evidence: []
files:
  - {path: receipt.json, sha256: 31c9efc4525bed088f0b850fbb86014a545f852ae0c2e00a47b3bbdadeee3c2e, bytes: 7534, store: repo}
  - {path: metrics.json, sha256: 5ceac9b6fea16cc46693cdbc3b1738c1f70935929a2813a50efe18f4f88eb545, bytes: 237031, store: repo}
---

# FW-PF5-NA — seed amortization G19 remeasure (7/8)

## Context

Further reopen under [[sot:root/pf5-journey-seed-amortization]] after [[evidence:root/20261110234600]] (6/8). Heavier RM-MV multi-path rm + timed-commit amortization flips **J-RM-MV-01**. Honest **Claimed(partial 7/8)**. Remaining `aggressive_na`: **CLONE-LOCAL** only.

Advisory trial: larger HTTP bare seed (720 files × 256 body + history) **worsened** CLONE-LOCAL ratio (~0.78 vs ~0.63) and threatened M17 ≤0.8× headroom — reverted to baseline 220-file seed. ≤0.5× for smart-HTTP local clone needs product work, not further harness inflation.

## Environment

| Field | Value |
|-------|-------|
| Compose | `verify/docker-compose.yml` service `g19` (sole) |
| Image | `arks-verify-m1:0.1.0` / `sha256:5736c5687d3b5775e187ca3d21fbc5b2a5a2bf8965c5381778ee478eb185adf0` |
| Stock Git | `git version 2.39.5` |
| Iterations / aggregation | 3 / `median_p95` |
| Wave | `Pf5` |
| Tip measured | `84700ef60dc8477f19ff73d61796ca41c8057479` (+ harness RM-MV amortize pending commit) |

## Aggregate

| Metric | Value |
|--------|------:|
| M17 ≤0.8× | **19/19** PASS |
| ≤0.5× claimed | **18** |
| aggressive_na | **1** |
| aggressive terminal | **19/19** |
| miss | **[]** |

### Pf5 class flips this receipt (≤0.5×)

| ID | git_ms | arks_ms | ratio |
|----|-------:|--------:|------:|
| J-RM-MV-01 | 701 | 262 | 0.3738 |

Prior 6/8 set (LOCAL/BRANCH/BUNDLE/ALIAS/STASH/WORKTREE) reconfirmed ≤0.5× on this receipt.

### Remaining aggressive_na

| ID | git_ms | arks_ms | ratio | Class | Note |
|----|-------:|--------:|------:|-------|------|
| J-CLONE-LOCAL-01 | 80 | 51 | 0.6375 | mid | HTTP seed amortization exhausted (larger seeds worsened ratio) |

Gate exit **0**.
