---
id: evidence:root/20261108093059
title: M2 clean-room G10 G1 receipts
status: active
created: 2026-11-08T09:30:59Z
updated: 2026-11-08T18:56:00Z
tags: [m2, clean-room, g000-workspace-lib-tests, g001-loose-object-interop, g010-store-facade, historical, non-carrying]
rel:
  depends_on: [sot:root/product-verification-clean-room, sot:root/interop-test-matrix, plan:root/20261108090647, sot:root/arks-store]
---

# M2 clean-room G10 / G1 receipts

## Context

Milestone **M2** (Git-first storage facade / Phase S). Authoritative venue: Docker clean-room under `verify/` per [[sot:root/product-verification-clean-room]]. Image pins: `verify/images.lock.json`. Matrix: `verify/matrices/matrix-m2.md`. Unit SoT: [[sot:root/arks-store]].

**Roadmap v0.3.1:** This receipt is **historical / non-carrying alone** for M2 exit. Carrying restart receipt: [[evidence:root/20261108190033]] (after M1 restart [[evidence:root/20261108185319]], [[plan:root/20261108090647]]). Do not treat this document as automatic M2 exit carry-forward.

Host `cargo test` results are **advisory only** and are not this receipt.

## Environment

| Field | Value |
|-------|-------|
| Compose | `verify/docker-compose.yml` |
| Image | `arks-verify-m1:0.1.0` |
| Image digest (local Id) | `sha256:ec03b48a402fbab032687fdb5f85948adac9733b8c1b4682797ad4ceeac411b0` |
| Base | `rust:1.99.1-bookworm@sha256:e6f2cff35d0f309acf8433b4f05878ed77092602c5065cc81801177da0bb3a9c` |
| Stock Git (in-image) | `git version 2.39.5` |
| rustc (in-image) | `rustc 1.99.1 (58cf8f9c3 2026-10-13)` |
| `CARGO_BUILD_JOBS` | `2` |
| Host Docker engine | Linux (`desktop-linux`) |

## Results

| Gate | Command | Outcome | Notes |
|------|---------|---------|-------|
| **G0** | `docker compose -f verify/docker-compose.yml run --rm g000-workspace-lib-tests` | **PASS** | Workspace: arks-hash 9, arks-oid 8, arks-object 7 + g001-loose-object-interop_oracle 3, arks-store 8 |
| **G10** | `docker compose -f verify/docker-compose.yml run --rm g010-store-facade` | **PASS** | `verify/gates/g010-store-facade` git_oracle 7 |
| **G1** regression | (embedded in `run-g010-store-facade.sh`) | **PASS** | `arks-object` g001-loose-object-interop_oracle 3 + `verify/gates/g001-loose-object-interop` git_oracle 4 |

Combinatorics covered (automated): M2-C01…C11 per `verify/matrices/matrix-m2.md`. M2-C12 (full G1–G4-through-facade) deferred post-M4. M2-C13 CELIR path blocked. M2-C14 lease out of scope.

## Non-claims

- No CELIR activation; `celir` feature remains default-off stub only.
- No refs/index/repo product claims (M3).
- No CLI porcelain; no pack writers; no remotes; no leases.
- Workspace Cargo graph: **zero** `gix`/gitoxide edges.
- Full G1–G4-through-facade is **not** claimed (phased G10 objects-subset only).

## Reproduce

```bash
docker compose -f verify/docker-compose.yml build toolchain
docker compose -f verify/docker-compose.yml run --rm g000-workspace-lib-tests
docker compose -f verify/docker-compose.yml run --rm g010-store-facade
```
