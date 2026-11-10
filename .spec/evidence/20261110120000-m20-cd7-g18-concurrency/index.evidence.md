---
id: evidence:root/20261110120000
title: M20 Cd-7 clean-room G18 concurrency claim expansion
status: active
created: 2026-11-10T12:30:00Z
updated: 2026-11-10T12:30:00Z
tags: [g018-concurrency-issue-compare, m20, cd-7, clean-room, concurrency, issue-resolution, lease, verified]
rel:
  depends_on: [plan:root/20261110120000, sot:root/program-roadmap, sot:root/interop-test-matrix, sot:root/multi-agent-git-concurrency-scenarios, sot:root/lease-concurrency-intent, sot:root/product-verification-clean-room, evidence:root/20261110110000, adr:root/20261109032955, research:root/20261110021546]
  evidence: []
files:
  - {path: receipt.json, sha256: e039cd68b162363fb1cc7b320d588d461b00900ab1d2e6717a087b320da29521, bytes: 12284, store: repo}
---

# M20 Cd-7 — clean-room G18 concurrency claim expansion

## Context

Closes Wave **Cd-7** of [[plan:root/20261110120000]]: Docker lease-uplift induce-compare for twelve bucket B differentiate IDs + Cd-1..Cd-6 regression (**60**/60 dual PASS). Does **not** claim the ~221 catalog is solved. L3 refuse-closed; CELIR excluded; lease ≠ skip Git locks. No absolute PERF SLO claims.

Attached: `receipt.json` (`arks-g018-concurrency-issue-compare-issue-compare/v1`, wave `Cd-7`).

## Environment

| Field | Value |
|-------|-------|
| Compose | `verify/docker-compose.yml` service `g018-concurrency-issue-compare` |
| Entrypoint | `verify/scripts/run-g018-concurrency-issue-compare.sh` |
| Image | `arks-verify-m1:0.1.0` |
| Image digest | `sha256:5736c5687d3b5775e187ca3d21fbc5b2a5a2bf8965c5381778ee478eb185adf0` |
| Stock Git (in-image) | `git version 2.39.5` |
| Tree SHA at G18 run | pre-land working tree (Cd-7 harness + lease notes; base tip `2d5259e`) |
| Host Docker engine | Linux (`desktop-linux`) |

## Aggregate

| Metric | Value |
|--------|------:|
| Scenario rows | 60 |
| git pathology_pass | 60 |
| arks resolved_pass | 60 |
| fail outcomes | 0 |

## Non-claims

Remainder Closed-deferred **50** + N/A **9** + L3 refuse valve. No “221 solved.”
