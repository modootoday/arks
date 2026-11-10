---
id: evidence:root/20261109073100
title: M17 exit — full G19 journey allow-list PASS
status: active
created: 2026-11-09T07:31:00Z
updated: 2026-11-10T00:57:42Z
tags: [g019-comparative-performance, m17, pf3, exit, comparative-perf, clean-room, verified]
rel:
  depends_on: [plan:root/20261109043916, sot:root/program-roadmap, sot:root/clean-room-journey-compare, evidence:root/20261109064500, evidence:root/20261109070000, evidence:root/20261109213000]
  evidence: []
files:
  - {path: receipt.json, sha256: 59f57208ee500ab82ded362d1a94b807854f17b644ae7097ab14050aa80a5178, bytes: 4861, store: repo}
  - {path: metrics.json, sha256: 5d254a8da3c42282be4c827d86108295c53aed842e2762542754a98b25402f4b, bytes: 34848, store: repo}
---

# M17 exit — full G19 journey allow-list

Closes milestone **M17** (Phase Pf3) of [[sot:root/program-roadmap]]: G19 allow-list = **all** current G16 journeys with arks_median ≤ 0.8 × git_median (`median_p95`, release), or SoT N/A with rationale. Does **not** redefine M14 frozen allow-list receipt or M9–M16 exits.

## Docker G19 result (authoritative)

| Journey | ratio | pass |
|---------|-------|------|
| J-LOCAL-01 | 0.6897 | yes |
| J-BRANCH-01 | 0.6667 | yes |
| J-MERGE-FF-01 | 0.3333 | yes |
| J-REBASE-01 | 0.3810 | yes |
| J-BUNDLE-01 | 0.6667 | yes |
| J-CLONE-LOCAL-01 | 0.5254 | yes |
| J-CLONE-PATH-01 | 0.3243 | yes |
| J-ALIAS-01 | 0.6897 | yes |
| J-WORKTREE-01 | 0.6364 | yes |
| J-TAG-01 | N/A | timer-floor (arks==git ~4ms) |
| J-STASH-01 | 0.5000 | yes |
| J-CHERRY-01 | 0.6667 | yes |
| J-DIFF-01 | N/A | timer-floor (arks==git ~8ms) |
| J-RM-MV-01 | 0.7500 | yes |
| J-ARCHIVE-01 | 0.6667 | yes |
| J-REFLOG-01 | 0.5000 | yes |
| J-REVERT-01 | 0.3333 | yes |
| J-REPACK-01 | 0.3030 | yes |
| J-SUBMODULE-01 | 0.1720 | yes |

## Pf3-2 engine / harness (summary)

- Match Git 2.39 `core.fsync` default (no lock fsync when unset); hardlink-or-copy local clone; checkout OID skip; index `ce_match_stat` short-circuit on `add` / ensure_clean.
- Pack zlib `Compression::fast()`; incremental cherry-pick/rebase materialize; streamed packed-refs tag list.
- G16 harness: untimed seeds, bulk amortization, pipe-drain sampler (avoids large-diff deadlock), exclusive G19 lock.
- SoT N/A: **J-TAG-01**, **J-DIFF-01** only — measured timer-floor equality after amortization; G16 functional still required.

## Environment

| Field | Value |
|-------|-------|
| Compose | `docker compose -f verify/docker-compose.yml run --rm g019-comparative-performance` |
| Image | `arks-verify-m1:0.1.0` / `sha256:5736c5687d3b5775e187ca3d21fbc5b2a5a2bf8965c5381778ee478eb185adf0` |
| Stock Git | `git version 2.39.5` |
| Iterations / aggregation | 3 / `median_p95` |
| Tip at receipt (pre-land) | `95aff8a10ab1568afcd612c6d7870f10eb09d5f7` |
| Land tip (this evidence commit parent / pin) | `1ae7da7803338d802e6a153a95436b780aea1bdc` |

## Reproduce

```bash
docker compose -f verify/docker-compose.yml run --rm g019-comparative-performance
spec check
```
