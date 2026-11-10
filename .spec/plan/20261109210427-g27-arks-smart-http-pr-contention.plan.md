---
id: plan:root/20261109210427
title: G27 arks smart HTTP PR contention clean-room
status: active
created: 2026-11-09T21:04:27Z
updated: 2026-11-09T21:15:01Z
tags: [g27, clean-room, smart-http, server, branch, pr-sim, contention]
rel:
  depends_on:
    - sot:root/arks-smart-http-pr-contention-clean-room
    - sot:root/product-verification-clean-room
    - sot:root/interop-test-matrix
    - sot:root/arks-server
    - sot:root/production-residual-register
    - sot:root/program-roadmap
  implements: [sot:root/arks-smart-http-pr-contention-clean-room]
  evidence: [evidence:root/20261109210427]
---

# G27 — arks smart HTTP + synthetic PR contention clean-room

## Goal

Claim **arks smart-HTTP server** as GitHub stand-in for thousands-class branch / synthetic PR-ref scale under N concurrent stock-Git agents (clone/fetch/push) with integrity fail-closed in Docker clean-room. Live GitHub Deferred.

## Done rules

| Disposition | Meaning |
|-------------|---------|
| **Verified / Claimed** | Docker `g27` PASS + evidence with actual N (≥2048 branches, ≥2048 PR refs, ≥8 agents) |
| **FAIL** | Below minimum N, integrity failure, sole-runner conflict, torn tip |
| **Out / Deferred** | GitHub Checks/merge-queue/protection, live forge throughput (G8+) |

## Honesty

arks smart-HTTP PR-contention ≠ GitHub PR UX / Checks / merge-queue / live forge throughput. G27 does **not** redefine M11/G15 and does **not** replace G26 local sim.

## Claimed set

| ID | Behavior |
|----|----------|
| **G27-C01** | ≥2048 `refs/heads/br-*` + ≥2048 `refs/pull/N/head` (+ ≥half `merge`) seeded; `git ls-remote` via arks smart HTTP agrees |
| **G27-C02** | ≥8 concurrent stock `git clone` against arks smart HTTP succeed |
| **G27-C03** | ≥8 concurrent `git fetch` succeed; no torn objects |
| **G27-C04** | ≥8 concurrent distinct-ref pushes succeed; server refs agree |
| **G27-C05** | Contended same-ref push fail-closed; tip stock-resolvable |
| **G27-C06** | Sole-runner lock; refuse overlap with heavy G17/G19/G22–G26 |

## Waves

| Wave | Work | Exit |
|------|------|------|
| G27-0 | SoT + plan + matrix + interop/roadmap/residual rows | Docs |
| G27-1 | Harness + tests + compose `g27` | Code ready |
| G27-exit | Docker PASS + evidence + residual/roadmap stamp | Claimed |

## Record

| Wave | Status | Tip / evidence |
|------|--------|----------------|
| G27-0 | Done | SoT + matrix + interop/roadmap/residual |
| G27-1 | Done | harness `run-g27.sh` + `verify/g27` + gzip POST fix |
| G27-exit | **Claimed** | Docker PASS; [[evidence:root/20261109210427]]; N=2048/2048/8; parent tip `bbd1573` |
