---
id: evidence:root/20261110222000
title: FW-G8-DEFAULT Claimed (default-on when secrets; SKIP receipt without secrets)
status: active
created: 2026-11-10T22:20:00Z
updated: 2026-11-10T00:00:20Z
tags: [g8, fw-g8-default, claimed, clean-room, skip, live-forge]
rel:
  depends_on: [adr:root/20261110220000, sot:root/interop-test-matrix, sot:root/product-verification-clean-room, sot:root/production-residual-register]
  evidence: []
files:
  - {path: receipt.json, sha256: fbc4d980ab26570da45a12df3451cd35dc3aaa7644bf0a6a5f28e4c64b3742b3, bytes: 458, store: repo}
---

# FW-G8-DEFAULT — Claimed (policy + harness; SKIP without secrets)

## Context

Supersedes refuse-closed default ([[adr:root/20261110151000]]) with [[adr:root/20261110220000]]: G8 live runs **by default** when `ARKS_G8_TOKEN`/`GITHUB_TOKEN` + `ARKS_G8_REPO_URL` are present (`ARKS_G8_LIVE=1` not required). This receipt records the honest **SKIP** path when secrets are absent — **not** a live PASS.

## Environment

| Field | Value |
|-------|-------|
| Compose | `verify/docker-compose.yml` service `g8` |
| Entrypoint | `verify/scripts/run-g8.sh` |
| Image | `arks-verify-m1:0.1.0` |
| Stock Git | `git version 2.39.5` |
| Secrets in env | absent (`ARKS_G8_TOKEN` / `GITHUB_TOKEN` / `ARKS_G8_REPO_URL` unset) |
| Disposition | `skip-secrets-absent` |

## Outcome

| Claim | Result |
|-------|--------|
| **FW-G8-DEFAULT** | **Claimed (default-on when secrets)** |
| Live forge this run | **Did not run** (honest SKIP) |
| Gate exit | **0** |

No invented live PASS. Operator force-off remains `ARKS_G8_LIVE=0`.
