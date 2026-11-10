---
id: sot:root/storage-celir-boundary
title: Storage CELIR boundary
status: active
created: 2026-11-08T08:21:10Z
updated: 2026-11-09T20:28:15Z
tags: [storage, celir, boundary, facade, indexing-service, dual-path, celir-index, enablement]
verify: manual
rel:
  depends_on: [sot:root/product-intent, sot:root/crate-architecture, sot:root/git-compat-surface, sot:root/product-verification-clean-room, sot:root/lease-concurrency-intent, adr:root/20261110052030, sot:root/celir-index-dual-path]
  evidence: [research:root/20261108083640, evidence:root/20261108192100]
---

# Storage CELIR boundary

Version **0.8.1**. Normative **boundary** between arks’s Git-compatible object store and the external CELIR project at the **local** workspace `C:\workspaces\discrete-circuit`. Product intent: [[sot:root/product-intent]]. Crate placement: [[sot:root/crate-architecture]]. Compatibility: [[sot:root/git-compat-surface]]. Lease boundary: [[sot:root/lease-concurrency-intent]]. Enablement ADR: [[adr:root/20261110052030]]. Dual-path companion slice: [[sot:root/celir-index-dual-path]], [[plan:root/20261109202815]]. Rationale research (historical): [[research:root/20261108083640]]. Inbound CELIR response receipt (historical): [[evidence:root/20261108192100]].

## Product decision (normative — v0.8.0)

**Dual-path opt-in:** filesystem `.git` remains the **sole** Git-compat **object store**; CELIR MAY enter the product Cargo graph **only** as an **optional indexing service** behind feature **`celir-index`** (default **OFF**).

1. **Only object-store path for Git-compat:** filesystem `.git` / Git-compat store via `arks-store` (loose/pack/ref as claimed). No alternate CELIR-backed product object store. No silent dual-write of Git objects into CELIR.
2. **Optional Cargo dependency (index-only):** arks **MAY** take path-deps on discrete-circuit crates **`celir-index`** (`packages/celir-index/`) and **`celir-index-runtime`** (`crates/celir-index-runtime/`) behind non-default Cargo feature **`celir-index`** on `arks-store`. Default features / default product builds MUST show **zero** CELIR edges unless the feature is explicitly enabled.
3. **No CELIR ObjectStore implementor:** Do **not** implement `put` / `get` / `exists` (or equivalent) via CELIR. Do **not** add `arks-celir` as a product object-store adapter crate. A thin compile-gated indexing companion module under `arks-store` is allowed ([[adr:root/20261110052030]]).
4. **Role:** CELIR = **indexing service only** — build / publish / open / query of **product-private** index artifacts (sidecar and/or rebuild-from-`.git`). Not Git object / ref / pack / index **store**; not stock-Git OID interchange.
5. **VCS Refuse retained:** CELIR’s **Refuse** of a frozen VCS object-store / put-get-exists Git contract ([[evidence:root/20261108192100]]; indexing-service pack) remains durable for the **object store**.
6. **Multi-writer:** Concurrent writers on filesystem Git are coordinated by **`arks-lease`** ([[sot:root/lease-concurrency-intent]]). CELIR index stores remain single-writer per CELIR norms.
7. **Verification honesty:** Enabling `celir-index` is **not** “CELIR-backed Git verified.” Git-compat product verified claims still require filesystem/stock-Git clean-room ([[sot:root/product-verification-clean-room]]).

**Hard rule:** Do **not** invent CELIR APIs, wire formats, or schema IDs in this repository. Do **not** smuggle CELIR into Rings 0–5 as a silent Git object-store replacement. Do **not** treat dirty WIP on discrete-circuit as freeze-ready API.

**Authority for CELIR facts (when citing):** the **local working tree** at `C:\workspaces\discrete-circuit` (committed + dirty + untracked on disk). That tree currently has **no git remote** — do **not** treat GitHub or any remote clone as source of truth for CELIR capability. Prefer committed consumer-pack paths over chat transcripts.

