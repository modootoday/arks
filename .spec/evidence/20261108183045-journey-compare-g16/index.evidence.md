---
id: evidence:root/20261108183045
title: Clean-room G16 journey-compare (func + timing)
status: active
created: 2026-11-08T18:30:45Z
updated: 2026-11-09T22:48:31Z
tags: [g16, clean-room, journey-compare, timing, metrics, verified, additive]
rel:
  depends_on: [sot:root/clean-room-journey-compare, plan:root/20261108182457, sot:root/product-verification-clean-room, sot:root/interop-test-matrix, sot:root/cli-surface, sot:root/public-surface, adr:root/20261108083507]
files:
  - {path: metrics.json, sha256: e8a771f8503ec7516b00e9ba35e61d2757975fac88a7238effa6fb5137f988ef, bytes: 14440, store: repo}
---

# Clean-room G16 journey-compare receipts

## Context

Additive **G16** journey-compare surface per [[sot:root/clean-room-journey-compare]] / [[plan:root/20261108182457]]. Records side-by-side **functional** outcomes and **wall-clock** metrics (`arks-journey-metrics/v1`) for first-wave catalog rows. **Does not** unlock or redefine M9 (G14), M10 (G11), or M11 (G15). Host runs are advisory only.

Attached metrics: `metrics.json` in this evidence directory (snapshot of clean-room output with lock digest stamped).

## Environment

| Field | Value |
|-------|-------|
| Compose | `verify/docker-compose.yml` service `g16` |
| Entrypoint | `verify/scripts/run-g16.sh` |
| Image | `arks-verify-m1:0.1.0` |
| Image digest (lock / local Id) | `sha256:bab37bd1ea03832a5ca30aa41e598eafc8bef7a5b1026d65acefb75e3f4a73c1` |
| Base | `rust:1.99.1-bookworm@sha256:e6f2cff35d0f309acf8433b4f05878ed77092602c5065cc81801177da0bb3a9c` |
| Stock Git (in-image) | `git version 2.39.5` |
| rustc (in-image) | `rustc 1.99.1 (58cf8f9c3 2026-10-13)` |
| `arks` profile | **release** (compile excluded from timings) |
| Tree SHA at run | `b51e900dd56d55068a1c6afbabef38a3bb38d4ad` |
| Tree SHA (evidence tip) | `2107c5a0cecef43edb1d5a28db6448003d1bbb30` |
| Host Docker engine | Linux (`desktop-linux`) |
| `CARGO_BUILD_JOBS` | `2` |

## Results

| Row | git | arks | Notes |
|-----|-----|------|-------|
| J-LOCAL-01 | PASS (81 ms) | PASS (71 ms) | same-`.git` peer read OK |
| J-BRANCH-01 | PASS (36 ms) | PASS (130 ms) | |
| J-MERGE-FF-01 | PASS (51 ms) | PASS (164 ms) | `--ff-only` only |
| J-REBASE-01 | PASS (68 ms) | PASS (252 ms) | non-interactive |
| J-BUNDLE-01 | PASS (27 ms) | PASS (57 ms) | packaged-object stand-in; not `repack`/`gc` |
| J-CLONE-LOCAL-01 | PASS (103 ms) | PASS (184 ms) | in-clean-room smart-HTTP peer (not G8) |
| J-ALIAS-01 | — | PASS alias (77 ms) | PATH argv shim |
| J-REFUSE-01 | peer_implements | PASS refuse | `worktree list`; no `.git` mutation on arks/alias |
| gix-exit | — | PASS | zero `gix`/`gitoxide` edges |
| Unit Specfold | — | PASS | `spec check` 0/0 at land |

**Timing honesty:** single cold iteration (`aggregation=raw`); container noise; **no perf SLO**; **do not** cite as “faster than Git” on public surface. Full step breakdown in `metrics.json`.

## Non-claims

- Not a substitute for G14 / G11 / G15 production bars.
- No live GitHub (G8); no CELIR; no lease HighConcurrency product claims; no SSH/`git://` expansion.
- Refuse-closed inventory (`repack`, `gc`, `worktree`, …) remains refuse/out-of-catalog.
- Public marketing perf superiority forbidden without this receipt class + [[sot:root/public-surface]] wording.

## Reproduce

```bash
docker compose -f verify/docker-compose.yml run --rm g16
```
