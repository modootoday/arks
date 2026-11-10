---
id: plan:root/20261109212020
title: G28 live private arks journeys
status: active
created: 2026-11-09T21:20:20Z
updated: 2026-11-09T21:34:00Z
tags: [g028-live-private-arks, clean-room, journey, live, github, private, opt-in, offline-clone]
rel:
  depends_on:
    - sot:root/g028-live-private-arks-live-private-arks-journeys
    - sot:root/product-verification-clean-room
    - sot:root/interop-test-matrix
    - sot:root/clean-room-journey-compare
    - sot:root/production-residual-register
    - adr:root/20261110151000
  implements: [sot:root/g028-live-private-arks-live-private-arks-journeys]
  evidence: [evidence:root/20261109212500, evidence:root/20261109213400]
---

# G28 — live private arks journeys

## Goal

Drive **arks** through **G16-derived normal-user journeys** against a **temporary private GitHub** repo in Docker clean-room with **network opt-in**. Completeness of journey coverage — **not** contention/scale. Offline-only clone rows close via additive `g028-offline-clones`.

## Done rules

| Disposition | Meaning |
|-------------|---------|
| **Verified / Claimed** | Docker `g028-live-private-arks` PASS + evidence with journey PASS/FAIL table; secrets scrubbed; cleanup recorded |
| **Refuse-closed** | Opt-in secrets absent (default) — receipt only |
| **FAIL** | Any claimed live-applicable journey functional failure |
| **Offline clones** | **J-CLONE-LOCAL-01** / **J-CLONE-PATH-01** via `g028-offline-clones` (G16-aligned); separate receipt |
| **Out** | Stress/throughput |

## Honesty

Functional journeys on live private forge ≠ G16 timing compare ≠ G26/G27 scale/contention ≠ GitHub Checks/merge-queue product claim. Explicit user unlock supersedes G8 default Refuse for this opt-in run only.

## Waves

| Wave | Work | Exit |
|------|------|------|
| G28-0 | SoT + plan + matrix + interop/residual rows | Docs |
| G28-1 | Harness `run-g028-live-private-arks.sh` + compose `g028-live-private-arks` | Code ready |
| G28-exit | Create private repo → Docker PASS → evidence → cleanup → commit | Claimed |
| G28-offline-clones | `run-g028-offline-clones.sh` + compose `g028-offline-clones` → both IDs PASS | Claimed |

## Record

| Wave | Status | Tip / evidence |
|------|--------|----------------|
| G28-0 | Done | SoT + matrix + interop/residual |
| G28-1 | Done | `run-g028-live-private-arks.sh` + compose `g028-live-private-arks` |
| G28-exit | **Claimed** | Docker PASS; [[evidence:root/20261109212500]]; tip before `244df73` |
| G28-offline-clones | **Claimed** | Docker PASS; [[evidence:root/20261109213400]]; tip `a60e82e` |
