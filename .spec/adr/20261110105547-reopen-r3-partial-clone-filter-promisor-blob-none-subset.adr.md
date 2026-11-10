---
id: adr:root/20261110105547
title: Reopen R3 — partial-clone filter/promisor (blob:none subset)
status: active
created: 2026-11-10T10:55:47Z
updated: 2026-11-10T10:55:47Z
tags: [r3, partial-clone, filter, promisor, blob-none, symmetry, s1, arks-remote]
rel:
  depends_on:
    - sot:root/arks-remote
    - sot:root/production-residual-register
    - research:root/20261110105436
    - sot:root/interop-test-matrix
    - research:root/20261110095142
  supersedes: []
  evidence: [evidence:root/20261110105855]
---

# Reopen R3 — partial-clone filter/promisor (`blob:none` subset)

## Context

Residual **R3** / historical ladder **L6** closed filter/promisor as honest **N/A** (CLI refuse `--filter`; **≠** Claimed). Expanded symmetry DoD [[research:root/20261110105436]] **S1** reopens the implementable protocol gap: pack-protocol `filter` + promisor remotes per [partial-clone](https://git-scm.com/docs/partial-clone).

## Decision

Amend [[sot:root/arks-remote]] to claim an additive **partial-clone subset**:

1. **Filter-spec:** `--filter=blob:none` only on pack transports (smart HTTP, SSH, `git://`). Other filter-specs remain refuse-closed until separately claimed.
2. **Wire:** when the peer advertises `filter` ([gitprotocol-capabilities](https://git-scm.com/docs/gitprotocol-capabilities) / [gitprotocol-pack](https://git-scm.com/docs/gitprotocol-pack)), request `filter` on the want line and send `filter blob:none` before the flush. Refuse `--filter` against peers that omit the capability.
3. **Repository extension:** after a successful filtered pack, set `core.repositoryformatversion=1` and `extensions.partialClone=<remote>` ([git-config](https://git-scm.com/docs/git-config) `extensions.partialClone`).
4. **Promisor packs:** mark ingested filtered packs (and demand-fetch backfill packs) with a sibling `.promisor` file (gitrepository-layout / partial-clone design).
5. **Checkout honesty:** clone/checkout MUST demand-fetch missing HEAD-tree blobs from the promisor remote so the worktree matches stock `git clone --filter=blob:none` user-visible success (historical blobs MAY remain absent).
6. **Non-claims remain:** `tree:0`, `blob:limit=*`, sparse OID filters, `deepen-since` / `--unshallow`, multi-promisor ordering product, server-side filter product as a sole claim.

## Consequences

- Residual **R3** → **Claimed (host wire)** only with carrying evidence (stock `git http-backend` + `uploadpack.allowFilter` / `allowAnySHA1InWant` peer preferred).
- Clean-room symmetry twin is **S6** (expand beyond L8); host green alone is not Verified.
- Public/`docs/git-vs-arks.md` MAY cite the subset only after evidence lands; until then keep N/A honesty.
- Symmetry ladder **S1** Done when SoT + wire + evidence + residual flip agree; then **S2**.
