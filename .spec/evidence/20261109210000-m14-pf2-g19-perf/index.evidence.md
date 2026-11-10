---
id: evidence:root/20261109210000
title: M14 Pf-2 clean-room G19 comparative performance
status: active
created: 2026-11-09T21:00:00Z
updated: 2026-11-10T00:57:44Z
tags: [g19, m14, pf-2, clean-room, comparative-perf, verified]
rel:
  depends_on: [plan:root/20261109024636, evidence:root/20261109170000, evidence:root/20261109203000]
  evidence: []
files:
  - {path: receipt.json, sha256: 91a16777681d3cb99a861eeba854b7e3d6b1a369a1e9cfce680a851430d7eb04, bytes: 789, store: repo}
---

# M14 Pf-2 — clean-room G19 comparative ≥20%

Closes Wave **Pf-2** of [[plan:root/20261109024636]]: expands allow-list with **J-REPACK-01** (M15 Fs-3 `repack -ad` multi-object) while regressing **J-SUBMODULE-01**. G16 functional green (J-REFUSE-02 retired → J-REPACK-01).

| Journey | git_median_ms | arks_median_ms | ratio | pass |
|---------|---------------|----------------|-------|------|
| J-SUBMODULE-01 | 464 | 153 | 0.3297 | yes |
| J-REPACK-01 | 33 | 26 | 0.7879 | yes |

| Field | Value |
|-------|-------|
| Compose | `g19` |
| Image | `arks-verify-m1:0.1.0` / `sha256:bab37bd1ea03832a5ca30aa41e598eafc8bef7a5b1026d65acefb75e3f4a73c1` |
| Stock Git | `git version 2.39.5` |
| Iterations / aggregation | 3 / `median_p95` |
| Tree SHA at G19 land | `62952f6ddc456c013cca490c602161d60efc66df` |

Reproduce: `docker compose -f verify/docker-compose.yml run --rm g19`
