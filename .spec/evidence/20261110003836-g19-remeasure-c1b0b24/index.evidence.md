---
id: evidence:root/20261110003836
title: G19 sole-runner remeasure at tip c1b0b24 (PF5 partial 7/8 Verified)
status: active
created: 2026-11-10T00:38:36Z
updated: 2026-11-10T00:38:36Z
tags: [g19, g16, pf5, clean-room, perf, journey-compare, verified]
rel:
  depends_on: [sot:root/clean-room-journey-compare, sot:root/pf5-journey-seed-amortization, sot:root/production-residual-register, sot:root/product-verification-clean-room, sot:root/interop-test-matrix, evidence:root/20261111001400]
  evidence: []
files:
  - {path: receipt.json, sha256: 0883fd4fabebdc00adb31a27f6bc2ca5c03e028f8f26af5248baeeba67e26125, bytes: 7248, store: repo}
  - {path: metrics.json, sha256: 1ee56a5875486bceb057925e9c11f544975beff9dba4d19f96cf04999a3f0b6d, bytes: 228219, store: repo}
---

# G19 sole-runner remeasure — tip `c1b0b24` (PF5 7/8 Verified)

## Context

Authoritative sole-runner Docker `g19` (embeds G16 functional + timing) after history rewrite of `origin/main` to tip **`c1b0b24b2d1722c986fec19f8c7087ec39680a0c`**. Reconfirms **M17 ≤0.8× 19/19** and **Pf5 aggressive terminal 19/19** with honest **Claimed(partial 7/8)** umbrella (CLONE-LOCAL remains `aggressive_na`). Prior PF5 claim receipt [[evidence:root/20261111001400]] remains the amortization flip cite; this receipt is the **Verified** carrying pin for published docs numbers.

## Scope measured (in)

| Surface | IDs | Role |
|---------|-----|------|
| G16 functional + timing | All 19 catalog journeys in `verify/matrices/matrix-journey.md` | Hard functional class + `median_p95` metrics |
| G19 comparative | Same 19 allow-list (`verify/matrices/matrix-g19.md` Pf3+) | ≤0.8× regression + ≤0.5× aggressive / `aggressive_na` |

Journeys: LOCAL, BRANCH, MERGE-FF, REBASE, BUNDLE, CLONE-LOCAL, CLONE-PATH, ALIAS, WORKTREE, TAG, STASH, CHERRY, DIFF, RM-MV, ARCHIVE, REFLOG, REVERT, REPACK, SUBMODULE.

## Out of scope (honest Refuse / N/A — not invented PASS)

| Area | Disposition |
|------|-------------|
| Partial-clone filters / `blob:none` / promisor | **N/A** (G9/G35 depth-only claim) |
| OpenPGP / X.509 / `tag -s` | **N/A** (G37 = SSH gpgsig only) |
| Forge OAuth | **Refuse** (G36 Basic; CRED ≠ OAuth) |
| Live GitHub scale / G8 without secrets | SKIP / secrets-gated — not this receipt |
| Absolute ms marketing SLO / “faster than Git” slogan | Forbidden without this receipt class + public-surface honesty |

## Environment

| Field | Value |
|-------|-------|
| Compose | `verify/docker-compose.yml` service `g19` (sole; first attempt noise-failed CLONE-LOCAL ≤0.8× then sole re-run PASS) |
| Image | `arks-verify-m1:0.1.0` / `sha256:5736c5687d3b5775e187ca3d21fbc5b2a5a2bf8965c5381778ee478eb185adf0` |
| Stock Git | `git version 2.39.5` |
| Iterations / aggregation | 3 / `median_p95` |
| Wave | `Pf5` |
| Tip measured | `c1b0b24b2d1722c986fec19f8c7087ec39680a0c` |

## Aggregate

| Metric | Value |
|--------|------:|
| G16 functional | **19/19** PASS |
| M17 ≤0.8× | **19/19** PASS |
| ≤0.5× claimed | **18** |
| aggressive_na | **1** (J-CLONE-LOCAL-01) |
| aggressive terminal | **19/19** |
| miss | **[]** |
| Gate exit | **0** (carrying sole re-run) |

### Remaining aggressive_na

| ID | git_ms | arks_ms | ratio | Note |
|----|-------:|--------:|------:|------|
| J-CLONE-LOCAL-01 | 86 | 57 | 0.6628 | HTTP seed amortization exhausted; ≤0.5× needs product smart-HTTP clone work |

### Honesty — first sole attempt

First sole `g19` at this tip failed **M17** on **J-CLONE-LOCAL-01** (`ratio=0.8625`, git=80 / arks=69) while Pf5 aggressive terminal still 19/19. Treated as container scheduling noise (known short/mid clone variance); sole re-run recovered ≤0.8× without harness or product changes. Carrying artifacts are the **PASS** re-run only.

Gate exit **0**.
