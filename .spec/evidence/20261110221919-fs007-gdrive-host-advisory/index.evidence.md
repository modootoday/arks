---
id: evidence:root/20261110221919
title: FS-007 Google Drive (G:) host advisory probe
status: active
created: 2026-11-10T22:19:19Z
updated: 2026-11-10T22:19:19Z
tags: [fs-007, win-003, google-drive, drivefs, host-advisory, cloud-sync, honesty]
rel:
  depends_on:
    - research:root/20261110222000
    - sot:root/multi-agent-git-concurrency-scenarios
    - sot:root/production-residual-register
  evidence: []
files:
  - probe-receipt.json
  - probe-log.txt
---

# FS-007 — Google Drive (G:) host advisory probe (≠ Claimed)

## Disposition

| Item | Status |
|------|--------|
| Venue | Windows host `G:` — Google Drive for desktop (Drive File Stream) |
| Mode (observed) | **Streaming-likely** — virtual FAT32 volume; `mirror_item` rows = 0 |
| Cloud Filter / OneDrive placeholders | **Not observed** (DriveFS ≠ CfAPI) |
| Bounded ops | `git init` / `add` / `commit` / lock rename / cleanup OK |
| Multi-device sync races | **Not tested** |
| Specfold class | **Advisory host probe** — `claimed=false`, `live_pass=false` |
| FS-007 product Claimed | **Not earned** |

## What this proves

Single-host throwaway repo under `G:\내 드라이브\.arks-fs007-probe-*` exercised stock Git lock/rename paths on DriveFS without reparse-point placeholders. Useful for **detect / refuse / harness design**, not for inventing FS-007 Claimed or WIN-003 OneDrive parity.

## What this does not prove

- Multi-device concurrent writers / server-side conflict duplicates
- Offline hydration races / placeholder `RECALL_ON_DATA_ACCESS` (OneDrive CfAPI)
- SELinux/AppArmor lock-rename denial (FS-014)
- arks CLI parity on DriveFS (stock git only in this probe)

Unlock research: [[research:root/20261110222000]].
