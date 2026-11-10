---
id: evidence:root/20261110070000
title: M20 Cx-7 clean-room G17 concurrency claim expansion
status: active
created: 2026-11-10T07:30:00Z
updated: 2026-11-10T07:30:00Z
tags: [g17, m20, cx-7, clean-room, concurrency, issue-resolution, verified]
rel:
  depends_on: [plan:root/20261110070000, sot:root/program-roadmap, sot:root/interop-test-matrix, sot:root/multi-agent-git-concurrency-scenarios, sot:root/product-verification-clean-room, evidence:root/20261110060000, research:root/20261110021546]
  evidence: []
files:
  - {path: receipt.json, sha256: 112df574f6572136b7b0563fcb9541a84d6bf5e6b84f9e91cc48b7dad4382eed, bytes: 18735, store: repo}
---

# M20 Cx-7 — clean-room G17 concurrency claim expansion

## Context

Closes Wave **Cx-7** of [[plan:root/20261110070000]]: Docker induce-compare for twelve additional mitigate IDs + prior regression (**95**/95 dual PASS). Does **not** claim the ~221 catalog is solved. L3 refuse-closed; CELIR excluded.

Attached: `receipt.json` (`arks-g17-issue-compare/v1`, wave `Cx-7`).

## Environment

| Field | Value |
|-------|-------|
| Compose | `verify/docker-compose.yml` service `g17` |
| Entrypoint | `verify/scripts/run-g17.sh` |
| Image | `arks-verify-m1:0.1.0` |
| Image digest | `sha256:5736c5687d3b5775e187ca3d21fbc5b2a5a2bf8965c5381778ee478eb185adf0` |
| Stock Git (in-image) | `git version 2.39.5` |
| Tree SHA at G17 run | pre-land working tree (Cx-7 harness + mitigations; base tip `f2622ab`) |
| Host Docker engine | Linux (`desktop-linux`) |

## Aggregate

| Metric | Value |
|--------|------:|
| Scenario rows | 95 |
| git pathology_pass | 95 |
| arks resolved_pass | 95 |
| fail outcomes | 0 |

## Non-claims

Remainder Closed-deferred **110** + N/A **9** + L3 refuse valve. No “221 solved.”
