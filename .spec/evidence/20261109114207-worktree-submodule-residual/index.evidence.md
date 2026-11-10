---
id: evidence:root/20261109114207
title: Post-M9 residual worktree + submodule implement (G16 + G14)
status: active
created: 2026-11-09T11:42:07Z
updated: 2026-11-09T22:48:32Z
tags: [g16, g14, clean-room, worktree, submodule, residual, verified]
rel:
  depends_on: [plan:root/20261109022956, sot:root/production-residual-register, sot:root/clean-room-journey-compare, sot:root/cli-flag-parity-m9, sot:root/cli-surface, sot:root/arks-repo, sot:root/arks, sot:root/product-verification-clean-room, sot:root/interop-test-matrix, adr:root/20261108083507, evidence:root/20261108150000, evidence:root/20261109065018]
  evidence: []
files:
  - {path: metrics.json, sha256: 5c66ef5a0ca26612de07da02c87a26df6716a3219921b1f7a9a8157f50825e46, bytes: 42652, store: repo}
---

# Post-M9 residual — `worktree` + `submodule` implement receipts

## Context

Closes residual client inventory for **`worktree`** and **`submodule`** per [[plan:root/20261109022956]] without reopening M9/M10/M11 exit bars. Historical M9 refuse for these commands ([[evidence:root/20261108150000]]) remains the frozen M9 ledger; this receipt is the residual implement close. G16 catalog **v0.4.0**: **J-WORKTREE-01** / **J-SUBMODULE-01** replace retired **J-REFUSE-01** / **J-REFUSE-03**. **J-REFUSE-02** (`repack`) stays refuse-closed.

Attached metrics: `metrics.json` (G16 clean-room `arks-journey-metrics/v1`, `aggregation=median_p95`, `iterations=3`).

## Environment

| Field | Value |
|-------|-------|
| Compose | `verify/docker-compose.yml` services `g16` + `g14` |
| Entrypoints | `verify/scripts/run-g16.sh`, `verify/scripts/run-g14.sh` |
| Image | `arks-verify-m1:0.1.0` |
| Image digest (lock / local Id) | `sha256:bab37bd1ea03832a5ca30aa41e598eafc8bef7a5b1026d65acefb75e3f4a73c1` |
| Stock Git (in-image) | `git version 2.39.5` |
| rustc (in-image) | `rustc 1.99.1 (58cf8f9c3 2026-10-13)` |
| `arks` profile (G16) | **release** |
| Iterations / aggregation | **3** / `median_p95` |
| Tree SHA at G16 run | working tree mounted (pre-land); tip-pin below |
| Tree SHA (evidence tip) | `01c1d34151b5bc5157c00b01cf7892406fa20e89` |
| Host Docker engine | Linux (`desktop-linux`) |
| `CARGO_BUILD_JOBS` | `2` |

## G16 functional results (claimed residual rows)

| Row | git | arks / alias | Notes |
|-----|-----|--------------|-------|
| J-WORKTREE-01 | PASS | PASS | `add`/`list`/`move`/`remove`/`prune`; locked remove fail-closed |
| J-SUBMODULE-01 | PASS | PASS | local `add`/`status`/`deinit`/`init`/`update`; gitlink + `.gitmodules` |
| J-REFUSE-02 | peer_implements | PASS refuse (+ alias) | `repack -ad` remains refuse |
| Prior catalog (Wave-1/2/3) | PASS | PASS | No absolute-ms / ≥20% bars |

## G14 residual honesty

| Check | Outcome |
|-------|---------|
| Help lists `worktree` / `submodule` | PASS |
| Remaining refuse-closed omitted | PASS |
| Alias shim claimed list/status | PASS |
| Alias refuse `repack` / `range-diff` | PASS |
| Embedded G13/G12/G7/G6 (+ prior) regression | PASS |
| gix-exit | PASS |
| `spec check` | PASS (0 errors / 0 warnings) |

## Claimed subcommand matrix (pin Git 2.39.x)

### worktree
`add` (`-b`), `list`, `remove` (`-f`), `prune`, `move` — refuse `--porcelain`/`lock`/`unlock`/`--detach`/`--orphan`.

### submodule
`add` (local/`file://`), `init`, `update` (`--init`), `status`, `deinit` (`-f`) — refuse `--recursive`/`--remote`/non-local add.

## Non-claims

- Does **not** redefine M9/M10/M11 production exits.
- No absolute-ms / ≥20% performance bars; timings record-only.
- Nested submodule `--recursive`, SSH submodule URLs, worktree porcelain remain out.
- Zero `gix`; CELIR out of product deps; lease lock rules unchanged.

## Reproduce

```bash
docker compose -f verify/docker-compose.yml run --rm g16
docker compose -f verify/docker-compose.yml run --rm g14
spec check
```
