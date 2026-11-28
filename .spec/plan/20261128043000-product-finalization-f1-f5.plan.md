---
id: plan:root/20261128043000
title: Product finalization and release-readiness ladder (F1–F5)
status: archived
created: 2026-11-28T04:30:00Z
updated: 2026-11-28T06:22:00Z
tags: [plan, finalization, release-readiness, clean-room, docs, agy, f-ladder]
verify: manual
rel:
  depends_on:
    - sot:root/product-finalization-and-release-readiness
    - sot:root/agy-automated-development
    - sot:root/autonomous-operation
    - sot:root/release-symmetry-precheck
    - research:root/20261128043000
---

# Product finalization and release-readiness ladder (F1–F5)

Executable finalization queue governed by [[sot:root/product-finalization-and-release-readiness]] and
executed under [[sot:root/agy-automated-development]]. Each unit maps 1:1 to a task-scoped
`agy/<lane>/<slug>` branch and Pull Request.

Status values: `open`, `in-progress`, `done` (with PR link and receipt), `owner-blocked` (Level A2).

| Unit | Branch | Class | Lane | Hot files | Status | Exit criterion |
|------|--------|-------|------|-----------|--------|----------------|
| **F1** | `agy/fix/finalization-test-and-manifest-hygiene` | **A0** | `L-cli` | `Cargo.toml` | `done` (PR `#80`, `5d2dc88`: `GIT_CONFIG_GLOBAL` override fix + unit test, canonical `modootoday/arks` repo URL and crate descriptions) | Isolate `HOME`/`GIT_CONFIG_*` in `crates/arks/tests/credential_os_cli.rs`; fix `repository = "https://github.com/modootoday/arks"` in `Cargo.toml` and update stale Wave descriptions in `crates/arks/Cargo.toml` and `crates/arks-remote/Cargo.toml` (keep `version = "0.0.0"`, `publish = false`). Exit: unmodified `cargo test --workspace`, `fmt`, `clippy`, `agy-governance-check.py gate`, and 7 required CI checks green. |
| **F2** | `agy/verify/final-pre-release-cleanroom-receipt` | **A0** | `L-verify` | none | `done` (`.spec/evidence/20261128052500-final-pre-release-receipt/` at `5d2dc88`: 1,852/1,852 options, 4,446/4,549 SAME, 77 post-pin + 26 design + 0 timing/open/unreasoned; `dod-symmetry-prerelease` with `BLOCK_HUB_MISSING=1` OK; `latest-inventory` untracked=0) | After F1 merges: run sole-runner Docker clean-room (1) argv-inventory + full flag-corpus compare (1,852/1,852 options, 0 unreasoned DIFFs including `#76` `format-patch` additions), (2) `dod-symmetry-prerelease` with `ARKS_RELEASE_BLOCK_HUB_MISSING=1` (0 Missing, 0 untracked pin leaves), (3) `run-symmetry-latest-inventory.sh` (0 untracked latest leaves). Commit scrubbed receipt under `.spec/evidence/<ts>-final-pre-release-receipt/`. |
| **F3** | `agy/docs/finalization-public-surface-and-changelog` | **A0** | `L-gov` | `docs/completion.md` | `done` (56 raw `[[...]]` tokens converted across `README.md` + 8 `docs/*.md` pages, 0 remaining; `README.md`/`docs/completion.md`/`docs/arks-hub.md` synced to F2 receipt; `CHANGELOG.md` added; public-doc wiki-link guard added to `agy-governance-check.py`) | After F2 merges: remove all 29 raw `[[...]]` Specfold wiki links across `README.md` and `docs/**`; sync `README.md`, `docs/completion.md`, and pillar pages with F2 receipt; add `CHANGELOG.md` (`[Unreleased]`); add public-docs wiki-link check to `verify/scripts/agy-governance-check.py`. Exit: 0 raw `[[...]]` in `README.md`/`docs/**` and `gate` green. |
| **F4** | `agy/gov/release-pipeline-dry-run-and-hub-gate` (+ `agy/verify/f4-release-dry-run-receipt`) | **A1** (`needs-human`) | `L-gov` / `L-verify` | none | `done` (PR `#83` `block_hub_missing: true`, PR `#84` `gh 2.105.0` `event` out-of-scope; live GHA `release-symmetry` run `46114404035` + `release` dry-run `46114405826` at `4d4f810` all green, 3-OS dist + SHA256SUMS recorded at `.spec/evidence/20261128061000-release-dry-run/`, 0 releases created) | After F2/F3 merge: set `block_hub_missing: true` in `.github/workflows/release.yml`; execute `workflow_dispatch` runs of `release-symmetry.yml` (`block_hub_missing=true`) and `release.yml` (`dry_run=true`) on `main`; record run URLs, 3-OS `dist` artifact list, and `SHA256SUMS` under `.spec/evidence/<ts>-release-dry-run/` (honestly labeled dry-run, not live publish). |
| **F5** | `agy/gov/release-v0-1-0-cut` (owner-approved via Issue `#86`) | **A2** | `L-gov` | `Cargo.toml` | `done` (owner approved Option A `v0.1.0` GitHub Release binaries with `publish = false`; version bumped in `Cargo.toml`/`Cargo.lock`, `CHANGELOG.md` promoted to `0.1.0`, annotated tag `v0.1.0` + GitHub Release published; plan archived into [[sot:root/autonomous-operation]] §11 maintenance mode) | Owner approves version (`v0.1.0`), decides GitHub-Release-only vs crates.io `publish = true` strategy, tags `v0.1.0`, reviews/publishes draft GitHub Release (and optional OIDC Environments), and confirms transition to [[sot:root/autonomous-operation]] §11 maintenance mode. |

## Verification sequence per unit

1. **Before work:** `python3 -I verify/scripts/agy-governance-check.py sweep --strict`.
2. **Branch:** `git checkout -b <unit-branch> origin/main`.
3. **Build / execute:** Bottom-up, one concern per commit, owner noreply identity, zero trailers.
4. **Before push:** `python3 -I verify/scripts/agy-governance-check.py gate` (plus sole-runner Docker
   clean-room for F2).
5. **PR & merge:** Push and open PR in the same turn with `Parallel Lease`, `Verification`, and
   `Class`; enable `--auto --merge` for A0/A1 (with `needs-human` + recorded measurement for A1).
6. **After merge:** Update this table row to `done` (with PR number and receipt ID) and move to the
   next unit.
