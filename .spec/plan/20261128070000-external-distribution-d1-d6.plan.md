---
id: plan:root/20261128070000
title: External distribution and multi-registry OIDC publishing ladder (D1–D6)
status: active
created: 2026-11-28T07:00:00Z
updated: 2026-11-28T07:00:00Z
tags: [plan, release, distribution, oidc, github-pages, crates-io, pypi, npm, rubygems, d-ladder]
verify: manual
rel:
  depends_on:
    - sot:root/external-distribution-and-registry-publishing
    - sot:root/agy-automated-development
    - sot:root/autonomous-operation
    - sot:root/release-symmetry-precheck
    - research:root/20261110085731
---

# External distribution and multi-registry OIDC publishing ladder (D1–D6)

Executable external-distribution queue governed by
[[sot:root/external-distribution-and-registry-publishing]] and executed under
[[sot:root/agy-automated-development]]. Each unit maps 1:1 to a task-scoped `agy/<lane>/<slug>`
branch and Pull Request.

Status values: `open`, `in-progress`, `done` (with PR link and receipt), `owner-blocked` (external
registry UI / one-time owner bootstrap required).

| Unit | Branch | Class | Lane | Hot files | Status | Exit criterion |
|------|--------|-------|------|-----------|--------|----------------|
| **D1** | `agy/gov/d1-github-pages-and-env-hardening` | **A1** (`needs-human`) | `L-gov` | none | `done` (PR `#89`, `5a6a742`; live Pages run `46118731570` → `https://modootoday.github.io/arks/` HTTP 200; 4 Environments hardened with `v*` tag policy + `myso-kr` reviewer) | Land `.github/workflows/pages.yml`, enable GitHub Pages (`build_type: workflow`), verify live `https://modootoday.github.io/arks/` HTTP 200, and enforce custom `v*` tag policy + required reviewer `myso-kr` on `crates`/`npm`/`pypi`/`rubygems` GitHub Environments via API. |
| **D2** | `agy/gov/d2-crates-io-publishability` | **A0** | `L-gov` | `Cargo.toml` | `done` (`cargo package --workspace --locked` verified all 18 crates; `scripts/bootstrap-crates-publish.sh` + topological OIDC `publish-crates` in `release.yml`) | Pin internal workspace dep versions in `Cargo.toml`, add publishable crate metadata across all 18 crates (only `crates/arks` has `[[bin]]`), verify `cargo package` + workspace tests, add `scripts/bootstrap-crates-publish.sh` (`--dry-run` green), and wire topological 18-crate OIDC publishing in `.github/workflows/release.yml`. |
| **D3** | `agy/gov/d3-d5-pypi-npm-rubygems-packaging` | **A0** | `L-gov` | none | `done` (`packaging/pypi/` 3-OS PEP 427/600 wheel packer verified via isolated `pip install --no-index` -> `arks version 0.1.0`; `publish-pypi` OIDC wired in `release.yml`) | Land `packaging/pypi/` (`pyproject.toml`, `build_wheels.py`), verify local Linux wheel build + isolated `pip install --no-index` + `arks --version`, and wire `publish-pypi` in `.github/workflows/release.yml` (`pypa/gh-action-pypi-publish@release/v1`, OIDC). |
| **D4** | `agy/gov/d3-d5-pypi-npm-rubygems-packaging` | **A0** | `L-gov` | none | `done` (`packaging/npm/` 3 platform pkgs + root `arks` shim verified via `npm pack` + offline `npm install` -> `arks version 0.1.0`; `scripts/bootstrap-npm-publish.sh` + `publish-npm` OIDC wired in `release.yml`) | Land `packaging/npm/` (`build_packages.mjs`, `bin/arks.js`) for root `arks` + `arks-linux-x64` / `arks-darwin-arm64` / `arks-win32-x64`, verify local `npm pack` + offline `npm install` + `arks --version`, add `scripts/bootstrap-npm-publish.sh`, and wire `publish-npm` OIDC (`--provenance`) in `.github/workflows/release.yml`. |
| **D5** | `agy/gov/d3-d5-pypi-npm-rubygems-packaging` | **A0** | `L-gov` | none | `done` (`packaging/rubygems/` 3-OS platform gem packer verified via isolated `gem install --local` -> `arks version 0.1.0`; `publish-rubygems` OIDC via `configure-rubygems-credentials@v1` + `gem push` wired in `release.yml`) | Land `packaging/rubygems/` (`build_gems.rb`, `exe/arks`) for `x86_64-linux` / `arm64-darwin` / `x64-mingw-ucrt` platform gems, and wire `publish-rubygems` OIDC (`rubygems/configure-rubygems-credentials@v1` + `gem push`) in `.github/workflows/release.yml`. |
| **D6** | `agy/verify/d6-multichannel-dryrun-and-handoff` | **A0** | `L-verify` | `docs/release.md` | `open` | Upgrade `dry-run-report` and `scripts/release-channel-readiness.sh` to build and smoke-test all 4 channels from 3-OS `dist` artifacts, run live GHA `release.yml` (`dry_run=true`) on `main`, commit scrubbed receipt under `.spec/evidence/`, sync `README.md`/`docs/getting-started.md`/`docs/release.md`, and open `needs-owner` handoff issue for external registry Trusted Publisher registration + `v0.1.1` tag cut. |
