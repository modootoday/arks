---
id: plan:root/20261110050000
title: M20 Concurrency Cx-5 claim expansion
status: archived
created: 2026-11-10T05:00:00Z
updated: 2026-11-10T05:20:00Z
tags: [milestone, m20, phase-cx, concurrency, mitigate-with-compat, g017-concurrency-issue-compare, claim-expansion, done]
rel:
  depends_on:
    - sot:root/program-roadmap
    - sot:root/multi-agent-git-concurrency-scenarios
    - research:root/20261110021546
    - plan:root/20261110040000
    - evidence:root/20261110040000
  implements: [sot:root/program-roadmap]
  evidence: [evidence:root/20261110050000]
---

# M20 — Concurrency Cx-5 claim expansion

## Goal

Promote **twelve** further Closed-deferred mitigate IDs (research bucket A) via Docker **G17**. Does **not** claim 221 solved. L3 refuse-closed; CELIR excluded.

## Claimed set — Wave Cx-5

| ID | Oracle intent |
|----|---------------|
| **IDX-004** | Plant `sharedindex.lock` / split-index race → refuse IDX-004 |
| **IDX-011** | Index extension rewrite under `index.lock` refuse IDX-011 |
| **HOOK-003** | `post-index-change` honesty via inspect / add path |
| **MISC-019** | Plant `.gitattributes.lock` → attributes edit refuse MISC-019 |
| **OBJ-001** | Concurrent identical loose create OK; inspect cites OBJ-001 |
| **OBJ-005** | Plant partial/corrupt loose object → read refuse OBJ-005 |
| **PACK-001** | Plant pack writer lock → `repack` refuse PACK-001 |
| **WT-005** | Stale linked `gitdir` after main move → repair/status honesty WT-005 |
| **LOCK-003** | Inspect cites rename-atomicity reader honesty LOCK-003 |
| **MISC-002** | Archive/checkout-index to occupied prefix refuse MISC-002 |
| **MISC-003** | `bundle create` under `HEAD.lock` refuse MISC-003 |
| **SHAL-003** | Partial-clone filter mismatch inspect honesty SHAL-003 |

Done = Docker G17 **71**/71 (12 new + 59 regression).

## Status

**Closed** — Docker G17 **71**/71 dual PASS ([[evidence:root/20261110050000]]). Disposition: Claimed **78** unique + N/A **9** + Closed-deferred **134**. L3 refuse-closed; CELIR excluded. Does **not** mean 221 races solved.
