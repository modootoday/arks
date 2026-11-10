---
id: evidence:root/20261109223000
title: M25 G32 git:// daemon Claimed
status: active
created: 2026-11-09T22:30:00Z
updated: 2026-11-09T22:30:00Z
tags: [m25, g032-git-daemon-wire, git-daemon, clean-room, verified, fw-srv-git]
rel:
  depends_on:
    - adr:root/20261109220230
    - plan:root/20261109220230
    - sot:root/arks-server
    - evidence:root/20261109222500
---

# M25 G32 `git://` daemon (FW-SRV-GIT)

## Context

`arks daemon` TCP listener per [git-daemon](https://git-scm.com/docs/git-daemon) + gitprotocol-pack duplex upload-pack. Unlock [[adr:root/20261109220230]].

## Environment

| Field | Value |
|-------|-------|
| Compose | `verify/docker-compose.yml` service `g032-git-daemon-wire` |
| Image | `arks-verify-m1:0.1.0` |
| Image Id | `sha256:b235193c9ae45991f057aec1551feb769bf796ca917af1745110917cf53dfada` |
| Docker re-verify | sole-runner `compose run --rm g032-git-daemon-wire` PASS after duplex NAK / blocking-socket fix |

## Results

| Gate | Command | Outcome |
|------|---------|---------|
| **G32** | `docker compose … run --rm g032-git-daemon-wire` | **PASS** — stock `git clone git://…` with `--export-all`; refuse without export marker |

## Non-claims

- receive-pack over `git://` remains disabled unless `--enable=receive-pack`.
- Product auth / informative-errors expansion not claimed.
