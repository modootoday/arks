---
name: review-git-compat
description: >
  Use when reviewing changes that claim Git format, protocol, or stock-Git
  interchange compatibility: check primary citations, unknown-extension fail-closed,
  and that performance work did not invent semantics.
---

# Review Git compatibility

## Focus

1. Every new semantic claim cites a catalogued primary source (`git-compat-surface` + research).
2. No invented capabilities, extension keys, or on-disk meanings.
3. Unknown `extensions.*` / unsupported features fail closed where required.
4. Optional caches remain optional (must not change object identity / ref history).
5. Stock-Git interchange intent preserved for the claimed subset.
6. Clean-room still required for product “verified” claims (rule `60`).

## Method

`git status` + `git diff` → severity-ordered findings with paths → **invented semantics: yes/no**.

Owning SoT: `.spec/sot/git-compat-surface.sot.md`. Remotes: `.spec/sot/remote-host-compatibility-surface.sot.md`.
