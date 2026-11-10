---
id: adr:root/20261110210000
title: O2 reopen — partial-clone filter-spec tree:0
status: active
created: 2026-11-10T21:00:00Z
updated: 2026-11-10T21:00:00Z
tags: [o2, r3, partial-clone, filter, tree-0, promisor, arks-remote]
rel:
  depends_on:
    - research:root/20261110205403
    - adr:root/20261110105547
    - sot:root/arks-remote
    - sot:root/interop-test-matrix
    - sot:root/production-residual-register
  supersedes: []
  evidence: [evidence:root/20261110210500]
---

# O2 — Reopen partial-clone filter-spec `tree:0`

## Context

[[adr:root/20261110105547]] Claimed `--filter=blob:none` and left `tree:0` / `blob:limit=*` as non-claims. Optional ladder [[research:root/20261110205403]] **O2** reopens SoTable filter-specs beyond `blob:none`. [partial-clone](https://git-scm.com/docs/partial-clone) / [git-rev-list `--filter`](https://git-scm.com/docs/git-rev-list#Documentation/git-rev-list.txt---filterltfilter-specgt) document `tree:<depth>` — `tree:0` omits blobs and trees at depth ≥ 0 except trees immediately referenced by commits (root trees).

## Decision

1. **Claim `--filter=tree:0`** on the same pack transports as `blob:none` (smart HTTP, SSH, `git://`) when the peer advertises `filter`.
2. **Wire:** request `filter` on the want line; send `filter tree:0` before flush; promisor + `extensions.partialClone` unchanged.
3. **Checkout backfill:** demand-fetch missing nested trees **and** blobs required for HEAD materialization (iterative OID want until the HEAD tree is complete). User-visible worktree parity with stock `git clone --filter=tree:0`.
4. **Retain refuse** for other filter-specs (`blob:limit=*`, sparse OID filters, combined/unknown specs) until separately claimed.
5. **Evidence:** host wire + expand G39-class clean-room twin before public Claimed flip.

## Non-claims

| Spec / surface | Disposition |
|----------------|-------------|
| `blob:limit=*`, `sparse:oid=…`, combine filters | Still refuse |
| Local / `file://` `--filter` | Still refuse |
| Server-side filter as sole server product | Still N/A |
| Inventing filter caps | Forbidden |

## Consequences

- Amend [[sot:root/arks-remote]] claimed filter subset: `blob:none` **and** `tree:0`.
- Residual R3 / FW-G9 text flips to include `tree:0` only with carrying evidence.
- Expand matrix G39 (or additive twin) for `tree:0` + refuse residual specs.
- Next **O3** (interactive scripted subset).
