---
id: evidence:root/20261109200000
title: M15 Fs-5 clean-room G20 range-diff and interactive exclude
status: active
created: 2026-11-09T20:00:00Z
updated: 2026-11-10T00:57:44Z
tags: [g20, m15, fs-5, clean-room, range-diff, interactive, verified]
rel:
  depends_on: [plan:root/20261109035335, sot:root/cli-flag-parity-full, evidence:root/20261109193000]
  evidence: []
files:
  - {path: receipt.json, sha256: 35f5ec90da9edd6b5101bfba8c0f1a494c4b9078b28eb8d2e820ec526632d97e, bytes: 1325, store: repo}
---

# M15 Fs-5 — range-diff + interactive/editor honesty

Closes Wave **Fs-5** of [[plan:root/20261109035335]]: `range-diff` subject pairwise compare, `commit -a`, explicit exclude for editor `commit` / `rebase -i`. Credential-helper and `git daemon` remain product exclusions. G20 PASS including Fs-1..4 + G14.

| ID | pass |
|----|------|
| M15-FS5-01 range-diff | yes |
| M15-FS5-02 commit -a | yes |
| M15-FS5-03 interactive/editor exclude | yes |

| Field | Value |
|-------|-------|
| Compose | `g20` |
| Image | `arks-verify-m1:0.1.0` / `sha256:bab37bd1ea03832a5ca30aa41e598eafc8bef7a5b1026d65acefb75e3f4a73c1` |
| Stock Git | `git version 2.39.5` |
| Tree SHA at G20 land | `1fff17f3218c152903c0066921269911ffadd2c1` |

Reproduce: `docker compose -f verify/docker-compose.yml run --rm g20`