## Catalog pin (external citation)

Canonical committed capability inventory frozen against:

| Field | Value |
|-------|--------|
| Tree | `C:\workspaces\discrete-circuit` |
| **Pinned HEAD (catalog Current)** | `ef6ca5afdbf8ec95e8ca237ad895a4356ab82689` (2026-11-10) — indexing-service pack; VCS Refuse reconfirmed; export/import gaps honest |
| Pack-bearing intro | `a7c7d0eb73d79dec24b412307611493147627373` — indexing-service reopen precondition body |
| Tip hygiene (retains pack) | `3a5af1ca059da8e8b70212c5720aac1fb41098bb` — catalog/README pin-changelog; **Current** row still `ef6ca5af…` |
| Prior pin (stale) | `b30f5af6ff1131e75ac7b1c4755f97ea01b6deaa` — superseded (historical Refuse receipt) |
| Older prior (stale) | `725c4e0ad9f476616ce6d4385917b9e7c28adbd3` — superseded |
| Branch | `master` |
| Remote | **none** — local-only reference |
| Working tree | May be dirty. Do **not** treat uncommitted WIP as freeze-ready API |
| Inbound receipt (historical) | [[evidence:root/20261108192100]] |
| Enablement | [[adr:root/20261110052030]] |

**Re-verify rule:** Before citing or re-pinning, run `git rev-parse HEAD` in `C:\workspaces\discrete-circuit` and confirm `.agents/consumer/arks/indexing-service.md` exists at the chosen commit. Prefer the catalog **Current** pin or any newer committed tip that retains that file.

## Indexing-service consumer pack (cite)

CELIR authored an **arks** consumer pack under the discrete-circuit tree. Cite these **committed** paths (relative to `C:\workspaces\discrete-circuit`) — not chat:

| Priority | Path | Role |
|----------|------|------|
| P0 | `.agents/consumer/arks/indexing-service.md` | **Primary** indexing-service contract (acceptance A–G; interop table) |
| P0 | `.agents/consumer/arks/capability-catalog.md` | Catalog pin package |
| P0 | `.agents/consumer/arks/refuse-forbidden.md` | Refuse / Forbidden one-pager |
| P1 | `.agents/consumer/arks/concurrency-norms.md` | Single-writer index; arks-lease on FS Git |
| P1 | `.agents/consumer/arks/durability-barriers.md` | Process-exit barriers only |
| Index | `.agents/consumer/arks/README.md` | Pack reading order + cite advice |

### Real packages (do not invent APIs)

| Package / crate | Path under discrete-circuit | Responsibility |
|-----------------|----------------------------|----------------|
| `celir-index` | `packages/celir-index/` | Bounded index formats; single-writer Store / Writer / Snapshot |
| `celir-index-runtime` | `crates/celir-index-runtime/` | Immutable-root composition; publication::build / query::Snapshot |
| `celir-resource-lifecycle` | `packages/celir-resource-lifecycle/` | Owned Scratch lifetimes (supporting; not required for minimal `celir-index` feature) |

### Dual-path honesty (normative split)

| Axis | Meaning |
|------|---------|
| **Enablement** | [[adr:root/20261110052030]] opens optional feature **`celir-index`** (default OFF) |
| **Role** | Indexing service only — product-private index artifacts |
| **Still Refuse (CELIR)** | VCS put/get/exists for Git object/ref/pack; multi-writer CELIR store API; power-loss durability language until evidence |
| **Default product Cargo** | CELIR edges **absent** unless `--features celir-index` (or equivalent) is opted in |
| **Git-compat object store** | Filesystem `.git` sole path — unchanged |

### Export / import honesty (normative — do not invent)

Stock-Git object/ref/pack interchange remains an **arks** responsibility ([[sot:root/git-compat-surface]]). Do **not** invent CELIR Git OID export/import APIs in arks specs or code.

