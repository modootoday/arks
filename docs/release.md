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

## Channel model

Near-term KEEP channels use **GitHub Actions OIDC** (no long-lived registry tokens) plus per-registry **Environments**:

| Environment | Job | Notes |
|-------------|-----|-------|
| `crates` | `publish-crates` | Skip while `publish = false` on the CLI crate |
| `npm` | `publish-npm` | Skip until an `npm/` package tree exists |
| `pypi` | `publish-pypi` | Skip until PyPI packaging exists |
| `rubygems` | `publish-rubygems` | Skip until a gemspec exists |

Ungated on every `v*` tag: Cross-OS **dist** binaries and a **draft** GitHub Release with checksum / unsigned assets (`SHA256SUMS`). Signing-constrained catalogs (winget, apt signed repos, Authenticode, Apple notarization, Chocolatey) stay out of scope.

Configure Environments and trusted publishers in the GitHub / registry UIs — YAML cannot invent reviewers. Design detail lives under `.spec/research/` (OIDC release research) and `.github/workflows/release.yml`.

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

[Contribute](../contribute/) [Getting started](../getting-started/) [Limits](../limits/)
