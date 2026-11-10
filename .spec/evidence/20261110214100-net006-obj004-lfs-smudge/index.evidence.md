---
id: evidence:root/20261110214100
title: NET-006 / OBJ-004 / in-process LFS smudge host evidence
status: active
created: 2026-11-10T21:41:00Z
updated: 2026-11-10T21:41:00Z
tags: [net-006, obj-004, lfs, smudge, full-symmetry]
rel:
  depends_on:
    - adr:root/20261110214000
    - research:root/20261110212414
  evidence: []
files: []
---

# NET-006 / OBJ-004 / LFS smudge evidence

Host smoke (Windows) vs tip after [[adr:root/20261110214000]].

| Slice | How | Result |
|-------|-----|--------|
| **OBJ-004** fsck OidMismatch | Corrupt loose zlib under existing OID path → `arks fsck` | **PASS** (refuse with `OBJ-004`) |
| **OBJ-004** sha256 CLI refuse | `extensions.objectFormat=sha256` + format v1 → `arks status` | **PASS** (refuse with `OBJ-004`; library sha256 remains Claimed) |
| **LFS** in-process smudge | Pointer blob + `.git/lfs/objects/…` → `checkout --` / `reset --hard` | **PASS** (worktree = payload) |
| **LFS** unit | `arks-remote` `lfs_smudge_tests` | **PASS** (2/2) |
| **NET-006** HTTP bundle-uri | `clone … --bundle-uri http://127.0.0.1:…/seed.bundle` | **PASS** (download + unbundle note) |
| **NET-006** lock race | Plant `arks-bundle-uri/<key>.lock` → second clone | **PASS** (refuse with `NET-006`) |

Non-claims: full LFS network transfer/server; cryptographic SHA-1 collision immunity; server `bundle-uri` capability advertise; full sha256 porcelain CLI.
