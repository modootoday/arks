---
id: adr:root/20261110162000
title: Pf5 journey-class seed amortization path
status: active
created: 2026-11-10T16:20:00Z
updated: 2026-11-10T08:00:00Z
tags: [pf5, g19, perf, aggressive-na, amortization, residual]
rel:
  depends_on:
    - sot:root/clean-room-journey-compare
    - sot:root/pf5-journey-seed-amortization
    - sot:root/interop-test-matrix
    - sot:root/production-residual-register
    - plan:root/20261110150000
    - plan:root/20261110220000
    - plan:root/20261109125438
    - evidence:root/20261110153000
    - evidence:root/20261109143000
    - evidence:root/20261110224500
---

# Pf5 journey-class seed amortization path

## Context

M18 **Pf4-3** closed with **11/19** journeys ≤0.5× and **8/19** journey-class residuals classified **`aggressive_na`** in Docker G19 ([[evidence:root/20261110153000]], [[evidence:root/20261109143000]]). Residual reasons cite **quantum timer floors** and **process-tax** (short multi-cmd walls) where stock Git medians are already sub-millisecond to low-ms — ≤0.5× is not honestly claimable without changing the measurement harness (seed amortization, in-process multi-command batching, or revised journey definitions).

M22 closed Pf5 as **terminal SoT disposition** — not a Deferred implement wave ([[plan:root/20261110150000]]).

## Decision

1. **Historical disposition — Closed (aggressive_na):** Through M22, the eight Pf5 journeys stayed **Closed (aggressive_na)** under M17 ≤0.8× measured greens.
2. **Reopen (authorized):** [[sot:root/pf5-journey-seed-amortization]] + [[plan:root/20261110220000]] authorize harness seed amortization and sole Docker G19 remeasure. Agents MUST NOT run sole-runner G19 in parallel with other Docker gates.
3. **Claim honesty:** A journey flips to ≤0.5× **claimed** only with G19 receipt; otherwise it remains `aggressive_na` under the amortization SoT cite. Umbrella MAY be **Claimed(partial N/8)**.
4. **Journey inventory (frozen class):**

   | Journey | Class | Baseline Pf5 disposition |
   |---------|-------|--------------------------|
   | J-BRANCH-01 | quantum timer floor | Closed (aggressive_na) until G19 flip |
   | J-BUNDLE-01 | quantum timer floor | Closed (aggressive_na) until G19 flip |
   | J-RM-MV-01 | quantum timer floor | Closed (aggressive_na) until G19 flip |
   | J-STASH-01 | quantum timer floor | Closed (aggressive_na) until G19 flip |
   | J-LOCAL-01 | process-tax | Closed (aggressive_na) until G19 flip |
   | J-ALIAS-01 | process-tax | Closed (aggressive_na) until G19 flip |
   | J-CLONE-LOCAL-01 | mid | Closed (aggressive_na) until G19 flip |
   | J-WORKTREE-01 | mid | Closed (aggressive_na) until G19 flip |

5. **Forbidden:** Silent keep-at-0.8× as a substitute for ≤0.5× marketing; inventing remeasured ratios without G19 receipt; in-process multi-command daemon without a dedicated future ADR.

## Consequences

- [[sot:root/production-residual-register]] **FW-PF5-NA** and P6 journey rows track per-journey Claimed vs remaining NA after G19.
- Harness changes land under [[plan:root/20261110220000]]; carrying M17/M18 receipts remain valid for ≤0.8× / prior ≤0.5× sets.

## Supersedes / amends

- Documents the reopen path previously named inline on **FW-PF5-NA** only; reopen now live via [[sot:root/pf5-journey-seed-amortization]].
- Does **not** redefine M17/M18 carrying receipts.
