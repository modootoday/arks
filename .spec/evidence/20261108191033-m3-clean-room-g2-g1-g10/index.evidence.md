---
id: evidence:root/20261108191033
title: M3 clean-room G2 G1 G10 receipts
status: active
created: 2026-11-08T10:10:33Z
updated: 2026-11-08T10:10:33Z
tags: [m3, clean-room, g0, g1, g2, g10, verified, v0.3.1]
rel:
  depends_on:
    - sot:root/product-verification-clean-room
    - sot:root/interop-test-matrix
    - sot:root/program-roadmap
    - plan:root/20261108090648
    - sot:root/arks-refs
    - sot:root/arks-index
    - sot:root/arks-repo
    - evidence:root/20261108190033
---

# M3 clean-room G2 / G1 / G10 receipts (roadmap v0.3.1)

## Context

Milestone **M3** (Refs + index + repo / Phase R) under program roadmap **v0.3.1**. Authoritative venue: Docker clean-room under `verify/` per [[sot:root/product-verification-clean-room]]. Image pins: `verify/images.lock.json`. Matrix: `verify/matrices/matrix-m3.md`. Unit SoTs: [[sot:root/arks-refs]], [[sot:root/arks-index]], [[sot:root/arks-repo]]. Depends on M2 restart [[evidence:root/20261108190033]].

Host `cargo test` results are **advisory only** and are not this receipt.

Pre-restart untracked M3 trees were **gap-filled** against current SoTs and are **not** treated as auto-verified without this receipt.

## Environment

| Field | Value |
|-------|-------|
| Compose | `verify/docker-compose.yml` |
| Image | `arks-verify-m1:0.1.0` |
| Image digest (local Id / RepoDigest) | `sha256:5b5f73aea95a7f11473dab09e5b385c8979a30275f3180462a998df1f0a11320` |
| Base | `rust:1.99.1-bookworm@sha256:e6f2cff35d0f309acf8433b4f05878ed77092602c5065cc81801177da0bb3a9c` |
| Stock Git (in-image) | `git version 2.39.5` |
| rustc (in-image) | `rustc 1.99.1 (58cf8f9c3 2026-10-13)` |
| `CARGO_BUILD_JOBS` | `2` |
| Host Docker engine | Linux (`desktop-linux`) |
| Tree SHA (commit) | `6dfffb387c211a26a9072d117ced251cd93da917` |
| Workspace members exercised | `arks-hash`, `arks-oid`, `arks-object`, `arks-store`, `arks-refs`, `arks-index`, `arks-repo` |

## Results

| Gate | Command | Outcome | Notes |
|------|---------|---------|-------|
| **G0** | `docker compose -f verify/docker-compose.yml run --rm g0` | **PASS** | workspace includes refs/index/repo unit tests |
| **G2** | `docker compose -f verify/docker-compose.yml run --rm g2` | **PASS** | `verify/gates/g2` `git_oracle` **9** |
| **G10** regression | (embedded in `run-g2.sh`) | **PASS** | `verify/gates/g10` `git_oracle` **7** |
| **G1** regression | (embedded in `run-g2.sh`) | **PASS** | `arks-object` `g1_oracle` **5** + `verify/gates/g1` `git_oracle` **6** |

Combinatorics covered (automated): M3-C01…C13 per `verify/matrices/matrix-m3.md`. Deferred: M3-C14 packed-refs write; M3-C15 full G3; M3-C16 CELIR; M3-C17 lease L1+.

## Non-claims

- No packed-refs rewrite (read-only packed-refs; loose overrides).
- No full **G3** index-commit round-trip (M5).
- No pack/protocol product claims (M4); no CLI porcelain; no remotes.
- No CELIR activation; no L1+ lease API (L0 Git locks only).
- Workspace Cargo graph: **zero** `gix`/gitoxide edges.
- Does **not** unlock M4+ implementation claims until M4 plan work begins under its own SoTs.

## Reproduce

```bash
docker compose -f verify/docker-compose.yml build toolchain
docker compose -f verify/docker-compose.yml run --rm g0
docker compose -f verify/docker-compose.yml run --rm g2
```
