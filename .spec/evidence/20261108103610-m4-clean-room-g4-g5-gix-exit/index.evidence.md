---
id: evidence:root/20261108103610
title: M4 clean-room G4 G5 gix-exit receipts
status: active
created: 2026-11-08T10:36:10Z
updated: 2026-11-08T10:36:10Z
tags: [m4, clean-room, g0, g1, g2, g4, g5, g10, gix-exit, verified, v0.3.1]
rel:
  depends_on:
    - sot:root/product-verification-clean-room
    - sot:root/interop-test-matrix
    - sot:root/program-roadmap
    - plan:root/20261108090649
    - plan:root/20261108085816
    - sot:root/arks-pack
    - sot:root/arks-protocol
    - adr:root/20261108083507
    - evidence:root/20261108191033
---

# M4 clean-room G4 / G5 / gix-exit receipts (roadmap v0.3.1)

## Context

Milestone **M4** (Pack + protocol client libs / Phase P + **gix-exit**) under program roadmap **v0.3.1**. Authoritative venue: Docker clean-room under `verify/` per [[sot:root/product-verification-clean-room]]. Image pins: `verify/images.lock.json`. Matrix: `verify/matrices/matrix-m4.md`. Unit SoTs: [[sot:root/arks-pack]], [[sot:root/arks-protocol]]. Depends on M3 [[evidence:root/20261108191033]].

Host `cargo test` results are **advisory only** and are not this receipt.

## Environment

| Field | Value |
|-------|-------|
| Compose | `verify/docker-compose.yml` |
| Image | `arks-verify-m1:0.1.0` |
| Image digest (local Id) | `sha256:5b5f73aea95a7f11473dab09e5b385c8979a30275f3180462a998df1f0a11320` |
| Base | `rust:1.99.1-bookworm@sha256:e6f2cff35d0f309acf8433b4f05878ed77092602c5065cc81801177da0bb3a9c` |
| Stock Git (in-image) | `git version 2.39.5` |
| rustc (in-image) | `rustc 1.99.1 (58cf8f9c3 2026-10-13)` |
| `CARGO_BUILD_JOBS` | `2` |
| Host Docker engine | Linux (`desktop-linux`) |
| Tree SHA (commit) | `b42c99f1b8f35bbc3f8f89a831c9a670ce03d432` |
| Workspace members exercised | `arks-hash`, `arks-oid`, `arks-object`, `arks-store`, `arks-refs`, `arks-index`, `arks-repo`, `arks-pack`, `arks-protocol` |

## Results

| Gate | Command | Outcome | Notes |
|------|---------|---------|-------|
| **G0** | `docker compose -f verify/docker-compose.yml run --rm g0` | **PASS** | workspace includes pack/protocol unit tests |
| **G4** | `docker compose -f verify/docker-compose.yml run --rm g4` | **PASS** | `verify/gates/g4` `git_oracle` **3** |
| **G5** | (embedded in `run-g4.sh`) | **PASS** | `verify/gates/g5` offline **4** + `arks-protocol` unit/g5_offline |
| **G2** regression | (embedded in `run-g4.sh`) | **PASS** | `verify/gates/g2` `git_oracle` **9** |
| **G10** regression | (embedded in `run-g4.sh`) | **PASS** | `verify/gates/g10` `git_oracle` **7** |
| **G1** regression | (embedded in `run-g4.sh`) | **PASS** | `arks-object` `g1_oracle` **5** + `verify/gates/g1` `git_oracle` **6** |
| **gix-exit** | `cargo metadata` graph scan | **PASS** | zero packages/edges named `gix` / `gitoxide` |

Combinatorics covered (automated): M4-C01…C14 per `verify/matrices/matrix-m4.md`. Deferred: M4-C15 delta encode / MIDX / `.rev`; M4-C16 remotes; M4-C17 CLI/G3; M4-C18 CELIR/lease.

## Non-claims

- No CLI porcelain (M5); no remotes / live upload-pack (M6).
- No pack delta **encode**, pack v3, MIDX, or `.rev` product claims.
- No CELIR activation; no L1+ lease API.
- Workspace Cargo graph: **zero** `gix`/gitoxide edges (gix-exit complete; never introduced).
- Does **not** unlock M5+ implementation claims until M5 plan work begins under its own SoTs.

## Reproduce

```bash
docker compose -f verify/docker-compose.yml build toolchain
docker compose -f verify/docker-compose.yml run --rm g0
docker compose -f verify/docker-compose.yml run --rm g4
# optional standalone:
docker compose -f verify/docker-compose.yml run --rm g5
```
