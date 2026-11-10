---
id: sot:root/core-object-model
title: Core object model
status: active
created: 2026-11-08T08:21:10Z
updated: 2026-11-08T09:15:00Z
tags: [objects, refs, identity, core, engine]
verify: manual
rel:
  depends_on: [sot:root/product-intent, sot:root/crate-architecture, sot:root/git-compat-surface, research:root/20261108071913, research:root/20261108071914, research:root/20261108071915, adr:root/20261108083507]
---

# Core object model

Version 0.2.2. Normative **inner** model for identity, objects, and refs as arks core types map onto Git-compatible semantics. **Does not rewrite on-disk formats** — formats and primary URLs live in [[sot:root/git-compat-surface]] and domain research ([[research:root/20261108071913]], [[research:root/20261108071914]], [[research:root/20261108071915]]).

Crate mapping: [[sot:root/crate-architecture]] (`arks-hash`, `arks-oid`, `arks-object`, `arks-refs`, `arks-store`, `arks-repo`). Engine ownership: [[adr:root/20261108083507]].

## Rule

### Identity

1. **Object ID (OID)** is the hash of the **uncompressed** object bytes (`type SP size NUL` + payload), per gitformat-loose and repository object format (SHA-1 or SHA-256). Summarized in [[research:root/20261108071913]]; do not invent alternate identity functions in Git-compat mode.
2. **arks-oid** represents an OID as a fixed-width digest selected by the repository’s object format — not a path string.
3. **arks-hash** computes digests **in-engine**; it MUST match stock Git for the same uncompressed bytes and hash version. Matching MAY be proven via clean-room oracle comparison; the product path MUST NOT shell out to `git hash-object` to obtain OIDs.
4. Optional caches (commit-graph, bitmaps, MIDX, …) MUST NOT change OID identity ([[sot:root/git-compat-surface]]).

### Object kinds (content model)

| Kind | Role in core | Format authority |
|------|--------------|------------------|
| `blob` | File content payload | gitformat-loose + plumbing docs (via research) |
| `tree` | Directory listing of mode/name/oid | Same; entry ordering/encoding as stock Git accepts |
| `commit` | Snapshot + parents + metadata | Same |
| `tag` | Annotated tag object | Same |

1. Core MAY expose typed Rust enums/structs for these kinds in `arks-object`.
2. Unknown types MUST NOT be invented for Git-compat interchange.
3. Payload field layouts are **not** restated here — implementers follow primary docs cited by research/git-compat.

### Storage views (logical)

| View | Meaning | Crate |
|------|---------|-------|
| Loose object | Single zlib-compressed object file under `objects/aa/…` | `arks-object` + `arks-store` fs backend |
| Packed object | Object inside pack (+ idx) | `arks-pack` + store |
| Store facade | put/get/exists independent of backend | `arks-store` ([[sot:root/storage-celir-boundary]]) |

CELIR MUST NOT appear in the content model; it is only a future backend behind the facade.

### Refs (name → OID)

1. A **ref** is a name pointing at an OID (or a symref to another ref), per gitrepository-layout / protocol refname rules summarized in [[research:root/20261108071914]].
2. **arks-refs** validates refnames with the same refuse rules stock Git documents (`check-ref-format` / gitprotocol-common).
3. Files backend (loose + `packed-refs`) is first; reftable only when claimed and extension-gated ([[sot:root/git-compat-surface]]).
4. Ref updates that claim Git-compat MUST use Git lock semantics (see [[sot:root/lease-concurrency-intent]] L0).
5. HEAD, peeled tags (`^{}`), and replace refs follow documented Git behavior — cite primary sources at implementation time; do not invent.

### Index (staging)

1. The index is a separate artifact (gitformat-index) owned conceptually by `arks-index`, not a fourth object kind.
2. Index extensions follow fail-closed / documented rules in [[research:root/20261108071915]] and git-compat.

### Layering relative to outer systems

```text
arks-hash / arks-oid
        ↓
arks-object  →  arks-pack / arks-index
        ↓
arks-refs + arks-store → arks-repo
        ↓
arks-protocol / remotes / CLI   (outer; must consume, not redefine, this model)
```

Outer layers MAY add product metadata (leases, agent annotations) **beside** this model; they MUST NOT change OID computation or refname legality for Git-compat paths.

### Engine parsing / IO

1. Loose/pack/ref/index **decode and encode** for claimed subsets live in engine crates ([[sot:root/crate-architecture]]); they are not delegated to a `git` subprocess on product paths ([[adr:root/20261108083507]]).
2. Memory policy for concurrent decode/encode (arena/pool/zero-copy principles) is guided by [[research:root/20261108083502]] but MUST NOT change OID identity or on-disk Git-compat layouts.
3. Concurrent writers remain subject to [[sot:root/lease-concurrency-intent]] (Git locks on Git-compat paths; leases additive).

### Out of scope

- Full ABNF restatement of pack/index/ref formats (use primary docs).
- Protocol capability tables (protocols research + remote SoT).
- Porcelain status/pathspec UX (sibling).
- Whether early milestones introduce transitional cold `gix` (default: never; OID/hash and hot object paths stay in-house; zero-`gix` end state and M4 gix-exit locked in [[adr:root/20261108083507]]).

## Why

Implementers need a crisp inner vocabulary (OID, object kinds, refs, store facade) without pasting copyrighted format grammars. Anchoring that vocabulary to git-compat and research keeps bottom-up crates aligned with stock Git from the first hash function upward, while engine ownership keeps identity computation under arks control for concurrency and allocator work.

## Verification

1. `spec check` clean for this SoT and linked research.
2. Hash/OID/object round-trips against in-image stock Git in clean-room when those crates claim readiness ([[sot:root/product-verification-clean-room]]; slices in [[plan:root/20261108082110]]).
3. Every new content-model claim cites git-compat or catalogued primary research — no invented types.
4. Refuse tests: wrong object-format hash; illegal refnames; unknown repository extensions when format version requires refusal.
5. Product hash/object/ref paths MUST NOT require spawning `git` to function.