| Capability | Exists today? | What it is | Not |
|------------|---------------|------------|-----|
| CELIR → Git OID / pack / ref export | **No** | Absent | A CELIR product export of Git objects |
| Git OID → CELIR object-store import | **No** | Absent (Refuse put/get/exists) | Silent Git-compat store |
| CELIR **index-root** migration | **Yes (CELIR formats only)** | `import_from` between CELIR store roots (DCMF1/DCMF2) | Git OID mapping; in-place `.git` rewrite |
| Product-private index as **sidecar** beside Git | **arks pattern (M24)** | Non-bare: `<worktree>/.arks/celir-index/` **beside** `.git`; bare: `<bare>/.arks/celir-index/` — outside object/ref/pack meaning | Treating sidecars as Git `objects/` / `refs/` |
| **Rebuild-from-`.git`** | **arks SHOULD** | arks rebuilds product-private indexes from Git-readable content it already owns; CELIR supplies index build/publish/open/query only | CELIR inventing OID enumeration or pack parsers as a VCS API |

**Honest gap:** There is **no** committed CELIR API that exports or imports Git OIDs. Interop with Git content is either (1) **arks-owned rebuild-from-`.git`** into a CELIR index root, or (2) **arks-owned sidecar** index directories that never replace filesystem Git semantics. Closing a richer interchange (if ever) needs arks storage SoT + new CELIR-authored SoT/ADR — not this boundary inventing APIs.

## Rule

### Why this boundary (normative)

1. Product Git-compat work targets the **same `.git` filesystem store** stock Git uses ([[sot:root/git-compat-surface]]). A second **object-store** lineage remains unnecessary and **forbidden** as a silent replace.
2. CELIR **refuses** a frozen VCS product / object-store API — see resolved lock #2, [[evidence:root/20261108192100]], and the indexing-service pack Refuse reconfirm.
3. Optional index-only path-deps (feature `celir-index`, default OFF) let product-private sidecar / rebuild-from-`.git` work proceed without opening a CELIR object store ([[adr:root/20261110052030]]).
4. **Performance second on Git-compat paths.** Claimed Git-compat interchange MUST honor [[sot:root/git-compat-surface]] regardless of any external index-perf research.

### Why vs product status (normative split)

| Axis | Normative meaning |
|------|-------------------|
| **Historical why CELIR was considered** | Index-build performance vs SQLite on qualified local CELIR workloads (cite discrete-circuit evidence; recipe-scoped only) — [[research:root/20261108083640]] |
| **Product status (v0.8.0)** | **Optional dual-path** — FS Git sole object store; `celir-index` feature MAY path-dep index crates (default OFF) |
| **Still forbidden** | Invented CELIR APIs; CELIR ObjectStore; silent replace of filesystem Git object store; CELIR-backed “verified” Git claims without clean-room; inventing Git OID export/import |

### Index vs SQLite — claim limits (research archive)

| Claim class | Allowed statement | Forbidden |
|-------------|-------------------|-----------|
| Build (M0 whole-data) | CELIR construction faster than native SQLite on the **recorded** 70.5M-row recipe (ratio ~0.73765; &lt;0.9 gate); lower final bytes/RSS in that cohort | Universal “CELIR beats SQLite at indexing”; using this as product-store authorization |
| Open / nonempty query | CELIR open validates immutable root and is materially **slower**; most nonempty queries **slower** than SQLite on the same whole-data gates | Claiming open/query superiority; merging cohorts |

Evidence paths (all under `C:\workspaces\discrete-circuit`):

- `crates/celir-index-runtime/.spec/artifacts/initialized-composition-whole-20261026.md` (+ `.json`)
- `packages/celir-index/.spec/artifacts/external-comparison/README.md`
- Roadmap M0 / matrix: `packages/celir-rules/.spec/plans/lightweight-stack-roadmap.md`

