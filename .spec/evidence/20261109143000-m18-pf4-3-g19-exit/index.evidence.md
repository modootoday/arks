---
id: evidence:root/20261109143000
title: M18 Pf4-3 / M18 exit — G19 sole run (19/19 ≤0.8×; 11/19 ≤0.5×; journey-class residuals)
status: active
created: 2026-11-09T14:30:00Z
updated: 2026-11-10T00:57:43Z
tags: [g019-comparative-performance, m18, pf4, comparative-perf, clean-room, verified, na-free, exit]
rel:
  depends_on: [plan:root/20261109125438, research:root/20261109125438, sot:root/program-roadmap, sot:root/clean-room-journey-compare, sot:root/production-residual-register, evidence:root/20261109090000, evidence:root/20261109130000, evidence:root/20261109141500]
  evidence: []
files:
  - {path: receipt.json, sha256: 1f6d6402a81638081c3078c437806623bcb1f16aa2dedf77fe9c9b44c89fe716, bytes: 5007, store: repo}
  - {path: metrics.json, sha256: 77e34486a231a6a77b2378ff494f7bf04d5780b0448690a5c322f162275daf35, bytes: 49470, store: repo}
---

# M18 Pf4-3 / M18 exit — sole Docker G19

Wave **Pf4-3** of [[plan:root/20261109125438]]: parallel checkout collect/materialize (threshold ≥96 jobs) + Pf4-2 engine retained. Closes **M18** via progressive ≤0.5× claim set **plus** journey-class residual register (not catalog-wide ≤0.5×).

## Ratio table (authoritative)

| Journey | git_ms | arks_ms | ratio | ≤0.5× | Class |
|---------|-------:|--------:|------:|:-----:|-------|
| J-LOCAL-01 | 60 | 40 | 0.6667 | no | process-tax |
| J-BRANCH-01 | 6 | 4 | 0.6667 | no | quantum |
| J-MERGE-FF-01 | 4 | 2 | 0.5000 | yes | claimed |
| J-REBASE-01 | 21 | 8 | 0.3810 | yes | claimed |
| J-BUNDLE-01 | 6 | 4 | 0.6667 | no | quantum |
| J-CLONE-LOCAL-01 | 55 | 31 | 0.5636 | no | mid residual |
| J-CLONE-PATH-01 | 37 | 12 | 0.3243 | yes | claimed |
| J-ALIAS-01 | 60 | 40 | 0.6667 | no | process-tax |
| J-WORKTREE-01 | 24 | 14 | 0.5833 | no | mid residual |
| J-TAG-01 | 177 | 64 | 0.3616 | yes | claimed |
| J-STASH-01 | 12 | 8 | 0.6667 | no | quantum-border |
| J-CHERRY-01 | 21 | 6 | 0.2857 | yes | claimed |
| J-DIFF-01 | 199 | 74 | 0.3719 | yes | claimed |
| J-RM-MV-01 | 8 | 6 | 0.7500 | no | quantum |
| J-ARCHIVE-01 | 32 | 16 | 0.5000 | yes | claimed |
| J-REFLOG-01 | 8 | 4 | 0.5000 | yes | claimed |
| J-REVERT-01 | 6 | 2 | 0.3333 | yes | claimed |
| J-REPACK-01 | 29 | 10 | 0.3448 | yes | claimed |
| J-SUBMODULE-01 | 92 | 16 | 0.1739 | yes | claimed |

Gate exit **0**. Aggressive ≤0.5× hits **11/19**. M17 ≤0.8× **19/19** PASS; `na: {}`.

Vs Pf4-1 [[evidence:root/20261109130000]] (9/19) and Pf4-2 [[evidence:root/20261109141500]] (11/19): DIFF/ARCHIVE remain in claim set; CHERRY strong; residuals are journey-class (not engine regressions against ≤0.8×).

## Environment

| Field | Value |
|-------|-------|
| Compose | `docker compose -f verify/docker-compose.yml run --rm -e ARKS_G19_WAVE=Pf4 g019-comparative-performance` |
| Image | `arks-verify-m1:0.1.0` / `sha256:5736c5687d3b5775e187ca3d21fbc5b2a5a2bf8965c5381778ee478eb185adf0` |
| Stock Git | `git version 2.39.5` |
| Iterations / aggregation | 3 / `median_p95` |
| Tip at receipt (pre-land) | `7c349747ebb4e658e425835ccf84b235f3493389` |

## Journey-class residual (M18 exit honesty)

| Class | IDs | Aggressive target |
|-------|-----|-------------------|
| **claimed ≤0.5×** | MERGE-FF, REBASE, CLONE-PATH, TAG, CHERRY, DIFF, ARCHIVE, REFLOG, REVERT, REPACK, SUBMODULE | Must stay ≤0.5× on regression G19 |
| **quantum** | BRANCH, BUNDLE, RM-MV, STASH | ≤0.8× only; timer floor dominates |
| **process-tax** | LOCAL, ALIAS | ≤0.8× only until in-process multi-command SoT opens |
| **mid residual** | CLONE-LOCAL, WORKTREE | ≤0.8×; opportunistic engine wins allowed |

Normative home: [[sot:root/production-residual-register]] + [[plan:root/20261109125438]].
