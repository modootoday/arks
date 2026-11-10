---
id: evidence:root/20261111032001
title: g8-live Environment secrets re-probe (still zero) — live latency Blocked
status: active
created: 2026-11-10T18:20:01Z
updated: 2026-11-10T18:20:01Z
tags: [g8-live, secrets, blocked, honesty]
verify: manual
rel:
  depends_on:
    - evidence:root/20261111025000
    - sot:root/interop-test-matrix
    - adr:root/20261110220000
---

# g8-live secrets re-probe

| Item | Value |
|------|-------|
| Probe | `gh api repos/modootoday/arks/environments/g8-live/secrets` |
| `total_count` | **0** |
| Secret names | `[]` |
| Disposition | **Blocked** — no live latency remasure; harness Claimed remains |
| Force-off | `ARKS_G8_LIVE=0` |

Matches prior [[evidence:root/20261111025000]]. No invented medians. Unblock: operator mint PAT + `ARKS_G8_TOKEN` / `ARKS_G8_REPO_URL` (`total_count ≥ 2`) then workflow + evidence.
