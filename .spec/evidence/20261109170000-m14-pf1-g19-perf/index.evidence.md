---
id: evidence:root/20261109170000
title: M14 Pf-1 clean-room G19 comparative performance
status: active
created: 2026-11-09T17:00:00Z
updated: 2026-11-10T00:57:43Z
tags: [g19, m14, pf-1, clean-room, comparative-perf, verified]
rel:
  depends_on: [plan:root/20261109024636, sot:root/clean-room-journey-compare, sot:root/interop-test-matrix, sot:root/public-surface, evidence:root/20261109163000, evidence:root/20261109140000]
  evidence: []
files:
  - {path: receipt.json, sha256: 4d931320dc9bdc45493889e8a0e7826c13c08e5e42ca408383544a62f95bf056, bytes: 625, store: repo}
---

# M14 Pf-1 — clean-room G19 comparative ≥20%

## Context

Closes Wave **Pf-1** of [[plan:root/20261109024636]] after M12+M13 claimed-set exit. Allow-list: `J-SUBMODULE-01` only (stable ≤0.8×). `J-CLONE-LOCAL-01` deferred (run-to-run noise). Public copy MUST cite this receipt; no marketing outside receipts.

Attached: `receipt.json` (`arks-g19-perf-comparative/v1`, wave `Pf-1`).

## Environment

| Field | Value |
|-------|-------|
| Compose | `g19` → G16 timing + allow-list gate |
| Image | `arks-verify-m1:0.1.0` / `sha256:bab37bd1ea03832a5ca30aa41e598eafc8bef7a5b1026d65acefb75e3f4a73c1` |
| Stock Git | `git version 2.39.5` |
| Iterations / aggregation | 3 / `median_p95` |
| Tree SHA at G19 run | `dedbf012654fb6c085fa59a88fcd953c76daabf1`
| Tree SHA (evidence tip) | `e92e04a` |

## Results

| Journey | git_median_ms | arks_median_ms | ratio | pass |
|---------|---------------|----------------|-------|------|
| J-SUBMODULE-01 | 498 | 166 | 0.3333 | yes |

## Non-claims

Does not redefine M9–M13. Does not claim full G16 allow-list ≥20%. Pf-2+ required to expand.

## Reproduce

```bash
docker compose -f verify/docker-compose.yml run --rm g19
spec check
```
