---
id: plan:root/20261108182457
title: Clean-room journey compare harness
status: active
created: 2026-11-08T18:24:57Z
updated: 2026-11-09T06:45:00Z
tags: [verification, docker, clean-room, journey, g16, harness, bottom-up]
rel:
  depends_on:
    - sot:root/clean-room-journey-compare
    - sot:root/product-verification-clean-room
    - sot:root/interop-test-matrix
    - sot:root/cli-surface
    - sot:root/arks-remote
    - adr:root/20261108083507
---

# Clean-room journey compare harness

## Goal

Land **G16** dual-runner harness + journey catalog metrics per [[sot:root/clean-room-journey-compare]], expand Wave-2/Wave-3 additive rows (path/`file://` clone + optional RSS/CPU), close median/p95 aggregation posture, and produce Docker clean-room evidence. Additive only — do not redefine M9/M10/M11 or replace G0–G15.

## Steps (bottom-up)

1. **Specfold** — Accept SoT `sot:root/clean-room-journey-compare`; cross-link interop matrix + clean-room SoT + research matrix row; `spec check`.
2. **Inner product slice** — `arks-remote` local path / same-host `file://` clone via object-directory copy (+ pack→loose for checkout); wire `arks clone` dispatch; unit tests. Cite git-clone(1) local forms; no hardlink/`--shared`; no piped upload-pack claim.
3. **Inner harness crate** — `verify/g16/` outside root workspace members (same pattern as `verify/g14`):
   - Catalog runners for Wave-1 + Wave-2 + Wave-3 rows (incl. **J-CLONE-PATH-01**).
   - Dual lanes (`git` / `arks` / alias where required).
   - Wall-clock via `std::time::Instant` (no gix; no heavy bench deps).
   - Optional Linux child `peak_rss_kb` / `cpu_ms` sampling; omit when unavailable.
   - Multi-iteration (`ARKS_JOURNEY_ITERATIONS`, default 3) with **median + nearest-rank p95** aggregation.
   - Emit `verify/g16/out/metrics.json` (`arks-journey-metrics/v1`, `aggregation=median_p95`).
   - Functional hard-fail on claimed mismatches; refuse rows assert arks non-zero + no `.git` mutation.
4. **Entrypoint** — `verify/scripts/run-g16.sh` (LF-only): print stock Git/rustc; gix-exit scan; `cargo build -p arks --release`; run g16 tests; print metrics path.
5. **Compose / docs** — service `g16` in `verify/docker-compose.yml`; `verify/matrix-journey.md`; thin README mention; optional BUGSIM CRLF note.
6. **Clean-room evidence** — `compose run --rm g16`; file `.spec/evidence/<ts>-journey-compare-g16/` with metrics snapshot + honesty residuals.
7. **Land** — English commit; push when coordinator authorized.

## Record

| Artifact | Path |
|----------|------|
| SoT | `.spec/sot/clean-room-journey-compare.sot.md` |
| Plan | this file |
| Matrix | `verify/matrix-journey.md` |
| Script | `verify/scripts/run-g16.sh` |
| Crate | `verify/g16/` |
| Metrics schema | `arks-journey-metrics/v1` (`median_p95` + optional RSS/CPU) |
| Evidence | `.spec/evidence/*-journey-compare-g16/` |

### Non-goals (plan)

- Absolute ms pass/fail perf bars (deferred; must not redefine production).
- Live GitHub G8 journeys.
- Implementing refuse-closed porcelain.
- CELIR / lease product claims in journeys.
- Hardlink/`--shared` local clone; piped `file://` upload-pack clone.
