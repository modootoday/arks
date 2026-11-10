---
id: evidence:root/20261109205534
title: M24 G23 CELIR dual-path Git verified clean-room
status: active
created: 2026-11-09T20:55:34Z
updated: 2026-11-10T00:57:44Z
tags: [m24, g023-celir-dual-path, celir, dual-path, clean-room, verified, porcelain]
rel:
  depends_on: [sot:root/celir-dual-path-git-verified, adr:root/20261109204916, plan:root/20261109204933, sot:root/product-verification-clean-room, sot:root/celir-index-dual-path, sot:root/storage-celir-boundary, evidence:root/20261109204428]
files:
  - {path: receipt.json, sha256: 27eaa8fb6842b751b4f0b1d459849ee0e26e936ac8e7329846a018ebb4c297a4, bytes: 443, store: repo}
---

# M24 — G23 CELIR dual-path Git verified

## Authority

| Field | Value |
|-------|--------|
| Gate | **G23** (`verify/scripts/run-g023-celir-dual-path.sh` / compose `g023-celir-dual-path`) |
| Matrix | `verify/matrices/matrix-g023-celir-dual-path.md` |
| Claim | **CELIR dual-path Git verified** ([[adr:root/20261109204916]]) |
| ObjectStore | filesystem `.git` only |
| CELIR role | index sidecar only |
| Stock git | `git version 2.39.5` |
| rustc | `1.99.1` |
| Result | **PASS** |
| Parent tip | `b11b952` |

## Suite

| Step | Result |
|------|--------|
| Default `arks-store` CELIR-free | OK |
| `cargo build -p arks` + porcelain init/add/commit/status/log | PASS |
| Same-repo rebuild/publish/open/query + stock `git` open | PASS |
| ObjectStore filesystem-only | PASS |

## Honesty

- **Verified:** CELIR dual-path Git (FS sole ObjectStore + CELIR index sidecar).
- **Refuse retained:** CELIR as ObjectStore / put/get/exists.
- Not a claim that CELIR stores Git objects.
