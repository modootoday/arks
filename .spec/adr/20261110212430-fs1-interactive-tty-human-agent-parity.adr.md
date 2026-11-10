---
id: adr:root/20261110212430
title: FS1 — Interactive TTY + editor fallback (human+agent git parity)
status: accepted
created: 2026-11-10T21:24:30Z
updated: 2026-11-10T21:24:30Z
tags: [fs1, interactive, tty, rebase-i, add-p, human-agent, full-symmetry]
rel:
  depends_on:
    - research:root/20261110212414
    - adr:root/20261110210300
    - adr:root/20261110113200
    - sot:root/cli-surface
    - sot:root/cli-flag-parity-complete
    - sot:root/product-intent
---

# FS1 — Interactive TTY + editor fallback

## Context

O3 ([[adr:root/20261110210300]]) Claimed scripted/non-TTY interactive subset and retained TTY/`pixel TUI` as N/A under an **agent non-interactive** convenience bar. Owner DoD correction ([[research:root/20261110212414]]) requires **human+agent git feature symmetry**: casual N/A for real interactive git flows is overruled.

Stock git:

- `rebase -i` opens the todo via `GIT_SEQUENCE_EDITOR`, falling back to the ordinary editor (`GIT_EDITOR` / `core.editor` / `EDITOR`) — **not** a separate curses menu for the default path.
- `add -p` prompts on a TTY for each change (`y`/`n`/`a`/`d`/`q`/…).

## Decision

1. **Reopen** interactive TTY/editor paths as **Claimed** under FS1 (supersedes O3 retain of “TTY UI not claimed” for these two surfaces).
2. **`rebase -i` editor resolution (git parity):**
   - Prefer `ARKS_SEQUENCE_EDITOR` / `GIT_SEQUENCE_EDITOR`.
   - Else fall back to `ARKS_EDITOR` / `GIT_EDITOR` / `EDITOR`.
   - If none set, refuse honestly (same class as stock git without an editor).
3. **`add -p` TTY prompt (file-granularity Claimed):**
   - When stdin is a TTY and `ARKS_ADD_PATCH_ANSWERS` is unset: print a per-path prompt and read answers interactively.
   - Retain `ARKS_ADD_PATCH_ANSWERS` and non-TTY stdin scripted paths.
   - Hunk-level `-p`, `add -i`, `add -e`, `stash -p` remain for **FS3** (not Claimed here).
4. **Non-claim this ADR:** inventing a curses/pixel chrome clone; mergetool GUI identity; proc-receive (**FS2**).

## Consequences

- Public comparison / limits flip TTY interactive rows from N/A → Claimed (FS1) with evidence.
- Product-intent “no TUI required for production CLI” remains true as *agent-friendly defaults exist*; it MUST NOT be read as forbidding TTY prompts for human parity.
- O3 evidence remains valid for scripted paths; FS1 adds TTY/editor-fallback evidence.
