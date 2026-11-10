---
id: evidence:root/20261109141500
title: M18 Pf4-2 progress — G19 sole run (19/19 ≤0.8×; 11/19 ≤0.5×)
status: active
created: 2026-11-09T14:15:00Z
updated: 2026-11-09T22:48:33Z
tags: [g19, m18, pf4, comparative-perf, clean-room, verified, na-free, progress]
rel:
  depends_on: [plan:root/20261109125438, research:root/20261109125438, sot:root/program-roadmap, sot:root/clean-room-journey-compare, evidence:root/20261109090000, evidence:root/20261109130000]
  evidence: []
files:
  - {path: receipt.json, sha256: 58223445bfa0d84d128d7ef752b212e20858a754e398bcd06c6de7e83ed40ee3, bytes: 5242, store: repo}
  - {path: metrics.json, sha256: c70c607a037d6ce45eb0792ee212eb555a700b8d28b500ae32e5c950a39035de, bytes: 51509, store: repo}
---

# M18 Pf4-2 progress — sole Docker G19

Wave **Pf4-2** of [[plan:root/20261109125438]]: parallel bulk `add` loose writes, archive blob prefetch, hot-path loose read (no re-hash), streaming zlib write, Mutex store, checkout dir cache — measured under sole Docker `g19`.

## Before / after vs Pf4-1 [[evidence:root/20261109130000]]

| Journey | Pf4-1 ratio | Pf4-2 ratio | Δ | ≤0.5× |
|---------|------------:|------------:|---|:-----:|
| J-LOCAL-01 | 0.6897 | 0.6897 | 0 | no |
| J-BRANCH-01 | 0.6667 | 0.6667 | 0 | no |
| J-MERGE-FF-01 | 0.5000 | 0.3333 | −0.17 | yes |
| J-REBASE-01 | 0.4444 | 0.3158 | −0.13 | yes |
| J-BUNDLE-01 | 0.6667 | 0.6667 | 0 | no |
| J-CLONE-LOCAL-01 | 0.5714 | 0.6042 | +0.03 | no |
| J-CLONE-PATH-01 | 0.3243 | 0.2927 | −0.03 | yes |
| J-ALIAS-01 | 0.6897 | 0.6897 | 0 | no |
| J-WORKTREE-01 | 0.6364 | 0.5833 | −0.05 | no |
| J-TAG-01 | 0.3575 | 0.3636 | +0.01 | yes |
| J-STASH-01 | 0.5000 | 0.6667 | +0.17 | no |
| **J-CHERRY-01** | **0.6667** | **0.5000** | **−0.17** | **yes** |
| **J-DIFF-01** | **0.5669** | **0.4386** | **−0.13** | **yes** |
| J-RM-MV-01 | 0.7500 | 0.7500 | 0 | no |
| **J-ARCHIVE-01** | **0.6667** | **0.3333** | **−0.33** | **yes** |
| J-REFLOG-01 | 0.5000 | 0.5000 | 0 | yes |
| J-REVERT-01 | 0.3333 | 0.3333 | 0 | yes |
| J-REPACK-01 | 0.3448 | 0.2632 | −0.08 | yes |
| J-SUBMODULE-01 | 0.1739 | 0.1758 | +0.00 | yes |

Gate exit **0**. Aggressive ≤0.5× hits **11/19** (was 9/19). M17 ≤0.8× regression **19/19** PASS; `na: {}`.

### DIFF / ARCHIVE step drill (material)

| Step | Pf4-1 arks_ms | Pf4-2 arks_ms | git_ms (Pf4-2) |
|------|--------------:|--------------:|---------------:|
| DIFF `add -A` | 31 | **17** | 27 |
| DIFF `diff --cached` | 6 | 6 | 12 |
| ARCHIVE ×4 total | 22 | **10** | 30 |

## Environment

| Field | Value |
|-------|-------|
| Compose | `docker compose -f verify/docker-compose.yml run --rm -e ARKS_G19_WAVE=Pf4 g19` |
| Image | `arks-verify-m1:0.1.0` / `sha256:5736c5687d3b5775e187ca3d21fbc5b2a5a2bf8965c5381778ee478eb185adf0` |
| Stock Git | `git version 2.39.5` |
| Iterations / aggregation | 3 / `median_p95` |
| Tip at receipt (pre-land) | `a99b86acb641c631dbb0a60a6e7420a751e3228b` |

## Residual for Pf4-3

Still above 0.5×: LOCAL/ALIAS (process tax), BRANCH/BUNDLE/RM-MV (quantum), CLONE-LOCAL, WORKTREE, STASH. Next wave: clone/checkout further; honest journey-class targets for quantum residuals if seed/timer floor dominates.
