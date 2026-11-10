---
id: evidence:root/20261109221500
title: M25 G29 dumb HTTP Claimed
status: active
created: 2026-11-09T22:15:00Z
updated: 2026-11-09T22:15:00Z
tags: [m25, g29, dumb-http, clean-room, verified, fw-srv-dumb]
rel:
  depends_on:
    - adr:root/20261109220230
    - plan:root/20261109220230
    - sot:root/arks-server
    - sot:root/interop-test-matrix
    - sot:root/product-verification-clean-room
    - evidence:root/20261108231300
---

# M25 G29 dumb HTTP (FW-SRV-DUMB)

## Context

Additive dumb HTTP beside Claimed smart HTTP (G15). Unlock [[adr:root/20261109220230]]. Primary: [gitprotocol-http](https://git-scm.com/docs/gitprotocol-http) dumb; [git-update-server-info](https://git-scm.com/docs/git-update-server-info).

## Environment

| Field | Value |
|-------|-------|
| Compose | `verify/docker-compose.yml` service `g29` |
| Image | `arks-verify-m1:0.1.0` |
| Image Id | `sha256:b235193c9ae45991f057aec1551feb769bf796ca917af1745110917cf53dfada` |
| Stock Git | in-image pin (2.39.5 class) |
| Tree (pre-commit tip) | `f20bcc827aa17dbe638fc6f8c8bc9312ab72b210` |

## Results

| Gate | Command | Outcome |
|------|---------|---------|
| **G29** | `docker compose … run --rm g29` | **PASS** — update-server-info + stock `git clone` via `--dumb-only`; arks-server unit 4 |

## Non-claims

- Dumb-HTTP-only as sole server path remains forbidden (smart HTTP still required).
- G30/G31/G32 not claimed by this receipt.
