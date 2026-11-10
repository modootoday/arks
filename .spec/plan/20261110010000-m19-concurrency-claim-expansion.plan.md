---
id: plan:root/20261110010000
title: M19 Concurrency catalog claim expansion
status: active
created: 2026-11-10T01:00:00Z
updated: 2026-11-10T02:10:00Z
tags: [milestone, m19, phase-cx, concurrency, mitigate-with-compat, g17, claim-expansion, issue-resolution, done]
rel:
  depends_on:
    - sot:root/program-roadmap
    - sot:root/multi-agent-git-concurrency-scenarios
    - sot:root/lease-concurrency-intent
    - sot:root/production-residual-register
    - sot:root/interop-test-matrix
    - sot:root/product-verification-clean-room
    - sot:root/git-compat-surface
    - research:root/20261109023117
    - plan:root/20261109024634
    - plan:root/20261109024635
    - evidence:root/20261109140000
    - evidence:root/20261109163000
    - evidence:root/20261110010000
    - evidence:root/20261110020000
  implements: [sot:root/program-roadmap]
---

# M19 — Concurrency catalog claim expansion (Phase Cx)

## Goal

Schedule a **new** explicit claim-set of high-value **local filesystem** concurrency catalog IDs (IDX/REF/WT/OBJ/SEQ-adjacent mitigate class) that remained **unclaimed** after M12 (G17 Cm-1..Cm-3) and M13 (G18 Cd-1..Cd-3). Prove each ID with Docker clean-room **G17** induce-compare (extend proven harness — do **not** invent a folklore G22).

**Honesty:** This milestone does **not** claim the ~221 catalog is solved. It closes **only** the Cx-1 claimed IDs below. L3 remains refuse-closed; G8/G9 stay Closed-deferred. Does **not** reopen or rewrite M12/M13 historical exits.

**Done is not SoT/code claim.** Issue-resolution Done rule ([[sot:root/program-roadmap]]): each claimed ID MUST show stock **`git`** pathology and **`arks`** resolved under the same induced setup in Docker.

## Primary cites (summarize only)

