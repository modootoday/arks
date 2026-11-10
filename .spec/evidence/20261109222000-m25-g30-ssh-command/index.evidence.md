---
id: evidence:root/20261109222000
title: M25 G30 SSH command-mode Claimed
status: active
created: 2026-11-09T22:20:00Z
updated: 2026-11-09T22:20:00Z
tags: [m25, g030-server-ssh-command, ssh, upload-pack, clean-room, verified, fw-srv-ssh]
rel:
  depends_on:
    - adr:root/20261109220230
    - plan:root/20261109220230
    - sot:root/arks-server
    - evidence:root/20261109221500
---

# M25 G30 SSH command-mode (FW-SRV-SSH)

## Context

Duplex gitprotocol-pack via `arks upload-pack` / `arks receive-pack` (same wire as SSH ForceCommand). No product SSHD. Unlock [[adr:root/20261109220230]].

## Environment

| Field | Value |
|-------|-------|
| Compose | `verify/docker-compose.yml` service `g030-server-ssh-command` |
| Image | `arks-verify-m1:0.1.0` |
| Image Id | `sha256:b235193c9ae45991f057aec1551feb769bf796ca917af1745110917cf53dfada` |
| Prior tip | `ad36bb767f7ff723d5ae532a05fe10b498a80b51` |

## Results

| Gate | Command | Outcome |
|------|---------|---------|
| **G30** | `docker compose … run --rm g030-server-ssh-command` | **PASS** — stock git clone/fetch/push via `--upload-pack` / `--receive-pack` shims; pack ad omits HTTP `# service=` |

## Non-claims

- Product SSHD / key CA not claimed.
- G31/G32 not claimed by this receipt.
