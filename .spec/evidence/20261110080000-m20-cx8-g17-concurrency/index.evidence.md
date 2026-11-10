---
id: evidence:root/20261110080000
title: M20 Cx-8 clean-room G17 concurrency claim expansion
status: active
created: 2026-11-10T08:30:00Z
updated: 2026-11-10T08:30:00Z
tags: [g017-concurrency-issue-compare, m20, cx-8, clean-room, concurrency, issue-resolution, verified]
rel:
  depends_on: [plan:root/20261110080000, sot:root/program-roadmap, sot:root/interop-test-matrix, sot:root/multi-agent-git-concurrency-scenarios, sot:root/product-verification-clean-room, evidence:root/20261110070000, research:root/20261110021546]
  evidence: []
files:
  - {path: receipt.json, sha256: 7bf849f32b37cb8066dd6e3e7cc4dc18b00fb49324d759d2eb4d2fc16f639688, bytes: 21001, store: repo}
---

# M20 Cx-8 — clean-room G17 concurrency claim expansion

## Context

Closes Wave **Cx-8** of [[plan:root/20261110080000]]: Docker induce-compare for twelve additional mitigate IDs + prior regression (**107**/107 dual PASS). Does **not** claim the ~221 catalog is solved. L3 refuse-closed; CELIR excluded.

Attached: `receipt.json` (`arks-g017-concurrency-issue-compare-issue-compare/v1`, wave `Cx-8`).

## Environment

| Field | Value |
|-------|-------|
| Compose | `verify/docker-compose.yml` service `g017-concurrency-issue-compare` |
| Entrypoint | `verify/scripts/run-g017-concurrency-issue-compare.sh` |
| Image | `arks-verify-m1:0.1.0` |
| Image digest | `sha256:5736c5687d3b5775e187ca3d21fbc5b2a5a2bf8965c5381778ee478eb185adf0` |
| Stock Git (in-image) | `git version 2.39.5` |
| Tree SHA at G17 run | pre-land working tree (Cx-8 harness + mitigations; base tip `7a80b4a`) |
| Host Docker engine | Linux (`desktop-linux`) |

## Aggregate

| Metric | Value |
|--------|------:|
| Scenario rows | 107 |
| git pathology_pass | 107 |
| arks resolved_pass | 107 |
| fail outcomes | 0 |

## Non-claims

Remainder Closed-deferred **98** + N/A **9** + L3 refuse valve. No “221 solved.”
