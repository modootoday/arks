# English only (top-level) — Hangul ban in tracked docs

All authored project text is **English only**. Chat/coordinator briefings in other languages do not waive this for Specfold or `docs/`. Owning SoT: `.spec/sot/language-english-only.sot.md`.

## Applies to

- `.spec/**` authored prose (SoTs, plans, research narrative, evidence summaries)
- `AGENTS.md`, `.agents/**`
- `docs/` and public Pages copy
- Code comments / docstrings
- Commit messages and PR text

## Hangul / non-English ban (MUST)

- **Never** commit Hangul (or other non-English authored prose) into tracked Specfold, `docs/`, `AGENTS.md`, `.agents/**`, README/CONTRIBUTING, or comments.
- Operator Korean briefings stay in **chat only** (or gitignored `.spec.local/` scratch if needed) — convert before any tracked commit.
- History rewrites that remove Hangul from Spec/docs are intentional; do not reintroduce Hangul “for the operator.”

## Allowed exceptions

- Proper nouns / product ids: `arks`, `CELIR`, `humanymous`, locked crate/bin names
- Immutable evidence, third-party quotations, intentional multilingual test fixtures
- Unicode symbols (not a language waiver)

## Cite (do not fork)

- `C:\workspaces\humanymous\.agents\rules\10-rust-conventions.md`
- `C:\workspaces\humanymous\.spec\sot\rust-migration.sot.md` (English authored documentation / comments)

If requirements seem to need non-English authored prose in-tree: **stop and ask** (rule `40-ambiguity-ask`).
