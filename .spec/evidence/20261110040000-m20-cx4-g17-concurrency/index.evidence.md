---
id: evidence:root/20261110040000
title: M20 Cx-4 clean-room G17 concurrency claim expansion
status: active
created: 2026-11-10T04:30:00Z
updated: 2026-11-10T04:30:00Z
tags: [g17, m20, cx-4, clean-room, concurrency, issue-resolution, verified]
rel:
  depends_on: [plan:root/20261110040000, sot:root/program-roadmap, sot:root/interop-test-matrix, sot:root/multi-agent-git-concurrency-scenarios, sot:root/product-verification-clean-room, sot:root/production-residual-register, evidence:root/20261110030000, research:root/20261110021546, research:root/20261109023117]
  evidence: []
files:
  - {path: receipt.json, sha256: 19d3c93b0034ece8a61b1568a14669f4555d58c3c0700192bcc3be301d88de01, bytes: 12421, store: repo}
---

# M20 Cx-4 — clean-room G17 concurrency claim expansion

## Context

Closes Wave **Cx-4** of [[plan:root/20261110040000]] under the Issue-resolution Done rule ([[sot:root/program-roadmap]]): each newly claimed ID reproduced in Docker — stock **`git`** exhibits pathology; **`arks`** meets resolved criteria; comparative receipt attached. Prior Cm-1..Cm-3 + Cx-1 + Cx-2 + Cx-3 rows re-verified green in the same receipt (**59**/59 dual PASS). Does **not** redefine M9–M19 exits. Does **not** claim the ~221 catalog is solved. L3 remains refuse-closed; CELIR excluded; G8/G9 stay Closed-deferred.

Attached: `receipt.json` (`arks-g17-issue-compare/v1`, wave `Cx-4`).

## Environment

| Field | Value |
|-------|-------|
| Compose | `verify/docker-compose.yml` service `g17` |
| Entrypoint | `verify/scripts/run-g17.sh` |
| Image | `arks-verify-m1:0.1.0` |
| Image digest | `sha256:5736c5687d3b5775e187ca3d21fbc5b2a5a2bf8965c5381778ee478eb185adf0` |
| Stock Git (in-image) | `git version 2.39.5` |
| Tree SHA at G17 run | pre-land working tree (Cx-4 harness + mitigations; base tip `6e73664`) |
| Host Docker engine | Linux (`desktop-linux`) |

## Cx-4 G17 results (12 new)

| ID | git | arks | Notes |
|----|-----|------|-------|
| REF-009 | pathology_pass | resolved_pass | logs/HEAD.lock → refuse commit |
| REF-022 | pathology_pass | resolved_pass | reader-safe for-each-ref + inspect |
| IDX-013 | pathology_pass | resolved_pass | index.lock → refuse --chmod |
| IDX-010 | pathology_pass | resolved_pass | index.lock → refuse skip-worktree |
| HOOK-002 | pathology_pass | resolved_pass | pre-auto-gc honesty |
| HOOK-005 | pathology_pass | resolved_pass | nested index write refuse |
| MISC-014 | pathology_pass | resolved_pass | reflog expire refuse |
| MISC-011 | pathology_pass | resolved_pass | hooks swap inspect honesty |
| SHAL-001 | pathology_pass | resolved_pass | shallow.lock → refuse fetch |
| WT-006 | pathology_pass | resolved_pass | config.worktree.lock refuse |
| SUB-006 | pathology_pass | resolved_pass | gitlink mismatch honesty |
| PACK-008 | pathology_pass | resolved_pass | colliding pack lock refuse |

## Regression (same receipt)

Cm-1..Cm-3 (13) + Cx-1 (10) + Cx-2 (12) + Cx-3 (12) = 47 prior rows: all `pathology_pass` / `resolved_pass`.

## Aggregate

| Metric | Value |
|--------|------:|
| Scenario rows | 59 |
| git pathology_pass | 59 |
| arks resolved_pass | 59 |
| fail outcomes | 0 |

## Non-claims

- Remainder of the 221 catalog is **not** resolved by Cx-4 (Closed-deferred **146** + N/A **9** + L3 refuse valve).
- No L3 / ≥20% / CELIR product-deps claims.
