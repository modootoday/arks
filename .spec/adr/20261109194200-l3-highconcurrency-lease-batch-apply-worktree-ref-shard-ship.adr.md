---
id: adr:root/20261109194200
title: L3 HighConcurrency lease batch-apply worktree-ref-shard ship shape
status: active
created: 2026-11-09T19:42:00Z
updated: 2026-11-09T19:43:05Z
tags: [m23, lease, l3, high-concurrency, batch-apply, worktree, ref-shard, dual-path]
rel:
  depends_on: [sot:root/lease-concurrency-intent, sot:root/arks-lease, sot:root/git-compat-surface, sot:root/storage-celir-boundary, sot:root/product-verification-clean-room, sot:root/program-roadmap, research:root/20261108083502, adr:root/20261109122126, adr:root/20261108083507, plan:root/20261110150000]
  supersedes: [adr:root/20261108232800]
  evidence: [evidence:root/20261109194858]
---

# L3 HighConcurrency lease batch-apply worktree-ref-shard ship shape

## Context

[[adr:root/20261108232800]] **refuse-closed** L3 for M10/M22 production bars: `WriterMode::HighConcurrency` MUST refuse; G11 proved L1→L2 + valve only. M22 recorded L3 as **Refuse-closed (terminal)** until a **superseding ADR** + clean-room expansion ([[plan:root/20261110150000]], [[sot:root/production-residual-register]]).

[[sot:root/lease-concurrency-intent]] left open: lock sharding vs worktree-per-agent vs lease+batch vs dual mode. [[research:root/20261108083502]] principles: pre-coordinate with leases; shrink exclusive critical sections (parallel prep + short locked tip update); batch when possible; explicit product mode if layout diverges; no CELIR; lease ≠ skip Git locks on Git-compat paths.

M23 opens a **ship track** for one **claimed shape** only. Shared-index N-way remains **out** of this claim.

## Decision

1. **Claimed L3 shape (normative).** Opt-in `WriterMode::HighConcurrency` ships as:
   - **Lease pre-coordination** (L1/L2 additive grants on typed resources: `refns:…`, `domain:worktree:…`, related scopes).
   - **Parallel prep** of blob/tree (and commit payload) bytes **outside** exclusive Git lock critical sections.
   - **Ordered batch apply** under a **short** Git lock critical section: acquire documented ref `*.lock` (and index lock only when that shard’s index is touched) in deterministic order; write tips; release locks.
   - **Worktree / ref-shard isolation**: concurrent agents target **distinct** ref namespaces and/or worktree domains; they MUST NOT contend on one shared index as the N-way path.
2. **Shared-index N-way — deferred / OUT.** Concurrent writers against one shared `index.lock` as the primary N-way relief path is **not** claimed. A future ADR MAY reopen it; this ADR forbids marketing or removing refuse for that shape.
3. **Valve.** `high-concurrency` Cargo feature and/or `WriterMode::HighConcurrency` remain the explicit opt-in. Default remains `WriterMode::GitCompat` (L0 + L1/L2). No silent fallback from HighConcurrency to GitCompat on failure of the pipeline API.
4. **Git locks still required.** HighConcurrency MAY shrink the lock window via prep+batch, but MUST still take Git lock bytes for tip/index mutations that stock Git would lock. Lease grant alone NEVER authorizes skipping `*.lock` / `index.lock`.
5. **Dual-path clean-room Done.** Removing `HighConcurrencyRefused` requires Docker clean-room receipts proving:
   - **GitCompat** path unchanged (L0 locks + L1/L2 additive; prior G11 regress).
   - **HighConcurrency** claimed shape: N writers (tens→hundreds topology in stress matrix) with parallel prep + short ordered batch apply + tip verify; worktree/ref-shard isolation holds.
   - **Export/import** when product-private staging diverges from stock layout; if the claimed apply writes stock-readable refs/objects only, export MAY be identity (document honesty).
6. **CELIR / engine.** Zero CELIR Cargo edges ([[sot:root/storage-celir-boundary]]). Native engine only ([[adr:root/20261108083507]]). Allocator helpers remain ring-internal ([[adr:root/20261109122126]]).
7. **Supersession.** This ADR **supersedes** the refuse-close ship prohibition of [[adr:root/20261108232800]] **for the claimed shape above**, contingent on M23 plan waves + clean-room evidence. Until evidence is stamped, runtime MAY keep refuse during implement-behind-valve waves; unlock wave removes refuse only with receipts. Historical M10 G11 L1→L2 evidence is **not** rewritten.

## Consequences

- [[plan:root/20261109194208]] (M23) is the executable wave plan.
- [[sot:root/lease-concurrency-intent]] / [[sot:root/arks-lease]] close open questions (1)–(3)/(5) for the claimed shape; shared-index N-way is unlocked separately under [[adr:root/20261110074000]] (**M26** / **G34**) — still OUT of this L3 claim.
- [[sot:root/program-roadmap]] adds **M23**; residual register moves L3 from “Refuse-closed terminal” to **M23 active ship track** (Claimed only after unlock evidence).
- Implementers MUST NOT remove `HighConcurrencyRefused` in a design-only or induce-only wave.
- Public copy MUST NOT claim “L3 verified” or “faster than Git” without clean-room receipts.

## Supersedes / amends

- **Supersedes** [[adr:root/20261108232800]] refuse-close **as the lasting ship prohibition** for the **lease + batch-apply + worktree/ref-shard** shape.
- Does **not** claim shared-index N-way.
- Does **not** amend M10 L1→L2 carrying receipts or allocator ring-internal identity.
