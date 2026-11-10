---
id: evidence:root/20261110020000
title: M19 Cx-2 clean-room G17 concurrency claim expansion
status: active
created: 2026-11-10T02:00:00Z
updated: 2026-11-10T02:00:00Z
tags: [g17, m19, cx-2, clean-room, concurrency, issue-resolution, verified]
rel:
  depends_on: [plan:root/20261110010000, sot:root/program-roadmap, sot:root/interop-test-matrix, sot:root/multi-agent-git-concurrency-scenarios, sot:root/product-verification-clean-room, sot:root/production-residual-register, evidence:root/20261110010000, research:root/20261109023117]
  evidence: []
files:
  - {path: receipt.json, sha256: 3f2fa07b4b5a020f36aa39bcd2b52ea74ba188849daeb537317c92864b18b4ca, bytes: 7455, store: repo}
---

# M19 Cx-2 — clean-room G17 concurrency claim expansion

## Context

Closes Wave **Cx-2** of [[plan:root/20261110010000]] under the Issue-resolution Done rule ([[sot:root/program-roadmap]]): each newly claimed ID reproduced in Docker — stock **`git`** exhibits pathology; **`arks`** meets resolved criteria; comparative receipt attached. Prior Cm-1..Cm-3 + Cx-1 rows re-verified green in the same receipt (**35**/35 dual PASS). Does **not** redefine M9–M18 exits. Does **not** claim the ~221 catalog is solved. L3 remains refuse-closed; G8/G9 stay Closed-deferred.

Attached: `receipt.json` (`arks-g17-issue-compare/v1`, wave `Cx-2`).

## Environment

| Field | Value |
|-------|-------|
| Compose | `verify/docker-compose.yml` service `g17` |
| Entrypoint | `verify/scripts/run-g17.sh` |
| Image | `arks-verify-m1:0.1.0` |
| Image digest | `sha256:5736c5687d3b5775e187ca3d21fbc5b2a5a2bf8965c5381778ee478eb185adf0` |
| Stock Git (in-image) | `git version 2.39.5` |
| Tree SHA at G17 run | pre-land working tree (Cx-2 harness + mitigations) |
| Host Docker engine | Linux (`desktop-linux`) |

## Cx-2 G17 results (12 new)

| ID | git | arks | Notes |
|----|-----|------|-------|
| BISECT-001 | pathology_pass | resolved_pass | Stock clobbers session; arks exclusive refuse |
| BISECT-002 | pathology_pass | resolved_pass | Dirty mid-bisect refuse |
| SEQ-006 | pathology_pass | resolved_pass | Interactive todo → refuse concurrent rebase |
| SEQ-007 | pathology_pass | resolved_pass | rebase-apply → refuse concurrent am |
| MISC-007 | pathology_pass | resolved_pass | Parallel am / shared apply state refuse |
| STASH-001 | pathology_pass | resolved_pass | refs/stash.lock → refuse stash push |
| STASH-002 | pathology_pass | resolved_pass | index.lock → refuse stash pop |
| REF-021 | pathology_pass | resolved_pass | HEAD.lock → refuse symbolic-ref |
| TOCTOU-003 | pathology_pass | resolved_pass | Dirty WT → refuse checkout overwrite |
| MISC-005 | pathology_pass | resolved_pass | index.lock → refuse reset --hard |
| CORR-002 | pathology_pass | resolved_pass | Refuse gc --force |
| LOCK-004 | pathology_pass | resolved_pass | Porcelain die-class under lock |

## Regression (same receipt)

Cm-1..Cm-3 (13) + Cx-1 (10) = 23 prior rows: all `pathology_pass` / `resolved_pass`.

## Aggregate

| Metric | Value |
|--------|------:|
| Scenario rows | 35 |
| git pathology_pass | 35 |
| arks resolved_pass | 35 |
| fail outcomes | 0 |

## Non-claims

- Remainder of the 221 catalog is **not** resolved by Cx-2.
- No L3 / ≥20% / CELIR product-deps claims.
