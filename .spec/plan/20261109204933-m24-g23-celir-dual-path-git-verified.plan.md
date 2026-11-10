---
id: plan:root/20261109204933
title: M24 successor G23 CELIR dual-path Git verified
status: active
created: 2026-11-09T20:49:33Z
updated: 2026-11-10T05:49:00Z
tags: [m24, g23, celir, dual-path, clean-room, porcelain, verified]
rel:
  depends_on:
    - sot:root/celir-dual-path-git-verified
    - adr:root/20261109204916
    - sot:root/celir-index-dual-path
    - sot:root/storage-celir-boundary
    - sot:root/product-verification-clean-room
    - sot:root/interop-test-matrix
    - plan:root/20261109203430
  implements: [sot:root/celir-dual-path-git-verified, adr:root/20261109204916]
  evidence: [evidence:root/20261109205534]
---

# M24 successor — G23 CELIR dual-path Git verified

## Goal

Close the G22 honesty residual “CELIR-backed Git verified” with an honest Done: Docker clean-room proves Git porcelain/interop **while** celir-index build/publish/open/query works; FS `.git` sole ObjectStore; stock Git opens the repo.

## Done rules

| Disposition | Meaning |
|-------------|---------|
| **Verified** | Docker `g23` PASS + evidence; ObjectStore Refuse held; stock-readable |
| **FAIL / blocked** | Missing discrete-circuit, assertion failure — no fake PASS |
| **Out** | CELIR ObjectStore; default-on feature |

## Claimed set

| ID | Behavior |
|----|----------|
| **G23-C01** | arks porcelain init/add/commit/status/log on FS `.git` with stock Git peer readable |
| **G23-C02** | celir-index rebuild/publish/open/query on same worktree; lookup matches seeded blob |
| **G23-C03** | ObjectStore remains filesystem; default tree CELIR-free |
| **G23-C04** | Sole-runner lock + discrete-circuit mount |

## Waves

| Wave | Work | Exit |
|------|------|------|
| G23-0 | SoT + ADR + matrix + interop row | Docs |
| G23-1 | Harness + tests | Code ready |
| G23-exit | Docker PASS + evidence + honesty | Verified |

## Record

| Wave | Status | Tip / evidence |
|------|--------|----------------|
| G23-0 | Closed | SoT + ADR + matrix + interop |
| G23-1 | Closed | harness + g23_dual_path_git |
| G23-exit | Closed (Verified) | [[evidence:root/20261109205534]]; Docker `g23` PASS |
