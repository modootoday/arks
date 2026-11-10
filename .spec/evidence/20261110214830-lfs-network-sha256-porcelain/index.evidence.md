---
id: evidence:root/20261110214830
title: LFS network batch/server + sha256 porcelain host evidence
status: active
created: 2026-11-10T21:48:30Z
updated: 2026-11-10T21:48:30Z
tags: [lfs, sha256, network, porcelain, full-symmetry]
rel:
  depends_on:
    - adr:root/20261110214800
    - research:root/20261110212414
  evidence: []
files:
  - host-e2e.txt
---

# LFS network + sha256 porcelain evidence

Host smoke (Windows) vs tip after [[adr:root/20261110214800]]. Receipt: `host-e2e.txt`.

| Slice | How | Result |
|-------|-----|--------|
| **sha256 porcelain** | `arks init --object-format=sha256`; add/commit/log/fsck/hash-object; checkout restore | **PASS** (64-hex OIDs; fsck ok) |
| **LFS server batch+GET** | `arks-server` unit `lfs_batch_download_and_get_object` | **PASS** |
| **LFS server batch+PUT** | unit `lfs_batch_upload_and_put_object` (PUT before DAV) | **PASS** |
| **LFS network fetch** | `serve-http` + `ARKS_LFS_URL` → `arks lfs fetch` stores under `$GIT_DIR/lfs/objects` | **PASS** |
| **LFS network smudge** | delete local LFS cache → `checkout --` batch+GET | **PASS** (worktree = payload) |
| **LFS network push** | local object → `arks lfs push` batch+PUT on server | **PASS** |

Unit: local smudge still **PASS** (`lfs_smudge_tests` 2/2).

**Still Impossible (not claimed):** SHA-1 collision immunity (crypto); forge Checks/PR host UX; L9 / live G8 non-FS; host/env FS-007 / FS-014 / WIN-003.
