---
id: evidence:root/20261110223000
title: FW-GH-LIVE-SCALE Claimed (harness ready; no-measurement without secrets)
status: active
created: 2026-11-10T22:30:00Z
updated: 2026-11-10T00:00:20Z
tags: [g8, fw-gh-live-scale, claimed, harness, skip, no-measurement]
rel:
  depends_on: [adr:root/20261110221000, adr:root/20261110220000, sot:root/interop-test-matrix, sot:root/production-residual-register]
  evidence: []
files:
  - {path: receipt.json, sha256: 6a119a9a3f613d8a0178ae9eda67e8c1c58eb661eb8c7f651056d11b57f1fcc3, bytes: 418, store: repo}
  - {path: samples.json, sha256: 0f1a4d82b032a10d939d9d362ed9113af43b7e063b72fd5500abae3d6e4730d7, bytes: 16, store: repo}
---

# FW-GH-LIVE-SCALE — Claimed (harness ready; no-measurement)

## Context

Supersedes Closed (harness + opt-in) ([[adr:root/20261110161000]]) with [[adr:root/20261110221000]]: scale harness **Claimed**; records real `ls-remote` samples when secrets exist; honest **no-measurement** when absent. Never invent latency SLOs.

## Environment

| Field | Value |
|-------|-------|
| Compose | `verify/docker-compose.yml` service `g8-live-scale` |
| Entrypoint | `verify/scripts/run-g8-live-scale.sh` |
| Matrix | `verify/matrix-g8-live-scale.md` |
| Secrets in env | absent |
| Disposition | `skip-no-measurement` |
| Samples | `[]` |

## Outcome

| Claim | Result |
|-------|--------|
| **FW-GH-LIVE-SCALE** | **Claimed (harness ready)** |
| Live measurement this run | **Did not run** (honest no-measurement) |
| Gate exit | **0** |

No invented medians or product SLOs. Force-off remains `ARKS_G8_SCALE_MEASURE=0`.
