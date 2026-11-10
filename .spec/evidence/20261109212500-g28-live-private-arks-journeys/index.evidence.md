---
id: evidence:root/20261109212500
title: G28 live private arks journeys
status: active
created: 2026-11-09T21:25:00Z
updated: 2026-11-10T00:57:44Z
tags: [g28, clean-room, journey, live, github, private, verified]
rel:
  depends_on: [sot:root/g28-live-private-arks-journeys, plan:root/20261109212020, sot:root/product-verification-clean-room, sot:root/interop-test-matrix, sot:root/clean-room-journey-compare, sot:root/production-residual-register, adr:root/20261110151000]
files:
  - {path: receipt.json, sha256: cdf58199f4e6d1e4492d37fd66316638c1cf9b6a4995406fda9a66ed96d5f05e, bytes: 3004, store: repo}
---

# G28 — live private arks journeys

## Authority

| Field | Value |
|-------|--------|
| Gate | **G28** (`verify/scripts/run-g28.sh` / compose `g28`) |
| Matrix | `verify/matrices/matrix-g28.md` |
| Stock git (image) | `git version 2.39.5` |
| Image | `arks-verify-m1:0.1.0` |
| Parent tip before wave | `244df73` |
| Result | **PASS** (`disposition=closed-implement`, PASS=21 FAIL=0 N/A=2) |
| Throwaway private repo | `https://github.com/myso-kr/arks-g28-live-journeys-20261110062418` |
| Submodule peer | `https://github.com/myso-kr/arks-g28-live-sub-20261110062418` |

## Journey table

| ID | Status |
|----|--------|
| J-CLONE-LOCAL-01 | N/A on this live receipt — closed PASS offline [[evidence:root/20261109213400]] |
| J-CLONE-PATH-01 | N/A on this live receipt — closed PASS offline [[evidence:root/20261109213400]] |
| J-REMOTE-CLONE-01 | PASS |
| J-LOCAL-01 | PASS |
| J-BRANCH-01 | PASS |
| J-PUSH-FETCH-01 | PASS |
| J-MERGE-FF-01 | PASS |
| J-PULL-01 | PASS |
| J-TAG-01 | PASS |
| J-STASH-01 | PASS |
| J-DIFF-01 | PASS |
| J-RM-MV-01 | PASS |
| J-CHERRY-01 | PASS |
| J-REBASE-01 | PASS |
| J-REVERT-01 | PASS |
| J-REFLOG-01 | PASS |
| J-BUNDLE-01 | PASS |
| J-ARCHIVE-01 | PASS |
| J-REPACK-01 | PASS |
| J-WORKTREE-01 | PASS |
| J-SUBMODULE-01 | PASS (local-path peer; HTTPS submodule add unclaimed) |
| J-PR-GH-01 | PASS |
| J-ALIAS-01 | PASS |

## Honesty

- Functional journeys on **live private** GitHub with **arks** in Docker clean-room (network opt-in).
- **Not** G16 timing/perf compare; **not** G26/G27 scale/contention; **not** Checks/merge-queue product claim.
- Unsigned seed required: GitHub `--add-readme` signed commits (`gpgsig`) are **unclaimed** / fail-closed in arks today.
- Explicit user unlock supersedes G8 default Refuse for this opt-in run only; default without secrets remains Refuse-closed.
- Secrets via env/`gh auth` only — never committed.

## Cleanup

**Done.** After `delete_repo` scope grant, all ephemeral private repos deleted (`gh repo view` → not found):

- `myso-kr/arks-g28-live-journeys-20261110062418` (PASS venue)
- `myso-kr/arks-g28-live-sub-20261110062418` (submodule peer)
- `myso-kr/arks-g28-live-journeys-20261110062241` (abandoned signed-readme seed)
- `myso-kr/arks-g28-live-sub-20261110062241` (abandoned)

G28 Claimed status unchanged.
