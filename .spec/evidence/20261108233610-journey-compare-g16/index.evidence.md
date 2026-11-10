---
id: evidence:root/20261108233610
title: Clean-room G16 journey-compare tip re-run (Wave-1–3 catalog)
status: active
created: 2026-11-08T23:36:10Z
updated: 2026-11-09T22:48:31Z
tags: [g16, clean-room, journey-compare, timing, metrics, verified, additive, tip-rerun]
rel:
  depends_on: [sot:root/clean-room-journey-compare, plan:root/20261108182457, sot:root/arks-remote, sot:root/product-verification-clean-room, sot:root/interop-test-matrix, sot:root/cli-surface, sot:root/public-surface, adr:root/20261108083507, evidence:root/20261109065018]
files:
  - {path: metrics.json, sha256: c4e07ce75a1039107fa83f3488444cad45a032bec151dce00b90171dd1b7f857, bytes: 39157, store: repo}
---

# Clean-room G16 journey-compare receipts (tip re-run)

## Context

Authoritative **G16** tip re-run on current `main` per [[sot:root/clean-room-journey-compare]] **v0.3.0** / [[plan:root/20261108182457]]. Prior Wave-3 close receipt: [[evidence:root/20261109065018]]. Records side-by-side **functional** outcomes and **median + nearest-rank p95** wall-clock metrics (`arks-journey-metrics/v1`, `aggregation=median_p95`, `iterations=3`) for Wave-1 + Wave-2 + Wave-3 catalog rows, plus optional Linux child **`peak_rss_kb` / `cpu_ms`** when sampled. **Does not** unlock or redefine M9 (G14), M10 (G11), or M11 (G15). Host runs are advisory only.

Note: evidence id / `created` use host/container wall-clock UTC at this run (`2026-11-08T23:36:10Z`). The prior Wave-3 evidence id prefix (`20261109065018`) sorts later than this id even though this tip SHA is newer — treat tip SHA + `arks_revision` as authority for “current tip,” not id lexicographic order.

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
| Tree SHA at run | `bea074bacd230b50345cd17b96cb7a3e313eca45` |
| Tree SHA (evidence tip) | `c880cbb1794b628df74c9c28f85f0be34032e13c` |
| Host Docker engine | Linux (`desktop-linux`) |
| `CARGO_BUILD_JOBS` | `2` |

## Results (median wall_ms_total; p95 in parentheses; optional peak_rss_kb)

| Row | git | arks / alias | Notes |
|-----|-----|--------------|-------|
| J-LOCAL-01 | PASS 29 (34) rss≈4564 | PASS 70 (95) rss≈3020 | same-`.git` peer read OK |
| J-BRANCH-01 | PASS 64 (80) | PASS 144 (160) | |
| J-MERGE-FF-01 | PASS 66 (78) | PASS 175 (199) | `--ff-only` only |
| J-REBASE-01 | PASS 121 (126) | PASS 299 (324) | non-interactive |
| J-BUNDLE-01 | PASS 32 (34) | PASS 63 (66) | packaged-object stand-in |
| J-CLONE-LOCAL-01 | PASS 99 (101) | PASS 99 (112) | in-clean-room smart-HTTP (not G8) |
| J-CLONE-PATH-01 | PASS 50 (61) rss≈4604 | PASS 119 (458) rss≈2744 | path + `file://` local object-dir copy |
| J-ALIAS-01 | — | PASS alias 99 (99) | PATH argv shim |
| J-REFUSE-01 | peer_implements | PASS refuse | `worktree list` |
| J-TAG-01 | PASS 16 (23) | PASS 40 (56) | annotated tag |
| J-STASH-01 | PASS 43 (56) | PASS 60 (66) | push/list/pop |
| J-CHERRY-01 | PASS 26 (34) | PASS 101 (108) | single clean cherry-pick |
| J-DIFF-01 | PASS 18 (26) | PASS 49 (55) | |
| J-RM-MV-01 | PASS 34 (38) | PASS 94 (107) | |
| J-ARCHIVE-01 | PASS 12 (12) | PASS 18 (26) | |
| J-REFLOG-01 | PASS 16 (147) | PASS 29 (32) | |
| J-REVERT-01 | PASS 10 (12) | PASS 39 (39) | |
| J-REFUSE-02 | peer_implements | PASS refuse | `repack -ad` |
| J-REFUSE-03 | peer_implements | PASS refuse | `submodule status` |
| gix-exit | — | PASS | zero `gix`/`gitoxide` edges |

**Timing honesty:** cold workdir per lane; median + nearest-rank p95 over 3 iterations; container scheduling noise is real; **no absolute ms pass/fail bars**; **do not** cite as “faster than Git” on public surface. J-CLONE-PATH-01 arks p95 (458) shows iteration skew vs median (119) — record-only.

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
