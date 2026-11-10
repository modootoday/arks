---
id: sot:root/cli-flag-parity-m9
title: CLI flag-parity audit (M9)
status: active
created: 2026-11-08T14:10:00Z
updated: 2026-11-09T02:30:00Z
tags: [m9, cli, flag-parity, audit, wave-pc, production, residual]
verify: manual
rel:
  depends_on:
    - sot:root/cli-surface
    - sot:root/arks
    - plan:root/20261108183812
    - plan:root/20261109022956
    - sot:root/program-roadmap
    - sot:root/production-residual-register
  governs: [crates/arks]
---

# CLI flag-parity audit (M9 / Wave Pc)

Version **0.2.0**. Recorded **claimed** vs **refuse** flags for the closed inventory (− product exclusions). Unknown `-` options MUST refuse via `refuse_unknown_flags` (no silent ignore). Primary man pages: [git(1)](https://git-scm.com/docs/git) and per-command pages for the clean-room pin (Git 2.39.x).

**Residual amend (post-M9):** `worktree` + `submodule` moved from refuse-closed → implement per [[plan:root/20261109022956]]; M9 exit receipt [[evidence:root/20261108150000]] unchanged as historical bar.

## Rule

1. Every implement-row command below MUST refuse unlisted options with non-zero exit.
2. Refuse-closed commands are omitted from `--help` and always non-zero.
3. Product exclusions (GUI / LFS / scalar / forge B / server M11 / leases M10 / CELIR) are not inventory implement rows.

## Inventory flag ledger (claimed → refuse)

| Command | Claimed flags / forms | Refuse (non-zero) |
|---------|----------------------|-------------------|
| `init` | path optional | bare/template/shared |
| `hash-object` | `-w` | other `-t` |
| `cat-file` | `-t` `-s` `-p` | batch / textconv |
| `update-ref` | `-d` | `--stdin` |
| `rev-parse` | rev names | `--parseopt` extras |
| `add` | paths | `-A`/`-u` |
| `status` | (none) | porcelain v2 flags |
| `commit` | `-m` `--allow-empty` | `--amend` |
| `log` / `show` | (none) | `--graph` etc. |
| `remote` | `add`/`remove`/list | SSH-only |
| `fetch`/`push`/`clone` | smart HTTP claimed | SSH / unclaimed filters |
| `pull` | FF; `--rebase` | non-FF merge pull |
| `branch` | `-d`/`-D`/`-m` | `-a`/`-r` |
| `checkout` | `-b` `--` paths | `--orphan` |
| `switch` | `-c`/`-C` | `--detach` |
| `restore` | `--staged` `--worktree` `--source` | `--overlay` |
| `reset` | `--soft`/`--mixed`/`--hard` | `--keep`/`--merge` |
| `diff` | `--cached`/`--staged` | binary/word-diff |
| `merge` | `--ff-only` | `--no-ff` / strategies |
| `rm`/`mv` | `--cached` | unclaimed force edges |
| `clean` | `-f` `-d` | `-x` |
| `tag` | `-d` `-a` `-m` | `-s`/`-u` |
| `describe` | `--tags` | `--dirty` |
| `shortlog` | (none) | unclaimed grouping |
| `config` | `--get`/`--set`/`--list`/`--unset` | `--global`/`--system` |
| `grep` | pattern | `-P` |
| `stash` | push/list/pop | other subcommands |
| `cherry-pick`/`revert` | single commit | ranges / conflicted |
| `rebase` | non-interactive upstream | `-i` / conflicted |
| `format-patch` | `-1` `A..B` `-o` | `--cover-letter` / binary |
| `am` | one patch file | `--3way` / interactive |
| `bisect` | start/bad/good/reset | run/skip/visualize |
| `bundle` | create/`--all`/list-heads/unbundle/**verify** | thin-pack edges |
| `archive` | `--format=tar` `-o` tree-ish | zip / remote |
| `reflog` | show / default HEAD | expire/delete/exists |
| `worktree` | `add` (`-b`); `list`; `remove` (`-f`); `prune`; `move` | `--porcelain`/`-v`/`-z`; `--detach`/`--orphan`/`--lock`; `lock`/`unlock`; main-worktree remove/move |
| `submodule` | `add`; `init`; `update` (`--init`); `status`; `deinit` (`-f`) | `--recursive`; `--remote`; SSH/`git://` add; custom `!command` update |

## Refuse-closed (omit help)

`repack`, `sparse-checkout`, `maintenance`, `backfill`, `range-diff`, `notes`, `filter-branch`.

(`worktree` / `submodule` were M9 refuse-closed; residual implement [[plan:root/20261109022956]].)

## Honesty residuals (client; post–M9 close)

| Residual | M9 disposition | Notes |
|----------|----------------|-------|
| Pin-advance after Git upgrades | **Closed-as-process** | [[sot:root/clean-room-git-pin-advance]]; ledger + G14 + new evidence required; pin 2.39.5 only for [[evidence:root/20261108150000]] |
| Credential helper / askpass | **Refuse-closed (production)** | No helper protocol in product Rust; smart-HTTP client claims assume non-interactive credential path per plan exclusions |
| Refuse-closed commands (§ above) | **Honest** | Omitted from help; non-zero under alias (G14) |
| `worktree` / `submodule` | **Residual implement** | Claimed matrix in [[plan:root/20261109022956]]; register [[sot:root/production-residual-register]] |
| M9-C19 (lease / CELIR / server / forge) | **Out of scope** | **M10** / **M11** / product exclusions — not client alias inventory |

## Verification

G14 (`verify/matrices/matrix-m009-clean-room-cli-parity.md`) spot-checks help omission, unknown-flag refuse, and this ledger’s presence.
