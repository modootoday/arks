---
id: evidence:root/20261110105855
title: R3 partial-clone blob:none Claimed (host wire)
status: active
created: 2026-11-10T10:58:55Z
updated: 2026-11-10T10:58:55Z
tags: [r3, s1, partial-clone, filter, promisor, blob-none, host-wire, claimed, symmetry]
rel:
  depends_on:
    - adr:root/20261110105547
    - sot:root/arks-remote
    - research:root/20261110105436
    - sot:root/production-residual-register
  evidence: []
files: []
---

# R3 — Partial-clone `blob:none` Claimed (host wire)

## Claimed subset (honest)

| In | Out |
|----|-----|
| `--filter=blob:none` on smart HTTP (pack-protocol `filter` cap) | `tree:0`, `blob:limit=*`, sparse OID filters |
| `extensions.partialClone=<remote>` + format v1 | Local / `file://` / dumb HTTP `--filter` |
| Promisor pack sibling `.promisor` | Multi-promisor ordering product |
| Demand-fetch missing HEAD-tree blobs so worktree matches stock clone | Claiming historical blobs remain without backfill |
| SSH / `git://` same wire path as HTTP | Server-side filter as sole server product |

Primary: [partial-clone](https://git-scm.com/docs/partial-clone), [gitprotocol-pack](https://git-scm.com/docs/gitprotocol-pack) `filter-request`, [gitprotocol-capabilities](https://git-scm.com/docs/gitprotocol-capabilities) `filter`, [git-clone `--filter`](https://git-scm.com/docs/git-clone#Documentation/git-clone.txt---filterltfilter-specgt), [git-config](https://git-scm.com/docs/git-config) `extensions.partialClone`. Unlock: [[adr:root/20261110105547]]. Symmetry ladder: [[research:root/20261110105436]] **S1**.

## Host advisory (Claimed bar for this reopen)

| Check | Outcome |
|-------|---------|
| `cargo test -p arks-remote stock_git_http_backend_partial_clone_blob_none` | **PASS** — stock `git http-backend` peer with `uploadpack.allowFilter` + `allowAnySHA1InWant`; worktree `README` present; `extensions.partialClone`; ≥1 `.promisor` |
| `cargo test -p arks-remote stock_git_http_backend_shallow_clone_depth_1` | PASS (G35 regress) |

Host green alone is not Docker Verified. Symmetry clean-room twin is ladder **S6**.

## Non-claims

- Invented filter-specs beyond `blob:none`.
- Live forge / G8 latency PASS.
- G19 remasure.
