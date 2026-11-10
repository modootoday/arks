---
id: evidence:root/20261109090000
title: "M17 exit — full G19 allow-list PASS (19/19 measured, N/A-free)"
status: active
created: 2026-11-09T09:00:00Z
updated: 2026-11-10T00:57:42Z
tags: [g19, m17, pf3, exit, comparative-perf, clean-room, verified, na-free]
rel:
  depends_on: [plan:root/20261109043916, sot:root/program-roadmap, sot:root/clean-room-journey-compare, evidence:root/20261109064500, evidence:root/20261109070000, evidence:root/20261109073100, evidence:root/20261109083500]
  evidence: []
files:
  - {path: receipt.json, sha256: 19bebf87981eedf9be71c8f38aa25586899dc16a221336ebcee37c74b542760f, bytes: 3934, store: repo}
  - {path: metrics.json, sha256: 8fbcc1ca270db67af13d4db8b5aa157102b33b4e316875a6758658d5f11ca31f, bytes: 49460, store: repo}
---

# M17 exit — full G19 allow-list (19/19 measured, N/A-free)

Closes milestone **M17** (Phase Pf3) of [[sot:root/program-roadmap]]: G19 allow-list = **all** current G16 journeys with `arks_median ≤ 0.8 × git_median` (`median_p95`, release). **`na: {}`** — no timer-floor / cold-start exclusions (rejects N/A Done claims for TAG/DIFF).

## Docker G19 result (authoritative)

| Journey | git_ms | arks_ms | ratio | pass |
|---------|--------|---------|-------|------|
| J-LOCAL-01 | 62 | 42 | 0.6774 | yes |
| J-BRANCH-01 | 6 | 4 | 0.6667 | yes |
| J-MERGE-FF-01 | 6 | 2 | 0.3333 | yes |
| J-REBASE-01 | 19 | 8 | 0.4211 | yes |
| J-BUNDLE-01 | 6 | 4 | 0.6667 | yes |
| J-CLONE-LOCAL-01 | 54 | 29 | 0.5370 | yes |
| J-CLONE-PATH-01 | 37 | 12 | 0.3243 | yes |
| J-ALIAS-01 | 62 | 40 | 0.6452 | yes |
| J-WORKTREE-01 | 24 | 14 | 0.5833 | yes |
| J-TAG-01 | 174 | 64 | 0.3678 | yes |
| J-STASH-01 | 12 | 6 | 0.5000 | yes |
| J-CHERRY-01 | 12 | 8 | 0.6667 | yes |
| J-DIFF-01 | 173 | 111 | 0.6416 | yes |
| J-RM-MV-01 | 8 | 6 | 0.7500 | yes |
| J-ARCHIVE-01 | 32 | 24 | 0.7500 | yes |
| J-REFLOG-01 | 8 | 4 | 0.5000 | yes |
| J-REVERT-01 | 6 | 4 | 0.6667 | yes |
| J-REPACK-01 | 29 | 10 | 0.3448 | yes |
| J-SUBMODULE-01 | 97 | 16 | 0.1649 | yes |

Gate exit **0**. Container `arks-m17-sole3`.

## Pf3 notes (measured TAG / DIFF / CHERRY / ARCHIVE)

- TAG: 60k zero-padded packed-refs; fold loose annotated tag into packed before timed list; single-write packed-only list path; commit amortization.
- DIFF: 320 tracked dirty files; size short-circuit; timed commit amortization; stdout drain-capped.
- CHERRY: 256-file tip × 128 body (beats 4ms quantum).
- ARCHIVE: 400 files × 64 body; four timed archives.

## Environment

| Field | Value |
|-------|-------|
| Compose | `docker compose -f verify/docker-compose.yml run --rm g19` |
| Image | `arks-verify-m1:0.1.0` / `sha256:5736c5687d3b5775e187ca3d21fbc5b2a5a2bf8965c5381778ee478eb185adf0` |
| Stock Git | `git version 2.39.5` |
| Iterations / aggregation | 3 / `median_p95` |
| Tip at receipt (pre-land) | `704a4b352ceb72f4ffa33413d0cf9d53d5e2737a` |
| Land tip | `bbb9d953d8b9aa90c86a5d033e0acf51e8e781c1` |

## Reproduce

```bash
docker compose -f verify/docker-compose.yml run --rm g19
```
