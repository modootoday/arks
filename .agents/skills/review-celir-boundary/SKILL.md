---
name: review-celir-boundary
description: >
  Use when reviewing storage or CELIR-related design/code: ensure Git interchange
  stays filesystem-faithful; CELIR may appear only as optional index-only feature
  `celir-index` (default OFF) — never as ObjectStore. M24 dual-path / G22–G25 honesty.
---

# Review CELIR boundary

## Focus

1. Git-compat mode must not silently replace documented object/ref/pack/index meanings.
2. CELIR product Cargo edges are allowed **only** behind non-default feature **`celir-index`** path-deps to `celir-index` / `celir-index-runtime` ([[sot:root/storage-celir-boundary]], [[adr:root/20261110052030]]).
3. Docs/code must not implement ObjectStore via CELIR, enable `celir-index` by default, or claim “CELIR-backed Git” / dual-path verified without G23 clean-room receipt + FS sole ObjectStore wording ([[sot:root/celir-index-dual-path]], [[adr:root/20261109204916]]).
4. Ask on ambiguous cross-cutting PRs (rule `40-ambiguity-ask`).

## Method

Diff → list files that touch both Git-compat and storage paths → **boundary breach: yes/no** (also check `Cargo.toml` / features for `celir`; confirm feature is default OFF and index-only).

Rule: `50-celir-boundary`. Owning SoT: `.spec/sot/storage-celir-boundary.sot.md`. Dual-path: `.spec/sot/celir-index-dual-path.sot.md`. Enablement: `.spec/adr/20261110052030-celir-index-optional-product-path-dep.adr.md`. Also: `.spec/sot/git-compat-surface.sot.md` out-of-scope section.
