---
id: plan:root/20261110085731
title: GitHub Actions Cross-OS CI and OIDC release (pointer)
status: active
created: 2026-11-10T08:57:31Z
updated: 2026-11-10T00:35:00Z
tags: [github-actions, ci, cross-os, oidc, release, perf-ci, plan, pointer]
rel:
  depends_on:
    - research:root/20261110085731
    - sot:root/product-verification-clean-room
    - sot:root/clean-room-journey-compare
    - sot:root/public-surface
---

# GitHub Actions Cross-OS CI and OIDC release (pointer)

## Goal

Cross-OS CI, perf CI venue honesty (G16/G19), and OIDC multi-registry release for arks. Survey apt/winget/signing channels only to mark them **OUT / Deferred**. **Phases 1–2 workflow YAML landed** (2026-11-10): `.github/workflows/ci.yml`, `.github/workflows/release.yml`. **Perf CI research disposition landed** in research §B (no separate `perf.yml` required; authoritative G19 remains sole-runner Docker).

## Authority

Full cite-backed design: [[research:root/20261110085731]] (includes KEEP vs EXCLUDE channel table + perf CI job disposition).

## Phases (summary)

| Phase | Work | Workflows? |
|------:|------|------------|
| **0** | Research + this pointer; signing-constrained channels OUT; perf CI venue disposition; `spec check` | No |
| **1** | Cross-OS matrix + MSRV + cache + clean-room **image smoke** (advisory) | Yes (CI only) |
| **2** | Dist binaries + GitHub Release (checksum/unsigned) + crates.io / PyPI / npm OIDC; optional RubyGems | Yes (release) |
| **3+** | Re-open only by owner decision: Homebrew tap (non-cert) and/or EXCLUDE’d signing channels (winget / apt GPG / Authenticode / notarization / Chocolatey); optional `gha-advisory` journey/trend jobs per research §B | Only if re-opened |

## Non-goals (this pointer)

- Inventing Git semantics or redefining G19 venue.
- Long-lived registry tokens when trusted publishing exists.
- Treating GHA wall-clock as product-verified perf.
- Authoring a Verified G19 / Criterion-as-Verified workflow on hosted runners.
- Near-term **winget**, **apt** (signed repo/PPA), **Chocolatey**, **Authenticode**, or **Apple notarization** as in-scope release channels.

## GitHub Environments (created on `modootoday/arks`)

Mirror of sibling release envs (`specfold` / `jekyll-theme-modootoday`). Workflow jobs MUST use these names (not a single `release` env):

| Environment | Required reviewer | Deployment branch policy | Env secrets/vars |
|-------------|-------------------|--------------------------|------------------|
| `crates` | `myso-kr` | custom tag `v*` | none (OIDC) |
| `npm` | `myso-kr` | tag `v*` | none |
| `pypi` | `myso-kr` | tag `v*` | none |
| `rubygems` | `myso-kr` | tag `v*` | none |

Also present (unchanged): `github-pages`. Settings: https://github.com/modootoday/arks/settings/environments

## Workflows landed

| File | Trigger | Notes |
|------|---------|-------|
| `.github/workflows/ci.yml` | PR + push `main` | lint / Cross-OS test / MSRV / clean-room image smoke on pinned runners |
| `.github/workflows/release.yml` | tag `v*` | ungated dist + draft Release; OIDC publish jobs with `environment: crates` / `npm` / `pypi` / `rubygems` |

**Runner pins:** `ubuntu-24.04`, `windows-2025`, `macos-15` (Annotation disposition in research § Runner-image Annotations).

**Human residual:** configure trusted publishers on crates.io / npm / PyPI / RubyGems bound to `release.yml` + matching Environment name; flip `publish = false` / land wrapper trees before first real publish; Environment reviewers already set (`myso-kr`).

## Research topics (Phase 0 exit checklist)

| Topic | Disposition | Cite |
|-------|-------------|------|
| Cross-OS + runner Annotation pins | KEEP; workflows pin `ubuntu-24.04` / `windows-2025` / `macos-15` | research §A + Annotations |
| OIDC multi-registry (crates/npm/PyPI/RubyGems) | KEEP; Environments + `release.yml` | research §D + Environments table |
| apt / winget (+ Chocolatey / Authenticode / notarization) | EXCLUDE / Deferred (signing constraints); surveyed with reasons | research §E + scope table |
| Perf CI (journey/G19 venue + advisory shapes) | Venue honesty landed; Verified G19 OUT of GHA; no `perf.yml` required | research §B |

## Next

1. Resolve open naming/musl/Release-asset decisions in a future SoT if product intent requires.
2. Land npm / PyPI / gem wrapper packaging when product is publish-ready; enable `cargo publish` for crates.
3. Optional later: `gha-advisory` journey exit-0 / trend jobs only under research §B labels — never as Verified.
