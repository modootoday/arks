---
id: evidence:root/20261110114200
title: G19 secondary/historical mimalloc remasure at 97daecd (NOT primary tip pin; PF5 7/8; CLONE-LOCAL aggressive_na; REVERT miss)
status: active
created: 2026-11-10T02:42:00Z
updated: 2026-11-10T02:42:00Z
tags: [g019-comparative-performance, g016-journey-compare, pf5, clean-room, perf, journey-compare, verified, clone-local, verify-layout]
rel:
  depends_on: [sot:root/clean-room-journey-compare, sot:root/pf5-journey-seed-amortization, sot:root/production-residual-register, sot:root/product-verification-clean-room, evidence:root/20261110113900, evidence:root/20261110110900]
  evidence: []
files:
  - {path: receipt.json, sha256: 0deef9202a6d18f7d5e1aac896cf2f34182def8ac65543bf334d90914078883c, bytes: 7257, store: repo}
  - {path: metrics.json, sha256: 146f35b86070698a9a9cce7ade8a20111fe072edf24aacc4e2cfdc4616e07c50, bytes: 228246, store: repo}
  - {path: arks-pf5-rm-97daecd2.log, sha256: 0530575ac9cf54b24b73b59692599fbc0b346bb1b572ada506866fd781a5af4e, bytes: 13758, store: repo}
---

# G19 secondary/historical mimalloc remasure — `97daecd` (NOT primary tip pin; PF5 7/8; A not flipped)

## Context

Authoritative sole Docker remasure (`--name arks-pf5-rm-97daecd2`) at tip `97daecd` (“Keep PATH checkout cap 3; use mimalloc on arks”) after prior attempt `arks-pf5-rm-97daecd` died SIGTERM 143 with no ratios. Clean complete (ratios + receipt written; not OOM 137 / SIGTERM 143). Gate script reported **G19 FAIL** on short-wall **J-REVERT-01** aggressive miss **0.6667** (not in `aggressive_na`). Cites [[sot:root/pf5-journey-seed-amortization]] — residual **FW-PF5-J-CLONE-LOCAL**. **J-CLONE-LOCAL-01** remains `aggressive_na` at ratio **0.7363** — A ≤0.5× not met (do not market / do not flip PF5 8/8). **J-CLONE-PATH-01** ≤0.5× **PASS** at **0.4789**. Mimalloc tip **worsened** LOCAL vs prior tip pin `7a3d2dc` (0.6703); subsequent tip revert `83d9732` is product response (this pack measures `97daecd` only).

## Scope measured (in)

| Surface | IDs | Role |
|---------|-----|------|
| G16 functional + timing | All 19 catalog journeys | Hard functional + `median_p95` |
| G19 comparative | Same allow-list (`verify/matrices/matrix-g019-comparative-performance.md`) | ≤0.8× + ≤0.5× / `aggressive_na` |

## Environment

| Field | Value |
|-------|-------|
| Compose | `verify/docker-compose.yml` service `g019-comparative-performance` (sole; shared cargo volume) |
| Paths | `verify/{gates,matrices,fixtures,scripts}` |
| Image | `arks-verify-m1:0.1.0` / `sha256:5736c5687d3b5775e187ca3d21fbc5b2a5a2bf8965c5381778ee478eb185adf0` |
| Stock Git | `git version 2.39.5` |
| Iterations / aggregation | 3 / `median_p95` |
| Wave | `Pf5` |
| Tip measured | `97daecdd1f5925b52526b555c925dfcbbbfa66ae` |
| Wrapper exit file | `EXIT=` (blank); gate FAIL on REVERT — **not** 137/143 |

## Aggregate

| Metric | Value |
|--------|------:|
| G16 functional | **19/19** PASS |
| M17 ≤0.8× | **19/19** PASS |
| ≤0.5× claimed | **17** |
| aggressive_na | **1** (J-CLONE-LOCAL-01) |
| aggressive terminal | **18/19** |
| miss | **['J-REVERT-01']** (0.6667; short-wall; G19 FAIL) |

### Remaining aggressive_na

| ID | git_ms | arks_ms | ratio | Note |
|----|-------:|--------:|------:|------|
| J-CLONE-LOCAL-01 | 91 | 67 | **0.7363** | ≤0.5× needs further product smart-HTTP clone work |

### CLONE PATH (aggressive PASS)

| ID | git_ms | arks_ms | ratio |
|----|-------:|--------:|------:|
| J-CLONE-PATH-01 | 71 | 34 | **0.4789** |

## Honesty

Do **not** flip PF5 to 8/8. This pack is **secondary/historical mimalloc tip remasure only** — **NOT** the primary tip pin. It is **worse** on LOCAL than primary tip remasure `7a3d2dc` LOCAL **0.6703** / PATH **0.4615** ([[evidence:root/20261110113900]]). Historical **best-better-LOCAL** remains `44dc566` LOCAL **0.6556** / PATH **0.4675** ([[evidence:root/20261110110900]]) — not this tip. Do **not** invent REVERT `aggressive_na` without SoT reopen; record the miss honestly. No restart of sibling SIGTERM `arks-pf5-rm-83d9732` (outside this pack).
