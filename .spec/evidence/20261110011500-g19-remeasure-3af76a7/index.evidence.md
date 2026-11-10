---
id: evidence:root/20261110011500
title: G19 sole-runner remeasure at tip 3af76a7 (PF5 partial 7/8 Verified; layout cdbc907+)
status: active
created: 2026-11-10T01:15:00Z
updated: 2026-11-10T01:15:00Z
tags: [g019-comparative-performance, g016-journey-compare, pf5, clean-room, perf, journey-compare, verified, verify-layout]
rel:
  depends_on: [sot:root/clean-room-journey-compare, sot:root/pf5-journey-seed-amortization, sot:root/production-residual-register, sot:root/product-verification-clean-room, sot:root/interop-test-matrix, evidence:root/20261111001400, evidence:root/20261110003836]
  evidence: []
files:
  - {path: receipt.json, sha256: 9ff07cfe89185948c2acdb235b164da2893ed7d196c78873513defca7f0a149b, bytes: 7256, store: repo}
  - {path: metrics.json, sha256: f6f254465eede2b4b74cdf48119318daca68308103414bb1f7cf71f72b78bced, bytes: 228211, store: repo}
---

# G19 sole-runner remeasure — tip `3af76a7` (PF5 7/8 Verified)

## Context

Authoritative sole-runner Docker `g019-comparative-performance` after verify layout relocate (`cb17373` / `cdbc907`) and A–E product land (`3af76a7`: pack caps, quarantine/hooks, dumb-HTTP client, PF5 clone hot path). Reconfirms **M17 ≤0.8× 19/19** and **Pf5 aggressive terminal 19/19** with honest **Claimed(partial 7/8)** (CLONE-LOCAL remains `aggressive_na` at ratio **0.604**). Host `cargo test` is advisory/unreliable (OOM exit 4294967295); this receipt is Docker-only Verified.

Follow-up tip `14142ef` is lint/harness-only (rustfmt + mock `delete-refs` + Windows loopback) and does not reopen G19 measurement.

## Scope measured (in)

| Surface | IDs | Role |
|---------|-----|------|
| G16 functional + timing | All 19 catalog journeys | Hard functional + `median_p95` |
| G19 comparative | Same allow-list (`verify/matrices/matrix-g019-comparative-performance.md`) | ≤0.8× + ≤0.5× / `aggressive_na` |

## Environment

| Field | Value |
|-------|-------|
| Compose | `verify/docker-compose.yml` service `g019-comparative-performance` (sole) |
| Paths | `verify/{gates,matrices,fixtures,scripts}` |
| Image | `arks-verify-m1:0.1.0` / `sha256:5736c5687d3b5775e187ca3d21fbc5b2a5a2bf8965c5381778ee478eb185adf0` |
| Stock Git | `git version 2.39.5` |
| Iterations / aggregation | 3 / `median_p95` |
| Wave | `Pf5` |
| Tip measured | `3af76a7a9661f0f1634014327b61e504a3c20b16` |

## Aggregate

| Metric | Value |
|--------|------:|
| G16 functional | **19/19** PASS |
| M17 ≤0.8× | **19/19** PASS |
| ≤0.5× claimed | **18** |
| aggressive_na | **1** (J-CLONE-LOCAL-01) |
| aggressive terminal | **19/19** |
| miss | **[]** |
| Gate exit | **0** |

### Remaining aggressive_na

| ID | git_ms | arks_ms | ratio | Note |
|----|-------:|--------:|------:|------|
| J-CLONE-LOCAL-01 | 101 | 61 | 0.604 | HTTP seed amortization exhausted; ≤0.5× needs further product smart-HTTP clone work |

## Honesty

- **A (CLONE-LOCAL ≤0.5×):** not flipped — keep `aggressive_na`; do not market ≤0.5× for this journey.
- **B–E:** product code Claimed at tip; client dumb / hooks / caps Verified only when matching Docker gates (G7/G15/G29+) land receipts — host cargo is not authority.
