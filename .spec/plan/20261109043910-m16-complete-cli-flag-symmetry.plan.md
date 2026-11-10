---
id: plan:root/20261109043910
title: M16 complete CLI flag symmetry
status: archived
created: 2026-11-09T04:39:10Z
updated: 2026-11-09T06:32:00Z
tags: [milestone, m16, phase-sy, cli, parity, complete-symmetry, g21, alias]
rel:
  depends_on:
    - sot:root/program-roadmap
    - sot:root/cli-surface
    - sot:root/cli-flag-parity-complete
    - sot:root/cli-flag-parity-full
    - sot:root/production-residual-register
    - sot:root/interop-test-matrix
    - sot:root/arks-remote
    - sot:root/git-compat-surface
    - sot:root/lease-concurrency-intent
    - plan:root/20261109035335
    - evidence:root/20261109203000
    - evidence:root/20261109044908
    - evidence:root/20261109045705
    - evidence:root/20261109050238
    - evidence:root/20261109054500
    - evidence:root/20261109061600
  implements: [sot:root/program-roadmap, sot:root/cli-surface, sot:root/cli-flag-parity-complete]
  governs: [crates/arks, crates/arks-remote]
---

# M16 — Complete CLI flag symmetry (Phase Sy)

## Goal

Raise the **active** alias/`git=arks` bar from M15 inventory close (which still allowed per-flag refuse leftovers) to **complete symmetry**: every pin-Git CLI command/flag is **implement** or **N/A exclusion** with rationale. Historical M9/M15 exits and receipts stay frozen. Ledger: [[sot:root/cli-flag-parity-complete]]. Gate: **G21**.

**Ordering vs M17/G19:** Functional Sy waves first for touched surface. Full G19 journey allow-list is **M17** ([[plan:root/20261109043916]]) after (or interleaved only on already-green functional units).

## Normative bar (M16 exit)

| Field | Rule |
|-------|------|
| Classification | No production refuse/partial leftovers on alias surface |
| Ledger | [[sot:root/cli-flag-parity-complete]] Sy-1..Sy-5 green |
| Exclusions | Only §Product N/A exclusions (+ command-local N/A) |
| Clean-room | **G21** PASS every Sy wave; G14/G20 regression |
| Engine | Zero gix; CELIR out; lease ≠ skip locks; L3 still N/A |
| SSH | **Client** SSH implement (Sy-4); **server** SSH remains N/A |

## Waves

| Wave | Focus | Gate |
|------|-------|------|
| **Sy-1** | Everyday porcelain flags (`status -b`, `log --graph/--oneline`, `branch -a/-r`, `add --chmod`, `clean -x`, `describe --dirty`, `switch --detach`, `config --global`, `diff --stat`, `init --bare`, …) | G21 Sy-1 |
| **Sy-2** | Worktree porcelain (`list --porcelain`, lock/unlock, `--detach`); submodule `--recursive` / `--remote`; checkout `--orphan` | G21 Sy-2 |
| **Sy-3** | Plumbing breadth (`update-index`, `read-tree`, `diff-tree`, `for-each-ref`, `rev-list`, `show-ref`, `pack-objects`/`index-pack`, `check-ignore`, …) | G21 Sy-3 |
| **Sy-4** | Client SSH transport for clone/fetch/push/submodule add (OpenSSH helper; no product `git` subprocess) | G21 Sy-4 |
| **Sy-5** | Remaining depth: stash show/apply/drop; cherry-pick/revert ranges + sequencer continue/abort; bisect run; notes append; blame/fsck/count-objects/pack-refs; detached commit; range-diff `...` | G21 Sy-5 |

Wave **Done** = Docker G21 PASS for claimed IDs + G14/G20 regression + evidence receipt + tip pin.

## Steps

1. Canon: activate complete ledger + roadmap/residual/interop/cli-surface amendments; `spec check`.
2. Stand up `verify/matrices/matrix-g21.md` + `verify/scripts/run-g21.sh` / compose service (mirror G20 pattern).
3. Implement Sy-1 bottom-up (inner libs → CLI) with host tests advisory; Docker G21 Sy-1.
4. Sy-2 → Sy-5 sequentially (SSH Sy-4 may need fixture OpenSSH in clean-room image).
5. Residual register: mark complete-symmetry Closed; public honesty update.
6. Hand off to M17 for full G19 allow-list.

## Out of scope

- Rewriting M9–M15 historical exits/receipts.
- Inventing uncited Git semantics.
- Host-only “verified.”
- Server SSH / `git://` / dumb HTTP / L3 / CELIR / LFS / GUI / credential-helper.
- Treating M14 frozen `{J-SUBMODULE-01,J-REPACK-01}` as the lasting perf bar (that is M17).

## Record

| Event | Notes |
|-------|-------|
| Plan created | 2026-11-09 — canon start at tip ~`c90f1bb` |
| Sy-1 Done | Docker G21 PASS + G20/G14 regression; [[evidence:root/20261109044908]] |
| Sy-2 Done | Docker G21 PASS; [[evidence:root/20261109045705]] |
| Sy-3 Done | Docker G21 PASS plumbing breadth; [[evidence:root/20261109050238]] |
| Sy-2 Done | Docker G21 PASS (M16-SY2-01..03) + G20/G14 regression; [[evidence:root/20261109045705]] |
| Sy-4 Done | Docker G21 PASS client SSH (M16-SY4-01..02) + G20/G14 regression; image+openssh; [[evidence:root/20261109054500]] |
| Sy-5 Done | Docker G21 PASS remaining depth (M16-SY5-01..03) + G20/G14 regression; [[evidence:root/20261109061600]] |
| Sy-5 cover/am | `format-patch --cover-letter` + `am --3way` + G21 M16-SY5-04; [[evidence:root/20261109063000]] |
| M16 exit | Sy-1..Sy-5 G21 green; active alias bar = implement\|N/A; handoff M17 Pf3 |
