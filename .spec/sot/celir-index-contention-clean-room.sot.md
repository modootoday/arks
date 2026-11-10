---
id: sot:root/celir-index-contention-clean-room
title: CELIR index contention clean-room
status: active
created: 2026-11-09T20:39:49Z
updated: 2026-11-09T20:45:00Z
tags: [celir, indexing-service, clean-room, contention, g22, m24, lease, dual-path]
verify: clean-room
rel:
  governs: [verify/gates/g22, verify/scripts/run-g22.sh, verify/matrices/matrix-g22.md]
  depends_on:
    - sot:root/celir-index-dual-path
    - sot:root/storage-celir-boundary
    - adr:root/20261110052030
    - sot:root/lease-concurrency-intent
    - sot:root/product-verification-clean-room
    - sot:root/interop-test-matrix
    - sot:root/program-roadmap
    - plan:root/20261109203430
    - plan:root/20261109203949
  evidence: [evidence:root/20261109204428]
---

# CELIR index contention clean-room

Version **0.1.0**. Normative **Docker clean-room** gate **G22** for optional feature `celir-index` under ultra-high agent contention. Companion unit SoT: [[sot:root/celir-index-dual-path]]. Boundary: [[sot:root/storage-celir-boundary]]. Lease: [[sot:root/lease-concurrency-intent]]. Authority: [[sot:root/product-verification-clean-room]]. Milestone: **M24** ([[plan:root/20261109203430]]); staged plan: [[plan:root/20261109203949]].

## Scope

| In | Out |
|----|-----|
| Clean-room **G22** (`verify/scripts/run-g22.sh` / compose `g22`) | Extending **G11-L3** to CELIR (G11-L3 stays CELIR-free / shared-index N-way OUT) |
| N-way concurrent agents (tens→hundreds) on FS `.git` + leases + CELIR index build/publish/open/query / rebuild-from-`.git` | CELIR as ObjectStore; default-on `celir-index` |
| Index integrity after contention; fail-closed publication; stock-Git readability of `.git` | Shared-index N-way Git index claim (lease L3 OUT) |
| discrete-circuit path-deps mounted as sibling tree in clean-room | Fake PASS without CELIR crates / invented digests |

## Rule

### Gate identity

1. Gate ID **G22** `celir-index-contention`. Matrix: `verify/matrices/matrix-g22.md`. Entrypoint: `verify/scripts/run-g22.sh`. Compose service: `g22`.
2. G22 is **additive** to M24 Claimed (host advisory). Host greens alone MUST NOT authorize “index contention verified.”
3. G22 MUST NOT reopen or redefine G11-L3 claimed shape; CELIR remains excluded from G11-L3 (`verify/matrices/matrix-l3.md`).

### Topology (N scale)

| Tier | N agents | Required for Verified |
|------|----------:|-----------------------|
| Smoke | 8 | Host advisory OK |
| Stress-tens | 32 | Clean-room required |
| Stress-hundreds | 128 | Clean-room required |

Agents coordinate FS Git writers via **`arks-lease`** (ref-shard / `index:` scope). CELIR index roots remain **single-writer** per CELIR norms; concurrent publishers MUST serialize on `index:celir-index` (or fail closed).

### Assertions (PASS criteria)

1. **Index integrity** — after contention settles, `open_sidecar` + exact `lookup_oid_kind` returns the published effect for seeded `(oid, kind)` rows; no panic; no silent wrong Present.
2. **No torn publication** — concurrent rebuild/publish without the index lease yields `Err` or at most one consistent winner; a subsequent leased rebuild leaves a readable snapshot. Never a green lookup with conflicting/torn effects treated as success.
3. **No silent ObjectStore** — CELIR path does not implement `ObjectStore` put/get/exists; FS `.git` remains sole object store ([[adr:root/20261110052030]]).
4. **Stock-readable `.git`** — in-image stock `git` can `cat-file` / resolve objects written during the storm.
5. **Fail-closed** — lock / budget / lease conflicts surface as errors; leases release on failure paths exercised by the suite.
6. **Sole runner** — gate holds an exclusive lock dir (same OOM lesson as G19); MUST NOT overlap heavy G17/G19 catalogs.

### Clean-room discrete-circuit mount

1. Product path-deps resolve `../../../discrete-circuit/{packages,crates}/…` from `crates/arks-store` ([[adr:root/20261110052030]]).
2. Compose service `g22` MUST mount the host sibling tree so that from `/workspaces/gir/crates/arks-store` the relative path reaches `/workspaces/discrete-circuit/…` (working_dir `/workspaces/gir`).
3. If the mount or packages are absent, the gate MUST **FAIL** (or refuse closed) — do **not** stub CELIR or fake PASS.
4. Default product graph (no `celir-index`) remains CELIR-edge free; G22 enables the feature only inside its test invocation.

### Honesty

1. G22 PASS = **index-contention verified (clean-room)** for the claimed dual-path companion under load — **not** “CELIR-backed Git verified.”
2. M24 host advisory evidence remains historical; Verified language for this gate requires a G22 receipt under `.spec/evidence/`.

## Why

M24 shipped sidecar / rebuild / query with host-advisory tests only. Product verification authority is Docker clean-room. Ultra-high concurrency is the realistic multi-agent load for lease + single-writer index publication; without G22, “works under contention” is folklore.

## Verification

1. `docker compose -f verify/docker-compose.yml run --rm g22` PASS.
2. Receipt stamped under `.spec/evidence/` with gate, N scale, stock git/rustc, tip SHA.
3. `spec check` clean for this SoT, [[plan:root/20261109203949]], and matrix cross-links.
4. Residual / roadmap record advisory → verified for **this gate** only when receipt PASS.
