---
id: plan:root/20261109024636
title: M14 Comparative performance unlock
status: archived
created: 2026-11-09T02:46:36Z
updated: 2026-11-09T21:30:00Z
tags: [milestone, m14, phase-pf, performance, comparative, g16, g19]
rel:
  depends_on:
    - sot:root/program-roadmap
    - sot:root/clean-room-journey-compare
    - sot:root/public-surface
    - sot:root/interop-test-matrix
    - sot:root/multi-agent-git-concurrency-scenarios
    - research:root/20261109023117
    - plan:root/20261109024634
    - plan:root/20261109024635
    - evidence:root/20261109140000
    - evidence:root/20261109163000
  implements: [sot:root/program-roadmap]
---

# M14 — Comparative performance unlock (Phase Pf) — **future, after issue-resolution**

## Goal

Stage the **future** unlock where clean-room comparative timing may become a **pass/fail** product gate: arks MUST be **≥20% faster** (wall-clock) than in-image stock Git on an explicitly claimed G16 journey subset under `median_p95` / release — gate **G19**.

**Ordering (normative):** **Performance / timing speed optimization comes AFTER issue-resolution verification.** M12 (**G17**) and M13 (**G18**) claimed sets MUST close first. Do **not** start Pf engineering waves or imply perf-opt as parallel **Done** while the issue track is open. Record-only timings during Cm/Cd are observability only.

**Separation:** G19 MUST NOT substitute for **G17/G18**. “Issue fixed” and “faster by ≥20%” are different bars. Until M14 exit, G16 timings remain **record-only** per [[sot:root/clean-room-journey-compare]].

## Normative bar (when claimed)

| Field | Rule |
|-------|------|
| **Comparative floor** | For each claimed journey row, arks median wall-clock ≤ **80%** of stock Git median (**≥20% faster**), same clean-room image, same seed, release `arks`, iterations ≥ 3, `aggregation=median_p95` |
| **Functional class** | Still hard gate — perf never excuses functional mismatch |
| **Claim set** | Explicit allow-list (not all G16 / not all catalog IDs) |
| **Public copy** | MAY state comparative win only with evidence + [[sot:root/public-surface]] honesty |

## Relation to concurrency catalog

PERF-001 / PERF-006 / AGENT-003 may improve measured journeys after M12/M13, but they **auto-pass neither** G17/G18 nor G19. Issue-resolution receipts prove pathology/resolve; G19 proves speed.

## In scope

1. Amend journey-compare + interop matrix for **G19** at M14 exit only.
2. Freeze journey allow-list + aggregation.
3. Clean-room evidence authorizing public comparative language.

## Out of scope (until exit)

- Treating ≥20% as M9–M13 or G17/G18 exit criterion.
- Host-laptop SLOs; redefining CLI-parity / lease / server bars; G8 as perf oracle.

## Depends on

- **M12 exit and M13 exit** — both required before Pf / M14 engineering or Done claims.
- Open issue-track claim-sets MUST NOT be treated as “good enough for parallel ≥20% work.”

## Exit gates

| Gate | Clean-room | Advisory |
|------|------------|----------|
| **G19** `perf-comparative` | Authoritative ≥20% on allow-list | Host timings advisory only |
| G16 functional + G14 regression | Must stay green | Host |

## Ordered work items

1. **Design** — Close absolute-perf open in journey-compare; name allow-list; wire G19. **Done (Pf-1)** — `verify/matrix-g19.md`; compose `g19`.
2. **Engine** — Optimizations for claimed rows only (no premature L3). Pf-1 uses already-passing journeys; Pf-2 expands via opt.
3. **Verify** — G19 receipt; public-surface honesty.

## Claimed set — Wave Pf-1

| Journey ID | Bar |
|------------|-----|
| **J-SUBMODULE-01** | arks_median ≤ 0.8 × git_median |

Wave Pf-1 **Done** = Docker G19 PASS for allow-list + G16 functional green. `J-CLONE-LOCAL-01` deferred to Pf-2 (run-to-run ratio noise).

## Pf-1 close

| Journey | G19 |
|---------|-----|
| J-SUBMODULE-01 | PASS (ratio 0.3333) |

Evidence: [[evidence:root/20261109170000]].

## Claimed set — Wave Pf-2

| Journey ID | Bar |
|------------|-----|
| **J-SUBMODULE-01** | Pf-1 regression |
| **J-REPACK-01** | arks_median ≤ 0.8 × git_median (multi-object `repack -ad`) |

Wave Pf-2 **Done** = Docker G19 PASS for expanded allow-list + G16 functional green. `J-CLONE-LOCAL-01` remains Open for Pf-3+.

## Pf-2 close

| Journey | G19 |
|---------|-----|
| J-SUBMODULE-01 | PASS |
| J-REPACK-01 | PASS |

Evidence: [[evidence:root/20261109210000]].

## M14 exit allow-list (frozen)

Explicit comparative allow-list for **M14 Done** (not all G16):

| Journey ID | Wave introduced |
|------------|-----------------|
| **J-SUBMODULE-01** | Pf-1 |
| **J-REPACK-01** | Pf-2 |

Remaining G16 IDs (incl. `J-CLONE-LOCAL-01`) stay **Open** for post-M14 residual expansion — they do **not** block M14 exit. Exit evidence: [[evidence:root/20261109213000]].

## Cross-links

[[sot:root/program-roadmap]], [[sot:root/clean-room-journey-compare]], [[sot:root/interop-test-matrix]], [[plan:root/20261109024634]], [[plan:root/20261109024635]]
