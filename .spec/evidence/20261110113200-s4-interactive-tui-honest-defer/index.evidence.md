---
id: evidence:root/20261110113200
title: S4 interactive TUI honest Specfold defer
status: active
created: 2026-11-10T11:32:00Z
updated: 2026-11-10T11:32:00Z
tags: [s4, interactive, tui, defer, agent-bar, symmetry, honesty]
rel:
  depends_on:
    - adr:root/20261110113200
    - research:root/20261110105436
    - sot:root/cli-flag-parity-complete
    - sot:root/cli-surface
    - evidence:root/20261109200000
  evidence: []
files: []
---

# S4 — Interactive / patch flows: honest defer (not Claimed)

## Disposition

| Surface | Disposition | Why |
|---------|-------------|-----|
| `rebase -i` / `--interactive` | **N/A / refuse** | Agent non-interactive bar; Fs-5 Verified exclude |
| `add -p` / `-i` / `-e` | **N/A / refuse** | Same; explicit S4 refuse message |
| `commit` without `-m` | **N/A / refuse** | Editor/TUI message path; Fs-5 |
| `stash -p`; mergetool/difftool GUI | **N/A** | [[sot:root/cli-flag-parity-complete]] |
| Non-interactive `rebase` / `commit -m` / `add` paths | **Claimed** (unchanged) | Prefer real parity slices already shipped |

Unlock/decision: [[adr:root/20261110113200]]. Ladder: [[research:root/20261110105436]] **S4**.

## Carrying honesty (no invent PASS)

| Check | Outcome |
|-------|---------|
| Prior clean-room G20 Fs-5 interactive exclude | **PASS** — [[evidence:root/20261109200000]] (`M15-FS5-03`) |
| Specfold SoT agent bar | [[sot:root/cli-surface]] §Agent-friendly; [[sot:root/cli-flag-parity-complete]] global N/A row |
| Tip wire refuse strings | `rebase -i` / `commit` without `-m` / `add -p` fail closed with exclude honesty |

**Not** a TUI Claimed/PASS receipt. S4 Done = Specfold-honest defer with SoT reason.
