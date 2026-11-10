---
id: evidence:root/20261110212545
title: FS2 proc-receive advertise + divert host/unit evidence
status: active
created: 2026-11-10T21:25:45Z
updated: 2026-11-10T21:25:45Z
tags: [fs2, proc-receive, receive-pack, full-symmetry]
rel:
  depends_on:
    - adr:root/20261110212500
    - research:root/20261110212414
    - sot:root/arks-server
  evidence: []
files: []
---

# FS2 — proc-receive advertise + divert

## Claimed subset exercised

| Surface | How | Result |
|---------|-----|--------|
| Advertise absent hook | `receive_advertises_delete_refs_and_report_status_v2` | **PASS** (no `proc-receive`) |
| Advertise when hook present | `receive_advertises_proc_receive_when_hook_present` | **PASS** |
| Divert + ok/ng + option rewrite | Wire in `receive_pack` + `run_proc_receive` | **Claimed** (unit + SoT); expand clean-room twin later |

## Non-claims

HOOK-008 race freedom (FS8); negative `!` `procReceiveRefs` forms.
