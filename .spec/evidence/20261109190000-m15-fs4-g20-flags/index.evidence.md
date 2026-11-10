---
id: evidence:root/20261109190000
title: M15 Fs-4 clean-room G20 add -A and status --porcelain
status: active
created: 2026-11-09T19:00:00Z
updated: 2026-11-10T00:57:44Z
tags: [g20, m15, fs-4, clean-room, flags, verified]
rel:
  depends_on: [plan:root/20261109035335, evidence:root/20261109183000]
  evidence: []
files:
  - {path: receipt.json, sha256: cb984e8c8e3ec4c280b0062973924df43321e5d7aceaad20a7302e6143e3a783, bytes: 670, store: repo}
---

# M15 Fs-4 — add -A + status --porcelain

Closes Wave **Fs-4** (flag gaps; landed ahead of Fs-3 refuse-closed per alias value). G20 PASS including Fs-1/Fs-2 regression + G14.

| ID | pass |
|----|------|
| M15-FS4-01 | yes |

Reproduce: `docker compose -f verify/docker-compose.yml run --rm g20`
