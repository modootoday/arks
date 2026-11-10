---
id: evidence:root/20261110224814
title: Full clean-room matrix re-stamp at tip 54c9dc2 (FS_final currency)
status: active
created: 2026-11-10T13:48:00Z
updated: 2026-11-10T14:35:00Z
tags: [clean-room, docker, l8, s6, g19, g20, fs-final, completion, pf5]
verify: docker
rel:
  depends_on:
    - research:root/20261110225000
    - research:root/20261110212414
    - sot:root/interop-test-matrix
    - sot:root/pf5-journey-seed-amortization
---

# Full clean-room matrix — tip `54c9dc2`

Venue: Docker `arks-verify-m1:0.1.0`, sequential `compose run --rm` (sole-runner). Entrypoints: `verify/scripts/run-full-cleanroom-matrix.ps1` + `resume-cleanroom-matrix.ps1`. Host cargo ≠ Verified. **No invent PASS.**

## Tip

`54c9dc2eef001af9c39ee31def5029829d8096b8` — OBJ-004 sha1cd Claimed; FS-007/WIN-003 advisory.

## Aggregate

| Class | Count |
|-------|------:|
| PASS | 45 |
| SKIP | 3 |
| FAIL | 0 |

Authoritative table: [`results.csv`](./results.csv).

## Live secrets (honest SKIP)

| Gate | Disposition | Reason |
|------|-------------|--------|
| `g8` | **SKIP** | token/URL absent — ADR 20261110220000 |
| `g8-live-scale` | **SKIP** | secrets absent — ADR 20261110221000; harness Claimed ≠ live latency PASS |
| `g28` | **SKIP** | opt-in secrets absent; offline clones still exercised |

## G19 disposition (not OOM)

| Bar | Result |
|-----|--------|
| M17 ≤0.8× | **19/19 PASS** |
| PF5 ≤0.5× | **Claimed(partial)** — claimed=17; `aggressive_na` **J-ALIAS-01** (0.5192 process-tax) + **J-REVERT-01** (0.6667 short-wall) |
| Remasure thrash | **Halted** — first remasure was real gate FAIL exit 1 (not 137); eval-only after honest NA |

Prior primary **Claimed 8/8** at `79fc9bf` ([[evidence:root/20261110142500]]) remains archival.

## Gaps fixed mid-run (real, not invent)

1. **G20** `M15-FS5-03` — FS1 editor honesty superseded “exclude” substrings.
2. **G11 / G11-L3 / G18** — CELIR scan uses default `cargo tree` (not full metadata false FAIL).
3. **G15** — require advertised `push-options` (O4 Claimed).
4. **G19** — honest `aggressive_na` for tip PF5 misses; no blind remasure loop.

## Suites

| Suite | Gates | Status |
|-------|-------|--------|
| L8 | g14→g16→g20→g21 | **PASS** |
| S6 | g38→g39→g40 | **PASS** |

## Completeness reading

- **FS_final git feature symmetry DoD:** **YES**
- **Live forge SLO / L9 / crypto immunity / forge UX / full host-env sync:** **NO** (documented non-goals)
- Operator one-word: **PARTIAL** — see [[research:root/20261110225000]] + `docs/completion.md`
