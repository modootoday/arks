---
id: evidence:root/20261110050000
title: M20 Cx-5 clean-room G17 concurrency claim expansion
status: active
created: 2026-11-10T05:20:00Z
updated: 2026-11-10T05:20:00Z
tags: [g17, m20, cx-5, clean-room, concurrency, issue-resolution, verified]
rel:
  depends_on: [plan:root/20261110050000, sot:root/program-roadmap, sot:root/interop-test-matrix, sot:root/multi-agent-git-concurrency-scenarios, sot:root/product-verification-clean-room, evidence:root/20261110040000, research:root/20261110021546]
  evidence: []
files:
  - {path: receipt.json, sha256: dd0e2b11e1dfe553a0811e9837d955efd9ea9ea9d1378c22a63c05f09b78fe98, bytes: 14799, store: repo}
---

# M20 Cx-5 — clean-room G17 concurrency claim expansion

## Context

Closes Wave **Cx-5** of [[plan:root/20261110050000]]: Docker induce-compare for twelve additional mitigate IDs + prior regression (**71**/71 dual PASS). Does **not** claim the ~221 catalog is solved. L3 refuse-closed; CELIR excluded.

Attached: `receipt.json` (`arks-g17-issue-compare/v1`, wave `Cx-5`).

## Environment

| Field | Value |
|-------|-------|
| Compose | `verify/docker-compose.yml` service `g17` |
| Entrypoint | `verify/scripts/run-g17.sh` |
| Image | `arks-verify-m1:0.1.0` |
| Image digest | `sha256:5736c5687d3b5775e187ca3d21fbc5b2a5a2bf8965c5381778ee478eb185adf0` |
| Stock Git (in-image) | `git version 2.39.5` |
| Tree SHA at G17 run | pre-land working tree (Cx-5 harness + mitigations; base tip `a01231e`) |
| Host Docker engine | Linux (`desktop-linux`) |

## Cx-5 G17 results (12 new)

| ID | git | arks | Notes |
|----|-----|------|-------|
| IDX-004 | pathology_pass | resolved_pass | sharedindex.lock refuse |
| IDX-011 | pathology_pass | resolved_pass | index extension under lock refuse |
| HOOK-003 | pathology_pass | resolved_pass | post-index-change inspect |
| MISC-019 | pathology_pass | resolved_pass | .gitattributes.lock refuse |
| OBJ-001 | pathology_pass | resolved_pass | identical loose idempotent |
| OBJ-005 | pathology_pass | resolved_pass | partial loose refuse |
| PACK-001 | pathology_pass | resolved_pass | pack writer lock → repack refuse |
| WT-005 | pathology_pass | resolved_pass | stale gitdir repair honesty |
| LOCK-003 | pathology_pass | resolved_pass | rename atomicity inspect |
| MISC-002 | pathology_pass | resolved_pass | occupied archive -o refuse |
| MISC-003 | pathology_pass | resolved_pass | bundle under HEAD.lock refuse |
| SHAL-003 | pathology_pass | resolved_pass | filter mismatch inspect |

## Aggregate

| Metric | Value |
|--------|------:|
| Scenario rows | 71 |
| git pathology_pass | 71 |
| arks resolved_pass | 71 |
| fail outcomes | 0 |

## Non-claims

Remainder Closed-deferred **134** + N/A **9** + L3 refuse valve. No “221 solved.”
