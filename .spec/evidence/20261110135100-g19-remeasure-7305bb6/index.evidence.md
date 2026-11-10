---
id: evidence:root/20261110135100
title: G19 sole-runner remeasure at tip 7305bb6 (superseded tip; PF5 partial 7/8; CLONE-LOCAL aggressive_na)
status: active
created: 2026-11-10T03:51:00Z
updated: 2026-11-10T03:51:00Z
tags: [g19, g16, pf5, clean-room, perf, journey-compare, verified, clone-local, verify-layout]
rel:
  depends_on: [sot:root/clean-room-journey-compare, sot:root/pf5-journey-seed-amortization, sot:root/production-residual-register, sot:root/product-verification-clean-room, evidence:root/20261110113900, evidence:root/20261110114200, evidence:root/20261110110900]
  evidence: []
files:
  - {path: receipt.json, sha256: 78fd3b602302e15e7ac38bd72d04f65b397b9aec33a4357599ddad96b323b9bb, bytes: 7255, store: repo}
  - {path: metrics.json, sha256: 1f9a97a67b5e0b84794d8cf92e881b36a1ee7e2c4339a036fa5ed3ec8f020416, bytes: 228213, store: repo}
  - {path: arks-pf5-rm-7305bb6.log, sha256: 65161eff08a879e4e4670693b08dfa1e5b1150fb3e4842f71f81fa9cd4e5a966, bytes: 14202, store: repo}
---

# G19 sole-runner remeasure — tip `7305bb6` (superseded; PF5 7/8; A not flipped)

## Context

Authoritative sole Docker remasure (`--name arks-pf5-rm-7305bb6`) measured tip `7305bb6` (“Skip credential fill on unauthenticated HTTP exchange.”) after Docker quiet + CI SUCCESS on that tip. Clean complete (G19 PASS; no OOM 137 / SIGTERM 143). **Tip subsequently moved** to `1a02559` (revert of that commit) before residual tip-pin land — this pack is a **completed remasure of superseded tip `7305bb6`**, not a claim that current `origin/main` is still `7305bb6`. Do **not** remasure the new tip in this hop; watch `1a02559` CI instead. Cites [[sot:root/pf5-journey-seed-amortization]] — residual **FW-PF5-J-CLONE-LOCAL**. Reconfirms **M17 ≤0.8×** and Pf5 aggressive terminal with honest **Claimed(partial 7/8)**. **J-CLONE-LOCAL-01** remains `aggressive_na` at ratio **0.6818** — A ≤0.5× not met (do not market / do not flip PF5 8/8). **J-CLONE-PATH-01** ≤0.5× **PASS** at **0.4627**.

## Scope measured (in)

| Surface | IDs | Role |
|---------|-----|------|
| G16 functional + timing | All 19 catalog journeys | Hard functional + `median_p95` |
| G19 comparative | Same allow-list (`verify/matrices/matrix-g19.md`) | ≤0.8× + ≤0.5× / `aggressive_na` |

## Environment

| Field | Value |
|-------|-------|
| Runner | sole `docker run` → `verify/scripts/run-g19.sh` (shared cargo volume `arks-verify-m1_arks-cargo-target`; same gate path as compose `g19`) |
| Paths | `verify/{gates,matrices,fixtures,scripts}` |
| Image | `arks-verify-m1:0.1.0` / `sha256:5736c5687d3b5775e187ca3d21fbc5b2a5a2bf8965c5381778ee478eb185adf0` |
| Stock Git | `git version 2.39.5` |
| Iterations / aggregation | 3 / `median_p95` |
| Wave | `Pf5` |
| Tip measured | `7305bb645bb98db232189fa60a78b51fe805685a` |
| CI | SUCCESS all jobs — https://github.com/modootoday/arks/actions/runs/42751950350 |

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
| J-CLONE-LOCAL-01 | 88 | 60 | **0.6818** | ≤0.5× needs further product smart-HTTP clone work |

### CLONE PATH (aggressive PASS)

| ID | git_ms | arks_ms | ratio |
|----|-------:|--------:|------:|
| J-CLONE-PATH-01 | 67 | 31 | **0.4627** |

## Honesty

Do **not** flip PF5 to 8/8. Do **not** treat this pack as current-tip primary after `origin/main` advanced to `1a02559`. Carrying primary tip remasure remains `7a3d2dc` LOCAL **0.6703** / PATH **0.4615** ([[evidence:root/20261110113900]]) until a sole remasure of the live tip. This pack records superseded-tip `7305bb6` LOCAL **0.6818** / PATH **0.4627**. Secondary/historical mimalloc tip remasure `97daecd` LOCAL **0.7363** / PATH **0.4789** ([[evidence:root/20261110114200]]) stays secondary — **NOT** primary. Historical **best-better-LOCAL** remains `44dc566` LOCAL **0.6556** / PATH **0.4675** ([[evidence:root/20261110110900]]).
