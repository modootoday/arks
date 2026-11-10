---
id: evidence:root/20261110224500
title: FW-PF5-NA seed amortization G19 remeasure (Claimed partial 4/8)
status: active
created: 2026-11-10T22:45:00Z
updated: 2026-11-10T00:00:20Z
tags: [g19, pf5, clean-room, perf, aggressive, amortization, verified]
rel:
  depends_on: [sot:root/pf5-journey-seed-amortization, sot:root/clean-room-journey-compare, sot:root/production-residual-register, adr:root/20261110162000, plan:root/20261110220000, evidence:root/20261110153000]
  evidence: []
files:
  - {path: receipt.json, sha256: d671024e10b624587dd3f3bb9169cc9e45890459695312121e118b522e7748ea, bytes: 8250, store: repo}
  - {path: metrics.json, sha256: e14f2102c263c6172426b979e943ef22edff062697553fcae7781ce736873391, bytes: 133967, store: repo}
---

# FW-PF5-NA — seed amortization G19 remeasure

## Context

Reopens [[adr:root/20261110162000]] under [[sot:root/pf5-journey-seed-amortization]] / [[plan:root/20261110220000]]. Harness adds catalog-faithful timed add/commit amortization + larger seeds for Pf5 leftovers. Honest **Claimed(partial 4/8)**.

## Environment

| Field | Value |
|-------|-------|
| Compose | `verify/docker-compose.yml` service `g19` (sole) |
| Image | `arks-verify-m1:0.1.0` / `sha256:5736c5687d3b5775e187ca3d21fbc5b2a5a2bf8965c5381778ee478eb185adf0` |
| Stock Git | `git version 2.39.5` |
| Iterations / aggregation | 3 / `median_p95` |
| Wave | `Pf5` |

## Aggregate

| Metric | Value |
|--------|------:|
| M17 ≤0.8× | **19/19** PASS |
| ≤0.5× claimed | **15** |
| aggressive_na | **4** |
| aggressive terminal | **19/19** |
| miss | **[]** |

### Pf5 class flips (≤0.5× claimed this receipt)

| ID | git_ms | arks_ms | ratio |
|----|-------:|--------:|------:|
| J-LOCAL-01 | 202 | 94 | 0.4653 |
| J-BRANCH-01 | 196 | 92 | 0.4694 |
| J-BUNDLE-01 | 264 | 123 | 0.4659 |
| J-ALIAS-01 | 202 | 98 | 0.4851 |

### Remaining aggressive_na

| ID | git_ms | arks_ms | ratio | Class |
|----|-------:|--------:|------:|-------|
| J-CLONE-LOCAL-01 | 107 | 80 | 0.7477 | mid |
| J-WORKTREE-01 | 169 | 94 | 0.5562 | mid |
| J-STASH-01 | 175 | 99 | 0.5657 | quantum |
| J-RM-MV-01 | 146 | 86 | 0.5890 | quantum |

Gate exit **0**.
