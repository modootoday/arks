---
id: sot:root/verify-directory-conventions
title: Verify directory conventions and consistency
status: active
created: 2026-11-10T23:25:00Z
updated: 2026-11-10T23:25:00Z
tags: [verify, clean-room, docker, conventions, layout, naming, skip, strict, matrices, claimed-residual, design, no-invent-claimed]
verify: manual
rel:
  depends_on:
    - research:root/20261111082440
    - research:root/20261110095355
    - sot:root/product-verification-clean-room
    - sot:root/structural-conventions
    - sot:root/verify-cleanroom-symmetry-matrix
    - sot:root/arks-claimed-residual-gates
    - sot:root/release-symmetry-precheck
    - sot:root/production-residual-register
    - sot:root/language-english-only
    - sot:root/public-surface
  evidence: []
  governs:
    - verify/README.md
    - verify/docker-compose.yml
    - verify/scripts/
    - verify/matrices/
    - verify/gates/
    - verify/fixtures/
    - verify/out/
    - verify/Dockerfile
    - verify/images.lock.json
---

# Verify directory conventions and consistency

Document revision **0.1.0** (SoT edit counter only — **not** a product/crate release). Normative **design** for how the product clean-room tree under `verify/` is laid out, named, and extended. Complements [[sot:root/product-verification-clean-room]] (authority) and [[sot:root/structural-conventions]] §7 (grouped overlay). Survey pain: [[research:root/20261111082440]].

**Honesty:** This SoT is **ops/harness convention** — **not** product Claimed. Scaffold / SKIP greens **MUST NOT** invent PASS/Claimed. English-only Specfold ([[sot:root/language-english-only]]).

## Rule

### 1. Directory tree contract

| Path | Owns | MUST NOT own |
|------|------|----------------|
| `verify/Dockerfile` | Clean-room image recipe | Host toolchains |
| `verify/images.lock.json` | Pin digests + expected stock Git / rustc / OpenSSH strings | Floating `latest` authority |
| `verify/docker-compose.yml` | Service → script entry map | Specfold SoTs |
| `verify/scripts/` | Deterministic entrypoints (`run-*.sh`, evaluators, host-advisory helpers) | Crate unit tests |
| `verify/matrices/` | Gate/milestone Markdown matrices + living symmetry CSVs | Product Claimed prose |
| `verify/gates/<id>/` | Per-gate oracles, locks, `out/` receipts (out-of-workspace Cargo when needed) | Default workspace membership |
| `verify/fixtures/` | Shared oracle intent/bytes (no secrets) | Crate-private `testdata/` |
| `verify/out/` | Cross-gate inventory / prerelease dumps (gitignored) | Per-gate Claimed receipts |
| `verify/README.md` | Operator map + authority reminder | Second product SoT |

Grouped layout is **mandatory** — no flat dump of unrelated `g*` crates or `matrix-*.md` at `verify/` root ([[research:root/20261110095355]], [[sot:root/structural-conventions]] rule 23).

```text
verify/
  Dockerfile
  docker-compose.yml
  images.lock.json
  README.md
  scripts/                 # compose argv entrypoints
  matrices/                # matrix-*.md + *-matrix.csv
  gates/<id>/              # gN | gN-suffix | fs*-advisory
  fixtures/                # shared oracle intent
  out/                     # gitignored cross-gate dumps
```

### 2. Service and script naming

| Family | Compose service | Script pattern | Notes |
|--------|-----------------|----------------|-------|
| Product / milestone gate | `gN` or `gN-<suffix>` | `run-gN.sh` / `run-gN-<suffix>.sh` | Gate dir `verify/gates/gN/` (or matching suffix) |
| Symmetry DoD pillar | `dod-<concern>-matrix` | `run-<concern>-…-matrix-e2e.sh` | Pillars: cli / daemon / hub |
| Symmetry orchestrator | `dod-symmetry` | `run-symmetry-dod.sh` (+ aliases `run-symmetry-{git,daemon,hub}.sh`) | Halt on hard FAIL only |
| Release precheck | `dod-symmetry-prerelease` | `run-symmetry-prerelease.sh` | [[sot:root/release-symmetry-precheck]] |
| Related hosting | `dod-self-host` | `run-self-host-e2e.sh` | Adjacent, **not** a fourth pillar |
| Claimed residual | `dod-claimed-<concern>` | `run-claimed-<concern>-e2e.sh` | Register under [[sot:root/arks-claimed-residual-gates]] |
| Claimed residual umbrella | `dod-claimed-residual` | `run-claimed-residual-dod.sh` | Orchestrates B-slots |
| Host advisory | `fsNNN-*-advisory` | `run-fsNNN-*-advisory.sh` | Explicitly non-DoD |
| Host wrappers | *(none required)* | `*.ps1` under `scripts/` | Drive compose only; never sole Verified judge |

