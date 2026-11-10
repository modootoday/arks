---
id: sot:root/crate-architecture
title: Crate architecture
status: active
created: 2026-11-08T08:21:10Z
updated: 2026-11-10T05:20:30Z
tags: [crates, architecture, rust, bottom-up, engine]
verify: manual
rel:
  depends_on: [sot:root/product-intent, sot:root/product-naming, sot:root/git-compat-surface, adr:root/20261108083507, adr:root/20261109122126, adr:root/20261110052030, sot:root/storage-celir-boundary, sot:root/lease-concurrency-intent, sot:root/remote-host-compatibility-surface]
  evidence: [research:root/20261108083502]
  governs: [crates, crates/arks-hash, crates/arks-oid, crates/arks-object, crates/arks-store, crates/arks-refs, crates/arks-index, crates/arks-repo, crates/arks-pack, crates/arks-protocol, crates/arks-remote, crates/arks-lease, crates/arks-server, crates/arks]
---

# Crate architecture

Version 0.2.10. Normative **workspace crate graph** and **inner → outer dependency rule** for arks. Product name / primary crate / bin: [[sot:root/product-naming]]. Build principle: [[sot:root/product-intent]]. Compatibility envelope: [[sot:root/git-compat-surface]]. Native engine + zero-`gix` end state: [[adr:root/20261108083507]]. CELIR dual-path (index-only opt-in): [[sot:root/storage-celir-boundary]], [[adr:root/20261110052030]].

Convention affinity (cite, do not fork): humanymous crate-architecture patterns at `C:\workspaces\humanymous\.spec\sot\crate-architecture.sot.md` — kebab-case packages, acyclic responsibility graph, no artificial fragmentation. Cross-cutting SRP / flat-list / context-dir / language-standard layout rules: [[sot:root/structural-conventions]].

Workspace members present: `crates/arks-hash`, `crates/arks-oid`, `crates/arks-object` (M1), `crates/arks-store` (M2), `crates/arks-refs`, `crates/arks-index`, `crates/arks-repo` (M3), `crates/arks-pack`, `crates/arks-protocol` (M4), `crates/arks-remote` (M6), `crates/arks-lease` (M10), `crates/arks-server` (M11), `crates/arks` (M5–M9 CLI + Wave Sv). Unit SoTs: [[sot:root/arks-hash]], [[sot:root/arks-oid]], [[sot:root/arks-object]], [[sot:root/arks-store]], [[sot:root/arks-refs]], [[sot:root/arks-index]], [[sot:root/arks-repo]], [[sot:root/arks-pack]], [[sot:root/arks-protocol]], [[sot:root/arks-remote]], [[sot:root/arks-lease]], [[sot:root/arks-server]], [[sot:root/arks]].

## Rule

### Engine ownership (normative)

1. **Engine crates own parsing and IO.** Rings 0–5 implement Git formats/protocols **in-process** in Rust. They MUST NOT require the stock `git` binary to hash, read/write objects, update refs, read/write index/pack, or speak claimed protocol subsets ([[adr:root/20261108083507]]).
2. **Stock `git` is oracle-only.** The `git` executable MAY be invoked from clean-room acceptance, fixture generators, or explicitly labeled developer tooling — never as the architecture for product core ops.
3. **No thin-wrapper architecture.** A CLI that primarily shells out to `git` for claimed features fails [[sot:root/product-intent]] and this SoT.
4. **Concurrency / memory layering.** Lease runtime stays in Ring 6 (`arks-lease`). Allocator / arena / pool **policy** for hot paths is an engine concern (principles in [[research:root/20261108083502]]); shared helpers MUST NOT create outer→inner cycles or invent CELIR APIs / path-deps. Allocator helper **identity** is **Closed** as **ring-internal modules** (no dedicated `arks-alloc` crate) per [[adr:root/20261109122126]] until a future ADR reopens placement with an L3 (or other measured) claim.
5. **`gix` / gitoxide — zero end state.** Rings **0–4** runtime, `arks-lease`, allocator / arena policy used on product paths, and the high-concurrency commit pipeline MUST be **in-house** and MUST NOT depend on `gix`/gitoxide for hashing, OID types, loose object IO, refs, store facade, or commit orchestration ([[adr:root/20261108083507]]). **End state:** the workspace MUST have **no** Cargo dependency on `gix` / gitoxide — not as a permanent optional feature and not as a permanent `dev-dependencies` design. Prefer never introducing `gix`. Transitional cold `gix` MAY appear only if labeled transitional, feature-gated / temporary `dev-dependencies`, non-exported from Rings 0–4, and **removed before the gix-exit gate (before M4 exit)** per [[adr:root/20261108083507]] and [[plan:root/20261108085816]]. Stock Git remains the only verification oracle — `gix` is never a substitute for product “verified” claims.

