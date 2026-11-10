---
id: evidence:root/20261110073800
title: FW-GPGSIG SSH create/verify Claimed (G37 clean-room PASS)
status: active
created: 2026-11-10T07:38:00Z
updated: 2026-11-10T08:00:00Z
tags: [fw-gpgsig, g37, ssh, commit-sign, verify-commit, clean-room, claimed]
rel:
  depends_on:
    - sot:root/arks-object
    - sot:root/production-residual-register
    - sot:root/product-verification-clean-room
    - sot:root/interop-test-matrix
    - evidence:root/20261110170000
  evidence: []
files: []
---

# FW-GPGSIG SSH create/verify Claimed

## Context

Closes residual **FW-GPGSIG** create/verify for the **SSH** subset (`gpg.format=ssh`). Read path already Claimed ([[evidence:root/20261110170000]]). OpenPGP/`gpg.format=openpgp` and X.509 remain **N/A**.

| ID | Claim |
|----|-------|
| **FW-GPGSIG** | **Claimed (read+create+verify SSH)** — encode/decode `gpgsig` SP continuations; `arks commit -S` / `commit.gpgSign` + `arks verify-commit` via `gpg.ssh.program` (default `ssh-keygen`) `-Y sign` / `-Y verify`. |

Primary: [gitformat-signature](https://git-scm.com/docs/gitformat-signature), [git-commit `-S`](https://git-scm.com/docs/git-commit), [git-verify-commit](https://git-scm.com/docs/git-verify-commit), [git-config `gpg.format` / `gpg.ssh.*`](https://git-scm.com/docs/git-config).

## Environment

| Field | Value |
|-------|-------|
| Compose | `verify/docker-compose.yml` service `g37` (`--no-deps`) |
| Entrypoint | `verify/scripts/run-g37.sh` |
| Matrix | `verify/matrices/matrix-g37.md` |
| Image | `arks-verify-m1:0.1.0` |
| Stock Git | `git version 2.39.5` |
| OpenSSH | `OpenSSH_9.2p1 Debian-2+deb12u10` |
| Keys | Ephemeral ed25519 generated inside container `/tmp` only — never tracked |

## Outcome

| Gate | Result |
|------|--------|
| G37 `g37_ssh_commit_sign_verify_stock_roundtrip` | **PASS** (arks `commit -S` + `verify-commit` + stock `git verify-commit`) |
| G37 `g37_refuse_openpgp_format` | **PASS** |
| Host advisory `cargo test -p arks-object commit::` | PASS (advisory) |

Product verification authority: Docker clean-room G37 **PASS**. No private keys committed. OpenPGP/X.509 remain refuse-closed.
