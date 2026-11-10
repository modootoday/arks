---
id: evidence:root/20261109222500
title: M25 G31 protocol v2 Claimed
status: active
created: 2026-11-09T22:25:00Z
updated: 2026-11-09T22:25:00Z
tags: [m25, g31, protocol-v2, ls-refs, fetch, clean-room, verified, fw-srv-v2]
rel:
  depends_on:
    - adr:root/20261109220230
    - plan:root/20261109220230
    - sot:root/arks-server
    - evidence:root/20261109222000
---

# M25 G31 protocol v2 (FW-SRV-V2)

## Context

Claimed subset: capability advertisement + `ls-refs` + `fetch` over smart HTTP with `Git-Protocol: version=2`. Primary: [gitprotocol-v2](https://git-scm.com/docs/gitprotocol-v2).

## Environment

| Field | Value |
|-------|-------|
| Compose | `verify/docker-compose.yml` service `g31` |
| Image | `arks-verify-m1:0.1.0` |
| Image Id | `sha256:b235193c9ae45991f057aec1551feb769bf796ca917af1745110917cf53dfada` |
| Prior tip | `94d6ef20db41ecd7f13394078ad7aadc0a47ab3a` |
| Docker re-verify | sole-runner `compose run --rm g31` PASS after ls-refs/fetch framing fix |

## Results

| Gate | Command | Outcome |
|------|---------|---------|
| **G31** | `docker compose … run --rm g31` | **PASS** — v2 ad honesty; stock `git -c protocol.version=2 clone`; v1 regression |

## Non-claims

- Unadvertised v2 commands (object-info, bundle-uri, etc.) remain unclaimed.
- G32 not claimed by this receipt.
