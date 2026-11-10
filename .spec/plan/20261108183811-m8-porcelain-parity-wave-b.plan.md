---
id: plan:root/20261108183811
title: M8 Porcelain parity wave B
status: active
created: 2026-11-08T18:38:11Z
updated: 2026-11-08T13:40:00Z
tags: [milestone, m8, phase-pb, cli, porcelain, parity]
rel:
  depends_on:
    - sot:root/program-roadmap
    - plan:root/20261108183810
    - sot:root/cli-surface
    - sot:root/git-compat-surface
    - sot:root/interop-test-matrix
    - sot:root/product-verification-clean-room
    - sot:root/arks
    - sot:root/arks-remote
    - research:root/20261108183800
    - evidence:root/20261108133000
  implements: [sot:root/program-roadmap]
---

# M8 — Porcelain parity wave B (Phase Pb)

## Goal

Deliver [[sot:root/cli-surface]] **Wave Pb**: history-editing and patch workflows toward Git CLI symmetric compatibility. **Not production** (production = **M9**).

## Depends on

**M7** exit proven — [[evidence:root/20261108133000]] (G12 + G6/G7 regression; gix-exit). Implementation tree SHA `7c074ef783c2b66d45d0aa0db83337ee157165a3` (evidence tip `8668ef959d4147bef530902905e80591b30ac856`).

## Frozen Wave Pb checklist (implement / refuse / exclude)

Primary cites: [git(1)](https://git-scm.com/docs/git); per-command man pages for clean-room pin (Git 2.39.x in `verify/images.lock.json`). Classification before G13 claim. Matrix: `verify/matrix-m8.md`.

| Command | Class | Claimed forms (M8) | Refuse / defer |
|---------|-------|--------------------|----------------|
| `rebase` | **implement** | Non-interactive `rebase <upstream>`: replay first-parent linear commits of `HEAD` not in `<upstream>` onto `<upstream>`; clean apply only | Interactive `-i` / `--onto` extras / autosquash; conflicted apply (non-zero, no partial claim) |
| `cherry-pick` | **implement** | Single commit `cherry-pick <commit>`; clean tree apply onto `HEAD` | Commit ranges; `--continue`/`--abort` state machine beyond refuse; conflicted apply |
| `revert` | **implement** | Single commit `revert <commit>`; clean inverse apply onto `HEAD` | Ranges; conflicted apply; `-n`/`--no-commit` → Pc |
| `bisect` | **implement** | `start` / `bad` / `good` / `reset` on first-parent ancestry | `run` / `skip` / `visualize` / `replay` → Pc |
| `format-patch` | **implement** | `-1 [<commit>]` or `<A>..<B>` first-parent series; write numbered `.patch` files to cwd (or `-o <dir>`) | `--cover-letter`; `--stdout` MAY; binary patches refuse |
| `am` | **implement** | Apply one mailbox/`.patch` file produced by claimed `format-patch` | `--3way`; interactive; multi-patch directories beyond single file as claimed in tests |
| `bundle` | **implement** | `create <file> --all` (or listed refs); `list-heads <file>`; `unbundle <file>` | `verify` deepen → Pc; thin pack edges unclaimed |
| `pull` | **deepen** | Retain Pa FF pull; add `pull --rebase` = fetch + claimed `rebase` onto upstream | Non-FF merge pull still refuse; unclaimed pull strategies |
| `range-diff` | **refuse** | — | Deferred to **Pc**; not in `--help` |
| `notes` | **refuse** | — | Deferred to **Pc**; not in `--help` |
| `filter-branch` | **refuse** | — | Deprecated path; refuse; modern replacements → Pc inventory |
| Wave L+H+Pa set | **retain** | Prior G6/G7/G12 inventory remains green | Do not regress G12 / G7 |

**Apply semantics (normative for claimed cherry-pick / revert / rebase / am):** For each path changed between base and tip trees, if `HEAD` blob equals base blob (or path absent on both sides as applicable), replace with tip blob / delete; otherwise **refuse** with non-zero (no silent partial rewrite). No 3-way content merge engine in M8.

**Exclude (product):** GUI tools, LFS, forge APIs (B), server (**M11**), CELIR, L1+ leases (**M10**). Annotated/signed tags remain deferred (**Pc**) unless a later SoT amendment claims them.

## Out of scope

- Final inventory / flag-parity / alias-ready closure / **G14** (M9).
- Calling this milestone production.
- Lease runtime; CELIR; Git server; inventing wire/object semantics; `gix`.
- Interactive rebase TUI; `range-diff` / `notes` (refused to Pc).

## Exit gates

| Gate | Clean-room | Advisory |
|------|------------|----------|
| **G13** cli-porcelain-pb | Authoritative (`verify/g13` + `run-g13.sh`) | Host |
| G12 + G7 (+ G6 / prior) regression | Clean-room (embedded in `run-g13.sh`) | Host |
| Help honesty / refuse | Clean-room G13 | Host |
| Zero `gix` | `run-g13.sh` metadata scan | — |
| Unit Specfold | `spec check` | — |

## Ordered work items

1. **Design** — Freeze Pb checklist (this section); matrix `verify/matrix-m8.md`; unit SoT [[sot:root/arks]] Wave Pb rows; `cli-surface` Pb freeze pointer.
2. **Scaffold** — `crates/arks` `cmd_pb` module; G13 crate + compose service + `run-g13.sh` (LF).
3. **Implement** — cherry-pick/revert → rebase → format-patch/am → bisect → bundle → `pull --rebase`; refuse closed for deferred.
4. **Verify** — G13 + G12/G7 regressions; bug-sim fixes; evidence receipt; **not** production claim.

## Open questions (resolved for M8)

| Topic | Resolution |
|-------|------------|
| Interactive rebase | **Refuse** (`-i`); non-interactive only |
| `range-diff` / `notes` | **Refuse** to Pc (honest help omission) |
| Conflicted history edits | **Refuse** non-zero; no claim of conflict markers |
| `filter-branch` | **Refuse**; replacements owned by Pc inventory |
| Annotated tags | Stay deferred to **Pc** (not required for G13) |

## Cross-links

[[sot:root/program-roadmap]], [[sot:root/cli-surface]], [[sot:root/interop-test-matrix]], [[sot:root/arks]], [[research:root/20261108183800]], `verify/matrix-m8.md`, [[evidence:root/20261108133000]]
