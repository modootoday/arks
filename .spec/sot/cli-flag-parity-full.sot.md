---
id: sot:root/cli-flag-parity-full
title: CLI flag-parity audit (full symmetric)
status: active
created: 2026-11-09T03:54:00Z
updated: 2026-11-09T04:45:00Z
tags: [m15, cli, flag-parity, audit, wave-fs, full-symmetric, alias, historical]
verify: manual
rel:
  depends_on:
    - sot:root/cli-surface
    - sot:root/cli-flag-parity-m9
    - sot:root/arks
    - sot:root/program-roadmap
    - sot:root/production-residual-register
    - sot:root/interop-test-matrix
    - plan:root/20261109035335
    - plan:root/20261108183812
    - sot:root/cli-flag-parity-complete
  governs: [crates/arks]
  evidence: [evidence:root/20261109203000]
---

# CLI flag-parity audit (full symmetric / M15) — historical

Version **0.2.0**. **Historical** M15 inventory-close ledger against the clean-room pin ([git(1) 2.39.0](https://git-scm.com/docs/git/2.39.0)). Carrying exit: [[evidence:root/20261109203000]]. **Active** alias honesty after M15 is [[sot:root/cli-flag-parity-complete]] (**M16** — no production refuse leftovers). Historical M9 ledger [[sot:root/cli-flag-parity-m9]] remains the frozen M9 close.

Unknown `-` options MUST refuse via `refuse_unknown_flags` (no silent ignore). Primary: per-command man pages for the pin.

## Rule

1. Product exclusions (GUI / LFS / scalar / forge B / CELIR admin / lease admin valves) are **not** implement rows.
2. Every command in the active inventory MUST be **implement** or **explicit exclude** (SoT-amended). Carrying forward M9 “refuse-closed forever” without an exclude amendment is forbidden for M15 exit.
3. Wave Fs-N may claim a subset; unclaimed rows stay Open (honest refuse) until claimed — but M15 exit requires the full inventory (− exclusions) claimed implement.
4. Hooks: for claimed porcelain that runs hooks under stock Git, arks MUST run **sync** hook scripts from `$GIT_DIR/hooks` (user content). Inventing async/out-of-band hook schedulers remains forbidden (no L3). `--no-verify` skips when the pin documents it.

## Product exclusions (not inventory)

| Exclusion | Cite |
|-----------|------|
| `gitk`, `git-gui`, `git-citool` / GUI | git(1); [[sot:root/cli-surface]] |
| `scalar` | git(1) |
| Git LFS | separate project |
| Forge PR/Checks/Apps UX | [[sot:root/remote-host-compatibility-surface]] scope B |
| CELIR admin | [[sot:root/storage-celir-boundary]] |
| Lease admin / L3 valves | [[sot:root/lease-concurrency-intent]] |
| Credential helper / askpass | Remain **exclude** until SoT amend (non-interactive smart-HTTP assumed) |
| `git daemon` / `git://` server | Exclude until remote-host SoT amend (M11 smart HTTP claimed) |

## Wave Fs-1 claimed ledger (active)

| Command | Claimed flags / forms | Still refuse / Open (later Fs) |
|---------|----------------------|-------------------------------|
| `add` | paths; **`-A`/`--all`**; **`-u`/`--update`**; directory recursion | `--chmod` / patch modes |
| `status` | human; **`--porcelain`/`-s`** (v1 XY subset) | `-b` / porcelain v2 |
| `commit` | `-m` `--allow-empty` `--no-verify` **`--amend`** **`-a`/`--all`** | editor message without `-m` (**exclude**) |
| hooks (commit / update-ref) | sync `pre-commit`, `commit-msg`, `reference-transaction`/`update` when present | async/out-of-band runners |
| `merge` | `--ff-only`; **`--no-ff`** when FF-capable; **diverged three-way** (clean auto-merge or conflicted index/markers) | octopus / strategy options |
| `pull` | FF; `--rebase`; **three-way merge pull** (Fs-2) | credentialed interactive |
| `write-tree` | (default index → tree OID) | `--missing-ok` edges |
| `commit-tree` | `<tree>` `-p` `<parent>` `-m` | multiple `-F` |
| `ls-files` | (tracked index paths) | `-c`/`-o`/`-d` extras |
| `ls-tree` | `<tree-ish>` | `-r` depth edges beyond claimed |
| `symbolic-ref` | read `HEAD` / named symref; set `HEAD` → branch | `--short` extras |
| `merge-base` | two commits → common ancestor OID | `--all` / octopus |

Inherited implement rows from [[sot:root/cli-flag-parity-m9]] + worktree/submodule residual remain claimed unless this ledger narrows them.

## Wave Fs-3 claimed ledger (active)

Primary cites: [git-notes](https://git-scm.com/docs/git-notes), [git-sparse-checkout](https://git-scm.com/docs/git-sparse-checkout), [git-repack](https://git-scm.com/docs/git-repack), [git-gc](https://git-scm.com/docs/git-gc), [gitrepository-layout](https://git-scm.com/docs/gitrepository-layout). Lease honesty: [[sot:root/lease-concurrency-intent]] (`maint:gc`); CORR-001 / PACK-004 / MAINT-003 — lease ≠ skip Git locks; no `gc --prune=now` under writer-risk class.

| Command | Claimed flags / forms | Still refuse / Open / exclude |
|---------|----------------------|-------------------------------|
| `notes` | `add -m <msg> [<object>]`; `show [<object>]` (default `refs/notes/commits`, fanout 0) | `append`/`edit`/`merge`/`prune`; custom `--ref` |
| `sparse-checkout` | `init --cone`; `set <dir>...`; `list` (cone patterns + skip-worktree) | non-cone; `add`/`reapply`/`disable` extras |
| `repack` | `-a` / `-d` / `-ad` under `maint:gc` lease (reachable pack; optional delete redundant loose/old packs) | `--window` / geometric / cruft |
| `gc` | default consolidation (= repack `-ad` path under lease) | `--prune=now` **refuse** (CORR-001); `--auto` / detach |
| `maintenance` | `run` (same lease-honest consolidation as `gc`) | task scheduling / `start`/`stop` |
| `filter-branch` | — | **exclude** (successor honesty: use external history-rewrite tools; not alias inventory) |
| `backfill` | — | **exclude** (partial-clone backfill not claimed) |
| `range-diff` | — | moved to Fs-5 claimed |

## Wave Fs-5 claimed ledger (active)

| Command / form | Claimed | Still refuse / exclude |
|----------------|---------|------------------------|
| `range-diff` | `<oldA>..<oldB> <newA>..<newB>` first-parent subject pairwise (`=`/`!`) | symmetric `...`; full dual-diff emit; merge commits |
| `commit -a` / `--all` | stage tracked updates then commit (with `-m`) | — |
| `commit` without `-m` | — | **exclude** (editor/TUI message path) |
| `rebase -i` | — | **exclude** (interactive/TUI); non-interactive `rebase <upstream>` remains |
| Credential helper / askpass | — | **exclude** (unchanged product exclusion) |
| `git daemon` / `git://` | — | **exclude** (unchanged until remote-host SoT amend) |

## Later waves (Open until claimed)

None for M15 Phase Fs after Fs-5. Per-flag refuse gaps that remained at M15 exit (e.g. `status -b`, `add --chmod`, worktree porcelain extras, submodule `--recursive`, client SSH, plumbing breadth) are **scheduled into M16** [[sot:root/cli-flag-parity-complete]] / [[plan:root/20261109043910]] — they MUST NOT remain as lasting production refuse.

## Verification

Clean-room **G20** (`verify/matrices/matrix-g20.md`) for each Fs wave claim set (historical). Active complete-symmetry verification is **G21** under [[sot:root/cli-flag-parity-complete]]. G14 regression must stay green. Host `cargo test` advisory only.
