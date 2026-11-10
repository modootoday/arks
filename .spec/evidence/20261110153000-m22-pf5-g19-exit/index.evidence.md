---
id: evidence:root/20261110153000
title: M22 Rc-3 Pf5 G19 aggressive leftovers terminal
status: active
created: 2026-11-10T15:30:00Z
updated: 2026-11-10T15:30:00Z
tags: [g19, m22, pf5, clean-room, perf, aggressive, verified]
rel:
  depends_on: [plan:root/20261110150000, sot:root/clean-room-journey-compare, sot:root/interop-test-matrix, sot:root/production-residual-register, evidence:root/20261109143000]
  evidence: []
files:
  - {path: receipt.json, sha256: a87cadc201fff4c156b6cda826cfdf769b5be148c9b5653e9cf9466ecb9115a4, bytes: 8441, store: repo}
  - {path: metrics.json, sha256: 5061b2824b7433de344c5cf475551d80c102a2a4cd4db5922be8b937895224dd, bytes: 51508, store: repo}
---

# M22 Rc-3 — Pf5 G19 aggressive leftovers terminal

## Context

Closes M18 journey-class leftovers for ≤0.5× honesty: every G16 journey is either ≤0.5× **claimed** or **aggressive_na** with SoT cite. M17 ≤0.8× remains N/A-free measured. No silent keep-at-0.8.

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
| ≤0.5× claimed | **11** |
| aggressive_na | **8** |
| aggressive terminal | **19/19** |
| miss | **[]** |

### aggressive_na (SoT-cited)

| Class | IDs |
|-------|-----|
| quantum | BRANCH, BUNDLE, RM-MV, STASH |
| process-tax | LOCAL, ALIAS |
| mid | CLONE-LOCAL, WORKTREE |

Gate exit **0**.