### Dependency law (normative)

1. **Inner → outer only.** A crate MAY depend on equal-or-inner crates. A crate MUST NOT depend on a strictly outer crate.
2. **Core never depends on CLI or remote.** Libraries under the Core and Protocol rings MUST NOT depend on `arks` (bin/CLI package), porcelain UX crates, or remote-host product API crates.
3. **No cycles** among workspace members (runtime edges). Dev-only test utilities MAY depend outward for fixtures but MUST NOT create a runtime cycle.
4. **Facades over backends.** The product object-store backend is the **filesystem Git store** behind `arks-store` ([[sot:root/storage-celir-boundary]]). Core object/ref code depends on the facade. **MUST NOT** implement ObjectStore via CELIR. Optional non-default feature **`celir-index`** on `arks-store` MAY path-dep `celir-index` / `celir-index-runtime` for product-private indexing only ([[adr:root/20261110052030]]). Do **not** add an `arks-celir` object-store adapter crate.
5. **One responsibility per crate.** Do not invent a new `arks-*` package for every helper function; split only when dependency direction, target (lib vs bin), or consumer isolation requires it.
6. **No `git` substring** in crate/package/bin names ([[sot:root/product-naming]]).

### Rings (inner → outer)

| Ring | Role | May depend on |
|------|------|---------------|
| **0 Identity** | Hash + OID types | External hash crates only |
| **1 Objects** | Loose object encode/decode; content model | Ring 0 |
| **2 Pack / index artifacts** | Pack, idx, optional rev/MIDX readers (writers phased) | Rings 0–1 |
| **3 Refs** | Refname validation, loose/packed refs (files backend first) | Rings 0–1 |
| **4 Repo / store facade** | Layout discovery, object store trait, filesystem backend | Rings 0–3 |
| **5 Protocol** | pkt-line, capability strings, pack protocol helpers | Rings 0–4 as needed |
| **6 Remotes / server / lease** | Remote client, Git server transports, lease runtime | Rings 0–5 |
| **7 CLI / porcelain** | `arks` bin, user-facing commands | Any inner ring; **sibling UX SoT owns command shape** |

### Proposed crate names

| Crate | Ring | Responsibility |
|-------|------|----------------|
| `arks-hash` | 0 | Object-format hashing (SHA-1 / SHA-256 per repo object format); no I/O |
| `arks-oid` | 0 | OID newtype, hex parse/format, compare |
| `arks-object` | 1 | Loose object header + payload types (`blob`/`tree`/`commit`/`tag`); zlib stream codec |
| `arks-pack` | 2 | Pack + idx (+ phased `.rev`/MIDX) decode; encode later |
| `arks-index` | 2 | Index file (gitformat-index) read/write for claimed versions |
| `arks-refs` | 3 | Refname rules, loose refs, `packed-refs`, HEAD/symref; reftable later if claimed |
| `arks-store` | 4 | Object-store **facade** trait; filesystem Git backend only; optional non-default feature `celir-index` (indexing companion, not ObjectStore) |
| `arks-repo` | 4 | Repository discovery/layout (`GIT_DIR`, common dir); composes store + refs |
| `arks-protocol` | 5 | pkt-line framing; capability advertise/parse helpers |
| `arks-remote` | 6 | Git remote **client** for claimed protocols; GitHub-class = verify peer ([[sot:root/remote-host-compatibility-surface]]); not forge REST |
| `arks-server` | 6 | Git **server** transport bindings (smart HTTP / claimed SSH / optional daemon) over protocol helpers ([[sot:root/remote-host-compatibility-surface]] **M11**); not forge UX |
| `arks-lease` | 6 | **Default** product lease runtime + **opt-in** high-concurrency pipeline ([[sot:root/lease-concurrency-intent]], [[research:root/20261108083502]]); must not replace Git lock bytes on Git-compat paths |
| `arks` | 7 | Primary package + CLI binary `arks` ([[sot:root/product-naming]]) |

