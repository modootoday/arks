---
id: adr:root/20261108232800
title: L3 high-concurrency opt-in refuse-close for production bars
status: superseded
created: 2026-11-08T23:28:00+09:00
updated: 2026-11-09T19:43:05Z
tags: [m10, lease, l3, high-concurrency, refuse-close, dual-path, allocator-defer]
rel:
  depends_on: [sot:root/lease-concurrency-intent, sot:root/arks-lease, plan:root/20261108183813, adr:root/20261108083507]
  evidence: [evidence:root/20261108231249]
---

# L3 high-concurrency opt-in refuse-close for production bars

## Context

Milestone **M10** production exit for **default product leases** is proven by clean-room **G11** ([[evidence:root/20261108231249]]): **L1** (in-process grant/release/expire) and **L2** (durable on-disk lease state under `.git/arks/leases/`) ship as the **default product capability** per [[sot:root/arks-lease]] and [[plan:root/20261108183813]].

[[sot:root/lease-concurrency-intent]] locks **dual-path** behavior: **Git-compat mode** (default interchange; Git lock bytes honored) versus **high-concurrency product mode** (opt-in / selectable; N-way pipeline aimed at N≈hundreds). The opt-in valve (`high-concurrency` feature and/or `WriterMode`) MUST exist so callers cannot silently enter an unimplemented N-way path.

Lane 3 residual close must **not** invent or ship an N-way commit pipeline in this slice. Allocator helper **crate placement** remains principles-only ([[research:root/20261108083502]]); no product crate may be named or wired here.

## Decision

1. **Refuse-close — L3 product claim for current M10 production bars.** The **N-way high-concurrency commit pipeline** is **NOT** claimed as shipped for M10 production. **G11** scope in [[evidence:root/20261108231249]] covers **L1→L2** and dual-path **valve** behavior only, not a working N-way writer.
2. **Runtime requirement until a future claim.** `WriterMode::HighConcurrency`, the `high-concurrency` Cargo feature, and any equivalent runtime mode selector **MUST** continue to **refuse** with a **clear, explicit error** (no silent fallback, no partial N-way). Removing refuse behavior or claiming L3 “verified” requires a **new ADR** plus an expanded clean-room **G11** (or successor gate) that proves the pipeline and export/import rules when divergent.
3. **Dual-path default unchanged.** **Git-compat** / `WriterMode::GitCompat` remains the default product writer path. Product leases (L1/L2) **pre-coordinate** agents but **MUST NOT** skip Git ref/index lock bytes on Git-compat paths ([[sot:root/lease-concurrency-intent]]).
4. **Opt-in valve remains.** The feature flag and mode enum **stay** in the product surface so high-concurrency can be claimed later without breaking API stability — but **enablement without implementation** is forbidden; refuse-close is normative until decision (2) is superseded.
5. **Allocator helper crate placement — deferred Open (explicit) at time of this ADR.** Where arena/pool/zero-copy helpers live (**dedicated crate vs ring-internal modules**) was **not** decided here. **Superseded for placement identity** by [[adr:root/20261109122126]] (ring-internal; no dedicated crate). Principles remain [[research:root/20261108083502]].

## Consequences

- [[sot:root/arks-lease]] L3 row is **stub + refuse**, not pipeline bytes, for M10 production marketing and “verified” claims.
- [[plan:root/20261108183813]] open questions for sharding, worktree-per-agent, batch commit, and CLI valve remain **Open**; L3 **shipping** is **refuse-closed** here, not silently omitted.
- Implementation MUST NOT land N-way pipeline logic under the `high-concurrency` feature in this residual close.
- Future L3 work: dedicated research/ADR → implementation behind opt-in → clean-room gate expansion; zero `gix` / zero CELIR per [[adr:root/20261108083507]].

## Supersedes / amends

- Clarifies and **closes** (as refuse) the M10 “L3 as claimed ship” ambiguity left open in [[sot:root/arks-lease]] rule 4 and the L3 table note (“until ADR closes remaining Opens”).
- Does **not** amend L1/L2 default lease capability or [[evidence:root/20261108231249]] receipts for L1→L2.
