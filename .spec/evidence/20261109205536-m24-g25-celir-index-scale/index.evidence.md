---
id: evidence:root/20261109205536
title: M24 G25 CELIR index scale clean-room
status: active
created: 2026-11-09T20:55:36Z
updated: 2026-11-09T22:48:40Z
tags: [m24, g25, celir, scale, clean-room, verified, contention, oom]
rel:
  depends_on: [sot:root/celir-index-scale-clean-room, plan:root/20261109204934, evidence:root/20261109204428, sot:root/product-verification-clean-room, sot:root/celir-index-contention-clean-room]
files:
  - {path: receipt.json, sha256: b72426743a2345e35d4e1ce694c85d6b31ad0549856da23b4fd2dd298f8fa193, bytes: 428, store: repo}
---

# M24 — G25 CELIR index scale

## Authority

| Field | Value |
|-------|--------|
| Gate | **G25** (`verify/scripts/run-g25.sh` / compose `g25`) |
| Matrix | `verify/matrices/matrix-g25.md` |
| Required N | **256** PASS |
| Stretch | **384** PASS; **512** PASS |
| Stock git | `git version 2.39.5` |
| rustc | `1.99.1` |
| Result | **PASS** |
| Parent tip | `b11b952` |
| Historical | G22 N=32/128 [[evidence:root/20261109204428]] |

## Suite

| Step | Result |
|------|--------|
| N=256 concurrent open/query | PASS |
| N=384 stretch | PASS |
| N=512 stretch | PASS |
| Sole-runner + discrete-circuit | OK |

## Honesty

- **Verified:** index-contention **scale** for N≥256 (stretch through 512 green this run).
- Does not claim dual-path Git (G23) or L3+index joint (G24) by itself.
- Sole-runner OOM policy held; no fake green.
