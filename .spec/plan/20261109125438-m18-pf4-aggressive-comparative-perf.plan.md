---
id: plan:root/20261109125438
title: M18 Pf4 aggressive comparative perf
status: archived
created: 2026-11-09T12:54:38Z
updated: 2026-11-09T14:30:00Z
tags: [milestone, m18, phase-pf4, g019-comparative-performance, perf, journey, comparative, aggressive]
rel:
  depends_on:
    - sot:root/program-roadmap
    - sot:root/clean-room-journey-compare
    - sot:root/interop-test-matrix
    - sot:root/product-verification-clean-room
    - sot:root/multi-agent-git-concurrency-scenarios
    - research:root/20261109125438
    - plan:root/20261109043916
    - plan:root/20261109024636
    - evidence:root/20261109090000
    - evidence:root/20261109130000
    - evidence:root/20261109141500
    - evidence:root/20261109143000
  implements: [sot:root/program-roadmap, sot:root/clean-room-journey-compare]
---

# M18 — Pf4 aggressive comparative performance

## Goal

After **M17 / Pf3** full G19 exit (19/19 ≤0.8×, N/A-free — [[evidence:root/20261109090000]]), stage **Phase Pf4** for a **materially stronger** timing bar on the same G16 journey catalog without rewriting M14/M16/M17 historical exits.

## Normative bars

| Bar | Rule | Relation to history |
|-----|------|---------------------|
| **M17 regression** | Every current G16 journey: arks_median ≤ **0.8×** git_median | Frozen — MUST stay green |
| **Pf4 aggressive** | Claimed progressive set: arks_median ≤ **0.5×** git_median (`median_p95`, release) | Additive — does **not** redefine M17 |
| **N/A** | Forbidden for current catalog (`na: {}`) | Same honesty as M17 |
| **Venue** | Docker sole `g019-comparative-performance` (no concurrent G19; OOM/137) | [[sot:root/product-verification-clean-room]] |

## Claimed set — Wave Pf4-1 (progressive)

Target ≤0.5× on journeys with headroom (absolute ms or ratio room). Short quantum journeys MAY remain residual if timer floor dominates.

| ID | Pf4-1 intent |
|----|--------------|
| J-DIFF-01 | Hot path: buffered unified emit + bulk `add` index map |
| J-ARCHIVE-01 | Buffered ustar write; keep streaming |
| J-LOCAL-01 / J-ALIAS-01 | Benefit from add/index wins + regression |
| J-CLONE-LOCAL-01 / J-TAG-01 / J-REPACK-01 / J-SUBMODULE-01 | Already strong — regression + count toward ≤0.5× when still under |
| Remaining catalog | Keep ≤0.8×; document residual vs 0.5× |

## Waves

| Wave | Focus | Done |
|------|-------|------|
| **Pf4-1** | Research land + hot-path impl (diff BufWriter, add HashMap stage, untracked sort-once, archive BufWriter) + sole Docker G19 | Receipt + ratio table vs M17; ≥ material improvement on DIFF/ARCHIVE; M17 bar green; claim ≤0.5× IDs achieved |
| **Pf4-2** | Residual >0.5× — parallel bulk add + archive prefetch + hot loose read/write + Mutex store | Expanded ≤0.5× claim set (11/19) |
| **Pf4-3** | Catalog-wide ≤0.5× or SoT journey-class targets with residual register | **Done** — M18 exit |

## Constraints

- Zero-gix; CELIR out; lease ≠ skip locks; no Git semantic invention.
- Bottom-up: object/index helpers before CLI compose (rule `70`).
- One Docker G19 at a time.
- G8 remains Closed-deferred.

## Out of scope

- Redefining M14/M17 receipts.
- Host-laptop SLOs as verified.
- In-process multi-command daemon (Pf4-3 Open only if SoT opens).

## Steps (Pf4-1)

1. Land [[research:root/20261109125438]]; amend roadmap M18 stub; expand matrix-g019-comparative-performance Pf4 notes.
2. Implement hot paths; host `cargo test` advisory.
3. Sole `docker compose … run --rm g019-comparative-performance`; compare to [[evidence:root/20261109090000]].
4. Evidence receipt + `spec check` + commit + push.
5. `/goal` next wave if residuals remain above 0.5×.

## Record

| Event | Notes |
|-------|-------|
| Plan created | 2026-11-09 — after M17 exit [[evidence:root/20261109090000]] |
| Research | [[research:root/20261109125438]] |
| Pf4-1 Done | 2026-11-09 — sole Docker G19 PASS; DIFF `diff --cached` 23→6 ms; ARCHIVE 0.75→0.67; ≤0.5× hits 9/19; [[evidence:root/20261109130000]] |
| Pf4-2 Done | 2026-11-09 — sole Docker G19 PASS; DIFF `add -A` 31→17 ms (ratio 0.57→0.44); ARCHIVE 0.67→0.33; CHERRY ≤0.5×; hits 11/19; [[evidence:root/20261109141500]] |
| Pf4-3 / M18 exit | 2026-11-09 — sole Docker G19 PASS; 11/19 ≤0.5× claim + journey-class residual register; M17 ≤0.8× green; [[evidence:root/20261109143000]] |
