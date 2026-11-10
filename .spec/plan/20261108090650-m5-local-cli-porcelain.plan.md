---
id: plan:root/20261108090650
title: M5 Local CLI foundation
status: active
created: 2026-11-08T09:06:50Z
updated: 2026-11-08T10:48:37Z
tags: [milestone, m5, phase-l, cli, foundation]
rel:
  depends_on:
    - sot:root/program-roadmap
    - plan:root/20261108090649
    - sot:root/cli-surface
    - sot:root/crate-architecture
    - sot:root/interop-test-matrix
    - sot:root/product-verification-clean-room
    - sot:root/arks
    - evidence:root/20261108103610
  evidence: [evidence:root/20261108104837]
  implements: [sot:root/program-roadmap]
---

# M5 — Local CLI foundation (Phase L)

## Goal

Thin `arks` binary composing proven libraries for [[sot:root/cli-surface]] **Wave L** local foundation commands; unlock full **G3** via commit/index path. **Not production** — production is **M9** CLI-parity closure ([[sot:root/program-roadmap]]).

## In scope / artifacts

| Artifact | Role |
|----------|------|
| package/bin `arks` | Wave L: init, hash-object, cat-file, update-ref, rev-parse, add, status, commit, log, show |
| Help / UX | Lists only implemented commands; refuse unimplemented flags |

## Out of scope

- Wave H remotes (M6).
- Waves Pa/Pb/Pc full inventory (M7–M9).
- Lease / CELIR / forge UX.
- Claiming CLI-parity production.

## Depends on

**M4** exit (including gix-exit).

## Exit gates

| Gate | Clean-room | Advisory |
|------|------------|----------|
| **G6** cli-local-foundation | Authoritative when harness exists | Host argv smoke |
| **G3** index-commit-roundtrip (**full**) | Authoritative | Host |
| G1–G4 regression | Clean-room | Host |

## Ordered work items

1. **Design** — Map each Wave L command to owning lib APIs; refuse unimplemented flags.
2. **Scaffold** — `arks` bin package; wire workspace.
3. **Implement** — plumbing commands first, then add/status/commit/log/show.
4. **Verify** — G6 + full G3 + G1–G4 regression; help surface audit.

## Open questions

- Pack helper bins: **closed for M5** — library-only (`arks-pack`); no Wave L pack bins ([[sot:root/arks]]).
- Checkout materialization shared with future clone (design stub OK; no server role) — still open until M6 clone design.

## Cross-links

[[sot:root/program-roadmap]], [[sot:root/cli-surface]], [[sot:root/interop-test-matrix]], [[sot:root/arks]], `verify/matrix-m5.md`
