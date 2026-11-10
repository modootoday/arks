---
id: evidence:root/20261110152000
title: M22 Rc-1 G8 live-forge default Refuse-closed
status: active
created: 2026-11-10T15:20:00Z
updated: 2026-11-10T15:20:00Z
tags: [g8, m22, refuse-closed, clean-room, live-forge, verified]
rel:
  depends_on: [plan:root/20261110150000, adr:root/20261110151000, sot:root/interop-test-matrix, sot:root/product-verification-clean-room, sot:root/production-residual-register]
  evidence: []
files:
  - {path: receipt.json, sha256: 0cd8563e5852970bdfa40341069629e4fe39966b85b458d1891c49acd95711f7, bytes: 449, store: repo}
---

# M22 Rc-1 — G8 default Refuse-closed

## Context

Unlocks G8 network policy ([[adr:root/20261110151000]]) and records clean-room **Refuse-closed** disposition when opt-in live secrets are absent. G7 remains default offline remote proof. No fake live green.

## Environment

| Field | Value |
|-------|-------|
| Compose | `verify/docker-compose.yml` service `g8` |
| Entrypoint | `verify/scripts/run-g8.sh` |
| Image | `arks-verify-m1:0.1.0` |
| Stock Git | `git version 2.39.5` |
| Disposition | `refuse-closed` |

## Outcome

Gate exit **0**. Receipt disposition **refuse-closed** (`opt-in secrets absent`). ADR cite `adr:root/20261110151000`.
