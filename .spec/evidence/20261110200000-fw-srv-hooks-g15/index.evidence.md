---
id: evidence:root/20261110200000
title: FW-SRV-HOOKS G15 receive-pack githooks honor list
status: active
created: 2026-11-10T08:00:00Z
updated: 2026-11-10T08:00:00Z
tags: [m11, g015-smart-http, githooks, receive-pack, clean-room, verified, fw-srv-hooks, pre-receive, update, post-receive]
rel:
  depends_on:
    - sot:root/arks-server
    - sot:root/production-residual-register
    - sot:root/interop-test-matrix
    - sot:root/product-verification-clean-room
    - evidence:root/20261108231300
---

# FW-SRV-HOOKS — G15 additive M11-C14 Claimed

## Context

Claim receive-pack server-side githooks honor list on arks smart HTTP: `pre-receive`, `update`, `post-receive` (sync only; ignore `*.sample`). Primary: [githooks](https://git-scm.com/docs/githooks), [git-receive-pack](https://git-scm.com/docs/git-receive-pack). Unit SoT: [[sot:root/arks-server]] §Receive-pack githooks. Matrix row **M11-C14** in `verify/matrices/matrix-m11.md`. Residual **FW-SRV-HOOKS**. Does **not** invent G33+; additive under frozen G15.

## Environment

| Field | Value |
|-------|-------|
| Compose | `verify/docker-compose.yml` service `g015-smart-http` |
| Image | `arks-verify-m1:0.1.0` |
| Image Id | `sha256:b235193c9ae45991f057aec1551feb769bf796ca917af1745110917cf53dfada` |
| Stock Git | `git version 2.39.5` |
| rustc | `rustc 1.99.1 (58cf8f9c3 2026-10-13)` |
| Tree (pre-commit tip) | `eea8ebaa92b287e777657700bdefbdae4fec2bd1` |
| Honor list | `pre-receive`, `update`, `post-receive` |

## Results

| Gate | Command | Outcome |
|------|---------|---------|
| **G15** (full suite + M11-C14) | `docker compose -f verify/docker-compose.yml run --rm g015-smart-http` | **PASS** |
| arks-server units | includes `receive_pre_receive_*` / `receive_update_*` / `receive_post_receive_*` | **PASS** |
| `g015-smart-http_receive_pack_githooks_honor_list` | stock Git push vs rejecting/allowing hooks | **PASS** |
| G14 / G7 / G5 regression | embedded in `run-g015-smart-http.sh` | **PASS** |

## Non-claims

- `proc-receive` / push-options / `post-update` / quarantine object dir — not claimed.
- Host Windows bash path quirks are advisory; clean-room Linux exec of hook path is authoritative.
