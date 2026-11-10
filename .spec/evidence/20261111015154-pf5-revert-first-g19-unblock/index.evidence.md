---
id: evidence:root/20261111015154
title: Pf5 REVERT-first unblock — sole G19 (REVERT ≤0.5×; ALIAS aggressive_na)
status: active
created: 2026-11-10T16:51:54Z
updated: 2026-11-10T16:53:05Z
tags:
  [
    g19,
    pf5,
    revert,
    alias,
    aggressive_na,
    external-deps-unblock,
    clean-room,
  ]
rel:
  depends_on:
    - research:root/20261111012840
    - sot:root/pf5-journey-seed-amortization
    - sot:root/clean-room-journey-compare
    - sot:root/production-residual-register
    - adr:root/20261110162000
    - evidence:root/20261111003444
  evidence: []
files:
  - {path: g19/receipt.json, sha256: 0915ec13eb2f255aadd954cf573e61c8bd71b86deeccf501318b6a3f3983699e, bytes: 7343, store: repo}
  - {path: g19/metrics.json, sha256: 4a973a46cc3a86671b4e9d79dd45bd1f7c4c954bf434042735ed7fde11d66bbf, bytes: 244732, store: repo}
  - {path: g19-run.log, sha256: b644691decaaff3cbc8c9326a71447de64f528fe18d798d27b30891c115a3f3d, bytes: 29096, store: repo}
  - {path: g19/ub-notes.md, sha256: 8670f7e7806794fdc18e50e05720170701bd665a4b4e8ff85b6be135120f51de, bytes: 535, store: repo}
---

# Pf5 REVERT-first unblock — sole G19

## Context

Executes Plan B from [[research:root/20261111012840]] rank 3: **REVERT-first** harness nudge `PF5_REVERT_AMORTIZE_COMMITS` **12→20**, then **exactly one** sole Docker G19 (`--name sole-rm-pf5-ub`). No ALIAS seed bump; no PATH/`exec` redesign this wave (ALIAS still miss → honest `aggressive_na`). No second remasure.

Working-tree harness change measured under git tip base `14082e6` (revision field records HEAD; amortize constant change is in the mounted workspace).

## Environment

| Field | Value |
|-------|-------|
| Runner | sole `docker compose … run --rm --name sole-rm-pf5-ub g19` |
| Image | `arks-verify-m1:0.1.0` / `sha256:5736c5687d3b5775e187ca3d21fbc5b2a5a2bf8965c5381778ee478eb185adf0` |
| Stock Git | `git version 2.39.5` |
| Iterations / aggregation | 3 / `median_p95` |
| Wave | `Pf5` |
| HEAD at remasure | `14082e6512c5c893d719ad3195f809d3e2ca51dc` |

## Aggregate

| Metric | Value |
|--------|------:|
| M17 ≤0.8× | **19/19 PASS** |
| ≤0.5× claimed | **18** |
| aggressive_na | **1** (`J-ALIAS-01`) |
| miss | **[]** |
| Gate | **G19 PASS** (exit 0) |

### Targeted journeys

| Journey | git_ms | arks_ms | ratio | Disposition |
|---------|-------:|--------:|------:|-------------|
| J-REVERT-01 | 164 | 82 | **0.5000** | **Claimed ≤0.5×** (was U1 `aggressive_na` 0.5417 @ +12) |
| J-ALIAS-01 | 366 | 203 | **0.5546** | `aggressive_na` process-tax (was U1 0.7906); no PATH redesign this wave |

## Honesty

- One sole remasure only — **no stampede**.
- ALIAS miss retained as honest `aggressive_na` / umbrella **Claimed(partial)**.
- Live G8 / L9 / crypto immunity / full FS sync **not** claimed by this pack.
