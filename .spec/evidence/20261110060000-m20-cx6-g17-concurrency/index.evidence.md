---
id: evidence:root/20261110060000
title: M20 Cx-6 clean-room G17 concurrency claim expansion
status: active
created: 2026-11-10T06:30:00Z
updated: 2026-11-10T06:30:00Z
tags: [g17, m20, cx-6, clean-room, concurrency, issue-resolution, verified]
rel:
  depends_on: [plan:root/20261110060000, sot:root/program-roadmap, sot:root/interop-test-matrix, sot:root/multi-agent-git-concurrency-scenarios, sot:root/product-verification-clean-room, evidence:root/20261110050000, research:root/20261110021546]
  evidence: []
files:
  - {path: receipt.json, sha256: 0334459fdc6458b096bbf37035f744fa791a6f855ac8c4292cca55985ef84ab1, bytes: 16993, store: repo}
---

# M20 Cx-6 — clean-room G17 concurrency claim expansion

## Context

Closes Wave **Cx-6** of [[plan:root/20261110060000]]: Docker induce-compare for twelve additional mitigate IDs + prior regression (**83**/83 dual PASS). Does **not** claim the ~221 catalog is solved. L3 refuse-closed; CELIR excluded.

Attached: `receipt.json` (`arks-g17-issue-compare/v1`, wave `Cx-6`).

## Environment

| Field | Value |
|-------|-------|
| Compose | `verify/docker-compose.yml` service `g17` |
| Entrypoint | `verify/scripts/run-g17.sh` |
| Image | `arks-verify-m1:0.1.0` |
| Image digest | `sha256:5736c5687d3b5775e187ca3d21fbc5b2a5a2bf8965c5381778ee478eb185adf0` |
| Stock Git (in-image) | `git version 2.39.5` |
| Tree SHA at G17 run | pre-land working tree (Cx-6 harness + mitigations; base tip `6d1c512`) |
| Host Docker engine | Linux (`desktop-linux`) |

## Aggregate

| Metric | Value |
|--------|------:|
| Scenario rows | 83 |
| git pathology_pass | 83 |
| arks resolved_pass | 83 |
| fail outcomes | 0 |

## Non-claims

Remainder Closed-deferred **122** + N/A **9** + L3 refuse valve. No “221 solved.”
