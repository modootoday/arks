---
id: evidence:root/20261109183000
title: M15 Fs-2 clean-room G20 diverged three-way merge
status: active
created: 2026-11-09T18:30:00Z
updated: 2026-11-09T22:48:40Z
tags: [g20, m15, fs-2, clean-room, merge, verified]
rel:
  depends_on: [plan:root/20261109035335, sot:root/cli-flag-parity-full, evidence:root/20261109180000]
  evidence: []
files:
  - {path: receipt.json, sha256: 96abc92ef52f48d2028e66cf68009647f0e6f28ab0bb9d503e66ed6ea9943067, bytes: 627, store: repo}
---

# M15 Fs-2 — clean-room G20 diverged three-way merge

## Context

Closes Wave **Fs-2** of [[plan:root/20261109035335]]: diverged recursive three-way merge (clean auto-merge + conflicted index/markers) and non-ff `pull` merge path. Fs-1 rows re-verified in the same G20 run. G14 regression green. Does **not** complete M15 exit (Fs-3+ remain).

## Environment

| Field | Value |
|-------|-------|
| Compose | `g20` |
| Image | `arks-verify-m1:0.1.0` / `sha256:bab37bd1ea03832a5ca30aa41e598eafc8bef7a5b1026d65acefb75e3f4a73c1` |
| Stock Git | `git version 2.39.5` |
| Tree SHA at G20 land | `986335af3ff8dec382ed200423db212715636bde` |

## Results

| ID | pass |
|----|------|
| M15-FS2-01 diverged clean merge | yes |
| M15-FS2-02 diverged conflict | yes |
| Fs-1 rows | yes (regression) |
| G14 | yes |

## Reproduce

```bash
docker compose -f verify/docker-compose.yml run --rm g20
```

