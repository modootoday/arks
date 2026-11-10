---
id: evidence:root/20261109180000
title: M15 Fs-1 clean-room G20 full CLI symmetric (amend/hooks/merge-no-ff/plumbing)
status: active
created: 2026-11-09T18:00:00Z
updated: 2026-11-09T22:48:33Z
tags: [g20, m15, fs-1, clean-room, cli-full-parity, verified]
rel:
  depends_on: [plan:root/20261109035335, sot:root/cli-surface, sot:root/cli-flag-parity-full, sot:root/interop-test-matrix, sot:root/program-roadmap, evidence:root/20261108150000]
  evidence: []
files:
  - {path: receipt.json, sha256: 73af1f6626f20e89302df323774295c0794035376fca39bf07f9f3690f82730c, bytes: 452, store: repo}
---

# M15 Fs-1 — clean-room G20 full CLI symmetric (wave 1)

## Context

Closes Wave **Fs-1** of [[plan:root/20261109035335]] after SoT expansion for **full Git↔arks CLI symmetric parity** (active alias bar = M15; M9 historical exit frozen). Claimed: `commit --amend`, sync hook runner, `merge --no-ff` (FF-capable), plumbing minimum (`write-tree` / `commit-tree` / `ls-files` / `ls-tree` / `symbolic-ref` / `merge-base`). Pack-aware store ingest supporting clone shipped in the same land tip. G14 regression green. Does **not** complete M15 exit (Fs-2+ remain). Does **not** redefine M9–M14 historical exits. M14/G19 continues only on functionally green units after Fs waves.

Attached: `receipt.json` (`arks-g20-cli-full-parity/v1`, wave `Fs-1`).

## Environment

| Field | Value |
|-------|-------|
| Compose | `g20` → Fs-1 suite + G14 regression |
| Image | `arks-verify-m1:0.1.0` / `sha256:bab37bd1ea03832a5ca30aa41e598eafc8bef7a5b1026d65acefb75e3f4a73c1` |
| Stock Git | `git version 2.39.5` |
| Tree SHA at G20 run | fe0795dc6205edf805ed36a469bd82ff06f4d4e0 |

## Results

| ID | pass |
|----|------|
| M15-FS1-01 commit --amend | yes |
| M15-FS1-02 sync pre-commit hook | yes |
| M15-FS1-03 merge --no-ff | yes |
| M15-FS1-04 plumbing minimum | yes |
| M15-FS1-05 G14 regression | yes (suite) |

## Non-claims

Diverged recursive merge (Fs-2); former refuse-closed porcelain (Fs-3); remaining flag gaps (Fs-4+); M14 ≥20%; L3; CELIR.

## Reproduce

```bash
docker compose -f verify/docker-compose.yml run --rm g20
spec check
```
