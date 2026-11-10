---
id: evidence:root/20261109213400
title: G28 offline clone journeys (J-CLONE-LOCAL-01 / J-CLONE-PATH-01)
status: active
created: 2026-11-09T21:34:00Z
updated: 2026-11-10T00:57:44Z
tags: [g28, clean-room, journey, offline, clone, smart-http, file, verified]
rel:
  depends_on: [sot:root/g28-live-private-arks-journeys, plan:root/20261109212020, sot:root/product-verification-clean-room, sot:root/interop-test-matrix, sot:root/clean-room-journey-compare, sot:root/production-residual-register, evidence:root/20261109212500]
files:
  - {path: receipt.json, sha256: 9f15cc2173336b2c0d0189e6af94b774b7b1077f3aa552f6c89720b1dd605dc7, bytes: 1091, store: repo}
  - {path: journeys.json, sha256: e26b024efc79aef0012f5c8885ce6524d975164818d0ccbc9d6ac53f1c279b2b, bytes: 253, store: repo}
---

# G28 — offline clone journeys

## Authority

| Field | Value |
|-------|--------|
| Gate | **G28** additive offline closure (`verify/scripts/run-g28-offline-clones.sh` / compose `g28-offline-clones`) |
| Matrix | `verify/matrices/matrix-g28.md` |
| Stock git (image) | `git version 2.39.5` |
| Image | `arks-verify-m1:0.1.0` |
| Parent tip | `a60e82e` |
| Result | **PASS** (`disposition=closed-implement`, PASS=2 FAIL=0) |
| Seed align | G16 `J-CLONE-LOCAL-01` / `J-CLONE-PATH-01` (git `http-backend` peer; path + `file://`; README+bulk bare) |

## Journey table

| ID | Status |
|----|--------|
| J-CLONE-LOCAL-01 | **PASS** (arks clone smart-HTTP local peer) |
| J-CLONE-PATH-01 | **PASS** (arks path + `file://` clone) |

## Honesty

- Closes the two G28 checklist rows previously recorded **N/A** on the live forge run ([[evidence:root/20261109212500]]).
- Offline functional only — **not** G16 timing/perf compare; **not** live GitHub; **not** G26/G27 scale/contention.
- Live-private journey Claimed status remains grounded in [[evidence:root/20261109212500]]; this receipt is the additive offline clone gate.
