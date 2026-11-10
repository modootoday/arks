---
id: evidence:root/20261109205847
title: G26 local branch + synthetic PR-env clean-room
status: active
created: 2026-11-09T20:58:47Z
updated: 2026-11-10T00:57:44Z
tags: [g26, clean-room, branch, pr-sim, lease, verified, stream-b]
rel:
  depends_on: [sot:root/local-branch-pr-sim-clean-room, plan:root/20261109205507, sot:root/product-verification-clean-room, sot:root/interop-test-matrix, sot:root/lease-concurrency-intent, sot:root/program-roadmap, sot:root/production-residual-register]
files:
  - {path: receipt.json, sha256: 5a7e745db8b51ba03fcd0da99469184a16b3f6102bb2b4c69185654c804f8149, bytes: 319, store: repo}
---

# G26 — Local branch + synthetic PR-env clean-room

## Authority

| Field | Value |
|-------|--------|
| Gate | **G26** (`verify/scripts/run-g26.sh` / compose `g26`) |
| Matrix | `verify/matrices/matrix-g26.md` |
| Stream | **B** (umbrella A∧B; A Claimed at `fa78685`) |
| Stock git (image) | `git version 2.39.5` |
| rustc (image) | `1.99.1` |
| N scale | **2048** branches + **2048** PR heads (+1024 merge) + **32** workers |
| Result | **PASS** |
| Parent tip before wave | `fa78685` |

## Suite

| Step | Result |
|------|--------|
| `cargo test -p arks-lease --test g26_branch_pr_sim` | 5/5 PASS |
| Sole-runner lock | held |
| Heavy-gate overlap refuse | OK (no G17/G19/G22–G25 lock) |

## Assertions held

1. Branch scale N=2048; stock `for-each-ref` agrees with arks list.
2. Synthetic `refs/pull/N/head` N=2048 + half `merge`; stock resolve OK.
3. Leased concurrent tip workers N=32; tips stock-readable.
4. Contended lease fail-closed (1 win / N-1 AlreadyHeld).
5. Planted ref.lock induce-compare: stock pathology; arks/lease path errors (no lock skip).

## Honesty

- **Local branch + synthetic PR-env verified (G26)** — **not** GitHub PR throughput, rate limit, Actions, or protection capacity.
- Live forge latency remains **Deferred** (G8+ measurement).
- Umbrella complete only when **A∧B** both Claimed (A already `fa78685`; this stamp closes B).
