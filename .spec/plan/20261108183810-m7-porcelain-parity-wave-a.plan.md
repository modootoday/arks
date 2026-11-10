---
id: plan:root/20261108183810
title: M7 Porcelain parity wave A
status: active
created: 2026-11-08T18:38:10Z
updated: 2026-11-08T13:15:00Z
tags: [milestone, m7, phase-pa, cli, porcelain, parity]
rel:
  depends_on:
    - sot:root/program-roadmap
    - plan:root/20261108090651
    - sot:root/cli-surface
    - sot:root/git-compat-surface
    - sot:root/interop-test-matrix
    - sot:root/product-verification-clean-room
    - sot:root/arks
    - sot:root/arks-remote
    - research:root/20261108183800
    - evidence:root/20261108112000
  implements: [sot:root/program-roadmap]
---

# M7 — Porcelain parity wave A (Phase Pa)

## Goal

Deliver [[sot:root/cli-surface]] **Wave Pa**: main local branching / worktree / diff / merge foundations and related commands toward Git CLI symmetric compatibility. **Not production** (production = M9).

## Depends on

**M6** exit proven — [[evidence:root/20261108112000]] (G7 + G1–G6 regression; gix-exit).

## Frozen Wave Pa checklist (implement / refuse / exclude)

Primary cites: [git(1)](https://git-scm.com/docs/git); per-command man pages for clean-room pin (Git 2.39.x in `verify/images.lock.json`). Classification before G12 claim. Matrix: `verify/matrices/matrix-m7.md`.

| Command | Class | Claimed forms (M7) | Refuse / defer |
|---------|-------|--------------------|----------------|
| `branch` | **implement** | List local; create `branch <name>`; delete `-d`/`-D`; rename `-m` | `-a`/`-r` tracking lists; `--set-upstream-to` extras |
| `checkout` | **implement** | Switch branch; `-b <name>`; path restore from HEAD (`checkout -- <path>`) | `--orphan`; detached OID as primary UX (MAY refuse); unclaimed long options |
| `switch` | **implement** | Switch branch; `-c`/`-C` create | `--detach`; unclaimed |
| `restore` | **implement** | Paths to worktree and/or `--staged` from `--source` HEAD/index default | `--overlay`; unclaimed |
| `reset` | **implement** | `--soft` / `--mixed` (default) / `--hard` to commit/HEAD | `--keep` / `--merge`; pathspec reset beyond claimed |
| `diff` | **implement** | No-arg WT vs index; `--cached`/`--staged` index vs HEAD; `A B` commit/tree pair unified text for blobs | Binary/word-diff refuse; fancy `--stat` MAY |
| `merge` | **implement** | Fast-forward when possible; `--ff-only` | Non-FF conflict resolution; `--no-ff`; octopus; other strategies |
| `rm` | **implement** | Remove paths from index + WT; `--cached` | Directory recursion beyond claimed |
| `mv` | **implement** | Rename/move in WT + index | Unclaimed force overwrite edges |
| `clean` | **implement** | `-f` remove untracked files | `-x` ignored; `-d` directories refuse (Pc) |
| `tag` | **implement** | Lightweight create/list; delete `-d` | Annotated `-a`/`-s`/`-u` → Pb/Pc |
| `describe` | **implement** | Nearest lightweight tag ancestry (`describe` / `--tags`) | `--dirty` |
| `shortlog` | **implement** | Author summary over first-parent from HEAD | Unclaimed grouping extras |
| `config` | **implement** | Local repo: `--get` / `--set` / `--list` / `--unset` simple keys | `--global`/`--system` |
| `grep` | **implement** | Fixed-string/basic pattern over tracked worktree paths | `-P` PCRE |
| `stash` | **implement** | `push` (default), `list`, `pop` minimal WIP | Other stash subcommands refuse |
| `worktree` | **refuse** | — | Deferred to **Pc**; not in `--help` |
| `reflog` | **refuse** | — | Deferred to **Pc**; not in `--help` |
| `repack` | **refuse** | — | Not required for Pa; not in `--help` |
| `pull` | **implement** | `fetch` + `merge --ff-only` of tracked upstream when configured | Non-FF; rebase pull → **Pb** |
| `status` / `log` / `show` | **deepen** | Keep L claims; deepen only flags Pa tests need | Unclaimed decorate/graph |
| Wave L+H set | **retain** | Prior G6/G7 inventory remains green | Do not regress G7 |

**Exclude (product):** GUI tools, LFS, forge APIs (B), server (**M11**), CELIR, L1+ leases (**M10**).

## Out of scope

- Wave Pb history-editing set (M8).
- Inventory / flag-parity **closure** (M9).
- Lease / CELIR / forge / GUI / LFS.
- Calling this milestone production.
- Lease runtime; Git server; inventing wire/object semantics; `gix`.

## Exit gates

| Gate | Clean-room | Advisory |
|------|------------|----------|
| **G12** cli-porcelain-pa | Authoritative (`verify/gates/g12` + `run-g12.sh`) | Host |
| G6 + G7 regression | Clean-room (embedded in `run-g12.sh`) | Host |
| Help honesty / refuse | Clean-room G12 | Host |
| Zero `gix` | `run-g12.sh` metadata scan | — |
| Unit Specfold | `spec check` | — |

## Ordered work items

1. **Design** — Freeze Pa checklist (this section); matrix `verify/matrices/matrix-m7.md`; unit SoT [[sot:root/arks]] Wave Pa rows.
2. **Scaffold** — CLI modules under `crates/arks`; G12 crate + compose service.
3. **Implement** — branching → checkout/reset → diff → merge/pull → tree edits → names/config/grep/stash; refuse closed for deferred.
4. **Verify** — G12 + G6/G7 regressions; bug-sim fixes; evidence receipt; **not** production claim.

## Open questions (resolved for M7)

| Topic | Resolution |
|-------|------------|
| `worktree` / `reflog` / `repack` in Pa | **Refuse** to Pc (honest help omission) |
| `stash` depth | Minimal push/list/pop **in** Pa |
| Non-FF `merge` | **Refuse** until Pb/Pc; FF + `--ff-only` only |
| Annotated tags | **Refuse** (Pb/Pc) |

## Cross-links

[[sot:root/program-roadmap]], [[sot:root/cli-surface]], [[sot:root/interop-test-matrix]], [[sot:root/arks]], [[research:root/20261108183800]], `verify/matrices/matrix-m7.md`
