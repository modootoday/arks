---
name: commit-governance
description: >
  Use when preparing git commits for arks Specfold or agent governance: Spec-first
  order, one concern per commit, Version≠release, English-only messages, no push,
  and leave sibling-owned dirty paths unstaged.
---

# Commit governance

## Preconditions

- Rules `05`, `26`, `27`, `28`, `29`, `35`, `91`, `90`.
- Read `.spec/README.md` commit-order + Version sections.
- Optional machine journey (gitignored): `.spec.local/sot/post-spec-canon-commit-order.sot.md` (rule `35` — guidance only).

## Steps

1. `git status` / `git diff` — identify owned paths vs sibling dirt.
2. If tracked `.spec/sot/**` meaning changed and code also changed → **two commits** (Spec first).
3. Stage **only** owned paths (`git add <paths>` — never blind `git add -A` with foreign dirt; never stage `.spec.local/` / `.env`).
4. English subject; no Claimed/PASS/Verified invention (rules `28`/`29`); no Hangul (hooks scan `git commit`).
5. Do **not** push / force-push unless the user explicitly authorizes (hooks deny force to main/master).
6. After commit: report tip SHA(s); confirm sibling dirty paths remain unstaged.

## Gotchas

- SoT body `Version` bump ≠ product release (rule `26`).
- `.spec.local/**` must stay untracked.
- Governance lane should not rewrite `crates/**` owned by a sibling CLI agent.
