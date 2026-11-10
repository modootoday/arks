# Commit hygiene & multitask coordination

Keep history stepwise and avoid clobbering sibling agents.

## Must

1. **One concern per commit** — Spec meaning, one stub surface, one harness, or one docs slice — not a mega-mix.
2. **Stage only owned paths** — leave unrelated dirty trees unstaged (especially sibling CLI/crate WIP).
3. **Spec-first** when public SoT meaning changes (rule `27`).
4. **English commit subjects/bodies** (rule `05`).
5. **No push** unless the owner explicitly authorizes (push-deferred / empty remote policy).
6. Before multi-file edits, check sibling ownership (rule `90-session-overlap`).

## Must not

- `git add -A` / `git add .` when the working tree has sibling-owned dirty crates.
- Force-push, mass-rebase, or amend foreign commits unless the user explicitly orders it.
- Invent Claimed/PASS in commit messages (rule `28`).
- Commit `.spec.local/**` or secrets.
- Clobber sibling factory/serve/hub crate work from a governance lane — prefer `.agents/`, `AGENTS.md`, `.spec` governance SoT, hooks/skills (tiny cross-links OK).

## Cite

- `.spec/README.md` §Commit order
- Local machine journey (gitignored): `.spec.local/sot/post-spec-canon-commit-order.sot.md`
- skill `commit-governance`
- humanymous cite (do not fork ledger): `C:\workspaces\humanymous\.agents\rules\91-git-contention.md`
