---
id: evidence:root/20261109054500
title: M16 Sy-4 G21 client SSH transport
status: active
created: 2026-11-09T05:45:00Z
updated: 2026-11-09T05:45:00Z
tags: [g21, m16, sy-4, ssh, clean-room, verified, complete-symmetry]
rel:
  depends_on:
    - plan:root/20261109043910
    - sot:root/cli-flag-parity-complete
    - sot:root/arks-remote
    - sot:root/cli-surface
    - sot:root/interop-test-matrix
    - sot:root/program-roadmap
    - evidence:root/20261109050238
  evidence: []
files:
  - {path: receipt.json, sha256: 74ebe5b6c872a1207fa044db1133d1cf432e1417572806987b02458e4ff2baef, bytes: 621, store: repo}
---

# M16 Sy-4 — G21 client SSH transport

Closes wave **Sy-4** of [[plan:root/20261109043910]]: OpenSSH client helper for
`ssh://` and SCP-like `user@host:path` clone/fetch/push plus `submodule add`.
No product-path `git` subprocess. Server SSH command-mode remains **N/A**.

## Claim rows (PASS)

| ID | Focus |
|----|-------|
| M16-SY4-01 | `clone` / `fetch` / `push` over loopback `ssh://` fixture; SCP-like clone; stock Git peer sees push |
| M16-SY4-02 | `submodule add` over SSH URL fixture |
| Regression | G20 + G14 |

Fixture: `verify/gates/g21/README-ssh-fixture.md` (ephemeral host/client keys; no secrets in tree).

## Environment

| Field | Value |
|-------|-------|
| Compose | `g21` |
| Image | `arks-verify-m1:0.1.0` / `sha256:5736c5687d3b5775e187ca3d21fbc5b2a5a2bf8965c5381778ee478eb185adf0` |
| Stock Git | `git version 2.39.5` |
| OpenSSH | `OpenSSH_9.2p1 Debian-2+deb12u10` |

## Reproduce

```bash
docker compose -f verify/docker-compose.yml build toolchain
docker compose -f verify/docker-compose.yml run --rm g21
spec check
```
