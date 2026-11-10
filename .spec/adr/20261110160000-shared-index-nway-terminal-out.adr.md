---
id: adr:root/20261110160000
title: Shared-index N-way terminal OUT of product L3
status: superseded
created: 2026-11-10T16:00:00Z
updated: 2026-11-10T07:40:00Z
tags: [l3, shared-index, refuse-close, terminal, m23, residual, superseded]
rel:
  depends_on:
    - adr:root/20261109194200
    - adr:root/20261108232800
    - sot:root/lease-concurrency-intent
    - sot:root/arks-lease
    - sot:root/production-residual-register
    - plan:root/20261110150000
    - evidence:root/20261109194858
---

# Shared-index N-way terminal OUT of product L3

## Context

[[adr:root/20261109194200]] ships L3 **HighConcurrency** as lease + parallel prep + ordered batch apply + **worktree/ref-shard isolation**. Concurrent writers against **one shared `index.lock`** as the primary N-way relief path is explicitly **not** claimed. M23 unlock evidence ([[evidence:root/20261109194858]]) does **not** prove shared-index N-way.

Prior residual wording tracked shared-index as **Deferred non-blocking**, which M22 forbids as a lasting work-item disposition ([[plan:root/20261110150000]]). **M24** / **G22–G25** prove CELIR **index** dual-path under leases; they MUST NOT be read as reopening shared-index N-way ([[sot:root/celir-index-dual-path]], [[adr:root/20261109204916]]).

## Decision

1. **Terminal disposition — Refuse-closed (OUT):** Shared-index N-way concurrent writers on one shared Git index is **out of product scope** for the shipped L3 claimed shape and MUST remain **Refuse-closed (terminal)** in residual honesty until a **new milestone ADR** explicitly reopens it.
2. **Not required for L3 Done:** Product L3 Done is the M23 claimed shape only; shared-index N-way is **not** silent debt and **not** a Deferred work queue item.
3. **Reopen boundary:** Reopen ONLY via a superseding ADR whose milestone id is **≠ M24**, **≠ G22**, **≠ G24** index-contention receipts as a substitute claim. Such an ADR MUST cite primary Git index/lock semantics and add clean-room gates before any product claim.
4. **Public copy:** MUST NOT imply shared-index N-way ships or is “coming soon.” Worktree/ref-shard isolation remains the honest concurrency story ([[sot:root/public-surface]]).

## Consequences

- [[sot:root/production-residual-register]] **FW-SHARED-IDX** becomes **Refuse-closed (terminal OUT)** citing this ADR; section B “shared-index” row aligns.
- [[adr:root/20261109194200]] consequence “shared-index N-way stays open/deferred” is **superseded** by this terminal OUT disposition for residual honesty (the ship ADR’s claimed shape is unchanged).
- Agents MUST NOT schedule shared-index implementation under M24/G22/G24 without the reopen ADR above.

## Supersedes / amends

- **Amends** residual deferred language for shared-index N-way under [[adr:root/20261109194200]] §Consequences.
- Does **not** change the M23 claimed shape, G11-L3 receipts, or CELIR index-only boundaries.

## Superseded by

- [[adr:root/20261110074000]] (M26 shared-index N-way unlock; gate **G34**). Historical refuse text retained for audit trail only.
