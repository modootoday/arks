---
id: evidence:root/20261110113900
title: G19 sole-runner remeasure at tip 7a3d2dc (PF5 partial 7/8; CLONE-LOCAL aggressive_na)
status: active
created: 2026-11-10T02:39:00Z
updated: 2026-11-10T02:39:00Z
tags: [g19, g16, pf5, clean-room, perf, journey-compare, verified, clone-local, verify-layout]
rel:
  depends_on: [sot:root/clean-room-journey-compare, sot:root/pf5-journey-seed-amortization, sot:root/production-residual-register, sot:root/product-verification-clean-room, evidence:root/20261110111700, evidence:root/20261110110900]
  evidence: []
files:
  - {path: receipt.json, sha256: b1970e5891361939ff4926e3f48dec885b275336ff29ea9e3146293c996dd610, bytes: 7253, store: repo}
  - {path: metrics.json, sha256: c95f6036ad441d2a633ffaba17da508286993a96a6f9c84a4c1a0db952329518, bytes: 228228, store: repo}
  - {path: arks-pf5-rm-7a3d2dcb.log, sha256: cdbb96742b5a8076eaaa41d0d01e34f3cf0f4ee99fdd85dc85c581a3f57a136f, bytes: 13566, store: repo}
---

# G19 sole-runner remeasure — tip `7a3d2dc` (PF5 7/8; A not flipped)

## Context

Authoritative sole Docker remasure (`--name arks-pf5-rm-7a3d2dcb`) at tip `7a3d2dc` (“Serial smart-HTTP checkout; PATH worker cap 3”) after Docker quiet. Clean complete (G19 PASS; no OOM 137 / SIGTERM 143). Cites [[sot:root/pf5-journey-seed-amortization]] — residual **FW-PF5-J-CLONE-LOCAL**. Reconfirms **M17 ≤0.8×** and Pf5 aggressive terminal with honest **Claimed(partial 7/8)**. **J-CLONE-LOCAL-01** remains `aggressive_na` at ratio **0.6703** — A ≤0.5× not met (do not market / do not flip PF5 8/8). **J-CLONE-PATH-01** ≤0.5× **PASS** at **0.4615**.

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
| Tip measured | `7a3d2dc04bea2c39c9d50d19541595b0b1580617` |

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
| J-CLONE-LOCAL-01 | 91 | 61 | 0.6703 | ≤0.5× needs further product smart-HTTP clone work |

### CLONE PATH (aggressive PASS)

| ID | git_ms | arks_ms | ratio |
|----|-------:|--------:|------:|
| J-CLONE-PATH-01 | 78 | 36 | 0.4615 |

## Honesty

Do **not** flip PF5 to 8/8. This tip remasure improves on prior tip pin `91fbf64` LOCAL **0.7262** / PATH **0.4928** ([[evidence:root/20261110111700]]) but stays above ≤0.5× on LOCAL. Historical **best-better-LOCAL** remains `44dc566` LOCAL **0.6556** / PATH **0.4675** ([[evidence:root/20261110110900]]) — not this tip. Prior carrying pin `e466d56` @ 0.6705 ([[evidence:root/20261110014600]]) is essentially matched.