- [api-lockfile](https://git-scm.com/docs/api-lockfile)
- [git-update-ref](https://git-scm.com/docs/git-update-ref)
- [git-worktree](https://git-scm.com/docs/git-worktree)
- [git-rebase](https://git-scm.com/docs/git-rebase) / sequencer state
- [git-update-index](https://git-scm.com/docs/git-update-index)
- [gitrepository-layout](https://git-scm.com/docs/gitrepository-layout)

## Claimed set — Wave Cx-1 (G17 extension)

Selected from remaining Open **mitigate-with-compat** (and lock/sequencer honesty) local races preferred by induce-compare fit. Density matches prior G17 waves (~10 IDs).

| ID | git pathology oracle (intent) | arks resolved oracle (intent) |
|----|-------------------------------|-------------------------------|
| **REF-007** | Planted `HEAD.lock` blocks HEAD update | Fail-closed update-ref/HEAD path; `arks lock inspect` cites REF-007; lease ≠ skip |
| **LOCK-001** | Exclusive `O_CREAT\|O_EXCL` `.lock` blocks second writer | Fail-closed under held lock; inspect exclusive-create / LOCK-001 honesty |
| **WT-002** | Same branch checked out in two worktrees | `worktree add` fail-closed (“already checked out”) |
| **WT-004** | Locked worktree blocks remove | `worktree remove` refuse while locked; unlock then remove |
| **SEQ-001** | Shared rebase state under `.git` | Second `rebase` refuse when `rebase-merge` / `rebase-apply` / sequencer present |
| **SEQ-002** | `MERGE_HEAD` vs rebase collision | `rebase` refuse when merge in progress |
| **SEQ-003** | Shared `CHERRY_PICK_HEAD` / sequencer | Second `cherry-pick` refuse while pick in progress |
| **REF-018** | Pseudoref / sequencer state overwritten class | Conflicting porcelain refuse when `MERGE_HEAD` (or sibling state) present |
| **MISC-012** | `update-index --refresh` under held `index.lock` | Fail-closed under lock; unlocked refresh does not invent optional-lock dialect |
| **TOCTOU-005** | Userspace poll-then-create skips O_EXCL | Write path never clears/skips locks after absence poll; inspect cites TOCTOU-005 |

Wave Cx-1 **Done** = Docker G17 PASS for **all ten** rows + Cm-1..Cm-3 regression green in the same receipt.

## Cx-1 close

| ID | G17 |
|----|-----|
| REF-007 | PASS |
| LOCK-001 | PASS |
| WT-002 | PASS |
| WT-004 | PASS |
| SEQ-001 | PASS |
| SEQ-002 | PASS |
| SEQ-003 | PASS |
| REF-018 | PASS |
| MISC-012 | PASS |
| TOCTOU-005 | PASS |

Cm-1..Cm-3 regression re-verified (13 rows). Aggregate **23**/23 dual PASS. Evidence: [[evidence:root/20261110010000]].

**M19 Cx-1 exit proven.**

## Claimed set — Wave Cx-2 (G17 extension)

| ID | git pathology oracle (intent) | arks resolved oracle (intent) |
|----|-------------------------------|-------------------------------|
| **BISECT-001** | Stock second start clobbers session | Exclusive bisect refuse (BISECT-001) |
| **BISECT-002** | Dirty mid-bisect checkout overwrite fail | `bisect` mark refuse when dirty |
| **SEQ-006** | Interactive todo / rebase-merge blocks rebase | Refuse SEQ-006/SEQ-001 |
| **SEQ-007** | `rebase-apply` blocks `am` | Refuse SEQ-007 |
| **MISC-007** | Parallel am / shared apply state | Cite MISC-007 refuse |
| **STASH-001** | `refs/stash.lock` blocks stash | Refuse STASH-001 |
| **STASH-002** | `index.lock` blocks stash pop | Refuse STASH-002 |
| **REF-021** | `HEAD.lock` blocks symbolic-ref | Fail-closed REF-021 |
| **TOCTOU-003** | Dirty WT blocks checkout overwrite | Refuse TOCTOU-003 |
| **MISC-005** | `index.lock` blocks reset --hard | Refuse MISC-005 |
| **CORR-002** | Stock offers `gc --force` | Refuse CORR-002 |
| **LOCK-004** | Porcelain die-class under lock | Die-class + LOCK-004 inspect |

Wave Cx-2 **Done** = Docker G17 PASS for **all twelve** rows + Cm-1..Cm-3 + Cx-1 regression (**35**/35). Evidence: [[evidence:root/20261110020000]].

**M19 Cx-2 exit proven.** Catalog disposition register closes remaining IDs as Closed-deferred / N/A (bare Open count **0**) — does **not** mean all races solved.

## Explicitly deferred / out of scope (post Cx-2)

| Class | Examples | Disposition |
|-------|----------|-------------|
| Differentiate lease uplift | AGENT-001, SEQ-005, IDX-003, REF-002, REF-010, OBJ-003, NET-001/007, PERF-001/006 | **Closed-deferred** until future Cd/Cx claim-set |
| L3 N-way | HighConcurrency valve | **Refuse-closed** ([[adr:root/20261108232800]]) |
| Opportunity out-of-scope | FS-007, NET-006, OBJ-004, … (9) | **N/A-out-of-scope** |
| Remainder after M12∪M13∪Cx-1∪Cx-2 | 170 IDs | **Closed-deferred** (full ID list in catalog SoT) |

## In scope

1. SoT/plan/matrix updates for M19 + Cx-1 oracles; `spec check`.
2. Engine/CLI mitigations required for Cx-1 resolved oracles only (sequencer exclusivity, refresh lock honor, inspect notes).
3. Extend `verify/gates/g17` + `run-g17.sh` + `verify/matrices/matrix-g17.md` (reuse G17 protocol).
4. Docker clean-room G17 receipt + residual honesty.

## Out of scope

- Redefining M9–M18 historical exits or M12/M13 carrying receipts.
- Inventing Git lock dialects; CELIR product deps; L3 ship; ≥20% / Pf remapping.
- Declaring unclaimed catalog IDs resolved.

## Depends on

- M12 G17 Cm-1..Cm-3 Done — [[evidence:root/20261109140000]] (and prior Cm receipts).
- M13 G18 claimed-set Done — [[evidence:root/20261109163000]] (historical; not reopened).
- Catalog [[sot:root/multi-agent-git-concurrency-scenarios]].

## Exit gates

| Gate | Clean-room | Advisory |
|------|------------|----------|
| **G17** for Cx-1 (10 IDs) + Cm-1..Cm-3 regression | Authoritative | Host unit |
| G14 regression preferred | Should stay green | Host |

Milestone exit = every Cx-1 ID has G17 PASS — not “code merged.”

## Ordered work items

1. **SoT / matrix** — Freeze Cx-1; amend roadmap M19; residual register; interop matrix G17 note; `verify/matrices/matrix-g17.md` Cx-1 section; `spec check`.
2. **Mitigations** — Sequencer/pseudoref exclusivity; `update-index --refresh` honors `index.lock`; lock inspect notes (REF-007 / LOCK-001 / TOCTOU-005); WT-002/WT-004 already claimed — prove via G17.
3. **Harness** — Add Cx-1 tests to `verify/gates/g17`; extend `run-g17.sh` oracle list / counts.
4. **Verify** — Docker G17 PASS; evidence under `.spec/evidence/`; residual honesty.
5. **Close** — Mark M19 Cx-1 Done only with receipts; commit locally.

## Cross-links

[[sot:root/program-roadmap]], [[sot:root/interop-test-matrix]], [[sot:root/multi-agent-git-concurrency-scenarios]], [[plan:root/20261109024634]], [[sot:root/production-residual-register]]
