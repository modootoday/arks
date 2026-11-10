---
id: evidence:root/20261110100000
title: M20 Cd-5 clean-room G18 concurrency claim expansion
status: active
created: 2026-11-10T10:30:00Z
updated: 2026-11-10T10:30:00Z
tags: [g18, m20, cd-5, clean-room, concurrency, issue-resolution, lease, verified]
rel:
  depends_on: [plan:root/20261110100000, sot:root/program-roadmap, sot:root/interop-test-matrix, sot:root/multi-agent-git-concurrency-scenarios, sot:root/lease-concurrency-intent, sot:root/product-verification-clean-room, evidence:root/20261110090000, adr:root/20261109032955, research:root/20261110021546]
  evidence: []
files:
  - {path: receipt.json, sha256: b2426c6771e45dcd5d0134576f26e6b89eb44010c46d0a7728a7371f9972307a, bytes: 7777, store: repo}
---

# M20 Cd-5 — clean-room G18 concurrency claim expansion

## Context

Closes Wave **Cd-5** of [[plan:root/20261110100000]]: Docker lease-uplift induce-compare for twelve bucket B differentiate IDs + Cd-1..Cd-4 regression (**36**/36 dual PASS). Does **not** claim the ~221 catalog is solved. L3 refuse-closed; CELIR excluded; lease ≠ skip Git locks.

Attached: `receipt.json` (`arks-g18-issue-compare/v1`, wave `Cd-5`).

## Environment

| Field | Value |
|-------|-------|
| Compose | `verify/docker-compose.yml` service `g18` |
| Entrypoint | `verify/scripts/run-g18.sh` |
| Image | `arks-verify-m1:0.1.0` |
| Image digest | `sha256:5736c5687d3b5775e187ca3d21fbc5b2a5a2bf8965c5381778ee478eb185adf0` |
| Stock Git (in-image) | `git version 2.39.5` |
| Tree SHA at G18 run | pre-land working tree (Cd-5 harness + lease notes; base tip `881223e`) |
| Host Docker engine | Linux (`desktop-linux`) |

## Aggregate

| Metric | Value |
|--------|------:|
| Scenario rows | 36 |
| git pathology_pass | 36 |
| arks resolved_pass | 36 |
| fail outcomes | 0 |

## Non-claims

Remainder Closed-deferred **74** + N/A **9** + L3 refuse valve. No “221 solved.”
