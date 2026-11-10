---
id: evidence:root/20261108190033
title: M2 restart clean-room G10 G1 receipts
status: active
created: 2026-11-08T10:00:33Z
updated: 2026-11-08T10:00:33Z
tags: [m2, restart, clean-room, g0, g1, g10, verified, v0.3.1]
rel:
  depends_on:
    - sot:root/product-verification-clean-room
    - sot:root/interop-test-matrix
    - sot:root/program-roadmap
    - plan:root/20261108090647
    - sot:root/arks-store
    - evidence:root/20261108185319
---

# M2 restart clean-room G10 / G1 receipts (roadmap v0.3.1)

## Context

Milestone **M2** (Git-first storage facade / Phase S) **restart / re-verify** under program roadmap **v0.3.1**. Authoritative venue: Docker clean-room under `verify/` per [[sot:root/product-verification-clean-room]]. Image pins: `verify/images.lock.json`. Matrix: `verify/matrices/matrix-m2.md`. Unit SoT: [[sot:root/arks-store]]. Depends on M1 restart [[evidence:root/20261108185319]].

Host `cargo test` results are **advisory only** and are not this receipt.

**Supersedes for M2 exit claims:** [[evidence:root/20261108093059]] (historical; non-carrying alone under v0.3.1).

## Environment

| Field | Value |
|-------|-------|
| Compose | `verify/docker-compose.yml` |
| Image | `arks-verify-m1:0.1.0` |
| Image digest (local Id / RepoDigest) | `sha256:934af464dbf23563ecbf52fbf67d33b01a3aad100f23406423290e3f66db6bfc` |
| Base | `rust:1.99.1-bookworm@sha256:e6f2cff35d0f309acf8433b4f05878ed77092602c5065cc81801177da0bb3a9c` |
| Stock Git (in-image) | `git version 2.39.5` |
| rustc (in-image) | `rustc 1.99.1 (58cf8f9c3 2026-10-13)` |
| `CARGO_BUILD_JOBS` | `2` |
| Host Docker engine | Linux (`desktop-linux`) |
| Tree SHA (commit) | `3d768fadbb796311fdc30b2efffa36115196a1e3` |
| Workspace members exercised | `arks-hash`, `arks-oid`, `arks-object`, `arks-store` |

## Results

| Gate | Command | Outcome | Notes |
|------|---------|---------|-------|
| **G0** | `docker compose -f verify/docker-compose.yml run --rm g0` | **PASS** | `arks-hash` 12; `arks-oid` 12; `arks-object` 12 + `g1_oracle` 5; `arks-store` 11 |
| **G10** | `docker compose -f verify/docker-compose.yml run --rm g10` | **PASS** | `verify/gates/g10` `git_oracle` 7 |
| **G1** regression | (embedded in `run-g10.sh`) | **PASS** | `arks-object` `g1_oracle` 5 + `verify/gates/g1` `git_oracle` 6 |

Combinatorics covered (automated): M2-C01…C11 per `verify/matrices/matrix-m2.md`. M2-C12 (full G1–G4-through-facade) deferred post-M4. M2-C13 CELIR path blocked. M2-C14 lease out of scope.

## Non-claims

- No CELIR activation; `celir` feature remains default-off stub only.
- No refs/index/repo product claims (M3).
- No CLI porcelain; no pack writers; no remotes; no leases; no alias UX.
- Workspace Cargo graph: **zero** `gix`/gitoxide edges.
- Full G1–G4-through-facade is **not** claimed (phased G10 objects-subset only).
- Does **not** unlock M3+ implementation claims until M3 plan work begins under its own SoTs.

## Reproduce

```bash
docker compose -f verify/docker-compose.yml build toolchain
docker compose -f verify/docker-compose.yml run --rm g0
docker compose -f verify/docker-compose.yml run --rm g10
```
