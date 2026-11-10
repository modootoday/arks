---
id: plan:root/20261109035335
title: Full CLI symmetric parity residual
status: archived
created: 2026-11-09T03:53:35Z
updated: 2026-11-09T20:30:00Z
tags: [milestone, m15, phase-fs, cli, parity, alias, residual, g20]
rel:
  depends_on:
    - sot:root/program-roadmap
    - sot:root/cli-surface
    - sot:root/cli-flag-parity-m9
    - sot:root/production-residual-register
    - sot:root/interop-test-matrix
    - sot:root/git-compat-surface
    - sot:root/public-surface
    - plan:root/20261108183812
    - plan:root/20261109022956
    - evidence:root/20261108150000
    - evidence:root/20261109114207
    - evidence:root/20261109180000
    - evidence:root/20261109183000
    - evidence:root/20261109190000
    - evidence:root/20261109193000
    - evidence:root/20261109200000
  implements: [sot:root/program-roadmap, sot:root/cli-surface]
  governs: [crates/arks]
---

# M15 — Full CLI symmetric parity (Phase Fs)

## Goal

Raise the **active** alias-honest inventory from M9’s closed implement/**refuse** ledger to **full Git↔arks CLI symmetric replacement** against the clean-room pin ([git(1)](https://git-scm.com/docs/git/2.39.0) command list): every stock Git CLI command/feature that is **not** an explicit product exclusion becomes **claimed implement** with a flag matrix vs pin Git. M9 historical exit + receipt remain frozen; this track does **not** rewrite M9/M10/M11 bars.

**Exclusions (unchanged product refuse):** GUI/`gitk`/`git-gui`/`git-citool`, Git LFS, `scalar`, CELIR admin, forge PR/Checks/Apps UX (remote-host scope B), lease admin / L3 valves (lease SoT), byte-identical stderr unless a gate asserts it.

**Ordering vs M14:** Functional full-CLI gates (**G20**) first. **M14/G19 ≥20%** continues only on journeys whose functional claim is already green; perf MUST NOT block or redefine CLI symmetry.

## Normative bar (M15 exit)

| Field | Rule |
|-------|------|
| **Inventory** | Main porcelain + required ancillary + plumbing minimum from git(1) for the pin, minus exclusions — classified **implement** (not silent refuse) in [[sot:root/cli-flag-parity-full]] |
| **Flag parity** | Every documented option on claimed commands: implement or explicit **exclude**; silent ignore forbidden |
| **Hooks** | Sync hooks honored for claimed porcelain (scripts are user content); `--no-verify` where pin documents it |
| **Alias** | `alias git=arks` honesty for the **full** claimed inventory (not everyday subset) |
| **Clean-room** | **G20** green for every claimed wave; G14/G16 functional regression; host advisory only |
| **Engine** | Zero `gix`; CELIR out of product deps; lease ≠ skip Git locks; L3 refuse-closed unless SoT/ADR opens |

## Waves (ordered)

| Wave | Claim focus | Gate |
|------|-------------|------|
| **Fs-1** | `commit --amend`; sync hook runner (replace “refuse when hooks present”); `merge --no-ff` when histories are FF-capable (merge commit); plumbing minimum (`write-tree`, `commit-tree`, `ls-files`, `ls-tree`, `symbolic-ref`, `merge-base`); pack-aware store read/ingest supporting clone | G20 Fs-1 |
| **Fs-2** | True non-FF / recursive tree merge + conflicted index path; `pull` merge (non-rebase); expand merge strategies claimed subset | G20 Fs-2 |
| **Fs-3** | Former refuse-closed porcelain: `notes`, `range-diff`, `sparse-checkout` (cone), `gc`/`repack`/`maintenance` under lease honesty (no CORR-001 races), `filter-branch` successor honesty or documented exclude amend | G20 Fs-3 |
| **Fs-4** | Plumbing completeness for scripting (`update-index`, `read-tree`, `diff-tree`, `for-each-ref`, `rev-list`, `pack-objects`/`index-pack` as claimed); remaining flag gaps on existing commands (`add -A`/`-u`, `status --porcelain`, …) | G20 Fs-4 |
| **Fs-5** | Interactive / editor-driven forms where SoT claims them (or explicit exclude); credential-helper protocol only if SoT amends exclusion; server daemon only if remote-host SoT amends | G20 Fs-5 |

Wave **Done** = Docker **G20** PASS for that wave’s claimed IDs + G14 regression green + evidence receipt + tip pin.

## Fs-1 close

| ID | G20 |
|----|-----|
| M15-FS1-01..04 | PASS |
| G14 regression | PASS |

Evidence: [[evidence:root/20261109180000]].

## Fs-2 close

| ID | G20 |
|----|-----|
| M15-FS2-01 diverged clean | PASS |
| M15-FS2-02 conflict markers | PASS |
| Fs-1 + G14 regression | PASS |

Evidence: [[evidence:root/20261109183000]].

## Fs-4 close

| ID | G20 |
|----|-----|
| M15-FS4-01 | PASS |
| Fs-1/Fs-2 + G14 regression | PASS |

Evidence: [[evidence:root/20261109190000]].

## Fs-3 close

| ID | G20 |
|----|-----|
| M15-FS3-01 notes add/show | PASS |
| M15-FS3-02 sparse-checkout cone | PASS |
| M15-FS3-03 repack -ad + lease | PASS |
| M15-FS3-04 gc / --prune=now refuse | PASS |
| M15-FS3-05 filter-branch/backfill exclude | PASS |
| Fs-1/2/4 + G14 regression | PASS |

Evidence: [[evidence:root/20261109193000]].

## Fs-5 close

| ID | G20 |
|----|-----|
| M15-FS5-01 range-diff | PASS |
| M15-FS5-02 commit -a | PASS |
| M15-FS5-03 interactive/editor exclude | PASS |
| Fs-1..4 + G14 regression | PASS |

Evidence: [[evidence:root/20261109200000]].

## M15 exit

All Fs-1..Fs-5 claim sets green under Docker **G20** + G14 regression. Residual register Full CLI row **Closed**. Public alias honesty cites full-inventory bar (− exclusions). Carrying evidence: [[evidence:root/20261109203000]].

## Out of scope

- Rewriting M9–M14 historical exits or carrying receipts.
- Inventing wire/object/index semantics.
- Treating host greens as verified.
- Starting M15 Done while listing refuse rows as “parity.”
- Letting M14 perf redefine functional claim sets.

## Exit

M15 exit = all Fs waves’ claimed sets green under G20 + public honesty cites full-inventory alias bar + residual register row Closed.

## Cross-links

[[sot:root/program-roadmap]], [[sot:root/cli-surface]], [[sot:root/cli-flag-parity-full]], [[sot:root/production-residual-register]], [[sot:root/interop-test-matrix]], [[plan:root/20261109024636]]
