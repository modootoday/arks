---
id: evidence:root/20261110234600
title: FW-PF5-NA seed amortization G19 remeasure (Claimed partial 6/8)
status: active
created: 2026-11-10T23:46:00Z
updated: 2026-11-10T23:46:00Z
tags: [g019-comparative-performance, pf5, clean-room, perf, aggressive, amortization, verified]
rel:
  depends_on: [sot:root/pf5-journey-seed-amortization, sot:root/clean-room-journey-compare, sot:root/production-residual-register, adr:root/20261110162000, plan:root/20261110220000, evidence:root/20261110224500]
  evidence: []
files:
  - {path: receipt.json, sha256: 31700fd24cafd6031f5a227fbe9aedd5142a2989cfff998a5642b3b6bc5b2031, bytes: 7296, store: repo}
  - {path: metrics.json, sha256: 19d71f9928b41065063d600003b08e3c541476e9b8eecb64c93258a6ff5fc745, bytes: 218062, store: repo}
---

# FW-PF5-NA — seed amortization G19 remeasure (6/8)

## Context

Further reopen under [[sot:root/pf5-journey-seed-amortization]] after [[evidence:root/20261110224500]] (4/8). Heavier timed-commit / residual seeds flip **STASH** + **WORKTREE**. Honest **Claimed(partial 6/8)**. Remaining `aggressive_na`: **RM-MV**, **CLONE-LOCAL**.

## Environment

| Field | Value |
|-------|-------|
| Compose | `verify/docker-compose.yml` service `g019-comparative-performance` (sole) |
| Image | `arks-verify-m1:0.1.0` / `sha256:5736c5687d3b5775e187ca3d21fbc5b2a5a2bf8965c5381778ee478eb185adf0` |
| Stock Git | `git version 2.39.5` |
| Iterations / aggregation | 3 / `median_p95` |
| Wave | `Pf5` |
| Tip measured | `24b2c794de4f2de73c2fbfc093eff662e604e1cf` (+ harness amortize pending commit) |

## Aggregate

| Metric | Value |
|--------|------:|
| M17 ≤0.8× | **19/19** PASS |
| ≤0.5× claimed | **17** |
| aggressive_na | **2** |
| aggressive terminal | **19/19** |
| miss | **[]** |

### Pf5 class flips this receipt (≤0.5×)

| ID | git_ms | arks_ms | ratio |
|----|-------:|--------:|------:|
| J-STASH-01 | 534 | 222 | 0.4157 |
| J-WORKTREE-01 | 491 | 206 | 0.4196 |

Prior 4/8 set (LOCAL/BRANCH/BUNDLE/ALIAS) reconfirmed ≤0.5× on this receipt.

### Remaining aggressive_na

| ID | git_ms | arks_ms | ratio | Class |
|----|-------:|--------:|------:|-------|
| J-CLONE-LOCAL-01 | 73 | 46 | 0.6301 | mid |
| J-RM-MV-01 | 394 | 198 | 0.5025 | quantum |

Gate exit **0**.
