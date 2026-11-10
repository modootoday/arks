---
id: evidence:root/20261110110000
title: M20 Cd-6 clean-room G18 concurrency claim expansion
status: active
created: 2026-11-10T11:30:00Z
updated: 2026-11-10T11:30:00Z
tags: [g18, m20, cd-6, clean-room, concurrency, issue-resolution, lease, verified]
rel:
  depends_on: [plan:root/20261110110000, sot:root/program-roadmap, sot:root/interop-test-matrix, sot:root/multi-agent-git-concurrency-scenarios, sot:root/lease-concurrency-intent, sot:root/product-verification-clean-room, evidence:root/20261110100000, adr:root/20261109032955, research:root/20261110021546]
  evidence: []
files:
  - {path: receipt.json, sha256: d49177b5152040a47d86d5ffa6cd72c9662f5e3f3f23c0924944025547cb88d8, bytes: 10249, store: repo}
---

# M20 Cd-6 — clean-room G18 concurrency claim expansion

## Context

Closes Wave **Cd-6** of [[plan:root/20261110110000]]: Docker lease-uplift induce-compare for twelve bucket B differentiate IDs + Cd-1..Cd-5 regression (**48**/48 dual PASS). Does **not** claim the ~221 catalog is solved. L3 refuse-closed; CELIR excluded; lease ≠ skip Git locks.

Attached: `receipt.json` (`arks-g18-issue-compare/v1`, wave `Cd-6`).

## Environment

| Field | Value |
|-------|-------|
| Compose | `verify/docker-compose.yml` service `g18` |
| Entrypoint | `verify/scripts/run-g18.sh` |
| Image | `arks-verify-m1:0.1.0` |
| Image digest | `sha256:5736c5687d3b5775e187ca3d21fbc5b2a5a2bf8965c5381778ee478eb185adf0` |
| Stock Git (in-image) | `git version 2.39.5` |
| Tree SHA at G18 run | pre-land working tree (Cd-6 harness + lease notes; base tip `83bf711`) |
| Host Docker engine | Linux (`desktop-linux`) |

## Aggregate

| Metric | Value |
|--------|------:|
| Scenario rows | 48 |
| git pathology_pass | 48 |
| arks resolved_pass | 48 |
| fail outcomes | 0 |

## Non-claims

Remainder Closed-deferred **62** + N/A **9** + L3 refuse valve. No “221 solved.”
