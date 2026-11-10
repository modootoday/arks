---
id: evidence:root/20261110130000
title: M20 Cd-8 clean-room G18 concurrency claim expansion
status: active
created: 2026-11-10T13:30:00Z
updated: 2026-11-10T13:30:00Z
tags: [g18, m20, cd-8, clean-room, concurrency, issue-resolution, lease, verified]
rel:
  depends_on: [plan:root/20261110130000, sot:root/program-roadmap, sot:root/interop-test-matrix, sot:root/multi-agent-git-concurrency-scenarios, sot:root/lease-concurrency-intent, sot:root/product-verification-clean-room, evidence:root/20261110120000, adr:root/20261109032955, research:root/20261110021546]
  evidence: []
files:
  - {path: receipt.json, sha256: 11b395fb22a79aadb09dc50849bd55ba26906d86be0b0a25df1fe398a00cfae7, bytes: 14559, store: repo}
---

# M20 Cd-8 — clean-room G18 concurrency claim expansion

## Context

Closes Wave **Cd-8** of [[plan:root/20261110130000]]: Docker lease-uplift for last researched bucket B leftovers (PERF coordination + CFG-002 + AGENT-004 NFS honesty) + Cd-1..Cd-7 regression (**72**/72 dual PASS). Absolute PERF ms / ≥20% remain M14/G19-separated. Does **not** claim the ~221 catalog is solved. L3 refuse-closed; CELIR excluded; lease ≠ skip Git locks.

Attached: `receipt.json` (`arks-g18-issue-compare/v1`, wave `Cd-8`).

## Environment

| Field | Value |
|-------|-------|
| Compose | `verify/docker-compose.yml` service `g18` |
| Entrypoint | `verify/scripts/run-g18.sh` |
| Image | `arks-verify-m1:0.1.0` |
| Image digest | `sha256:5736c5687d3b5775e187ca3d21fbc5b2a5a2bf8965c5381778ee478eb185adf0` |
| Stock Git (in-image) | `git version 2.39.5` |
| Tree SHA at G18 run | pre-land working tree (Cd-8 harness + lease notes; base tip `2b2e384`) |
| Host Docker engine | Linux (`desktop-linux`) |

## Aggregate

| Metric | Value |
|--------|------:|
| Scenario rows | 72 |
| git pathology_pass | 72 |
| arks resolved_pass | 72 |
| fail outcomes | 0 |

## Non-claims

Researched **bucket B exhausted**. Remainder Closed-deferred **38** (policy/G8/host-FS/platform) + N/A **9** + L3 refuse valve. Bucket A remains Docker-induce-thin. No “221 solved.” Absolute PERF SLOs stay M14/G19.
