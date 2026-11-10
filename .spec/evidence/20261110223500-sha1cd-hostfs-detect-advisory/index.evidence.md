---
id: evidence:root/20261110223500
title: OBJ-004 sha1cd detect+refuse + FS-007/WIN-003 host detect advisory
status: active
created: 2026-11-10T22:35:00Z
updated: 2026-11-10T22:35:00Z
tags: [obj-004, sha1cd, fs-007, win-003, hostfs, claimed, advisory]
rel:
  depends_on:
    - adr:root/20261110223000
    - research:root/20261110222000
    - evidence:root/20261110221919
  evidence: []
files:
  - summary.txt
  - fs007-host-advisory-receipt.json
---

# OBJ-004 sha1cd + host cloud detect (Claimed detect / Advisory host)

## Disposition

| Item | Class | Status |
|------|-------|--------|
| SHA-1 collision **detect+refuse** (`sha1collisiondetection`) | **Claimed** | G0 `arks-hash` SHAttered fixture refuse PASS |
| SHA-1 collision **immunity** | **Impossible** | Unchanged |
| FS-007 full multi-device sync safety | **Impossible** | Unchanged |
| FS-007 / DriveFS / sync-root **detect** | **Advisory** | `arks host-env` + `arks-hostfs`; G: prior [[evidence:root/20261110221919]] |
| WIN-003 OneDrive **hydration** | **Impossible** | Unchanged |
| WIN-003 CfAPI / path **detect** | **Advisory** | `is_onedrive_placeholder` + path heuristic; skip-if-absent |

## Proof pointers

- Docker G0 workspace tests green including `refuses_shattered_pdf_fixture_when_present`.
- `verify/scripts/run-fs007-host-advisory.sh` → `arks-fs007-host-advisory/v1` `claimed=false`.
- Unlock: [[adr:root/20261110223000]].
