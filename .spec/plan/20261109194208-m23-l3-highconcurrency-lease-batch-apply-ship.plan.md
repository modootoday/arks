---
id: plan:root/20261109194208
title: M23 L3 HighConcurrency lease batch-apply ship
status: archived
created: 2026-11-09T19:42:08Z
updated: 2026-11-09T19:50:00Z
tags: [milestone, m23, l3, high-concurrency, lease, batch-apply, g011-lease-product, clean-room]
rel:
  depends_on:
    - adr:root/20261109194200
    - sot:root/lease-concurrency-intent
    - sot:root/arks-lease
    - sot:root/program-roadmap
    - sot:root/production-residual-register
    - sot:root/product-verification-clean-room
    - sot:root/interop-test-matrix
    - research:root/20261108083502
    - plan:root/20261110150000
    - adr:root/20261108232800
  implements: [adr:root/20261109194200, sot:root/lease-concurrency-intent, sot:root/arks-lease]
---

# M23 — L3 HighConcurrency ship (lease + batch-apply + worktree/ref-shard)

## Goal (binding Done bar)

Ship the **claimed L3 shape** locked by [[adr:root/20261109194200]]:

**lease pre-coordinate → parallel blob/tree prep → short ordered Git-lock batch apply → release → tip verify**, with **worktree/ref-shard isolation**. **Shared-index N-way OUT.**

**SUCCESS** only when all hold:

1. Superseding ADR active; SoT/roadmap/residual updated; `spec check` clean.
2. Induce bar: matrix + scripts for L3 stress (N tens→hundreds topology); pathology (GitCompat serial lock storm / refuse) vs resolved (HighConcurrency claimed shape) criteria documented.
3. Implementation behind valve: GitCompat L0+L1/L2 unchanged; HighConcurrency pipeline API + CLI surface; lease ≠ skip locks.
4. Unlock: Docker clean-room dual-path (GitCompat + HighConcurrency + export/import honesty) green; remove `HighConcurrencyRefused` only with receipts; evidence stamped; honesty docs.
5. Commit+push each Done wave (no force). CELIR excluded. No fake greens.

**FORBIDDEN:** refuse-only removal; CELIR; skip Git locks; claim shared-index N-way; host-only “verified.”

## Baseline (post M22)

| Item | State |
|------|-------|
| L3 | Refuse-closed terminal ([[adr:root/20261108232800]], M22 residual) |
| G11 | L1→L2 + dual-path refuse ([[evidence:root/20261108231249]]) |
| Tip | ~`2c94eba` on `origin/main` |

## Claimed shape (non-negotiable)

| Element | Normative |
|---------|-----------|
| Prep | Parallel object prep outside exclusive lock CS |
| Lease | Grant on shard resource before apply |
| Apply | Ordered batch tip/index writes under short Git locks |
| Isolation | Distinct `refns:` / `domain:worktree:` shards |
| Out | Shared-index N-way (separate ADR required) |
| Default | `WriterMode::GitCompat` |

## Done rules

| Disposition | Allowed when |
|-------------|--------------|
| **Closed (Claimed)** | ADR + impl + Docker dual-path receipts; refuse removed |
| **Hard-block ADR** | Explicit superseding refuse/re-scope if ship blocked |
| **Host green alone** | Advisory only — not Done |

## Waves

| Wave | Focus | Done |
|------|-------|------|
| **Hc-0** | Design lock: ADR active + this plan + SoT/roadmap/residual | `spec check`; commit+push |
| **Hc-1** | Induce bar: `verify/matrices/matrix-l003-high-concurrency.md` (+ G11 expansion / stress harness); pathology vs resolved | Scripts + matrix; commit+push |
| **Hc-2** | Implement behind valve (pipeline + isolation); unit/advisory tests; refuse still on until unlock | Host tests advisory; commit+push |
| **Hc-3** | Unlock claim: Docker dual-path green → remove refuse; evidence; honesty | Evidence id; residual Claimed; commit+push |
| **Hc-exit** | Recount; L3 Claimed or hard-block ADR | Final push |

## Out of scope

- CELIR product deps; `gix` reintroduction.
- Shared-index N-way claim.
- Redefining M9–M22 historical exits / carrying receipts.
- Marketing “faster than Git” outside receipts; “all races fixed.”

## Status

**Closed (Claimed)** — Hc-0 `126e9fa`; Hc-1 `e87e7c2`; Hc-2 `dd2712b`; Hc-3 unlock [[evidence:root/20261109194858]] (Docker g011-lease-productl3 PASS; HighConcurrency opt-in accepted; shared-index N-way OUT).
