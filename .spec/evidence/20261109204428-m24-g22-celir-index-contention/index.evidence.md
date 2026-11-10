---
id: evidence:root/20261109204428
title: M24 G22 CELIR index contention clean-room
status: active
created: 2026-11-09T20:44:28Z
updated: 2026-11-09T22:48:40Z
tags: [m24, g22, celir, indexing-service, clean-room, contention, verified, dual-path]
rel:
  depends_on: [sot:root/celir-index-contention-clean-room, sot:root/celir-index-dual-path, plan:root/20261109203949, plan:root/20261109203430, sot:root/storage-celir-boundary, adr:root/20261110052030, sot:root/lease-concurrency-intent, sot:root/product-verification-clean-room, sot:root/interop-test-matrix, sot:root/program-roadmap, sot:root/production-residual-register, evidence:root/20261109203236]
files:
  - {path: receipt.json, sha256: 40716219d1dfe94093fb74a6414ef455e2776b8b8d5132946051140baea071a7, bytes: 334, store: repo}
---

# M24 — G22 CELIR index contention clean-room

## Authority

| Field | Value |
|-------|--------|
| Gate | **G22** (`verify/scripts/run-g22.sh` / compose `g22`) |
| Matrix | `verify/matrices/matrix-g22.md` |
| Milestone | **M24** index-contention verified (clean-room) |
| Stock git (image) | `git version 2.39.5` |
| rustc (image) | `1.99.1` |
| N scale | **32** (tens) + **128** (hundreds) |
| Result | **PASS** |
| discrete-circuit | `/workspaces/discrete-circuit` (sibling mount) |
| Parent tip before wave | `552da8f` |

## Suite

| Step | Result |
|------|--------|
| `cargo test -p arks-lease --features celir-index-stress --test g22_index_contention` | 5/5 PASS |
| `cargo test -p arks-store --features celir-index` | 18/18 PASS |
| Default `arks-store` tree CELIR-free | OK |
| Sole-runner lock | held |

## Assertions held

1. Index open/query integrity after N=32/128 readonly contention.
2. Concurrent rebuild without index lease fail-closed; leased rebuild restores Present.
3. Shard writers + index readers; stock `git cat-file` readable.
4. ObjectStore remains filesystem; CELIR index-only.
5. No fake PASS without discrete-circuit packages.

## Honesty

- **Index-contention verified (G22)** — not “CELIR-backed Git verified.”
- G11-L3 shared-index N-way remains OUT.
- Host advisory [[evidence:root/20261109203236]] remains historical Claimed evidence for feature ship.
