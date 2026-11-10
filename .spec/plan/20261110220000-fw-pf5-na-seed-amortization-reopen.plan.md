---
id: plan:root/20261110220000
title: FW-PF5-NA seed amortization reopen (≤0.5×)
status: archived
created: 2026-11-10T22:00:00Z
updated: 2026-11-11T00:14:00Z
tags: [fw-pf5, g19, perf, amortization, residual]
rel:
  depends_on:
    - sot:root/pf5-journey-seed-amortization
    - sot:root/clean-room-journey-compare
    - sot:root/production-residual-register
    - adr:root/20261110162000
    - plan:root/20261110150000
    - evidence:root/20261110153000
  implements: [sot:root/pf5-journey-seed-amortization]
---

# FW-PF5-NA — seed amortization reopen

## Goal

Reopen [[adr:root/20261110162000]] via [[sot:root/pf5-journey-seed-amortization]] and flip as many of the eight Pf5 `aggressive_na` journeys as possible to ≤0.5× in sole Docker G19. Honest **Claimed(partial N/8)** is success.

## Waves

| Wave | Work | Done |
|------|------|------|
| **A0** | Land SoT + this plan; amend journey SoT open-decision row; ADR reopen note | `spec check` |
| **A1** | Harness seed amortization in `verify/g16/tests/journey_compare.rs`; conditional `aggressive_na` in `g19_eval.py` | Host compile advisory |
| **A2** | Sole Docker G19; evidence + residual FW-PF5-* updates | Receipt; push tip |

## Exit

[[evidence:root/20261110224500]] — sole Docker G19 PASS; **Claimed(partial 4/8)** (LOCAL/BRANCH/BUNDLE/ALIAS). Follow-on [[evidence:root/20261110234600]] — **Claimed(partial 6/8)** (+STASH/WORKTREE). Follow-on [[evidence:root/20261111001400]] — **Claimed(partial 7/8)** (+RM-MV); residual CLONE-LOCAL remains `aggressive_na` (HTTP seed amortization exhausted).

## Non-goals

In-process daemon; semantic invention; concurrent G19; merge to main (agent branch only).
