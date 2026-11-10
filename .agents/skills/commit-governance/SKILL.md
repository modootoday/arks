---
name: commit-governance
description: >
  Use when preparing git commits for arks Specfold or agent governance: Spec-first
  order, one concern per commit, Version≠release, English-only messages, no push,
  and leave sibling-owned dirty paths unstaged.
---

# Commit governance

## Preconditions

- Rules `05`, `26`, `27`, `28`, `91`, `90`.
- Read `.spec/README.md` commit-order + Version sections.
- Optional machine journey (gitignored): `.spec.local/sot/post-spec-canon-commit-order.sot.md`.

## Steps

1. `git status` / `git diff` — identify owned paths vs sibling dirt.
2. If tracked `.spec/sot/**` meaning changed and code also changed → **two commits** (Spec first).
3. Stage **only** owned paths (`git add <paths>` — never blind `git add -A` with foreign dirt).
4. English subject; no Claimed/PASS invention; no Hangul.
5. Do **not** push unless the user explicitly authorizes.
6. After commit: report tip SHA(s); confirm sibling dirty paths remain unstaged.

## Gotchas

- SoT body `Version` bump ≠ product release (rule `26`).
- `.spec.local/**` must stay untracked.
- Governance lane should not rewrite `crates/**` owned by a sibling CLI agent.
