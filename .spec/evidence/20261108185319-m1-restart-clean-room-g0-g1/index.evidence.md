---
id: evidence:root/20261108185319
title: M1 restart clean-room G0 G1 receipts
status: active
created: 2026-11-08T18:53:19Z
updated: 2026-11-08T18:53:19Z
tags: [m1, restart, clean-room, g0, g1, verified, v0.3.1]
rel:
  depends_on:
    - sot:root/product-verification-clean-room
    - sot:root/interop-test-matrix
    - sot:root/program-roadmap
    - plan:root/20261108090646
    - sot:root/arks-hash
    - sot:root/arks-oid
    - sot:root/arks-object
---

# M1 restart clean-room G0 / G1 receipts (roadmap v0.3.1)

## Context

Milestone **M1** (engine identity + loose objects) **restart / re-verify** under program roadmap **v0.3.1**. Authoritative venue: Docker clean-room under `verify/` per [[sot:root/product-verification-clean-room]]. Image pins: `verify/images.lock.json`. Matrix: `verify/matrices/matrix-m1.md`.

Host `cargo test` results are **advisory only** and are not this receipt.

**Supersedes for M1 exit claims:** [[evidence:root/20261108092148]] (historical; non-carrying alone under v0.3.1).

## Environment

| Field | Value |
|-------|-------|
| Compose | `verify/docker-compose.yml` |
| Image | `arks-verify-m1:0.1.0` |
| Image digest (local Id / RepoDigest) | `sha256:99688900673cc46a19591ae596ff7e5687f50398ada9660580c644474f3ce2df` |
| Base | `rust:1.99.1-bookworm@sha256:e6f2cff35d0f309acf8433b4f05878ed77092602c5065cc81801177da0bb3a9c` |
| Stock Git (in-image) | `git version 2.39.5` |
| rustc (in-image) | `rustc 1.99.1 (58cf8f9c3 2026-10-13)` |
| `CARGO_BUILD_JOBS` | `2` |
| Host Docker engine | Linux (`desktop-linux`) |
| Tree SHA (commit) | `6a4fffc2689d2065d31e334a793aefd23a900f10` |
| Workspace members exercised | `arks-hash`, `arks-oid`, `arks-object`, `arks-store` (store present; M1 claims are hash/oid/object) |

## Results

| Gate | Command | Outcome | Notes |
|------|---------|---------|-------|
| **G0** | `docker compose -f verify/docker-compose.yml run --rm g0` | **PASS** | `arks-hash` 12; `arks-oid` 12; `arks-object` 12 + `g1_oracle` 5; `arks-store` 8 (workspace) |
| **G1** | `docker compose -f verify/docker-compose.yml run --rm g1` | **PASS** | `arks-object` `g1_oracle` 5 + `verify/gates/g1` `git_oracle` 6 |

Combinatorics covered (automated): M1-C01…C07 and M1-C11…C13 per `verify/matrices/matrix-m1.md`. M1-C08 stock-Git tree/commit/tag payloads deferred (opaque kind framing covered on G0 as M1-C11). M1-C09/C10 out of scope.

## Adversarial / bug-simulation (G0)

Hash/OID/loose edges exercised in-crate: wrong digest lengths; near-miss hex lengths; mixed-case hex; NUL-in-payload hashing; type-token OID divergence; unknown kind / missing NUL / bad size / corrupt zlib; OID mismatch on corrupt loose; write_loose idempotency; claimed kind tokens with opaque payloads.

## Non-claims

- No CLI porcelain; no pack writers; no remotes; no CELIR; no leases; no alias UX.
- Workspace Cargo graph: **zero** `gix`/gitoxide edges.
- SHA-256 object-format interop with stock Git is **not** claimed at G1 (M1-C07 = G0 API only).
- Does **not** unlock M2+ or CLI-parity production (M9).

## Reproduce

```bash
docker compose -f verify/docker-compose.yml build toolchain
docker compose -f verify/docker-compose.yml run --rm g0
docker compose -f verify/docker-compose.yml run --rm g1
```
