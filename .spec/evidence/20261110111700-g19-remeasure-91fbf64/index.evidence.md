---
id: evidence:root/20261110111700
title: G19 sole-runner remeasure at tip 91fbf64 (PF5 partial 7/8; CLONE-LOCAL aggressive_na)
status: active
created: 2026-11-10T02:17:00Z
updated: 2026-11-10T02:17:00Z
tags: [g19, g16, pf5, clean-room, perf, journey-compare, verified, clone-local, verify-layout]
rel:
  depends_on: [sot:root/clean-room-journey-compare, sot:root/pf5-journey-seed-amortization, sot:root/production-residual-register, sot:root/product-verification-clean-room, evidence:root/20261110014600]
  evidence: []
files:
  - {path: receipt.json, sha256: 326323b97c8fe7dfbc51cf8056315b740b646c8349983ae9c5d0ece24d146057, bytes: 7253, store: repo}
  - {path: metrics.json, sha256: fbd000a25fdbe91f4df395be8cf9a605f6f3d18a667fef0798e3d8c0721ee3bd, bytes: 228235, store: repo}
  - {path: arks-pf5-rm-91fbf64c.log, sha256: d4d0a3b0c2f40787830c597307f9378f2a1b08f93492c913e9d0f9c07faf1f3f, bytes: 13520, store: repo}
---

# G19 sole-runner remeasure — tip `91fbf64` (PF5 7/8; A not flipped)

## Context

Authoritative sole Docker remasure (`--name arks-pf5-rm-91fbf64c`) at tip `91fbf64` after prior OOM/stacked-abort incidents. Warm cargo target finished release in ~0.03s (no compile OOM). Cites [[sot:root/pf5-journey-seed-amortization]] — residual **FW-PF5-J-CLONE-LOCAL**. Reconfirms **M17 ≤0.8×** and Pf5 aggressive terminal with honest **Claimed(partial 7/8)**. **J-CLONE-LOCAL-01** remains `aggressive_na` at ratio **0.7262** — A ≤0.5× not met (do not market / do not flip PF5 8/8).

## Scope measured (in)

| Surface | IDs | Role |
|---------|-----|------|
| G16 functional + timing | All 19 catalog journeys | Hard functional + `median_p95` |
| G19 comparative | Same allow-list (`verify/matrices/matrix-g19.md`) | ≤0.8× + ≤0.5× / `aggressive_na` |

## Environment

| Field | Value |
|-------|-------|
| Compose | `verify/docker-compose.yml` service `g19` (sole; shared cargo volume) |
| Paths | `verify/{gates,matrices,fixtures,scripts}` |
| Image | `arks-verify-m1:0.1.0` / `sha256:5736c5687d3b5775e187ca3d21fbc5b2a5a2bf8965c5381778ee478eb185adf0` |
| Stock Git | `git version 2.39.5` |
| Iterations / aggregation | 3 / `median_p95` |
| Wave | `Pf5` |
| Tip measured | `91fbf6438ea8d1688ebdb0c387af0b3c0938af80` |

## Aggregate

| Metric | Value |
|--------|------:|
| G16 functional | **19/19** PASS |
| M17 ≤0.8× | **19/19** PASS |
| ≤0.5× claimed | **18** |
| aggressive_na | **1** (J-CLONE-LOCAL-01) |
| aggressive terminal | **19/19** |
| miss | **[]** |

### Remaining aggressive_na

| ID | git_ms | arks_ms | ratio | Note |
|----|-------:|--------:|------:|------|
| J-CLONE-LOCAL-01 | 84 | 61 | 0.7262 | ≤0.5× needs further product smart-HTTP clone work |

### CLONE PATH (aggressive PASS)

| ID | git_ms | arks_ms | ratio |
|----|-------:|--------:|------:|
| J-CLONE-PATH-01 | 69 | 34 | 0.4928 |

## Honesty

Do **not** flip PF5 to 8/8. Prior carrying pin 0.6705 @ `e466d56` ([[evidence:root/20261110014600]]) remains the better residual pin; this tip remasure is honest and tip-proof but **worse** on LOCAL. Stacked `arks-verify-lane-a` abort (exit 143) and earlier tip OOM 137 are **not** product fails for A.
