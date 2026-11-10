---
id: evidence:root/20261109213000
title: M14 exit — comparative ≥20% on frozen G19 allow-list
status: active
created: 2026-11-09T21:30:00Z
updated: 2026-11-09T22:48:41Z
tags: [g19, m14, exit, comparative-perf, clean-room, verified]
rel:
  depends_on: [plan:root/20261109024636, sot:root/program-roadmap, evidence:root/20261109170000, evidence:root/20261109210000, evidence:root/20261109203000]
  evidence: []
files:
  - {path: receipt.json, sha256: 6245e7f6135f1165332f68b573150099b4640302c9692e1aef663f0cdf6ee9f8, bytes: 821, store: repo}
---

# M14 exit — comparative performance unlock

Closes milestone **M14** (Phase Pf) of [[sot:root/program-roadmap]]: frozen G19 allow-list proves arks **≥20% faster** (median wall-clock) than in-image stock Git under `median_p95` / release. Does **not** redefine M9–M13 or M15. Remaining G16 journeys stay Open for post-M14 residual expansion.

## Allow-list (frozen)

| Journey | Wave | ratio | pass |
|---------|------|-------|------|
| J-SUBMODULE-01 | Pf-1 | 0.3297 | yes |
| J-REPACK-01 | Pf-2 | 0.7879 | yes |

Wave evidence: [[evidence:root/20261109170000]] (Pf-1), [[evidence:root/20261109210000]] (Pf-2). This receipt is the authoritative Docker G19 run that closes the frozen set.

## Environment

| Field | Value |
|-------|-------|
| Compose | `g19` → G16 timing + allow-list gate |
| Image | `arks-verify-m1:0.1.0` / `sha256:bab37bd1ea03832a5ca30aa41e598eafc8bef7a5b1026d65acefb75e3f4a73c1` |
| Stock Git | `git version 2.39.5` |
| Iterations / aggregation | 3 / `median_p95` |
| Tree SHA at M14 exit land | `62952f6ddc456c013cca490c602161d60efc66df` |

## Reproduce

```bash
docker compose -f verify/docker-compose.yml run --rm g19
spec check
```
