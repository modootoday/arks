---
id: plan:root/20261109024634
title: M12 Concurrency compat mitigation
status: active
created: 2026-11-09T02:46:34Z
updated: 2026-11-09T14:00:00Z
tags: [milestone, m12, phase-cm, concurrency, mitigate-with-compat, multi-agent, locks, worktree, submodule, g17, issue-resolution]
rel:
  depends_on:
    - sot:root/program-roadmap
    - sot:root/multi-agent-git-concurrency-scenarios
    - sot:root/lease-concurrency-intent
    - sot:root/production-residual-register
    - sot:root/cli-surface
    - sot:root/git-compat-surface
    - sot:root/clean-room-journey-compare
    - sot:root/interop-test-matrix
    - sot:root/product-verification-clean-room
    - research:root/20261109023117
    - plan:root/20261108183812
    - plan:root/20261109022956
    - evidence:root/20261109114207
    - evidence:root/20261108150000
  implements: [sot:root/program-roadmap]
---

# M12 — Concurrency issue-resolution — compat mitigation (Phase Cm)

## Goal

Absorb catalog rows classed **`mitigate-with-compat`** from [[sot:root/multi-agent-git-concurrency-scenarios]] / [[research:root/20261109023117]] into the product program **without** inventing Git lock dialects, **without** shipping L3, and **without** redefining M9/M10/M11 exits.

**Done is not SoT/code claim.** Per [[sot:root/program-roadmap]] Issue-resolution Done rule: each claimed scenario MUST be **reproduced** in Docker clean-room under stock **`git`** (pathology) and under **`arks`** (resolved), comparatively verified — gate **G17**.

## Primary cites (summarize only)

