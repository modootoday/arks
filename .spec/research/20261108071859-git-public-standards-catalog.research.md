---
id: research:root/20261108071859
title: Git public standards catalog
status: active
created: 2026-11-08T07:18:59Z
updated: 2026-11-08T07:35:25Z
tags: [git, standards, catalog, primary-source]
---

# Git public standards catalog

Catalog of **authoritative public** Git technical documentation for GIR compatibility work. Prefer Git project / git-scm primary sources. Non-authoritative material is marked secondary. Do not invent semantics beyond cited docs.

Research snapshot date: **2026-11-08**. Git man-page change logs on git-scm were observed spanning releases through **2.56.x** for several protocols/formats (exact per-page dates noted where fetched).

## Question

Which publicly documented Git on-disk formats, wire protocols, repository layout rules, and interop features must GIR treat as normative for compatibility, and where do those rules live?

## Method

1. Enumerate modern man pages on [git-scm.com/docs](https://git-scm.com/docs) named `gitformat-*` and `gitprotocol-*` (plus repository layout, hash transition, and closely related technical pages).
2. Cross-check the Git upstream tree `Documentation/technical/` via the [git/git GitHub API listing](https://api.github.com/repos/git/git/contents/Documentation/technical) (design notes; some are normative, some advisory).
3. Prefer primary pages over Pro Git book chapters; cite the book only as secondary narrative.
4. Record URL, scope, authority tier, and why it matters for GIR interop. Domain detail lives in linked research docs.

## Authority tiers

| Tier | Meaning |
|------|---------|
| **P0 — Normative format/protocol** | git-scm man pages that define bytes, ABNF, MUST/SHOULD wire or on-disk behavior (`gitformat-*`, `gitprotocol-*`, layout, index). |
| **P1 — Normative adjacent** | Official Git docs that define repository extensions, config keys, or command contracts required for interop (e.g. `git-config` `extensions.*`, `gitrepository-layout`). |
| **P2 — Technical design (Git project)** | `Documentation/technical/*.adoc` design notes; follow when they describe shipped formats (reftable, partial-clone, shallow, MIDX narrative). Treat as primary when mirrored on git-scm under the same title. |
| **S — Secondary** | Pro Git book, blogs, Stack Overflow. Use only as pointers back to P0–P2. |

## Findings

### A. Modern format man pages (P0)

| Doc | URL | Scope | Why for GIR |
|-----|-----|-------|-------------|
| gitformat-loose | https://git-scm.com/docs/gitformat-loose | Loose object zlib files; type/size prefix; SHA-1/SHA-256 object id; optional object-map under compat hash | Must read/write interchangeable loose objects |
| gitformat-pack | https://git-scm.com/docs/gitformat-pack | Pack v2/v3, idx v1/v2, reverse index `.rev`, `.mtimes`, multi-pack-index | Pack is the dominant store and wire payload |
| gitformat-index | https://git-scm.com/docs/gitformat-index | Index versions 2/3/4, extensions (cache-tree, resolve-undo, split-index, untracked, fsmonitor, sparse, sdir) | Working-tree interop with stock Git |
| gitformat-bundle | https://git-scm.com/docs/gitformat-bundle | Bundle v2/v3, prerequisites, refs, embedded pack, capabilities | Offline transport compatibility |
| gitformat-chunk | https://git-scm.com/docs/gitformat-chunk | Shared chunk TOC used by commit-graph and MIDX | Correct parsing of chunked files |
| gitformat-commit-graph | https://git-scm.com/docs/gitformat-commit-graph | CGPH commit-graph chunks, hash version, generation data, Bloom | Optional acceleration; must not corrupt if written |
| gitformat-signature | https://git-scm.com/docs/gitformat-signature | Detached signature embedding (PGP/SSH/X.509 armor) in objects/pushes | Preserve signed object bytes |

Legacy aliases still redirect on git-scm (same content family): `pack-format`, `commit-graph-format`, `index-format` → prefer `gitformat-*` names.

Domain notes: [[research:root/20261108071913]], [[research:root/20261108071915]].

### B. Modern protocol man pages (P0)

| Doc | URL | Scope | Why for GIR |
|-----|-----|-------|-------------|
| gitprotocol-common | https://git-scm.com/docs/gitprotocol-common | ABNF, pkt-line, flush/delim/response-end, refname rules | Shared framing for all transports |
| gitprotocol-capabilities | https://git-scm.com/docs/gitprotocol-capabilities | v1 capability advertisement (multi_ack, side-band, ofs-delta, shallow, filter, …) | Negotiation must not invent caps |
| gitprotocol-pack | https://git-scm.com/docs/gitprotocol-pack | fetch-pack/upload-pack and send-pack/receive-pack over ssh/git/file | Classic pack protocol |
| gitprotocol-http | https://git-scm.com/docs/gitprotocol-http | Dumb + smart HTTP, info/refs, Content-Types, POST services | Dominant hosting transport |
| gitprotocol-v2 | https://git-scm.com/docs/gitprotocol-v2 | Protocol v2 command model, ls-refs, fetch, object-info, bundle-uri | Required for modern servers |

Legacy aliases: `pack-protocol`, `http-protocol`, `protocol-v2`, `protocol-capabilities`, `protocol-common`.

Domain notes: [[research:root/20261108071916]].

### C. Repository layout, versions, hash transition (P0/P1)

| Doc | URL | Scope | Why for GIR |
|-----|-----|-------|-------------|
| gitrepository-layout | https://git-scm.com/docs/gitrepository-layout | `$GIT_DIR` layout: objects, refs, packed-refs, logs, shallow, alternates, worktrees, modules, index, hooks | On-disk interchange with stock Git |
| repository-version | https://git-scm.com/docs/repository-version | `core.repositoryformatversion` 0/1; unknown `extensions.*` MUST refuse | Hard compatibility gate |
| hash-function-transition | https://git-scm.com/docs/hash-function-transition | SHA-256 object format design, compatibility mapping | Documented hash transition path |
| git-config (`extensions.*`) | https://git-scm.com/docs/git-config | Extension keys (`objectFormat`, `compatObjectFormat`, `partialClone`, `refStorage`, …) | Claiming/understanding extensions |

Domain notes: [[research:root/20261108071917]], [[research:root/20261108071914]].

### D. Technical design docs (P2 — Git `Documentation/technical/`)

Source listing (upstream `master` at research time): https://github.com/git/git/tree/master/Documentation/technical

Many are also published on git-scm under the same basename (without `.adoc`).

| Doc (basename) | Typical URL | Scope | Why for GIR |
|----------------|-------------|-------|-------------|
| reftable | https://git-scm.com/docs/reftable | Reftable binary format + stack | Alternate ref storage (`extensions.refStorage=reftable`) |
| partial-clone | https://git-scm.com/docs/partial-clone | Promisor packs, filter capability, `extensions.partialClone` | Partial clone interop |
| shallow | https://git-scm.com/docs/shallow | Shallow graft file / deepen semantics (design) | Shallow clone edge cases |
| multi-pack-index | https://git-scm.com/docs/multi-pack-index | MIDX design narrative (format bytes in gitformat-pack) | Multi-pack lookup |
| bitmap-format | https://git-scm.com/docs/bitmap-format | Pack reachability bitmaps | Optional; ignore-safe for older clients |
| bundle-uri | https://git-scm.com/docs/bundle-uri | Bundle URI advertisement (v2) | Seed clones via bundles |
| packfile-uri | https://git-scm.com/docs/packfile-uri | Packfile-URI in protocol | CDN-style pack fetch |
| sparse-checkout | https://git-scm.com/docs/sparse-checkout | Sparse checkout design | Cone/non-cone patterns |
| sparse-index | https://git-scm.com/docs/sparse-index | Sparse index design | Index extension interplay |
| commit-graph | https://git-scm.com/docs/commit-graph | Commit-graph feature design | Complements gitformat-commit-graph |
| racy-git | https://git-scm.com/docs/racy-git | Racy-git problem | Index mtime correctness |
| parallel-checkout | https://git-scm.com/docs/parallel-checkout | Parallel checkout protocol to workers | Optional performance path |
| long-running-process-protocol | https://git-scm.com/docs/long-running-process-protocol | pkt-line sideband to helpers | Credential/filter helpers |
| pack-heuristics | https://git-scm.com/docs/pack-heuristics | Pack delta heuristics (non-normative packing choices) | Perf only; output still pack-format |
| large-object-promisors | https://git-scm.com/docs/large-object-promisors | Large object promisor design | Future partial-clone related |
| connectivity-check | (technical/) | Connectivity rules | Fsck/transfer safety |
| remembering-renames / directory-rename-detection / trivial-merge / paint-down-to-common / send-pack-pipeline / rerere / scalar / platform-support / build-systems / unit-tests / unambiguous-types / api-* | technical/ | Implementation APIs & heuristics | Mostly non-wire; cite only if GIR reimplements same helper |

### E. Everyday / plumbing docs needed for interop (P1)

Not format specs, but command contracts that define observable interop:

| Area | Primary entry points | Why |
|------|----------------------|-----|
| Object model narrative | https://git-scm.com/docs/gitcore-tutorial , https://git-scm.com/docs/user-manual | Commit/tree/blob/tag semantics in prose |
| Plumbing | `git-hash-object`, `git-cat-file`, `git-mktree`, `git-commit-tree`, `git-update-ref`, `git-pack-objects`, `git-index-pack`, `git-upload-pack`, `git-receive-pack`, `git-fetch-pack`, `git-send-pack` on https://git-scm.com/docs | Byte-compatible CLI/plumbing expectations |
| Refs | `git-pack-refs`, `git-show-ref`, `git-check-ref-format`, `git-update-ref` | Ref naming and packed-refs |
| Notes / replace | `git-notes`, `git-replace` | Optional namespaces `refs/notes`, `refs/replace` |
| Bundle / archive | `git-bundle`, `git-archive` | Offline interchange |
| Transfer porcelains | `git-fetch`, `git-push`, `git-clone` | Capability defaults users expect |
| Hooks | https://git-scm.com/docs/githooks | Server-side hook contracts |
| Ignore / attributes | `gitignore`, `gitattributes` | Working-tree side effects |

### F. Secondary (S) — use only as pointers

| Source | URL | Note |
|--------|-----|------|
| Pro Git — Transfer Protocols | https://git-scm.com/book/en/v2/Git-Internals-Transfer-Protocols | Narrative; defer to gitprotocol-* |
| Pro Git — Git Objects / Packfiles | https://git-scm.com/book/en/v2/Git-Internals-Git-Objects | Narrative; defer to gitformat-* |
| IETF RFCs cited by Git docs | e.g. RFC 5234 (ABNF), RFC 1738/2616/2617 as referenced from gitprotocol-http | Only where Git docs cite them |

### G. Explicitly deferred (not Git public standards)

- GIR lease / concurrency improvements — product SoT later
- CELIR storage — product SoT later
- crates.io rename — naming only; not a Git standards issue

- GitHub-class remote URL/auth/capability expectations (host docs + blogs): [[research:root/20261108073228]]; normative remote SoT: [[sot:root/remote-host-compatibility-surface]] — not a substitute for gitprotocol-* bytes.

## Domain research map

| Domain | Document |
|--------|----------|
| Objects & packs | [[research:root/20261108071913]] |
| Refs & reftable | [[research:root/20261108071914]] |
| Index | [[research:root/20261108071915]] |
| Wire protocols | [[research:root/20261108071916]] |
| Extensions & interop features | [[research:root/20261108071917]] |
| GitHub-class remote hosting | [[research:root/20261108073228]] |
| Compatibility SoT | [[sot:root/git-compat-surface]] |
| Remote/host compatibility SoT | [[sot:root/remote-host-compatibility-surface]] |

## Access / residual gaps

- Full verbatim text of every `Documentation/technical/api-*.adoc` was not ingested; API docs are implementation-oriented, not wire SoT.
- `git-config` extensions list is large; catalog cites the page — exhaustive key-by-key extraction is a follow-up slice.
- Exact shipped Git version pin for “interop baseline” is not chosen yet (see SoT).
