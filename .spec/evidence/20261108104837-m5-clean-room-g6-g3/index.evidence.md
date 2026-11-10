---
id: evidence:root/20261108104837
title: M5 clean-room G6 G3 Wave L
status: active
created: 2026-11-08T10:48:37Z
updated: 2026-11-08T10:48:37Z
tags: [m5, clean-room, g0, g1, g2, g3, g4, g5, g6, g10, wave-l, verified, v0.3.1]
rel:
  depends_on:
    - sot:root/product-verification-clean-room
    - sot:root/interop-test-matrix
    - sot:root/program-roadmap
    - plan:root/20261108090650
    - sot:root/arks
    - sot:root/cli-surface
    - sot:root/arks-object
    - adr:root/20261108083507
    - evidence:root/20261108103610
---

# M5 clean-room G6 / G3 Wave L receipts (roadmap v0.3.1)

## Context

Milestone **M5** (Local CLI foundation / Phase L) under program roadmap **v0.3.1**. Authoritative venue: Docker clean-room under `verify/` per [[sot:root/product-verification-clean-room]]. Image pins: `verify/images.lock.json`. Matrix: `verify/matrices/matrix-m5.md`. Unit SoT: [[sot:root/arks]]. Depends on M4 [[evidence:root/20261108103610]].

Host `cargo test` / host CLI smoke results are **advisory only** and are not this receipt.

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
| Tree SHA (commit) | `75bbc510fa612379f8ad095de95d71ba664a77a0` |
| Workspace members exercised | Rings 0–5 libs + `arks` Wave L bin |

## Results

| Gate | Command | Outcome | Notes |
|------|---------|---------|-------|
| **G0** | `docker compose -f verify/docker-compose.yml run --rm g0` | **PASS** | workspace includes `arks` + tree/commit unit tests |
| **G6** | `docker compose -f verify/docker-compose.yml run --rm g6` | **PASS** | `verify/gates/g6` argv suite **6** |
| **G3** | (embedded in `run-g6.sh`; also `g3` service) | **PASS** | `verify/gates/g3` `git_oracle` **3** |
| **G4** regression | (embedded in `run-g6.sh`) | **PASS** | `verify/gates/g4` **3** |
| **G5** regression | (embedded in `run-g6.sh`) | **PASS** | `verify/gates/g5` **4** + `arks-protocol` |
| **G2** regression | (embedded in `run-g6.sh`) | **PASS** | `verify/gates/g2` **9** |
| **G10** regression | (embedded in `run-g6.sh`) | **PASS** | `verify/gates/g10` **7** |
| **G1** regression | (embedded in `run-g6.sh`) | **PASS** | `arks-object` `g1_oracle` **5** + `verify/gates/g1` **6** |
| **gix-exit** | `cargo metadata` graph scan | **PASS** | zero packages/edges named `gix` / `gitoxide` |

Combinatorics covered (automated): M5-C01…C14 per `verify/matrices/matrix-m5.md`. Deferred: M5-C15 remotes; M5-C16 CELIR/lease; M5-C17 Pa/Pb/Pc.

## Non-claims

- **Not production** — production is **M9** CLI-parity closure.
- No Wave H remotes / forge APIs (M6+).
- No Pa/Pb/Pc porcelain inventory closure.
- No CELIR activation; no L1+ lease API.
- Pack helpers remain library-only (`arks-pack`); no Wave L pack bins.
- Workspace Cargo graph: **zero** `gix`/gitoxide edges (unchanged since M4 gix-exit).

## Reproduce

```bash
docker compose -f verify/docker-compose.yml build toolchain
docker compose -f verify/docker-compose.yml run --rm g0
docker compose -f verify/docker-compose.yml run --rm g6
# optional standalone:
docker compose -f verify/docker-compose.yml run --rm g3
```
