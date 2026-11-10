---
id: plan:root/20261109205507
title: G26 local branch PR sim clean-room
status: active
created: 2026-11-09T20:55:07Z
updated: 2026-11-09T21:00:00Z
tags: [g026-local-branch-pr-sim, clean-room, branch, pr-sim, lease, merge]
rel:
  depends_on:
    - sot:root/local-branch-pr-sim-clean-room
    - sot:root/product-verification-clean-room
    - sot:root/interop-test-matrix
    - sot:root/lease-concurrency-intent
    - sot:root/production-residual-register
    - sot:root/program-roadmap
  implements: [sot:root/local-branch-pr-sim-clean-room]
---

# G26 — Local branch + synthetic PR clean-room

## Goal

Claim **local** thousands-class branch / synthetic PR-ref scale and leased concurrent merge integrity in Docker clean-room. GitHub live latency/API remains Deferred.

## Done rules

| Disposition | Meaning |
|-------------|---------|
| **Verified / Claimed** | Docker `g026-local-branch-pr-sim` PASS + evidence with actual N (≥2048 branches, ≥2048 PR refs, ≥32 workers) |
| **FAIL** | Below minimum N, integrity failure, sole-runner conflict, lock skip |
| **Out / Deferred** | GitHub rate limit, Actions, protection, live forge throughput (future G8+) |

## Honesty

Local PR-ref simulation ≠ GitHub PR throughput claim. Stream B Claimed does **not** complete an A∧B umbrella while stream A (CELIR dual-path / G22 successors) remains open.

## Claimed set

| ID | Behavior |
|----|----------|
| **G26-C01** | ≥2048 `refs/heads/br-*`; stock `for-each-ref` + arks list agree |
| **G26-C02** | ≥2048 `refs/pull/N/head` (+ ≥half `merge`); stock resolve samples |
| **G26-C03** | ≥32 leased concurrent tip/merge workers; consistent tip; stock-readable |
| **G26-C04** | Fail-closed on contended lease / Git lock; lease ≠ skip locks |
| **G26-C05** | Induce-compare: planted ref lock → stock pathology; arks errors |
| **G26-C06** | Sole-runner lock; refuse overlap with heavy G17/G19/G22–G25 |

## Waves

| Wave | Work | Exit |
|------|------|------|
| G26-0 | SoT + plan + matrix + interop row | Docs |
| G26-1 | Harness + tests + compose `g026-local-branch-pr-sim` | Code ready |
| G26-exit | Docker PASS + evidence + residual/roadmap | Claimed |

## Record

| Wave | Status | Tip / evidence |
|------|--------|----------------|
| G26-0 | Done | SoT + matrix + interop row |
| G26-1 | Done | harness `run-g026-local-branch-pr-sim.sh` + `g026-local-branch-pr-sim_branch_pr_sim` |
| G26-exit | **Claimed** | Docker PASS; [[evidence:root/20261109205847]]; parent tip `fa78685` |
