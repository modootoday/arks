---
id: plan:root/20261109024635
title: M13 Concurrency differentiate lease uplift
status: active
created: 2026-11-09T02:46:35Z
updated: 2026-11-09T04:05:00Z
tags: [milestone, m13, phase-cd, concurrency, differentiate-beyond-git, lease, multi-agent, hooks, locks, g18, issue-resolution]
rel:
  depends_on:
    - sot:root/program-roadmap
    - sot:root/multi-agent-git-concurrency-scenarios
    - sot:root/lease-concurrency-intent
    - sot:root/arks-lease
    - sot:root/interop-test-matrix
    - sot:root/product-verification-clean-room
    - adr:root/20261108232800
    - adr:root/20261109032955
    - research:root/20261109023117
    - research:root/20261108083502
    - plan:root/20261108183813
    - plan:root/20261109024634
    - evidence:root/20261108231249
    - evidence:root/20261109140000
  implements: [sot:root/program-roadmap]
---

# M13 — Concurrency issue-resolution — differentiate / lease uplift (Phase Cd)

## Goal

Absorb top **`differentiate-beyond-git`** catalog classes into **product lease grant shapes and coordination** after M10 L1→L2 and M12 compat mitigations — still **additive** to Git locks on Git-compat paths, still **L3 refuse-closed** unless a **superseding ADR** + G11 expansion ([[adr:root/20261108232800]]).

**Done is not SoT/code claim.** Gate **G18**: for each claimed scenario ID, Docker clean-room must reproduce stock **`git`** pathology and prove **`arks`** meets SoT resolved criteria (Issue-resolution Done rule in [[sot:root/program-roadmap]]).

## Catalog → scheduled mapping (Open → M13)

| Rank / theme | Scenario IDs (non-exhaustive) | M13 product intent |
|--------------|-------------------------------|--------------------|
| Index write storms | IDX-001, IDX-002, IDX-008, AGENT-006 | Lease path-set / index-writer grants; coordinate before `index.lock` — **lease ≠ skip lock** |
| Ref tip CAS / namespaces | REF-001, REF-006, REF-011, NET-002, AGENT-002 | Ref-namespace leases; agent branch scheduler hooks |
| Stale locks / eviction | LOCK-002, AGENT-005, FS-006 | Lease fencing + stale-lock reclaim for dead agents |
| Retry herds | PERF-001, PERF-006, AGENT-003 | Coordinated backoff / lease wait queues (not G19 perf claim) |
| Worktree + submodule domains | WT-001, SUB-001, AGENT-007 | Cross-domain lease composition |
| Hook amplification | HOOK-001, HOOK-006, AGENT-008 | Lease-aware hook budgets |
| Commit / blackboard races | AGENT-001, SEQ-005, MISC-004 | Shared-worktree / sequencer exclusion leases |
| Maint / prune under writers | PACK-004, MAINT-003, CORR-001, OBJ-003 | Writer–gc coordination leases |
| Fetch stampedes | NET-001, NET-007, SHAL-002 | Client-side lease/queue (G8 still deferred) |
| L3 N-way pipeline | catalog + allocator research | Remains **opt-in / refuse-closed** until superseding ADR |

Claimed sets per wave must list explicit IDs; unlisted stay Open. Slice Done = G18 PASS for every ID in that wave’s set.

## In scope

1. Lease grant schema uplift (ADR before on-disk format claim).
2. **G18** harness (extends G17 protocol with lease-enabled arks peer).
3. Dual-path honesty; CELIR excluded; allocator placement deferred Open unless ADR closes.

## Out of scope

- Shipping **L3** without superseding ADR + expanded G11.
- Redefining M9/M10/M11; treating **≥20%** / perf-opt as this gate or as parallel Done (**M14/G19** only **after** M12+M13); inventing lock dialects; G8; host-only Done.

## Depends on

- **M10** — [[evidence:root/20261108231249]].
- **M12** — G17 green for M12 claimed set.
- Catalog [[sot:root/multi-agent-git-concurrency-scenarios]] (221 scenarios).

## Exit gates

| Gate | Clean-room | Advisory |
|------|------------|----------|
| **G18** for full M13 claimed set | Authoritative per-ID PASS | Host |
| G11 + G17 + G14 regression | Must stay green | Host |

## Ordered work items

1. **Design** — ADR(s) for grant schema + fencing; freeze first Cd wave claim-set. **Done (Cd-1)** — [[adr:root/20261109032955]]; matrix `verify/matrices/matrix-g18.md`.
2. **Implement** — `arks-lease` uplift + `arks lease` CLI + G18 runner (`verify/gates/g18`, `run-g18.sh`, compose `g18`).
3. **Verify** — Docker G18 PASS per claimed ID; evidence receipts; no L3 overclaim.

## Claimed set — Wave Cd-1 (first execution slice)

