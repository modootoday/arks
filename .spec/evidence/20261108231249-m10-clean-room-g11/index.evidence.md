---
id: evidence:root/20261108231249
title: M10 clean-room G11 lease capability
status: active
created: 2026-11-08T23:12:49Z
updated: 2026-11-08T23:12:49Z
tags: [m10, clean-room, g11, lease, dual-path, verified, v0.3.1]
rel:
  depends_on:
    - sot:root/product-verification-clean-room
    - sot:root/interop-test-matrix
    - sot:root/program-roadmap
    - plan:root/20261108183813
    - sot:root/arks-lease
    - sot:root/lease-concurrency-intent
    - sot:root/public-surface
    - adr:root/20261108083507
    - evidence:root/20261108150000
---

# M10 clean-room G11 lease-product receipts

## Context

Milestone **M10** (default lease capability + optional high-concurrency valve / Phase **E**) under program roadmap **v0.3.1+**. This exit is the **lease capability** bar — it does **not** redefine **M9** CLI-parity production. Authoritative venue: Docker clean-room under `verify/` per [[sot:root/product-verification-clean-room]]. Image pins: `verify/images.lock.json`. Matrix: `verify/matrix-m10.md`. Unit SoT: [[sot:root/arks-lease]]. Depends on M9 [[evidence:root/20261108150000]].

Host `cargo test -p arks-lease` results are **advisory only**.

## Environment

| Field | Value |
|-------|-------|
| Compose | `verify/docker-compose.yml` service `g11` |
| Image | `arks-verify-m1:0.1.0` |
| Image digest (local Id) | `sha256:a6c5cd96b72c88cc3b7519d00faeb7883dbd6ed2d6151e7f756a7223b748aa89` |
| Base | `rust:1.99.1-bookworm@sha256:e6f2cff35d0f309acf8433b4f05878ed77092602c5065cc81801177da0bb3a9c` |
| Stock Git (in-image) | `git version 2.39.5` |
| rustc (in-image) | `rustc 1.99.1 (58cf8f9c3 2026-10-13)` |
| `CARGO_BUILD_JOBS` | `2` |
| Host Docker engine | Linux (`desktop-linux`) |
| Tree SHA (implementation) | `1a91dae11444e21233028df46f18d11a21f595b6` |
| Workspace members exercised | Rings 0–6 libs + `arks-lease` + `arks` bin |

## Results

| Gate | Command | Outcome | Notes |
|------|---------|---------|-------|
| **gix-exit** | `cargo metadata` graph scan in `run-g11.sh` | **PASS** | zero `gix` / `gitoxide` edges |
| **no-CELIR** | metadata scan | **PASS** | zero `celir` edges |
| **G11** | `docker compose … run --rm g11` | **PASS** | `arks-lease` **6** + `verify/g11` **2** |
| **G14** regression | (embedded) | **PASS** | `verify/g14` **9** |
| **G7** regression | (embedded) | **PASS** | `verify/g7` **3** |
| Unit Specfold | `spec check` | **PASS** | (recorded at closeout) |

Dual-path proof: `WriterMode::GitCompat` default; `HighConcurrency` → `HighConcurrencyRefused`. Lease ≠ skip Git locks: `lease_held_still_honors_git_ref_lock` (competing `*.lock` → `LockFailed` while lease held).

## Bug simulation (fixed / avoided)

1. `verify/scripts/run-g11.sh` LF-only (CRLF `set: pipefail` class). See `verify/g11/BUGSIM.md`.
2. HighConcurrency must refuse — no silent N-way.
3. Lease must not bypass ref locks.

## Non-claims

- No L3 N-way pipeline shipped (opt-in valve refuses).
- No forge; no CELIR; no production-server claim from this receipt (**M11** / **G15** separate).
- M9 CLI-parity production remains [[evidence:root/20261108150000]] — unchanged.

## Reproduce

```bash
docker compose -f verify/docker-compose.yml build toolchain
docker compose -f verify/docker-compose.yml run --rm g11
```