**MUST:**

1. Every authoritative compose service names exactly one primary `verify/scripts/run-*.sh` (or a documented helper such as `verify/gates/g41/seed.sh` for seed peers).
2. New **DoD / residual / hosting** services use the `dod-` prefix; new **G-series** product gates keep `gN` / `gN-<suffix>`.
3. Script basenames are kebab-case ASCII; PowerShell helpers stay host-advisory.

**MUST NOT:** invent a third product-gate naming family without amending this SoT + [[sot:root/product-verification-clean-room]].

### 3. Artifact output locations

| Kind | Normative path | Git |
|------|----------------|-----|
| Per-gate receipts / metrics / locks | `verify/gates/<id>/out/` (+ `.run.lock.dir/` when used) | **gitignored** (`.gitignore`) |
| Cross-gate symmetry / prerelease inventory | `verify/out/<topic>/` (e.g. `symmetry-prerelease`, `symmetry-inventory`) | **gitignored** |
| Scrubbed Claimed / Verified evidence | `.spec/evidence/<id>/` | Tracked when scrubbed |
| Host-only exploratory dumps | Prefer `verify/out/<topic>-host/` or `.spec.local/` | Never cite as Verified |

**MUST:**

1. Authoritative Docker runs write receipts under the paths above (or env override documented in the script header, e.g. `ARKS_INVENTORY_OUT`).
2. Windows hosts create `out/` directories before Tee/redirect wrappers when the host shell creates the file (missing dir aborts before Docker evidence lands — see `verify/README.md`).
3. Host inventory copies **MUST** be labeled host / advisory in path or filename; they **MUST NOT** overwrite clean-room pin dumps silently.

### 4. SKIP / scaffold vs STRICT fail-closed

| Mode | Env | Expected behavior |
|------|-----|-------------------|
| Default scaffold | `ARKS_DOD_STRICT` unset or `0` | Unfinished DoD / residual / hosting slots **MAY** exit 0 with explicit `SKIP` / `scaffold` language |
| Fail-closed | `ARKS_DOD_STRICT=1` | Unfinished must-implement / Claimed rows **MUST** exit non-zero — **no invent PASS** |
| Release hub Missing policy | `ARKS_RELEASE_BLOCK_HUB_MISSING` | Default `0` = hub v1 Missing advisory; `1` = block (release SoT) |

**ABSOLUTE:**

1. Smoke / SKIP ≠ Claimed ([[sot:root/verify-cleanroom-symmetry-matrix]], [[sot:root/arks-claimed-residual-gates]]).
2. Host cargo greens are advisory only ([[sot:root/product-verification-clean-room]]).
3. New scaffold scripts **MUST** print SKIP/scaffold (or equivalent honesty) on the default path and honor `ARKS_DOD_STRICT=1`.

### 5. Clean-room pin (normative) vs host (non-normative)

| Surface | Normative judge | Host role |
|---------|-----------------|-----------|
| Stock Git / git-daemon | Clean-room pin **`git version 2.39.5`** (`verify/images.lock.json`) | Inventory / skew only (e.g. host 2.56) — **MUST NOT** override pin DoD |
| Image / rustc / OpenSSH | `arks-verify-m1:0.1.0` + lock expected strings | Advisory rebuild convenience |
| Engine | Linux containers; on Windows Desktop **desktop-linux** | Host PowerShell drives compose only |
| Cargo target | Compose named volume → `/cargo-target` (`CARGO_TARGET_DIR`) | Host `target/` / `target/*.exe` advisory |

Advancing the Git pin remains [[sot:root/clean-room-git-pin-advance]]. Latest-inventory continuity remains [[sot:root/release-symmetry-precheck]] — latest green ≠ pin Claimed.

