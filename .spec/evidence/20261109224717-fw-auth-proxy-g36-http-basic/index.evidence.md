---
id: evidence:root/20261109224717
title: FW-AUTH-PROXY G36 HTTP Basic Claimed (Docker PASS)
status: active
created: 2026-11-09T22:47:17Z
updated: 2026-11-10T07:58:30Z
tags: [fw-auth-proxy, g36, http-basic, clean-room, verified, claimed]
rel:
  depends_on:
    - adr:root/20261109224236
    - sot:root/arks-server
    - sot:root/interop-test-matrix
    - sot:root/product-verification-clean-room
    - evidence:root/20261108231300
---

# FW-AUTH-PROXY G36 HTTP Basic

## Context

Optional in-process HTTP Basic on `arks serve-http` / `http-backend` ([[adr:root/20261109224236]]). Additive gate **G36**. Forge OAuth remains Refuse. G15 unauthenticated smart HTTP remains Claimed ([[evidence:root/20261108231300]]).

## Host advisory

| Check | Outcome |
|-------|---------|
| `cargo test -p arks-server` | **PASS** — includes Basic challenge/accept + hooks honor tests |
| `cargo test --manifest-path verify/g36/Cargo.toml` | **PASS** (4) — 401 challenge, URL userinfo clone, wrong/right cred, unauth default |

## Docker sole-runner (product authority)

| Field | Value |
|-------|-------|
| Compose | `verify/docker-compose.yml` service `g36` |
| Entrypoint | `verify/scripts/run-g36.sh` |
| Matrix | `verify/matrix-g36.md` |
| Image | `arks-verify-m1:0.1.0` |
| Image Id | `sha256:b235193c9ae45991f057aec1551feb769bf796ca917af1745110917cf53dfada` |
| Tip at verify | `d18c93f3254dc6f3dd2067f8922416d8e120cd69` |
| Outcome | **PASS** — G36-C01..C04 |

## Non-claims

- Forge OAuth / GitHub App / session-cookie auth server remain **Refuse-closed**.
- Apache `htpasswd` crypt/bcrypt algorithm parity not claimed (plaintext credential file only).
