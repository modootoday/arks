---
id: evidence:root/20261108143000
title: M8 clean-room G13 Wave Pb porcelain
status: active
created: 2026-11-08T14:30:00Z
updated: 2026-11-08T14:30:00Z
tags: [m8, clean-room, g13, g12, g7, g6, wave-pb, porcelain, verified, v0.3.1]
rel:
  depends_on:
    - sot:root/product-verification-clean-room
    - sot:root/interop-test-matrix
    - sot:root/program-roadmap
    - plan:root/20261108183811
    - sot:root/arks
    - sot:root/cli-surface
    - sot:root/arks-remote
    - adr:root/20261108083507
    - evidence:root/20261108133000
---

# M8 clean-room G13 Wave Pb receipts (roadmap v0.3.1+)

## Context

Milestone **M8** (Porcelain parity wave B / Phase **Pb**) under program roadmap **v0.3.1+**. Authoritative venue: Docker clean-room under `verify/` per [[sot:root/product-verification-clean-room]]. Image pins: `verify/images.lock.json`. Matrix: `verify/matrices/matrix-m8.md`. Unit SoT: [[sot:root/arks]] Wave Pb. Depends on M7 [[evidence:root/20261108133000]].

**Not production** — production is **M9**. Host `cargo test` results are **advisory only**.

## Environment

| Field | Value |
|-------|-------|
| Compose | `verify/docker-compose.yml` service `g13` |
| Image | `arks-verify-m1:0.1.0` |
| Image digest (local Id) | `sha256:a6c5cd96b72c88cc3b7519d00faeb7883dbd6ed2d6151e7f756a7223b748aa89` |
| Base | `rust:1.99.1-bookworm@sha256:e6f2cff35d0f309acf8433b4f05878ed77092602c5065cc81801177da0bb3a9c` |
| Stock Git (in-image) | `git version 2.39.5` |
| rustc (in-image) | `rustc 1.99.1 (58cf8f9c3 2026-10-13)` |
| `CARGO_BUILD_JOBS` | `2` |
| Host Docker engine | Linux (`desktop-linux`) |
| Tree SHA (implementation) | `885bb544961e1d74beb700c2921a18ad7a3ec53b` |
| Tree SHA (evidence tip) | `8ceb852d1e18dccb56859f06bde5c2dc40e335ca` |
| Workspace members exercised | Rings 0–6 libs + `arks` Wave L+H+Pa+Pb bin |

## Results

| Gate | Command | Outcome | Notes |
|------|---------|---------|-------|
| **gix-exit** | `cargo metadata` graph scan in `run-g13.sh` | **PASS** | zero `gix` / `gitoxide` edges |
| **G13** | `docker compose -f verify/docker-compose.yml run --rm g13` | **PASS** | `verify/gates/g13` Wave Pb suite **10** |
| **G12** regression | (embedded in `run-g13.sh`) | **PASS** | `verify/gates/g12` **7** |
| **G7** regression | (embedded) | **PASS** | `verify/gates/g7` **3** |
| **G6** regression | (embedded) | **PASS** | `verify/gates/g6` **6** |
| **G3** regression | (embedded) | **PASS** | `verify/gates/g3` **3** |
| **G4** regression | (embedded) | **PASS** | `verify/gates/g4` **3** |
| **G5** regression | (embedded) | **PASS** | `verify/gates/g5` **4** + `arks-protocol` |
| **G2** regression | (embedded) | **PASS** | `verify/gates/g2` **9** |
| **G10** regression | (embedded) | **PASS** | `verify/gates/g10` **7** |
| **G1** regression | (embedded) | **PASS** | `arks-object` `g1_oracle` **5** + `verify/gates/g1` **6** |
| Unit Specfold | `spec check` | **PASS** | 0 errors / 0 warnings |

Combinatorics covered (automated): M8-C01…C15 per `verify/matrices/matrix-m8.md`. Residual Open: M8-C17 Pc deferrals (`range-diff`/`notes`/interactive rebase/annotated tags); M8-C18 M9 closure; M8-C19 out-of-scope.

## Bug simulation (fixed / avoided)

1. `verify/scripts/run-g13.sh` authored LF-only up front (same CRLF `set: pipefail` class as G12/G7). See `verify/gates/g13/BUGSIM.md`.
2. Host advisory `arks_bin()` continues to honor `CARGO_TARGET_DIR` in clean-room and prefer workspace `target/debug/arks(.exe)` on host (avoids Win32 error 193 from Linux ELF under `target-cleanroom`).

## Non-claims

- **Not production** — production is **M9** CLI-parity closure.
- `range-diff` / `notes` / `filter-branch` / interactive rebase (refuse → Pc).
- Annotated/signed tags; `worktree` / `reflog` / `repack` (still refuse → Pc).
- Conflicted history edits (refuse non-zero; no 3-way merge engine).
- No Git server / `arks-server` (**M11**).
- No forge APIs (B); no CELIR; no L1+ lease runtime (**M10**).
- Workspace Cargo graph: **zero** `gix`/gitoxide edges.

## Reproduce

```bash
docker compose -f verify/docker-compose.yml build toolchain
docker compose -f verify/docker-compose.yml run --rm g13
```
