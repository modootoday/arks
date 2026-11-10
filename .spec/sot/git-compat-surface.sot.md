---
id: sot:root/git-compat-surface
title: Git compatibility surface
status: active
created: 2026-11-08T07:19:00Z
updated: 2026-11-08T10:54:46Z
tags: [git, compatibility, standards-first, engine, formats, protocols, dot-git, layout]
verify: manual
rel:
  depends_on: [research:root/20261108071859, research:root/20261108071913, research:root/20261108071914, research:root/20261108071915, research:root/20261108071916, research:root/20261108071917, adr:root/20261108083507, sot:root/cli-surface, sot:root/product-intent]
  evidence: [research:root/20261108071859, research:root/20261108071913, research:root/20261108071914, research:root/20261108071915, research:root/20261108071916, research:root/20261108071917, research:root/20261108073228, research:root/20261108183800]
---

# Git compatibility surface

Version **0.3.1**. Normative **intent** for arks: honor publicly documented Git **format and protocol** semantics so repositories and wire sessions remain interchangeable with stock Git on the **same on-disk `.git` layout**. Performance work is welcome only inside that envelope. This SoT does **not** restate full format grammars; those live in cited primary docs and domain research.

**Layer split (normative):**

| Layer | Meaning | Owner |
|-------|---------|-------|
| **Format / protocol / `.git` layout** | Objects, refs, index, packs, repository directory layout, pkt-line, HTTP/v2, extensions | **This SoT** |
| **CLI argv / porcelain / alias drop-in** | Command names, flags, refuse policy, parity inventory, `git`→`arks` alias suitability | [[sot:root/cli-surface]] |
| **Product production** | Both layers closed (same-`.git` + CLI symmetry + alias-ready + remotes + engine) | [[sot:root/program-roadmap]] |

**Compatibility = in-engine fidelity.** arks implements claimed surfaces in its **native Rust engine** ([[adr:root/20261108083507]], [[sot:root/product-intent]]). “Compatible” at this layer means semantic / byte / protocol fidelity to Git primary docs **and** round-trip interoperability with stock Git — **not** wrapping the `git` executable as the product runtime. Full product compatibility **also** requires Git CLI symmetric support ([[sot:root/cli-surface]]); format interop alone is **not** production.

## Rule

### Standards-first, performance-second

