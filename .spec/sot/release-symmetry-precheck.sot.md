---
id: sot:root/release-symmetry-precheck
title: Release symmetry precheck (git / daemon / gh)
status: active
created: 2026-11-10T19:45:00Z
updated: 2026-11-10T19:45:00Z
tags: [release, verify, symmetry, dod, git, daemon, hub, gh, gha, precheck, pin, latest-inventory, no-invent-pass]
verify: manual
rel:
  depends_on:
    - sot:root/verify-cleanroom-symmetry-matrix
    - sot:root/product-verification-clean-room
    - sot:root/clean-room-git-pin-advance
    - sot:root/cli-git-help-matrix
    - sot:root/arks-daemon-git-command-matrix
    - sot:root/arks-hub-gh-command-matrix
    - sot:root/public-surface
  governs:
    - .github/workflows/release-symmetry.yml
    - .github/workflows/release.yml
    - verify/scripts/run-symmetry-prerelease.sh
    - verify/scripts/run-symmetry-latest-inventory.sh
    - verify/docker-compose.yml
---

# Release symmetry precheck (git / daemon / gh)

Version **0.1.0**. Normative **ops/CI** gate that blocks a tagged release when clean-room CLI symmetry tracking breaks. Extends [[sot:root/verify-cleanroom-symmetry-matrix]]; does **not** invent product Claimed/PASS from green CI alone.

**Published binary:** **`arks` only.** Product paths MUST NOT spawn `git` or `gh`. Stock `git` / `git-daemon` / surveyed `gh` are **oracles** for inventory + matrices only.

**Remote empty — no push** until the owner authorizes.

## When the gate runs

| Trigger | Role |
|---------|------|
| `workflow_dispatch` on `.github/workflows/release-symmetry.yml` | Manual pre-release / tip check |
| `workflow_call` from `.github/workflows/release.yml` | **Hard dependency** before `dist` (tag push and dry-run dispatch) |
| Tag push `v*` via `release.yml` | Same: release cannot package while the gate is red |

Host Windows operators MAY run the same entrypoints locally (`docker compose … dod-symmetry-prerelease` + inventory script). **Authoritative CI venue** is Linux Docker for pin DoD + Linux runner for latest-inventory continuity ([[sot:root/product-verification-clean-room]]).

## Pin vs latest (honesty — do not collapse)

| Oracle | Where | Purpose | Release effect |
|--------|-------|---------|----------------|
| **Pin Git / git-daemon `2.39.5`** | Clean-room image `arks-verify-m1:0.1.0` (`verify/images.lock.json`) | Normative three-pillar DoD judge | **Release-blocking** per table below |
| **Surveyed `gh` pin** (currently **2.103.3**) | Hub design / living CSVs ([[sot:root/arks-hub-gh-command-matrix]]) | Hub leaf inventory currency | Untracked leaves **block**; inventoried v1 Missing policy below |
| **Host / runner “latest” Git + `gh`** | GHA ubuntu job or operator host | Continuously track newer upstream surface **without** pretending the clean-room pin advanced | Untracked cmds **block**; inventoried latest-only Partial/Missing = **advisory** |

Advancing the **normative** Git pin remains [[sot:root/clean-room-git-pin-advance]] (gap audit + lock + evidence). Latest-inventory green **MUST NOT** be labeled pin-DoD Claimed.

## What fails the job (release-blocking)

| Condition | Effect |
|-----------|--------|
| Pillar script **hard FAIL** (help/CSV honesty mismatch, missing required SoT, broken `arks daemon` / `arks hub` entry) | **FAIL** |
| Pin CLI living CSV **`Missing` count > 0** (`verify/matrices/cli-git-help-matrix.csv` / `survey_status`) | **FAIL** |
| Daemon living CSV **`Missing` count > 0** (`verify/matrices/daemon-git-arks-e2e-matrix.csv`) | **FAIL** |
| **Untracked** pin `git --list-cmds` name (present in pin oracle, absent from CLI CSV) | **FAIL** |
| **Untracked** pin `git daemon -h` long-option (absent from daemon CSV) | **FAIL** |
| **Untracked** `gh` top-level / reference leaf vs hub living/design CSV | **FAIL** |
| **Untracked** runner/latest `git --list-cmds` or `gh` leaf vs living CSVs | **FAIL** (tracking lost — must inventory as Partial / Refuse/N/A / Missing before release) |

## What does **not** fail by default (advisory)

| Condition | Effect |
|-----------|--------|
| **Partial** rows (flag-depth / subset honesty) | Summary only — tracked debt, not a silent PASS |
| Hub **v1 `Missing`** counts while rows remain inventoried | **Advisory** by default (`ARKS_RELEASE_BLOCK_HUB_MISSING=0`). Flip to **1** when hub v1 Missing=0 becomes the release bar — **do not invent PASS** while Missing > 0 |
| Latest-only commands already inventoried as Partial / Refuse / Missing | Advisory delta in the job summary (follow-upstream backlog) |
| Host cargo / non-compose greens | Advisory only |

Smoke ≠ Claimed. This gate proves **inventory continuity + pin Missing=0 + hard honesty**, not full man-page / hub REST Claimed.

## Entrypoints

| Entry | Action |
|-------|--------|
| `verify/scripts/run-symmetry-prerelease.sh` | Pin DoD pillars + pin untracked checks + Missing policy |
| `verify/scripts/run-symmetry-latest-inventory.sh` | Latest/host git + gh untracked + advisory deltas |
| compose `dod-symmetry-prerelease` | Linux-in-container pin gate |
| `.github/workflows/release-symmetry.yml` | GHA pin job + latest-inventory job |
| `release.yml` job `symmetry-precheck` | `needs` gate before `dist` |

```bash
docker compose -f verify/docker-compose.yml run --rm dod-symmetry-prerelease
ARKS_INVENTORY_MODE=latest bash verify/scripts/run-symmetry-latest-inventory.sh
```

## Why

Releases must keep following upstream CLI surface **without lying**: pin **2.39.5** stays the normative DoD judge until pin-advance evidence exists; latest inventory still **blocks** when tracking is lost (untracked leaves), while Partial / hub Missing stay honest advisory until their bars flip.

## Verification

1. SoT linked from [[sot:root/verify-cleanroom-symmetry-matrix]] and release workflows exist.
2. `release.yml` `dist` lists `needs: [symmetry-precheck]` (or equivalent hard dependency).
3. Local/CI pin run fails closed on injected untracked / Missing; Partial alone does not fail.
4. No Claimed language in job summaries; artifacts upload counts + untracked lists.
