---
id: evidence:root/20261108092148
title: M1 clean-room G0 G1 receipts
status: active
created: 2026-11-08T09:21:48Z
updated: 2026-11-08T18:53:19Z
tags: [m1, clean-room, g000-workspace-lib-tests, g001-loose-object-interop, historical, superseded-for-exit]
rel:
  depends_on: [sot:root/product-verification-clean-room, sot:root/interop-test-matrix, plan:root/20261108090646]
---

# M1 clean-room G0 / G1 receipts (historical)

## Context

Milestone **M1** (engine identity + loose objects). **Historical** receipt. Under roadmap **v0.3.1**, M1 exit claims MUST cite the restart receipt [[evidence:root/20261108185319]]; this document remains gap-fill context only.

Host `cargo test` results are **advisory only** and are not this receipt.

## Environment

| Field | Value |
|-------|-------|
| Compose | `verify/docker-compose.yml` |
| Image | `arks-verify-m1:0.1.0` |
| Image digest (local Id / RepoDigest) | `sha256:ec03b48a402fbab032687fdb5f85948adac9733b8c1b4682797ad4ceeac411b0` |
| Base | `rust:1.99.1-bookworm@sha256:e6f2cff35d0f309acf8433b4f05878ed77092602c5065cc81801177da0bb3a9c` |
| Stock Git (in-image) | `git version 2.39.5` |
| rustc (in-image) | `rustc 1.99.1 (58cf8f9c3 2026-10-13)` |
| `CARGO_BUILD_JOBS` | `2` |
| Host Docker engine | Linux (`desktop-linux`) |

## Results

| Gate | Command | Outcome | Notes |
|------|---------|---------|-------|
| **G0** | `docker compose -f verify/docker-compose.yml run --rm g000-workspace-lib-tests` | **PASS** | Workspace tests: arks-hash 9, arks-oid 8, arks-object 7 + g001-loose-object-interop_oracle 3 |
| **G1** | `docker compose -f verify/docker-compose.yml run --rm g001-loose-object-interop` | **PASS** | `arks-object` g001-loose-object-interop_oracle 3 + `verify/gates/g001-loose-object-interop` git_oracle 4 |

Combinatorics covered (automated): M1-C01…C06 per `verify/matrices/matrix-m1.md`. M1-C07 SHA-256 API units on G0 only (not stock-Git G1 claim). M1-C08+ deferred.

## Non-claims

- No CLI porcelain; no pack writers; no remotes; no CELIR; no leases.
- Workspace Cargo graph: **zero** `gix`/gitoxide edges.
- SHA-256 object-format interop with stock Git is **not** claimed at G1.

## Reproduce

```bash
docker compose -f verify/docker-compose.yml build toolchain
docker compose -f verify/docker-compose.yml run --rm g000-workspace-lib-tests
docker compose -f verify/docker-compose.yml run --rm g001-loose-object-interop
```