### Resolved locks (CELIR → arks; Answered / Refuse)

Historical locks closed by the inbound boundary response at pin `b30f5af6…` ([[evidence:root/20261108192100]]). **Reconfirmed** by the indexing-service pack at pin `ef6ca5af…`. These locks **forbid CELIR as a Git object store**; they do **not** forbid the scoped indexing-service Cargo edge opened by [[adr:root/20261110052030]].

| # | Topic | Status | Normative lock |
|---|-------|--------|----------------|
| 1 | **Consumer facade** | **Refuse** (VCS product freeze) | No CELIR surface is a frozen VCS product API. Indexing-service surfaces are **not** product object-store adoption. |
| 2 | **VCS object-store contract** | **Refuse** (reconfirmed) | CELIR does **not** provide put/get/exists Git object/ref/pack API. Stock-Git OID/pack/ref interchange remains arks [[sot:root/git-compat-surface]] responsibility. |
| 3 | **Durability** | **Answered** (hard limits) | Process-exit scope barriers only on CELIR side; power-loss unqualified — N/A for arks product store (filesystem Git owns durability semantics). |
| 4 | **Concurrency** | **Answered** (CELIR-side) | CELIR single-writer — **N/A for arks product multi-writer**. Product multi-writer on the filesystem Git store is owned by **`arks-lease`** ([[sot:root/lease-concurrency-intent]]). |
| 5–9 | Recovery / ownership / delivery / campaign / catalog | **Answered** | Historical; see [[evidence:root/20261108192100]]; consumer pack recovery-identity / resource-ownership / delivery-wording. |
| 10 | **Indexing-service identity** | **Answered** (enabled dual-path) | Optional dual-path = indexing service only; feature `celir-index` default OFF; not Git store. |
| 11 | **Export / import** | **Answered** (gaps explicit) | No Git OID export/import APIs; sidecar + rebuild-from-`.git` are the honest patterns; CELIR `import_from` = index-root migration only. |
| 12 | **Product Cargo (index-only)** | **Enabled (opt-in)** | Path-deps to `celir-index` + `celir-index-runtime` behind feature `celir-index` ([[adr:root/20261110052030]]). |

### Historical phases (superseded — research archive)

Prior SoT versions (≤0.5.x) described Phase A filesystem-only, Phase B lab path-deps, Phase C clean-room subset readiness, Phase D optional non-compat default. **v0.6.0–v0.7.0** cancelled those phases and **excluded** all Cargo edges. **v0.8.0** re-opens a **scoped** indexing-service edge only (not historical Phase B/C/D object-store readiness).

| Historical phase | Product status now |
|------------------|--------------------|
| **A** | Remains the only product **object-store** behavior: filesystem Git store |
| **B** (lab path-dep / feature) | **Superseded** by v0.8.0 `celir-index` **index-only** opt-in — not a CELIR object-store lab |
| **C** (explicit-subset readiness flip) | **Out of product scope** for object store |
| **D** (optional default) | **Out of product scope** — `celir-index` MUST stay default OFF |

### Readiness status (normative — arks)

