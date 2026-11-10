---
id: evidence:root/20261109065018
title: Clean-room G16 journey-compare Wave-3 path/file:// + optional RSS/CPU
status: active
created: 2026-11-09T06:50:18Z
updated: 2026-11-10T00:57:42Z
tags: [g16, clean-room, journey-compare, timing, metrics, verified, additive, path-clone, rss-cpu]
rel:
  depends_on: [sot:root/clean-room-journey-compare, plan:root/20261108182457, sot:root/arks-remote, sot:root/product-verification-clean-room, sot:root/interop-test-matrix, sot:root/cli-surface, sot:root/public-surface, adr:root/20261108083507, evidence:root/20261108200811]
files:
  - {path: metrics.json, sha256: 318ccf8dcb403e2bdc26c75d95bd59925d7ccf4556870bfbd17faba327513165, bytes: 37568, store: repo}
---

# Clean-room G16 journey-compare receipts (Wave-3 path/`file://` + optional RSS/CPU)

## Context

Additive **G16** Wave-3 close per [[sot:root/clean-room-journey-compare]] **v0.3.0** / [[plan:root/20261108182457]] / [[sot:root/arks-remote]] **v0.2.0**. Prior Wave-2 receipt: [[evidence:root/20261108200811]]. Records side-by-side **functional** outcomes and **median + nearest-rank p95** wall-clock metrics (`arks-journey-metrics/v1`, `aggregation=median_p95`, `iterations=3`) for Wave-1 + Wave-2 + **J-CLONE-PATH-01**, plus optional Linux child **`peak_rss_kb` / `cpu_ms`** when sampled. **Does not** unlock or redefine M9 (G14), M10 (G11), or M11 (G15). Host runs are advisory only.

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
| Tree SHA at run | `814e6a803c9579d81fcf5c412ed855361cfb87f8` |
| Tree SHA (evidence tip) | `f8d047a1e25bfd3bbcc5e58b9c457a4789f78748` |
| Host Docker engine | Linux (`desktop-linux`) |
| `CARGO_BUILD_JOBS` | `2` |

## Results (median wall_ms_total; p95 in parentheses; optional peak_rss_kb)

| Row | git | arks / alias | Notes |
|-----|-----|--------------|-------|
| J-LOCAL-01 | PASS 34 (39) rss≈4600 | PASS 71 (1066) rss≈3084 | same-`.git` peer read OK |
| J-BRANCH-01 | PASS 44 (46) | PASS 175 (298) | |
| J-MERGE-FF-01 | PASS 56 (64) | PASS 187 (195) | `--ff-only` only |
| J-REBASE-01 | PASS 80 (86) | PASS 269 (302) | non-interactive |
| J-BUNDLE-01 | PASS 36 (40) | PASS 64 (65) | packaged-object stand-in |
| J-CLONE-LOCAL-01 | PASS 88 (101) | PASS 108 (142) | in-clean-room smart-HTTP (not G8) |
| J-CLONE-PATH-01 | PASS 58 (59) rss≈4548 | PASS 128 (157) rss≈2792 | path + `file://` local object-dir copy |
| J-ALIAS-01 | — | PASS alias 78 (87) | PATH argv shim |
| J-REFUSE-01 | peer_implements | PASS refuse | `worktree list` |
| J-TAG-01 | PASS 14 (14) | PASS 35 (44) | annotated tag |
| J-STASH-01 | PASS 39 (40) | PASS 65 (72) | push/list/pop |
| J-CHERRY-01 | PASS 26 (35) | PASS 120 (185) | single clean cherry-pick |
| J-DIFF-01 | PASS 18 (20) | PASS 46 (49) | |
| J-RM-MV-01 | PASS 36 (38) | PASS 94 (96) | |
| J-ARCHIVE-01 | PASS 10 (10) | PASS 18 (18) | |
| J-REFLOG-01 | PASS 14 (16) | PASS 25 (27) | |
| J-REVERT-01 | PASS 12 (14) | PASS 35 (41) | |
| J-REFUSE-02 | peer_implements | PASS refuse | `repack -ad` |
| J-REFUSE-03 | peer_implements | PASS refuse | `submodule status` |
| gix-exit | — | PASS | zero `gix`/`gitoxide` edges |
| Unit Specfold | — | PASS | `spec check` at land |

**Timing honesty:** cold workdir per lane; median + nearest-rank p95 over 3 iterations; container scheduling noise is real; **no absolute ms pass/fail bars**; **do not** cite as “faster than Git” on public surface.

**Optional RSS/CPU honesty:** `peak_rss_kb` sampled from Linux `/proc/<pid>/status` `VmHWM` while the child runs. `cpu_ms` from `/proc/<pid>/stat` utime+stime with **CLK_TCK=100** (~10 ms ticks); short steps commonly record **`cpu_ms=0`** (sub-tick) — that is measured, not invented. Fields omitted on venues without `/proc`. Full breakdown in `metrics.json`.

## Non-claims

- Not a substitute for G14 / G11 / G15 production bars.
- No live GitHub (G8); no CELIR; no lease HighConcurrency product claims; no SSH/`git://` expansion.
- Local clone claim is **object-directory copy** (+ pack→loose for checkout) — **not** hardlink/`--shared` alternates and **not** piped `file://` upload-pack.
- Absolute perf regression fail gates remain deferred.
- Refuse-closed inventory remains refuse/out-of-catalog.
- Public marketing perf / RSS superiority forbidden without this receipt class + [[sot:root/public-surface]] wording.

## Reproduce

```bash
docker compose -f verify/docker-compose.yml run --rm g16
```
