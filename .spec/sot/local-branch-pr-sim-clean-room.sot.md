---
id: sot:root/local-branch-pr-sim-clean-room
title: Local branch PR sim clean-room
status: active
created: 2026-11-09T20:55:07Z
updated: 2026-11-09T20:55:07Z
tags: [clean-room, branch, pr-sim, refs, merge, lease, g26, concurrency]
verify: clean-room
rel:
  governs: [verify/g26, verify/scripts/run-g26.sh, verify/matrix-g26.md]
  depends_on:
    - sot:root/product-verification-clean-room
    - sot:root/interop-test-matrix
    - sot:root/lease-concurrency-intent
    - sot:root/git-compat-surface
    - sot:root/program-roadmap
    - sot:root/production-residual-register
    - plan:root/20261109205507
  evidence: [evidence:root/20261109205847]
---

# Local branch + synthetic PR clean-room

Version **0.1.0**. Normative Docker clean-room gate **G26** for **local** thousands-class branch / synthetic PR-ref scale and concurrent merge workers under `arks-lease`. Authority: [[sot:root/product-verification-clean-room]]. Lease: [[sot:root/lease-concurrency-intent]]. Matrix: `verify/matrix-g26.md`. Plan: [[plan:root/20261109205507]].

## Scope

| In | Out |
|----|-----|
| Gate **G26** (`verify/scripts/run-g26.sh` / compose `g26`) | Live GitHub API, rate limits, Actions, branch protection, merge queue |
| Thousands-class `refs/heads/*` and synthetic `refs/pull/N/head` (+ optional `merge`) | Claiming GitHub PR throughput / forge latency |
| Concurrent merge / tip-update workers under leases + Git `*.lock` | Replacing G17/G18 catalog claims |
| Stock-Git pathology compare where induce fits; fail-closed integrity | CELIR / ObjectStore changes (stream A / G22–G25) |
| Sole-runner vs heavy G17/G19/G22–G25 | Fake PASS without Docker receipt |

## Rule

### Gate identity

1. Gate ID **G26** `local-branch-pr-sim`. Entrypoint: `verify/scripts/run-g26.sh`. Compose: `g26`.
2. G26 is **additive** and independent of CELIR dual-path gates (G22–G25). It does **not** close stream A.
3. Host greens are advisory only.

### Topology (documented actual N)

| Dimension | Target class | Required for Verified (minimum) |
|-----------|--------------|----------------------------------|
| Branch heads (`refs/heads/br-*`) | Thousands | **N_branches ≥ 2048** |
| Synthetic PR refs (`refs/pull/N/head`, optional `merge`) | Thousands | **N_pr ≥ 2048** |
| Concurrent merge / tip workers | Tens (memory-bound) | **N_workers ≥ 32** |

Receipt MUST record the **actual** N achieved. Stretch above minimum is allowed; OOM → document lower actual N and FAIL only if below minimum.

### Assertions (PASS)

1. **Branch scale** — after creating N_branches heads, stock `git for-each-ref refs/heads/br-` count matches; arks `FilesystemRefStore::list_refs` agrees.
2. **PR-ref scale** — synthetic `refs/pull/N/head` (and at least half with `refs/pull/N/merge`) exist; stock Git resolves sample heads.
3. **Leased concurrent merges** — N_workers tip-update / merge-style workers under `refns:` leases + Git locks leave a consistent tip; no torn OID; stock Git reads tip.
4. **Fail-closed** — concurrent grant on the same integration ref without serialization yields `AlreadyHeld` (or Git lock error); no silent skip of locks (lease ≠ skip locks).
5. **Induce-compare** — planted `*.lock` on the integration ref: stock `git update-ref` / merge path exhibits pathology; arks path refuses or errors without skipping the lock.
6. **Sole runner** — exclusive lock dir; MUST refuse overlap with heavy G17/G19/G22–G25 lock dirs (OOM lesson).

### Honesty

1. G26 PASS = **local branch + synthetic PR-env verified** at recorded N — **not** GitHub PR throughput, forge latency, Actions, or protection-rule capacity.
2. Live forge / GitHub connectivity remains **Deferred** for future G8+ measurement; do not fill invented latency numbers.
3. Umbrella goals that also require stream A (CELIR dual-path / G22 successors) stay open until A∧B both Claimed.

## Why

Multi-agent workflows often look like “thousands of branches / PR heads contending to integrate.” Local `.git` can simulate that shape without a forge. Without a clean-room gate, scale and leased merge integrity remain folklore; without honesty bounds, local sim is misread as GitHub capacity.

## Verification

1. Docker `compose … run --rm g26` PASS + receipt under `verify/g26/out/`.
2. Evidence stamp under `.spec/evidence/` cites actual N and tip SHA.
3. Residual / roadmap rows say **local PR-sim verified** only with G26 receipt; GitHub deferred remains explicit.
4. `spec check` clean for this SoT and [[plan:root/20261109205507]].
