---
id: adr:root/20261110113200
title: S4 interactive / patch flows — honest Specfold defer
status: active
created: 2026-11-10T11:32:00Z
updated: 2026-11-10T11:32:00Z
tags: [s4, interactive, tui, defer, agent-bar, symmetry]
rel:
  depends_on:
    - research:root/20261110105436
    - sot:root/cli-flag-parity-complete
    - sot:root/cli-surface
    - sot:root/arks
    - sot:root/production-residual-register
    - evidence:root/20261109200000
  supersedes: []
  evidence: [evidence:root/20261110113200]
---

# S4 — Interactive / patch flows: honest Specfold defer

## Context

Symmetry ladder [[research:root/20261110105436]] **S4** asks whether `rebase -i`, `add -p`, and editor-required `commit` (no `-m`) can be Claimed. Specfold already freezes an **agent non-interactive** bar:

- [[sot:root/cli-surface]] — “Stable non-interactive defaults; no TUI required for production CLI”; interactive rebase refused to help omit.
- [[sot:root/cli-flag-parity-complete]] — interactive editor/TUI flows are **global N/A** (`commit` without `-m`; `rebase -i`; `add -i`/`-p`; `stash -p`; mergetool/difftool GUI).
- [[sot:root/arks]] — `rebase` claimed non-interactive only; `-i` out.
- Clean-room [[evidence:root/20261109200000]] (M15 Fs-5 / G20) already **Verified** refuse honesty for editor commit + `rebase -i`.

Implementing a pixel/TUI clone or inventing “Claimed interactive PASS” would violate Specfold honesty and the product agent bar.

## Decision

1. **Defer (N/A retain)** for full interactive TUI parity: `rebase -i` / `--interactive`, `add -p`/`-i`/`-e`, `stash -p`, mergetool/difftool GUI, editor-required `commit` without `-m`.
2. **Keep Claimed non-interactive slices** unchanged: `rebase <upstream>`, `commit -m`, `add` path/`-A`/`-u`, `format-patch` / `am`, sequencer `--continue`/`--abort` on clean conflict markers where already Claimed.
3. **Refuse messages** MUST stay explicit (`excluded` / `interactive` / `S4` / `Fs-5`) so alias/`git` drop-in fails closed without silent no-op.
4. **Do not** invent Claimed/PASS for TUI. Reopen only via SoT amend that lifts the agent non-interactive bar (owner/product decision) plus wire + evidence.

## Consequences

- Symmetry **S4 Done** as **honest defer** (disposition receipt), not Claimed TUI.
- Public `docs/git-vs-arks.md` / `limits.md` keep Interactive TUI as **N/A**.
- Next **S5** (R5 live forge secrets or documented block).
- **L9** stays closed.
