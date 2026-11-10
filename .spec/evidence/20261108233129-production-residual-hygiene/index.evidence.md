---
id: evidence:root/20261108233129
title: Production residual hygiene G14 G11 G15 G7
status: active
created: 2026-11-08T23:31:29Z
updated: 2026-11-08T23:31:29Z
tags: [residual, hygiene, g014-cli-parity-production, g011-lease-product, g015-smart-http, g007-smart-http-client, production, refuse-close, verified]
rel:
  depends_on:
    - sot:root/product-verification-clean-room
    - sot:root/production-residual-register
    - sot:root/clean-room-git-pin-advance
    - sot:root/public-surface
    - sot:root/program-roadmap
    - adr:root/20261108232800
    - evidence:root/20261108150000
    - evidence:root/20261108231249
    - evidence:root/20261108231300
---

# Production residual hygiene (post-M9/M10/M11)

## Context

Closes **residual Opens** after carrying production bars without inventing a fourth bar. Normative register: [[sot:root/production-residual-register]]. Carrying exits remain:

| Bar | Evidence |
|-----|----------|
| M9 CLI-parity | [[evidence:root/20261108150000]] |
| M10 lease L1→L2 | [[evidence:root/20261108231249]] |
| M11 production-server | [[evidence:root/20261108231300]] |

This receipt proves **refuse-close / process / defer** honesty plus **full clean-room regression** after residual Specfold updates. Authoritative venue: Docker clean-room under `verify/`.

## Environment

| Field | Value |
|-------|-------|
| Compose | `verify/docker-compose.yml` services `g014-cli-parity-production`, `g011-lease-product`, `g015-smart-http`, `g007-smart-http-client` |
| Image | `arks-verify-m1:0.1.0` |
| Image digest (local Id) | `sha256:bab37bd1ea03832a5ca30aa41e598eafc8bef7a5b1026d65acefb75e3f4a73c1` |
| Base | `rust:1.99.1-bookworm@sha256:e6f2cff35d0f309acf8433b4f05878ed77092602c5065cc81801177da0bb3a9c` |
| Stock Git (in-image) | `git version 2.39.5` |
| rustc (in-image) | `rustc 1.99.1 (58cf8f9c3 2026-10-13)` |
| `CARGO_BUILD_JOBS` | `2` |
| Host Docker engine | Linux (`desktop-linux`) |
| Tree SHA (residual close) | `7906e6bedbe1ffdc492c3c15437e4ec45a410fc5` |
| Specfold | `spec check` **0** errors / **0** warnings |

## Results

| Gate | Command | Outcome | Notes |
|------|---------|---------|-------|
| **gix-exit** / **no-CELIR** | metadata scans in gate scripts | **PASS** | all four services |
| **G14** | `docker compose … run --rm g014-cli-parity-production` | **PASS** | Pc **9** + G13/G12/G7/G6 + prior |
| **G11** | `docker compose … run --rm g011-lease-product` | **PASS** | lease **6** + g011-lease-product **2** + G14/G7 |
| **G15** | `docker compose … run --rm g015-smart-http` | **PASS** | server **4** + g015-smart-http **3** + G14/G7/G5 |
| **G7** | `docker compose … run --rm g007-smart-http-client` | **PASS** | remote **3** + prior (standalone) |
| Unit Specfold | `spec check` | **PASS** | residual SoTs + L3 ADR |

## Residual dispositions frozen

| Cluster | Disposition |
|---------|-------------|
| A Client | Pin-advance **process** [[sot:root/clean-room-git-pin-advance]]; credential-helper/askpass **refuse-closed**; Pc honesty + alias/same-`.git` re-smoked via G14 |
| B Lease | L3 **refuse-closed** [[adr:root/20261108232800]]; allocator placement **deferred Open** |
| C Server | SSH / `git://` / dumb HTTP / protocol v2 / product auth / hooks **refuse-closed**; G15 matrix frozen smart HTTP |
| D Hygiene | This receipt |

## Bug simulation

1. Image rebuild may refresh local Id — residual hygiene records the Id used for this run (`bab37bd1…`); prior M9/M10/M11 receipts remain historical for their own runs.
2. `spec check` L3: SoTs must not appear under `rel.evidence` (fixed: residual SoTs under `depends_on`).
3. Prior gate BUGSIM notes still apply (`verify/gates/g014-cli-parity-production`, `g011-lease-product`, `g015-smart-http`).

## Non-claims

- No fourth “true production” bar.
- No L3 N-way pipeline; no credential-helper; no SSH/`git://`/dumb HTTP server; no protocol v2 product; no forge auth; no server hooks honored.
- M9/M10/M11 carrying receipts unchanged in meaning.

## Reproduce

```bash
docker compose -f verify/docker-compose.yml build toolchain
docker compose -f verify/docker-compose.yml run --rm g014-cli-parity-production
docker compose -f verify/docker-compose.yml run --rm g011-lease-product
docker compose -f verify/docker-compose.yml run --rm g015-smart-http
docker compose -f verify/docker-compose.yml run --rm g007-smart-http-client
spec check
```
