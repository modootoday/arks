---
id: evidence:root/20261110141000
title: M21 Dz-1 clean-room G17 host-FS platform claims
status: active
created: 2026-11-10T14:10:00Z
updated: 2026-11-10T14:10:00Z
tags: [g17, m21, dz-1, clean-room, concurrency, host-fs, verified]
rel:
  depends_on: [plan:root/20261110140000, sot:root/program-roadmap, sot:root/interop-test-matrix, sot:root/multi-agent-git-concurrency-scenarios, sot:root/product-verification-clean-room, evidence:root/20261110080000, research:root/20261110021546]
  evidence: []
files:
  - {path: receipt.json, sha256: 5895a4671357fa71405d6df9e5f2cb3d4de03b2af7afe5f666d077cbea5ff221, bytes: 23153, store: repo}
---

# M21 Dz-1 — clean-room G17 host-FS / platform claims

## Context

Closes Wave **Dz-1** of [[plan:root/20261110140000]]: Docker host-FS/platform honesty for twelve formerly Closed-deferred IDs + Cm/Cx regression (**119**/119 dual PASS). Does **not** claim NFS/SMB dialects invented. CELIR excluded; L3 refuse-closed. Deferred work queue remaining after this wave: **26**.

Attached: `receipt.json` (G17 issue-compare, wave `Dz-1`).

## Environment

| Field | Value |
|-------|-------|
| Compose | `verify/docker-compose.yml` service `g17` |
| Entrypoint | `verify/scripts/run-g17.sh` |
| Image | `arks-verify-m1:0.1.0` |
| Image digest | `sha256:5736c5687d3b5775e187ca3d21fbc5b2a5a2bf8965c5381778ee478eb185adf0` |
| Stock Git (in-image) | `git version 2.39.5` |
| Tree SHA at G17 run | tip `8ea777e` + Dz-1 harness |
| Host Docker engine | Linux (`desktop-linux`) |

## Aggregate

| Metric | Value |
|--------|------:|
| Scenario rows | 119 |
| git pathology_pass | 119 |
| arks resolved_pass | 119 |
| fail outcomes | 0 |

## Non-claims

No “221 solved.” Absolute PERF SLOs stay M14/G19. Host-FS notes are honesty (cite lockfile same-FS), not multi-host green claims.
