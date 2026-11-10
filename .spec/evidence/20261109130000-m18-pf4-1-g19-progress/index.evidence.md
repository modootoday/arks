---
id: evidence:root/20261109130000
title: M18 Pf4-1 progress — G19 sole run (19/19 ≤0.8×; 9/19 ≤0.5×)
status: active
created: 2026-11-09T13:00:00Z
updated: 2026-11-09T22:48:33Z
tags: [g19, m18, pf4, comparative-perf, clean-room, verified, na-free, progress]
rel:
  depends_on: [plan:root/20261109125438, research:root/20261109125438, sot:root/program-roadmap, sot:root/clean-room-journey-compare, evidence:root/20261109090000]
  evidence: []
files:
  - {path: receipt.json, sha256: 9f89a9ee9cb21a4615e67fd0e6050fd56d10467ffdf455c11799bf39f07d2289, bytes: 5241, store: repo}
  - {path: metrics.json, sha256: 53902cba577256c671cd617d90f05fc2fecba48fc231fd7f1d284d4dd0544713, bytes: 51511, store: repo}
---

# M18 Pf4-1 progress — sole Docker G19

Wave **Pf4-1** of [[plan:root/20261109125438]]: hot-path wins (diff `BufWriter`, bulk `add` HashMap stage, untracked sort-once, archive `BufWriter`) measured under sole Docker `g19`.

## Before / after vs M17 exit [[evidence:root/20261109090000]]

| Journey | M17 ratio | Pf4-1 ratio | Δ | ≤0.5× |
|---------|----------:|------------:|---|:-----:|
| J-LOCAL-01 | 0.6774 | 0.6897 | +0.01 | no |
| J-BRANCH-01 | 0.6667 | 0.6667 | 0 | no |
| J-MERGE-FF-01 | 0.3333 | 0.5000 | +0.17 | yes |
| J-REBASE-01 | 0.4211 | 0.4444 | +0.02 | yes |
| J-BUNDLE-01 | 0.6667 | 0.6667 | 0 | no |
| J-CLONE-LOCAL-01 | 0.5370 | 0.5714 | +0.03 | no |
| J-CLONE-PATH-01 | 0.3243 | 0.3243 | 0 | yes |
| J-ALIAS-01 | 0.6452 | 0.6897 | +0.04 | no |
| J-WORKTREE-01 | 0.5833 | 0.6364 | +0.05 | no |
| J-TAG-01 | 0.3678 | 0.3575 | −0.01 | yes |
| J-STASH-01 | 0.5000 | 0.5000 | 0 | yes |
| J-CHERRY-01 | 0.6667 | 0.6667 | 0 | no |
| **J-DIFF-01** | **0.6416** | **0.5669** | **−0.07** | no |
| J-RM-MV-01 | 0.7500 | 0.7500 | 0 | no |
| **J-ARCHIVE-01** | **0.7500** | **0.6667** | **−0.08** | no |
| J-REFLOG-01 | 0.5000 | 0.5000 | 0 | yes |
| J-REVERT-01 | 0.6667 | 0.3333 | −0.33 | yes |
| J-REPACK-01 | 0.3448 | 0.3448 | 0 | yes |
| J-SUBMODULE-01 | 0.1649 | 0.1739 | +0.01 | yes |

Gate exit **0**. Aggressive ≤0.5× hits **9/19**. M17 ≤0.8× regression **19/19** PASS; `na: {}`.

### DIFF step drill (material)

| Step | M17 arks_ms | Pf4-1 arks_ms | git_ms (Pf4-1) |
|------|------------:|--------------:|---------------:|
| `diff` / `diff-2` | 4 / 4 | 2 / 2 | 4 / 4 |
| `add -A` | 32 | 31 | 27 |
| **`diff --cached`** | **23** | **6** | 12 |

## Environment

| Field | Value |
|-------|-------|
| Compose | `docker compose -f verify/docker-compose.yml run --rm -e ARKS_G19_WAVE=Pf4 g19` |
| Image | `arks-verify-m1:0.1.0` / `sha256:5736c5687d3b5775e187ca3d21fbc5b2a5a2bf8965c5381778ee478eb185adf0` |
| Stock Git | `git version 2.39.5` |
| Iterations / aggregation | 3 / `median_p95` |
| Tip at receipt (pre-land) | `23275b5207d9f278e837778fb7fc1aa6723158fb` |

## Residual for Pf4-2

Still above 0.5×: LOCAL/ALIAS, BRANCH/BUNDLE (quantum), CLONE-LOCAL, WORKTREE, CHERRY, DIFF, RM-MV, ARCHIVE. Next wave: further add/object IO and ARCHIVE flatten/read; journey-class targets if quantum-bound.
