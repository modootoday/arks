---
id: research:root/20261128043000
title: Finalization readiness gap audit at tip 6666549
status: active
created: 2026-11-28T04:30:00Z
updated: 2026-11-28T04:30:00Z
tags: [research, finalization, release-readiness, audit, symmetry, docs, packaging, clean-room]
verify: manual
rel:
  depends_on:
    - sot:root/product-intent
    - sot:root/program-roadmap
    - sot:root/production-residual-register
    - sot:root/autonomous-operation
    - sot:root/agy-automated-development
    - sot:root/release-symmetry-precheck
    - sot:root/public-surface
    - plan:root/20261124000000
  related:
    - sot:root/product-finalization-and-release-readiness
    - plan:root/20261128043000
---

# Finalization readiness gap audit at tip `6666549`

Audit date: **2026-11-28 UTC**. Tip audited on `origin/main`: **`6666549`** (after PRs `#75`–`#78`
landed `agy` parallel governance, salvaged `format-patch` and `push`/`report-status` work, and
hardened `agy-governance-check.py`). This note records factual gaps between current engine
completion (`version = "0.0.0"`) and a shippable, zero-drift release candidate. It makes no new
product Claimed assertion.

## 1. Confirmed product baseline on `main` (`6666549`)

- **Core product goal ([[sot:root/product-intent]]):** single published Rust binary `arks` (never
  spawning `git` or `gh`), filesystem `.git` as sole Git-compat object store (CELIR withdrawn,
  [[adr:root/20261119200000]]), standards compatibility first / performance second, default product
  leases + opt-in high-concurrency shapes, three peer-symmetry pillars + adjacent self-host +
  optional `arks agent` extension.
- **Symmetry CSV baseline (living matrices under `verify/matrices/`):**
  - `cli-git-help-matrix.csv` (195 rows): **128 `Verified`**, 36 `Partial` (each with a derived
    reason from Q20), 21 `Parity-candidate`, 6 `arks-only`, 4 `Synonym-wired`, **0 `Missing`**.
  - `daemon-git-arks-e2e-matrix.csv` (39 rows): **37 `Parity`**, **2 `arks-only`**, **0 `Partial`**,
    **0 `Missing`**.
  - `hub-gh-arks-e2e-matrix.csv` (248 rows): **171 `Claimed`**, **77 `Refuse/N/A`**, **0 `Partial`**,
    **0 `Missing`**, **0 `Deferred`**.
- **Autonomous queue ([[plan:root/20261124000000]]):** Q1–Q23 all `done`; zero open PRs; zero
  orphan remote branches; `agy-governance-check.py sweep --strict` exit 0.
- **Crate stubs scan:** `rg "todo!\(|unimplemented!\(" crates/` returned **0 hits**.

## 2. Concrete gaps blocking final product release readiness

### Gap 1 — Host test isolation and legacy Cargo metadata (`F1`)

1. `crates/arks/tests/credential_os_cli.rs` (`used_as_credential_helper`) invokes `arks credential`
   without isolating `HOME` / `GIT_CONFIG_GLOBAL` / `GIT_CONFIG_NOSYSTEM`. On any developer host or
   `agy` VM whose global `~/.gitconfig` configures `credential.helper = !...gh auth git-credential`,
   the test fails because `arks` rightly refuses spawning `gh`.
2. Root `Cargo.toml` line 49 still sets `repository = "https://github.com/humanymous/gir"` (legacy
   pre-rename URL instead of `https://github.com/modootoday/arks`).
3. Crate descriptions remain outdated:
   - `crates/arks/Cargo.toml`: `"arks for Git — Wave L+H+Pa+Pb CLI (not production; production = M9)"`
   - `crates/arks-remote/Cargo.toml`: `"Generic Git smart HTTP client (Wave H subset; not production)"`
   These should reflect the actual repository URL and current product role while keeping
   `version = "0.0.0"` and `publish = false` untouched until the owner's A2 release cut.

### Gap 2 — Post-salvage clean-room Docker re-receipt (`F2`)

1. The latest committed argv-inventory and flag-corpus receipt
   (`.spec/evidence/20261125062639-argv-corpus-receipt-after-queue/`) was captured at `4814ae2`
   (before PR `#76` added 48 new `format-patch` corpus lines and header/numbering/threading fixes,
   and before PR `#77` landed per-ref `push` `report-status` and `receive-pack` ref resolution).
