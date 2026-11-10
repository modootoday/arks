---
id: plan:root/20261110150000
title: M22 residual terminal close
status: archived
created: 2026-11-10T15:00:00Z
updated: 2026-11-10T15:40:00Z
tags: [milestone, m22, residual, terminal-close, g8, g19, pf5, server, l3, judgment]
rel:
  depends_on:
    - sot:root/program-roadmap
    - sot:root/production-residual-register
    - sot:root/interop-test-matrix
    - sot:root/product-verification-clean-room
    - sot:root/clean-room-journey-compare
    - sot:root/arks-server
    - sot:root/public-surface
    - adr:root/20261108232800
    - plan:root/20261110140000
    - plan:root/20261109125438
    - evidence:root/20261109143000
  implements: [sot:root/program-roadmap, sot:root/production-residual-register]
---

# M22 — Residual terminal close (post-M21)

## Goal (binding Done bar)

Unattended close **all remaining arks work after M21** so judgment checklist No/Partial items and residual Closed-deferred rows that are still work items become **terminal**.

**SUCCESS** only when all hold:

1. [[sot:root/production-residual-register]] has **zero bare Opens** and **no Closed-deferred that is still a work item**.
2. **G8** live-forge policy unlocked; clean-room/live smoke **Closed** or **Refuse-closed with ADR**.
3. M18 journey-class leftovers promoted so G19 shows **19/19 ≤0.5×** OR each leftover is **Refuse/N/A with SoT cite** (no silent keep-at-0.8).
4. Server gaps **SSH / `git://` / dumb HTTP / protocol v2** each terminal: implement+verify OR N/A/Refuse with SoT.
5. **L3** shipped with clean-room bar OR Refuse-closed ADR remains explicit terminal.
6. Judgment checklist No/Partial items all terminal.
7. `spec check` clean; commit+push each wave.

**FORBIDDEN:** stop because historically Closed-deferred or journey-class documented.

CELIR excluded. No force-push. No fake greens.

## Baseline (post M21)

| Item | State |
|------|-------|
| Catalog | Deferred=0 Claimed=212 N/A=9 (+ L3 valve refuse) |
| G17 / G18 | 145/145 ; 72/72 |
| M16 / M17 / M18 | alias Done; 19/19 ≤0.8×; **11/19 ≤0.5×** + journey-class residuals |
| Tip | ~`ff67014` on `origin/main` |

## Judgment checklist → work queue

| Checklist | Prior | Terminal target |
|-----------|-------|-----------------|
| Live GitHub smoke (G8) | No (policy deferred) | Unlock SoT + Closed or Refuse ADR |
| Full git server replace | No / Partial (smart HTTP only) | Each of SSH/`git://`/dumb/v2 terminal |
| ≤0.5× journeys | Partial 11/19 | 19/19 or Refuse/N/A cite per leftover |
| L3 HighConcurrency | No (refuse) | Ship+bar OR explicit Refuse ADR terminal |
| Public-flip / G9 Closed-deferred | deferred work items | Terminal Closed / N/A / Refuse |

## Done rules (normative)

| Disposition | Allowed when |
|-------------|--------------|
| **Closed (implement+verify)** | Docker clean-room receipt for the claimed gate |
| **Refuse-closed** | ADR + owning SoT; product valve or out-of-bar surface |
| **N/A** | Primary-doc / product-intent cite; not a silent keep |
| **Closed-deferred** | **Forbidden as M22 exit** for any row still a work item |

Prefer implement+verify over Refuse. Refuse/N/A only with primary-doc or product-intent cite.

## Waves

| Wave | Focus | Done |
|------|-------|------|
| **Rc-0** | This plan + roadmap/residual Done-bar canon | `spec check`; commit+push |
| **Rc-1** | G8 policy unlock + harness + Closed or Refuse ADR | Evidence; residual G8 terminal |
| **Rc-2** | Server SSH / `git://` / dumb / v2 terminals | SoT honesty (+ impl only if claimed) |
| **Rc-3** | Pf5: ≤0.5× leftovers or aggressive N/A/Refuse cites | Docker G19 + evidence |
| **Rc-4** | L3 terminal Refuse refresh (or ship) | Residual + ADR cite |
| **Rc-5** | G9 + public-flip Closed-deferred → terminal | Residual empty of work deferred |
| **Rc-exit** | Residual recount; umbrella SUCCESS | Final commit+push |

## Out of scope

- CELIR product deps.
- Force-push; fake greens; inventing Git wire semantics.
- Redefining M9–M21 historical exits / carrying receipts.
- Marketing “all races fixed” or “faster than Git” outside receipts.

## Status

**Closed** — Rc-0…Rc-exit Done. G8 Refuse-closed [[evidence:root/20261110152000]]; Pf5 G19 19/19 terminal [[evidence:root/20261110153000]]; server/L3/G9/public-flip terminal in residual register; carrying exit [[evidence:root/20261110154000]].