**Object-store exclusion:** `arks-celir` ObjectStore adapter — do **not** add. **Allowed opt-in:** `arks-store` feature `celir-index` → path-deps `celir-index`, `celir-index-runtime` (default OFF) ([[sot:root/storage-celir-boundary]], [[adr:root/20261110052030]]).

### Graph summary (consumer → dependency)

```text
arks (CLI) → arks-repo, arks-protocol, arks-remote?, arks-server?, arks-lease?, arks-index, …
arks-remote → arks-protocol, arks-repo (or store+refs)
arks-server → arks-protocol, arks-repo (or store+refs); MUST NOT depend on arks-remote
arks-lease → arks-repo / arks-refs (coordination); NEVER invents object IDs
arks-repo → arks-store, arks-refs, arks-object, arks-oid
arks-store → arks-object, arks-pack?, arks-oid, arks-hash
arks-store (feature celir-index) → celir-index, celir-index-runtime  # index-only; not ObjectStore
arks-refs → arks-oid (+ object existence checks via store traits as needed)
arks-pack / arks-index → arks-object, arks-oid, arks-hash
arks-object → arks-oid, arks-hash
arks-oid → (none workspace)
arks-hash → (none workspace)
```

Forbidden examples: `arks-hash` → `arks`; `arks-object` → `arks-remote`; `arks-store` → `arks` CLI types.

### Layout intent

Prefer a root virtual workspace with members under `crates/arks-*` (and bin package `arks`) — Cargo ecosystem standard first ([[sot:root/structural-conventions]] §7). Exact tree is design-open until first Cargo.toml lands; update this SoT when membership is real.

### Out of scope here

- Porcelain command taxonomy / help UX (sibling).
- Pinning third-party crate versions (implementation ADR later). Whether any early milestone introduces transitional cold `gix` (default: no) is an implementation choice under [[adr:root/20261108083507]]; exit before M4 is mandatory if introduced.
- Publishing schedule to crates.io (naming constraints already in [[sot:root/product-naming]]).
- Final high-concurrency commit scheduler design (opt-in L3 refuse-closed until superseding ADR; [[sot:root/lease-concurrency-intent]], [[adr:root/20261108232800]]).

## Why

An explicit inner→outer graph lets AI implementers start at `arks-hash` / `arks-oid` without waiting on CLI or remotes, and prevents accidental coupling that would force Git-semantic inventions at the edges. Declaring engine ownership (no `git` subprocess for core ops) keeps concurrency and allocator policy under arks control while stock Git remains the clean-room peer. Aligning names with [[sot:root/product-naming]] avoids registry and brand collisions.

## Verification

1. `spec check` clean for this SoT.
2. When workspace exists: `cargo metadata` shows no path from Rings 0–5 into Ring 7 packages; no path from Rings 0–4 into `arks-remote` except via documented Ring 6 adapters; **default** (no `celir-index` feature) graph has **zero** required `celir-*` / discrete-circuit edges; with `--features celir-index`, edges are limited to index crates cited by [[sot:root/storage-celir-boundary]].
3. Product paths in Rings 0–5 MUST NOT spawn `git` for claimed ops; clean-room/oracle use of `git` is allowed only in test/acceptance harnesses.
4. Default / product builds of Rings 0–4 and `arks-lease` MUST NOT take a `gix`/gitoxide dependency; any transitional cold use must follow [[adr:root/20261108083507]] and be absent by M4 exit (gix-exit). Workspace Cargo graph at/after M4 MUST show zero `gix`/gitoxide edges.
5. New crate proposals update this SoT before merge.
6. Clean-room gates for crate slices follow [[sot:root/product-verification-clean-room]] and [[plan:root/20261108082110]].
