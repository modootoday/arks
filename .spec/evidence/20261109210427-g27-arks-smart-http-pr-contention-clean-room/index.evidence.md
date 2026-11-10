---
id: evidence:root/20261109210427
title: G27 arks smart HTTP PR contention clean-room
status: active
created: 2026-11-09T21:04:27Z
updated: 2026-11-10T00:57:44Z
tags: [g27, clean-room, smart-http, server, branch, pr-sim, contention, verified]
rel:
  depends_on: [sot:root/arks-smart-http-pr-contention-clean-room, plan:root/20261109210427, sot:root/product-verification-clean-room, sot:root/interop-test-matrix, sot:root/arks-server, sot:root/program-roadmap, sot:root/production-residual-register]
files:
  - {path: receipt.json, sha256: a090cd99d30f139a62c3d1b3d7d7cc5c44da1c5680553df061a064751df4bbf6, bytes: 376, store: repo}
  - {path: scale.json, sha256: 0ab5e566000144b8e1b7c5546059958744df5995ff57dc5d68bc5419b4090472, bytes: 58, store: repo}
---

# G27 — arks smart HTTP + synthetic PR contention clean-room

## Authority

| Field | Value |
|-------|--------|
| Gate | **G27** (`verify/scripts/run-g27.sh` / compose `g27`) |
| Matrix | `verify/matrices/matrix-g27.md` |
| Stock git (image) | `git version 2.39.5` |
| rustc (image) | `1.99.1` |
| N scale | **2048** branches + **2048** PR heads + **8** concurrent agents |
| Result | **PASS** |
| Parent tip before wave | `bbd1573` |

## Suite

| Step | Result |
|------|--------|
| `cargo build -p arks` | OK |
| `cargo test --manifest-path verify/gates/g27/Cargo.toml -- --test-threads=1` | 5/5 PASS |
| Sole-runner lock | held |
| Heavy-gate overlap refuse | OK (no G17/G19/G22–G26 lock) |

## Assertions held

1. Seed scale N=2048/2048 visible via `git ls-remote` against arks smart HTTP.
2. Concurrent stock `git clone` N=8 succeed.
3. Concurrent `git fetch` N=8 after tip advance succeed.
4. Concurrent distinct-ref pushes N=8 succeed; server refs agree.
5. Contended same-ref push fail-closed (≤1 win); tip stock-resolvable.

## Server fix exercised

Stock git/curl **gzip-compresses** upload-pack POST bodies once advertisements grow (~32+ refs). `arks serve-http` now decompresses `Content-Encoding: gzip` / gzip-magic request bodies (plus HTTP `100 Continue`).

## Honesty

- **arks smart-HTTP branch + synthetic PR-env contention verified (G27)** — **not** GitHub PR UX, Checks, merge-queue, rate limit, or live forge throughput.
- Live forge latency remains **Deferred** (G8+ measurement).
- SSH / `git://` / dumb HTTP / protocol v2 Remain Refuse.
