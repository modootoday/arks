---
id: evidence:root/20261110013400
title: G19 sole-runner remeasure at tip df79de9 (PF5 partial 7/8; CLONE-LOCAL aggressive_na)
status: active
created: 2026-11-10T01:34:00Z
updated: 2026-11-10T01:34:00Z
tags: [g19, g16, pf5, clean-room, perf, journey-compare, verified, clone-local, verify-layout]
rel:
  depends_on: [sot:root/clean-room-journey-compare, sot:root/pf5-journey-seed-amortization, sot:root/production-residual-register, sot:root/product-verification-clean-room, evidence:root/20261110012500, evidence:root/20261110011500]
  evidence: []
files:
  - {path: receipt.json, sha256: f24c525cb3dc4efd9bbbc221968ce411b858d555aecb3163aca603728e152fda, bytes: 7259, store: repo}
  - {path: metrics.json, sha256: d066e4d519c1e23d37e5a86417e2406b9d39945987a4ceb79c1b642b4f393f32, bytes: 228250, store: repo}
---

# G19 sole-runner remeasure — tip `df79de9` (PF5 7/8; A not flipped)

## Context

Authoritative sole Docker `g19` (`--name arks-g19-sole-df79de9`) after pack-cache Arc + deferred pack persist (`dd953b9` / `df79de9`) on the relocated layout (`verify/gates|matrices|fixtures`, scripts `verify/scripts/`). Reconfirms **M17 ≤0.8× 19/19** and **Pf5 aggressive terminal 19/19** with honest **Claimed(partial 7/8)**. **J-CLONE-LOCAL-01** remains `aggressive_na` at ratio **0.7412** — A ≤0.5× not met (do not market).

Follow-up tip `5bb5e78` (client dumb-HTTP offline proof + CLI N/A honesty) does not reopen G19 measurement.

## Scope measured (in)

| Surface | IDs | Role |
|---------|-----|------|
| G16 functional + timing | All 19 catalog journeys | Hard functional + `median_p95` |
| G19 comparative | Same allow-list (`verify/matrices/matrix-g19.md`) | ≤0.8× + ≤0.5× / `aggressive_na` |

## Environment

| Field | Value |
|-------|-------|
| Compose | `verify/docker-compose.yml` service `g19` (sole) |
| Paths | `verify/{gates,matrices,fixtures,scripts}` |
| Image | `arks-verify-m1:0.1.0` / `sha256:5736c5687d3b5775e187ca3d21fbc5b2a5a2bf8965c5381778ee478eb185adf0` |
| Stock Git | `git version 2.39.5` |
| Iterations / aggregation | 3 / `median_p95` |
| Wave | `Pf5` |
| Tip measured | `df79de9e0e494da5ba4e7502bd9338f7dc28c714` |

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
| J-CLONE-LOCAL-01 | 85 | 63 | 0.7412 | HTTP seed amortization exhausted; ≤0.5× needs further product smart-HTTP clone work |

## Honesty

- **A (CLONE-LOCAL ≤0.5×):** not flipped — keep `aggressive_na`; do not market ≤0.5× for this journey.
- **B–E:** product Claimed in-tree; client dumb HTTP offline proof at `5bb5e78`; hooks/quarantine/caps Claimed; proc-receive remains N/A fail-closed; CLI parity SoT keeps server G29–G32 Claimed (not N/A).
