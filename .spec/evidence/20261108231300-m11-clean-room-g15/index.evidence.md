---
id: evidence:root/20261108231300
title: M11 clean-room G15 production-server
status: active
created: 2026-11-08T23:13:00Z
updated: 2026-11-08T23:13:00Z
tags: [m11, clean-room, g15, server, smart-http, production-server, verified, v0.3.1]
rel:
  depends_on:
    - sot:root/product-verification-clean-room
    - sot:root/interop-test-matrix
    - sot:root/program-roadmap
    - plan:root/20261108105446
    - sot:root/arks-server
    - sot:root/remote-host-compatibility-surface
    - sot:root/cli-surface
    - sot:root/public-surface
    - adr:root/20261108083507
    - evidence:root/20261108150000
---

# M11 clean-room G15 git-server receipts (production-server)

## Context

Milestone **M11** (Git server role / Phase **Sv**) under program roadmap **v0.3.1+**. This exit is the **production-server** bar — it does **not** redefine **M9** CLI-parity production. **M10 lease is not a prerequisite.** Authoritative venue: Docker clean-room under `verify/`. Matrix: `verify/matrices/matrix-m11.md`. Unit SoT: [[sot:root/arks-server]]. Depends on M9 [[evidence:root/20261108150000]].

Host `cargo test -p arks-server` / host G15 runs are **advisory only**.

## Environment

| Field | Value |
|-------|-------|
| Compose | `verify/docker-compose.yml` service `g15` |
| Image | `arks-verify-m1:0.1.0` |
| Image digest (local Id) | `sha256:a6c5cd96b72c88cc3b7519d00faeb7883dbd6ed2d6151e7f756a7223b748aa89` |
| Base | `rust:1.99.1-bookworm@sha256:e6f2cff35d0f309acf8433b4f05878ed77092602c5065cc81801177da0bb3a9c` |
| Stock Git (in-image) | `git version 2.39.5` |
| rustc (in-image) | `rustc 1.99.1 (58cf8f9c3 2026-10-13)` |
| `CARGO_BUILD_JOBS` | `2` |
| Host Docker engine | Linux (`desktop-linux`) |
| Tree SHA (implementation) | `1a91dae11444e21233028df46f18d11a21f595b6` |
| Claimed transport | Smart HTTP (required); SSH / `git://` / dumb HTTP **not claimed** |

## Results

| Gate | Command | Outcome | Notes |
|------|---------|---------|-------|
| **gix-exit** | metadata scan in `run-g15.sh` | **PASS** | zero `gix` / `gitoxide` |
| **no-CELIR** | metadata scan | **PASS** | |
| **no lease dep** | `arks-server` ↛ `arks-lease` | **PASS** | lease not required |
| **G15** | `docker compose … run --rm g15` | **PASS** | stock `git` clone/fetch/push + cap honesty; `verify/gates/g15` **3** + `arks-server` **4** |
| **G14** regression | (embedded) | **PASS** | **9** |
| **G7** regression | (embedded) | **PASS** | **3** |
| **G5** regression | (embedded) | **PASS** | `verify/gates/g5` **4** + `arks-protocol` |

Wave Sv CLI shipped: `arks upload-pack`, `arks receive-pack`, `arks serve-http`, `arks http-backend`.

Honest caps claimed: upload-pack (`ofs-delta`, `side-band-64k`, `no-progress`, `symref=HEAD:…`, `agent=arks/…`); receive-pack (`report-status`, `agent=arks/…`). Refused/not advertised: protocol v2 product, `delete-refs`, `report-status-v2`.

## Bug simulation (fixed / avoided)

1. `run-g15.sh` LF-only. See `verify/gates/g15/BUGSIM.md`.
2. Prefer native `arks.exe` over Linux ELF in shared target dirs on Windows host advisory runs.
3. Dumb-HTTP-only forbidden as sole path — suite uses smart HTTP.

## Non-claims

- No forge (B); no CELIR; no SSH / `git://` / dumb HTTP product claim.
- No lease-over-wire; M10 not required for this bar.
- M9 CLI-parity production remains [[evidence:root/20261108150000]] — unchanged.

## Reproduce

```bash
docker compose -f verify/docker-compose.yml build toolchain
docker compose -f verify/docker-compose.yml run --rm g15
```
