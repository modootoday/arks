# Object store / CELIR boundary

**CELIR is not adopted** (`.spec/adr/20261119200000-celir-not-adopted-withdraw-m24.adr.md`). Filesystem `.git` is the sole Git-compat **object store**. Lease/concurrency is owned by its own SoT. Neither may silently rewrite Git-compat store semantics.

## Must

- Keep Git-compat SoTs and code free of silent alternate-store semantics that replace documented Git object/ref/index/pack meanings in Git-compat mode.
- Keep CELIR (`celir-index`, `celir-index-runtime`, `celir-data`, any `discrete-circuit` package) **out of the Cargo graph entirely** — no feature, no path dependency, no companion crate.
- Treat **M24** and gates **G22–G25** as **Withdrawn**; their evidence is history only.
- Point lease work at `.spec/sot/lease-concurrency-intent.sot.md` on the filesystem store; L3 Claimed shape is **M23**; shared-index N-way is **M26 / G34**.
- When a change touches both Git interchange and storage/lease ideas, split work or ask (rule `40-ambiguity-ask`).

## Cite

- Decision: `.spec/adr/20261119200000-celir-not-adopted-withdraw-m24.adr.md` (supersedes `20261110052030`).
- Boundary SoT: `.spec/sot/storage-celir-boundary.sot.md`; lease: `.spec/sot/lease-concurrency-intent.sot.md`.
- Review skills: `review-celir-boundary`, `review-lease`.
