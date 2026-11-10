---
id: plan:root/20261108090645
title: M0 Specfold program canon
status: active
created: 2026-11-08T09:06:45Z
updated: 2026-11-08T18:38:00Z
tags: [milestone, m0, specfold, design]
rel:
  depends_on:
    - sot:root/program-roadmap
    - sot:root/agent-dev-readiness
    - sot:root/product-verification-clean-room
    - research:root/20261108090644
  implements: [sot:root/program-roadmap]
---

# M0 — Specfold program canon

## Goal

Make the normative product brain coherent enough that implementers can stage the path to **CLI-parity production (M9)** without inventing Git semantics or architecture.

## In scope

- Active SoTs: product intent, crate architecture, core object model, git-compat, **CLI symmetric surface**, clean-room, interop matrix, CELIR boundary, lease intent, remote/public/structural, naming, English-only, governance/readiness.
- Plans: development build order, core build slice, program milestones / Wave-1, M0–M10 + CELIR plan set.
- ADR: native engine + zero-`gix` end state ([[adr:root/20261108083507]]).
- `spec check` clean (or residual warnings documented).

## Out of scope

- Any Rust/Cargo workspace or crate implementation.
- Closing clean-room image digests; inventing Gate recipes.
- Claiming product verified.

## Depends on

— (program start)

## Exit gates

| Gate | Venue |
|------|-------|
| `spec check` | Docs / Specfold (not interop clean-room) |

## Ordered work items (design → scaffold → implement → verify)

1. **Design** — Ensure roadmap defines **CLI-parity production (M9)** vs differentiator (M10) ([[sot:root/program-roadmap]] v0.3+).
2. **Design** — Per-milestone staged plans linked from roadmap (including porcelain waves).
3. **Scaffold** — Specfold `[[assets]]` / AGENTS router only as needed (no Cargo).
4. **Implement** — N/A for product code.
5. **Verify** — `spec check`; audits [[research:root/20261108090644]], [[research:root/20261108183820]].

## Open questions

- Clean-room image/compose/digest paths (owned by clean-room SoT).
- Whether `.spec/` stays fully public with the repo ([[sot:root/public-surface]] Open).

## Cross-links

[[sot:root/program-roadmap]], [[plan:root/20261108085816]], [[sot:root/agent-dev-readiness]], [[adr:root/20261108083507]]
