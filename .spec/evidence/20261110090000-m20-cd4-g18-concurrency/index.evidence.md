---
id: evidence:root/20261110090000
title: M20 Cd-4 clean-room G18 concurrency claim expansion
status: active
created: 2026-11-10T09:30:00Z
updated: 2026-11-10T09:30:00Z
tags: [g018-concurrency-issue-compare, m20, cd-4, clean-room, concurrency, issue-resolution, lease, verified]
rel:
  depends_on: [plan:root/20261110090000, sot:root/program-roadmap, sot:root/interop-test-matrix, sot:root/multi-agent-git-concurrency-scenarios, sot:root/lease-concurrency-intent, sot:root/product-verification-clean-room, evidence:root/20261110080000, adr:root/20261109032955, research:root/20261110021546]
  evidence: []
files:
  - {path: receipt.json, sha256: 07f5431391248d8beb085a7fcb2f67d2ce5d17b7f1d914dde55ad4f3cafba6b3, bytes: 5094, store: repo}
---

# M20 Cd-4 — clean-room G18 concurrency claim expansion

## Context

Closes Wave **Cd-4** of [[plan:root/20261110090000]]: Docker lease-uplift induce-compare for twelve bucket B differentiate IDs + Cd-1..Cd-3 regression (**24**/24 dual PASS). Does **not** claim the ~221 catalog is solved. L3 refuse-closed; CELIR excluded; lease ≠ skip Git locks.

Attached: `receipt.json` (`arks-g018-concurrency-issue-compare-issue-compare/v1`, wave `Cd-4`).

## Environment

| Field | Value |
|-------|-------|
| Compose | `verify/docker-compose.yml` service `g018-concurrency-issue-compare` |
| Entrypoint | `verify/scripts/run-g018-concurrency-issue-compare.sh` |
| Image | `arks-verify-m1:0.1.0` |
| Image digest | `sha256:5736c5687d3b5775e187ca3d21fbc5b2a5a2bf8965c5381778ee478eb185adf0` |
| Stock Git (in-image) | `git version 2.39.5` |
| Tree SHA at G18 run | pre-land working tree (Cd-4 harness + lease notes; base tip `5392938`) |
| Host Docker engine | Linux (`desktop-linux`) |

## Aggregate

| Metric | Value |
|--------|------:|
| Scenario rows | 24 |
| git pathology_pass | 24 |
| arks resolved_pass | 24 |
| fail outcomes | 0 |

## Non-claims

Remainder Closed-deferred **86** + N/A **9** + L3 refuse valve. No “221 solved.”
