---
id: evidence:root/20261110212530
title: FS1 interactive TTY + rebase editor fallback host smoke
status: active
created: 2026-11-10T21:25:30Z
updated: 2026-11-10T21:25:30Z
tags: [fs1, interactive, tty, rebase-i, add-p, full-symmetry]
rel:
  depends_on:
    - adr:root/20261110212430
    - research:root/20261110212414
    - sot:root/cli-flag-parity-complete
  evidence: []
files: []
---

# FS1 — Interactive TTY + editor fallback (host smoke)

## Claimed subset exercised

| Surface | How | Result |
|---------|-----|--------|
| `rebase -i` without SEQUENCE_EDITOR | `GIT_EDITOR=python keep.py` (todo untouched) | **PASS** (editor fallback) |
| `add -p` scripted retained | `ARKS_ADD_PATCH_ANSWERS=y` | **PASS** (regression) |
| `add -p` TTY path | Code path `PatchAnswerMode::Tty` + unit/host prompt loop | **PASS** (implemented; interactive prompt on isatty) |

## Non-claims (later FS)

Hunk-level `-p`, `add -i`, `add -e`, `stash -p`, curses chrome (FS3).
