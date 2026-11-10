---
id: evidence:root/20261110030000
title: M20 Cx-3 clean-room G17 concurrency claim expansion
status: active
created: 2026-11-10T03:00:00Z
updated: 2026-11-10T03:00:00Z
tags: [g017-concurrency-issue-compare, m20, cx-3, clean-room, concurrency, issue-resolution, verified]
rel:
  depends_on: [plan:root/20261110030000, sot:root/program-roadmap, sot:root/interop-test-matrix, sot:root/multi-agent-git-concurrency-scenarios, sot:root/product-verification-clean-room, sot:root/production-residual-register, evidence:root/20261110020000, research:root/20261110021546, research:root/20261109023117]
  evidence: []
files:
  - {path: receipt.json, sha256: 371de500fe2a42633a9d33586c6a257ef09578296955a15e40b2cbe73abd68fb, bytes: 9732, store: repo}
---

# M20 Cx-3 — clean-room G17 concurrency claim expansion

## Context

Closes Wave **Cx-3** of [[plan:root/20261110030000]] under the Issue-resolution Done rule ([[sot:root/program-roadmap]]): each newly claimed ID reproduced in Docker — stock **`git`** exhibits pathology; **`arks`** meets resolved criteria; comparative receipt attached. Prior Cm-1..Cm-3 + Cx-1 + Cx-2 rows re-verified green in the same receipt (**47**/47 dual PASS). Does **not** redefine M9–M19 exits. Does **not** claim the ~221 catalog is solved. L3 remains refuse-closed; CELIR excluded; G8/G9 stay Closed-deferred.

Attached: `receipt.json` (`arks-g017-concurrency-issue-compare-issue-compare/v1`, wave `Cx-3`).

## Environment

| Field | Value |
|-------|-------|
| Compose | `verify/docker-compose.yml` service `g017-concurrency-issue-compare` |
| Entrypoint | `verify/scripts/run-g017-concurrency-issue-compare.sh` |
| Image | `arks-verify-m1:0.1.0` |
| Image digest | `sha256:5736c5687d3b5775e187ca3d21fbc5b2a5a2bf8965c5381778ee478eb185adf0` |
| Stock Git (in-image) | `git version 2.39.5` |
| Tree SHA at G17 run | pre-land working tree (Cx-3 harness + mitigations; base tip `6db356f`) |
| Host Docker engine | Linux (`desktop-linux`) |

## Cx-3 G17 results (12 new)

| ID | git | arks | Notes |
|----|-----|------|-------|
| REF-012 | pathology_pass | resolved_pass | packed-refs.lock → refuse pack-refs |
| REF-016 | pathology_pass | resolved_pass | notes lock → refuse notes add |
| IDX-014 | pathology_pass | resolved_pass | index.lock → refuse read-tree |
| SPARSE-001 | pathology_pass | resolved_pass | sparse-checkout.lock → refuse set |
| MISC-018 | pathology_pass | resolved_pass | index.lock → refuse reapply |
| SUB-008 | pathology_pass | resolved_pass | move with submodule refuse |
| TOCTOU-007 | pathology_pass | resolved_pass | incomplete admin → refuse prune |
| PACK-009 | pathology_pass | resolved_pass | *.keep inspect honesty |
| MAINT-002 | pathology_pass | resolved_pass | gc.log inspect honesty |
| NET-009 | pathology_pass | resolved_pass | non-empty clone dest refuse |
| CFG-004 | pathology_pass | resolved_pass | format v1 unknown extension refuse |
| MISC-017 | pathology_pass | resolved_pass | NOTES_MERGE_* refuse |

## Regression (same receipt)

Cm-1..Cm-3 (13) + Cx-1 (10) + Cx-2 (12) = 35 prior rows: all `pathology_pass` / `resolved_pass`.

## Aggregate

| Metric | Value |
|--------|------:|
| Scenario rows | 47 |
| git pathology_pass | 47 |
| arks resolved_pass | 47 |
| fail outcomes | 0 |

## Non-claims

- Remainder of the 221 catalog is **not** resolved by Cx-3 (Closed-deferred **158** + N/A **9** + L3 refuse valve).
- No L3 / ≥20% / CELIR product-deps claims.
