---
id: plan:root/20261109043916
title: M17 full G19 journey allow-list
status: archived
created: 2026-11-09T04:39:16Z
updated: 2026-11-09T07:32:00Z
tags: [milestone, m17, phase-pf3, g19, perf, journey, comparative]
rel:
  depends_on:
    - sot:root/program-roadmap
    - sot:root/clean-room-journey-compare
    - sot:root/interop-test-matrix
    - sot:root/product-verification-clean-room
    - plan:root/20261109024636
    - plan:root/20261109043910
    - evidence:root/20261109213000
  implements: [sot:root/program-roadmap, sot:root/clean-room-journey-compare]
---

# M17 — Full G19 journey allow-list (Phase Pf3)

## Goal

Expand comparative performance gate **G19** from the frozen M14 allow-list `{J-SUBMODULE-01, J-REPACK-01}` to **all current G16 timing journeys** in `verify/matrix-journey.md`, such that for every journey ID: **arks_median ≤ 0.8 × git_median** (`median_p95`, release), **or** the journey is explicitly **N/A** in SoT with rationale (must be rare; default is meet the bar).

Functional full-symmetry (**M16 / G21**) for touched surface has priority; perf work continues only on functionally green units. Product Done requires **both** M16 complete-symmetry ledger green **and** this full G19 allow-list green.

## Normative bar (M17 exit)

| Field | Rule |
|-------|------|
| Allow-list | Every required ID in `verify/matrix-journey.md` (current G16 catalog) |
| Ratio | arks median ≤ 0.8 × git median |
| Venue | Docker clean-room `g19` only; host advisory |
| N/A | Journey may be SoT-N/A’d only with explicit reason (e.g. network-bound G8-class — none currently in G16 required set) |
| Regression | G16 functional green; G14/G20/G21 as applicable |

## Waves

| Wave | Focus |
|------|-------|
| **Pf3-1** | Add all G16 IDs to `verify/matrix-g19.md`; baseline Docker timings; identify failers |
| **Pf3-2** | Optimize failers (alloc/IO/pack/index paths) until ≤0.8×; no semantic invention |
| **Pf3-3** | Final G19 PASS receipt for full allow-list; pin tip SHA; residual register Closed |

## Steps

1. After M16 Sy waves that touch a journey’s surface are functionally green (or journey already green under G16/G20).
2. Expand matrix-g19 allow-list; run Docker G19; record ratios.
3. Optimize until green or document SoT N/A (ask if N/A temptation is large).
4. Evidence receipt + public honesty.

## Out of scope

- Redefining M14 historical exit (frozen allow-list receipt stands).
- Substituting host timings for verified.
- Blocking M16 functional work on perf.

## Record

| Event | Notes |
|-------|-------|
| Plan created | 2026-11-09 — after M14 exit evidence:root/20261109213000 |
| Pf3-1 matrix | 2026-11-09 — allow-list expanded; Docker baseline [[evidence:root/20261109064500]] — 2/19 PASS (REPACK+SUBMODULE); 17 failers → Pf3-2 |
| Pf3-2 progress | 2026-11-09 — fsync-match Git 2.39 default; hardlink-or-copy local clone; checkout OID skip; harness seed amortization — [[evidence:root/20261109070000]] PASS clones+repack+submodule; TAG N/A cold-start; residual short porcelain blocked by ~5ms vs ~1ms process floor |
| Pf3-2/Pf3-3 Done | 2026-11-09 — Docker G19 exit 0; **19/19 measured PASS**; `na: {}` (no TAG/DIFF timer-floor N/A); tip land cites [[evidence:root/20261109090000]] |


