---
id: plan:root/20261109022956
title: Post-M9 worktree and submodule residual implement
status: active
created: 2026-11-09T02:29:56Z
updated: 2026-11-09T02:29:56Z
tags: [residual, worktree, submodule, cli, post-m9, clean-room, g16]
rel:
  depends_on:
    - sot:root/cli-surface
    - sot:root/cli-flag-parity-m9
    - sot:root/production-residual-register
    - sot:root/clean-room-journey-compare
    - sot:root/interop-test-matrix
    - sot:root/arks-repo
    - sot:root/arks
    - plan:root/20261108183812
    - sot:root/git-compat-surface
    - sot:root/product-verification-clean-room
  implements:
    - sot:root/production-residual-register
    - sot:root/cli-surface
---

# Post-M9 residual — `worktree` + `submodule` implement

## Goal

Move **`worktree`** and **`submodule`** from M9 Wave Pc **refuse-closed** to **claimed implement** inventory without reopening or widening the M9/M10/M11 production **exit bars**. Historical M9 receipt [[evidence:root/20261108150000]] stays valid for the then-frozen refuse ledger; this plan is a **residual inventory amendment** tracked in [[sot:root/production-residual-register]].

## Primary cites (summarize only)

- [git-worktree(1)](https://git-scm.com/docs/git-worktree)
- [gitrepository-layout](https://git-scm.com/docs/gitrepository-layout) (`worktrees/`, `commondir`, `modules/`)
- [git-submodule(1)](https://git-scm.com/docs/git-submodule)
- [gitmodules(5)](https://git-scm.com/docs/gitmodules)
- [gitsubmodules(7)](https://git-scm.com/docs/gitsubmodules) (gitlink mode `160000`)

## Claimed subcommand matrix (pin Git 2.39.x)

### `worktree`

| Subcommand | Claimed forms | Refuse / notes |
|------------|---------------|----------------|
| `add` | `worktree add [-b <new-branch>] <path> [<commit-ish>]` — linked worktree under `$GIT_COMMON_DIR/worktrees/<id>`; checkout into `<path>`; same object store as main | `--detach`, `--orphan`, `--lock`, `-f`/`--force`, `--checkout=false`, guess-remote |
| `list` | `worktree list` human lines (path + short OID + branch/detached) | `--porcelain`, `-v`, `-z` |
| `remove` | `worktree remove [-f] <worktree>` — clean linked only; `-f` allows dirty | Main worktree; locked worktree without unlock |
| `prune` | `worktree prune` — drop stale `$GIT_COMMON_DIR/worktrees/<id>` when `gitdir` target missing | `--expire` custom windows; dry-run `-n` optional refuse |
| `move` | `worktree move <worktree> <new-path>` — rewrite `gitdir` + worktree `.git` file | Main worktree; locked; path with submodules (out) |

**Semantics:** linked worktrees share `$GIT_COMMON_DIR` objects/refs; per-worktree `HEAD`/`index`/`commondir`/`gitdir` under `worktrees/<id>`. Branch already checked out in another worktree → **fail-closed** on `add`. Presence of `worktrees/<id>/locked` → **fail-closed** on `remove`/`move`. `lock`/`unlock` porcelain remain **refuse** (locked file still honored).

### `submodule`

| Subcommand | Claimed forms | Refuse / notes |
|------------|---------------|----------------|
| `add` | `submodule add [--] <repository> [<path>]` — local path / same-host `file://` clone into `<path>`; write `.gitmodules`; stage gitlink `160000` + `.gitmodules` | SSH/`git://`/smart-HTTP remote add; `--depth`; `--reference`; `--name` extras; recursive nest absorb |
| `init` | `submodule init [--] [<path>...]` — copy `submodule.<name>.url` from `.gitmodules` into `$GIT_DIR/config` | Custom `!command` update forms |
| `update` | `submodule update [--init] [--] [<path>...]` — clone if needed (local/file URL) into `.git/modules/<name>` + checkout recorded gitlink commit | `--remote`; rebase/merge update strategies; `--recursive` nested |
| `status` | `submodule status [--] [<path>...]` — print recorded/checkout SHA with `-`/`+` prefixes per man page | `--recursive`; `--cached` optional deepen later |
| `deinit` | `submodule deinit [-f|--force] [--] <path>...` — empty worktree path; unset `submodule.<name>.*` in config | `--all` MAY deepen later |

**Object model:** superproject records gitlink tree/index mode **`160000`** + `.gitmodules` (`submodule.<name>.path` / `.url`). Nested submodule paths (single gitlink under a nested directory) are **in scope**; **nested submodule recursion** (`--recursive`) is **out** / refuse. Checkout of trees containing gitlinks MUST stage the gitlink and leave an empty directory (no recurse) — not refuse the whole checkout.

## Engine / CLI placement (bottom-up)

1. **`arks-repo`** — resolve `commondir` / `$GIT_COMMON_DIR`; expose `common_dir` + work-tree path; open objects from common dir; dual-root refs (per-worktree `HEAD` vs shared `refs/`).
2. **`arks-refs`** — optional common-dir base for shared refs (no CELIR; L0 locks unchanged).
3. **`arks-remote` checkout** — skip-populate gitlinks (index `160000`) instead of hard refuse.
4. **`arks` CLI** — `cmd_worktree` / `cmd_submodule`; help lists both; remaining Pc refuse list drops these two only.
5. **Zero `gix`**; CELIR out of product deps; lease/default lock rules unchanged.

## Verify rows (replace G16 refuse probes)

| ID | Replaces | Functional class |
|----|----------|------------------|
| **J-WORKTREE-01** | J-REFUSE-01 | `worktree add` → `list` → `move` → `remove`/`prune`; same-`.git`/commondir; locked remove fails closed; alias lane PASS |
| **J-SUBMODULE-01** | J-REFUSE-03 | local `submodule add` → `status` → `deinit` → `init` → `update`; gitlink + `.gitmodules`; alias PASS |
| **J-REFUSE-02** | (kept) | `repack -ad` remains refuse-closed |

G14 residual re-smoke: help lists `worktree`/`submodule`; unknown flags refuse; alias shim exercises claimed subcommands (not refuse). Matrix: `verify/matrices/matrix-journey.md` + `verify/matrices/matrix-m9.md` honesty note that M9 historical refuse is superseded by this residual for these two commands only.

## Non-goals

- Widening M9/M10/M11 **exit** definitions or inventing a fourth production bar.
- Absolute-ms / ≥20% performance bars.
- `repack` / `sparse-checkout` / `maintenance` / other Pc refuse rows.
- Nested submodule recursion; SSH submodule URLs; worktree `--porcelain`.

## Exit checklist

- [x] SoT / flag ledger / residual register / journey catalog updated; `spec check` clean
- [x] Native engine + CLI claimed matrix green on host unit/integration (advisory)
- [x] Docker clean-room G16 (successor rows) + G14 residual honesty green; evidence receipt [[evidence:root/20261109114207]]
- [x] Public/docs honesty updated; tip-pin evidence SHA to land commit
- [x] Push SoT/plan/code/evidence
