---
id: evidence:root/20261109083500
title: M17 exit — full G19 allow-list PASS (N/A-free)
status: active
created: 2026-11-09T08:35:00Z
updated: 2026-11-10T00:57:42Z
tags: [g019-comparative-performance, m17, pf3, exit, comparative-perf, clean-room, verified, na-free]
rel:
  depends_on: [plan:root/20261109043916, sot:root/program-roadmap, sot:root/clean-room-journey-compare, evidence:root/20261109064500, evidence:root/20261109070000, evidence:root/20261109073100]
  evidence: []
files:
  - {path: receipt.json, sha256: 5b89f4df950161dc049000dc6ee3a5c906758c174b788738b0c7bfabe70c7c42, bytes: 3934, store: repo}
  - {path: metrics.json, sha256: dfdb1867364211b9b09caa2262d0063ccf83b10484dc9142932a20d2a6be2e4e, bytes: 41872, store: repo}
---

# M17 exit — full G19 allow-list (N/A-free)

Closes milestone **M17** (Phase Pf3) of [[sot:root/program-roadmap]]: G19 allow-list = **all** current G16 journeys with `arks_median ≤ 0.8 × git_median` (`median_p95`, release). **No SoT N/A** for cold-start / timer-floor (supersedes TAG/DIFF N/A in [[evidence:root/20261109073100]]).

## Docker G19 result (authoritative)

| Journey | ratio | pass |
|---------|-------|------|
| J-LOCAL-01 | 0.7241 | yes |
| J-BRANCH-01 | 0.6667 | yes |
| J-MERGE-FF-01 | 0.3333 | yes |
| J-REBASE-01 | 0.4211 | yes |
| J-BUNDLE-01 | 0.6667 | yes |
| J-CLONE-LOCAL-01 | 0.5370 | yes |
| J-CLONE-PATH-01 | 0.3429 | yes |
| J-ALIAS-01 | 0.6897 | yes |
| J-WORKTREE-01 | 0.5833 | yes |
| J-TAG-01 | 0.2963 | yes |
| J-STASH-01 | 0.5000 | yes |
| J-CHERRY-01 | 0.6667 | yes |
| J-DIFF-01 | 0.6102 | yes |
| J-RM-MV-01 | 0.7500 | yes |
| J-ARCHIVE-01 | 0.6667 | yes |
| J-REFLOG-01 | 0.5000 | yes |
| J-REVERT-01 | 0.5714 | yes |
| J-REPACK-01 | 0.2273 | yes |
| J-SUBMODULE-01 | 0.1569 | yes |

`na: {}` in receipt. Gate exit **0**.

## Pf3-2 notes (TAG / DIFF)

- TAG: 60k zero-padded packed-refs; streamed `&str` list + `BufWriter`; four timed `tag` lists.
- DIFF: 320 dirty files × 96 body repeat; size short-circuit skips SHA1 when length differs; timed commit amortization; sampler drain-capped (no OOM).
- Exclusive G19 lock; thin LTO for Docker Desktop link survival.

## Environment

| Field | Value |
|-------|-------|
| Compose | `docker compose -f verify/docker-compose.yml run --rm g019-comparative-performance` |
| Image | `arks-verify-m1:0.1.0` / `sha256:5736c5687d3b5775e187ca3d21fbc5b2a5a2bf8965c5381778ee478eb185adf0` |
| Stock Git | `git version 2.39.5` |
| Iterations / aggregation | 3 / `median_p95` |
| Tip at receipt | `fcba4c5bf4fbecaa0392dd7aa48128ba3ee8e13b` |
| Land tip (this evidence commit) | `6b71368fa33fb822854a9ccabf9c00b26f156281` |

## Reproduce

```bash
docker compose -f verify/docker-compose.yml run --rm g019-comparative-performance
```