### 6. Matrix CSV / Markdown placement vs gates

| Artifact | Placement | Owning SoT (examples) |
|----------|-----------|------------------------|
| Milestone / gate claim matrices | `verify/matrices/matrix-m*.md`, `matrix-g*.md`, `matrix-journey.md`, … | Gate / milestone SoTs |
| Living symmetry CSVs | `verify/matrices/*-matrix.csv` (cli / daemon / hub status ledgers) | Pillar SoTs + [[sot:root/verify-cleanroom-symmetry-matrix]] |
| Hub design leaf CSV | `.spec/sot/arks-hub-gh-command-matrix.csv` (design inventory) | [[sot:root/arks-hub-gh-command-matrix]] — living status still under `verify/matrices/` |
| Per-gate oracle code | `verify/gates/<id>/` | Gate SoT / plan; **not** under `matrices/` |

**MUST:** matrices describe claims; gates execute oracles; Specfold SoTs own normative meaning. Do not bury living CSVs under `gates/*/`.

### 7. How new Claimed residual gates register

When adding a residual Claimed slot (B-series or successor):

1. **Spec first** — amend [[sot:root/arks-claimed-residual-gates]] (acceptance + honesty) **before** or with the harness commit; do not invent Claimed from green SKIP.
2. **Compose + script names** — `dod-claimed-<concern>` → `verify/scripts/run-claimed-<concern>-e2e.sh`; wire into `dod-claimed-residual` orchestrator.
3. **Default SKIP / STRICT** — honor §4; print design SoT path in script header.
4. **Receipts** — gate-local under `verify/gates/…/out/` **or** documented `verify/out/claimed-<concern>/` while scaffold; scrubbed Claimed evidence → `.spec/evidence/<id>/`.
5. **Cross-links** — residual register next-queue + [[sot:root/verify-cleanroom-symmetry-matrix]] adjacent note + `.spec/README` agent entry if operators need discovery.
6. **Coordinate** — do not clobber sibling in-flight B-gate scripts; prefer Spec-only commits until harness ownership is clear.

Related hosting stays `dod-self-host` ([[sot:root/arks-self-host-server]]) — complementary, **not** a substitute for B1/B2/B7/B9.

### 8. Relationship map (do not collapse)

| Concern | SoT | Compose anchors |
|---------|-----|-----------------|
| Clean-room authority | [[sot:root/product-verification-clean-room]] | `gN`, image lock |
| Tree grouping / Cargo boundary | [[sot:root/structural-conventions]] §7 + this SoT | `scripts/` `matrices/` `gates/` `fixtures/` |
| Three symmetry pillars | [[sot:root/verify-cleanroom-symmetry-matrix]] | `dod-*-matrix`, `dod-symmetry` |
| Release pin / untracked precheck | [[sot:root/release-symmetry-precheck]] | `dod-symmetry-prerelease` |
| Claimed residual B-slots | [[sot:root/arks-claimed-residual-gates]] | `dod-claimed-*` |
| Honesty ledger | [[sot:root/production-residual-register]] | *(citations only)* |
| Directory consistency (this doc) | **this SoT** | naming / out / SKIP / registration |

## Why

`verify/` accumulated G-gates, symmetry DoD, release precheck, self-host scaffold, and Claimed residual slots under one tree. Without a single convention SoT, agents duplicate out paths, mix host and pin authority, or register new residuals without Spec/residual cross-links. This document freezes the directory contract so Spec-first waves stay consistent and fail-closed.

## Verification

1. `spec check` clean for this SoT and [[research:root/20261111082440]] when Spec CLI is available; otherwise peer review of links.
2. Cross-links from [[sot:root/verify-cleanroom-symmetry-matrix]], [[sot:root/arks-claimed-residual-gates]], [[sot:root/release-symmetry-precheck]], [[sot:root/production-residual-register]], [[sot:root/product-verification-clean-room]], [[sot:root/structural-conventions]], and `.spec/README.md`.
3. `verify/README.md` English map matches §1–§5 (authority + tree + naming families).
4. Clean-room pin check: container reports stock Git matching `verify/images.lock.json` (`git version 2.39.5`).
5. No product Claimed language added to public `docs/` from this design-only wave.