| Item | Status | Meaning for arks |
|------|--------|------------------|
| CELIR in product Cargo graph | **Opt-in (`celir-index`)** | Default OFF; path-deps to index crates when enabled |
| CELIR as object backend | **Out of product scope / Refuse** | Filesystem Git store only |
| CELIR VCS object-store API | **Refuse** (external) | No CELIR put/get/exists Git API |
| Indexing-service pack | **Citable + enablement basis** | A–G Done at pin; ADR opens Cargo edge |
| Git OID export/import via CELIR | **Absent / Refuse** | Sidecar or rebuild-from-`.git` only |
| Filesystem Git object store | **Required / sole** | Default and only product backend for Git-compat slices |
| `arks-store` ObjectStore facade | **Filesystem (+ memory test) only** | No CELIR ObjectStore implementor |
| `arks-store` feature `celir-index` | **Allowed (default OFF)** | Indexing companion module / path-deps only |
| Dual-path companion (sidecar / rebuild / query) | **M24 Claimed + G22/G23/G24/G25 Verified** | [[sot:root/celir-index-dual-path]], [[plan:root/20261109203430]], [[evidence:root/20261109203236]], [[evidence:root/20261109204428]], [[evidence:root/20261109205534]], [[evidence:root/20261109205535]], [[evidence:root/20261109205536]] — ObjectStore Refuse retained; honest claim = **CELIR dual-path Git verified** ([[adr:root/20261109204916]]) |
| `arks-celir` object-store adapter | **Not a product crate** | Do not add |
| Indexing-perf rationale | **Research archive** | Does not authorize object-store backends |
| Multi-writer (product) | **`arks-lease` on FS** | CELIR concurrency notes N/A for Git store |

### Facade (arks side)

1. **`arks-store` owns the ObjectStore facade.** Callers depend on store traits — put, get, exists — expressed in arks types ([[sot:root/core-object-model]]). **Filesystem** (+ memory test) backends only.
2. **Optional indexing companion** (feature `celir-index`): compile-gated module MAY link `celir-index` / `celir-index-runtime` for product-private index build/publish/open/query. It MUST NOT implement ObjectStore.
3. **No silent dual-write** of Git objects into CELIR.
4. **Git-compat mode remains filesystem-faithful** for claimed subsets.
5. **Concurrency:** product multi-writer coordination is **`arks-lease` on the filesystem Git store** ([[sot:root/lease-concurrency-intent]]).

### Forbidden

1. Inventing CELIR function signatures, JNI headers, SQL schemas, or graph contracts in arks specs or code.
2. Enabling `celir-index` by default or pulling CELIR into default product builds.
3. Implementing ObjectStore (or equivalent put/get/exists) via CELIR; silent replace of the filesystem Git object store.
4. Claiming “CELIR-backed arks verified” without clean-room filesystem/stock-Git evidence for Git-compat claims.
5. Using CELIR research as a reason to diverge object IDs or pack bytes in Git-compat mode ([[sot:root/git-compat-surface]]).
6. Citing a CELIR **remote** repository as authoritative while the local tree is the working reference for cites.
7. Treating storage gate OPEN or dirty WIP on discrete-circuit as arks API freeze.
8. Inventing CELIR Git OID export/import APIs; treating CELIR `import_from` as Git interop.
9. Smuggling CELIR into Rings 0–5 as Git object/ref/pack meaning.

## Why

arks commits to the filesystem `.git` object store for Git-compat interchange. CELIR refuses a VCS object-store contract. v0.8.0 keeps that Refuse while allowing an **explicit, default-off** indexing-service Cargo edge so sidecar and rebuild-from-`.git` product work can proceed under [[adr:root/20261110052030]] without false “future silent store” drift.

## Verification

1. `spec check` clean for this SoT.
2. Default workspace `cargo metadata` / `cargo check` (no `celir-index` feature) shows **no** required CELIR edges on the default graph.
3. `cargo check -p arks-store --features celir-index` succeeds when discrete-circuit packages exist at the cited paths.
4. `arks-store` ObjectStore backends remain filesystem (+ memory test) only — no CELIR ObjectStore implementor.
5. Product prose that mentions CELIR MUST describe **dual-path opt-in** (index-only; VCS Refuse; FS default) — not silent store replacement or unverified CELIR-backed Git.
6. Multi-writer product claims cite [[sot:root/lease-concurrency-intent]] on the filesystem store — not CELIR concurrency.
7. Pin re-verify: `git rev-parse` equals catalog Current (or newer tip retaining `.agents/consumer/arks/indexing-service.md`); file present at that commit.
8. Export/import prose MUST state OID APIs absent; sidecar / rebuild-from-`.git` only.
