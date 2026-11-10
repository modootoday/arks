# CELIR / lease boundary

CELIR MAY enter the product Cargo graph **only** as optional **index-only** feature `celir-index` (default OFF). Filesystem `.git` remains the sole Git-compat **object store**. Lease/concurrency is owned by its own SoT. Neither may silently rewrite Git-compat store semantics.

## Must

- Keep Git-compat SoTs and code free of silent alternate-store semantics that replace documented Git object/ref/index/pack meanings in Git-compat mode.
- Do **not** implement ObjectStore `put` / `get` / `exists` (or equivalent) via CELIR.
- Optional path-deps to `celir-index` / `celir-index-runtime` are allowed **only** behind non-default feature `celir-index` on `arks-store` ([[adr:root/20261110052030]]).
- Dual-path companion ship is **M24** ([[sot:root/celir-index-dual-path]]); contention/dual-path/joint/scale gates are **G22–G25**.
- “CELIR dual-path Git verified” requires G23 receipt and MUST keep FS `.git` as sole ObjectStore wording.
- Point lease work at `.spec/sot/lease-concurrency-intent.sot.md` on the filesystem store; L3 Claimed shape is **M23** (shared-index N-way OUT).
- When a change touches both Git interchange and storage/lease ideas, split work or ask (rule `40-ambiguity-ask`).

## Cite

- Owning boundary SoTs: `.spec/sot/storage-celir-boundary.sot.md`, `.spec/sot/lease-concurrency-intent.sot.md`, `.spec/sot/celir-index-dual-path.sot.md`.
- Enablement: `.spec/adr/20261110052030-celir-index-optional-product-path-dep.adr.md`.
- Out-of-scope notes: `.spec/sot/git-compat-surface.sot.md` (Explicitly out of scope).
- Review skills: `review-celir-boundary`, `review-lease`.
- Humanymous structural bottom-up (general pattern only): `C:\workspaces\humanymous\.spec\sot\structural-development.sot.md`.
