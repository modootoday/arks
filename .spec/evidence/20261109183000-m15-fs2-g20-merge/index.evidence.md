---
id: evidence:root/20261109183000
title: M15 Fs-2 clean-room G20 diverged three-way merge
status: active
created: 2026-11-09T18:30:00Z
updated: 2026-11-10T00:57:43Z
tags: [g020-cli-symmetric-parity, m15, fs-2, clean-room, merge, verified]
rel:
  depends_on: [plan:root/20261109035335, sot:root/cli-flag-parity-full, evidence:root/20261109180000]
  evidence: []
files:
  - {path: receipt.json, sha256: 23202c5085bb6c7aacc3ed46648a8ba7a87dd9b7b865ecf26e029f53fb8c19ac, bytes: 590, store: repo}
---

# M15 Fs-2 — clean-room G20 diverged three-way merge

## Context

Closes Wave **Fs-2** of [[plan:root/20261109035335]]: diverged recursive three-way merge (clean auto-merge + conflicted index/markers) and non-ff `pull` merge path. Fs-1 rows re-verified in the same G20 run. G14 regression green. Does **not** complete M15 exit (Fs-3+ remain).

## Environment

| Field | Value |
|-------|-------|
| Compose | `g020-cli-symmetric-parity` |
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
docker compose -f verify/docker-compose.yml run --rm g020-cli-symmetric-parity
```

