---
title: Workflows
description: Everyday Git-compatible workflows with the native arks CLI
nav_order: 50
sections:
  - {name: daily, filter: true}
  - branches
  - remotes
  - signing
  - {c: cta, name: next}
---

# Workflows

These are common day-to-day flows with the **native** `arks` binary — the same shapes multi-session environments already expect from Git. For the full command list, run `arks --help`. For contention and leases, see [Concurrent sessions](../concurrent-sessions/).

> [!TIP]
> Filter the daily table below, or open site search with `kbd:Ctrl+K`.

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

## Signed commits and tags

With `gpg.format=ssh` configured:

```bash
arks commit -S -m "Signed change"
arks verify-commit HEAD
```

With `gpg.format=openpgp` (or `x509`) and a local GnuPG/`gpgsm` program:

```bash
arks tag -s v1.0 -m "Signed tag"
arks verify-tag v1.0
```

> [!NOTE]
> OpenPGP / X.509 commit signing and `tag -s` / `verify-tag` are claimed via spawned `gpg` / `gpgsm` (clean-room **G38**). Forge signature UX is not claimed — see [Limits](../limits/).

## Next

[Getting started](../getting-started/) [Alias `git` → `arks`](../alias-git-arks/) [Concurrent sessions](../concurrent-sessions/)
