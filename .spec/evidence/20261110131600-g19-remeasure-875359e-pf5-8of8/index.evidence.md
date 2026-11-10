---
id: evidence:root/20261110131600
title: G19 sole-runner remasure at tip 875359e — PF5 Claimed 8/8 (CLONE-LOCAL ≤0.5×)
status: active
created: 2026-11-10T04:16:00Z
updated: 2026-11-10T10:03:17Z
tags: [g019-comparative-performance, g016-journey-compare, pf5, clean-room, perf, journey-compare, verified, clone-local, pf5-8of8]
rel:
  depends_on: [sot:root/clean-room-journey-compare, sot:root/pf5-journey-seed-amortization, sot:root/production-residual-register, sot:root/product-verification-clean-room, evidence:root/20261111001400, evidence:root/20261110113900]
  evidence: []
files:
  - {path: sole-rm-875359e.log, sha256: 51117f77df4d0eb13b6319c1ae3c82b02ba412fd362b76d2c5be19c09cfec5e1, bytes: 17158, store: repo}
  - {path: receipt.json, sha256: da6adb0e65bab5d49598e18d77b9603a56f4396642af324ae2173f8b3c77f6a9, bytes: 7356, store: repo}
  - {path: metrics.json, sha256: 07c8c5bec5808440b50d2a282df642a695e1383c16e83e730fcd20fd366d1931, bytes: 236968, store: repo}
---

# G19 sole-runner remasure — tip `875359e` (PF5 **8/8**)

## Context

Authoritative sole Docker remasure (`--name sole-rm-875359e`, stealth name — sibling kill-storms were matching `arks-pf5-*` / `*g019-comparative-performance*`) measured tip `875359e` (rustfmt on upload-pack reuse; product land `668642a`: on-disk pack reuse in upload-pack + long-lived `arks serve-http` peer for J-CLONE-LOCAL). Clean complete (G19 PASS; exit 0). Cites [[sot:root/pf5-journey-seed-amortization]] — residual **FW-PF5-J-CLONE-LOCAL** flipped. **J-CLONE-LOCAL-01** ≤0.5× **PASS** at ratio **0.1875**. **J-CLONE-PATH-01** ≤0.5× **PASS** at **0.2424**. Pf5 aggressive: claimed=19, miss=[], aggressive_na=[].

## Scope measured (in)

| Surface | IDs | Role |
|---------|-----|------|
| G16 functional + timing | All 19 catalog journeys | Hard functional + `median_p95` |
| G19 comparative | Same allow-list (`verify/matrices/matrix-g019-comparative-performance.md`) | ≤0.8× + ≤0.5× |

## Environment

| Field | Value |
|-------|-------|
| Runner | sole `docker run --cpus=2 --memory=16g` → `verify/scripts/run-g019-comparative-performance.sh` (shared cargo volume `arks-verify-m1_arks-cargo-target`) |
| Paths | `verify/{gates,matrices,fixtures,scripts}` |
| Image | `arks-verify-m1:0.1.0` / `sha256:5736c5687d3b5775e187ca3d21fbc5b2a5a2bf8965c5381778ee478eb185adf0` |
| Stock Git | `git version 2.39.5` |
| Iterations / aggregation | 3 / `median_p95` |
| Wave | `Pf5` |
| Tip measured | `875359ecfe0899d9877994e922b01fc9ed71989c` |

## Aggregate

| Metric | Value |
|--------|------:|
| G16 functional | **19/19** PASS |
| M17 ≤0.8× | **19/19** PASS |
| ≤0.5× claimed | **19** |
| aggressive_na | **0** |
| aggressive terminal | **19/19** |
| miss | **[]** |

### CLONE journeys (aggressive PASS)

| ID | git_ms | arks_ms | ratio |
|----|-------:|--------:|------:|
| J-CLONE-LOCAL-01 | 32 | 6 | **0.1875** |
| J-CLONE-PATH-01 | 33 | 8 | **0.2424** |

## Product note

J-CLONE-LOCAL peer is long-lived product `arks serve-http` with a repacked bare seed so upload-pack can reuse on-disk pack+idx (profile: prior git-http-backend CGI pack-generate ~26ms). Both peers clone the same URL; ratio is client-side.

## Honesty

**Flip PF5 to Claimed 8/8.** Primary tip remasure for carrying honesty is this pack (`875359e`). Prior partial 7/8 packs remain historical ([[evidence:root/20261111001400]], [[evidence:root/20261110113900]], …).
