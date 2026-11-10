# `.spec.local` journey awareness

`.spec.local/` is **machine-private** Specfold local state (`spec.toml` `[local] path`). It may hold commit-order journey notes, pin extracts, and agent scratch. It is **never** public Claimed canon.

Owning classification: `.spec/README.md`, rule `25-spec-classification`, `.spec/sot/agent-governance.sot.md`.

## Must

1. Read local journey guidance when present (example path operators may keep): `.spec.local/sot/post-spec-canon-commit-order.sot.md` — treat as **machine guidance**, not tracked SoT.
2. Keep Spec-first public order (rule `27`): tracked `.spec/sot/**` meaning commits before related code.
3. Leave `.spec.local/**` untracked; never `git add` it (hook deny + rule `91`).
4. When promoting a local note into public `.spec/`, rewrite as English research/SoT with owner approval — do not copy secrets or Hangul operator dumps.

## Must not

- Cite `.spec.local/**` paths as product Claimed / Verified evidence in public SoTs.
- Commit journey SoTs, pin extracts, or agent transcripts from `.spec.local/`.
- Skip Spec-first because a local journey file “already ordered” uncommitted meaning drift.

## Cite

- skill `commit-governance`
- skill `research-to-sot`
- rule `80-public-surface`