2. Neither `dod-symmetry-prerelease` (with `ARKS_RELEASE_BLOCK_HUB_MISSING=1`) nor
   `run-symmetry-latest-inventory.sh` has a committed post-Q23 / post-`#76`–`#77` receipt, and
   `.github/workflows/release-symmetry.yml` has zero recorded runs (`gh run list` empty).

### Gap 3 — Human-facing public docs lag and Specfold wiki-link leakage (`F3`)

Per [[sot:root/public-surface]] (human vs agent vocabulary split), `README.md` and `docs/**` must
render cleanly on GitHub and GitHub Pages without raw internal Specfold wiki syntax (`[[...]]`)
and must match current receipts:

1. **29 raw Specfold wiki links** leak into public Markdown:
   - `README.md` (4), `docs/completion.md` (10), `docs/self-host.md` (5), `docs/agent.md` (3),
     `docs/daemon.md` (2), `docs/performance.md` (2), `docs/ports.md` (1), `docs/forge-scale.md` (1),
     `docs/arks-hub.md` (1).
2. **`README.md` honesty lag** (behind `docs/completion.md` and Q17–Q23 receipts):
   - Still states `arks daemon` has `"Docker receipt pending"` (closed by Q19).
   - Still states hub v1 leaves are `"Partial"` (closed by Q17/Q18: 171/171 Claimed).
   - Still states `"CLI survey Partial×117 remain"` (closed by Q20: 128 Verified / 36 Partial).
   - Still states `Windows file-mtime Partial`, `ssh-smoke Partial`, `smokes Partial` (closed by Q21–Q23).
   - Still mentions `"Optional index sidecar"` (CELIR withdrawn per [[adr:root/20261119200000]]).
3. **`CHANGELOG.md` absent:** no human-readable changelog exists for the first public release.
4. **Regression guard missing:** `verify/scripts/agy-governance-check.py` does not yet reject raw
   Specfold `[[...]]` links inside `README.md` or `docs/**`.

### Gap 4 — Release pipeline unexercised dry-run and hub gate toggle (`F4`)

1. `.github/workflows/release.yml` and `.github/workflows/release-symmetry.yml` have **0 recorded
   runs** on GitHub Actions.
2. `release.yml` line 61 still sets `block_hub_missing: false` even though hub v1 `Missing` is now
   **0** (171/171 Claimed, 0 Missing). Flipping `block_hub_missing: true` makes all three symmetry
   pillars release-blocking on Missing > 0 as designed in [[sot:root/release-symmetry-precheck]].
3. A `workflow_dispatch` dry-run of `release.yml` (`dry_run=true`) has never been executed to prove
   end-to-end 3-OS `dist` packaging (`x86_64-unknown-linux-gnu`, `x86_64-pc-windows-msvc`,
   `aarch64-apple-darwin`), `LICENSE*` bundling, `SHA256SUMS`, and `release-channel-readiness.sh`.

### Gap 5 — First public release cut and maintenance-mode handoff (`F5`, Level A2)

1. `Cargo.toml` stays at `version = "0.0.0"` with `publish = false` across all crates and `0` Git
   tags.
2. Choosing the first release version (e.g. `v0.1.0`), flipping `version` (and optionally
   `publish = true` for `crates/arks` if publishing to crates.io), pushing the `v*` tag, and
   approving draft GitHub Release / OIDC Environments (`crates`, `npm`, `pypi`, `rubygems`) are
   strictly **Level A2 (owner-only)** acts.

## 3. Summary table

| Phase | Scope | Autonomy | Blocking release? |
|-------|-------|----------|-------------------|
| **F1** | Test isolation (`credential_os_cli.rs`) + `Cargo.toml` repo URL & crate descriptions | **A0** | Yes (metadata + host test reliability) |
| **F2** | Sole-runner Docker clean-room re-receipt (argv/corpus + pin symmetry + latest inventory) | **A0** | Yes (evidence currency after `#76`–`#77`) |
| **F3** | `README.md` / `docs/**` public-surface cleanse + `CHANGELOG.md` + wiki-link guard | **A0** | Yes (public docs honesty & rendering) |
| **F4** | `release.yml` `block_hub_missing: true` + `release-symmetry` & `release` dry-run proof | **A1** | Yes (release pipeline verification) |
| **F5** | Version bump, `v0.1.0` tag, draft Release publish, optional OIDC approvals, §11 maintenance mode | **A2** | Owner decision |
