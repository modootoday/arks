---
id: evidence:root/20261110142500
title: G19 sole-runner remasure at tip 79fc9bf — PF5 Claimed 8/8 (CLONE-LOCAL 0.1579)
status: active
created: 2026-11-10T04:25:00Z
updated: 2026-11-10T04:25:00Z
tags: [g19, g16, pf5, clean-room, perf, journey-compare, verified, clone-local, pf5-8of8]
rel:
  depends_on: [sot:root/clean-room-journey-compare, sot:root/pf5-journey-seed-amortization, sot:root/production-residual-register, sot:root/product-verification-clean-room, evidence:root/20261110131600, evidence:root/20261111001400]
  evidence: []
files:
  - {path: receipt.json, sha256: 47cada83bcd0efc74d82c60deddeb144f5e7c76b6711c978ff1d969adaa069a3, bytes: 7080, store: repo}
  - {path: arks-pf5-rm-50ad3ee.out.log, sha256: 0d887911c42ac7915a3d831209853911ce70f64f94d1da39b8ca5d48277e602c, bytes: 8485, store: repo}
  - {path: README.md, sha256: 55e617ac3e21d05429d3402a3a54fc6717bb370a5d03f7c45ebb44e664e2ed65, bytes: 524, store: repo}
---

# G19 sole-runner remasure — tip `79fc9bf` (PF5 **8/8**)

## Context

Authoritative sole Docker remasure measured tip `79fc9bf` (product cut `668642a`: on-disk pack reuse in upload-pack + long-lived `arks serve-http` peer for J-CLONE-LOCAL; seed `git repack -ad`). Clean complete (G19 PASS). Cites [[sot:root/pf5-journey-seed-amortization]]. **J-CLONE-LOCAL-01** ≤0.5× **PASS** at ratio **0.1579** (6/38). **J-CLONE-PATH-01** ≤0.5× **PASS** at **0.1739** (8/46). Pf5 aggressive: claimed=19, miss=[], aggressive_na=[].

## Environment

| Field | Value |
|-------|-------|
| Runner | sole `docker run --cpus=2` → G19 (`CARGO_BUILD_JOBS=1`) |
| Image | `arks-verify-m1:0.1.0` / `sha256:5736c5687d3b5775e187ca3d21fbc5b2a5a2bf8965c5381778ee478eb185adf0` |
| Stock Git | `git version 2.39.5` |
| Iterations / aggregation | 3 / `median_p95` |
| Wave | `Pf5` |
| Tip measured | `79fc9bfd1ddd865b2ecc28cbaebaa86bd5d8807c` |

## Aggregate

| Metric | Value |
|--------|------:|
| ≤0.5× claimed | **19** |
| aggressive_na | **0** |
| miss | **[]** |

### CLONE journeys (aggressive PASS)

| ID | git_ms | arks_ms | ratio |
|----|-------:|--------:|------:|
| J-CLONE-LOCAL-01 | 38 | 6 | **0.1579** |
| J-CLONE-PATH-01 | 46 | 8 | **0.1739** |

## Honesty

**Primary / carrying tip remasure for PF5 Claimed 8/8.** Prior carrying [[evidence:root/20261110131600]] (`875359e`, LOCAL **0.1875**) remains archival. Partial 7/8 packs remain historical ([[evidence:root/20261111001400]], …).
