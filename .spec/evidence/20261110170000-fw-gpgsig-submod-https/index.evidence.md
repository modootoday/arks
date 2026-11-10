---
id: evidence:root/20261110170000
title: FW-GPGSIG read + FW-SUBMOD-HTTPS Claimed (G7 clean-room)
status: active
created: 2026-11-10T16:50:00Z
updated: 2026-11-10T16:50:00Z
tags: [fw-gpgsig, fw-submod-https, g7, clean-room, claimed, residual]
rel:
  depends_on:
    - sot:root/arks-object
    - sot:root/clean-room-journey-compare
    - sot:root/production-residual-register
    - sot:root/product-verification-clean-room
    - sot:root/interop-test-matrix
  evidence: []
files: []
---

# FW-GPGSIG read + FW-SUBMOD-HTTPS Claimed

## Context

Closes residual **P0** implementables:

| ID | Claim |
|----|-------|
| **FW-GPGSIG** | **Claimed (read)** — commit decode skips `gpgsig` / `encoding` / `mergetag` with SP-prefixed continuations ([gitformat-signature](https://git-scm.com/docs/gitformat-signature)). **Refuse** create (`commit -S`) and cryptographic `verify-commit`. |
| **FW-SUBMOD-HTTPS** | **Claimed** — `arks submodule add` over loopback smart HTTP via `arks-remote` `clone_http` ([git-submodule(1)](https://git-scm.com/docs/git-submodule)). |

## Environment

| Field | Value |
|-------|-------|
| Compose | `verify/docker-compose.yml` service `g7` |
| Entrypoint | `verify/scripts/run-g7.sh` |
| Image | `arks-verify-m1:0.1.0` |
| Stock Git | `git version 2.39.5` |
| Discrete-circuit mount | `/discrete-circuit` (parse-time optional path-deps; feature OFF) |

## Outcome

| Gate | Result |
|------|--------|
| G7 (includes `submodule_add_https_smart_http_loopback` + `arks-object` / G1 regressions) | **PASS** |
| Host advisory `cargo test -p arks-object commit::` | PASS (gpgsig multiline + encoding) |

No fake live forge green. G28 unsigned **create** honesty retained; signed-commit **read** unblocked.
