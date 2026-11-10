---
id: adr:root/20261110074000
title: M26 shared-index N-way unlock
status: active
created: 2026-11-10T07:40:00Z
updated: 2026-11-10T07:40:00Z
tags: [m26, shared-index, lease, index-lock, unlock, g34, phase-si]
rel:
  depends_on:
    - adr:root/20261109194200
    - adr:root/20261110160000
    - sot:root/lease-concurrency-intent
    - sot:root/arks-lease
    - sot:root/git-compat-surface
    - sot:root/program-roadmap
    - sot:root/production-residual-register
    - sot:root/interop-test-matrix
    - sot:root/product-verification-clean-room
    - research:root/20261108071915
    - research:root/20261109023117
    - evidence:root/20261109194858
  supersedes: [adr:root/20261110160000]
  evidence: [evidence:root/20261110074100, evidence:root/20261110074200]
---

# M26 shared-index N-way unlock

## Context

[[adr:root/20261110160000]] **Refuse-closed** shared-index N-way concurrent writers on one shared Git index as a lasting OUT of the M23 L3 claimed shape. Reopen was permitted **only** via a new milestone ADR whose id is **≠ M24** and whose clean-room gates are **≠ G22 / ≠ G24** (CELIR index contention / L3+CELIR joint must not substitute).

L3 ship remains worktree/ref-shard ([[adr:root/20261109194200]], [[evidence:root/20261109194858]]). M25 owns server transports (**G29–G32**). Highest prior milestone on tip is **M25**; next free id is **M26**. Highest prior additive gate on tip is **G32**; **G33** is reserved for another residual track — this unlock uses **G34**.

Primary Git semantics (no invention):

- Index bytes: [gitformat-index](https://git-scm.com/docs/gitformat-index) ([[research:root/20261108071915]]).
- Exclusive lock: `$GIT_DIR/index.lock` via [api-lockfile](https://git-scm.com/docs/api-lockfile) (`O_CREAT|O_EXCL` → write → `rename`) ([[research:root/20261109023117]]).
- Lease ≠ skip Git locks on Git-compat paths ([[sot:root/lease-concurrency-intent]]).

## Decision

1. **Supersede refuse-closed** for **FW-SHARED-IDX**. Shared-index N-way becomes **claimable** under milestone **M26** (Phase **Si**) — not product-Verified until clean-room **G34** PASS + evidence.
2. **Claimed shape (M26):** N concurrent writers coordinate a **single shared** `$GIT_DIR/index` by:
   - **Product lease** pre-coordination on typed resource `index:shared` (exclusive grant; second holder `AlreadyHeld`).
   - **Git `index.lock` still taken** for every index mutation via `arks-index` lock APIs — lease NEVER authorizes skipping lock bytes.
   - Writers MAY prep entry payloads outside the lock CS; the exclusive apply CS is lease-held + `index.lock` → rename.
3. **Does not rewrite L3.** M23 claimed shape (lease + batch-apply + worktree/ref-shard) stays Claimed and remains the default HighConcurrency story. Shared-index is an **additive** opt-in path under M26, not a silent replacement of worktree/ref-shard.
4. **Forbidden substitutes:** MUST NOT cite **M24**, **G22**, or **G24** as proof of shared-index N-way. MUST NOT pull CELIR ObjectStore. Zero `gix`.
5. **Verification ladder:**
   - **SoT unlock Claimed** — this ADR + plan + residual/SoT amend + `spec check`.
   - **Host advisory** — ≥2-writer shared-index path under `arks-lease` (not Verified).
   - **Product Verified** — sole-runner Docker **G34** `shared-index-contention` only.

## Consequences

- [[sot:root/production-residual-register]] **FW-SHARED-IDX** → **Claimed (SoT unlock)** with honesty **Verified pending G34** until Docker PASS.
- [[sot:root/lease-concurrency-intent]], [[sot:root/arks-lease]], [[sot:root/program-roadmap]], [[sot:root/interop-test-matrix]] amend for M26 / G34.
- [[plan:root/20261110074000]] is the executable wave plan.
- Public copy MUST NOT imply shared-index N-way is Verified before G34 evidence; MAY say it is unlocked / claimable under M26.

## Supersedes / amends

- **Supersedes** [[adr:root/20261110160000]] Refuse-closed (terminal OUT) for FW-SHARED-IDX.
- Does **not** supersede [[adr:root/20261109194200]] L3 worktree/ref-shard ship shape.
- Does **not** reopen CELIR ObjectStore or redefine M24/G22–G25.
