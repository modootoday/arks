---
name: research-to-sot
description: >
  Use when promoting a Specfold research note into normative SoT/plan meaning,
  or deciding to keep research as survey-only. Enforces evidence honesty,
  English-only, and Spec-first commit order.
---

# Research → SoT pipeline

## Preconditions

- Skills `design-sot`, `commit-governance`
- Rules `05`, `25`, `26`, `27`, `28`, `29`, `35`
- Classification: `.spec/README.md`

## Steps

1. **Classify the research:** survey / options / recommendation / ready-to-norm.
2. If recommendation only: keep `status: active` research; do **not** silently rewrite product SoTs.
3. If promoting to SoT:
   - Draft or amend owning `.spec/sot/*.sot.md` with MUST/MAY/out-of-scope.
   - Mark **design / Partial** until Claimed + evidence (rule `29`).
   - Bump SoT `Version` only on meaning change (rule `26`).
   - English only — convert any chat/Hangul notes first (rule `05`).
4. Cross-link `rel.depends_on` / research id; do not paste large copyrighted primaries.
5. **Commit Spec meaning first** (rule `27`); implementation later.
6. Never promote `.spec.local/**` dumps as public Claimed (rule `35`).
7. `spec check` before done.

## Refuse

- Invent Claimed/PASS from research alone.
- Re-Claim trimmed surfaces from archival research.
- Commit secrets, pin extracts, or Hangul operator briefings.