- [api-lockfile](https://git-scm.com/docs/api-lockfile)
- [git-status](https://git-scm.com/docs/git-status) (`--no-optional-locks`)
- [git-update-ref](https://git-scm.com/docs/git-update-ref)
- [git-gc](https://git-scm.com/docs/git-gc)
- [git-worktree](https://git-scm.com/docs/git-worktree)
- [githooks](https://git-scm.com/docs/githooks)
- [gitrepository-layout](https://git-scm.com/docs/gitrepository-layout)

## Catalog → scheduled mapping (Open → M12)

| Rank / theme | Scenario IDs (non-exhaustive) | M12 product intent | Resolved criteria (SoT intent) |
|--------------|-------------------------------|--------------------|--------------------------------|
| Optional-lock readers | IDX-008, AGENT-006 | Agent/profile path: status-class reads without optional locks | git: optional-lock contention / writer block class holds; arks: readers do not take optional locks / writers proceed per claim |
| Stale locks | LOCK-002 (+ related) | Detect/report stale `*.lock`; refuse unsafe delete under live writer | git: stale lock blocks next writer; arks: inspect + safe recovery path meets claim (no silent dual-write) |
| Index / packed-refs honesty | IDX-001 (compat side) | Fail-closed + clear errors; no sharded index writer in M12 | git: exclusive lock serialization / hard fail class; arks: honest fail/retry messaging without dialect invention |
| Worktree / submodule domains | WT-001, SUB-001, AGENT-007 (compat) | Harden fail-closed on claimed matrix (residual Done) | git: multi-lock / composition pathology as induced; arks: claimed fail-closed + shared-store honesty |
| Hook amplification | HOOK-001, HOOK-006 (compat) | Document sync hooks in critical sections; refuse inventing async hooks | git: hook latency × N collapse class when induced; arks: documented refuse/warn per claim |
| auto-gc / prune | PACK-004, MAINT-003, CORR-001 (compat) | Safer refuse-aggressive prune under writers | git: prune/writer race pathology when induced; arks: refuse/safer path per claim |
| Retry herd (compat only) | AGENT-003 (docs/backoff guidance) | Document retry/backoff; no lease scheduler yet | Optional claim; if claimed, git stampede class vs arks guidance path |

Unlisted IDs remain **Open**. Selecting them for a later wave requires amending the claimed set + new G17 receipts.

## Claimed set — Wave Cm-1 (first execution slice)

| ID | git pathology oracle (intent) | arks resolved oracle (intent) |
|----|-------------------------------|-------------------------------|
| **AGENT-006** / **IDX-008** | Status-class optional locks contend with writers | Agent/profile path: no optional locks on claimed reads; writer progress per SoT |
| **LOCK-002** | Stale lock after kill blocks subsequent write | Inspect/report + safe recovery without skipping live-writer locks |
| **WT-001** | Linked worktrees share common store; induced lock-domain fail | Claimed worktree matrix fail-closed / honesty (residual surface) |
| **SUB-001** | Submodule multiplies lock domains under induced concurrency | Claimed submodule matrix fail-closed / honesty |

Wave Cm-1 **Done** = G17 PASS receipt covering **all four** rows above (compose `g17`). Deferred rows stay Open.

## In scope

1. **G17 harness** — Docker clean-room dual-peer issue-repro compare (`verify/` compose service + matrix); evidence schema citing scenario ID + git outcome + arks outcome.
2. SoT accept for M12 exit + Cm-1 claim-set oracles.
3. Engine/CLI mitigations required for Cm-1 resolved oracles (optional-locks profile, stale-lock inspect, WT/SUB harden).
4. Public honesty: mitigations ≠ L3 ≠ ≥20% perf.

## Out of scope

- Redefining M9/M10/M11 exits or rewriting historical receipts.
- **L3** N-way ship ([[adr:root/20261108232800]]).
- Absolute ms / **≥20%** comparative perf (**M14 / G19**) — **later follow-on only after M12+M13 issue-resolution closes**; no parallel perf-opt Done beside this track.
- CELIR; G8 live smoke; differentiate lease grants (**M13 / G18**).
- Declaring unclaimed catalog IDs resolved.

## Depends on

- **M9** — [[evidence:root/20261108150000]].
- Post-M9 worktree/submodule residual **Done** — [[plan:root/20261109022956]], [[evidence:root/20261109114207]] (~`d99b1ba` at expansion).
- Catalog SoT/research active.

## Exit gates

| Gate | Clean-room | Advisory |
|------|------------|----------|
| **G17** for full M12 claimed set (all waves) | Authoritative per-ID PASS | Host unit/integration |
| G14 (+ G16 functional) regression | Must stay green | Host |

Milestone exit = every claimed ID across accepted M12 waves has G17 PASS — not “code merged.”

## Ordered work items

1. **SoT / matrix** — Freeze Cm-1 claim-set oracles; add `verify/matrix-g17.md`; wire compose `g17`; `spec check`. **Done (Cm-1).**
2. **Harness** — Implement induce → git peer → arks peer → compare runner inside clean-room image. **Done** (`verify/g17`, `run-g17.sh`).
3. **Mitigations** — Land code required for Cm-1 resolved oracles only. **Done** (optional-locks profile, `arks lock`, WT/SUB DotGitFile honesty).
4. **Verify** — Docker G17 PASS for all Cm-1 IDs; evidence [[evidence:root/20261109030456]]; residual honesty; G14/G16 regression. **Done (Cm-1).**
5. **Cm-2** — Claim IDX-001, HOOK-001/HOOK-006, PACK-004/MAINT-003, AGENT-003; mitigations + G17; evidence [[evidence:root/20261109123000]]. **Done (Cm-2).**
6. **Cm-3** — Claim IDX-002, AGENT-005, AGENT-007, CORR-001, FS-006 (preferred Open mitigate wave; matches roadmap ranks 1/3/5/7); mitigations + G17; evidence [[evidence:root/20261109140000]]. **Done (Cm-3).**

## Claimed set — Wave Cm-2 (second execution slice)

| ID | git pathology oracle (intent) | arks resolved oracle (intent) |
|----|-------------------------------|-------------------------------|
| **IDX-001** | Exclusive index.lock blocks concurrent writers | Fail-closed with clear exclusive-lock + retry guidance; no sharded index writer dialect |
| **HOOK-001** / **HOOK-006** | Sync hooks in critical sections amplify / storm under N | Refuse inventing async hook runners; --no-verify claimed escape; sync-critical honesty |
| **PACK-004** / **MAINT-003** | auto-gc / detach / pack consolidation under writers | Safer refuse-closed gc/
epack/maintenance (no concurrent prune races) |
| **AGENT-003** | Blind retry herds amplify lock storms | Documented backoff/retry guidance on lock fail + rks lock note; no lease scheduler (M13) |

Wave Cm-2 **Done** = G17 PASS receipt covering **all four** rows above + Cm-1 regression green. Deferred rows stay Open.

## Cm-1 close

| ID | G17 |
|----|-----|
| AGENT-006 / IDX-008 | PASS |
| LOCK-002 | PASS |
| WT-001 | PASS |
| SUB-001 | PASS |

Evidence: [[evidence:root/20261109030456]]. Full M12 exit still requires any later claimed waves.



## Cm-2 close

| ID | G17 |
|----|-----|
| IDX-001 | PASS |
| HOOK-001 / HOOK-006 | PASS |
| PACK-004 / MAINT-003 | PASS |
| AGENT-003 | PASS |

Cm-1 regression re-verified in the same receipt. Evidence: [[evidence:root/20261109123000]]. Full M12 exit still requires any later claimed waves (Cm-3+).

## Claimed set — Wave Cm-3 (third execution slice)

Chosen from remaining Open mitigate IDs preferred by the execution goal (roadmap ranks 1/3/5/7). No alternate wave named in SoT/plan.

| ID | git pathology oracle (intent) | arks resolved oracle (intent) |
|----|-------------------------------|-------------------------------|
| **IDX-002** | Background/optional status refresh holds `index.lock`; foreground writers fail | Default status takes **no** optional locks; writers proceed when no live lock |
| **AGENT-005** | Ephemeral container kill leaves stale lock on volume; next writer blocked | `arks lock inspect` + `clear --stale` recover dead/evicted PID; live refuse; eviction honesty |
| **AGENT-007** | Worktree + submodule composition multiplies lock domains | Combined matrix fail-closed + multi-domain inspect honesty (no invented dialect) |
| **CORR-001** | `gc --prune=now` under writers risks object deletion races | Refuse-closed `gc`/`repack`/`maintenance` citing prune=now corruption class |
| **FS-006** | Bind-mount host+container dual writers; PID liveness can mislead | Inspect warns bind-mount / cross-ns honesty; write path still honors locks (lease ≠ skip) |

Wave Cm-3 **Done** = G17 PASS receipt covering **all five** rows + Cm-1/Cm-2 regression green. Deferred rows stay Open.

## Cm-3 close

| ID | G17 |
|----|-----|
| IDX-002 | PASS |
| AGENT-005 | PASS |
| AGENT-007 | PASS |
| CORR-001 | PASS |
| FS-006 | PASS |

Cm-1/Cm-2 regression re-verified in the same receipt. Evidence: [[evidence:root/20261109140000]]. Full M12 exit still requires any later claimed waves (Cm-4+) if additional Open mitigate IDs are scheduled; otherwise proceed to **M13** differentiate/lease (G18) for remaining differentiate-track rows.

## Non-goals / honesty

- Host greens / SoT prose alone ≠ Done.
- G16 journey timings ≠ G17 issue-resolution; record-only timings do **not** start M14.
- Perf ≥20% is a **later** milestone after M12+M13 — not this plan’s Done.
- CELIR excluded; L3 honesty; G8 deferred.

## Cross-links

[[sot:root/program-roadmap]], [[sot:root/interop-test-matrix]], [[sot:root/multi-agent-git-concurrency-scenarios]], [[research:root/20261109023117]], [[evidence:root/20261109114207]]
