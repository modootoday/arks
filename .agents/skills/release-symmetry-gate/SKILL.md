---
name: release-symmetry-gate
description: >
  Use when preparing a tagged release or interpreting release-symmetry CI:
  pin Missing=0 / untracked-leaf blocks, Partial advisory, no invent PASS from
  green workflows alone.
---

# Release symmetry gate

## Preconditions

- SoT: `.spec/sot/release-symmetry-precheck.sot.md`
- Workflows: `.github/workflows/release-symmetry.yml`, `release.yml`
- Scripts: `verify/scripts/run-symmetry-prerelease.sh`, `run-symmetry-latest-inventory.sh`
- Rules `28`, `61`, `91` (no push until ordered)

## Blocking vs advisory (short)

| Condition | Effect |
|-----------|--------|
| Pin pillar hard FAIL / pin CSV Missing>0 / untracked pin or gh leaf | **FAIL** (release-blocking) |
| Partial rows / hub v1 Missing (default) / latest-only inventoried debt | **Advisory** |
| Host cargo green alone | Advisory — not pin-DoD |

## Steps

1. Read the precheck SoT tables (pin vs latest honesty).
2. Prefer clean-room prerelease compose / workflow_dispatch over host-only runs.
3. If Missing>0 or untracked leaves: fix inventory/implementation — do not relabel as PASS.
4. Do **not** `git push` tags/remotes unless the owner explicitly authorizes (push-deferred).
5. Smoke ≠ Claimed — gate proves inventory continuity + pin honesty, not full man-page Claimed.

## Done

- [ ] Pin vs latest not collapsed
- [ ] No invented PASS language in summary
- [ ] Push still deferred unless ordered
