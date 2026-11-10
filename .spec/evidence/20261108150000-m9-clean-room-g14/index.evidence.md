---
id: evidence:root/20261108150000
title: M9 clean-room G14 CLI-parity production
status: active
created: 2026-11-08T15:00:00Z
updated: 2026-11-08T15:00:00Z
tags: [m9, clean-room, g14, g13, g12, g7, g6, wave-pc, porcelain, production, alias, verified, v0.3.1]
rel:
  depends_on:
    - sot:root/product-verification-clean-room
    - sot:root/interop-test-matrix
    - sot:root/program-roadmap
    - plan:root/20261108183812
    - sot:root/arks
    - sot:root/cli-surface
    - sot:root/cli-flag-parity-m9
    - sot:root/arks-remote
    - sot:root/public-surface
    - adr:root/20261108083507
    - evidence:root/20261108143000
---

# M9 clean-room G14 Wave Pc receipts (CLI-parity production)

## Context

Milestone **M9** (Porcelain parity C + inventory closure / Phase **Pc**) under program roadmap **v0.3.1+**. This exit **is CLI-parity production** (same `.git` + CLI inventory + flag-parity + alias-ready + remote client). Authoritative venue: Docker clean-room under `verify/` per [[sot:root/product-verification-clean-room]]. Image pins: `verify/images.lock.json`. Matrix: `verify/matrix-m9.md`. Unit SoT: [[sot:root/arks]] Wave Pc. Flag-parity: [[sot:root/cli-flag-parity-m9]]. Alias: `docs/alias-git-arks.md`. Depends on M8 [[evidence:root/20261108143000]].

Host `cargo test` results are **advisory only**.

## Environment

| Field | Value |
|-------|-------|
| Compose | `verify/docker-compose.yml` service `g14` |
| Image | `arks-verify-m1:0.1.0` |
| Image digest (local Id) | `sha256:a6c5cd96b72c88cc3b7519d00faeb7883dbd6ed2d6151e7f756a7223b748aa89` |
| Base | `rust:1.99.1-bookworm@sha256:e6f2cff35d0f309acf8433b4f05878ed77092602c5065cc81801177da0bb3a9c` |
| Stock Git (in-image) | `git version 2.39.5` |
| rustc (in-image) | `rustc 1.99.1 (58cf8f9c3 2026-10-13)` |
| `CARGO_BUILD_JOBS` | `2` |
| Host Docker engine | Linux (`desktop-linux`) |
| Tree SHA (implementation) | `e15cb573ba9918e858f6347727dd1189cac703f9` |
| Tree SHA (evidence tip) | `6403d18deab018536c72c0c858b671e8e2399ac7` |
| Workspace members exercised | Rings 0–6 libs + `arks` Wave L+H+Pa+Pb+Pc bin |

## Results

| Gate | Command | Outcome | Notes |
|------|---------|---------|-------|
| **gix-exit** | `cargo metadata` graph scan in `run-g14.sh` | **PASS** | zero `gix` / `gitoxide` edges |
| **G14** | `docker compose -f verify/docker-compose.yml run --rm g14` | **PASS** | `verify/g14` Wave Pc suite **9** |
| **G13** regression | (embedded in `run-g14.sh`) | **PASS** | `verify/g13` **10** |
| **G12** regression | (embedded) | **PASS** | `verify/g12` **7** |
| **G7** regression | (embedded) | **PASS** | `verify/g7` **3** |
| **G6** regression | (embedded) | **PASS** | `verify/g6` **6** |
| **G3** regression | (embedded) | **PASS** | `verify/g3` **3** |
| **G4** regression | (embedded) | **PASS** | `verify/g4` **3** |
| **G5** regression | (embedded) | **PASS** | `verify/g5` **4** + `arks-protocol` |
| **G2** regression | (embedded) | **PASS** | `verify/g2` **9** |
| **G10** regression | (embedded) | **PASS** | `verify/g10` **7** |
| **G1** regression | (embedded) | **PASS** | `arks-object` `g1_oracle` **5** + `verify/g1` **6** |
| Unit Specfold | `spec check` | **PASS** | 0 errors / 0 warnings |

Combinatorics covered (automated): M9-C01…C16, C18 per `verify/matrix-m9.md`. Residual: M9-C19 out-of-scope (lease/CELIR/server/forge); pin-advance closed-as-process ([[sot:root/clean-room-git-pin-advance]]); credential-helper refuse-closed for M9 production.

## Bug simulation (fixed / avoided)

1. `verify/scripts/run-g14.sh` authored/normalized LF-only (same CRLF `set: pipefail` class as G13/G12/G7). See `verify/g14/BUGSIM.md`.
2. G12/G13 regression expectations updated for Pc claims (`reflog`, `clean -d`, `bundle verify`) so prior gates do not falsely require refuse.
3. Alias tests use PATH argv shim (not shell `alias`) inside cargo tests.

## Non-claims

- No Git server / `arks-server` (**M11** / production-server).
- No forge APIs (B); no CELIR; no L1+ lease runtime (**M10**).
- Refuse-closed inventory rows remain unimplemented: `worktree`, `repack`, `sparse-checkout`, `submodule`, `maintenance`, `backfill`, `range-diff`, `notes`, `filter-branch`, interactive rebase, signed tags, non-FF merge.
- Workspace Cargo graph: **zero** `gix`/gitoxide edges.

## Reproduce

```bash
docker compose -f verify/docker-compose.yml build toolchain
docker compose -f verify/docker-compose.yml run --rm g14
```