1. **Primary sources only for semantics.** Object, index, pack, ref, and protocol behavior arks claims to support MUST be backed by a Git project / git-scm primary document catalogued in [[research:root/20261108071859]] (or a later superseding research doc). Secondary sources may point, never define.
2. **Do not invent.** Bytes, ABNF productions, capability names, extension keys, and refuse/ignore rules are not design knobs. If undocumented, treat as out-of-scope or research-gap — not as license to improvise.
3. **Implemented in-engine.** Claimed format/protocol behavior MUST be realized by arks Rust crates ([[sot:root/crate-architecture]]). Stock `git` is the clean-room **peer/oracle**, not the implementation ([[adr:root/20261108083507]]).
4. **Negotiation gates change.** Wire or on-disk behavior that differs from classic defaults is allowed only when a documented version, capability, or `extensions.*` mechanism selects it (see repository-version and gitprotocol-* docs).
5. **Unknown extensions fail closed.** On `core.repositoryformatversion=1`, unknown or unimplemented `extensions.*` keys/values MUST cause arks to refuse the repository (repository-version). Do not set extensions arks does not implement.
6. **Optional caches are optional.** commit-graph, bitmaps, MIDX, untracked cache, fsmonitor, and similar ignore-safe artifacts MUST NOT change object identity or ref history. Prefer omit/ignore over writing an incompatible cache.
7. **Performance never breaks documented semantics.** Faster packing, concurrency, leasing, allocator policy, or storage (including future CELIR) MUST preserve documented object IDs, pack/index/ref meanings, and negotiated wire results unless a versioned capability explicitly changes them ([[sot:root/lease-concurrency-intent]], [[research:root/20261108083502]]).
8. **Interchange test is stock Git.** A repository or session arks writes SHOULD be readable/usable by current stock Git for the claimed feature set; a repository stock Git writes for that set SHOULD be readable by arks.
9. **Same on-disk `.git` layout (normative).** In Git-compat mode, arks MUST read and write the **same repository directory layout** stock Git uses — typically a `.git/` directory (or a bare repository root) containing the claimed paths under [gitrepository-layout](https://git-scm.com/docs/gitrepository-layout) (objects, refs, HEAD, index, config, hooks, … as claimed). arks MUST NOT use a **parallel proprietary store** as the day-to-day Git-compat repository for claimed workflows. Optional CELIR **indexing** sidecars (feature `celir-index`) MUST stay **outside** Git object/ref/pack meaning and MUST NOT silently replace this layout for Git-compat mode ([[sot:root/storage-celir-boundary]], [[adr:root/20261110052030]], [[sot:root/product-intent]]).

### Must honor (compatibility in-scope)

When GIR implements the corresponding feature, it MUST follow the cited primary docs:

| Surface | Primary hubs | Research |
|---------|--------------|----------|
| Loose objects + packs + idx/rev/MIDX | gitformat-loose, gitformat-pack, gitformat-chunk | [[research:root/20261108071913]] |
| Bundles + signatures | gitformat-bundle, gitformat-signature | [[research:root/20261108071913]] |
| Commit-graph (if written/read) | gitformat-commit-graph | [[research:root/20261108071913]] |
| Index v2/v3/v4 + documented extensions | gitformat-index | [[research:root/20261108071915]] |
| Repository layout paths (same `.git` / bare root; no parallel proprietary Git-compat store) | gitrepository-layout | catalog + refs research; this SoT §Rule #9 |
| Refs, packed-refs, refnames, reflogs | layout, gitprotocol-common, check-ref-format, update-ref; reftable if enabled | [[research:root/20261108071914]] |
| pkt-line + v1 pack protocol + HTTP dumb/smart + v2 | gitprotocol-common/capabilities/pack/http/v2 | [[research:root/20261108071916]] |
| Shallow, filter/partial-clone, alternates, replace | capabilities, partial-clone, shallow, layout | [[research:root/20261108071917]] |
| SHA-256 / objectFormat transition | hash-function-transition, git-config extensions | [[research:root/20261108071917]] |

### Remote / host compatibility

Git **formats and wire** alone are not enough for real workflows: arks MUST interoperate as a Git **remote client** with **any** peer speaking claimed protocols (GitHub-class hosts = representative verify peer) and, under **M11**, as a Git **server** (upload-pack / receive-pack for claimed transports). Normative remote/host intent, the (A) Git-wire vs (B) product-API scope split, **locked server role**, and the test-plan sketch live in [[sot:root/remote-host-compatibility-surface]]. Citation catalogs: [[research:root/20261108073228]], [[research:root/20261108071916]]. Host product APIs (REST/GraphQL PRs/Checks/Apps) are design-mapped there but are **not** Git-remote MVP and are **not** production-server.

### Explicitly out of scope (this SoT)

- **Lease / concurrency product design** beyond what Git already documents for ref locking and index locking — owned by [[sot:root/lease-concurrency-intent]].
- **CELIR object store or other alternate storage** that is not advertised as a Git extension — boundary in [[sot:root/storage-celir-boundary]]; optional index-only dual-path MUST NOT silently replace Git object store semantics **or** the same-`.git` layout for Git-compat mode.
- **Inner product intent / crate graph / object-model vocabulary** — [[sot:root/product-intent]], [[sot:root/crate-architecture]], [[sot:root/core-object-model]] (cite; do not fork format grammars there).
- **Thin `git` CLI wrapper architecture** — rejected by [[adr:root/20261108083507]]; this SoT only defines fidelity/interop rules the engine must meet.
- **gitoxide/`gix` as product or permanent workspace dependency** — rejected. Hot paths always in-house; end state is zero Cargo edges on `gix`/gitoxide (including optional features and permanent `dev-dependencies`); transitional cold use only under [[adr:root/20261108083507]] with mandatory exit before M4.
- **CLI command/flag inventory, refuse-vs-implement, and porcelain wave scheduling** — owned by [[sot:root/cli-surface]] / [[sot:root/program-roadmap]]. This SoT does not defer or redefine CLI-parity end-state; it only owns bytes/wire.
- **Inventing new capabilities or extensions** without a Git-project-documented registration path (`extensions.*` claim note in git-config; protocol capability docs).
- **Remote/host product layer** (generic client + GitHub-class verify, forge APIs, **M11** server role) — owned by [[sot:root/remote-host-compatibility-surface]]; this SoT keeps formats/protocols.
- **Treating “formats green + CLI subset” as product production** — superseded; production requires CLI inventory closure + alias-ready + same-`.git` ([[sot:root/program-roadmap]]).
- **Parallel proprietary on-disk repo for Git-compat daily workflows** — rejected; CELIR/lease modes that diverge must export/import back to stock-readable `.git` ([[sot:root/product-intent]], [[sot:root/lease-concurrency-intent]]).

### Phasing principle

Implementations MAY ship a subset of **format/protocol** surfaces (e.g. read packs before write reftable). Claims in docs/tests MUST name the subset. Advertising a capability or writing an extension implies the MUST-honor row above. CLI delivery waves are phased under [[sot:root/cli-surface]]; they do **not** authorize permanent “partial porcelain” as the product compatibility bar.

## Why

arks’s product goal is a **native** Git-compatible engine **and** CLI that later adds performance (leases, allocator policy, storage). Format/protocol debt is existential: diverging object IDs, pack layouts, **`.git` directory shapes**, or capability handling forks the ecosystem. Users must keep one on-disk repo that stock Git and arks both open. CLI debt (including `alias git=arks`) is owned by [[sot:root/cli-surface]] but presupposes this same-`.git` foundation. Implementing fidelity in-engine (with stock Git as peer) allows concurrency and memory policy without becoming a wrapper.

## Verification

Manual until harness exists. **Authoritative** venue for product interop/acceptance claims is the Docker clean-room defined in [[sot:root/product-verification-clean-room]] (matrix: [[research:root/20261108073650]]). Host stock-Git runs are advisory only.

Minimum checks for changes touching Git-compat code or specs:

1. `spec check` clean for affected documents; links to primary URLs remain accurate.
2. Every new semantic claim cites a P0/P1/P2 source from [[research:root/20261108071859]].
3. When implementation exists: round-trip fixtures with **in-image** stock Git for the claimed subset (init/hash-object/update-ref/pack-objects/index-pack/fetch/push as applicable), executed in clean-room — stock Git as oracle, arks engine as system under test.
4. Refuse tests: unknown `extensions.*`, unknown bundle v3 capability, mismatched object-format.
5. Performance PRs must list which documented semantics were preserved and which capability/version gates any intentional difference.
6. PRs that implement claimed surfaces by shelling out to `git` fail this SoT and [[adr:root/20261108083507]].

Baseline Git version for CI fixtures: **to be pinned** with the clean-room image lock (track git-scm man-page versions used in research: observed through 2.56.x on 2026-11-08).
