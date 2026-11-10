---
id: research:root/20261108183820
title: CLI-parity roadmap restructure audit
status: active
created: 2026-11-08T18:38:20Z
updated: 2026-11-08T18:38:20Z
tags: [roadmap, milestones, audit, production, cli-parity]
rel:
  depends_on:
    - sot:root/program-roadmap
    - sot:root/cli-surface
    - sot:root/product-intent
    - sot:root/git-compat-surface
    - sot:root/interop-test-matrix
    - sot:root/public-surface
    - plan:root/20261108082148
    - research:root/20261108090644
    - research:root/20261108183800
---

# CLI-parity roadmap restructure audit

Audit of raising arks production from **M6 subset Git-compat** to **M9 CLI-parity** (single bar). Design only.

## Prior bar (v0.2) — superseded

| Axis | Old expectation |
|------|-----------------|
| Production | M6 = local Phase A CLI + GitHub remotes + clean-room G0–G7 |
| Full porcelain | Deferred forever as non-production |
| Lease | M7 differentiator |

## New bar (v0.3)

| Axis | Expectation |
|------|-------------|
| Production | **M9** = Git CLI symmetric inventory closure + formats + remotes + G14 |
| Porcelain | Phased M5→M9; end-state not deferred |
| Lease | **M10** differentiator |
| Dual production bars | **Rejected** |

## Holes closed this pass

| ID | Finding | Disposition |
|----|---------|-------------|
| **C1** | Compatibility omitted CLI symmetry | Locked in product-intent + cli-surface |
| **C2** | git-compat SoT conflated formats with “enough compatibility” | Layer split; production points at CLI SoT |
| **C3** | M6 labeled production | Remotes-only; production → M9 |
| **C4** | No porcelain wave gates | G12–G14 added |
| **C5** | Public surface could overclaim drop-in git | Honesty rules added |
| **C6** | Build-order / rule 70 / AGENTS still said production=M6 | Updated in same change set |
| **C7** | Same-`.git` layout + `alias git=arks` drop-in not explicit as first-class production | Locked in product-intent, git-compat, cli-surface, roadmap M9/G14, public-surface |

## Residual opens

| ID | Open |
|----|------|
| R1 | Exact Pa/Pb/Pc command partition tables |
| R2 | Clean-room Git pin digest still Open |
| R3 | Server-role Open |
| R4 | Allocator crate Open |
| R5 | G8 network policy Open |
| R6 | CELIR readiness not flipped |
| R7 | Plumbing inventory minimum detail |
| R8 | Prior M1/M2 implementation trees need gap-fill/re-verify under restart-from-M1 policy |

## Alignment check

| Layer | Order |
|-------|-------|
| Phases | C → S → R → P → L → H → Pa → Pb → Pc → E |
| Milestones | M0…M4 engine; M5 L; M6 H; M7–M9 porcelain; **M9 production**; M10 lease |
| Gates | + G12/G13/G14; G6 renamed foundation |

## Record

| Date (UTC) | Note |
|------------|------|
| 2026-11-08 | Restructure audit for CLI-parity raise. No Rust in this pass. |
