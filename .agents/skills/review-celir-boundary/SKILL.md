---
name: review-celir-boundary
description: >
  Use when reviewing storage or CELIR-related design/code: ensure Git interchange
  stays filesystem-faithful and CELIR (not adopted; ADR 20261119200000) stays out
  of the Cargo graph entirely. M24 / G22–G25 are Withdrawn.
---

# Review CELIR boundary

## Focus

1. Git-compat mode must not silently replace documented object/ref/pack/index meanings.
2. **No** CELIR Cargo edges in any form — no feature, path dependency or companion crate (ADR `20261119200000`, supersedes `20261110052030`).
3. Docs/code must not implement ObjectStore via CELIR, re-add `celir-*` dependencies, or present CELIR as a product path; historical M24/G22–G25 receipts are history only.
4. Ask on ambiguous cross-cutting PRs (rule `40-ambiguity-ask`).

## Method

Diff → list files that touch both Git-compat and storage paths → **boundary breach: yes/no** (also check `Cargo.toml` / features for `celir`; confirm there is no `celir` feature or dependency).

Rule: `50-celir-boundary`. Owning SoT: `.spec/sot/storage-celir-boundary.sot.md`. Decision: `.spec/adr/20261119200000-celir-not-adopted-withdraw-m24.adr.md` (supersedes `20261110052030`). Also: `.spec/sot/git-compat-surface.sot.md` out-of-scope section.
