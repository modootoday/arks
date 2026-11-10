---
id: adr:root/20261110223000
title: OBJ-004 sha1cd detect+refuse + FS-007/WIN-003 host detect advisory
status: accepted
created: 2026-11-10T22:30:00Z
updated: 2026-11-10T22:30:00Z
tags: [obj-004, sha1cd, fs-007, win-003, hostfs, advisory, claimed]
rel:
  depends_on:
    - research:root/20261110222000
    - evidence:root/20261110223500
    - evidence:root/20261110221919
    - sot:root/production-residual-register
---

# OBJ-004 sha1cd detect+refuse + host cloud detect advisory

## Decision

1. **OBJ-004 Claimed (detect+refuse):** replace classic `sha1` in `arks-hash` with `sha1collisiondetection` (`safe_hash=false`, `detect_collisions=true`). `Hasher::finalize` / `hash_object` / `hash_bytes` return `Err(HashError::Sha1Collision)` when a cryptanalytic collision attack is detected. Absolute cryptographic **immunity** remains **Impossible**.
2. **FS-007 / WIN-003 Advisory:** ship `arks-hostfs` + `arks host-env` for DriveFS volume-label detect, OneDrive path / CfAPI `RECALL_ON_DATA_ACCESS` detect, Dropbox/Syncthing markers. Optional compose `fs007-host-advisory`. Do **not** Claim full multi-device cloud-sync safety or OneDrive hydration.
3. Live G8 remains secrets-blocked; L9 remains owner-closed.

## Consequences

- Residual **FW-CAT-OBJ-004** cites sha1cd Claimed detect+refuse.
- Residual **FW-CAT-FS-007** / **FW-CAT-WIN-003** stay Impossible for full hazard closure; detect slices are advisory with evidence.
- SHAttered PDF fixture proves CD wiring on raw streams; file-level collisions may not flag after `blob <size>\\0` alignment shift (documented honesty).
