---
id: adr:root/20261110210300
title: O3 reopen — interactive scripted / non-TTY subset
status: active
created: 2026-11-10T21:03:00Z
updated: 2026-11-10T21:03:00Z
tags: [o3, s4, interactive, scripted, rebase-i, add-p, editor-commit]
rel:
  depends_on:
    - research:root/20261110205403
    - adr:root/20261110113200
    - sot:root/cli-surface
    - sot:root/cli-flag-parity-complete
    - sot:root/arks
    - sot:root/production-residual-register
  supersedes: []
  evidence: [evidence:root/20261110210600]
---

# O3 — Reopen interactive flows as scripted / non-TTY Claimed subset

## Context

[[adr:root/20261110113200]] deferred full interactive TUI under the agent non-interactive bar. Optional ladder [[research:root/20261110205403]] **O3** reopens the maximum Specfold-honest product bar: prefer working **non-TTY scripted modes** + documented TTY behavior over fake Claimed pixel/TUI.

## Decision

1. **Claim scripted `rebase -i` / `--interactive`:** generate a default `git-rebase-todo` (`pick` lines), invoke `GIT_SEQUENCE_EDITOR` / `ARKS_SEQUENCE_EDITOR` (required), parse claimed verbs `pick` / `drop` (or `#` commented) / `reword` (message via `GIT_EDITOR`/`ARKS_EDITOR`), then replay. Reorder allowed. Refuse unknown todo verbs fail-closed.
2. **Claim editor-required `commit` without `-m`:** when `-F <file>` / `-t <file>` present, or `GIT_EDITOR` / `ARKS_EDITOR` / `EDITOR` is set, write `.git/COMMIT_EDITMSG` and spawn the editor (non-interactive scripted editors OK). Without `-m`/`-F`/`-t` and without an editor env, refuse with honesty (no silent empty commit).
3. **Claim scripted `add -p` / `--patch`:** hunk apply driven by answers from stdin when not a TTY, or from `ARKS_ADD_PATCH_ANSWERS` (sequence of `y`/`n`/`a`/`d`/`q`). Full curses/`-i` menu TUI remains **N/A**. `-e` hunk edit remains refuse until claimed.
4. **TTY documentation:** when stdin is a TTY and no scripted answers/editor/sequence-editor is configured, refuse with a message pointing at the scripted env vars — do **not** invent a pixel TUI Claimed.
5. **Retain N/A:** mergetool/difftool GUI, `stash -p`, full interactive `add -i` menu.

## Consequences

- Amend [[sot:root/cli-flag-parity-complete]] / [[sot:root/cli-surface]] / [[sot:root/arks]] for the Claimed scripted subset; supersede S4 full-defer only for these rows.
- Evidence + e2e (host and/or clean-room) before public Claimed flip.
- Full curses TUI stays N/A (honest).
