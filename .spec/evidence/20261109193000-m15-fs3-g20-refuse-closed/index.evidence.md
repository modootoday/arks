---
id: evidence:root/20261109193000
title: M15 Fs-3 clean-room G20 former refuse-closed porcelain
status: active
created: 2026-11-09T19:30:00Z
updated: 2026-11-09T22:48:40Z
tags: [g20, m15, fs-3, clean-room, notes, sparse-checkout, repack, gc, verified]
rel:
  depends_on: [plan:root/20261109035335, sot:root/cli-flag-parity-full, evidence:root/20261109190000]
  evidence: []
files:
  - {path: receipt.json, sha256: 216923e212fd9486128612302e77031e0f4d8a648eb427fd39e80846a0db236a, bytes: 1147, store: repo}
---

# M15 Fs-3 — former refuse-closed porcelain

Closes Wave **Fs-3** of [[plan:root/20261109035335]]: `notes` add/show, cone `sparse-checkout`, lease-honest `repack`/`gc`/`maintenance run`, explicit exclude for `filter-branch`/`backfill`. `range-diff` remains Open for Fs-5. G20 PASS including Fs-1/2/4 + G14 regression. M9 historical exit frozen.

| ID | pass |
|----|------|
| M15-FS3-01 notes add/show | yes |
| M15-FS3-02 sparse-checkout cone | yes |
| M15-FS3-03 repack -ad + maint:gc lease | yes |
| M15-FS3-04 gc / --prune=now refuse | yes |
| M15-FS3-05 filter-branch/backfill exclude | yes |

| Field | Value |
|-------|-------|
| Compose | `g20` |
| Image | `arks-verify-m1:0.1.0` / `sha256:bab37bd1ea03832a5ca30aa41e598eafc8bef7a5b1026d65acefb75e3f4a73c1` |
| Stock Git | `git version 2.39.5` |
| Tree SHA at G20 land | `0009ef0726b4623910f6a764fb0901924b10952a` |

Reproduce: `docker compose -f verify/docker-compose.yml run --rm g20`
