---
id: research:root/20261110095355
title: Verify and fixtures Rust-standard layout
status: active
created: 2026-11-10T09:53:55Z
updated: 2026-11-10T09:53:55Z
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

Dated research + layout decision for arks `verify/` and `fixtures/` relative to Cargo norms and [[sot:root/structural-conventions]].

Convention cites (do not fork): humanymous `rust-migration.sot.md` Structure and naming (unit beside impl; integration under crate `tests/`; fixtures under a context-specific fixture directory); humanymous `container-test-environments.sot.md` (Docker clean-room as product authority, orthogonal to Cargo source trees).

## Verdict

**Rust-standard fit today: N** (partial).

| Area | Current | Fit |
|------|---------|-----|
| Workspace crates under `crates/arks-*` | Yes | Y |
| Crate `tests/` for single-crate integration | Sparse (`arks-object`, `arks-protocol`, `arks-lease`); many gates only under `verify/g*` | Partial |
| Crate `testdata/` / context fixtures | Absent; root `verify/fixtures/m1/` is advisory OID docs | N |
| Product clean-room under `verify/` | Compose + scripts + lock present | Y (role) |
| Flat dump of `verify/g*` + `matrix-*.md` | ~35 siblings each | N (SRP / flat-list ban) |
| Duplicate G1/G5 oracles | Crate `tests/` **and** `verify/g{1,5}` mini-crates | N (anti-fragmentation) |

Product clean-room is **not** a Cargo concern; keeping it outside `crates/` is correct ([[sot:root/product-verification-clean-room]], research sketch `verify/fixtures/` in [[research:root/20261108073650]]). The defect is **flat ownership** inside `verify/` and a **root `fixtures/`** that is neither crate `testdata` nor grouped under the clean-room tree.

## Decision (normative intent → implement now)

### Stays workspace `verify/` (product clean-room)

- `Dockerfile`, `docker-compose.yml`, `images.lock.json`
- `scripts/` — deterministic gate entrypoints (`run-g*.sh`, `g19_eval.py`, helpers)
- `matrices/` — gate/milestone matrix markdown (was flat `matrix-*.md`)
- `gates/gN/` — per-gate harness: out-of-workspace Cargo oracles, locks, `out/` receipts, gate-local BUGSIM notes
- `fixtures/` — committed clean-room / oracle **intent and shared bytes** (no secrets); M1 OID docs move here from repo-root `fixtures/`

### Moves into crate Rust-standard paths

| Kind | Path | Rule |
|------|------|------|
| Unit tests | `src/**` `#[cfg(test)]` | Beside owning module |
| Single-crate integration | `crates/arks-<ctx>/tests/` | Host-advisory; clean-room scripts MAY invoke via `cargo test -p …` (G22–G26, G34 pattern) |
| Crate-owned binary/json fixtures | `crates/arks-<ctx>/testdata/` | When bytes are consumed only by that crate’s tests |
| Benches | `crates/arks-<ctx>/benches/` | When claimed |

Multi-crate, stock-Git-in-image, journey catalog, and compose-bound oracles **remain** under `verify/gates/` (not workspace members) so default host `cargo test` stays small.

### Naming / SRP

- No new flat catalog at `verify/` root beyond harness entry files (`README`, compose, Dockerfile, lock).
- Gate id stays `gN` under `gates/`; matrix file `matrices/matrix-<id>.md`.
- Do not invent a second product-verify root (`e2e/`, `qualification/` rename deferred).

## Target layout

```text
verify/
  README.md
  Dockerfile
  docker-compose.yml
  images.lock.json
  scripts/                 # run-g*.sh, g19_eval.py, locate-arks.ps1
  matrices/                # matrix-m*.md, matrix-g*.md, matrix-journey.md, …
  gates/
    g1/ … g37/             # clean-room oracles / out / locks (not workspace members)
    g8-live-scale/
  fixtures/
    m1/                    # OID / blob intent docs (from former repo-root fixtures/)

crates/arks-<ctx>/
  src/
  tests/                   # integration (preferred for single-crate gates)
  testdata/                # optional crate-local bytes
  benches/                 # optional
```

### G19 migration map (sibling Docker sole run)

| Old | New |
|-----|-----|
| `verify/scripts/run-g19.sh` | unchanged path |
| `verify/scripts/run-g16.sh` | unchanged path; internals → `verify/gates/g16` |
| `verify/gates/g16/` | `verify/gates/g16/` |
| `verify/gates/g16/out/metrics.json` | `verify/gates/g16/out/metrics.json` |
| `verify/gates/g19/` | `verify/gates/g19/` |
| `verify/gates/g19/out/receipt.json` | `verify/gates/g19/out/receipt.json` |
| `verify/matrices/matrix-g19.md` | `verify/matrices/matrix-g19.md` |
| `verify/matrices/matrix-journey.md` | `verify/matrices/matrix-journey.md` |

No half-moved paths: scripts, compose argv (still `verify/scripts/…`), SoT cites, gitignore, and AGENTS pointers update in the same change.

## Dedup follow-through (same change window where cheap)

- **G1 / G5:** crate `tests/` already exist; clean-room scripts already (G1) or SHOULD invoke `-p` tests. Keep `verify/gates/g{1,5}` as the authoritative clean-room cargo manifests until a later slice deletes duplicates after evidence parity — do **not** delete in this move.
- Prefer new single-crate gates as `crates/…/tests/` + thin `verify/scripts/run-gN.sh` (G34 model), not new flat `verify/gN` Cargo packages at repo `verify/` root.

## Out of scope

- Merging all multi-crate oracle crates into workspace members.
- Renaming gate IDs or matrix schemas.
- Humanymous `qualification/` tree fork.
