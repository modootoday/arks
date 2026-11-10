---
id: evidence:root/20261110012500
title: G19 sole-runner remeasure at tip 6bc4786 (PF5 partial 7/8; CLONE-LOCAL aggressive_na)
status: active
created: 2026-11-10T01:25:00Z
updated: 2026-11-10T10:03:17Z
tags: [g019-comparative-performance, g016-journey-compare, pf5, clean-room, perf, journey-compare, verified, clone-local]
rel:
  depends_on: [sot:root/clean-room-journey-compare, sot:root/pf5-journey-seed-amortization, sot:root/production-residual-register, evidence:root/20261110011500]
  evidence: []
files:
  - {path: receipt.json, sha256: 66ff439e27f982c4fe194f0a24fc6ced00a2a93cb7728a90980ceef989ef0329, bytes: 7533, store: repo}
  - {path: metrics.json, sha256: 8cd86bd53503309ffad3ecf1feede4beb279cce30d6595e9fac967cf1a4493e2, bytes: 236994, store: repo}
---

# G19 sole-runner remeasure — tip `6bc4786` (PF5 7/8; A not flipped)

## Context

Sole Docker `g019-comparative-performance` after A–E land + hooks BrokenPipe / G14 docs-path relocate (`6bc4786`). **M17 ≤0.8× 19/19** and aggressive terminal **19/19** with honest **Claimed(partial 7/8)**. **J-CLONE-LOCAL-01** remains `aggressive_na` at ratio **0.7053** — A ≤0.5× not met (do not market).

Prior pin [[evidence:root/20261110011500]] (`3af76a7`, ratio 0.604) remains valid history. Follow-up tip `f6ac3e2` (clippy + pack RwLock + lazy credential discover) needs its own G19 pin.

## Aggregate

| Metric | Value |
|--------|------:|
| Tip measured | `6bc4786d3b20d503389b5a436b10ccae7a1be52f` |
| M17 ≤0.8× | **19/19** PASS |
| ≤0.5× claimed | **18** |
| aggressive_na | **1** (J-CLONE-LOCAL-01 @ 0.7053) |
| Gate exit | **0** |

## Related Docker gates

- **G15 PASS** on tree including hooks BrokenPipe + G14 path fix (`verify/gates/g015-smart-http/out/docker-g015-smart-http-6bc4786.log`).
