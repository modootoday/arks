---
id: sot:root/celir-index-scale-clean-room
title: CELIR index scale clean-room
status: active
created: 2026-11-09T20:49:15Z
updated: 2026-11-10T05:49:00Z
tags: [celir, indexing-service, scale, g25, m24, clean-room, contention, oom]
verify: clean-room
rel:
  governs: [verify/g25, verify/scripts/run-g25.sh, verify/matrix-g25.md]
  depends_on:
    - sot:root/celir-index-contention-clean-room
    - sot:root/celir-index-dual-path
    - sot:root/product-verification-clean-room
    - sot:root/storage-celir-boundary
    - adr:root/20261109204916
    - plan:root/20261109204934
  evidence: [evidence:root/20261109204428, evidence:root/20261109205536]
---

# CELIR index scale clean-room

Version **0.1.0**. Normative Docker clean-room gate **G25** extending G22 N-scale beyond 32/128 toward higher hundreds under sole-runner OOM policy. Parent: [[sot:root/celir-index-contention-clean-room]]. Authority: [[sot:root/product-verification-clean-room]].

## Scope

| In | Out |
|----|-----|
| Gate **G25** (`verify/scripts/run-g25.sh` / compose `g25`) | Fake PASS after OOM / exit 137 |
| Required tier **N=256** concurrent readonly open/query after publish | Redefining G22 historical N=32/128 Verified |
| Optional stretch **N=384** and **N=512** under sole-runner OOM policy | Concurrent heavy G17/G19/G22/G23/G24 beside this gate |
| Same integrity / ObjectStore Refuse / stock-readable assertions as G22 | CELIR as ObjectStore |

## Rule

### Topology (N scale)

| Tier | N agents | Required for Verified |
|------|----------:|-----------------------|
| G22 historical | 32 / 128 | Already verified ([[evidence:root/20261109204428]]) — not re-proven here |
| Scale-required | **256** | Clean-room required |
| Scale-stretch | 384 | Attempt; OOM → hard-block ADR/receipt (no fake green) |
| Scale-stretch | 512 | Attempt; OOM → hard-block ADR/receipt (no fake green) |

### Assertions

1. After N=256 query storm, lookup effects match published Present; stock `git cat-file` OK.
2. Sole-runner lock held; discrete-circuit mount present or FAIL.
3. If stretch tier OOMs (137 / killed), gate MUST record **hard-block** for that tier only — N=256 PASS still authorizes scale Verified for the required tier.
4. ObjectStore Refuse retained.

### Honesty

G25 PASS = **index-contention scale verified** for N≥256. Does not claim dual-path Git (G23) or L3+index joint (G24).

## Why

G22 stopped at 128. Multi-agent realism pushes toward higher hundreds; sole-runner OOM policy forbids overlapping catalogs and forbids inventing greens after kill.

## Verification

1. `docker compose -f verify/docker-compose.yml run --rm g25` PASS (N=256 required).
2. Evidence receipt lists attempted N tiers and outcomes.
3. `spec check` clean for this SoT and [[plan:root/20261109204934]].
