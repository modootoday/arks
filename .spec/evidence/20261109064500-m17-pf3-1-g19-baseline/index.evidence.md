---
id: evidence:root/20261109064500
title: M17 Pf3-1 G19 full allow-list baseline
status: active
created: 2026-11-09T06:45:00Z
updated: 2026-11-10T00:57:42Z
tags: [g19, m17, pf3, baseline, clean-room, perf]
rel:
  depends_on: [plan:root/20261109043916, sot:root/interop-test-matrix, sot:root/clean-room-journey-compare, sot:root/program-roadmap, evidence:root/20261109061600]
  evidence: []
files:
  - {path: receipt.json, sha256: cb05fda20d11be150102cf09305bb398e33219537aabc2fe7f55f6257ac63e7e, bytes: 3976, store: repo}
---

# M17 Pf3-1 — G19 full allow-list baseline

Pf3-1 baseline after M16 Sy-1..Sy-5 G21 green. Allow-list = all 19 G16 journey IDs.

## Result (authoritative Docker)

| Field | Value |
|-------|-------|
| Compose | `g19` |
| Image | `arks-verify-m1:0.1.0` / `sha256:5736c5687d3b5775e187ca3d21fbc5b2a5a2bf8965c5381778ee478eb185adf0` |
| Stock Git | `git version 2.39.5` |
| Aggregation | `median_p95` / iterations=3 / release + thin LTO |
| Pass (<=0.8x) | **J-REPACK-01** (0.6857), **J-SUBMODULE-01** (0.2683) |
| Fail | 17 remaining IDs (ratios 1.58x-7.70x); closest miss **J-CLONE-LOCAL-01** (0.9194) |

## Hard block note

Short porcelain journeys remain process/implementation-overhead dominated vs stock Git C CLI. Meeting <=0.8x on the full catalog requires a dedicated Pf3-2 optimization campaign (hot-path IO, startup, workload amortization) — not closable as a single wave after LTO + REPACK scale alone.

## Reproduce

```bash
docker compose -f verify/docker-compose.yml run --rm g19
```