Chosen from top differentiate track not closed under M12 mitigate-only: REF tip CAS / namespaces (roadmap rank 2) + retry-herd lease coordination (rank 4 AGENT-003 uplift beyond docs).

| ID | git pathology oracle (intent) | arks resolved oracle (intent) |
|----|-------------------------------|-------------------------------|
| **REF-001** | Per-ref loose `.lock` contention blocks tip writers | Exclusive `refns:` lease; non-holder AlreadyHeld; holder may update-ref; Git locks still honored |
| **REF-006** | Tip CAS / expected-OID race class fails under dual writers | Lease serializes tip writers before CAS; loser wait honesty (not blind CAS herd) |
| **REF-011** | Unpartitioned agent tmp/ref namespaces collide | Partitioned `refns:refs/heads/agents/<id>/` leases; overlapping prefix AlreadyHeld |
| **AGENT-003** | Blind retry herds amplify lock storms | Durable lease grant + AlreadyHeld wait/backoff (M13 scheduler surface); write path honors live locks |

Wave Cd-1 **Done** = G18 PASS receipt covering **all four** rows. Deferred differentiate rows stay Open for Cd-2+.

## Cd-1 close

| ID | G18 |
|----|-----|
| REF-001 | PASS |
| REF-006 | PASS |
| REF-011 | PASS |
| AGENT-003 | PASS |

Evidence: [[evidence:root/20261109153000]]. Full M13 exit still requires later Cd waves for remaining differentiate-track rows.

## Claimed set — Wave Cd-2 (second execution slice)

Lease fencing + WT/SUB cross-domain leases (ranks 3/5; differentiate uplift beyond M12 mitigate-only).

| ID | git pathology oracle (intent) | arks resolved oracle (intent) |
|----|-------------------------------|-------------------------------|
| **LOCK-002** | Stale lock blocks writers | TTL fencing via `lease expire`; next holder grants; Git locks still honored |
| **WT-001** | Common-dir lock fails linked-wt writers | Exclusive `domain:worktree:` lease + lock honesty |
| **SUB-001** | Submodule lock-domain blocks writers | Exclusive `domain:submodule:` lease + lock honesty |
| **AGENT-007** | Multi-domain composition fails | Concurrent distinct domain leases; inspect composition; overlap AlreadyHeld |

Wave Cd-2 **Done** = G18 PASS for **all four** rows + Cd-1 regression green.

## Cd-2 close

| ID | G18 |
|----|-----|
| LOCK-002 | PASS |
| WT-001 | PASS |
| SUB-001 | PASS |
| AGENT-007 | PASS |

Cd-1 regression re-verified in the same receipt. Evidence: [[evidence:root/20261109160000]]. Full M13 exit still requires later Cd waves for remaining differentiate-track rows.

## Claimed set — Wave Cd-3 (third execution slice)

| ID | git pathology oracle (intent) | arks resolved oracle (intent) |
|----|-------------------------------|-------------------------------|
| **HOOK-001** / **HOOK-006** | Sync hooks amplify critical sections | Exclusive `hook:critical` lease; no async hook invention |
| **PACK-004** / **MAINT-003** | gc/maint under writers | Exclusive `maint:gc` lease; `arks gc` refuse-closed |
| **AGENT-002** | Fan-in without merge coordinator | Agent `refns` partitions + main merge lease |
| **IDX-001** | Exclusive index.lock writer storm | `index:paths:default` path-set lease; locks honored |

Wave Cd-3 **Done** = G18 PASS for **all four** rows + Cd-1/Cd-2 regression green.

## Cd-3 close

| ID | G18 |
|----|-----|
| HOOK-001/HOOK-006 | PASS |
| PACK-004/MAINT-003 | PASS |
| AGENT-002 | PASS |
| IDX-001 | PASS |

Cd-1/Cd-2 regression re-verified. Evidence: [[evidence:root/20261109163000]].

## M13 claimed-set exit

**Claimed set** = Cd-1 ∪ Cd-2 ∪ Cd-3 (12 G18 scenario rows). All PASS with receipts [[evidence:root/20261109153000]], [[evidence:root/20261109160000]], [[evidence:root/20261109163000]].

Unclaimed residual (**Closed-deferred out-of-milestone**, not blocking this exit or development-ladder Done): NET fetch queues, AGENT-001/SEQ blackboard, PERF-001 batch beyond lease wait, L3 N-way (refuse-closed). New claim-set plan + G17/G18 required before “issue resolved.” **M14 / G19** may proceed after this exit. See [[sot:root/production-residual-register]].

## Cross-links

[[sot:root/program-roadmap]], [[sot:root/interop-test-matrix]], [[sot:root/lease-concurrency-intent]], [[adr:root/20261108232800]], [[adr:root/20261109032955]], [[plan:root/20261109024634]]
