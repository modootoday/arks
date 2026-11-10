---
id: plan:root/20261110040000
title: M20 Concurrency Cx-4 claim expansion
status: archived
created: 2026-11-10T04:00:00Z
updated: 2026-11-10T04:30:00Z
tags: [milestone, m20, phase-cx, concurrency, mitigate-with-compat, g17, claim-expansion]
rel:
  depends_on:
    - sot:root/program-roadmap
    - sot:root/multi-agent-git-concurrency-scenarios
    - research:root/20261110021546
    - plan:root/20261110030000
    - evidence:root/20261110030000
  implements: [sot:root/program-roadmap]
  evidence: [evidence:root/20261110040000]
---

# M20 — Concurrency Cx-4 claim expansion

## Goal

Promote **twelve** additional Closed-deferred mitigate IDs (research bucket A) via Docker **G17**. Does **not** claim 221 solved. L3 refuse-closed; CELIR excluded.

## Claimed set — Wave Cx-4

| ID | Oracle intent |
|----|---------------|
| **REF-009** | Plant reflog/logs lock → update-ref / commit class refuse REF-009 |
| **REF-022** | `for-each-ref` under `packed-refs.lock` remains reader-safe; inspect cites REF-022 |
| **IDX-013** | `update-index --chmod` under `index.lock` refuse IDX-013 |
| **IDX-010** | skip-worktree bit flip under `index.lock` refuse IDX-010 |
| **HOOK-002** | `gc` / inspect cites HOOK-002 pre-auto-gc honesty |
| **HOOK-005** | Nested index write from hook under held lock refuse HOOK-005 |
| **MISC-014** | `reflog expire` under contention refuse MISC-014 |
| **MISC-011** | Mid-flight hooks swap inspect honesty MISC-011 |
| **SHAL-001** | Plant `shallow.lock` → deepen/fetch-class refuse SHAL-001 |
| **WT-006** | Plant `config.worktree.lock` → config refuse WT-006 |
| **SUB-006** | Gitlink vs nested checkout mismatch status honesty SUB-006 |
| **PACK-008** | Plant colliding pack `.tmp` / index-pack lock refuse PACK-008 |

Done = Docker G17 **59**/59 (12 new + 47 regression).

## Status

**Closed** — Docker G17 **59**/59 dual PASS ([[evidence:root/20261110040000]]). Disposition after close: Claimed **66** unique + N/A **9** + Closed-deferred **146**. L3 refuse-closed; CELIR excluded. Does **not** mean 221 races solved.

## Primary cites

https://git-scm.com/docs/api-lockfile · git-update-ref · gitrepository-layout · git-update-index · githooks · git-gc · git-worktree · git-clone / shallow
