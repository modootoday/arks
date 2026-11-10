---
id: sot:root/l3-celir-index-joint-clean-room
title: L3 HighConcurrency plus CELIR index joint clean-room
status: active
created: 2026-11-09T20:49:15Z
updated: 2026-11-10T05:49:00Z
tags: [celir, indexing-service, l3, highconcurrency, g24, m24, m23, clean-room, joint, lease]
verify: clean-room
rel:
  governs: [verify/g24, verify/scripts/run-g24.sh, verify/matrix-g24.md]
  depends_on:
    - sot:root/lease-concurrency-intent
    - adr:root/20261109194200
    - sot:root/celir-index-dual-path
    - sot:root/celir-index-contention-clean-room
    - sot:root/storage-celir-boundary
    - sot:root/product-verification-clean-room
    - adr:root/20261109204916
    - plan:root/20261109204935
  evidence: [evidence:root/20261109205535]
---

# L3 HighConcurrency plus CELIR index joint clean-room

Version **0.1.0**. Normative Docker clean-room gate **G24** proving **WriterMode::HighConcurrency** claimed-shape path and CELIR index sidecar **together**. L3 ADR: [[adr:root/20261109194200]]. Index companion: [[sot:root/celir-index-dual-path]]. G22 parent: [[sot:root/celir-index-contention-clean-room]].

## Scope

| In | Out |
|----|-----|
| Gate **G24** (`verify/scripts/run-g24.sh` / compose `g24`) | Shared-index N-way (remains OUT) |
| Explicit `WriterMode::HighConcurrency` + `prep_hash_parallel` + `apply_batch` shard storm | Redefining G11-L3 as CELIR-bearing |
| Concurrent CELIR index readers / leased rebuild on same FS `.git` | lease ≠ skip Git locks |
| N=32 required; N=128 preferred | CELIR ObjectStore; default-on feature |
| Stock-readable `.git`; ObjectStore Refuse | Fake PASS without discrete-circuit |

## Rule

### Gate identity

1. Gate ID **G24** `l3-celir-index-joint`. Matrix: `verify/matrix-g24.md`.
2. G11-L3 (`verify/matrix-l3.md`) stays CELIR-free. G24 is the **successor joint** gate — additive.
3. Host greens MUST NOT authorize joint Verified.

### Assertions (PASS)

1. **HC valve** — `WriterSession::new(WriterMode::HighConcurrency)` succeeds (opt-in; not default).
2. **Shard apply** — N=32 (required) distinct `refs/heads/agent-*` tips via `apply_batch`; tips_verified; leases empty after.
3. **Index together** — published sidecar open/query remains consistent during/after HC apply; leased rebuild restores Present.
4. **Locks still taken** — competing `*.lock` on a shard still fails Pipeline(LockFailed); lease released (lease ≠ skip locks).
5. **Stock Git** — `git cat-file` / `show-ref` readable for applied OIDs.
6. **ObjectStore Refuse** — CELIR index-only; FS sole store.
7. **Sole runner** — exclusive lock; no overlap with heavy catalogs.

### Honesty

G24 PASS = **L3 + CELIR index joint verified**. Does not reopen shared-index N-way. Does not claim CELIR ObjectStore. Does not redefine G11-L3.

## Why

G22 and G11-L3 were separate. Product multi-agent reality uses HighConcurrency writers and optional CELIR indexes on the same FS `.git`; without a joint clean-room gate, co-existence is folklore.

## Verification

1. `docker compose -f verify/docker-compose.yml run --rm g24` PASS.
2. Evidence receipt under `.spec/evidence/`.
3. `spec check` clean for this SoT and [[plan:root/20261109204935]].
