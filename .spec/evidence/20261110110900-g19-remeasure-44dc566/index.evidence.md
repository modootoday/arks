---
id: evidence:root/20261110110900
title: G19 sole-runner remeasure at tip 44dc566 (PF5 partial 7/8; PATH ≤0.5 PASS; LOCAL aggressive_na)
status: active
created: 2026-11-10T11:09:00Z
updated: 2026-11-10T11:09:00Z
tags: [g19, g16, pf5, clean-room, perf, journey-compare, verified, clone-local, clone-path]
rel:
  depends_on: [sot:root/clean-room-journey-compare, sot:root/pf5-journey-seed-amortization, sot:root/production-residual-register, sot:root/product-verification-clean-room, evidence:root/20261110014600]
  evidence: []
files:
  - {path: receipt.json, sha256: 4f984865f0209a4578c47af9b06af55f3018eb7636efd8e80e6b5c8e2d330b34, bytes: 6958, store: repo}
  - {path: arks-pf5-rm-44dc566b.log, sha256: d8e165332fa9f29a99b922a41921d101525fdb8d2cb0d3b44039495ff2d8fd94, bytes: 13793, store: repo}
---

# G19 sole-runner remeasure — tip `44dc566` (PF5 7/8; A not flipped)

## Context

Authoritative **best complete** sole Docker `g19` (`--name arks-pf5-rm-44dc566b`) at tip `44dc566` (“Avoid zero-filling HTTP read chunks”). Cites [[sot:root/pf5-journey-seed-amortization]] — HTTP seed amortization exhausted; residual **FW-PF5-J-CLONE-LOCAL**. Reconfirms **M17 ≤0.8× 19/19** and **Pf5 aggressive terminal 19/19** with honest **Claimed(partial 7/8)**. **J-CLONE-PATH-01** ≤0.5× **PASS** at **0.4675**; **J-CLONE-LOCAL-01** remains `aggressive_na` at **0.6556** — A ≤0.5× not met (do not market). Later tip remasure attempts after this receipt were SIGTERM/SIGKILL’d mid-run (OOM=false); do not stampede.

## Scope measured (in)

| Surface | IDs | Role |
|---------|-----|------|
| G16 functional + timing | All 19 catalog journeys | Hard functional + `median_p95` |
| G19 comparative | Same allow-list (`verify/matrices/matrix-g19.md`) | ≤0.8× + ≤0.5× / `aggressive_na` |

## Environment

| Field | Value |
|-------|-------|
| Compose | `verify/docker-compose.yml` service `g19` (sole) |
| Image | `arks-verify-m1:0.1.0` / `sha256:5736c5687d3b5775e187ca3d21fbc5b2a5a2bf8965c5381778ee478eb185adf0` |
| Stock Git | `git version 2.39.5` |
| Iterations / aggregation | 3 / `median_p95` |
| Wave | `Pf5` |
| Tip measured | `44dc5663f11c325d3a0bf507e04e073e67c44b64` |

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

### Clone honesty

| ID | git_ms | arks_ms | ratio | Bar |
|----|-------:|--------:|------:|-----|
| J-CLONE-PATH-01 | 77 | 36 | **0.4675** | ≤0.5× **PASS** |
| J-CLONE-LOCAL-01 | 90 | 59 | **0.6556** | ≤0.8× PASS; ≤0.5× **aggressive_na** |

## Honesty

- **A (CLONE-LOCAL ≤0.5×):** not flipped — keep `aggressive_na`; PF5 stays **7/8**.
- **Tip honesty** uses [[evidence:root/20261110111700]] (`91fbf64`: LOCAL **0.7262** / PATH **0.4928**). This receipt remains the historical best-better-LOCAL sample only.
- Receipt rebuilt into Specfold from the complete sole log (`arks-pf5-rm-44dc566b.log`); gate `out/` was later overwritten by tip churn.
