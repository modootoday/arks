---
id: evidence:root/20261109064000
title: M17 Pf3-1 G19 full-allow-list baseline (failers identified)
status: active
created: 2026-11-09T06:40:00Z
updated: 2026-11-10T00:57:42Z
tags: [g019-comparative-performance, m17, pf3, baseline, clean-room, perf]
rel:
  depends_on: [plan:root/20261109043916, sot:root/clean-room-journey-compare, sot:root/interop-test-matrix, evidence:root/20261109213000, evidence:root/20261109063000]
  evidence: []
files:
  - {path: receipt.json, sha256: b15879511b537ab15afef817cd784fc18658d8208bcaeb2e88f91cf544b4d582, bytes: 3972, store: repo}
---

# M17 Pf3-1 — G19 full allow-list baseline

Docker `g019-comparative-performance` against tip `1f2ed35` after M16 exit. Allow-list = all 19 required
G16 journeys. **Gate FAIL** (expected for Pf3-1): only `J-REPACK-01` and
`J-SUBMODULE-01` meet ≤0.8×. Remaining IDs are Pf3-2 optimization targets
(process-startup dominated short porcelain vs stock Git).

## Pass / fail summary

| ID | Ratio | Pass |
|----|-------|------|
| J-REPACK-01 | 0.725 | yes |
| J-SUBMODULE-01 | 0.350 | yes |
| J-CLONE-LOCAL-01 | 0.898 | no (closest failer) |
| (16 others) | 1.6–4.5 | no |

## Environment

| Field | Value |
|-------|-------|
| Compose | `g019-comparative-performance` |
| Image | `arks-verify-m1:0.1.0` / `sha256:5736c5687d3b5775e187ca3d21fbc5b2a5a2bf8965c5381778ee478eb185adf0` |
| Stock Git | `git version 2.39.5` |
| Tip | `1f2ed35fa538854e9e2ccdad8a76e7accf5ddd85` |
