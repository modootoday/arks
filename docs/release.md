---
title: Release channels
description: Maintainer release-channel ops — OIDC Environments, tag publish, dry-run
nav_order: 85
sections:
  - model
  - dry-run
  - live
  - honesty
  - {c: cta, name: next}
---

# Release channels

Maintainer notes for shipping **arks** binaries and (later) language-registry packages. End users still install from source or Release assets until registry packaging is flipped on — see [Getting started](../getting-started/).

## Version and tag strategy

| Layer | Rule |
|-------|------|
| Workspace / crate `version` | **`0.1.0`** (first public binary release [`v0.1.0`](https://github.com/modootoday/arks/releases/tag/v0.1.0); all 18 workspace crates packaged and verified for crates.io, with only `crates/arks` declaring `[[bin]]`) |
| Spec SoT `Version **0.x.y**` | Document revision only — **not** a crates.io / git-tag release (see `.agents/rules/26-sot-version-vs-release.md`) |
| Git tag | Push an annotated tag matching `v*` (for example `v0.1.0`) when cutting a public release |
| Published artifact | **Single binary `arks` only** (no separate `arks-daemon` / `arks-hub` / `arks-agent` release binaries) |

Do **not** bump crate version or cut a `v*` tag from Spec document revisions alone. Behavioral DoD met ≠ automatic tag — see [arks-dod-behavioral](https://github.com/modootoday/arks/blob/main/.spec/sot/arks-dod-behavioral.sot.md). Release inventory gate: [release-symmetry-precheck](https://github.com/modootoday/arks/blob/main/.spec/sot/release-symmetry-precheck.sot.md). External distribution specification: [external-distribution-and-registry-publishing](https://github.com/modootoday/arks/blob/main/.spec/sot/external-distribution-and-registry-publishing.sot.md).

## Pre-release symmetry gate

`.github/workflows/release.yml` job `dist` **needs** `symmetry-precheck` (reusable `.github/workflows/release-symmetry.yml` with `block_hub_missing: true`). Pin Missing=0 across CLI, daemon, and hub v1 plus untracked leaves **block** packaging; Partial rows stay **advisory**. Green gate ≠ product Claimed/PASS.

## Release artifacts checklist

On a live `v*` tag (after symmetry-precheck):

1. Cross-OS **dist** builds of **`arks`** only (`linux-gnu`, `windows-msvc`, `aarch64-apple-darwin`).
2. Archives + `SHA256SUMS` (checksum / unsigned).
3. **Draft** GitHub Release with those assets.
4. Approve-gated OIDC registry jobs (`crates` / `npm` / `pypi` / `rubygems`) — each Environment enforces tag policy `v*` and required reviewer `myso-kr`.

## Channel model

Near-term KEEP channels use **GitHub Actions OIDC** (no long-lived registry tokens) plus per-registry **Environments**:

| Channel / Environment | Job / Workflow | Packaging status | Live registry status |
|-----------------------|----------------|------------------|----------------------|
| **GitHub Releases** | `github-release` in `release.yml` | Ready (`3-OS` archives + `SHA256SUMS`) | **Live (`v0.1.0`+)** |
| **GitHub Pages** | `.github/workflows/pages.yml` (`github-pages`) | Ready (`docs/` Jekyll build) | **Live** ([modootoday.github.io/arks](https://modootoday.github.io/arks/)) |
| `crates` (**crates.io**) | `publish-crates` (`scripts/bootstrap-crates-publish.sh`) | Verified (`18/18` crates via `cargo package --locked`) | `18/18` bootstrapped at `0.1.0`; Trusted Publishers bound; OIDC live (`v0.1.1`+) |
| `pypi` (**PyPI**, `arks-cli`) | `publish-pypi` (`packaging/pypi/build_wheels.py`) | Verified (`3` PEP 427/600 wheels + venv pip smoke) | Trusted Publisher bound; OIDC live (`v0.1.1`+) |
| `npm` (**npm**, `@modootoday/arks`) | `publish-npm` (`packaging/npm/build_packages.mjs`) | Verified (`@modootoday/arks` + 3 scoped platform pkgs + offline smoke) | Trusted Publishers bound on all 4 `@modootoday/*` pkgs; OIDC live (`v0.1.1`+) |
| `rubygems` (**RubyGems**, `arks`) | `publish-rubygems` (`packaging/rubygems/build_gems.rb`) | Verified (`3` platform gems + isolated gem smoke) | Pending Trusted Publisher bound; OIDC live (`v0.1.2`+) |

Ungated on every `v*` tag: Cross-OS **dist** binaries and a **draft** GitHub Release with checksum / unsigned assets (`SHA256SUMS`). Signing-constrained catalogs (winget, apt signed repos, Authenticode, Apple notarization, Chocolatey) stay out of scope.

Configure trusted publishers in the registry UIs — YAML cannot register external accounts. Design detail lives in [external-distribution-and-registry-publishing](https://github.com/modootoday/arks/blob/main/.spec/sot/external-distribution-and-registry-publishing.sot.md) and `.github/workflows/release.yml`.

## Dry-run (no publish PASS)

From the Actions UI, run the **release** workflow with **dry_run** enabled (`workflow_dispatch`). That path:

1. Builds the same dist matrix and checksums.
2. Prints a readiness report (`scripts/release-channel-readiness.sh`).
3. Does **not** create a GitHub Release and does **not** call registry publish.

A green dry-run means **ops wiring is ready**, not that crates.io / npm / PyPI / RubyGems published.

Local structural check (no secrets):

```bash
bash scripts/release-channel-readiness.sh
```

## Live tag publish

1. Push an annotated or lightweight tag matching `v*` (for example `v0.1.0`).
2. Wait for dist + draft Release assets.
3. Approve each Environment job you intend to publish (`crates` / `npm` / `pypi` / `rubygems`).
4. Ensure the matching trusted publisher on the registry is bound to workflow `release.yml` and that Environment name before approving.

Do not paste tokens into the repo. Fail-closed skips remain until packaging and trusted publishers are ready.

## Honesty

- Host / Actions green is **not** product-verified clean-room evidence.
- Dry-run success is **not** a Verified registry publish PASS.
- crates.io / docs.rs timing remains an owner policy choice — see [Limits](../limits/) and product docs for what is claimed.

## Next

[Contribute](../contribute/) [Forge scale harness](../forge-scale/) [Getting started](../getting-started/) [Limits](../limits/)
