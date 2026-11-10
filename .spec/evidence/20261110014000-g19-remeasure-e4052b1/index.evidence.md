---
id: evidence:root/20261110014000
title: G19 sole-runner remeasure at tip e4052b1 (PF5 partial 7/8 Verified; M17 green)
status: active
created: 2026-11-10T01:40:00Z
updated: 2026-11-10T01:40:00Z
tags: [g19, g16, pf5, clean-room, perf, journey-compare, verified, clone-local]
rel:
  depends_on: [sot:root/clean-room-journey-compare, sot:root/pf5-journey-seed-amortization, sot:root/production-residual-register, evidence:root/20261110011500, evidence:root/20261110012500]
  evidence: []
files:
  - {path: receipt.json, sha256: 1009505f80269f2adf6b5d9721a8738c7a32e05540dd5932391fe280b7c13cb7, bytes: 7255, store: repo}
  - {path: metrics.json, sha256: db285d9522487207fff1feb272ecc5c15a5258acda980a46b073c8de19b0d3e8, bytes: 228224, store: repo}
---

# G19 sole-runner remeasure — tip `e4052b1` (PF5 7/8 Verified)

## Context

Carrying sole Docker `g19` after pack-persist / Arc pack-cache experiments that briefly regressed **J-CLONE-LOCAL-01** above M17 ≤0.8× (noise/overlap runs at `8c65583` / mid tips). Tip **`e4052b1`** (“Avoid cloning pack bytes on synchronous ingest”) restores **M17 19/19** with honest **Claimed(partial 7/8)** — CLONE-LOCAL remains `aggressive_na` at ratio **0.6421**.

Host cargo remains advisory/unreliable. Overlapping Docker G19 containers are forbidden (sole-runner lock).

## Aggregate

| Metric | Value |
|--------|------:|
| Tip measured | `e4052b1f1809b642990696f2cd1b5d5fd05ed523` |
| M17 ≤0.8× | **19/19** PASS |
| ≤0.5× claimed | **18** |
| aggressive_na | **1** (J-CLONE-LOCAL-01 @ 0.6421) |
| Gate exit | **0** |

### Remaining aggressive_na

| ID | git_ms | arks_ms | ratio | Note |
|----|-------:|--------:|------:|------|
| J-CLONE-LOCAL-01 | 95 | 61 | 0.6421 | ≤0.5× needs further product smart-HTTP clone work |

## Related

- G15 PASS earlier at hooks/G14-path tip (`verify/gates/g15/out/docker-g15-6bc4786.log`).
- Prior pins: [[evidence:root/20261110011500]] (`3af76a7`), [[evidence:root/20261110012500]] (`6bc4786`).
