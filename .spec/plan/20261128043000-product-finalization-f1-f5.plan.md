---
id: plan:root/20261128043000
title: Product finalization and release-readiness ladder (F1–F5)
status: active
created: 2026-11-28T04:30:00Z
updated: 2026-11-28T04:30:00Z
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
| **F1** | `agy/fix/finalization-test-and-manifest-hygiene` | **A0** | `L-cli` | `Cargo.toml` | `open` | Isolate `HOME`/`GIT_CONFIG_*` in `crates/arks/tests/credential_os_cli.rs`; fix `repository = "https://github.com/modootoday/arks"` in `Cargo.toml` and update stale Wave descriptions in `crates/arks/Cargo.toml` and `crates/arks-remote/Cargo.toml` (keep `version = "0.0.0"`, `publish = false`). Exit: unmodified `cargo test --workspace`, `fmt`, `clippy`, `agy-governance-check.py gate`, and 7 required CI checks green. |
| **F2** | `agy/verify/final-pre-release-cleanroom-receipt` | **A0** | `L-verify` | none | `open` | After F1 merges: run sole-runner Docker clean-room (1) argv-inventory + full flag-corpus compare (1,852/1,852 options, 0 unreasoned DIFFs including `#76` `format-patch` additions), (2) `dod-symmetry-prerelease` with `ARKS_RELEASE_BLOCK_HUB_MISSING=1` (0 Missing, 0 untracked pin leaves), (3) `run-symmetry-latest-inventory.sh` (0 untracked latest leaves). Commit scrubbed receipt under `.spec/evidence/<ts>-final-pre-release-receipt/`. |
| **F3** | `agy/docs/finalization-public-surface-and-changelog` | **A0** | `L-gov` | `docs/completion.md` | `open` | After F2 merges: remove all 29 raw `[[...]]` Specfold wiki links across `README.md` and `docs/**`; sync `README.md`, `docs/completion.md`, and pillar pages with F2 receipt; add `CHANGELOG.md` (`[Unreleased]`); add public-docs wiki-link check to `verify/scripts/agy-governance-check.py`. Exit: 0 raw `[[...]]` in `README.md`/`docs/**` and `gate` green. |
| **F4** | `agy/gov/release-pipeline-dry-run-and-hub-gate` | **A1** (`needs-human`) | `L-gov` | none | `open` | After F2/F3 merge: set `block_hub_missing: true` in `.github/workflows/release.yml`; execute `workflow_dispatch` runs of `release-symmetry.yml` (`block_hub_missing=true`) and `release.yml` (`dry_run=true`) on `main`; record run URLs, 3-OS `dist` artifact list, and `SHA256SUMS` under `.spec/evidence/<ts>-release-dry-run/` (honestly labeled dry-run, not live publish). |
| **F5** | *(owner-directed release cut)* | **A2** | `L-gov` | `Cargo.toml` | `owner-blocked` | Owner approves version (`v0.1.0`), decides GitHub-Release-only vs crates.io `publish = true` strategy, tags `v0.1.0`, reviews/publishes draft GitHub Release (and optional OIDC Environments), and confirms transition to [[sot:root/autonomous-operation]] §11 maintenance mode. |

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
