---
id: evidence:root/20261110012500
title: G19 sole-runner remeasure at tip 6bc4786 (PF5 partial 7/8; CLONE-LOCAL aggressive_na)
status: active
created: 2026-11-10T01:25:00Z
updated: 2026-11-10T01:25:00Z
tags: [g19, g16, pf5, clean-room, perf, journey-compare, verified, clone-local]
rel:
  depends_on: [sot:root/clean-room-journey-compare, sot:root/pf5-journey-seed-amortization, sot:root/production-residual-register, evidence:root/20261110011500]
  evidence: []
files:
  - {path: receipt.json, sha256: acd4a98b7217ee61110b558ac926298a03ddbb2bfc3965a57b2055c5adec21da, bytes: 7255, store: repo}
  - {path: metrics.json, sha256: 6373eefdbd789f037e24a2a0a6610c608ae67568ea29c8e8dbf2a35f1bfb3f2e, bytes: 228218, store: repo}
---

# G19 sole-runner remeasure — tip `6bc4786` (PF5 7/8; A not flipped)

## Context

Sole Docker `g19` after A–E land + hooks BrokenPipe / G14 docs-path relocate (`6bc4786`). **M17 ≤0.8× 19/19** and aggressive terminal **19/19** with honest **Claimed(partial 7/8)**. **J-CLONE-LOCAL-01** remains `aggressive_na` at ratio **0.7053** — A ≤0.5× not met (do not market).

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

- **G15 PASS** on tree including hooks BrokenPipe + G14 path fix (`verify/gates/g15/out/docker-g15-6bc4786.log`).
