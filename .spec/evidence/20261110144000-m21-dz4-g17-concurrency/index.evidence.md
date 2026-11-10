---
id: evidence:root/20261110144000
title: M21 Dz-4 clean-room G17 Deferred=0 close
status: active
created: 2026-11-10T14:40:00Z
updated: 2026-11-10T14:40:00Z
tags: [g17, m21, dz-4, clean-room, concurrency, deferred-zero, verified]
rel:
  depends_on: [plan:root/20261110140000, sot:root/program-roadmap, sot:root/interop-test-matrix, sot:root/multi-agent-git-concurrency-scenarios, sot:root/production-residual-register, sot:root/product-verification-clean-room, evidence:root/20261110143000, research:root/20261110021546]
  evidence: []
files:
  - {path: receipt.json, sha256: 822f9a2bedb7bd08b16e985e950b4b12fb3bb9dd674e265511283ccd9f64ae51, bytes: 27918, store: repo}
---

# M21 Dz-4 — clean-room G17 Deferred=0 close

## Context

Closes Wave **Dz-4** and **M21**: `CI-006`, `TOCTOU-004` + full regression (**145**/145 dual PASS). Catalog disposition: **Deferred=0 Claimed=212 Refuse=0 N/A=9** (+ L3 valve refuse-closed). CELIR excluded. No force-push. No fake greens.

## Environment

| Field | Value |
|-------|-------|
| Compose | `verify/docker-compose.yml` service `g17` |
| Entrypoint | `verify/scripts/run-g17.sh` |
| Image | `arks-verify-m1:0.1.0` |
| Image digest | `sha256:5736c5687d3b5775e187ca3d21fbc5b2a5a2bf8965c5381778ee478eb185adf0` |
| Stock Git | `git version 2.39.5` |

## Aggregate

| Metric | Value |
|--------|------:|
| Scenario rows | 145 |
| git pathology_pass | 145 |
| arks resolved_pass | 145 |
| fail outcomes | 0 |

## Non-claims

Terminal dispositions closed ≠ marketing “all Git races fixed.” Absolute PERF SLOs remain M14/G19. L3 HighConcurrency remains refuse-closed.
