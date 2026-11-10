---
id: research:root/20261111082440
title: Verify directory conventions — survey pain points
status: active
created: 2026-11-10T23:24:40Z
updated: 2026-11-11T08:40:00Z
tags: [verify, clean-room, docker, conventions, layout, naming, folders, skip, strict, survey]
verify: manual
rel:
  depends_on:
    - sot:root/product-verification-clean-room
    - sot:root/structural-conventions
    - sot:root/verify-cleanroom-symmetry-matrix
    - sot:root/arks-claimed-residual-gates
    - sot:root/release-symmetry-precheck
    - research:root/20261110095355
  evidence: []
---

# Verify directory conventions — survey pain points

Dated survey of `verify/` layout and naming skew at tip ≥ `0f25c27`. Feeds design SoT [[sot:root/verify-directory-conventions]]. **Does not invent Claimed/PASS.**

## Current tree (observed)

```text
verify/
  Dockerfile                 # rust:1.99.1-bookworm + apt git → pin 2.39.5
  docker-compose.yml         # gN + dod-* + advisory + g41 helpers
  images.lock.json           # digests + stock_git / rustc / openssh
  README.md
  scripts/                   # run-g*.sh, run-*-e2e.sh, run-symmetry-*.sh, *.ps1
  matrices/                  # matrix-m*.md, matrix-g*.md, *-matrix.csv, matrix-journey.md
  gates/gN/                  # oracles, locks, out/ receipts (gitignored)
  gates/fs*-*advisory/       # host-advisory receipts
  fixtures/                  # shared oracle intent/bytes (no secrets)
  out/                       # cross-gate inventory / prerelease dumps (gitignored)
```

Compose service families:

| Family | Examples | Role |
|--------|----------|------|
| Milestone / product gates | `g0`…`g41`, `g8-live-scale`, `g28-offline-clones` | Authoritative G-series oracles |
| DoD / symmetry | `dod-cli-matrix`, `dod-daemon-matrix`, `dod-hub-matrix`, `dod-symmetry` | Three-pillar smoke / honesty |
| Release precheck | `dod-symmetry-prerelease` | Pin Missing / untracked leaves |
| Related hosting | `dod-self-host` | Self-host scaffold (≠ Claimed) |
| Claimed residual | `dod-claimed-*`, `dod-claimed-residual` | B1–B9 design/SKIP slots |
| Host advisory | `fs007-host-advisory`, `fs014-lsm-advisory` | Explicit non-DoD |
| Helpers | `toolchain`, `g41-seed`, `g41-dav` | Build / fixture peers |

## Pain points

1. **Dual out roots.** Gate receipts live under `verify/gates/<id>/out/`; symmetry / prerelease dumps use `verify/out/…`. Operators sometimes Tee into wrong roots or leave host-only copies (`symmetry-inventory-latest-host`) next to clean-room paths without labeling authority.
2. **Naming skew across families.** G-gates use short `gN` compose + `run-gN.sh`; DoD uses `dod-<concern>` + longer `run-*-matrix-e2e.sh` / `run-symmetry-*.sh` / `run-claimed-*.sh`. Both are intentional once documented — undocumented they look like two harnesses.
3. **Matrix media mix.** Milestone/journey matrices are Markdown (`matrix-m*.md`, `matrix-g*.md`); symmetry living tables are CSV under the same `matrices/` folder; hub design CSV still also lives under `.spec/sot/`. Placement is workable but needs a single registration rule.
4. **SKIP vs STRICT scattered.** Default scaffold exit-0 + `ARKS_DOD_STRICT=1` fail-closed appears on symmetry, self-host, claimed residual, and release precheck env flags (`ARKS_RELEASE_BLOCK_HUB_MISSING`). Agents need one convention SoT so new gates do not invent a third mode.
5. **Host vs Docker authority drift.** README still mentions `target-cleanroom` in places while compose uses named volume `arks-cargo-target` → `/cargo-target`. Host PowerShell helpers (`*.ps1`) and host Git **2.56** inventory are advisory; pin Git **2.39.5** remains normative — easy to cite the wrong judge.
6. **B-gate slot growth.** Claimed residual slots (`dod-claimed-*`) landed beside symmetry DoD; without a registration checklist, new residuals risk clobbering sibling WIP or skipping residual-register / umbrella cross-links.
7. **Self-host / prerelease adjacency.** `dod-self-host` and `dod-symmetry-prerelease` sit next to pillars but are not fourth pillars — naming alone does not encode that honesty without Spec pointers.
8. **Internal folder naming skew.** Top-level `verify/` folders are already kebab (`scripts/` `matrices/` `gates/` `fixtures/` `out/`), but gate **ids** mix clean `gN` / `gN-<kebab>` (`g8-live-scale`, `g28-offline-clones`) with jammed legacy `g11l3`. Nested gate children are mostly Cargo-shaped (`src/` `tests/` `out/`) plus rare peers (`g41/httpd/`, `g41/seed.sh`). Ephemeral `gates/*/target/` and repo-root `target-cleanroom/` / `target-pilot/` look like verify layout unless documented as host/Cargo spill. No `dod-*` directories exist under `gates/` — residuals are script-first — which is correct once stated, confusing when agents invent empty `gates/dod-*` scaffolds.

## Non-goals of this research

- Rewriting compose services or gate scripts mid sibling B-gate fix.
- Promoting SKIP greens to Claimed.
- Advancing the Git pin beyond `2.39.5`.
- Mass-renaming `g11l3` or other legacy gate paths in the Spec-only convention wave.

## Follow-on

Normative directory / naming / **folder orthography** / out / SKIP-STRICT / registration rules (+ Migration appendix) → [[sot:root/verify-directory-conventions]] (document revision **0.2.0+**).
