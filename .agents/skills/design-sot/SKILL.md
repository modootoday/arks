---
name: design-sot
description: >
  Use when writing or extending a SoT or plan before code: new specification,
  design panel, or spec-first design. Prefer over implementing when the user
  asks for design/SoT (even without saying "SoT").
---

# Design SoT

## Goal

Normative SoT/plan slice: vocabulary, decision rules, non-coverage — **before** code.

## Steps

1. Brief (problem, constraints, non-goals ≤15 lines).
2. `spec context <path>`; read owning `.spec/sot` / research; cite primary Git docs for any semantic claim.
3. Check CELIR/lease boundary (rule `50-celir-boundary`) — do not fold storage invention into Git-compat SoTs.
4. Draft one SoT: version, scope, MUST/MAY/out-of-scope, verification intent, open decisions.
5. Cross-link related SoTs; do not paste large copyrighted primary text.
6. `spec check`. Do not implement unless the user also asked for code.

## Cite

- Skill pattern: `C:\workspaces\humanymous\.agents\skills\design-sot\SKILL.md`
- Governance: `.spec/sot/agent-governance.sot.md`
