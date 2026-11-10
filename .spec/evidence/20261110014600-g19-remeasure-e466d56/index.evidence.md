---
id: evidence:root/20261110014600
title: G19 sole-runner remeasure at tip e466d56 (PF5 partial 7/8; CLONE-LOCAL aggressive_na)
status: active
created: 2026-11-10T01:46:00Z
updated: 2026-11-10T01:46:00Z
tags: [g19, g16, pf5, clean-room, perf, journey-compare, verified, clone-local, verify-layout]
rel:
  depends_on: [sot:root/clean-room-journey-compare, sot:root/pf5-journey-seed-amortization, sot:root/production-residual-register, sot:root/product-verification-clean-room, evidence:root/20261110014000, evidence:root/20261110013400]
  evidence: []
files:
  - {path: receipt.json, sha256: c6a158f4162289649d78ae0a5348d639392179279da0379896067cd2a8d5f321, bytes: 7253, store: repo}
  - {path: metrics.json, sha256: 4938a775fc50dcd00763144edb8808e51a813701c1c033c942791a70d304d186, bytes: 228222, store: repo}
---

# G19 sole-runner remeasure — tip `e466d56` (PF5 7/8; A not flipped)

## Context

Authoritative sole Docker `g19` (`--name arks-g19-sole-e466d56`) after product smart-HTTP work: zlib-rs inflate, omit side-band (raw PACK), serial Docker checkout workers, RAM-first same-thread pack+idx persist, and revert of broken cleartext TcpStream HTTP (CGI EOF). Cites [[sot:root/pf5-journey-seed-amortization]] — HTTP seed amortization exhausted; residual **FW-PF5-J-CLONE-LOCAL**. Reconfirms **M17 ≤0.8× 19/19** and **Pf5 aggressive terminal 19/19** with honest **Claimed(partial 7/8)**. **J-CLONE-LOCAL-01** remains `aggressive_na` at ratio **0.6705** — A ≤0.5× not met (do not market).

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
| Tip measured | `e466d56ed0c59bd77df0fae3e255a5ad9a615c46` |

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
| J-CLONE-LOCAL-01 | 88 | 59 | 0.6705 | HTTP seed amortization exhausted; ≤0.5× needs further product smart-HTTP clone work |

## Honesty

- **A (CLONE-LOCAL ≤0.5×):** not flipped — keep `aggressive_na`; do not market ≤0.5× for this journey.
- Cleartext TcpStream client on `e5b957c`/`505ce85` caused CGI `EOF before headers`; reverted on this tip while keeping RAM-first persist + prior zlib-rs/raw-pack/serial-checkout levers.
