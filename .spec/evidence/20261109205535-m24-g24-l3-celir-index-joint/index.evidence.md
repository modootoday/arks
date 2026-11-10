---
id: evidence:root/20261109205535
title: M24 G24 L3 plus CELIR index joint clean-room
status: active
created: 2026-11-09T20:55:35Z
updated: 2026-11-09T22:48:40Z
tags: [m24, g24, celir, l3, highconcurrency, clean-room, verified, joint]
rel:
  depends_on: [sot:root/l3-celir-index-joint-clean-room, plan:root/20261109204935, adr:root/20261109194200, sot:root/product-verification-clean-room, sot:root/celir-index-dual-path, evidence:root/20261109204428]
files:
  - {path: receipt.json, sha256: 5cc5dfc0677ac7f0c6f6981a43b2481e70fe636fa8b7cbcdb17f13161016b241, bytes: 364, store: repo}
---

# M24 — G24 L3 HighConcurrency + CELIR index joint

## Authority

| Field | Value |
|-------|--------|
| Gate | **G24** (`verify/scripts/run-g24.sh` / compose `g24`) |
| Matrix | `verify/matrices/matrix-g24.md` |
| N scale | **32** + **128** |
| Shared-index N-way | **OUT** |
| Stock git | `git version 2.39.5` |
| rustc | `1.99.1` |
| Result | **PASS** |
| Parent tip | `b11b952` |

## Suite

| Step | Result |
|------|--------|
| WriterMode::HighConcurrency opt-in | PASS |
| N=32 / N=128 HC shard apply + index readers | PASS |
| Competing `*.lock` → Pipeline; leases empty | PASS |
| ObjectStore filesystem-only | PASS |

## Honesty

- **Verified:** L3 + CELIR index joint under FS `.git`.
- Does **not** redefine G11-L3 as CELIR-bearing; shared-index N-way remains OUT.
- lease ≠ skip Git locks.
