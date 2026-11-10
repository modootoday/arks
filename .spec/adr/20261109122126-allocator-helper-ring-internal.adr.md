---
id: adr:root/20261109122126
title: Allocator helper identity ring-internal
status: active
created: 2026-11-09T12:21:26Z
updated: 2026-11-09T12:21:26Z
tags: [allocator, crate-architecture, l3, residual-close]
rel:
  depends_on:
    - sot:root/crate-architecture
    - sot:root/lease-concurrency-intent
    - sot:root/arks-lease
    - sot:root/product-intent
    - research:root/20261108083502
    - adr:root/20261108232800
    - adr:root/20261108083507
  evidence: []
---

# Allocator helper identity ring-internal

## Context

[[adr:root/20261108232800]] left **allocator helper crate placement** as an explicit **Deferred Open**: arena/pool/zero-copy helpers might live in a dedicated workspace crate **or** as ring-internal modules. [[research:root/20261108083502]] records principles only; [[sot:root/crate-architecture]] forbids inventing a new `arks-*` package for every helper and keeps Rings 0–4 free of outer→inner cycles and CELIR edges.

L3 N-way remains **refuse-closed** ([[adr:root/20261108232800]]); no product path currently needs a shipped allocator API. Residual register / program-roadmap still listed the identity as **Open**, blocking an honest “development milestones finished” close.

## Decision

1. **No dedicated allocator helper crate** for the current development ladder (M0–M17 residual close). Do **not** add `arks-alloc` (or any synonym) as a workspace member until a **future** ADR that also claims L3 (or another measured hot-path) and updates [[sot:root/crate-architecture]] rings.
2. **Identity = ring-internal modules.** When arena/pool/zero-copy helpers are introduced, they MUST live as modules inside the consuming Ring **0–4** crate(s) that need them (or shared only via equal-or-inner edges). Shared helpers MUST NOT create outer→inner cycles, MUST NOT depend on `gix`/gitoxide, and MUST NOT invent CELIR APIs ([[adr:root/20261108083507]], [[sot:root/storage-celir-boundary]]).
3. **Principles unchanged.** Follow [[research:root/20261108083502]] (hot-path arenas/pools, minimize cross-thread allocator traffic, zero-copy where safe, measurable claims). This ADR closes **placement identity** only — it does **not** ship L3, claim allocator performance, or authorize skipping Git locks on Git-compat paths.
4. **Supersedes deferral.** The “allocator helper crate placement — deferred Open” clause in [[adr:root/20261108232800]] §Decision (5) is **closed** by this decision for the residual register. A future dedicated-crate ADR MAY reopen placement only with ring-graph + clean-room evidence.

## Consequences

- [[sot:root/crate-architecture]], [[sot:root/lease-concurrency-intent]], [[sot:root/arks-lease]], [[sot:root/program-roadmap]], and [[sot:root/production-residual-register]] MUST record allocator identity as **Closed (ring-internal; no dedicated crate)**.
- Implementers MUST NOT invent an allocator workspace crate “for completeness” while L3 is refuse-closed.
- Host unit greens that touch future arena helpers remain advisory; product allocator claims still require clean-room evidence when advertised.
