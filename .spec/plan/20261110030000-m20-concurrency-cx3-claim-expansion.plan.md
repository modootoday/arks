---
id: plan:root/20261110030000
title: M20 Concurrency Cx-3 claim expansion
status: active
created: 2026-11-10T03:00:00Z
updated: 2026-11-10T03:15:00Z
tags: [milestone, m20, phase-cx, concurrency, mitigate-with-compat, g017-concurrency-issue-compare, claim-expansion, issue-resolution, done]
rel:
  depends_on:
    - sot:root/program-roadmap
    - sot:root/multi-agent-git-concurrency-scenarios
    - sot:root/lease-concurrency-intent
    - sot:root/production-residual-register
    - sot:root/interop-test-matrix
    - sot:root/product-verification-clean-room
    - sot:root/git-compat-surface
    - research:root/20261109023117
    - research:root/20261110021546
    - plan:root/20261110010000
    - evidence:root/20261110020000
  implements: [sot:root/program-roadmap]
---

# M20 — Concurrency Cx-3 claim expansion (Phase Cx)

## Goal

Promote **twelve** Closed-deferred **mitigate-with-compat** catalog IDs into an explicit G17 claim-set (Cx-3), selected by [[research:root/20261110021546]] for local filesystem induce-compare fit. Prove each ID with Docker clean-room **G17** (extend harness — do **not** invent folklore G22).

**Honesty:** Does **not** claim the 221 catalog is solved. L3 remains refuse-closed; CELIR excluded; G8/G9 stay Closed-deferred. Does **not** reopen M12/M13/M19 historical exits.

## Primary cites

- https://git-scm.com/docs/api-lockfile
- https://git-scm.com/docs/git-pack-refs
- https://git-scm.com/docs/gitrepository-layout
- https://git-scm.com/docs/git-sparse-checkout
- https://git-scm.com/docs/git-worktree
- https://git-scm.com/docs/git-notes
- https://git-scm.com/docs/git-clone
- https://git-scm.com/docs/git-gc
- https://git-scm.com/docs/git-read-tree

## Claimed set — Wave Cx-3 (G17)

| ID | git pathology oracle (intent) | arks resolved oracle (intent) |
|----|-------------------------------|-------------------------------|
| **REF-012** | Planted `packed-refs.lock` blocks `pack-refs` | Fail-closed; cite REF-012 |
| **REF-016** | Planted `refs/notes/commits.lock` blocks `notes add` | Fail-closed; cite REF-016 |
| **IDX-014** | Planted `index.lock` blocks `read-tree` | Fail-closed; cite IDX-014 |
| **SPARSE-001** | Planted `info/sparse-checkout.lock` blocks `sparse-checkout set` | Fail-closed; cite SPARSE-001 |
| **MISC-018** | Planted `index.lock` blocks `sparse-checkout reapply` | Fail-closed; cite MISC-018 |
| **SUB-008** | `worktree move` with `modules/` present | Refuse SUB-008 (stock: cannot move with submodules) |
| **TOCTOU-007** | Incomplete worktree admin vs prune | Refuse prune of incomplete-in-progress / cite TOCTOU-007 |
| **PACK-009** | Leftover `*.keep` after crash class | `lock inspect` / maint honesty cites PACK-009 |
| **MAINT-002** | Planted `gc.log` suppresses auto-gc class | Inspect/maint honesty cites MAINT-002 |
| **NET-009** | Clone into existing non-empty dest | Refuse NET-009 |
| **CFG-004** | Format v1 + unknown `extensions.*` | Refuse open/ops CFG-004 |
| **MISC-017** | Planted `NOTES_MERGE_*` state | Refuse notes mutate; cite MISC-017 |

Wave Cx-3 **Done** = Docker G17 PASS for **all twelve** + Cm-1..Cm-3 + Cx-1 + Cx-2 regression (**47**/47).

## Cx-3 close

All twelve IDs **PASS**. Aggregate **47**/47 dual PASS. Evidence: [[evidence:root/20261110030000]].

**M20 Cx-3 exit proven.** Next promoteable mitigate IDs remain in research bucket A ([[research:root/20261110021546]]); differentiate in bucket B (Cd-4+); L3 refuse-closed.

## In scope

1. SoT/plan/matrix/roadmap M20; catalog disposition recount; `spec check`.
2. Engine/CLI mitigations for Cx-3 oracles only.
3. Extend `verify/gates/g017-concurrency-issue-compare` + `run-g017-concurrency-issue-compare.sh` + `verify/matrices/matrix-g017-concurrency-issue-compare.md`.
4. Docker G17 receipt + residual honesty.

## Out of scope

- L3 ship; CELIR deps; ≥20% remapping; inventing lock dialects.
- Declaring remaining deferred IDs resolved.
- Cd-4 differentiate wave (scheduled after Cx-3 close via recursive goals).

## Exit gates

| Gate | Clean-room | Advisory |
|------|------------|----------|
| **G17** Cx-3 (12) + prior regression | Authoritative | Host unit |

## Ordered work items

1. Freeze Cx-3 in catalog/matrix/roadmap; research sync.
2. Mitigations + harness tests.
3. Docker G17; evidence; commit; push.
4. CreateGoal next wave from remaining bucket A/B.

## Cross-links

[[sot:root/program-roadmap]], [[sot:root/multi-agent-git-concurrency-scenarios]], [[research:root/20261110021546]], [[plan:root/20261110010000]]
