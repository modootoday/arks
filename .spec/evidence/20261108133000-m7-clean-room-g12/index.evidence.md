---
id: evidence:root/20261108133000
title: M7 clean-room G12 Wave Pa porcelain
status: active
created: 2026-11-08T13:30:00Z
updated: 2026-11-08T13:30:00Z
tags: [m7, clean-room, g12, g6, g7, wave-pa, porcelain, verified, v0.3.1]
rel:
  depends_on:
    - sot:root/product-verification-clean-room
    - sot:root/interop-test-matrix
    - sot:root/program-roadmap
    - plan:root/20261108183810
    - sot:root/arks
    - sot:root/cli-surface
    - sot:root/arks-remote
    - adr:root/20261108083507
    - evidence:root/20261108112000
---

# M7 clean-room G12 Wave Pa receipts (roadmap v0.3.1+)

## Context

Milestone **M7** (Porcelain parity wave A / Phase **Pa**) under program roadmap **v0.3.1+**. Authoritative venue: Docker clean-room under `verify/` per [[sot:root/product-verification-clean-room]]. Image pins: `verify/images.lock.json`. Matrix: `verify/matrices/matrix-m7.md`. Unit SoT: [[sot:root/arks]] Wave Pa. Depends on M6 [[evidence:root/20261108112000]].

**Not production** — production is **M9**. Host `cargo test` results are **advisory only**.

## Environment

| Field | Value |
|-------|-------|
| Compose | `verify/docker-compose.yml` service `g12` |
| Image | `arks-verify-m1:0.1.0` |
| Image digest (local Id) | `sha256:a6c5cd96b72c88cc3b7519d00faeb7883dbd6ed2d6151e7f756a7223b748aa89` |
| Base | `rust:1.99.1-bookworm@sha256:e6f2cff35d0f309acf8433b4f05878ed77092602c5065cc81801177da0bb3a9c` |
| Stock Git (in-image) | `git version 2.39.5` |
| rustc (in-image) | `rustc 1.99.1 (58cf8f9c3 2026-10-13)` |
| `CARGO_BUILD_JOBS` | `2` |
| Host Docker engine | Linux (`desktop-linux`) |
| Tree SHA (implementation) | `7c074ef783c2b66d45d0aa0db83337ee157165a3` |
| Tree SHA (evidence tip) | `903a27b300eda1d30aef8b87a91a8c0c7d97066c` |
| Workspace members exercised | Rings 0–6 libs + `arks` Wave L+H+Pa bin |

## Results

| Gate | Command | Outcome | Notes |
|------|---------|---------|-------|
| **gix-exit** | `cargo metadata` graph scan in `run-g12.sh` | **PASS** | zero `gix` / `gitoxide` edges |
| **G12** | `docker compose -f verify/docker-compose.yml run --rm g12` | **PASS** | `verify/gates/g12` Wave Pa suite **7** |
| **G7** regression | (embedded in `run-g12.sh`) | **PASS** | `verify/gates/g7` **3** |
| **G6** regression | (embedded) | **PASS** | `verify/gates/g6` **6** |
| **G3** regression | (embedded) | **PASS** | `verify/gates/g3` **3** |
| **G4** regression | (embedded) | **PASS** | `verify/gates/g4` **3** |
| **G5** regression | (embedded) | **PASS** | `verify/gates/g5` **4** + `arks-protocol` |
| **G2** regression | (embedded) | **PASS** | `verify/gates/g2` **9** |
| **G10** regression | (embedded) | **PASS** | `verify/gates/g10` **7** |
| **G1** regression | (embedded) | **PASS** | `arks-object` `g1_oracle` **5** + `verify/gates/g1` **6** |
| Unit Specfold | `spec check` | **PASS** | 0 errors / 0 warnings |

Combinatorics covered (automated): M7-C01…C16 per `verify/matrices/matrix-m7.md`. Residual Open: M7-C18 Pb set; M7-C19 M9 closure; M7-C20 out-of-scope.

## Bug simulation (fixed)

1. `verify/scripts/run-g12.sh` initially had CRLF endings → would fail `set: pipefail` in Linux clean-room; rewritten to LF (`.gitattributes` `*.sh text eol=lf`). See `verify/gates/g12/BUGSIM.md`.
2. Host advisory `arks_bin()` preferred stale Linux ELF under `target-cleanroom` on Windows (Win32 error 193); fixed to honor `CARGO_TARGET_DIR` in clean-room and prefer workspace `target/debug/arks(.exe)` on host.
3. G6 help assertion used substring `worktree` which matched help prose (`Restore worktree…`); narrowed to command-list line match and reworded help descriptions to `working-tree`.
4. `diff` looked up unstored dirty-blob OIDs in the object store; fixed to diff against worktree bytes.

## Non-claims

- **Not production** — production is **M9** CLI-parity closure.
- Non-FF merge / `--no-ff`; annotated/signed tags; `worktree` / `reflog` / `repack` (refuse → Pc).
- Rebase-form `pull` → **M8** Wave Pb.
- No Git server / `arks-server` (**M11**).
- No forge APIs (B); no CELIR; no L1+ lease runtime (**M10**).
- Workspace Cargo graph: **zero** `gix`/gitoxide edges.

## Reproduce

```bash
docker compose -f verify/docker-compose.yml build toolchain
docker compose -f verify/docker-compose.yml run --rm g12
```
