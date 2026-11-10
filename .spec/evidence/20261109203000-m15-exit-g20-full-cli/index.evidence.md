---
id: evidence:root/20261109203000
title: M15 exit — full CLI symmetric parity (G20 Fs-1..Fs-5)
status: active
created: 2026-11-09T20:30:00Z
updated: 2026-11-10T00:57:44Z
tags: [g020-cli-symmetric-parity, m15, exit, full-cli, clean-room, verified]
rel:
  depends_on: [plan:root/20261109035335, sot:root/cli-flag-parity-full, sot:root/program-roadmap, sot:root/production-residual-register, evidence:root/20261109180000, evidence:root/20261109183000, evidence:root/20261109190000, evidence:root/20261109193000, evidence:root/20261109200000]
  evidence: []
files:
  - {path: receipt.json, sha256: 14d08a0131139b5ccc1bec4e4ca816020f019bde97915bd3467c814e51456910, bytes: 1325, store: repo}
---

# M15 exit — full CLI symmetric parity

Closes milestone **M15** (Phase Fs) of [[sot:root/program-roadmap]]: active `alias git=arks` bar is **full** git(1) symmetric inventory (− product exclusions), proven by Docker **G20** for Fs-1..Fs-5 + G14 regression. M9 historical exit frozen. Residual register Full CLI row Closed.

## Wave evidence

| Wave | Evidence | Focus |
|------|----------|-------|
| Fs-1 | [[evidence:root/20261109180000]] | amend / hooks / merge --no-ff / plumbing |
| Fs-2 | [[evidence:root/20261109183000]] | diverged three-way merge |
| Fs-4 | [[evidence:root/20261109190000]] | add -A / status --porcelain |
| Fs-3 | [[evidence:root/20261109193000]] | notes / sparse-checkout / gc·repack lease / excludes |
| Fs-5 | [[evidence:root/20261109200000]] | range-diff / commit -a / interactive exclude |

## Exit receipt (this folder)

| Suite | pass |
|-------|------|
| G20 Fs-1..Fs-5 (15 tests) | yes |
| G14 regression | yes |

## Environment

| Field | Value |
|-------|-------|
| Compose | `g020-cli-symmetric-parity` |
| Image | `arks-verify-m1:0.1.0` / `sha256:bab37bd1ea03832a5ca30aa41e598eafc8bef7a5b1026d65acefb75e3f4a73c1` |
| Stock Git | `git version 2.39.5` |
| Tree SHA at M15 exit land | `6faa00dad0bc00b688315c4026f0842c47101066` |

## Exclusions (unchanged)

GUI/`gitk`/`git-gui`/`git-citool`, Git LFS, `scalar`, CELIR admin, forge PR/Checks/Apps UX, credential-helper/askpass, `git daemon`/`git://`, lease admin/L3 valves.

## Reproduce

```bash
docker compose -f verify/docker-compose.yml run --rm g020-cli-symmetric-parity
spec check
```
