---
id: evidence:root/20261110210600
title: O3 scripted interactive subset host smoke
status: active
created: 2026-11-10T21:06:00Z
updated: 2026-11-10T21:06:00Z
tags: [o3, interactive, scripted, rebase-i, add-p, editor-commit]
rel:
  depends_on:
    - adr:root/20261110210300
    - research:root/20261110205403
    - sot:root/cli-flag-parity-complete
  evidence: []
files: []
---

# O3 — Scripted interactive subset (host smoke)

## Claimed subset exercised

| Surface | How | Result |
|---------|-----|--------|
| `commit` without `-m` | `ARKS_EDITOR=python edit.py` | **PASS** (editor-msg) |
| `add -p` | `ARKS_ADD_PATCH_ANSWERS=y` file-granularity | **PASS** |
| `rebase -i` | `GIT_SEQUENCE_EDITOR=python seq.py` (pick-only todo) | **PASS** |

## Non-claims

Full curses/TTY TUI, `add -i` menu, `-e` hunk edit, `stash -p`, mergetool GUI.
