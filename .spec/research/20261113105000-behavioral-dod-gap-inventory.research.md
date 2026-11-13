---
id: research:root/20261113105000
title: Behavioral DoD gap inventory (post Spec + first BP subset)
status: active
created: 2026-11-13T10:50:00Z
updated: 2026-11-13T10:50:00Z
tags: [research, dod, behavioral, gap-inventory, no-invent-claimed]
verify: manual
rel:
  depends_on:
    - sot:root/arks-dod-behavioral
    - research:root/20261113194123
  evidence:
    - evidence:root/20261113104851-dod-behavioral-cli
---

# Behavioral DoD gap inventory (2026-11-13)

Tip at inventory: Spec `4536ffe` + product/harness wave (this research). Judge: Docker pin git **2.39.5**.

## Counts

| Bucket | Count | Notes |
|--------|------:|-------|
| BP-PORCELAIN named cmds (§C) | **24** cmd families (+ `remote` 6 subcommands) | Local 18 + remotes 5 families |
| BP-PORCELAIN subset Claimed this wave | **~10** families | [[evidence:root/20261113104851-dod-behavioral-cli]] |
| BP-PORCELAIN still open | **14** | show/diff/checkout/switch/restore/reset/rm/mv/merge/rebase/clone/fetch/pull/push |
| Closed stubs this wave | **3** | `remote show` / `get-url` / `set-url` (were NotImplemented) |
| Hub terminal×6 | **0 Partial / 6 Refuse** | CSV reclass under §D |
| Hub v1 Claimed | **94** | Reality audit still open beyond prior IW |
| Daemon Parity | **20** | Carrying; BP-DAEMON floor |
| CLI survey Partial outside BP | **117** | Non-DoD |
| Hub later Deferred | **119** | Non-DoD |

## Code vs stub (DoD-IN)

| Surface | Was | Now |
|---------|-----|-----|
| `remote show` / `get-url` / `set-url` | NotImplemented exit stub | Implemented + Docker BP greens |
| Other §C local/remotes | Bodies exist (dispatch → cmd::*) | Need STRICT peer recipes before Claimed |
| Hub terminal×6 | Partial stub-OK under optimized DoD | Refuse/N/A (honest) |

## Behavioral DoD met?

**No.** Floor OD + Missing=0 insufficient; BP-PORCELAIN remainder open; BP-DROPIN full §C open; Claimed×94 audit not re-proven this wave.
