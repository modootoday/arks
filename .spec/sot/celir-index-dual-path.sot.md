---
id: sot:root/celir-index-dual-path
title: CELIR index dual-path companion
status: active
created: 2026-11-09T20:28:15Z
updated: 2026-11-09T20:45:00Z
tags: [celir, indexing-service, dual-path, sidecar, rebuild, query, celir-index, m24, g022-celir-index-contention]
verify: clean-room
rel:
  governs: [crates/arks-store]
  depends_on:
    - sot:root/storage-celir-boundary
    - adr:root/20261110052030
    - sot:root/arks-store
    - sot:root/lease-concurrency-intent
    - sot:root/product-verification-clean-room
    - sot:root/crate-architecture
    - sot:root/program-roadmap
    - sot:root/celir-index-contention-clean-room
  evidence: [evidence:root/20261109203236, evidence:root/20261109204428]
---

# CELIR index dual-path companion

Version **0.2.0**. Normative product slice for program milestone **M24** and the optional `arks-store` feature **`celir-index`** (default **OFF**): product-private sidecar **beside `.git`**, arks-owned **rebuild-from-`.git`**, and build → publish → open → query wiring that cites real discrete-circuit APIs only. Boundary: [[sot:root/storage-celir-boundary]]. Enablement: [[adr:root/20261110052030]]. Milestone plan: [[plan:root/20261109203430]]. Prior companion waves folded: [[plan:root/20261109202815]].

## Scope

| In | Out |
|----|-----|
| Compile-gated companion under `arks-store` (`--features celir-index`) | CELIR as ObjectStore / put / get / exists |
| Sidecar layout under git-dir product-private path | Silent dual-write of Git objects into CELIR |
| Rebuild-from-`.git` into a CELIR index root (arks enumerates; CELIR indexes) | Invented CELIR Git OID export/import APIs |
| Build / publish / open / query via `celir-index` + `celir-index-runtime` | Default-on feature; Rings 0–5 silent replace |
| Host unit tests (advisory) for feature-on + feature-off green | Bare “CELIR-backed Git verified” without dual-path body / G23 receipt |
| Clean-room **G22** index-contention under leases (tens→hundreds) | Extending G11-L3 to CELIR / shared-index N-way |

## Rule

### Feature lock

1. Feature name **`celir-index`** on `arks-store`; default features MUST stay empty for this flag ([[adr:root/20261110052030]]).
2. Companion module MUST NOT implement [`ObjectStore`](crates/arks-store) `put` / `get` / `exists`.
3. Path-deps allowed: `celir-index`, `celir-index-runtime`, and (when needed for `Corpus`) `celir-data` — all optional, feature-gated. Cite discrete-circuit consumer pack `.agents/consumer/arks/indexing-service.md` at catalog pin (or newer tip retaining that file).

### Sidecar layout (product-private)

1. **Non-bare:** sidecar root MUST resolve to **`<worktree>/.arks/celir-index/`** — **beside** `.git`, not under it.
2. **Bare:** sidecar root MUST resolve to **`<bare-root>/.arks/celir-index/`** (product-private under the bare root; never under `objects/` / `refs/` / pack).
3. That path is **arks product-private metadata**. It MUST NOT be treated as Git `objects/`, `refs/`, `packed-refs`, `index`, or pack meaning ([[sot:root/git-compat-surface]]).
4. Opening / publishing a sidecar MUST use CELIR store APIs against that directory only — never rewrite Git object bytes in place.

### Rebuild-from-`.git` (arks-owned)

1. arks MUST enumerate Git-readable content it already owns (loose + pack-resolved objects via `FilesystemGitStore` / objects dir walk) and project rows into a CELIR-admitted corpus or JSONL source.
2. CELIR supplies index **build / publish / open / query** only — arks MUST NOT invent OID export/import on the CELIR side.
3. Claimed rebuild schema (product-private, not a Git format): columns `oid`, `kind`; effect = decimal UTF-8 byte length of the object payload. Keys are exact string tuples for CELIR lookup.
4. Row / entry budgets MUST fail closed at CELIR `StoreLimits` / corpus limits — do not silently truncate without an error or explicit capped-rebuild API returning the applied cap.
5. Multi-writer coordination for concurrent Git writers remains **`arks-lease`** on the filesystem store ([[sot:root/lease-concurrency-intent]]). CELIR index roots stay single-writer.

### Query surface

1. After publish, callers MAY open a readonly CELIR `Store` / `Snapshot` (or runtime `Snapshot`) on the sidecar root and perform exact lookup / bounded scan per CELIR APIs.
2. Query results are **index hints** over product-private rows — not proof of Git object store membership and not a substitute for `ObjectStore::exists` / `get`.

### Build → publish → open → query wiring

| Step | Real API (cite; do not invent) | Package |
|------|--------------------------------|---------|
| Create / open writer | `celir_index::Writer::create` / `open` | `celir-index` |
| Publish snapshot | `Writer::commit_snapshot` (`celir_data::Corpus`) | `celir-index` + `celir-data` |
| Open readonly | `celir_index::Store::open_readonly` → `snapshot` | `celir-index` |
| Lookup | `Snapshot::lookup` | `celir-index` |
| Immutable-root build (optional JSONL path) | `celir_index_runtime::build` / `publication` | `celir-index-runtime` |
| Runtime open / query | `celir_index_runtime::Snapshot::open` + `query` | `celir-index-runtime` |

### Honesty / verification

1. Enabling `celir-index` alone is **not** Verified. Prefer the honest name **CELIR dual-path Git verified** ([[adr:root/20261109204916]]); bare “CELIR-backed Git” without dual-path body is forbidden.
2. Git-compat product verified claims still require filesystem/stock-Git clean-room ([[sot:root/product-verification-clean-room]]).
3. Companion **Claimed** (feature ship) completion authority remains: SoT+plan Done rules + host `cargo check`/`test` both modes + `spec check` — labeled **advisory** for contention / product load.
4. **Index-contention verified** requires Docker clean-room gate **G22** per [[sot:root/celir-index-contention-clean-room]] (N=32 and N=128 tiers). Host greens MUST NOT authorize contention-verified language.
5. **CELIR dual-path Git verified** requires **G23** ([[sot:root/celir-dual-path-git-verified]]). Scale **G25**; L3+index joint **G24**.
6. Prose and evidence MUST say **advisory (host)** vs **verified (clean-room G22/G23/G24/G25)** explicitly.

### Forbidden

1. ObjectStore via CELIR; default-on `celir-index`.
2. Invented CELIR signatures / schemas / Git OID export-import.
3. Treating sidecar paths as Git interchange.
4. Claiming CELIR multi-writer or power-loss durability for Git commits.

## Why

ADR enablement opened Cargo edges; remaining product work is the honest dual-path: keep FS `.git` as sole object store while allowing opt-in product-private indexes built from content arks already reads.

## Verification

1. `spec check` clean for this SoT and [[plan:root/20261109202815]].
2. `cargo check -p arks-store` (default) — no required CELIR edge.
3. `cargo check -p arks-store --features celir-index` — succeeds with discrete-circuit present.
4. `cargo test -p arks-store` and `cargo test -p arks-store --features celir-index` — green (host advisory).
5. Companion APIs do not implement ObjectStore; non-bare sidecar path stays under `<worktree>/.arks/celir-index/` (beside `.git`).
6. Evidence / residual language records advisory vs verified split; milestone identity is **M24**.
7. Clean-room G22 PASS receipt required before residual/roadmap may say index-contention **verified**.
