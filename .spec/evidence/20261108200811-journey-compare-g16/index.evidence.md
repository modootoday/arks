---
id: evidence:root/20261108200811
title: Clean-room G16 journey-compare Wave-2 + median/p95
status: active
created: 2026-11-08T20:08:11Z
updated: 2026-11-10T00:57:42Z
tags: [g16, clean-room, journey-compare, timing, metrics, verified, additive, median-p95]
rel:
  depends_on: [sot:root/clean-room-journey-compare, plan:root/20261108182457, sot:root/product-verification-clean-room, sot:root/interop-test-matrix, sot:root/cli-surface, sot:root/public-surface, adr:root/20261108083507, evidence:root/20261108183045]
files:
  - {path: metrics.json, sha256: 8fcec9ac8ab658b769af85fb14026a468a50022f2f55856cbfc4277766c24032, bytes: 31810, store: repo}
---

# Clean-room G16 journey-compare receipts (Wave-2 + median/p95)

## Context

Additive **G16** journey-compare expansion per [[sot:root/clean-room-journey-compare]] **v0.2.0** / [[plan:root/20261108182457]]. Prior Wave-1 receipt: [[evidence:root/20261108183045]]. Records side-by-side **functional** outcomes and **median + nearest-rank p95** wall-clock metrics (`arks-journey-metrics/v1`, `aggregation=median_p95`, `iterations=3`) for Wave-1 + Wave-2 catalog rows. **Does not** unlock or redefine M9 (G14), M10 (G11), or M11 (G15). Host runs are advisory only.

Attached metrics: `metrics.json` in this evidence directory (snapshot of clean-room output with lock digest stamped).

## Environment

| Field | Value |
|-------|-------|
| Compose | `verify/docker-compose.yml` service `g16` |
| Entrypoint | `verify/scripts/run-g16.sh` |
| Image | `arks-verify-m1:0.1.0` |
| Image digest (lock / local Id) | `sha256:bab37bd1ea03832a5ca30aa41e598eafc8bef7a5b1026d65acefb75e3f4a73c1` |
| Stock Git (in-image) | `git version 2.39.5` |
| rustc (in-image) | `rustc 1.99.1 (58cf8f9c3 2026-10-13)` |
| `arks` profile | **release** (compile excluded from timings) |
| Iterations / aggregation | **3** / `median_p95` |
| Tree SHA at run | `74fc493675dbb20bee51a805c5c866530bb2df29` |
| Tree SHA (evidence tip) | `b52392bee7f47158c6bed4eb2ea356b30eb5ae35` |
| Host Docker engine | Linux (`desktop-linux`) |
| `CARGO_BUILD_JOBS` | `2` |

## Results (median wall_ms_total; p95 in parentheses)

| Row | git | arks / alias | Notes |
|-----|-----|--------------|-------|
| J-LOCAL-01 | PASS 22 (22) | PASS 59 (62) | same-`.git` peer read OK |
| J-BRANCH-01 | PASS 39 (45) | PASS 155 (504) | container noise visible in p95 |
| J-MERGE-FF-01 | PASS 52 (53) | PASS 174 (868) | `--ff-only` only; p95 noise |
| J-REBASE-01 | PASS 68 (68) | PASS 252 (258) | non-interactive |
| J-BUNDLE-01 | PASS 27 (61) | PASS 57 (59) | packaged-object stand-in |
| J-CLONE-LOCAL-01 | PASS 101 (149) | PASS 108 (202) | in-clean-room smart-HTTP (not G8) |
| J-ALIAS-01 | — | PASS alias 75 (77) | PATH argv shim |
| J-REFUSE-01 | peer_implements | PASS refuse | `worktree list` |
| J-TAG-01 | PASS 12 (17) | PASS 32 (35) | annotated tag; stock Git reads tag |
| J-STASH-01 | PASS 31 (39) | PASS 65 (126) | push/list/pop |
| J-CHERRY-01 | PASS 19 (21) | PASS 96 (96) | single clean cherry-pick |
| J-DIFF-01 | PASS 13 (17) | PASS 39 (104) | `diff` + `diff --cached` |
| J-RM-MV-01 | PASS 32 (32) | PASS 92 (97) | tracked mv + rm |
| J-ARCHIVE-01 | PASS 7 (10) | PASS 18 (18) | `--format=tar` |
| J-REFLOG-01 | PASS 11 (23) | PASS 23 (36) | `reflog` / `reflog show` |
| J-REVERT-01 | PASS 9 (10) | PASS 31 (32) | `revert HEAD` |
| J-REFUSE-02 | peer_implements | PASS refuse | `repack -ad` |
| J-REFUSE-03 | peer_implements | PASS refuse | `submodule status` |
| gix-exit | — | PASS | zero `gix`/`gitoxide` edges |
| Unit Specfold | — | PASS | `spec check` 0/0 at land |

**Timing honesty:** cold workdir per lane; median + nearest-rank p95 over 3 iterations; container scheduling noise is real (see elevated p95 on some arks rows); **no absolute ms pass/fail bars**; **do not** cite as “faster than Git” on public surface. Full step breakdown in `metrics.json`.

## Non-claims

- Not a substitute for G14 / G11 / G15 production bars.
- No live GitHub (G8); no CELIR; no lease HighConcurrency product claims; no SSH/`git://` expansion.
- Absolute perf regression fail gates remain deferred.
- Refuse-closed inventory remains refuse/out-of-catalog.
- Public marketing perf superiority forbidden without this receipt class + [[sot:root/public-surface]] wording.

## Reproduce

```bash
docker compose -f verify/docker-compose.yml run --rm g16
```
