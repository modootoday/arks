---
title: Workflows
description: Everyday Git-compatible workflows with the native arks CLI
sections:
  - daily
  - branches
  - remotes
  - signing
  - {c: cta, name: next}
---

# Workflows

These are common day-to-day flows with the **native** `arks` binary — the same shapes multi-session environments already expect from Git. For the full command list, run `arks --help`. For contention and leases, see [Concurrent sessions](../concurrent-sessions/).

## Daily local work

| Goal | Commands |
|------|----------|
| Create a repo | `arks init` |
| Stage and commit | `arks add`, `arks commit -m "…"` |
| Inspect | `arks status`, `arks diff`, `arks show`, `arks log` |
| Undo / move tips | `arks reset` (`--soft` / `--mixed` / `--hard`) |
| Paths | `arks rm`, `arks mv`, `arks restore` |
| Stash | `arks stash push`, `arks stash list`, `arks stash pop` |

## Branches and history

| Goal | Commands |
|------|----------|
| Branches | `arks branch`, `arks checkout` / `arks switch` |
| Fast-forward merge | `arks merge --ff-only <topic>` |
| Rebase (non-interactive) | `arks rebase <onto>` |
| Cherry-pick / revert | `arks cherry-pick`, `arks revert` |
| Tags | `arks tag -a -m "…"`, `arks tag` |
| Reflog | `arks reflog` |

## Remotes and packaging

| Goal | Commands |
|------|----------|
| Clone / fetch / pull / push | `arks clone`, `arks fetch`, `arks pull`, `arks push` |
| Remotes | `arks remote` |
| Bundle | `arks bundle create`, `arks bundle verify` |
| Archive | `arks archive --format=tar -o … HEAD` |
| Repack | `arks repack -ad` |
| Worktree | `arks worktree add` / `list` / `move` / `remove` |
| Submodule (claimed forms) | `arks submodule add` / `status` / `deinit` / `init` / `update` |

## SSH-signed commits

With `gpg.format=ssh` configured:

```bash
arks commit -S -m "Signed change"
arks verify-commit HEAD
```

OpenPGP / X.509 / `tag -s` are not claimed — see [Limits](../limits/).

## Next

[Getting started](../getting-started/) [Alias `git` → `arks`](../alias-git-arks/) [Concurrent sessions](../concurrent-sessions/)
