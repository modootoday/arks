---
id: research:root/20261110095355
title: Verify and fixtures Rust-standard layout
status: active
created: 2026-11-10T09:53:55Z
updated: 2026-11-10T11:20:00Z
tags: [verify, fixtures, testdata, rust, cargo, structure, clean-room, srp]
verify: manual
rel:
  depends_on:
    - sot:root/structural-conventions
    - sot:root/crate-architecture
    - sot:root/product-verification-clean-room
  evidence: []
---

# Verify and fixtures Rust-standard layout

Dated research + layout decision for arks `verify/` and crate test/fixture homes relative to Cargo norms and [[sot:root/structural-conventions]].

## Citations (language / convention)

| Source | Scope used |
|--------|------------|
| [Cargo Book — Package Layout](https://doc.rust-lang.org/cargo/guide/project-layout.html) | `src/`, `tests/`, `benches/`, `examples/` are package-conventional; integration tests live under package `tests/` |
| [Rust Book — Test Organization](https://doc.rust-lang.org/book/ch11-03-test-organization.html) | Unit beside code; integration as separate crates under `tests/`; shared helpers as `tests/common/mod.rs` (not `tests/common.rs`) |
| humanymous `rust-migration.sot.md` Structure and naming | Unit beside impl; integration under crate `tests/`; fixtures under a context-specific fixture directory — cite live, do not fork |
| humanymous `container-test-environments.sot.md` | Docker clean-room as product authority; orthogonal to Cargo source trees |

Common Rust Git-tooling practice (gix / gitoxide-class workspaces; stock-Git oracle harnesses): keep library unit/integration under the owning crate; keep stock-Git / multi-crate / compose-bound oracles **outside** default workspace members so host `cargo test --workspace` stays small. Arks maps that role to grouped `verify/`.

## Verdict

**Rust-standard fit after grouped reorg (cb17373+) + path-depth repair: Y** for layout ownership; **Partial** for oracle dedup.

| Area | Current | Fit |
|------|---------|-----|
| Workspace crates under `crates/arks-*` | Yes | Y |
| Crate `tests/` for single-crate integration | Present for `arks-object`, `arks-protocol`, `arks-lease` (G22–G26, G34 model) | Y (pattern) / Partial (coverage) |
| Crate `testdata/` | Absent — no crate-private binary fixtures claimed yet; M1 OID docs are shared intent under `verify/fixtures/m001-clean-room-g0-g1/` | Y (nothing to move) |
| Product clean-room under grouped `verify/` | `scripts/`, `matrices/`, `gates/gN/`, `fixtures/` | Y |
| Flat dump of `verify/g*` + `matrix-*.md` at `verify/` root | Removed | Y |
| Gate harness `CARGO_MANIFEST_DIR` → repo-root fallbacks | After `gates/` nesting, must be `../../..` (not `../..`) | Y (repaired this slice) |
| Duplicate G1/G5 oracles | Crate `tests/` **and** `verify/gates/g{1,5}` mini-crates | Partial (keep until evidence parity) |

Product clean-room is **not** a Cargo package-layout concern; keeping it outside `crates/` matches Cargo Book package layout (which scopes `tests/` to a **package**) plus humanymous clean-room authority. Shared oracle **intent/bytes** belong under `verify/fixtures/`; crate-only bytes would use `crates/arks-<ctx>/testdata/` when introduced.

## Decision (normative; implemented)

### Stays under workspace `verify/` (product clean-room)

- `Dockerfile`, `docker-compose.yml`, `images.lock.json`
- `scripts/` — deterministic gate entrypoints (`run-g*.sh`, `g019-comparative-performance_eval.py`, helpers)
- `matrices/` — gate/milestone matrix markdown
- `gates/gN/` — per-gate harness: out-of-workspace Cargo oracles, locks, `out/` receipts
- `fixtures/` — committed clean-room / oracle **intent and shared bytes** (no secrets); M1 OID docs

### Lives in Rust-standard crate paths

| Kind | Path | Rule |
|------|------|------|
| Unit tests | `src/**` `#[cfg(test)]` | Beside owning module |
| Single-crate integration | `crates/arks-<ctx>/tests/` | Host-advisory; clean-room scripts MAY invoke via `cargo test -p …` (G22–G26, G34) |
| Crate-owned binary/json fixtures | `crates/arks-<ctx>/testdata/` | When bytes are consumed only by that crate’s tests |
| Benches | `crates/arks-<ctx>/benches/` | When claimed |

Multi-crate, stock-Git-in-image, journey catalog, and compose-bound oracles **remain** under `verify/gates/` (not workspace members).

### Naming / SRP

- No flat catalog at `verify/` root beyond harness entry files (`README`, compose, Dockerfile, lock, `.gitattributes`).
- Gate id stays `gN` under `gates/`; matrix file `matrices/matrix-<id>.md`.
- Do not invent a second product-verify root (`e2e/`, `qualification/` rename deferred).

## Tree (before → after)

```text
# Before (flat; pre-cb17373)
verify/
  g001-loose-object-interop/ … g037-gpgsig-ssh/ …          # flat gate packages
  matrix-*.md           # flat matrices
  scripts/
  Dockerfile …
fixtures/m001-clean-room-g0-g1/            # repo-root advisory OID docs

# After (current)
verify/
  README.md
  Dockerfile
  docker-compose.yml
  images.lock.json
  scripts/                 # run-g*.sh, g019-comparative-performance_eval.py, locate-arks.ps1
  matrices/                # matrix-m*.md, matrix-g*.md, matrix-g016-journey-compare.md, …
  gates/
    g001-loose-object-interop/ … g037-gpgsig-ssh/             # clean-room oracles / out / locks (not workspace members)
    g008-live-scale/
  fixtures/
    m1/                    # OID / blob intent docs

crates/arks-<ctx>/
  src/
  tests/                   # integration (preferred for single-crate gates)
  testdata/                # optional crate-local bytes (none required yet)
  benches/                 # optional
```

### Path-depth rule (post-`gates/` nesting)

`CARGO_MANIFEST_DIR` for a gate package is `verify/gates/gN/`. Repo-root fallbacks MUST use three parents (`../../..`); `target-cleanroom` defaults MUST use `../../../target-cleanroom`. Gate-local artifacts (`out/receipt.json`, self path reads) stay at `CARGO_MANIFEST_DIR` with **no** extra `..`. Docker authoritative runs usually prefer `CARGO_TARGET_DIR` / `ARKS_BIN` first; the relative fallbacks are for host-advisory honesty.

### G19 migration map (sibling Docker sole run — do not clash)

| Old | New |
|-----|-----|
| `verify/scripts/run-g019-comparative-performance.sh` | unchanged path |
| `verify/scripts/run-g016-journey-compare.sh` | unchanged path; internals → `verify/gates/g016-journey-compare` |
| `verify/g016-journey-compare/` | `verify/gates/g016-journey-compare/` |
| `verify/g016-journey-compare/out/metrics.json` | `verify/gates/g016-journey-compare/out/metrics.json` |
| `verify/g019-comparative-performance/` | `verify/gates/g019-comparative-performance/` |
| `verify/g019-comparative-performance/out/receipt.json` | `verify/gates/g019-comparative-performance/out/receipt.json` |
| `verify/matrix-g019-comparative-performance.md` | `verify/matrices/matrix-g019-comparative-performance.md` |
| `verify/matrix-g016-journey-compare.md` | `verify/matrices/matrix-g016-journey-compare.md` |
| `fixtures/m001-clean-room-g0-g1/` | `verify/fixtures/m001-clean-room-g0-g1/` |
| `verify/gN/` (all gates) | `verify/gates/gN/` |
| `verify/matrix-*.md` | `verify/matrices/matrix-*.md` |

## Dedup follow-through (deferred; honest)

- **G1 / G5:** crate `tests/` already exist; clean-room scripts invoke both `-p` and `--manifest-path verify/gates/g{1,5}`. Keep gate manifests until a later slice deletes duplicates after evidence parity — do **not** delete in a layout-only slice.
- Prefer new single-crate gates as `crates/…/tests/` + thin `verify/scripts/run-gN.sh` (G34 model).

## Remaining gaps

1. G1/G5 dual harness bodies (anti-fragmentation residual).
2. Historical evidence log lines under `.spec/evidence/**` may still quote pre-move `/workspace/verify/gN` — leave as archival; do not rewrite to invent Verified/PASS.
3. No crate `testdata/` yet — add only when a crate owns private fixture bytes.

## Out of scope

- Merging all multi-crate oracle crates into workspace members.
- Renaming gate IDs or matrix schemas.
- Humanymous `qualification/` tree fork.
- Long Docker G19 remasure (sole-runner ownership).
